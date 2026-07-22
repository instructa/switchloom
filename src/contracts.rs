//! Stable serialized contracts shared by routing, host adapters, and integrations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Integration {
    Standalone,
    Planr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupSpecV1 {
    pub schema_version: u32,
    pub host: String,
    pub integration: Integration,
    pub usage_policy: String,
    pub selected_roles: BTreeMap<String, SetupRoleSelection>,
    pub routes: Vec<SetupRouteMapping>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_default: Option<SetupDefaultRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupRoleSelection {
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn: Option<SetupSpawnPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupSpawnPolicy {
    pub agent_type: String,
    pub task_name: String,
    pub fork_turns: ForkPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupRouteMapping {
    pub work_type: String,
    pub role: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupDefaultRoute {
    pub role: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvaluationEvidence {
    #[serde(default)]
    pub evaluation_ids: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexV2RuntimeEvidence {
    pub(crate) schema_version: u32,
    pub(crate) evidence_id: String,
    pub(crate) observed_at: String,
    pub(crate) installed_version: CodexInstalledVersionEvidence,
    pub(crate) runtime_class: RuntimeClass,
    pub(crate) backend_selection_owner: String,
    pub(crate) switchloom_ownership: Vec<String>,
    pub(crate) codex_ownership: Vec<String>,
    pub(crate) trust_and_discovery: CodexTrustDiscoveryEvidence,
    pub(crate) parallelism: CodexParallelismEvidence,
    pub(crate) role_precedence: Vec<String>,
    pub(crate) shared_filesystem: bool,
    pub(crate) delegation_modes: DelegationModesV1,
    pub(crate) claim_provenance: BTreeMap<String, Vec<CodexClaimProvenance>>,
    pub(crate) negative_contracts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexInstalledVersionEvidence {
    pub(crate) command: String,
    pub(crate) stdout: String,
    pub(crate) stdout_sha256: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexTrustDiscoveryEvidence {
    pub(crate) trust_boundary: String,
    pub(crate) discovery_behavior: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexParallelismEvidence {
    pub(crate) max_parallel_children: u32,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexClaimProvenance {
    pub(crate) kind: String,
    pub(crate) source: String,
    pub(crate) observed_at: String,
    pub(crate) codex_version: String,
    pub(crate) observed_value: Value,
    pub(crate) required_raw_fragments: Vec<String>,
    #[serde(default)]
    pub(crate) source_url: Option<String>,
    #[serde(default)]
    pub(crate) source_path: Option<String>,
    #[serde(default)]
    pub(crate) raw_output: Option<String>,
    #[serde(default)]
    pub(crate) raw_output_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DispatchEvidenceV1 {
    pub schema_version: u32,
    pub package_digest: String,
    pub host_version: String,
    pub requested_dispatch: RequestedDispatchEvidence,
    pub child_identity: ChildIdentityEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_effort: Option<String>,
    pub nonce: String,
    pub raw_evidence_refs: Vec<String>,
    pub verdict: GuaranteeLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RequestedDispatchEvidence {
    pub semantic_role: String,
    pub profile: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_turns: Option<ForkPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChildIdentityEvidence {
    pub host: String,
    pub role: String,
    pub agent_role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DispatchEvidenceContractV1 {
    pub schema_version: u32,
    pub required_verdicts: Vec<GuaranteeLevel>,
    pub receipt_schema: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeClass {
    NativeSubagent,
    ExternalRunner,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum GuaranteeLevel {
    Deterministic,
    Advisory,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGuarantee {
    pub level: GuaranteeLevel,
    pub reason: String,
    pub evidence_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostCapabilityV1 {
    pub schema_version: u32,
    pub host: String,
    pub host_version_constraints: HostVersionConstraints,
    pub runtime_class: RuntimeClass,
    pub runtime_behavior: RuntimeBehaviorV1,
    pub discovery_artifacts: Vec<String>,
    pub dispatch_fields: Vec<String>,
    pub model_control: ControlCapability,
    pub effort_control: ControlCapability,
    pub context_semantics: ContextSemantics,
    pub nesting: NestingCapability,
    pub parallelism: ParallelismCapability,
    pub observability: ObservabilityCapability,
    pub guarantees: BTreeMap<String, CapabilityGuarantee>,
    pub known_limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostVersionConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<String>,
    pub evidence_max_age_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlCapability {
    pub level: GuaranteeLevel,
    pub field: String,
    pub evidence_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContextSemantics {
    pub supports_fork_none: bool,
    pub supports_fork_all: bool,
    pub requires_bounded_context_for_overrides: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NestingCapability {
    pub max_depth: u32,
    pub level: GuaranteeLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ParallelismCapability {
    pub max_parallel_children: u32,
    pub level: GuaranteeLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservabilityCapability {
    pub requested_dispatch: GuaranteeLevel,
    pub effective_identity: GuaranteeLevel,
    pub effective_model: GuaranteeLevel,
    pub raw_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBehaviorV1 {
    pub capability_version: String,
    pub installed_host_version_source: String,
    pub backend_selection_source: String,
    pub trust_boundary: String,
    pub discovery_behavior: String,
    pub role_precedence: Vec<String>,
    pub shared_filesystem: bool,
    pub delegation_modes: DelegationModesV1,
    pub source_references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DelegationModesV1 {
    pub explicit_agent_type_dispatch: bool,
    pub ultra_auto_delegation: bool,
    pub automatic_delegation_requires_ultra: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostAdapterV1 {
    pub schema_version: u32,
    pub adapter_id: String,
    pub adapter_version: String,
    pub runtime_class: RuntimeClass,
    pub accepts_intent_schema: String,
    pub emitted_artifact_modes: Vec<String>,
    pub dispatch_recipe: DispatchRecipeV1,
    pub lifecycle_owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DispatchRecipeV1 {
    pub invocation: String,
    pub required_fields: Vec<String>,
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdapterContractV1 {
    pub schema_version: u32,
    pub routing_intent: RoutingIntentV1,
    pub capability: HostCapabilityV1,
    pub adapter: HostAdapterV1,
    pub dispatch_evidence: DispatchEvidenceContractV1,
    pub planr_handoff: PlanrHandoffV1,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HostBinding {
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) host: String,
    pub(crate) runtime_class: RuntimeClass,
    pub(crate) default_role: Option<String>,
    #[serde(default)]
    pub(crate) capability_evidence: Vec<String>,
    #[serde(default)]
    pub(crate) known_limitations: Vec<String>,
    pub(crate) capabilities: BindingCapabilities,
    pub(crate) profiles: BTreeMap<String, BindingProfile>,
    #[serde(default)]
    pub(crate) routes: Vec<BindingRoute>,
    pub(crate) verification: BindingVerification,
    #[serde(default)]
    pub(crate) artifacts: Vec<BindingArtifact>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BindingCapabilities {
    pub(crate) model_override: bool,
    pub(crate) effort_override: bool,
    pub(crate) fork_none: bool,
    pub(crate) fork_all: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BindingProfile {
    pub(crate) profile: String,
    pub(crate) client: String,
    pub(crate) model: String,
    pub(crate) agent_type: Option<String>,
    pub(crate) effort: Option<String>,
    pub(crate) cost_tier: Option<String>,
    pub(crate) fork_turns: Option<ForkPolicy>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BindingRoute {
    pub(crate) work_type: String,
    pub(crate) role: String,
    #[serde(default)]
    pub(crate) fallback_roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BindingVerification {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) max_age_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BindingArtifact {
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) content: String,
}

#[derive(Debug)]
pub(crate) struct CompiledHostAdapter {
    pub(crate) requirements: Vec<HostRequirement>,
    pub(crate) profiles: BTreeMap<String, Profile>,
    pub(crate) routes: Vec<Route>,
    pub(crate) route_default: Option<DefaultRoute>,
    pub(crate) artifacts: Vec<SourceArtifact>,
    pub(crate) adapter_contract: AdapterContractV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeReport {
    pub host: String,
    pub command: Option<String>,
    pub available: bool,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    pub authentication: String,
    pub limitation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ProbeDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub repair: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SetupModelOption {
    pub(crate) id: &'static str,
    pub(crate) efforts: &'static [&'static str],
    pub(crate) tier: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanrHandoffV1 {
    pub schema_version: u32,
    pub switchloom_package: String,
    pub semantic_role_contract: String,
    pub required_consumer_behavior: Vec<String>,
    pub forbidden_duplicate_ownership: Vec<String>,
    pub certification_report_reference: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PolicySource {
    pub policy_id: String,
    pub host: String,
    pub policy_version: String,
    pub binding_id: String,
    pub binding_version: String,
    pub generated_at: String,
    pub requirements: Vec<HostRequirement>,
    pub profiles: BTreeMap<String, Profile>,
    pub routes: Vec<Route>,
    pub route_default: Option<DefaultRoute>,
    pub artifacts: Vec<SourceArtifact>,
    pub evidence: EvaluationEvidence,
    pub adapter_contract: AdapterContractV1,
    pub policy: PolicyContract,
    #[serde(skip)]
    pub(crate) policy_toml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostRequirement {
    pub host: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub client: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_tier: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_turns: Option<ForkPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ForkPolicy {
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RouteSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Route {
    #[serde(rename = "match")]
    pub selector: RouteSelector,
    pub profile: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DefaultRoute {
    pub profile: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifact {
    pub path: String,
    pub media_type: String,
    pub mode: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoutingIntentV1 {
    pub schema_version: u32,
    pub integration: Integration,
    pub semantic_roles: Vec<String>,
    pub role_requests: Vec<RoutingRoleIntentV1>,
    pub required_guarantees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoutingRoleIntentV1 {
    pub semantic_role: String,
    pub requested_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_effort: Option<String>,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoutingBundleV1 {
    pub schema_version: u32,
    pub bundle_id: String,
    pub policy_id: String,
    pub policy_version: String,
    pub generated_at: String,
    pub source: BundleSource,
    pub policy: PolicyContract,
    pub requirements: Vec<HostRequirement>,
    pub profiles: BTreeMap<String, Profile>,
    pub routes: Vec<Route>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_default: Option<DefaultRoute>,
    pub artifacts: Vec<BundleArtifact>,
    pub evidence: EvaluationEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_contract: Option<AdapterContractV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BundleSource {
    pub package: String,
    pub package_version: String,
    pub integration: Integration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BundleArtifact {
    pub path: String,
    pub media_type: String,
    pub mode: String,
    pub content: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyContract {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub usage: UsageLimits,
    pub transitions: TransitionPolicy,
    pub materiality: MaterialityPolicy,
    pub execution: ExecutionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageLimits {
    pub max_active_agents: u32,
    pub max_parallel_readers: u32,
    pub max_parallel_writers: u32,
    pub max_depth: u32,
    pub max_attempts: u32,
    pub max_wall_time_seconds: u32,
    pub max_tool_calls: u32,
    pub review_reserve_percent: u32,
    pub budget_exhaustion: String,
    pub metering: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransitionPolicy {
    pub retry: RetryPolicy,
    pub availability_fallback: AvailabilityFallbackPolicy,
    pub quality_escalation: QualityEscalationPolicy,
    pub quota_downgrade: QuotaDowngradePolicy,
    pub safety_stop: SafetyStopPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetryPolicy {
    pub max_same_route_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityFallbackPolicy {
    pub max_fallbacks: u32,
    pub require_same_capability_class: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QualityEscalationPolicy {
    pub max_escalations: u32,
    pub require_verification_evidence: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuotaDowngradePolicy {
    pub enabled: bool,
    pub max_downgrades: u32,
    pub noncritical_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SafetyStopPolicy {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MaterialityPolicy {
    pub changed_files_threshold: u32,
    pub changed_lines_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPolicy {
    pub max_read_scope_entries: u32,
    pub max_write_scope_entries: u32,
    pub roles: BTreeMap<String, ExecutionRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRole {
    #[serde(default)]
    pub tools: Vec<String>,
    pub filesystem: FilesystemPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FilesystemPolicy {
    #[serde(default)]
    pub read_roots: Vec<String>,
    #[serde(default)]
    pub write_roots: Vec<String>,
    pub allow_overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleInspection {
    pub schema_version: u32,
    pub bundle_id: String,
    pub policy_id: String,
    pub integration: Integration,
    pub profile_count: usize,
    pub route_count: usize,
    pub artifact_count: usize,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicySummary {
    pub policy_id: String,
    pub host: String,
    pub policy_version: String,
    pub binding_id: String,
    pub binding_version: String,
    pub profile_count: usize,
    pub artifact_count: usize,
    pub evidence_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationReport {
    pub schema_version: u32,
    pub suite_id: String,
    pub suite_version: String,
    pub suite_sha256: String,
    pub policy_id: String,
    pub host: String,
    pub bundle_sha256: String,
    pub scenario_count: usize,
    pub offline_reproducible: bool,
    pub live_evidence: Option<serde_json::Value>,
    pub status: String,
    pub recommended: bool,
}
