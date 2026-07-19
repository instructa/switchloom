# Planr Hard-Cut Handoff Receipt

Date: 2026-07-19

This receipt is the Goal A gate for starting the Planr Goal B hard cut. It
does not modify `/Users/kregenrek/projects/planr`; it records the independently
published Switchloom release and the exact Planr routing responsibilities that
now have a standalone owner or an explicit deletion action.

## Superseding Candidate Status

The public `v0.2.0` release remains the latest published Switchloom release, but
its original native Codex discovery evidence had a release-gating gap: the
published package generated repository-local Codex role files, yet the accepted
receipt did not prove the checksum-identified installed candidate executed the
full status/uninstall lifecycle while preserving unmanaged repository content.

The current unpublished superseding candidate is `0.2.1`. It must replace
`v0.2.0` before Planr Goal B starts. The canonical retained `0.2.1` candidate
proof chain is:

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

Do not use the `v0.2.0` publication as the final Goal A handoff. Publish and
smoke-test the superseding `0.2.1` candidate first, then update the public
release identity below to the verified public bytes.

## Release Identity

- Repository: `https://github.com/instructa/switchloom`
- Production website: `https://switchloom.ai`
- Release: `https://github.com/instructa/switchloom/releases/tag/v0.2.0`
- Release workflow: `https://github.com/instructa/switchloom/actions/runs/29643657292`
- Tag: `v0.2.0`
- Commit: `e1db0f7d9cdb213fc9a48f46e8696bdbe9f616dd`
- Commit subject: `release 0.2.0: Setup contract and CLI lifecycle`
- npm package: `switchloom@0.2.0`
- npm tarball: `https://registry.npmjs.org/switchloom/-/switchloom-0.2.0.tgz`
- npm integrity:
  `sha512-Cd8YCpQGZ1PuNe0TSBd3MR9S2jA0o+ikdKCg2Sh+sn1ldIRBksjRk0axJOZdyCWrQC6OHdUc3KX7CJGEjrgfPQ==`
- npm shasum: `3eba2b8c35963dcb15b815b08361da7cb4d00208`
- Homebrew formula: `instructa/tap/switchloom`, version `0.2.0`

## Release Artifacts

GitHub release `v0.2.0` is public and non-draft. It contains aggregate
`SHA256SUMS` and four platform archives:

| Asset | SHA-256 |
| --- | --- |
| `SHA256SUMS` | `2aae33623f13bb68798fc1c21831cd2310e48285c3c2b661cadbf34455d063d8` |
| `switchloom-darwin-arm64.tar.gz` | `028e1de5218b0493d826c1043551330e3581f677722c5597ff7dc6bc59b294a6` |
| `switchloom-darwin-x86_64.tar.gz` | `dcb18b7d782a91b439b8c45623ee3de5dcf574789d33438567390b4994b6e046` |
| `switchloom-linux-arm64.tar.gz` | `73af0de9b07e9095c6ef9798596f93668ea038b2f52d70ec9a37f97ff34cb50e` |
| `switchloom-linux-x86_64.tar.gz` | `a6be2974ce3b7ddf12f989931c458d619bf5740069803733ac1edb29a3054217` |

The Homebrew formula points at the same four `v0.2.0` release archives and
uses the matching platform checksums above.

## Package Graph

Switchloom has no Planr build or runtime dependency in either published npm
metadata or the Rust workspace metadata.

| Command | Result |
| --- | --- |
| `npm view switchloom@0.2.0 name version bin dependencies optionalDependencies peerDependencies --json` | Published package is `switchloom` version `0.2.0`, exposes bins `switchloom` and `model-routing` pointing to `npm/bin/model-routing.js`, and reports no runtime, optional, or peer dependency objects. |
| `jq -r '.dependencies // {}, .peerDependencies // {}, .optionalDependencies // {}' package.json` | Local package metadata prints `{}` for each dependency graph. |
| `cargo metadata --format-version 1 --no-deps \| jq -r '.packages[] \| select(.name=="model-routing") \| {name,version,dependencies:[.dependencies[].name],targets:[.targets[] \| {name,kind}]}'` | Rust package is `model-routing` version `0.2.0`; dependencies are `anyhow`, `clap`, `ed25519-dalek`, `serde`, `serde_json`, `sha2`, and `toml`; targets are library `model_routing` plus binaries `model-routing` and `switchloom`. No dependency is named `planr`. |
| `sh scripts/check-migration-manifest.sh` | The checker scans local package metadata and fails on direct `planr` entries in `package.json`, `Cargo.toml`, or `Cargo.lock`. |

## Verification Receipts

| Area | Evidence |
| --- | --- |
| Release publication | Planr item `i-publish-and-smoke-test-the-switc-3ca4`, completion log `2026-07-18 13:04:32`, records public GitHub release, green Release run, npm Trusted Publisher publish, Homebrew formula test, production website verification, and exact-version npm/Homebrew lifecycle smokes. |
| Release review | Planr review `i-review-publish-and-smoke-test-th-e99e`, verdict complete at `2026-07-18 13:04:57`. |
| Public GitHub check | `gh release view v0.2.0 --repo instructa/switchloom --json tagName,targetCommitish,url,publishedAt,isDraft,isPrerelease,assets` returned public non-draft `v0.2.0` with the five assets listed above. |
| Public workflow check | `gh run view 29643657292 --repo instructa/switchloom --json conclusion,status,url,createdAt,updatedAt,headSha,event,workflowName` returned `conclusion: success`, `status: completed`, `headSha: e1db0f7d9cdb213fc9a48f46e8696bdbe9f616dd`. |
| Public npm check | `npm view switchloom@0.2.0 version dist.integrity dist.shasum dist.tarball --json` returned version `0.2.0` and the integrity/shasum/tarball listed above; `npm view switchloom@0.2.0 name version bin dependencies optionalDependencies peerDependencies --json` returned the bin contract and no dependency objects. |
| Public website check | `curl -I https://switchloom.ai` returned HTTP `200`, Cloudflare, `content-security-policy`, `permissions-policy`, `referrer-policy`, and `x-content-type-options` headers. |
| Public Homebrew check | `gh api repos/instructa/homebrew-tap/contents/Formula/switchloom.rb --jq '.content' \| base64 --decode` returned formula version `0.2.0`, release archive URLs, matching platform SHA-256 values, and `switchloom --version` formula test. |
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
- `generated-artifact` rows transfer current Switchloom v0.2.0 outputs: optional
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
   `gh release view v0.2.0 --repo instructa/switchloom`,
   `npm view switchloom@0.2.0 version`, and
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
   `switchloom@0.2.0` and assertions that Planr consumes declarations, resolves
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
