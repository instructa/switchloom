import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

describe("website security headers", () => {
  it("permits Astro's inline island bootstrap and the Plausible endpoint", async () => {
    const headers = await readFile(resolve("website/public/_headers"), "utf8");

    expect(headers).toContain("script-src 'self' 'unsafe-inline' https://analytics.int.macherjek.com");
    expect(headers).toContain("connect-src 'self' https://analytics.int.macherjek.com");
    expect(headers).toContain("font-src 'self' data:");
    expect(headers).toContain("object-src 'none'");
    expect(headers).toContain("frame-ancestors 'none'");
  });
});
