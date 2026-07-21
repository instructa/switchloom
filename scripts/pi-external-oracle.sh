#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'usage: %s [routing-bin]\n' "$0" >&2
  printf 'live Pi external runner execution is required for certification\n' >&2
  exit 2
}

routing_bin="${1:-target/debug/model-routing}"
host="pi-external"
runtime_host="pi"
profile="pi-worker"
agent_type="switchloom-pi-worker"
requested_provider="openai"
requested_model_id="gpt-4o-mini"
requested_model="$requested_provider/$requested_model_id"
requested_thinking="low"
artifact_path=".pi/workflows/model-routing-preset-runner.json"
report_root="${SWITCHLOOM_PI_REPORT_ROOT:-reports/native-host-certification}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
report_dir="$report_root/$host/$timestamp"
workdir="$report_dir/workdir"
mkdir -p "$workdir"

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

hash_text() {
  shasum -a 256 | awk '{print "sha256:" $1}'
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
      printf 'live Pi command timed out after %s seconds; report: %s\n' "$timeout_seconds" "$report_dir" >&2
      return 124
    fi
    sleep 1
  done
  wait "$child_pid"
}

test -x "$routing_bin"
command -v pi >/dev/null
package_digest="sha256:$(hash_file "$routing_bin")"
host_version="$(pi --version 2>&1 | tail -n 1)"
nonce="$(uuidgen | tr '[:upper:]' '[:lower:]')"

"$routing_bin" compile balanced --host "$host" --output "$workdir/bundle.json"
"$routing_bin" apply "$workdir/bundle.json" --repository "$workdir" > "$workdir/apply.json"
test -f "$workdir/$artifact_path"
cp "$workdir/$artifact_path" "$workdir/workflow.json"

prompt="Return only this nonce and no other text: $nonce"
prompt_sha256="$(printf '%s' "$prompt" | hash_text)"

env \
  RUNTIME_HOST="$runtime_host" \
  NONCE="$nonce" \
  ARTIFACT_PATH="$artifact_path" \
  REQUESTED_PROVIDER="$requested_provider" \
  REQUESTED_MODEL_ID="$requested_model_id" \
  REQUESTED_MODEL="$requested_model" \
  REQUESTED_THINKING="$requested_thinking" \
  AGENT_TYPE="$agent_type" \
  PROMPT_SHA256="$prompt_sha256" \
  node -e '
const fs = require("node:fs");
const payload = {
  host: process.env.RUNTIME_HOST,
  nonce: process.env.NONCE,
  argv: [
    "env",
    "PI_CODING_AGENT_DIR=.pi-agent",
    "PI_OFFLINE=1",
    "pi",
    "--print",
    "--no-session",
    "--no-tools",
    "--no-extensions",
    "--no-skills",
    "--provider",
    process.env.REQUESTED_PROVIDER,
    "--model",
    process.env.REQUESTED_MODEL_ID,
    "--thinking",
    process.env.REQUESTED_THINKING
  ],
  env: {
    PI_CODING_AGENT_DIR: ".pi-agent",
    PI_OFFLINE: "1"
  },
  requested: {
    profile: "pi-worker",
    agent_type: process.env.AGENT_TYPE,
    provider_model: process.env.REQUESTED_MODEL,
    thinking: process.env.REQUESTED_THINKING,
    isolation: {
      session: "none",
      tools: "none",
      extensions: "none",
      skills: "none"
    }
  },
  prompt_sha256: process.env.PROMPT_SHA256,
  artifact_path: process.env.ARTIFACT_PATH
};
fs.writeFileSync(process.argv[1], JSON.stringify(payload, null, 2));
' "$workdir/requested-invocation.json"

host_timeout_seconds="${SWITCHLOOM_PI_TIMEOUT_SECONDS:-180}"
run_pi_host() {
  (
    cd "$workdir"
    mkdir -p .pi-agent
    PI_CODING_AGENT_DIR="$PWD/.pi-agent" \
      PI_OFFLINE=1 \
      pi --print --no-session --no-tools --no-extensions --no-skills \
        --provider "$requested_provider" \
        --model "$requested_model_id" \
        --thinking "$requested_thinking" \
        "$prompt"
  )
}

run_with_timeout "$host_timeout_seconds" run_pi_host > "$workdir/host-output.txt" 2> "$workdir/host-output.stderr"

node scripts/validate-pi-runtime-evidence.mjs \
  --workflow "$workdir/workflow.json" \
  --invocation "$workdir/requested-invocation.json" \
  --stdout "$workdir/host-output.txt" \
  --stderr "$workdir/host-output.stderr" \
  --workflow-receipt "$workdir/workflow-receipt.json" \
  --dispatch-receipt "$workdir/dispatch-evidence.json" \
  --package-digest "$package_digest" \
  --host-version "$host_version" \
  --profile "$profile" \
  --model "$requested_model" \
  --thinking "$requested_thinking" \
  --agent-type "$agent_type" \
  > "$workdir/runtime-evidence-validate.stdout" \
  2> "$workdir/runtime-evidence-validate.stderr"

"$routing_bin" evidence validate "$workdir/dispatch-evidence.json" --bundle "$workdir/bundle.json" \
  > "$workdir/evidence-validate.stdout" \
  2> "$workdir/evidence-validate.stderr"

printf 'Pi external runner certification oracle passed\n'
printf 'host: %s\n' "$host"
printf 'runtime host: %s\n' "$runtime_host"
printf 'workflow receipt: %s\n' "$workdir/workflow-receipt.json"
printf 'dispatch receipt: %s\n' "$workdir/dispatch-evidence.json"
printf 'bundle: %s\n' "$workdir/bundle.json"
printf 'invocation: %s\n' "$workdir/requested-invocation.json"
printf 'host output: %s\n' "$workdir/host-output.txt"
printf 'validation: %s\n' "$workdir/evidence-validate.stdout"
printf 'report: %s\n' "$report_dir"
