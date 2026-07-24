#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = new URL("../fixtures/codex-doctor-v0.3.2/", import.meta.url);
const readJson = async (name) => JSON.parse(await readFile(new URL(name, root), "utf8"));
const expected = await readJson("expected-v0.3.2-doctor.json");
const spawn = await readJson("authenticated-spawn-oracle.json");
const provenance = await readJson("same-package-repository-provenance.json");

assert.equal(expected.v0_3_2_false_warnings.length, 3);
assert.deepEqual(
  expected.v0_3_2_false_warnings.map(({ legacy_role }) => legacy_role),
  ["model_routing_terra_high", "model_routing_terra_mechanical", "model_routing_sol_high"],
);
assert.deepEqual(
  expected.expected_v2_flag_diagnostics_for_intentionally_broken_state,
  ["codex_v2_activation_conflict", "codex_v2_metadata_conflict"],
);
assert.equal(spawn.children.length, 3);
assert.ok(spawn.proof_requirements.every((requirement) => requirement.length > 0));
assert.equal(provenance.repository_local_npx.reported_version, "0.3.1");
assert.equal(provenance.registry_tarball.reported_version, "0.3.2");
assert.equal(provenance.neutral_directory_npx.reported_version, "0.3.2");
console.log("Codex Doctor v0.3.2 regression fixtures are structurally consistent.");
