export type ModelFamilyId = "sol" | "terra" | "luna";

export type ModelFamilyInfo = {
  id: ModelFamilyId;
  /** Relative capability mass: Luna lightest, Sol heaviest. */
  strength: 1 | 2 | 3;
  short: string;
};

const MODEL_FAMILY_BY_ID: Record<string, ModelFamilyInfo> = {
  "gpt-5.6-sol": {
    id: "sol",
    strength: 3,
    short: "For complex, open-ended work where deeper investigation and polish matter.",
  },
  "gpt-5.6-terra": {
    id: "terra",
    strength: 2,
    short: "Pragmatic all-rounder for everyday implementation and multi-step work.",
  },
  "gpt-5.6-luna": {
    id: "luna",
    strength: 1,
    short: "Fast option for clear, well-scoped work and high-volume workflows.",
  },
};

export function modelFamilyInfo(modelId: string): ModelFamilyInfo | undefined {
  return MODEL_FAMILY_BY_ID[modelId];
}

const EFFORT_LABELS: Record<string, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  xhigh: "Extra High",
  ultra: "Ultra",
  max: "Max",
};

export function formatEffortLabel(effort: string) {
  return EFFORT_LABELS[effort] ?? effort;
}

const EFFORT_HINTS: Record<string, string> = {
  low: "Faster replies with lighter reasoning.",
  medium: "Balanced depth for most day-to-day work.",
  high: "Deeper reasoning when quality matters more.",
  xhigh: "Extra depth for harder, multi-step problems.",
  ultra: "Max depth plus multi-agent help. Costs more.",
  max: "Maximum available reasoning for this model.",
};

export function effortHint(effort: string): string | undefined {
  return EFFORT_HINTS[effort];
}
