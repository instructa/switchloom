import { assertAlchemyRuntime } from "./scripts/check-alchemy-runtime.mjs";

assertAlchemyRuntime();

const [{ default: alchemy }, { Website }] = await Promise.all([
  import("alchemy"),
  import("alchemy/cloudflare"),
]);

const app = await alchemy("model-routing");

export const presetCatalog = await Website("preset-catalog", {
  name: `model-routing-${app.stage}-catalog`,
  domains: app.stage === "prod" ? ["switchloom.ai"] : [],
  assets: "./dist/website",
  build: {
    command: "pnpm site:build",
    memoize: false,
  },
  dev: "pnpm site:serve",
  spa: false,
});

console.log({ url: presetCatalog.url });

await app.finalize();
