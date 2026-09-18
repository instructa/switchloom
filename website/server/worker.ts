import handler from '@tanstack/react-start/server-entry';
import { DurableObject } from 'cloudflare:workers';
import wasm from '../../target/web/routing_bg.wasm?module';
import { initSync, prepare as corePrepare, complete } from '../../target/web/routing';
import { apiError, evaluate, HttpError, json, prepare, readJson } from './routing';
import { PROMPT_LIMIT } from '../src/lib/playground';
import { LIMITS, quotaPlan, type Counter } from './quota';
initSync({ module: wasm });
const core = { prepare: corePrepare, complete };
export interface Env { TYPESAFE_API_KEY?: string; DEVELOPER_MODE?: string; QUOTA: DurableObjectNamespace<Quota> }
export class Quota extends DurableObject<Env> {
  async fetch(request: Request) {
    if (request.method === 'GET') {
      const session = new URL(request.url).searchParams.get('session');
      const counter = await this.ctx.storage.get<Counter>(`s:${session}`);
      return json({ remaining: Math.max(0, LIMITS.session - (counter && counter.reset > Date.now() ? counter.count : 0)) });
    }
    const { session, ip } = await request.json() as { session: string; ip: string };
    const now = Date.now();
    const plan = await this.ctx.storage.transaction(async storage => {
      const current = await storage.get<Counter>([`s:${session}`, `d:${ip}`, `m:${ip}`, 'global']);
      const p = quotaPlan(session, ip, now, current);
      if (p.allowed) { await storage.put(p.updates); await storage.setAlarm((Math.floor(now / 86400000) + 2) * 86400000); }
      return p;
    });
    return json({ remaining: plan.remaining, retryAfter: plan.retryAfter }, plan.allowed ? 200 : 429);
  }
  async alarm() { await this.ctx.storage.deleteAll(); }
}
function session(request: Request) {
  return /(?:^|;\s*)sw_session=([a-f0-9-]{36})(?:;|$)/.exec(request.headers.get('cookie') ?? '')?.[1];
}
function sameOrigin(request: Request) {
  if (request.headers.get('origin') !== new URL(request.url).origin || request.headers.get('sec-fetch-site') === 'cross-site') throw new HttpError(403, 'Origin is not allowed.');
}
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (env.DEVELOPER_MODE && env.DEVELOPER_MODE !== 'false') return json({ error: 'Developer mode is unavailable in this runtime.' }, 503);
    const path = new URL(request.url).pathname;
    try {
      if (path.startsWith('/api/local')) return json({ error: 'Not found.' }, 404);
      if (path === '/api/config' && request.method === 'GET') {
        const sid = session(request) ?? crypto.randomUUID();
        let remaining: number | null = null;
        try { const quota = env.QUOTA.get(env.QUOTA.idFromName(new Date().toISOString().slice(0, 10))); const response = await quota.fetch(`https://quota/status?session=${sid}`); remaining = ((await response.json()) as { remaining: number }).remaining; } catch { /* Mutations still fail closed when quota is unavailable. */ }
        return json({ developerMode: false, promptLimit: PROMPT_LIMIT, dailyLimit: LIMITS.session, remaining }, 200, {
          'Set-Cookie': `sw_session=${sid}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400${new URL(request.url).protocol === 'https:' ? '; Secure' : ''}`,
        });
      }
      if (path === '/api/route') {
        if (request.method !== 'POST') throw new HttpError(405, 'Use POST.');
        sameOrigin(request);
        const sid = session(request); if (!sid) throw new HttpError(401, 'Reload the Playground to start a session.');
        const start = performance.now(); const prepared = prepare(core, await readJson(request));
        let remaining: number | null = null;
        if (!prepared.decision) {
          if (!env.TYPESAFE_API_KEY) throw new HttpError(503, 'TypeSafe is not configured.');
          const ipBytes = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(`${env.TYPESAFE_API_KEY}:${request.headers.get('CF-Connecting-IP') ?? 'unknown'}`));
          const ip = Array.from(new Uint8Array(ipBytes), b => b.toString(16).padStart(2, '0')).join('');
          const quota = env.QUOTA.get(env.QUOTA.idFromName(new Date().toISOString().slice(0, 10)));
          let allowance: Response;
          try { allowance = await quota.fetch('https://quota/reserve', { method: 'POST', body: JSON.stringify({ session: sid, ip }) }); }
          catch { throw new HttpError(503, 'Quota service unavailable. Try again later.'); }
          const data = await allowance.json() as { remaining: number; retryAfter: number };
          if (!allowance.ok) return json({ error: 'Daily or burst limit reached.', ...data }, 429, { 'Retry-After': String(data.retryAfter) });
          remaining = data.remaining;
        }
        try { return json({ decision: await evaluate(core, prepared, env.TYPESAFE_API_KEY, request.signal), elapsedMs: Math.round(performance.now() - start), remaining }); }
        catch (error) { const failure = apiError(error); return json({ ...(await failure.json() as object), remaining }, failure.status); }
      }
      if (path.startsWith('/api/')) return json({ error: 'Not found.' }, 404);
      const response = await handler.fetch(request);
      const secured = new Response(response.body, response);
      secured.headers.set('X-Content-Type-Options', 'nosniff');
      secured.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
      secured.headers.set('Content-Security-Policy', "default-src 'self'; script-src 'self' 'unsafe-inline' https://analytics.int.macherjek.com; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self' https://analytics.int.macherjek.com; object-src 'none'; base-uri 'self'; frame-ancestors 'none'");
      return secured;
    } catch (error) { return apiError(error); }
  },
};
