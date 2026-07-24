#!/usr/bin/env sh
set -eu

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
workdir="$(mktemp -d "${TMPDIR:-/tmp}/switchloom-archive-test.XXXXXX")"
trap 'rm -rf "$workdir"' EXIT HUP INT TERM

mkdir -p "$workdir/good" "$workdir/bad"
for member in LICENSE README.md SHA256SUMS model-routing; do
  : > "$workdir/good/$member"
  : > "$workdir/bad/$member"
done
: > "$workdir/bad/unexpected"

tar -czf "$workdir/good.tar.gz" -C "$workdir/good" LICENSE README.md SHA256SUMS model-routing
tar -czf "$workdir/bad.tar.gz" -C "$workdir/bad" LICENSE README.md SHA256SUMS model-routing unexpected

sh "$repo_root/scripts/verify-release-archive.sh" "$workdir/good.tar.gz"
if sh "$repo_root/scripts/verify-release-archive.sh" "$workdir/bad.tar.gz"; then
  echo "archive validator accepted an unexpected member" >&2
  exit 1
fi
