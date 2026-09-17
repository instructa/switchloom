import alchemy from "alchemy";
import { Website } from "alchemy/cloudflare";

const app = await alchemy("model-routing");

export const site = await Website("preset-catalog", {
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

console.log({ url: site.url });

await app.finalize();
