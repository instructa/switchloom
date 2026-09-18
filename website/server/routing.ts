import { PROMPT_LIMIT, type Decision, type RoutingInput } from '../src/lib/playground.ts';
export type Core = { prepare: (json: string) => string; complete: (json: string, response: string) => string };
export class HttpError extends Error {
  constructor(public status: number, message: string) { super(message); }
}
export async function boundedText(body: ReadableStream<Uint8Array> | null, limit: number): Promise<string> {
  if (!body) return '';
  const reader = body.getReader(); const chunks: Uint8Array[] = []; let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read(); if (done) break;
      size += value.byteLength;
      if (size > limit) { await reader.cancel(); throw new HttpError(413, 'Input is too large.'); }
      chunks.push(value);
    }
  } finally { reader.releaseLock(); }
  const all = new Uint8Array(size); let offset = 0;
  for (const chunk of chunks) { all.set(chunk, offset); offset += chunk.length; }
  return new TextDecoder('utf-8', { fatal: true }).decode(all);
}
export async function readJson(request: Request, limit = 24_000): Promise<unknown> {
  if (!request.headers.get('content-type')?.startsWith('application/json')) throw new HttpError(415, 'Expected JSON.');
  try { return JSON.parse(await boundedText(request.body, limit)); }
  catch (error) { if (error instanceof HttpError) throw error; throw new HttpError(400, 'Invalid JSON.'); }
}
export function prepare(core: Core, input: unknown, promptLimit = PROMPT_LIMIT) {
  const data = input as RoutingInput;
  if (!data || typeof data.request?.task !== 'string' || data.request.task.length > promptLimit ||
      typeof data.request.context !== 'string' || data.request.context.length > promptLimit) throw new HttpError(400, `Prompt and context must each fit ${promptLimit} characters.`);
  const json = JSON.stringify(input);
  try {
    const result = JSON.parse(core.prepare(json)) as { decision?: Decision; query?: unknown };
    return { ...result, json };
  } catch { throw new HttpError(400, 'Invalid routing configuration or assignment.'); }
}
export async function evaluate(core: Core, prepared: ReturnType<typeof prepare>, key: string | undefined, signal?: AbortSignal): Promise<Decision> {
  if (prepared.decision) return prepared.decision;
  if (!key?.trim()) throw new HttpError(503, 'TypeSafe is not configured.');
  try {
    const response = await fetch('https://api.typesafe.ai/v1/systemone', {
      method: 'POST', headers: { Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(prepared.query), redirect: 'manual',
      signal: signal ? AbortSignal.any([signal, AbortSignal.timeout(5000)]) : AbortSignal.timeout(5000),
    });
    if (!response.ok) { await response.body?.cancel(); throw new HttpError(502, `TypeSafe returned HTTP ${response.status}. No assignment was made.`); }
    return JSON.parse(core.complete(prepared.json, await boundedText(response.body, 65536))) as Decision;
  } catch (error) {
    if (error instanceof HttpError) throw error;
    throw new HttpError(502, error instanceof Error && error.name === 'TimeoutError'
      ? 'TypeSafe exceeded the request deadline. No assignment was made.'
      : 'TypeSafe could not return a valid decision. No assignment was made.');
  }
}
export function json(data: unknown, status = 200, headers?: HeadersInit) {
  return Response.json(data, { status, headers: { 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff', ...headers } });
}
export function apiError(error: unknown) {
  return json({ error: error instanceof HttpError ? error.message : 'Request failed.' }, error instanceof HttpError ? error.status : 500);
}
