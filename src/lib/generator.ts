export const ROLE_IDS = ["orchestrator", "implementer", "reviewer", "verifier"] as const;
export const HOST_IDS = ["codex", "cursor", "claude-code"] as const;
export const PRESET_IDS = ["light", "balanced", "high"] as const;

export type RoleId = (typeof ROLE_IDS)[number];
export type HostId = (typeof HOST_IDS)[number];
export type PresetId = (typeof PRESET_IDS)[number];

export type ModelOption = {
  id: string;
  label: string;
  provider?: string;
  efforts: readonly string[];
  tier: "standard" | "premium";
};

export type RoleAssignment = { model: string; effort?: string };
export type GeneratorConfig = {
  host: HostId;
  roles: RoleId[];
  assignments: Record<RoleId, RoleAssignment>;
};
export type HostCatalog = Record<HostId, ModelOption[]>;

export const ROLES: Record<RoleId, { label: string; short: string; instructions: string; writable: boolean }> = {
  orchestrator: {
    label: "Orchestrator",
    short: "Plans, delegates, and owns the final answer.",
    instructions: "Plan the work, delegate bounded tasks, keep decisions coherent, and retain final synthesis ownership.",
    writable: true,
  },
  implementer: {
    label: "Implementer",
    short: "Writes code and runs focused tests.",
    instructions: "Implement one bounded change, preserve existing behavior, and run focused verification before returning evidence.",
    writable: true,
  },
  reviewer: {
    label: "Reviewer",
    short: "Finds defects independently.",
    instructions: "Review independently, lead with concrete findings, and do not edit the implementation under review.",
    writable: false,
  },
  verifier: {
    label: "Verifier",
    short: "Proves the result actually works.",
    instructions: "Verify acceptance criteria with reproducible commands or live checks and report evidence without changing product code.",
    writable: false,
  },
};

export const HOSTS: Record<HostId, {
  label: string;
  note: string;
  effortLabel: string | null;
  defaults: Record<RoleId, RoleAssignment>;
}> = {
  codex: {
    label: "Codex",
    note: "Native project agents with per-role model and reasoning effort.",
    effortLabel: "Reasoning",
    defaults: {
      orchestrator: { model: "gpt-5.6-sol", effort: "medium" },
      implementer: { model: "gpt-5.6-terra", effort: "high" },
      reviewer: { model: "gpt-5.6-sol", effort: "high" },
      verifier: { model: "gpt-5.6-terra", effort: "medium" },
    },
  },
  cursor: {
    label: "Cursor",
    note: "Project agents with current frontier models and per-role reasoning effort; Cursor remains model authority.",
    effortLabel: "Reasoning",
    defaults: {
      orchestrator: { model: "fable-5", effort: "high" },
      implementer: { model: "composer-2.5" },
      reviewer: { model: "gpt-5.6-sol", effort: "high" },
      verifier: { model: "gpt-5.6-terra", effort: "medium" },
    },
  },
  "claude-code": {
    label: "Claude Code",
    note: "Native subagents using Claude model aliases and role prompts.",
    effortLabel: "Effort",
    defaults: {
      orchestrator: { model: "opus", effort: "high" },
      implementer: { model: "sonnet", effort: "medium" },
      reviewer: { model: "opus", effort: "high" },
      verifier: { model: "sonnet", effort: "medium" },
    },
  },
};

export const PRESETS: Record<PresetId, { label: string; short: string }> = {
  light: { label: "Light", short: "Prioritizes lower-cost models and effort." },
  balanced: { label: "Balanced", short: "A practical quality and cost mix for daily work." },
  high: { label: "High", short: "Prioritizes stronger models and deeper reasoning." },
};

const PRESET_ASSIGNMENTS: Record<HostId, Record<PresetId, Record<RoleId, RoleAssignment>>> = {
  codex: {
    light: {
      orchestrator: { model: "gpt-5.6-terra", effort: "low" },
      implementer: { model: "gpt-5.6-luna", effort: "low" },
      reviewer: { model: "gpt-5.6-terra", effort: "low" },
      verifier: { model: "gpt-5.6-luna", effort: "low" },
    },
    balanced: HOSTS.codex.defaults,
    high: {
      orchestrator: { model: "gpt-5.6-sol", effort: "medium" },
      implementer: { model: "gpt-5.6-sol", effort: "medium" },
      reviewer: { model: "gpt-5.6-sol", effort: "medium" },
      verifier: { model: "gpt-5.6-sol", effort: "medium" },
    },
  },
  cursor: {
    light: {
      orchestrator: { model: "gpt-5.6-luna", effort: "low" },
      implementer: { model: "composer-2.5" },
      reviewer: { model: "gpt-5.6-luna", effort: "low" },
      verifier: { model: "gpt-5.6-luna", effort: "low" },
    },
    balanced: HOSTS.cursor.defaults,
    high: {
      orchestrator: { model: "fable-5", effort: "max" },
      implementer: { model: "gpt-5.6-sol", effort: "xhigh" },
      reviewer: { model: "gpt-5.6-sol", effort: "max" },
      verifier: { model: "gpt-5.6-terra", effort: "xhigh" },
    },
  },
  "claude-code": {
    light: {
      orchestrator: { model: "sonnet", effort: "medium" },
      implementer: { model: "sonnet", effort: "medium" },
      reviewer: { model: "sonnet", effort: "medium" },
      verifier: { model: "sonnet", effort: "medium" },
    },
    balanced: HOSTS["claude-code"].defaults,
    high: {
      orchestrator: { model: "opus", effort: "high" },
      implementer: { model: "opus", effort: "high" },
      reviewer: { model: "opus", effort: "high" },
      verifier: { model: "opus", effort: "high" },
    },
  },
};

// Codex publishes model-specific effort support through its model manifest.
// Ultra is intentionally exposed only as a manual choice: unlike the regular
// effort ladder, it enables automatic multi-agent delegation.
export const CODEX_FRONTIER_MODELS: readonly ModelOption[] = [
  { id: "gpt-5.6-sol", label: "Sol", provider: "OpenAI", efforts: ["low", "medium", "high", "xhigh", "ultra"], tier: "premium" },
  { id: "gpt-5.6-terra", label: "Terra", provider: "OpenAI", efforts: ["low", "medium", "high", "xhigh", "ultra"], tier: "standard" },
  { id: "gpt-5.6-luna", label: "Luna", provider: "OpenAI", efforts: ["low", "medium", "high", "xhigh"], tier: "standard" },
] as const;

function modelLabel(model: string) {
  const labels: Record<string, string> = {
    "gpt-5.6-sol": "Sol",
    "gpt-5.6-terra": "Terra",
    "gpt-5.6-luna": "Luna",
    "fable-5": "Fable 5",
    "claude-fable-5": "Fable 5",
    "claude-opus-4-8": "Opus 4.8",
    "claude-sonnet-5": "Sonnet 5",
    "grok-4.5": "Grok 4.5",
    "composer-2.5": "Composer 2.5",
    opus: "Opus",
    sonnet: "Sonnet",
  };
  return labels[model] ?? model;
}

export const CURSOR_FRONTIER_MODELS: readonly ModelOption[] = [
  { id: "gpt-5.6-sol", label: "GPT-5.6 Sol", provider: "OpenAI", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "premium" },
  { id: "gpt-5.6-terra", label: "GPT-5.6 Terra", provider: "OpenAI", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "standard" },
  { id: "gpt-5.6-luna", label: "GPT-5.6 Luna", provider: "OpenAI", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "standard" },
  { id: "fable-5", label: "Fable 5", provider: "Anthropic", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "premium" },
  { id: "claude-opus-4-8", label: "Opus 4.8", provider: "Anthropic", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "premium" },
  { id: "claude-sonnet-5", label: "Sonnet 5", provider: "Anthropic", efforts: ["low", "medium", "high", "xhigh", "max"], tier: "standard" },
  { id: "grok-4.5", label: "Grok 4.5", provider: "Cursor", efforts: ["low", "medium", "high"], tier: "premium" },
  { id: "composer-2.5", label: "Composer 2.5", provider: "Cursor", efforts: [], tier: "standard" },
] as const;

type CatalogProfile = { model?: string; effort?: string; cost_tier?: string };
type CatalogShape = {
  compositions?: Array<{
    binding?: {
      host?: string;
      profiles?: Record<string, CatalogProfile | undefined>;
    };
  }>;
};

export function hostCatalogFrom(catalog: CatalogShape): HostCatalog {
  const grouped: Record<HostId, Map<string, { efforts: Set<string>; tier: "standard" | "premium" }>> = {
    codex: new Map(),
    cursor: new Map(),
    "claude-code": new Map(),
  };
  for (const composition of catalog.compositions ?? []) {
    const host = composition.binding?.host;
    if (!HOST_IDS.includes(host as HostId)) continue;
    for (const profile of Object.values(composition.binding?.profiles ?? {})) {
      if (!profile?.model) continue;
      const values = grouped[host as HostId];
      const current = values.get(profile.model) ?? { efforts: new Set<string>(), tier: "standard" as const };
      if (profile.effort) current.efforts.add(profile.effort);
      if (profile.cost_tier === "premium") current.tier = "premium";
      values.set(profile.model, current);
    }
  }
  const result: HostCatalog = { codex: [], cursor: [], "claude-code": [] };
  for (const host of HOST_IDS) {
    result[host] = [...grouped[host]].map(([id, value]) => ({
      id,
      label: modelLabel(id),
      efforts: [...value.efforts],
      tier: value.tier,
    }));
  }
  // Keep current host manifests authoritative where the composition catalog is
  // intentionally narrower than the host's model picker.
  result.codex = CODEX_FRONTIER_MODELS.map((model) => ({ ...model }));
  // Cursor exposes many transient and legacy models, so keep its researched
  // frontier set instead of mirroring every catalog entry.
  result.cursor = CURSOR_FRONTIER_MODELS.map((model) => ({ ...model }));
  for (const host of HOST_IDS) {
    if (result[host].length === 0) throw new Error(`canonical catalog has no ${host} model profiles`);
    for (const [role, assignment] of Object.entries(HOSTS[host].defaults)) {
      const model = result[host].find((candidate) => candidate.id === assignment.model);
      if (!model) throw new Error(`canonical catalog has no ${host} default model for ${role}: ${assignment.model}`);
      if (assignment.effort && !model.efforts.includes(assignment.effort)) {
        throw new Error(`canonical catalog has no ${host} default effort for ${role}: ${assignment.effort}`);
      }
    }
  }
  return result;
}

export function createConfig(host: HostId = "codex"): GeneratorConfig {
  return {
    host,
    roles: [...ROLE_IDS],
    assignments: structuredClone(HOSTS[host].defaults),
  };
}

export function changeHost(config: GeneratorConfig, host: HostId): GeneratorConfig {
  return { ...createConfig(host), roles: [...config.roles] };
}

export function applyPreset(config: GeneratorConfig, preset: PresetId, catalog: HostCatalog): GeneratorConfig {
  const assignments = structuredClone(PRESET_ASSIGNMENTS[config.host][preset]);
  for (const [role, assignment] of Object.entries(assignments)) {
    const model = catalog[config.host].find((candidate) => candidate.id === assignment.model);
    if (!model) throw new Error(`${preset} preset has no ${config.host} model for ${role}: ${assignment.model}`);
    if (assignment.effort && !model.efforts.includes(assignment.effort)) {
      throw new Error(`${preset} preset has no ${config.host} effort for ${role}: ${assignment.effort}`);
    }
  }
  return { ...config, assignments };
}

export function setRoles(config: GeneratorConfig, roles: readonly string[]): GeneratorConfig {
  const valid = ROLE_IDS.filter((role) => role === "orchestrator" || roles.includes(role));
  return { ...config, roles: valid.slice(0, 4) };
}

export function setModel(config: GeneratorConfig, role: RoleId, modelId: string, catalog: HostCatalog): GeneratorConfig {
  const model = catalog[config.host].find((candidate) => candidate.id === modelId);
  if (!model) throw new Error(`unsupported ${config.host} model: ${modelId}`);
  return {
    ...config,
    assignments: {
      ...config.assignments,
      [role]: { model: model.id, effort: model.efforts[0] },
    },
  };
}

export function setEffort(config: GeneratorConfig, role: RoleId, effort: string, catalog: HostCatalog): GeneratorConfig {
  const assignment = config.assignments[role];
  const model = catalog[config.host].find((candidate) => candidate.id === assignment.model);
  if (!model?.efforts.includes(effort)) throw new Error(`unsupported effort ${effort} for ${assignment.model}`);
  return { ...config, assignments: { ...config.assignments, [role]: { ...assignment, effort } } };
}

function yaml(role: RoleId, assignment: RoleAssignment) {
  const lines = ["---", `name: switchloom-${role}`, `model: ${assignment.model}`];
  if (assignment.effort) lines.push(`effort: ${assignment.effort}`);
  lines.push("---", ROLES[role].instructions, "");
  return lines.join("\n");
}

function codexAgent(role: RoleId, assignment: RoleAssignment) {
  const definition = ROLES[role];
  return [
    `name = "switchloom_${role}"`,
    `description = "${definition.short}"`,
    `model = "${assignment.model}"`,
    `model_reasoning_effort = "${assignment.effort}"`,
    `sandbox_mode = "${definition.writable ? "workspace-write" : "read-only"}"`,
    "",
    "developer_instructions = \"\"\"",
    definition.instructions,
    "\"\"\"",
    "",
  ].join("\n");
}

export function generateFiles(config: GeneratorConfig): Record<string, string> {
  const files: Record<string, string> = {};
  for (const role of config.roles) {
    const assignment = config.assignments[role];
    if (config.host === "codex") files[`.codex/agents/switchloom-${role}.toml`] = codexAgent(role, assignment);
    if (config.host === "cursor") files[`.cursor/agents/switchloom-${role}.md`] = yaml(role, assignment);
    if (config.host === "claude-code") files[`.claude/agents/switchloom-${role}.md`] = yaml(role, assignment);
  }

  const routeLines = config.roles.map((role) => `- ${ROLES[role].label}: \`switchloom-${role}\` — ${ROLES[role].short}`);
  files[config.host === "codex" ? ".codex/skills/switchloom-routing/SKILL.md" : "SWITCHLOOM.md"] = [
    "# Switchloom routing",
    "",
    `Host: ${HOSTS[config.host].label}`,
    "",
    "Route work to the narrowest suitable role. The orchestrator keeps synthesis ownership.",
    "",
    ...routeLines,
    "",
  ].join("\n");
  files["switchloom.config.json"] = `${JSON.stringify(config, null, 2)}\n`;
  files["README-SWITCHLOOM.md"] = [
    "# Your Switchloom setup",
    "",
    `Generated for ${HOSTS[config.host].label} with ${config.roles.length} roles.`,
    "",
    "Extract this archive into the root of your repository, review the generated files, and commit the files you want to share with your team.",
    "The selected host remains authoritative for model availability, execution, and billing.",
    "",
  ].join("\n");
  return files;
}

export function setupSummary(config: GeneratorConfig) {
  return [
    `${HOSTS[config.host].label} team`,
    ...config.roles.map((role) => {
      const value = config.assignments[role];
      return `${ROLES[role].label}: ${value.model}${value.effort ? ` · ${value.effort}` : ""}`;
    }),
  ].join("\n");
}
