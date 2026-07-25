import assert from "node:assert/strict";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const release = path.resolve("scripts/release.sh");
const run = (args, env = {}) => spawnSync("sh", [release, ...args], { encoding: "utf8", env: { ...process.env, ...env } });

test("release requires an explicit external gate, store, and selected run before repository checks", () => { const result = run(["0.3.3", "summary"]); assert.notEqual(result.status, 0); assert.match(result.stderr, /--external-gate <tool> --external-store <store> --external-run <selected-run>/); });
test("release passes the explicit store with candidate identity before branch and remote checks", async t => { const root = await mkdtemp(path.join(os.tmpdir(), "release-gate-")), gate = path.join(root, "gate.sh"), receipt = path.join(root, "receipt"); t.after(() => rm(root, { recursive: true, force: true })); await writeFile(gate, "#!/usr/bin/env sh\nprintf '%s\\n' \"$*\" > \"$RELEASE_GATE_RECEIPT\"\nexit 17\n"); await chmod(gate, 0o755); const result = run(["--external-gate", gate, "--external-store", "/private/evals", "--external-run", "releases/0.3.3/run", "0.3.3", "summary"], { RELEASE_GATE_RECEIPT: receipt }); assert.equal(result.status, 17); assert.match(await readFile(receipt, "utf8"), /--root .* --store \/private\/evals --run releases\/0\.3\.3\/run --subject [a-f0-9]{40} --version 0\.3\.3/); });
