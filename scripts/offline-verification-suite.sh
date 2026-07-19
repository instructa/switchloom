#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workdir="$(mktemp -d /private/tmp/model-routing-offline-suite.XXXXXX)"
package_files="$workdir/package-files.txt"
metadata="$workdir/cargo-metadata.json"
package_repo="$workdir/package-repo"

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

cargo fmt --manifest-path "$repo_root/Cargo.toml" --all -- --check
cargo clippy --manifest-path "$repo_root/Cargo.toml" --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path "$repo_root/Cargo.toml" --workspace --all-targets --all-features
sh "$repo_root/scripts/check-migration-manifest.sh"
node --test "$repo_root"/scripts/*.test.mjs "$repo_root"/website/*.test.mjs
node "$repo_root/scripts/regenerate-preset-catalog.mjs" --routing-bin "$repo_root/target/debug/model-routing"
cargo run --manifest-path "$repo_root/Cargo.toml" --bin model-routing -- catalog verify "$repo_root/website/data/catalog.json"
node "$repo_root/scripts/build-site.mjs"
cargo run --manifest-path "$repo_root/Cargo.toml" --bin model-routing -- evaluate balanced --host codex-openai > "$workdir/evaluate-codex.json"
require_contains '"status": "experimental"' "$workdir/evaluate-codex.json"
require_contains '"recommended": false' "$workdir/evaluate-codex.json"
require_contains '"offline_reproducible": true' "$workdir/evaluate-codex.json"
require_contains '"live_evidence": null' "$workdir/evaluate-codex.json"
cargo metadata --manifest-path "$repo_root/Cargo.toml" --format-version 1 --no-deps > "$metadata"
require_absent '"name":"planr"' "$metadata"
if git -C "$repo_root" rev-parse --verify HEAD >/dev/null 2>&1; then
  package_manifest="$repo_root/Cargo.toml"
else
  rsync -a \
    --exclude .git \
    --exclude .planr \
    --exclude target \
    --exclude dist \
    "$repo_root/" \
    "$package_repo/"
  git -C "$package_repo" init --quiet
  git -C "$package_repo" config user.email model-routing-oracle@example.invalid
  git -C "$package_repo" config user.name "Model Routing Oracle"
  git -C "$package_repo" add .
  git -C "$package_repo" commit --quiet -m "package oracle"
  package_manifest="$package_repo/Cargo.toml"
fi
cargo package --manifest-path "$package_manifest" --workspace --allow-dirty --no-verify --offline
cargo package --manifest-path "$package_manifest" --workspace --list --allow-dirty --no-verify --offline > "$package_files"
require_absent '.planr' "$package_files"
require_absent 'receipt' "$package_files"
require_absent 'credential' "$package_files"
require_absent 'secret' "$package_files"
betterleaks dir "$repo_root"
trivy fs \
  --skip-db-update \
  --skip-java-db-update \
  --scanners vuln,secret,misconfig \
  --skip-dirs "$repo_root/node_modules" \
  --skip-dirs "$repo_root/target" \
  --skip-dirs "$repo_root/dist" \
  --skip-dirs "$repo_root/.pnpm-store" \
  "$repo_root"
zizmor "$repo_root/.github/workflows"

printf 'offline verification suite passed\n'
printf 'receipts: %s\n' "$workdir"
printf 'format/lint/unit/package/website/catalog/security/workflow gates passed\n'
printf 'offline evaluation remained experimental with no live evidence claim\n'
printf 'cargo metadata has no Planr dependency\n'
