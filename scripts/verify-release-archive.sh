#!/usr/bin/env sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <release-archive.tar.gz>" >&2
  exit 1
fi

archive="$1"
expected_members='LICENSE
README.md
SHA256SUMS
model-routing'
members="$(tar -tzf "$archive")"
members="$(printf '%s\n' "$members" | LC_ALL=C sort)"

if [ "$members" != "$expected_members" ]; then
  echo "unexpected release archive members in $archive" >&2
  printf '%s\n' "$members" >&2
  exit 1
fi
