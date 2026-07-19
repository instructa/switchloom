# Planr Hard-Cut Handoff Receipt

Date: 2026-07-19

This receipt is the Goal A gate for starting the Planr Goal B hard cut. It
does not modify `/Users/kregenrek/projects/planr`; it records the independently
published Switchloom release and the exact Planr routing responsibilities that
now have a standalone owner or an explicit deletion action.

## Superseding Release Status

The public `v0.2.0` release had a native Codex discovery evidence gap: the
published package generated repository-local Codex role files, yet the accepted
receipt did not prove the checksum-identified installed candidate executed the
full status/uninstall lifecycle while preserving unmanaged repository content.

The public superseding release is now `v0.2.1`. It replaces `v0.2.0` for the
final Goal A handoff. The canonical retained `0.2.1` candidate proof chain is:

- `/private/tmp/model-routing-codex-standalone.COTXUA` proves exact `0.2.1`
  package-byte execution, repository-local Codex role discovery, authentic Terra
  High maker and Sol High reviewer spawns with `fork_turns = none`, and
  unchanged global Codex config hash
  `18cbbaee5e263f8bb011a174913721a8866a4d156d62c7fc74bc2d7103abcb3c`.
- The canonical `0.2.1` candidate crate hash in that receipt is
  `e5c2eee164433f3433f8aef2aca46624d88a46c8b4db502aa63804017b83769e`, and
  the installed binary hash is
  `7e214789885a66e4788c778fd8d7681c1a67de0aed5f728735ff678c8965049f`.
- The same `0.2.1` installed binary ran `status` and `uninstall` only against
  the generated temporary repository. It removed seven Switchloom-managed
  artifacts, left `bundle_id: null` with no managed artifacts on post-status,
  and preserved the unmanaged sentinel file byte-identically.
- `/private/tmp/model-routing-codex-standalone.yGExSi` remains a `0.2.0`
  behavioral precursor only. It is useful for comparing the same proof shape,
  but it is not the canonical `0.2.1` release-candidate proof because its crate
  and installed binary are `0.2.0`.
- `/Users/kregenrek/projects/planr` remained at protected HEAD
  `d7f0afae24643d4a1e475474426c37ca00e5dbe3` with
  `git status --porcelain=v1 -z` SHA-256
  `45d1759e7ff4c3f45b15eaf3c331f82186173610427a5e0a38cdd945151d5f33`.

Do not use the `v0.2.0` publication as the final Goal A handoff. The
superseding public `v0.2.1` bytes are the release gate for Goal B.

## Release Identity

- Repository: `https://github.com/instructa/switchloom`
- Production website: `https://switchloom.ai`
- Release: `https://github.com/instructa/switchloom/releases/tag/v0.2.1`
- Release workflow: `https://github.com/instructa/switchloom/actions/runs/29682448374`
- Tag: `v0.2.1`
- Commit: `56ce22ad33d7c8d2cf7ff1836639ee27ede36d67`
- Commit subject: `release 0.2.1: Superseding native provenance and Codex discovery proof`
- npm package: `switchloom@0.2.1`
- npm tarball: `https://registry.npmjs.org/switchloom/-/switchloom-0.2.1.tgz`
- npm integrity:
  `sha512-vUKHxYXHt7Sx7MkYQz5MRZ0Ll544iHoadHGCgvJPUYkpUzQWtzjt1o3xhyeQwExCA6tuLQ5vZnLPz+fO5uMiXg==`
- npm shasum: `e813283f54d0d64b5fd4835e17687aaaf3b0a6cb`
- Homebrew formula: `instructa/tap/switchloom`, version `0.2.1`

## Release Artifacts

GitHub release `v0.2.1` is public and non-draft. It contains aggregate
`SHA256SUMS` and four platform archives:

| Asset | SHA-256 |
| --- | --- |
| `SHA256SUMS` | `3a72c5a6b4d3ceafda42593b0ceaa08ae36c5d2e908226eadfa2d82873567b7e` |
| `switchloom-darwin-arm64.tar.gz` | `2ec8344ecd38b41d4af47a77f6d5ff7882d4298ea73555d4c7cffa984dcdea0d` |
| `switchloom-darwin-x86_64.tar.gz` | `fc70d0b52c9b0e4932a505debba8d2aa75e0478189574f928fef6fdd312faf5b` |
| `switchloom-linux-arm64.tar.gz` | `561ebc772633f285326c08a3a9b9a8b2316bde0ea99f0d9a32def4a1838bf355` |
| `switchloom-linux-x86_64.tar.gz` | `0ba788c46404892f7bd7c1828eb9a52e9b92b6deba64bf71fed5eb3d39728deb` |

The Homebrew formula points at the same four `v0.2.1` release archives and
uses the matching platform checksums above.

## Package Graph

Switchloom has no Planr build or runtime dependency in either published npm
metadata or the Rust workspace metadata.

| Command | Result |
| --- | --- |
| `npm view switchloom@0.2.1 name version bin dependencies optionalDependencies peerDependencies --json` | Published package is `switchloom` version `0.2.1`, exposes bins `switchloom` and `model-routing` pointing to `npm/bin/model-routing.js`, and reports no runtime, optional, or peer dependency objects. |
| `jq -r '.dependencies // {}, .peerDependencies // {}, .optionalDependencies // {}' package.json` | Local package metadata prints `{}` for each dependency graph. |
| `cargo metadata --format-version 1 --no-deps \| jq -r '.packages[] \| select(.name=="model-routing") \| {name,version,dependencies:[.dependencies[].name],targets:[.targets[] \| {name,kind}]}'` | Rust package is `model-routing` version `0.2.1`; dependencies are `anyhow`, `clap`, `ed25519-dalek`, `serde`, `serde_json`, `sha2`, and `toml`; targets are library `model_routing` plus binaries `model-routing` and `switchloom`. No dependency is named `planr`. |
| `sh scripts/check-migration-manifest.sh` | The checker scans local package metadata and fails on direct `planr` entries in `package.json`, `Cargo.toml`, or `Cargo.lock`. |

## Verification Receipts

| Area | Evidence |
| --- | --- |
| Release publication | Planr item `i-publish-and-smoke-test-the-super-b218` records public `v0.2.1` GitHub release, green Release workflow, npm Trusted Publisher package identity, Homebrew formula identity, production website deployment, and public-byte runtime receipt. |
| Release review | Pending until an independent review closes this final item. |
| Public GitHub check | `gh release view v0.2.1 --repo instructa/switchloom --json tagName,targetCommitish,url,publishedAt,isDraft,isPrerelease,assets` returned public non-draft `v0.2.1` with the five assets listed above. |
| Public workflow check | `gh run view 29682448374 --repo instructa/switchloom --json conclusion,status,url,createdAt,updatedAt,headSha,event,workflowName` returned `conclusion: success`, `status: completed`, `headSha: 56ce22ad33d7c8d2cf7ff1836639ee27ede36d67`. |
| Public npm check | `npm view switchloom@0.2.1 version dist.integrity dist.shasum dist.tarball --json` returned version `0.2.1` and the integrity/shasum/tarball listed above; `npm view switchloom@0.2.1 name version bin dependencies optionalDependencies peerDependencies --json` returned the bin contract and no dependency objects. |
| Public website check | Production deploy to `https://switchloom.ai` exited `0`; `/private/tmp/switchloom-public-0.2.1.MN008L/website-home.headers` and `/private/tmp/switchloom-public-0.2.1.MN008L/website-catalog.headers` record live HTTP `200`, and `/private/tmp/switchloom-public-0.2.1.MN008L/website-live.sha256` records exact catalog and bundle byte equality. |
| Public Homebrew check | `gh api repos/instructa/homebrew-tap/contents/Formula/switchloom.rb --jq '.content' \| base64 --decode` returned formula version `0.2.1`, release archive URLs, matching platform SHA-256 values, and `switchloom --version` formula test. |
| Public-byte runtime check | `/private/tmp/switchloom-public-0.2.1.MN008L` records the exact public npm proof. Fresh repository `/private/tmp/switchloom-public-0.2.1.MN008L/repository-retry-4` installed public `switchloom@0.2.1`; `/private/tmp/switchloom-public-0.2.1.MN008L/npm-tarball.sha1` matches npm shasum `e813283f54d0d64b5fd4835e17687aaaf3b0a6cb`, and `/private/tmp/switchloom-public-0.2.1.MN008L/npm-tarball.sha256` records tarball SHA-256 `028176063ce20b4981aa4e13199b25169b2f8296f648eeeec9291e6955e7549a`. `/private/tmp/switchloom-public-0.2.1.MN008L/retry-4-codex-runtime-evidence.json` validates complete parent `019f79dc-79cf-7342-90ae-f81ff1075e5a` runtime evidence: worker `model_routing_terra_high`, `fork_turns = none`, `gpt-5.6-terra` high; reviewer `model_routing_sol_high`, `fork_turns = none`, `gpt-5.6-sol` high; `/private/tmp/switchloom-public-0.2.1.MN008L/retry-4-validate-runtime-evidence.stdout` says `codex runtime evidence validation passed`. Global Codex config hashes in `retry-4-global-config-before.sha256` and `retry-4-global-config-after.sha256` are identical at `18cbbaee5e263f8bb011a174913721a8866a4d156d62c7fc74bc2d7103abcb3c`. Lifecycle cleanup is recorded by `retry-4-uninstall.json`, which removed only seven Switchloom-managed artifacts; `retry-4-status-after-uninstall.json` reports `bundle_id: null` and no artifacts; unmanaged `user-preserved.toml` survived with SHA-256 `ae9ae1bb9273d0b5f6641c430eb98efc1134a4c25bccf1a0dbf602bea29bd16b` in `retry-4-sentinel-after.sha256`; inventory after uninstall is `retry-4-inventory-after-uninstall.txt`. |
| Website/CLI parity | Planr item `i-prove-website-cli-standal-f9f5` and follow-up fixes record full `site:check`, Cloudflare verification, generated SetupSpec transport, desktop/mobile browser checks, CLI replay, and Planr recipe lifecycle evidence. |
| Authenticated Codex oracles | Planr item `i-pass-offline-safety-website-and-5aac` and follow-up logs record authenticated standalone and Planr-integrated Codex evidence for effective model, effort, role, non-`all` fork policy, Planr loop dispatch, and negative receipt checks. |
| Migration manifest | `sh scripts/check-migration-manifest.sh` verifies `docs/migration-manifest.tsv` covers the frozen Planr routing inventory, legacy command transfers, active Planr consumer/deletion mappings from a case-insensitive whole current Planr repo scan for routing lexical variants, unique type/source mappings, current generated artifact targets, and no direct Planr package dependency. |

## Migration Ownership

The exhaustive mapping is `docs/migration-manifest.tsv`.

- `source-file` rows move or replace frozen `planr-routing/*` source,
  website, fixture, policy, evaluation, and documentation ownership into this
  standalone Switchloom repository.
- `generated-current` rows are deleted and regenerated from standalone
  Switchloom source; dependency install artifacts are never moved or published.
- `cli-command` rows transfer old `planr-routing ...` commands to the
  standalone `model-routing`/`switchloom` command surface.
- `generated-artifact` rows transfer current Switchloom v0.2.1 outputs: optional
  `.planr/agents.toml` and `.planr/policy.toml`; Codex
  `.codex/agents/model-routing-luna-xhigh.toml`,
  `.codex/agents/model-routing-sol-high.toml`,
  `.codex/agents/model-routing-sol-medium.toml`,
  `.codex/agents/model-routing-sol-ultra.toml`,
  `.codex/agents/model-routing-terra-high.toml`, and
  `.codex/agents/model-routing-terra-medium.toml`; Claude Code
  `.claude/agents/model-routing-preset-worker.md`; and Cursor
  `.cursor/agents/model-routing-preset-worker.md`.
- `planr-consumer` rows are the only rows that remain for Goal B. They are
  either Planr-owned neutral orchestration surfaces that stay, or legacy routing
  compiler/catalog surfaces that must be replaced or deleted after this receipt
  is reviewed.

The active Planr files currently covered by `planr-consumer` rows include:
`Cargo.toml`, `Cargo.lock`, `package.json`, `pnpm-workspace.yaml`,
`pnpm-lock.yaml`, `README.md`, `CHANGELOG.md`, `src/routing_bundle.rs`,
`src/routing_bundle/tests.rs`, `src/app/routing.rs`, `src/cli.rs`,
`src/main.rs`, `src/rolefiles.rs`, `src/app/agents.rs`,
`src/app/agents_init.rs`, `tests/e2e.rs`, `docs/MODEL_ROUTING.md`,
`docs/ROUTING_BUNDLES.md`, `docs/MCP_CONTRACT.md`, `docs/CLI_REFERENCE.md`,
`docs/GOALS.md`, `docs/EXAMPLE_WEBAPP.md`, `docs/fixtures/mcp-contract.json`,
`docs/INSTALL.md`, `docs/HOOKS.md`, `docs/ARCHITECTURE.md`,
`docs/documentation/CONTRACT.md`,
`docs/documentation/INFORMATION_ARCHITECTURE.md`, `docs/SKILLS.md`,
`docs/CODEX.md`,
`apps/docs/content/docs/contributing/architecture.mdx`,
`apps/docs/content/docs/reference/cli-generated.mdx`,
`apps/docs/content/docs/reference/cli.mdx`,
`apps/docs/content/docs/reference/configuration-and-storage.mdx`,
`apps/docs/redirects.mjs`,
`apps/docs/scripts/verify-shell.mjs`, and
`plugins/planr/skills/planr-loop/SKILL.md`. The checker derives this list from
the live `/Users/kregenrek/projects/planr` tree by scanning the whole repo
case-insensitively for `planr[- ]routing`,
`routing[_ -]bundles?`, and `routingbundle`, with explicit exclusions for
operational state, generated/dependency directories, and the legacy
`planr-routing/` producer subtree already covered by frozen `source-file` rows.
It fails if any discovered file lacks a manifest row or if any `(type, source)`
pair is duplicated.

## Remaining Planr Surface

After Goal B, Planr should keep only provider-neutral orchestration surfaces:

- Read `.planr/agents.toml` and `.planr/policy.toml` declarations emitted by
  Switchloom optional Planr mode.
- Resolve worker/reviewer routes from those declarations during pick/routing.
- Enforce Planr-owned execution and usage constraints.
- Record declared-versus-effective evidence in Planr workflow logs.
- Keep Planr-owned agent registry, initialization, plugin worker/reviewer roles,
  app policy, execution policy, and usage policy behavior.

Planr must not retain a routing policy compiler, model catalog, preset registry,
host-artifact compiler, website generator, package publisher, or compatibility
wrapper for removed `planr routing bundle` commands.

## Goal B Deletion Oracle

Goal B should run from `/Users/kregenrek/projects/planr` only after this item's
review is complete.

1. Confirm the Switchloom release gate still holds:
   `gh release view v0.2.1 --repo instructa/switchloom`,
   `npm view switchloom@0.2.1 version`, and
   `gh api repos/instructa/homebrew-tap/contents/Formula/switchloom.rb`.
2. Delete or replace every `planr-consumer` row in
   `docs/migration-manifest.tsv` whose disposition is `keep-then-delete`,
   `split`, or `replace`.
3. Keep the `planr-consumer` rows whose disposition is `keep` as Planr-owned
   neutral orchestration and policy code, not Switchloom compiler code.
4. Remove root workspace and package wiring for the legacy producer:
   `Cargo.toml` workspace member, `Cargo.lock` package state, root
   `package.json` scripts, `pnpm-workspace.yaml`, and `pnpm-lock.yaml`.
5. Remove legacy user-facing `planr routing bundle` command paths and any tests
   that require Planr to compile Switchloom-owned bundles internally.
6. Replace Planr bundle/route tests with fixtures produced by
   `switchloom@0.2.1` and assertions that Planr consumes declarations, resolves
   routes, and logs effective evidence.
7. Regenerate Planr CLI docs and generated fixtures so `docs/CLI_REFERENCE.md`,
   `docs/fixtures/mcp-contract.json`,
   `apps/docs/content/docs/reference/cli-generated.mdx`, and
   `apps/docs/scripts/verify-shell.mjs` no longer expose or assert
   `planr routing bundle inspect|preview|apply`.
8. Update `docs/MCP_CONTRACT.md` to remove or rewrite RoutingBundle v1
   inspect/preview/apply capabilities that no longer exist after the hard cut.
9. Update README, CHANGELOG, architecture docs, app docs,
   `configuration-and-storage.mdx`, install/skills/Codex docs, and example docs
   so current routing ownership points to released Switchloom instead of
   `planr-routing`.
10. Preserve `apps/docs/redirects.mjs` and
   `docs/documentation/INFORMATION_ARCHITECTURE.md` only if they are historical
   redirect inventory. Rewrite or delete them if they describe a current
   routing-bundles feature surface.
11. Run Planr's own format, lint, unit, integration, docs, and app-docs
   verification commands, including the docs shell verifier that replaces the
   legacy command assertions.
12. Run negative ownership scans in the Planr repo for legacy compiler/catalog
   ownership strings, including `planr-routing`, `routing bundle`, and
   Switchloom model catalog/preset compiler code.
13. Re-run this repo's checker from `/Users/kregenrek/projects/model-routing`:
   `sh scripts/check-migration-manifest.sh`. It must either find no remaining
   legacy Planr files or find only rows explicitly retained as Planr-owned
   neutral orchestration.

The hard cut is complete only when Planr can pass its tests using released
Switchloom declarations or fixtures, with no Planr-side compiler/catalog source
of truth remaining.
