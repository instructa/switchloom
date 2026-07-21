import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import {
  applyPreset,
  createConfig,
  HOST_IDS,
  hostCatalogFrom,
  lifecycleCommands,
  PRESET_IDS,
  ROLE_IDS,
  recipeApplyCommand,
  setEffort,
  setIntegration,
  setModel,
  setupConfigToml,
  setupRecipe,
  setupTransportFrom,
  shellQuote,
} from "./generator";
import type { GeneratorConfig, RoleId } from "./generator";

type LifecycleReport = {
  repository: string;
  action: string;
  bundle_id: string;
  artifacts: Array<{ path: string; status: string; sha256: string }>;
};
type BadTransportCase =
  | { name: string; args: string[] }
  | { name: string; toml: string };

const roots: string[] = [];
const switchloomBin = resolve("target/debug/switchloom");
const catalog = JSON.parse(await readFile(resolve("website/data/catalog.json"), "utf8"));
const hostCatalog = hostCatalogFrom(catalog);
const setupTransport = setupTransportFrom(catalog);

beforeAll(() => {
  const result = spawnSync("cargo", ["build", "--bins"], { encoding: "utf8" });
  expect(result.status, result.stderr || result.stdout).toBe(0);
});

afterAll(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function tempRepo(prefix: string) {
  const root = await mkdtemp(join(tmpdir(), prefix));
  roots.push(root);
  const repository = join(root, "repo");
  await mkdir(repository);
  return { root, repository };
}

function run(args: string[], cwd = resolve(".")) {
  const result = spawnSync(switchloomBin, args, {
    cwd,
    encoding: "utf8",
    env: process.env,
  });
  if (result.status !== 0) {
    throw new Error(`${switchloomBin} ${args.join(" ")} failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  }
  return result.stdout;
}

function runRejected(args: string[]) {
  return spawnSync(switchloomBin, args, {
    encoding: "utf8",
    env: process.env,
  });
}

function report(output: string): LifecycleReport {
  return JSON.parse(output) as LifecycleReport;
}

function normalized(report: LifecycleReport) {
  return {
    action: report.action,
    bundle_id: report.bundle_id,
    artifacts: report.artifacts.map(({ path, status, sha256 }) => ({ path, status, sha256 })),
  };
}

async function validateCliParity(name: string, config: GeneratorConfig) {
  const configToml = setupConfigToml(config, hostCatalog);
  const recipe = setupRecipe(config, hostCatalog, setupTransport.recipePrefix);
  const { root, repository } = await tempRepo(`switchloom-website-contract-${name}-`);
  const configPath = join(root, "config.toml");
  await writeFile(configPath, configToml);

  const fromConfigOutput = run(["preview", "--config", configPath, "--repository", repository]);
  const fromRecipeOutput = run(["preview", "--recipe", recipe, "--repository", repository]);
  const repeatedConfigOutput = run(["preview", "--config", configPath, "--repository", repository]);
  const repeatedRecipeOutput = run(["preview", "--recipe", recipe, "--repository", repository]);
  const fromConfig = report(fromConfigOutput);
  const fromRecipe = report(fromRecipeOutput);
  const repeatedConfig = report(repeatedConfigOutput);
  const repeatedRecipe = report(repeatedRecipeOutput);

  expect(normalized(fromRecipe), name).toEqual(normalized(fromConfig));
  expect(normalized(repeatedConfig), name).toEqual(normalized(fromConfig));
  expect(normalized(repeatedRecipe), name).toEqual(normalized(fromConfig));
  expect(fromRecipeOutput, name).toBe(fromConfigOutput);
  expect(repeatedConfigOutput, name).toBe(fromConfigOutput);
  expect(repeatedRecipeOutput, name).toBe(fromConfigOutput);
  expect(
    fromConfig.artifacts.every((artifact) => /^[a-f0-9]{64}$/.test(artifact.sha256)),
    name,
  ).toBe(true);
  expect(fromConfig.artifacts.map((artifact) => artifact.path), name).toContain(".switchloom/config.toml");
  if (config.integration === "standalone") {
    expect(fromConfig.artifacts.map((artifact) => artifact.path), name).not.toContain(".planr/agents.toml");
  } else {
    expect(fromConfig.artifacts.map((artifact) => artifact.path), name).toContain(".planr/agents.toml");
    expect(fromConfig.artifacts.map((artifact) => artifact.path), name).toContain(".planr/policy.toml");
  }
}

function roleSubsets(): RoleId[][] {
  const optionalRoles = ROLE_IDS.filter((role) => role !== "orchestrator");
  return Array.from({ length: 1 << optionalRoles.length }, (_, mask) => [
    "orchestrator",
    ...optionalRoles.filter((_, index) => (mask & (1 << index)) !== 0),
  ] as RoleId[]);
}

function withRoles(config: GeneratorConfig, roles: readonly RoleId[]): GeneratorConfig {
  return { ...config, roles: [...roles] };
}

function caseToken(value: string) {
  return value.replaceAll(/[^a-zA-Z0-9]+/g, "-").replaceAll(/^-|-$/g, "");
}

describe("website SetupSpec to CLI parity", () => {
  it("validates every website host/preset/integration/role-subset config and recipe through the Rust lifecycle", async () => {
    for (const host of HOST_IDS) {
      for (const preset of PRESET_IDS) {
        for (const roles of roleSubsets()) {
          for (const integration of ["standalone", "planr"] as const) {
            const config = withRoles(setIntegration(applyPreset(createConfig(host), preset, hostCatalog), integration), roles);
            await validateCliParity(
              `${host}-${preset}-${integration}-${roles.join("-")}`,
              config,
            );
          }
        }
      }
    }
  }, 120_000);

  it("validates every website-selectable per-role model and effort through Rust config and recipe compilation", async () => {
    for (const host of HOST_IDS) {
      for (const role of ROLE_IDS) {
        const roleSet = role === "orchestrator" ? ["orchestrator"] as RoleId[] : ["orchestrator", role] as RoleId[];
        for (const model of hostCatalog[host].models) {
          const efforts = model.efforts.length > 0 ? model.efforts : [undefined];
          for (const effort of efforts) {
            let config = withRoles(createConfig(host), roleSet);
            config = setModel(config, role, model.id, hostCatalog);
            if (effort) config = setEffort(config, role, effort, hostCatalog);
            for (const integration of ["standalone", "planr"] as const) {
              await validateCliParity(
                `${host}-${role}-${caseToken(model.id)}-${effort ?? "no-effort"}-${integration}`,
                setIntegration(config, integration),
              );
            }
          }
        }
      }
    }
  }, 180_000);

  it("keeps preset coverage over all roles as a compact smoke for copied website defaults", async () => {
    for (const host of HOST_IDS) {
      for (const preset of PRESET_IDS) {
        for (const integration of ["standalone", "planr"] as const) {
          const config = setIntegration(applyPreset(createConfig(host), preset, hostCatalog), integration);
          await validateCliParity(`${host}-${preset}-${integration}-all-roles`, config);
        }
      }
    }
  }, 60_000);

  it("executes the copied standalone website command through preview/apply/update/status/rollback/uninstall", async () => {
    const config = applyPreset(createConfig("codex"), "balanced", hostCatalog);
    const command = recipeApplyCommand(config, hostCatalog, setupTransport.recipePrefix);
    const recipe = command.match(/--recipe '([^']+)'/)?.[1];
    expect(recipe).toBeTruthy();
    expect(command).toBe(`npx switchloom@0.3.0 apply --recipe ${shellQuote(recipe!)} --repository .`);

    const { repository } = await tempRepo("switchloom-website-standalone-");
    const preview = report(run(["preview", "--recipe", recipe!, "--repository", repository]));
    expect(preview.artifacts.map((artifact) => artifact.path)).toContain(".switchloom/config.toml");
    expect(preview.artifacts.map((artifact) => artifact.path)).not.toContain(".planr/agents.toml");

    const applied = report(run(["apply", "--recipe", recipe!, "--repository", repository, "--yes"]));
    expect(applied.action).toBe("apply");
    await expect(readFile(join(repository, ".planr/agents.toml"), "utf8")).rejects.toThrow();
    expect(await readFile(join(repository, ".switchloom/config.toml"), "utf8")).toContain('integration = "standalone"');

    expect(report(run(["update", "--repository", repository])).action).toBe("update");
    expect(JSON.parse(run(["status", "--repository", repository])).artifacts.length).toBeGreaterThan(0);
    expect(report(run(["rollback", "--repository", repository])).action).toBe("rollback");
    expect(report(run(["uninstall", "--repository", repository])).action).toBe("uninstall");
    await expect(readFile(join(repository, ".model-routing/manifest.json"), "utf8")).rejects.toThrow();
  });

  it("executes a Planr-mode website command and writes Planr declarations plus thin native roles", async () => {
    const config = setIntegration(applyPreset(createConfig("codex"), "balanced", hostCatalog), "planr");
    const commands = lifecycleCommands(config, hostCatalog, setupTransport.recipePrefix);
    const recipe = commands[2].match(/--recipe '([^']+)'/)?.[1];
    expect(recipe).toBeTruthy();

    const { repository } = await tempRepo("switchloom-website-planr-");
    const applied = report(run(["apply", "--recipe", recipe!, "--repository", repository, "--yes"]));
    const paths = applied.artifacts.map((artifact) => artifact.path);
    expect(paths).toContain(".planr/agents.toml");
    expect(paths).toContain(".planr/policy.toml");
    expect(paths).toContain(".codex/agents/switchloom_implementer.toml");
    expect(paths).toContain(".codex/agents/switchloom_reviewer.toml");

    const agents = await readFile(join(repository, ".planr/agents.toml"), "utf8");
    const policy = await readFile(join(repository, ".planr/policy.toml"), "utf8");
    const worker = await readFile(join(repository, ".codex/agents/switchloom_implementer.toml"), "utf8");
    const reviewer = await readFile(join(repository, ".codex/agents/switchloom_reviewer.toml"), "utf8");
    expect(agents).toContain("switchloom_implementer");
    expect(agents).toContain('work_type = "code"');
    expect(policy).toContain('id = "balanced"');
    expect(worker).toContain("Protocol preload: $planr-work");
    expect(reviewer).toContain("Protocol preload: $planr-review");
    expect(worker).not.toContain("model-routing");
    expect(reviewer).not.toContain("model-routing");
  });

  it("rejects malformed website transports before repository mutation", async () => {
    const badCases: BadTransportCase[] = [
      { name: "malformed recipe", args: ["preview", "--recipe", "sw1_not-base64!", "--repository"] },
      { name: "oversized recipe", args: ["preview", "--recipe", `sw1_${"A".repeat(90_000)}`, "--repository"] },
      {
        name: "unknown version config",
        toml: 'schema_version = 999\nhost = "codex"\nintegration = "standalone"\nusage_policy = "balanced"\n',
      },
      {
        name: "unsupported model config",
        toml: [
          "schema_version = 1",
          'host = "codex"',
          'integration = "standalone"',
          'usage_policy = "balanced"',
          "",
          "[[routes]]",
          'work_type = "code"',
          'role = "worker"',
          "",
          "[selected_roles.worker]",
          'model = "not-a-model"',
          'effort = "high"',
          "",
        ].join("\n"),
      },
      {
        name: "route to omitted role config",
        toml: [
          "schema_version = 1",
          'host = "codex-openai"',
          'integration = "planr"',
          'usage_policy = "balanced"',
          "",
          "[[routes]]",
          'work_type = "code"',
          'role = "implementer"',
          "fallbacks = []",
          "",
          "[route_default]",
          'role = "orchestrator"',
          "fallbacks = []",
          "",
          "[selected_roles.orchestrator]",
          'model = "gpt-5.6-sol"',
          'effort = "medium"',
          "",
          "[selected_roles.orchestrator.spawn]",
          'agent_type = "switchloom_orchestrator"',
          'task_name = "orchestrator"',
          "",
          "[selected_roles.orchestrator.spawn.fork_turns]",
          'mode = "none"',
          "",
        ].join("\n"),
      },
      {
        name: "duplicate codex agent type config",
        toml: [
          "schema_version = 1",
          'host = "codex-openai"',
          'integration = "planr"',
          'usage_policy = "balanced"',
          "",
          "[[routes]]",
          'work_type = "code"',
          'role = "orchestrator"',
          "fallbacks = []",
          "",
          "[route_default]",
          'role = "orchestrator"',
          "fallbacks = []",
          "",
          "[selected_roles.orchestrator]",
          'model = "gpt-5.6-sol"',
          'effort = "medium"',
          "",
          "[selected_roles.orchestrator.spawn]",
          'agent_type = "switchloom_shared"',
          'task_name = "orchestrator"',
          "",
          "[selected_roles.orchestrator.spawn.fork_turns]",
          'mode = "none"',
          "",
          "[selected_roles.implementer]",
          'model = "gpt-5.6-terra"',
          'effort = "high"',
          "",
          "[selected_roles.implementer.spawn]",
          'agent_type = "switchloom_shared"',
          'task_name = "implementer"',
          "",
          "[selected_roles.implementer.spawn.fork_turns]",
          'mode = "none"',
          "",
        ].join("\n"),
      },
      {
        name: "integration confused config",
        toml: 'schema_version = 1\nhost = "codex"\nintegration = "maybe"\nusage_policy = "balanced"\n',
      },
    ];

    for (const badCase of badCases) {
      const { root, repository } = await tempRepo(`switchloom-website-invalid-${badCase.name.replaceAll(" ", "-")}-`);
      let args: string[];
      if ("args" in badCase) {
        args = [...badCase.args, repository];
      } else {
        const badConfig = join(root, "bad.toml");
        await writeFile(badConfig, badCase.toml);
        args = ["preview", "--config", badConfig, "--repository", repository];
      }
      const result = runRejected(args);
      expect(result.status, badCase.name).not.toBe(0);
      await expect(readFile(join(repository, ".switchloom/config.toml"), "utf8")).rejects.toThrow();
      await expect(readFile(join(repository, ".model-routing/manifest.json"), "utf8")).rejects.toThrow();
    }
  });
});
