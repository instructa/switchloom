#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workdir="$(mktemp -d /private/tmp/model-routing-codex-standalone.XXXXXX)"

require_contains() {
  local pattern="$1"
  shift
  if ! rg -F -q "$pattern" "$@"; then
    printf 'missing expected pattern %s in %s\n' "$pattern" "$*" >&2
    exit 1
  fi
}

require_absent() {
  local pattern="$1"
  shift
  if rg -F -q "$pattern" "$@"; then
    printf 'unexpected pattern %s in %s\n' "$pattern" "$*" >&2
    exit 1
  fi
}

cargo run --manifest-path "$repo_root/Cargo.toml" -- compile balanced \
  --host codex-openai \
  --output "$workdir/standalone.json" \
  > "$workdir/compile.stdout" \
  2> "$workdir/compile.stderr"

cargo run --manifest-path "$repo_root/Cargo.toml" -- inspect "$workdir/standalone.json" \
  > "$workdir/inspect.json" \
  2> "$workdir/inspect.stderr"

cargo run --manifest-path "$repo_root/Cargo.toml" -- preview "$workdir/standalone.json" \
  --repository "$workdir" \
  > "$workdir/preview.json" \
  2> "$workdir/preview.stderr"

cargo run --manifest-path "$repo_root/Cargo.toml" -- apply "$workdir/standalone.json" \
  --repository "$workdir" \
  > "$workdir/apply.json" \
  2> "$workdir/apply.stderr"

test -d "$workdir/.codex/agents"
test ! -e "$workdir/.planr"
require_contains 'name = "model_routing_terra_high"' "$workdir/.codex/agents/model-routing-terra-high.toml"
require_contains 'model = "gpt-5.6-terra"' "$workdir/.codex/agents/model-routing-terra-high.toml"
require_contains 'model_reasoning_effort = "high"' "$workdir/.codex/agents/model-routing-terra-high.toml"
require_contains 'name = "model_routing_sol_high"' "$workdir/.codex/agents/model-routing-sol-high.toml"
require_contains 'model = "gpt-5.6-sol"' "$workdir/.codex/agents/model-routing-sol-high.toml"
require_contains 'model_reasoning_effort = "high"' "$workdir/.codex/agents/model-routing-sol-high.toml"
require_absent 'fork_turns = "all"' "$workdir/.codex/agents/model-routing-terra-high.toml" "$workdir/.codex/agents/model-routing-sol-high.toml"
require_contains '"mode": "none"' "$workdir/standalone.json"

cat > "$workdir/oracle-prompt.md" <<'PROMPT'
Spawn exactly two child agents and wait for both to finish:
- target `/root/standalone_worker` with agent_type `model_routing_terra_high`
- target `/root/standalone_reviewer` with agent_type `model_routing_sol_high`

The worker must inspect the generated repository without editing files and return its effective route values.
The reviewer must independently inspect the generated repository and the worker receipt without editing files.

Your final answer must include these exact receipt blocks. Use MISSING for any field you cannot independently observe:

CHILD_COMPLETION_RECEIPT worker
receiver=/root/standalone_worker
role=model_routing_terra_high
model=gpt-5.6-terra
effort=high
fork=none

CHILD_COMPLETION_RECEIPT reviewer
receiver=/root/standalone_reviewer
role=model_routing_sol_high
model=gpt-5.6-sol
effort=high
fork=none

STANDALONE_CODEX_ORACLE=pass
edited_files=none
PROMPT

codex exec \
  --json \
  --ephemeral \
  --skip-git-repo-check \
  -C "$workdir" \
  -s workspace-write \
  -c approval_policy='"never"' \
  -o "$workdir/codex-last-message.txt" \
  "$(cat "$workdir/oracle-prompt.md")" \
  > "$workdir/codex-events.jsonl" \
  2> "$workdir/codex.stderr"

test -s "$workdir/codex-events.jsonl"
node "$repo_root/scripts/validate-codex-effective-receipts.mjs" \
  "$workdir/codex-last-message.txt" \
  > "$workdir/validate-effective-receipts.stdout" \
  2> "$workdir/validate-effective-receipts.stderr"
require_contains '/root/standalone_worker' "$workdir/codex-last-message.txt"
require_contains '/root/standalone_reviewer' "$workdir/codex-last-message.txt"
require_contains 'edited_files=none' "$workdir/codex-last-message.txt"
require_absent 'MISSING' "$workdir/codex-last-message.txt"
require_contains 'model_routing_terra_high' "$workdir/codex-events.jsonl"
require_contains 'model_routing_sol_high' "$workdir/codex-events.jsonl"
require_contains 'codex effective child receipt validation passed' "$workdir/validate-effective-receipts.stdout"

cargo run --manifest-path "$repo_root/Cargo.toml" -- status \
  --repository "$workdir" \
  > "$workdir/status.json" \
  2> "$workdir/status.stderr"

cargo run --manifest-path "$repo_root/Cargo.toml" -- uninstall \
  --repository "$workdir" \
  > "$workdir/uninstall.json" \
  2> "$workdir/uninstall.stderr"

test ! -e "$workdir/.model-routing/manifest.json"
test ! -e "$workdir/.codex/agents/model-routing-terra-high.toml"
test ! -e "$workdir/.codex/agents/model-routing-sol-high.toml"

printf 'codex standalone oracle passed\n'
printf 'receipts: %s\n' "$workdir"
printf 'worker effective route: model_routing_terra_high gpt-5.6-terra high fork none\n'
printf 'review effective route: model_routing_sol_high gpt-5.6-sol high fork none\n'
printf 'planr integration artifacts: absent\n'
printf 'uninstall: manifest and generated Codex agents removed\n'
