import { realpath, stat, readdir, mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname, join, relative, isAbsolute } from 'node:path';
import { homedir } from 'node:os';
import { createHash, randomUUID } from 'node:crypto';
import { HttpError } from '../server/routing.ts';
export async function folders(path = homedir()) {
  const root = await realpath(path);
  const entries = await readdir(root, { withFileTypes: true });
  return { path: root, parent: dirname(root), folders: entries.filter(e => e.isDirectory() && !e.name.startsWith('.')).map(e => e.name).sort().slice(0, 200) };
}
export async function newRun(root: string, mode: string) {
  if (!isAbsolute(root)) throw new HttpError(400, 'Choose an absolute output folder.');
  const canonical = await realpath(root); if (!(await stat(canonical)).isDirectory()) throw new HttpError(400, 'Choose a directory.');
  const id = `${new Date().toISOString().replace(/[:.]/g, '-')}-${mode}-${randomUUID().slice(0, 8)}`;
  const dir = join(canonical, `switchloom-${id}`);
  await mkdir(dir, { mode: 0o700 });
  await mkdir(join(dir, 'project')); await mkdir(join(dir, 'artifacts'), { mode: 0o700 });
  return { id, dir, project: join(dir, 'project'), artifacts: join(dir, 'artifacts') };
}
export async function referenceImage(path: string | undefined) {
  if (path === undefined) return;
  if (typeof path !== 'string' || !isAbsolute(path)) throw new HttpError(400, 'Choose an absolute PNG reference path.');
  const canonical = await realpath(path);
  const info = await stat(canonical);
  if (!info.isFile() || info.size > 8 * 1024 * 1024) throw new HttpError(400, 'Reference must be a PNG file of at most 8 MB.');
  const bytes = await readFile(canonical);
  if (![137, 80, 78, 71, 13, 10, 26, 10].every((byte, index) => bytes[index] === byte)) throw new HttpError(400, 'Reference must be a PNG image.');
  return { bytes, sha256: createHash('sha256').update(bytes).digest('hex') };
}
export async function artifactPath(root: string, name: string) {
  if (!['summary.json', 'events.jsonl', 'config.json', 'inputs.json', 'prompt.md', 'diff.patch', 'screenshot.png', 'checks.json'].includes(name)) throw new HttpError(404, 'Unknown artifact.');
  const canonical = await realpath(root); const path = await realpath(join(root, name));
  const rel = relative(canonical, path);
  if (rel.startsWith('..') || isAbsolute(rel) || !(await stat(path)).isFile()) throw new HttpError(403, 'Invalid artifact path.');
  if ((await stat(path)).size > 32 * 1024 * 1024) throw new HttpError(413, 'Artifact too large.');
  return path;
}
export async function loadRun(dir: string) {
  const artifacts = join(await realpath(dir), 'artifacts');
  const configPath = await artifactPath(artifacts, 'config.json');
  const summaryPath = await artifactPath(artifacts, 'summary.json');
  try {
    const config = JSON.parse(await readFile(configPath, 'utf8'));
    const summary = JSON.parse(await readFile(summaryPath, 'utf8'));
    if (!config || !['vanilla', 'subagents', 'routing', 'jev'].includes(config.mode) || typeof config.task !== 'string' ||
        !config.profiles || typeof config.profiles !== 'object' || Array.isArray(config.profiles) ||
        !config.baseline || typeof config.baseline.model !== 'string' || typeof config.baseline.effort !== 'string' ||
        Object.values(config.profiles).some((p: any) => !p || typeof p.model !== 'string' || typeof p.effort !== 'string' || !Array.isArray(p.capabilities) || p.capabilities.some((c: unknown) => typeof c !== 'string')) ||
        !summary || typeof summary.id !== 'string' || summary.mode !== config.mode || typeof summary.project !== 'string' || typeof summary.dir !== 'string' ||
        !['completed', 'failed', 'interrupted', 'limited', 'needs_attention'].includes(summary.status) ||
        typeof summary.final !== 'string' || ![summary.turns, summary.jevCalls, summary.elapsedMs].every(n => Number.isFinite(n) && n >= 0) ||
        !summary.usage || typeof summary.usage !== 'object' || !Array.isArray(summary.checks) ||
        summary.checks.some((c: any) => !c || typeof c.command !== 'string' || !(c.exitCode === null || Number.isInteger(c.exitCode)))) throw new Error('Invalid result');
    return { artifacts, config, summary: { ...summary, approvals: [], active: undefined } };
  } catch { throw new HttpError(400, 'This folder does not contain a completed Switchloom run.'); }
}
export async function saveJson(path: string, data: unknown) { await writeFile(path, `${JSON.stringify(data, null, 2)}\n`, { mode: 0o600 }); }
