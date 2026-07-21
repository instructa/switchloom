#!/usr/bin/env node
import { spawnSync } from "node:child_process";

const result = spawnSync("cargo", [
  "run", "--quiet", "--manifest-path", new URL("../Cargo.toml", import.meta.url).pathname,
  "-p", "xtask", "--", "certify", "opencode", ...process.argv.slice(2),
], { stdio: "inherit" });
process.exit(result.status ?? 1);
