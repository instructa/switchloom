#!/usr/bin/env node
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

process.chdir(new URL("../", import.meta.url).pathname);
const report = JSON.parse(readFileSync("retained-evidence/evidence-validator-parity/report.json", "utf8"));
assert.equal(report.schema_version, 1);
assert.equal(report.cases.length, 24, "parity matrix must name exactly 24 cases");
assert.equal(new Set(report.cases.map(({ name }) => name)).size, 24, "parity case names must be unique");
assert.deepEqual(report.results.legacy, { harness_exit: 0, passed: 24, failed: 0 });
assert.deepEqual(report.results.rust, { harness_exit: 0, passed: 24, failed: 0 });
assert.equal(report.results.semantic_agreement, "24/24");

for (const artifact of [...report.implementations, ...report.corpus]) {
  const digest = createHash("sha256").update(readFileSync(artifact.path)).digest("hex");
  assert.equal(digest, artifact.sha256, `digest changed for ${artifact.path}`);
}

const tests = report.corpus.map(({ path }) => path);
for (const mode of ["legacy", "rust"]) {
  const result = spawnSync(process.execPath, ["--test", ...tests], {
    cwd: process.cwd(),
    encoding: "utf8",
    env: { ...process.env, VALIDATOR_MODE: mode },
  });
  if (result.status !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
  }
  assert.equal(result.status, 0, `${mode} validator corpus failed`);
  assert.match(result.stdout, /tests 24/);
  assert.match(result.stdout, /pass 24/);
  assert.match(result.stdout, /fail 0/);
}

console.log("evidence validator differential parity passed: 24/24 named cases, identical corpus generators, pinned implementation digests");
