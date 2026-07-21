#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
routing_bin="${1:-$repo_root/target/debug/switchloom}"
workdir="$(mktemp -d /private/tmp/switchloom-website-planr-oracle.XXXXXX)"
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

require_regex() {
  local pattern="$1"
  shift
  if ! rg -q "$pattern" "$@"; then
    printf 'missing expected regex %s in %s\n' "$pattern" "$*" >&2
    exit 1
  fi
}

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

routing_bin_sha256="$(hash_file "$routing_bin")"
codex_version="$(codex --version 2>&1 | rg '^codex(-cli)? ' | tail -n 1)"

json="$workdir/website-planr-setup.json"
cat > "$json" <<'JSON'
{
  "schema_version": 1,
  "host": "codex-openai",
  "integration": "planr",
  "usage_policy": "balanced",
  "selected_roles": {
    "orchestrator": {
      "model": "gpt-5.6-sol",
      "effort": "medium",
      "spawn": {
        "agent_type": "switchloom_orchestrator",
        "task_name": "orchestrator",
        "fork_turns": { "mode": "none" }
      }
    },
    "implementer": {
      "model": "gpt-5.6-terra",
      "effort": "high",
      "spawn": {
        "agent_type": "switchloom_implementer",
        "task_name": "implementer",
        "fork_turns": { "mode": "none" }
      }
    },
    "reviewer": {
      "model": "gpt-5.6-sol",
      "effort": "high",
      "spawn": {
        "agent_type": "switchloom_reviewer",
        "task_name": "reviewer",
        "fork_turns": { "mode": "none" }
      }
    }
  },
  "routes": [
    { "work_type": "planning", "role": "orchestrator", "fallbacks": [] },
    { "work_type": "code", "role": "implementer", "fallbacks": [] },
    { "work_type": "review", "role": "reviewer", "fallbacks": [] },
    { "work_type": "verification", "role": "reviewer", "fallbacks": [] }
  ],
  "route_default": { "role": "orchestrator", "fallbacks": [] }
}
JSON
recipe="sw1_$(node -e 'const fs=require("node:fs"); process.stdout.write(Buffer.from(fs.readFileSync(process.argv[1])).toString("base64url"))' "$json")"
printf 'npx switchloom@latest apply --recipe '\''%s'\'' --repository .\n' "$recipe" > "$workdir/copied-command.txt"

planr --db "$db" project init "Switchloom Website Planr Oracle" --json > "$workdir/project-init.json"

"$routing_bin" apply --recipe "$recipe" --repository "$workdir" --yes \
  > "$workdir/apply.json" \
  2> "$workdir/apply.stderr"

require_contains ".planr/agents.toml" "$workdir/apply.json"
require_contains ".planr/policy.toml" "$workdir/apply.json"
require_contains ".codex/agents/switchloom_implementer.toml" "$workdir/apply.json"
require_contains ".codex/agents/switchloom_reviewer.toml" "$workdir/apply.json"
require_contains "Protocol preload: \$planr-work" "$workdir/.codex/agents/switchloom_implementer.toml"
require_contains "Protocol preload: \$planr-review" "$workdir/.codex/agents/switchloom_reviewer.toml"
require_absent "model-routing-native-routing" "$workdir/.codex/agents/switchloom_implementer.toml" "$workdir/.codex/agents/switchloom_reviewer.toml"

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

  worker_json="$(planr --db "$db" item create "Website worker route item" \
    --description "Website Planr recipe should route code work to the generated native implementer role" \
    --work-type code \
    --json)"
  printf '%s\n' "$worker_json" > worker-create.json
  worker_id="$(printf '%s' "$worker_json" | extract_id)"
  planr --db "$db" item route "$worker_id" --json > worker-route.json
  planr --db "$db" pick --peek --json --work-type code > worker-pick-peek.json

  review_json="$(planr --db "$db" item create "Website review route item" \
    --description "Website Planr recipe should route review work to the generated native reviewer role" \
    --work-type review \
    --json)"
  printf '%s\n' "$review_json" > review-create.json
  review_id="$(printf '%s' "$review_json" | extract_id)"
  planr --db "$db" item route "$review_id" --json > review-route.json
  planr --db "$db" pick --peek --json --work-type review > review-pick-peek.json

  plan_json="$(planr plan new "Website Planr Child Agent Oracle" --platform cli --json)"
  printf '%s\n' "$plan_json" > child-plan-new.json
  plan_path="$(printf '%s' "$plan_json" | sed -n 's/.*"path": "\([^"]*\)".*/\1/p' | head -1)"
  plan_id="$(printf '%s' "$plan_json" | sed -n 's/.*"id": "\([^"]*\)".*/\1/p' | head -1)"
  cat > "$plan_path/TASKS.md" <<'TASKS'
# Tasks

### TASK-001: Record website Planr worker evidence

Goal:
Complete this disposable oracle item from the generated `switchloom_implementer` child agent without editing source files.

Acceptance criteria:
- A completion log records that the generated website implementer role executed.
- The command `printf website-planr-worker-ok` is recorded as verification evidence.
TASKS
  planr context add "GOAL CONTRACT $plan_id: DONE when every in-scope map item is closed with log evidence, all reviews are closed with verdict complete, no open approvals remain, and a live verification log exists. Iteration budget: 4." \
    --tag goal-contract \
    --json > child-goal-context.json
  planr map build --from "$plan_id" > child-map-build.txt
  sed -n 's/.*  \([^ ]*\) \[ready\].*/\1/p' child-map-build.txt | head -1 > child-worker-item-id.txt
  printf '%s\n' "$plan_id" > child-plan-id.txt
)

test ! -e "$sentinel_hit"
mkdir -p "$workdir/child-receipts"
cat > "$workdir/expected-spawn-receipts.json" <<JSON
{
  "package_digest": "sha256:$routing_bin_sha256",
  "host_version": "$codex_version",
  "children": [
    {
      "semantic_role": "implementer",
      "profile": "switchloom_implementer",
      "agent_type": "switchloom_implementer",
      "task_name": "implementer",
      "canonical_task": "/root/implementer",
      "model": "gpt-5.6-terra",
      "effort": "high",
      "completion_contains": "SWITCHLOOM_IMPLEMENTER_CHILD_DONE"
    },
    {
      "semantic_role": "reviewer",
      "profile": "switchloom_reviewer",
      "agent_type": "switchloom_reviewer",
      "task_name": "reviewer",
      "canonical_task": "/root/reviewer",
      "model": "gpt-5.6-sol",
      "effort": "high",
      "completion_contains": "SWITCHLOOM_REVIEWER_CHILD_DONE"
    }
  ]
}
JSON

require_contains '"profile": "switchloom_implementer"' \
  "$workdir/worker-route.json" \
  "$workdir/worker-pick-peek.json" \
  "$workdir/routing-dispatch.json"
require_contains '"profile": "switchloom_reviewer"' \
  "$workdir/review-route.json" \
  "$workdir/review-pick-peek.json" \
  "$workdir/routing-dispatch.json"
require_absent "model-routing" \
  "$workdir/worker-route.json" \
  "$workdir/worker-pick-peek.json" \
  "$workdir/review-route.json" \
  "$workdir/review-pick-peek.json" \
  "$workdir/routing-dispatch.json"

child_plan_id="$(cat "$workdir/child-plan-id.txt")"
env PATH="$sentinel_path" codex exec \
  --json \
  --skip-git-repo-check \
  -C "$workdir" \
  -s workspace-write \
  -c approval_policy='"never"' \
  -o "$workdir/child-dispatch-last-message.txt" \
  "This is a disposable Switchloom website oracle. Do not edit source files outside child-receipts/. Do not set PLANR_WORKER_ID in this outer driver turn. Do not run planr pick, planr done, planr log add, or planr review close in this outer driver turn. First inspect .planr/agents.toml and the generated .codex/agents/switchloom_implementer.toml and .codex/agents/switchloom_reviewer.toml files. Then call spawn_agent with agent_type exactly switchloom_implementer, task_name exactly implementer, fork_turns exactly none, and no model or reasoning_effort override. Tell that child: Use \$planr-work on plan $child_plan_id in this repository; complete the ready code item by running printf website-planr-worker-ok, logging that exact command as verification evidence, requesting review, and stopping. Write child-receipts/implementer.json with role switchloom_implementer, canonical_task /root/implementer, protocol planr-work, model gpt-5.6-terra, effort high, the picked item id, and the verification log id. End your final answer with SWITCHLOOM_IMPLEMENTER_CHILD_DONE. Wait for that child to finish. Then call spawn_agent with agent_type exactly switchloom_reviewer, task_name exactly reviewer, fork_turns exactly none, and no model or reasoning_effort override. Tell that child: Use \$planr-review on plan $child_plan_id in this repository; pick the ready review item, rerun printf website-planr-worker-ok, close the review complete, and stop. Write child-receipts/reviewer.json with role switchloom_reviewer, canonical_task /root/reviewer, protocol planr-review, model gpt-5.6-sol, effort high, the review item id, and verdict complete. End your final answer with SWITCHLOOM_REVIEWER_CHILD_DONE. Wait for that child to finish. After both children finish, run read-only Planr audit/log/trace commands only and return a concise receipt naming both spawned task identities, both generated agent_type values, the effective child model/effort observed by each child, the verification log id, and the review verdict." \
  > "$workdir/child-dispatch-events.jsonl" \
  2> "$workdir/child-dispatch.stderr"

test -s "$workdir/child-dispatch-events.jsonl"
test ! -e "$sentinel_hit"
node "$repo_root/scripts/validate-codex-spawn-state.mjs" \
  --events "$workdir/child-dispatch-events.jsonl" \
  --workdir "$workdir" \
  --expect "$workdir/expected-spawn-receipts.json" \
  > "$workdir/codex-spawn-receipts.json"
node "$repo_root/scripts/validate-codex-runtime-evidence.mjs" \
  "$workdir/codex-spawn-receipts.json" \
  --expect "$workdir/expected-spawn-receipts.json" \
  > "$workdir/validate-codex-spawn-receipts.stdout" \
  2> "$workdir/validate-codex-spawn-receipts.stderr"

(
  cd "$workdir"
  planr plan audit "$child_plan_id" --json > child-final-audit.json
  planr map show --json > child-map-show.json
  planr log list --json > child-log-list.json
  planr trace item "$(cat child-worker-item-id.txt)" --json > child-worker-trace.json
  review_item_id="$(sed -n 's/.*"id": "\([^"]*\)".*/\1/p' child-map-show.json | rg '^i-review-' | head -1)"
  planr trace item "$review_item_id" --json > child-review-trace.json
)

require_contains '"holds": true' "$workdir/child-final-audit.json"
require_contains "switchloom_implementer" "$workdir/child-dispatch-events.jsonl" "$workdir/child-dispatch-last-message.txt"
require_contains "switchloom_reviewer" "$workdir/child-dispatch-events.jsonl" "$workdir/child-dispatch-last-message.txt"
require_contains "gpt-5.6-terra" "$workdir/child-dispatch-last-message.txt"
require_contains "gpt-5.6-sol" "$workdir/child-dispatch-last-message.txt"
require_contains "collab_tool_call" "$workdir/child-dispatch-events.jsonl"
require_contains '"status":"completed"' "$workdir/child-dispatch-events.jsonl"
require_contains '"agent_type": "switchloom_implementer"' "$workdir/codex-spawn-receipts.json"
require_contains '"agent_type": "switchloom_reviewer"' "$workdir/codex-spawn-receipts.json"
require_contains '"canonical_task": "/root/implementer"' "$workdir/codex-spawn-receipts.json"
require_contains '"canonical_task": "/root/reviewer"' "$workdir/codex-spawn-receipts.json"
require_contains '"model": "gpt-5.6-terra"' "$workdir/codex-spawn-receipts.json"
require_contains '"effort": "high"' "$workdir/codex-spawn-receipts.json"
require_contains '"kind": "verification"' "$workdir/child-log-list.json"
require_contains "printf website-planr-worker-ok" "$workdir/child-log-list.json"
require_contains "review verdict: complete" "$workdir/child-log-list.json"
require_contains '"profile": "switchloom_implementer"' "$workdir/child-worker-trace.json"
require_contains '"profile": "switchloom_reviewer"' "$workdir/child-review-trace.json"
require_regex '"worker_id": "switchloom[_-]implementer' "$workdir/child-map-show.json" "$workdir/child-worker-trace.json"
require_regex '"worker_id": "switchloom[_-]reviewer' "$workdir/child-map-show.json" "$workdir/child-review-trace.json"
require_contains '"role": "switchloom_implementer"' "$workdir/child-receipts/implementer.json"
require_contains '"canonical_task": "/root/implementer"' "$workdir/child-receipts/implementer.json"
require_contains '"protocol": "planr-work"' "$workdir/child-receipts/implementer.json"
require_contains '"role": "switchloom_reviewer"' "$workdir/child-receipts/reviewer.json"
require_contains '"canonical_task": "/root/reviewer"' "$workdir/child-receipts/reviewer.json"
require_contains '"protocol": "planr-review"' "$workdir/child-receipts/reviewer.json"
require_absent "PLANR_WORKER_ID=website-oracle" "$workdir/child-dispatch-events.jsonl" "$workdir/child-dispatch-last-message.txt"
require_absent "PLANR_WORKER_ID=switchloom-reviewer" "$workdir/child-dispatch-events.jsonl" "$workdir/child-dispatch-last-message.txt"

printf 'website Planr setup oracle passed\n'
printf 'receipts: %s\n' "$workdir"
printf 'copied command: %s\n' "$(cat "$workdir/copied-command.txt")"
printf 'worker route: switchloom_implementer\n'
printf 'review route: switchloom_reviewer\n'
printf "child dispatch: codex exec spawned switchloom_implementer and switchloom_reviewer with persisted child thread receipts\n"
printf 'model-routing sentinel: untouched\n'
