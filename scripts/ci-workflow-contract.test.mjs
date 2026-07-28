import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

const read = (file) => readFile(file, "utf8");
const ci = await read(".github/workflows/ci.yml");
const candidate = await read(".github/workflows/release-candidate.yml");
const packageJson = JSON.parse(await read("package.json"));

test("CI classifies paths and conditionally runs every broad job", () => {
  assert.match(ci, /classify:\n    name: Classify changes/);
  assert.match(ci, /node scripts\/ci-router\.mjs --github-output "\$GITHUB_OUTPUT"/);
  for (const job of ["rust", "website", "distribution"]) {
    assert.match(ci, new RegExp(`${job}:\\n[\\s\\S]*?needs: classify[\\s\\S]*?needs\\.classify\\.outputs\\.${job} == 'true'`));
  }
  assert.match(ci, /parity: \$\{\{ steps\.router\.outputs\.parity \}\}/);
});

test("website fast checks exclude CLI parity and routing-sensitive changes add it explicitly", () => {
  assert.match(packageJson.scripts["site:test"], /--exclude website\/src\/lib\/website-cli-parity\.test\.ts/);
  assert.match(packageJson.scripts["site:test:parity"], /^vitest run website\/src\/lib\/website-cli-parity\.test\.ts --hookTimeout=120000$/);
  assert.match(ci, /Install Rust for CLI parity\n        if: \$\{\{ needs\.classify\.outputs\.parity == 'true' \}\}/);
  assert.match(ci, /Verify website and CLI parity\n        if: \$\{\{ needs\.classify\.outputs\.parity == 'true' \}\}\n        run: pnpm site:test:parity/);
});

test("CI keeps a stable fail-closed summary and cancels superseded feature runs", () => {
  assert.match(ci, /group: ci-\$\{\{ github\.workflow \}\}-\$\{\{ github\.event\.pull_request\.number \|\| github\.ref \}\}/);
  assert.match(ci, /cancel-in-progress: true/);
  assert.match(ci, /summary:\n    name: CI\n    needs: \[classify, rust, website, distribution\]\n    if: \$\{\{ always\(\) \}\}/);
  assert.match(ci, /test "\$\{\{ needs\.classify\.result \}\}" = "success"/);
  assert.match(ci, /false:skipped/);
});

test("release candidates are manual-only and automated secret scan is absent", async () => {
  assert.match(candidate, /^on:\n  workflow_dispatch:/m);
  assert.doesNotMatch(candidate, /^  push:/m);
  assert.doesNotMatch(candidate, /^  pull_request:/m);
  await assert.rejects(access(".github/workflows/secret-scan.yml"));
});
