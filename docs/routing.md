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

`switchloom route` accepts `task`, optional text `context`, optional
`capability`, and required boolean `jev`. Input is bounded to 32 KiB.
An explicit capability uses its owner without a network call. With `jev:false`
and no explicit capability, the CLI abstains with `explicit_capability_required`.
With `jev:true`, one TypeSafe Choice selects the immediate next capability or
`clarify`. Only enabled capabilities and their catalog instructions are eligible.
The complete probability distribution is validated; confidence below `0.7`
abstains. This threshold is uncalibrated, not a quality guarantee.

Standalone `route` uses default ownership. `switchloom handoff` accepts that
request plus `caller` and `threads`. Each configured slot carries `thread_id`,
`host_id`, `model`, `effort` and `capabilities`. It validates model settings,
unique targets and unique capability ownership before any network request.
Disabled capabilities have no entry. Total handoff input is bounded to 64 KiB.
Unknown valid GPT IDs are passed unchanged; Codex owns actual availability.

Jev receives only task/context and capability criteria. Slot/thread/host IDs,
model settings and keys are not part of its judgment state. The selected
capability deterministically binds to its configured owner and model. Jev does
not optimize model prices or run per tool call. Model and tool routing remain
separate; tool routing is outside this release.

Output is `{decision, action, dispatch}`. Decisions include actual API usage
when called. `dispatch` has the exact `threadId`, `hostId`, `model`, `thinking`
and `prompt` arguments for `send_message_to_thread`. `continue_here` keeps the
step in the caller's current turn without a self-message. `clarify` sends nothing.
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

With Jev enabled, the [skill](../skills/switchloom/SKILL.md) prepares a handoff
once per meaningful step. Explicit ownership bypasses classification. In vanilla
mode the agent uses the same ownership map directly with Codex tools; it needs
neither the CLI nor an API key. Both modes retain the same message protocol:

1. Resolve the target, host and checkout; dispatch only to an idle task without
   an outstanding assignment. Preserve pending work if busy; never poll.
2. Include objective, constraints, ownership, relevant evidence and return IDs.
   Send once, record the assignment and end the turn. Check acceptance before
   resending an ambiguous send.
3. Recipient replies with `Result for:`, objective, sender IDs, diff/evidence,
   limitations and requested next action, then ends its turn.
4. Consume the result as a continuation. No echoes, acknowledgments or
   progress-only exchanges. Preserve ownership and pending work across compaction.

No subagent substitutes, worktrees, session store or separate runtime. Only the
assigned owner writes files or uses shared tools. Selected review/validation
capabilities determine checks. Self-review is not independent review.

## Verification

Tests cover board resets, prompt omission, custom models, configured ownership,
abstention, offline vanilla, HTTP and dispatch shape. Earlier live role-routing
smokes predate the capability contract and do not validate it. Two capability smokes with `jev-1.13.0` selected implementation/Sol and
planning/Astra after clarifying that absent execution details alone do not
prevent classification. This is not independent calibration. Track the full
desktop flow in [TASKS.md](../TASKS.md). Compare Jev and
vanilla on equal tasks and starting states, including all routing and model
calls, latency and repair work. API cost and subscription quota are separate.
No measured quality or savings claim is established.
