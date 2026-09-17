# Ownership

Codex Desktop owns persistent tasks, execution and tool loops. Switchloom
prepares assignments and the agent dispatches them; no waiting runtime exists.

- `catalog.toml`: model suggestions, effort choices, ordered card defaults,
  capability instructions and default owners. `src/catalog.rs` reads and
  validates it; the website consumes its generated JSON.
- `website/src/lib/generator.ts`: board state, unique ownership, disable/restore
  transitions and one bootstrap prompt. `Generator.tsx` renders and edits it.
- The selected workflow: per-slot model/effort/capabilities. Every handoff carries
  them; neither the router nor the skill reapplies defaults during execution.
- `src/decision.rs`: explicit capability selection, Jev choice and abstention.
  Capability criteria come from the catalog, without a second policy table.
- `src/handoff.rs`: validates task targets and binds the configured owner to the
  Codex dispatch shape. `src/typesafe.rs`: credentials and bounded HTTP only.
- `skills/switchloom/SKILL.md`: resolves IDs, checks readiness, invokes task tools
  and resumes from substantive results. Vanilla uses the prompt's same ownership
  map directly, without a classifier or an alternative model policy.

Model syntax and reasoning validation live in `catalog.rs`; the UI uses the
generated choices for immediate feedback. Codex owns actual availability.
Keep fixes with these owners. No role adapters, live model updater, tool router,
settings database or compatibility translator is needed.

## Retained persisted-data boundary

`src/cleanup.rs` is isolated from routing. `status_repository` and
`uninstall_repository` read `.model-routing/manifest.json` written by earlier
Switchloom installations. They preserve changed files and unrelated Codex
settings. Already missing files lose ownership on uninstall. Recorded
`ownership_content` distinguishes installed agent registrations from feature
flags that were already present in the user's configuration.

`recover_pending_transactions` reads existing `.model-routing/txn-*/journal.json`
files before cleanup. Backups and staging files must stay inside that recorded
transaction; targets must remain managed repository paths. Recovery failures
retain the journal and backups. Paths, symlink parents and file types are
checked before reading or mutating recorded targets.

These manifest/journal readers are the exact exception to the hard cut. They
allow existing user data to be safely cleaned up. They do not accept setup
recipes, generate bundles, install roles or recreate removed integrations.
Previously installed host paths remain allowed only in this cleanup module.
There is no new migration layer.

## Verification and packaging

Clap owns the CLI command declarations. Tests exercise routing, HTTP, CLI
behavior, cleanup and packaging; there is no copied command manifest or test
of module names/import spelling. Documentation filenames are not a release
contract.

`xtask release` owns version checks, catalog regeneration, package inventories
and native provenance. The npm launcher owns selecting and verifying its
native executable. GitHub Actions owns CI execution. Existing secret hooks,
package exclusions and `security:check` remain active.

Alchemy owns website deployment. `site:build` calls Astro directly; there is
no bundle-publication script. The existing Alchemy resource IDs and Worker name
are retained because they identify deployed resources, not supported presets.
