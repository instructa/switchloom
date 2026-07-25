#!/usr/bin/env sh
# Credential-bound release handoff. Deterministic preparation, verification,
# packaging, inventories, provenance, and cleanliness policy live in xtask.
set -eu

cd "$(dirname "$0")/.."

external_gate=""
external_store=""
external_run=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --external-gate) external_gate="${2:-}"; shift 2 ;;
    --external-store) external_store="${2:-}"; shift 2 ;;
    --external-run) external_run="${2:-}"; shift 2 ;;
    --) shift; break ;;
    -*) echo "unknown release option: $1" >&2; exit 1 ;;
    *) break ;;
  esac
done
version="${1:-}"
summary="${2:-}"
if [ -z "$external_gate" ] || [ -z "$external_store" ] || [ -z "$external_run" ] || [ -z "$version" ] || [ -z "$summary" ] || [ "$#" -ne 2 ]; then
  echo "usage: scripts/release.sh --external-gate <tool> --external-store <store> --external-run <selected-run> <version> \"release summary\"" >&2
  exit 1
fi

if [ ! -x "$external_gate" ]; then
  echo "external release gate is not executable: $external_gate" >&2
  exit 1
fi

# This handoff must happen before any fetch, version rewrite, package build,
# commit, tag, push, or model execution.  The external owner validates the
# selected immutable run against the exact candidate and requested version.
subject="$(git rev-parse HEAD)"
"$external_gate" --root "$PWD" --store "$external_store" --run "$external_run" --subject "$subject" --version "$version"

branch="$(git rev-parse --abbrev-ref HEAD)"
if [ "$branch" != "main" ]; then
  echo "release must run on main (current: $branch)" >&2
  exit 1
fi

git fetch origin main --tags
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo "local main must exactly match origin/main before releasing" >&2
  exit 1
fi
if git rev-parse "v$version" >/dev/null 2>&1; then
  echo "tag v$version already exists" >&2
  exit 1
fi

rc_run_id="${SWITCHLOOM_RC_RUN_ID:-}"
if [ -n "$rc_run_id" ]; then
  rc_line="$(gh run view "$rc_run_id" --json conclusion,headSha --jq '.conclusion + " " + .headSha')"
  rc_conclusion="${rc_line%% *}"
  rc_sha="${rc_line#* }"
  if [ "$rc_conclusion" != "success" ]; then
    echo "release candidate run $rc_run_id is not successful: $rc_conclusion" >&2
    exit 1
  fi
  if [ "$(git rev-parse "$rc_sha^{tree}")" != "$(git rev-parse 'HEAD^{tree}')" ]; then
    echo "release candidate run $rc_run_id does not match the current source tree" >&2
    exit 1
  fi
fi

cargo run --quiet -p xtask -- release prepare --version "$version"
cargo run --quiet -p xtask -- release verify
if [ -z "$rc_run_id" ]; then
  cargo run --quiet -p xtask -- release package
fi

if [ "${RELEASE_DRY_RUN:-0}" = "1" ]; then
  echo "release dry run passed for v$version"
  exit 0
fi

git add -- Cargo.toml Cargo.lock xtask/Cargo.toml package.json \
  fixtures/routing-bundle-v1/valid-balanced-codex.json \
  fixtures/routing-bundle-v1/valid-balanced-mixed.json \
  website/data/catalog.json website/data/bundles
if ! git diff --cached --quiet; then
  git commit -m "release $version: $summary"
fi
git tag -a "v$version" -m "Switchloom v$version: $summary"
git push origin HEAD "v$version"

echo "released v$version; GitHub Actions owns publication"
