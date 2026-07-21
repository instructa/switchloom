#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workdir="$(mktemp -d /private/tmp/model-routing-codex-standalone.XXXXXX)"
git -C "$workdir" init -q
codex_home_config_before_hash=""
codex_home_config_before_path=""
codex_auth_mode="${SWITCHLOOM_CODEX_AUTH_MODE:-current}"
if [ "$codex_auth_mode" = "current" ]; then
  codex_home="${CODEX_HOME:-$HOME/.codex}"
elif [ "$codex_auth_mode" = "isolated" ]; then
  codex_home="${SWITCHLOOM_CODEX_HOME:-/private/tmp/switchloom-codex-auth-home}"
else
  printf 'unsupported SWITCHLOOM_CODEX_AUTH_MODE: %s\n' "$codex_auth_mode" >&2
  exit 1
fi
codex_auth_wait_seconds="${SWITCHLOOM_CODEX_AUTH_WAIT_SECONDS:-0}"
package_target="$workdir/package-target"
package_src="$workdir/package-src"
install_root="$workdir/package-install"
external_routing_bin="${SWITCHLOOM_CODEX_ROUTING_BIN:-}"
external_package_digest="${SWITCHLOOM_CODEX_PACKAGE_DIGEST:-}"

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

strip_codex_project_trust_entry() {
  local input="$1"
  local output="$2"
  local header="[projects.\"$workdir\"]"
  awk -v header="$header" '
    $0 == header {
      skip = 1
      next
    }
    skip && $0 ~ /^\[/ {
      skip = 0
    }
    !skip {
      print
    }
  ' "$input" > "$output"
}

restore_current_codex_home_config() {
  if [ "$codex_auth_mode" != "current" ]; then
    return
  fi
  if [ ! -f "$codex_home_config_before_path" ]; then
    return
  fi
  if [ ! -f "$codex_home/config.toml" ]; then
    printf 'current CODEX_HOME config disappeared after authenticated Codex run: %s\n' "$codex_home/config.toml" \
      > "$workdir/codex-home-config-restore.txt"
    return 1
  fi

  local stripped="$workdir/codex-home-config-after-stripped.toml"
  strip_codex_project_trust_entry "$codex_home/config.toml" "$stripped"
  if cmp -s "$codex_home_config_before_path" "$stripped"; then
    cp "$codex_home_config_before_path" "$codex_home/config.toml"
    if ! cmp -s "$codex_home_config_before_path" "$codex_home/config.toml"; then
      printf 'failed to restore current CODEX_HOME config snapshot: %s\n' "$codex_home/config.toml" \
        > "$workdir/codex-home-config-restore.txt"
      return 1
    fi
    printf 'removed transient Codex trust entry for %s\n' "$workdir" \
      > "$workdir/codex-home-config-restore.txt"
  else
    {
      printf 'current CODEX_HOME config changed beyond the transient trust entry for %s\n' "$workdir"
      printf 'before: %s\n' "$codex_home_config_before_path"
      printf 'after stripped: %s\n' "$stripped"
    } > "$workdir/codex-home-config-restore.txt"
    return 1
  fi
}

cleanup_current_codex_home_config_on_exit() {
  local status=$?
  if ! restore_current_codex_home_config; then
    if [ "$status" -eq 0 ]; then
      status=1
    fi
  fi
  exit "$status"
}

run_restore_current_codex_home_config_trap_child() {
  codex_auth_mode="current"
  codex_home="${SWITCHLOOM_CODEX_ORACLE_TEST_CHILD_HOME:?}"
  codex_home_config_before_path="${SWITCHLOOM_CODEX_ORACLE_TEST_CHILD_BEFORE:?}"
  trap cleanup_current_codex_home_config_on_exit EXIT
  {
    printf '[projects."%s"]\n' "$workdir"
    printf 'trust_level = "trusted"\n'
  } >> "$codex_home/config.toml"
  exit 7
}

run_restore_current_codex_home_config_regression() {
  local test_root="$workdir/restore-regression"
  local trust_config="$test_root/trust-only/config.toml"
  local mutation_config="$test_root/non-trust-mutation/config.toml"
  local trap_config="$test_root/trap-failure/config.toml"
  local trap_child_status
  mkdir -p "$test_root/trust-only" "$test_root/non-trust-mutation" "$test_root/trap-failure"

  codex_auth_mode="current"
  codex_home="$test_root/trust-only"
  codex_home_config_before_path="$test_root/trust-only-before.toml"
  printf '[features]\nlocal = true\n' > "$codex_home_config_before_path"
  cp "$codex_home_config_before_path" "$trust_config"
  {
    printf '[projects."%s"]\n' "$workdir"
    printf 'trust_level = "trusted"\n'
  } >> "$trust_config"
  restore_current_codex_home_config
  cmp -s "$codex_home_config_before_path" "$trust_config"

  codex_home="$test_root/trap-failure"
  codex_home_config_before_path="$test_root/trap-failure-before.toml"
  printf '[features]\nlocal = true\n' > "$codex_home_config_before_path"
  cp "$codex_home_config_before_path" "$trap_config"
  set +e
  SWITCHLOOM_CODEX_ORACLE_TEST_RESTORE_CHILD=1 \
    SWITCHLOOM_CODEX_ORACLE_TEST_CHILD_HOME="$codex_home" \
    SWITCHLOOM_CODEX_ORACLE_TEST_CHILD_BEFORE="$codex_home_config_before_path" \
    bash "$repo_root/scripts/codex-standalone-oracle.sh" \
    > "$test_root/trap-child.stdout" \
    2> "$test_root/trap-child.stderr"
  trap_child_status=$?
  set -e
  if [ "$trap_child_status" -ne 7 ]; then
    printf 'restore trap regression preserved status %s, expected 7\n' "$trap_child_status" >&2
    exit 1
  fi
  cmp -s "$codex_home_config_before_path" "$trap_config"

  codex_home="$test_root/non-trust-mutation"
  codex_home_config_before_path="$test_root/non-trust-mutation-before.toml"
  printf '[features]\nlocal = true\n' > "$codex_home_config_before_path"
  cp "$codex_home_config_before_path" "$mutation_config"
  {
    printf '[projects."%s"]\n' "$workdir"
    printf 'trust_level = "trusted"\n'
    printf '[profiles.default]\n'
    printf 'model = "gpt-5.6-sol"\n'
  } >> "$mutation_config"
  if restore_current_codex_home_config; then
    printf 'restore regression failed to reject non-trust mutation\n' >&2
    exit 1
  fi

  printf 'restore regression passed\n'
}

if [ "${SWITCHLOOM_CODEX_ORACLE_TEST_RESTORE_CHILD:-0}" = "1" ]; then
  run_restore_current_codex_home_config_trap_child
fi

if [ "${SWITCHLOOM_CODEX_ORACLE_TEST_RESTORE:-0}" = "1" ]; then
  run_restore_current_codex_home_config_regression
  exit 0
fi

record_lifecycle_codex_home_hash() {
  local phase="$1"
  local hash
  hash="$(hash_optional_file "$lifecycle_codex_home/config.toml")"
  printf '%s %s\n' "$phase" "$hash" >> "$workdir/lifecycle-codex-home-hashes.txt"
  if [ "$hash" != "$lifecycle_codex_home_config_before_hash" ]; then
    printf 'isolated lifecycle CODEX_HOME config changed during %s: %s\n' "$phase" "$lifecycle_codex_home/config.toml" >&2
    exit 1
  fi
}

assert_lifecycle_project_preserved() {
  local phase="$1"
  require_contains '[agents.local_reviewer]' "$lifecycle_workdir/.codex/config.toml"
  require_contains 'config_file = "./agents/local-reviewer.toml"' "$lifecycle_workdir/.codex/config.toml"
  require_contains '[features]' "$lifecycle_workdir/.codex/config.toml"
  require_contains 'local = true' "$lifecycle_workdir/.codex/config.toml"
  require_contains 'name = "local_reviewer"' "$lifecycle_workdir/.codex/agents/local-reviewer.toml"
  printf '%s project-local unrelated Codex config and role preserved\n' "$phase" >> "$workdir/lifecycle-preservation.txt"
}

if [ -n "$external_routing_bin" ]; then
  routing_bin="$external_routing_bin"
  test -x "$routing_bin"
  routing_bin_sha256="$(hash_file "$routing_bin")"
  if [ -n "$external_package_digest" ]; then
    case "$external_package_digest" in
      sha256:*) crate_sha256="${external_package_digest#sha256:}" ;;
      *)
        printf 'SWITCHLOOM_CODEX_PACKAGE_DIGEST must use sha256:<hex>\n' >&2
        exit 1
        ;;
    esac
  else
    crate_sha256="$routing_bin_sha256"
  fi
  crate_path="$routing_bin"
  printf '%s  %s\n' "$crate_sha256" "$crate_path" > "$workdir/package-external.sha256"
  printf '%s  %s\n' "$routing_bin_sha256" "$routing_bin" > "$workdir/package-installed-binary.sha256"
else
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
fi
codex_version="$(codex --version 2>&1 | rg '^codex(-cli)? ' | tail -n 1)"

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

lifecycle_workdir="$(mktemp -d /private/tmp/model-routing-codex-lifecycle.XXXXXX)"
git -C "$lifecycle_workdir" init -q
mkdir -p "$lifecycle_workdir/.codex/agents"
cat > "$lifecycle_workdir/.codex/config.toml" <<'TOML'
[agents.local_reviewer]
config_file = "./agents/local-reviewer.toml"

[features]
local = true
TOML
cat > "$lifecycle_workdir/.codex/agents/local-reviewer.toml" <<'TOML'
name = "local_reviewer"
TOML
lifecycle_codex_home="$workdir/lifecycle-codex-home"
mkdir -p "$lifecycle_codex_home"
cat > "$lifecycle_codex_home/config.toml" <<TOML
[projects."$lifecycle_workdir"]
trust_level = "trusted"
TOML
require_absent '[agents.' "$lifecycle_codex_home/config.toml"
require_absent '[profiles.' "$lifecycle_codex_home/config.toml"
require_absent 'profile = ' "$lifecycle_codex_home/config.toml"
lifecycle_codex_home_config_before_hash="$(hash_optional_file "$lifecycle_codex_home/config.toml")"
printf 'before %s\n' "$lifecycle_codex_home_config_before_hash" > "$workdir/lifecycle-codex-home-hashes.txt"
assert_lifecycle_project_preserved "before"

env CODEX_HOME="$lifecycle_codex_home" "$routing_bin" preview "$workdir/standalone.json" \
  --repository "$lifecycle_workdir" \
  > "$workdir/lifecycle-preview.json" \
  2> "$workdir/lifecycle-preview.stderr"
record_lifecycle_codex_home_hash "after-preview"
assert_lifecycle_project_preserved "after-preview"

env CODEX_HOME="$lifecycle_codex_home" "$routing_bin" apply "$workdir/standalone.json" \
  --repository "$lifecycle_workdir" \
  > "$workdir/lifecycle-apply.json" \
  2> "$workdir/lifecycle-apply.stderr"
record_lifecycle_codex_home_hash "after-apply"
assert_lifecycle_project_preserved "after-apply"

"$routing_bin" compile read-only-audit \
  --host codex-openai \
  --output "$workdir/read-only-codex.json" \
  > "$workdir/lifecycle-update-compile.stdout" \
  2> "$workdir/lifecycle-update-compile.stderr"
env CODEX_HOME="$lifecycle_codex_home" "$routing_bin" update "$workdir/read-only-codex.json" \
  --repository "$lifecycle_workdir" \
  > "$workdir/lifecycle-update.json" \
  2> "$workdir/lifecycle-update.stderr"
record_lifecycle_codex_home_hash "after-update"
assert_lifecycle_project_preserved "after-update"

env CODEX_HOME="$lifecycle_codex_home" "$routing_bin" rollback \
  --repository "$lifecycle_workdir" \
  > "$workdir/lifecycle-rollback.json" \
  2> "$workdir/lifecycle-rollback.stderr"
record_lifecycle_codex_home_hash "after-rollback"
assert_lifecycle_project_preserved "after-rollback"

env CODEX_HOME="$lifecycle_codex_home" "$routing_bin" uninstall \
  --repository "$lifecycle_workdir" \
  > "$workdir/lifecycle-uninstall.json" \
  2> "$workdir/lifecycle-uninstall.stderr"
record_lifecycle_codex_home_hash "after-uninstall"
assert_lifecycle_project_preserved "after-uninstall"
require_absent 'model_routing_terra_high' "$lifecycle_workdir/.codex/config.toml"
require_absent 'model_routing_sol_high' "$lifecycle_workdir/.codex/config.toml"

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
codex_home_config_before_path="$workdir/codex-home-config-before.toml"
if [ "$codex_auth_mode" = "current" ] && [ -f "$codex_home/config.toml" ]; then
  cp "$codex_home/config.toml" "$codex_home_config_before_path"
  trap cleanup_current_codex_home_config_on_exit EXIT
fi
{
  printf 'repository .codex/config.toml\n'
  printf 'isolated CODEX_HOME: %s\n' "$codex_home"
  printf 'CODEX_HOME auth mode: %s\n' "$codex_auth_mode"
  if [ "$codex_auth_mode" = "isolated" ]; then
    printf 'isolated CODEX_HOME config contains trust/auth prerequisites only\n'
  else
    printf 'current CODEX_HOME used for auth only; project trust supplied by runtime override\n'
    printf 'global config before sha256: %s\n' "$codex_home_config_before_hash"
  fi
  printf 'no CODEX_HOME agent registrations and no profile supplied to codex exec\n'
  printf 'reload boundary: fresh codex exec process starts after repository apply\n'
  printf 'lifecycle isolated CODEX_HOME: %s\n' "$lifecycle_codex_home"
  printf 'lifecycle isolated CODEX_HOME before sha256: %s\n' "$lifecycle_codex_home_config_before_hash"
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

implementer_message="Inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_IMPLEMENTER_DONE."
reviewer_message="Independently inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_REVIEWER_DONE."
implementer_message_sha256="$(printf '%s' "$implementer_message" | shasum -a 256 | awk '{print $1}')"
reviewer_message_sha256="$(printf '%s' "$reviewer_message" | shasum -a 256 | awk '{print $1}')"
max_spawn_message_bytes=512

cat > "$workdir/expected-spawn-receipts.json" <<JSON
{
  "package_digest": "sha256:$crate_sha256",
  "host_version": "$codex_version",
  "children": [
    {
      "semantic_role": "implementer",
      "profile": "codex-terra-high",
      "kind": "implementer",
      "agent_type": "model_routing_terra_high",
      "task_name": "standalone_implementer",
      "canonical_task": "/root/standalone_implementer",
      "model": "gpt-5.6-terra",
      "effort": "high",
      "message_sha256": "$implementer_message_sha256",
      "max_message_bytes": $max_spawn_message_bytes,
      "allow_encrypted_message": true,
      "completion_contains": "SWITCHLOOM_STANDALONE_IMPLEMENTER_DONE"
    },
    {
      "semantic_role": "reviewer",
      "profile": "codex-sol-high",
      "kind": "reviewer",
      "agent_type": "model_routing_sol_high",
      "task_name": "standalone_reviewer",
      "canonical_task": "/root/standalone_reviewer",
      "model": "gpt-5.6-sol",
      "effort": "high",
      "message_sha256": "$reviewer_message_sha256",
      "max_message_bytes": $max_spawn_message_bytes,
      "allow_encrypted_message": true,
      "completion_contains": "SWITCHLOOM_STANDALONE_REVIEWER_DONE"
    }
  ]
}
JSON

cat > "$workdir/oracle-prompt.md" <<'PROMPT'
Use the native collaboration spawn_agent tool exactly twice, then wait for both child agents to finish.

Your first tool call must be spawn_agent with exactly these fields:
- agent_type: model_routing_terra_high
- task_name: standalone_implementer
- fork_turns: none
- message: Inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_IMPLEMENTER_DONE.

Your second tool call must be spawn_agent with exactly these fields:
- agent_type: model_routing_sol_high
- task_name: standalone_reviewer
- fork_turns: none
- message: Independently inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_REVIEWER_DONE.

Do not omit agent_type. Do not change either message. Do not pass model or reasoning_effort in either spawn call. Do not call wait_agent or answer before both spawn_agent calls have succeeded.

After both children finish, return a short final answer containing:
SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE
PROMPT

codex_exec_args=(
  --json
  --ignore-user-config
  -C "$workdir"
  -s workspace-write
  -c 'approval_policy="never"'
  -c "projects.\"$workdir\".trust_level=\"trusted\""
  -c "agents.model_routing_terra_high.config_file=\"$workdir/.codex/agents/model-routing-terra-high.toml\""
  -c "agents.model_routing_sol_high.config_file=\"$workdir/.codex/agents/model-routing-sol-high.toml\""
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
  < /dev/null \
  > "$workdir/codex-events.jsonl" \
  2> "$workdir/codex.stderr"
restore_current_codex_home_config

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
  --expect "$workdir/expected-spawn-receipts.json" \
  > "$workdir/validate-runtime-evidence.stdout" \
  2> "$workdir/validate-runtime-evidence.stderr"
require_contains 'SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE' "$workdir/codex-last-message.txt"
require_contains '"kind": "implementer"' "$workdir/codex-runtime-evidence.json"
require_contains '"agent_type": "model_routing_terra_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"agent_type": "model_routing_sol_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"message_sha256":' "$workdir/codex-runtime-evidence.json"
require_contains '"fork_turns": "none"' "$workdir/codex-runtime-evidence.json"
require_contains '"agent_role": "model_routing_terra_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"agent_role": "model_routing_sol_high"' "$workdir/codex-runtime-evidence.json"
require_contains '"model": "gpt-5.6-terra"' "$workdir/codex-runtime-evidence.json"
require_contains '"reasoning_effort": "high"' "$workdir/codex-runtime-evidence.json"
require_contains 'codex runtime evidence validation passed' "$workdir/validate-runtime-evidence.stdout"
require_contains 'repository .codex/config.toml' "$workdir/agent-role-source.txt"
require_contains 'reload boundary: fresh codex exec process starts after repository apply' "$workdir/agent-role-source.txt"
require_contains 'after-uninstall' "$workdir/lifecycle-codex-home-hashes.txt"
require_contains 'after-uninstall project-local unrelated Codex config and role preserved' "$workdir/lifecycle-preservation.txt"
if [ "$codex_auth_mode" = "isolated" ]; then
  require_absent '[agents.' "$codex_home/config.toml"
else
  assert_current_codex_home_read_only_auth_shape
  codex_home_config_after_hash="$(hash_optional_file "$codex_home/config.toml")"
  printf 'global config after sha256: %s\n' "$codex_home_config_after_hash" >> "$workdir/agent-role-source.txt"
  if [ "$codex_home_config_after_hash" != "$codex_home_config_before_hash" ]; then
    printf 'global config sha256 changed during authenticated Codex run; no global Switchloom agent registrations were present before or after: %s -> %s\n' "$codex_home_config_before_hash" "$codex_home_config_after_hash" >> "$workdir/agent-role-source.txt"
    exit 1
  else
    printf 'global config sha256 unchanged: %s\n' "$codex_home_config_after_hash" >> "$workdir/agent-role-source.txt"
  fi
fi

printf 'codex standalone oracle passed\n'
printf 'receipts: %s\n' "$workdir"
printf 'package crate: %s\n' "$crate_path"
printf 'package crate sha256: %s\n' "$crate_sha256"
printf 'installed binary: %s\n' "$routing_bin"
printf 'installed binary sha256: %s\n' "$routing_bin_sha256"
printf 'runtime evidence: %s\n' "$workdir/codex-runtime-evidence.json"
printf 'agent role source proof: %s\n' "$workdir/agent-role-source.txt"
printf 'lifecycle CODEX_HOME hash proof: %s\n' "$workdir/lifecycle-codex-home-hashes.txt"
printf 'lifecycle preservation proof: %s\n' "$workdir/lifecycle-preservation.txt"
printf 'implementer effective route: model_routing_terra_high gpt-5.6-terra high fork none\n'
printf 'review effective route: model_routing_sol_high gpt-5.6-sol high fork none\n'
printf 'planr integration artifacts: absent\n'
printf 'preserved generated repository and retained auth evidence; uninstall not run\n'
