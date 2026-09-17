import react from "@astrojs/react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

export default defineConfig({
  srcDir: "./website/src",
  integrations: [react()],
  outDir: "./dist/website",
  publicDir: "./website/public",
  vite: {
    plugins: [
      tailwindcss(),
      {
        name: "separate-mode-cache",
        // Prerendering must not replace the running dev server's React cache.
        config: (_config, { mode }) => ({
          cacheDir: `./node_modules/.vite/astro-${mode}`,
        }),
      },
    ],
  },
});
