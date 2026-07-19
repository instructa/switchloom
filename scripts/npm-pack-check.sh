#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cache_dir="${TMPDIR:-/tmp}/switchloom-npm-cache"

cd "$repo_root"
package_version="$(node -p 'JSON.parse(require("fs").readFileSync("package.json", "utf8")).version')"
current_target="$(node -e 'const os=require("os"); const osMap={darwin:"darwin",linux:"linux"}; const archMap={arm64:"arm64",x64:"x86_64"}; const platform=osMap[os.platform()]; const arch=archMap[os.arch()]; process.stdout.write(platform&&arch ? platform+"-"+arch : "");')"

if [ -z "$current_target" ]; then
  printf 'unsupported npm native target for this platform\n' >&2
  exit 1
fi

for native in npm/native/*/model-routing; do
  if rg -a -F -q '0.1.1' "$native"; then
    printf 'stale native version string found in %s\n' "$native" >&2
    exit 1
  fi
  if ! rg -a -F -q "$package_version" "$native"; then
    printf 'package version %s not found in %s\n' "$package_version" "$native" >&2
    exit 1
  fi
done

node scripts/validate-native-provenance.mjs

native_version="$(./npm/native/"$current_target"/model-routing --version)"
wrapper_version="$(node npm/bin/model-routing.js --version)"
expected_version="model-routing $package_version"
if [ "$native_version" != "$expected_version" ]; then
  printf 'selected native version mismatch: expected %s, got %s\n' "$expected_version" "$native_version" >&2
  exit 1
fi
if [ "$wrapper_version" != "$expected_version" ]; then
  printf 'wrapper version mismatch: expected %s, got %s\n' "$expected_version" "$wrapper_version" >&2
  exit 1
fi

npm_config_cache="$cache_dir" npm pack --dry-run
