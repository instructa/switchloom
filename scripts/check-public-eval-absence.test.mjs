import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const checker = path.resolve("scripts/check-public-eval-absence.mjs");
const fixture = async (t) => { const root = await mkdtemp(path.join(os.tmpdir(), "public-eval-absence-")); t.after(() => rm(root, { recursive: true, force: true })); await mkdir(path.join(root, "evidence/codex/0.145.0"), { recursive: true }); await writeFile(path.join(root, "evidence/codex/0.145.0/runtime-evidence.json"), "{}\n"); await writeFile(path.join(root, "evidence/codex/0.145.0/exact-version-capture.txt"), "fixture\n"); return root; };
const check = (root) => spawnSync(process.execPath, [checker], { cwd: root, encoding: "utf8" });

test("absence gate accepts only the retained runtime fixtures", async (t) => { const root = await fixture(t), result = check(root); assert.equal(result.status, 0, result.stderr); assert.match(result.stdout, /external-owner paths absent/); });
for (const candidate of ["docs/release-eval-private-operations.md", "scripts/generate-release-eval-attestation.mjs", "scripts/run-routing-pilot-codex.test.mjs", "schemas/release-eval/v1/attestation.schema.json", "fixtures/release-routing-suite-v1/suite.json"]) test(`absence gate rejects ${candidate}`, async (t) => { const root = await fixture(t), target = path.join(root, candidate); await mkdir(path.dirname(target), { recursive: true }); await writeFile(target, "reintroduced\n"); const result = check(root); assert.notEqual(result.status, 0); assert.match(result.stderr, /public maintainer-eval path remains/); assert.match(result.stderr, new RegExp(candidate.startsWith("schemas/") || candidate.startsWith("fixtures/") ? candidate.split("/").slice(0, 2).join("/") : candidate)); });
