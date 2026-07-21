#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."
exec cargo run --quiet -p xtask -- release verify --inventory-only --require-provenance
