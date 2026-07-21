import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("OpenCode compatibility path delegates its contract to Rust xtask", () => {
  const result = spawnSync(process.execPath, ["scripts/validate-opencode-runtime-evidence.mjs", "--help"], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /--package-digest/);
});
