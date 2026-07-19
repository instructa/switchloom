import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const parent = "11111111-1111-4111-8111-111111111111";
const worker = "22222222-2222-4222-8222-222222222222";
const reviewer = "33333333-3333-4333-8333-333333333333";
const workdir = "/tmp/switchloom-fresh-repo";

function child(kind, agentType, taskName, childThreadId, model, effort) {
  return {
    kind,
    agent_type: agentType,
    task_name: taskName,
    canonical_task: `/root/${taskName}`,
    parent_thread_id: parent,
    child_thread_id: childThreadId,
    spawn: {
      agent_type: agentType,
      task_name: taskName,
      fork_turns: "none",
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
    children: [
      child("worker", "model_routing_terra_high", "worker", worker, "gpt-5.6-terra", "high"),
      child("reviewer", "model_routing_sol_high", "reviewer", reviewer, "gpt-5.6-sol", "high"),
    ],
  };
}

async function withReceipt(receipt, callback) {
  const root = await mkdtemp(join(tmpdir(), "switchloom-codex-evidence-"));
  try {
    const path = join(root, "receipt.json");
    await writeFile(path, typeof receipt === "string" ? receipt : JSON.stringify(receipt, null, 2));
    return callback(path);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

function run(path) {
  return spawnSync(process.execPath, ["scripts/validate-codex-runtime-evidence.mjs", path], {
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
