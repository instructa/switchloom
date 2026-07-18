#!/bin/sh
set -eu

manifest="${1:-docs/migration-manifest.tsv}"
planr_root="${PLANR_ROOT:-/Users/kregenrek/projects/planr}"
baseline_tag="${PLANR_BASELINE_TAG:-v1.5.0}"
baseline_commit="${PLANR_BASELINE_COMMIT:-7a01ad54cb41fd755f368a79339a96a997f693d0}"
baseline_source_hash="${PLANR_BASELINE_SOURCE_HASH:-d32166f38448fcc2cb5632b24214625b45ae6326}"

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/model-routing-manifest.XXXXXX")"
trap 'rm -rf "$tmpdir"' EXIT

actual_sources="$tmpdir/actual-sources.txt"
manifest_sources="$tmpdir/manifest-sources.txt"
missing_sources="$tmpdir/missing-sources.txt"
unexpected_sources="$tmpdir/unexpected-sources.txt"
active_planr_files="$tmpdir/active-planr-routing-files.txt"
duplicate_rows="$tmpdir/duplicate-rows.txt"

find_active_planr_files() {
  root="$1"
  rg -il 'planr[- ]routing|routing[_ -]bundles?|routingbundle' "$root" 2>/dev/null \
    | sed "s#^$root/##" \
    | awk '
      $0 ~ /^\.git\// { next }
      $0 ~ /^\.planr\// { next }
      $0 ~ /^\.alchemy\// { next }
      $0 ~ /^\.pnpm-store\// { next }
      $0 ~ /^node_modules\// { next }
      $0 ~ /^target\// { next }
      $0 ~ /^dist\// { next }
      $0 ~ /^planr-routing\// { next }
      { print }
    ' \
    | sort
}

awk -F '\t' 'NR > 1 && $1 == "source-file" { print $3 }' "$manifest" \
  | sort > "$manifest_sources"

source_hash="$(git hash-object "$manifest_sources")"
if [ "$source_hash" != "$baseline_source_hash" ]; then
  echo "migration manifest source inventory hash is $source_hash, expected $baseline_source_hash" >&2
  exit 1
fi

if [ -d "$planr_root/.git" ]; then
  resolved_commit="$(git -C "$planr_root" rev-parse "$baseline_tag")"
  if [ "$resolved_commit" != "$baseline_commit" ]; then
    echo "baseline tag $baseline_tag resolved to $resolved_commit, expected $baseline_commit" >&2
    exit 1
  fi

  git -C "$planr_root" ls-tree -r --name-only "$baseline_commit" -- planr-routing \
    | sort > "$actual_sources"

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
fi

if awk -F '\t' 'NR > 1 && $3 ~ /[*?]/ { print $0; found = 1 } END { exit found ? 0 : 1 }' "$manifest" > "$tmpdir/wildcards.txt"; then
  echo "migration manifest contains wildcard source entries:" >&2
  cat "$tmpdir/wildcards.txt" >&2
  exit 1
fi

if awk -F '\t' 'NR > 1 { key = $1 "\t" $3; count[key]++; rows[key] = rows[key] "\n" $0 } END { for (key in count) if (count[key] > 1) { print rows[key]; found = 1 } exit found ? 0 : 1 }' "$manifest" > "$duplicate_rows"; then
  echo "migration manifest contains duplicate type/source mappings:" >&2
  cat "$duplicate_rows" >&2
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

check_entry_with_target() {
  type="$1"
  source="$2"
  target="$3"
  if ! awk -F '\t' -v type="$type" -v source="$source" -v target="$target" \
    'NR > 1 && $1 == type && $3 == source && $5 == target { found = 1 } END { exit found ? 0 : 1 }' "$manifest"; then
    echo "migration manifest missing $type entry: $source -> $target" >&2
    exit 1
  fi
}

check_absent_entry() {
  type="$1"
  source="$2"
  if awk -F '\t' -v type="$type" -v source="$source" \
    'NR > 1 && $1 == type && $3 == source { found = 1 } END { exit found ? 0 : 1 }' "$manifest"; then
    echo "migration manifest contains stale $type entry: $source" >&2
    exit 1
  fi
}

check_no_direct_planr_package_dependency() {
  if grep -Eq '"(@[^"]+/)?planr"[[:space:]]*:|^planr[[:space:]]*=' package.json Cargo.toml Cargo.lock; then
    echo "local package metadata contains a direct Planr dependency" >&2
    exit 1
  fi
}

check_artifact_in_file() {
  artifact="$1"
  file="$2"
  if ! grep -Fq "\"path\": \"$artifact\"" "$file"; then
    echo "current compiler output $file does not contain artifact path: $artifact" >&2
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
check_entry_with_target generated-artifact ".codex/agents/model-routing-luna-xhigh.toml" ".codex/agents/model-routing-luna-xhigh.toml"
check_entry_with_target generated-artifact ".codex/agents/model-routing-sol-high.toml" ".codex/agents/model-routing-sol-high.toml"
check_entry_with_target generated-artifact ".codex/agents/model-routing-sol-medium.toml" ".codex/agents/model-routing-sol-medium.toml"
check_entry_with_target generated-artifact ".codex/agents/model-routing-sol-ultra.toml" ".codex/agents/model-routing-sol-ultra.toml"
check_entry_with_target generated-artifact ".codex/agents/model-routing-terra-high.toml" ".codex/agents/model-routing-terra-high.toml"
check_entry_with_target generated-artifact ".codex/agents/model-routing-terra-medium.toml" ".codex/agents/model-routing-terra-medium.toml"
check_entry_with_target generated-artifact ".claude/agents/model-routing-preset-worker.md" ".claude/agents/model-routing-preset-worker.md"
check_entry_with_target generated-artifact ".cursor/agents/model-routing-preset-worker.md" ".cursor/agents/model-routing-preset-worker.md"

check_artifact_in_file ".planr/agents.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".planr/policy.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".codex/agents/model-routing-luna-xhigh.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".codex/agents/model-routing-sol-high.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".codex/agents/model-routing-sol-medium.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".codex/agents/model-routing-sol-ultra.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".codex/agents/model-routing-terra-high.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".codex/agents/model-routing-terra-medium.toml" "fixtures/routing-bundle-v1/valid-balanced-codex.json"
check_artifact_in_file ".claude/agents/model-routing-preset-worker.md" "website/data/bundles/balanced-claude-native.json"
check_artifact_in_file ".cursor/agents/model-routing-preset-worker.md" "website/data/bundles/balanced-cursor-openai.json"

check_absent_entry generated-artifact ".codex/agents/planr-worker.toml"
check_absent_entry generated-artifact ".codex/agents/planr-reviewer.toml"
check_absent_entry generated-artifact ".claude/agents/planr-worker.md"
check_absent_entry generated-artifact ".claude/agents/planr-reviewer.md"
check_absent_entry generated-artifact ".cursor/agents/planr-worker.md"
check_absent_entry generated-artifact ".cursor/agents/planr-reviewer.md"

if [ -d "$planr_root" ]; then
  find_active_planr_files "$planr_root" > "$active_planr_files"

  while IFS= read -r active_file; do
    [ -n "$active_file" ] || continue
    check_entry planr-consumer "$active_file"
  done < "$active_planr_files"
fi

check_no_direct_planr_package_dependency

echo "migration manifest covers $(wc -l < "$manifest_sources" | tr -d ' ') frozen planr-routing files plus required commands, active Planr consumer mappings for case-insensitive routing variants, current generated artifacts, and no direct Planr package dependency"
