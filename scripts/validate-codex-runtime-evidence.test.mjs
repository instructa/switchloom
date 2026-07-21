import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("Codex compatibility path delegates its contract to Rust xtask", () => {
  const result = spawnSync(process.execPath, ["scripts/validate-codex-runtime-evidence.mjs", "--help"], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Validate an extracted Codex persisted-runtime receipt/);
});
