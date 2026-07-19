#!/usr/bin/env node
import { readFile } from "node:fs/promises";

const [receiptPath] = process.argv.slice(2);
if (!receiptPath) {
  throw new Error("usage: node scripts/validate-codex-runtime-evidence.mjs <receipt.json>");
}

const schemaVersion = "switchloom.codex_runtime_evidence.v1";
const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const requiredChildren = new Map([
  ["worker", { agent_type: "model_routing_terra_high", model: "gpt-5.6-terra", effort: "high" }],
  ["reviewer", { agent_type: "model_routing_sol_high", model: "gpt-5.6-sol", effort: "high" }],
]);

function fail(message) {
  throw new Error(`codex runtime evidence validation failed: ${message}`);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function assertUuid(value, label) {
  assert(typeof value === "string" && uuidPattern.test(value), `${label} must be a UUID`);
}

const receipt = JSON.parse(await readFile(receiptPath, "utf8"));
assert(receipt.schema_version === schemaVersion, `schema_version must be ${schemaVersion}`);
assert(receipt.run?.status === "complete", "run.status must be complete");
assert(receipt.run?.complete_marker === "SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE", "run complete marker missing");
assert(receipt.run?.evidence_source === "codex_persisted_spawn_state", "run.evidence_source must be codex_persisted_spawn_state");
assertUuid(receipt.run?.parent_thread_id, "run.parent_thread_id");
assert(typeof receipt.run?.parent_session === "string" && receipt.run.parent_session.endsWith(".jsonl"), "run.parent_session must name a persisted session jsonl");
assert(typeof receipt.run?.workdir === "string" && receipt.run.workdir.startsWith("/"), "run.workdir must be absolute");
assert(Array.isArray(receipt.children), "children must be an array");
assert(receipt.children.length === requiredChildren.size, "children must contain worker and reviewer only");

const seenKinds = new Set();
for (const child of receipt.children) {
  const expected = requiredChildren.get(child.kind);
  assert(expected, `unexpected child kind ${child.kind}`);
  assert(!seenKinds.has(child.kind), `duplicate child kind ${child.kind}`);
  seenKinds.add(child.kind);
  assert(child.agent_type === expected.agent_type, `${child.kind} agent_type mismatch`);
  assert(child.task_name && /^[a-z][a-z0-9_]*$/.test(child.task_name), `${child.kind} task_name is invalid`);
  assert(child.canonical_task === `/root/${child.task_name}`, `${child.kind} canonical_task mismatch`);
  assert(child.parent_thread_id === receipt.run.parent_thread_id, `${child.kind} parent_thread_id mismatch`);
  assertUuid(child.child_thread_id, `${child.kind} child_thread_id`);
  assert(child.child_thread_id !== receipt.run.parent_thread_id, `${child.kind} child_thread_id must differ from parent`);
  assert(child.spawn?.agent_type === child.agent_type, `${child.kind} spawn agent_type mismatch`);
  assert(child.spawn?.task_name === child.task_name, `${child.kind} spawn task_name mismatch`);
  assert(child.spawn?.fork_turns === "none", `${child.kind} fork_turns must be none`);
  assert(!("model" in child.spawn), `${child.kind} spawn must not manually override model`);
  assert(!("reasoning_effort" in child.spawn), `${child.kind} spawn must not manually override effort`);
  assert(
    child.spawn_output?.task_name === child.canonical_task || child.spawn_output?.agent_id === child.child_thread_id,
    `${child.kind} spawn output task mismatch`,
  );
  assert(child.session?.agent_role === child.agent_type, `${child.kind} session agent_role mismatch`);
  assert(
    child.session?.agent_path === child.canonical_task || child.session?.agent_path === null,
    `${child.kind} session agent_path mismatch`,
  );
  assert(child.session?.thread_source === "subagent", `${child.kind} session thread_source mismatch`);
  assert(child.session?.parent_thread_id === receipt.run.parent_thread_id, `${child.kind} session parent mismatch`);
  assert(typeof child.session?.session_file === "string" && child.session.session_file.endsWith(".jsonl"), `${child.kind} session file missing`);
  assert(child.state?.agent_role === child.agent_type, `${child.kind} state agent_role mismatch`);
  assert(
    child.state?.agent_path === child.canonical_task || child.state?.agent_path === null,
    `${child.kind} state agent_path mismatch`,
  );
  assert(child.state?.model === expected.model, `${child.kind} effective model mismatch`);
  assert(child.state?.reasoning_effort === expected.effort, `${child.kind} effective effort mismatch`);
  assert(child.state?.thread_source === "subagent", `${child.kind} state thread_source mismatch`);
  assert(child.state?.cwd === receipt.run.workdir, `${child.kind} state cwd mismatch`);
  assert(child.state.model !== "gpt-5.6-sol" || child.state.reasoning_effort !== "medium", `${child.kind} inherited Sol Medium evidence is forbidden`);
  assert(child.final_answer?.message_type === "FINAL_ANSWER", `${child.kind} final answer marker missing`);
}

for (const kind of requiredChildren.keys()) {
  assert(seenKinds.has(kind), `missing ${kind} child evidence`);
}

console.log("codex runtime evidence validation passed");
