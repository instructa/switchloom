#!/usr/bin/env node
import { readFile } from "node:fs/promises";

const positional = [];
const options = new Map();
for (let index = 2; index < process.argv.length; index += 1) {
  const arg = process.argv[index];
  if (arg === "--expect") {
    options.set("expect", process.argv[index + 1]);
    index += 1;
  } else {
    positional.push(arg);
  }
}

const [receiptPath] = positional;
if (!receiptPath || positional.length > 1 || (options.has("expect") && !options.get("expect"))) {
  throw new Error("usage: node scripts/validate-codex-runtime-evidence.mjs <receipt.json> [--expect <expected.json>]");
}

const schemaVersion = "switchloom.codex_runtime_evidence.v1";
const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const sha256DigestPattern = /^sha256:[0-9a-f]{64}$/;
const sha256HexPattern = /^[0-9a-f]{64}$/;
const codexVersionPattern = /^codex(?:-cli)?\s+\d+\.\d+\.\d+(?:\b|[-+])/;

function fail(message) {
  throw new Error(`codex runtime evidence validation failed: ${message}`);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function assertUuid(value, label) {
  assert(typeof value === "string" && uuidPattern.test(value), `${label} must be a UUID`);
}

function assertString(value, label) {
  assert(typeof value === "string" && value.trim().length > 0, `${label} must not be blank`);
}

function expectedChildrenFrom(receipt, expected) {
  const children = expected?.children ?? receipt.children;
  assert(Array.isArray(children), "expected children must be an array");
  return new Map(children.map((child) => {
    const kind = child.semantic_role ?? child.kind;
    const model = child.model ?? child.state?.model;
    const effort = child.effort ?? child.state?.reasoning_effort;
    assertString(kind, "expected child semantic_role");
    assertString(child.agent_type, `${kind} expected agent_type`);
    assertString(child.profile, `${kind} expected profile`);
    assertString(child.task_name, `${kind} expected task_name`);
    assertString(child.message_sha256 ?? child.message_ciphertext_sha256 ?? child.input?.message_sha256, `${kind} expected message hash`);
    assertString(model, `${kind} expected model`);
    assertString(effort, `${kind} expected effort`);
    return [kind, { ...child, model, effort }];
  }));
}

const receipt = JSON.parse(await readFile(receiptPath, "utf8"));
const expectedReceipt = options.has("expect")
  ? JSON.parse(await readFile(options.get("expect"), "utf8"))
  : undefined;
const requiredChildren = expectedChildrenFrom(receipt, expectedReceipt);
assert(receipt.schema_version === schemaVersion, `schema_version must be ${schemaVersion}`);
assert(receipt.run?.status === "complete", "run.status must be complete");
assert(receipt.run?.complete_marker === "SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE", "run complete marker missing");
assert(receipt.run?.evidence_source === "codex_persisted_spawn_state", "run.evidence_source must be codex_persisted_spawn_state");
assertUuid(receipt.run?.parent_thread_id, "run.parent_thread_id");
assert(typeof receipt.run?.parent_session === "string" && receipt.run.parent_session.endsWith(".jsonl"), "run.parent_session must name a persisted session jsonl");
assert(typeof receipt.run?.workdir === "string" && receipt.run.workdir.startsWith("/"), "run.workdir must be absolute");
assert(Array.isArray(receipt.children), "children must be an array");
assert(receipt.children.length === requiredChildren.size, "children must contain worker and reviewer only");
assert(Array.isArray(receipt.dispatch_evidence), "dispatch_evidence must be an array");
assert(receipt.dispatch_evidence.length === requiredChildren.size, "dispatch_evidence must contain worker and reviewer receipts only");

const seenKinds = new Set();
for (const child of receipt.children) {
  const expected = requiredChildren.get(child.kind);
  assert(expected, `unexpected child kind ${child.kind}`);
  assert(!seenKinds.has(child.kind), `duplicate child kind ${child.kind}`);
  seenKinds.add(child.kind);
  assert(child.agent_type === expected.agent_type, `${child.kind} agent_type mismatch`);
  assert(child.task_name === expected.task_name, `${child.kind} task_name mismatch`);
  assert(/^[a-z][a-z0-9_]*$/.test(child.task_name), `${child.kind} task_name is invalid`);
  assert(child.canonical_task === `/root/${child.task_name}`, `${child.kind} canonical_task mismatch`);
  assert(child.parent_thread_id === receipt.run.parent_thread_id, `${child.kind} parent_thread_id mismatch`);
  assertUuid(child.child_thread_id, `${child.kind} child_thread_id`);
  assert(child.child_thread_id !== receipt.run.parent_thread_id, `${child.kind} child_thread_id must differ from parent`);
  assert(child.spawn?.surface === "collaboration.spawn_agent", `${child.kind} spawn surface must be Codex V2 collaboration.spawn_agent`);
  assert(child.spawn?.agent_type === child.agent_type, `${child.kind} spawn agent_type mismatch`);
  assert(child.spawn?.task_name === child.task_name, `${child.kind} spawn task_name mismatch`);
  assert(child.spawn?.fork_turns === "none", `${child.kind} fork_turns must be none`);
  assertString(child.spawn?.call_id, `${child.kind} spawn call_id`);
  assert(!("model" in child.spawn), `${child.kind} spawn must not manually override model`);
  assert(!("reasoning_effort" in child.spawn), `${child.kind} spawn must not manually override effort`);
  const expectedMessageSha256 = expected.message_sha256 ?? expected.input?.message_sha256;
  const expectedCiphertextSha256 = expected.message_ciphertext_sha256;
  const expectedMaxMessageBytes = expected.max_message_bytes ?? expected.input?.max_message_bytes;
  if (expectedMessageSha256 !== undefined) {
    assert(sha256HexPattern.test(expectedMessageSha256), `${child.kind} expected message_sha256 must be lowercase sha256 hex`);
  }
  if (expectedCiphertextSha256 !== undefined) {
    assert(sha256HexPattern.test(expectedCiphertextSha256), `${child.kind} expected message_ciphertext_sha256 must be lowercase sha256 hex`);
  }
  assert(Number.isInteger(expectedMaxMessageBytes) && expectedMaxMessageBytes > 0, `${child.kind} expected max_message_bytes must be positive`);
  assert(sha256HexPattern.test(child.input?.message_sha256), `${child.kind} input message_sha256 must be lowercase sha256 hex`);
  const messageEncoding = child.input?.message_encoding ?? "plaintext";
  if (messageEncoding === "plaintext") {
    assert(expectedMessageSha256 !== undefined, `${child.kind} plaintext input requires expected message_sha256`);
    assert(child.input.message_sha256 === expectedMessageSha256, `${child.kind} input message_sha256 mismatch`);
    assert(child.input.message_plaintext_verdict === "deterministic", `${child.kind} input message_plaintext_verdict mismatch`);
  } else if (messageEncoding === "codex-encrypted") {
    if (expectedCiphertextSha256 !== undefined) {
      assert(child.input.message_sha256 === expectedCiphertextSha256, `${child.kind} input message_ciphertext_sha256 mismatch`);
    }
    assert(child.input.message_plaintext_verdict === "unsupported", `${child.kind} encrypted input cannot claim deterministic plaintext`);
    if (child.input.message_plaintext_intent_sha256 !== undefined) {
      assert(expectedMessageSha256 !== undefined, `${child.kind} encrypted input intent hash has no expected plaintext hash`);
      assert(child.input.message_plaintext_intent_sha256 === expectedMessageSha256, `${child.kind} input message_plaintext_intent_sha256 mismatch`);
    }
  } else {
    fail(`${child.kind} unsupported input message_encoding ${messageEncoding}`);
  }
  assert(Number.isInteger(child.input?.message_bytes) && child.input.message_bytes > 0, `${child.kind} input message_bytes must be positive`);
  assert(child.input.message_bytes <= expectedMaxMessageBytes, `${child.kind} input message_bytes exceeds max_message_bytes`);
  assert(child.input?.max_message_bytes === expectedMaxMessageBytes, `${child.kind} input max_message_bytes mismatch`);
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

  const dispatchEvidence = receipt.dispatch_evidence.find((evidence) => {
    return evidence?.requested_dispatch?.semantic_role === child.kind
      && evidence?.requested_dispatch?.agent_type === child.agent_type
      && evidence?.child_identity?.task_name === child.task_name;
  });
  assert(dispatchEvidence, `${child.kind} dispatch_evidence receipt missing`);
  assert(dispatchEvidence.schema_version === 1, `${child.kind} dispatch_evidence schema_version mismatch`);
  assertString(dispatchEvidence.package_digest, `${child.kind} dispatch_evidence package_digest`);
  assertString(dispatchEvidence.host_version, `${child.kind} dispatch_evidence host_version`);
  assert(sha256DigestPattern.test(dispatchEvidence.package_digest), `${child.kind} dispatch_evidence package_digest must be sha256:<64 lowercase hex>`);
  assert(codexVersionPattern.test(dispatchEvidence.host_version), `${child.kind} dispatch_evidence host_version must come from codex --version`);
  if (expectedReceipt) {
    assert(dispatchEvidence.package_digest === expectedReceipt.package_digest, `${child.kind} package_digest mismatch`);
    assert(dispatchEvidence.host_version === expectedReceipt.host_version, `${child.kind} host_version mismatch`);
  }
  assert(dispatchEvidence.requested_dispatch.profile === expected.profile, `${child.kind} requested profile mismatch`);
  assert(dispatchEvidence.requested_dispatch.model === expected.model, `${child.kind} requested model mismatch`);
  assert(dispatchEvidence.requested_dispatch.effort === expected.effort, `${child.kind} requested effort mismatch`);
  assert(dispatchEvidence.requested_dispatch.agent_type === child.agent_type, `${child.kind} requested agent_type mismatch`);
  assert(dispatchEvidence.requested_dispatch.fork_turns?.mode === "none", `${child.kind} requested fork_turns must be none`);
  assert(!("turns" in dispatchEvidence.requested_dispatch.fork_turns), `${child.kind} fork_turns none must not include turns`);
  assert(dispatchEvidence.requested_dispatch.message_sha256 === child.input.message_sha256, `${child.kind} requested message_sha256 mismatch`);
  assert(dispatchEvidence.requested_dispatch.message_encoding === messageEncoding, `${child.kind} requested message_encoding mismatch`);
  assert(dispatchEvidence.requested_dispatch.message_plaintext_verdict === child.input.message_plaintext_verdict, `${child.kind} requested message_plaintext_verdict mismatch`);
  assert(dispatchEvidence.requested_dispatch.message_plaintext_intent_sha256 === child.input.message_plaintext_intent_sha256, `${child.kind} requested message_plaintext_intent_sha256 mismatch`);
  assert(dispatchEvidence.requested_dispatch.message_bytes === child.input.message_bytes, `${child.kind} requested message_bytes mismatch`);
  assert(dispatchEvidence.requested_dispatch.max_message_bytes === expectedMaxMessageBytes, `${child.kind} requested max_message_bytes mismatch`);
  assert(dispatchEvidence.child_identity.host === "codex", `${child.kind} child host mismatch`);
  assert(dispatchEvidence.child_identity.role === child.kind, `${child.kind} child role mismatch`);
  assert(dispatchEvidence.child_identity.agent_role === child.agent_type, `${child.kind} child agent_role mismatch`);
  assert(dispatchEvidence.child_identity.agent_type === child.agent_type, `${child.kind} child agent_type mismatch`);
  assert(dispatchEvidence.effective_model === child.state.model, `${child.kind} effective model receipt mismatch`);
  assert(dispatchEvidence.effective_effort === child.state.reasoning_effort, `${child.kind} effective effort receipt mismatch`);
  assert(dispatchEvidence.nonce === `${receipt.run.parent_thread_id}:${child.child_thread_id}:${child.spawn.call_id}`, `${child.kind} dispatch_evidence nonce must bind parent thread, child thread, and spawn call`);
  assert(
    !dispatchEvidence.nonce.includes("nonce-") && !dispatchEvidence.nonce.includes("placeholder"),
    `${child.kind} dispatch_evidence nonce must be runtime-derived, not a placeholder`,
  );
  assert(Array.isArray(dispatchEvidence.raw_evidence_refs) && dispatchEvidence.raw_evidence_refs.length > 0, `${child.kind} raw evidence refs missing`);
  assert(
    dispatchEvidence.raw_evidence_refs.includes(`codex-session:${receipt.run.parent_session}`)
      && dispatchEvidence.raw_evidence_refs.includes(`codex-session:${child.session.session_file}`)
      && dispatchEvidence.raw_evidence_refs.includes(`state_5.sqlite:thread_spawn_edges:${receipt.run.parent_thread_id}:${child.child_thread_id}`)
      && dispatchEvidence.raw_evidence_refs.includes(`spawn_call:${child.spawn.call_id}`),
    `${child.kind} raw evidence refs must bind parent session, child session, spawn edge, and spawn call`,
  );
  assert(dispatchEvidence.verdict === "deterministic", `${child.kind} dispatch_evidence verdict mismatch`);
}

for (const kind of requiredChildren.keys()) {
  assert(seenKinds.has(kind), `missing ${kind} child evidence`);
}

console.log("codex runtime evidence validation passed");
