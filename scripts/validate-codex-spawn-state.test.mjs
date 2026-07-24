import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const parent = "11111111-1111-4111-8111-111111111111";
const worker = "22222222-2222-4222-8222-222222222222";
const reviewer = "33333333-3333-4333-8333-333333333333";
const extra = "44444444-4444-4444-8444-444444444444";
const packageDigest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const hostVersion = "codex-cli 0.145.0";
const encryptedWorkerMessage = "gAAAAABqEncryptedWorkerMessageForSwitchloom1234567890";
const changedEncryptedWorkerMessage = "gAAAAABqChangedWorkerMessageForSwitchloom1234567890";

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function line(record) {
  return `${JSON.stringify(record)}\n`;
}

function childSpec(kind, agentType, taskName, threadId, model, effort, message) {
  return {
    semantic_role: kind,
    profile: kind === "worker" ? "codex-terra-high" : "codex-sol-high",
    kind,
    agent_type: agentType,
    task_name: taskName,
    canonical_task: `/root/${taskName}`,
    model,
    effort,
    message_sha256: sha256(message),
    max_message_bytes: 512,
    completion_contains: `SWITCHLOOM_${kind.toUpperCase()}_DONE`,
    thread_id: threadId,
    message,
  };
}

function baseChildren() {
  return [
    childSpec("worker", "model_routing_terra_high", "worker", worker, "gpt-5.6-terra", "high", "worker bounded task input"),
    childSpec("reviewer", "model_routing_sol_high", "reviewer", reviewer, "gpt-5.6-sol", "high", "reviewer bounded task input"),
  ];
}

async function writeFixture(overrides = {}) {
  const root = await mkdtemp(join(tmpdir(), "switchloom-codex-spawn-state-"));
  const workdir = join(root, "workdir");
  const sessions = join(root, "sessions");
  await mkdir(sessions, { recursive: true });
  await mkdir(workdir, { recursive: true });
  const stateDb = join(root, "state_5.sqlite");
  const eventsPath = join(root, "events.jsonl");
  const expectPath = join(root, "expected.json");

  const children = baseChildren();
  const expected = {
    package_digest: packageDigest,
    host_version: hostVersion,
    children: children.map(({ thread_id, message, ...child }) => ({
      ...child,
      max_message_bytes: overrides.smallMaxFor === child.kind ? 3 : child.max_message_bytes,
      ...(overrides.encryptedMessageFor === child.kind
        ? { message_ciphertext_sha256: sha256(encryptedWorkerMessage) }
        : {}),
      ...(overrides.allowEncryptedMessageFor === child.kind
        ? { allow_encrypted_message: true }
        : {}),
    })),
  };
  await writeFile(expectPath, JSON.stringify(expected, null, 2));
  await writeFile(eventsPath, line({ type: "thread.started", thread_id: parent }));

  const parentRecords = [
    { type: "session_meta", payload: { id: parent, thread_source: "user", cwd: workdir } },
  ];
  const dbRows = [];
  for (const child of children) {
    const callId = `call-${child.kind}`;
    let message = overrides.changedMessageFor === child.kind ? `${child.message} changed` : child.message;
    if (overrides.encryptedMessageFor === child.kind) {
      message = overrides.changedEncryptedMessageFor === child.kind
        ? changedEncryptedWorkerMessage
        : encryptedWorkerMessage;
    }
    const args = {
      agent_type: child.agent_type,
      task_name: child.task_name,
      fork_turns: "none",
    };
    if (overrides.omitMessageFor !== child.kind) {
      args.message = message;
    }
    parentRecords.push({
      type: "response_item",
      payload: {
        type: "function_call",
        namespace: "collaboration",
        name: "spawn_agent",
        call_id: callId,
        arguments: JSON.stringify(args),
      },
    });
    parentRecords.push({
      type: "response_item",
      payload: {
        type: "function_call_output",
        call_id: callId,
        output: JSON.stringify({ task_name: child.canonical_task }),
      },
    });
    parentRecords.push({
      type: "event_msg",
      payload: {
        type: "sub_agent_activity",
        event_id: callId,
        kind: "started",
        agent_thread_id: child.thread_id,
        agent_path: child.canonical_task,
      },
    });
    parentRecords.push({
      type: "response_item",
      payload: {
        type: "agent_message",
        author: child.canonical_task,
        recipient: "/root",
        content: [{ text: `Message Type: FINAL_ANSWER\n${child.completion_contains}` }],
      },
    });
    await writeFile(join(sessions, `${child.thread_id}.jsonl`), [
      line({
        type: "session_meta",
        payload: {
          id: child.thread_id,
          parent_thread_id: parent,
          thread_source: "subagent",
          agent_path: child.canonical_task,
          agent_role: child.agent_type,
          source: {
            subagent: {
              thread_spawn: {
                parent_thread_id: parent,
                agent_path: child.canonical_task,
                agent_role: child.agent_type,
              },
            },
          },
        },
      }),
      line({
        type: "turn_context",
        payload: {
          model: child.model,
          effort: child.effort,
          collaboration_mode: {
            settings: {
              model: child.model,
              reasoning_effort: child.effort,
            },
          },
        },
      }),
    ].join(""));
    dbRows.push({
      id: child.thread_id,
      agent_path: child.canonical_task,
      agent_role: child.agent_type,
      model: child.model,
      reasoning_effort: child.effort,
      thread_source: "subagent",
      cwd: workdir,
    });
  }

  if (overrides.extraChild) {
    parentRecords.push({
      type: "response_item",
      payload: {
        type: "function_call",
        namespace: "collaboration",
        name: "spawn_agent",
        call_id: "call-extra",
        arguments: JSON.stringify({
          agent_type: "model_routing_luna_xhigh",
          task_name: "unrelated",
          fork_turns: "none",
          message: "unrelated task",
        }),
      },
    });
    parentRecords.push({
      type: "event_msg",
      payload: {
        type: "sub_agent_activity",
        event_id: "call-extra",
        kind: "started",
        agent_thread_id: extra,
        agent_path: "/root/unrelated",
      },
    });
    dbRows.push({
      id: extra,
      agent_path: "/root/unrelated",
      agent_role: "model_routing_luna_xhigh",
      model: "gpt-5.6-luna",
      reasoning_effort: "xhigh",
      thread_source: "subagent",
      cwd: workdir,
    });
  }

  await writeFile(join(sessions, `${parent}.jsonl`), parentRecords.map(line).join(""));

  const schema = [
    "create table thread_spawn_edges(parent_thread_id text, child_thread_id text, status text);",
    "create table threads(id text, agent_path text, agent_role text, model text, reasoning_effort text, thread_source text, cwd text);",
  ];
  const inserts = [];
  for (const row of dbRows) {
    inserts.push(`insert into thread_spawn_edges values('${parent}','${row.id}','completed');`);
    inserts.push(`insert into threads values('${row.id}','${row.agent_path}','${row.agent_role}','${row.model}','${row.reasoning_effort}','${row.thread_source}','${row.cwd}');`);
  }
  const sqlite = spawnSync("sqlite3", [stateDb], {
    input: [...schema, ...inserts].join("\n"),
    encoding: "utf8",
  });
  assert.equal(sqlite.status, 0, sqlite.stderr);

  return { root, workdir, eventsPath, expectPath, sessions, stateDb };
}

function run(fixture) {
  return spawnSync(process.execPath, [
    "scripts/validate-codex-spawn-state.mjs",
    "--events", fixture.eventsPath,
    "--workdir", fixture.workdir,
    "--expect", fixture.expectPath,
    "--state-db", fixture.stateDb,
    "--sessions-dir", fixture.sessions,
  ], {
    cwd: new URL("..", import.meta.url),
    encoding: "utf8",
  });
}

async function withFixture(overrides, callback) {
  const fixture = await writeFixture(overrides);
  try {
    return await callback(fixture);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
}

test("extracts strict V2 spawn state with bounded task input", async () => {
  await withFixture({}, (fixture) => {
    const result = run(fixture);
    assert.equal(result.status, 0, result.stderr);
    const receipt = JSON.parse(result.stdout);
    assert.equal(receipt.children[0].input.message_sha256, sha256("worker bounded task input"));
    assert.equal(receipt.dispatch_evidence[0].requested_dispatch.max_message_bytes, 512);
  });
});

test("rejects unrelated extra children", async () => {
  await withFixture({ extraChild: true }, (fixture) => {
    const result = run(fixture);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /exactly 2 V2 spawn_agent calls/);
  });
});

test("rejects missing or changed bounded task input", async () => {
  await withFixture({ omitMessageFor: "worker" }, (fixture) => {
    const result = run(fixture);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /spawn message missing/);
  });

  await withFixture({ changedMessageFor: "worker" }, (fixture) => {
    const result = run(fixture);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /message_sha256 mismatch/);
  });

  await withFixture({ smallMaxFor: "worker" }, (fixture) => {
    const result = run(fixture);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /exceeds max_message_bytes/);
  });
});

test("rejects substituted encrypted bounded task input", async () => {
  await withFixture({ encryptedMessageFor: "worker" }, (fixture) => {
    const result = run(fixture);
    assert.equal(result.status, 0, result.stderr);
    const receipt = JSON.parse(result.stdout);
    assert.equal(receipt.children[0].input.message_encoding, "codex-encrypted");
    assert.equal(receipt.children[0].input.message_plaintext_verdict, "unsupported");
  });

  await withFixture({ encryptedMessageFor: "worker", changedEncryptedMessageFor: "worker" }, (fixture) => {
    const result = run(fixture);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /message_ciphertext_sha256 mismatch/);
  });
});
