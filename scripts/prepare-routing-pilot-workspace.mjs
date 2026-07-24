#!/usr/bin/env node
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { readFile } from "node:fs/promises";
const [taskId, armId] = process.argv.slice(2);
if (!taskId || !armId) throw new Error("usage: prepare-routing-pilot-workspace.mjs <task-id> <arm-id>");
const pilot = JSON.parse(await readFile(new URL("../fixtures/routing-pilot-v1/pilot.json", import.meta.url), "utf8"));
const task = pilot.tasks.find(({ id }) => id === taskId), arm = pilot.arms.find(({ id }) => id === armId);
if (!task || !arm) throw new Error("unknown pilot task or arm");
const workspace = await mkdtemp(path.join(os.tmpdir(), `routing-pilot-${taskId}-${armId}-`));
for (const [file, content] of Object.entries(task.seed_files)) { const target = path.join(workspace, file); await mkdir(path.dirname(target), { recursive: true }); await writeFile(target, content); }
await writeFile(path.join(workspace, "pilot-contract.json"), `${JSON.stringify({ task, arm, oracle: task.oracle }, null, 2)}\n`);
console.log(workspace);
