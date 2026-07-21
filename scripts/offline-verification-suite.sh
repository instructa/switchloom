#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workdir="$(mktemp -d /private/tmp/model-routing-offline-suite.XXXXXX)"
metadata="$workdir/cargo-metadata.json"

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
node "$repo_root/scripts/check-evidence-validator-parity.mjs"
node --test "$repo_root"/scripts/*.test.mjs "$repo_root"/website/*.test.mjs
cargo run --manifest-path "$repo_root/Cargo.toml" -p xtask -- release prepare --allow-dirty
node "$repo_root/scripts/build-site.mjs"
cargo metadata --manifest-path "$repo_root/Cargo.toml" --format-version 1 --no-deps > "$metadata"
require_absent '"name":"planr"' "$metadata"
cargo run --manifest-path "$repo_root/Cargo.toml" -p xtask -- release verify --inventory-only
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
printf 'offline evaluation contract passed through Rust tests\n'
printf 'cargo metadata has no Planr dependency\n'
