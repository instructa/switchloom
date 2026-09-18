# Playground and local benchmarks

One TanStack Start application serves the workflow board and `/playground`.
The public Cloudflare Worker evaluates routing only. A separate, loopback-only
Node entrypoint adds the Codex benchmark harness. Local execution code is not
included in the public client or Worker bundle.

## Start locally

Use Node 24+, pnpm, Rust and the Codex CLI. Install the web-core tools once:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.127 --locked
pnpm install
pnpm exec playwright install chromium
pnpm site:local
```

Open `http://127.0.0.1:4173/playground`. `site:local` sets `DEVELOPER_MODE=true`
and binds to `127.0.0.1`. The ordinary `site:dev` command runs the public Worker
locally; its secret can be supplied in the ignored `.dev.vars` file. Never use
a `VITE_` prefix for credentials. `site:build` rejects developer mode; setting the flag on the deployed
Worker never enables local endpoints.

`TYPESAFE_API_KEY` and `OPENAI_API_KEY` are read by the local server from the environment. An export in
`.zshrc` is available when starting from an interactive zsh. The local
Connections panel can instead hold a TypeSafe key in server memory for that
process. With `OPENAI_API_KEY` set, the harness logs in automatically using its
ephemeral Codex credential store. Keys are not saved in browser storage or included in prompts.

Do not edit the local runtime or rebuild WASM during a benchmark: Vite restarts
the server and the active run is interrupted. Saved artifacts remain readable.

The harness uses `codex app-server` over stdio and recognizes the existing Codex
login. Connections also supports Codex's own ChatGPT login and OpenAI API-key
login. Explicit logins restart the local app-server with an ephemeral credential
store; new credentials stay in that process and do not replace the saved login.
Keys entered through Connections must be re-entered after restarting the local
server; environment keys reconnect automatically.
The hosted Playground offers neither login nor OpenAI execution.

## Public Playground

Prompts and context are bounded to 4,000 characters each, with a 24 KB HTTP body
limit. Routing uses the board's active capabilities and model settings. With
Jev off, choose the capability explicitly; no provider call or API key is
needed. With Jev on, TypeSafe selects a capability and the Rust router resolves
its configured owner. Models remain changeable on the board.

The graph displays the actual decision. Bars are returned probabilities, not
an invented explanation. Confidence, the applicable dispatch threshold,
missing-context score, latency and provider token usage are shown separately.
Suggestions and abstentions are visible and do not dispatch work. Changing the
prompt or board clears the current result; recent decisions remain in the
in-memory session history.

Public limits are 10 Jev calls per session per UTC day, 100 per IP per UTC day,
6 per IP per minute and 10,000 total per UTC day. A daily Durable Object reserves
all counters atomically before a provider request. Failed provider calls consume
the reservation. A quota failure blocks the request. Explicit assignments are
free. The HttpOnly session cookie is not an account or a unique-person check;
IP and global limits bound cookie resets. IPs are hashed using a server secret.

The TypeSafe key stays in a Cloudflare secret binding. No prompt or result
database is created by the public application.

For production, authenticate Alchemy with Cloudflare and export
`TYPESAFE_API_KEY`. Set `ALCHEMY_PASSWORD` in the ignored
`.env.production.local` file (permissions `0600`) to encrypt secrets in local
Alchemy state. Keep that password and `.alchemy/` for subsequent deployments;
neither belongs in Git. Alchemy does not write secrets into its generated
Wrangler configuration.

```sh
pnpm site:check
pnpm exec alchemy deploy --stage prod --env-file .env.production.local
```

This updates the existing production Worker and `switchloom.ai`. It does not
publish the npm package or enable the local benchmark harness.

## Run a benchmark

1. Configure the board, then open Playground. In Benchmark choose the mode.
2. Select an output folder. Each run creates a fresh project and sibling
   artifacts directory. No existing project is overwritten.
3. Paste the task or import a text/Markdown file; optionally supply an absolute
   PNG reference path (up to 8 MB). Choose duration and turn limits.
4. Run. The graph and event stream show actual task and turn activity. Approve
   requested commands or file changes individually, or stop the run.
5. Inspect the final result, usage, command outcomes and diff. Start a Vite/Next
   preview, or enter a manually started local preview URL. Capture a screenshot
   at the fixed 1440 × 1000 viewport.
6. Repeat the same task/configuration for the other modes. Use Open saved run to
   compare results after restarting the local server.

| Mode | Execution |
| --- | --- |
| Vanilla | One selected model; no capability routing. |
| Coordinator + subagents | Selected coordinator delegates to Codex subagents using the configured model/effort pairs; no Jev. |
| Routing | Persistent configured tasks; the coordinator supplies capabilities. |
| Routing + Jev | Same ownership; Jev selects every new objective, the host resumes bound work. |

With Jev, the host routes the original user objective directly before any
coordinator turn. Results resume the coordination owner; without one, the planning owner or first
assigned slot coordinates. That is a turn in an existing task, not another
manager or subagent. Coordinator turns are read-only, return one next-step response without tools,
and cannot select capabilities in Jev mode. Vanilla and persistent-team tasks
disable Codex subagents through task-local overrides. The subagent benchmark
mode enables them explicitly and permits delegation across configured models;
this is the comparison baseline, distinct from the Desktop prompt's same-model
subagent rule. Persistent task handoffs belong to the host.
Ordinary implementation needs no advisor plan. A blocked worker retains its
assignment while the coordinator obtains advice, then resumes that same worker
with the answer and no new judgment. The next worker receives the original source
result with owner/capability/objective as well as the coordinator's note, preserving
the advisor's wording. Technical planning belongs to the planning owner. Each slot keeps its model, effort and task context within
a run. Worker results resume the coordinator through app-server notifications;
there is no polling or per-tool-call routing. Interactive MCP input receives a
cancel response and an `interaction/cancelled` event; the agent can continue and
report the unavailable action. Other unsupported Codex interactions or uncertain
routing stop with a visible result rather than selecting a fallback.

The harness starts tasks with workspace-write. Codex may persist its normal
project trust entry. Worker turns use workspace-write with network access for dependency
installation and API-backed apps, with user-reviewed approvals. Coordinator turns
remain read-only without network. Model and agent settings are passed as runtime
overrides rather than written to personal TOML. Local tools, skills and account configuration are inherited from the
active Codex profile and recorded where
reported by app-server; this is not a container or an independent credential
vault. Generated app previews run only after an explicit local action, on a
separate port. The screenshot browser cannot navigate to the control origin.

Each slot starts a new task with its own conversation history; the harness does
not fork or read another task's full history into its prompt. Each worker gets
the original user task on first use, its assignment and the supplied handoff
context. The coordinator receives worker results; subsequent worker assignments
include the verbatim source result alongside the coordinator's note.
Codex memory use and generation are disabled for the harness process and tasks.
All workers share the project filesystem. This is
controlled message passing, not an isolated memory or filesystem boundary.
The Desktop workflow can reuse existing tasks, which retain their earlier context.

Artifacts are `config.json`, `inputs.json` (prompt/image hashes and runtime policy),
`prompt.md`, `summary.json`, `events.jsonl`, `checks.json`,
`diff.patch` and, after capture, `screenshot.png` and `capture.json`. They can
contain project code and task text; keep run directories private. Configured
keys and common OpenAI key patterns are redacted from live events and preview
logs. Command failures are not automatically test failures: commands such as
`rg` can return nonzero when nothing matches. Imported results are read-only.

## Comparison and current evidence

Use the same task, models/efforts, budgets, tools, skills and acceptance checks.
Runs start from empty project directories with fresh task contexts. Existing
source-snapshot import and automated quality scoring are not implemented. Repeat
trials in varied order; do not reuse another variant's output or conversation.
Zero recorded Jev calls means a run does not measure Jev's effect. Subscription
usage, API tokens and quality scores are separate measurements.

The reference is copied to `reference.png` in each fresh project and attached to
the first turn of each main task. Subagents are instructed to inspect that local
file. Check actual image use before comparing visual results. Root and descendant
usage events are tracked separately; missing reports are marked incomplete, not
treated as zero. Descendant models are recorded when Codex exposes them; an
unverified model is not proof that the selected model set was respected.
Use the same billing mode and standard service tier in all arms. Time and host
turn limits are not dollar limits. The pilot uses these existing limits and records
usage; a separate dollar-budget system is outside its scope.

Verified locally on 17 September: a Luna → Astra → Luna → Sol → Luna run used
three persistent tasks over five turns and completed the requested file check
with zero Jev calls. An actual Jev run routed the original implementation to
Sol, which created a function and passed its test, then returned to Luna:
two turns, one Jev call. Eight live Playground routing probes also matched their
expected capabilities, including CLI/MCP lookup and ordinary implementation.
These runs used a local process with Codex subagents disabled.
Screenshot capture of a Codex-generated HTML page
produced a 1440 × 1000 image; saved runs were reopened in the comparison table.
The Cloudflare runtime test used a mocked provider:
15 concurrent requests allowed exactly 10, blocked 5, and local endpoints returned
404. These are integration checks, not a Pokedex quality benchmark.

The full Pokedex comparison, confidence calibration, public deployment and Codex
Desktop cross-task resume test remain open in [TASKS.md](../TASKS.md).

## Ownership

`catalog.toml` owns definitions. `src/decision.rs` owns routing policy;
`src/web.rs` exposes it as WASM for both web entrypoints. `website/server` owns
HTTP, limits and public composition. `website/local` owns app-server process,
local access checks, events and files. Codex owns each execution/tool loop.
Neither React nor the local adapter implements another classifier policy.

References: [Codex App Server](https://learn.chatgpt.com/docs/app-server),
[TanStack Start hosting](https://tanstack.com/start/latest/docs/framework/react/guide/hosting),
[TypeSafe Choice](https://docs.typesafe.ai/primitives/choice).
