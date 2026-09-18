import { mkdtemp, mkdir, writeFile, symlink, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vitest';
import { guardLocal } from './server';
import { artifactPath, newRun } from './files';
const token = 'a'.repeat(64);
function request(extra: Record<string, string> = {}, method = 'POST') {
  return new Request('http://127.0.0.1:4173/api/local/run', { method, headers: { host: '127.0.0.1:4173', origin: 'http://127.0.0.1:4173', cookie: `sw_local=${token}`, 'x-switchloom-csrf': token, ...extra } });
}
it('requires loopback host, matching origin, session cookie and mutation token', () => {
  expect(() => guardLocal(request(), token)).not.toThrow();
  for (const headers of ([{ origin: 'http://127.0.0.1:5174' }, { host: 'evil.test' }, { cookie: '' }, { 'x-switchloom-csrf': '' }, { 'sec-fetch-site': 'cross-site' }] as Record<string, string>[])) expect(() => guardLocal(request(headers), token)).toThrow();
  expect(() => guardLocal(request({ origin: 'https://evil.test' }, 'GET'), token, true)).toThrow();
});
it('creates independent projects and prevents artifact traversal or symlink escape', async () => {
  const root = await mkdtemp(join(tmpdir(), 'switchloom-files-'));
  try {
    const a = await newRun(root, 'vanilla'); const b = await newRun(root, 'jev'); expect(a.project).not.toBe(b.project);
    await writeFile(join(root, 'outside'), 'private'); await symlink(join(root, 'outside'), join(a.artifacts, 'summary.json'));
    await expect(artifactPath(a.artifacts, 'summary.json')).rejects.toMatchObject({ status: 403 });
    await expect(artifactPath(a.artifacts, '../outside')).rejects.toMatchObject({ status: 404 });
  } finally { await rm(root, { recursive: true, force: true }); }
});
