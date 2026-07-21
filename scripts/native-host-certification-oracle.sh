#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'usage: %s <claude-native|cursor-openai|cursor-fable-grok> [routing-bin]\n' "$0" >&2
  printf 'live host execution is the default and only certification mode\n' >&2
  exit 2
}

host="${1:-}"
test -n "$host" || usage
routing_bin="${2:-target/debug/model-routing}"
mode="${SWITCHLOOM_NATIVE_HOST_ORACLE_MODE:-live}"
if [ "$mode" != "live" ]; then
  printf 'unsupported SWITCHLOOM_NATIVE_HOST_ORACLE_MODE=%s; certification requires live\n' "$mode" >&2
  exit 2
fi
report_root="${SWITCHLOOM_NATIVE_HOST_REPORT_ROOT:-reports/native-host-certification}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
report_dir="$report_root/$host/$timestamp"
workdir="$report_dir/workdir"
mkdir -p "$workdir"

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

run_with_timeout() {
  local timeout_seconds="$1"
  shift
  "$@" &
  local child_pid="$!"
  local deadline=$((SECONDS + timeout_seconds))
  while kill -0 "$child_pid" 2>/dev/null; do
    if [ "$SECONDS" -ge "$deadline" ]; then
      kill "$child_pid" 2>/dev/null || true
      wait "$child_pid" 2>/dev/null || true
      printf 'live host command timed out after %s seconds; report: %s\n' "$timeout_seconds" "$report_dir" >&2
      return 124
    fi
    sleep 1
  done
  wait "$child_pid"
}

case "$host" in
  claude-native)
    runtime_host="claude-code"
    cli_bin="claude"
    profile="claude-native-worker"
    semantic_role="worker"
    agent_role="model-routing-preset-worker"
    requested_model="sonnet"
    requested_effort="medium"
    artifact_path=".claude/agents/model-routing-preset-worker.md"
    host_version="$(claude --version 2>&1 | tail -n 1)"
    invocation_json='["claude","--print","--output-format","json","--agent","model-routing-preset-worker","--model","sonnet","--effort","medium"]'
    ;;
  cursor-openai)
    runtime_host="cursor"
    cli_bin="cursor-agent"
    profile="cursor-openai-worker"
    semantic_role="worker"
    agent_role="model-routing-preset-worker"
    requested_model="gpt-5.4-mini"
    requested_effort=""
    artifact_path=".cursor/agents/model-routing-preset-worker.md"
    host_version="$(cursor-agent --version 2>&1 | tail -n 1)"
    invocation_json='["cursor-agent","--print","--output-format","json","--trust","--model","gpt-5.4-mini"]'
    ;;
  cursor-fable-grok)
    runtime_host="cursor"
    cli_bin="cursor-agent"
    profile="cursor-grok-worker"
    semantic_role="worker"
    agent_role="model-routing-preset-worker"
    requested_model="cursor-grok-4.5-medium"
    requested_effort=""
    artifact_path=".cursor/agents/model-routing-preset-worker.md"
    host_version="$(cursor-agent --version 2>&1 | tail -n 1)"
    invocation_json='["cursor-agent","--print","--output-format","json","--trust","--model","cursor-grok-4.5-medium"]'
    ;;
  *)
    usage
    ;;
esac

test -x "$routing_bin"
command -v "$cli_bin" >/dev/null
package_digest="sha256:$(hash_file "$routing_bin")"
nonce="$(uuidgen | tr '[:upper:]' '[:lower:]')"

"$routing_bin" compile balanced --host "$host" --output "$workdir/bundle.json"
"$routing_bin" apply "$workdir/bundle.json" --repository "$workdir" > "$workdir/apply.json"
test -f "$workdir/$artifact_path"

# shellcheck disable=SC2016
env \
  RUNTIME_HOST="$runtime_host" \
  MODE="$mode" \
  NONCE="$nonce" \
  INVOCATION_JSON="$invocation_json" \
  ARTIFACT_PATH="$artifact_path" \
  node -e '
const fs = require("node:fs");
const payload = {
  host: process.env.RUNTIME_HOST,
  mode: process.env.MODE,
  nonce: process.env.NONCE,
  argv: JSON.parse(process.env.INVOCATION_JSON),
  prompt: `Return only this nonce and do not edit files: ${process.env.NONCE}`,
  artifact_path: process.env.ARTIFACT_PATH
};
fs.writeFileSync(process.argv[1], JSON.stringify(payload, null, 2));
' "$workdir/requested-invocation.json"

prompt="Return only this nonce and do not edit files: $nonce"
host_timeout_seconds="${SWITCHLOOM_NATIVE_HOST_TIMEOUT_SECONDS:-180}"
run_claude_host() {
  (
    cd "$workdir"
    claude --print --output-format json --agent model-routing-preset-worker --model "$requested_model" --effort "$requested_effort" "$prompt"
  )
}

run_cursor_host() {
  (
    cd "$workdir"
    cursor-agent --print --output-format json --trust --model "$requested_model" "$prompt"
  )
}

if [ "$host" = "claude-native" ]; then
  run_with_timeout "$host_timeout_seconds" run_claude_host > "$workdir/host-output.json" 2> "$workdir/host-output.stderr"
else
  run_with_timeout "$host_timeout_seconds" run_cursor_host > "$workdir/host-output.json" 2> "$workdir/host-output.stderr"
fi
if ! rg -F -q "$nonce" "$workdir/host-output.json" "$workdir/host-output.stderr"; then
  printf 'live host output did not return nonce %s; report: %s\n' "$nonce" "$report_dir" >&2
  exit 1
fi

# shellcheck disable=SC2016
env \
  PACKAGE_DIGEST="$package_digest" \
  HOST_VERSION="$host_version" \
  RUNTIME_HOST="$runtime_host" \
  HOST="$host" \
  PROFILE="$profile" \
  SEMANTIC_ROLE="$semantic_role" \
  AGENT_ROLE="$agent_role" \
  REQUESTED_MODEL="$requested_model" \
  REQUESTED_EFFORT="$requested_effort" \
  NONCE="$nonce" \
  MODE="$mode" \
  WORKDIR="$workdir" \
  node -e '
const fs = require("node:fs");
const outputPath = `${process.env.WORKDIR}/host-output.json`;
let output = {};
try {
  output = JSON.parse(fs.readFileSync(outputPath, "utf8"));
} catch {
  output = {};
}
const effectiveModel = output.effective_model ?? output.model ?? output.result?.model ?? null;
const effectiveEffort = output.effective_effort ?? output.effort ?? output.result?.effort ?? null;
const requestedEffort = process.env.REQUESTED_EFFORT || null;
const deterministic = process.env.MODE === "live"
  && effectiveModel === process.env.REQUESTED_MODEL
  && (!requestedEffort || effectiveEffort === requestedEffort);
const rawRefs = [
  "requested-invocation:requested-invocation.json#argv",
  "host-output:host-output.json",
  "host-stderr:host-output.stderr"
];
if (deterministic) {
  rawRefs.push(`host-authenticated-effective-model:${process.env.RUNTIME_HOST}:host-output.json#model`);
  if (requestedEffort) {
    rawRefs.push(`host-authenticated-effective-effort:${process.env.RUNTIME_HOST}:host-output.json#effort`);
  }
}
const receipt = {
  schema_version: 1,
  package_digest: process.env.PACKAGE_DIGEST,
  host_version: process.env.HOST_VERSION,
  requested_dispatch: {
    semantic_role: process.env.SEMANTIC_ROLE,
    profile: process.env.PROFILE,
    model: process.env.REQUESTED_MODEL,
    agent_type: process.env.AGENT_ROLE,
    fork_turns: { mode: "none" }
  },
  child_identity: {
    host: process.env.RUNTIME_HOST,
    role: process.env.SEMANTIC_ROLE,
    agent_role: process.env.AGENT_ROLE,
    agent_type: process.env.AGENT_ROLE,
    task_name: process.env.AGENT_ROLE
  },
  nonce: process.env.NONCE,
  raw_evidence_refs: rawRefs,
  verdict: deterministic ? "deterministic" : "advisory"
};
if (requestedEffort) receipt.requested_dispatch.effort = requestedEffort;
if (effectiveModel) receipt.effective_model = effectiveModel;
if (effectiveEffort) receipt.effective_effort = effectiveEffort;
fs.writeFileSync(`${process.env.WORKDIR}/dispatch-evidence.json`, JSON.stringify(receipt, null, 2));
'

"$routing_bin" evidence validate "$workdir/dispatch-evidence.json" --bundle "$workdir/bundle.json" \
  > "$workdir/evidence-validate.stdout" \
  2> "$workdir/evidence-validate.stderr"

printf 'native host certification oracle passed\n'
printf 'host: %s\n' "$host"
printf 'mode: %s\n' "$mode"
printf 'receipt: %s\n' "$workdir/dispatch-evidence.json"
printf 'bundle: %s\n' "$workdir/bundle.json"
printf 'invocation: %s\n' "$workdir/requested-invocation.json"
printf 'host output: %s\n' "$workdir/host-output.json"
printf 'validation: %s\n' "$workdir/evidence-validate.stdout"
printf 'report: %s\n' "$report_dir"
