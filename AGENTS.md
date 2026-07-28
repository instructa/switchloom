# Switchloom contributor guidance

Keep changes narrowly scoped and preserve unrelated work already present in the tree.

## Verification

Run the smallest check that covers the files changed. For CI routing or workflow
changes, run `node --test scripts/ci-router.test.mjs scripts/ci-workflow-contract.test.mjs`.
For npm wrapper changes, run `node --test npm/*.test.mjs`; for Rust and website
changes, choose the owning crate/test or website test first.

Use `pnpm site:test` for ordinary website changes. Run the slower
`pnpm site:test:parity` only when the website generator, catalog, onboarding
transport, or the corresponding Rust routing/lifecycle contract changes.

Do not routinely replay the full workspace, site build, native release matrix,
browser checks, or security scans. Escalate to a broader check only when a shared
boundary changed, focused evidence is insufficient, or a release/security task
explicitly requires it. Use the existing `pnpm security:check` only for such
security-scoped work; do not weaken its scripts or hooks.
