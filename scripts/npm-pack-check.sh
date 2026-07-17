#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cache_dir="${TMPDIR:-/tmp}/switchloom-npm-cache"

cd "$repo_root"
npm_config_cache="$cache_dir" npm pack --dry-run
