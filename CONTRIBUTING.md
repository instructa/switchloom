# Contributing

Switchloom targets ordinary Codex tasks with Sol, Astra and optional Luna.
TypeSafe/Jev selects the next capability from bounded task context. Keep one owner
for model assignments and routing policy; avoid alternate host integrations.

## Development

Run the smallest check covering the change. For Rust use the owning test or
crate, plus formatting and Clippy when Rust code changes. For website work use
`pnpm site:test`; regenerate the catalog and run `pnpm site:check` when task
capabilities change. Release preparation and packaging
belong to `xtask release`.

Do not commit local execution state, credentials, generated receipts or global
host configuration. Cleanup commands must remain repository-scoped
and preserve unrelated configuration and user edits.

## Ownership

`catalog.toml` owns model suggestions, reasoning choices, model defaults, capabilities
and their instructions. Add new model suggestions there, then run `pnpm catalog:regenerate`.
Custom GPT IDs can be selected without a catalog release. `src/decision.rs`
owns routing decisions; `src/typesafe.rs` owns TypeSafe HTTP. Codex owns the
execution loop and tool discovery. `src/handoff.rs` binds decisions to tasks;
`skills/switchloom/SKILL.md` owns the desktop send/end-turn/return sequence.
See [ownership](docs/ownership.md) before
adding another policy path or configuration layer.
