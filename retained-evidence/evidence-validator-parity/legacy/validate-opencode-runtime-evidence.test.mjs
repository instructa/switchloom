import { mkdtemp, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
import assert from "node:assert/strict";

const script = process.env.VALIDATOR_MODE === "rust"
  ? new URL("../../../scripts/validate-opencode-runtime-evidence.mjs", import.meta.url).pathname
  : new URL("./validate-opencode-runtime-evidence.mjs", import.meta.url).pathname;

async function runFixture(name, events) {
  const dir = await mkdtemp(join(tmpdir(), `opencode-evidence-${name}-`));
  const jsonl = join(dir, "host-output.jsonl");
  const invocation = join(dir, "requested-invocation.json");
  const receipt = join(dir, "dispatch-evidence.json");
  await writeFile(jsonl, events.map((event) => JSON.stringify(event)).join("\n"));
  await writeFile(invocation, JSON.stringify({ nonce: "nonce-123" }));
  const result = spawnSync(process.execPath, [
    script,
    "--jsonl", jsonl,
    "--invocation", invocation,
    "--receipt", receipt,
    "--package-digest", "sha256:abc",
    "--host-version", "1.14.17",
    "--profile", "opencode-worker",
    "--model", "opencode/gpt-5-nano",
    "--variant", "low",
    "--worker", "model-routing-preset-worker",
  ], { cwd: process.cwd(), encoding: "utf8" });
  return { ...result, receipt };
}

test("accepts correlated OpenCode Task evidence for the worker", async () => {
  const result = await runFixture("valid", [
    { type: "tool_call", tool: "Task", id: "call-1", agent: "model-routing-preset-worker", model: "opencode/gpt-5-nano", variant: "low" },
    { type: "tool_result", toolCallID: "call-1", agent: "model-routing-preset-worker", result: "nonce-123", model: "opencode/gpt-5-nano", variant: "low" },
  ]);
  assert.equal(result.status, 0, result.stderr);
  const receipt = JSON.parse(await readFile(result.receipt, "utf8"));
  assert.equal(receipt.child_identity.agent_role, "model-routing-preset-worker");
  assert.equal(receipt.effective_model, "opencode/gpt-5-nano");
  assert.equal(receipt.effective_effort, "low");
});

test("rejects a driver-only echoed nonce", async () => {
  const result = await runFixture("driver-echo", [
    { type: "message", agent: "model-routing-preset-driver", result: "nonce-123", model: "opencode/gpt-5-nano" },
  ]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /no structured Task invocation/);
});

test("rejects a task invocation whose result does not come from the worker", async () => {
  const result = await runFixture("mismatched-child", [
    { type: "tool_call", tool: "Task", id: "call-1", agent: "model-routing-preset-worker", model: "opencode/gpt-5-nano" },
    { type: "tool_result", toolCallID: "call-1", agent: "other-worker", result: "nonce-123" },
  ]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /worker result came from other-worker/);
});

test("rejects an echoed nonce in the requested input event", async () => {
  const result = await runFixture("input-echo", [
    { type: "input", prompt: "Return nonce-123 using model-routing-preset-worker" },
    { type: "tool_call", tool: "Task", id: "call-1", agent: "model-routing-preset-worker" },
  ]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /was not returned by an explicit/);
});

test("rejects a later driver message that mentions the worker and nonce", async () => {
  const result = await runFixture("later-driver-echo", [
    { type: "tool_call", tool: "Task", id: "call-1", agent: "model-routing-preset-worker", model: "opencode/gpt-5-nano" },
    { type: "message", agent: "model-routing-preset-driver", text: "model-routing-preset-worker returned nonce-123" },
  ]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /was not returned by an explicit/);
});
