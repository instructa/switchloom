#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workdir="$(mktemp -d /private/tmp/model-routing-planr-oracle.XXXXXX)"
db="$workdir/.planr/planr.sqlite"

extract_id() {
  sed -n 's/.*"id": "\([^"]*\)".*/\1/p' | head -1
}

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

planr --db "$db" project init "Model Routing Planr Oracle" --json > "$workdir/project-init.json"

cargo run --manifest-path "$repo_root/Cargo.toml" --bin model-routing -- compile balanced \
  --host codex-openai \
  --integration planr \
  --output "$workdir/planr.json" \
  > "$workdir/compile.stdout" \
  2> "$workdir/compile.stderr"

cargo run --manifest-path "$repo_root/Cargo.toml" --bin model-routing -- apply "$workdir/planr.json" \
  --repository "$workdir" \
  > "$workdir/apply.json" \
  2> "$workdir/apply.stderr"

sentinel_bin="$workdir/sentinel-bin"
sentinel_hit="$workdir/model-routing-sentinel-hit"
mkdir -p "$sentinel_bin"
printf '#!/usr/bin/env bash\nprintf "model-routing invoked\\n" > "%s"\nexit 99\n' "$sentinel_hit" > "$sentinel_bin/model-routing"
chmod +x "$sentinel_bin/model-routing"
sentinel_path="$sentinel_bin:$PATH"

(
  cd "$workdir"
  export PATH="$sentinel_path"
  planr --db "$db" agents check --json > agents-check.json
  planr --db "$db" agents list --json > agents-list.json
  planr --db "$db" prompt routing --client codex --json > routing-dispatch.json

  worker_json="$(planr --db "$db" item create "Oracle worker item" \
    --description "Worker route should expose generated native role" \
    --work-type code \
    --json)"
  printf '%s\n' "$worker_json" > worker-create.json
  worker_id="$(printf '%s' "$worker_json" | extract_id)"
  planr --db "$db" item route "$worker_id" --json > worker-route.json
  PLANR_WORKER_ID=oracle-worker planr --db "$db" pick --peek --json --work-type code > worker-pick-peek.json
  PLANR_WORKER_ID=oracle-worker planr --db "$db" pick --json --work-type code > worker-pick.json

  review_json="$(planr --db "$db" item create "Oracle review item" \
    --description "Review route should expose generated native role" \
    --work-type review \
    --json)"
  printf '%s\n' "$review_json" > review-create.json
  review_id="$(printf '%s' "$review_json" | extract_id)"
  planr --db "$db" item route "$review_id" --json > review-route.json
  PLANR_WORKER_ID=oracle-reviewer planr --db "$db" pick --peek --json --work-type review > review-pick-peek.json
  PLANR_WORKER_ID=oracle-reviewer planr --db "$db" pick --json --work-type review > review-pick.json

  plan_json="$(planr plan new "Loop Oracle Plan" --platform cli --json)"
  printf '%s\n' "$plan_json" > loop-plan-new.json
  plan_path="$(printf '%s' "$plan_json" | sed -n 's/.*"path": "\([^"]*\)".*/\1/p' | head -1)"
  plan_id="$(printf '%s' "$plan_json" | sed -n 's/.*"id": "\([^"]*\)".*/\1/p' | head -1)"
  cat > "$plan_path/TASKS.md" <<'TASKS'
# Tasks

### TASK-001: Record oracle worker evidence

Goal:
Complete this disposable oracle item without editing source files.

Acceptance criteria:
- A completion log records that the generated worker role executed.
- The command `printf oracle-worker-ok` is recorded as verification evidence.
TASKS
  planr context add "GOAL CONTRACT $plan_id: DONE when every in-scope map item is closed with log evidence, all reviews are closed with verdict complete, no open approvals remain, and a live verification log exists. Iteration budget: 4." \
    --tag goal-contract \
    --json > loop-goal-context.json
  printf '%s\n' "$plan_id" > loop-plan-id.txt
)

test ! -e "$sentinel_hit"

require_contains '"profile": "model_routing_terra_high"' \
  "$workdir/worker-route.json" \
  "$workdir/worker-pick-peek.json" \
  "$workdir/worker-pick.json" \
  "$workdir/routing-dispatch.json"
require_contains '"profile": "model_routing_sol_high"' \
  "$workdir/review-route.json" \
  "$workdir/review-pick-peek.json" \
  "$workdir/review-pick.json" \
  "$workdir/routing-dispatch.json"
require_contains 'name = "model_routing_terra_high"' "$workdir/.codex/agents/model-routing-terra-high.toml"
require_contains 'name = "model_routing_sol_high"' "$workdir/.codex/agents/model-routing-sol-high.toml"
require_contains "Protocol preload: \$planr-work" "$workdir/.codex/agents/model-routing-terra-high.toml"
require_contains "Protocol preload: \$planr-review" "$workdir/.codex/agents/model-routing-sol-high.toml"
require_contains '[agents.model_routing_terra_high]' "$workdir/.codex/config.toml"
require_contains 'config_file = "./agents/model-routing-terra-high.toml"' "$workdir/.codex/config.toml"
require_contains '[agents.model_routing_sol_high]' "$workdir/.codex/config.toml"
require_contains 'config_file = "./agents/model-routing-sol-high.toml"' "$workdir/.codex/config.toml"
require_absent 'model-routing' \
  "$workdir/worker-route.json" \
  "$workdir/worker-pick-peek.json" \
  "$workdir/worker-pick.json" \
  "$workdir/review-route.json" \
  "$workdir/review-pick-peek.json" \
  "$workdir/review-pick.json" \
  "$workdir/routing-dispatch.json" \
  "$workdir/.codex/agents/model-routing-terra-high.toml" \
  "$workdir/.codex/agents/model-routing-sol-high.toml"

loop_plan_id="$(cat "$workdir/loop-plan-id.txt")"
env PATH="$sentinel_path" codex exec \
  --json \
  --ephemeral \
  --skip-git-repo-check \
  -C "$workdir" \
  -s workspace-write \
  -c approval_policy='"never"' \
  -o "$workdir/loop-last-message.txt" \
  "Use \$planr-loop on plan $loop_plan_id. The loop contract is stored in Planr context (tag: goal-contract). Continue until the contract holds or four iterations are exhausted. This is a disposable oracle: do not edit source files; record evidence with Planr commands only. When dispatching work or review, use the generated Codex project agents selected by Planr routing; do not pass hard-coded model flags." \
  > "$workdir/loop-codex-events.jsonl" \
  2> "$workdir/loop-codex.stderr"

test -s "$workdir/loop-codex-events.jsonl"
test ! -e "$sentinel_hit"

(
  cd "$workdir"
  planr plan audit "$loop_plan_id" --json > loop-final-audit.json
  planr map show --json > loop-map-show.json
  planr log list --json > loop-log-list.json
)

require_contains '"holds": true' "$workdir/loop-final-audit.json"
require_contains 'model_routing_terra_high' "$workdir/loop-codex-events.jsonl" "$workdir/loop-last-message.txt"
require_contains 'model_routing_sol_high' "$workdir/loop-codex-events.jsonl" "$workdir/loop-last-message.txt"
require_contains 'collab_tool_call' "$workdir/loop-codex-events.jsonl"
require_contains '"kind": "verification"' "$workdir/loop-log-list.json"
require_contains 'printf oracle-worker-ok' "$workdir/loop-log-list.json"
require_contains 'review verdict: complete' "$workdir/loop-log-list.json"

cargo run --manifest-path "$repo_root/Cargo.toml" --bin model-routing -- uninstall \
  --repository "$workdir" \
  > "$workdir/uninstall.json" \
  2> "$workdir/uninstall.stderr"

test ! -e "$workdir/.model-routing/manifest.json"
test ! -e "$workdir/.planr/agents.toml"
test ! -e "$workdir/.planr/policy.toml"

post_json="$(planr --db "$db" item create "Post uninstall ordinary item" \
  --description "Planr still creates ordinary work after generated files are removed" \
  --work-type generic \
  --json)"
printf '%s\n' "$post_json" > "$workdir/post-uninstall-create.json"
post_id="$(printf '%s' "$post_json" | extract_id)"
planr --db "$db" item show "$post_id" --json > "$workdir/post-uninstall-show.json"
planr --db "$db" project show --json > "$workdir/post-uninstall-project.json"

printf 'planr adapter oracle passed\n'
printf 'receipts: %s\n' "$workdir"
printf 'worker route: model_routing_terra_high\n'
printf 'review route: model_routing_sol_high\n'
printf "loop dispatch: codex exec ran \$planr-loop without model overrides\n"
printf 'worker agent: generated model_routing_terra_high role observed in loop evidence\n'
printf 'review agent: generated model_routing_sol_high role observed in loop evidence\n'
printf 'model-routing sentinel: untouched\n'
printf 'post uninstall item: %s\n' "$post_id"
