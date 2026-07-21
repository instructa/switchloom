#!/usr/bin/env node
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";

function usage() {
  console.error("usage: validate-pi-runtime-evidence.mjs --workflow <workflow.json> --invocation <requested-invocation.json> --stdout <host-output.txt> --stderr <host-output.stderr> --workflow-receipt <workflow-receipt.json> --dispatch-receipt <dispatch-evidence.json> --package-digest <sha256:...> --host-version <version> --profile <profile> --model <provider/model> --thinking <level> --agent-type <agent>");
  process.exit(2);
}

function arg(name) {
  const index = process.argv.indexOf(`--${name}`);
  if (index === -1 || index + 1 >= process.argv.length) usage();
  return process.argv[index + 1];
}

function readJson(path, label) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    throw new Error(`${label} is not valid JSON: ${error.message}`);
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function arraysEqual(left, right) {
  return Array.isArray(left) && Array.isArray(right) && left.length === right.length && left.every((value, index) => value === right[index]);
}

function expectedInvocationArgv(workflowArgv) {
  return ["env", "PI_CODING_AGENT_DIR=.pi-agent", "PI_OFFLINE=1", ...workflowArgv];
}

function optionValue(argv, option) {
  const index = argv.indexOf(option);
  if (index === -1 || index + 1 >= argv.length) return null;
  return argv[index + 1];
}

function sha256Text(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

const workflowPath = arg("workflow");
const invocationPath = arg("invocation");
const stdoutPath = arg("stdout");
const stderrPath = arg("stderr");
const workflowReceiptPath = arg("workflow-receipt");
const dispatchReceiptPath = arg("dispatch-receipt");
const packageDigest = arg("package-digest");
const hostVersion = arg("host-version");
const profile = arg("profile");
const requestedModel = arg("model");
const requestedThinking = arg("thinking");
const agentType = arg("agent-type");

const workflow = readJson(workflowPath, "workflow");
const invocation = readJson(invocationPath, "requested invocation");
const stdout = readFileSync(stdoutPath, "utf8").trim();
readFileSync(stderrPath, "utf8");

const nonce = invocation.nonce;
assert(typeof nonce === "string" && nonce.length > 0, "requested invocation must include nonce");
assert(workflow.schema_version === 1, "workflow schema_version must be 1");
assert(workflow.runner === "pi", "workflow runner must be pi");
assert(workflow.runtime_class === "external-runner", "workflow runtime_class must be external-runner");

const args = workflow.arguments;
assert(args && typeof args === "object", "workflow must include typed arguments");
assert(args.agent_type === agentType, `workflow agent_type ${args.agent_type} does not match ${agentType}`);
assert(args.provider_model === requestedModel, `workflow provider_model ${args.provider_model} does not match ${requestedModel}`);
assert(args.thinking === requestedThinking, `workflow thinking ${args.thinking} does not match ${requestedThinking}`);
assert(args.task?.semantic_role === "worker", "workflow task semantic_role must be worker");
assert(args.task?.returns === "nonce-only", "workflow task must require nonce-only return");
assert(args.isolation?.session === "none", "workflow isolation must disable session persistence");
assert(args.isolation?.tools === "none", "workflow isolation must disable tools");
assert(args.isolation?.extensions === "none", "workflow isolation must disable extensions");
assert(args.isolation?.skills === "none", "workflow isolation must disable skills");
assert(Array.isArray(workflow.process?.argv), "workflow process argv must be recorded");
for (const required of ["--print", "--no-session", "--no-tools", "--no-extensions", "--no-skills", "--provider", "--model", "--thinking"]) {
  assert(workflow.process.argv.includes(required), `workflow process argv must include ${required}`);
}
assert(Array.isArray(invocation.argv), "requested invocation must include argv");
assert(invocation.env && typeof invocation.env === "object", "requested invocation must include env");

const expectedArgv = expectedInvocationArgv(workflow.process.argv);
assert(
  arraysEqual(invocation.argv, expectedArgv),
  `requested invocation argv does not match workflow process argv with report-local env boundary`,
);
assert(invocation.env.PI_CODING_AGENT_DIR === ".pi-agent", "requested invocation must set report-local PI_CODING_AGENT_DIR=.pi-agent");
assert(invocation.env.PI_OFFLINE === "1", "requested invocation must set PI_OFFLINE=1");
assert(invocation.argv[1] === "PI_CODING_AGENT_DIR=.pi-agent", "requested invocation argv must set report-local PI_CODING_AGENT_DIR=.pi-agent");
assert(invocation.argv[2] === "PI_OFFLINE=1", "requested invocation argv must set PI_OFFLINE=1");

const executedArgv = invocation.argv.slice(3);
assert(executedArgv[0] === "pi", "requested invocation must execute pi");
assert(executedArgv.includes("--print"), "requested invocation must use Pi print mode");
assert(executedArgv.includes("--no-session"), "requested invocation must disable session persistence");
assert(executedArgv.includes("--no-tools"), "requested invocation must disable tools");
assert(executedArgv.includes("--no-extensions"), "requested invocation must disable extensions");
assert(executedArgv.includes("--no-skills"), "requested invocation must disable skills");
const executedProvider = optionValue(executedArgv, "--provider");
const executedModel = optionValue(executedArgv, "--model");
const executedThinking = optionValue(executedArgv, "--thinking");
assert(executedProvider && executedModel, "requested invocation must include provider and model");
assert(executedThinking, "requested invocation must include thinking");
assert(`${executedProvider}/${executedModel}` === requestedModel, "requested invocation provider/model does not match requested model");
assert(executedThinking === requestedThinking, "requested invocation thinking does not match requested thinking");

const expectedPromptHash = sha256Text(`Return only this nonce and no other text: ${nonce}`);
assert(invocation.prompt_sha256 === expectedPromptHash, "requested invocation prompt hash does not match nonce task");

const normalizedStdout = stdout.replace(/\s+/g, " ").trim();
assert(normalizedStdout === nonce, `Pi child output did not exactly return nonce ${nonce}`);

const workflowReceipt = {
  schema_version: 1,
  runner: "pi",
  workflow: workflow.workflow,
  runtime_class: "external-runner",
  package_digest: packageDigest,
  host_version: hostVersion,
  invocation: {
    argv: invocation.argv,
    env: invocation.env,
    prompt_sha256: invocation.prompt_sha256,
  },
  requested: {
    semantic_role: "worker",
    profile,
    agent_type: agentType,
    provider_model: requestedModel,
    thinking: requestedThinking,
    isolation: args.isolation,
  },
  observed: {
    stdout_ref: "host-output:host-output.txt",
    stderr_ref: "host-stderr:host-output.stderr",
    nonce,
  },
  verdict: "advisory",
};

const dispatchReceipt = {
  schema_version: 1,
  package_digest: packageDigest,
  host_version: hostVersion,
  requested_dispatch: {
    semantic_role: "worker",
    profile,
    model: requestedModel,
    effort: requestedThinking,
    agent_type: agentType,
    fork_turns: { mode: "none" },
  },
  child_identity: {
    host: "pi",
    role: "worker",
    agent_role: agentType,
    agent_type: agentType,
    task_name: "model-routing-preset-runner",
  },
  effective_model: requestedModel,
  effective_effort: requestedThinking,
  nonce,
  raw_evidence_refs: [
    "workflow:workflow.json#arguments",
    "requested-invocation:requested-invocation.json#argv",
    "workflow-receipt:workflow-receipt.json",
    "host-output:host-output.txt",
    "host-stderr:host-output.stderr",
  ],
  verdict: "advisory",
};

writeFileSync(workflowReceiptPath, `${JSON.stringify(workflowReceipt, null, 2)}\n`);
writeFileSync(dispatchReceiptPath, `${JSON.stringify(dispatchReceipt, null, 2)}\n`);
console.log("pi runtime evidence validated");
