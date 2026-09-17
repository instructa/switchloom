import { describe, expect, it } from "vitest";
import catalog from "../../data/catalog.json";
import { buildWorkflowPrompt, defaultOptions, moveCapability, toggleModel, workflowError, workflowFromCatalog } from "./generator";
const workflow = workflowFromCatalog(catalog);

describe("Capability board", () => {
  it("starts with Luna max first, Jev on, and specialists disabled", () => {
    const options = defaultOptions(workflow);
    expect(workflow.slots.map(slot => slot.id)).toEqual(["luna", "sol", "astra"]);
    expect(options.models.luna).toEqual({ model: "gpt-5.6-luna", effort: "max", enabled: true });
    expect(options.jev).toBe(true);
    const prompt = buildWorkflowPrompt(workflow, options);
    expect(prompt).toContain("3 persistent Codex threads");
    expect(prompt).not.toMatch(/Browser use|Computer use|Visual verification|3D modeling/);
    expect(prompt).toContain("environment type local");
    expect(prompt).toContain("After handing off, end your turn");
  });
  it("restores a card's defaults on every reactivation, with unique owners", () => {
    let options = defaultOptions(workflow);
    options.models.luna = { model: "gpt-next", effort: "low", enabled: true };
    options = moveCapability(workflow, options, "browser", "luna");
    for (let i = 0; i < 3; i++) {
      options = toggleModel(workflow, options, "luna", false);
      expect(options.owners.coordination).toBeNull();
      expect(options.owners.browser).toBeNull();
      expect(buildWorkflowPrompt(workflow, options)).not.toContain('"luna":');
      options = moveCapability(workflow, options, "coordination", "sol");
      options = toggleModel(workflow, options, "luna", true);
      expect(options.models.luna).toEqual(defaultOptions(workflow).models.luna);
      expect(options.owners.coordination).toBe("luna");
      expect(options.owners.browser).toBeNull();
    }
  });
  it("resets every model's settings and capabilities without changing other settings", () => {
    for (const slot of workflow.slots) {
      let options = defaultOptions(workflow);
      options.jev = false;
      options.models[slot.id].effort = "low";
      options = moveCapability(workflow, options, "computer", slot.id);
      options = toggleModel(workflow, options, slot.id, false);
      expect(moveCapability(workflow, options, "browser", slot.id)).toBe(options);
      options = toggleModel(workflow, options, slot.id, true);
      expect(options).toEqual({ ...defaultOptions(workflow), jev: false });
    }
    const dirty = defaultOptions(workflow);
    dirty.models.sol.model = "gpt-next";
    dirty.owners.review = null;
    expect(defaultOptions(workflow).models.sol.model).toBe("gpt-5.6-sol");
    expect(defaultOptions(workflow).owners.review).toBe("astra");
  });
  it("exports exactly the assigned capabilities, custom models and vanilla mode", () => {
    let options = defaultOptions(workflow);
    options.models.sol = { model: "gpt-6-sol", effort: "max", enabled: true };
    options = moveCapability(workflow, options, "browser", "sol");
    options = moveCapability(workflow, options, "review", null);
    options.jev = false;
    const prompt = buildWorkflowPrompt(workflow, options);
    expect(prompt).toContain('"sol":{"model":"gpt-6-sol","effort":"max","capabilities":["implementation","debugging","validation","browser"]}');
    expect(prompt).toContain("Browser use:");
    expect(prompt).not.toContain("Code review:");
    expect(prompt).toContain("Vanilla mode");
    expect(prompt).not.toContain("$switchloom");
    expect(prompt).not.toContain("request.jev=true");
  });
  it("keeps invalid settings visible and blocks export without a usable assignment", () => {
    let options = defaultOptions(workflow);
    options.models.luna.effort = "ultra";
    expect(workflowError(workflow, options)).not.toBeNull();
    expect(() => buildWorkflowPrompt(workflow, options)).toThrow();
    options = toggleModel(workflow, options, "luna", false);
    expect(workflowError(workflow, options)).toBeNull();
    options = toggleModel(workflow, options, "sol", false);
    options = toggleModel(workflow, options, "astra", false);
    expect(workflowError(workflow, options)).toBe("Enable at least one model.");
    options = defaultOptions(workflow);
    for (const cap of workflow.capabilities) options = moveCapability(workflow, options, cap.id, null);
    expect(workflowError(workflow, options)).toBe("Assign at least one capability.");
  });
});
