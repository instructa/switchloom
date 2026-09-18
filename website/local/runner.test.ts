import { EventEmitter } from 'node:events';
import { mkdtemp, rm, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { beforeAll, afterEach, expect, it, vi } from 'vitest';
import { initSync, prepare as corePrepare, complete } from '../../target/web/routing.js';
import { Runner, type RunConfig } from './runner';
import type { Codex } from './codex';
class FakeCodex extends EventEmitter {
  calls: { method: string; params: any }[] = []; replies: any[] = []; index = 0;
  constructor(private outputs: string[] | null) { super(); }
  async start() {}
  async call(method: string, params: any = {}): Promise<any> {
    this.calls.push({ method, params });
    if (method === 'account/read') return { account: { type: 'chatgpt' } };
    if (method === 'thread/start') return { thread: { id: `thread-${++this.index}` } };
    if (method === 'turn/start') {
      const turn = { id: `turn-${this.calls.length}` };
      if (this.outputs) {
        const text = this.outputs.shift();
        setTimeout(() => {
          this.emit('message', { method: 'turn/started', params: { threadId: params.threadId, turn } });
          this.emit('message', { method: 'item/completed', params: { threadId: params.threadId, item: { type: 'agentMessage', text } } });
          this.emit('message', { method: 'turn/completed', params: { threadId: params.threadId, turn: { ...turn, status: 'completed' } } });
        }, 0);
      }
      return { turn };
    }
    return {};
  }
  respond(id: unknown, result: unknown) { this.replies.push({ id, result }); }
  reject(id: unknown) { this.replies.push({ id, rejected: true }); }
}
const core = { prepare: corePrepare, complete };
beforeAll(async () => initSync({ module: await readFile('target/web/routing_bg.wasm') }));
afterEach(() => vi.unstubAllGlobals());
function config(root: string, mode: RunConfig['mode'] = 'routing'): RunConfig {
  return { root, mode, task: 'Create a hello file and verify it.', baseline: { model: 'gpt-5.6-sol', effort: 'medium' }, profiles: {
    luna: { model: 'gpt-5.6-luna', effort: 'max', capabilities: ['coordination'] },
    sol: { model: 'gpt-5.6-sol', effort: 'medium', capabilities: ['implementation', 'validation'] },
  }, maxMinutes: 1, maxTurns: 6 };
}
function finished(runner: Runner) {
  return new Promise<any>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Run did not finish')), 5000);
    runner.listeners.add(e => { if (e.type === 'run/finished') { clearTimeout(timer); resolve(e.data); } });
  });
}
it('reuses slot tasks and returns worker evidence to the coordinator without Jev', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-run-'));
  const codex = new FakeCodex([JSON.stringify({ status: 'dispatch', objective: 'Create hello.txt', capability: 'implementation', summary: '' }), JSON.stringify({ status: 'complete', result: 'Created and verified hello.txt.' }), JSON.stringify({ status: 'complete', objective: '', capability: null, summary: 'Done, check passed.' })]);
  const runner = new Runner(codex as unknown as Codex, core, () => undefined);
  try {
    const done = finished(runner); await runner.start(config(root)); const result = await done;
    expect(result.status).toBe('completed'); expect(result.jevCalls).toBe(0);
    const turns = codex.calls.filter(c => c.method === 'turn/start');
    expect(turns.map(c => c.params.model)).toEqual(['gpt-5.6-luna', 'gpt-5.6-sol', 'gpt-5.6-luna']);
    expect(turns[0].params.threadId).toBe(turns[2].params.threadId);
    expect(turns[2].params.input[0].text).toContain('Created and verified');
    expect(turns[0].params.input[0].text).toContain('Create a hello file and verify it.');
    expect(turns[2].params.input[0].text).not.toContain('Create a hello file and verify it.');
    expect(codex.calls.filter(c => c.method === 'thread/start')).toHaveLength(2);
    for (const call of codex.calls.filter(c => c.method === 'thread/start')) {
      expect(call.params.sandbox).toBe('workspace-write');
      expect(call.params.config['features.multi_agent']).toBe(false);
      expect(call.params.config['memories.use_memories']).toBe(false);
      expect(call.params.config['memories.generate_memories']).toBe(false);
    }
  } finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});
it('Jev suggestions stop before dispatch, and record the actual classification call', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-run-'));
  const codex = new FakeCodex([JSON.stringify({ status: 'dispatch', objective: 'Determine the next useful action.', capability: null, summary: '' })]);
  const runner = new Runner(codex as unknown as Codex, core, () => 'test-key');
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ model: 'jev-1.13.0', answers: { route: { type: 'choice', choice: 'implementation', confidence: 0.6, probabilities: { implementation: 0.75, validation: 0.25 } }, context_missing: { type: 'noul', noul: 0.1 } }, usage: { input_tokens: 100, output_tokens: 10 } })));
  const cfg = config(root, 'jev'); cfg.profiles.sol.capabilities = ['implementation']; cfg.profiles.astra = { model: 'gpt-6-astra', effort: 'high', capabilities: ['validation'] };
  try { const done = finished(runner); await runner.start(cfg); const result = await done; expect(result.status).toBe('needs_attention'); expect(result.jevCalls).toBe(1); expect(codex.calls.filter(c => c.method === 'turn/start')).toHaveLength(0); }
  finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});
it('rejects concurrent starts and interrupts a running turn', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-run-')); const codex = new FakeCodex(null);
  const runner = new Runner(codex as unknown as Codex, core, () => undefined);
  try {
    const first = runner.start(config(root, 'vanilla')); await expect(runner.start(config(root))).rejects.toMatchObject({ status: 409 }); await first;
    await new Promise<void>(resolve => { const check = () => { if (runner.run?.active?.turnId) resolve(); else setTimeout(check, 1); }; check(); });
    await runner.stop(); expect(runner.summary()?.status).toBe('interrupted'); expect(codex.calls.some(c => c.method === 'turn/interrupt')).toBe(true);
  } finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});

it('routes new work through Jev and resumes the original worker after advice without reclassification', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-advice-'));
  const result = (status: string, result: string) => JSON.stringify({ status, result });
  const step = (status: string, objective: string, summary: string) => JSON.stringify({ status, objective, summary });
  const codex = new FakeCodex([
    result('blocked', 'Existing consumers disagree: decide whether labels should retain punctuation.'),
    step('dispatch', 'Decide punctuation handling for labels.', 'Implementation is blocked by conflicting consumers.'),
    result('complete', 'Preserve punctuation; trim whitespace only. Acceptance: a-b stays a-b.'),
    step('resume', '', 'Proceed with the original implementation.'),
    result('complete', 'Created hello.txt; checked punctuation and whitespace.'),
    step('complete', '', 'Implementation and checks complete.'),
  ]);
  const runner = new Runner(codex as unknown as Codex, core, () => 'test-key');
  const cfg = config(root, 'jev'); cfg.maxTurns = 8;
  cfg.profiles.astra = { model: 'gpt-6-astra', effort: 'high', capabilities: ['planning'] };
  const choices = ['implementation', 'planning'];
  vi.stubGlobal('fetch', vi.fn(async (_url, init) => {
    const query = JSON.parse(init.body); const ids = Object.keys(query.questions.route.criteria); const choice = choices.shift();
    return Response.json({ model: 'jev-1.13.0', answers: { route: { type: 'choice', choice, confidence: 0.99, probabilities: Object.fromEntries(ids.map(id => [id, id === choice ? 1 : 0])) }, context_missing: { type: 'noul', noul: 0 } }, usage: { input_tokens: 100, output_tokens: 10 } });
  }));
  try {
    const done = finished(runner); await runner.start(cfg); const summary = await done;
    expect(summary.status).toBe('completed'); expect(summary.jevCalls).toBe(2); expect(fetch).toHaveBeenCalledTimes(2);
    const turns = codex.calls.filter(c => c.method === 'turn/start');
    expect(turns.map(c => c.params.model)).toEqual(['gpt-5.6-sol', 'gpt-5.6-luna', 'gpt-6-astra', 'gpt-5.6-luna', 'gpt-5.6-sol', 'gpt-5.6-luna']);
    expect(turns[0].params.threadId).toBe(turns[4].params.threadId);
    expect(turns[4].params.input[0].text).toContain('Assignment: ' + cfg.task);
    expect(turns[4].params.input[0].text).toContain('Source result (verbatim, from astra, planning;');
    expect(turns[4].params.input[0].text).toContain('Preserve punctuation; trim whitespace only. Acceptance: a-b stays a-b.');
    expect(Object.keys(turns[1].params.outputSchema.properties)).toEqual(['status', 'objective', 'summary']);
    expect(runner.run?.events.filter(e => e.type === 'routing/resumed')).toHaveLength(1);
    expect(codex.calls.filter(c => c.method === 'thread/start')).toHaveLength(3);
  } finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});

it('snapshots the reference and sends identical image bytes to each task only on its first turn', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-image-'));
  const bytes = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+X1ZkAAAAASUVORK5CYII=', 'base64');
  const cfg = config(root); cfg.referenceImage = join(root, 'original.png'); await writeFile(cfg.referenceImage, bytes);
  const codex = new FakeCodex([
    JSON.stringify({ status: 'dispatch', objective: 'Create hello.txt', capability: 'implementation', summary: '' }),
    JSON.stringify({ status: 'complete', result: 'Created hello.txt.' }),
    JSON.stringify({ status: 'complete', objective: '', capability: null, summary: 'Done.' }),
  ]);
  const runner = new Runner(codex as unknown as Codex, core, () => undefined);
  try {
    const done = finished(runner); await runner.start(cfg); const result = await done;
    const inputs = codex.calls.filter(c => c.method === 'turn/start').map(c => c.params.input);
    expect(inputs.map(input => input.filter((i: any) => i.type === 'localImage').length)).toEqual([1, 1, 0]);
    expect(await readFile(inputs[0][1].path)).toEqual(bytes);
    expect(inputs[0][1].path).not.toBe(cfg.referenceImage);
    expect(inputs[0][1].path).toBe(inputs[1][1].path);
    const manifest = JSON.parse(await readFile(join(result.dir, 'artifacts', 'inputs.json'), 'utf8'));
    expect(manifest.referenceSha256).toBe(createHash('sha256').update(bytes).digest('hex'));
    expect(manifest.taskSha256).toBe(createHash('sha256').update(cfg.task).digest('hex'));
  } finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});

it('records subagent usage and completions without ending or replacing the coordinator turn', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-subagents-'));
  const codex = new FakeCodex(null); const runner = new Runner(codex as unknown as Codex, core, () => undefined);
  const cfg = config(root, 'subagents'); cfg.baseline = { model: 'gpt-5.6-luna', effort: 'max' };
  const emit = (method: string, params: any) => codex.emit('message', { method, params });
  try {
    const done = finished(runner); await runner.start(cfg);
    await vi.waitFor(() => expect(runner.run?.active?.turnId).toBeTruthy());
    const parent = runner.run!.active!.threadId;
    emit('thread/started', { thread: { id: 'child', parentThreadId: parent } });
    emit('turn/started', { threadId: 'child', turn: { id: 'child-turn' } });
    emit('item/completed', { threadId: 'child', item: { type: 'subAgentActivity', kind: 'interacted', agentThreadId: parent, agentPath: '/root' } });
    emit('item/completed', { threadId: parent, item: { type: 'agentMessage', text: 'Parent result with verification.' } });
    emit('item/completed', { threadId: 'child', item: { type: 'agentMessage', text: 'Child result must not replace parent result.' } });
    emit('thread/tokenUsage/updated', { threadId: 'child', tokenUsage: { total: { totalTokens: 321 } } });
    emit('turn/completed', { threadId: 'child', turn: { id: 'child-turn', status: 'completed' } });
    await vi.waitFor(() => expect(runner.run?.subagents.child.status).toBe('completed'));
    expect(runner.summary()?.status).toBe('running');
    emit('turn/completed', { threadId: parent, turn: { status: 'completed' } });
    const summary = await done;
    expect(summary.final).toBe('Parent result with verification.');
    expect(Object.keys(summary.subagents)).toEqual(['child']);
    expect(summary.subagentTurns).toBe(1); expect(summary.usage.child.total.totalTokens).toBe(321);
    expect(summary.usageCoverage).toEqual({ complete: false, missingThreadIds: [parent] });
    expect(codex.calls.find(c => c.method === 'thread/start')?.params.config['features.multi_agent_v2']).toEqual({ enabled: true, expose_spawn_agent_model_overrides: true });
  } finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});

it('cancels interactive MCP input without aborting the benchmark turn', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-elicitation-'));
  const codex = new FakeCodex(null); const runner = new Runner(codex as unknown as Codex, core, () => undefined);
  try {
    const done = finished(runner); await runner.start(config(root, 'vanilla'));
    await vi.waitFor(() => expect(runner.run?.active?.turnId).toBeTruthy());
    const threadId = runner.run!.active!.threadId;
    codex.emit('message', { id: 42, method: 'mcpServer/elicitation/request', params: { threadId, serverName: 'browser', mode: 'form', message: 'Interactive confirmation needed', requestedSchema: { type: 'object', properties: {} } } });
    await vi.waitFor(() => expect(codex.replies).toContainEqual({ id: 42, result: { action: 'cancel', content: null, _meta: null } }));
    expect(runner.summary()?.status).toBe('running');
    expect(codex.calls.some(call => call.method === 'turn/interrupt')).toBe(false);
    codex.emit('message', { method: 'item/completed', params: { threadId, item: { type: 'agentMessage', text: 'Built and tested. Interactive screenshot export unavailable.' } } });
    codex.emit('message', { method: 'turn/completed', params: { threadId, turn: { status: 'completed' } } });
    expect((await done).final).toContain('Interactive screenshot export unavailable.');
  } finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});

it('delivers a review/visual tie to the shared owner and preserves task context through completion', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-owner-'));
  const cfg = config(root, 'jev'); cfg.maxTurns = 6;
  cfg.task = 'Build a dashboard and compare its rendered layout with the reference before completion.';
  cfg.profiles.astra = { model: 'gpt-6-astra', effort: 'high', capabilities: ['review', 'visual'] };
  const codex = new FakeCodex([
    JSON.stringify({ status: 'complete', result: 'Dashboard implemented. Build and tests pass; visual comparison remains.' }),
    JSON.stringify({ status: 'dispatch', objective: 'Compare the rendered dashboard against the visual reference.', summary: 'Implementation and tests are done. The requested visual check is still open.' }),
    JSON.stringify({ status: 'complete', result: 'Compared desktop and mobile renders with the reference. No actionable visual findings. Evidence: artifacts/desktop.png and artifacts/mobile.png.' }),
    JSON.stringify({ status: 'complete', objective: '', summary: 'Implemented and tested by Sol; requested visual comparison completed by Astra with captured evidence.' }),
  ]);
  const runner = new Runner(codex as unknown as Codex, core, () => 'test-key');
  let judgments = 0;
  vi.stubGlobal('fetch', vi.fn(async (_url, init) => {
    const query = JSON.parse(init.body); const ids = Object.keys(query.questions.route.criteria);
    const reviewing = judgments++ > 0;
    return Response.json({ model: 'jev-1.13.0', answers: {
      route: { type: 'choice', choice: reviewing ? 'review' : 'implementation', confidence: reviewing ? 0.03 : 0.99,
        probabilities: Object.fromEntries(ids.map(id => [id, reviewing ? id === 'review' ? 0.53 : id === 'visual' ? 0.47 : 0 : id === 'implementation' ? 1 : 0])) },
      context_missing: { type: 'noul', noul: 0.1 },
    }, usage: { input_tokens: 100, output_tokens: 10 } });
  }));
  try {
    const done = finished(runner); await runner.start(cfg); const summary = await done;
    expect(summary.status).toBe('completed'); expect(summary.jevCalls).toBe(2);
    const turns = codex.calls.filter(c => c.method === 'turn/start');
    expect(turns.map(c => c.params.model)).toEqual(['gpt-5.6-sol', 'gpt-5.6-luna', 'gpt-6-astra', 'gpt-5.6-luna']);
    expect(turns[0].params.input[0].text.split(cfg.task)).toHaveLength(2);
    expect(turns[2].params.input[0].text).toContain('Visual design & review');
    expect(turns[2].params.input[0].text).toContain(cfg.task);
    expect(turns[2].params.input[0].text.split(cfg.task)).toHaveLength(2);
    expect(turns[3].params.input[0].text).toContain('artifacts/desktop.png');
    const decision = runner.run?.events.filter(e => e.type === 'routing/decision').at(-1)?.data;
    expect(decision.reason).toBe('jev_owner'); expect(decision.confidence).toBe(0.03); expect(decision.owner_probability).toBe(1);
  } finally { await runner.stop(); await rm(root, { recursive: true, force: true }); }
});
