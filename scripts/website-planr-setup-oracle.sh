#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
routing_bin="${1:-$repo_root/target/debug/model-routing}"
exec cargo run --quiet --manifest-path "$repo_root/Cargo.toml" -p xtask -- certify planr --routing-bin "$routing_bin"
