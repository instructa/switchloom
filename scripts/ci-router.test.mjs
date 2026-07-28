import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { classifyPaths } from "./ci-router.mjs";

const selected = (paths) => {
  const { reason, ...jobs } = classifyPaths(paths);
  return jobs;
};

test("documentation-only paths select no broad CI jobs", () => {
  assert.deepEqual(selected(["README.md", "docs/verification.md"]), { rust: false, website: false, distribution: false });
});

test("website paths select only the website job", () => {
  assert.deepEqual(selected(["website/src/pages/index.astro"]), { rust: false, website: true, distribution: false });
  assert.deepEqual(selected(["website/content/changelog.md"]), { rust: false, website: true, distribution: false });
});

test("Rust paths select Rust and distribution", () => {
  assert.deepEqual(selected(["src/lib.rs", "crates/policy/src/lib.rs"]), { rust: true, website: false, distribution: true });
});

test("npm paths select only distribution", () => {
  assert.deepEqual(selected(["npm/bin/model-routing.js", "package.json"]), { rust: false, website: false, distribution: true });
});

test("workflow and unknown paths fail closed to all jobs", () => {
  assert.deepEqual(selected([".github/workflows/ci.yml"]), { rust: true, website: true, distribution: true });
  assert.deepEqual(selected(["pnpm-lock.yaml"]), { rust: true, website: true, distribution: true });
});

test("the CLI writes GitHub Actions outputs and prints its JSON result", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "ci-router-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const output = path.join(root, "github-output");
  const result = spawnSync(process.execPath, ["scripts/ci-router.mjs", "--github-output", output, "src/lib.rs"], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), { rust: true, website: false, distribution: true, reason: "paths" });
  assert.equal(await readFile(output, "utf8"), "rust=true\nwebsite=false\ndistribution=true\nreason=paths\n");
});
