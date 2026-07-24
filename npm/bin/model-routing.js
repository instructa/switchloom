#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(here, "..", "..");
const packageMetadata = JSON.parse(fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"));

function platformTarget() {
  const osName = { darwin: "darwin", linux: "linux" }[os.platform()];
  const arch = { arm64: "arm64", x64: "x86_64" }[os.arch()];
  return osName && arch ? `${osName}-${arch}` : null;
}

const target = platformTarget();
const candidates = [
  { path: process.env.SWITCHLOOM_NATIVE_BIN, provenanceRequired: false },
  {
    path: target && path.join(here, "..", "native", target, "model-routing"),
    provenanceRequired: true,
  },
  { path: path.join(packageRoot, "target", "release", "model-routing"), provenanceRequired: false },
  { path: path.join(packageRoot, "target", "debug", "model-routing"), provenanceRequired: false },
].filter(({ path: candidate }) => Boolean(candidate));

const selected = candidates.find(({ path: candidate }) => fs.existsSync(candidate));
if (!selected) {
  if (!target) {
    console.error(`Switchloom has no native binary for ${os.platform()}-${os.arch()}.`);
    console.error("Supported platforms: darwin-arm64, darwin-x86_64, linux-arm64, linux-x86_64.");
  } else {
    console.error("Switchloom's native model-routing binary was not found.");
    console.error("Reinstall the package or set SWITCHLOOM_NATIVE_BIN=/absolute/path/to/model-routing.");
  }
  process.exit(127);
}

if (selected.provenanceRequired) {
  const version = spawnSync(selected.path, ["--version"], { encoding: "utf8" });
  const expected = `model-routing ${packageMetadata.version}`;
  if (version.error || version.status !== 0 || version.stdout.trim() !== expected) {
    console.error(
      `Switchloom native provenance mismatch: package switchloom@${packageMetadata.version} selected ${selected.path}, which reported ${JSON.stringify(version.stdout.trim())} instead of ${JSON.stringify(expected)}.`,
    );
    console.error("Reinstall the registry package or set SWITCHLOOM_NATIVE_BIN to an explicit verified binary.");
    process.exit(1);
  }
}

const result = spawnSync(selected.path, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 0);
