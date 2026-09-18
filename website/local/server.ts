import { randomBytes, timingSafeEqual } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { initSync, prepare as corePrepare, complete } from '../../target/web/routing.js';
import { PROMPT_LIMIT } from '../src/lib/playground.ts';
import { apiError, evaluate, HttpError, json, prepare, readJson } from '../server/routing.ts';
import { artifactPath, folders, loadRun, saveJson } from './files.ts';
import { Codex } from './codex.ts';
import { Preview } from './preview.ts';
import { Runner, type RunConfig } from './runner.ts';
export function guardLocal(request: Request, token: string, initial = false) {
  const url = new URL(request.url);
  if (!['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname) || request.headers.get('host') !== url.host ||
    request.headers.get('sec-fetch-site') === 'cross-site' || (request.headers.has('origin') && request.headers.get('origin') !== url.origin)) throw new HttpError(403, 'Local origin required.');
  if (initial && request.method === 'GET') return;
  const cookie = /(?:^|;\s*)sw_local=([a-f0-9]{64})(?:;|$)/.exec(request.headers.get('cookie') ?? '')?.[1];
  if (!cookie || !timingSafeEqual(Buffer.from(cookie), Buffer.from(token))) throw new HttpError(401, 'Reload the local Playground.');
  if (request.method !== 'GET' && (request.headers.get('origin') !== url.origin || request.headers.get('x-switchloom-csrf') !== token)) throw new HttpError(403, 'Local session verification failed.');
}
export async function createLocalServer() {
  if (process.env.DEVELOPER_MODE !== 'true') throw new Error('Local runner requires DEVELOPER_MODE=true.');
  initSync({ module: await readFile(resolve('target/web/routing_bg.wasm')) });
  const core = { prepare: corePrepare, complete }; const token = Buffer.from(randomBytes(32)).toString('hex');
  let typesafe = process.env.TYPESAFE_API_KEY;
  const secrets = new Set<string>([typesafe, process.env.OPENAI_API_KEY].filter((s): s is string => Boolean(s)));
  const redact = (value: any) => {
    let text = JSON.stringify(value) ?? 'null'; for (const secret of secrets) text = text.split(secret).join('[redacted]');
    text = text.replace(/sk-[A-Za-z0-9_-]{16,}/g, '[redacted]'); return JSON.parse(text);
  };
  const previewProcess = new Preview();
  const codex = new Codex();
  if (process.env.OPENAI_API_KEY?.trim()) await codex.login({ type: 'apiKey', apiKey: process.env.OPENAI_API_KEY.trim() });
  const runner = new Runner(codex, core, () => typesafe); runner.setRedactor(redact);
  const imported = new Map<string, { artifacts: string; summary: any; config: any }>();
  return {
    close() { previewProcess.stop(); void runner.stop('Local server closed.'); codex.stop(); },
    async fetch(request: Request): Promise<Response> {
      try {
        const url = new URL(request.url); const path = url.pathname;
        guardLocal(request, token, path === '/api/config');
        if (path === '/api/config' && request.method === 'GET') return json({ developerMode: true, promptLimit: PROMPT_LIMIT, dailyLimit: null, csrf: token, typesafeConfigured: Boolean(typesafe) }, 200, { 'Set-Cookie': `sw_local=${token}; HttpOnly; SameSite=Strict; Path=/` });
        if (path === '/api/route' && request.method === 'POST') {
          const start = performance.now(); const input = prepare(core, await readJson(request));
          return json({ decision: await evaluate(core, input, typesafe, request.signal), elapsedMs: Math.round(performance.now() - start), remaining: null });
        }
        if (path === '/api/local/account' && request.method === 'GET') {
          await codex.start(); const account = await codex.call('account/read', { refreshToken: false });
          return json({ authMode: account.account?.type ?? null, typesafeConfigured: Boolean(typesafe) });
        }
        if (path === '/api/local/login' && request.method === 'POST') {
          if (runner.run?.status === 'running') throw new HttpError(409, 'Finish the run before changing credentials.');
          const body = await readJson(request, 5000) as { type: string; key?: string };
          if (!['typesafe', 'apiKey', 'chatgpt'].includes(body.type)) throw new HttpError(400, 'Unknown login method.');
          if (body.type !== 'chatgpt' && (!body.key?.trim() || body.key.length > 4096)) throw new HttpError(400, 'Enter a valid key.');
          if (body.key) secrets.add(body.key);
          if (body.type === 'typesafe') { typesafe = body.key!.trim(); return json({ configured: true }); }
          const result = await codex.login(body.type === 'apiKey' ? { type: 'apiKey', apiKey: body.key! } : { type: 'chatgpt' });
          return json({ authUrl: result.authUrl ?? null, type: result.type });
        }
        if (path === '/api/local/folders' && request.method === 'GET') return json(await folders(url.searchParams.get('path') ?? undefined));
        if (path === '/api/local/run' && request.method === 'POST') return json(await runner.start(await readJson(request, 64_000) as RunConfig));
        if (path === '/api/local/run' && request.method === 'GET') return json({ run: runner.summary(), runs: [...runner.runs.values()].map(r => runner.summary(r)) });
        if (path === '/api/local/stop' && request.method === 'POST') { await runner.stop(); return json({ stopped: true }); }
        if (path === '/api/local/approval' && request.method === 'POST') {
          const body = await readJson(request, 1000) as { id: number | string; accept: boolean };
          if (typeof body.accept !== 'boolean') throw new HttpError(400, 'Choose approve or decline.');
          runner.approve(body.id, body.accept); return json({ accepted: true });
        }
        if (path === '/api/local/events' && request.method === 'GET') {
          let dispose: () => void = () => {};
          const body = new ReadableStream<Uint8Array>({
            start(controller) {
              const encoder = new TextEncoder();
              const emit = (event: unknown) => controller.enqueue(encoder.encode(`data: ${JSON.stringify(event)}\n\n`));
              for (const event of runner.run?.events ?? []) emit(event);
              const timer = setInterval(() => controller.enqueue(encoder.encode(': keepalive\n\n')), 15000);
              runner.listeners.add(emit);
              dispose = () => { clearInterval(timer); runner.listeners.delete(emit); };
              request.signal.addEventListener('abort', () => { dispose(); try { controller.close(); } catch {} }, { once: true });
            }, cancel() { dispose(); },
          });
          return new Response(body, { headers: { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-store', 'X-Accel-Buffering': 'no' } });
        }
        if (path === '/api/local/import' && request.method === 'POST') {
          const body = await readJson(request, 4000) as { path: string }; const run = await loadRun(body.path);
          const id = Buffer.from(randomBytes(8)).toString('hex'); imported.set(id, run); return json({ ...run.summary, id, config: run.config });
        }
        if (path === '/api/local/artifact' && request.method === 'GET') {
          const id = url.searchParams.get('run') ?? ''; const run = runner.runs.get(id) ?? imported.get(id);
          if (!run) throw new HttpError(404, 'Run not found.');
          const name = url.searchParams.get('name') ?? ''; const path = await artifactPath(run.artifacts, name);
          return new Response(await readFile(path), { headers: { 'Content-Type': name.endsWith('.png') ? 'image/png' : 'text/plain; charset=utf-8', 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff', 'Content-Security-Policy': "default-src 'none'; sandbox" } });
        }
        if (path === '/api/local/preview' && request.method === 'GET') return json(redact(previewProcess.status()));
        if (path === '/api/local/preview' && request.method === 'POST') {
          const body = await readJson(request, 2000) as { id: string; port: number; stop?: boolean };
          if (body.stop === true) { previewProcess.stop(); return json({ stopped: true }); }
          const run = runner.runs.get(body.id); if (!run) throw new HttpError(404, 'Run not found.');
          if (run.status === 'running') throw new HttpError(409, 'Wait for the benchmark to finish before previewing.');
          return json(redact(await previewProcess.start(run.project, body.port, Number(url.port))));
        }
        if (path === '/api/local/screenshot' && request.method === 'POST') {
          const { id, url: preview } = await readJson(request, 3000) as { id: string; url: string };
          const run = runner.runs.get(id); if (!run) throw new HttpError(404, 'Run not found.');
          const target = new URL(preview);
          if (target.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(target.hostname) || !target.port || target.port === url.port || target.username || target.password) throw new HttpError(400, 'Use a separate local preview port.');
          const { chromium } = await import('playwright');
          let browser: Awaited<ReturnType<typeof chromium.launch>> | undefined;
          try {
            browser = await chromium.launch({ headless: true });
            const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, serviceWorkers: 'block' });
            await page.route('**/*', async route => {
              const dest = new URL(route.request().url());
              if (dest.origin === url.origin || (route.request().isNavigationRequest() && dest.origin !== target.origin)) await route.abort(); else await route.continue();
            });
            await page.goto(target.href, { waitUntil: 'networkidle', timeout: 20000 });
            await page.screenshot({ path: join(run.artifacts, 'screenshot.png'), fullPage: true });
            await saveJson(join(run.artifacts, 'capture.json'), { url: target.href, viewport: { width: 1440, height: 1000 }, time: new Date().toISOString() });
            run.screenshot = true;
            await saveJson(join(run.artifacts, 'summary.json'), runner.summary(run));
            return json({ captured: true });
          } catch { throw new HttpError(502, 'Screenshot failed. Start the project preview and install Chromium with pnpm exec playwright install chromium.'); }
          finally { await browser?.close(); }
        }
        throw new HttpError(404, 'Not found.');
      } catch (error) { return apiError(error); }
    },
  };
}
