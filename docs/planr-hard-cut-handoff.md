# Planr Hard-Cut Handoff Receipt

Date: 2026-07-21

This receipt is the durable Goal A gate for starting the Planr Goal B hard cut.
It does not modify `/Users/kregenrek/projects/planr`; it records the independently
published Switchloom release and the exact Planr routing responsibilities that
now have a standalone owner or an explicit deletion action.

## Superseding Release Status

The current public hard-cut handoff release is `v0.3.0`. It supersedes the
earlier `v0.2.0`, `v0.2.1`, and `v0.2.2` receipts for Planr Goal B.

The retained `v0.3.0` proof chain is:

- PR `#19`, `https://github.com/instructa/switchloom/pull/19`, merged
  `2026-07-21T18:25:24Z`.
- Candidate head commit `e936b90c81d8f944ba50657c10ef9a2a564c53c5`.
- Release commit and merge commit
  `45c15fed09786c9dee1167744b1c43b87b47d505`.
- Annotated tag `v0.3.0`, tag object
  `8306c2533be0cbeb9c6801651cb08d0d244d7678`, peeling to the release commit.
- Release workflow `29857502558` completed successfully for the release commit.
- Main CI after merge passed in run `29857146170`; main Secret Scan passed in
  run `29857145475`.
- Public-byte certification retained fresh npm install, Homebrew install, live
  website, status, apply, uninstall, and preservation receipts under
  `/private/tmp/switchloom-public-v0.3.0-npm-fix.GMEH4n`,
  `/private/tmp/switchloom-public-v0.3.0-brew-fix.sA9meG`, and
  `/private/tmp/switchloom-public-v0.3.0-website-fix.Ou1eQ0`.
- The protected Planr repository remained at HEAD
  `bbc877d40191b2cbb289ed26df5e6fee25e4326d` with
  `git status --porcelain=v1 -uall` SHA-256
  `d6c56495c7e2a78aed2e641b0e928bc8a579bf31335db36456a9f05726827927`.
- The authenticated global Codex config hash remained
  `eec8236cc13bd729a46376d65624eaf795a70bd48ccee8ac707d904f735aaedd`.

Do not use earlier publications as the final Goal A handoff. The superseding
public `v0.3.0` bytes are the release gate for Goal B.

## Release Identity

- Repository: `https://github.com/instructa/switchloom`
- Production website: `https://switchloom.ai`
- Release: `https://github.com/instructa/switchloom/releases/tag/v0.3.0`
- Release workflow: `https://github.com/instructa/switchloom/actions/runs/29857502558`
- Tag: `v0.3.0`
- Release commit: `45c15fed09786c9dee1167744b1c43b87b47d505`
- Candidate commit: `e936b90c81d8f944ba50657c10ef9a2a564c53c5`
- npm package: `switchloom@0.3.0`
- npm tarball: `https://registry.npmjs.org/switchloom/-/switchloom-0.3.0.tgz`
- npm integrity:
  `sha512-NdEyH1zq+W6E5pZIHdG8oe2CQxVjtjVlvQlvdCMxDEwaroCLzXEMiUMvnd/kjgXGx5A/WYsFxWe/nU7eSNgjzw==`
- npm shasum: `7ccf5c8c693c33f5a83fad47529e630c46c8cf2c`
- npm file count: `20`
- npm unpacked size: `11898450`
- Homebrew formula: `instructa/tap/switchloom`, stable version `0.3.0`
- Homebrew tap HEAD: `2fdec6450d4444febc54fcf7ffe44e7072e59c74`

## Release Artifacts

GitHub release `v0.3.0` is public, non-draft, and non-prerelease. It contains
aggregate `SHA256SUMS` and four platform archives:

| Asset | SHA-256 |
| --- | --- |
| `SHA256SUMS` | `a08021d57bd639244fab0a8ea575f3a2a5d77037fb6dbbae8e29d7db2bbc45ec` |
| `switchloom-darwin-arm64.tar.gz` | `e87d40f28553996b27313c82823f5f633e8e616197484d3b110bc1dc19ecb039` |
| `switchloom-darwin-x86_64.tar.gz` | `81485d39947f7e27a8693014b8eab346cfe7eaa72d3466420ac24068be12946e` |
| `switchloom-linux-arm64.tar.gz` | `09e063d95dc55a12108f75eacc1f3e7e04a4783d537244b4a94180cd2c34f678` |
| `switchloom-linux-x86_64.tar.gz` | `4636f564f884b6e710a1aa78a7429731995c69006603fab4ea663f6378d40ded` |

The Homebrew formula points at the `v0.3.0` release archive for the current
stable macOS ARM formula URL and uses checksum
`e87d40f28553996b27313c82823f5f633e8e616197484d3b110bc1dc19ecb039`.

## Package Graph

Switchloom has no Planr build or runtime dependency in either published npm
metadata or the Rust workspace metadata.

| Command | Result |
| --- | --- |
| `npm view switchloom@0.3.0 name version bin dependencies optionalDependencies peerDependencies --json` | Published package is `switchloom` version `0.3.0`, exposes bins `switchloom` and `model-routing` through `npm/bin/model-routing.js`, and reports no runtime, optional, or peer dependency objects. |
| `jq -r '.dependencies // {}, .peerDependencies // {}, .optionalDependencies // {}' package.json` | Local package metadata prints `{}` for each dependency graph. |
| `cargo metadata --format-version 1 --no-deps \| jq -r '.packages[] \| select(.name=="model-routing") \| {name,version,dependencies:[.dependencies[].name],targets:[.targets[] \| {name,kind}]}'` | Rust package is `model-routing` version `0.3.0`; dependencies are `anyhow`, `clap`, `ed25519-dalek`, `serde`, `serde_json`, `sha2`, `thiserror`, and `toml`; targets are library `model_routing`, binaries `model-routing` and `switchloom`, and contract tests. No dependency is named `planr`. |
| `sh scripts/check-migration-manifest.sh` | The checker scans local package metadata and fails on direct `planr` entries in `package.json`, `Cargo.toml`, or `Cargo.lock`. |

## Verification Receipts

| Area | Evidence |
| --- | --- |
| Release PR and CI | PR `#19` was merged at `2026-07-21T18:25:24Z`; PR checks passed for Rust, four native builds, forbidden tracked paths, npm distribution, native-matrix package validation, TruffleHog verified secrets, and BetterLeaks. |
| Release review chain | Planr items in `switchloom-v0-3-0-reviewed-candidate-publication` were reviewed by `checker-release-revalidate` in independent mode. The exact review table below records the not-complete reviews, fix items, remediation reviews, and final follow-up verdicts. |
| Public GitHub check | `gh release view v0.3.0 --repo instructa/switchloom --json tagName,targetCommitish,url,publishedAt,isDraft,isPrerelease,assets` returned public non-draft `v0.3.0` with the five assets listed above. |
| Public workflow check | `gh run view 29857502558 --repo instructa/switchloom --json conclusion,status,url,createdAt,updatedAt,headSha,event,workflowName` returned `conclusion: success`, `status: completed`, and `headSha: 45c15fed09786c9dee1167744b1c43b87b47d505`. |
| Public npm check | `npm view switchloom@0.3.0 --json` returned version `0.3.0`, integrity and shasum listed above, tarball `https://registry.npmjs.org/switchloom/-/switchloom-0.3.0.tgz`, 20 files, unpacked size `11898450`, publish time `2026-07-21T18:34:48.188Z`, attestation URL `https://registry.npmjs.org/-/npm/v1/attestations/switchloom@0.3.0`, SLSA predicate `https://slsa.dev/provenance/v1`, and registry signature key `SHA256:DhQ8wR5APBvFHLF/+Tc+AYvPOdTpcIDqOhxsBHRwC7U`. |
| Public Homebrew check | `brew info --json=v2 instructa/tap/switchloom` returned stable version `0.3.0`, formula URL `https://github.com/instructa/switchloom/releases/download/v0.3.0/switchloom-darwin-arm64.tar.gz`, and the matching checksum. |
| Public asset checksum check | `tmpdir=$(mktemp -d /private/tmp/switchloom-release-v0.3.0-assets.XXXXXX); gh release download v0.3.0 --repo instructa/switchloom --pattern 'switchloom-*.tar.gz' --pattern SHA256SUMS --dir "$tmpdir"; cd "$tmpdir"; shasum -a 256 -c SHA256SUMS; cat SHA256SUMS` passed for all four public release archives and printed the archive hashes listed above. |
| Public npm provenance check | `root=$(mktemp -d /private/tmp/switchloom-public-v0.3.0-npm-tarball.XXXXXX); cd "$root"; npm pack switchloom@0.3.0 --json > pack.json; shasum switchloom-0.3.0.tgz; shasum -a 256 switchloom-0.3.0.tgz; tar -xzf switchloom-0.3.0.tgz package/npm/native/provenance.json package/package.json; node -e '...'` passed: tarball SHA-1 matched npm shasum `7ccf5c8c693c33f5a83fad47529e630c46c8cf2c`, tarball SHA-256 was `95ec712b3debab10e33f51652d644c531c5ea85fac4e90bb87369374514b68f5`, and `npm/native/provenance.json` recorded `package_version: 0.3.0`, git SHA `45c15fed09786c9dee1167744b1c43b87b47d505`, and four native targets. |
| Public npm runtime check | `/private/tmp/switchloom-public-v0.3.0-npm-fix.GMEH4n` records the complete fail-closed npm command in Planr item `i-fix-findings-for-review-certify-f413`, `log-c67dc0dd`: explicit `root`, `prefix`, `repo`, and `cache`; public `switchloom@0.3.0` install; version/help assertions; `compile`, `inspect`, `preview`, `apply`, `status`, `uninstall`; empty post-uninstall status; unchanged unmanaged sentinel SHA-256 `60ba63428d6029222a9d092142fcdc64d045d93650e0c25cb25cd7f319351fae`; unchanged authenticated Codex config SHA-256 `eec8236cc13bd729a46376d65624eaf795a70bd48ccee8ac707d904f735aaedd`. |
| Public Homebrew runtime check | `/private/tmp/switchloom-public-v0.3.0-brew-fix.sA9meG` records the complete fail-closed Homebrew command in Planr item `i-fix-findings-for-review-certify-f413`, `log-c67dc0dd`: formula `0.3.0` and checksum assertions, `brew reinstall instructa/tap/switchloom`, version/help assertions, `compile`, `inspect`, `preview`, `apply`, `status`, `uninstall`, empty post-uninstall status, and the same unchanged unmanaged sentinel and Codex config hashes. |
| Public website check | `pnpm exec alchemy deploy --stage prod` completed successfully and reported worker URL `https://model-routing-prod-catalog.office-35d.workers.dev`. `node scripts/verify-cloudflare-website.mjs https://switchloom.ai target/release/model-routing` passed with 28 catalog entries, 6 setup hosts, and `balanced-codex-openai` parity hash `3289f2d7231639a7084c4455c13448e89a1f596a2877672db5b1962d06f5f905`. `log-c67dc0dd` also records the complete fail-closed live guidance command that fetched the live index, catalog, and linked Generator JS and asserted npm/npx setup commands, lifecycle commands, Cursor advisory wording, and Claude unavailable/unverified wording. |
| Live guidance check | `/private/tmp/switchloom-public-v0.3.0-website-fix.Ou1eQ0` records `live website guidance fix passed: 28 compositions, 6 hosts, 1 Generator asset(s)`. |
| Migration manifest | `sh scripts/check-migration-manifest.sh` verifies `docs/migration-manifest.tsv` covers the frozen Planr routing inventory, legacy command transfers, active Planr consumer/deletion mappings from a case-insensitive whole current Planr repo scan for routing lexical variants, unique type/source mappings, current generated artifact targets, and no direct Planr package dependency. |
| Protected Planr baseline | Before and after this receipt update, `/Users/kregenrek/projects/planr` remained at HEAD `bbc877d40191b2cbb289ed26df5e6fee25e4326d` and dirty-state SHA-256 `d6c56495c7e2a78aed2e641b0e928bc8a579bf31335db36456a9f05726827927`. |

## Scoped Review Chain

| Implementation or fix item | Review item | Reviewer | Mode | Verdict | Finding-to-fix or follow-up link |
| --- | --- | --- | --- | --- | --- |
| `i-revalidate-the-clean-candidate-a-9e76` | `i-review-revalidate-the-clean-cand-665d` (`log-a2def4d6`) | `checker-release-revalidate` | independent | not-complete | Required fail-closed candidate/receipt/hash assertions and darwin-arm64 native hash coverage; fixed by `i-fix-findings-for-review-revalida-f671`. |
| `i-fix-findings-for-review-revalida-f671` | `i-follow-up-review-for-review-reva-dc3f` (`log-d6f2175e`) | `checker-release-revalidate` | independent | complete | Follow-up review closed the revalidation finding. |
| `i-fix-findings-for-review-revalida-f671` | `i-review-fix-findings-for-review-r-71f5` (`log-da78edac`) | `checker-release-revalidate` | independent | complete | Remediation review also closed complete. |
| `i-push-the-candidate-branch-and-op-c4a1` | `i-review-push-the-candidate-branch-52ef` (`log-063de355`) | `checker-release-revalidate` | independent | complete | PR `#19` branch/opening evidence accepted. |
| `i-complete-independent-review-and-f551` | `i-review-complete-independent-revi-ca76` (`log-7955f3e5`) | `checker-release-revalidate` | independent | complete | CI and independent review state accepted. |
| `i-merge-the-reviewed-candidate-and-9502` | `i-review-merge-the-reviewed-candid-02fe` (`log-ba285da6`) | `checker-release-revalidate` | independent | complete | Merge commit and main checks accepted. |
| `i-tag-v0-3-0-and-publish-configure-1559` | `i-review-tag-v0-3-0-and-publish-co-6c7b` (`log-61a9b630`) | `checker-release-revalidate` | independent | complete | Tag, release workflow, npm, Homebrew, and website publication accepted. |
| `i-certify-public-bytes-and-fresh-i-e1fa` | `i-review-certify-public-bytes-and-fb9b` (`log-76c6ba7c`) | `checker-release-revalidate` | independent | not-complete | Initial public-byte certification lacked reproducible fresh npm/Homebrew/language guidance commands; fixed by `i-fix-findings-for-review-certify-f413`. |
| `i-fix-findings-for-review-certify-f413` | `i-follow-up-review-for-review-cert-ddc5` (`log-3f1d697e`) | `checker-release-revalidate` | independent | complete | Follow-up certification review accepted `log-c67dc0dd` and closed the finding. |
| `i-fix-findings-for-review-certify-f413` | `i-review-fix-findings-for-review-c-3c54` (`log-37c75656`) | `checker-release-revalidate` | independent | complete | Remediation review also closed complete. |
| `i-finalize-durable-release-and-pla-2dac` | `i-review-finalize-durable-release-a933` (`log-92b96893`) | `checker-release-revalidate` | independent | not-complete | Required this fix item, `i-fix-findings-for-review-finalize-884d`, to add exact review identities, tested-host evidence, and reproducible public-byte command identities. |

## Tested Host Matrix

| Host/profile | Version | Evidence identity | Result |
| --- | --- | --- | --- |
| Codex implementer `codex-terra-high` | `codex-cli 0.144.5` | `retained-evidence/release-ready-v0.3.0/live/codex-openai/1784654270-96976-0/workdir/codex-runtime-evidence.json` (`sha256:df08c39c2fc66c96e44b0a3620b48ec9a3ad271b4b936d56da3ffaf77f7933b8`) | Final-candidate live deterministic. Package digest `sha256:b4dd82486d85f5b7a171cbc7b25836a99ad6c59d0fa0490acbfc18703f729c39` matches `retained-evidence/release-ready-v0.3.0/candidate.json` `packages.darwin_arm64_native.sha256` and `model-routing 0.3.0`; `model_routing_terra_high`, task `standalone_implementer`, `fork_turns: none`, effective model `gpt-5.6-terra`, effective effort `high`, nonce `019f85af-1ba9-7080-b4fa-402fdce321b1:019f85af-49d0-7860-b948-44b5c34d6413:call_CPMYdb9RLRqdYSF9VmtNrZx2`. |
| Codex reviewer `codex-sol-high` | `codex-cli 0.144.5` | `retained-evidence/release-ready-v0.3.0/live/codex-openai/1784654270-96976-0/workdir/codex-runtime-evidence.json` (`sha256:df08c39c2fc66c96e44b0a3620b48ec9a3ad271b4b936d56da3ffaf77f7933b8`) | Final-candidate live deterministic. Package digest `sha256:b4dd82486d85f5b7a171cbc7b25836a99ad6c59d0fa0490acbfc18703f729c39` matches `retained-evidence/release-ready-v0.3.0/candidate.json`; `model_routing_sol_high`, task `standalone_reviewer`, `fork_turns: none`, effective model `gpt-5.6-sol`, effective effort `high`, nonce `019f85af-1ba9-7080-b4fa-402fdce321b1:019f85af-5594-7fa0-a9a3-ae6d27f399bc:call_lRdM1G8hGMLyVqqYJsIhHHjt`. |
| Cursor OpenAI `cursor-openai-worker` | `2026.07.17-3e2a980` | `retained-evidence/release-ready-v0.3.0/live/cursor-openai/1784654493-22038-0/workdir/dispatch-evidence.json` (`sha256:8a774e60cdc3e3c9e29ad9603eedfa540c965635cb20715661e202e3f98dddaf`) | Final-candidate live nonce-correlated advisory. Package digest `sha256:b4dd82486d85f5b7a171cbc7b25836a99ad6c59d0fa0490acbfc18703f729c39` matches `retained-evidence/release-ready-v0.3.0/candidate.json`; requested model `gpt-5.4-mini`, role `model-routing-preset-worker`, nonce `cursor-ca900e62e38956c0534dbd4b4a22f4caef0be56da33acecae9df8afdf9f3e05f`; effective model and effort were not exposed by Cursor, so verdict remains `advisory`. |
| Cursor Fable/Grok `cursor-grok-worker` | `2026.07.17-3e2a980` | `retained-evidence/release-ready-v0.3.0/live/cursor-fable-grok/1784654511-23810-0/workdir/dispatch-evidence.json` (`sha256:0bdf08e18a5b4b99c600c4029928e76bb355d80341b2f54aed25db77b5e8d6cb`) | Final-candidate live nonce-correlated advisory. Package digest `sha256:b4dd82486d85f5b7a171cbc7b25836a99ad6c59d0fa0490acbfc18703f729c39` matches `retained-evidence/release-ready-v0.3.0/candidate.json`; requested model `cursor-grok-4.5-medium`, role `model-routing-preset-worker`, nonce `cursor-fc4820bb03cf0e4c29c70f8cc54a166d0f4f9175c086b4a076febc2497d27589`; effective model and effort were not exposed by Cursor, so verdict remains `advisory`. |
| Claude Code `claude-native` | `2.1.133 (Claude Code)` | `retained-evidence/release-ready-v0.3.0/live/claude-native/1784654534-22027-0/certification-report.json` (`sha256:fbcc3c7a027f1d9a8811189f5b66efed31386aabdf9eea7399d484d18df1dab8`) | Final-candidate skipped/unavailable/unverified. No live effective telemetry or certified dispatch receipt is claimed. |
| OpenCode `opencode-native` | Deterministic artifact coverage only | `reports/native-host-certification/opencode-native/certification-summary.json` | Unavailable/unverified as a live release gate. Static lifecycle and validator coverage pass, but no authentic nonce-bearing child receipt exists and deterministic upgrade is disallowed. |
| Pi `pi-external` | Deterministic artifact coverage only | `reports/native-host-certification/pi-external/certification-summary.json` | Unavailable/unverified as a live release gate. Static workflow and validator coverage pass, but provider authentication and nonce-only child evidence are absent and deterministic upgrade is disallowed. |

Older `reports/native-host-certification/*` Codex and Cursor artifacts remain
historical pre-release evidence only. The final `v0.3.0` tested-host gate is the
durable `retained-evidence/release-ready-v0.3.0/live/*` evidence above.

## Migration Ownership

The exhaustive mapping is `docs/migration-manifest.tsv`.

- `source-file` rows move or replace frozen `planr-routing/*` source, website,
  fixture, policy, evaluation, and documentation ownership into this standalone
  Switchloom repository.
- `generated-current` rows are deleted and regenerated from standalone
  Switchloom source; dependency install artifacts are never moved or published.
- `cli-command` rows transfer old `planr-routing ...` commands to the
  standalone `model-routing`/`switchloom` command surface.
- `generated-artifact` rows transfer current Switchloom `v0.3.0` outputs:
  optional `.planr/agents.toml` and `.planr/policy.toml`; Codex
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
case-insensitively for `planr[- ]routing`, `routing[_ -]bundles?`, and
`routingbundle`, with explicit exclusions for operational state,
generated/dependency directories, and the legacy `planr-routing/` producer
subtree already covered by frozen `source-file` rows. It fails if any discovered
file lacks a manifest row or if any `(type, source)` pair is duplicated.

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
   `gh release view v0.3.0 --repo instructa/switchloom`,
   `npm view switchloom@0.3.0 version`, and
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
   `switchloom@0.3.0` and assertions that Planr consumes declarations, resolves
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
