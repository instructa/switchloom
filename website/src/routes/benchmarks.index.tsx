import { createFileRoute, Link } from '@tanstack/react-router';
import { archiveDate, benchmarkAsset, benchmarkCost, benchmarks, dollars } from '../lib/benchmarks';
import '../styles/benchmarks.css';

export const Route = createFileRoute('/benchmarks/')({
  head: () => ({ meta: [
    { title: 'Benchmarks | Switchloom' },
    { name: 'description', content: 'Real runs with Astra, Luna, Sol and Jev. Costs, screenshots and the parts that did not work.' },
  ] }),
  component: Benchmarks,
});

function Benchmarks() {
  return <main className="shell benchmarks-page">
    <div className="benchmark-eyebrow">Field notes / Benchmarks</div>
    <h1>Does the routing actually help?</h1>
    <div className="benchmark-prose benchmark-intro">
      <p>I built Switchloom to find out whether mixing models could improve my workflow. Then I ran the comparisons. So far, the extra routing has not earned its place.</p>
      <p>These are the runs, including the unfinished ones. Screenshots, costs and what I would actually use again.</p>
    </div>
    <section className="benchmark-archive" aria-label="Benchmark archive">
      {benchmarks.map(benchmark => <Link to="/benchmarks/$slug" params={{ slug: benchmark.slug }} className="benchmark-entry" key={benchmark.slug}>
        <div className="benchmark-entry-image">
          <img src={benchmarkAsset(benchmark.slug, 'vanilla-single-model', 'desktop')} width={1440} height={900} alt={`Astra solo result from ${benchmark.label}`} loading="lazy" />
        </div>
        <div className="benchmark-entry-copy">
          <div className="benchmark-eyebrow"><span>Archived</span><time dateTime={benchmark.date}>{archiveDate(benchmark.date)}</time></div>
          <h2>{benchmark.label}</h2>
          <p className="benchmark-entry-title">{benchmark.headline}</p>
          <p>{benchmark.description}</p>
          <div className="benchmark-entry-meta"><span>4 strategies · {dollars(benchmarkCost(benchmark))} estimated</span><span aria-hidden="true">↗</span></div>
        </div>
      </Link>)}
    </section>
    <p className="benchmark-footnote">Two pilots on the same task. Useful observations, still a small sample. A cheaper run only matters if it finishes the work you needed.</p>
  </main>;
}
