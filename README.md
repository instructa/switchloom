# Switchloom

**Deterministic model routing for coding agents.**

Switchloom is a standalone policy compiler and repository-safe host artifact manager for agentic coding environments. It owns routing definitions, bundle schemas, model/effort/fork choices, catalog evidence, signatures, and host-specific artifacts for Codex, Claude Code, Cursor, OpenCode, Pi, and mixed-host setups. The primary command is `switchloom`; `model-routing` remains available as a compatibility alias.

Planr integration is optional. The package graph must build without Planr dependencies, and standalone operation is the default.

## Install

### Homebrew

```sh
brew install instructa/tap/switchloom
```

### npm

```sh
npm install --global switchloom@0.3.1
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

## Setup from the website

The generator at [switchloom.ai](https://switchloom.ai) produces only the versioned
`SetupSpecV1` transport. It does not compile or write host files in the browser. Its primary
action copies a shell-safe command like:

```sh
npx switchloom@0.3.1 preview --recipe 'sw1_...'
npx switchloom@0.3.1 apply --recipe 'sw1_...'
```

The secondary action downloads the same setup as a readable `.switchloom/config.toml`:

```sh
switchloom preview --config .switchloom/config.toml
switchloom apply --config .switchloom/config.toml
switchloom status
switchloom update
switchloom rollback
switchloom uninstall
```

Setup-backed apply previews the exact repository-local change set and asks for confirmation.
Use `--yes` only in an explicitly non-interactive workflow. Standalone mode never emits
`.planr`; optional Planr mode emits provider-neutral Planr declarations and thin native roles,
while Switchloom remains independent of Planr at build and runtime.

For direct CLI use, compile a bundle and run the same lifecycle against a
repository:

```sh
switchloom compile balanced --host codex-openai --output routing-bundle.json
switchloom preview routing-bundle.json --repository .
switchloom apply routing-bundle.json --repository .
switchloom doctor codex
```

`switchloom doctor <host>` is the install/version check for `codex`, `cursor`,
`claude-code`, `opencode`, `pi`, or a concrete binding id. For Codex it also
reports exact 0.145.0 support, repository-local V2 activation conflicts, and
trust/reload guidance without mutating host state. Run it before preview and
apply when setup depends on a locally installed host CLI.

## Current Status

The v0.3.1 public CLI compiles independently, preserves the frozen
Planr v1.5.0 routing inventory in [docs/migration-baseline.md](docs/migration-baseline.md),
and hard-cuts maintainer-only evaluation, catalog, registry, and live-verification
operations from the public command surface.

Current Planr handoff rules, runtime classes, and maintainer verification gates are in
[docs/model-routing-policy.md](docs/model-routing-policy.md). Planr consumes
Switchloom semantic-role declarations; it must not duplicate the model, effort,
host adapter, fork policy, catalog, website compiler, or artifact lifecycle.

## Baseline Commands

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo run -- --version
cargo run -- policy list
```

## Website Generator

The static Astro website is an above-the-fold team generator built with React and shadcn. Users first choose standalone or optional Planr integration, then choose Codex, Cursor, Claude Code, OpenCode, or Pi, select up to four explicit roles, start from a Light, Balanced, or High team preset, and optionally override each role's model and reasoning effort. The primary result is a CLI recipe; the secondary result is a readable setup config. Only the Rust CLI compiles and applies project-native files. Codex is shown as an internal V2 thread-tree path; Cursor, Claude Code, and OpenCode are native subagent paths; Pi is an external runner path; separate app tasks are not treated as Codex V2 child threads. The host remains authoritative for model availability, execution, and billing.

Claude Code model and effort options are derived at build time from the canonical catalog produced by the Rust compiler. Codex mirrors its current desktop picker: `low`, `medium`, `high`, and `xhigh`, while Terra and Sol additionally expose `ultra` as a manual-only mode. Pure `max` is intentionally omitted because the desktop picker does not expose it separately; Ultra sends Max reasoning plus automatic multi-agent delegation. Light, Balanced, and High never select Ultra, and Codex defaults keep mechanical work on certified Terra rather than Luna. Luna remains selectable only as an explicit experimental/unverified choice until authentic Codex 0.145.0 V2 evidence is independently reviewed. Cursor uses a deliberately small, researched frontier allowlist because its full picker changes frequently; the website presents those models in a searchable selector. Generated custom setups are local and unverified until the user reviews them.

```sh
cargo run -p xtask -- release prepare --allow-dirty
cargo run -p xtask -- release verify --inventory-only
pnpm site:check
pnpm site:dev
```

The website setup contract is equivalent to the CLI lifecycle: the copied
`npx switchloom@0.3.1 apply --recipe 'sw1_...' --repository .` command and the
downloadable `.switchloom/config.toml` both replay through CLI preview/apply
before any repository-local artifact is written.

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
commit a bracketed changelog section such as `## [0.3.1]`, then run:

```sh
RELEASE_DRY_RUN=1 scripts/release.sh 0.3.1 "Codex 0.145 native V2 compatibility"
scripts/release.sh 0.3.1 "Codex 0.145 native V2 compatibility"
```

The script requires a clean, synchronized `main`, runs the complete local
quality and security gates, creates an annotated tag, and pushes it. The tag
workflow builds macOS and Linux archives, publishes aggregate SHA-256
checksums, and makes the GitHub release public only after every build succeeds.
