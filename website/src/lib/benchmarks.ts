import archive from '../../data/benchmarks.json';

// Historical measurements stay independent of the editable model catalog.
export const benchmarks = archive;
export type Benchmark = (typeof benchmarks)[number];
export type BenchmarkRun = Benchmark['runs'][number];

export const archiveDate = (date: string) => new Date(date).toLocaleDateString('en-GB', {
  day: 'numeric', month: 'short', year: 'numeric', timeZone: 'UTC',
});
export const benchmarkAsset = (slug: string, run: string, view: 'desktop' | 'mobile') =>
  `/benchmarks/${slug}/${run}/${view}.png`;
export const dollars = (value: number) => `$${value.toFixed(2)}`;
export const minutes = (seconds: number) => `${(seconds / 60).toFixed(1)} min`;
export const runCost = (run: BenchmarkRun) => run.openaiUsd + run.jevUsd;
export const benchmarkCost = (benchmark: Benchmark) => benchmark.runs.reduce((sum, run) => sum + runCost(run), 0);
export const checkCounts = (run: BenchmarkRun) => ({
  passed: run.checks.filter(check => check.pass === true).length,
  failed: run.checks.filter(check => check.pass === false).length,
  unscored: run.checks.filter(check => check.pass === null).length,
});
