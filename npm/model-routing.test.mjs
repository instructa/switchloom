import assert from "node:assert/strict";
import { chmod, mkdtemp, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const wrapper = new URL("./bin/model-routing.js", import.meta.url);

test("npm wrapper forwards arguments to the selected native binary", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "switchloom-wrapper-"));
  const binary = path.join(directory, "model-routing");
  await writeFile(binary, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n", "utf8");
  await chmod(binary, 0o755);

  const result = spawnSync(process.execPath, [wrapper.pathname, "--version"], {
    encoding: "utf8",
    env: { ...process.env, SWITCHLOOM_NATIVE_BIN: binary },
  });

  assert.equal(result.status, 0);
  assert.equal(result.stdout, "--version\n");
  assert.equal(result.stderr, "");
});
