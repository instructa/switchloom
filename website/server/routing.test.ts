import { readFile } from 'node:fs/promises';
import { beforeAll, describe, expect, it, vi, afterEach } from 'vitest';
import { initSync, prepare as corePrepare, complete } from '../../target/web/routing.js';
import catalog from '../data/catalog.json';
import { defaultOptions } from '../src/lib/generator';
import { profilesFor } from '../src/lib/playground';
import { prepare, evaluate, readJson } from './routing';
const core = { prepare: corePrepare, complete };
const input = () => ({ request: { task: 'Implement the agreed fix.', context: '', routing: { mode: 'jev' } }, profiles: profilesFor(catalog, defaultOptions(catalog)) });
beforeAll(async () => { initSync({ module: await readFile('target/web/routing_bg.wasm') }); });
afterEach(() => vi.unstubAllGlobals());
describe('shared Rust web routing', () => {
  it('honors configured ownership offline and never calls a provider with Jev disabled', async () => {
    const fetch = vi.fn(); vi.stubGlobal('fetch', fetch);
    const data = input(); data.profiles.sol.model = 'gpt-6-sol';
    const prepared = prepare(core, { ...data, request: { ...data.request, routing: { mode: 'assigned', capability: 'implementation' } } });
    const result = await evaluate(core, prepared, undefined);
    expect(result.assignment?.model).toBe('gpt-6-sol'); expect(result.reason).toBe('explicit_capability'); expect(fetch).not.toHaveBeenCalled();
  });
  it('rejects oversized input and duplicate owners before network', () => {
    const data = input();
    expect(() => prepare(core, { ...data, request: { ...data.request, task: 'x'.repeat(4001) } })).toThrow();
    data.profiles.astra.capabilities.push('implementation'); expect(() => prepare(core, data)).toThrow();
  });
  it('keeps missing-context and confidence policy in Rust and strips provider failures', async () => {
    const prepared = prepare(core, input()); const query = prepared.query as { questions: { route: { criteria: Record<string, unknown> } } };
    const ids = Object.keys(query.questions.route.criteria); expect(ids).not.toContain('coordination');
    const response = { model: 'jev-1.13.0', answers: { route: { type: 'choice', choice: 'review', confidence: 0.8, probabilities: Object.fromEntries(ids.map(id => [id, id === 'review' ? 0.8 : id === 'implementation' ? 0.2 : 0])) }, context_missing: { type: 'noul', noul: 0.1 } }, usage: { input_tokens: 1000, output_tokens: 80 } };
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(response)));
    const result = await evaluate(core, prepared, 'test-only-key');
    expect(fetch).toHaveBeenCalledWith('https://api.typesafe.ai/v1/systemone', expect.objectContaining({ redirect: 'manual' }));
    expect(result.reason).toBe('suggested'); expect(result.act_threshold).toBe(0.85); expect(result.context_missing).toBe(0.1);
    response.answers.context_missing.noul = 0.9;
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(response)));
    expect((await evaluate(core, prepared, 'test-only-key')).assignment).toBeNull();
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('private provider diagnostic', { status: 302, headers: { Location: 'https://untrusted.example' } })));
    await expect(evaluate(core, prepared, 'test-only-key')).rejects.toThrow('No assignment was made');
  });
  it('bounds streamed request bodies instead of trusting Content-Length', async () => {
    const request = new Request('http://localhost/api/route', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ text: 'x'.repeat(25000) }) });
    await expect(readJson(request)).rejects.toMatchObject({ status: 413 });
  });
});
