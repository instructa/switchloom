import { describe, expect, it } from "vitest";

import { effortHint, formatEffortLabel, modelFamilyInfo } from "./model-family";

describe("model family metadata", () => {
  it("maps Sol/Terra/Luna to increasing celestial strength", () => {
    expect(modelFamilyInfo("gpt-5.6-luna")?.strength).toBe(1);
    expect(modelFamilyInfo("gpt-5.6-terra")?.strength).toBe(2);
    expect(modelFamilyInfo("gpt-5.6-sol")?.strength).toBe(3);
    expect(modelFamilyInfo("gpt-5.6-sol")?.short.length).toBeGreaterThan(20);
  });

  it("formats effort labels for the strength picker", () => {
    expect(formatEffortLabel("xhigh")).toBe("Extra High");
    expect(formatEffortLabel("ultra")).toBe("Ultra");
    expect(effortHint("ultra")).toMatch(/multi-agent/i);
    expect(effortHint("medium")).toMatch(/balanced/i);
  });
});
