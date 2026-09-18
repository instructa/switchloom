import { lazy, Suspense, useEffect, useState, useRef } from 'react';
import { Link } from '@tanstack/react-router';
import catalog from '../../data/catalog.json';
import Generator from './Generator';
import { useWorkflow } from '../lib/workflow-context';
import { modelLabel, workflowError, type WorkflowOptions } from '../lib/generator';
import { decisionLabel, profilesFor, PROMPT_LIMIT, type RouteResult, type Decision } from '../lib/playground';
import '../styles/playground.css';
const DeveloperPanel = import.meta.env.MODE === 'developer' ? lazy(() => import('./DeveloperPanel')) : null;
export type PlaygroundConfig = { developerMode: boolean; csrf?: string; dailyLimit: number | null; remaining?: number | null; typesafeConfigured?: boolean };
export function RoutingGraph({ options, decision, busy = false, activeSlot }: { options: WorkflowOptions; decision?: Decision; busy?: boolean; activeSlot?: string }) {
  const profiles = profilesFor(catalog, options);
  const capabilities = catalog.capabilities.filter(c => options.owners[c.id] && c.routable);
  const owner = activeSlot ?? decision?.assignment?.owner;
  const selected = decision?.assignment?.capability;
  const graph = useRef<HTMLDivElement>(null);
  const [paths, setPaths] = useState<{ d: string; active: boolean }[]>([]);
  useEffect(() => {
    const element = graph.current; if (!element) return;
    const measure = () => {
      const bounds = element.getBoundingClientRect();
      const point = (el: Element, edge: 'left' | 'right') => { const b = el.getBoundingClientRect(); return [b[edge] - bounds.left, b.top + b.height / 2 - bounds.top]; };
      const judge = element.querySelector('[data-judge]'); if (!judge) return;
      const connections: { d: string; active: boolean }[] = [];
      const curve = (from: number[], to: number[]) => `M${from[0]},${from[1]} C${(from[0]+to[0])/2},${from[1]} ${(from[0]+to[0])/2},${to[1]} ${to[0]},${to[1]}`;
      for (const cap of element.querySelectorAll('[data-capability]')) {
        const id = cap.getAttribute('data-capability')!;
        const target = element.querySelector(`[data-owner="${options.owners[id]}"]`);
        connections.push({ d: curve(point(judge, 'right'), point(cap, 'left')), active: selected === id });
        if (target) connections.push({ d: curve(point(cap, 'right'), point(target, 'left')), active: selected === id });
      }
      setPaths(connections);
    };
    const observer = new ResizeObserver(measure); observer.observe(element); measure();
    return () => observer.disconnect();
  }, [options, selected]);
  return <div ref={graph} className={`routing-graph${busy ? ' is-routing' : ''}`} aria-label="Routing graph" aria-busy={busy}>
    <svg className="graph-lines" aria-hidden="true">{paths.map((path, i) => <path key={i} d={path.d} className={path.active ? "selected-edge" : ""} />)}</svg>
    <div className="graph-stage"><span className="eyebrow">Input</span><div className="graph-node input-node">Prompt</div><div className="vertical-line" /><div data-judge className={`graph-node judge-node${decision?.judge_model ? ' active-node' : ''}`}><img src="/brand/jev.svg" alt="" width="18" height="26" />{options.jev ? 'Jev' : 'Direct'}</div></div>
    <div className="graph-connector" aria-hidden="true"></div>
    <div className="graph-stage capabilities-stage"><span className="eyebrow">Capability</span>{capabilities.map(cap => <div key={cap.id} data-capability={cap.id} className={`graph-node cap-node${selected === cap.id ? ' active-node' : ''}`}><span>{cap.label}</span>{decision?.probabilities[cap.id] !== undefined && <small>{Math.round(decision.probabilities[cap.id] * 100)}%</small>}</div>)}</div>
    <div className="graph-connector" aria-hidden="true"></div>
    <div className="graph-stage"><span className="eyebrow">Model</span>{Object.entries(profiles).map(([id, profile]) => <div key={id} data-owner={id} className={`graph-node target-node${owner === id ? ' active-node' : ''}`}><small>{id}</small><strong>{modelLabel(catalog, profile.model)}</strong><span>{profile.effort}</span></div>)}</div>
  </div>;
}
export default function Playground() {
  const { options, setOptions } = useWorkflow();
  const [config, setConfig] = useState<PlaygroundConfig | null>(null);
  const [task, setTask] = useState('Implement a searchable Pokédex with type filters and a detail view.');
  const [capability, setCapability] = useState('implementation');
  const [busy, setBusy] = useState(false); const [error, setError] = useState('');
  const [history, setHistory] = useState<{ task: string; result: RouteResult; fingerprint: string; time: string }[]>([]);
  const [remaining, setRemaining] = useState<number | null>(null);
  const fingerprint = JSON.stringify({ options, task, capability });
  const last = history[0]; const result = last?.fingerprint === fingerprint ? last.result : undefined;
  useEffect(() => {
    const controller = new AbortController();
    fetch('/api/config', { signal: controller.signal }).then(r => { if (!r.ok) throw new Error(); return r.json() as Promise<PlaygroundConfig>; }).then(data => { setConfig(data); setRemaining(data.remaining ?? null); }).catch(e => { if (e.name !== 'AbortError') setError('Could not connect to the Playground. Reload to try again.'); });
    return () => controller.abort();
  }, []);
  const configError = workflowError(catalog, options);
  const availableCapabilities = catalog.capabilities.filter(c => options.owners[c.id] && c.routable);
  const selectedCapability = availableCapabilities.some(c => c.id === capability) ? capability : availableCapabilities[0]?.id;
  async function route() {
    setBusy(true); setError('');
    try {
      const response = await fetch('/api/route', { method: 'POST', headers: { 'Content-Type': 'application/json', ...(config?.csrf ? { 'X-Switchloom-CSRF': config.csrf } : {}) },
        body: JSON.stringify({ request: { task, context: '', routing: options.jev ? { mode: 'jev' } : { mode: 'assigned', capability: selectedCapability } }, profiles: profilesFor(catalog, options) }),
      });
      const data = await response.json() as RouteResult & { error?: string };
      if (!response.ok) { if (data.remaining !== undefined) setRemaining(data.remaining); throw new Error(data.error); }
      if (data.remaining !== null) setRemaining(data.remaining); setHistory(current => [{ task, result: data, fingerprint, time: new Date().toLocaleTimeString() }, ...current].slice(0, 10));
    } catch (e) { setError(e instanceof Error ? e.message : 'Routing failed.'); }
    finally { setBusy(false); }
  }
  return <main className="shell playground-shell">
    <div className="playground-heading"><div><span className="eyebrow">ROUTING LAB</span><h1>Playground.</h1></div><span className="mode-badge">{config?.developerMode ? 'Local developer mode' : 'Jev playground'}</span></div>
    <div className="playground-grid">
      <section className="panel input-panel" aria-label="Prompt input"><div className="panel-heading"><h2>Prompt</h2><span className="muted">{task.length.toLocaleString()} / {PROMPT_LIMIT.toLocaleString()}</span></div>
        <textarea aria-label="Routing prompt" maxLength={PROMPT_LIMIT} value={task} onChange={e => setTask(e.target.value)} placeholder="What needs to happen next?" />
        <div className="prompt-options">
          <label className="check-label"><input type="checkbox" role="switch" checked={options.jev} onChange={e => setOptions(o => ({ ...o, jev: e.target.checked }))} /> Jev</label></div>
        {!options.jev && <label className="field">Capability<select value={selectedCapability ?? ''} onChange={e => setCapability(e.target.value)}>{availableCapabilities.map(c => <option key={c.id} value={c.id}>{c.label}</option>)}</select></label>}
        <button className="primary-button" type="button" disabled={busy || !config || !task.trim() || Boolean(configError) || (!options.jev && !selectedCapability)} onClick={route}>{busy ? 'Routing…' : 'Route prompt'} <span aria-hidden="true">↗</span></button>
        <div className="input-footer"><span>{config?.dailyLimit ? `${remaining ?? '—'} / ${config.dailyLimit} remaining today` : 'Local session'}</span><span>{options.jev ? 'Sent to TypeSafe' : 'No API call'}</span></div>
        {(error || configError) && <p className="error-message" role="alert">{error || configError}</p>}
        <details className="configuration"><summary>Configuration <span>↗</span></summary><Generator workflow={catalog} compact options={options} onChange={setOptions} /></details>
      </section>
      <section className="panel output-panel" aria-label="Routing result"><div className="panel-heading"><h2>Route</h2><span className={`status-dot${result ? ' is-active' : ''}`}>{busy ? 'Judging' : result ? decisionLabel[result.decision.reason] : 'Ready'}</span></div>
        <RoutingGraph options={options} decision={result?.decision} busy={busy} />
        {result ? <><div className="result-strip"><span>{result.decision.assignment ? `${result.decision.assignment.capability} → ${result.decision.assignment.owner}` : decisionLabel[result.decision.reason]}</span><span>{result.elapsedMs} ms</span></div>
          {Object.keys(result.decision.probabilities).length > 0 && <div className="probability-list"><div className="panel-heading"><h3>Capability probabilities</h3><span>{result.decision.usage?.input_tokens.toLocaleString()} in · {result.decision.usage?.output_tokens.toLocaleString()} out</span></div>{Object.entries(result.decision.probabilities).sort((a, b) => b[1] - a[1]).map(([id, probability]) => <div className="probability-row" key={id}><span>{catalog.capabilities.find(c => c.id === id)?.label ?? id}</span><div className="probability-track"><div style={{ width: `${probability * 100}%` }} /></div><span>{(probability * 100).toFixed(1)}%</span></div>)}<div className="signal-line"><span>Capability confidence {((result.decision.confidence ?? 0) * 100).toFixed(0)}%</span>{result.decision.reason === 'jev_owner' && <span>Same owner {((result.decision.owner_probability ?? 0) * 100).toFixed(0)}%</span>}<span>Dispatch ≥ {((result.decision.act_threshold ?? 0) * 100).toFixed(0)}%</span><span>Missing context {((result.decision.context_missing ?? 0) * 100).toFixed(0)}%</span></div></div>}
        </> : <div className="empty-result">{busy ? 'Evaluating…' : last ? 'Configuration changed. Run again to see the new route.' : 'Your next step, mapped to its owner.'}</div>}
      </section>
    </div>
    {history.length > 0 && <section className="history panel"><div className="panel-heading"><h2>Recent routes</h2><button className="text-button" onClick={() => setHistory([])}>Clear</button></div>{history.map((entry, index) => <div className="history-row" key={`${entry.time}-${index}`}><time>{entry.time}</time><span>{entry.task}</span><strong>{entry.result.decision.assignment?.owner ?? 'No assignment'}</strong><span>{entry.result.elapsedMs} ms</span></div>)}</section>}
    {config?.developerMode && DeveloperPanel && <Suspense fallback={<p>Loading local runner…</p>}><DeveloperPanel config={config} options={options} task={task} /></Suspense>}
    <footer className="playground-footer"><Link to="/">← Workflow</Link><a href="https://docs.typesafe.ai/introduction" target="_blank" rel="noreferrer">Powered by Jev ↗</a></footer>
  </main>;
}
