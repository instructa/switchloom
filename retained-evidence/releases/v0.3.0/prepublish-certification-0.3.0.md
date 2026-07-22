# Switchloom 0.3.0 Publish Certification

Date: 2026-07-21
Release Planr map: `switchloom-v0-3-0-reviewed-candidate-publication`
Final handoff Planr item: `i-finalize-durable-release-and-pla-2dac`

## Candidate

- Candidate version: `0.3.0`
- Candidate branch: `codex/switchloom-0.3.0-migration`
- Pull request: `https://github.com/instructa/switchloom/pull/19`
- Candidate commit: `e936b90c81d8f944ba50657c10ef9a2a564c53c5`
- Release and merge commit: `45c15fed09786c9dee1167744b1c43b87b47d505`
- Annotated tag: `v0.3.0`
- Tag object: `8306c2533be0cbeb9c6801651cb08d0d244d7678`
- Tag target: `45c15fed09786c9dee1167744b1c43b87b47d505`
- Protected Planr repository:
  `${HOME}/projects/planr`
- Protected Planr HEAD:
  `bbc877d40191b2cbb289ed26df5e6fee25e4326d`
- Protected Planr dirty-state hash:
  `d6c56495c7e2a78aed2e641b0e928bc8a579bf31335db36456a9f05726827927`

## Scoped Review Chain

All scoped implementation, review, and remediation items for the
`switchloom-v0-3-0-reviewed-candidate-publication` map were independently
reviewed by `checker-release-revalidate` in independent mode before this final
handoff receipt was prepared.

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

## Passed Gates

- `SWITCHLOOM_RC_RUN_ID=29856707722 RELEASE_DRY_RUN=1 scripts/release.sh 0.3.0 'Public CLI hard cut and deterministic setup lifecycle'`
- `SWITCHLOOM_RC_RUN_ID=29856707722 scripts/release.sh 0.3.0 'Public CLI hard cut and deterministic setup lifecycle'`
- `gh run view 29857502558 --repo instructa/switchloom --json conclusion,status,headSha,event,workflowName`
- `gh release view v0.3.0 --repo instructa/switchloom --json tagName,targetCommitish,url,publishedAt,isDraft,isPrerelease,assets`
- `npm view switchloom@0.3.0 version dist.integrity dist.shasum dist.tarball --json`
- `brew info --json=v2 instructa/tap/switchloom`
- `pnpm exec alchemy deploy --stage prod`
- `node scripts/verify-cloudflare-website.mjs https://switchloom.ai target/release/model-routing`
- `sh scripts/check-migration-manifest.sh`

## Published Artifacts

- GitHub release:
  `https://github.com/instructa/switchloom/releases/tag/v0.3.0`
- GitHub release state: public, non-draft, non-prerelease.
- Release workflow run: `29857502558`
- Public npm package: `switchloom@0.3.0`
- Public npm tarball:
  `https://registry.npmjs.org/switchloom/-/switchloom-0.3.0.tgz`
- Public npm shasum: `7ccf5c8c693c33f5a83fad47529e630c46c8cf2c`
- Public npm integrity:
  `sha512-NdEyH1zq+W6E5pZIHdG8oe2CQxVjtjVlvQlvdCMxDEwaroCLzXEMiUMvnd/kjgXGx5A/WYsFxWe/nU7eSNgjzw==`
- Public npm file count: `20`
- Public npm unpacked size: `11898450`
- Public npm publish time: `2026-07-21T18:34:48.188Z`
- Public npm attestation URL:
  `https://registry.npmjs.org/-/npm/v1/attestations/switchloom@0.3.0`
- Public npm attestation predicate:
  `https://slsa.dev/provenance/v1`
- Public npm registry signature key:
  `SHA256:DhQ8wR5APBvFHLF/+Tc+AYvPOdTpcIDqOhxsBHRwC7U`
- Public npm tarball SHA-256:
  `95ec712b3debab10e33f51652d644c531c5ea85fac4e90bb87369374514b68f5`
- Homebrew stable version: `0.3.0`
- Homebrew formula URL:
  `https://github.com/instructa/switchloom/releases/download/v0.3.0/switchloom-darwin-arm64.tar.gz`
- Homebrew formula checksum:
  `e87d40f28553996b27313c82823f5f633e8e616197484d3b110bc1dc19ecb039`
- Homebrew tap HEAD: `2fdec6450d4444febc54fcf7ffe44e7072e59c74`
- Website worker URL:
  `https://model-routing-prod-catalog.office-35d.workers.dev`

GitHub release asset SHA-256 values:

| Asset | SHA-256 |
| --- | --- |
| `SHA256SUMS` | `a08021d57bd639244fab0a8ea575f3a2a5d77037fb6dbbae8e29d7db2bbc45ec` |
| `switchloom-darwin-arm64.tar.gz` | `e87d40f28553996b27313c82823f5f633e8e616197484d3b110bc1dc19ecb039` |
| `switchloom-darwin-x86_64.tar.gz` | `81485d39947f7e27a8693014b8eab346cfe7eaa72d3466420ac24068be12946e` |
| `switchloom-linux-arm64.tar.gz` | `09e063d95dc55a12108f75eacc1f3e7e04a4783d537244b4a94180cd2c34f678` |
| `switchloom-linux-x86_64.tar.gz` | `4636f564f884b6e710a1aa78a7429731995c69006603fab4ea663f6378d40ded` |

Public asset checksum command:

- Command: `tmpdir=$(mktemp -d /private/tmp/switchloom-release-v0.3.0-assets.XXXXXX); gh release download v0.3.0 --repo instructa/switchloom --pattern 'switchloom-*.tar.gz' --pattern SHA256SUMS --dir "$tmpdir"; cd "$tmpdir"; shasum -a 256 -c SHA256SUMS; cat SHA256SUMS`
- Result: `shasum -a 256 -c SHA256SUMS` passed for all four public release
  archives; `cat SHA256SUMS` returned the archive hashes listed above.

Public npm provenance command:

- Command: `root=$(mktemp -d /private/tmp/switchloom-public-v0.3.0-npm-tarball.XXXXXX); cd "$root"; npm pack switchloom@0.3.0 --json > pack.json; shasum switchloom-0.3.0.tgz; shasum -a 256 switchloom-0.3.0.tgz; tar -xzf switchloom-0.3.0.tgz package/npm/native/provenance.json package/package.json; node -e 'const fs=require("fs"); const p=JSON.parse(fs.readFileSync("package/npm/native/provenance.json","utf8")); if (p.git_sha !== "45c15fed09786c9dee1167744b1c43b87b47d505") throw new Error("unexpected git_sha"); if (p.package_version !== "0.3.0") throw new Error("unexpected package_version"); if (!Array.isArray(p.targets) || p.targets.length !== 4) throw new Error("expected four native targets"); for (const t of p.targets) { if (t.git_sha !== p.git_sha || t.version !== "model-routing 0.3.0") throw new Error(); }'`
- Result: public tarball SHA-1 matched npm shasum
  `7ccf5c8c693c33f5a83fad47529e630c46c8cf2c`, public tarball SHA-256 was
  `95ec712b3debab10e33f51652d644c531c5ea85fac4e90bb87369374514b68f5`, and
  `npm/native/provenance.json` recorded `package_version: 0.3.0`,
  `git_sha: 45c15fed09786c9dee1167744b1c43b87b47d505`, and four native
  targets.

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

## Fresh Install Receipts

Public npm receipt:

- Root: `/private/tmp/switchloom-public-v0.3.0-npm-fix.GMEH4n`
- Install mode: fresh npm global-prefix install.
- Durable Planr evidence: `i-fix-findings-for-review-certify-f413`,
  `log-c67dc0dd`.
- Reproducible command: complete `set -euo pipefail` npm command in
  `log-c67dc0dd`, creating explicit `root`, `prefix`, `repo`, and `cache`,
  installing public `switchloom@0.3.0`, and failing closed on each assertion.
- Commands covered: `--version`, `--help`, `compile`, `inspect`, `preview`,
  `apply`, `status`, and `uninstall`; the help assertion also checked
  `policy`, `update`, `rollback`, and `doctor`.
- Unmanaged sentinel hash before and after:
  `60ba63428d6029222a9d092142fcdc64d045d93650e0c25cb25cd7f319351fae`
- Authenticated Codex config hash before and after:
  `eec8236cc13bd729a46376d65624eaf795a70bd48ccee8ac707d904f735aaedd`
- Post-uninstall status: no managed artifacts.

Public Homebrew receipt:

- Root: `/private/tmp/switchloom-public-v0.3.0-brew-fix.sA9meG`
- Install mode: `brew reinstall instructa/tap/switchloom`.
- Durable Planr evidence: `i-fix-findings-for-review-certify-f413`,
  `log-c67dc0dd`.
- Reproducible command: complete `set -euo pipefail` Homebrew command in
  `log-c67dc0dd`, creating explicit `root` and `repo`, asserting formula
  version `0.3.0` plus checksum
  `e87d40f28553996b27313c82823f5f633e8e616197484d3b110bc1dc19ecb039`,
  reinstalling `instructa/tap/switchloom`, and failing closed on each
  lifecycle assertion.
- Commands covered: formula version and checksum assertion, `--version`,
  `--help`, `compile`, `inspect`, `preview`, `apply`, `status`, and
  `uninstall`.
- Unmanaged sentinel hash before and after:
  `60ba63428d6029222a9d092142fcdc64d045d93650e0c25cb25cd7f319351fae`
- Authenticated Codex config hash before and after:
  `eec8236cc13bd729a46376d65624eaf795a70bd48ccee8ac707d904f735aaedd`
- Post-uninstall status: no managed artifacts.

Public website receipt:

- Root: `/private/tmp/switchloom-public-v0.3.0-website-fix.Ou1eQ0`
- Durable Planr evidence: `i-fix-findings-for-review-certify-f413`,
  `log-c67dc0dd`.
- Production verification:
  `node scripts/verify-cloudflare-website.mjs https://switchloom.ai target/release/model-routing`
- Result: 28 catalog entries, 6 setup hosts, and
  `balanced-codex-openai` parity hash
  `3289f2d7231639a7084c4455c13448e89a1f596a2877672db5b1962d06f5f905`.
- Live guidance result:
  `live website guidance fix passed: 28 compositions, 6 hosts, 1 Generator asset(s)`.
- Reproducible command: complete `set -euo pipefail` website command in
  `log-c67dc0dd`, fetching the live index, catalog, and linked Generator JS,
  then asserting `npm install -g switchloom@0.3.0`,
  `npx switchloom@0.3.0 apply --recipe`, lifecycle commands, Cursor advisory
  wording, and Claude unavailable/unverified wording.

## Limitations

- Claude Code is skipped/unavailable/unverified until live receipts exist; no
  Claude effective model or effort telemetry is certified for this release.
- Cursor host checks remain advisory where the host returns the nonce but not
  host-authenticated effective model or effort telemetry.
- This receipt updates only the Switchloom repository. The Planr repository is
  intentionally unchanged until Goal B starts after review.

## Handoff

`docs/planr-hard-cut-handoff.md` is the durable Goal B oracle. Goal B should
consume released `switchloom@0.3.0` declarations or fixtures and remove Planr's
legacy routing compiler/catalog ownership without reintroducing a second source
of truth.
