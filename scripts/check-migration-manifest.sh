#!/bin/sh
set -eu

manifest="${1:-docs/migration-manifest.tsv}"
planr_root="${PLANR_ROOT:-/Users/kregenrek/projects/planr}"
baseline_tag="${PLANR_BASELINE_TAG:-v1.5.0}"
baseline_commit="${PLANR_BASELINE_COMMIT:-7a01ad54cb41fd755f368a79339a96a997f693d0}"

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/model-routing-manifest.XXXXXX")"
trap 'rm -rf "$tmpdir"' EXIT

actual_sources="$tmpdir/actual-sources.txt"
manifest_sources="$tmpdir/manifest-sources.txt"
missing_sources="$tmpdir/missing-sources.txt"
unexpected_sources="$tmpdir/unexpected-sources.txt"

resolved_commit="$(git -C "$planr_root" rev-parse "$baseline_tag")"
if [ "$resolved_commit" != "$baseline_commit" ]; then
  echo "baseline tag $baseline_tag resolved to $resolved_commit, expected $baseline_commit" >&2
  exit 1
fi

git -C "$planr_root" ls-tree -r --name-only "$baseline_commit" -- planr-routing \
  | sort > "$actual_sources"

awk -F '\t' 'NR > 1 && $1 == "source-file" { print $3 }' "$manifest" \
  | sort > "$manifest_sources"

comm -23 "$actual_sources" "$manifest_sources" > "$missing_sources"
if [ -s "$missing_sources" ]; then
  echo "migration manifest omits frozen tracked planr-routing files:" >&2
  cat "$missing_sources" >&2
  exit 1
fi

comm -13 "$actual_sources" "$manifest_sources" > "$unexpected_sources"
if [ -s "$unexpected_sources" ]; then
  echo "migration manifest has unexpected source-file rows absent from frozen tracked tree:" >&2
  cat "$unexpected_sources" >&2
  exit 1
fi

if awk -F '\t' 'NR > 1 && $3 ~ /[*?]/ { print $0; found = 1 } END { exit found ? 0 : 1 }' "$manifest" > "$tmpdir/wildcards.txt"; then
  echo "migration manifest contains wildcard source entries:" >&2
  cat "$tmpdir/wildcards.txt" >&2
  exit 1
fi

check_entry() {
  type="$1"
  source="$2"
  if ! awk -F '\t' -v type="$type" -v source="$source" \
    'NR > 1 && $1 == type && $3 == source { found = 1 } END { exit found ? 0 : 1 }' "$manifest"; then
    echo "migration manifest missing $type entry: $source" >&2
    exit 1
  fi
}

check_entry cli-command "planr-routing policy list"
check_entry cli-command "planr-routing policy show <policy> --host <host>"
check_entry cli-command "planr-routing compile <policy> --host <host>"
check_entry cli-command "planr-routing probe <host>"
check_entry cli-command "planr-routing evaluate <policy> --host <host>"
check_entry cli-command "planr-routing catalog build"
check_entry cli-command "planr-routing catalog verify <file>"
check_entry cli-command "planr-routing registry sign <file> --signer <id> --private-key-file <path> --output <path>"
check_entry cli-command "planr-routing registry verify <file> --signature <path> --trusted-signer <id> --trusted-public-key-file <path>"

check_entry generated-artifact ".planr/agents.toml"
check_entry generated-artifact ".planr/policy.toml"
check_entry generated-artifact ".codex/agents/planr-worker.toml"
check_entry generated-artifact ".codex/agents/planr-reviewer.toml"
check_entry generated-artifact ".claude/agents/planr-worker.md"
check_entry generated-artifact ".claude/agents/planr-reviewer.md"
check_entry generated-artifact ".cursor/agents/planr-worker.md"
check_entry generated-artifact ".cursor/agents/planr-reviewer.md"

echo "migration manifest covers $(wc -l < "$actual_sources" | tr -d ' ') frozen planr-routing files plus required commands and generated artifacts"
