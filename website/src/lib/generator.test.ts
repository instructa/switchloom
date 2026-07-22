import { describe, expect, it } from "vitest";

import packageMetadata from "../../../package.json";

import { applyPreset, canRenderParentRecommendation, changeHost, CHILD_ROLE_IDS, choosePreset, createConfig, createPresetSelection, HOST_IDS, hostCatalogFrom, isChildRoleId, isPresetDirty, isPrimaryRecommendationId, lifecycleCommands, markPresetCustom, parentRecommendationEffortCopy, PRESETS, primaryRecommendation, PRIMARY_RECOMMENDATION_ID, recipeApplyCommand, removeChildRole, resetRolesToPreset, ROLE_IDS, selectedChildRoleIds, setEffort, setIntegration, setModel, setRoles, setupConfigToml, setupRecipe, setupSpec, setupSummary, setupTransportFrom, SWITCHLOOM_VERSION } from "./generator";

const generatedCatalog = {
  setupContract: {
    recipePrefix: "sw1_",
    configPath: ".switchloom/config.toml",
    hosts: [
      {
        id: "codex",
        binding: "codex-openai",
        models: [
          { id: "gpt-5.6-sol", efforts: ["low", "medium", "high", "xhigh", "max", "ultra"], tier: "premium" as const },
          { id: "gpt-5.6-terra", efforts: ["low", "medium", "high", "xhigh", "max", "ultra"], tier: "standard" as const },
          { id: "gpt-5.6-luna", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "standard" as const },
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
      {
        id: "opencode",
        binding: "opencode-native",
        models: [
          { id: "opencode/gpt-5-nano", efforts: ["low", "medium", "high", "max"], tier: "standard" as const },
          { id: "anthropic/claude-sonnet-4-5", efforts: ["low", "medium", "high"], tier: "standard" as const },
          { id: "anthropic/claude-opus-4-5", efforts: ["high", "max"], tier: "premium" as const },
        ],
      },
      {
        id: "pi",
        binding: "pi-external",
        models: [
          { id: "openai/gpt-4o-mini", efforts: ["low", "medium", "high", "xhigh"], tier: "standard" as const },
          { id: "google/gemini-2.5-flash", efforts: ["low", "medium", "high", "xhigh"], tier: "standard" as const },
          { id: "anthropic/claude-sonnet-4-5", efforts: ["low", "medium", "high", "xhigh"], tier: "premium" as const },
        ],
      },
    ],
  },
};

const hostCatalog = hostCatalogFrom(generatedCatalog);

describe("Switchloom generator", () => {
  it("derives the pinned CLI version from package metadata", () => {
    expect(SWITCHLOOM_VERSION).toBe(packageMetadata.version);
  });

  it("splits the primary recommendation id from generated child role ids", () => {
    expect(PRIMARY_RECOMMENDATION_ID).toBe("orchestrator");
    expect(CHILD_ROLE_IDS).toEqual(["implementer", "reviewer", "verifier"]);
    expect(ROLE_IDS).toEqual([PRIMARY_RECOMMENDATION_ID, ...CHILD_ROLE_IDS]);
    expect(isPrimaryRecommendationId("orchestrator")).toBe(true);
    expect(isChildRoleId("orchestrator")).toBe(false);
    expect(isChildRoleId("reviewer")).toBe(true);
  });

  it("keeps generated child roles separate from the parent recommendation", () => {
    const config = setRoles(createConfig(), ["implementer", "reviewer", "verifier", "not-a-role"]);
    expect(config.roles).toEqual(["implementer", "reviewer", "verifier"]);
  });

  it("tracks the parent recommendation separately from selected children", () => {
    const config = setRoles(createConfig(), ["reviewer", "not-a-role"]);
    expect(primaryRecommendation(config)).toEqual({
      id: "orchestrator",
      assignment: { model: "gpt-5.6-sol", effort: "medium" },
    });
    expect(selectedChildRoleIds(config)).toEqual(["reviewer"]);
    expect(selectedChildRoleIds(config)).not.toContain("orchestrator");
  });

  it("remembers the last selected preset through custom edits", () => {
    const balanced = createPresetSelection();
    expect(balanced).toEqual({ selected: "balanced", lastSelected: "balanced" });

    const high = choosePreset(balanced, "high");
    expect(high).toEqual({ selected: "high", lastSelected: "high" });
    expect(markPresetCustom(high)).toEqual({ selected: "custom", lastSelected: "high" });
  });

  it("detects dirty team state against a preset baseline", () => {
    const balanced = applyPreset(createConfig("cursor"), "balanced", hostCatalog);
    expect(isPresetDirty(balanced, "balanced", hostCatalog)).toBe(false);
    expect(isPresetDirty(setEffort(balanced, "reviewer", "max", hostCatalog), "balanced", hostCatalog)).toBe(true);
    expect(isPresetDirty(setRoles(balanced, ["implementer", "reviewer"]), "balanced", hostCatalog)).toBe(true);
  });

  it("removes selected children without dropping the final child", () => {
    const config = createConfig();
    const withoutReviewer = removeChildRole(config, "reviewer");
    expect(withoutReviewer.roles).toEqual(["implementer", "verifier"]);
    expect(removeChildRole(setRoles(config, ["verifier"]), "verifier").roles).toEqual(["verifier"]);
  });

  it("resets roles and assignments to the last selected preset", () => {
    const custom = setRoles(
      setEffort(applyPreset(createConfig("codex"), "high", hostCatalog), "reviewer", "ultra", hostCatalog),
      ["reviewer"],
    );

    const reset = resetRolesToPreset(custom, "high", hostCatalog);
    expect(reset.roles).toEqual(["implementer", "reviewer", "verifier"]);
    expect(reset.usagePolicy).toBe("max-quality");
    expect(reset.assignments.reviewer).toEqual({ model: "gpt-5.6-sol", effort: "medium" });
    expect(isPresetDirty(reset, "high", hostCatalog)).toBe(false);
  });

  it("resets incompatible assignments when the host changes", () => {
    const changed = changeHost(applyPreset(createConfig("codex"), "high", hostCatalog), "claude-code");
    expect(changed.assignments.orchestrator).toEqual({ model: "opus", effort: "high" });
    expect(changed.roles).toHaveLength(3);
    expect(changed.usagePolicy).toBe("balanced");
    expect(isPresetDirty(changed, "balanced", hostCatalog)).toBe(false);
  });

  it("keeps the selected child roster while applying Balanced host assignments", () => {
    const high = setRoles(applyPreset(createConfig("codex"), "high", hostCatalog), ["reviewer", "verifier"]);
    const changed = changeHost(high, "cursor");
    expect(changed.roles).toEqual(["reviewer", "verifier"]);
    expect(changed.usagePolicy).toBe("balanced");
    expect(changed.assignments.reviewer).toEqual({ model: "gpt-5.6-sol", effort: "high" });
    expect(isPresetDirty(changed, "balanced", hostCatalog)).toBe(true);
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
    expect(hostCatalog.opencode.binding).toBe("opencode-native");
    expect(hostCatalog.opencode.models.map((model) => model.id)).toEqual([
      "opencode/gpt-5-nano",
      "anthropic/claude-sonnet-4-5",
      "anthropic/claude-opus-4-5",
    ]);
    expect(hostCatalog.pi.binding).toBe("pi-external");
    expect(hostCatalog.pi.models.map((model) => model.id)).toEqual([
      "openai/gpt-4o-mini",
      "google/gemini-2.5-flash",
      "anthropic/claude-sonnet-4-5",
    ]);
    expect(() => hostCatalogFrom({ setupContract: { hosts: [] } })).toThrow(/canonical setup contract/);
  });

  it("uses the complete current Codex GPT-5.6 effort manifest", () => {
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-sol")?.label).toBe("Sol");
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-terra")?.label).toBe("Terra");
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-luna")?.label).toBe("Luna");
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-luna")?.disabledReason)
      .toBe("not supported yet in v2");
    expect(hostCatalog.cursor.models.find((model) => model.id === "gpt-5.6-luna")?.label).toBe("Luna");
    expect(hostCatalog.cursor.models.find((model) => model.id === "gpt-5.6-luna")?.disabledReason).toBeUndefined();
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-sol")?.efforts).toEqual([
      "low", "medium", "high", "xhigh", "max", "ultra",
    ]);
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-terra")?.efforts).toEqual([
      "low", "medium", "high", "xhigh", "max", "ultra",
    ]);
    expect(hostCatalog.codex.models.find((model) => model.id === "gpt-5.6-luna")?.efforts).toEqual([
      "low", "medium", "high", "xhigh", "max",
    ]);
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

  it("keeps Codex Light genuinely low and Max or Ultra out of Codex presets", () => {
    const codexLight = applyPreset(createConfig("codex"), "light", hostCatalog);
    expect(Object.values(codexLight.assignments).map((assignment) => assignment.effort)).toEqual([
      "low", "low", "low", "low",
    ]);
    expect(Object.values(codexLight.assignments).map((assignment) => assignment.model)).toEqual([
      "gpt-5.6-terra", "gpt-5.6-terra", "gpt-5.6-terra", "gpt-5.6-terra",
    ]);
    for (const host of HOST_IDS) {
      for (const preset of ["light", "balanced", "high"] as const) {
        expect(Object.values(applyPreset(createConfig(host), preset, hostCatalog).assignments))
          .not.toContainEqual(expect.objectContaining({ effort: "ultra" }));
      }
    }
    for (const preset of ["light", "balanced", "high"] as const) {
      expect(Object.values(applyPreset(createConfig("codex"), preset, hostCatalog).assignments))
        .not.toContainEqual(expect.objectContaining({ effort: "max" }));
    }
  });

  it("keeps Luna out of every certified Codex preset", () => {
    for (const preset of ["light", "balanced", "high"] as const) {
      expect(Object.values(applyPreset(createConfig("codex"), preset, hostCatalog).assignments))
        .not.toContainEqual(expect.objectContaining({ model: "gpt-5.6-luna" }));
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
    expect(PRESETS.high.short).toBe("Prioritizes the strongest model and a larger execution budget.");
  });

  it("keeps explicit Codex Orchestrator choices parent-only", () => {
    const configured = setEffort(createConfig("codex"), "orchestrator", "ultra", hostCatalog);
    expect(configured.assignments.orchestrator.effort).toBe("ultra");
    expect(setupSpec(configured, hostCatalog).selected_roles.orchestrator).toBeUndefined();
  });

  it("derives parent-card effort copy from each Codex preset without recommending a parent model", () => {
    expect(parentRecommendationEffortCopy(applyPreset(createConfig("codex"), "light", hostCatalog), "light"))
      .toBe("Set Codex reasoning to Low.");
    expect(parentRecommendationEffortCopy(applyPreset(createConfig("codex"), "balanced", hostCatalog), "balanced"))
      .toBe("Set Codex reasoning to Medium.");
    expect(parentRecommendationEffortCopy(applyPreset(createConfig("codex"), "high", hostCatalog), "high"))
      .toBe("Set Codex reasoning to Medium.");
  });

  it("summarizes native parents separately from generated child roles", () => {
    const nativeSummary = setupSummary(setRoles(createConfig("codex"), ["reviewer"]));
    expect(nativeSummary).toContain("1 generated child role");
    expect(nativeSummary).toContain("Host-managed parent: Orchestrator");
    expect(nativeSummary).toContain("Reviewer:");
    expect(nativeSummary).not.toContain("\nOrchestrator:");

    const externalSummary = setupSummary(setRoles(createConfig("pi"), ["reviewer"]));
    expect(externalSummary).toContain("2 focused roles");
    expect(externalSummary).not.toContain("Host-managed parent");
    expect(externalSummary).toContain("\nOrchestrator: openai/gpt-4o-mini · medium");
  });

  it("summarizes native parent ownership without recommending a parent model", () => {
    const summary = setupSummary(setRoles(createConfig("cursor"), ["implementer"]));
    expect(summary).toContain("Host-managed parent: Orchestrator (high)");
    expect(summary).toContain("Implementer: composer-2.5");
    expect(summary).not.toContain("fable-5");
  });

  it("gates the parent-card ownership treatment away from Pi external-runner configs", () => {
    expect(canRenderParentRecommendation(createConfig("codex"))).toBe(true);
    expect(canRenderParentRecommendation(createConfig("pi"))).toBe(false);
    expect(parentRecommendationEffortCopy(applyPreset(createConfig("pi"), "balanced", hostCatalog), "balanced"))
      .toBe("Set Pi thinking to Medium.");
  });

  it("does not write a selected Cursor parent model into generated agents", () => {
    const config = setModel(createConfig("cursor"), "orchestrator", "grok-4.5", hostCatalog);
    expect(setupSpec(setRoles(config, ["reviewer"]), hostCatalog).selected_roles).not.toHaveProperty("orchestrator");
  });

  it("writes selected Cursor reasoning effort into the generated agent", () => {
    const config = setEffort(createConfig("cursor"), "reviewer", "max", hostCatalog);
    expect(setupSpec(setRoles(config, ["reviewer"]), hostCatalog).selected_roles.reviewer.effort).toBe("max");
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

  it("serializes only child SetupSpecV1 roles with Codex spawn identities", () => {
    const config = setRoles(createConfig("codex"), ["reviewer"]);
    const spec = setupSpec(config, hostCatalog);
    expect(spec).toEqual({
      schema_version: 1,
      host: "codex-openai",
      integration: "standalone",
      usage_policy: "balanced",
      selected_roles: {
        reviewer: {
          model: "gpt-5.6-sol",
          effort: "high",
          spawn: { agent_type: "switchloom_reviewer", task_name: "reviewer", fork_turns: { mode: "none" } },
        },
      },
      routes: [
        { work_type: "code", role: "reviewer", fallbacks: [] },
        { work_type: "review", role: "reviewer", fallbacks: [] },
        { work_type: "verification", role: "reviewer", fallbacks: [] },
      ],
    });
  });

  it("preserves Pi external-runner orchestrator routing in SetupSpecV1", () => {
    const spec = setupSpec(setRoles(createConfig("pi"), ["reviewer"]), hostCatalog);
    expect(spec).toEqual({
      schema_version: 1,
      host: "pi-external",
      integration: "standalone",
      usage_policy: "balanced",
      selected_roles: {
        orchestrator: {
          model: "openai/gpt-4o-mini",
          effort: "medium",
        },
        reviewer: {
          model: "anthropic/claude-sonnet-4-5",
          effort: "high",
        },
      },
      routes: [
        { work_type: "planning", role: "orchestrator", fallbacks: [] },
        { work_type: "code", role: "reviewer", fallbacks: [] },
        { work_type: "review", role: "reviewer", fallbacks: [] },
        { work_type: "verification", role: "reviewer", fallbacks: [] },
      ],
      route_default: { role: "orchestrator", fallbacks: [] },
    });
  });

  it("writes Pi external-runner orchestrator routing to setup TOML", () => {
    const toml = setupConfigToml(setRoles(createConfig("pi"), ["reviewer"]), hostCatalog);
    expect(toml).toContain("[route_default]");
    expect(toml).toContain('role = "orchestrator"');
    expect(toml).toContain("[selected_roles.orchestrator]");
    expect(toml).toContain('model = "openai/gpt-4o-mini"');
    expect(toml).toContain("[selected_roles.reviewer]");
  });

  it("keeps deterministic review fallback on verifier when reviewer is removed", () => {
    const spec = setupSpec(setRoles(createConfig("codex"), ["implementer", "verifier"]), hostCatalog);
    expect(spec.selected_roles).toHaveProperty("implementer");
    expect(spec.selected_roles).toHaveProperty("verifier");
    expect(spec.selected_roles).not.toHaveProperty("reviewer");
    expect(spec.routes).toEqual([
      { work_type: "code", role: "implementer", fallbacks: [] },
      { work_type: "review", role: "verifier", fallbacks: [] },
      { work_type: "verification", role: "verifier", fallbacks: [] },
    ]);
  });

  it("keeps deterministic verification fallback on reviewer when verifier is removed", () => {
    const spec = setupSpec(setRoles(createConfig("codex"), ["implementer", "reviewer"]), hostCatalog);
    expect(spec.selected_roles).toHaveProperty("implementer");
    expect(spec.selected_roles).toHaveProperty("reviewer");
    expect(spec.selected_roles).not.toHaveProperty("verifier");
    expect(spec.routes).toEqual([
      { work_type: "code", role: "implementer", fallbacks: [] },
      { work_type: "review", role: "reviewer", fallbacks: [] },
      { work_type: "verification", role: "reviewer", fallbacks: [] },
    ]);
  });

  it("serializes Planr as an explicit setup mode before role tuning", () => {
    const spec = setupSpec(setIntegration(createConfig("claude-code"), "planr"), hostCatalog);
    expect(spec.integration).toBe("planr");
    expect(spec.host).toBe("claude-native");
    expect(spec.selected_roles.orchestrator).toBeUndefined();
  });

  it("creates a shell-safe npx recipe command and readable setup TOML", () => {
    const transport = setupTransportFrom(generatedCatalog);
    const config = setRoles(createConfig("codex"), ["implementer"]);
    const recipe = setupRecipe(config, hostCatalog, transport.recipePrefix);
    expect(recipe).toMatch(/^sw1_[A-Za-z0-9_-]+$/);
    const command = recipeApplyCommand(config, hostCatalog, transport.recipePrefix);
    expect(command).toBe(`npx switchloom@${SWITCHLOOM_VERSION} apply --recipe '${recipe}' --repository .`);
    const toml = setupConfigToml(config, hostCatalog);
    expect(toml).toContain('host = "codex-openai"');
    expect(toml).toContain('integration = "standalone"');
    expect(toml).toContain("[selected_roles.implementer.spawn]");
    expect(toml).not.toContain("[route_default]");
    expect(toml).not.toContain("[selected_roles.orchestrator]");
    expect(toml).not.toContain("switchloom_orchestrator");
    expect(toml).not.toContain(".codex/agents");
    expect(toml).not.toContain("switchloom.config.json");
  });

  it("shows the full CLI lifecycle without claiming custom setup verification", () => {
    const commands = lifecycleCommands(createConfig("cursor"), hostCatalog);
    expect(commands.map((entry) => entry.command)).toEqual([
      `npm install -g switchloom@${SWITCHLOOM_VERSION}`,
      expect.stringMatching(/^switchloom preview --recipe 'sw1_/),
      expect.stringMatching(/^switchloom apply --recipe 'sw1_/),
      "switchloom doctor cursor",
      "switchloom update --repository .",
      "switchloom status --repository .",
      "switchloom rollback --repository .",
      "switchloom uninstall --repository .",
    ]);
    expect(commands.every((entry) => entry.title && entry.description)).toBe(true);
    expect(commands.map((entry) => entry.command).join("\n")).not.toMatch(/recommend/i);
  });
});
