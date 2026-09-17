# Switchloom

**One workflow prompt for persistent Codex tasks, with optional TypeSafe/Jev routing.**

Assign capabilities to models, then copy the prompt into Codex Desktop with
your task. Each model uses a persistent task with its own context. Assignments
end the caller's turn; substantive replies resume it. Codex owns execution.

| Default model | Effort | Capabilities |
| --- | --- | --- |
| GPT-5.6 Luna | max | Coordination |
| GPT-5.6 Sol | medium | Implementation, debugging, tests & validation |
| GPT-6 Astra | high | Planning & architecture, code review |

Browser use, computer use, visual verification and spatial/3D modeling start
disabled. Move capabilities between cards or into Disabled, using drag-and-drop
or the arrow menu. Only assigned capabilities contribute instructions. Model
cards can be disabled. Re-enabling a card restores its default model, effort
and capabilities, reclaiming those capabilities from any other card. The reset
icon restores the whole board, including Jev on. Settings are session-local.

Choose another suggested model or enter a custom GPT ID. Codex validates
availability; no Switchloom update is needed for a new ID. Card IDs such as
`luna` identify slots, not fixed roles or model restrictions.

**1.0 is an unpublished local preview.** Unit/CLI/browser checks are distinct
from a complete multi-task desktop run. Desktop verification and representative
Jev-versus-vanilla measurements remain open in [TASKS.md](TASKS.md).

## Setup

Vanilla mode uses Codex task tools directly and needs no Switchloom CLI or
TypeSafe key. For Jev mode (enabled by default), install the CLI and skill:

```sh
cargo install --path . --locked
mkdir -p ~/.agents/skills/switchloom
cp skills/switchloom/SKILL.md ~/.agents/skills/switchloom/SKILL.md
```

Export `TYPESAFE_API_KEY` locally. Never paste keys into prompts; the website
does not accept them. If exported in `.zshrc`, use an interactive shell:
`zsh -ic 'exec switchloom handoff' < INPUT_FILE`. No OpenAI API key is required
by Switchloom; Codex uses its existing account.

The generated prompt authorizes creating missing tasks in the existing local
project checkout. It reuses suitable tasks, never requests worktrees, and
preserves exclusive file/tool ownership. If Luna is disabled, the planning
owner starts coordination; otherwise the first assigned card does. This does
not add another task or silently enable disabled specialist duties.

## CLI boundaries

Standalone `route` uses default catalog ownership:

```sh
switchloom route <<'JSON'
{"task":"Implement the agreed retry fix","capability":"implementation","jev":false}
JSON
```

`handoff` uses the workflow's configured ownership and returns Codex tool arguments:

```sh
switchloom handoff <<'JSON'
{
  "request": {"task":"Review the retry fix","capability":"review","jev":true},
  "caller": {"thread_id":"CALLER_ID","host_id":"local"},
  "threads": {
    "sol": {"thread_id":"SOL_ID","host_id":"local","model":"gpt-5.6-sol","effort":"medium","capabilities":["implementation","debugging","validation"]},
    "astra": {"thread_id":"ASTRA_ID","host_id":"local","model":"gpt-6-astra","effort":"high","capabilities":["planning","review"]}
  }
}
JSON
```

An explicit `capability` skips Jev. Otherwise `jev:true` requests one judgment
among assigned capabilities and clarification. `jev:false` never calls the
API; without an explicit capability it returns `explicit_capability_required`.
The output is `dispatch`, `continue_here` or `clarify`. Only the skill sends a
prepared dispatch. Errors never silently substitute a model. Models stay fixed
through their tool loop. See [routing](docs/routing.md).

## Existing installations

`switchloom status --repository .` and `switchloom uninstall --repository .`
inspect/remove earlier managed installations while preserving user edits.
See the retained persisted-data boundary in [ownership](docs/ownership.md).

## Development

`catalog.toml` owns model suggestions, reasoning choices, ordered model defaults
and capability instructions/default owners. Run `pnpm catalog:regenerate` after
changes. The Rust CLI and generated website catalog consume the same source.
`generator.ts` owns board state transitions and prompt composition; there is no
separate preview implementation or settings store.

See [CONTRIBUTING.md](CONTRIBUTING.md), [release policy](docs/package-policy.md)
and [CHANGELOG.md](CHANGELOG.md). MIT. See [LICENSE](LICENSE).
