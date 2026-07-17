import { describe, expect, it } from "vitest";

import { applyPreset, changeHost, CODEX_FRONTIER_MODELS, createConfig, CURSOR_FRONTIER_MODELS, generateFiles, HOST_IDS, hostCatalogFrom, setEffort, setModel, setRoles } from "./generator";

const hostCatalog = hostCatalogFrom({ compositions: [
  { binding: { host: "codex", profiles: {
    sol: { model: "gpt-5.6-sol", effort: "medium", cost_tier: "premium" },
    solHigh: { model: "gpt-5.6-sol", effort: "high", cost_tier: "premium" },
    solUltra: { model: "gpt-5.6-sol", effort: "ultra", cost_tier: "premium" },
    terra: { model: "gpt-5.6-terra", effort: "medium" },
    terraHigh: { model: "gpt-5.6-terra", effort: "high" },
    luna: { model: "gpt-5.6-luna", effort: "xhigh", cost_tier: "premium" },
  } } },
  { binding: { host: "cursor", profiles: {
    driver: { model: "fable-5", cost_tier: "premium" }, worker: { model: "grok-code-fast-1" },
    reviewer: { model: "gpt-5.5", cost_tier: "premium" }, verifier: { model: "gpt-5.4-mini" },
  } } },
  { binding: { host: "claude-code", profiles: {
    driver: { model: "opus", effort: "high", cost_tier: "premium" }, worker: { model: "sonnet", effort: "medium" },
  } } },
] });

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

  it("keeps Cursor focused on the researched frontier model set", () => {
    expect(hostCatalog.cursor.map((model) => model.id)).toEqual(
      CURSOR_FRONTIER_MODELS.map((model) => model.id),
    );
    expect(hostCatalog.cursor).toHaveLength(8);
  });

  it("uses the complete current Codex GPT-5.6 effort manifest", () => {
    expect(hostCatalog.codex).toEqual(CODEX_FRONTIER_MODELS);
    expect(hostCatalog.codex.find((model) => model.id === "gpt-5.6-sol")?.efforts).toEqual([
      "low", "medium", "high", "xhigh", "ultra",
    ]);
    expect(hostCatalog.codex.flatMap((model) => model.efforts)).not.toContain("max");
    expect(hostCatalog.codex.find((model) => model.id === "gpt-5.6-luna")?.efforts).not.toContain("ultra");
  });

  it("offers every current Cursor reasoning level for the full GPT-5.6 family", () => {
    const openAiModels = hostCatalog.cursor.filter((model) => model.id.startsWith("gpt-5.6-"));
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
    expect(generateFiles(configured)[".codex/agents/switchloom-orchestrator.toml"])
      .toContain('model_reasoning_effort = "ultra"');
  });

  it("writes a selected Cursor frontier model into the generated agent", () => {
    const config = setModel(createConfig("cursor"), "orchestrator", "grok-4.5", hostCatalog);
    const files = generateFiles(setRoles(config, ["orchestrator"]));
    expect(files[".cursor/agents/switchloom-orchestrator.md"]).toContain("model: grok-4.5");
  });

  it("writes selected Cursor reasoning effort into the generated agent", () => {
    const config = setEffort(createConfig("cursor"), "reviewer", "max", hostCatalog);
    const files = generateFiles(setRoles(config, ["orchestrator", "reviewer"]));
    expect(files[".cursor/agents/switchloom-reviewer.md"]).toContain("effort: max");
  });

  it("moves effort to a supported value when the model changes", () => {
    const changed = setModel(createConfig("codex"), "implementer", "gpt-5.6-luna", hostCatalog);
    expect(changed.assignments.implementer).toEqual({ model: "gpt-5.6-luna", effort: "low" });
    expect(() => setEffort(changed, "implementer", "ultra", hostCatalog)).toThrow(/unsupported effort/);
  });

  it("generates host-native Codex files and a portable manifest", () => {
    const config = setRoles(createConfig("codex"), ["orchestrator", "reviewer"]);
    const files = generateFiles(config);
    expect(Object.keys(files)).toContain(".codex/agents/switchloom-orchestrator.toml");
    expect(Object.keys(files)).toContain(".codex/agents/switchloom-reviewer.toml");
    expect(files[".codex/agents/switchloom-reviewer.toml"]).toContain('sandbox_mode = "read-only"');
    expect(JSON.parse(files["switchloom.config.json"]).roles).toEqual(["orchestrator", "reviewer"]);
  });

  it.each([
    ["cursor", ".cursor/agents/switchloom-implementer.md"],
    ["claude-code", ".claude/agents/switchloom-implementer.md"],
  ] as const)("generates %s project agent files", (host, expectedPath) => {
    const files = generateFiles(setRoles(createConfig(host), ["orchestrator", "implementer"]));
    expect(files[expectedPath]).toContain("name: switchloom-implementer");
  });
});
