import { createFileRoute, notFound } from '@tanstack/react-router';
import BenchmarkDetail from '../components/BenchmarkDetail';
import { benchmarks } from '../lib/benchmarks';
import '../styles/benchmarks.css';

export const Route = createFileRoute('/benchmarks/$slug')({
  loader: ({ params }) => {
    const benchmark = benchmarks.find(entry => entry.slug === params.slug);
    if (!benchmark) throw notFound();
    return benchmark;
  },
  head: ({ loaderData }) => ({ meta: [
    { title: `${loaderData?.label ?? 'Benchmark'} | Switchloom` },
    { name: 'description', content: loaderData?.description ?? 'Archived Switchloom benchmark.' },
  ] }),
  component: BenchmarkPage,
});

function BenchmarkPage() {
  const benchmark = Route.useLoaderData();
  return <BenchmarkDetail key={benchmark.slug} benchmark={benchmark} />;
}
