# Switchloom 0.3.1 Public-Byte Certification

Date: 2026-07-22
Release Planr map: `switchloom-v0-3-1-codex-0-145-native-v2-compatibility-and-published-byte-certification`
Certification item: `i-certify-public-bytes-and-finaliz-ad43`

This receipt supersedes the `v0.3.0` handoff as the current public-byte proof.
The machine-readable source is
`reports/release-ready-v0.3.1/public-byte-certification.json`.

## Reviewed Release Identity

- Candidate commit: `2f8ba006df06b88bb602d0698696c73e5963ff86`
- Release and merge commit: `d7165627e33be2fb17f2e2f8f1b289cc1a40bf83`
- Candidate and release tree: `2016988ee6ddf3514024ed93fdc810c41b388ce1`
- Pull request: `https://github.com/instructa/switchloom/pull/21`
- Annotated tag: `v0.3.1`
- Tag object: `a30cc81d17410075dad33ad68c39e1149c1b2bec`
- Release workflow: `29917032398`, successful for the release commit
- GitHub release: `https://github.com/instructa/switchloom/releases/tag/v0.3.1`
- npm package: `switchloom@0.3.1`
- Homebrew formula: `instructa/tap/switchloom`, stable `0.3.1`
- Website: `https://switchloom.ai`

PR `#21` completed 10 required checks before merge. The candidate review
`i-review-produce-and-independently-4f3b` closed complete in independent mode.
Publication review `i-review-publish-switchloom-v0-3-1-209e` found a stale
local-binary website parity claim. Fix item
`i-fix-findings-for-review-publish-9562` replaced it with the checksum-verified
public binary result, and both `i-review-fix-findings-for-review-p-f9ca` and
`i-follow-up-review-for-review-publ-b8c0` closed complete.

## Public Provenance

The npm tarball SHA-256 is
`52b8aa965ef81a3c9c8f94ffe4dfa62db1c92475bc6834f1366208ec72f52fba`,
registry shasum is `4fb17ae575f9a77f5920524743f07b69da3c3ea4`, and
integrity is
`sha512-y+wz1NDEljOmXVGWkrjgy9QIXAnJ2w29d2rVntEmefDGyyR7D1q044pjHX3gdgrCIWFm/VJdwBRVSL/xytpPFA==`.
Its native provenance hashes to
`8f207afe26e609ca9ff1c9bc8a36595abaa0c92bfbd04409999b9eaff4a5ba44`
and records release commit `d7165627e33be2fb17f2e2f8f1b289cc1a40bf83`.

| Target | GitHub archive SHA-256 | Extracted native SHA-256 |
| --- | --- | --- |
| `darwin-arm64` | `1b5d022e6e9839ea16cdc51cd55283d3f116f7d52261f40a52b2b0c20d6797bf` | `5c0884da0dda7bdd87e8e3ce9530faa1cdea980acea49dadd0b8edd21367b5a4` |
| `darwin-x86_64` | `e0a2e143d75b90651f714f144111c30e38cbfa3c8d407e0af45c70ea8fc519a4` | `2da9107175c73e24de7790f5219041550f29674c6af9a5adfb056afcde2cc08b` |
| `linux-arm64` | `2116b8cf934d24a04973c0fd76cbf88d9f17eaab9b032446e87d445bc8bb3b73` | `1c3808749f941b9badb79c199025cac2c22678148a0b338220b1e7d12d65cd92` |
| `linux-x86_64` | `836abf1689cc09e5254273cd379f285174be6886ce60932008e48732883af844` | `ee44f2df0e59063a5feb1e438644c3fccdacfd443746dea5e184686709cbbebd` |

All four npm native hashes match binaries extracted from the four public
GitHub archives. `SHA256SUMS` hashes to
`c6443b222478f57d476867f35c36c6e15348d09424f0db40a079535e62471e9d`.
The Homebrew formula at tap commit
`e74da41afe58f76bb8d6aa2e115501679ae6a527` uses the same darwin-arm64
archive and checksum.

## Fresh Install Lifecycles

The npm run used an isolated global prefix and cache under
`/private/tmp/switchloom-public-v0.3.1-npm.ncczr0`. The Homebrew run used
`brew reinstall instructa/tap/switchloom` and an isolated repository under
`/private/tmp/switchloom-public-v0.3.1-brew.4i8hYg`.

Both public channels returned `model-routing 0.3.1`, exposed `policy`,
`compile`, `inspect`, `preview`, `apply`, `update`, `status`, `uninstall`,
`rollback`, and `doctor`, and executed this non-destructive lifecycle:

1. Compile `balanced` and `low-usage` Codex bundles.
2. Inspect and preview the bundle, then apply and inspect status.
3. Update, roll back, and check status again.
4. Delete one managed Terra role, observe missing-state repair guidance, and
   run update to restore it.
5. Uninstall and assert that no managed artifacts remain.
6. Apply against a compatible pre-existing unmanaged V2 setting and prove its
   bytes remain unchanged.
7. Apply against a conflicting `multi_agent_v2 = false` setting and prove the
   command fails before partial managed state while preserving its bytes.

The npm and Homebrew binaries both hash to the public darwin-arm64 native hash
`5c0884da0dda7bdd87e8e3ce9530faa1cdea980acea49dadd0b8edd21367b5a4`.
Across both runs, the unrelated sentinel, project Codex config, project role,
and global Codex config remained byte-identical:

| State | SHA-256 |
| --- | --- |
| Unmanaged sentinel | `60ba63428d6029222a9d092142fcdc64d045d93650e0c25cb25cd7f319351fae` |
| Project `.codex/config.toml` | `205154e65c71a9da37e8e88334441d3873369a2da943d6f8875c0ca2d27aa8f1` |
| Project local role | `dc41b95480f42e96893940ea9621c0b4d941f9a2385abcc2c0692c5da947a0ce` |
| Compatible unmanaged V2 config | `6600e0b0294d38fc8f9ab0b0b82d99fea32cc39d4f18053d67925a5344928b36` |
| Conflicting false config | `d4387757c44460f8fb3396a67c2d2298ef64e564cd88cb1c81930d73d50ca223` |
| Global `~/.codex/config.toml` | `106482691dcada0fe1e862bffe7c59e771e804636a64b7495c5434995e378293` |

## Exact Codex 0.145 Oracle

The positive command used exact `@openai/codex@0.145.0`, an isolated
authenticated Codex home, and the public npm darwin-arm64 binary:

```sh
SWITCHLOOM_CODEX_RUNTIME_HOME=<isolated-authenticated-home> cargo run --quiet -p xtask -- certify codex --routing-bin reports/release-ready-v0.3.1/public-npm/extracted/package/npm/native/darwin-arm64/model-routing --report-root retained-evidence/release-ready-v0.3.1/live --timeout-seconds 600
```

The report has `success: true` and `live_verified: true`. It correlates the
parent thread, exact V2 spawn call arguments, custom `task_name`, registered
`agent_type`, `fork_turns: none`, complete child sessions, effective model and
effort, and dynamic nonce for:

- Terra High maker: `model_routing_terra_high`, task `standalone_maker`,
  effective `gpt-5.6-terra` with `high`, nonce
  `019f89b8-8cb6-74b3-bd94-77fd07a11d5a:019f89b8-9d85-71d3-bed8-4cf9f73cfc1f:call_E8YJ5duz3iKw7eBUi0qs7Sm1`.
- Sol High reviewer: `model_routing_sol_high`, task `standalone_reviewer`,
  effective `gpt-5.6-sol` with `high`, nonce
  `019f89b8-8cb6-74b3-bd94-77fd07a11d5a:019f89b8-a64a-70e2-a6ef-50d04eabd4f5:call_ZXL8ZbYOTMa43wAA35DHeNpj`.

Positive evidence:

- `retained-evidence/release-ready-v0.3.1/live/codex-openai/1784721995-22171-0/certification-report.json`, SHA-256
  `6c2c45f552db1961545f0bcd4119417517978eb2310d004f27265201e90a694a`.
- `retained-evidence/release-ready-v0.3.1/live/codex-openai/1784721995-22171-0/codex-runtime-evidence.json`, SHA-256
  `30127910811f5a6906a09a1b08ba777c18d1f858821abe24d986c16b54d41f76`.

The exact negative command added `--negative-fixture`. It succeeded only by
failing closed with `parent must contain exactly 2 V2 spawn_agent calls`.

- `retained-evidence/release-ready-v0.3.1/live/codex-openai-negative/1784722125-40469-0/certification-report.json`, SHA-256
  `b291e1eab3535628c901576681bc14c5911a7b5c4a30170d539f4e2b2d212299`.
- `retained-evidence/release-ready-v0.3.1/live/codex-openai-negative/1784722125-40469-0/codex-negative-fail-closed.txt`, SHA-256
  `f9920edf6c71288ee3e266c3e884647bc7df17c512a5038b0f9eb7c5d7881ede`.

The protected global-config snapshots match before and after both runs. The
capability boundary remains frozen in `docs/codex-v2-runtime-evidence.json`:
Codex owns effective backend selection and orchestration; Switchloom owns the
repository-local role declarations and requested-versus-effective contract.

## Live Website Guidance

`node scripts/verify-cloudflare-website.mjs https://switchloom.ai <public-v0.3.1-binary>`
passed with 28 compositions, 6 setup hosts, catalog SHA-256
`f879c6fdccca95abebcb65d521d9f115007165ec3c0e9fb1db47e81dae394026`,
and balanced Codex download parity
`da329ed10bbf9be4cc136173eb2702237662d6c13ad9e0f995e531535a0bc603`.

A fresh headless browser rendered the live Commands tab and verified the
`switchloom@0.3.1` install/lifecycle commands. Invoking the page's copy action
returned an exact `npx switchloom@0.3.1 apply --recipe 'sw1_...' --repository .`
command. Switching live host tabs verified exact Codex 0.145 Terra/Sol
certification, Luna experimental/unverified, Cursor advisory, and Claude
unavailable/unverified wording.

## Protection And Limits

- `/Users/kregenrek/projects/planr` remained at
  `bbc877d40191b2cbb289ed26df5e6fee25e4326d` with `-uall` status SHA-256
  `d6c56495c7e2a78aed2e641b0e928bc8a579bf31335db36456a9f05726827927`.
- The global Codex config remained SHA-256
  `106482691dcada0fe1e862bffe7c59e771e804636a64b7495c5434995e378293`.
- Fresh install execution covered darwin-arm64. Other platforms are correlated
  by CI provenance and public archive extraction, not executed on this host.
- Terra High and Sol High are deterministic for exact Codex 0.145.0. Luna is
  experimental/unverified; Cursor stays advisory; Claude Code, OpenCode, and Pi
  stay unavailable/unverified without equivalent authentic receipts.
- An initial isolated run without copied authentication returned HTTP 401 and
  is excluded. No auth file, session database, cache, or raw runtime workspace
  is committed; only the sanitized reports listed above are retained.

After this item and its independent review close, `planr plan audit
pln-45ebe887 --json` must report all clauses passing before Goal B starts.
