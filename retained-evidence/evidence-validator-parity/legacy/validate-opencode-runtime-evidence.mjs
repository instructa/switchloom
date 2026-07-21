#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";

function usage() {
  console.error("usage: validate-opencode-runtime-evidence.mjs --jsonl <host-output.jsonl> --invocation <requested-invocation.json> --receipt <dispatch-evidence.json> --package-digest <sha256:...> --host-version <version> --profile <profile> --model <model> --variant <variant> --worker <agent>");
  process.exit(2);
}

function arg(name) {
  const index = process.argv.indexOf(`--${name}`);
  if (index === -1 || index + 1 >= process.argv.length) usage();
  return process.argv[index + 1];
}

function parseJsonl(path) {
  return readFileSync(path, "utf8")
    .split(/\n+/)
    .filter((line) => line.trim())
    .map((line, index) => {
      try {
        return JSON.parse(line);
      } catch (error) {
        throw new Error(`host output line ${index + 1} is not JSON: ${error.message}`);
      }
    });
}

function visit(value, fn, path = []) {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => visit(entry, fn, path.concat(String(index))));
    return;
  }
  if (value && typeof value === "object") {
    for (const [key, entry] of Object.entries(value)) {
      fn(key, entry, path.concat(key), value);
      visit(entry, fn, path.concat(key));
    }
  }
}

function stringValueForKeys(value, keys) {
  let found = null;
  visit(value, (key, entry) => {
    if (found === null && keys.includes(key) && typeof entry === "string") found = entry;
  });
  return found;
}

function idValue(value) {
  return stringValueForKeys(value, ["id", "toolCallID", "toolCallId", "call_id", "callId", "taskID", "taskId"]);
}

function eventContains(value, needle) {
  return JSON.stringify(value).includes(needle);
}

function eventMentionsTask(value) {
  let structured = false;
  visit(value, (key, entry) => {
    const keyLower = key.toLowerCase();
    if (typeof entry === "string") {
      const valueLower = entry.toLowerCase();
      if ((keyLower.includes("tool") || keyLower.includes("type") || keyLower.includes("name")) && valueLower.includes("task")) {
        structured = true;
      }
    }
  });
  return structured;
}

function eventAgent(value) {
  return stringValueForKeys(value, ["agent", "agentName", "agent_name", "subagent", "subagentName", "taskAgent", "task_agent"]);
}

function eventIsResult(value) {
  let result = false;
  visit(value, (key, entry) => {
    const keyLower = key.toLowerCase();
    if (typeof entry === "string") {
      const valueLower = entry.toLowerCase();
      if ((keyLower.includes("type") || keyLower.includes("event") || keyLower.includes("kind")) && valueLower.includes("result")) {
        result = true;
      }
      if ((keyLower.includes("tool") || keyLower.includes("name")) && valueLower.includes("result")) {
        result = true;
      }
    }
  });
  return result;
}

function firstModel(value) {
  return stringValueForKeys(value, ["model", "modelID", "modelId", "providerModel", "provider_model"]);
}

function firstVariant(value) {
  return stringValueForKeys(value, ["variant", "effort", "reasoningEffort", "reasoning_effort"]);
}

const jsonlPath = arg("jsonl");
const invocationPath = arg("invocation");
const receiptPath = arg("receipt");
const packageDigest = arg("package-digest");
const hostVersion = arg("host-version");
const profile = arg("profile");
const requestedModel = arg("model");
const requestedVariant = arg("variant");
const worker = arg("worker");

const invocation = JSON.parse(readFileSync(invocationPath, "utf8"));
const nonce = invocation.nonce;
if (!nonce) throw new Error("requested invocation must include nonce");

const events = parseJsonl(jsonlPath);
if (events.length === 0) throw new Error("host output has no JSON events");

const taskInvocations = events
  .map((event, index) => ({ event, index, id: idValue(event) }))
  .filter(({ event, id }) => id && eventContains(event, worker) && eventMentionsTask(event));
if (taskInvocations.length === 0) {
  throw new Error(`no structured Task invocation with non-null call ID targeted ${worker}`);
}

const taskIds = new Set(taskInvocations.map(({ id }) => id).filter(Boolean));
const mismatchedResult = events
  .map((event) => ({ event, id: idValue(event), agent: eventAgent(event), result: eventIsResult(event) }))
  .find(({ event, id, agent, result }) => eventContains(event, nonce) && result && id && taskIds.has(id) && agent && agent !== worker);
if (mismatchedResult) {
  throw new Error(`worker result came from ${mismatchedResult.agent}, expected ${worker}`);
}
const workerResults = events
  .map((event, index) => ({ event, index, id: idValue(event), agent: eventAgent(event), result: eventIsResult(event) }))
  .filter(({ event, id, agent, result }) => {
    if (!eventContains(event, nonce)) return false;
    if (!result) return false;
    if (!id || !taskIds.has(id)) return false;
    if (agent !== worker) return false;
    return true;
  });
if (workerResults.length === 0) {
  throw new Error(`nonce ${nonce} was not returned by an explicit ${worker} Task result with matching call ID`);
}

const workerEvent = workerResults[0].event;
const effectiveModel = firstModel(workerEvent) ?? taskInvocations.map(({ event }) => firstModel(event)).find(Boolean) ?? null;
const effectiveVariant = firstVariant(workerEvent) ?? taskInvocations.map(({ event }) => firstVariant(event)).find(Boolean) ?? null;
const observedAgent = eventAgent(workerEvent);
if (!observedAgent) {
  throw new Error(`worker result is missing explicit child identity for ${worker}`);
}
if (observedAgent !== worker) {
  throw new Error(`worker result came from ${observedAgent}, expected ${worker}`);
}

const receipt = {
  schema_version: 1,
  package_digest: packageDigest,
  host_version: hostVersion,
  requested_dispatch: {
    semantic_role: "worker",
    profile,
    model: requestedModel,
    effort: requestedVariant,
    agent_type: worker,
    fork_turns: { mode: "none" },
  },
  child_identity: {
    host: "opencode",
    role: "worker",
    agent_role: observedAgent,
    agent_type: observedAgent,
    task_name: observedAgent,
  },
  nonce,
  raw_evidence_refs: [
    "requested-invocation:requested-invocation.json#argv",
    "host-output:host-output.jsonl#task",
    "host-stderr:host-output.stderr",
  ],
  verdict: "advisory",
};
if (effectiveModel) receipt.effective_model = effectiveModel;
if (effectiveVariant) receipt.effective_effort = effectiveVariant;

writeFileSync(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`);
console.log("opencode runtime evidence validated");
