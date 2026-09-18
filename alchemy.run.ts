import alchemy from 'alchemy';
import { TanStackStart, DurableObjectNamespace } from 'alchemy/cloudflare';
if (process.env.DEVELOPER_MODE === 'true') throw new Error('Developer mode cannot be deployed.');
const app = await alchemy('model-routing');
export const site = await TanStackStart('preset-catalog', {
  name: `model-routing-${app.stage}-catalog`,
  compatibilityDate: '2026-09-17',
  domains: app.stage === 'prod' ? ['switchloom.ai'] : [],
  bindings: {
    TYPESAFE_API_KEY: alchemy.secret.env('TYPESAFE_API_KEY'),
    DEVELOPER_MODE: 'false',
    QUOTA: DurableObjectNamespace('playground-quota', { className: 'Quota', sqlite: true }),
  },
  wrangler: { main: 'website/server/worker.ts', secrets: false },
  build: { command: 'pnpm site:build', memoize: false },
  dev: 'pnpm site:serve',
});
console.log({ url: site.url });
await app.finalize();
