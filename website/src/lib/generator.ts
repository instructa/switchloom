export const ROLE_IDS = ["orchestrator", "implementer", "reviewer", "verifier"] as const;
export const HOST_IDS = ["codex", "cursor", "claude-code", "opencode", "pi"] as const;
export const PRESET_IDS = ["light", "balanced", "high"] as const;

export type RoleId = (typeof ROLE_IDS)[number];
export type HostId = (typeof HOST_IDS)[number];
export type PresetId = (typeof PRESET_IDS)[number];
export type SetupIntegration = "standalone" | "planr";

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
  integration: SetupIntegration;
  usagePolicy: string;
  roles: RoleId[];
  assignments: Record<RoleId, RoleAssignment>;
};
export type HostCatalog = Record<HostId, { binding: string; models: ModelOption[] }>;

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
  runtime: string;
  effortLabel: string | null;
  defaults: Record<RoleId, RoleAssignment>;
}> = {
  codex: {
    label: "Codex",
    note: "Internal V2 child threads using project-local .codex roles; Codex remains orchestration and billing authority.",
    runtime: "V2 thread tree",
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
    note: "Native Cursor project agents with live nonce-correlated requested-routing evidence; effective model claims stay advisory unless Cursor exposes them.",
    runtime: "native subagent",
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
    note: "Native subagents using Claude model aliases and role prompts; this release keeps Claude unavailable/unverified until live host receipts exist.",
    runtime: "native subagent",
    effortLabel: "Effort",
    defaults: {
      orchestrator: { model: "opus", effort: "high" },
      implementer: { model: "sonnet", effort: "medium" },
      reviewer: { model: "opus", effort: "high" },
      verifier: { model: "sonnet", effort: "medium" },
    },
  },
  opencode: {
    label: "OpenCode",
    note: "Project-local OpenCode agents with Task permissions and provider-qualified models; not part of the current live release gate.",
    runtime: "native subagent",
    effortLabel: "Variant",
    defaults: {
      orchestrator: { model: "opencode/gpt-5-nano", effort: "medium" },
      implementer: { model: "opencode/gpt-5-nano", effort: "low" },
      reviewer: { model: "anthropic/claude-sonnet-4-5", effort: "medium" },
      verifier: { model: "opencode/gpt-5-nano", effort: "low" },
    },
  },
  pi: {
    label: "Pi",
    note: "External runner workflows using isolated print-mode process execution; separate from host-native child threads.",
    runtime: "external runner",
    effortLabel: "Thinking",
    defaults: {
      orchestrator: { model: "openai/gpt-4o-mini", effort: "medium" },
      implementer: { model: "openai/gpt-4o-mini", effort: "low" },
      reviewer: { model: "anthropic/claude-sonnet-4-5", effort: "high" },
      verifier: { model: "google/gemini-2.5-flash", effort: "low" },
    },
  },
};

export const PRESETS: Record<PresetId, { label: string; short: string }> = {
  light: { label: "Light", short: "Prioritizes lower-cost models and effort." },
  balanced: { label: "Balanced", short: "A practical quality and cost mix for daily work." },
  high: { label: "High", short: "Prioritizes stronger models and deeper reasoning." },
};

const PRESET_USAGE_POLICIES: Record<PresetId, string> = {
  light: "low-usage",
  balanced: "balanced",
  high: "max-quality",
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
  opencode: {
    light: {
      orchestrator: { model: "opencode/gpt-5-nano", effort: "low" },
      implementer: { model: "opencode/gpt-5-nano", effort: "low" },
      reviewer: { model: "opencode/gpt-5-nano", effort: "low" },
      verifier: { model: "opencode/gpt-5-nano", effort: "low" },
    },
    balanced: HOSTS.opencode.defaults,
    high: {
      orchestrator: { model: "anthropic/claude-opus-4-5", effort: "high" },
      implementer: { model: "opencode/gpt-5-nano", effort: "high" },
      reviewer: { model: "anthropic/claude-opus-4-5", effort: "max" },
      verifier: { model: "opencode/gpt-5-nano", effort: "max" },
    },
  },
  pi: {
    light: {
      orchestrator: { model: "google/gemini-2.5-flash", effort: "low" },
      implementer: { model: "google/gemini-2.5-flash", effort: "low" },
      reviewer: { model: "openai/gpt-4o-mini", effort: "low" },
      verifier: { model: "google/gemini-2.5-flash", effort: "low" },
    },
    balanced: HOSTS.pi.defaults,
    high: {
      orchestrator: { model: "anthropic/claude-sonnet-4-5", effort: "high" },
      implementer: { model: "openai/gpt-4o-mini", effort: "high" },
      reviewer: { model: "anthropic/claude-sonnet-4-5", effort: "xhigh" },
      verifier: { model: "openai/gpt-4o-mini", effort: "xhigh" },
    },
  },
};

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
    "opencode/gpt-5-nano": "GPT 5 Nano",
    "openai/gpt-4o-mini": "GPT 4o Mini",
    "google/gemini-2.5-flash": "Gemini 2.5 Flash",
    "anthropic/claude-sonnet-4-5": "Claude Sonnet 4.5",
    "anthropic/claude-opus-4-5": "Claude Opus 4.5",
    opus: "Opus",
    sonnet: "Sonnet",
  };
  return labels[model] ?? model;
}

function modelProvider(model: string) {
  if (model.startsWith("gpt-")) return "OpenAI";
  if (model.startsWith("openai/")) return "OpenAI";
  if (model.startsWith("google/")) return "Google";
  if (model.startsWith("opencode/")) return "OpenCode";
  if (model.startsWith("anthropic/")) return "Anthropic";
  if (model.startsWith("claude-") || model === "opus" || model === "sonnet" || model === "fable-5") return "Anthropic";
  if (model.startsWith("grok") || model.startsWith("composer")) return "Cursor";
  return undefined;
}

type CatalogShape = {
  setupContract?: {
    recipePrefix?: string;
    configPath?: string;
    hosts?: Array<{
      id?: string;
      binding?: string;
      models?: Array<{ id?: string; efforts?: string[]; tier?: string }>;
    }>;
  };
};

export function hostCatalogFrom(catalog: CatalogShape): HostCatalog {
  const result = {} as HostCatalog;
  for (const host of HOST_IDS) {
    const setupHost = catalog.setupContract?.hosts?.find((entry) => entry.id === host);
    if (!setupHost?.binding) throw new Error(`canonical setup contract has no ${host} binding`);
    const models = setupHost.models?.map((model) => {
      if (!model.id || !Array.isArray(model.efforts) || (model.tier !== "standard" && model.tier !== "premium")) {
        throw new Error(`canonical setup contract has invalid ${host} model entry`);
      }
      const tier: ModelOption["tier"] = model.tier === "premium" ? "premium" : "standard";
      return {
        id: model.id,
        label: modelLabel(model.id),
        provider: modelProvider(model.id),
        efforts: model.efforts,
        tier,
      };
    }) ?? [];
    result[host] = { binding: setupHost.binding, models };
  }
  for (const host of HOST_IDS) {
    if (result[host].models.length === 0) throw new Error(`canonical setup contract has no ${host} model profiles`);
    for (const [role, assignment] of Object.entries(HOSTS[host].defaults)) {
      const model = result[host].models.find((candidate) => candidate.id === assignment.model);
      if (!model) throw new Error(`canonical setup contract has no ${host} default model for ${role}: ${assignment.model}`);
      if (assignment.effort && !model.efforts.includes(assignment.effort)) {
        throw new Error(`canonical setup contract has no ${host} default effort for ${role}: ${assignment.effort}`);
      }
    }
  }
  return result;
}

export function setupTransportFrom(catalog: CatalogShape) {
  const recipePrefix = catalog.setupContract?.recipePrefix;
  const configPath = catalog.setupContract?.configPath;
  if (!recipePrefix || !configPath) throw new Error("canonical setup contract is missing transport metadata");
  return { recipePrefix, configPath };
}

export function createConfig(host: HostId = "codex"): GeneratorConfig {
  return {
    host,
    integration: "standalone",
    usagePolicy: "balanced",
    roles: [...ROLE_IDS],
    assignments: structuredClone(HOSTS[host].defaults),
  };
}

export function changeHost(config: GeneratorConfig, host: HostId): GeneratorConfig {
  return { ...createConfig(host), integration: config.integration, usagePolicy: config.usagePolicy, roles: [...config.roles] };
}

export function setIntegration(config: GeneratorConfig, integration: SetupIntegration): GeneratorConfig {
  return { ...config, integration };
}

export function applyPreset(config: GeneratorConfig, preset: PresetId, catalog: HostCatalog): GeneratorConfig {
  const assignments = structuredClone(PRESET_ASSIGNMENTS[config.host][preset]);
  for (const [role, assignment] of Object.entries(assignments)) {
    const model = catalog[config.host].models.find((candidate) => candidate.id === assignment.model);
    if (!model) throw new Error(`${preset} preset has no ${config.host} model for ${role}: ${assignment.model}`);
    if (assignment.effort && !model.efforts.includes(assignment.effort)) {
      throw new Error(`${preset} preset has no ${config.host} effort for ${role}: ${assignment.effort}`);
    }
  }
  return { ...config, usagePolicy: PRESET_USAGE_POLICIES[preset], assignments };
}

export function setRoles(config: GeneratorConfig, roles: readonly string[]): GeneratorConfig {
  const valid = ROLE_IDS.filter((role) => role === "orchestrator" || roles.includes(role));
  return { ...config, roles: valid.slice(0, 4) };
}

export function setModel(config: GeneratorConfig, role: RoleId, modelId: string, catalog: HostCatalog): GeneratorConfig {
  const model = catalog[config.host].models.find((candidate) => candidate.id === modelId);
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
  const model = catalog[config.host].models.find((candidate) => candidate.id === assignment.model);
  if (!model?.efforts.includes(effort)) throw new Error(`unsupported effort ${effort} for ${assignment.model}`);
  return { ...config, assignments: { ...config.assignments, [role]: { ...assignment, effort } } };
}

export type SetupSpecV1 = {
  schema_version: 1;
  host: string;
  integration: SetupIntegration;
  usage_policy: string;
  selected_roles: Record<string, {
    model: string;
    effort?: string;
    spawn?: {
      agent_type: string;
      task_name: string;
      fork_turns: { mode: "none" };
    };
  }>;
  routes: Array<{ work_type: string; role: string; fallbacks: string[] }>;
  route_default: { role: string; fallbacks: string[] };
};

function routingRole(config: GeneratorConfig, preferred: RoleId, fallback: RoleId = "orchestrator") {
  return config.roles.includes(preferred) ? preferred : fallback;
}

export function setupSpec(config: GeneratorConfig, catalog: HostCatalog): SetupSpecV1 {
  const selected_roles: SetupSpecV1["selected_roles"] = {};
  for (const role of config.roles) {
    const assignment = config.assignments[role];
    selected_roles[role] = {
      model: assignment.model,
      ...(assignment.effort ? { effort: assignment.effort } : {}),
      ...(config.host === "codex"
        ? {
            spawn: {
              agent_type: `switchloom_${role}`,
              task_name: role,
              fork_turns: { mode: "none" },
            },
          }
        : {}),
    };
  }
  const reviewFallback = routingRole(config, "reviewer");
  return {
    schema_version: 1,
    host: catalog[config.host].binding,
    integration: config.integration,
    usage_policy: config.usagePolicy,
    selected_roles,
    routes: [
      { work_type: "planning", role: "orchestrator", fallbacks: [] },
      { work_type: "code", role: routingRole(config, "implementer"), fallbacks: [] },
      { work_type: "review", role: reviewFallback, fallbacks: [] },
      { work_type: "verification", role: routingRole(config, "verifier", reviewFallback), fallbacks: [] },
    ],
    route_default: { role: "orchestrator", fallbacks: [] },
  };
}

function jsonRecipePayload(spec: SetupSpecV1) {
  return `${JSON.stringify(spec, null, 2)}\n`;
}

function base64Url(bytes: Uint8Array) {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 1) binary += String.fromCharCode(bytes[index]);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

export function setupRecipe(config: GeneratorConfig, catalog: HostCatalog, recipePrefix = "sw1_") {
  return `${recipePrefix}${base64Url(new TextEncoder().encode(jsonRecipePayload(setupSpec(config, catalog))))}`;
}

function tomlString(value: string) {
  return JSON.stringify(value);
}

function tomlArray(values: readonly string[]) {
  return `[${values.map(tomlString).join(", ")}]`;
}

export function setupConfigToml(config: GeneratorConfig, catalog: HostCatalog) {
  const spec = setupSpec(config, catalog);
  const lines = [
    "schema_version = 1",
    `host = ${tomlString(spec.host)}`,
    `integration = ${tomlString(spec.integration)}`,
    `usage_policy = ${tomlString(spec.usage_policy)}`,
    "",
  ];
  for (const route of spec.routes) {
    lines.push("[[routes]]", `work_type = ${tomlString(route.work_type)}`, `role = ${tomlString(route.role)}`, `fallbacks = ${tomlArray(route.fallbacks)}`, "");
  }
  lines.push("[route_default]", `role = ${tomlString(spec.route_default.role)}`, `fallbacks = ${tomlArray(spec.route_default.fallbacks)}`, "");
  for (const role of config.roles) {
    const selection = spec.selected_roles[role];
    lines.push(`[selected_roles.${role}]`, `model = ${tomlString(selection.model)}`);
    if (selection.effort) lines.push(`effort = ${tomlString(selection.effort)}`);
    lines.push("");
    if (selection.spawn) {
      lines.push(
        `[selected_roles.${role}.spawn]`,
        `agent_type = ${tomlString(selection.spawn.agent_type)}`,
        `task_name = ${tomlString(selection.spawn.task_name)}`,
        "",
        `[selected_roles.${role}.spawn.fork_turns]`,
        `mode = ${tomlString(selection.spawn.fork_turns.mode)}`,
        "",
      );
    }
  }
  return lines.join("\n");
}

export function shellQuote(value: string) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

export function recipeApplyCommand(config: GeneratorConfig, catalog: HostCatalog, recipePrefix = "sw1_") {
  return `npx switchloom@0.3.0 apply --recipe ${shellQuote(setupRecipe(config, catalog, recipePrefix))} --repository .`;
}

export function lifecycleCommands(config: GeneratorConfig, catalog: HostCatalog, recipePrefix = "sw1_") {
  const recipe = shellQuote(setupRecipe(config, catalog, recipePrefix));
  const host = config.host;
  return [
    "npm install -g switchloom@0.3.0",
    `switchloom preview --recipe ${recipe} --repository .`,
    `switchloom apply --recipe ${recipe} --repository .`,
    `switchloom doctor ${host}`,
    "switchloom update --repository .",
    "switchloom status --repository .",
    "switchloom rollback --repository .",
    "switchloom uninstall --repository .",
  ];
}

export function setupSummary(config: GeneratorConfig) {
  return [
    `${HOSTS[config.host].label} ${config.integration === "planr" ? "with Planr" : "standalone"} team`,
    ...config.roles.map((role) => {
      const value = config.assignments[role];
      return `${ROLES[role].label}: ${value.model}${value.effort ? ` · ${value.effort}` : ""}`;
    }),
  ].join("\n");
}
