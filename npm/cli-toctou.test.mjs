import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const binary = path.resolve("target/debug/model-routing");

function setupToml(integration) {
  return [
    "schema_version = 1",
    'host = "codex"',
    `integration = "${integration}"`,
    'usage_policy = "balanced"',
    "",
    "[[routes]]",
    'work_type = "code"',
    'role = "worker"',
    "",
    "[selected_roles.worker]",
    'model = "gpt-5.6-terra"',
    'effort = "high"',
    "",
    "[selected_roles.worker.spawn]",
    'agent_type = "switchloom_worker"',
    'task_name = "worker"',
    "",
    "[selected_roles.worker.spawn.fork_turns]",
    'mode = "none"',
    "",
  ].join("\n");
}

test("interactive setup apply uses the previewed config snapshot after confirmation", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "switchloom-toctou-"));
  const repository = path.join(directory, "repo");
  const config = path.join(directory, "setup.toml");
  const mutated = path.join(directory, "mutated.toml");
  await mkdir(repository);
  await writeFile(config, setupToml("standalone"));
  await writeFile(mutated, setupToml("planr"));

  const harness = String.raw`
import json, os, pty, select, subprocess, sys, time
binary, config, mutated, repository = sys.argv[1:5]
master, slave = pty.openpty()
proc = subprocess.Popen([binary, "apply", "--config", config, "--repository", repository], stdin=slave, stdout=slave, stderr=slave, close_fds=True)
os.close(slave)
output = b""
prompt = b"Type yes to continue:"
deadline = time.time() + 10
mutated_once = False
while time.time() < deadline:
    ready, _, _ = select.select([master], [], [], 0.05)
    if ready:
        chunk = os.read(master, 4096)
        if not chunk:
            break
        output += chunk
        if prompt in output and not mutated_once:
            with open(mutated, "rb") as src, open(config, "wb") as dst:
                dst.write(src.read())
            os.write(master, b"yes\n")
            mutated_once = True
    if proc.poll() is not None:
        break
code = proc.wait(timeout=5)
while True:
    ready, _, _ = select.select([master], [], [], 0)
    if not ready:
        break
    try:
        chunk = os.read(master, 4096)
    except OSError:
        break
    if not chunk:
        break
    output += chunk
os.close(master)
print(json.dumps({"code": code, "output": output.decode("utf-8", "replace"), "mutated": mutated_once}))
`;
  const result = spawnSync("python3", ["-c", harness, binary, config, mutated, repository], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  const observed = JSON.parse(result.stdout);
  assert.equal(observed.code, 0, observed.output);
  assert.equal(observed.mutated, true);
  assert.match(observed.output, /"path": ".switchloom\/config.toml"/);
  assert.doesNotMatch(observed.output, /\.planr\/agents.toml/);
  assert.match(observed.output, /"action": "apply"/);
  const persisted = await readFile(path.join(repository, ".switchloom/config.toml"), "utf8");
  assert.match(persisted, /integration = "standalone"/);
  assert.doesNotMatch(persisted, /integration = "planr"/);
  assert.match(persisted, /agent_type = "switchloom_worker"/);
  await assert.rejects(readFile(path.join(repository, ".planr/agents.toml"), "utf8"));
});

test("interactive setup apply aborts when repository symlink retargets after preview", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "switchloom-repo-toctou-"));
  const repoA = path.join(directory, "repo-a");
  const repoB = path.join(directory, "repo-b");
  const repoLink = path.join(directory, "repo-link");
  const config = path.join(directory, "setup.toml");
  await mkdir(repoA);
  await mkdir(repoB);
  await symlink(repoA, repoLink);
  await writeFile(config, setupToml("standalone"));

  const harness = String.raw`
import json, os, pty, select, subprocess, sys, time
binary, config, repo_link, repo_b = sys.argv[1:5]
master, slave = pty.openpty()
proc = subprocess.Popen([binary, "apply", "--config", config, "--repository", repo_link], stdin=slave, stdout=slave, stderr=slave, close_fds=True)
os.close(slave)
output = b""
prompt = b"Type yes to continue:"
mutated_once = False
deadline = time.time() + 10
while time.time() < deadline:
    ready, _, _ = select.select([master], [], [], 0.05)
    if ready:
        chunk = os.read(master, 4096)
        if not chunk:
            break
        output += chunk
        if prompt in output and not mutated_once:
            os.unlink(repo_link)
            os.symlink(repo_b, repo_link)
            os.write(master, b"yes\n")
            mutated_once = True
    if proc.poll() is not None:
        break
code = proc.wait(timeout=5)
while True:
    ready, _, _ = select.select([master], [], [], 0)
    if not ready:
        break
    try:
        chunk = os.read(master, 4096)
    except OSError:
        break
    if not chunk:
        break
    output += chunk
os.close(master)
print(json.dumps({"code": code, "output": output.decode("utf-8", "replace"), "mutated": mutated_once}))
`;
  const result = spawnSync("python3", ["-c", harness, binary, config, repoLink, repoB], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  const observed = JSON.parse(result.stdout);
  assert.equal(observed.mutated, true);
  assert.notEqual(observed.code, 0, observed.output);
  assert.match(observed.output, /repository state changed after preview/);
  await assert.rejects(readFile(path.join(repoA, ".switchloom/config.toml"), "utf8"));
  await assert.rejects(readFile(path.join(repoB, ".switchloom/config.toml"), "utf8"));
  await rm(repoLink);
});
