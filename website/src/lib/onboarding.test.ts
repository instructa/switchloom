import { describe, expect, it } from "vitest";

import { HOST_IDS, SWITCHLOOM_VERSION } from "./generator";
import { PROVIDER_ONBOARDING, providerOnboarding } from "./onboarding";

describe("provider onboarding templates", () => {
  it("defines one complete provider-owned template for every runtime", () => {
    expect(Object.keys(PROVIDER_ONBOARDING).sort()).toEqual([...HOST_IDS].sort());
    expect(new Set(Object.values(PROVIDER_ONBOARDING).map((template) => template.title)).size).toBe(HOST_IDS.length);

    for (const host of HOST_IDS) {
      const template = PROVIDER_ONBOARDING[host];
      expect(template.host).toBe(host);
      expect(template.icon).toMatch(/^\/brand\/[a-z-]+\.svg$/);
      expect(template.steps.map((step) => step.id)).toEqual(["requirements", "project", "install", "activate"]);
      expect(template.steps.every((step) => step.title.length > 0 && step.description.length > 0)).toBe(true);
      expect(template.steps.filter((step) => step.command?.kind === "apply")).toHaveLength(1);
    }
  });

  it("injects the exact generated recipe command without mutating provider copy", () => {
    const command = `npx switchloom@${SWITCHLOOM_VERSION} apply --recipe 'sw1_example' --repository .`;
    const onboarding = providerOnboarding("codex", command);

    expect(onboarding.steps.find((step) => step.id === "install")?.command).toBe(command);
    expect(onboarding.description).toContain("native V2 child roles");
    expect(onboarding.description).toContain("parent orchestrator");
    expect(onboarding.steps.find((step) => step.id === "project")?.description).toContain("Codex child-role files");
    expect(onboarding.steps.find((step) => step.id === "project")?.description).toContain("does not touch your global Codex settings");
    expect(PROVIDER_ONBOARDING.codex.steps.find((step) => step.id === "install")?.command).toEqual({ kind: "apply" });
  });
});
