import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["website/src/**/*.test.ts"],
  },
});
