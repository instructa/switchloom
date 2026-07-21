#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
host="${1:?usage: native-host-certification-oracle.sh <claude-native|cursor-openai|cursor-fable-grok> [routing-bin]}"
routing_bin="${2:-$repo_root/target/debug/model-routing}"
exec cargo run --quiet --manifest-path "$repo_root/Cargo.toml" -p xtask -- certify cursor --host "$host" --routing-bin "$routing_bin"
