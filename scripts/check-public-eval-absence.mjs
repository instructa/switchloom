#!/usr/bin/env node
import assert from "node:assert/strict";
import { existsSync } from "node:fs";

const absent = [
  "reports", "retained-evidence", "schemas/release-eval",
  "fixtures/release-eval-contract-v1", "fixtures/release-routing-suite-v1", "fixtures/routing-pilot-v1",
  "docs/local-release-eval.md", "docs/release-eval-contract.md", "docs/release-eval-extraction-inventory.md", "docs/release-eval-operations.md", "docs/release-eval-ownership.md", "docs/release-eval-private-operations.md", "docs/release-routing-suite.md", "docs/routing-efficiency-pilot.md", "docs/routing-quality-comparison.md",
  "scripts/check-evidence-validator-parity.mjs", "scripts/check-migration-manifest.sh", "scripts/check-release-eval-contract.mjs", "scripts/check-release-eval-supply-chain.mjs", "scripts/check-release-eval-supply-chain.test.mjs", "scripts/complete-release-eval-run.mjs",
  "scripts/compare-routing-pilot.mjs", "scripts/compare-routing-pilot.test.mjs", "scripts/extract-routing-pilot-telemetry.mjs", "scripts/extract-routing-pilot-telemetry.test.mjs", "scripts/finalize-release-eval-quality.mjs", "scripts/generate-release-eval-attestation.mjs", "scripts/generate-release-eval-attestation.test.mjs", "scripts/init-release-eval-store.mjs", "scripts/preflight-release-eval.mjs", "scripts/preflight-release-eval.test.mjs", "scripts/prepare-release-routing-suite.mjs", "scripts/prepare-release-routing-suite.test.mjs", "scripts/prepare-routing-pilot-workspace.mjs", "scripts/release-eval-egress-proxy.mjs", "scripts/release-eval-sandbox.mjs", "scripts/release-eval-sandbox.test.mjs", "scripts/release-eval-store.mjs", "scripts/release-eval-store.test.mjs", "scripts/run-local-release-eval.mjs", "scripts/run-local-release-eval.test.mjs", "scripts/run-routing-pilot-codex.mjs", "scripts/run-routing-pilot-codex.test.mjs", "scripts/validate-release-routing-suite.mjs", "scripts/validate-routing-pilot.mjs", "scripts/verify-release-eval-lifecycle.mjs", "scripts/verify-release-eval-store.mjs",
];
for (const path of absent) assert.equal(existsSync(path), false, `public maintainer-eval path remains: ${path}`);
for (const path of ["evidence/codex/0.145.0/runtime-evidence.json", "evidence/codex/0.145.0/exact-version-capture.txt"]) assert.equal(existsSync(path), true, `public runtime fixture is missing: ${path}`);
console.log(`public eval absence passed: ${absent.length} external-owner paths absent; 2 runtime fixtures retained`);
