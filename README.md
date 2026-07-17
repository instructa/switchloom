# Switchloom

**Deterministic model routing for coding agents.**

Switchloom is a standalone policy compiler and repository-safe host artifact manager for agentic coding environments. It owns routing definitions, bundle schemas, model/effort/fork choices, catalog evidence, signatures, and host-specific artifacts for Codex, Claude Code, Cursor, and mixed-host setups. The command-line interface remains `model-routing`.

Planr integration is optional. The package graph must build without Planr dependencies, and standalone operation is the default.

## Current Status

The v0.1.0 standalone baseline compiles independently and records the frozen Planr v1.5.0 routing inventory in [docs/migration-baseline.md](docs/migration-baseline.md).

## Baseline Commands

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo run -- --version
cargo run -- baseline
```

## Public Catalog

The static website is generated from the same canonical compiler output used by the CLI. Standalone Switchloom is the default; Planr is an optional integration mode shown as an explicit website control.

```sh
cargo run -- catalog build --output website/data/catalog.json
cargo run -- catalog verify website/data/catalog.json
node --test website/*.test.mjs
node scripts/build-site.mjs
```

The Cloudflare/Alchemy publication stack is repo-owned. Test deployments are pinned to the `test` stage:

```sh
node scripts/cloudflare-test.mjs deploy
node scripts/cloudflare-test.mjs destroy
```

## Repository Policy

Local Planr coordination state, credentials, receipts, generated reports, and build artifacts are ignored and excluded from published packages. See [docs/package-policy.md](docs/package-policy.md).

Install the repository-owned staged-file guard once per clone and run the full
secret, vulnerability, and misconfiguration scan before publishing:

```sh
pnpm hooks:install
pnpm security:check
```

## Releases

Releases are created only through the repository-owned script. Prepare and
commit a bracketed changelog section such as `## [0.1.0]`, then run:

```sh
RELEASE_DRY_RUN=1 scripts/release.sh 0.1.0 "Initial standalone release"
scripts/release.sh 0.1.0 "Initial standalone release"
```

The script requires a clean, synchronized `main`, runs the complete local
quality and security gates, creates an annotated tag, and pushes it. The tag
workflow builds macOS and Linux archives, publishes aggregate SHA-256
checksums, and makes the GitHub release public only after every build succeeds.
