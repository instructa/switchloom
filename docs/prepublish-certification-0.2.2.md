# Switchloom 0.2.2 Pre-Publish Certification

Date: 2026-07-21
Planr item: `i-pass-security-and-immutable-pre-bb8d`

## Candidate

- Candidate version: `0.2.2`
- Crate digest: `sha256:0e606445c036f71d79f781314a21089d667513d1b47e3cf02ee1e1fbc6571045`
- Local release binary digest: `sha256:cbc7538a742365022808c4a1bfe9375402afec4869d8e2aa0fe3e7b6465de1ab`
- Local release archive: `dist/switchloom-darwin-arm64.tar.gz`
- Package self-reference guard: `docs/prepublish-certification-*.md` is excluded
  from the Cargo package so the report can name final crate bytes without being
  embedded in those bytes.
- Planr handoff reference: `docs/model-routing-policy.md`, section
  `Planr Consumer Handoff`, is the current versioned consumer boundary for
  semantic roles, host/runtime ownership, and `fork_turns`. The prior public
  release hard-cut receipt remains `docs/planr-hard-cut-handoff.md`.

## Passed Gates

- `cargo build --release --locked`
- `node scripts/regenerate-preset-catalog.mjs --routing-bin target/release/model-routing`
- `cargo run --quiet --bin model-routing -- compile balanced --host codex-openai --integration planr --output fixtures/routing-bundle-v1/valid-balanced-codex.json`
- `cargo run --quiet --bin model-routing -- compile balanced --host mixed-host --integration planr --output fixtures/routing-bundle-v1/valid-balanced-mixed.json`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features` passed 63 library tests and both binary test targets.
- `CI=true pnpm site:check` passed 4 Vitest files, 27 Vitest tests, 5 Node tests, Astro check, and static build.
- `cargo package --locked --allow-dirty --no-verify` packaged 161 files as `model-routing v0.2.2`.
- `tar -tf target/package/model-routing-0.2.2.crate | rg 'prepublish-certification|Cargo.toml|README.md'` showed `Cargo.toml`, `Cargo.toml.orig`, and `README.md`; no prepublish certification report is included.
- `bash -n scripts/release.sh scripts/build-release.sh scripts/npm-pack-check.sh scripts/secleak-check.sh scripts/codex-standalone-oracle.sh scripts/native-host-certification-oracle.sh`
- `git diff --check`
- `git check-ignore -v reports/native-host-certification/current/certification-summary.json dist/website/index.html target/release/model-routing npm/native/darwin-arm64/model-routing .planr`
- `/Users/kregenrek/.agents/skills/secleak-check/scripts/secleak-check.sh` passed after removing generated `node_modules`; BetterLeaks found no leaks and Trivy reported zero Cargo vulnerabilities.
- `scripts/build-release.sh`
- `target/release/model-routing --version`
- `SWITCHLOOM_NATIVE_BIN="$PWD/target/release/model-routing" node npm/bin/model-routing.js --version`

## Runtime Receipts

Codex V2 packaged-crate oracle:

- Command: `scripts/codex-standalone-oracle.sh`
- Receipt root: `/private/tmp/model-routing-codex-standalone.qKKPk1`
- Host version: `codex-cli 0.144.5`
- Runtime evidence: `/private/tmp/model-routing-codex-standalone.qKKPk1/codex-runtime-evidence.json`
- Implementer route: `model_routing_terra_high`, `gpt-5.6-terra`, `high`, `fork_turns = none`
- Reviewer route: `model_routing_sol_high`, `gpt-5.6-sol`, `high`, `fork_turns = none`
- Dynamic nonces:
  - `019f8393-575d-7231-9f6f-7e8e2c14ea79:019f8393-73d5-7a53-a140-09d3421d9a73:call_lnUB8ajXVWsLnRo1GwNTnOY0`
  - `019f8393-575d-7231-9f6f-7e8e2c14ea79:019f8393-7e3a-7f13-8996-b0b76631dd75:call_aqAkF2xUCw9uqWgCAwmCAf7q`
- Isolated lifecycle `CODEX_HOME` config hash stayed equal before, preview, apply, update, rollback, and uninstall:
  `cffe9654848fcee350377eabaf834f619900c74e79f9dc2ad0bed671aeb3d983`
- Authenticated current `CODEX_HOME` config hash stayed equal before and after
  the live Codex run:
  `bf3893f2a38de141286a4993c40e9c52729463111c8093a8ac863ec16cfeca49`.
  The oracle removed only its transient Codex trust entry for
  `/private/tmp/model-routing-codex-standalone.qKKPk1` after verifying the
  stripped config matched the pre-run file. It recorded no global Switchloom
  agent registrations before or after.

Cursor OpenAI oracle:

- Command: `scripts/native-host-certification-oracle.sh cursor-openai target/release/model-routing`
- Report: `reports/native-host-certification/cursor-openai/20260720T221626Z`
- Host version: `2026.07.17-3e2a980`
- Package digest: `sha256:cbc7538a742365022808c4a1bfe9375402afec4869d8e2aa0fe3e7b6465de1ab`
- Requested model: `gpt-5.4-mini`
- Nonce: `94974c78-b3d9-4a4f-bcc0-641fcdb0c14d`
- Verdict: `advisory`; Cursor did not return host-authenticated effective model or effort telemetry.

Cursor Fable/Grok oracle:

- Command: `scripts/native-host-certification-oracle.sh cursor-fable-grok target/release/model-routing`
- Report: `reports/native-host-certification/cursor-fable-grok/20260720T221641Z`
- Host version: `2026.07.17-3e2a980`
- Package digest: `sha256:cbc7538a742365022808c4a1bfe9375402afec4869d8e2aa0fe3e7b6465de1ab`
- Requested model: `cursor-grok-4.5-medium`
- Nonce: `210cee72-0435-4c8b-9962-4692aea17871`
- Verdict: `advisory`; Cursor did not return host-authenticated effective model or effort telemetry.

Claude Code:

- Installed version: `2.1.133 (Claude Code)`
- Verdict: skipped/unverified.
- Reason: live Claude probe escalation was rejected for authenticated external CLI risk, and current plan context treats Claude Code as unavailable rather than a release-blocking live gate.

## Open Blocker

`bash scripts/npm-pack-check.sh` still fails:

```text
package version 0.2.2 not found in npm/native/darwin-arm64/model-routing
```

The ignored local `npm/native/*/model-routing` binaries are still `0.2.1`.
The `0.2.2` npm/native packed-byte certification cannot pass until the
release-candidate matrix rebuilds all four platform binaries and regenerates
`npm/native/provenance.json` for `0.2.2`. Do not publish from the current local
`npm/native` tree.

The authenticated Codex global-config equality gate now passes in receipt
`/private/tmp/model-routing-codex-standalone.qKKPk1`.
