#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function option(name, fallback) {
  const index = process.argv.indexOf(name);
  return index === -1 ? fallback : process.argv[index + 1];
}

const routingBin = resolve(packageRoot, option("--routing-bin", "target/release/model-routing"));
const catalog = resolve(packageRoot, "website/data/catalog.json");
const bundles = resolve(packageRoot, "website/data/bundles");

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

try {
  run(["catalog", "build", "--output", catalog]);
  run(["catalog", "verify", catalog]);
  const data = JSON.parse(readFileSync(catalog, "utf8"));
  rmSync(bundles, { recursive: true, force: true });
  mkdirSync(bundles, { recursive: true });
  for (const entry of data.compositions ?? []) {
    run([
      "compile",
      entry.policy.id,
      "--host",
      entry.binding.id,
      "--output",
      resolve(bundles, `${entry.entryId}.json`),
    ]);
  }
  console.log(`regenerated ${data.compositions.length} experimental Model Routing compositions and downloads`);
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
}
