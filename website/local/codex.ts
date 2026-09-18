import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';
import { EventEmitter } from 'node:events';
export type RpcMessage = { id?: number | string; method?: string; params?: Record<string, any>; result?: any; error?: { code: number; message: string } };
export class Codex extends EventEmitter {
  private child?: ChildProcessWithoutNullStreams;
  private pending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void; timer: ReturnType<typeof setTimeout> }>();
  private next = 1;
  private starting?: Promise<void>;
  private temporaryAuth = false;
  async start() {
    if (this.starting) return this.starting;
    this.starting = this.connect();
    try { await this.starting; } catch (e) { this.starting = undefined; throw e; }
  }
  private async connect() {
    const env = { ...process.env }; delete env.TYPESAFE_API_KEY; delete env.OPENAI_API_KEY;
    // Process-only benchmark settings. Never write the user's Codex configuration.
    this.child = spawn('codex', ['-c', 'memories.use_memories=false', '-c', 'memories.generate_memories=false',
      ...(this.temporaryAuth ? ['-c', 'cli_auth_credentials_store="ephemeral"'] : []), 'app-server'], { stdio: 'pipe', env });
    const child = this.child;
    this.child.stderr.resume(); // Never forward auth or provider diagnostics to browser/log files.
    const lines = createInterface({ input: this.child.stdout });
    lines.on('line', line => {
      let message: RpcMessage; try { message = JSON.parse(line); } catch { return; }
      if (message.id !== undefined && !message.method) {
        const p = this.pending.get(Number(message.id));
        if (p) { clearTimeout(p.timer); this.pending.delete(Number(message.id)); message.error ? p.reject(new Error(`Codex rejected request (${message.error.code}).`)) : p.resolve(message.result); }
      } else this.emit('message', message);
    });
    const disconnected = () => {
      if (this.child !== child) return;
      for (const p of this.pending.values()) { clearTimeout(p.timer); p.reject(new Error('Codex disconnected.')); }
      this.pending.clear(); this.child = undefined; this.starting = undefined; this.emit('disconnect');
    };
    this.child.once('error', disconnected); this.child.once('exit', disconnected);
    await this.call('initialize', { clientInfo: { name: 'switchloom_playground', version: '1.0.0' } });
    this.send({ method: 'initialized', params: {} });
  }
  call(method: string, params: Record<string, unknown> = {}): Promise<any> {
    if (!this.child?.stdin.writable) return Promise.reject(new Error('Codex is not running.'));
    const id = this.next++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`Codex ${method} timed out.`)); }, 30_000);
      this.pending.set(id, { resolve, reject, timer }); this.send({ id, method, params });
    });
  }
  respond(id: number | string, result: unknown) { this.send({ id, result }); }
  async login(params: { type: 'apiKey'; apiKey: string } | { type: 'chatgpt' }) {
    this.stop(); this.temporaryAuth = true;
    await this.start();
    return this.call('account/login/start', params);
  }
  reject(id: number | string) { this.send({ id, error: { code: -32601, message: 'This client does not support that request.' } }); }
  private send(value: unknown) { this.child?.stdin.write(`${JSON.stringify(value)}\n`); }
  stop() {
    const child = this.child; this.child = undefined; this.starting = undefined;
    for (const pending of this.pending.values()) { clearTimeout(pending.timer); pending.reject(new Error('Codex stopped.')); }
    this.pending.clear(); child?.kill();
  }
}
