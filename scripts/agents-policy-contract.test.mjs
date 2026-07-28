import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const guidance = await readFile("AGENTS.md", "utf8");

test("repository guidance requires focused Switchloom verification", () => {
  assert.match(guidance, /Run the smallest check that covers the files changed\./);
  assert.match(guidance, /node --test scripts\/ci-router\.test\.mjs scripts\/ci-workflow-contract\.test\.mjs/);
});

test("repository guidance forbids routine broad replay and preserves security checks", () => {
  assert.match(guidance, /Do not routinely replay the full workspace, site build, native release matrix,/);
  assert.match(guidance, /Escalate to a broader check only when a shared\n+boundary changed, focused evidence is insufficient, or a release\/security task/);
  assert.match(guidance, /pnpm security:check/);
  assert.match(guidance, /do not weaken its scripts or hooks/);
});
