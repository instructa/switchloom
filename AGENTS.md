# Switchloom

`catalog.toml` owns model suggestions, slot defaults, assignment and completion
instructions, and classifier criteria. `src/catalog.rs` reads that file and publishes
`website/data/catalog.json`. Do not add a second model table or a `roles.toml`.

`src/decision.rs` owns the next-capability judgment. `src/typesafe.rs` owns the
Jev request/response contract and native HTTP call, pinning `jev-1.13.0`. `src/handoff.rs` binds an existing Codex task.
The website board and `skills/switchloom/SKILL.md` must use the same fields:
slot id, capability and `routing` (`jev` or `assigned`). Coordination is a
board responsibility, not a work candidate. New automatic objectives must not
carry a capability chosen by the coordinator.

A suggestion is not a dispatch. An API failure is not a model choice. Do not
add a second routing policy, a generic proxy or a tool catalog.

`src/web.rs` exposes the same core as WASM. `website/server` owns web HTTP and
public quota; `website/local` owns the local app-server adapter and run artifacts.
Public builds must exclude local execution, credential and filesystem endpoints.
Keep developer mode loopback-only and retain origin/session/CSRF checks.

Use `pnpm site:test` for focused web changes and `pnpm site:check` for shared
web/runtime boundaries. Keep the existing security checks and hooks unchanged.
