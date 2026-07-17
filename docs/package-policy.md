# Package And Ignore Policy

The repository must be safe to publish from a dirty local coordination environment.

## Excluded From Git And Packages

- `.planr/planr.sqlite`, SQLite sidecars, transient Planr logs, and local receipts.
- `.claude/`, `.codex/`, and `.cursor/` host-local state.
- Credentials, private keys, `.env` files except `.env.example`, generated reports, and build output.
- Regenerated website/package output such as `dist/`, `coverage/`, `tmp/`, and `.crate` files.

The policy is enforced by `.gitignore`, `Cargo.toml` `exclude`, and the CI package-content audit.

## Publishable Inputs

Only source, fixtures, docs, CI metadata, and deterministic generator inputs should enter the package. Live verification receipts and authenticated-host evidence belong in release notes or reviewed handoff docs after secret scrubbing, not in the crate payload.
