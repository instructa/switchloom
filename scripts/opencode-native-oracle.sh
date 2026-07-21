#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'usage: %s [routing-bin]\n' "$0" >&2
  printf 'live OpenCode execution is required for certification\n' >&2
  exit 2
}

routing_bin="${1:-target/debug/model-routing}"
mode="${SWITCHLOOM_OPENCODE_ORACLE_MODE:-live}"
if [ "$mode" != "live" ]; then
  printf 'unsupported SWITCHLOOM_OPENCODE_ORACLE_MODE=%s; certification requires live\n' "$mode" >&2
  exit 2
fi

host="opencode-native"
runtime_host="opencode"
profile="opencode-worker"
agent_role="model-routing-preset-worker"
driver_agent_role="model-routing-preset-driver"
requested_model="opencode/gpt-5-nano"
requested_effort="low"
artifact_path=".opencode/agents/model-routing-preset-worker.md"
report_root="${SWITCHLOOM_OPENCODE_REPORT_ROOT:-reports/native-host-certification}"
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
      printf 'live OpenCode command timed out after %s seconds; report: %s\n' "$timeout_seconds" "$report_dir" >&2
      return 124
    fi
    sleep 1
  done
  wait "$child_pid"
}

test -x "$routing_bin"
command -v opencode >/dev/null
package_digest="sha256:$(hash_file "$routing_bin")"
host_version="$(opencode --version 2>&1 | tail -n 1)"
nonce="$(uuidgen | tr '[:upper:]' '[:lower:]')"

"$routing_bin" compile balanced --host "$host" --output "$workdir/bundle.json"
"$routing_bin" apply "$workdir/bundle.json" --repository "$workdir" > "$workdir/apply.json"
test -f "$workdir/$artifact_path"

# shellcheck disable=SC2016
env \
  RUNTIME_HOST="$runtime_host" \
  MODE="$mode" \
  NONCE="$nonce" \
  ARTIFACT_PATH="$artifact_path" \
  REQUESTED_MODEL="$requested_model" \
  REQUESTED_EFFORT="$requested_effort" \
  node -e '
const fs = require("node:fs");
const payload = {
  host: process.env.RUNTIME_HOST,
  mode: process.env.MODE,
  nonce: process.env.NONCE,
  argv: [
    "env",
    "XDG_DATA_HOME=.opencode-xdg/data",
    "XDG_STATE_HOME=.opencode-xdg/state",
    "XDG_CACHE_HOME=.opencode-xdg/cache",
    "opencode",
    "run",
    "--dir",
    ".",
    "--agent",
    "model-routing-preset-driver",
    "--model",
    process.env.REQUESTED_MODEL,
    "--variant",
    process.env.REQUESTED_EFFORT,
    "--format",
    "json"
  ],
  prompt: `Use the Task tool to invoke model-routing-preset-worker. The worker must return only this nonce and must not edit files: ${process.env.NONCE}. After the worker returns, return only the same nonce.`,
  artifact_path: process.env.ARTIFACT_PATH
};
fs.writeFileSync(process.argv[1], JSON.stringify(payload, null, 2));
' "$workdir/requested-invocation.json"

prompt="Use the Task tool to invoke model-routing-preset-worker. The worker must return only this nonce and must not edit files: $nonce. After the worker returns, return only the same nonce."
host_timeout_seconds="${SWITCHLOOM_OPENCODE_TIMEOUT_SECONDS:-180}"
run_opencode_host() {
  (
    cd "$workdir"
    mkdir -p .opencode-xdg/data .opencode-xdg/state .opencode-xdg/cache
    XDG_DATA_HOME="$PWD/.opencode-xdg/data" \
      XDG_STATE_HOME="$PWD/.opencode-xdg/state" \
      XDG_CACHE_HOME="$PWD/.opencode-xdg/cache" \
      opencode run --dir . --agent "$driver_agent_role" --model "$requested_model" --variant "$requested_effort" --format json "$prompt"
  )
}

run_with_timeout "$host_timeout_seconds" run_opencode_host > "$workdir/host-output.jsonl" 2> "$workdir/host-output.stderr"

node scripts/validate-opencode-runtime-evidence.mjs \
  --jsonl "$workdir/host-output.jsonl" \
  --invocation "$workdir/requested-invocation.json" \
  --receipt "$workdir/dispatch-evidence.json" \
  --package-digest "$package_digest" \
  --host-version "$host_version" \
  --profile "$profile" \
  --model "$requested_model" \
  --variant "$requested_effort" \
  --worker "$agent_role" \
  > "$workdir/runtime-evidence-validate.stdout" \
  2> "$workdir/runtime-evidence-validate.stderr"

"$routing_bin" evidence validate "$workdir/dispatch-evidence.json" --bundle "$workdir/bundle.json" \
  > "$workdir/evidence-validate.stdout" \
  2> "$workdir/evidence-validate.stderr"

printf 'OpenCode native certification oracle passed\n'
printf 'host: %s\n' "$host"
printf 'mode: %s\n' "$mode"
printf 'receipt: %s\n' "$workdir/dispatch-evidence.json"
printf 'bundle: %s\n' "$workdir/bundle.json"
printf 'invocation: %s\n' "$workdir/requested-invocation.json"
printf 'host output: %s\n' "$workdir/host-output.jsonl"
printf 'validation: %s\n' "$workdir/evidence-validate.stdout"
printf 'report: %s\n' "$report_dir"
