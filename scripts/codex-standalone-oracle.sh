#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workdir="$(mktemp -d /private/tmp/model-routing-codex-standalone.XXXXXX)"
git -C "$workdir" init -q
codex_auth_mode="${SWITCHLOOM_CODEX_AUTH_MODE:-current}"
if [ "$codex_auth_mode" = "current" ]; then
  codex_home="${CODEX_HOME:-$HOME/.codex}"
elif [ "$codex_auth_mode" = "isolated" ]; then
  codex_home="${SWITCHLOOM_CODEX_HOME:-/private/tmp/switchloom-codex-auth-home}"
else
  printf 'unsupported SWITCHLOOM_CODEX_AUTH_MODE: %s\n' "$codex_auth_mode" >&2
  exit 1
fi
codex_auth_wait_seconds="${SWITCHLOOM_CODEX_AUTH_WAIT_SECONDS:-900}"
package_target="$workdir/package-target"
package_src="$workdir/package-src"
install_root="$workdir/package-install"

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

assert_isolated_codex_home() {
  mkdir -p "$codex_home"
  if [ -d "$codex_home/agents" ]; then
    printf 'isolated CODEX_HOME must not contain agent registrations: %s/agents\n' "$codex_home" >&2
    exit 1
  fi
  if find "$codex_home" -maxdepth 1 -name '*.config.toml' -type f | rg -q .; then
    printf 'isolated CODEX_HOME must not contain profile config registrations: %s\n' "$codex_home" >&2
    find "$codex_home" -maxdepth 1 -name '*.config.toml' -type f >&2
    exit 1
  fi
  if [ -f "$codex_home/config.toml" ]; then
    require_absent '[agents.' "$codex_home/config.toml"
    require_absent 'config_file = "./agents/' "$codex_home/config.toml"
    require_absent 'config_file = "' "$codex_home/config.toml"
    require_absent '[profiles.' "$codex_home/config.toml"
    require_absent 'profile = ' "$codex_home/config.toml"
  fi
}

assert_current_codex_home_read_only_auth_shape() {
  local search_paths=()
  test -d "$codex_home"
  if [ -f "$codex_home/config.toml" ]; then
    search_paths+=("$codex_home/config.toml")
  fi
  if [ -d "$codex_home/agents" ]; then
    search_paths+=("$codex_home/agents")
  fi
  while IFS= read -r -d '' profile_config; do
    search_paths+=("$profile_config")
  done < <(find "$codex_home" -maxdepth 1 -name '*.config.toml' -type f -print0)

  if [ "${#search_paths[@]}" -gt 0 ]; then
    require_absent 'model_routing_terra_high' "${search_paths[@]}"
    require_absent 'model_routing_sol_high' "${search_paths[@]}"
    require_absent 'model-routing-terra-high.toml' "${search_paths[@]}"
    require_absent 'model-routing-sol-high.toml' "${search_paths[@]}"
  fi
}

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

hash_optional_file() {
  if [ -f "$1" ]; then
    hash_file "$1"
  else
    printf 'missing\n'
  fi
}

cargo package --manifest-path "$repo_root/Cargo.toml" \
  --allow-dirty \
  --no-verify \
  --offline \
  --target-dir "$package_target" \
  > "$workdir/package.stdout" \
  2> "$workdir/package.stderr"

crate_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_root/Cargo.toml" | head -n 1)"
crate_path="$package_target/package/model-routing-$crate_version.crate"
test -f "$crate_path"
crate_sha256="$(hash_file "$crate_path")"
printf '%s  %s\n' "$crate_sha256" "$crate_path" > "$workdir/package-crate.sha256"

mkdir -p "$package_src"
tar -xf "$crate_path" -C "$package_src"
crate_stem="$(basename "$crate_path" .crate)"
packaged_repo="$package_src/$crate_stem"
test -f "$packaged_repo/Cargo.toml"

cargo install --path "$packaged_repo" \
  --bin model-routing \
  --root "$install_root" \
  --locked \
  --offline \
  > "$workdir/package-install.stdout" \
  2> "$workdir/package-install.stderr"

routing_bin="$install_root/bin/model-routing"
test -x "$routing_bin"
routing_bin_sha256="$(hash_file "$routing_bin")"
printf '%s  %s\n' "$routing_bin_sha256" "$routing_bin" > "$workdir/package-installed-binary.sha256"

"$routing_bin" compile balanced \
  --host codex-openai \
  --output "$workdir/standalone.json" \
  > "$workdir/compile.stdout" \
  2> "$workdir/compile.stderr"

"$routing_bin" inspect "$workdir/standalone.json" \
  > "$workdir/inspect.json" \
  2> "$workdir/inspect.stderr"

"$routing_bin" preview "$workdir/standalone.json" \
  --repository "$workdir" \
  > "$workdir/preview.json" \
  2> "$workdir/preview.stderr"

"$routing_bin" apply "$workdir/standalone.json" \
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
require_contains '[agents.model_routing_terra_high]' "$workdir/.codex/config.toml"
require_contains 'config_file = "./agents/model-routing-terra-high.toml"' "$workdir/.codex/config.toml"
require_contains '[agents.model_routing_sol_high]' "$workdir/.codex/config.toml"
require_contains 'config_file = "./agents/model-routing-sol-high.toml"' "$workdir/.codex/config.toml"
require_contains '"mode": "none"' "$workdir/standalone.json"

if [ "$codex_auth_mode" = "isolated" ]; then
  assert_isolated_codex_home
  {
    printf 'model = "gpt-5.6-sol"\n'
    printf 'model_reasoning_effort = "high"\n'
    printf 'service_tier = "priority"\n'
    printf 'cli_auth_credentials_store = "auto"\n'
    printf 'mcp_oauth_credentials_store = "auto"\n\n'
    printf '[features]\n'
    printf 'multi_agent = true\n\n'
    printf '[projects."%s"]\n' "$workdir"
    printf 'trust_level = "trusted"\n'
  } > "$codex_home/config.toml"
  assert_isolated_codex_home
else
  assert_current_codex_home_read_only_auth_shape
fi
codex_home_config_before_hash="$(hash_optional_file "$codex_home/config.toml")"
{
  printf 'repository .codex/config.toml\n'
  printf 'isolated CODEX_HOME: %s\n' "$codex_home"
  printf 'CODEX_HOME auth mode: %s\n' "$codex_auth_mode"
  if [ "$codex_auth_mode" = "isolated" ]; then
    printf 'isolated CODEX_HOME config contains trust/auth prerequisites only\n'
  else
    printf 'current CODEX_HOME used read-only for auth; project trust supplied by runtime override\n'
    printf 'global config before sha256: %s\n' "$codex_home_config_before_hash"
  fi
  printf 'no CODEX_HOME agent registrations and no profile supplied to codex exec\n'
} > "$workdir/agent-role-source.txt"

auth_deadline=$((SECONDS + codex_auth_wait_seconds))
until env CODEX_HOME="$codex_home" codex login status \
  > "$workdir/codex-login-status.stdout" \
  2> "$workdir/codex-login-status.stderr"; do
  if [ "$SECONDS" -ge "$auth_deadline" ]; then
    printf 'isolated CODEX_HOME is missing Codex auth prerequisites after %s seconds: %s\n' "$codex_auth_wait_seconds" "$codex_home" >&2
    printf 'receipt: %s\n' "$workdir" >&2
    exit 1
  fi
  sleep 5
done
if [ "$codex_auth_mode" = "isolated" ]; then
  assert_isolated_codex_home
else
  assert_current_codex_home_read_only_auth_shape
  codex_home_config_after_login_hash="$(hash_optional_file "$codex_home/config.toml")"
  if [ "$codex_home_config_after_login_hash" != "$codex_home_config_before_hash" ]; then
    printf 'current CODEX_HOME config changed during login preflight: %s\n' "$codex_home/config.toml" >&2
    exit 1
  fi
fi

cat > "$workdir/expected-spawn-receipts.json" <<'JSON'
{
  "children": [
    {
      "kind": "worker",
      "agent_type": "model_routing_terra_high",
      "task_name": "standalone_worker",
      "canonical_task": "/root/standalone_worker",
      "model": "gpt-5.6-terra",
      "effort": "high",
      "completion_contains": "SWITCHLOOM_STANDALONE_WORKER_DONE"
    },
    {
      "kind": "reviewer",
      "agent_type": "model_routing_sol_high",
      "task_name": "standalone_reviewer",
      "canonical_task": "/root/standalone_reviewer",
      "model": "gpt-5.6-sol",
      "effort": "high",
      "completion_contains": "SWITCHLOOM_STANDALONE_REVIEWER_DONE"
    }
  ]
}
JSON

cat > "$workdir/oracle-prompt.md" <<'PROMPT'
Use the native collaboration spawn_agent tool exactly twice, then wait for both child agents to finish.

Your first tool call must be spawn_agent with exactly these control fields:
- agent_type: model_routing_terra_high
- task_name: standalone_worker
- fork_turns: none

Your second tool call must be spawn_agent with exactly these control fields:
- agent_type: model_routing_sol_high
- task_name: standalone_reviewer
- fork_turns: none

Do not omit agent_type. Do not pass model or reasoning_effort in either spawn call. Do not call wait_agent or answer before both spawn_agent calls have succeeded.

Worker message: Inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_WORKER_DONE.
Reviewer message: Independently inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_REVIEWER_DONE.

After both children finish, return a short final answer containing:
SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE
PROMPT

codex_exec_args=(
  --json
  -C "$workdir"
  -s workspace-write
  -c 'approval_policy="never"'
  -c "projects.\"$workdir\".trust_level=\"trusted\""
  -c multi_agent_v2.hide_spawn_agent_metadata=false
  -o "$workdir/codex-last-message.txt"
)
if [ "$codex_auth_mode" = "current" ]; then
  codex_exec_args+=(
    -c 'cli_auth_credentials_store="auto"'
    -c 'mcp_oauth_credentials_store="auto"'
  )
fi

env CODEX_HOME="$codex_home" codex exec "${codex_exec_args[@]}" "$(cat "$workdir/oracle-prompt.md")" \
  > "$workdir/codex-events.jsonl" \
  2> "$workdir/codex.stderr"

test -s "$workdir/codex-events.jsonl"
node "$repo_root/scripts/validate-codex-spawn-state.mjs" \
  --events "$workdir/codex-events.jsonl" \
  --workdir "$workdir" \
  --expect "$workdir/expected-spawn-receipts.json" \
  --state-db "$codex_home/state_5.sqlite" \
  --sessions-dir "$codex_home/sessions" \
  > "$workdir/codex-runtime-evidence.json"
node "$repo_root/scripts/validate-codex-runtime-evidence.mjs" \
  "$workdir/codex-runtime-evidence.json" \
  > "$workdir/validate-runtime-evidence.stdout" \
  2> "$workdir/validate-runtime-evidence.stderr"
require_contains 'SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE' "$workdir/codex-last-message.txt"
require_contains '"agent_type": "model_routing_terra_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"agent_type": "model_routing_sol_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"fork_turns": "none"' "$workdir/codex-runtime-evidence.json"
require_contains '"agent_role": "model_routing_terra_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"agent_role": "model_routing_sol_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"model": "gpt-5.6-terra"' "$workdir/codex-runtime-evidence.json"
require_contains '"reasoning_effort": "high"' "$workdir/codex-runtime-evidence.json"
require_contains 'codex runtime evidence validation passed' "$workdir/validate-runtime-evidence.stdout"
require_contains 'repository .codex/config.toml' "$workdir/agent-role-source.txt"
if [ "$codex_auth_mode" = "isolated" ]; then
  require_absent '[agents.' "$codex_home/config.toml"
else
  assert_current_codex_home_read_only_auth_shape
  codex_home_config_after_hash="$(hash_optional_file "$codex_home/config.toml")"
  printf 'global config after sha256: %s\n' "$codex_home_config_after_hash" >> "$workdir/agent-role-source.txt"
  if [ "$codex_home_config_after_hash" != "$codex_home_config_before_hash" ]; then
    printf 'current CODEX_HOME config changed during oracle run: %s\n' "$codex_home/config.toml" >&2
    exit 1
  fi
  printf 'global config sha256 unchanged: %s\n' "$codex_home_config_after_hash" >> "$workdir/agent-role-source.txt"
fi

printf 'codex standalone oracle passed\n'
printf 'receipts: %s\n' "$workdir"
printf 'package crate: %s\n' "$crate_path"
printf 'package crate sha256: %s\n' "$crate_sha256"
printf 'installed binary: %s\n' "$routing_bin"
printf 'installed binary sha256: %s\n' "$routing_bin_sha256"
printf 'runtime evidence: %s\n' "$workdir/codex-runtime-evidence.json"
printf 'agent role source proof: %s\n' "$workdir/agent-role-source.txt"
printf 'worker effective route: model_routing_terra_high gpt-5.6-terra high fork none\n'
printf 'review effective route: model_routing_sol_high gpt-5.6-sol high fork none\n'
printf 'planr integration artifacts: absent\n'
printf 'preserved generated repository and auth home; uninstall not run\n'
