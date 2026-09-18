import { Link } from '@tanstack/react-router';
import { useState } from 'react';
import { archiveDate, benchmarkAsset, benchmarkCost, benchmarks, checkCounts, dollars, minutes, runCost, type Benchmark, type BenchmarkRun } from '../lib/benchmarks';

export default function BenchmarkDetail({ benchmark }: { benchmark: Benchmark }) {
  const [view, setView] = useState<'desktop' | 'mobile'>('desktop');
  const jevCost = benchmark.runs.reduce((sum, run) => sum + run.jevUsd, 0);
  const jevCalls = benchmark.runs.reduce((sum, run) => sum + run.jevCalls, 0);
  const other = benchmarks.find(entry => entry.slug !== benchmark.slug);

  return <main className="shell benchmarks-page">
    <Link to="/benchmarks" className="benchmark-back">← All benchmarks</Link>
    <div className="benchmark-eyebrow"><span>Archived / {benchmark.label}</span><time dateTime={benchmark.date}>{archiveDate(benchmark.date)}</time></div>
    <h1>{benchmark.headline}</h1>
    <div className="benchmark-prose benchmark-intro">{benchmark.intro.map(paragraph => <p key={paragraph}>{paragraph}</p>)}</div>

    <section aria-labelledby="results-heading" className="benchmark-section">
      <div className="benchmark-section-heading"><h2 id="results-heading">The runs</h2><span>{dollars(benchmarkCost(benchmark))} total, estimated</span></div>
      <div className="benchmark-table-wrap" role="region" aria-label="Run results" tabIndex={0}>
        <table className="benchmark-table">
          <caption>One selected run per strategy. Costs include Jev. Elapsed time includes approval waits.</caption>
          <thead><tr><th scope="col">Strategy</th><th scope="col">Run status</th><th scope="col">Cost</th><th scope="col">Elapsed</th><th scope="col">Approval wait</th><th scope="col">Checks passed</th></tr></thead>
          <tbody>{benchmark.runs.map(run => <tr key={run.id}>
            <th scope="row"><a href={`#${run.id}`}>{run.label}</a>{run.repeated && <small>Replacement run</small>}</th>
            <td><span className={`benchmark-status ${run.status === 'Completed' ? 'is-completed' : 'is-incomplete'}`}>{run.status}</span></td>
            <td>{dollars(runCost(run))}</td><td>{minutes(run.elapsedSeconds)}</td><td>{minutes(run.approvalSeconds)}</td>
            <td><CheckSummary run={run} /></td>
          </tr>)}</tbody>
        </table>
      </div>
      <p className="benchmark-footnote">{jevCalls} Jev routing calls cost ${jevCost.toFixed(7)}. Totals cover the listed runs only. Prices are token-based estimates including cache reads and writes, not invoiced amounts.</p>
    </section>

    <section className="benchmark-section" aria-labelledby="screenshots-heading">
      <div className="benchmark-section-heading benchmark-gallery-heading">
        <h2 id="screenshots-heading">What came out</h2>
        <div className="benchmark-view" role="group" aria-label="Screenshot viewport">
          <button type="button" aria-pressed={view === 'desktop'} onClick={() => setView('desktop')}>Desktop</button>
          <button type="button" aria-pressed={view === 'mobile'} onClick={() => setView('mobile')}>Mobile</button>
        </div>
      </div>
      <details className="benchmark-reference"><summary>Original reference</summary><a href="/benchmarks/pokedex-reference.png" target="_blank" rel="noreferrer"><img src="/benchmarks/pokedex-reference.png" alt="Original red Pokédex reference supplied to every run" loading="lazy" /></a></details>
      <div className={`benchmark-gallery benchmark-gallery-${view}`}>
        {benchmark.runs.map(run => <ResultCard key={run.id} run={run} benchmark={benchmark} view={view} />)}
      </div>
      <p className="benchmark-footnote">Desktop 1440 × 900. Mobile 390 × 844. Original captures, no repairs to the apps. Mobile shows the first viewport. Visual assessment was not blinded.</p>
    </section>

    <section className="benchmark-section benchmark-prose" aria-labelledby="takeaway-heading">
      <h2 id="takeaway-heading">What I’m taking from this</h2>
      {benchmark.takeaway.map(paragraph => <p key={paragraph}>{paragraph}</p>)}
    </section>

    <section className="benchmark-section" aria-labelledby="setup-heading">
      <h2 id="setup-heading">How this was run</h2>
      <div className="benchmark-prose">
        <p>A React + TypeScript + Vite Pokédex with real API data, working controls and a supplied visual reference. Visual fidelity came first. The same brief went to every strategy.</p>
        <p>Codex CLI 0.154.0 through the app server, using API billing. Configured models were GPT-5.6 Luna max, GPT-5.6 Sol medium and GPT-6 Astra high. The cards list models that actually ran. The native subagent strategy could delegate between them. Persistent teams and Astra solo had subagents disabled.</p>
        <a className="benchmark-text-link" href="/benchmarks/pokedex-prompt.md">Read the full task ↗</a>
      </div>
      <ul className="benchmark-limitations">{benchmark.limits.map(limit => <li key={limit}>{limit}</li>)}</ul>
      <details className="benchmark-cache"><summary>Token and cache counts</summary>
        <div className="benchmark-table-wrap" role="region" aria-label="Token usage" tabIndex={0}><table className="benchmark-table">
          <caption>Cumulative tokens across each run’s tasks. Total input includes cache reads and writes.</caption>
          <thead><tr><th scope="col">Strategy</th><th scope="col">Input</th><th scope="col">Cache reads</th><th scope="col">Cache writes</th><th scope="col">Output</th></tr></thead>
          <tbody>{benchmark.runs.map(run => <tr key={run.id}><th scope="row">{run.label}</th>{(['inputTokens', 'cachedInputTokens', 'cacheWriteInputTokens', 'outputTokens'] as const).map(key => <td key={key}>{run.usage[key]?.toLocaleString('en-US') ?? 'Not recorded'}</td>)}</tr>)}</tbody>
        </table></div>
      </details>
      <p className="benchmark-footnote">Rates recorded on 18 Sep 2026. <a href="https://developers.openai.com/api/docs/pricing">OpenAI pricing ↗</a> · <a href="https://typesafe.ai/">TypeSafe pricing ↗</a>. Jev used the published $42 per billion input tokens rate.</p>
    </section>
    {other && <Link to="/benchmarks/$slug" params={{ slug: other.slug }} className="benchmark-next"><span>{other.label}</span><span>{other.headline} ↗</span></Link>}
  </main>;
}

function CheckSummary({ run }: { run: BenchmarkRun }) {
  const counts = checkCounts(run);
  return <><span>{counts.passed}/{run.checks.length}</span>{counts.failed > 0 && <small>{counts.failed} failed</small>}{counts.unscored > 0 && <small>{counts.unscored} unscored</small>}</>;
}

function ResultCard({ benchmark, run, view }: { benchmark: Benchmark; run: BenchmarkRun; view: 'desktop' | 'mobile' }) {
  const image = benchmarkAsset(benchmark.slug, run.id, view);
  return <article className="benchmark-result" id={run.id}>
    <header><div className="benchmark-result-title"><h3>{run.label}</h3><span>{dollars(runCost(run))}</span></div><p className="benchmark-models">{run.models.join(' / ')}</p><p className="benchmark-result-note">{run.note}</p></header>
    <a className="benchmark-screenshot" href={image} target="_blank" rel="noreferrer" aria-label={`Open ${run.label} ${view} screenshot at full size`}><img src={image} width={view === 'desktop' ? 1440 : 390} height={view === 'desktop' ? 900 : 844} alt={`${run.label} Pokédex at ${view === 'desktop' ? '1440 × 900' : '390 × 844'}`} loading="lazy" /></a>
    <details className="benchmark-checks"><summary>Functional checks <span><CheckSummary run={run} /></span></summary><ul>{run.checks.map(check => <li key={check.name}><span className="benchmark-check-status">{check.pass === null ? 'Unscored' : check.pass ? 'Pass' : 'Fail'}</span><div>{check.name}{check.note && <p>{check.note}</p>}</div></li>)}</ul></details>
  </article>;
}
