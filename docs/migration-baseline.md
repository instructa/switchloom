# Migration Baseline

This document records the immutable extraction baseline for the standalone Switchloom repository.

## Frozen Source

- Source repository: `/Users/kregenrek/projects/planr`
- Frozen tag: `v1.5.0`
- Frozen commit: `7a01ad54cb41fd755f368a79339a96a997f693d0`
- Baseline command: `git -C /Users/kregenrek/projects/planr rev-list -n 1 v1.5.0`
- Current standalone repository rule: do not edit `/Users/kregenrek/projects/planr` product code during Goal A.

## Ownership Disposition Manifest

The one-owner migration inventory is machine-checkable in
[`docs/migration-manifest.tsv`](migration-manifest.tsv). Each row has an
explicit type, stable id, frozen source or command/artifact identifier,
disposition, target owner/path, and notes. The manifest intentionally avoids
wildcard rows so fixtures, scripts, website files, website tests, CLI commands,
and generated artifact paths are reviewed individually.

The manifest covers:

- `43` frozen tracked `planr-routing` source files from the Planr v1.5.0 Git
  tree.
- `6` current generated/untracked `planr-routing` paths that are intentionally
  inventoried separately from frozen source.
- `16` Planr-side routing consumer, test, and plugin-role paths that Goal B must
  keep, split, replace, or delete.
- `9` frozen CLI command surfaces: `policy list`, `policy show`, `compile`,
  `probe`, `evaluate`, `catalog build`, `catalog verify`, `registry sign`, and
  `registry verify`.
- `8` generated repository artifact paths, including `.planr/agents.toml`,
  `.planr/policy.toml`, and host-native Codex, Claude Code, and Cursor role
  artifacts.

Run the comparison check with:

```sh
sh scripts/check-migration-manifest.sh
```

The check verifies that tag `v1.5.0` resolves to
`7a01ad54cb41fd755f368a79339a96a997f693d0`, compares every frozen tracked
`planr-routing` file from that Git tree to the `source-file` rows in both
directions, rejects wildcard source entries, and verifies all required CLI
command and generated artifact rows are present.

## Baseline Proof

The initial standalone package has no third-party or Planr dependencies. Verify with:

```sh
cargo metadata --no-deps --format-version 1
cargo tree --no-dedupe
sh scripts/check-migration-manifest.sh
```

Both commands should show only the `model-routing` package until later slices intentionally add standalone dependencies.
