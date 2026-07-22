# Switchloom 0.2.2 Publish Certification

Date: 2026-07-21
Pre-publish Planr item: `i-pass-security-and-immutable-pre-bb8d`
Publish Planr item: `i-publish-and-post-publish-verify-de77`

## Candidate

- Candidate version: `0.2.2`
- Source commit: `206ab2ba15724e438486ef5dfa1ee31888858c19`
- Crate digest: `sha256:b1a8a8a984ddb9be0e2457ccf8a747179f451c8bc4b69d0b9e4604b3b87a1349`
- Published npm tarball digest:
  `sha256:ace8594db2f972a754dbfd26590f3196fbd8abaf58648aef1407bc3c463eddc6`
- Published npm tarball path:
  `/private/tmp/switchloom-public-0.2.2/npm-pack/switchloom-0.2.2.tgz`
- Fresh public install repository:
  `/private/tmp/switchloom-public-0.2.2/fresh-npm`
- Installed macOS ARM native digest:
  `sha256:363c8e146988f315fb46e30083523b82c3f61486631188ea30d218549931b8b6`
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
- `cargo package --locked --no-verify` from clean worktree `/private/tmp/model-routing-clean-206ab2b` packaged 161 files as `model-routing v0.2.2`.
- `tar -xOf target/package/model-routing-0.2.2.crate model-routing-0.2.2/.cargo_vcs_info.json` showed git sha1 `206ab2ba15724e438486ef5dfa1ee31888858c19` and no dirty marker.
- `tar -tf target/package/model-routing-0.2.2.crate | rg 'prepublish-certification|Cargo.toml|README.md'` showed `Cargo.toml`, `Cargo.toml.orig`, and `README.md`; no prepublish certification report is included.
- `gh run watch 29820438528 --exit-status --interval 15` passed the four-target release-candidate native matrix and package validation job for commit `206ab2ba15724e438486ef5dfa1ee31888858c19`.
- `gh run download 29820438528 --dir /private/tmp/switchloom-rc-29820438528`
- `node scripts/validate-native-provenance.mjs` validated `npm/native/provenance.json` for `0.2.2`.
- `bash scripts/npm-pack-check.sh` passed with final matrix artifacts staged in ignored `npm/native`.
- `npm_config_cache=/private/tmp/switchloom-npm-cache npm pack --pack-destination /private/tmp/switchloom-rc-29820438528-pack` produced `switchloom-0.2.2.tgz`, 19 files, shasum `9bd5b6ef39609e3276a09433c502973744a05ecd`.
- `npm_config_cache=/private/tmp/switchloom-npm-cache npm install --prefix /private/tmp/switchloom-packed-0.2.2-29820438528 /private/tmp/switchloom-rc-29820438528-pack/switchloom-0.2.2.tgz`
- `/private/tmp/switchloom-packed-0.2.2-29820438528/node_modules/.bin/switchloom --version` returned `model-routing 0.2.2`.
- `bash -n scripts/release.sh scripts/build-release.sh scripts/npm-pack-check.sh scripts/secleak-check.sh scripts/codex-standalone-oracle.sh scripts/native-host-certification-oracle.sh`
- `git diff --check`
- `git check-ignore -v reports/native-host-certification/current/certification-summary.json dist/website/index.html target/release/model-routing npm/native/darwin-arm64/model-routing .planr`
- `${HOME}/.agents/skills/secleak-check/scripts/secleak-check.sh` passed after removing generated `node_modules`; BetterLeaks found no leaks and Trivy reported zero Cargo vulnerabilities.
- `scripts/build-release.sh`
- `target/release/model-routing --version`
- `SWITCHLOOM_NATIVE_BIN="$PWD/target/release/model-routing" node npm/bin/model-routing.js --version`

## Published Artifacts

- Release tag: `v0.2.2`
- Tag target: `206ab2ba15724e438486ef5dfa1ee31888858c19`
- Release workflow run: `29822125156`
- GitHub release:
  `https://github.com/instructa/switchloom/releases/tag/v0.2.2`
- GitHub release state: published, non-draft, non-prerelease,
  `2026-07-21T10:26:08Z`
- Public npm package: `switchloom@0.2.2`
- Public npm tarball:
  `https://registry.npmjs.org/switchloom/-/switchloom-0.2.2.tgz`
- Public npm shasum: `928fd2a847acc9f30a94ed96bf71500e41ea443d`
- Public npm integrity:
  `sha512-+N8UCvDIIAkWsxWSfE9nerP6u9S9hl6vb3hH6CWkx2mUDGmqmdAm8tmU8rL9csCn5RITox3Ite8320OndB4R7A==`
- Public npm publish time: `2026-07-21T10:26:27.505Z`
- Public npm tarball sha256:
  `ace8594db2f972a754dbfd26590f3196fbd8abaf58648aef1407bc3c463eddc6`
- GitHub release asset sha256:
  - `SHA256SUMS`:
    `b046c8ac4b5e3f6d12afc061676b1452d68eabc1e211d419145a6e924afb7958`
  - `switchloom-darwin-arm64.tar.gz`:
    `74743a04809a0f625a791a1dc10f72e5f21a7ef5d6d99ad872d5740ebccbd6a9`
  - `switchloom-darwin-x86_64.tar.gz`:
    `49f25b97a5958a948824da6b20176aaf0b04af39d2b7a895d5de5b5f72faa73b`
  - `switchloom-linux-arm64.tar.gz`:
    `7113cc9a2d8cd72e0f47ca257adea033f88b276fa0758d973a1a14b7af94e5f5`
  - `switchloom-linux-x86_64.tar.gz`:
    `fc996e41fbf69beab9538c7e8132a64afe463ebfe2d1060d7168a69c817d033a`
- `shasum -a 256 -c SHA256SUMS` passed for all four release archives
  downloaded from the public GitHub release.
- Homebrew tap formula `instructa/tap/switchloom` reports stable version
  `0.2.2` after `brew update`; formula checksums match the GitHub release
  archive checksums.
- Website deploy command: `pnpm exec alchemy deploy --stage prod`
- Website deploy resource URL:
  `https://model-routing-prod-catalog.office-35d.workers.dev`
- Public website verification:
  `node scripts/verify-cloudflare-website.mjs https://switchloom.ai /private/tmp/switchloom-public-0.2.2/fresh-npm/node_modules/switchloom/npm/native/darwin-arm64/model-routing`
- Website verification result: `cloudflare website verified:
  https://switchloom.ai/`, 28 catalog entries, 6 setup contract hosts,
  download parity for `balanced-codex-openai.json` digest
  `b31c37e9036b25f4ea413672eb227704830e4b4199de4041d0b36dc91d0c7980`.

## Runtime Receipts

Codex V2 public npm-byte oracle:

- Command: `SWITCHLOOM_CODEX_ROUTING_BIN=/private/tmp/switchloom-public-0.2.2/fresh-npm/node_modules/switchloom/npm/native/darwin-arm64/model-routing SWITCHLOOM_CODEX_PACKAGE_DIGEST=sha256:ace8594db2f972a754dbfd26590f3196fbd8abaf58648aef1407bc3c463eddc6 scripts/codex-standalone-oracle.sh`
- Receipt root: `/private/tmp/model-routing-codex-standalone.W1zxdu`
- Host version: `codex-cli 0.144.5`
- Runtime evidence:
  `/private/tmp/model-routing-codex-standalone.W1zxdu/codex-runtime-evidence.json`
- Runtime evidence validation: `codex runtime evidence validation passed`
- Package digest in evidence:
  `sha256:ace8594db2f972a754dbfd26590f3196fbd8abaf58648aef1407bc3c463eddc6`
- Implementer route: `model_routing_terra_high`, `gpt-5.6-terra`, `high`, `fork_turns = none`
- Reviewer route: `model_routing_sol_high`, `gpt-5.6-sol`, `high`, `fork_turns = none`
- Dynamic nonces:
  - `019f8438-2549-7122-95d3-5671dc3922d4:019f8438-42eb-7380-a28a-b1466f912c30:call_S8XqBDl58FOUyFr22NjOGUZa`
  - `019f8438-2549-7122-95d3-5671dc3922d4:019f8438-4d60-75d1-8bf2-0f832ce00674:call_7IJCWtqTUS9miLL16s5X3VCS`
- Isolated lifecycle `CODEX_HOME` config hash stayed equal before, preview, apply, update, rollback, and uninstall:
  `e7dbedaae9fbc83482530d1e9f837c41261ee9c53222ef4fdda0215b2c978f16`
- Authenticated current `CODEX_HOME` config hash stayed equal before and
  after the live Codex run:
  `c9434246a32b22e7236e9f9c12d93d925662fa94c32a028e54939721f0f09caf`.
  The retained receipt records this same value as the unchanged hash.
- Project-local unrelated Codex config and role were preserved before,
  after-preview, after-apply, after-update, after-rollback, and
  after-uninstall.

Cursor OpenAI oracle:

- Command: `scripts/native-host-certification-oracle.sh cursor-openai /private/tmp/switchloom-public-0.2.2/fresh-npm/node_modules/switchloom/npm/native/darwin-arm64/model-routing`
- Latest report: `reports/native-host-certification/cursor-openai/20260721T104854Z`
- Prior successful pre-publish report:
  `reports/native-host-certification/cursor-openai/20260721T073957Z`
- Host version: `2026.07.17-3e2a980`
- Package digest: `sha256:363c8e146988f315fb46e30083523b82c3f61486631188ea30d218549931b8b6`
- Requested model: `gpt-5.4-mini`
- Latest nonce: `e9bbdff7-18db-4782-89ec-b9ee11280fb4`
- Validation: `dispatch evidence validated`
- Verdict: `advisory`; Cursor returned the requested nonce from the
  authenticated host invocation, but did not return host-authenticated
  effective model or effort telemetry.

Cursor Fable/Grok oracle:

- Command: `scripts/native-host-certification-oracle.sh cursor-fable-grok /private/tmp/switchloom-public-0.2.2/fresh-npm/node_modules/switchloom/npm/native/darwin-arm64/model-routing`
- Latest report:
  `reports/native-host-certification/cursor-fable-grok/20260721T104909Z`
- Prior successful pre-publish report:
  `reports/native-host-certification/cursor-fable-grok/20260721T074022Z`
- Host version: `2026.07.17-3e2a980`
- Package digest: `sha256:363c8e146988f315fb46e30083523b82c3f61486631188ea30d218549931b8b6`
- Requested model: `cursor-grok-4.5-medium`
- Latest nonce: `8041256c-63b5-482c-bf5f-0906ef755849`
- Validation: `dispatch evidence validated`
- Verdict: `advisory`; Cursor returned the requested nonce from the
  authenticated host invocation, but did not return host-authenticated
  effective model or effort telemetry.

Claude Code:

- Installed version: `2.1.133 (Claude Code)`
- Verdict: skipped/unverified.
- Reason: live Claude probe escalation was rejected for authenticated external CLI risk, and current plan context treats Claude Code as unavailable rather than a release-blocking live gate.

## Final Matrix Artifacts

- Release-candidate run: `29820438528`
- Published release run: `29822125156`
- `npm/native/provenance.json` package version: `0.2.2`
- `npm/native/provenance.json` git SHA:
  `206ab2ba15724e438486ef5dfa1ee31888858c19`
- Target digests:
  - `darwin-arm64`: `363c8e146988f315fb46e30083523b82c3f61486631188ea30d218549931b8b6`
  - `darwin-x86_64`: `75185e10d4af108cc0f516b6856804db84f37ffce5e2b8f8a2e12bca9b6decfa`
  - `linux-arm64`: `de68eb234cb721364d6119790987f1111f121784f1d0f3977273fbcb3f9565cf`
  - `linux-x86_64`: `a85086b214ba73ab36cf290d2f535ecafead751d969fd2a553ab2bd872e93c01`

The final public-byte Codex receipt is
`/private/tmp/model-routing-codex-standalone.W1zxdu`.
