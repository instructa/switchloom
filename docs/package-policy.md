# Package And Ignore Policy

The repository must be safe to publish from a dirty local coordination environment.

## Excluded From Git And Packages

- `.planr/planr.sqlite`, SQLite sidecars, transient Planr logs, and local receipts.
- `.claude/`, `.codex/`, and `.cursor/` host-local state.
- Credentials, private keys, `.env` files except `.env.example`, generated reports, and build output.
- Regenerated website/package output such as `dist/`, `coverage/`, `tmp/`, and `.crate` files.
- Historical migration, handoff, and release records under `retained-evidence/`.

The policy is enforced by `.gitignore`, `Cargo.toml` `exclude`, and the CI package-content audit.

## Publishable Inputs

The npm tarball contains only package metadata, README, LICENSE, the launcher,
and supported native binaries. The Cargo source package additionally retains
the versioned Codex runtime evidence embedded by `src/evidence.rs`, plus source,
fixtures, current maintainer docs, CI metadata, and deterministic generator
inputs. Live verification receipts and authenticated-host evidence belong in
reviewed retained records after secret scrubbing, not in either payload.

## Documentation Owners

README and switchloom.ai own end-user setup and usage. Current maintainer
contracts remain in `docs/`; immutable runtime inputs live under `evidence/`;
historical migration, handoff, and release records live under
`retained-evidence/`.

## v0.3.1 Public Boundary

The published CLI owns `policy`, `compile`, `inspect`, `preview`, `apply`,
`update`, `status`, `rollback`, `uninstall`, and `doctor`. Maintainer verification,
catalog generation, and release packaging remain in the unpublished `xtask`
crate. Offline evaluation and registry signing remain library APIs.
