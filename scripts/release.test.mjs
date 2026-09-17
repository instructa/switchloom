import assert from "node:assert/strict";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

async function sandbox(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "switchloom-release-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, "scripts"));
  await mkdir(path.join(root, "bin"));
  await copyFile("scripts/release.sh", path.join(root, "scripts/release.sh"));
  await writeFile(path.join(root, "bin/git"), `#!/bin/sh
printf 'git %s\\n' "$*" >> "$RELEASE_TEST_TRACE"
case "$*" in
  'rev-parse --abbrev-ref HEAD') echo "$RELEASE_TEST_BRANCH" ;;
  fetch*) ;;
  'rev-parse HEAD'|'rev-parse origin/main') echo candidate ;;
  'rev-parse v1.2.3') exit 1 ;;
  *) exit 99 ;;
esac
`, { mode: 0o755 });
  await writeFile(path.join(root, "bin/cargo"), `#!/bin/sh
printf 'cargo %s\\n' "$*" >> "$RELEASE_TEST_TRACE"
exit "$RELEASE_TEST_CARGO_EXIT"
`, { mode: 0o755 });
  const trace = path.join(root, "trace");
  return {
    trace: () => readFile(trace, "utf8"),
    run: (extra = {}) => spawnSync("sh", [path.join(root, "scripts/release.sh"), "1.2.3", "summary"], {
      encoding: "utf8",
      env: { ...process.env, PATH: `${path.join(root, "bin")}:${process.env.PATH}`, RELEASE_DRY_RUN: "1", SWITCHLOOM_RC_RUN_ID: "", RELEASE_TEST_TRACE: trace, RELEASE_TEST_BRANCH: "main", RELEASE_TEST_CARGO_EXIT: "0", ...extra },
    }),
  };
}

test("release rejects non-main before remote operations", async (t) => {
  const fixture = await sandbox(t);
  const result = fixture.run({ RELEASE_TEST_BRANCH: "feature" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /release must run on main/);
  assert.doesNotMatch(await fixture.trace(), /fetch|cargo/);
});

test("release dry run retains preparation, verification and packaging without publishing", async (t) => {
  const fixture = await sandbox(t);
  const result = fixture.run();
  assert.equal(result.status, 0, result.stderr);
  const trace = await fixture.trace();
  assert.match(trace, /release prepare --version 1.2.3/);
  assert.match(trace, /release verify/);
  assert.match(trace, /release package/);
  assert.doesNotMatch(trace, /git (add|commit|tag|push)/);
});

test("release stops on failed preparation", async (t) => {
  const fixture = await sandbox(t);
  const result = fixture.run({ RELEASE_TEST_CARGO_EXIT: "17" });
  assert.equal(result.status, 17);
  assert.doesNotMatch(await fixture.trace(), /release verify|release package|git push/);
});
