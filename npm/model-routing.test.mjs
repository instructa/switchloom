import assert from "node:assert/strict";
import { chmod, copyFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const wrapper = new URL("./bin/model-routing.js", import.meta.url);

function platformTarget() {
  const osName = { darwin: "darwin", linux: "linux" }[os.platform()];
  const arch = { arm64: "arm64", x64: "x86_64" }[os.arch()];
  return osName && arch ? `${osName}-${arch}` : null;
}

async function writePackage(directory, nativeVersion) {
  const target = platformTarget();
  assert.ok(target, "test host must use a supported native target");
  await mkdir(path.join(directory, "npm/bin"), { recursive: true });
  await mkdir(path.join(directory, "npm/native", target), { recursive: true });
  await copyFile(wrapper, path.join(directory, "npm/bin/model-routing.js"));
  await writeFile(
    path.join(directory, "package.json"),
    JSON.stringify({ name: "switchloom", version: "0.3.3", type: "module" }),
  );
  const binary = path.join(directory, "npm/native", target, "model-routing");
  await writeFile(
    binary,
    `#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'model-routing ${nativeVersion}\\n'; else printf 'executed %s\\n' \"$*\"; fi\n`,
  );
  await chmod(binary, 0o755);
  return path.join(directory, "npm/bin/model-routing.js");
}

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

test("same-package repository launcher rejects a stale native binary while a registry package passes", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "switchloom-provenance-"));
  t.after(() => rm(directory, { force: true, recursive: true }));
  const staleRepositoryLauncher = await writePackage(path.join(directory, "switchloom"), "0.3.1");
  const registryLauncher = await writePackage(path.join(directory, "registry-switchloom"), "0.3.3");

  const stale = spawnSync(process.execPath, [staleRepositoryLauncher, "--version"], {
    encoding: "utf8",
  });
  assert.equal(stale.status, 1);
  assert.match(stale.stderr, /native provenance mismatch/);
  assert.match(stale.stderr, /switchloom@0\.3\.3/);
  assert.match(stale.stderr, /model-routing 0\.3\.1/);

  const registry = spawnSync(process.execPath, [registryLauncher, "--version"], {
    encoding: "utf8",
  });
  assert.equal(registry.status, 0, registry.stderr);
  assert.equal(registry.stdout, "model-routing 0.3.3\n");
});
