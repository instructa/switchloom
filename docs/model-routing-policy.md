# Model Routing Policy

Switchloom owns opinionated model selection and host bindings for supported agent hosts. The standalone compiler emits `RoutingBundle v1` JSON with deterministic profiles, routes, artifacts, hashes, and evidence labels.

Standalone compilation is the default:

```sh
model-routing compile balanced --host codex-openai --output routing-bundle.json
```

The default bundle contains repository-local host artifacts only. Optional Planr integration is explicit:

```sh
model-routing compile balanced --host codex-openai --integration planr --output routing-bundle.json
```

Inspect validates a bundle and emits a machine-readable summary:

```sh
model-routing inspect routing-bundle.json
```

## Adapter Contract

Each generated bundle carries `adapter_contract.schema_version = 1`. The contract separates Switchloom-owned routing declarations from host-owned runtime behavior:

- `runtime_class` is `native-subagent` for repository-local host agents and `external-runner` for process/workflow runners.
- `RoutingIntentV1` records semantic roles plus each role's requested model, effort, and adapter instructions.
- `HostCapabilityV1` records host version constraints, model/effort control, context semantics, nesting, parallelism, observability, and known limitations.
- `HostCapabilityV1.runtime_behavior` records the versioned host runtime facts behind those controls: installed host-version source, actual backend selection owner, trust/discovery behavior, role precedence, shared filesystem behavior, explicit child dispatch, Ultra automatic delegation, and source references.
- `HostAdapterV1` records the dispatch recipe and the Switchloom-managed artifact paths it emits.
- `DispatchEvidenceV1` is the persisted requested-versus-effective receipt: package digest, host version, requested dispatch, child identity with the host's effective `agent_role`, effective model/effort when observable, nonce, raw evidence references, and verdict.
- A `deterministic` dispatch-evidence verdict requires observed effective model and effort fields to match the request. If a host can silently override a model or does not expose the effective model, the receipt must remain `advisory` or `unsupported`.
- Guarantee levels are `deterministic`, `advisory`, and `unsupported`; required guarantees may not be `unsupported`.
- Switchloom owns semantic roles, model/effort identifiers, fork/context policy, generated artifacts, and managed lifecycle state.
- Host runtimes own effective execution, account/workspace precedence, billing, process/session behavior, and live requested-versus-effective evidence.
- Planr consumes semantic work types and roles from the contract; it must not duplicate model, effort, role, or fork normalization.

Offline evaluations remain `experimental` until authenticated live-host evidence and a maintainer signature are available.

### Codex V2 Runtime Contract

Codex V2 is modeled as `native-subagent`, not an external process runner.
Switchloom compiles repository-local `.codex/agents/*.toml` role files and the
managed `.codex/config.toml` registration entries. Codex owns project trust,
agent discovery after reload/restart, backend availability, execution timing,
parallel child scheduling, billing, and any account/workspace precedence that
affects the effective backend.

For Codex, the runtime behavior contract freezes `codex-cli 0.144.5` as both
the minimum and maximum supported capability version for this contract slice.
The digest-bound source artifact is
`docs/codex-v2-runtime-evidence.json#sha256:<digest>`. That artifact records
the `codex --version` observation, Codex account/workspace state as the actual
backend selection owner, a three-child limit derived from four active agents
including the root session, shared repository filesystem behavior, explicit
dispatch, Ultra behavior, and role precedence:

- `spawn_agent.agent_type` selects the registered project role file.
- The selected role file declares requested child model and effort fields when
  present.
- Omitted custom-agent model and effort fields inherit from the parent session.
- Parent live sandbox and approval choices are reapplied when spawning a child.
- Persisted Codex session/state and nonce-bearing child output are required
  before the effective model/effort claim is certified.

Native Codex setup is repository-local only: it writes managed `.codex`
project config and role files, preserves unrelated project and global Codex
configuration, and does not generate or instruct nested `codex exec` dispatch.
After apply/update/rollback, Codex may require trusting the project and
reloading or restarting the host session before the generated role
registrations are discoverable.

Explicit semantic-role dispatch and Ultra are separate modes. Light, Balanced,
and High presets use explicit `agent_type` dispatch with `fork_turns = none`;
Ultra is recorded as the automatic delegation mode and must remain separately
selected rather than becoming a default preset.

## Executable Setup Flow

The CLI and website share one setup intent. The website emits `SetupSpecV1`
recipes and config files from the same `setupContract` embedded in the generated
catalog; the CLI is the only writer of repository-local artifacts.

```sh
model-routing catalog build --output website/data/catalog.json
model-routing catalog verify website/data/catalog.json

model-routing compile balanced --host codex-openai --output routing-bundle.json
model-routing preview routing-bundle.json --repository .
model-routing apply routing-bundle.json --repository .

npx switchloom@latest preview --recipe 'sw1_...' --repository .
npx switchloom@latest apply --recipe 'sw1_...' --repository .
switchloom status --repository .
switchloom update --repository .
switchloom rollback --repository .
switchloom uninstall --repository .
switchloom doctor codex
switchloom certify reports/native-host-certification/<host>/<timestamp>/workdir/dispatch-evidence.json \
  --bundle reports/native-host-certification/<host>/<timestamp>/workdir/bundle.json
```

Use `--integration planr` only when the repository should receive optional
`.planr/agents.toml` and `.planr/policy.toml` declarations. Standalone setup
must not emit `.planr` files.

## Certification Flow

Certification requires a package digest, generated bundle, applied artifacts,
requested invocation, nonce-bearing child output, `DispatchEvidenceV1`, and the
bundle validator:

```sh
scripts/codex-standalone-oracle.sh
scripts/native-host-certification-oracle.sh cursor-openai target/debug/model-routing
scripts/native-host-certification-oracle.sh cursor-fable-grok target/debug/model-routing
scripts/native-host-certification-oracle.sh claude-native target/debug/model-routing
scripts/opencode-native-oracle.sh target/debug/model-routing
scripts/pi-external-oracle.sh target/debug/model-routing

switchloom certify reports/native-host-certification/<host>/<timestamp>/workdir/dispatch-evidence.json \
  --bundle reports/native-host-certification/<host>/<timestamp>/workdir/bundle.json
```

`switchloom doctor <host>` probes the selected host CLI and reports the detected
version without claiming authentication. `switchloom certify` is the
user-facing alias for the strict `evidence validate` receipt check.

For the active available-host release gate, Codex is the deterministic
effective-routing certification target. The two Cursor profiles are live
nonce-correlated requested-routing targets; their receipts remain `advisory`
unless Cursor exposes host-authenticated effective role/model telemetry. Claude
Code, OpenCode, and Pi remain unavailable or unverified unless their current
report directories contain authentic nonce-bearing child evidence and passing
validator output.

## Planr Consumer Handoff

Planr receives semantic intent only. The inputs are:

- `usage_policy`, `integration`, selected `host` binding, and `work_type` routes.
- Semantic roles with selected `profile`, `model`, `effort`, `agent_type`, and
  `fork_turns`.
- Runtime class: `native-subagent` for Codex, Claude Code, Cursor, and OpenCode
  host-native child dispatch; `external-runner` for Pi workflow/process dispatch.
- Release metadata: Switchloom package version, package digest, bundle id,
  catalog version, host version, report path, and validator stdout.

Forbidden duplicate ownership:

- No Planr-side model catalog, effort catalog, preset compiler, host adapter, or
  fork policy normalizer for Switchloom-owned inputs.
- No second website compiler, JSON-only setup path, or Planr-owned artifact
  lifecycle for Switchloom-managed files.
- No certified verdict for advisory, unsupported, unavailable, or unverified
  hosts without live nonce-bearing child evidence.

Minimum follow-up before release publication is to run the security and
published-byte gate, regenerate and verify the catalog from the release
candidate bytes, update website/docs version references if the package version
changes, and retain current reports for Codex, Cursor OpenAI, Cursor
Fable/Grok, and the unavailable or unverified Claude Code, OpenCode, and Pi
states.
