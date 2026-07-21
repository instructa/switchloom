# Preset Registry

Maintainers generate and verify catalog data for the CLI and public website from
the same package-owned compiler path:

```sh
cargo run -p xtask -- release prepare --allow-dirty
cargo run -p xtask -- release verify --inventory-only
```

Detached Ed25519 signing and verification are owned by the `model-routing`
library API. They are not standalone public CLI operations in v0.3.0.

Unsigned catalog entries remain experimental.

The public website uses this catalog at build time for Codex and Claude Code model, effort, and cost-tier options. Cursor is the narrow exception: its model picker changes frequently, so the generator exposes a reviewed frontier allowlist rather than every historical catalog profile. The current list was reviewed on 2026-07-17 against [CursorBench](https://cursor.com/de/cursorbench), Cursor's model documentation, and announcements for [Composer 2.5](https://cursor.com/changelog/composer-2-5) and [Grok 4.5](https://cursor.com/blog/grok-4-5). CursorBench is the source of truth for the exposed per-model reasoning levels; GPT-5.6 Luna, Terra, and Sol each expose Low, Medium, High, Extra High, and Max.

The generator lets a user assemble up to four explicit roles and download host-native project files. Light, Balanced, and High are transparent UI starting points that set every role at once; changing any model or effort switches the UI to Custom. Light maps to the hosts' actual `low` effort value—there is no host value named `light`. Codex Ultra remains a manual-only mode because it enables automatic multi-agent delegation; no preset selects it. Official and experimental catalog entries come from repository-owned inputs; signed and recommended states require trusted signatures and current evaluation evidence. Generated custom setups remain local/user-owned and unverified until reviewed.

Kimi K3 was reviewed on 2026-07-18 after its initial release. Moonshot currently exposes `kimi-k3` with Max thinking only and says Low and High will follow. It is not yet native in Cursor, Codex requires machine-local provider configuration that a repository ZIP cannot safely supply, and Claude Code routes its entire process through the Moonshot compatibility endpoint rather than selecting Kimi independently per role. Kimi K3 therefore remains pending instead of appearing as a selectable model that would generate incomplete host configuration.
