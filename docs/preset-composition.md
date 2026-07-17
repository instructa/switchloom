# Preset Composition

A preset combines one usage policy with one host binding. Composition is deterministic: the same policy, host, package version, and integration mode produce byte-identical JSON.

The compiler owns:

- Profiles, model identifiers, reasoning effort, cost tier, and agent type names.
- Routes and fallbacks.
- Host-native repository artifacts.
- Artifact hashes and catalog metadata.

Standalone bundles omit `.planr` declarations and generated workflow skills. `--integration planr` adds only `.planr/agents.toml` and `.planr/policy.toml` declarations alongside native host roles. The Planr registry uses Planr-supported profile fields and keys Codex profiles by their generated native agent type names, so Planr pick and route receipts expose the same opaque role names that Codex dispatch consumes. In Planr mode, worker and reviewer role instructions preload Planr's existing internal `$planr-work` and `$planr-review` protocols; users still enter through `$planr-goal` and `$planr-loop`, and no routing-specific workflow skill is generated.

Repository lifecycle commands validate the bundle before touching disk and operate only on repository-local host artifact paths:

```sh
model-routing preview routing-bundle.json --repository .
model-routing apply routing-bundle.json --repository .
model-routing update routing-bundle.json --repository .
model-routing status --repository .
model-routing uninstall --repository .
model-routing rollback --repository .
```

Managed state is stored in `.model-routing/manifest.json`. Apply refuses reserved/global paths, traversal, symlink parents, duplicate targets, parent/child target collisions, and existing files with different content. Apply and update stage all artifacts plus the manifest before committing; if a commit step fails, already committed targets are rolled back. Transaction journals are written as synced temp files and atomically renamed before mutations. Before any lifecycle command reads or writes managed state, it recovers leftover `.model-routing/txn-*` journals from interrupted transactions back to the previous coherent state and removes orphan transaction data. If immediate rollback or later recovery cannot restore a file, the command returns an error and leaves the transaction directory plus backups for a later recovery attempt.

Update is manifest-aware: unchanged managed files stay in place, files still matching the previous manifest can be replaced by the new bundle, and previous-only files are transactionally removed when they still match the old manifest. User-modified or missing previous-only files are preserved with repair guidance and residual ownership in the JSON report. Rollback applies the same rule in reverse: it restores the previous manifest snapshot and removes current-only managed files when their hashes still match.

Uninstall removes only files that still match the manifest hash. If any managed file is user-modified or missing, uninstall keeps a residual manifest so `status` can continue reporting ownership and repair state.
