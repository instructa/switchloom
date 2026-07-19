#!/usr/bin/env node
import { spawnSync } from "node:child_process";
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

function outputText(value) {
  if (typeof value === "string") return value;
  if (Array.isArray(value)) {
    return value.map((part) => part?.text ?? "").join("\n");
  }
  return "";
}

function parseToolJsonOutput(value, label) {
  const text = outputText(value).trim();
  try {
    return JSON.parse(text);
  } catch {
    const match = text.match(/\{[\s\S]*\}\s*$/);
    assert(match, `${label} did not contain JSON output`);
    try {
      return JSON.parse(match[0]);
    } catch (error) {
      fail(`${label} is not valid JSON: ${error.message}`);
    }
  }
}

function jsStringField(source, field) {
  const escaped = field.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = source.match(new RegExp(`${escaped}\\s*:\\s*["']([^"']+)["']`));
  return match?.[1];
}

function jsBooleanField(source, field) {
  const escaped = field.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = source.match(new RegExp(`${escaped}\\s*:\\s*(true|false)`));
  return match ? match[1] === "true" : undefined;
}

function sqliteJson(query) {
  assert(existsSync(stateDb), `Codex state DB not found at ${stateDb}`);
  const result = spawnSync("sqlite3", ["-json", stateDb, query], { encoding: "utf8" });
  assert(result.status === 0, `sqlite3 query failed: ${result.stderr || result.stdout}`);
  return result.stdout.trim() ? JSON.parse(result.stdout) : [];
}

const expected = JSON.parse(readFileSync(expectPath, "utf8"));
assert(Array.isArray(expected.children) && expected.children.length > 0, "expected children list is empty");

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

const observed = [];
for (const child of expected.children) {
  const canonicalTask = child.canonical_task;
  assert(typeof child.agent_type === "string" && child.agent_type.length > 0, "expected child missing agent_type");
  assert(typeof child.task_name === "string" && child.task_name.length > 0, "expected child missing task_name");
  assert(typeof canonicalTask === "string" && canonicalTask.startsWith("/root/"), `${child.agent_type} has invalid canonical_task`);

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
  assert(
    legacySpawnCalls.length + v1SpawnCalls.length === 1,
    `${child.agent_type} must have exactly one raw spawn_agent call`,
  );

  let spawnArgs;
  let spawnOutput;
  let childThreadId;
  let observedAgentPath;
  let spawnCallId;
  let spawnSurface;
  if (legacySpawnCalls.length === 1) {
    const spawnCall = legacySpawnCalls[0].payload;
    spawnCallId = spawnCall.call_id;
    spawnSurface = "collaboration.spawn_agent";
    spawnArgs = parseJsonObject(spawnCall.arguments, `spawn_agent arguments for ${child.agent_type}`);
    assert(spawnArgs.fork_turns === "none", `${child.agent_type} spawn did not use fork_turns=none`);
    assert(spawnArgs.task_name === child.task_name, `${child.agent_type} spawn task_name mismatch`);
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
  } else {
    const spawnCall = v1SpawnCalls[0].payload;
    spawnCallId = spawnCall.call_id;
    spawnSurface = "multi_agent_v1__spawn_agent";
    spawnArgs = {
      agent_type: jsStringField(spawnCall.input, "agent_type"),
      task_name: jsStringField(spawnCall.input, "task_name"),
      fork_turns: jsStringField(spawnCall.input, "fork_turns"),
      fork_context: jsBooleanField(spawnCall.input, "fork_context"),
    };
    assert(spawnArgs.agent_type === child.agent_type, `${child.agent_type} v1 spawn agent_type mismatch`);
    assert(spawnArgs.task_name === child.task_name, `${child.agent_type} v1 spawn task_name mismatch`);
    assert(spawnArgs.fork_turns === "none" || spawnArgs.fork_context !== true, `${child.agent_type} v1 spawn inherited context`);
    assert(!/\bmodel\s*:/.test(spawnCall.input), `${child.agent_type} v1 spawn manually overrode model`);
    assert(!/\breasoning_effort\s*:/.test(spawnCall.input), `${child.agent_type} v1 spawn manually overrode reasoning_effort`);

    const outputPayload = parentRecords.find((record) => {
      const payload = record.payload;
      return record.type === "response_item" && payload?.type === "custom_tool_call_output" && payload.call_id === spawnCall.call_id;
    })?.payload;
    spawnOutput = parseToolJsonOutput(outputPayload?.output, `v1 spawn_agent output for ${child.agent_type}`);
    assert(uuidPattern.test(spawnOutput.agent_id), `${child.agent_type} v1 spawn output missing child thread id`);
    childThreadId = spawnOutput.agent_id;
    observedAgentPath = null;

    const startedEvent = outerEvents.find((record) => {
      const item = record.item;
      return record.type === "item.completed"
        && item?.type === "collab_tool_call"
        && item.tool === "spawn_agent"
        && Array.isArray(item.receiver_thread_ids)
        && item.receiver_thread_ids.includes(childThreadId);
    });
    assert(startedEvent, `${child.agent_type} missing v1 spawn event for child thread`);
  }

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
    kind: child.kind,
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
}, null, 2));
