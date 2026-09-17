type Catalog = typeof import("../../data/catalog.json");
export type Workflow = Catalog;
export type ModelSettings = { model: string; effort: string };
export type WorkflowOptions = {
  jev: boolean;
  models: Record<string, ModelSettings & { enabled: boolean }>;
  owners: Record<string, string | null>;
};
export const workflowFromCatalog = (catalog: Catalog): Workflow => catalog;

export function defaultOptions(workflow: Workflow): WorkflowOptions {
  return {
    jev: true,
    models: Object.fromEntries(workflow.slots.map(({ id, model, effort }) => [id, { model, effort, enabled: true }])),
    owners: Object.fromEntries(workflow.capabilities.map(cap => [cap.id, cap.default_owner])),
  };
}

export function toggleModel(workflow: Workflow, options: WorkflowOptions, id: string, enabled: boolean): WorkflowOptions {
  const slot = workflow.slots.find(slot => slot.id === id);
  if (!slot || options.models[id].enabled === enabled) return options;
  const owners = { ...options.owners };
  for (const cap of workflow.capabilities) {
    if (owners[cap.id] === id) owners[cap.id] = null;
    if (enabled && cap.default_owner === id) owners[cap.id] = id;
  }
  return { ...options, owners, models: { ...options.models, [id]: enabled
    ? { model: slot.model, effort: slot.effort, enabled: true }
    : { ...options.models[id], enabled: false } } };
}

export function moveCapability(workflow: Workflow, options: WorkflowOptions, capability: string, owner: string | null): WorkflowOptions {
  if (!workflow.capabilities.some(cap => cap.id === capability) || (owner !== null && !options.models[owner]?.enabled)) return options;
  return { ...options, owners: { ...options.owners, [capability]: owner } };
}

export function selectedModels(workflow: Workflow, options: WorkflowOptions) {
  return workflow.slots.filter(slot => options.models[slot.id].enabled).map(slot => ({
    id: slot.id, ...options.models[slot.id],
    capabilities: workflow.capabilities.filter(cap => options.owners[cap.id] === slot.id),
  }));
}
export function reasoningEfforts(workflow: Workflow, model: string): string[] {
  return workflow.models.find(entry => entry.id === model)?.efforts ?? workflow.reasoning_efforts;
}
export function modelLabel(workflow: Workflow, model: string): string {
  return workflow.models.find(entry => entry.id === model)?.label ?? (model || "Custom model");
}
export function workflowError(workflow: Workflow, options: WorkflowOptions): string | null {
  const selected = selectedModels(workflow, options);
  if (!selected.length) return "Enable at least one model.";
  if (!selected.some(slot => slot.capabilities.length)) return "Assign at least one capability.";
  const supported = new Map(workflow.models.map(model => [model.id, new Set(model.efforts)]));
  const unlisted = new Set(workflow.reasoning_efforts);
  for (const slot of selected) {
    if (slot.model.length > 128 || !/^gpt-[a-z0-9][a-z0-9._-]*$/.test(slot.model)) return "Enter a valid GPT model ID.";
    if (!(supported.get(slot.model) ?? unlisted).has(slot.effort)) return `${slot.model}: select a supported effort.`;
  }
  return null;
}

export function buildWorkflowPrompt(workflow: Workflow, options: WorkflowOptions): string {
  const error = workflowError(workflow, options);
  if (error) throw new Error(error);
  // Empty enabled cards require no task until a capability is assigned.
  const slots = selectedModels(workflow, options).filter(slot => slot.capabilities.length);
  const profiles = Object.fromEntries(slots.map(slot => [slot.id, { model: slot.model, effort: slot.effort, capabilities: slot.capabilities.map(cap => cap.id) }]));
  const coordinator = options.owners.coordination ?? options.owners.planning ?? slots[0].id;
  return [
    `Run this task with ${slots.length} persistent Codex threads. Model slots and capability ownership:\n${JSON.stringify(profiles)}`,
    ...slots.map(slot => `${slot.id} (${slot.model}, ${slot.effort}):\n${slot.capabilities.map(cap => `- ${cap.label}: ${cap.instructions}`).join("\n")}`),
    "Reuse suitable project threads, including this thread when its model and assigned capabilities match. Resolve the project with list_projects and existing threads with list_threads. Create only missing threads with create_thread using the specified model and thinking, target type project and environment type local. This prompt authorizes creation and cross-thread messaging. Use the existing checkout directly; never create or switch to a worktree. Report missing capabilities or unavailable models without substituting them.",
    `Bootstrap once. New threads receive their assignments and project context, then end their setup turn until given work. Record pending creation IDs; never create replacements while setup is pending. Give each thread the complete slot/thread/host map, selected capabilities and these handoff rules in its first actionable assignment. Start coordination in ${coordinator}; if this thread owns no work, hand off and end its turn. No extra manager or replacement subagents.`,
    "Follow AGENTS.md, preserve unrelated changes and coordinate exclusive file and shared-tool ownership. Load only skills relevant to the assigned step. Do not paste whole skill bodies or conversation histories into handoffs. Split work into bounded steps; disabled capabilities add no workflow duties. Escalate a missing required capability instead of silently assigning it.",
    options.jev
      ? "Use $switchloom for handoffs with request.jev=true. Include each configured slot's thread_id, host_id, model, effort and capabilities in threads. Set request.capability for explicit assignments and known continuations; only an unclear next capability needs a TypeSafe/Jev judgment. Report router failures or abstention; do not silently choose a model. Keep the selected model fixed throughout its tool loop."
      : "Vanilla mode: route directly to the capability owner using the map above, without TypeSafe, Jev or an API key. Choose the next bounded step from the agreed plan; clarify ambiguous ownership. Keep the selected model fixed throughout its tool loop.",
    "Dispatch only to idle recipients without outstanding assignments; preserve pending work if busy. Use send_message_to_thread with the target's model and thinking, and include the return thread ID and host in each assignment. After handing off, end your turn; the recipient sends a substantive result back to resume work. Mark replies 'Result for:' with the objective and sender thread ID. Consume results as continuations; do not acknowledge or echo them. No polling, waiting loops or progress-only exchanges.",
    "Handoffs contain only objective, constraints, relevant files/diff, evidence and requested action. Reuse context and send deltas. Avoid duplicate investigation and repeated checks. Additional agents require explicit authorization and independent scope. Finish when the selected acceptance checks and actionable findings are resolved. Report completed work, evidence and remaining limitations. Preserve capability ownership, model settings, thread/host IDs, pending assignments and next action across compaction; recover them before creating replacements.",
    "Apply this workflow to the task described in this conversation. If no task has been given, ask for it before creating threads.",
  ].join("\n\n");
}
