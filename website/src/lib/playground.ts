import type { Workflow, WorkflowOptions } from './generator.ts';
export const PROMPT_LIMIT = 4000;
export type Profile = { model: string; effort: string; capabilities: string[] };
export type Routing = { mode: 'jev' } | { mode: 'assigned'; capability: string };
export type RoutingInput = { request: { task: string; context: string; routing: Routing }; profiles: Record<string, Profile> };
export type Decision = {
  assignment: { owner: string; capability: string; model: string; effort: string; instructions: string } | null;
  reason: 'explicit_capability' | 'jev' | 'jev_owner' | 'suggested' | 'uncertain' | 'missing_context' | 'explicit_capability_required';
  confidence: number | null; probabilities: Record<string, number>; judge_model: string | null;
  usage: { input_tokens: number; output_tokens: number } | null;
  context_missing: number | null; act_threshold: number | null;
  owner_probability: number | null;
};
export type RouteResult = { decision: Decision; elapsedMs: number; remaining: number | null };
export function profilesFor(workflow: Workflow, options: WorkflowOptions): Record<string, Profile> {
  return Object.fromEntries(workflow.slots.filter(s => options.models[s.id].enabled).map(s => [s.id, {
    model: options.models[s.id].model, effort: options.models[s.id].effort,
    capabilities: workflow.capabilities.filter(c => options.owners[c.id] === s.id).map(c => c.id),
  }]).filter(([, p]) => (p as Profile).capabilities.length));
}
export const decisionLabel: Record<Decision['reason'], string> = {
  explicit_capability: 'Assigned', jev: 'Assigned', jev_owner: 'Assigned · same owner', suggested: 'Suggestion · not dispatched',
  uncertain: 'Uncertain · no assignment', missing_context: 'More context needed', explicit_capability_required: 'Select a capability',
};
