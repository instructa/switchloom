import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import { buildSite } from "../../../scripts/build-site.mjs";

const roots: string[] = [];
afterEach(async () => Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true }))));

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "switchloom-site-"));
  roots.push(root);
  const sourceRoot = join(root, "website", "data");
  const outputRoot = join(root, "dist", "website");
  await mkdir(join(sourceRoot, "bundles"), { recursive: true });
  await mkdir(join(outputRoot, "_astro"), { recursive: true });
  await writeFile(join(sourceRoot, "catalog.json"), JSON.stringify({ compositions: [{ id: "one" }] }));
  await writeFile(join(sourceRoot, "bundles", "one.json"), "bundle");
  await writeFile(join(outputRoot, "index.html"), "astro");
  await writeFile(join(outputRoot, "_headers"), "headers");
  await writeFile(join(outputRoot, "_astro", "client.js"), "client");
  return { sourceRoot, outputRoot };
}

describe("Astro publication", () => {
  it("adds canonical catalog data without replacing the Astro build", async () => {
    const paths = await fixture();
    const files = await buildSite(paths);
    expect(files).toContain("index.html");
    expect(await readFile(join(paths.outputRoot, "data", "bundles", "one.json"), "utf8")).toBe("bundle");
  });

  it("rejects an empty canonical catalog before touching output data", async () => {
    const paths = await fixture();
    await writeFile(join(paths.sourceRoot, "catalog.json"), JSON.stringify({ compositions: [] }));
    await expect(buildSite(paths)).rejects.toThrow(/no compositions/);
  });

  it("refuses to publish inside the canonical data directory", async () => {
    const paths = await fixture();
    await expect(buildSite({ sourceRoot: paths.sourceRoot, outputRoot: join(paths.sourceRoot, "dist") }))
      .rejects.toThrow(/must be separate/);
  });
});
