import { spawn, type ChildProcess } from 'node:child_process';
import { readFile, access } from 'node:fs/promises';
import { join } from 'node:path';
import { HttpError } from '../server/routing.ts';
export class Preview {
  private child?: ChildProcess;
  private state = { status: 'stopped', url: '', output: '' };
  async start(project: string, port: number, controlPort: number) {
    if (!Number.isInteger(port) || port < 1024 || port > 65535 || port === controlPort) throw new HttpError(400, 'Choose a separate preview port between 1024 and 65535.');
    if (this.child) throw new HttpError(409, 'Stop the active preview first.');
    let pkg: { scripts?: { dev?: string } };
    try { pkg = JSON.parse(await readFile(join(project, 'package.json'), 'utf8')); } catch { throw new HttpError(400, 'Project package.json is missing.'); }
    const script = pkg.scripts?.dev ?? '';
    const hostFlag = /\bvite\b/.test(script) ? '--host' : /\bnext\b/.test(script) ? '--hostname' : null;
    if (!hostFlag) throw new HttpError(400, 'Automatic preview supports Vite and Next dev scripts. Start this project manually and enter its local URL.');
    const pnpm = await access(join(project, 'pnpm-lock.yaml')).then(() => true, () => false);
    const env = { ...process.env }; delete env.TYPESAFE_API_KEY; delete env.OPENAI_API_KEY; delete env.DEVELOPER_MODE;
    this.state = { status: 'starting', url: `http://127.0.0.1:${port}`, output: '' };
    this.child = spawn(pnpm ? 'pnpm' : 'npm', ['run', 'dev', ...(!pnpm ? ['--'] : []), hostFlag, '127.0.0.1', '--port', String(port), ...(hostFlag === '--host' ? ['--strictPort'] : [])], { cwd: project, env, stdio: ['ignore', 'pipe', 'pipe'], detached: process.platform !== 'win32' });
    const append = (chunk: Buffer) => { this.state.output = (this.state.output + chunk.toString()).slice(-6000); };
    const child = this.child;
    child.stdout?.on('data', chunk => { if (this.child === child) append(chunk); }); child.stderr?.on('data', chunk => { if (this.child === child) append(chunk); });
    child.once('spawn', () => { if (this.child === child) this.state.status = 'started'; });
    child.once('error', () => { if (this.child === child) { this.state.status = 'failed'; this.child = undefined; } });
    child.once('exit', code => { if (this.child === child) { this.state.status = code ? 'failed' : 'stopped'; this.child = undefined; } });
    return this.state;
  }
  status() { return this.state; }
  stop() {
    if (this.child?.pid) { try { if (process.platform === 'win32') this.child.kill(); else process.kill(-this.child.pid, 'SIGTERM'); } catch {} }
    this.state.status = 'stopped'; this.child = undefined;
  }
}
