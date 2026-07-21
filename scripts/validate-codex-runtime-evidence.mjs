#!/usr/bin/env node
import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);
const receipt = args[0]?.startsWith("--") ? undefined : args.shift();
const translated = receipt ? ["--receipt", receipt, ...args] : args;
const result = spawnSync("cargo", [
  "run", "--quiet", "--manifest-path", new URL("../Cargo.toml", import.meta.url).pathname,
  "-p", "xtask", "--", "certify", "codex", ...translated,
], { stdio: "inherit" });
process.exit(result.status ?? 1);
