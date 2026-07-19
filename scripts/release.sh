#!/usr/bin/env sh
# The supported release path. It validates one version across manifests, runs
# all local release gates, commits a version bump when needed, tags, and pushes.
# The tag-triggered GitHub workflow builds and publishes platform artifacts.
#
# Usage: scripts/release.sh <x.y.z[-alpha.N|-beta.N|-rc.N]> "release summary"
# Dry run: RELEASE_DRY_RUN=1 scripts/release.sh <current-version> "summary"
# Non-destructive local mode (requires a green RC matrix for the same tree):
# SWITCHLOOM_RC_RUN_ID=<run> SWITCHLOOM_RELEASE_REUSE_DEPS=1 scripts/release.sh ...
set -eu

cd "$(dirname "$0")/.."

version="${1:-}"
summary="${2:-}"
if ! echo "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-(alpha|beta|rc)\.[0-9]+)?$'; then
  echo "usage: scripts/release.sh <x.y.z[-alpha.N|-beta.N|-rc.N]> \"release summary\"" >&2
  exit 1
fi
if [ -z "$summary" ]; then
  echo "release summary must not be empty" >&2
  exit 1
fi

branch="$(git rev-parse --abbrev-ref HEAD)"
if [ "$branch" != "main" ]; then
  echo "release must run on main (current: $branch)" >&2
  exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
  echo "worktree is dirty; commit or stash before releasing" >&2
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
if ! grep -q "^## \[$version\]" CHANGELOG.md; then
  echo "CHANGELOG.md has no '## [$version]' section" >&2
  exit 1
fi

rc_run_id="${SWITCHLOOM_RC_RUN_ID:-}"
reuse_deps="${SWITCHLOOM_RELEASE_REUSE_DEPS:-0}"
if [ "$reuse_deps" = "1" ] && [ -z "$rc_run_id" ]; then
  echo "SWITCHLOOM_RELEASE_REUSE_DEPS=1 requires SWITCHLOOM_RC_RUN_ID" >&2
  exit 1
fi
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
  echo "verified release candidate run $rc_run_id for source tree $rc_sha"
fi

current_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if [ "${RELEASE_DRY_RUN:-0}" = "1" ] && [ "$version" != "$current_version" ]; then
  echo "dry runs require the current manifest version ($current_version)" >&2
  exit 1
fi

replace() {
  file="$1"
  pattern="$2"
  sed "$pattern" "$file" > "$file.release-tmp"
  mv "$file.release-tmp" "$file"
}

replace Cargo.toml "s/^version = \".*\"/version = \"$version\"/"
replace package.json "s/\"version\": \".*\"/\"version\": \"$version\"/"
# Refresh only the root package version in Cargo.lock before all subsequent
# locked builds and package checks.
cargo check --quiet
cargo run --quiet --bin model-routing -- compile balanced --host codex-openai --integration planr \
  --output fixtures/routing-bundle-v1/valid-balanced-codex.json
cargo run --quiet --bin model-routing -- compile balanced --host mixed-host --integration planr \
  --output fixtures/routing-bundle-v1/valid-balanced-mixed.json
cargo build --release --locked
node scripts/regenerate-preset-catalog.mjs

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
if [ "$reuse_deps" = "1" ]; then
  test -x node_modules/.bin/vitest
  test -x node_modules/.bin/astro
  ./node_modules/.bin/vitest run
  node --test website/alchemy-runtime.test.mjs website/cloudflare-launcher.test.mjs
  ./node_modules/.bin/astro check
  ./node_modules/.bin/astro build
  node scripts/build-site.mjs
  bash scripts/npm-pack-check.sh
else
  pnpm install --frozen-lockfile
  pnpm site:check
  pnpm pack:check
fi
cargo package --locked --allow-dirty --no-verify
scripts/secleak-check.sh
if [ -n "$rc_run_id" ]; then
  echo "release archives already verified by release candidate run $rc_run_id"
else
  scripts/build-release.sh
fi
SWITCHLOOM_NATIVE_BIN="$(pwd)/target/release/model-routing" node npm/bin/model-routing.js --version

if [ "${RELEASE_DRY_RUN:-0}" = "1" ]; then
  echo "release dry run passed for v$version"
  exit 0
fi

git add -- Cargo.toml Cargo.lock package.json \
  fixtures/routing-bundle-v1/valid-balanced-codex.json \
  fixtures/routing-bundle-v1/valid-balanced-mixed.json \
  website/data/catalog.json website/data/bundles
if ! git diff --cached --quiet; then
  git commit -m "release $version: $summary"
fi
git tag -a "v$version" -m "Switchloom v$version: $summary"
git push origin HEAD "v$version"

echo "released v$version; the Release workflow will publish signed checksums and platform archives"
