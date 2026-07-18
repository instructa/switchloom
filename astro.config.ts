import react from "@astrojs/react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

export default defineConfig({
  integrations: [react()],
  outDir: "./dist/website",
  publicDir: "./website/public",
  vite: {
    plugins: [tailwindcss()],
  },
});
