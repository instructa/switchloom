#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

node scripts/hooks/block-forbidden-staged-files.mjs --tracked
node scripts/hooks/block-sensitive-diff-content.mjs --worktree

betterleaks_args=(git --no-banner --redact=100)
if [[ -f .betterleaks.toml ]]; then
  betterleaks_args+=(--config .betterleaks.toml)
fi
betterleaks_args+=(.)
betterleaks "${betterleaks_args[@]}"

trivy fs \
  --scanners vuln,secret,misconfig \
  --skip-dirs node_modules \
  --skip-dirs target \
  --skip-dirs dist \
  --skip-dirs .git \
  --skip-dirs .planr \
  --skip-dirs reports \
  --exit-code 1 \
  .
