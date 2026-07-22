# Planr Hard-Cut Handoff Receipt

Date: 2026-07-22

This is the durable Goal A gate for the Planr Goal B hard cut. It does not
modify `/Users/kregenrek/projects/planr`; it records the independently reviewed
and published Switchloom owner plus the exact Planr responsibilities to keep,
replace, or delete.

## Superseding Release Status

The current public handoff release is `v0.3.1`. It supersedes all `v0.2.x` and
`v0.3.0` receipts for Goal B.

- Candidate commit: `2f8ba006df06b88bb602d0698696c73e5963ff86`
- Release and merge commit: `d7165627e33be2fb17f2e2f8f1b289cc1a40bf83`
- Reviewed and release tree: `2016988ee6ddf3514024ed93fdc810c41b388ce1`
- PR `#21`: `https://github.com/instructa/switchloom/pull/21`
- Annotated tag: `v0.3.1`, tag object
  `a30cc81d17410075dad33ad68c39e1149c1b2bec`, peeling to the release commit
- Release workflow `29917032398`: successful
- GitHub release: public, non-draft, non-prerelease
- npm: `switchloom@0.3.1`, trusted publication with two attestations
- Homebrew: `instructa/tap/switchloom`, stable `0.3.1`
- Website: `https://switchloom.ai`, 28 compositions and 6 setup hosts

The durable publication receipt is
`reports/release-ready-v0.3.1/publication.json`. The public-byte and lifecycle
receipt is `reports/release-ready-v0.3.1/public-byte-certification.json`, with
the human-readable record in `docs/prepublish-certification-0.3.1.md`.

## Release Identity

| Surface | Identity |
| --- | --- |
| GitHub source | `d7165627e33be2fb17f2e2f8f1b289cc1a40bf83` with tree `2016988ee6ddf3514024ed93fdc810c41b388ce1` |
| GitHub release | `https://github.com/instructa/switchloom/releases/tag/v0.3.1` |
| npm | `switchloom@0.3.1`, shasum `4fb17ae575f9a77f5920524743f07b69da3c3ea4` |
| npm tarball | SHA-256 `52b8aa965ef81a3c9c8f94ffe4dfa62db1c92475bc6834f1366208ec72f52fba` |
| npm provenance | SHA-256 `8f207afe26e609ca9ff1c9bc8a36595abaa0c92bfbd04409999b9eaff4a5ba44`, git SHA equals release commit |
| Homebrew tap | `e74da41afe58f76bb8d6aa2e115501679ae6a527` |
| Website catalog | SHA-256 `f879c6fdccca95abebcb65d521d9f115007165ec3c0e9fb1db47e81dae394026` |
| Balanced Codex download | SHA-256 `da329ed10bbf9be4cc136173eb2702237662d6c13ad9e0f995e531535a0bc603` |

GitHub `SHA256SUMS` hashes to
`c6443b222478f57d476867f35c36c6e15348d09424f0db40a079535e62471e9d`.

| Target | Archive SHA-256 | Native SHA-256 |
| --- | --- | --- |
| `darwin-arm64` | `1b5d022e6e9839ea16cdc51cd55283d3f116f7d52261f40a52b2b0c20d6797bf` | `5c0884da0dda7bdd87e8e3ce9530faa1cdea980acea49dadd0b8edd21367b5a4` |
| `darwin-x86_64` | `e0a2e143d75b90651f714f144111c30e38cbfa3c8d407e0af45c70ea8fc519a4` | `2da9107175c73e24de7790f5219041550f29674c6af9a5adfb056afcde2cc08b` |
| `linux-arm64` | `2116b8cf934d24a04973c0fd76cbf88d9f17eaab9b032446e87d445bc8bb3b73` | `1c3808749f941b9badb79c199025cac2c22678148a0b338220b1e7d12d65cd92` |
| `linux-x86_64` | `836abf1689cc09e5254273cd379f285174be6886ce60932008e48732883af844` | `ee44f2df0e59063a5feb1e438644c3fccdacfd443746dea5e184686709cbbebd` |

All npm native bytes match binaries extracted from the corresponding public
GitHub archives. Homebrew uses the same darwin-arm64 archive checksum.

## Package Graph

Switchloom has no Planr build or runtime dependency in published npm metadata
or Rust workspace metadata. The release gate verifies `package.json`,
`Cargo.toml`, and `Cargo.lock` contain no direct Planr dependency, while
`scripts/check-migration-manifest.sh` owns the migration inventory check.

## Verification Receipts

| Area | Evidence |
| --- | --- |
| Candidate and CI | PR `#21` had 10 green checks. Candidate review `i-review-produce-and-independently-4f3b` closed complete in independent mode. |
| Publication | Release workflow `29917032398` completed successfully. GitHub archives, npm trusted provenance, Homebrew, and both website endpoints correlate to the reviewed release source. |
| Publication remediation | Review `i-review-publish-switchloom-v0-3-1-209e` found stale local-binary parity evidence. Fix `i-fix-findings-for-review-publish-9562` used the checksum-verified public binary; remediation review `i-review-fix-findings-for-review-p-f9ca` and follow-up `i-follow-up-review-for-review-publ-b8c0` closed complete. |
| Fresh npm | `/private/tmp/switchloom-public-v0.3.1-npm.ncczr0` installed public `switchloom@0.3.1` in an isolated prefix/cache and passed version/help plus compile, inspect, preview, apply, status, update, rollback, repair, uninstall, compatible-unmanaged preservation, and conflict fail-closed assertions. |
| Fresh Homebrew | `/private/tmp/switchloom-public-v0.3.1-brew.4i8hYg` reinstalled stable `0.3.1` and passed the same lifecycle and preservation matrix. |
| Public Codex byte | Exact `codex-cli 0.145.0` ran the public npm darwin-arm64 SHA-256 `5c0884da...`; Terra High and Sol High V2 children correlated exact spawn calls, custom task names, registered agent types, `fork_turns: none`, child sessions, effective model/effort, and nonces. |
| Negative oracle | Exact Codex 0.145 negative fixture failed closed because the parent did not contain exactly two V2 `spawn_agent` calls. |
| Website | The public binary verified both production endpoints, 28 compositions, 6 hosts, and parity `da329ed1...`; a fresh rendered-page interaction verified the v0.3.1 install/lifecycle list, exact copied npx recipe, Codex certification, Luna experimental, Cursor advisory, and Claude unavailable wording. |
| Protected state | Global Codex config stayed SHA-256 `106482691dcada0fe1e862bffe7c59e771e804636a64b7495c5434995e378293`. Planr stayed at `bbc877d40191b2cbb289ed26df5e6fee25e4326d` with `-uall` status SHA-256 `d6c56495c7e2a78aed2e641b0e928bc8a579bf31335db36456a9f05726827927`. |

## Tested Host Matrix

| Host/profile | Status | Release claim |
| --- | --- | --- |
| Codex Terra High | Certified on exact `codex-cli 0.145.0` public bytes | Deterministic V2 maker dispatch to `gpt-5.6-terra`, effort `high`, with exact correlated spawn/session/nonce evidence. |
| Codex Sol High | Certified on exact `codex-cli 0.145.0` public bytes | Deterministic V2 reviewer dispatch to `gpt-5.6-sol`, effort `high`, with exact correlated spawn/session/nonce evidence. |
| Codex Luna | Experimental/unverified | Excluded from certified defaults until equivalent authentic evidence is independently accepted. |
| Cursor | Advisory | Native project agents and requested routing are supported, but effective model/effort claims remain advisory because the host does not expose that telemetry. |
| Claude Code | Unavailable/unverified | No live authenticated effective model/effort receipt is claimed. |
| OpenCode | Unavailable/unverified | Deterministic artifact coverage exists, but no authentic nonce-bearing child receipt upgrades it. |
| Pi | Unavailable/unverified | Deterministic workflow coverage exists, but provider-authenticated child evidence is absent. |

Sanitized positive evidence is retained at
`retained-evidence/release-ready-v0.3.1/live/codex-openai/1784721995-22171-0/`;
sanitized negative evidence is retained at
`retained-evidence/release-ready-v0.3.1/live/codex-openai-negative/1784722125-40469-0/`.
Raw auth, cache, session, database, and nested repository data is not committed.

## Migration Ownership

The exhaustive mapping is `docs/migration-manifest.tsv`.

- `source-file` rows transfer frozen `planr-routing/*` source, website,
  fixture, policy, evaluation, and documentation ownership to Switchloom.
- `generated-current` rows are deleted and regenerated from standalone
  Switchloom source; dependency-install artifacts are never moved or published.
- `cli-command` rows transfer legacy `planr routing bundle ...` commands to the
  standalone `model-routing`/`switchloom` surface.
- `generated-artifact` rows transfer current Switchloom `v0.3.1` outputs:
  optional `.planr/agents.toml` and `.planr/policy.toml`; repository-local Codex
  config and role files; Claude Code role prompts; and Cursor project agents.
- `planr-consumer` rows are the only Goal B rows. They are either Planr-owned
  neutral orchestration surfaces to retain or legacy compiler/catalog surfaces
  to replace or delete after this receipt is independently accepted.

The active Planr files covered by `planr-consumer` rows include `Cargo.toml`,
`Cargo.lock`, `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`,
`README.md`, `CHANGELOG.md`, `src/routing_bundle.rs`,
`src/routing_bundle/tests.rs`, `src/app/routing.rs`, `src/cli.rs`, `src/main.rs`,
`src/rolefiles.rs`, `src/app/agents.rs`, `src/app/agents_init.rs`, `tests/e2e.rs`,
`docs/MODEL_ROUTING.md`, `docs/ROUTING_BUNDLES.md`, `docs/MCP_CONTRACT.md`,
`docs/CLI_REFERENCE.md`, `docs/GOALS.md`, `docs/EXAMPLE_WEBAPP.md`,
`docs/fixtures/mcp-contract.json`, `docs/INSTALL.md`, `docs/HOOKS.md`,
`docs/ARCHITECTURE.md`, `docs/documentation/CONTRACT.md`,
`docs/documentation/INFORMATION_ARCHITECTURE.md`, `docs/SKILLS.md`,
`docs/CODEX.md`, `apps/docs/content/docs/contributing/architecture.mdx`,
`apps/docs/content/docs/reference/cli-generated.mdx`,
`apps/docs/content/docs/reference/cli.mdx`,
`apps/docs/content/docs/reference/configuration-and-storage.mdx`,
`apps/docs/redirects.mjs`, `apps/docs/scripts/verify-shell.mjs`, and
`plugins/planr/skills/planr-loop/SKILL.md`.

The checker scans the live Planr repository case-insensitively for
`planr[- ]routing`, `routing[_ -]bundles?`, and `routingbundle`, excluding only
operational state, generated/dependency directories, and the frozen producer
subtree already represented by `source-file` rows. It fails on uncovered files
or duplicate `(type, source)` mappings.

## Remaining Planr Surface

After Goal B, Planr should keep only provider-neutral orchestration:

- Read `.planr/agents.toml` and `.planr/policy.toml` declarations emitted by
  optional Switchloom Planr mode.
- Resolve worker/reviewer routes from those declarations during pick/routing.
- Enforce Planr-owned execution and usage constraints.
- Record declared-versus-effective evidence in Planr workflow logs.
- Keep Planr-owned agent registry, initialization, worker/reviewer roles, app
  policy, execution policy, and usage policy behavior.

Planr must not retain a routing policy compiler, model catalog, preset registry,
host-artifact compiler, website generator, package publisher, compatibility
wrapper, or second source of truth for removed routing bundle commands.

## Goal B Deletion Oracle

Goal B may start in `/Users/kregenrek/projects/planr` only after certification
item `i-certify-public-bytes-and-finaliz-ad43` and its independent review close,
the approval list is empty, and `planr plan audit pln-45ebe887 --json` passes.

1. Confirm `gh release view v0.3.1 --repo instructa/switchloom`, `npm view
   switchloom@0.3.1 version`, and the Homebrew formula still agree.
2. Delete or replace every `planr-consumer` row whose disposition is
   `keep-then-delete`, `split`, or `replace`; keep only rows marked `keep` as
   neutral Planr orchestration/policy code.
3. Remove root workspace/package wiring for the legacy producer, including its
   Cargo member/lock state and pnpm package/script state.
4. Remove legacy user-facing `planr routing bundle` paths and tests that require
   Planr to compile Switchloom-owned bundles.
5. Replace route tests with declarations or fixtures produced by
   `switchloom@0.3.1`, asserting only Planr consumption, resolution, policy, and
   effective-evidence logging.
6. Regenerate CLI docs and fixtures so they no longer expose or assert legacy
   bundle inspect/preview/apply capabilities.
7. Update README, changelog, architecture, install, skills, Codex, API/MCP, and
   app docs so current routing ownership points to Switchloom.
8. Run Planr format, lint, unit, integration, docs, and app-docs verification,
   plus negative scans for legacy compiler/catalog ownership.
9. Re-run `sh scripts/check-migration-manifest.sh` from this Switchloom repo. It
   must find no legacy Planr files or only explicitly retained neutral rows.

The hard cut is complete only when Planr passes its own tests using released
Switchloom declarations or fixtures and retains no compiler/catalog ownership.
