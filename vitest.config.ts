import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./website/src", import.meta.url)),
    },
  },
  test: {
    include: ["website/src/**/*.test.ts"],
  },
});
