# Ownership

Switchloom owns:

- Routing definitions, bundle schema versions, deterministic composition, canonical hashing, signatures, and evidence labels.
- Model, effort, role, fork, topology, host capability, and usage-profile catalogs.
- Repository-safe inspect, preview, apply, update, rollback, status, and uninstall lifecycle behavior.
- Generated Codex, Claude Code, Cursor, mixed-host, and optional Planr integration artifacts.
- Public website/catalog source and downloadable bundle parity.

Planr owns:

- Plans, maps, picks, reviews, approvals, audit workflow, and run evidence.
- Provider-neutral `.planr` declarations and opaque route resolution.
- Planr user-facing orchestration through `$planr-goal` and `$planr-loop`.

Wrong owners:

- Planr policy/compiler/catalog/apply code after Goal B.
- A generated routing workflow skill.
- A second website compiler or second apply path.
- User/global client configuration.
- Dynamic Rust plugins or compatibility wrappers around deleted Planr routing commands.
