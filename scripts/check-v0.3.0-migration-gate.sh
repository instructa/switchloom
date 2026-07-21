#!/usr/bin/env bash
set -euo pipefail

readonly expected_version="0.2.2"
readonly expected_tag="v0.2.2"
readonly expected_tag_commit="206ab2ba15724e438486ef5dfa1ee31888858c19"
readonly expected_npm_sha256="ace8594db2f972a754dbfd26590f3196fbd8abaf58648aef1407bc3c463eddc6"
readonly expected_native_sha256="363c8e146988f315fb46e30083523b82c3f61486631188ea30d218549931b8b6"
readonly expected_release_archive_sha256="74743a04809a0f625a791a1dc10f72e5f21a7ef5d6d99ad872d5740ebccbd6a9"
readonly expected_catalog_sha256="1e86d746a362de3de82dbd74579eef241fca0ca2a2f3278fa546d2e89f21ab5c"
readonly release_plan="pln-68a087d8"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir/.." rev-parse --show-toplevel)"
planr_root="${SWITCHLOOM_PLANR_ROOT:-$repo_root}"
cd "$repo_root"

for command_name in git planr node npm gh brew curl tar shasum; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'migration gate: required command is unavailable: %s\n' "$command_name" >&2
    exit 1
  fi
done

dirty_state="$(git status --porcelain=v1 --untracked-files=all)"
if [[ -n "$dirty_state" ]]; then
  printf '%s\n' 'migration gate: candidate tree is dirty; refusing migration' >&2
  printf '%s\n' "$dirty_state" >&2
  exit 1
fi

resolved_tag_commit="$(git rev-parse "${expected_tag}^{commit}")"
if [[ "$resolved_tag_commit" != "$expected_tag_commit" ]]; then
  printf 'migration gate: %s resolves to %s, expected %s\n' \
    "$expected_tag" "$resolved_tag_commit" "$expected_tag_commit" >&2
  exit 1
fi
if ! git merge-base --is-ancestor "$expected_tag_commit" HEAD; then
  printf 'migration gate: released commit %s is not an ancestor of HEAD\n' \
    "$expected_tag_commit" >&2
  exit 1
fi

package_version="$(node -p "JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version")"
cargo_version="$(sed -nE 's/^version = "([^"]+)"$/\1/p' Cargo.toml | head -n 1)"
if [[ "$package_version" != "$expected_version" || "$cargo_version" != "$expected_version" ]]; then
  printf 'migration gate: local versions disagree (npm=%s cargo=%s expected=%s)\n' \
    "$package_version" "$cargo_version" "$expected_version" >&2
  exit 1
fi

audit_json="$(cd "$planr_root" && planr plan audit "$release_plan" --json)"
AUDIT_JSON="$audit_json" node <<'NODE'
const audit = JSON.parse(process.env.AUDIT_JSON);
if (audit.holds !== true) {
  console.error("migration gate: release plan does not hold");
  process.exit(1);
}
NODE

npm_json="$(npm view "switchloom@$expected_version" version dist.tarball dist.shasum dist.integrity --json)"
npm_tarball_url="$(NPM_JSON="$npm_json" node <<'NODE'
const metadata = JSON.parse(process.env.NPM_JSON);
if (metadata.version !== "0.2.2") {
  console.error(`migration gate: npm resolved ${metadata.version}, expected 0.2.2`);
  process.exit(1);
}
if (metadata["dist.shasum"] !== "928fd2a847acc9f30a94ed96bf71500e41ea443d") {
  console.error("migration gate: npm shasum changed");
  process.exit(1);
}
if (metadata["dist.integrity"] !== "sha512-+N8UCvDIIAkWsxWSfE9nerP6u9S9hl6vb3hH6CWkx2mUDGmqmdAm8tmU8rL9csCn5RITox3Ite8320OndB4R7A==") {
  console.error("migration gate: npm integrity changed");
  process.exit(1);
}
process.stdout.write(metadata["dist.tarball"]);
NODE
)"

npm_sha256="$(curl -fsSL "$npm_tarball_url" | shasum -a 256 | awk '{print $1}')"
npm_native_sha256="$(curl -fsSL "$npm_tarball_url" | \
  tar -xzOf - package/npm/native/darwin-arm64/model-routing | shasum -a 256 | awk '{print $1}')"
if [[ "$npm_sha256" != "$expected_npm_sha256" || "$npm_native_sha256" != "$expected_native_sha256" ]]; then
  printf 'migration gate: npm bytes changed (tarball=%s native=%s)\n' \
    "$npm_sha256" "$npm_native_sha256" >&2
  exit 1
fi

release_json="$(gh release view "$expected_tag" --repo instructa/switchloom \
  --json tagName,isDraft,isPrerelease,assets)"
RELEASE_JSON="$release_json" node <<'NODE'
const release = JSON.parse(process.env.RELEASE_JSON);
const asset = release.assets.find(({name}) => name === "switchloom-darwin-arm64.tar.gz");
if (release.tagName !== "v0.2.2" || release.isDraft || release.isPrerelease) {
  console.error("migration gate: GitHub release is unresolved or not final");
  process.exit(1);
}
if (asset?.digest !== "sha256:74743a04809a0f625a791a1dc10f72e5f21a7ef5d6d99ad872d5740ebccbd6a9") {
  console.error("migration gate: GitHub release asset digest changed");
  process.exit(1);
}
NODE

release_archive_url="https://github.com/instructa/switchloom/releases/download/$expected_tag/switchloom-darwin-arm64.tar.gz"
release_archive_sha256="$(curl -fsSL "$release_archive_url" | shasum -a 256 | awk '{print $1}')"
release_native_sha256="$(curl -fsSL "$release_archive_url" | \
  tar -xzOf - model-routing | shasum -a 256 | awk '{print $1}')"
if [[ "$release_archive_sha256" != "$expected_release_archive_sha256" || \
      "$release_native_sha256" != "$expected_native_sha256" ]]; then
  printf 'migration gate: GitHub bytes changed (archive=%s native=%s)\n' \
    "$release_archive_sha256" "$release_native_sha256" >&2
  exit 1
fi

brew_json="$(brew info --json=v2 instructa/tap/switchloom)"
BREW_JSON="$brew_json" node <<'NODE'
const formula = JSON.parse(process.env.BREW_JSON).formulae[0];
if (formula?.versions?.stable !== "0.2.2") {
  console.error("migration gate: Homebrew stable version is not 0.2.2");
  process.exit(1);
}
if (formula?.urls?.stable?.checksum !== "74743a04809a0f625a791a1dc10f72e5f21a7ef5d6d99ad872d5740ebccbd6a9") {
  console.error("migration gate: Homebrew checksum disagrees with GitHub release");
  process.exit(1);
}
NODE

website_catalog_sha256="$(curl -fsSL https://switchloom.ai/data/catalog.json | \
  shasum -a 256 | awk '{print $1}')"
local_catalog_sha256="$(shasum -a 256 website/data/catalog.json | awk '{print $1}')"
if [[ "$website_catalog_sha256" != "$expected_catalog_sha256" || \
      "$local_catalog_sha256" != "$expected_catalog_sha256" ]]; then
  printf 'migration gate: website/local catalog bytes disagree (website=%s local=%s)\n' \
    "$website_catalog_sha256" "$local_catalog_sha256" >&2
  exit 1
fi

printf '%s\n' \
  "migration gate passed: release plan holds" \
  "tag commit: $expected_tag_commit" \
  "npm tarball sha256: $npm_sha256" \
  "npm/GitHub native sha256: $expected_native_sha256" \
  "GitHub/Homebrew archive sha256: $release_archive_sha256" \
  "website/local catalog sha256: $website_catalog_sha256"
