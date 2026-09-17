---
name: switchloom
description: Route bounded work with TypeSafe/Jev to configured capability owners in persistent Codex desktop threads. Dispatch an assignment, end the turn and resume on its result message. Use for Switchloom workflows and their handoffs.
---

# Switchloom task handoffs

Codex owns the threads and tool loops. Switchloom prepares one assignment;
the recipient replies to resume the caller. Follow the user's selected
responsibilities, including validation ownership and optional specialist work.

## Resolve owners once

Use the workflow's slot-to-thread/host map, model/effort and capabilities per slot.
Carry those selections into every handoff; never replace them with defaults.
Resolve names with `list_threads`;
verify the project checkout and host. Never infer ownership from a similar name
in another project. Use supplied context or `CODEX_THREAD_ID` for the caller;
never guess IDs. Recover the map and pending assignments after compaction.

Create missing tasks only when the user has authorized thread creation, as
in the generated workflow prompt. Resolve the project with `list_projects`;
use its existing checkout with environment type `local` and the configured
model/reasoning. Reuse this thread when suitable. New tasks end their setup
turn until assigned work; give them the complete map and selected rules in
the first actionable assignment. Record pending creation IDs and resolve
setup before dispatch; a client ID is not a thread ID. Do not fork, create
worktrees, replace pending tasks or add subagents. The generated prompt owns
the bootstrap responsibilities and selected models. The catalog supplies
defaults and suggestions; Codex validates actual model availability. New GPT
IDs can be used without waiting for a Switchloom catalog update.

Require `switchloom` and `send_message_to_thread`; discover deferred tools.
If a required capability is unavailable, report it without claiming dispatch.
Jev needs `TYPESAFE_API_KEY` in the CLI environment. If exported in `.zshrc`,
use `zsh -ic 'exec switchloom handoff' < INPUT_FILE`. Never print the key or
shell-config contents. Noninteractive zsh does not load `.zshrc`.

## Route one assignment

Write bounded task/context to a temporary JSON file with a file-writing tool
or structured subprocess input, not shell interpolation. Include relevant
ownership, acceptance criteria and the capability map when the recipient lacks them.

```json
{
  "request": {"task": "Review the cache fix", "context": "Relevant paths, evidence, ownership and acceptance criteria", "capability": "review", "jev": true},
  "caller": {"thread_id": "CALLER_ID", "host_id": "HOST_ID"},
  "threads": {
    "sol": {"thread_id": "SOL_ID", "host_id": "HOST_ID", "model": "gpt-5.6-sol", "effort": "medium", "capabilities": ["implementation", "debugging", "validation"]},
    "astra": {"thread_id": "ASTRA_ID", "host_id": "HOST_ID", "model": "gpt-6-astra", "effort": "high", "capabilities": ["planning", "review"]}
  }
}
```

Use the workflow's actual model settings and capability lists, not the example's
defaults. Slot IDs remain stable when their models change. Each capability has
one owner; disabled capabilities are absent. Run `switchloom handoff < INPUT_FILE`,
then remove the file. Explicit assignments use `request.capability` and skip Jev.
Otherwise omit it with `jev:true` to judge among assigned capabilities. Report
failed judgments; never hide missing keys or service errors. Route once per
meaningful step; keep the model fixed throughout its tool loop.

Vanilla workflows use the configured map directly with Codex tools and need no
CLI or key. If calling the CLI in vanilla mode, set `jev:false` and an explicit
capability; without one it abstains with `explicit_capability_required`.

On a nonzero exit, send nothing; report the error without an automatic retry.
`clarify` requires missing context or resolution of uncertainty.
`continue_here` executes the selected step in this turn without a self-message
or model switch. Only `dispatch` carries a send payload.

Inspect the recipient once with `read_thread` (`turnLimit: 1`,
`includeOutputs: false`) before a new assignment. Dispatch only when its
checkout matches and it is idle with no outstanding assignment. If busy or
blocked, preserve the next action and end the turn; do not interrupt or poll.
Pass `dispatch` unchanged to `send_message_to_thread` once. Record the
objective, target, ownership and expected result, then **end your turn**.
An ambiguous send requires checking whether it was accepted before any resend.

## Reply and resume

On completion, a blocker or a needed decision, the recipient sends one
substantive reply to the assignment's return thread and host. Preserve that
thread's model settings. Mark it `Result for:` with the objective and sender
thread ID. Include relevant files/diff, evidence, limitations and requested
action. Then end the turn. No acknowledgments, progress-only exchanges,
polling or waiting loops.

Match an incoming result to the pending objective and sender. Consume it as
a continuation, not an assignment to echo back or send through Jev again.
Advice resumes the original work; a coordinator passes the next bounded
step to its assigned owner. Follow the configured validation and review
responsibilities, reuse evidence and request repairs only for concrete
failures. Stop when acceptance criteria are met. Report blockers that require
user input or scope changes; do not create recurring repair or review loops.

Preserve the capability map with model/effort, file/tool ownership, pending assignment and next action
across compaction. Report actual work and evidence; do not claim savings from
one successful handoff.
