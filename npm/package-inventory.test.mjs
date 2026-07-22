import assert from "node:assert/strict";
import { copyFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const root = path.resolve(".");
const supportedTargets = [
  "darwin-arm64",
  "darwin-x86_64",
  "linux-arm64",
  "linux-x86_64",
];

test("npm pack admits only the explicit supported native targets", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "switchloom-npm-inventory-"));
  t.after(() => rm(directory, { force: true, recursive: true }));

  for (const file of ["package.json", "README.md", "LICENSE"]) {
    await copyFile(path.join(root, file), path.join(directory, file));
  }
  await mkdir(path.join(directory, "npm/bin"), { recursive: true });
  await copyFile(
    path.join(root, "npm/bin/model-routing.js"),
    path.join(directory, "npm/bin/model-routing.js"),
  );

  for (const target of [...supportedTargets, "unsupported"]) {
    const targetDirectory = path.join(directory, "npm/native", target);
    await mkdir(targetDirectory, { recursive: true });
    await writeFile(path.join(targetDirectory, "model-routing"), `${target}\n`);
  }
  await writeFile(
    path.join(directory, "npm/native/provenance.json"),
    '{"schema_version":"test"}\n',
  );
  await mkdir(path.join(directory, "docs"));
  await writeFile(path.join(directory, "docs/should-not-ship.md"), "private docs\n");
  await mkdir(path.join(directory, "reports"));
  await writeFile(path.join(directory, "reports/should-not-ship.json"), "{}\n");

  const packed = spawnSync("npm", ["pack", "--dry-run", "--json"], {
    cwd: directory,
    encoding: "utf8",
  });
  assert.equal(packed.status, 0, packed.stderr);
  const files = JSON.parse(packed.stdout)[0].files.map(({ path: file }) => file).sort();
  const expected = [
    "LICENSE",
    "README.md",
    "npm/bin/model-routing.js",
    ...supportedTargets.map((target) => `npm/native/${target}/model-routing`),
    "npm/native/provenance.json",
    "package.json",
  ].sort();

  assert.deepEqual(files, expected);
  assert.ok(!files.includes("npm/native/unsupported/model-routing"));
  assert.ok(files.every((file) => !file.startsWith("docs/") && !file.startsWith("reports/")));
});
