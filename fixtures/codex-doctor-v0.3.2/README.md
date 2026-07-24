# Codex Doctor v0.3.2 regression freeze

This directory freezes observations made before the semantic Doctor repair. It
does not change or certify current Doctor behavior.

`applied-semantic-recipe.toml` is the valid repository state: it registers the
semantic `switchloom_*` roles and enables both Codex V2 flags. The current
v0.3.2 Doctor nevertheless warns that three absent legacy
`model_routing_*` registrations are required; their exact messages are in
`expected-v0.3.2-doctor.json`.

`intentionally-broken-v2-flags.toml` is deliberately separate. A repair must
be able to diagnose the two false V2 flags without confusing them with the
valid semantic-role state.

`authenticated-spawn-oracle.json` distinguishes a declared route from proof:
a successful parent spawn call, a child-parent thread edge, effective child
`turn_context`, and child completion are all required. It also preserves the
pre-reload `unknown agent_type` result as a host-discovery failure rather than
a fallback.

`same-package-repository-provenance.json` freezes the npx self-shadowing
contrast. In a repository named `switchloom`, `npx switchloom@0.3.2 --version`
ran the stale local `npm/native/darwin-arm64/model-routing` and reported 0.3.1.
The packed registry artifact and a neutral-directory npx run reported 0.3.2
and generated `hide_spawn_agent_metadata = true`. Future release tests must
capture executable path and digest before claiming registry provenance.
