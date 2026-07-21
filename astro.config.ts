import react from "@astrojs/react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

export default defineConfig({
  srcDir: "./website/src",
  integrations: [react()],
  outDir: "./dist/website",
  publicDir: "./website/public",
  vite: {
    plugins: [tailwindcss()],
  },
});
