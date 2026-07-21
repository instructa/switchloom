import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const parent = "11111111-1111-4111-8111-111111111111";
const worker = "22222222-2222-4222-8222-222222222222";
const reviewer = "33333333-3333-4333-8333-333333333333";
const workdir = "/tmp/switchloom-fresh-repo";
const packageDigest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const hostVersion = "codex 0.144.0";

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function profileFor(kind) {
  return kind === "worker" ? "codex-terra-high" : "codex-sol-high";
}

function child(kind, agentType, taskName, childThreadId, model, effort, profile = profileFor(kind)) {
  const callId = `call-${kind}`;
  const message = `${kind} bounded task input`;
  return {
    kind,
    profile,
    agent_type: agentType,
    task_name: taskName,
    canonical_task: `/root/${taskName}`,
    parent_thread_id: parent,
    child_thread_id: childThreadId,
    spawn: {
      surface: "collaboration.spawn_agent",
      agent_type: agentType,
      task_name: taskName,
      fork_turns: "none",
      call_id: callId,
    },
    input: {
      message_sha256: sha256(message),
      message_bytes: Buffer.byteLength(message, "utf8"),
      max_message_bytes: 512,
      message_encoding: "plaintext",
      message_plaintext_verdict: "deterministic",
    },
    spawn_output: {
      task_name: `/root/${taskName}`,
    },
    session: {
      agent_role: agentType,
      agent_path: `/root/${taskName}`,
      thread_source: "subagent",
      parent_thread_id: parent,
      session_file: `${childThreadId}.jsonl`,
    },
    state: {
      agent_role: agentType,
      agent_path: `/root/${taskName}`,
      model,
      reasoning_effort: effort,
      thread_source: "subagent",
      cwd: workdir,
    },
    final_answer: {
      message_type: "FINAL_ANSWER",
    },
  };
}

function validReceipt() {
  const children = [
    child("worker", "model_routing_terra_high", "worker", worker, "gpt-5.6-terra", "high"),
    child("reviewer", "model_routing_sol_high", "reviewer", reviewer, "gpt-5.6-sol", "high"),
  ];
  return {
    schema_version: "switchloom.codex_runtime_evidence.v1",
    run: {
      status: "complete",
      complete_marker: "SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE",
      evidence_source: "codex_persisted_spawn_state",
      parent_thread_id: parent,
      parent_session: `${parent}.jsonl`,
      workdir,
    },
    children,
    dispatch_evidence: children.map((entry) => ({
      schema_version: 1,
      package_digest: packageDigest,
      host_version: hostVersion,
      requested_dispatch: {
        semantic_role: entry.kind,
        profile: entry.profile,
        model: entry.state.model,
        effort: entry.state.reasoning_effort,
        agent_type: entry.agent_type,
        fork_turns: {
          mode: "none",
        },
        message_sha256: entry.input.message_sha256,
        message_encoding: entry.input.message_encoding ?? "plaintext",
        message_plaintext_verdict: entry.input.message_plaintext_verdict ?? "deterministic",
        message_plaintext_intent_sha256: entry.input.message_plaintext_intent_sha256,
        message_bytes: entry.input.message_bytes,
        max_message_bytes: entry.input.max_message_bytes,
      },
      child_identity: {
        host: "codex",
        role: entry.kind,
        agent_role: entry.agent_type,
        agent_type: entry.agent_type,
        task_name: entry.task_name,
      },
      effective_model: entry.state.model,
      effective_effort: entry.state.reasoning_effort,
      nonce: `${parent}:${entry.child_thread_id}:${entry.spawn.call_id}`,
      raw_evidence_refs: [
        `codex-session:${parent}.jsonl`,
        `codex-session:${entry.child_thread_id}.jsonl`,
        `state_5.sqlite:thread_spawn_edges:${parent}:${entry.child_thread_id}`,
        `spawn_call:${entry.spawn.call_id}`,
      ],
      verdict: "deterministic",
    })),
  };
}

async function withReceipt(receipt, callback) {
  const root = await mkdtemp(join(tmpdir(), "switchloom-codex-evidence-"));
  try {
    const path = join(root, "receipt.json");
    await writeFile(path, typeof receipt === "string" ? receipt : JSON.stringify(receipt, null, 2));
    return await callback(path);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

function run(path) {
  const validator = process.env.VALIDATOR_MODE === "rust"
    ? new URL("../../../scripts/validate-codex-runtime-evidence.mjs", import.meta.url).pathname
    : new URL("./validate-codex-runtime-evidence.mjs", import.meta.url).pathname;
  return spawnSync(process.execPath, [validator, path], {
    cwd: new URL("..", import.meta.url),
    encoding: "utf8",
  });
}

function runWithExpect(path, expectPath) {
  const validator = process.env.VALIDATOR_MODE === "rust"
    ? new URL("../../../scripts/validate-codex-runtime-evidence.mjs", import.meta.url).pathname
    : new URL("./validate-codex-runtime-evidence.mjs", import.meta.url).pathname;
  return spawnSync(process.execPath, [validator, path, "--expect", expectPath], {
    cwd: new URL("..", import.meta.url),
    encoding: "utf8",
  });
}

test("accepts complete correlated Codex runtime evidence", async () => {
  await withReceipt(validReceipt(), (path) => {
    const result = run(path);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /runtime evidence validation passed/);
  });
});

test("rejects prose-only and incomplete evidence", async () => {
  await withReceipt("The worker used Terra High and the reviewer used Sol High.", (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /JSON|Unexpected token|not valid/i);
  });

  const incomplete = validReceipt();
  delete incomplete.run.complete_marker;
  await withReceipt(incomplete, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /complete marker missing/);
  });
});

test("rejects missing persisted source and uncorrelated child state", async () => {
  const synthetic = validReceipt();
  delete synthetic.run.evidence_source;
  await withReceipt(synthetic, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /evidence_source/);
  });

  const uncorrelated = validReceipt();
  uncorrelated.children[0].session.parent_thread_id = reviewer;
  await withReceipt(uncorrelated, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /session parent mismatch/);
  });
});

test("rejects inherited Sol Medium behavior", async () => {
  const inherited = validReceipt();
  inherited.children[1].state.model = "gpt-5.6-sol";
  inherited.children[1].state.reasoning_effort = "medium";
  await withReceipt(inherited, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /effective effort mismatch|inherited Sol Medium/);
  });
});

test("rejects receipts without correlated dispatch evidence", async () => {
  const missing = validReceipt();
  delete missing.dispatch_evidence;
  await withReceipt(missing, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /dispatch_evidence/);
  });

  const uncorrelated = validReceipt();
  uncorrelated.dispatch_evidence[0].child_identity.agent_role = "model_routing_sol_medium";
  await withReceipt(uncorrelated, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /agent_role mismatch/);
  });

  const missingNonce = validReceipt();
  missingNonce.dispatch_evidence[0].nonce = "";
  await withReceipt(missingNonce, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /nonce/);
  });

  const staleNonce = validReceipt();
  staleNonce.dispatch_evidence[0].nonce = `${parent}:${worker}:stale-call`;
  await withReceipt(staleNonce, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /nonce must bind parent thread, child thread, and spawn call/);
  });

  const echoedNonce = validReceipt();
  echoedNonce.dispatch_evidence[0].nonce = "nonce-123";
  await withReceipt(echoedNonce, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /nonce must bind parent thread, child thread, and spawn call/);
  });
});

test("rejects placeholder package and host provenance", async () => {
  const placeholder = validReceipt();
  placeholder.dispatch_evidence[0].package_digest = `codex-runtime:${parent}`;
  await withReceipt(placeholder, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /package_digest/);
  });

  const proseVersion = validReceipt();
  proseVersion.dispatch_evidence[0].host_version = "codex native persisted spawn state";
  await withReceipt(proseVersion, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /host_version/);
  });
});

test("rejects non-V2 spawn surface, direct override, and mismatched task identity", async () => {
  const external = validReceipt();
  external.children[0].spawn.surface = "multi_agent_v1__spawn_agent";
  await withReceipt(external, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /spawn surface/);
  });

  const directOverride = validReceipt();
  directOverride.children[0].spawn.model = "gpt-5.6-sol";
  await withReceipt(directOverride, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /manually override model/);
  });

  const mismatchedTask = validReceipt();
  mismatchedTask.children[0].task_name = "unrelated_worker";
  await withReceipt(mismatchedTask, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /canonical_task mismatch|task_name mismatch/);
  });
});

test("rejects missing, changed, and oversized bounded task input evidence", async () => {
  const missing = validReceipt();
  delete missing.children[0].input;
  await withReceipt(missing, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /expected message hash|expected message_sha256|input message_sha256/);
  });

  const changed = validReceipt();
  changed.children[0].input.message_sha256 = sha256("different message");
  await withReceipt(changed, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /message_sha256 mismatch/);
  });

  const oversized = validReceipt();
  oversized.children[0].input.message_bytes = oversized.children[0].input.max_message_bytes + 1;
  oversized.dispatch_evidence[0].requested_dispatch.message_bytes = oversized.children[0].input.message_bytes;
  await withReceipt(oversized, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /exceeds max_message_bytes/);
  });
});

test("rejects raw refs that do not bind parent and child Codex sessions", async () => {
  const missingChildSession = validReceipt();
  missingChildSession.dispatch_evidence[0].raw_evidence_refs = [
    `codex-session:${parent}.jsonl`,
    `state_5.sqlite:thread_spawn_edges:${parent}:${worker}`,
    "spawn_call:call-worker",
  ];
  await withReceipt(missingChildSession, (path) => {
    const result = run(path);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /raw evidence refs must bind/);
  });
});

test("accepts custom role receipts with explicit expected semantic roles and profiles", async () => {
  const implementer = child("implementer", "switchloom_implementer", "implementer", worker, "gpt-5.6-terra", "high", "switchloom_implementer");
  const customReviewer = child("reviewer", "switchloom_reviewer", "reviewer", reviewer, "gpt-5.6-sol", "high", "switchloom_reviewer");
  const custom = validReceipt();
  custom.children = [implementer, customReviewer];
  custom.dispatch_evidence = custom.children.map((entry) => ({
    schema_version: 1,
    package_digest: packageDigest,
    host_version: hostVersion,
    requested_dispatch: {
      semantic_role: entry.kind,
      profile: entry.profile,
      model: entry.state.model,
      effort: entry.state.reasoning_effort,
    agent_type: entry.agent_type,
    fork_turns: { mode: "none" },
    message_sha256: entry.input.message_sha256,
    message_encoding: entry.input.message_encoding ?? "plaintext",
    message_plaintext_verdict: entry.input.message_plaintext_verdict ?? "deterministic",
    message_plaintext_intent_sha256: entry.input.message_plaintext_intent_sha256,
    message_bytes: entry.input.message_bytes,
    max_message_bytes: entry.input.max_message_bytes,
  },
    child_identity: {
      host: "codex",
      role: entry.kind,
      agent_role: entry.agent_type,
      agent_type: entry.agent_type,
      task_name: entry.task_name,
    },
    effective_model: entry.state.model,
    effective_effort: entry.state.reasoning_effort,
    nonce: `${parent}:${entry.child_thread_id}:${entry.spawn.call_id}`,
    raw_evidence_refs: [
      `codex-session:${parent}.jsonl`,
      `codex-session:${entry.child_thread_id}.jsonl`,
      `state_5.sqlite:thread_spawn_edges:${parent}:${entry.child_thread_id}`,
      `spawn_call:${entry.spawn.call_id}`,
    ],
    verdict: "deterministic",
  }));

  const expected = {
    package_digest: packageDigest,
    host_version: hostVersion,
    children: [
      {
        semantic_role: "implementer",
        profile: "switchloom_implementer",
        agent_type: "switchloom_implementer",
        task_name: "implementer",
        message_sha256: implementer.input.message_sha256,
        max_message_bytes: implementer.input.max_message_bytes,
        model: "gpt-5.6-terra",
        effort: "high",
      },
      {
        semantic_role: "reviewer",
        profile: "switchloom_reviewer",
        agent_type: "switchloom_reviewer",
        task_name: "reviewer",
        message_sha256: customReviewer.input.message_sha256,
        max_message_bytes: customReviewer.input.max_message_bytes,
        model: "gpt-5.6-sol",
        effort: "high",
      },
    ],
  };
  await withReceipt(custom, async (path) => {
    await withReceipt(expected, (expectPath) => {
      const result = runWithExpect(path, expectPath);
      assert.equal(result.status, 0, result.stderr);
      assert.match(result.stdout, /runtime evidence validation passed/);
    });
  });
});
