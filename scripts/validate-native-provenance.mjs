#!/usr/bin/env node
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = resolve(new URL("..", import.meta.url).pathname);
const provenancePath = resolve(repoRoot, "npm/native/provenance.json");
const packagePath = resolve(repoRoot, "package.json");
const packageVersion = JSON.parse(readFileSync(packagePath, "utf8")).version;
const expectedTargets = new Set([
  "darwin-arm64",
  "darwin-x86_64",
  "linux-arm64",
  "linux-x86_64",
]);

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function assertShape(value, message) {
  assert(value && typeof value === "object" && !Array.isArray(value), message);
}

assert(existsSync(provenancePath), "npm/native/provenance.json is missing");
const provenance = JSON.parse(readFileSync(provenancePath, "utf8"));
assertShape(provenance, "provenance must be an object");
assert.equal(provenance.schema_version, "switchloom.native_provenance.v1");
assert.equal(provenance.package_version, packageVersion);
assert.equal(typeof provenance.git_sha, "string");
assert.match(provenance.git_sha, /^[0-9a-f]{40}$/);
assert(Array.isArray(provenance.targets), "targets must be an array");
assert.equal(provenance.targets.length, expectedTargets.size);

for (const target of provenance.targets) {
  assertShape(target, "target entry must be an object");
  assert(expectedTargets.delete(target.target), `unexpected target ${target.target}`);
  assert.equal(target.version, `model-routing ${packageVersion}`);
  assert.match(target.sha256, /^[0-9a-f]{64}$/);
  assert.equal(typeof target.rust_target, "string");
  assert.equal(typeof target.runner, "string");
  assert.equal(typeof target.built_at, "string");
  assert.equal(target.git_sha, provenance.git_sha);
  const binaryPath = resolve(repoRoot, target.path);
  assert(relative(repoRoot, binaryPath).startsWith("npm/native/"), `${target.path} must be under npm/native`);
  assert(existsSync(binaryPath), `${target.path} is missing`);
  assert.equal(sha256(binaryPath), target.sha256, `${target.path} sha256 mismatch`);
}

assert.equal(expectedTargets.size, 0, `missing targets: ${Array.from(expectedTargets).join(", ")}`);

const current = { darwin: "darwin", linux: "linux" }[process.platform];
const arch = { arm64: "arm64", x64: "x86_64" }[process.arch];
if (current && arch) {
  const target = `${current}-${arch}`;
  const binaryPath = resolve(repoRoot, "npm/native", target, "model-routing");
  if (existsSync(binaryPath)) {
    const result = spawnSync(binaryPath, ["--version"], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), `model-routing ${packageVersion}`);
  }
}

console.log(`native provenance validated for ${packageVersion}`);
