#!/usr/bin/env sh
# The supported release path. It validates one version across manifests, runs
# all local release gates, commits a version bump when needed, tags, and pushes.
# The tag-triggered GitHub workflow builds and publishes platform artifacts.
#
# Usage: scripts/release.sh <x.y.z[-alpha.N|-beta.N|-rc.N]> "release summary"
# Dry run: RELEASE_DRY_RUN=1 scripts/release.sh <current-version> "summary"
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
cargo run --quiet -- compile balanced --host codex-openai --integration planr \
  --output fixtures/routing-bundle-v1/valid-balanced-codex.json
cargo run --quiet -- compile balanced --host mixed-host --integration planr \
  --output fixtures/routing-bundle-v1/valid-balanced-mixed.json
cargo build --release --locked
node scripts/regenerate-preset-catalog.mjs

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
pnpm install --frozen-lockfile
pnpm site:check
pnpm pack:check
cargo package --locked --allow-dirty --no-verify
scripts/secleak-check.sh
scripts/build-release.sh
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
