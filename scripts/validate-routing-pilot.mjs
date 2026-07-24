#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
const pilot = JSON.parse(await readFile(new URL("../fixtures/routing-pilot-v1/pilot.json", import.meta.url), "utf8"));
assert.equal(pilot.currency_policy, "unavailable_without_billable_evidence");
assert.equal(pilot.tasks.length, 3);
assert.deepEqual(pilot.arms.map(({ id }) => id), ["single-sol-medium", "all-sol-multi-agent", "routed-sol-terra"]);
for (const arm of pilot.arms) assert.deepEqual(arm.parent, { model: "gpt-5.6-sol", effort: "medium" });
assert.deepEqual(pilot.arms[2].children.map(({ model, effort }) => [model, effort]), [["gpt-5.6-terra", "medium"], ["gpt-5.6-terra", "medium"], ["gpt-5.6-terra", "low"]]);
for (const task of pilot.tasks) {
  assert.ok(task.oracle.path);
  if (task.oracle.kind === "produced_file") assert.ok(task.seed_files[task.oracle.path] === undefined);
  else {
    assert.ok(task.seed_files[task.oracle.path] !== undefined);
    assert.notEqual(task.seed_files[task.oracle.path], task.oracle.equals, `${task.id} seed already satisfies its oracle`);
  }
}
console.log("routing pilot contract has 3 isolated deterministic tasks and 3 matched arms");
