import packageMetadata from "../../../package.json";

export const PRIMARY_RECOMMENDATION_ID = "orchestrator" as const;
export const CHILD_ROLE_IDS = ["implementer", "reviewer", "verifier"] as const;
export const ROLE_IDS = [PRIMARY_RECOMMENDATION_ID, ...CHILD_ROLE_IDS] as const;
export const HOST_IDS = ["codex", "cursor", "claude-code", "opencode", "pi"] as const;
export const PRESET_IDS = ["light", "balanced", "high"] as const;
export const SWITCHLOOM_VERSION = packageMetadata.version;

export type PrimaryRecommendationId = typeof PRIMARY_RECOMMENDATION_ID;
export type ChildRoleId = (typeof CHILD_ROLE_IDS)[number];
export type RoleId = (typeof ROLE_IDS)[number];
export type HostId = (typeof HOST_IDS)[number];
export type PresetId = (typeof PRESET_IDS)[number];
export type SetupIntegration = "standalone" | "planr";
export type OrchestrationPath = "native" | "extension";
export type WorkflowValidationStatus = "certified" | "experimental" | "planned";
export type WorkflowCapability = { codingAgent: string; executionPath: string; validationStatus: WorkflowValidationStatus; parentModel: string };

export type ModelOption = {
  id: string;
  label: string;
  provider?: string;
  efforts: readonly string[];
  tier: "standard" | "premium";
  disabledReason?: string;
};

export type RoleAssignment = { model: string; effort?: string };
export type GeneratorConfig = {
  host: HostId;
  orchestration: OrchestrationPath;
  integration: SetupIntegration;
  usagePolicy: string;
  roles: ChildRoleId[];
  assignments: Record<RoleId, RoleAssignment>;
  fallbackModels: Record<RoleId, string[]>;
};
export type PresetSelection = {
  selected: PresetId | "custom";
  lastSelected: PresetId;
};
export type HostCatalog = Record<HostId, { binding: string; models: ModelOption[] }>;
export type HostCapabilities = {
  nativeParentRecommendation: boolean;
};

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
  capabilities: HostCapabilities;
  defaults: Record<RoleId, RoleAssignment>;
}> = {
  codex: {
    label: "Codex",
    note: "Requires Codex 0.145+ and activates project-local Multi-Agent V2 roles; this release certifies exact 0.145.0 Terra and Sol routing, while Luna remains experimental/unverified. Codex remains orchestration and billing authority.",
    runtime: "V2 thread tree",
    effortLabel: "Reasoning",
    capabilities: { nativeParentRecommendation: true },
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
    capabilities: { nativeParentRecommendation: true },
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
    capabilities: { nativeParentRecommendation: true },
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
    capabilities: { nativeParentRecommendation: true },
    defaults: {
      orchestrator: { model: "opencode/gpt-5-nano", effort: "medium" },
      implementer: { model: "opencode/gpt-5-nano", effort: "low" },
      reviewer: { model: "anthropic/claude-sonnet-4-5", effort: "medium" },
      verifier: { model: "opencode/gpt-5-nano", effort: "low" },
    },
  },
  pi: {
    label: "Pi",
    note: "Your active Pi session is the Orchestrator and delegates provider-qualified child roles through Pi Subagents.",
    runtime: "Pi Subagents",
    effortLabel: "Thinking",
    capabilities: { nativeParentRecommendation: true },
    defaults: {
      orchestrator: { model: "openai-codex/gpt-5.6-terra", effort: "medium" },
      implementer: { model: "anthropic/claude-sonnet-5", effort: "medium" },
      reviewer: { model: "anthropic/claude-fable-5", effort: "high" },
      verifier: { model: "openrouter/google/gemini-3.6-flash", effort: "low" },
    },
  },
};

export const PRESETS: Record<PresetId, { label: string; short: string }> = {
  light: { label: "Light", short: "Prioritizes lower-cost models and effort." },
  balanced: { label: "Balanced", short: "A practical quality and cost mix for daily work." },
  high: { label: "High", short: "Prioritizes the strongest model and a larger execution budget." },
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
      implementer: { model: "gpt-5.6-terra", effort: "low" },
      reviewer: { model: "gpt-5.6-terra", effort: "low" },
      verifier: { model: "gpt-5.6-terra", effort: "low" },
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
      orchestrator: { model: "openai-codex/gpt-5.6-luna", effort: "low" },
      implementer: { model: "openai-codex/gpt-5.6-luna", effort: "low" },
      reviewer: { model: "openai-codex/gpt-5.6-luna", effort: "low" },
      verifier: { model: "openrouter/google/gemini-3.5-flash-lite", effort: "low" },
    },
    balanced: HOSTS.pi.defaults,
    high: {
      orchestrator: { model: "anthropic/claude-fable-5", effort: "high" },
      implementer: { model: "openai-codex/gpt-5.6-sol", effort: "high" },
      reviewer: { model: "anthropic/claude-opus-5", effort: "xhigh" },
      verifier: { model: "openai-codex/gpt-5.6-terra", effort: "high" },
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
    "openai-codex/gpt-5.6-luna": "GPT-5.6 Luna",
    "openai-codex/gpt-5.6-terra": "GPT-5.6 Terra",
    "openai-codex/gpt-5.6-sol": "GPT-5.6 Sol",
    "anthropic/claude-sonnet-5": "Claude Sonnet 5",
    "anthropic/claude-opus-5": "Claude Opus 5",
    "anthropic/claude-fable-5": "Claude Fable 5",
    "openrouter/google/gemini-3.5-flash-lite": "Gemini 3.5 Flash-Lite",
    "openrouter/google/gemini-3.6-flash": "Gemini 3.6 Flash",
    "openrouter/x-ai/grok-4.5": "Grok 4.5",
    "openrouter/moonshotai/kimi-k3": "Kimi K3",
    "openrouter/minimax/minimax-m3": "MiniMax M3",
    "openrouter/z-ai/glm-5.2": "GLM-5.2",
    "openrouter/stepfun/step-3.7-flash": "Step 3.7 Flash",
    "openrouter/xiaomi/mimo-v2.5": "MiMo V2.5",
    "openrouter/auto": "Auto Router (dynamic)",
    "anthropic/claude-opus-4-5": "Claude Opus 4.5",
    opus: "Opus",
    sonnet: "Sonnet",
  };
  return labels[model] ?? model;
}

function modelProvider(model: string) {
  if (model.startsWith("gpt-")) return "OpenAI";
  if (model.startsWith("openai/")) return "OpenAI";
  if (model.startsWith("openai-codex/")) return "OpenAI Codex";
  if (model.startsWith("google/")) return "Google";
  if ([
    "openrouter/moonshotai/kimi-k3",
    "openrouter/minimax/minimax-m3",
    "openrouter/z-ai/glm-5.2",
    "openrouter/stepfun/step-3.7-flash",
    "openrouter/xiaomi/mimo-v2.5",
  ].includes(model)) return "OpenRouter · open weights";
  if (model.startsWith("openrouter/")) return "OpenRouter";
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
    workflowCapabilities?: { capabilities?: CatalogWorkflowCapability[] };
  };
};

type CatalogWorkflowCapability = {
  codingAgent?: unknown;
  executionPath?: unknown;
  validationStatus?: unknown;
  parentModel?: unknown;
};

export function hostCatalogFrom(catalog: CatalogShape): HostCatalog {
  const result = {} as HostCatalog;
  const setupHosts = new Map(catalog.setupContract?.hosts?.map((entry) => [entry.id, entry]) ?? []);
  for (const host of HOST_IDS) {
    const setupHost = setupHosts.get(host);
    if (!setupHost?.binding) throw new Error(`canonical setup contract has no ${host} binding`);
    const models = setupHost.models?.filter((model) => !model.id?.startsWith("openai-compatible/")).map((model) => {
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
        ...(host === "codex" && model.id === "gpt-5.6-luna"
          ? { disabledReason: "not supported yet in v2" }
          : {}),
      };
    }) ?? [];
    result[host] = { binding: setupHost.binding, models };
  }
  for (const host of HOST_IDS) {
    if (result[host].models.length === 0) throw new Error(`canonical setup contract has no ${host} model profiles`);
    const modelById = new Map(result[host].models.map((model) => [model.id, model]));
    for (const [role, assignment] of Object.entries(HOSTS[host].defaults)) {
      const model = modelById.get(assignment.model);
      if (!model) throw new Error(`canonical setup contract has no ${host} default model for ${role}: ${assignment.model}`);
      const modelEfforts = new Set(model.efforts);
      if (assignment.effort && !modelEfforts.has(assignment.effort)) {
        throw new Error(`canonical setup contract has no ${host} default effort for ${role}: ${assignment.effort}`);
      }
    }
  }
  return result;
}

export function workflowCapabilitiesFrom(catalog: CatalogShape): WorkflowCapability[] {
  return (catalog.setupContract?.workflowCapabilities?.capabilities ?? []).map((capability, index) => {
    if (
      typeof capability.codingAgent !== "string"
      || typeof capability.executionPath !== "string"
      || typeof capability.parentModel !== "string"
      || !isWorkflowValidationStatus(capability.validationStatus)
    ) {
      throw new Error(`canonical setup contract has invalid workflow capability at index ${index}`);
    }
    return {
      codingAgent: capability.codingAgent,
      executionPath: capability.executionPath,
      validationStatus: capability.validationStatus,
      parentModel: capability.parentModel,
    };
  });
}

function isWorkflowValidationStatus(value: unknown): value is WorkflowValidationStatus {
  return value === "certified" || value === "experimental" || value === "planned";
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
    orchestration: host === "pi" ? "extension" : "native",
    integration: "standalone",
    usagePolicy: "balanced",
    roles: [...CHILD_ROLE_IDS],
    assignments: structuredClone(HOSTS[host].defaults),
    fallbackModels: emptyFallbackModels(),
  };
}

function emptyFallbackModels(): Record<RoleId, string[]> {
  return { orchestrator: [], implementer: [], reviewer: [], verifier: [] };
}

export function isPrimaryRecommendationId(role: string): role is PrimaryRecommendationId {
  return role === PRIMARY_RECOMMENDATION_ID;
}

export function isChildRoleId(role: string): role is ChildRoleId {
  return CHILD_ROLE_IDS.includes(role as ChildRoleId);
}

export function primaryRecommendation(config: GeneratorConfig) {
  return {
    id: PRIMARY_RECOMMENDATION_ID,
    assignment: config.assignments[PRIMARY_RECOMMENDATION_ID],
  };
}

export function canRenderParentRecommendation(config: GeneratorConfig) {
  return HOSTS[config.host].capabilities.nativeParentRecommendation;
}

function effortDisplayName(effort: string) {
  const labels: Record<string, string> = {
    low: "Low",
    medium: "Medium",
    high: "High",
    xhigh: "XHigh",
    max: "Max",
    ultra: "Ultra",
  };
  return labels[effort] ?? effort;
}

export function parentRecommendationEffortCopy(config: GeneratorConfig, preset: PresetId) {
  const host = HOSTS[config.host];
  const recommendation = primaryRecommendation(config);
  const effortLabel = host.effortLabel?.toLowerCase();
  if (!recommendation.assignment.effort || !effortLabel) {
    return `${PRESETS[preset].label} keeps ${host.label} parent orchestration host-managed.`;
  }
  if (config.host === "pi") {
    return `Use ${modelLabel(recommendation.assignment.model)} with Pi ${effortLabel} ${effortDisplayName(recommendation.assignment.effort)} in the active session.`;
  }
  return `Set ${host.label} ${effortLabel} to ${effortDisplayName(recommendation.assignment.effort)}.`;
}

export function selectedChildRoleIds(config: GeneratorConfig): ChildRoleId[] {
  return [...config.roles];
}

export function setupSpecRoleIds(config: GeneratorConfig): RoleId[] {
  return canRenderParentRecommendation(config) ? selectedChildRoleIds(config) : [PRIMARY_RECOMMENDATION_ID, ...config.roles];
}

export function createPresetSelection(initial: PresetId = "balanced"): PresetSelection {
  return { selected: initial, lastSelected: initial };
}

export function choosePreset(selection: PresetSelection, preset: PresetId): PresetSelection {
  return { ...selection, selected: preset, lastSelected: preset };
}

export function markPresetCustom(selection: PresetSelection): PresetSelection {
  return { ...selection, selected: "custom" };
}

export function changeHost(config: GeneratorConfig, host: HostId): GeneratorConfig {
  return { ...createConfig(host), integration: config.integration, roles: [...config.roles] };
}

export function setOrchestration(config: GeneratorConfig, orchestration: OrchestrationPath): GeneratorConfig {
  if (config.host === "pi" && orchestration !== "extension") throw new Error("Pi uses Pi Subagents");
  if (orchestration === "extension" && config.host !== "pi") throw new Error("Pi is the only extension-backed runtime");
  return { ...config, orchestration };
}

export function workflowValidationStatus(config: GeneratorConfig, capabilities: readonly WorkflowCapability[]): WorkflowValidationStatus {
  const path = config.host === "pi" ? "extension" : config.host === "claude-code" ? "sidecar" : "native";
  return capabilities.find((entry) => entry.codingAgent === config.host && entry.executionPath === path)?.validationStatus ?? "experimental";
}

export function setIntegration(config: GeneratorConfig, integration: SetupIntegration): GeneratorConfig {
  return { ...config, integration };
}

export function applyPreset(config: GeneratorConfig, preset: PresetId, catalog: HostCatalog): GeneratorConfig {
  const assignments = structuredClone(PRESET_ASSIGNMENTS[config.host][preset]);
  const modelById = new Map(catalog[config.host].models.map((model) => [model.id, model]));
  for (const [role, assignment] of Object.entries(assignments)) {
    const model = modelById.get(assignment.model);
    if (!model) throw new Error(`${preset} preset has no ${config.host} model for ${role}: ${assignment.model}`);
    const modelEfforts = new Set(model.efforts);
    if (assignment.effort && !modelEfforts.has(assignment.effort)) {
      throw new Error(`${preset} preset has no ${config.host} effort for ${role}: ${assignment.effort}`);
    }
  }
  return { ...config, usagePolicy: PRESET_USAGE_POLICIES[preset], assignments, fallbackModels: emptyFallbackModels() };
}

export function setRoles(config: GeneratorConfig, roles: readonly string[]): GeneratorConfig {
  const selected = new Set(roles);
  const valid = CHILD_ROLE_IDS.filter((role) => selected.has(role));
  return { ...config, roles: valid.length > 0 ? valid : config.roles };
}

export function removeChildRole(config: GeneratorConfig, role: ChildRoleId): GeneratorConfig {
  const childRoles = selectedChildRoleIds(config);
  if (childRoles.length <= 1 || !childRoles.includes(role)) return config;
  return setRoles(config, childRoles.filter((candidate) => candidate !== role));
}

export function resetRolesToPreset(config: GeneratorConfig, preset: PresetId, catalog: HostCatalog): GeneratorConfig {
  return applyPreset(setRoles(config, CHILD_ROLE_IDS), preset, catalog);
}

function roleAssignmentEquals(left: RoleAssignment, right: RoleAssignment) {
  return left.model === right.model && left.effort === right.effort;
}

function roleListEquals(left: readonly ChildRoleId[], right: readonly ChildRoleId[]) {
  return left.length === right.length && left.every((role, index) => role === right[index]);
}

export function isPresetDirty(config: GeneratorConfig, preset: PresetId, catalog: HostCatalog): boolean {
  const baseline = resetRolesToPreset(config, preset, catalog);
  return (
    config.usagePolicy !== baseline.usagePolicy ||
    !roleListEquals(config.roles, baseline.roles) ||
    ROLE_IDS.some((role) => !roleAssignmentEquals(config.assignments[role], baseline.assignments[role]))
  );
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

export function setFallbackModels(config: GeneratorConfig, role: RoleId, models: readonly string[], catalog: HostCatalog): GeneratorConfig {
  if (config.host !== "pi") throw new Error("fallbacks require Pi Subagents");
  const supported = new Set(catalog.pi.models.map((model) => model.id));
  if (models.some((model) => !supported.has(model) || model === config.assignments[role].model)) throw new Error("unsupported Pi fallback model");
  return { ...config, fallbackModels: { ...config.fallbackModels, [role]: [...new Set(models)] } };
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
  route_default?: { role: string; fallbacks: string[] };
  workflow?: {
    schema_version: 1;
    coding_agent: "pi";
    execution_path: "extension";
    validation_status: "experimental";
    parent_model: "runtime-managed";
    topology: "sequential";
    roles: Record<string, { provider: string; model: string; thinking?: string; fallback_models: string[] }>;
  };
};

function routingRole(config: GeneratorConfig, preferred: ChildRoleId, fallbacks: readonly ChildRoleId[] = []) {
  const selectedRoles = selectedChildRoleIds(config);
  const selected = new Set(selectedRoles);
  for (const role of [preferred, ...fallbacks]) {
    if (selected.has(role)) return role;
  }
  return selectedRoles[0];
}

function splitProviderModel(qualifiedModel: string) {
  const separator = qualifiedModel.indexOf("/");
  if (separator <= 0 || separator === qualifiedModel.length - 1) {
    throw new Error(`model must use provider/model form: ${qualifiedModel}`);
  }
  return {
    provider: qualifiedModel.slice(0, separator),
    model: qualifiedModel.slice(separator + 1),
  };
}

export function setupSpec(config: GeneratorConfig, catalog: HostCatalog): SetupSpecV1 {
  const hasNativeParentRecommendation = canRenderParentRecommendation(config);
  const selected_roles: SetupSpecV1["selected_roles"] = {};
  for (const role of setupSpecRoleIds(config)) {
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
  const codeRole = routingRole(config, "implementer", ["reviewer", "verifier"]);
  const reviewRole = routingRole(config, "reviewer", ["verifier", "implementer"]);
  const verificationRole = routingRole(config, "verifier", ["reviewer", "implementer"]);
  if (config.host === "pi") {
    return {
      schema_version: 1,
      host: catalog.pi.binding,
      integration: config.integration,
      usage_policy: config.usagePolicy,
      selected_roles,
      routes: [
        { work_type: "code", role: codeRole, fallbacks: [] },
        { work_type: "review", role: reviewRole, fallbacks: [] },
        { work_type: "verification", role: verificationRole, fallbacks: [] },
      ],
      workflow: {
        schema_version: 1,
        coding_agent: "pi",
        execution_path: "extension",
        validation_status: "experimental",
        parent_model: "runtime-managed",
        topology: "sequential",
        roles: Object.fromEntries((Object.entries(selected_roles) as [ChildRoleId, SetupSpecV1["selected_roles"][ChildRoleId]][]).map(([role, selection]) => {
          const { provider, model } = splitProviderModel(selection.model);
          return [role, {
            provider,
            model,
            ...(selection.effort ? { thinking: selection.effort } : {}),
            fallback_models: config.fallbackModels[role].map((fallback) => splitProviderModel(fallback).model),
          }];
        })),
      },
    };
  }
  if (!hasNativeParentRecommendation) {
    const reviewFallback = routingRole(config, "reviewer");
    const spec: SetupSpecV1 = {
      schema_version: 1,
      host: catalog[config.host].binding,
      integration: config.integration,
      usage_policy: config.usagePolicy,
      selected_roles,
      routes: [
        { work_type: "planning", role: PRIMARY_RECOMMENDATION_ID, fallbacks: [] },
        { work_type: "code", role: routingRole(config, "implementer"), fallbacks: [] },
        { work_type: "review", role: reviewFallback, fallbacks: [] },
        { work_type: "verification", role: routingRole(config, "verifier", [reviewFallback]), fallbacks: [] },
      ],
      route_default: { role: PRIMARY_RECOMMENDATION_ID, fallbacks: [] },
    };
    return spec;
  }
  return {
    schema_version: 1,
    host: catalog[config.host].binding,
    integration: config.integration,
    usage_policy: config.usagePolicy,
    selected_roles,
    routes: [
      { work_type: "code", role: codeRole, fallbacks: [] },
      { work_type: "review", role: reviewRole, fallbacks: [] },
      { work_type: "verification", role: verificationRole, fallbacks: [] },
    ],
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

function decodeBase64Url(value: string) {
  const padded = value.replaceAll("-", "+").replaceAll("_", "/").padEnd(Math.ceil(value.length / 4) * 4, "=");
  const binary = atob(padded);
  return new TextDecoder().decode(Uint8Array.from(binary, (character) => character.charCodeAt(0)));
}

/**
 * Restores only recipes this version can reproduce exactly. This keeps stale or
 * hand-edited links from silently becoming a different team configuration.
 */
export function configFromRecipe(recipe: string, catalog: HostCatalog, recipePrefix = "sw1_"): GeneratorConfig | null {
  if (!recipe.startsWith(recipePrefix)) return null;
  try {
    const spec = JSON.parse(decodeBase64Url(recipe.slice(recipePrefix.length))) as Partial<SetupSpecV1>;
    const host = HOST_IDS.find((id) => catalog[id].binding === spec.host);
    if (!host || (spec.integration !== "standalone" && spec.integration !== "planr") || typeof spec.usage_policy !== "string") return null;
    if (!spec.selected_roles || typeof spec.selected_roles !== "object") return null;

    const orchestration: OrchestrationPath = host === "pi" ? "extension" : "native";
    const roleEntries = Object.entries(spec.selected_roles);
    const selectedRoles = roleEntries.map(([role]) => role);
    const childRoles = CHILD_ROLE_IDS.filter((role) => selectedRoles.includes(role));
    const restoredBase = { ...createConfig(host), orchestration };
    const expectedRoles = canRenderParentRecommendation(restoredBase) ? childRoles : [PRIMARY_RECOMMENDATION_ID, ...childRoles];
    if (childRoles.length === 0 || selectedRoles.length !== expectedRoles.length || expectedRoles.some((role) => !selectedRoles.includes(role))) return null;

    const config: GeneratorConfig = {
      ...restoredBase,
      orchestration,
      integration: spec.integration,
      usagePolicy: spec.usage_policy,
      roles: childRoles,
    };
    for (const role of expectedRoles) {
      const selection = spec.selected_roles[role];
      const model = catalog[host].models.find((candidate) => candidate.id === selection?.model);
      if (!model || (selection.effort !== undefined && !model.efforts.includes(selection.effort))) return null;
      config.assignments[role] = { model: selection.model, ...(selection.effort ? { effort: selection.effort } : {}) };
    }
    if (host === "pi") {
      const workflow = spec.workflow;
      if (
        !workflow
        || workflow.coding_agent !== "pi"
        || workflow.execution_path !== "extension"
        || !workflow.roles
        || "model_access" in workflow
        || (workflow as { selection_mode?: unknown }).selection_mode === "gateway_auto"
      ) return null;
      for (const role of expectedRoles) {
        const workflowRole = workflow.roles[role];
        if (!workflowRole || !Array.isArray(workflowRole.fallback_models)) return null;
        const fallbacks = workflowRole.fallback_models.map((fallback) =>
          catalog.pi.models.find((candidate) => splitProviderModel(candidate.id).model === fallback)?.id,
        );
        if (fallbacks.some((fallback) => !fallback)) return null;
        config.fallbackModels[role] = fallbacks as string[];
      }
    }
    return setupRecipe(config, catalog, recipePrefix) === recipe ? config : null;
  } catch {
    return null;
  }
}

export function shareUrl(config: GeneratorConfig, catalog: HostCatalog, currentUrl: string, recipePrefix = "sw1_") {
  const url = new URL(currentUrl);
  url.searchParams.set("recipe", setupRecipe(config, catalog, recipePrefix));
  return url.toString();
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
  if (spec.route_default) {
    lines.push("[route_default]", `role = ${tomlString(spec.route_default.role)}`, `fallbacks = ${tomlArray(spec.route_default.fallbacks)}`, "");
  }
  for (const role of setupSpecRoleIds(config)) {
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
  if (config.host === "pi") {
    return "# Pi Subagents is Experimental until credentialed live runtime receipts are available; no applyable recipe is emitted.";
  }
  return `npx switchloom@${SWITCHLOOM_VERSION} apply --recipe ${shellQuote(setupRecipe(config, catalog, recipePrefix))} --repository .`;
}

export type LifecycleCommand = {
  id: string;
  title: string;
  description: string;
  command: string;
};

export function lifecycleCommands(config: GeneratorConfig, catalog: HostCatalog, recipePrefix = "sw1_"): LifecycleCommand[] {
  const recipe = shellQuote(setupRecipe(config, catalog, recipePrefix));
  const host = config.host;
  return [
    {
      id: "install",
      title: "Install",
      description: "Install the Switchloom CLI globally.",
      command: `npm install -g switchloom@${SWITCHLOOM_VERSION}`,
    },
    {
      id: "preview",
      title: "Preview",
      description: "Dry-run the recipe and list files that would be written.",
      command: `switchloom preview --recipe ${recipe} --repository .`,
    },
    {
      id: "apply",
      title: "Apply",
      description: "Write the team setup into this repository.",
      command: `switchloom apply --recipe ${recipe} --repository .`,
    },
    {
      id: "doctor",
      title: "Doctor",
      description: "Check that the host setup looks healthy.",
      command: `switchloom doctor ${host}`,
    },
    {
      id: "update",
      title: "Update",
      description: "Refresh an existing Switchloom install in place.",
      command: "switchloom update --repository .",
    },
    {
      id: "status",
      title: "Status",
      description: "Show the current repository install state.",
      command: "switchloom status --repository .",
    },
    {
      id: "rollback",
      title: "Rollback",
      description: "Restore the previous Switchloom snapshot.",
      command: "switchloom rollback --repository .",
    },
    {
      id: "uninstall",
      title: "Uninstall",
      description: "Remove Switchloom files from the repository.",
      command: "switchloom uninstall --repository .",
    },
  ];
}

export function setupSummary(config: GeneratorConfig) {
  const summaryRoles = setupSpecRoleIds(config);
  return [
    `${HOSTS[config.host].label} ${config.integration === "planr" ? "with Planr" : "standalone"} team`,
    ...(canRenderParentRecommendation(config)
      ? [`${config.roles.length} generated child ${config.roles.length === 1 ? "role" : "roles"}`]
      : [`${summaryRoles.length} focused ${summaryRoles.length === 1 ? "role" : "roles"}`]),
    ...(canRenderParentRecommendation(config)
      ? [`Host-managed parent: ${ROLES.orchestrator.label}${config.assignments.orchestrator.effort ? ` (${config.assignments.orchestrator.effort})` : ""}`]
      : []),
    ...summaryRoles.map((role) => {
      const value = config.assignments[role];
      return `${ROLES[role].label}: ${value.model}${value.effort ? ` · ${value.effort}` : ""}`;
    }),
  ].join("\n");
}
