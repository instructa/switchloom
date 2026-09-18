# Codex capability routing

`catalog.toml` defines model suggestions, reasoning choices, default slots and
capabilities. `src/catalog.rs` reads it and exports the website catalog. Users
can assign any capability to any enabled card, including custom GPT model IDs.
The board's owner map is the single source for the generated prompt and its
per-slot capability lists. Disabled capabilities add no instructions.

Disabling a card releases its capabilities into Disabled. Re-enabling restores
that card's model, effort and default capabilities, reclaiming them from any
other card. Non-default capabilities remain Disabled. Reset restores the entire
board. No state is persisted. Empty cards create no task.

## Decision and binding

`switchloom route` accepts `task`, optional text `context` and `routing`:

- `{"mode":"jev"}` asks Jev to select a work capability for a new objective.
- `{"mode":"assigned","capability":"implementation"}` uses the supplied
  capability's configured owner without a network call.

Automatic requests contain no preselected capability. Assigned mode is for
explicit user assignments, saved continuations, or coordinator selection when
Jev is disabled. Knowing the ownership map is not an explicit assignment.
Input is bounded to 32 KiB. A sole eligible work capability requires an explicit
assignment; one candidate is not evidence that it fits. No eligible work
capabilities returns `missing_context`.

The catalog marks work capabilities as `routable`. Coordination stays on the
board to choose the workflow controller, but is never offered to Jev or dispatched
as a work assignment. Ordinary implementation includes inspecting files and
routine implementation choices. Planning addresses concrete blocking decisions
or explicit requests for a plan. Mechanical includes simple factual retrieval,
obvious commands and fully specified transformations. CLI/MCP are tool access
methods. Computer and Browser use cover standalone graphical interaction
objectives; incidental tool use stays within the current assignment and shared
tool ownership. Visual design & review defines visual direction and evaluates
rendered results. Its owner returns a design brief or concrete corrections to
the implementation owner. Software architecture belongs to Planning; code
correctness belongs to Code review. No mandatory design step is added.

With more than one work candidate, one Jev request asks a Choice and a separate
Noul for missing context. Classifier criteria are the catalog `what`, `not_for`
and examples, not assignment instructions. State is `task`, `context` and
`enabled_capabilities`. Slot and thread IDs never go to TypeSafe. The judge
model is pinned to `jev-1.13.0`.

A missing-context Noul at or above `0.7` abstains. Otherwise a routine
capability acts at `0.7` and can be suggested from `0.5`. Review, browser,
computer, visual and spatial work act at `0.85` and can be suggested from
`0.7`. A suggestion keeps the assignment and sends nothing. These floors are
uncalibrated starting points, not a quality guarantee. The labeled cases in
`evaluations/capability-cases.toml` are not sent to Jev and are not a result.

If capability confidence is too low to act, candidates assigned to the same
owner can still identify a recipient. The core sums their Choice probabilities
and requires the strictest act floor among those positive-probability candidates.
At least two candidates must share the owner. This returns `jev_owner` and passes
their relevant duties with the original objective; it does not turn them into a
mandatory sequence. Moving a candidate to another card immediately changes this
calculation. Missing context still blocks dispatch. `owner_probability` is this
sum, not Jev confidence: original confidence and probabilities remain unchanged.
This ownership rule is also uncalibrated.

Standalone `route` uses default ownership. `switchloom handoff` accepts that
request plus `caller` and `threads`. Each configured slot carries `thread_id`,
`host_id`, `model`, `effort` and `capabilities`. It validates model settings,
unique targets and unique capability ownership before any network request.
Disabled capabilities have no entry. Total handoff input is bounded to 64 KiB.
Unknown valid GPT IDs are passed unchanged; Codex owns actual availability.

Jev receives the named state and classifier criteria. Slot/thread/host IDs,
model settings and keys are not part of its judgment state. The selected
capability deterministically binds to its configured owner and model. Jev does
not optimize model prices or run per tool call. Model and tool routing remain
separate; tool routing is outside this release.

Output is `{decision, action, dispatch}`. Decisions include actual API usage
when called. `dispatch` has the exact `threadId`, `hostId`, `model`, `thinking`
and `prompt` arguments for `send_message_to_thread`. `continue_here` keeps the
step in the caller's current turn without a self-message. `suggest` and
`clarify` send nothing.
Handoff output contains supplied context and must not be logged publicly.

Transport has a five-second deadline, a 64 KiB response bound, no retries or
redirects and sanitized failures. Keys come only from `TYPESAFE_API_KEY`.
Input/service errors exit nonzero with no JSON dispatch on stdout.

## Dispatch, end turn and resume

The generated prompt authorizes bootstrap in the existing project checkout,
with `environment: local`. Reuse suitable tasks; resolve pending setup before
sending. Creation is performed by Codex, never by the CLI. The coordination
owner starts the workflow; without one, the planning owner or first assigned
slot sequences work. This adds no specialist duties or extra manager task.

The [skill](../skills/switchloom/SKILL.md) prepares one handoff per meaningful
step in both modes. Every new unassigned objective uses Jev in the enabled workflow. Saved
continuations and direct user assignments use assigned mode. With Jev disabled,
the coordinator selects a work capability in assigned mode and needs no TypeSafe key. Both modes retain
the same message protocol. `suggest` is not a send:

1. Resolve the target, host and checkout; dispatch only to an idle task without
   an outstanding assignment. Preserve pending work if busy; never poll.
2. Include objective, constraints, ownership, relevant evidence and return IDs.
   Send once, record the assignment and end the turn. Check acceptance before
   resending an ambiguous send.
3. Recipient replies with `Result for:`, objective, sender IDs, diff/evidence,
   limitations and requested next action, then ends its turn.
4. Consume result messages without classifying them. If work is blocked, save
   its objective and owner while obtaining advice. Return that advice to the
   original owner in assigned mode; keep relevant plan and evidence in context.
   Route a genuinely new unassigned objective once. No echoes, acknowledgments or
   progress-only exchanges. Preserve ownership and pending work across compaction.

Desktop workers may use subagents for independent parts of their assigned step
with the same selected model and effort. They must verify those settings and
retain capability and file/tool ownership. If settings cannot be verified, the
owning task does the work. Subagents do not replace persistent tasks, and the
coordinator does not spawn them. This is a prompt rule, not a technical model
lock: Codex's model-override exposure flag does not prevent model changes through
configured agent defaults or roles. Switchloom does not edit personal Codex
settings or TOML files. The Desktop workflow creates no worktrees or separate runtime.
The optional local app-server harness has the same selection and return contract.
It routes the original objective directly in Jev mode, then the coordinator
sequences results and new objectives. Worker turns return a complete or blocked
result. The host retains suspended assignments and restores them on resume;
coordinator output cannot select a capability in Jev mode.
The assigned owner coordinates exclusive file and shared-tool access. Selected review/validation
capabilities determine checks. Self-review is not independent review.

The catalog's `completion_instructions` is shared by the Desktop prompt and
local harness. For UI work with a supplied reference or requested visual checks,
completion includes comparison of the rendered result by the configured visual
owner; a standalone executor performs it itself. Concrete findings return to
implementation. A build is not visual evidence, and unavailable checks remain
explicit limitations. This is an agent instruction, not a runtime proof that a
check happened; logs and artifacts must establish that in benchmark results.

Each task retains its model, reasoning and native tool loop. Existing histories
are extended with assignments and results; a worker receives the original
objective once rather than duplicated in provenance labels. No proxy, tool-call
classifier or custom cache layer is inserted.
Persistent tasks have separate histories, so common prefixes do not by themselves
prove cache reuse. Compare actual cached-input, cache-write and output usage.

## Verification

Tests cover board resets, prompt omission, custom models, configured ownership,
abstention, offline vanilla, HTTP, dispatch shape and resuming a blocked worker
after advice without reclassification. Eight live requests through `/api/route`
on 17 September matched their expected capabilities: CLI and filesystem-MCP
lookups → mechanical/Luna; three ordinary implementations → implementation/Sol;
an ownership decision → planning/Astra; native and browser GUI work → their
respective capabilities. These development probes are not independent calibration.
An actual Jev/Codex run selected Sol, implemented and tested a small function,
then returned to Luna: two turns, one Jev call. A separate five-turn run without
Jev exercised Luna → Astra → Luna → Sol → Luna with three persistent tasks.
Track the full Desktop flow in [TASKS.md](../TASKS.md). Compare Jev and
vanilla on equal tasks and starting states, including all routing and model
calls, latency and repair work. API cost and subscription quota are separate.
No measured quality or savings claim is established.
