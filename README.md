# Switchloom

**Deterministic model routing for coding agents.**

Switchloom is a standalone policy compiler and repository-safe host artifact manager for agentic coding environments. It owns routing definitions, bundle schemas, model/effort/fork choices, catalog evidence, signatures, and host-specific artifacts for Codex, Claude Code, Cursor, and mixed-host setups. The command-line interface remains `model-routing`.

Planr integration is optional. The package graph must build without Planr dependencies, and standalone operation is the default.

## Install

### Homebrew

```sh
brew install instructa/tap/switchloom
```

### npm

```sh
npm install --global switchloom
```

Both channels install the branded `switchloom` command and the compatibility
alias `model-routing`:

```sh
switchloom --version
model-routing --version
```

### Release archive

Download the archive for your platform from the
[latest GitHub release](https://github.com/instructa/switchloom/releases/latest),
verify it against `SHA256SUMS`, and place `model-routing` on your `PATH`.

## Current Status

The v0.1.1 standalone baseline compiles independently and records the frozen Planr v1.5.0 routing inventory in [docs/migration-baseline.md](docs/migration-baseline.md).

## Baseline Commands

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo run -- --version
cargo run -- baseline
```

## Website Generator

The static Astro website is an above-the-fold team generator built with React and shadcn. Users choose Codex, Cursor, or Claude Code, select up to four explicit roles, start from a Light, Balanced, or High team preset, and optionally override each role's model and reasoning effort before downloading project-native agent files as a ZIP. The host remains authoritative for model availability, execution, and billing.

Claude Code model and effort options are derived at build time from the canonical catalog produced by the Rust compiler. Codex mirrors its current desktop picker: `low`, `medium`, `high`, and `xhigh`, while Terra and Sol additionally expose `ultra` as a manual-only mode. Pure `max` is intentionally omitted because the desktop picker does not expose it separately; Ultra sends Max reasoning plus automatic multi-agent delegation. Cursor uses a deliberately small, researched frontier allowlist because its full picker changes frequently; the website presents those models in a searchable selector. Generated custom setups are local and unverified until the user reviews them.

```sh
cargo run -- catalog build --output website/data/catalog.json
cargo run -- catalog verify website/data/catalog.json
pnpm site:check
pnpm site:dev
```

The Cloudflare/Alchemy publication stack is repo-owned and requires Node.js 22 or newer. Test deployments are pinned to the `test` stage; production publishes the custom `switchloom.ai` domain only from the explicit `prod` stage:

```sh
node scripts/cloudflare-test.mjs deploy
node scripts/cloudflare-test.mjs destroy
pnpm exec alchemy deploy --stage prod
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
