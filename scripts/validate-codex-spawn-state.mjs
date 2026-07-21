#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { basename, join, resolve } from "node:path";

function usage() {
  console.error(
    "usage: validate-codex-spawn-state.mjs --events <jsonl> --workdir <dir> --expect <json> [--state-db <sqlite>] [--sessions-dir <dir>]",
  );
  process.exit(2);
}

const args = new Map();
for (let i = 2; i < process.argv.length; i += 2) {
  if (!process.argv[i]?.startsWith("--") || !process.argv[i + 1]) usage();
  args.set(process.argv[i].slice(2), process.argv[i + 1]);
}

const eventsPath = args.get("events");
const workdir = args.get("workdir");
const expectPath = args.get("expect");
const stateDb = args.get("state-db") ?? join(homedir(), ".codex", "state_5.sqlite");
const sessionsDir = args.get("sessions-dir") ?? join(homedir(), ".codex", "sessions");
const archivedSessionsDir = args.get("archived-sessions-dir") ?? join(homedir(), ".codex", "archived_sessions");
if (!eventsPath || !workdir || !expectPath) usage();

const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const sha256DigestPattern = /^sha256:[0-9a-f]{64}$/;
const codexVersionPattern = /^codex(?:-cli)?\s+\d+\.\d+\.\d+(?:\b|[-+])/;

function fail(message) {
  console.error(`codex spawn state validation failed: ${message}`);
  process.exit(1);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function readJsonl(path) {
  return readFileSync(path, "utf8")
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line, index) => {
      try {
        return JSON.parse(line);
      } catch (error) {
        fail(`${path}:${index + 1} is not JSON: ${error.message}`);
      }
    });
}

function walk(dir, suffix, hits = []) {
  if (!existsSync(dir)) return hits;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(path, suffix, hits);
    } else if (entry.isFile() && entry.name.endsWith(suffix)) {
      hits.push(path);
    }
  }
  return hits;
}

function findSession(threadId) {
  assert(uuidPattern.test(threadId), `invalid thread id ${threadId}`);
  const suffix = `${threadId}.jsonl`;
  const hits = [...walk(sessionsDir, suffix), ...walk(archivedSessionsDir, suffix)];
  assert(hits.length > 0, `no persisted Codex session found for ${threadId}`);
  assert(hits.length === 1, `multiple persisted Codex sessions found for ${threadId}: ${hits.join(", ")}`);
  return hits[0];
}

function parseJsonObject(value, label) {
  assert(typeof value === "string" && value.length > 0, `${label} is not a JSON string`);
  try {
    return JSON.parse(value);
  } catch (error) {
    fail(`${label} is not valid JSON: ${error.message}`);
  }
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function byteLength(value) {
  return Buffer.byteLength(value, "utf8");
}

function jsStringField(source, field) {
  const escaped = field.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = source.match(new RegExp(`${escaped}\\s*:\\s*["']([^"']+)["']`));
  return match?.[1];
}

function sqliteJson(query) {
  assert(existsSync(stateDb), `Codex state DB not found at ${stateDb}`);
  const result = spawnSync("sqlite3", ["-json", stateDb, query], { encoding: "utf8" });
  assert(result.status === 0, `sqlite3 query failed: ${result.stderr || result.stdout}`);
  return result.stdout.trim() ? JSON.parse(result.stdout) : [];
}

const expected = JSON.parse(readFileSync(expectPath, "utf8"));
assert(Array.isArray(expected.children) && expected.children.length > 0, "expected children list is empty");
assert(sha256DigestPattern.test(expected.package_digest), "expected package_digest must be sha256:<64 lowercase hex>");
assert(codexVersionPattern.test(expected.host_version), "expected host_version must be observed codex --version output");

const outerEvents = readJsonl(eventsPath);
const started = outerEvents.find((record) => record.type === "thread.started");
assert(started?.thread_id, "Codex exec JSONL did not contain thread.started.thread_id");
const parentThreadId = started.thread_id;
assert(uuidPattern.test(parentThreadId), `invalid parent thread id ${parentThreadId}`);

const parentSessionPath = findSession(parentThreadId);
const parentRecords = readJsonl(parentSessionPath);
const parentMeta = parentRecords.find((record) => record.type === "session_meta")?.payload;
assert(parentMeta?.id === parentThreadId, "parent session_meta id does not match thread.started");
assert(parentMeta?.thread_source === "user", "parent session is not a user thread");
assert(resolve(parentMeta?.cwd ?? "") === resolve(workdir), "parent session cwd does not match oracle workdir");

const edgeRows = sqliteJson(`
  select
    e.parent_thread_id,
    e.child_thread_id,
    e.status,
    t.agent_path,
    t.agent_role,
    t.model,
    t.reasoning_effort,
    t.thread_source,
    t.cwd
  from thread_spawn_edges e
  join threads t on t.id = e.child_thread_id
  where e.parent_thread_id = '${parentThreadId}'
`);

const expectedByIdentity = new Map();
const expectedCanonicalTasks = new Set();
const expectedAgentTypes = new Set();
for (const child of expected.children) {
  assert(typeof child.agent_type === "string" && child.agent_type.length > 0, "expected child missing agent_type");
  assert(typeof child.task_name === "string" && child.task_name.length > 0, "expected child missing task_name");
  assert(typeof child.semantic_role === "string" && child.semantic_role.length > 0, `${child.agent_type} missing semantic_role`);
  assert(typeof child.profile === "string" && child.profile.length > 0, `${child.agent_type} missing profile`);
  assert(typeof child.canonical_task === "string" && child.canonical_task.startsWith("/root/"), `${child.agent_type} has invalid canonical_task`);
  if ("message_sha256" in child) {
    assert(typeof child.message_sha256 === "string" && /^[0-9a-f]{64}$/.test(child.message_sha256), `${child.agent_type} expected message_sha256 must be lowercase sha256 hex`);
  }
  if ("message_ciphertext_sha256" in child) {
    assert(typeof child.message_ciphertext_sha256 === "string" && /^[0-9a-f]{64}$/.test(child.message_ciphertext_sha256), `${child.agent_type} expected message_ciphertext_sha256 must be lowercase sha256 hex`);
  }
  assert(Number.isInteger(child.max_message_bytes) && child.max_message_bytes > 0, `${child.agent_type} expected max_message_bytes must be positive`);
  const identity = `${child.agent_type}\0${child.task_name}`;
  assert(!expectedByIdentity.has(identity), `${child.agent_type} duplicate expected agent_type/task_name`);
  expectedByIdentity.set(identity, child);
  expectedCanonicalTasks.add(child.canonical_task);
  expectedAgentTypes.add(child.agent_type);
}

const allV2SpawnCalls = parentRecords.filter((record) => {
  const payload = record.payload;
  return record.type === "response_item"
    && payload?.type === "function_call"
    && payload.namespace === "collaboration"
    && payload.name === "spawn_agent";
});
assert(allV2SpawnCalls.length === expected.children.length, `parent must contain exactly ${expected.children.length} V2 spawn_agent calls`);
for (const record of allV2SpawnCalls) {
  const payload = record.payload;
  const callArgs = parseJsonObject(payload.arguments, `spawn_agent arguments for ${payload.call_id}`);
  const identity = `${callArgs.agent_type ?? ""}\0${callArgs.task_name ?? ""}`;
  assert(expectedByIdentity.has(identity), `unexpected spawn_agent call for agent_type=${callArgs.agent_type ?? "<missing>"} task_name=${callArgs.task_name ?? "<missing>"}`);
}

const allStartedActivities = parentRecords.filter((record) => {
  const payload = record.payload;
  return record.type === "event_msg"
    && payload?.type === "sub_agent_activity"
    && payload.kind === "started";
});
assert(allStartedActivities.length === expected.children.length, `parent must contain exactly ${expected.children.length} sub_agent_activity started events`);
for (const record of allStartedActivities) {
  const payload = record.payload;
  assert(expectedCanonicalTasks.has(payload.agent_path), `unexpected started child path ${payload.agent_path ?? "<missing>"}`);
}

assert(edgeRows.length === expected.children.length, `parent must contain exactly ${expected.children.length} persisted child edges`);
for (const edge of edgeRows) {
  assert(expectedAgentTypes.has(edge.agent_role), `unexpected persisted child agent_role ${edge.agent_role ?? "<missing>"}`);
  if (edge.agent_path !== null) {
    assert(expectedCanonicalTasks.has(edge.agent_path), `unexpected persisted child path ${edge.agent_path}`);
  }
}

const observed = [];
for (const child of expected.children) {
  const canonicalTask = child.canonical_task;

  const legacySpawnCalls = parentRecords.filter((record) => {
    const payload = record.payload;
    if (record.type !== "response_item" || payload?.type !== "function_call") return false;
    if (payload.namespace !== "collaboration" || payload.name !== "spawn_agent") return false;
    const callArgs = parseJsonObject(payload.arguments, `spawn_agent arguments for ${payload.call_id}`);
    return callArgs.agent_type === child.agent_type && callArgs.task_name === child.task_name;
  });
  const v1SpawnCalls = parentRecords.filter((record) => {
    const payload = record.payload;
    if (record.type !== "response_item" || payload?.type !== "custom_tool_call") return false;
    if (payload.name !== "exec" || !payload.input?.includes("multi_agent_v1__spawn_agent")) return false;
    return jsStringField(payload.input, "agent_type") === child.agent_type;
  });
  assert(v1SpawnCalls.length === 0, `${child.agent_type} V1 spawn evidence is not accepted for Codex V2 certification`);
  assert(
    legacySpawnCalls.length === 1,
    `${child.agent_type} must have exactly one raw spawn_agent call`,
  );

  let spawnArgs;
  let spawnOutput;
  let childThreadId;
  let observedAgentPath;
  let spawnCallId;
  let spawnSurface;
  const spawnCall = legacySpawnCalls[0].payload;
  spawnCallId = spawnCall.call_id;
  spawnSurface = "collaboration.spawn_agent";
  spawnArgs = parseJsonObject(spawnCall.arguments, `spawn_agent arguments for ${child.agent_type}`);
  assert(spawnArgs.agent_type === child.agent_type, `${child.agent_type} spawn agent_type mismatch`);
  assert(spawnArgs.fork_turns === "none", `${child.agent_type} spawn did not use fork_turns=none`);
  assert(spawnArgs.task_name === child.task_name, `${child.agent_type} spawn task_name mismatch`);
  assert(typeof spawnArgs.message === "string" && spawnArgs.message.length > 0, `${child.agent_type} spawn message missing`);
  const messageBytes = byteLength(spawnArgs.message);
  assert(messageBytes <= child.max_message_bytes, `${child.agent_type} spawn message exceeds max_message_bytes`);
  const messageSha256 = sha256(spawnArgs.message);
  const messageInput = {
    message_sha256: messageSha256,
    message_bytes: messageBytes,
    max_message_bytes: child.max_message_bytes,
    message_encoding: "plaintext",
    message_plaintext_verdict: "deterministic",
  };
  if (messageSha256 !== child.message_sha256) {
    assert(/^gAAAA[A-Za-z0-9_-]+={0,2}$/.test(spawnArgs.message), `${child.agent_type} spawn message_sha256 mismatch`);
    assert(child.allow_encrypted_message === true || child.message_ciphertext_sha256, `${child.agent_type} encrypted spawn message cannot certify expected plaintext`);
    if (child.message_ciphertext_sha256) {
      assert(messageSha256 === child.message_ciphertext_sha256, `${child.agent_type} encrypted spawn message_ciphertext_sha256 mismatch`);
    }
    messageInput.message_encoding = "codex-encrypted";
    messageInput.message_plaintext_verdict = "unsupported";
    if (child.message_sha256) {
      messageInput.message_plaintext_intent_sha256 = child.message_sha256;
    }
  }
  assert(!("model" in spawnArgs), `${child.agent_type} spawn manually overrode model`);
  assert(!("reasoning_effort" in spawnArgs), `${child.agent_type} spawn manually overrode reasoning_effort`);

  const outputPayload = parentRecords.find((record) => {
    const payload = record.payload;
    return record.type === "response_item" && payload?.type === "function_call_output" && payload.call_id === spawnCall.call_id;
  })?.payload;
  spawnOutput = parseJsonObject(outputPayload?.output, `spawn_agent output for ${child.agent_type}`);
  assert(spawnOutput.task_name === canonicalTask, `${child.agent_type} spawn output task_name mismatch`);

  const startedActivity = parentRecords.find((record) => {
    const payload = record.payload;
    return record.type === "event_msg"
      && payload?.type === "sub_agent_activity"
      && payload.event_id === spawnCall.call_id
      && payload.kind === "started";
  })?.payload;
  assert(startedActivity, `${child.agent_type} missing sub_agent_activity started event`);
  assert(uuidPattern.test(startedActivity.agent_thread_id), `${child.agent_type} started event missing child thread id`);
  assert(startedActivity.agent_path === canonicalTask, `${child.agent_type} started event agent_path mismatch`);
  childThreadId = startedActivity.agent_thread_id;
  observedAgentPath = startedActivity.agent_path;

  const finalMessages = parentRecords.filter((record) => {
    const payload = record.payload;
    return record.type === "response_item"
      && payload?.type === "agent_message"
      && payload.author === canonicalTask
      && payload.recipient === "/root";
  }).map((record) => record.payload);
  let finalMessage = finalMessages.find((payload) => {
    const finalText = payload.content?.map((part) => part.text ?? "").join("\n") ?? "";
    return finalText.includes("Message Type: FINAL_ANSWER")
      && (!child.completion_contains || finalText.includes(child.completion_contains));
  });
  if (!finalMessage) {
    finalMessage = parentRecords.find((record) => {
      const payload = record.payload;
      const text = payload?.content?.map((part) => part.text ?? "").join("\n") ?? "";
      return record.type === "response_item"
        && payload?.type === "message"
        && payload.role === "user"
        && text.includes("<subagent_notification>")
        && text.includes(childThreadId)
        && (!child.completion_contains || text.includes(child.completion_contains));
    })?.payload;
  }
  assert(finalMessage, `${child.agent_type} missing child FINAL_ANSWER payload in parent session`);
  const finalText = finalMessage.content?.map((part) => part.text ?? "").join("\n") ?? "";
  if (child.completion_contains) {
    assert(finalText.includes(child.completion_contains), `${child.agent_type} final answer missing ${child.completion_contains}`);
  }

  const edge = edgeRows.find((row) => row.child_thread_id === childThreadId);
  assert(edge, `${child.agent_type} missing thread_spawn_edges row`);
  assert(edge.parent_thread_id === parentThreadId, `${child.agent_type} edge parent mismatch`);
  assert(edge.status && edge.status !== "unknown", `${child.agent_type} edge has empty status`);
  if (edge.agent_path !== null) {
    assert(edge.agent_path === canonicalTask, `${child.agent_type} state agent_path mismatch`);
  }
  assert(edge.agent_role === child.agent_type, `${child.agent_type} state agent_role mismatch`);
  assert(edge.model === child.model, `${child.agent_type} effective model mismatch: expected ${child.model}, observed ${edge.model}`);
  assert(edge.reasoning_effort === child.effort, `${child.agent_type} effective effort mismatch: expected ${child.effort}, observed ${edge.reasoning_effort}`);
  assert(edge.thread_source === "subagent", `${child.agent_type} state thread_source mismatch`);
  assert(resolve(edge.cwd) === resolve(workdir), `${child.agent_type} state cwd mismatch`);

  const childSessionPath = findSession(childThreadId);
  const childRecords = readJsonl(childSessionPath);
  const childMeta = childRecords.find((record) => record.type === "session_meta")?.payload;
  assert(childMeta?.id === childThreadId, `${child.agent_type} child session id mismatch`);
  assert(childMeta?.parent_thread_id === parentThreadId, `${child.agent_type} child parent_thread_id mismatch`);
  assert(childMeta?.thread_source === "subagent", `${child.agent_type} child thread_source mismatch`);
  if (childMeta?.agent_path !== null && childMeta?.agent_path !== undefined) {
    assert(childMeta?.agent_path === canonicalTask, `${child.agent_type} child session agent_path mismatch`);
  }
  assert(childMeta?.agent_role === child.agent_type, `${child.agent_type} child session agent_role mismatch`);
  assert(childMeta?.source?.subagent?.thread_spawn?.parent_thread_id === parentThreadId, `${child.agent_type} child source parent mismatch`);
  if (childMeta?.source?.subagent?.thread_spawn?.agent_path !== null && childMeta?.source?.subagent?.thread_spawn?.agent_path !== undefined) {
    assert(childMeta?.source?.subagent?.thread_spawn?.agent_path === canonicalTask, `${child.agent_type} child source agent_path mismatch`);
  }
  assert(childMeta?.source?.subagent?.thread_spawn?.agent_role === child.agent_type, `${child.agent_type} child source agent_role mismatch`);

  const childContext = childRecords.find((record) => record.type === "turn_context")?.payload;
  assert(childContext?.model === child.model, `${child.agent_type} child turn_context model mismatch`);
  assert(childContext?.effort === child.effort, `${child.agent_type} child turn_context effort mismatch`);
  assert(childContext?.collaboration_mode?.settings?.model === child.model, `${child.agent_type} collaboration model mismatch`);
  assert(childContext?.collaboration_mode?.settings?.reasoning_effort === child.effort, `${child.agent_type} collaboration effort mismatch`);

  observed.push({
    kind: child.semantic_role,
    profile: child.profile,
    agent_type: child.agent_type,
    task_name: child.task_name,
    canonical_task: canonicalTask,
    parent_thread_id: parentThreadId,
    child_thread_id: childThreadId,
    model: edge.model,
    effort: edge.reasoning_effort,
    parent_session: basename(parentSessionPath),
    child_session: basename(childSessionPath),
    spawn: {
      surface: spawnSurface,
      agent_type: spawnArgs.agent_type,
      task_name: spawnArgs.task_name,
      fork_turns: spawnArgs.fork_turns ?? "none",
      call_id: spawnCallId,
    },
    input: messageInput,
    spawn_output: {
      task_name: spawnOutput.task_name,
      agent_id: spawnOutput.agent_id,
    },
    session: {
      agent_role: childMeta.agent_role,
      agent_path: childMeta.agent_path ?? observedAgentPath,
      thread_source: childMeta.thread_source,
      parent_thread_id: childMeta.parent_thread_id,
      session_file: basename(childSessionPath),
    },
    state: {
      agent_role: edge.agent_role,
      agent_path: edge.agent_path,
      model: edge.model,
      reasoning_effort: edge.reasoning_effort,
      thread_source: edge.thread_source,
      cwd: edge.cwd,
    },
    final_answer: {
      message_type: "FINAL_ANSWER",
    },
  });
}

assert(observed.length === expected.children.length, "not all expected children were observed");
const dispatchEvidence = observed.map((child) => ({
  schema_version: 1,
  package_digest: expected.package_digest,
  host_version: expected.host_version,
  requested_dispatch: {
    semantic_role: child.kind,
    profile: child.profile,
    model: child.model,
    effort: child.effort,
    agent_type: child.agent_type,
    fork_turns: {
      mode: child.spawn.fork_turns,
    },
    message_sha256: child.input.message_sha256,
    message_encoding: child.input.message_encoding,
    message_plaintext_verdict: child.input.message_plaintext_verdict,
    message_plaintext_intent_sha256: child.input.message_plaintext_intent_sha256,
    message_bytes: child.input.message_bytes,
    max_message_bytes: child.input.max_message_bytes,
  },
  child_identity: {
    host: "codex",
    role: child.kind,
    agent_role: child.state.agent_role,
    agent_type: child.agent_type,
    task_name: child.task_name,
  },
  effective_model: child.model,
  effective_effort: child.effort,
  nonce: `${parentThreadId}:${child.child_thread_id}:${child.spawn.call_id}`,
  raw_evidence_refs: [
    `codex-session:${child.parent_session}`,
    `codex-session:${child.child_session}`,
    `state_5.sqlite:thread_spawn_edges:${parentThreadId}:${child.child_thread_id}`,
    `spawn_call:${child.spawn.call_id}`,
  ],
  verdict: "deterministic",
}));
console.log(JSON.stringify({
  schema_version: "switchloom.codex_runtime_evidence.v1",
  run: {
    status: "complete",
    complete_marker: "SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE",
    evidence_source: "codex_persisted_spawn_state",
    parent_thread_id: parentThreadId,
    parent_session: basename(parentSessionPath),
    workdir: resolve(workdir),
  },
  children: observed,
  dispatch_evidence: dispatchEvidence,
}, null, 2));
