import { EventEmitter } from 'node:events';
import { PassThrough, Writable } from 'node:stream';
import { expect, it, vi } from 'vitest';
const processes = vi.hoisted(() => [] as any[]);
vi.mock('node:child_process', () => ({
  spawn: (_program: string, args: string[], options: any) => {
    const child: any = new EventEmitter();
    child.args = args; child.options = options; child.messages = [];
    child.stdout = new PassThrough(); child.stderr = new PassThrough(); child.kill = vi.fn();
    child.stdin = new Writable({ write(chunk, _encoding, done) {
      const message = JSON.parse(chunk.toString()); child.messages.push(message);
      if (message.id) queueMicrotask(() => child.stdout.write(JSON.stringify({ id: message.id, result: {} }) + '\n'));
      done();
    } });
    processes.push(child); return child;
  },
}));
import { Codex } from './codex';

it('keeps explicit API login in an ephemeral process store and secrets out of arguments/environment', async () => {
  const codex = new Codex();
  try {
    await codex.start(); const savedLoginProcess = processes.at(-1);
    await codex.login({ type: 'apiKey', apiKey: 'test-private-key' });
    const temporary = processes.at(-1);
    expect(savedLoginProcess.kill).toHaveBeenCalledOnce();
    expect(savedLoginProcess.args).not.toContain('cli_auth_credentials_store="ephemeral"');
    expect(temporary.args).toContain('cli_auth_credentials_store="ephemeral"');
    expect(temporary.args.join(' ')).not.toContain('test-private-key');
    expect(temporary.options.env.OPENAI_API_KEY).toBeUndefined();
    expect(temporary.options.env.TYPESAFE_API_KEY).toBeUndefined();
    expect(temporary.messages.find((m: any) => m.method === 'account/login/start').params).toEqual({ type: 'apiKey', apiKey: 'test-private-key' });
    expect(temporary.args).toContain('memories.use_memories=false');
    expect(temporary.args).toContain('memories.generate_memories=false');
  } finally { codex.stop(); }
});
