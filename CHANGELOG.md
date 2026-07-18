# Changelog

All notable changes to Switchloom are recorded here.

## [0.2.0] - 2026-07-18

- Added the strict, versioned `SetupSpecV1` contract with deterministic TOML, canonical JSON,
  bounded `sw1_` recipes, and one compiler path for built-in and custom compositions.
- Added `switchloom` as a native binary and config/recipe-driven preview, apply, update,
  status, rollback, and uninstall flows backed by repository-local `.switchloom/config.toml`.
- Replaced browser-owned ZIP generation with copied CLI recipe commands and readable setup
  config downloads for standalone and optional Planr integration.
- Added exhaustive website-to-Rust contract parity, fresh-repository lifecycle oracles,
  deployed desktop/mobile checks, and authenticated Codex child execution receipts.
- Hardened confirmation against config, repository-content, and repository-symlink TOCTOU,
  rejected identity/path collisions, and bounded untrusted recipe decoding before allocation.

## [0.1.1] - 2026-07-17

- Added installable npm CLI packaging for `switchloom` and `model-routing` commands.
- Added Homebrew formula generation and release-channel automation.
- Documented npm, Homebrew, and direct archive installation.

## [0.1.0] - 2026-07-17

- Established the independent repository baseline, package metadata, CI and release automation, local-state policy, and Planr v1.5.0 extraction inventory.
- Transferred the public website, catalog regeneration, and Alchemy/Cloudflare publication stack to standalone Switchloom with explicit optional Planr integration controls.
- Added deterministic repository-safe lifecycle management, host bindings, signed catalog metadata, security guardrails, and release packaging.
