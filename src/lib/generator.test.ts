import { describe, expect, it } from "vitest";

import { applyPreset, changeHost, createConfig, HOST_IDS, hostCatalogFrom, lifecycleCommands, recipeApplyCommand, setEffort, setIntegration, setModel, setRoles, setupConfigToml, setupRecipe, setupSpec, setupTransportFrom } from "./generator";

const generatedCatalog = {
  setupContract: {
    recipePrefix: "sw1_",
    configPath: ".switchloom/config.toml",
    hosts: [
      {
        id: "codex",
        binding: "codex-openai",
        models: [
          { id: "gpt-5.6-sol", efforts: ["low", "medium", "high", "xhigh", "ultra"], tier: "premium" as const },
          { id: "gpt-5.6-terra", efforts: ["low", "medium", "high", "xhigh", "ultra"], tier: "standard" as const },
          { id: "gpt-5.6-luna", efforts: ["low", "medium", "high", "xhigh"], tier: "standard" as const },
        ],
      },
      {
        id: "cursor",
        binding: "cursor-openai",
        models: [
          { id: "gpt-5.6-sol", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "premium" as const },
          { id: "gpt-5.6-terra", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "standard" as const },
          { id: "gpt-5.6-luna", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "standard" as const },
          { id: "fable-5", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "premium" as const },
          { id: "claude-opus-4-8", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "premium" as const },
          { id: "claude-sonnet-5", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "standard" as const },
          { id: "grok-4.5", efforts: ["low", "medium", "high"], tier: "premium" as const },
          { id: "composer-2.5", efforts: [], tier: "standard" as const },
        ],
      },
      {
        id: "claude-code",
        binding: "claude-native",
        models: [
          { id: "opus", efforts: ["medium", "high"], tier: "premium" as const },
          { id: "sonnet", efforts: ["medium", "high"], tier: "standard" as const },
        ],
      },
    ],
  },
};

const hostCatalog = hostCatalogFrom(generatedCatalog);

describe("Switchloom generator", () => {
  it("always keeps the orchestrator and caps the team at four roles", () => {
    const config = setRoles(createConfig(), ["implementer", "reviewer", "verifier", "not-a-role"]);
    expect(config.roles).toEqual(["orchestrator", "implementer", "reviewer", "verifier"]);
  });

  it("resets incompatible assignments when the host changes", () => {
    const changed = changeHost(createConfig("codex"), "claude-code");
    expect(changed.assignments.orchestrator).toEqual({ model: "opus", effort: "high" });
    expect(changed.roles).toHaveLength(4);
  });

  it("derives host models from the generated setup contract", () => {
    expect(hostCatalog.cursor.models.map((model) => model.id)).toEqual([
      "gpt-5.6-sol",
      "gpt-5.6-terra",
      "gpt-5.6-luna",
      "fable-5",
      "claude-opus-4-8",
      "claude-sonnet-5",
      "grok-4.5",
      "composer-2.5",
    ]);
    expect(hostCatalog.cursor.binding).toBe("cursor-openai");
    expect(() => hostCatalogFrom({ setupContract: { hosts: [] } })).toThrow(/canonical setup contract/);
  });

  it("uses the complete current Codex GPT-5.6 effort manifest", () => {
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-sol")?.efforts).toEqual([
      "low", "medium", "high", "xhigh", "ultra",
    ]);
    expect(hostCatalog.codex.models.flatMap((model) => model.efforts)).not.toContain("max");
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-luna")?.efforts).not.toContain("ultra");
  });

  it("offers every current Cursor reasoning level for the full GPT-5.6 family", () => {
    const openAiModels = hostCatalog.cursor.models.filter((model) => model.id.startsWith("gpt-5.6-"));
    expect(openAiModels.map((model) => model.id)).toEqual([
      "gpt-5.6-sol",
      "gpt-5.6-terra",
      "gpt-5.6-luna",
    ]);
    for (const model of openAiModels) {
      expect(model.efforts).toEqual(["low", "medium", "high", "xhigh", "max"]);
    }
  });

  it("applies cost, balanced, and quality presets across every role", () => {
    const base = createConfig("cursor");
    expect(applyPreset(base, "light", hostCatalog).assignments.orchestrator).toEqual({
      model: "gpt-5.6-luna",
      effort: "low",
    });
    expect(applyPreset(base, "balanced", hostCatalog).assignments.reviewer).toEqual({
      model: "gpt-5.6-sol",
      effort: "high",
    });
    expect(applyPreset(base, "high", hostCatalog).assignments.reviewer).toEqual({
      model: "gpt-5.6-sol",
      effort: "max",
    });
  });

  it("keeps Light genuinely low and Ultra out of every preset", () => {
    const codexLight = applyPreset(createConfig("codex"), "light", hostCatalog);
    expect(Object.values(codexLight.assignments).map((assignment) => assignment.effort)).toEqual([
      "low", "low", "low", "low",
    ]);
    for (const host of HOST_IDS) {
      for (const preset of ["light", "balanced", "high"] as const) {
        expect(Object.values(applyPreset(createConfig(host), preset, hostCatalog).assignments))
          .not.toContainEqual(expect.objectContaining({ effort: "ultra" }));
      }
    }
  });

  it("uses Sol at medium effort for every Codex High role", () => {
    const codexHigh = applyPreset(createConfig("codex"), "high", hostCatalog);
    expect(Object.values(codexHigh.assignments)).toEqual([
      { model: "gpt-5.6-sol", effort: "medium" },
      { model: "gpt-5.6-sol", effort: "medium" },
      { model: "gpt-5.6-sol", effort: "medium" },
      { model: "gpt-5.6-sol", effort: "medium" },
    ]);
  });

  it("allows Ultra only as an explicit Codex role choice", () => {
    const configured = setEffort(createConfig("codex"), "orchestrator", "ultra", hostCatalog);
    expect(configured.assignments.orchestrator.effort).toBe("ultra");
    expect(setupSpec(configured, hostCatalog).selected_roles.orchestrator.effort).toBe("ultra");
  });

  it("writes a selected Cursor frontier model into the generated agent", () => {
    const config = setModel(createConfig("cursor"), "orchestrator", "grok-4.5", hostCatalog);
    expect(setupSpec(setRoles(config, ["orchestrator"]), hostCatalog).selected_roles.orchestrator.model).toBe("grok-4.5");
  });

  it("writes selected Cursor reasoning effort into the generated agent", () => {
    const config = setEffort(createConfig("cursor"), "reviewer", "max", hostCatalog);
    expect(setupSpec(setRoles(config, ["orchestrator", "reviewer"]), hostCatalog).selected_roles.reviewer.effort).toBe("max");
  });

  it("moves effort to a supported value when the model changes", () => {
    const changed = setModel(createConfig("codex"), "implementer", "gpt-5.6-luna", hostCatalog);
    expect(changed.assignments.implementer).toEqual({ model: "gpt-5.6-luna", effort: "low" });
    expect(() => setEffort(changed, "implementer", "ultra", hostCatalog)).toThrow(/unsupported effort/);
  });

  it("maps UI presets onto canonical Rust usage policy ids", () => {
    expect(applyPreset(createConfig("codex"), "light", hostCatalog).usagePolicy).toBe("low-usage");
    expect(applyPreset(createConfig("codex"), "balanced", hostCatalog).usagePolicy).toBe("balanced");
    expect(applyPreset(createConfig("codex"), "high", hostCatalog).usagePolicy).toBe("max-quality");
  });

  it("serializes only SetupSpecV1 with Codex spawn identities", () => {
    const config = setRoles(createConfig("codex"), ["orchestrator", "reviewer"]);
    const spec = setupSpec(config, hostCatalog);
    expect(spec).toEqual({
      schema_version: 1,
      host: "codex-openai",
      integration: "standalone",
      usage_policy: "balanced",
      selected_roles: {
        orchestrator: {
          model: "gpt-5.6-sol",
          effort: "medium",
          spawn: { agent_type: "switchloom_orchestrator", task_name: "orchestrator", fork_turns: { mode: "none" } },
        },
        reviewer: {
          model: "gpt-5.6-sol",
          effort: "high",
          spawn: { agent_type: "switchloom_reviewer", task_name: "reviewer", fork_turns: { mode: "none" } },
        },
      },
      routes: [
        { work_type: "planning", role: "orchestrator", fallbacks: [] },
        { work_type: "code", role: "orchestrator", fallbacks: [] },
        { work_type: "review", role: "reviewer", fallbacks: [] },
        { work_type: "verification", role: "reviewer", fallbacks: [] },
      ],
      route_default: { role: "orchestrator", fallbacks: [] },
    });
  });

  it("serializes Planr as an explicit setup mode before role tuning", () => {
    const spec = setupSpec(setIntegration(createConfig("claude-code"), "planr"), hostCatalog);
    expect(spec.integration).toBe("planr");
    expect(spec.host).toBe("claude-native");
    expect(spec.selected_roles.orchestrator).not.toHaveProperty("spawn");
  });

  it("creates a shell-safe npx recipe command and readable setup TOML", () => {
    const transport = setupTransportFrom(generatedCatalog);
    const config = setRoles(createConfig("codex"), ["orchestrator", "implementer"]);
    const recipe = setupRecipe(config, hostCatalog, transport.recipePrefix);
    expect(recipe).toMatch(/^sw1_[A-Za-z0-9_-]+$/);
    const command = recipeApplyCommand(config, hostCatalog, transport.recipePrefix);
    expect(command).toMatch(/^npx switchloom@latest apply --recipe 'sw1_[A-Za-z0-9_-]+' --repository \.$/);
    const toml = setupConfigToml(config, hostCatalog);
    expect(toml).toContain('host = "codex-openai"');
    expect(toml).toContain('integration = "standalone"');
    expect(toml).toContain("[selected_roles.implementer.spawn]");
    expect(toml).not.toContain(".codex/agents");
    expect(toml).not.toContain("switchloom.config.json");
  });

  it("shows the full CLI lifecycle without claiming custom setup verification", () => {
    const commands = lifecycleCommands(createConfig("cursor"), hostCatalog);
    expect(commands).toEqual([
      "npm install -g switchloom",
      expect.stringMatching(/^switchloom preview --recipe 'sw1_/),
      expect.stringMatching(/^switchloom apply --recipe 'sw1_/),
      "switchloom update --repository .",
      "switchloom status --repository .",
      "switchloom rollback --repository .",
      "switchloom uninstall --repository .",
    ]);
    expect(commands.join("\n")).not.toMatch(/recommend/i);
  });
});
