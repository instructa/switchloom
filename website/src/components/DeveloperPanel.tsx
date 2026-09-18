import { useEffect, useState } from 'react';
import catalog from '../../data/catalog.json';
import { defaultOptions, type WorkflowOptions } from '../lib/generator';
import { profilesFor, type Decision } from '../lib/playground';
import { RoutingGraph, type PlaygroundConfig } from './Playground';
type Summary = { id: string; mode: string; status: string; dir: string; project: string; turns: number; subagentTurns?: number; jevCalls: number; elapsedMs: number; final: string; usage: Record<string, unknown>; usageCoverage?: { complete: boolean; missingThreadIds: string[] }; checks: { command: string; exitCode: number | null }[]; approvals: { id: string | number; method: string; params: unknown }[]; active?: { slot: string }; screenshot?: boolean; imported?: boolean; config: { baseline: { model: string; effort: string }; profiles: ReturnType<typeof profilesFor> } };
type Event = { sequence: number; time: string; type: string; data: any };
function runOptions(run: Summary): WorkflowOptions {
  const options = defaultOptions(catalog);
  const profiles = run.mode === 'vanilla' ? { sol: { ...run.config.baseline, capabilities: ['implementation'] } } : run.config.profiles;
  options.jev = run.mode === 'jev';
  for (const slot of catalog.slots) options.models[slot.id] = { ...(profiles[slot.id] ?? options.models[slot.id]), enabled: Boolean(profiles[slot.id]) };
  for (const cap of catalog.capabilities) options.owners[cap.id] = Object.keys(profiles).find(s => profiles[s].capabilities.includes(cap.id)) ?? null;
  return options;
}
export default function DeveloperPanel({ config, options, task }: { config: PlaygroundConfig; options: WorkflowOptions; task: string }) {
  const [account, setAccount] = useState<string | null>(null); const [error, setError] = useState('');
  const [key, setKey] = useState(''); const [keyType, setKeyType] = useState('typesafe'); const [authUrl, setAuthUrl] = useState('');
  const [typeSafeReady, setTypeSafeReady] = useState(config.typesafeConfigured);
  const [mode, setMode] = useState('vanilla'); const [baseline, setBaseline] = useState('sol');
  const [root, setRoot] = useState(''); const [folder, setFolder] = useState<{ path: string; parent: string; folders: string[] } | null>(null);
  const [benchmarkTask, setBenchmarkTask] = useState(''); const [referenceImage, setReferenceImage] = useState(''); const [maxMinutes, setMinutes] = useState(30); const [maxTurns, setTurns] = useState(24);
  const [busy, setBusy] = useState(false); const [runs, setRuns] = useState<Summary[]>([]); const [selected, setSelected] = useState<string>('');
  const [events, setEvents] = useState<Event[]>([]); const [liveDecision, setLiveDecision] = useState<Decision>(); const [activeSlot, setActiveSlot] = useState<string>();
  const [previewUrl, setPreviewUrl] = useState('http://127.0.0.1:5174'); const [capture, setCapture] = useState('');
  const [previewOutput, setPreviewOutput] = useState(''); const [importPath, setImportPath] = useState('');
  const run = runs.find(r => r.id === selected); const running = runs.some(r => r.status === 'running');
  async function api(path: string, body?: unknown): Promise<any> {
    const response = await fetch(`/api/local/${path}`, body === undefined ? {} : { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Switchloom-CSRF': config.csrf! }, body: JSON.stringify(body) });
    const data: any = await response.json(); if (!response.ok) throw new Error(data.error ?? 'Local request failed.'); return data;
  }
  async function action(work: () => Promise<void>) { setError(''); setBusy(true); try { await work(); } catch (e) { setError(e instanceof Error ? e.message : 'Request failed.'); } finally { setBusy(false); } }
  async function refreshAccount() { const data = await api('account'); setAccount(data.authMode); setTypeSafeReady(data.typesafeConfigured); }
  useEffect(() => {
    let alive = true;
    fetch('/api/local/account').then(r => { if (!r.ok) throw new Error(); return r.json() as Promise<any>; }).then(data => { if (alive) { setAccount(data.authMode); setTypeSafeReady(data.typesafeConfigured); } }).catch(() => { if (alive) setError('Codex is unavailable. Install the Codex CLI first.'); });
    fetch('/api/local/run').then(r => { if (!r.ok) throw new Error(); return r.json() as Promise<any>; }).then(data => { if (alive && data.runs) { setRuns(data.runs); setSelected(data.run?.id ?? ''); } }).catch(() => { if (alive) setError('Could not load local runs. Reload to try again.'); });
    const source = new EventSource('/api/local/events');
    source.onopen = () => { setEvents([]); setError(''); };
    source.onmessage = message => {
      const event: Event = JSON.parse(message.data);
      setEvents(current => event.type === 'run/started' ? [event] : [...current, event].slice(-300));
      if (event.type === 'routing/decision') setLiveDecision(event.data);
      if (event.type === 'turn/requested') { setActiveSlot(event.data.slot === 'baseline' ? 'sol' : event.data.slot); if (event.data.controller) setLiveDecision(undefined); }
      if (event.type === 'run/progress') setRuns(current => current.map(r => r.id === event.data.id ? { ...r, ...event.data } : r));
      if (event.type === 'thread/tokenUsage/updated') setRuns(current => current.map(r => r.status === 'running' ? { ...r, usage: { ...r.usage, [event.data.threadId]: event.data.tokenUsage } } : r));
      if (event.type === 'run/started' || event.type === 'run/finished') {
        setRuns(current => [...current.filter(r => r.id !== event.data.id), event.data]); setSelected(event.data.id);
        if (event.type === 'run/finished') setActiveSlot(undefined);
      }
      if (event.type === 'approval/requested') setRuns(current => current.map(r => r.status === 'running' ? { ...r, approvals: [...r.approvals, event.data] } : r));
      if (event.type === 'approval/responded') setRuns(current => current.map(r => ({ ...r, approvals: r.approvals.filter(a => a.id !== event.data.id) })));
    };
    source.onerror = () => { if (alive) setError('Live connection interrupted. Events resume after reconnect.'); };
    return () => { alive = false; source.close(); };
  }, []);
  async function start() {
    const created = await api('run', { mode, task: benchmarkTask || task, root, referenceImage: referenceImage.trim() || undefined, profiles: profilesFor(catalog, options), baseline: { model: options.models[baseline].model, effort: options.models[baseline].effort }, maxMinutes, maxTurns });
    setRuns(current => [...current.filter(r => r.id !== created.id), created]); setSelected(created.id); setCapture(''); setLiveDecision(undefined);
  }
  return <section className="panel developer-panel" aria-label="Local benchmark">
    <div className="panel-heading"><h2>Benchmark</h2><span>{account === 'chatgpt' ? 'Codex subscription' : account === 'apiKey' ? 'OpenAI API' : 'Not connected'}</span></div>
    <div className="developer-body">
      <details><summary>Connections</summary><div className="dev-actions"><span className="eyebrow">{typeSafeReady ? 'Jev connected' : 'Jev key missing'}</span><button className="secondary-button" disabled={busy || running} onClick={() => action(async () => { const data = await api('login', { type: 'chatgpt' }); if (data.authUrl && new URL(data.authUrl).protocol === 'https:') setAuthUrl(data.authUrl); })}>Connect Codex</button><button className="text-button" onClick={() => action(refreshAccount)}>Refresh login</button>{authUrl && <a href={authUrl} target="_blank" rel="noreferrer">Finish login ↗</a>}</div>
        <p className="muted" style={{ fontSize: 11 }}>New logins stay in this local process.</p>
        <div className="dev-grid"><label className="field">Credential<select value={keyType} onChange={e => { setKeyType(e.target.value); setKey(''); }}><option value="typesafe">TypeSafe · local session</option><option value="apiKey">OpenAI · local session</option></select></label><label className="field">API key<input type="password" autoComplete="off" value={key} onChange={e => setKey(e.target.value)} /></label></div><div className="dev-actions"><button className="secondary-button" disabled={!key || busy || running} onClick={() => action(async () => { const credential = key; setKey(''); await api('login', { type: keyType, key: credential }); await refreshAccount(); })}>Use key</button></div>
      </details>
      <div className="dev-grid"><label className="field">Mode<select value={mode} disabled={running} onChange={e => setMode(e.target.value)}><option value="vanilla">Vanilla · single model</option><option value="subagents">Coordinator + subagents</option><option value="routing">Routing · without Jev</option><option value="jev">Routing + Jev</option></select></label><label className="field">{mode === 'subagents' ? 'Coordinator model' : 'Baseline model'}<select value={baseline} disabled={running || !['vanilla', 'subagents'].includes(mode)} onChange={e => setBaseline(e.target.value)}>{catalog.slots.map(s => <option key={s.id} value={s.id}>{options.models[s.id].model} · {options.models[s.id].effort}</option>)}</select></label>
        <label className="field">Minutes<input type="number" min="1" max="120" value={maxMinutes} disabled={running} onChange={e => setMinutes(Number(e.target.value))} /></label><label className="field">Max turns<input type="number" min="1" max="100" value={maxTurns} disabled={running} onChange={e => setTurns(Number(e.target.value))} /></label></div>
      <div className="dev-actions"><button className="secondary-button" disabled={busy || running} onClick={() => action(async () => setFolder(await api('folders')))}>Choose output folder</button><span className="folder-path">{root || 'No folder selected'}</span></div>
      {folder && <div className="folder-dialog" role="group" aria-label="Choose output folder"><div className="folder-path">{folder.path}</div><div className="folder-list"><button className="secondary-button" onClick={() => action(async () => setFolder(await api(`folders?path=${encodeURIComponent(folder.parent)}`)))}>↑ Parent</button>{folder.folders.map(name => <button className="secondary-button" key={name} onClick={() => action(async () => setFolder(await api(`folders?path=${encodeURIComponent(`${folder.path}/${name}`)}`)))}>{name}/</button>)}</div><div className="dev-actions"><button className="primary-button" onClick={() => { setRoot(folder.path); setFolder(null); }}>Use this folder</button><button className="text-button" onClick={() => setFolder(null)}>Cancel</button></div></div>}
      <label className="field">Benchmark task<textarea rows={4} maxLength={32000} placeholder="Uses the Playground prompt when empty." value={benchmarkTask} disabled={running} onChange={e => setBenchmarkTask(e.target.value)} /></label>
      <label className="field">Reference PNG<input placeholder="/absolute/path/reference.png" value={referenceImage} disabled={running} onChange={e => setReferenceImage(e.target.value)} /></label>
      <div className="dev-actions"><label className="secondary-button">Import task<input type="file" accept=".txt,.md" style={{ display: 'none' }} onChange={e => { const file = e.target.files?.[0]; if (file) void action(async () => { if (file.size > 32000) throw new Error('Task file exceeds 32 KB.'); setBenchmarkTask(await file.text()); }); }} /></label><span className="muted" style={{ fontSize: 10 }}>Each run creates a fresh project directory.</span></div>
      <div className="dev-actions"><button className="primary-button" disabled={busy || running || !root || !account || (mode === 'jev' && !typeSafeReady)} onClick={() => action(start)}>Run benchmark ↗</button>{running && <button className="secondary-button" disabled={busy} onClick={() => action(async () => { await api('stop', {}); })}>Stop</button>}</div>
      {error && <p role="alert" className="error-message">{error}</p>}
      <details><summary>Open saved run</summary><div className="dev-actions"><label className="field">Run directory<input value={importPath} onChange={e => setImportPath(e.target.value)} placeholder="/path/to/switchloom-…" /></label><button className="secondary-button" disabled={busy || !importPath} onClick={() => action(async () => { const saved = await api('import', { path: importPath }); setRuns(r => [...r, { ...saved, imported: true }]); setSelected(saved.id); })}>Open results</button></div></details>
      {runs.length > 0 && <label className="field">Results<select value={selected} onChange={e => { setSelected(e.target.value); setCapture(''); }}>{runs.map(r => <option key={r.id} value={r.id}>{r.mode} · {r.status} · {r.id}</option>)}</select></label>}
      {runs.length > 1 && <div className="comparison-wrap"><table className="comparison"><thead><tr><th>Run</th><th>Status</th><th>Time</th><th>Turns</th><th>Jev calls</th><th>Failed commands</th></tr></thead><tbody>{runs.map(r => <tr key={r.id}><td><button className="text-button" onClick={() => { setSelected(r.id); setCapture(''); }}>{r.mode}</button></td><td>{r.status}</td><td>{Math.round(r.elapsedMs / 1000)}s</td><td>{r.turns}</td><td>{r.jevCalls}</td><td>{r.checks.filter(c => c.exitCode !== null && c.exitCode !== 0).length}</td></tr>)}</tbody></table></div>}
      {run && <><div className="run-stats"><strong>{run.status}</strong><span>{run.turns} turns</span>{Boolean(run.subagentTurns) && <span>{run.subagentTurns} subagent turns</span>}<span>{run.jevCalls} Jev calls</span><span>{Math.round(run.elapsedMs / 1000)} s</span></div><div className="folder-path">{run.project}</div>
        {run.status === 'running' && <RoutingGraph options={runOptions(run)} decision={liveDecision} activeSlot={activeSlot} />}
        {run.approvals.map(a => <div className="approval" key={a.id}><strong>Codex needs approval</strong><pre>{JSON.stringify(a.params, null, 2)}</pre><div className="dev-actions"><button className="primary-button" onClick={() => action(async () => { await api('approval', { id: a.id, accept: true }); })}>Approve once</button><button className="secondary-button" onClick={() => action(async () => { await api('approval', { id: a.id, accept: false }); })}>Decline</button></div></div>)}
        {run.final && <div className="run-result">{run.final}</div>}
        <div className="dev-actions">{(run.status === 'running' ? ['config.json', 'events.jsonl'] : ['config.json', 'summary.json', 'events.jsonl', 'checks.json', 'diff.patch']).map(name => <a className="text-button" key={name} href={`/api/local/artifact?run=${encodeURIComponent(run.id)}&name=${name}`} target="_blank" rel="noreferrer">{name}</a>)}</div>
        <details><summary>Token usage</summary>{run.usageCoverage?.complete === false && <p className="muted">Usage is incomplete for {run.usageCoverage.missingThreadIds.length} task(s).</p>}<pre className="run-result">{JSON.stringify(run.usage, null, 2)}</pre></details>
        <div className="dev-actions"><label className="field">Local preview URL<input value={previewUrl} onChange={e => setPreviewUrl(e.target.value)} /></label><button className="secondary-button" disabled={busy || running || run.imported} onClick={() => action(async () => { const result = await api('preview', { id: run.id, port: Number(new URL(previewUrl).port) }); setPreviewUrl(result.url); setPreviewOutput(result.output); })}>Start preview</button><button className="text-button" onClick={() => action(async () => { await api('preview', { stop: true }); })}>Stop preview</button><button className="text-button" onClick={() => action(async () => { const p = await api('preview'); setPreviewOutput(p.output); })}>Preview log</button><button className="secondary-button" disabled={busy || run.imported} onClick={() => action(async () => { await api('screenshot', { id: run.id, url: previewUrl }); setRuns(r => r.map(item => item.id === run.id ? { ...item, screenshot: true } : item)); setCapture(`/api/local/artifact?run=${encodeURIComponent(run.id)}&name=screenshot.png&t=${Date.now()}`); })}>Capture screenshot</button><a href={/^http:\/\/(127\.0\.0\.1|localhost):\d+\/?/.test(previewUrl) ? previewUrl : undefined} target="_blank" rel="noreferrer">Open preview ↗</a></div>
        {previewOutput && <pre className="run-events">{previewOutput}</pre>}
        {(capture || run.screenshot) && <img className="screenshot-preview" src={capture || `/api/local/artifact?run=${encodeURIComponent(run.id)}&name=screenshot.png`} alt="Benchmark project screenshot" width="1440" height="1000" />}
      </>}
      {events.length > 0 && <details open><summary>Live events</summary><div className="run-events" role="log" aria-label="Benchmark events">{events.map(e => <details key={`${e.time}-${e.sequence}`}><summary>{new Date(e.time).toLocaleTimeString()} · {e.type}</summary><pre>{JSON.stringify(e.data, null, 2)}</pre></details>)}</div></details>}
    </div>
  </section>;
}
