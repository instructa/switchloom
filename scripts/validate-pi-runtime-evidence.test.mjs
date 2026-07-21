import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { test } from "node:test";

const script = "scripts/validate-pi-runtime-evidence.mjs";

function promptHash(nonce) {
  return `sha256:${createHash("sha256").update(`Return only this nonce and no other text: ${nonce}`).digest("hex")}`;
}

function fixture(name, overrides = {}) {
  const dir = mkdtempSync(join(tmpdir(), `pi-evidence-${name}-`));
  mkdirSync(dir, { recursive: true });
  const workflow = {
    schema_version: 1,
    workflow: "model-routing-preset-runner",
    runner: "pi",
    runtime_class: "external-runner",
    arguments: {
      agent_type: overrides.agentType ?? "switchloom-pi-worker",
      provider_model: overrides.model ?? "openai/gpt-4o-mini",
      thinking: overrides.thinking ?? "low",
      isolation: {
        session: overrides.session ?? "none",
        tools: overrides.tools ?? "none",
        extensions: "none",
        skills: "none",
        agent_dir: "report-workdir/.pi-agent",
      },
      task: {
        semantic_role: "worker",
        profile: "pi-worker",
        returns: overrides.returns ?? "nonce-only",
      },
    },
    process: {
      argv: ["pi", "--print", "--no-session", "--no-tools", "--no-extensions", "--no-skills", "--provider", "openai", "--model", "gpt-4o-mini", "--thinking", "low"],
      state_boundary: "PI_CODING_AGENT_DIR is set to a report-local directory for every certification run",
    },
    security: {
      filesystem_tools: "disabled",
      session_persistence: "disabled",
      native_subagents: "not used",
    },
  };
  const invocation = {
    nonce: "nonce-123",
    argv: overrides.invocationArgv ?? ["env", "PI_CODING_AGENT_DIR=.pi-agent", "PI_OFFLINE=1", ...workflow.process.argv],
    env: overrides.invocationEnv ?? { PI_CODING_AGENT_DIR: ".pi-agent", PI_OFFLINE: "1" },
    prompt_sha256: overrides.promptSha256 ?? promptHash("nonce-123"),
  };
  writeFileSync(join(dir, "workflow.json"), JSON.stringify(workflow, null, 2));
  writeFileSync(join(dir, "requested-invocation.json"), JSON.stringify(invocation, null, 2));
  writeFileSync(join(dir, "host-output.txt"), `${overrides.stdout ?? "nonce-123"}\n`);
  writeFileSync(join(dir, "host-output.stderr"), overrides.stderr ?? "");
  return dir;
}

function run(dir, extraArgs = []) {
  return spawnSync(process.execPath, [
    script,
    "--workflow", join(dir, "workflow.json"),
    "--invocation", join(dir, "requested-invocation.json"),
    "--stdout", join(dir, "host-output.txt"),
    "--stderr", join(dir, "host-output.stderr"),
    "--workflow-receipt", join(dir, "workflow-receipt.json"),
    "--dispatch-receipt", join(dir, "dispatch-evidence.json"),
    "--package-digest", "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "--host-version", "0.66.1",
    "--profile", "pi-worker",
    "--model", "openai/gpt-4o-mini",
    "--thinking", "low",
    "--agent-type", "switchloom-pi-worker",
    ...extraArgs,
  ], { encoding: "utf8" });
}

test("accepts a nonce-only Pi workflow receipt", () => {
  const dir = fixture("valid");
  const result = run(dir);
  assert.equal(result.status, 0, result.stderr);
  const workflowReceipt = JSON.parse(readFileSync(join(dir, "workflow-receipt.json"), "utf8"));
  const dispatchEvidence = JSON.parse(readFileSync(join(dir, "dispatch-evidence.json"), "utf8"));
  assert.equal(workflowReceipt.observed.nonce, "nonce-123");
  assert.equal(workflowReceipt.requested.isolation.tools, "none");
  assert.equal(dispatchEvidence.child_identity.host, "pi");
  assert.equal(dispatchEvidence.requested_dispatch.agent_type, "switchloom-pi-worker");
  assert.equal(dispatchEvidence.verdict, "advisory");
});

test("rejects non-nonce Pi output", () => {
  const dir = fixture("bad-output", { stdout: "nonce-123 plus explanation" });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /did not exactly return nonce/);
});

test("rejects workflows that enable tools", () => {
  const dir = fixture("tools", { tools: "default" });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /disable tools/);
});

test("rejects mismatched agent type", () => {
  const dir = fixture("agent", { agentType: "switchloom-pi-driver" });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /agent_type/);
});

test("rejects workflows without nonce-only task contract", () => {
  const dir = fixture("task", { returns: "summary" });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /nonce-only/);
});

test("rejects replayed invocation argv that changes provider, model, and isolation", () => {
  const dir = fixture("replay", {
    invocationArgv: [
      "env",
      "PI_CODING_AGENT_DIR=/tmp/shared-agent",
      "PI_OFFLINE=1",
      "pi",
      "--print",
      "--provider",
      "anthropic",
      "--model",
      "claude-opus-4-5",
    ],
    invocationEnv: { PI_CODING_AGENT_DIR: "/tmp/shared-agent", PI_OFFLINE: "1" },
  });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /argv does not match workflow process argv|PI_CODING_AGENT_DIR/);
});

test("rejects invocation argv missing thinking", () => {
  const dir = fixture("missing-thinking", {
    invocationArgv: ["env", "PI_CODING_AGENT_DIR=.pi-agent", "PI_OFFLINE=1", "pi", "--print", "--no-session", "--no-tools", "--no-extensions", "--no-skills", "--provider", "openai", "--model", "gpt-4o-mini"],
  });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /argv does not match workflow process argv|thinking/);
});

test("rejects invocation env outside the report-local Pi boundary", () => {
  const workflowArgv = ["pi", "--print", "--no-session", "--no-tools", "--no-extensions", "--no-skills", "--provider", "openai", "--model", "gpt-4o-mini", "--thinking", "low"];
  const dir = fixture("bad-env", {
    invocationArgv: ["env", "PI_CODING_AGENT_DIR=.pi-agent", "PI_OFFLINE=1", ...workflowArgv],
    invocationEnv: { PI_CODING_AGENT_DIR: "/tmp/shared-agent", PI_OFFLINE: "1" },
  });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /PI_CODING_AGENT_DIR/);
});

test("rejects prompt hashes that do not bind to the nonce task", () => {
  const dir = fixture("bad-prompt", {
    promptSha256: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  });
  const result = run(dir);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /prompt hash/);
});
