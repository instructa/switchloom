#!/usr/bin/env node
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const baseUrl = process.argv[2];
const routingBin = resolve(packageRoot, process.argv[3] ?? "target/release/model-routing");

if (!baseUrl) {
  throw new Error("usage: node scripts/verify-cloudflare-website.mjs <url> [routing-bin]");
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function fetchOk(path) {
  const url = new URL(path, baseUrl);
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`${url} returned HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

function run(args) {
  const result = spawnSync(routingBin, args, {
    cwd: packageRoot,
    stdio: "inherit",
    env: process.env,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${routingBin} ${args.join(" ")} exited with status ${result.status}`);
  }
}

const html = new TextDecoder().decode(await fetchOk("/"));
if (!html.includes("Switchloom")) throw new Error("deployed homepage does not contain Switchloom branding");
if (!html.includes("Build your coding-agent team")) {
  throw new Error("deployed homepage does not expose the setup generator");
}

const catalogBytes = await fetchOk("/data/catalog.json");
const catalog = JSON.parse(new TextDecoder().decode(catalogBytes));
if (catalog.schemaVersion !== 1 || catalog.compositions?.length !== 28) {
  throw new Error("deployed catalog has unexpected shape");
}
if (catalog.setupContract?.recipePrefix !== "sw1_" || catalog.setupContract?.configPath !== ".switchloom/config.toml") {
  throw new Error("deployed catalog is missing setup transport metadata");
}
if (catalog.setupContract?.transport?.mayContainCredentials !== false || catalog.setupContract?.transport?.mayContainScripts !== false) {
  throw new Error("deployed setup transport does not advertise credential/script-free recipes");
}
for (const host of ["codex", "cursor", "claude-code", "opencode", "pi"]) {
  const setupHost = catalog.setupContract?.hosts?.find((candidate) => candidate.id === host);
  if (!setupHost?.binding || !Array.isArray(setupHost.models) || setupHost.models.length === 0) {
    throw new Error(`deployed setup contract missing ${host} host inputs`);
  }
}

const scriptMatches = [...html.matchAll(/(?:src|component-url)="([^"]*Generator[^"]*\.js)"/g)];
if (scriptMatches.length === 0) throw new Error("deployed homepage missing generator client script");
const generatorScript = new TextDecoder().decode(await fetchOk(scriptMatches.at(-1)[1]));
for (const required of [
  "Copy npx recipe command",
  "Download .switchloom/config.toml",
  "Standalone or With Planr",
  "switchloom preview --recipe",
  "switchloom rollback --repository .",
]) {
  if (!generatorScript.includes(required)) throw new Error(`deployed generator missing ${required}`);
}
for (const removed of ["jszip", "Download setup (.zip)", "switchloom.config.json"]) {
  if (generatorScript.toLowerCase().includes(removed.toLowerCase())) {
    throw new Error(`deployed generator still contains removed browser artifact path: ${removed}`);
  }
}

const entry = catalog.compositions.find(
  (candidate) => candidate.policy?.id === "balanced" && candidate.binding?.id === "codex-openai",
);
if (!entry) throw new Error("deployed catalog missing balanced + codex-openai");

const remoteBundle = await fetchOk(`/data/bundles/${entry.entryId}.json`);
const workdir = mkdtempSync(join(tmpdir(), "model-routing-cloudflare-parity-"));
const localBundle = join(workdir, `${entry.entryId}.json`);
run(["compile", entry.policy.id, "--host", entry.binding.id, "--output", localBundle]);
const localBytes = readFileSync(localBundle);
const remoteHash = sha256(remoteBundle);
const localHash = sha256(localBytes);
if (remoteHash !== localHash || Buffer.compare(Buffer.from(remoteBundle), localBytes) !== 0) {
  throw new Error(`deployed bundle does not match CLI output: remote ${remoteHash}, local ${localHash}`);
}

console.log(`cloudflare website verified: ${new URL(baseUrl).href}`);
console.log(`catalog entries: ${catalog.compositions.length}`);
console.log(`setup contract hosts: ${catalog.setupContract.hosts.length}`);
console.log(`download parity: ${entry.entryId}.json ${remoteHash}`);
