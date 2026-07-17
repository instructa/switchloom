#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(here, "..", "..");

function platformTarget() {
  const osName = { darwin: "darwin", linux: "linux" }[os.platform()];
  const arch = { arm64: "arm64", x64: "x86_64" }[os.arch()];
  return osName && arch ? `${osName}-${arch}` : null;
}

const target = platformTarget();
const candidates = [
  process.env.SWITCHLOOM_NATIVE_BIN,
  target && path.join(here, "..", "native", target, "model-routing"),
  path.join(packageRoot, "target", "release", "model-routing"),
  path.join(packageRoot, "target", "debug", "model-routing"),
].filter(Boolean);

const binary = candidates.find((candidate) => fs.existsSync(candidate));
if (!binary) {
  if (!target) {
    console.error(`Switchloom has no native binary for ${os.platform()}-${os.arch()}.`);
    console.error("Supported platforms: darwin-arm64, darwin-x86_64, linux-arm64, linux-x86_64.");
  } else {
    console.error("Switchloom's native model-routing binary was not found.");
    console.error("Reinstall the package or set SWITCHLOOM_NATIVE_BIN=/absolute/path/to/model-routing.");
  }
  process.exit(127);
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 0);
