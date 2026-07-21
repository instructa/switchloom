#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."
set -- release package
if [ -n "${SWITCHLOOM_TARGET:-}" ]; then
  set -- "$@" --target "$SWITCHLOOM_TARGET"
fi
if [ -n "${SWITCHLOOM_CARGO_TARGET:-}" ]; then
  set -- "$@" --cargo-target "$SWITCHLOOM_CARGO_TARGET"
fi
exec cargo run --quiet -p xtask -- "$@"
