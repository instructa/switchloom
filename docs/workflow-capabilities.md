# Workflow capability boundaries

Switchloom separates the execution path from its validation status. A catalog
entry is actionable only when it is **Certified**; Experimental and Planned
entries describe a boundary and must not be treated as generated support.

| Coding agent | Path | Status | Boundary |
| --- | --- | --- | --- |
| Codex | Native | Certified | The active Codex session is the parent; project roles use the certified native contract. |
| Pi | Pi Subagents | Experimental | The active Pi session remains the orchestrator; generated configuration is not a certified support claim until live receipts exist. |
| Claude Code → Codex | Sidecar | Planned | No generated sidecar configuration is provided. |
| Cursor | Direct BYOK | Experimental | Requested routing remains unproven without a live requested/effective-model receipt; effective-model claims remain advisory. |
| Cursor | Gateway workaround | Experimental | OpenAI-base-URL and OpenRouter workarounds are not generated support. |

Cursor's documented BYOK providers are Direct access. Its OpenAI-base-URL and
OpenRouter workarounds are non-actionable. Claude Code gateway configuration is
limited to Claude-compatible providers; non-Claude routing is unsupported.
`codex-plugin-cc` remains a separate Claude-to-Codex sidecar with no generated
configuration.

Provider/model choices are filtered by the capability catalog. Pi fallback models
are emitted as provider-qualified values and are used only for provider/model
startup failures, not task failures.

Pi Subagents and OpenCode use provider-qualified role models directly. Their
runtimes own provider login and availability: use Pi `/login` from the active
main session and OpenCode `/connect`. Switchloom writes neither credentials nor
provider endpoints. OpenRouter is a native provider choice, including
runtime-selected `openrouter/auto` where the host supports it.
