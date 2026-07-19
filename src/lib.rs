//! Standalone model-routing policy compiler.
//!
//! This package is the sole owner of named usage policies, model names, host
//! bindings, routing topologies, and generated host artifacts. It emits the
//! provider-neutral `RoutingBundle v1` contract consumed by supported host and optional integration adapters.

use anyhow::{Context, Result, bail};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
const GENERATED_AT: &str = "2026-07-16T00:00:00Z";
const GENERATED_AT_UNIX: i64 = 1_784_160_000;
pub const SETUP_CONFIG_PATH: &str = ".switchloom/config.toml";
pub const SETUP_RECIPE_PREFIX: &str = "sw1_";
const CODEX_CONFIG_PATH: &str = ".codex/config.toml";
const MAX_SETUP_RECIPE_BYTES: usize = 65_536;
const MAX_SETUP_RECIPE_ENCODED_BYTES: usize = encoded_base64url_len(MAX_SETUP_RECIPE_BYTES);
const EVALUATION_SUITE: &str = include_str!("../evaluations/preset-suite-v1.toml");
const MANIFEST_PATH: &str = ".model-routing/manifest.json";
const TRANSACTION_JOURNAL: &str = "journal.json";
thread_local! {
    static TRANSACTION_FAIL_AFTER_WRITES: Cell<usize> = const { Cell::new(0) };
    static TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER: Cell<usize> = const { Cell::new(0) };
    static TRANSACTION_RETURN_JOURNAL_ERROR_AFTER: Cell<usize> = const { Cell::new(0) };
    static TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER: Cell<usize> = const { Cell::new(0) };
    static TRANSACTION_FAIL_RESTORE: Cell<bool> = const { Cell::new(false) };
}

const POLICIES: [(&str, &str); 4] = [
    ("balanced", include_str!("../usage-policies/balanced.toml")),
    (
        "low-usage",
        include_str!("../usage-policies/low-usage.toml"),
    ),
    (
        "max-quality",
        include_str!("../usage-policies/max-quality.toml"),
    ),
    (
        "read-only-audit",
        include_str!("../usage-policies/read-only-audit.toml"),
    ),
];

const BINDINGS: [(&str, &str); 5] = [
    (
        "codex-openai",
        include_str!("../host-bindings/codex-openai.toml"),
    ),
    (
        "cursor-openai",
        include_str!("../host-bindings/cursor-openai.toml"),
    ),
    (
        "cursor-fable-grok",
        include_str!("../host-bindings/cursor-fable-grok.toml"),
    ),
    (
        "claude-native",
        include_str!("../host-bindings/claude-native.toml"),
    ),
    (
        "mixed-host",
        include_str!("../host-bindings/mixed-host.toml"),
    ),
];

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
    pub policy: PolicyContract,
    #[serde(skip)]
    policy_toml: String,
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
pub struct EvaluationEvidence {
    #[serde(default)]
    pub evaluation_ids: Vec<String>,
    pub status: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleReport {
    pub action: String,
    pub bundle_id: Option<String>,
    pub repository: String,
    pub artifacts: Vec<LifecycleArtifactReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleArtifactReport {
    pub path: String,
    pub mode: String,
    pub status: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<String>,
}

pub struct PreparedSetupLifecycle {
    bundle: RoutingBundleV1,
    bundle_input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManagedManifest {
    schema_version: u32,
    bundle_id: String,
    bundle_sha256: String,
    artifacts: Vec<ManagedArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous: Option<ManagedSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManagedArtifact {
    path: String,
    sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManagedSnapshot {
    bundle_id: String,
    bundle_sha256: String,
    artifacts: Vec<ManagedArtifact>,
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

#[derive(Debug, Deserialize)]
struct HostBinding {
    id: String,
    version: String,
    host: String,
    default_role: Option<String>,
    capabilities: BindingCapabilities,
    profiles: BTreeMap<String, BindingProfile>,
    #[serde(default)]
    routes: Vec<BindingRoute>,
    verification: BindingVerification,
    #[serde(default)]
    artifacts: Vec<BindingArtifact>,
}

#[derive(Debug, Deserialize)]
struct BindingCapabilities {
    model_override: bool,
    effort_override: bool,
    fork_none: bool,
    fork_all: bool,
}

#[derive(Debug, Deserialize)]
struct BindingProfile {
    profile: String,
    client: String,
    model: String,
    agent_type: Option<String>,
    effort: Option<String>,
    cost_tier: Option<String>,
    fork_turns: Option<ForkPolicy>,
}

#[derive(Debug, Deserialize)]
struct BindingRoute {
    work_type: String,
    role: String,
    #[serde(default)]
    fallback_roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BindingVerification {
    id: String,
}

#[derive(Debug, Deserialize)]
struct BindingArtifact {
    path: String,
    kind: String,
    content: String,
}

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

pub fn setup_spec_for_policy(
    policy: &str,
    host: &str,
    integration: Integration,
) -> Result<SetupSpecV1> {
    let binding = binding_for_selector(host)?;
    let selected_roles = binding
        .profiles
        .iter()
        .map(|(role, profile)| {
            (
                role.clone(),
                SetupRoleSelection {
                    model: profile.model.clone(),
                    effort: profile.effort.clone(),
                    spawn: setup_spawn_policy_for_binding_role(
                        setup_runtime_host(&binding),
                        role,
                        profile,
                    ),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let routes = binding
        .routes
        .iter()
        .map(|route| SetupRouteMapping {
            work_type: route.work_type.clone(),
            role: route.role.clone(),
            fallbacks: route.fallback_roles.clone(),
        })
        .collect();
    let route_default = binding.default_role.clone().map(|role| SetupDefaultRoute {
        role,
        fallbacks: Vec::new(),
    });
    let spec = SetupSpecV1 {
        schema_version: 1,
        host: binding.id.clone(),
        integration,
        usage_policy: policy.to_string(),
        selected_roles,
        routes,
        route_default,
    };
    validate_setup_spec(&spec)?;
    Ok(spec)
}

pub fn validate_setup_spec(spec: &SetupSpecV1) -> Result<()> {
    if spec.schema_version != 1 {
        bail!("unsupported setup schema_version {}", spec.schema_version);
    }
    if spec.usage_policy.trim().is_empty() {
        bail!("setup usage_policy must not be blank");
    }
    if spec.selected_roles.is_empty() {
        bail!("setup selected_roles must not be empty");
    }
    let binding = binding_for_selector(&spec.host)?;
    let canonical_host = setup_runtime_host(&binding);
    let model_catalog = setup_model_catalog(canonical_host);
    for (role, selection) in &spec.selected_roles {
        validate_setup_identifier("role", role)?;
        if selection.model.trim().is_empty() {
            bail!("setup role `{role}` model must not be blank");
        }
        let matches_binding = selection_matches_binding_profile(role, selection, &binding);
        if !matches_binding {
            validate_model_effort(canonical_host, role, selection, &model_catalog)?;
        }
        validate_setup_spawn_policy(canonical_host, role, selection, matches_binding)?;
        reject_setup_secret_like("role", role)?;
        reject_setup_secret_like("model", &selection.model)?;
        if let Some(effort) = &selection.effort {
            reject_setup_secret_like("effort", effort)?;
        }
        if let Some(spawn) = &selection.spawn {
            reject_setup_secret_like("agent_type", &spawn.agent_type)?;
            reject_setup_secret_like("task_name", &spawn.task_name)?;
        }
    }
    validate_setup_identity_collisions(spec, canonical_host, &binding)?;
    if spec.routes.is_empty() && spec.route_default.is_none() {
        bail!("setup must declare routes or route_default");
    }
    for route in &spec.routes {
        validate_setup_identifier("work_type", &route.work_type)?;
        validate_setup_route_role(&spec.selected_roles, &route.role)?;
        for fallback in &route.fallbacks {
            validate_setup_route_role(&spec.selected_roles, fallback)?;
        }
    }
    if let Some(default) = &spec.route_default {
        validate_setup_route_role(&spec.selected_roles, &default.role)?;
        for fallback in &default.fallbacks {
            validate_setup_route_role(&spec.selected_roles, fallback)?;
        }
    }
    let _ = show_policy(&spec.usage_policy, &binding.id)?;
    Ok(())
}

pub fn setup_spec_from_json(input: &str) -> Result<SetupSpecV1> {
    let spec: SetupSpecV1 =
        serde_json::from_str(input).context("setup spec is not valid SetupSpecV1 JSON")?;
    validate_setup_spec(&spec)?;
    Ok(spec)
}

pub fn setup_spec_from_toml(input: &str) -> Result<SetupSpecV1> {
    let spec: SetupSpecV1 =
        toml::from_str(input).context("setup spec is not valid SetupSpecV1 TOML")?;
    validate_setup_spec(&spec)?;
    Ok(spec)
}

pub fn setup_spec_to_canonical_json(spec: &SetupSpecV1) -> Result<String> {
    validate_setup_spec(spec)?;
    let mut json = serde_json::to_string_pretty(spec)?;
    json.push('\n');
    Ok(json)
}

pub fn setup_spec_to_canonical_toml(spec: &SetupSpecV1) -> Result<String> {
    validate_setup_spec(spec)?;
    let mut toml = toml::to_string_pretty(spec)?;
    if !toml.ends_with('\n') {
        toml.push('\n');
    }
    Ok(toml)
}

pub fn setup_spec_to_recipe(spec: &SetupSpecV1) -> Result<String> {
    let json = setup_spec_to_canonical_json(spec)?;
    if json.len() > MAX_SETUP_RECIPE_BYTES {
        bail!("setup recipe exceeds {MAX_SETUP_RECIPE_BYTES} bytes");
    }
    Ok(format!(
        "{SETUP_RECIPE_PREFIX}{}",
        encode_base64url(json.as_bytes())
    ))
}

pub fn setup_spec_from_recipe(recipe: &str) -> Result<SetupSpecV1> {
    let payload = recipe
        .strip_prefix(SETUP_RECIPE_PREFIX)
        .ok_or_else(|| anyhow::anyhow!("setup recipe must start with `{SETUP_RECIPE_PREFIX}`"))?;
    if payload.is_empty() {
        bail!("setup recipe payload must not be empty");
    }
    validate_base64url_payload_len(payload)?;
    let decoded = decode_base64url(payload)?;
    if decoded.len() > MAX_SETUP_RECIPE_BYTES {
        bail!("setup recipe exceeds {MAX_SETUP_RECIPE_BYTES} bytes");
    }
    let json = String::from_utf8(decoded).context("setup recipe payload is not UTF-8")?;
    setup_spec_from_json(&json)
}

pub fn compile_setup_spec(spec: &SetupSpecV1) -> Result<RoutingBundleV1> {
    let source = source_from_setup_spec(spec)?;
    let bundle = compile_source(source, spec.integration)?;
    validate_bundle(&bundle)?;
    Ok(bundle)
}

pub fn compile_setup_json(input: &str) -> Result<String> {
    let spec = setup_spec_from_json(input)?;
    let mut json = serde_json::to_string_pretty(&compile_setup_spec(&spec)?)?;
    json.push('\n');
    Ok(json)
}

pub fn preview_setup_config_file(repository: &Path, config_file: &Path) -> Result<LifecycleReport> {
    let spec = read_setup_config_file(config_file)?;
    preview_setup(repository, &spec)
}

pub fn preview_setup_recipe(repository: &Path, recipe: &str) -> Result<LifecycleReport> {
    let spec = setup_spec_from_recipe(recipe)?;
    preview_setup(repository, &spec)
}

pub fn preview_saved_setup(repository: &Path) -> Result<LifecycleReport> {
    let spec = read_saved_setup_config(repository)?;
    preview_setup(repository, &spec)
}

pub fn apply_setup_config_file(repository: &Path, config_file: &Path) -> Result<LifecycleReport> {
    let spec = read_setup_config_file(config_file)?;
    apply_setup(repository, &spec)
}

pub fn apply_setup_recipe(repository: &Path, recipe: &str) -> Result<LifecycleReport> {
    let spec = setup_spec_from_recipe(recipe)?;
    apply_setup(repository, &spec)
}

pub fn apply_saved_setup(repository: &Path) -> Result<LifecycleReport> {
    let spec = read_saved_setup_config(repository)?;
    apply_setup(repository, &spec)
}

pub fn update_setup_config_file(repository: &Path, config_file: &Path) -> Result<LifecycleReport> {
    let spec = read_setup_config_file(config_file)?;
    update_setup(repository, &spec)
}

pub fn update_setup_recipe(repository: &Path, recipe: &str) -> Result<LifecycleReport> {
    let spec = setup_spec_from_recipe(recipe)?;
    update_setup(repository, &spec)
}

pub fn update_saved_setup(repository: &Path) -> Result<LifecycleReport> {
    let spec = read_saved_setup_config(repository)?;
    update_setup(repository, &spec)
}

pub fn prepare_setup_config_file(config_file: &Path) -> Result<PreparedSetupLifecycle> {
    prepare_setup_lifecycle(&read_setup_config_file(config_file)?)
}

pub fn prepare_setup_recipe(recipe: &str) -> Result<PreparedSetupLifecycle> {
    prepare_setup_lifecycle(&setup_spec_from_recipe(recipe)?)
}

pub fn prepare_saved_setup(repository: &Path) -> Result<PreparedSetupLifecycle> {
    prepare_setup_lifecycle(&read_saved_setup_config(repository)?)
}

pub fn preview_prepared_setup(
    repository: &Path,
    prepared: &PreparedSetupLifecycle,
) -> Result<LifecycleReport> {
    preview_bundle(repository, &prepared.bundle)
}

pub fn apply_prepared_setup(
    repository: &Path,
    prepared: &PreparedSetupLifecycle,
    confirmed_preview: &LifecycleReport,
) -> Result<LifecycleReport> {
    let current_preview = preview_prepared_setup(repository, prepared)?;
    if !same_lifecycle_plan(&current_preview, confirmed_preview) {
        bail!("repository state changed after preview; rerun preview/apply and confirm again");
    }
    apply_bundle_json(
        Path::new(&confirmed_preview.repository),
        &prepared.bundle,
        &prepared.bundle_input,
    )
}

pub fn setup_contract_catalog_value() -> Result<Value> {
    let hosts = ["codex", "claude-code", "cursor", "mixed-host"]
        .into_iter()
        .map(|host| {
            let binding = binding_for_selector(host)?;
            let runtime_host = setup_runtime_host(&binding);
            Ok(json!({
                "id": host,
                "binding": binding.id,
                "runtimeHost": runtime_host,
                "supportsPlanrIntegration": true,
                "models": setup_model_catalog(runtime_host).into_iter().map(|option| json!({
                    "id": option.id,
                    "efforts": option.efforts,
                    "tier": option.tier,
                })).collect::<Vec<_>>(),
                "defaultSpec": setup_spec_for_policy("balanced", &binding.id, Integration::Standalone)?,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "schemaVersion": 1,
        "setupSpecVersion": 1,
        "configPath": SETUP_CONFIG_PATH,
        "recipePrefix": SETUP_RECIPE_PREFIX,
        "transport": {
            "encoding": "base64url-no-padding",
            "maxDecodedBytes": MAX_SETUP_RECIPE_BYTES,
            "mayContainCredentials": false,
            "mayContainScripts": false,
        },
        "hosts": hosts,
    }))
}

pub fn setup_contract_catalog_json() -> Result<String> {
    let mut output = serde_json::to_string_pretty(&setup_contract_catalog_value()?)?;
    output.push('\n');
    Ok(output)
}

pub fn list_policies() -> Result<Vec<PolicySummary>> {
    let mut summaries = Vec::new();
    for (policy, _) in POLICIES {
        for (host, _) in BINDINGS {
            let source = show_policy(policy, host)?;
            summaries.push(PolicySummary {
                policy_id: source.policy_id,
                host: source.host,
                policy_version: source.policy_version,
                binding_id: source.binding_id,
                binding_version: source.binding_version,
                profile_count: source.profiles.len(),
                artifact_count: source.artifacts.len() + 2,
                evidence_status: source.evidence.status,
            });
        }
    }
    Ok(summaries)
}

pub fn show_policy(policy: &str, host: &str) -> Result<PolicySource> {
    let policy_raw = POLICIES
        .iter()
        .find(|(id, _)| *id == policy)
        .map(|(_, raw)| *raw)
        .ok_or_else(|| anyhow::anyhow!("unknown routing policy `{policy}`"))?;
    let binding_raw = BINDINGS
        .iter()
        .find(|(id, _)| *id == host)
        .map(|(_, raw)| *raw)
        .ok_or_else(|| anyhow::anyhow!("unknown routing host `{host}`"))?;
    let policy_contract: PolicyContract = toml::from_str(policy_raw)?;
    validate_policy_contract(&policy_contract)?;
    let binding: HostBinding = toml::from_str(binding_raw)?;

    let profiles = binding
        .profiles
        .values()
        .map(|profile| {
            (
                profile.profile.clone(),
                Profile {
                    client: profile.client.clone(),
                    model: profile.model.clone(),
                    agent_type: profile.agent_type.clone(),
                    effort: profile.effort.clone(),
                    cost_tier: profile.cost_tier.clone(),
                    capabilities: Vec::new(),
                    skill: None,
                    notes: None,
                    fork_turns: profile.fork_turns.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let routes = binding
        .routes
        .iter()
        .map(|route| {
            Ok(Route {
                selector: RouteSelector {
                    work_type: Some(route.work_type.clone()),
                    plan: None,
                },
                profile: binding_profile_id(&binding, &route.role)?.to_string(),
                fallbacks: route
                    .fallback_roles
                    .iter()
                    .map(|role| binding_profile_id(&binding, role).map(ToOwned::to_owned))
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let route_default = binding
        .default_role
        .as_deref()
        .map(|role| -> Result<DefaultRoute> {
            Ok(DefaultRoute {
                profile: binding_profile_id(&binding, role)?.to_string(),
                fallbacks: Vec::new(),
            })
        })
        .transpose()?;
    let artifacts = binding
        .artifacts
        .into_iter()
        .map(|artifact| SourceArtifact {
            media_type: media_type_for(&artifact.path, &artifact.kind),
            path: artifact.path,
            mode: "create".to_string(),
            content: artifact.content,
        })
        .collect();
    let mut capabilities = Vec::new();
    if binding.capabilities.model_override {
        capabilities.push("model_override".to_string());
    }
    if binding.capabilities.effort_override {
        capabilities.push("reasoning_effort".to_string());
    }
    if binding.capabilities.fork_none {
        capabilities.push("fork_none".to_string());
    }
    if binding.capabilities.fork_all {
        capabilities.push("bounded_context_fork".to_string());
    }
    Ok(PolicySource {
        policy_id: policy_contract.id.clone(),
        host: host.to_string(),
        policy_version: policy_contract.version.clone(),
        binding_id: binding.id,
        binding_version: binding.version,
        generated_at: GENERATED_AT.to_string(),
        requirements: vec![HostRequirement {
            host: binding.host,
            capabilities,
        }],
        profiles,
        routes,
        route_default,
        artifacts,
        evidence: EvaluationEvidence {
            evaluation_ids: vec![binding.verification.id],
            status: "experimental".to_string(),
        },
        policy: policy_contract,
        policy_toml: policy_raw.to_string(),
    })
}

pub fn compile_policy(
    policy: &str,
    host: &str,
    integration: Integration,
) -> Result<RoutingBundleV1> {
    compile_setup_spec(&setup_spec_for_policy(policy, host, integration)?)
}

#[cfg(test)]
fn compile_builtin_policy_direct(
    policy: &str,
    host: &str,
    integration: Integration,
) -> Result<RoutingBundleV1> {
    compile_source(show_policy(policy, host)?, integration)
}

fn compile_source(source: PolicySource, integration: Integration) -> Result<RoutingBundleV1> {
    validate_source(&source)?;
    let mut artifacts = Vec::new();
    if integration == Integration::Planr {
        let registry = render_registry(&source)?;
        artifacts.push(bundle_artifact(SourceArtifact {
            path: ".planr/agents.toml".to_string(),
            media_type: "application/toml".to_string(),
            mode: "replace".to_string(),
            content: registry,
        }));
        artifacts.push(bundle_artifact(SourceArtifact {
            path: ".planr/policy.toml".to_string(),
            media_type: "application/toml".to_string(),
            mode: "replace".to_string(),
            content: source.policy_toml.clone(),
        }));
    }
    let mut host_artifacts = source
        .artifacts
        .iter()
        .filter(|artifact| include_artifact_for_integration(artifact, integration))
        .cloned()
        .map(|artifact| artifact_for_integration(artifact, integration))
        .collect::<Vec<_>>();
    if let Some(codex_config) = render_codex_agent_registration_artifact(&host_artifacts)? {
        host_artifacts.push(codex_config);
    }
    artifacts.extend(host_artifacts.into_iter().map(bundle_artifact));
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(RoutingBundleV1 {
        schema_version: 1,
        bundle_id: format!(
            "{}-{}@{}+{}",
            source.policy_id, source.host, source.policy_version, source.binding_version
        ),
        policy_id: source.policy_id,
        policy_version: source.policy_version,
        generated_at: source.generated_at,
        source: BundleSource {
            package: "model-routing".to_string(),
            package_version: PACKAGE_VERSION.to_string(),
            integration,
        },
        policy: source.policy,
        requirements: source.requirements,
        profiles: source.profiles,
        routes: source.routes,
        route_default: source.route_default,
        artifacts,
        evidence: source.evidence,
    })
}

pub fn compile_json(policy: &str, host: &str, integration: Integration) -> Result<String> {
    let mut json = serde_json::to_string_pretty(&compile_policy(policy, host, integration)?)?;
    json.push('\n');
    Ok(json)
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
}

pub fn probe_host(host: &str, command_override: Option<&str>) -> Result<ProbeReport> {
    let source = show_policy("balanced", host)?;
    let requirement = source
        .requirements
        .first()
        .ok_or_else(|| anyhow::anyhow!("binding has no host requirement"))?;
    let default_command = match requirement.host.as_str() {
        "codex" => Some("codex"),
        "cursor" => Some("cursor-agent"),
        "claude-code" => Some("claude"),
        "mixed-host" => None,
        _ => None,
    };
    let command = command_override.or(default_command);
    let (available, version, limitation) = if let Some(command) = command {
        match Command::new(command).arg("--version").output() {
            Ok(output) if output.status.success() => (
                true,
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
                None,
            ),
            Ok(output) => (
                false,
                None,
                Some(format!("version probe exited with {}", output.status)),
            ),
            Err(error) => (false, None, Some(error.to_string())),
        }
    } else {
        (
            false,
            None,
            Some("mixed-host bindings require separate probes for each declared host".to_string()),
        )
    };
    Ok(ProbeReport {
        host: host.to_string(),
        command: command.map(ToOwned::to_owned),
        available,
        version,
        capabilities: requirement.capabilities.clone(),
        authentication: "not_tested".to_string(),
        limitation,
    })
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

pub fn evaluate_policy(policy: &str, host: &str) -> Result<EvaluationReport> {
    let suite: toml::Value = toml::from_str(EVALUATION_SUITE)?;
    let suite_id = suite
        .get("id")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("evaluation suite is missing id"))?;
    let suite_version = suite
        .get("version")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("evaluation suite is missing version"))?;
    let scenario_count = suite
        .get("tasks")
        .and_then(toml::Value::as_array)
        .map_or(0, Vec::len);
    let bundle = compile_json(policy, host, Integration::Standalone)?;
    Ok(EvaluationReport {
        schema_version: 1,
        suite_id: suite_id.to_string(),
        suite_version: suite_version.to_string(),
        suite_sha256: sha256(EVALUATION_SUITE.as_bytes()),
        policy_id: policy.to_string(),
        host: show_policy(policy, host)?.host,
        bundle_sha256: sha256(bundle.as_bytes()),
        scenario_count,
        offline_reproducible: scenario_count > 0,
        live_evidence: None,
        status: "experimental".to_string(),
        recommended: false,
    })
}

pub fn catalog_value() -> Result<Value> {
    let mut compositions = Vec::new();
    for summary in list_policies()? {
        let source = show_policy(&summary.policy_id, &summary.host)?;
        let report = evaluate_policy(&summary.policy_id, &summary.host)?;
        let bundle = compile_policy(&summary.policy_id, &summary.host, Integration::Standalone)?;
        compositions.push(json!({
            "id": format!("{}-{}@{}+{}", summary.policy_id, summary.binding_id, summary.policy_version, summary.binding_version),
            "entryId": format!("{}-{}", summary.policy_id, summary.binding_id),
            "entryVersion": format!("{}+{}", summary.policy_version, summary.binding_version),
            "status": report.status,
            "statusLabel": "Experimental",
            "recommended": false,
            "freshness": "current",
            "lifecycle": "published",
            "replacement": Value::Null,
            "policy": {
                "id": summary.policy_id,
                "version": summary.policy_version,
                "usage": source.policy.usage,
                "transitions": source.policy.transitions,
                "materiality": source.policy.materiality,
                "execution": source.policy.execution,
            },
            "binding": {
                "id": summary.binding_id,
                "selector": summary.host,
                "version": summary.binding_version,
                "host": source.requirements.first().map(|requirement| requirement.host.clone()),
                "profiles": bundle.profiles,
                "dispatch": bundle.routes,
            },
            "compatibility": {
                "hosts": source.requirements.iter().map(|requirement| requirement.host.clone()).collect::<Vec<_>>(),
                "minModelRoutingVersion": "0.1.0",
                "maxModelRoutingVersion": Value::Null,
            },
            "enforcement": [
                {"dimension": "Repository writes", "state": "verified", "detail": "Core previews and applies only allowlisted repository-local bundle artifacts."},
                {"dimension": "Model and effort", "state": "host_enforced", "detail": "The package generates exact host roles; the host remains execution authority."},
                {"dimension": "Effective route evidence", "state": "unavailable", "detail": "No authenticated live-host evidence is published for this generated catalog entry."}
            ],
            "evaluation": {
                "suiteId": report.suite_id,
                "suiteVersion": report.suite_version,
                "evaluatedAtUnix": GENERATED_AT_UNIX,
                "reviewAtUnix": Value::Null,
                "status": report.status,
                "metrics": {"runs": 0, "oracle_passes": 0, "average_quality_score_bps": Value::Null},
                "thresholds": {},
                "resultHashes": [],
                "fixtureSha256": report.suite_sha256,
            },
            "registry": {
                "id": "model-routing-official",
                "version": PACKAGE_VERSION,
                "manifestSha256": report.bundle_sha256,
                "signer": Value::Null,
                "signatureVerified": false,
                "trustedMaintainer": false,
                "artifacts": bundle.artifacts.iter().map(|artifact| json!({"path": artifact.path, "sha256": artifact.sha256})).collect::<Vec<_>>(),
            },
            "command": format!("model-routing compile {} --host {} --output routing-bundle.json", source.policy_id, source.host),
        }));
    }
    Ok(json!({
        "schemaVersion": 1,
        "generatedAtUnix": GENERATED_AT_UNIX,
        "setupContract": setup_contract_catalog_value()?,
        "source": {
            "state": "package_generated",
            "entryCount": compositions.len(),
            "trust": "model_routing_unsigned_catalog_v1",
            "message": "Entries stay experimental until authenticated live evidence and an offline maintainer signature pass."
        },
        "compositions": compositions,
    }))
}

pub fn catalog_json() -> Result<String> {
    let mut output = serde_json::to_string_pretty(&catalog_value()?)?;
    output.push('\n');
    Ok(output)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrySignature {
    pub algorithm: String,
    pub signer: String,
    pub content_sha256: String,
    pub value: String,
}

pub fn sign_registry(
    content: &[u8],
    signer: &str,
    private_key_hex: &str,
) -> Result<RegistrySignature> {
    if signer.trim().is_empty() {
        bail!("registry signer must not be blank");
    }
    let seed = decode_hex::<32>(private_key_hex.trim()).ok_or_else(|| {
        anyhow::anyhow!("private key file must contain exactly 64 hexadecimal characters")
    })?;
    let key = SigningKey::from_bytes(&seed);
    let signature = key.sign(content);
    Ok(RegistrySignature {
        algorithm: "ed25519".to_string(),
        signer: signer.to_string(),
        content_sha256: sha256(content),
        value: encode_hex(&signature.to_bytes()),
    })
}

pub fn verify_registry_signature(
    content: &[u8],
    signature: &RegistrySignature,
    trusted_signer: &str,
    trusted_public_key_hex: &str,
) -> Result<()> {
    if signature.algorithm != "ed25519" || signature.content_sha256 != sha256(content) {
        bail!("registry signature metadata does not match content");
    }
    if trusted_signer.trim().is_empty() || signature.signer != trusted_signer {
        bail!("registry signature signer does not match the trusted signer");
    }
    let public_key = decode_hex::<32>(trusted_public_key_hex.trim())
        .ok_or_else(|| anyhow::anyhow!("trusted registry public key is invalid"))?;
    let signature_bytes = decode_hex::<64>(&signature.value)
        .ok_or_else(|| anyhow::anyhow!("registry signature value is invalid"))?;
    let key = VerifyingKey::from_bytes(&public_key)?;
    key.verify(content, &Signature::from_bytes(&signature_bytes))?;
    Ok(())
}

pub fn inspect_bundle_json(input: &str) -> Result<BundleInspection> {
    let bundle = validate_bundle_json(input)?;
    Ok(BundleInspection {
        schema_version: bundle.schema_version,
        bundle_id: bundle.bundle_id,
        policy_id: bundle.policy_id,
        integration: bundle.source.integration,
        profile_count: bundle.profiles.len(),
        route_count: bundle.routes.len(),
        artifact_count: bundle.artifacts.len(),
        valid: true,
    })
}

pub fn validate_bundle_json(input: &str) -> Result<RoutingBundleV1> {
    let value: Value = serde_json::from_str(input).context("bundle is not valid JSON")?;
    validate_raw_bundle_shape(&value)?;
    let bundle: RoutingBundleV1 = serde_json::from_value(value).map_err(|error| {
        anyhow::anyhow!("bundle does not match RoutingBundle v1 schema: {error}")
    })?;
    validate_bundle(&bundle)?;
    Ok(bundle)
}

pub fn validate_bundle(bundle: &RoutingBundleV1) -> Result<()> {
    if bundle.schema_version != 1 {
        bail!("unsupported schema_version {}", bundle.schema_version);
    }
    if bundle.bundle_id.trim().is_empty()
        || bundle.policy_id.trim().is_empty()
        || bundle.policy_version.trim().is_empty()
    {
        bail!("bundle id, policy id, and policy version must not be blank");
    }
    if bundle.source.package != "model-routing" {
        bail!("bundle source package must be model-routing");
    }
    if bundle.policy.id != bundle.policy_id || bundle.policy.version != bundle.policy_version {
        bail!("bundle policy contract does not match bundle policy identifiers");
    }
    validate_policy_contract(&bundle.policy)?;
    for route in &bundle.routes {
        if !bundle.profiles.contains_key(&route.profile) {
            bail!("route references unknown profile `{}`", route.profile);
        }
        for fallback in &route.fallbacks {
            if !bundle.profiles.contains_key(fallback) {
                bail!("route fallback references unknown profile `{fallback}`");
            }
        }
    }
    if let Some(default) = &bundle.route_default {
        if !bundle.profiles.contains_key(&default.profile) {
            bail!(
                "default route references unknown profile `{}`",
                default.profile
            );
        }
        for fallback in &default.fallbacks {
            if !bundle.profiles.contains_key(fallback) {
                bail!("default route fallback references unknown profile `{fallback}`");
            }
        }
    }
    let mut artifact_paths = BTreeSet::new();
    for artifact in &bundle.artifacts {
        if artifact.path.trim().is_empty() {
            bail!("artifact path must not be blank");
        }
        if !artifact_paths.insert(artifact.path.clone()) {
            bail!("duplicate artifact path `{}`", artifact.path);
        }
        if artifact.mode != "create" && artifact.mode != "replace" {
            bail!(
                "artifact `{}` has unsupported mode `{}`",
                artifact.path,
                artifact.mode
            );
        }
        let expected = sha256(artifact.content.as_bytes());
        if artifact.sha256 != expected {
            bail!("artifact `{}` sha256 mismatch", artifact.path);
        }
    }
    Ok(())
}

pub fn preview_bundle_file(repository: &Path, bundle_file: &Path) -> Result<LifecycleReport> {
    let bundle = read_bundle_file(bundle_file)?;
    preview_bundle(repository, &bundle)
}

pub fn apply_bundle_file(repository: &Path, bundle_file: &Path) -> Result<LifecycleReport> {
    let bundle_input = fs::read_to_string(bundle_file)
        .with_context(|| format!("failed to read bundle `{}`", bundle_file.display()))?;
    let bundle = validate_bundle_json(&bundle_input)?;
    apply_bundle_json(repository, &bundle, &bundle_input)
}

fn apply_bundle_json(
    repository: &Path,
    bundle: &RoutingBundleV1,
    bundle_input: &str,
) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let planned = plan_artifacts(&repository, bundle, None)?;
    ensure_apply_is_safe(&planned)?;
    let manifest = manifest_from_bundle(bundle, sha256(bundle_input.as_bytes()), None);
    commit_transaction(&repository, &planned, &manifest)?;
    Ok(report_from_plan(
        "apply",
        &repository,
        Some(&bundle.bundle_id),
        &planned,
    ))
}

pub fn update_bundle_file(repository: &Path, bundle_file: &Path) -> Result<LifecycleReport> {
    let bundle_input = fs::read_to_string(bundle_file)
        .with_context(|| format!("failed to read bundle `{}`", bundle_file.display()))?;
    let bundle = validate_bundle_json(&bundle_input)?;
    update_bundle_json(repository, &bundle, &bundle_input)
}

fn update_bundle_json(
    repository: &Path,
    bundle: &RoutingBundleV1,
    bundle_input: &str,
) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let current = read_manifest(&repository)?
        .ok_or_else(|| anyhow::anyhow!("no model-routing manifest found"))?;
    let planned = plan_artifacts(&repository, bundle, Some(&current))?;
    ensure_update_is_safe(&planned)?;
    let manifest = manifest_from_plan(
        &bundle.bundle_id,
        sha256(bundle_input.as_bytes()),
        &planned,
        Some(snapshot_from_manifest(&current)),
    );
    commit_transaction(&repository, &planned, &manifest)?;
    Ok(report_from_plan(
        "update",
        &repository,
        Some(&bundle.bundle_id),
        &planned,
    ))
}

pub fn status_repository(repository: &Path) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let Some(manifest) = read_manifest(&repository)? else {
        return Ok(LifecycleReport {
            action: "status".to_string(),
            bundle_id: None,
            repository: repository.display().to_string(),
            artifacts: Vec::new(),
        });
    };
    let mut reports = Vec::new();
    for artifact in &manifest.artifacts {
        let target = resolve_repository_target(&repository, &artifact.path)?;
        let status = status_for_managed_artifact(&target, artifact)?;
        reports.push(LifecycleArtifactReport {
            path: artifact.path.clone(),
            mode: "managed".to_string(),
            status: status.to_string(),
            sha256: artifact.sha256.clone(),
            repair: repair_for_status(status),
        });
    }
    Ok(LifecycleReport {
        action: "status".to_string(),
        bundle_id: Some(manifest.bundle_id),
        repository: repository.display().to_string(),
        artifacts: reports,
    })
}

pub fn uninstall_repository(repository: &Path) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let manifest = read_manifest(&repository)?
        .ok_or_else(|| anyhow::anyhow!("no model-routing manifest found"))?;
    let mut reports = Vec::new();
    for artifact in &manifest.artifacts {
        let target = resolve_repository_target(&repository, &artifact.path)?;
        let status = uninstall_managed_artifact(&target, artifact)?;
        reports.push(LifecycleArtifactReport {
            path: artifact.path.clone(),
            mode: "managed".to_string(),
            status: status.to_string(),
            sha256: artifact.sha256.clone(),
            repair: repair_for_status(status),
        });
    }
    let residual_artifacts = manifest
        .artifacts
        .iter()
        .zip(reports.iter())
        .filter(|(_, report)| report.status != "removed")
        .map(|(artifact, _)| ManagedArtifact {
            path: artifact.path.clone(),
            sha256: artifact.sha256.clone(),
            content: artifact.content.clone(),
        })
        .collect::<Vec<_>>();
    if residual_artifacts.is_empty() {
        remove_manifest(&repository)?;
    } else {
        let residual = ManagedManifest {
            schema_version: 1,
            bundle_id: manifest.bundle_id.clone(),
            bundle_sha256: manifest.bundle_sha256.clone(),
            artifacts: residual_artifacts,
            previous: manifest.previous.clone(),
        };
        write_manifest_file(&repository, &residual)?;
    }
    Ok(LifecycleReport {
        action: "uninstall".to_string(),
        bundle_id: Some(manifest.bundle_id),
        repository: repository.display().to_string(),
        artifacts: reports,
    })
}

pub fn rollback_repository(repository: &Path) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let manifest = read_manifest(&repository)?
        .ok_or_else(|| anyhow::anyhow!("no model-routing manifest found"))?;
    let previous = manifest
        .previous
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no rollback snapshot found"))?;
    let planned = plan_rollback_artifacts(&repository, &manifest, &previous)?;
    ensure_update_is_safe(&planned)?;
    let rollback_manifest = manifest_from_plan(
        &previous.bundle_id,
        previous.bundle_sha256.clone(),
        &planned,
        None,
    );
    commit_transaction(&repository, &planned, &rollback_manifest)?;
    Ok(report_from_plan(
        "rollback",
        &repository,
        Some(&rollback_manifest.bundle_id),
        &planned,
    ))
}

fn read_bundle_file(bundle_file: &Path) -> Result<RoutingBundleV1> {
    let input = fs::read_to_string(bundle_file)
        .with_context(|| format!("failed to read bundle `{}`", bundle_file.display()))?;
    validate_bundle_json(&input)
}

fn read_setup_config_file(config_file: &Path) -> Result<SetupSpecV1> {
    let input = fs::read_to_string(config_file)
        .with_context(|| format!("failed to read setup config `{}`", config_file.display()))?;
    setup_spec_from_toml(&input)
}

fn read_saved_setup_config(repository: &Path) -> Result<SetupSpecV1> {
    let repository = canonicalize_existing_repository(repository)?;
    let config_path = repository.join(SETUP_CONFIG_PATH);
    read_setup_config_file(&config_path)
}

fn preview_setup(repository: &Path, spec: &SetupSpecV1) -> Result<LifecycleReport> {
    let prepared = prepare_setup_lifecycle(spec)?;
    preview_bundle(repository, &prepared.bundle)
}

fn apply_setup(repository: &Path, spec: &SetupSpecV1) -> Result<LifecycleReport> {
    let prepared = prepare_setup_lifecycle(spec)?;
    apply_bundle_json(repository, &prepared.bundle, &prepared.bundle_input)
}

fn update_setup(repository: &Path, spec: &SetupSpecV1) -> Result<LifecycleReport> {
    let prepared = prepare_setup_lifecycle(spec)?;
    update_bundle_json(repository, &prepared.bundle, &prepared.bundle_input)
}

fn prepare_setup_lifecycle(spec: &SetupSpecV1) -> Result<PreparedSetupLifecycle> {
    let normalized_config = setup_spec_to_canonical_toml(spec)?;
    let mut bundle = compile_setup_spec(spec)?;
    bundle.artifacts.push(bundle_artifact(SourceArtifact {
        path: SETUP_CONFIG_PATH.to_string(),
        media_type: "application/toml".to_string(),
        mode: "replace".to_string(),
        content: normalized_config,
    }));
    bundle
        .artifacts
        .sort_by(|left, right| left.path.cmp(&right.path));
    validate_bundle(&bundle)?;
    let mut bundle_input = serde_json::to_string_pretty(&bundle)?;
    bundle_input.push('\n');
    Ok(PreparedSetupLifecycle {
        bundle,
        bundle_input,
    })
}

fn same_lifecycle_plan(left: &LifecycleReport, right: &LifecycleReport) -> bool {
    left.action == right.action
        && left.bundle_id == right.bundle_id
        && left.repository == right.repository
        && left.artifacts == right.artifacts
}

#[derive(Debug)]
struct PlannedArtifact {
    path: String,
    target: PathBuf,
    mode: String,
    content: Option<String>,
    managed_content: Option<String>,
    sha256: String,
    status: String,
}

fn preview_bundle(repository: &Path, bundle: &RoutingBundleV1) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let planned = plan_artifacts(&repository, bundle, None)?;
    Ok(report_from_plan(
        "preview",
        &repository,
        Some(&bundle.bundle_id),
        &planned,
    ))
}

fn plan_artifacts(
    repository: &Path,
    bundle: &RoutingBundleV1,
    current_manifest: Option<&ManagedManifest>,
) -> Result<Vec<PlannedArtifact>> {
    let mut seen_targets = BTreeSet::new();
    let mut planned = Vec::new();
    for artifact in &bundle.artifacts {
        let target = resolve_repository_target(repository, &artifact.path)?;
        let key = target.display().to_string();
        if !seen_targets.insert(key) {
            bail!("duplicate resolved artifact target `{}`", artifact.path);
        }
        let managed_entry = current_manifest.and_then(|manifest| {
            manifest
                .artifacts
                .iter()
                .find(|managed| managed.path == artifact.path)
        });
        if artifact.path == CODEX_CONFIG_PATH {
            planned.push(plan_codex_config_artifact(
                repository,
                artifact,
                target,
                managed_entry,
            )?);
            continue;
        }
        let status = if target.exists() {
            let metadata = fs::symlink_metadata(&target)
                .with_context(|| format!("failed to inspect `{}`", target.display()))?;
            if metadata.file_type().is_symlink() {
                bail!("artifact target `{}` is a symlink", artifact.path);
            }
            let current = fs::read(&target)
                .with_context(|| format!("failed to read `{}`", target.display()))?;
            let current_sha = sha256(&current);
            if current_sha == artifact.sha256 {
                "unchanged"
            } else if let Some(managed) = managed_entry {
                if current_sha == managed.sha256 {
                    "update"
                } else {
                    "preserved-modified"
                }
            } else {
                "conflict"
            }
        } else {
            ensure_parent_is_safe(repository, &target)?;
            "create"
        };
        planned.push(PlannedArtifact {
            path: artifact.path.clone(),
            target,
            mode: artifact.mode.clone(),
            content: Some(artifact.content.clone()),
            managed_content: Some(artifact.content.clone()),
            sha256: artifact.sha256.clone(),
            status: status.to_string(),
        });
    }
    if let Some(manifest) = current_manifest {
        for artifact in &manifest.artifacts {
            if bundle
                .artifacts
                .iter()
                .any(|bundle_artifact| bundle_artifact.path == artifact.path)
            {
                continue;
            }
            let target = resolve_repository_target(repository, &artifact.path)?;
            let (status, content) = if artifact.path == CODEX_CONFIG_PATH {
                let status = preserved_or_removed_status(&target, artifact)?;
                let content = if status == "removed" {
                    remove_managed_codex_config_entries(&target, artifact)?
                } else {
                    artifact.content.clone()
                };
                (status, content)
            } else {
                let status = preserved_or_removed_status(&target, artifact)?;
                let content = if status == "removed" {
                    None
                } else {
                    artifact.content.clone()
                };
                (status, content)
            };
            planned.push(PlannedArtifact {
                path: artifact.path.clone(),
                target,
                mode: "delete".to_string(),
                content,
                managed_content: artifact.content.clone(),
                sha256: artifact.sha256.clone(),
                status,
            });
        }
    }
    reject_parent_child_targets(&planned)?;
    Ok(planned)
}

fn ensure_apply_is_safe(planned: &[PlannedArtifact]) -> Result<()> {
    for artifact in planned {
        if artifact.status == "conflict" || artifact.status == "preserved-modified" {
            bail!(
                "artifact target `{}` already exists with different content",
                artifact.path
            );
        }
    }
    Ok(())
}

fn ensure_update_is_safe(planned: &[PlannedArtifact]) -> Result<()> {
    for artifact in planned {
        if artifact.status == "conflict" {
            bail!(
                "artifact target `{}` already exists with unmanaged content",
                artifact.path
            );
        }
    }
    Ok(())
}

fn reject_parent_child_targets(planned: &[PlannedArtifact]) -> Result<()> {
    for (index, left) in planned.iter().enumerate() {
        let left_relative = Path::new(&left.path);
        for right in planned.iter().skip(index + 1) {
            let right_relative = Path::new(&right.path);
            if left_relative.starts_with(right_relative)
                || right_relative.starts_with(left_relative)
            {
                bail!(
                    "artifact targets `{}` and `{}` have a parent-child collision",
                    left.path,
                    right.path
                );
            }
        }
    }
    Ok(())
}

fn plan_rollback_artifacts(
    repository: &Path,
    current_manifest: &ManagedManifest,
    previous: &ManagedSnapshot,
) -> Result<Vec<PlannedArtifact>> {
    let mut planned = Vec::new();
    for artifact in &previous.artifacts {
        let content = artifact.content.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "rollback artifact `{}` has no stored content",
                artifact.path
            )
        })?;
        let target = resolve_repository_target(repository, &artifact.path)?;
        let current = current_manifest
            .artifacts
            .iter()
            .find(|managed| managed.path == artifact.path);
        if artifact.path == CODEX_CONFIG_PATH {
            planned.push(plan_codex_config_rollback_artifact(
                repository, artifact, content, target, current,
            )?);
            continue;
        }
        let status = if target.exists() {
            let current_content = fs::read(&target)
                .with_context(|| format!("failed to read `{}`", target.display()))?;
            let current_sha = sha256(&current_content);
            if current_sha == artifact.sha256 {
                "unchanged"
            } else if let Some(managed) = current {
                if current_sha == managed.sha256 {
                    "rollback"
                } else {
                    "preserved-modified"
                }
            } else {
                "rollback"
            }
        } else {
            ensure_parent_is_safe(repository, &target)?;
            "create"
        };
        planned.push(PlannedArtifact {
            path: artifact.path.clone(),
            target,
            mode: "replace".to_string(),
            content: Some(content),
            managed_content: artifact.content.clone(),
            sha256: artifact.sha256.clone(),
            status: status.to_string(),
        });
    }
    for artifact in &current_manifest.artifacts {
        if previous
            .artifacts
            .iter()
            .any(|previous_artifact| previous_artifact.path == artifact.path)
        {
            continue;
        }
        let target = resolve_repository_target(repository, &artifact.path)?;
        let (status, content) = if artifact.path == CODEX_CONFIG_PATH {
            let status = preserved_or_removed_status(&target, artifact)?;
            let content = if status == "removed" {
                remove_managed_codex_config_entries(&target, artifact)?
            } else {
                artifact.content.clone()
            };
            (status, content)
        } else {
            let status = preserved_or_removed_status(&target, artifact)?;
            let content = if status == "removed" {
                None
            } else {
                artifact.content.clone()
            };
            (status, content)
        };
        planned.push(PlannedArtifact {
            path: artifact.path.clone(),
            target,
            mode: "delete".to_string(),
            content,
            managed_content: artifact.content.clone(),
            sha256: artifact.sha256.clone(),
            status,
        });
    }
    reject_parent_child_targets(&planned)?;
    Ok(planned)
}

fn plan_codex_config_artifact(
    repository: &Path,
    artifact: &BundleArtifact,
    target: PathBuf,
    managed_entry: Option<&ManagedArtifact>,
) -> Result<PlannedArtifact> {
    let (status, content) = if target.exists() {
        ensure_artifact_target_is_regular(&target, &artifact.path)?;
        let current = fs::read_to_string(&target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        if let Some(managed) = managed_entry {
            if codex_config_contains_managed_entries(&current, managed)? {
                if codex_config_has_unmanaged_conflict(&current, &artifact.content, Some(managed))?
                {
                    ("conflict".to_string(), Some(current))
                } else if managed.content.as_deref() == Some(artifact.content.as_str())
                    && codex_config_contains_desired_entries(&current, &artifact.content)?
                {
                    ("unchanged".to_string(), Some(current))
                } else {
                    (
                        "update".to_string(),
                        merge_codex_config_entries(
                            Some(&current),
                            Some(managed),
                            &artifact.content,
                        )?,
                    )
                }
            } else {
                ("preserved-modified".to_string(), Some(current))
            }
        } else if codex_config_has_unmanaged_conflict(&current, &artifact.content, None)? {
            ("conflict".to_string(), Some(current))
        } else if codex_config_contains_desired_entries(&current, &artifact.content)? {
            ("unchanged".to_string(), Some(current))
        } else {
            (
                "update".to_string(),
                merge_codex_config_entries(Some(&current), None, &artifact.content)?,
            )
        }
    } else {
        ensure_parent_is_safe(repository, &target)?;
        ("create".to_string(), Some(artifact.content.clone()))
    };
    Ok(PlannedArtifact {
        path: artifact.path.clone(),
        target,
        mode: artifact.mode.clone(),
        content,
        managed_content: Some(artifact.content.clone()),
        sha256: artifact.sha256.clone(),
        status,
    })
}

fn plan_codex_config_rollback_artifact(
    repository: &Path,
    artifact: &ManagedArtifact,
    desired_content: String,
    target: PathBuf,
    current: Option<&ManagedArtifact>,
) -> Result<PlannedArtifact> {
    let (status, content) = if target.exists() {
        ensure_artifact_target_is_regular(&target, &artifact.path)?;
        let current_text = fs::read_to_string(&target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        if let Some(managed) = current {
            if codex_config_contains_managed_entries(&current_text, managed)? {
                if codex_config_has_unmanaged_conflict(
                    &current_text,
                    &desired_content,
                    Some(managed),
                )? {
                    ("conflict".to_string(), Some(current_text))
                } else if managed.content.as_deref() == Some(desired_content.as_str())
                    && codex_config_contains_desired_entries(&current_text, &desired_content)?
                {
                    ("unchanged".to_string(), Some(current_text))
                } else {
                    (
                        "rollback".to_string(),
                        merge_codex_config_entries(
                            Some(&current_text),
                            Some(managed),
                            &desired_content,
                        )?,
                    )
                }
            } else {
                ("preserved-modified".to_string(), Some(current_text))
            }
        } else if codex_config_has_unmanaged_conflict(&current_text, &desired_content, None)? {
            ("conflict".to_string(), Some(current_text))
        } else {
            (
                "rollback".to_string(),
                merge_codex_config_entries(Some(&current_text), None, &desired_content)?,
            )
        }
    } else {
        ensure_parent_is_safe(repository, &target)?;
        ("create".to_string(), Some(desired_content.clone()))
    };
    Ok(PlannedArtifact {
        path: artifact.path.clone(),
        target,
        mode: "replace".to_string(),
        content,
        managed_content: Some(desired_content),
        sha256: artifact.sha256.clone(),
        status,
    })
}

fn ensure_artifact_target_is_regular(target: &Path, artifact_path: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{artifact_path}` is a symlink");
    }
    if !metadata.is_file() {
        bail!("artifact target `{artifact_path}` is not a file");
    }
    Ok(())
}

fn preserved_or_removed_status(target: &Path, artifact: &ManagedArtifact) -> Result<String> {
    if !target.exists() {
        return Ok("missing".to_string());
    }
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{}` is a symlink", artifact.path);
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        return if codex_config_contains_managed_entries(&current, artifact)? {
            Ok("removed".to_string())
        } else {
            Ok("preserved-modified".to_string())
        };
    }
    let current =
        fs::read(target).with_context(|| format!("failed to read `{}`", target.display()))?;
    if sha256(&current) == artifact.sha256 {
        Ok("removed".to_string())
    } else {
        Ok("preserved-modified".to_string())
    }
}

fn status_for_managed_artifact(target: &Path, artifact: &ManagedArtifact) -> Result<&'static str> {
    if !target.exists() {
        return Ok("missing");
    }
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{}` is a symlink", artifact.path);
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        return if codex_config_contains_managed_entries(&current, artifact)? {
            Ok("managed")
        } else {
            Ok("modified")
        };
    }
    let content =
        fs::read(target).with_context(|| format!("failed to read `{}`", target.display()))?;
    if sha256(&content) == artifact.sha256 {
        Ok("managed")
    } else {
        Ok("modified")
    }
}

fn uninstall_managed_artifact(target: &Path, artifact: &ManagedArtifact) -> Result<&'static str> {
    if !target.exists() {
        return Ok("missing");
    }
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{}` is a symlink", artifact.path);
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        if !codex_config_contains_managed_entries(&current, artifact)? {
            return Ok("preserved-modified");
        }
        match remove_managed_codex_config_entries(target, artifact)? {
            Some(content) => fs::write(target, content.as_bytes())
                .with_context(|| format!("failed to write `{}`", target.display()))?,
            None => fs::remove_file(target)
                .with_context(|| format!("failed to remove `{}`", target.display()))?,
        }
        return Ok("removed");
    }
    let content =
        fs::read(target).with_context(|| format!("failed to read `{}`", target.display()))?;
    if sha256(&content) != artifact.sha256 {
        Ok("preserved-modified")
    } else {
        fs::remove_file(target)
            .with_context(|| format!("failed to remove `{}`", target.display()))?;
        Ok("removed")
    }
}

fn codex_config_contains_managed_entries(
    current_content: &str,
    managed: &ManagedArtifact,
) -> Result<bool> {
    let managed_content = managed.content.as_deref().ok_or_else(|| {
        anyhow::anyhow!("managed artifact `{}` has no stored content", managed.path)
    })?;
    Ok(
        !codex_config_has_unmanaged_conflict(current_content, managed_content, None)?
            && codex_config_contains_desired_entries(current_content, managed_content)?,
    )
}

fn codex_config_contains_desired_entries(
    current_content: &str,
    desired_content: &str,
) -> Result<bool> {
    let current = codex_agent_entries(current_content)?;
    let desired = codex_agent_entries(desired_content)?;
    Ok(desired
        .iter()
        .all(|(name, desired_entry)| current.get(name) == Some(desired_entry)))
}

fn codex_config_has_unmanaged_conflict(
    current_content: &str,
    desired_content: &str,
    previously_managed: Option<&ManagedArtifact>,
) -> Result<bool> {
    let current = codex_agent_entries(current_content)?;
    let desired = codex_agent_entries(desired_content)?;
    let old_keys = previously_managed
        .and_then(|managed| managed.content.as_deref())
        .map(codex_agent_entry_names)
        .transpose()?
        .unwrap_or_default();
    Ok(desired.iter().any(|(name, desired_entry)| {
        !old_keys.contains(name)
            && current
                .get(name)
                .is_some_and(|entry| entry != desired_entry)
    }))
}

fn merge_codex_config_entries(
    current_content: Option<&str>,
    previously_managed: Option<&ManagedArtifact>,
    desired_content: &str,
) -> Result<Option<String>> {
    let mut root = match current_content {
        Some(content) => parse_toml_root(content)?,
        None => toml::value::Table::new(),
    };
    if let Some(managed) = previously_managed {
        let managed_content = managed.content.as_deref().ok_or_else(|| {
            anyhow::anyhow!("managed artifact `{}` has no stored content", managed.path)
        })?;
        remove_codex_agent_entries(&mut root, &codex_agent_entry_names(managed_content)?)?;
    }
    upsert_codex_agent_entries(&mut root, codex_agent_entries(desired_content)?)?;
    render_toml_root(root)
}

fn remove_managed_codex_config_entries(
    target: &Path,
    managed: &ManagedArtifact,
) -> Result<Option<String>> {
    let current = fs::read_to_string(target)
        .with_context(|| format!("failed to read `{}`", target.display()))?;
    let managed_content = managed.content.as_deref().ok_or_else(|| {
        anyhow::anyhow!("managed artifact `{}` has no stored content", managed.path)
    })?;
    let mut root = parse_toml_root(&current)?;
    remove_codex_agent_entries(&mut root, &codex_agent_entry_names(managed_content)?)?;
    render_toml_root(root)
}

fn parse_toml_root(content: &str) -> Result<toml::value::Table> {
    match toml::from_str::<toml::Value>(content)? {
        toml::Value::Table(table) => Ok(table),
        _ => bail!("Codex config must be a TOML table"),
    }
}

fn codex_agent_entry_names(content: &str) -> Result<BTreeSet<String>> {
    Ok(codex_agent_entries(content)?.into_keys().collect())
}

fn codex_agent_entries(content: &str) -> Result<BTreeMap<String, toml::Value>> {
    let root = parse_toml_root(content)?;
    let Some(agents) = root.get("agents") else {
        return Ok(BTreeMap::new());
    };
    let agents = agents
        .as_table()
        .ok_or_else(|| anyhow::anyhow!("Codex config `agents` must be a table"))?;
    Ok(agents
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect())
}

fn remove_codex_agent_entries(
    root: &mut toml::value::Table,
    names: &BTreeSet<String>,
) -> Result<()> {
    let Some(agents_value) = root.get_mut("agents") else {
        return Ok(());
    };
    let agents = agents_value
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Codex config `agents` must be a table"))?;
    for name in names {
        agents.remove(name);
    }
    if agents.is_empty() {
        root.remove("agents");
    }
    Ok(())
}

fn upsert_codex_agent_entries(
    root: &mut toml::value::Table,
    entries: BTreeMap<String, toml::Value>,
) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    if !root.contains_key("agents") {
        root.insert(
            "agents".to_string(),
            toml::Value::Table(toml::value::Table::new()),
        );
    }
    let agents = root
        .get_mut("agents")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| anyhow::anyhow!("Codex config `agents` must be a table"))?;
    for (name, value) in entries {
        agents.insert(name, value);
    }
    Ok(())
}

fn render_toml_root(root: toml::value::Table) -> Result<Option<String>> {
    if root.is_empty() {
        return Ok(None);
    }
    let mut content = toml::to_string_pretty(&toml::Value::Table(root))?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(Some(content))
}

fn commit_transaction(
    repository: &Path,
    planned: &[PlannedArtifact],
    manifest: &ManagedManifest,
) -> Result<()> {
    let txn_root = repository.join(".model-routing").join(format!(
        "txn-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let stage_root = txn_root.join("stage");
    let backup_root = txn_root.join("backup");
    fs::create_dir_all(&stage_root)
        .with_context(|| format!("failed to create `{}`", stage_root.display()))?;
    fs::create_dir_all(&backup_root)
        .with_context(|| format!("failed to create `{}`", backup_root.display()))?;

    let mut writes = Vec::new();
    for (index, artifact) in planned.iter().enumerate() {
        if artifact.status == "unchanged"
            || artifact.status == "preserved-modified"
            || artifact.status == "missing"
        {
            continue;
        }
        let staged = match &artifact.content {
            Some(content) => {
                let staged = stage_root.join(format!("artifact-{index}"));
                fs::write(&staged, content.as_bytes())
                    .with_context(|| format!("failed to stage `{}`", artifact.path))?;
                Some(staged)
            }
            None => None,
        };
        writes.push(TransactionalWrite {
            label: artifact.path.clone(),
            target: artifact.target.clone(),
            staged,
            backup: backup_root.join(format!("artifact-{index}")),
            committed: false,
            backup_created: false,
            had_original: artifact.target.exists(),
        });
    }

    let manifest_path = repository.join(MANIFEST_PATH);
    let manifest_stage = stage_root.join("manifest.json");
    fs::write(&manifest_stage, serde_json::to_vec_pretty(manifest)?)
        .with_context(|| format!("failed to stage `{MANIFEST_PATH}`"))?;
    writes.push(TransactionalWrite {
        label: MANIFEST_PATH.to_string(),
        target: manifest_path.clone(),
        staged: Some(manifest_stage),
        backup: backup_root.join("manifest.json"),
        committed: false,
        backup_created: false,
        had_original: manifest_path.exists(),
    });

    write_transaction_journal(repository, &txn_root, &writes)?;
    let result = commit_writes(&mut writes);
    if let Err(error) = result {
        if let Err(rollback_error) = rollback_writes(&writes) {
            return Err(error).with_context(|| {
                format!(
                    "transaction rollback incomplete; retained `{}` for recovery: {rollback_error:#}",
                    txn_root.display()
                )
            });
        }
        fs::remove_dir_all(&txn_root)
            .with_context(|| format!("failed to remove `{}`", txn_root.display()))?;
        return Err(error);
    }
    fs::remove_dir_all(&txn_root)
        .with_context(|| format!("failed to remove `{}`", txn_root.display()))?;
    Ok(())
}

#[derive(Debug)]
struct TransactionalWrite {
    label: String,
    target: PathBuf,
    staged: Option<PathBuf>,
    backup: PathBuf,
    committed: bool,
    backup_created: bool,
    had_original: bool,
}

fn commit_writes(writes: &mut [TransactionalWrite]) -> Result<()> {
    let txn_root = writes
        .first()
        .and_then(|write| write.backup.parent())
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("transaction has no writes"))?;
    let repository = txn_root
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| anyhow::anyhow!("transaction root is outside repository metadata"))?
        .to_path_buf();
    for index in 0..writes.len() {
        if let Some(parent) = writes[index].target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create `{}`", parent.display()))?;
        }
        if writes[index].target.exists() {
            fs::rename(&writes[index].target, &writes[index].backup)
                .with_context(|| format!("failed to backup `{}`", writes[index].label))?;
            writes[index].backup_created = true;
            write_transaction_journal(&repository, &txn_root, writes)?;
        }
        if let Some(staged) = &writes[index].staged {
            maybe_return_staged_rename_error()?;
            fs::rename(staged, &writes[index].target)
                .with_context(|| format!("failed to commit `{}`", writes[index].label))?;
        }
        writes[index].committed = true;
        write_transaction_journal(&repository, &txn_root, writes)?;
        maybe_fail_after_transaction_write()?;
    }
    Ok(())
}

fn rollback_writes(writes: &[TransactionalWrite]) -> Result<()> {
    for write in writes.iter().rev() {
        if !write.committed && !write.backup_created {
            continue;
        }
        maybe_fail_during_restore()?;
        if write.target.exists() {
            fs::remove_file(&write.target)
                .with_context(|| format!("failed to remove `{}`", write.target.display()))?;
        }
        if write.had_original {
            if let Some(parent) = write.target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create `{}`", parent.display()))?;
            }
            fs::rename(&write.backup, &write.target).with_context(|| {
                format!(
                    "failed to restore `{}` from `{}`",
                    write.target.display(),
                    write.backup.display()
                )
            })?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionJournal {
    schema_version: u32,
    writes: Vec<TransactionJournalWrite>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionJournalWrite {
    label: String,
    target: String,
    staged: Option<String>,
    backup: String,
    committed: bool,
    #[serde(default)]
    backup_created: bool,
    had_original: bool,
}

fn write_transaction_journal(
    repository: &Path,
    txn_root: &Path,
    writes: &[TransactionalWrite],
) -> Result<()> {
    let journal = TransactionJournal {
        schema_version: 1,
        writes: writes
            .iter()
            .map(|write| {
                Ok(TransactionJournalWrite {
                    label: write.label.clone(),
                    target: repository_relative(repository, &write.target)?,
                    staged: write
                        .staged
                        .as_ref()
                        .map(|staged| repository_relative(repository, staged))
                        .transpose()?,
                    backup: repository_relative(repository, &write.backup)?,
                    committed: write.committed,
                    backup_created: write.backup_created,
                    had_original: write.had_original,
                })
            })
            .collect::<Result<Vec<_>>>()?,
    };
    let journal_path = txn_root.join(TRANSACTION_JOURNAL);
    let temp_path = txn_root.join(format!("{TRANSACTION_JOURNAL}.tmp"));
    fs::write(&temp_path, serde_json::to_vec_pretty(&journal)?).with_context(|| {
        format!(
            "failed to write transaction journal temp `{}`",
            temp_path.display()
        )
    })?;
    sync_file(&temp_path)?;
    maybe_return_journal_error()?;
    maybe_fail_during_journal_replace();
    fs::rename(&temp_path, &journal_path).with_context(|| {
        format!(
            "failed to replace transaction journal `{}`",
            journal_path.display()
        )
    })?;
    sync_directory(txn_root)?;
    Ok(())
}

fn recover_pending_transactions(repository: &Path) -> Result<()> {
    let metadata_dir = repository.join(".model-routing");
    if !metadata_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&metadata_dir)
        .with_context(|| format!("failed to read `{}`", metadata_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with("txn-") {
            continue;
        }
        recover_transaction(repository, &entry.path())?;
    }
    Ok(())
}

fn recover_transaction(repository: &Path, txn_root: &Path) -> Result<()> {
    let journal_path = txn_root.join(TRANSACTION_JOURNAL);
    if journal_path.exists() {
        let input = fs::read(&journal_path)
            .with_context(|| format!("failed to read `{}`", journal_path.display()))?;
        let journal: TransactionJournal = serde_json::from_slice(&input)
            .with_context(|| format!("failed to parse `{}`", journal_path.display()))?;
        for write in journal.writes.iter().rev() {
            recover_transaction_write(repository, write).with_context(|| {
                format!("failed to recover transaction write `{}`", write.label)
            })?;
        }
    }
    fs::remove_dir_all(txn_root)
        .with_context(|| format!("failed to remove `{}`", txn_root.display()))?;
    Ok(())
}

fn recover_transaction_write(repository: &Path, write: &TransactionJournalWrite) -> Result<()> {
    maybe_fail_during_restore()?;
    let target = repository.join(&write.target);
    let backup = repository.join(&write.backup);
    if backup.exists() {
        if target.exists() {
            fs::remove_file(&target)
                .with_context(|| format!("failed to remove `{}`", target.display()))?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create `{}`", parent.display()))?;
        }
        fs::rename(&backup, &target).with_context(|| {
            format!(
                "failed to restore `{}` from `{}`",
                target.display(),
                backup.display()
            )
        })?;
        return Ok(());
    }
    if !write.had_original
        && write
            .staged
            .as_ref()
            .is_some_and(|staged| !repository.join(staged).exists())
        && target.exists()
    {
        fs::remove_file(&target)
            .with_context(|| format!("failed to remove partial `{}`", target.display()))?;
    }
    Ok(())
}

fn repository_relative(repository: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(repository)
        .with_context(|| format!("`{}` is outside repository", path.display()))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("path `{}` is not UTF-8", path.display()))?
        .to_string())
}

fn sync_file(path: &Path) -> Result<()> {
    fs::File::open(path)
        .with_context(|| format!("failed to open `{}` for sync", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync `{}`", path.display()))?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)
        .with_context(|| format!("failed to open directory `{}` for sync", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync directory `{}`", path.display()))?;
    Ok(())
}

fn maybe_fail_after_transaction_write() -> Result<()> {
    TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return;
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            panic!("injected transaction interruption after committed write");
        }
    });
    Ok(())
}

fn maybe_fail_during_journal_replace() {
    TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return;
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            panic!("injected transaction interruption during journal replacement");
        }
    });
}

fn maybe_return_journal_error() -> Result<()> {
    TRANSACTION_RETURN_JOURNAL_ERROR_AFTER.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return Ok(());
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            bail!("injected transaction journal update error");
        }
        Ok(())
    })
}

fn maybe_return_staged_rename_error() -> Result<()> {
    TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return Ok(());
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            bail!("injected staged rename error after backup");
        }
        Ok(())
    })
}

fn maybe_fail_during_restore() -> Result<()> {
    TRANSACTION_FAIL_RESTORE.with(|fail| {
        if fail.replace(false) {
            bail!("injected transaction restore failure");
        }
        Ok(())
    })
}

fn canonicalize_existing_repository(repository: &Path) -> Result<PathBuf> {
    let canonical = repository
        .canonicalize()
        .with_context(|| format!("repository `{}` does not exist", repository.display()))?;
    if !canonical.is_dir() {
        bail!("repository `{}` is not a directory", canonical.display());
    }
    Ok(canonical)
}

fn resolve_repository_target(repository: &Path, artifact_path: &str) -> Result<PathBuf> {
    if artifact_path.trim().is_empty() {
        bail!("artifact path must not be blank");
    }
    let path = Path::new(artifact_path);
    if path.is_absolute() {
        bail!("artifact path `{artifact_path}` must be repository-relative");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir => bail!("artifact path `{artifact_path}` must not traverse"),
            _ => bail!("artifact path `{artifact_path}` is unsupported"),
        }
    }
    let normalized_text = normalized
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("artifact path `{artifact_path}` is not UTF-8"))?;
    if normalized_text.starts_with(".model-routing/") {
        bail!("artifact path `{artifact_path}` targets a reserved path");
    }
    if normalized_text == SETUP_CONFIG_PATH {
        return Ok(repository.join(normalized));
    }
    if !allowed_repository_target(normalized_text) {
        bail!("artifact path `{artifact_path}` is not an allowed host artifact path");
    }
    Ok(repository.join(normalized))
}

fn allowed_repository_target(path: &str) -> bool {
    if path == ".codex/config.toml" {
        return true;
    }
    [
        ".codex/agents/",
        ".claude/agents/",
        ".cursor/agents/",
        ".planr/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn ensure_parent_is_safe(repository: &Path, target: &Path) -> Result<()> {
    let mut current = repository.to_path_buf();
    let relative = target
        .strip_prefix(repository)
        .map_err(|_| anyhow::anyhow!("target escaped repository"))?;
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            let Component::Normal(part) = component else {
                bail!("artifact parent contains unsupported component");
            };
            current.push(part);
            if current.exists() {
                let metadata = fs::symlink_metadata(&current)
                    .with_context(|| format!("failed to inspect `{}`", current.display()))?;
                if metadata.file_type().is_symlink() {
                    bail!("artifact parent `{}` is a symlink", current.display());
                }
                if !metadata.is_dir() {
                    bail!("artifact parent `{}` is not a directory", current.display());
                }
            }
        }
    }
    Ok(())
}

fn manifest_from_bundle(
    bundle: &RoutingBundleV1,
    bundle_sha256: String,
    previous: Option<ManagedSnapshot>,
) -> ManagedManifest {
    ManagedManifest {
        schema_version: 1,
        bundle_id: bundle.bundle_id.clone(),
        bundle_sha256,
        artifacts: bundle
            .artifacts
            .iter()
            .map(|artifact| ManagedArtifact {
                path: artifact.path.clone(),
                sha256: artifact.sha256.clone(),
                content: Some(artifact.content.clone()),
            })
            .collect(),
        previous,
    }
}

fn manifest_from_plan(
    bundle_id: &str,
    bundle_sha256: String,
    planned: &[PlannedArtifact],
    previous: Option<ManagedSnapshot>,
) -> ManagedManifest {
    ManagedManifest {
        schema_version: 1,
        bundle_id: bundle_id.to_string(),
        bundle_sha256,
        artifacts: planned
            .iter()
            .filter(|artifact| artifact.status != "removed")
            .map(|artifact| ManagedArtifact {
                path: artifact.path.clone(),
                sha256: artifact.sha256.clone(),
                content: artifact.managed_content.clone(),
            })
            .collect(),
        previous,
    }
}

fn snapshot_from_manifest(manifest: &ManagedManifest) -> ManagedSnapshot {
    ManagedSnapshot {
        bundle_id: manifest.bundle_id.clone(),
        bundle_sha256: manifest.bundle_sha256.clone(),
        artifacts: manifest.artifacts.clone(),
    }
}

fn write_manifest_file(repository: &Path, manifest: &ManagedManifest) -> Result<()> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create `{}`", parent.display()))?;
    }
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("failed to write `{}`", manifest_path.display()))?;
    Ok(())
}

fn remove_manifest(repository: &Path) -> Result<()> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if manifest_path.exists() {
        fs::remove_file(&manifest_path)
            .with_context(|| format!("failed to remove `{}`", manifest_path.display()))?;
    }
    Ok(())
}

fn read_manifest(repository: &Path) -> Result<Option<ManagedManifest>> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if !manifest_path.exists() {
        return Ok(None);
    }
    let input = fs::read(&manifest_path)
        .with_context(|| format!("failed to read `{}`", manifest_path.display()))?;
    Ok(Some(serde_json::from_slice(&input).with_context(|| {
        format!("failed to parse `{}`", manifest_path.display())
    })?))
}

fn report_from_plan(
    action: &str,
    repository: &Path,
    bundle_id: Option<&str>,
    planned: &[PlannedArtifact],
) -> LifecycleReport {
    LifecycleReport {
        action: action.to_string(),
        bundle_id: bundle_id.map(ToOwned::to_owned),
        repository: repository.display().to_string(),
        artifacts: planned
            .iter()
            .map(|artifact| LifecycleArtifactReport {
                path: artifact.path.clone(),
                mode: artifact.mode.clone(),
                status: artifact.status.clone(),
                sha256: artifact.sha256.clone(),
                repair: repair_for_status(&artifact.status),
            })
            .collect(),
    }
}

fn repair_for_status(status: &str) -> Option<String> {
    match status {
        "modified" | "preserved-modified" => Some(
            "user-modified file preserved; run update or rollback after reconciling local edits"
                .to_string(),
        ),
        "missing" => Some(
            "managed file is missing; run update to recreate or uninstall to drop ownership"
                .to_string(),
        ),
        _ => None,
    }
}

fn validate_raw_bundle_shape(value: &Value) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("bundle root must be a JSON object"))?;
    let schema_version = object
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("bundle schema_version must be an integer"))?;
    if schema_version != 1 {
        bail!("unsupported schema_version {schema_version}");
    }
    let allowed_root = BTreeSet::from([
        "schema_version",
        "bundle_id",
        "policy_id",
        "policy_version",
        "generated_at",
        "source",
        "policy",
        "requirements",
        "profiles",
        "routes",
        "route_default",
        "artifacts",
        "evidence",
    ]);
    for key in object.keys() {
        if !allowed_root.contains(key.as_str()) {
            bail!("unknown bundle field `{key}`");
        }
    }
    let source = object
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("bundle source must be an object"))?;
    if !source.contains_key("integration") {
        bail!("bundle source.integration is required");
    }
    let artifacts = object
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle artifacts must be an array"))?;
    let allowed_artifact = BTreeSet::from(["path", "media_type", "mode", "content", "sha256"]);
    for artifact in artifacts {
        let artifact_object = artifact
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("bundle artifact must be an object"))?;
        let path = artifact_object
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        if artifact_object.contains_key("content") && artifact_object.contains_key("content_ref") {
            bail!("artifact `{path}` cannot define both content and content_ref");
        }
        for key in artifact_object.keys() {
            if !allowed_artifact.contains(key.as_str()) {
                bail!("artifact `{path}` has unknown field `{key}`");
            }
        }
    }
    Ok(())
}

fn validate_policy_contract(policy: &PolicyContract) -> Result<()> {
    if policy.schema_version != 1 {
        bail!(
            "unsupported policy schema_version {}",
            policy.schema_version
        );
    }
    if policy.id.trim().is_empty() || policy.version.trim().is_empty() {
        bail!("policy id and version must not be blank");
    }
    if policy.usage.max_parallel_writers > policy.usage.max_active_agents {
        bail!("policy max_parallel_writers cannot exceed max_active_agents");
    }
    if policy.usage.max_parallel_readers > policy.usage.max_active_agents {
        bail!("policy max_parallel_readers cannot exceed max_active_agents");
    }
    if policy.usage.review_reserve_percent > 100 {
        bail!("policy review_reserve_percent cannot exceed 100");
    }
    if policy.execution.max_write_scope_entries == 0 {
        for role in policy.execution.roles.values() {
            if !role.filesystem.write_roots.is_empty() {
                bail!("policy with zero write scope cannot declare writable roots");
            }
        }
    }
    Ok(())
}

fn validate_source(source: &PolicySource) -> Result<()> {
    if source.policy_id.trim().is_empty() || source.host.trim().is_empty() {
        bail!("routing policy id and host must not be blank");
    }
    for route in &source.routes {
        if !source.profiles.contains_key(&route.profile) {
            bail!("route references unknown profile `{}`", route.profile);
        }
    }
    if let Some(default) = &source.route_default {
        if !source.profiles.contains_key(&default.profile) {
            bail!(
                "default route references unknown profile `{}`",
                default.profile
            );
        }
    }
    if source.evidence.status == "recommended" {
        bail!("policy sources cannot claim recommended without the evaluation gate");
    }
    for profile in source.profiles.values() {
        validate_profile_fork_policy(profile)?;
    }
    validate_policy_contract(&source.policy)?;
    Ok(())
}

fn source_from_setup_spec(spec: &SetupSpecV1) -> Result<PolicySource> {
    validate_setup_spec(spec)?;
    let binding = binding_for_selector(&spec.host)?;
    let mut source = show_policy(&spec.usage_policy, &binding.id)?;
    if setup_matches_binding(spec, &binding)? {
        return Ok(source);
    }
    let runtime_host = setup_runtime_host(&binding);
    let model_catalog = setup_model_catalog(runtime_host);
    let profiles = spec
        .selected_roles
        .iter()
        .map(|(role, selection)| {
            let option = model_catalog
                .iter()
                .find(|option| option.id == selection.model)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "setup role `{role}` model `{}` is not supported by host `{}`",
                        selection.model,
                        spec.host
                    )
                })?;
            if runtime_host == "codex"
                && selection.spawn.is_none()
                && selection_matches_binding_profile(role, selection, &binding)
            {
                return Ok((
                    role.clone(),
                    profile_from_binding_profile(binding.profiles.get(role).ok_or_else(|| {
                        anyhow::anyhow!("setup role `{role}` is missing from binding")
                    })?),
                ));
            }
            let agent_type = if runtime_host == "codex" {
                Some(
                    selection
                        .spawn
                        .as_ref()
                        .ok_or_else(|| {
                            anyhow::anyhow!("setup role `{role}` must declare Codex spawn policy")
                        })?
                        .agent_type
                        .clone(),
                )
            } else {
                None
            };
            Ok((
                role.clone(),
                Profile {
                    client: runtime_host.to_string(),
                    model: selection.model.clone(),
                    agent_type,
                    effort: selection.effort.clone(),
                    cost_tier: Some(option.tier.to_string()),
                    capabilities: Vec::new(),
                    skill: None,
                    notes: Some("custom SetupSpecV1 role".to_string()),
                    fork_turns: selection
                        .spawn
                        .as_ref()
                        .map(|spawn| spawn.fork_turns.clone()),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let routes = spec
        .routes
        .iter()
        .map(|route| Route {
            selector: RouteSelector {
                work_type: Some(route.work_type.clone()),
                plan: None,
            },
            profile: route.role.clone(),
            fallbacks: route.fallbacks.clone(),
        })
        .collect();
    let route_default = spec.route_default.as_ref().map(|default| DefaultRoute {
        profile: default.role.clone(),
        fallbacks: default.fallbacks.clone(),
    });
    source.profiles = profiles;
    source.routes = routes;
    source.route_default = route_default;
    source.artifacts = render_setup_artifacts(
        runtime_host,
        &spec.selected_roles,
        &binding,
        &source.artifacts,
    )?;
    source.evidence = EvaluationEvidence {
        evaluation_ids: Vec::new(),
        status: "custom-unverified".to_string(),
    };
    Ok(source)
}

fn setup_matches_binding(spec: &SetupSpecV1, binding: &HostBinding) -> Result<bool> {
    if canonical_binding_id(&spec.host) != binding.id {
        return Ok(false);
    }
    if spec.selected_roles.len() != binding.profiles.len() {
        return Ok(false);
    }
    for (role, binding_profile) in &binding.profiles {
        let Some(selection) = spec.selected_roles.get(role) else {
            return Ok(false);
        };
        if selection.model != binding_profile.model
            || selection.effort != binding_profile.effort
            || !selection_spawn_matches_binding(
                setup_runtime_host(binding),
                role,
                selection,
                binding_profile,
            )
        {
            return Ok(false);
        }
    }
    if spec.routes.len() != binding.routes.len() {
        return Ok(false);
    }
    for (setup_route, binding_route) in spec.routes.iter().zip(binding.routes.iter()) {
        if setup_route.work_type != binding_route.work_type
            || setup_route.role != binding_route.role
            || setup_route.fallbacks != binding_route.fallback_roles
        {
            return Ok(false);
        }
    }
    Ok(match (&spec.route_default, &binding.default_role) {
        (None, None) => true,
        (Some(setup), Some(binding_role)) => {
            setup.role == *binding_role && setup.fallbacks.is_empty()
        }
        _ => false,
    })
}

fn render_setup_artifacts(
    runtime_host: &str,
    roles: &BTreeMap<String, SetupRoleSelection>,
    binding: &HostBinding,
    binding_artifacts: &[SourceArtifact],
) -> Result<Vec<SourceArtifact>> {
    roles
        .iter()
        .map(|(role, selection)| {
            if runtime_host == "codex"
                && selection.spawn.is_none()
                && selection_matches_binding_profile(role, selection, binding)
            {
                return binding_artifact_for_role(binding, binding_artifacts, role);
            }
            let file_role = identifier_token(role);
            let path = setup_artifact_path(runtime_host, role, selection)?;
            let (kind, content) = match runtime_host {
                "codex" => {
                    let spawn = selection.spawn.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("setup role `{role}` must declare Codex spawn policy")
                    })?;
                    let agent_type = spawn.agent_type.clone();
                    let effort = selection
                        .effort
                        .clone()
                        .unwrap_or_else(|| "medium".to_string());
                    (
                        "codex_agent",
                        format!(
                            "name = \"{agent_type}\"\ndescription = \"Switchloom custom {role} role.\"\nmodel = \"{}\"\nmodel_reasoning_effort = \"{effort}\"\n\ndeveloper_instructions = \"\"\"\nSpawn with agent_type `{agent_type}`, task_name `{}`, and fork_turns `{}`. The live parent permission profile remains authoritative; this role declares routing intent and expected ownership evidence, not filesystem permission enforcement.\n\"\"\"\n",
                            selection.model, spawn.task_name, spawn.fork_turns.mode
                        ),
                    )
                }
                "claude-code" => {
                    let effort = selection
                        .effort
                        .clone()
                        .unwrap_or_else(|| "medium".to_string());
                    (
                        "claude_agent",
                        format!(
                            "---\nname: switchloom-{file_role}\nmodel: {}\neffort: {effort}\n---\nFollow the repository-local Switchloom setup role `{role}` and preserve routing evidence.\n",
                            selection.model
                        ),
                    )
                }
                "cursor" => {
                    (
                        "cursor_agent",
                        format!(
                            "---\nname: switchloom-{file_role}\nmodel: {}\n---\nFollow the repository-local Switchloom setup role `{role}` and preserve routing evidence.\n",
                            selection.model
                        ),
                    )
                }
                "mixed-host" => {
                    (
                        "routing_role",
                        format!(
                            "role = \"{role}\"\nmodel = \"{}\"\n{}\n",
                            selection.model,
                            selection
                                .effort
                                .as_ref()
                                .map(|effort| format!("effort = \"{effort}\""))
                                .unwrap_or_default()
                        ),
                    )
                }
                other => bail!("unsupported setup runtime host `{other}`"),
            };
            let media_type = media_type_for(&path, kind);
            Ok(SourceArtifact {
                path,
                media_type,
                mode: "replace".to_string(),
                content,
            })
        })
        .collect()
}

fn profile_from_binding_profile(profile: &BindingProfile) -> Profile {
    Profile {
        client: profile.client.clone(),
        model: profile.model.clone(),
        agent_type: profile.agent_type.clone(),
        effort: profile.effort.clone(),
        cost_tier: profile.cost_tier.clone(),
        capabilities: Vec::new(),
        skill: None,
        notes: None,
        fork_turns: profile.fork_turns.clone(),
    }
}

fn binding_artifact_for_role(
    binding: &HostBinding,
    artifacts: &[SourceArtifact],
    role: &str,
) -> Result<SourceArtifact> {
    let profile = binding
        .profiles
        .get(role)
        .ok_or_else(|| anyhow::anyhow!("setup role `{role}` is missing from binding"))?;
    let agent_type = profile
        .agent_type
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("setup role `{role}` has no binding agent_type"))?;
    artifacts
        .iter()
        .find(|artifact| {
            artifact
                .content
                .contains(&format!("name = \"{agent_type}\""))
        })
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("binding role `{role}` has no generated host artifact"))
}

fn binding_artifact_path_for_role(binding: &HostBinding, role: &str) -> Result<String> {
    let profile = binding
        .profiles
        .get(role)
        .ok_or_else(|| anyhow::anyhow!("setup role `{role}` is missing from binding"))?;
    let agent_type = profile
        .agent_type
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("setup role `{role}` has no binding agent_type"))?;
    binding
        .artifacts
        .iter()
        .find(|artifact| {
            artifact
                .content
                .contains(&format!("name = \"{agent_type}\""))
        })
        .map(|artifact| artifact.path.clone())
        .ok_or_else(|| anyhow::anyhow!("binding role `{role}` has no generated host artifact"))
}

fn validate_profile_fork_policy(profile: &Profile) -> Result<()> {
    let requires_explicit_fork = profile.client == "codex"
        && profile.agent_type.is_some()
        && profile.agent_type.as_deref() != Some("model_routing_sol_medium");
    if !requires_explicit_fork {
        return Ok(());
    }
    let Some(fork_turns) = &profile.fork_turns else {
        bail!(
            "codex profile `{}` must declare fork_turns none or positive bounded when overriding model or effort",
            profile.agent_type.as_deref().unwrap_or("<unknown>")
        );
    };
    match fork_turns.mode.as_str() {
        "none" => Ok(()),
        "bounded" => match fork_turns.turns {
            Some(turns) if turns > 0 => Ok(()),
            _ => bail!(
                "codex profile `{}` bounded fork_turns must use positive turns",
                profile.agent_type.as_deref().unwrap_or("<unknown>")
            ),
        },
        "all" => bail!(
            "codex profile `{}` must not use fork_turns all with model or effort overrides",
            profile.agent_type.as_deref().unwrap_or("<unknown>")
        ),
        other => bail!(
            "codex profile `{}` has unsupported fork_turns mode `{other}`",
            profile.agent_type.as_deref().unwrap_or("<unknown>")
        ),
    }
}

fn binding_profile_id<'a>(binding: &'a HostBinding, role: &str) -> Result<&'a str> {
    binding
        .profiles
        .get(role)
        .map(|profile| profile.profile.as_str())
        .ok_or_else(|| anyhow::anyhow!("binding route references unknown role `{role}`"))
}

fn binding_for_selector(selector: &str) -> Result<HostBinding> {
    let binding_id = canonical_binding_id(selector);
    let raw = BINDINGS
        .iter()
        .find(|(id, _)| *id == binding_id)
        .map(|(_, raw)| *raw)
        .ok_or_else(|| anyhow::anyhow!("unknown setup host `{selector}`"))?;
    Ok(toml::from_str(raw)?)
}

fn canonical_binding_id(selector: &str) -> &str {
    match selector {
        "codex" => "codex-openai",
        "claude-code" => "claude-native",
        "cursor" => "cursor-openai",
        other => other,
    }
}

fn setup_runtime_host(binding: &HostBinding) -> &str {
    binding.host.as_str()
}

#[derive(Debug, Clone, Copy)]
struct SetupModelOption {
    id: &'static str,
    efforts: &'static [&'static str],
    tier: &'static str,
}

fn setup_model_catalog(host: &str) -> Vec<SetupModelOption> {
    match host {
        "codex" => vec![
            SetupModelOption {
                id: "gpt-5.6-sol",
                efforts: &["low", "medium", "high", "xhigh", "ultra"],
                tier: "premium",
            },
            SetupModelOption {
                id: "gpt-5.6-terra",
                efforts: &["low", "medium", "high", "xhigh", "ultra"],
                tier: "standard",
            },
            SetupModelOption {
                id: "gpt-5.6-luna",
                efforts: &["low", "medium", "high", "xhigh"],
                tier: "standard",
            },
        ],
        "cursor" => vec![
            SetupModelOption {
                id: "gpt-5.6-sol",
                efforts: &["low", "medium", "high", "xhigh", "max"],
                tier: "premium",
            },
            SetupModelOption {
                id: "gpt-5.6-terra",
                efforts: &["low", "medium", "high", "xhigh", "max"],
                tier: "standard",
            },
            SetupModelOption {
                id: "gpt-5.6-luna",
                efforts: &["low", "medium", "high", "xhigh", "max"],
                tier: "standard",
            },
            SetupModelOption {
                id: "fable-5",
                efforts: &["low", "medium", "high", "xhigh", "max"],
                tier: "premium",
            },
            SetupModelOption {
                id: "claude-opus-4-8",
                efforts: &["low", "medium", "high", "xhigh", "max"],
                tier: "premium",
            },
            SetupModelOption {
                id: "claude-sonnet-5",
                efforts: &["low", "medium", "high", "xhigh", "max"],
                tier: "standard",
            },
            SetupModelOption {
                id: "grok-4.5",
                efforts: &["low", "medium", "high"],
                tier: "premium",
            },
            SetupModelOption {
                id: "composer-2.5",
                efforts: &[],
                tier: "standard",
            },
        ],
        "claude-code" => vec![
            SetupModelOption {
                id: "opus",
                efforts: &["medium", "high"],
                tier: "premium",
            },
            SetupModelOption {
                id: "sonnet",
                efforts: &["medium", "high"],
                tier: "standard",
            },
        ],
        "mixed-host" => vec![
            SetupModelOption {
                id: "gpt-5.6-sol",
                efforts: &["medium", "high", "xhigh"],
                tier: "premium",
            },
            SetupModelOption {
                id: "gpt-5.6-terra",
                efforts: &["low", "medium", "high"],
                tier: "standard",
            },
            SetupModelOption {
                id: "opus",
                efforts: &["high"],
                tier: "premium",
            },
            SetupModelOption {
                id: "sonnet",
                efforts: &["medium"],
                tier: "standard",
            },
        ],
        _ => Vec::new(),
    }
}

fn validate_model_effort(
    host: &str,
    role: &str,
    selection: &SetupRoleSelection,
    catalog: &[SetupModelOption],
) -> Result<()> {
    let option = catalog
        .iter()
        .find(|option| option.id == selection.model)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "setup role `{role}` model `{}` is not supported by host `{host}`",
                selection.model
            )
        })?;
    match (&selection.effort, option.efforts.is_empty()) {
        (None, true) => Ok(()),
        (Some(_), true) => bail!(
            "setup role `{role}` model `{}` does not accept effort",
            selection.model
        ),
        (None, false) => bail!(
            "setup role `{role}` model `{}` requires effort",
            selection.model
        ),
        (Some(effort), false) if option.efforts.contains(&effort.as_str()) => Ok(()),
        (Some(effort), false) => bail!(
            "setup role `{role}` effort `{effort}` is not supported for model `{}` on host `{host}`",
            selection.model
        ),
    }
}

fn selection_matches_binding_profile(
    role: &str,
    selection: &SetupRoleSelection,
    binding: &HostBinding,
) -> bool {
    binding.profiles.get(role).is_some_and(|profile| {
        selection.model == profile.model
            && selection.effort == profile.effort
            && selection_spawn_matches_binding(
                setup_runtime_host(binding),
                role,
                selection,
                profile,
            )
    })
}

fn setup_spawn_policy_for_binding_role(
    runtime_host: &str,
    role: &str,
    profile: &BindingProfile,
) -> Option<SetupSpawnPolicy> {
    if runtime_host != "codex" {
        return None;
    }
    Some(SetupSpawnPolicy {
        agent_type: profile.agent_type.clone()?,
        task_name: identifier_token(role),
        fork_turns: profile.fork_turns.clone()?,
    })
}

fn selection_spawn_matches_binding(
    runtime_host: &str,
    role: &str,
    selection: &SetupRoleSelection,
    profile: &BindingProfile,
) -> bool {
    if runtime_host != "codex" {
        return selection.spawn.is_none();
    }
    match (&selection.spawn, &profile.agent_type, &profile.fork_turns) {
        (None, None, None) => true,
        (None, Some(_), None) => true,
        (Some(spawn), Some(agent_type), Some(fork_turns)) => {
            spawn.agent_type == *agent_type
                && spawn.task_name == identifier_token(role)
                && spawn.fork_turns == *fork_turns
        }
        _ => false,
    }
}

fn validate_setup_spawn_policy(
    runtime_host: &str,
    role: &str,
    selection: &SetupRoleSelection,
    matches_binding: bool,
) -> Result<()> {
    if runtime_host != "codex" {
        if selection.spawn.is_some() {
            bail!("setup role `{role}` spawn policy is only supported for Codex hosts");
        }
        return Ok(());
    }
    if matches_binding && selection.spawn.is_none() {
        return Ok(());
    }
    let Some(spawn) = &selection.spawn else {
        bail!(
            "setup role `{role}` must declare Codex spawn policy with exact agent_type, task_name, and fork_turns"
        );
    };
    if spawn.task_name.contains('/') || spawn.task_name.starts_with('.') {
        bail!(
            "setup role `{role}` task_name must be a local lowercase identifier, not a canonical task path"
        );
    }
    validate_setup_snake_identifier("agent_type", &spawn.agent_type)?;
    validate_setup_snake_identifier("task_name", &spawn.task_name)?;
    let expected_task_name = identifier_token(role);
    if spawn.task_name != expected_task_name {
        bail!(
            "setup role `{role}` task_name `{}` must match `{expected_task_name}`",
            spawn.task_name
        );
    }
    if spawn.agent_type.trim().is_empty() {
        bail!("setup role `{role}` agent_type must not be blank");
    }
    let fork_turns = &spawn.fork_turns;
    {
        match fork_turns.mode.as_str() {
            "none" => {
                if fork_turns.turns.is_some() {
                    bail!("setup role `{role}` fork_turns none must not declare turns");
                }
            }
            "bounded" => match fork_turns.turns {
                Some(turns) if turns > 0 => {}
                _ => bail!("setup role `{role}` bounded fork_turns must use positive turns"),
            },
            "all" => {
                bail!("setup role `{role}` must not use fork_turns all for Codex role overrides")
            }
            other => bail!("setup role `{role}` has unsupported fork_turns mode `{other}`"),
        }
    }
    Ok(())
}

fn validate_setup_snake_identifier(kind: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if !valid {
        bail!("setup {kind} `{value}` must use lowercase ASCII letters, digits, or `_`");
    }
    reject_setup_secret_like(kind, value)
}

fn validate_setup_identifier(kind: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        });
    if !valid {
        bail!("setup {kind} `{value}` must use lowercase ASCII letters, digits, `_`, or `-`");
    }
    reject_setup_secret_like(kind, value)
}

fn validate_setup_identity_collisions(
    spec: &SetupSpecV1,
    runtime_host: &str,
    binding: &HostBinding,
) -> Result<()> {
    let mut normalized_roles = BTreeMap::<String, String>::new();
    let mut artifact_paths = BTreeMap::<String, String>::new();
    let mut codex_agent_types = BTreeMap::<String, String>::new();
    let mut codex_task_names = BTreeMap::<String, String>::new();
    for (role, selection) in &spec.selected_roles {
        let normalized = identifier_token(role);
        if let Some(existing) = normalized_roles.insert(normalized.clone(), role.clone()) {
            bail!("setup roles `{existing}` and `{role}` both normalize to `{normalized}`");
        }
        if runtime_host == "codex" {
            let agent_type = if let Some(spawn) = &selection.spawn {
                spawn.agent_type.clone()
            } else if selection_matches_binding_profile(role, selection, binding) {
                binding
                    .profiles
                    .get(role)
                    .and_then(|profile| profile.agent_type.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            if !agent_type.is_empty() {
                if let Some(existing) = codex_agent_types.insert(agent_type.clone(), role.clone()) {
                    bail!(
                        "setup roles `{existing}` and `{role}` both declare Codex agent_type `{agent_type}`"
                    );
                }
            }
            if let Some(spawn) = &selection.spawn {
                if let Some(existing) =
                    codex_task_names.insert(spawn.task_name.clone(), role.clone())
                {
                    bail!(
                        "setup roles `{existing}` and `{role}` both declare Codex task_name `{}`",
                        spawn.task_name
                    );
                }
            }
        }
        let artifact_path = if runtime_host == "codex"
            && selection.spawn.is_none()
            && selection_matches_binding_profile(role, selection, binding)
        {
            Some(binding_artifact_path_for_role(binding, role)?)
        } else if runtime_host != "codex" || selection.spawn.is_some() {
            Some(setup_artifact_path(runtime_host, role, selection)?)
        } else {
            None
        };
        if let Some(artifact_path) = artifact_path {
            if let Some(existing) = artifact_paths.insert(artifact_path.clone(), role.clone()) {
                bail!(
                    "setup roles `{existing}` and `{role}` both generate artifact path `{artifact_path}`"
                );
            }
        }
    }
    Ok(())
}

fn setup_artifact_path(
    runtime_host: &str,
    role: &str,
    selection: &SetupRoleSelection,
) -> Result<String> {
    let file_role = identifier_token(role);
    Ok(match runtime_host {
        "codex" => {
            let spawn = selection.spawn.as_ref().ok_or_else(|| {
                anyhow::anyhow!("setup role `{role}` must declare Codex spawn policy")
            })?;
            format!(".codex/agents/{}.toml", spawn.agent_type)
        }
        "claude-code" => format!(".claude/agents/switchloom-{file_role}.md"),
        "cursor" => format!(".cursor/agents/switchloom-{file_role}.md"),
        "mixed-host" => format!(".model-routing/roles/{file_role}.toml"),
        other => bail!("unsupported setup runtime host `{other}`"),
    })
}

fn validate_setup_route_role(
    roles: &BTreeMap<String, SetupRoleSelection>,
    role: &str,
) -> Result<()> {
    if !roles.contains_key(role) {
        bail!("setup route references unknown role `{role}`");
    }
    Ok(())
}

fn reject_setup_secret_like(kind: &str, value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    for token in [
        "api_key",
        "apikey",
        "token",
        "secret",
        "credential",
        "password",
    ] {
        if lower.contains(token) {
            bail!("setup {kind} must not contain credential-like token `{token}`");
        }
    }
    Ok(())
}

fn identifier_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn media_type_for(path: &str, kind: &str) -> String {
    if path.ends_with(".toml") {
        "application/toml"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".md") || kind.ends_with("_skill") || kind.ends_with("_agent") {
        "text/markdown"
    } else {
        "text/plain"
    }
    .to_string()
}

fn include_artifact_for_integration(artifact: &SourceArtifact, integration: Integration) -> bool {
    if artifact.path.contains("/skills/")
        || artifact
            .content
            .contains("name: model-routing-native-routing")
    {
        return false;
    }
    integration == Integration::Planr || !artifact.path.starts_with(".planr/")
}

fn artifact_for_integration(
    mut artifact: SourceArtifact,
    integration: Integration,
) -> SourceArtifact {
    if integration == Integration::Planr {
        artifact.content = render_planr_native_role(&artifact);
    }
    artifact
}

fn render_codex_agent_registration_artifact(
    artifacts: &[SourceArtifact],
) -> Result<Option<SourceArtifact>> {
    #[derive(Serialize)]
    struct CodexAgentRegistrationConfig {
        agents: BTreeMap<String, CodexAgentRegistration>,
    }

    #[derive(Serialize)]
    struct CodexAgentRegistration {
        config_file: String,
    }

    let mut agents = BTreeMap::new();
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.path.starts_with(".codex/agents/"))
    {
        let parsed: toml::Value = toml::from_str(&artifact.content)
            .with_context(|| format!("Codex agent artifact `{}` must be TOML", artifact.path))?;
        let agent_type = parsed
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                anyhow::anyhow!("Codex agent artifact `{}` must declare name", artifact.path)
            })?;
        let Some(file_name) = artifact.path.strip_prefix(".codex/") else {
            bail!(
                "Codex agent artifact `{}` must be relative to .codex",
                artifact.path
            );
        };
        if let Some(existing) = agents.insert(
            agent_type.to_string(),
            CodexAgentRegistration {
                config_file: format!("./{file_name}"),
            },
        ) {
            bail!(
                "Codex agent_type `{agent_type}` is registered by multiple artifacts, including `{}`",
                existing.config_file
            );
        }
    }
    if agents.is_empty() {
        return Ok(None);
    }
    let mut content = toml::to_string_pretty(&CodexAgentRegistrationConfig { agents })?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(Some(SourceArtifact {
        path: ".codex/config.toml".to_string(),
        media_type: "application/toml".to_string(),
        mode: "replace".to_string(),
        content,
    }))
}

fn render_planr_native_role(artifact: &SourceArtifact) -> String {
    let protocol = if is_reviewer_role(artifact) {
        Some((
            "$planr-review",
            "Use the existing Planr internal review protocol for exactly one Planr review item. Read the pick packet, audit the target item and evidence, report findings first, and return the review verdict through Planr. Do not create or invoke any routing-specific, goal, or loop workflow skill. Planr users enter only through $planr-goal and $planr-loop.",
        ))
    } else if is_worker_role(artifact) {
        Some((
            "$planr-work",
            "Use the existing Planr internal worker protocol for exactly one picked Planr item. Read the pick packet, implement only that item, log changed files and real verification commands, request review through Planr, and stop. Do not create or invoke any routing-specific, goal, or loop workflow skill. Planr users enter only through $planr-goal and $planr-loop.",
        ))
    } else {
        None
    };
    let Some((protocol_name, instructions)) = protocol else {
        return artifact.content.clone();
    };
    if artifact.path.starts_with(".codex/agents/") {
        rewrite_codex_developer_instructions(&artifact.content, protocol_name, instructions)
    } else {
        rewrite_markdown_agent_body(&artifact.content, protocol_name, instructions)
    }
}

fn is_worker_role(artifact: &SourceArtifact) -> bool {
    artifact.path.contains("terra-high")
        || artifact.path.contains("luna-xhigh")
        || artifact.path.contains("preset-worker")
        || artifact.path.contains("implementer")
        || artifact.content.contains("Normal implementation")
        || artifact.content.contains("Bounded checklist")
        || artifact.content.contains("custom implementer role")
}

fn is_reviewer_role(artifact: &SourceArtifact) -> bool {
    artifact.path.contains("sol-high")
        || artifact.path.contains("reviewer")
        || artifact.path.contains("verifier")
        || artifact.content.contains("Independent final review")
        || artifact.content.contains("custom reviewer role")
        || artifact.content.contains("custom verifier role")
}

fn rewrite_codex_developer_instructions(
    content: &str,
    protocol_name: &str,
    instructions: &str,
) -> String {
    let marker = "developer_instructions = \"\"\"\n";
    if let Some(start) = content.find(marker) {
        let body_start = start + marker.len();
        if let Some(end) = content[body_start..].find("\n\"\"\"") {
            let body_end = body_start + end;
            let mut output = String::new();
            output.push_str(&content[..body_start]);
            output.push_str(instructions);
            output.push_str("\n\nProtocol preload: ");
            output.push_str(protocol_name);
            output.push_str(&content[body_end..]);
            return output;
        }
    }
    format!("{content}\n\nProtocol preload: {protocol_name}\n{instructions}\n")
}

fn rewrite_markdown_agent_body(content: &str, protocol_name: &str, instructions: &str) -> String {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let split = "---\n".len() + end + "\n---\n".len();
            return format!(
                "{}Protocol preload: {}\n\n{}\n",
                &content[..split],
                protocol_name,
                instructions
            );
        }
    }
    format!("Protocol preload: {protocol_name}\n\n{instructions}\n")
}

fn render_registry(source: &PolicySource) -> Result<String> {
    #[derive(Serialize)]
    struct Registry {
        profiles: BTreeMap<String, PlanrRegistryProfile>,
        routes: Vec<Route>,
        #[serde(skip_serializing_if = "Option::is_none")]
        route_default: Option<DefaultRoute>,
    }
    #[derive(Serialize)]
    struct PlanrRegistryProfile {
        client: String,
        model: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effort: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cost_tier: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        capabilities: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    }
    let profile_key = |profile_id: &str| -> String {
        source
            .profiles
            .get(profile_id)
            .and_then(|profile| profile.agent_type.clone())
            .unwrap_or_else(|| profile_id.to_string())
    };
    let profiles = source
        .profiles
        .iter()
        .map(|(id, profile)| {
            (
                profile_key(id),
                PlanrRegistryProfile {
                    client: profile.client.clone(),
                    model: profile.model.clone(),
                    effort: profile.effort.clone(),
                    cost_tier: profile.cost_tier.clone(),
                    capabilities: profile.capabilities.clone(),
                    skill: profile.skill.clone(),
                    notes: profile
                        .agent_type
                        .as_ref()
                        .map(|agent_type| format!("native_agent_type={agent_type}"))
                        .or_else(|| profile.notes.clone()),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let routes = source
        .routes
        .iter()
        .map(|route| Route {
            selector: route.selector.clone(),
            profile: profile_key(&route.profile),
            fallbacks: route
                .fallbacks
                .iter()
                .map(|fallback| profile_key(fallback))
                .collect(),
        })
        .collect::<Vec<_>>();
    let route_default = source.route_default.as_ref().map(|default| DefaultRoute {
        profile: profile_key(&default.profile),
        fallbacks: default
            .fallbacks
            .iter()
            .map(|fallback| profile_key(fallback))
            .collect(),
    });
    Ok(toml::to_string_pretty(&Registry {
        profiles,
        routes,
        route_default,
    })?)
}

fn bundle_artifact(source: SourceArtifact) -> BundleArtifact {
    BundleArtifact {
        sha256: sha256(source.content.as_bytes()),
        path: source.path,
        media_type: source.media_type,
        mode: source.mode,
        content: source.content,
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
        output
    })
}

fn decode_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2 {
        return None;
    }
    let mut decoded = [0_u8; N];
    for (index, output) in decoded.iter_mut().enumerate() {
        *output = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(decoded)
}

fn encode_base64url(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(first >> 2) as usize] as char);
        output.push(TABLE[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((second & 0b0000_1111) << 2) | (third >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(TABLE[(third & 0b0011_1111) as usize] as char);
        }
    }
    output
}

const fn encoded_base64url_len(decoded_len: usize) -> usize {
    let full_chunks = decoded_len / 3;
    match decoded_len % 3 {
        0 => full_chunks * 4,
        1 => full_chunks * 4 + 2,
        _ => full_chunks * 4 + 3,
    }
}

fn validate_base64url_payload_len(input: &str) -> Result<()> {
    if input.len() > MAX_SETUP_RECIPE_ENCODED_BYTES {
        bail!(
            "setup recipe payload exceeds {MAX_SETUP_RECIPE_ENCODED_BYTES} base64url characters for {MAX_SETUP_RECIPE_BYTES} decoded bytes"
        );
    }
    Ok(())
}

fn decode_base64url(input: &str) -> Result<Vec<u8>> {
    validate_base64url_payload_len(input)?;
    if input
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
    {
        bail!("setup recipe payload must be unpadded base64url");
    }
    let mut sextets = Vec::with_capacity(input.len());
    for byte in input.bytes() {
        sextets.push(match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => unreachable!(),
        });
    }
    if sextets.len() % 4 == 1 {
        bail!("setup recipe payload has invalid base64url length");
    }
    let mut output = Vec::with_capacity(sextets.len() / 4 * 3);
    for chunk in sextets.chunks(4) {
        let a = chunk[0];
        let b = *chunk
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("invalid base64url payload"))?;
        output.push((a << 2) | (b >> 4));
        if let Some(c) = chunk.get(2) {
            output.push(((b & 0b0000_1111) << 4) | (c >> 2));
            if let Some(d) = chunk.get(3) {
                output.push(((c & 0b0000_0011) << 6) | d);
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("model-routing-{name}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn complete_policy_binding_pool_compiles_deterministically() {
        let summaries = list_policies().unwrap();
        assert_eq!(summaries.len(), 20);
        for summary in summaries {
            let first =
                compile_json(&summary.policy_id, &summary.host, Integration::Standalone).unwrap();
            let second =
                compile_json(&summary.policy_id, &summary.host, Integration::Standalone).unwrap();
            assert_eq!(first, second);
            assert!(!first.contains(".planr/policy.toml"));
            assert!(first.contains("\"package\": \"model-routing\""));
        }
    }

    #[test]
    fn planr_integration_is_explicit_and_adds_planr_declarations() {
        let standalone = compile_json("balanced", "codex-openai", Integration::Standalone).unwrap();
        let planr = compile_json("balanced", "codex-openai", Integration::Planr).unwrap();
        assert!(!standalone.contains(".planr/agents.toml"));
        assert!(planr.contains(".planr/agents.toml"));
        assert!(planr.contains(".planr/policy.toml"));
    }

    #[test]
    fn setup_spec_roundtrips_through_canonical_toml_json_and_recipe() {
        let spec =
            setup_spec_for_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        let json = setup_spec_to_canonical_json(&spec).unwrap();
        let toml = setup_spec_to_canonical_toml(&spec).unwrap();
        let recipe = setup_spec_to_recipe(&spec).unwrap();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(toml.contains("schema_version = 1"));
        assert!(recipe.starts_with(SETUP_RECIPE_PREFIX));
        assert_eq!(setup_spec_from_json(&json).unwrap(), spec);
        assert_eq!(setup_spec_from_toml(&toml).unwrap(), spec);
        assert_eq!(setup_spec_from_recipe(&recipe).unwrap(), spec);
    }

    #[test]
    fn setup_recipe_enforces_exact_pre_decode_size_boundaries() {
        assert_eq!(MAX_SETUP_RECIPE_BYTES % 3, 1);
        assert_eq!(
            MAX_SETUP_RECIPE_ENCODED_BYTES,
            (MAX_SETUP_RECIPE_BYTES / 3) * 4 + 2
        );

        let boundary_payload = encode_base64url(&vec![0_u8; MAX_SETUP_RECIPE_BYTES]);
        assert_eq!(boundary_payload.len(), MAX_SETUP_RECIPE_ENCODED_BYTES);
        assert_eq!(
            decode_base64url(&boundary_payload).unwrap().len(),
            MAX_SETUP_RECIPE_BYTES
        );
        let boundary_error =
            setup_spec_from_recipe(&format!("{SETUP_RECIPE_PREFIX}{boundary_payload}"))
                .unwrap_err()
                .to_string();
        assert!(!boundary_error.contains("exceeds"));

        let first_oversized_payload = encode_base64url(&vec![0_u8; MAX_SETUP_RECIPE_BYTES + 1]);
        assert_eq!(
            first_oversized_payload.len(),
            MAX_SETUP_RECIPE_ENCODED_BYTES + 1
        );
        assert!(
            setup_spec_from_recipe(&format!("{SETUP_RECIPE_PREFIX}{first_oversized_payload}"))
                .unwrap_err()
                .to_string()
                .contains("exceeds")
        );

        let very_large_payload = "A".repeat(MAX_SETUP_RECIPE_ENCODED_BYTES * 4);
        assert!(
            setup_spec_from_recipe(&format!("{SETUP_RECIPE_PREFIX}{very_large_payload}"))
                .unwrap_err()
                .to_string()
                .contains("base64url characters")
        );
    }

    #[test]
    fn built_in_presets_compile_through_setup_spec_without_output_drift() {
        let spec = setup_spec_for_policy("balanced", "codex-openai", Integration::Planr).unwrap();
        assert_eq!(
            compile_setup_spec(&spec).unwrap(),
            compile_builtin_policy_direct("balanced", "codex-openai", Integration::Planr).unwrap()
        );
    }

    #[test]
    fn public_host_aliases_preserve_built_in_preset_output() {
        for (alias, binding) in [
            ("codex", "codex-openai"),
            ("cursor", "cursor-openai"),
            ("claude-code", "claude-native"),
        ] {
            let spec = setup_spec_for_policy("balanced", alias, Integration::Standalone).unwrap();
            assert_eq!(spec.host, binding);
            assert_eq!(
                compile_setup_spec(&spec).unwrap(),
                compile_builtin_policy_direct("balanced", binding, Integration::Standalone)
                    .unwrap()
            );
        }
    }

    #[test]
    fn codex_default_spec_can_be_partially_tuned_into_custom_standalone_bundle() {
        let mut spec = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
        let worker = spec.selected_roles.get_mut("worker").unwrap();
        worker.model = "gpt-5.6-sol".to_string();
        worker.effort = Some("medium".to_string());
        worker.spawn = Some(SetupSpawnPolicy {
            agent_type: "switchloom_worker".to_string(),
            task_name: "worker".to_string(),
            fork_turns: ForkPolicy {
                mode: "none".to_string(),
                turns: None,
            },
        });

        let bundle = compile_setup_spec(&spec).unwrap();
        validate_bundle(&bundle).unwrap();
        assert_eq!(bundle.source.integration, Integration::Standalone);
        assert_eq!(bundle.evidence.status, "custom-unverified");
        assert_eq!(
            bundle.profiles.get("driver").unwrap().agent_type.as_deref(),
            Some("model_routing_sol_medium")
        );
        assert!(bundle.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/agents/model-routing-sol-medium.toml"
                && artifact
                    .content
                    .contains("name = \"model_routing_sol_medium\"")
        }));
        assert!(bundle.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/agents/switchloom_worker.toml"
                && artifact.content.contains("name = \"switchloom_worker\"")
        }));
        assert!(
            bundle
                .artifacts
                .iter()
                .all(|artifact| !artifact.path.starts_with(".planr/"))
        );
    }

    #[test]
    fn codex_default_spec_can_be_partially_tuned_into_custom_planr_bundle() {
        let mut spec = setup_spec_for_policy("balanced", "codex", Integration::Planr).unwrap();
        let reviewer = spec.selected_roles.get_mut("reviewer").unwrap();
        reviewer.model = "gpt-5.6-terra".to_string();
        reviewer.effort = Some("high".to_string());
        reviewer.spawn = Some(SetupSpawnPolicy {
            agent_type: "switchloom_reviewer".to_string(),
            task_name: "reviewer".to_string(),
            fork_turns: ForkPolicy {
                mode: "none".to_string(),
                turns: None,
            },
        });

        let bundle = compile_setup_spec(&spec).unwrap();
        validate_bundle(&bundle).unwrap();
        assert_eq!(bundle.source.integration, Integration::Planr);
        assert_eq!(bundle.evidence.status, "custom-unverified");
        assert_eq!(
            bundle.profiles.get("driver").unwrap().agent_type.as_deref(),
            Some("model_routing_sol_medium")
        );
        assert!(bundle.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/agents/model-routing-sol-medium.toml"
                && artifact
                    .content
                    .contains("name = \"model_routing_sol_medium\"")
        }));
        assert!(bundle.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/agents/switchloom_reviewer.toml"
                && artifact.content.contains("name = \"switchloom_reviewer\"")
        }));
        assert!(
            bundle
                .artifacts
                .iter()
                .any(|artifact| artifact.path == ".planr/agents.toml")
        );
    }

    #[test]
    fn fully_custom_setup_compiles_as_unverified_host_native_bundle() {
        let spec = SetupSpecV1 {
            schema_version: 1,
            host: "codex".to_string(),
            integration: Integration::Standalone,
            usage_policy: "balanced".to_string(),
            selected_roles: BTreeMap::from([
                (
                    "orchestrator".to_string(),
                    SetupRoleSelection {
                        model: "gpt-5.6-sol".to_string(),
                        effort: Some("medium".to_string()),
                        spawn: Some(SetupSpawnPolicy {
                            agent_type: "switchloom_orchestrator".to_string(),
                            task_name: "orchestrator".to_string(),
                            fork_turns: ForkPolicy {
                                mode: "none".to_string(),
                                turns: None,
                            },
                        }),
                    },
                ),
                (
                    "implementer".to_string(),
                    SetupRoleSelection {
                        model: "gpt-5.6-terra".to_string(),
                        effort: Some("high".to_string()),
                        spawn: Some(SetupSpawnPolicy {
                            agent_type: "switchloom_implementer".to_string(),
                            task_name: "implementer".to_string(),
                            fork_turns: ForkPolicy {
                                mode: "none".to_string(),
                                turns: None,
                            },
                        }),
                    },
                ),
            ]),
            routes: vec![SetupRouteMapping {
                work_type: "code".to_string(),
                role: "implementer".to_string(),
                fallbacks: vec!["orchestrator".to_string()],
            }],
            route_default: Some(SetupDefaultRoute {
                role: "orchestrator".to_string(),
                fallbacks: Vec::new(),
            }),
        };
        let bundle = compile_setup_spec(&spec).unwrap();
        assert_eq!(bundle.source.integration, Integration::Standalone);
        assert_eq!(bundle.evidence.status, "custom-unverified");
        assert!(bundle.profiles.contains_key("implementer"));
        assert!(bundle.artifacts.iter().any(|artifact| artifact.path
            == ".codex/agents/switchloom_implementer.toml"
            && artifact.content.contains("model = \"gpt-5.6-terra\"")));
        assert!(bundle.artifacts.iter().any(|artifact| {
            artifact.content.contains("task_name `implementer`")
                && !artifact.content.contains("sandbox_mode")
        }));
        assert!(
            bundle
                .artifacts
                .iter()
                .all(|artifact| !artifact.path.starts_with(".planr/"))
        );
        validate_bundle(&bundle).unwrap();
    }

    #[test]
    fn custom_setup_rejects_duplicate_codex_spawn_identities() {
        let duplicate_agent_type = SetupSpecV1 {
            schema_version: 1,
            host: "codex".to_string(),
            integration: Integration::Standalone,
            usage_policy: "balanced".to_string(),
            selected_roles: BTreeMap::from([
                (
                    "implementer".to_string(),
                    SetupRoleSelection {
                        model: "gpt-5.6-terra".to_string(),
                        effort: Some("high".to_string()),
                        spawn: Some(SetupSpawnPolicy {
                            agent_type: "switchloom_shared".to_string(),
                            task_name: "implementer".to_string(),
                            fork_turns: ForkPolicy {
                                mode: "none".to_string(),
                                turns: None,
                            },
                        }),
                    },
                ),
                (
                    "reviewer".to_string(),
                    SetupRoleSelection {
                        model: "gpt-5.6-sol".to_string(),
                        effort: Some("high".to_string()),
                        spawn: Some(SetupSpawnPolicy {
                            agent_type: "switchloom_shared".to_string(),
                            task_name: "reviewer".to_string(),
                            fork_turns: ForkPolicy {
                                mode: "none".to_string(),
                                turns: None,
                            },
                        }),
                    },
                ),
            ]),
            routes: vec![SetupRouteMapping {
                work_type: "code".to_string(),
                role: "implementer".to_string(),
                fallbacks: vec!["reviewer".to_string()],
            }],
            route_default: Some(SetupDefaultRoute {
                role: "implementer".to_string(),
                fallbacks: Vec::new(),
            }),
        };
        assert!(
            compile_setup_spec(&duplicate_agent_type)
                .unwrap_err()
                .to_string()
                .contains("both declare Codex agent_type `switchloom_shared`")
        );

        let duplicate_task_name = SetupSpecV1 {
            schema_version: 1,
            host: "codex".to_string(),
            integration: Integration::Standalone,
            usage_policy: "balanced".to_string(),
            selected_roles: BTreeMap::from([
                (
                    "foo-bar".to_string(),
                    SetupRoleSelection {
                        model: "gpt-5.6-terra".to_string(),
                        effort: Some("high".to_string()),
                        spawn: Some(SetupSpawnPolicy {
                            agent_type: "switchloom_foo_bar".to_string(),
                            task_name: "foo_bar".to_string(),
                            fork_turns: ForkPolicy {
                                mode: "none".to_string(),
                                turns: None,
                            },
                        }),
                    },
                ),
                (
                    "foo_bar".to_string(),
                    SetupRoleSelection {
                        model: "gpt-5.6-sol".to_string(),
                        effort: Some("high".to_string()),
                        spawn: Some(SetupSpawnPolicy {
                            agent_type: "switchloom_foo_bar_alt".to_string(),
                            task_name: "foo_bar".to_string(),
                            fork_turns: ForkPolicy {
                                mode: "none".to_string(),
                                turns: None,
                            },
                        }),
                    },
                ),
            ]),
            routes: vec![SetupRouteMapping {
                work_type: "code".to_string(),
                role: "foo-bar".to_string(),
                fallbacks: vec!["foo_bar".to_string()],
            }],
            route_default: None,
        };
        assert!(
            compile_setup_spec(&duplicate_task_name)
                .unwrap_err()
                .to_string()
                .contains("both normalize to `foo_bar`")
        );
    }

    #[test]
    fn custom_setup_rejects_normalized_artifact_path_collisions() {
        for (host, model, effort, expected_path) in [
            (
                "claude-code",
                "sonnet",
                Some("medium"),
                ".claude/agents/switchloom-foo_bar.md",
            ),
            (
                "cursor",
                "composer-2.5",
                None,
                ".cursor/agents/switchloom-foo_bar.md",
            ),
            (
                "mixed-host",
                "sonnet",
                Some("medium"),
                ".model-routing/roles/foo_bar.toml",
            ),
        ] {
            let spec = SetupSpecV1 {
                schema_version: 1,
                host: host.to_string(),
                integration: Integration::Standalone,
                usage_policy: "balanced".to_string(),
                selected_roles: BTreeMap::from([
                    (
                        "foo-bar".to_string(),
                        SetupRoleSelection {
                            model: model.to_string(),
                            effort: effort.map(ToOwned::to_owned),
                            spawn: None,
                        },
                    ),
                    (
                        "foo_bar".to_string(),
                        SetupRoleSelection {
                            model: model.to_string(),
                            effort: effort.map(ToOwned::to_owned),
                            spawn: None,
                        },
                    ),
                ]),
                routes: vec![SetupRouteMapping {
                    work_type: "code".to_string(),
                    role: "foo-bar".to_string(),
                    fallbacks: vec!["foo_bar".to_string()],
                }],
                route_default: None,
            };
            let error = compile_setup_spec(&spec).unwrap_err().to_string();
            assert!(
                error.contains("both normalize to `foo_bar`") || error.contains(expected_path),
                "expected normalized collision for {host}, got {error}"
            );
        }
    }

    #[test]
    fn successful_custom_setups_validate_final_bundles_for_each_host_family() {
        for (host, role, model, effort) in [
            ("claude-code", "implementer", "sonnet", Some("medium")),
            ("cursor", "implementer", "composer-2.5", None),
            ("mixed-host", "implementer", "sonnet", Some("medium")),
        ] {
            let spec = SetupSpecV1 {
                schema_version: 1,
                host: host.to_string(),
                integration: Integration::Standalone,
                usage_policy: "balanced".to_string(),
                selected_roles: BTreeMap::from([(
                    role.to_string(),
                    SetupRoleSelection {
                        model: model.to_string(),
                        effort: effort.map(ToOwned::to_owned),
                        spawn: None,
                    },
                )]),
                routes: vec![SetupRouteMapping {
                    work_type: "code".to_string(),
                    role: role.to_string(),
                    fallbacks: Vec::new(),
                }],
                route_default: None,
            };
            let bundle = compile_setup_spec(&spec).unwrap();
            validate_bundle(&bundle).unwrap();
            assert_eq!(bundle.evidence.status, "custom-unverified");
        }
    }

    #[test]
    fn setup_config_lifecycle_persists_normalized_config_and_reuses_manifest_flow() {
        let repository = temp_repo("setup-config-lifecycle");
        let config_file = repository.join("input.setup.toml");
        let original = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
        let original_toml = setup_spec_to_canonical_toml(&original).unwrap();
        fs::write(&config_file, &original_toml).unwrap();

        let preview = preview_setup_config_file(&repository, &config_file).unwrap();
        assert_eq!(preview.action, "preview");
        assert!(
            preview.artifacts.iter().any(|artifact| {
                artifact.path == SETUP_CONFIG_PATH && artifact.status == "create"
            })
        );

        let applied = apply_setup_config_file(&repository, &config_file).unwrap();
        assert_eq!(applied.action, "apply");
        assert_eq!(
            fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
            original_toml
        );
        assert!(
            !repository.join(".planr").exists(),
            "standalone setup must not create .planr"
        );
        let status = status_repository(&repository).unwrap();
        assert!(status.artifacts.iter().any(|artifact| {
            artifact.path == SETUP_CONFIG_PATH && artifact.status == "managed"
        }));
        let saved_preview = preview_saved_setup(&repository).unwrap();
        assert!(saved_preview.artifacts.iter().any(|artifact| {
            artifact.path == SETUP_CONFIG_PATH && artifact.status == "unchanged"
        }));

        let mut updated =
            setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
        let worker = updated.selected_roles.get_mut("worker").unwrap();
        worker.model = "gpt-5.6-sol".to_string();
        worker.effort = Some("medium".to_string());
        worker.spawn = Some(SetupSpawnPolicy {
            agent_type: "switchloom_worker".to_string(),
            task_name: "worker".to_string(),
            fork_turns: ForkPolicy {
                mode: "none".to_string(),
                turns: None,
            },
        });
        let updated_file = repository.join("updated.setup.toml");
        let updated_toml = setup_spec_to_canonical_toml(&updated).unwrap();
        fs::write(&updated_file, &updated_toml).unwrap();
        let update = update_setup_config_file(&repository, &updated_file).unwrap();
        assert_eq!(update.action, "update");
        assert_eq!(
            fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
            updated_toml
        );
        assert!(
            repository
                .join(".codex/agents/switchloom_worker.toml")
                .exists()
        );

        let rollback = rollback_repository(&repository).unwrap();
        assert_eq!(rollback.action, "rollback");
        assert_eq!(
            fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
            original_toml
        );
        assert!(
            !repository
                .join(".codex/agents/switchloom_worker.toml")
                .exists()
        );

        let uninstall = uninstall_repository(&repository).unwrap();
        assert_eq!(uninstall.action, "uninstall");
        assert!(!repository.join(SETUP_CONFIG_PATH).exists());
        assert!(!repository.join(".model-routing/manifest.json").exists());
    }

    #[test]
    fn setup_recipe_apply_persists_config_and_rejects_existing_conflicts() {
        let repository = temp_repo("setup-recipe-lifecycle");
        let spec = setup_spec_for_policy("balanced", "codex", Integration::Planr).unwrap();
        let recipe = setup_spec_to_recipe(&spec).unwrap();

        let preview = preview_setup_recipe(&repository, &recipe).unwrap();
        assert_eq!(preview.action, "preview");
        assert!(preview.artifacts.iter().any(|artifact| {
            artifact.path == ".planr/agents.toml" && artifact.status == "create"
        }));
        apply_setup_recipe(&repository, &recipe).unwrap();
        assert_eq!(
            fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
            setup_spec_to_canonical_toml(&spec).unwrap()
        );
        assert!(repository.join(".planr/agents.toml").exists());

        let conflict_repo = temp_repo("setup-recipe-conflict");
        fs::create_dir_all(conflict_repo.join(".switchloom")).unwrap();
        fs::write(conflict_repo.join(SETUP_CONFIG_PATH), "not managed\n").unwrap();
        let error = apply_setup_recipe(&conflict_repo, &recipe)
            .unwrap_err()
            .to_string();
        assert!(error.contains(SETUP_CONFIG_PATH));
    }

    #[test]
    fn prepared_setup_apply_aborts_when_repository_plan_changes_after_preview() {
        let repository = temp_repo("prepared-setup-toctou");
        let spec = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
        let prepared = prepare_setup_lifecycle(&spec).unwrap();
        let preview = preview_prepared_setup(&repository, &prepared).unwrap();
        fs::create_dir_all(repository.join(".switchloom")).unwrap();
        fs::write(repository.join(SETUP_CONFIG_PATH), "external change\n").unwrap();
        let error = apply_prepared_setup(&repository, &prepared, &preview)
            .unwrap_err()
            .to_string();
        assert!(error.contains("repository state changed after preview"));
        assert_eq!(
            fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
            "external change\n"
        );
        assert!(!repository.join(".model-routing/manifest.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn prepared_setup_apply_aborts_when_repository_symlink_retargets_after_preview() {
        use std::os::unix::fs::symlink;

        let root = temp_repo("prepared-setup-symlink");
        let repo_a = root.join("repo-a");
        let repo_b = root.join("repo-b");
        let link = root.join("repo-link");
        fs::create_dir_all(&repo_a).unwrap();
        fs::create_dir_all(&repo_b).unwrap();
        symlink(&repo_a, &link).unwrap();

        let spec = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
        let prepared = prepare_setup_lifecycle(&spec).unwrap();
        let preview = preview_prepared_setup(&link, &prepared).unwrap();
        assert_eq!(
            preview.repository,
            repo_a.canonicalize().unwrap().display().to_string()
        );

        fs::remove_file(&link).unwrap();
        symlink(&repo_b, &link).unwrap();
        let error = apply_prepared_setup(&link, &prepared, &preview)
            .unwrap_err()
            .to_string();
        assert!(error.contains("repository state changed after preview"));
        assert!(!repo_a.join(SETUP_CONFIG_PATH).exists());
        assert!(!repo_b.join(SETUP_CONFIG_PATH).exists());
        assert!(!repo_a.join(".model-routing/manifest.json").exists());
        assert!(!repo_b.join(".model-routing/manifest.json").exists());
    }

    #[test]
    fn setup_spec_rejects_unknown_fields_and_invalid_combinations() {
        let unknown = r#"{
  "schema_version": 1,
  "host": "codex",
  "integration": "standalone",
  "usage_policy": "balanced",
  "selected_roles": {},
  "routes": [],
  "unexpected": true
}"#;
        assert!(
            format!("{:#}", setup_spec_from_json(unknown).unwrap_err()).contains("unknown field")
        );

        let invalid_effort = SetupSpecV1 {
            schema_version: 1,
            host: "codex".to_string(),
            integration: Integration::Standalone,
            usage_policy: "balanced".to_string(),
            selected_roles: BTreeMap::from([(
                "implementer".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-luna".to_string(),
                    effort: Some("ultra".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_implementer".to_string(),
                        task_name: "implementer".to_string(),
                        fork_turns: ForkPolicy {
                            mode: "none".to_string(),
                            turns: None,
                        },
                    }),
                },
            )]),
            routes: vec![SetupRouteMapping {
                work_type: "code".to_string(),
                role: "implementer".to_string(),
                fallbacks: Vec::new(),
            }],
            route_default: None,
        };
        assert!(
            validate_setup_spec(&invalid_effort)
                .unwrap_err()
                .to_string()
                .contains("is not supported")
        );

        let mut invalid_fork = invalid_effort;
        invalid_fork
            .selected_roles
            .get_mut("implementer")
            .unwrap()
            .model = "gpt-5.6-terra".to_string();
        invalid_fork
            .selected_roles
            .get_mut("implementer")
            .unwrap()
            .effort = Some("high".to_string());
        invalid_fork
            .selected_roles
            .get_mut("implementer")
            .unwrap()
            .spawn
            .as_mut()
            .unwrap()
            .fork_turns = ForkPolicy {
            mode: "all".to_string(),
            turns: None,
        };
        assert!(
            validate_setup_spec(&invalid_fork)
                .unwrap_err()
                .to_string()
                .contains("must not use fork_turns all")
        );

        let mut missing_spawn = invalid_fork.clone();
        missing_spawn
            .selected_roles
            .get_mut("implementer")
            .unwrap()
            .spawn = None;
        assert!(
            validate_setup_spec(&missing_spawn)
                .unwrap_err()
                .to_string()
                .contains("must declare Codex spawn policy")
        );

        let mut name_mismatch = invalid_fork.clone();
        let spawn = name_mismatch
            .selected_roles
            .get_mut("implementer")
            .unwrap()
            .spawn
            .as_mut()
            .unwrap();
        spawn.fork_turns = ForkPolicy {
            mode: "none".to_string(),
            turns: None,
        };
        spawn.task_name = "wrong_name".to_string();
        assert!(
            validate_setup_spec(&name_mismatch)
                .unwrap_err()
                .to_string()
                .contains("must match `implementer`")
        );

        let mut task_path = name_mismatch;
        task_path
            .selected_roles
            .get_mut("implementer")
            .unwrap()
            .spawn
            .as_mut()
            .unwrap()
            .task_name = "/root/task".to_string();
        assert!(
            validate_setup_spec(&task_path)
                .unwrap_err()
                .to_string()
                .contains("not a canonical task path")
        );

        let legacy_fork_context = r#"{
  "schema_version": 1,
  "host": "codex",
  "integration": "standalone",
  "usage_policy": "balanced",
  "selected_roles": {
    "implementer": {
      "model": "gpt-5.6-terra",
      "effort": "high",
      "fork_context": "none"
    }
  },
  "routes": [{"work_type": "code", "role": "implementer"}]
}"#;
        assert!(
            format!(
                "{:#}",
                setup_spec_from_json(legacy_fork_context).unwrap_err()
            )
            .contains("fork_context")
        );
    }

    #[test]
    fn setup_contract_catalog_exposes_transport_and_host_options() {
        let catalog = setup_contract_catalog_value().unwrap();
        assert_eq!(catalog["configPath"], SETUP_CONFIG_PATH);
        assert_eq!(catalog["recipePrefix"], SETUP_RECIPE_PREFIX);
        assert!(catalog["hosts"].as_array().unwrap().iter().any(|host| {
            host["id"] == "codex"
                && host["models"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|model| model["id"] == "gpt-5.6-sol")
        }));
    }

    #[test]
    fn planr_integration_is_skill_free_and_preloads_existing_protocols() {
        for host in BINDINGS.map(|(host, _)| host) {
            let bundle = compile_policy("balanced", host, Integration::Planr).unwrap();
            assert!(
                bundle
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.path == ".planr/agents.toml")
            );
            assert!(
                bundle
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.path == ".planr/policy.toml")
            );
            assert!(
                bundle
                    .artifacts
                    .iter()
                    .all(|artifact| !artifact.path.contains("/skills/"))
            );
            let content = serde_json::to_string(&bundle).unwrap();
            assert!(!content.contains("model-routing-native-routing"));
            assert!(!content.contains("planr-native-routing"));
            assert!(!content.contains("Protocol preload: $planr-goal"));
            assert!(!content.contains("Protocol preload: $planr-loop"));

            let worker_protocol = bundle.artifacts.iter().any(|artifact| {
                (artifact.path.contains("terra-high")
                    || artifact.path.contains("luna-xhigh")
                    || artifact.path.contains("preset-worker"))
                    && artifact.content.contains("Protocol preload: $planr-work")
            });
            assert!(worker_protocol, "missing Planr worker preload for {host}");

            if host == "codex-openai" || host == "mixed-host" {
                assert!(
                    bundle.artifacts.iter().any(|artifact| artifact
                        .content
                        .contains("Protocol preload: $planr-review")),
                    "missing Planr review preload for {host}"
                );
            }

            if host == "codex-openai" {
                let registry = bundle
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.path == ".planr/agents.toml")
                    .expect("missing Planr registry");
                assert!(!registry.content.contains("agent_type ="));
                assert!(!registry.content.contains("fork_turns"));
                assert!(
                    registry
                        .content
                        .contains("[profiles.model_routing_terra_high]")
                );
                assert!(
                    registry
                        .content
                        .contains("[profiles.model_routing_sol_high]")
                );
                assert!(
                    registry
                        .content
                        .contains("profile = \"model_routing_terra_high\"")
                );
                assert!(
                    registry
                        .content
                        .contains("profile = \"model_routing_sol_high\"")
                );
            }
        }
    }

    #[test]
    fn valid_generated_bundles_pass_strict_inspection() {
        for integration in [Integration::Standalone, Integration::Planr] {
            let bundle = compile_json("balanced", "codex-openai", integration).unwrap();
            let inspection = inspect_bundle_json(&bundle).unwrap();
            assert!(inspection.valid);
            assert_eq!(inspection.integration, integration);
            assert_eq!(inspection.policy_id, "balanced");
        }
    }

    #[test]
    fn invalid_contract_fixtures_fail_for_named_reasons() {
        for (fixture, expected) in [
            (
                include_str!("../fixtures/routing-bundle-v1/invalid-unsupported-version.json"),
                "unsupported schema_version 2",
            ),
            (
                include_str!("../fixtures/routing-bundle-v1/invalid-dual-artifact-payload.json"),
                "cannot define both content and content_ref",
            ),
            (
                include_str!("../fixtures/routing-bundle-v1/invalid-artifact-hash.json"),
                "sha256 mismatch",
            ),
            (
                include_str!("../fixtures/routing-bundle-v1/invalid-unknown-source-field.json"),
                "unknown field `unexpected`",
            ),
            (
                include_str!(
                    "../fixtures/routing-bundle-v1/invalid-unknown-policy-usage-field.json"
                ),
                "unknown field `unexpected`",
            ),
            (
                include_str!("../fixtures/routing-bundle-v1/invalid-unknown-profile-field.json"),
                "unknown field `unexpected`",
            ),
        ] {
            let error = inspect_bundle_json(fixture).unwrap_err().to_string();
            assert!(
                error.contains(expected),
                "expected `{expected}` in `{error}`"
            );
        }
    }

    #[test]
    fn standalone_policy_contract_changes_enforceable_output() {
        let balanced = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        let read_only =
            compile_policy("read-only-audit", "codex-openai", Integration::Standalone).unwrap();

        assert_ne!(balanced.policy, read_only.policy);
        assert_eq!(read_only.policy.usage.max_parallel_writers, 0);
        assert_eq!(
            read_only
                .policy
                .execution
                .roles
                .get("worker")
                .unwrap()
                .filesystem
                .write_roots,
            Vec::<String>::new()
        );

        let mut balanced_value: Value = serde_json::from_str(
            &compile_json("balanced", "codex-openai", Integration::Standalone).unwrap(),
        )
        .unwrap();
        let mut read_only_value: Value = serde_json::from_str(
            &compile_json("read-only-audit", "codex-openai", Integration::Standalone).unwrap(),
        )
        .unwrap();
        for value in [&mut balanced_value, &mut read_only_value] {
            let object = value.as_object_mut().unwrap();
            object.remove("bundle_id");
            object.remove("policy_id");
        }
        assert_ne!(balanced_value, read_only_value);
    }

    #[test]
    fn codex_and_mixed_bindings_keep_native_bounded_fork_topology() {
        let codex = compile_json("balanced", "codex-openai", Integration::Standalone).unwrap();
        let mixed = compile_json("balanced", "mixed-host", Integration::Standalone).unwrap();
        assert!(codex.contains("gpt-5.6-sol"));
        assert!(codex.contains("\"fork_turns\""));
        assert!(codex.contains("\"mode\": \"none\""));
        assert!(!codex.contains("fork_turns: \\\"all\\\""));
        assert!(mixed.contains("fable-5"));
        assert!(mixed.contains("gpt-5.6-terra"));
        assert_ne!(codex, mixed);
    }

    #[test]
    fn codex_child_overrides_require_bounded_fork_policy() {
        let mut profile = Profile {
            client: "codex".to_string(),
            model: "gpt-5.6-terra".to_string(),
            agent_type: Some("model_routing_terra_high".to_string()),
            effort: Some("high".to_string()),
            cost_tier: None,
            capabilities: Vec::new(),
            skill: None,
            notes: None,
            fork_turns: None,
        };
        assert!(
            validate_profile_fork_policy(&profile)
                .unwrap_err()
                .to_string()
                .contains("must declare fork_turns")
        );

        profile.fork_turns = Some(ForkPolicy {
            mode: "all".to_string(),
            turns: None,
        });
        assert!(
            validate_profile_fork_policy(&profile)
                .unwrap_err()
                .to_string()
                .contains("must not use fork_turns all")
        );

        profile.fork_turns = Some(ForkPolicy {
            mode: "bounded".to_string(),
            turns: Some(2),
        });
        validate_profile_fork_policy(&profile).unwrap();
    }

    #[test]
    fn lifecycle_preview_apply_status_and_uninstall_are_repository_safe() {
        let repository = temp_repo("lifecycle");
        let bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        let preview = preview_bundle(&repository, &bundle).unwrap();
        assert_eq!(preview.action, "preview");
        assert_eq!(preview.artifacts.len(), 7);
        assert!(
            preview
                .artifacts
                .iter()
                .all(|artifact| artifact.status == "create")
        );

        let bundle_file = repository.join("bundle.json");
        fs::write(
            &bundle_file,
            compile_json("balanced", "codex-openai", Integration::Standalone).unwrap(),
        )
        .unwrap();
        let applied = apply_bundle_file(&repository, &bundle_file).unwrap();
        assert_eq!(applied.action, "apply");
        assert!(repository.join(MANIFEST_PATH).exists());
        assert!(
            repository
                .join(".codex/agents/model-routing-sol-medium.toml")
                .exists()
        );

        let status = status_repository(&repository).unwrap();
        assert_eq!(status.action, "status");
        assert!(
            status
                .artifacts
                .iter()
                .all(|artifact| artifact.status == "managed")
        );

        let uninstalled = uninstall_repository(&repository).unwrap();
        assert_eq!(uninstalled.action, "uninstall");
        assert!(
            uninstalled
                .artifacts
                .iter()
                .all(|artifact| artifact.status == "removed")
        );
        assert!(!repository.join(MANIFEST_PATH).exists());
        assert!(
            !repository
                .join(".codex/agents/model-routing-sol-medium.toml")
                .exists()
        );
    }

    #[test]
    fn lifecycle_rejects_unsafe_paths_and_conflicts() {
        let repository = temp_repo("unsafe");
        let mut bundle =
            compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();

        bundle.artifacts[0].path = ".model-routing/unsafe.toml".to_string();
        assert!(
            preview_bundle(&repository, &bundle)
                .unwrap_err()
                .to_string()
                .contains("reserved path")
        );

        let mut bundle =
            compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        bundle.artifacts[0].path = "../escape.toml".to_string();
        assert!(
            preview_bundle(&repository, &bundle)
                .unwrap_err()
                .to_string()
                .contains("must not traverse")
        );

        let bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        let target = repository.join(&bundle.artifacts[0].path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "user edit").unwrap();
        let error = apply_bundle_file_with_bundle(&repository, &bundle)
            .unwrap_err()
            .to_string();
        assert!(error.contains("already exists with different content"));
    }

    #[test]
    fn lifecycle_rejects_parent_child_targets_without_partial_apply() {
        let repository = temp_repo("parent-child");
        let mut bundle =
            compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        bundle.artifacts.truncate(2);
        bundle.artifacts[0].path = ".codex/agents/collision".to_string();
        bundle.artifacts[0].content = "parent".to_string();
        bundle.artifacts[0].sha256 = sha256(bundle.artifacts[0].content.as_bytes());
        bundle.artifacts[1].path = ".codex/agents/collision/child.toml".to_string();
        bundle.artifacts[1].content = "child".to_string();
        bundle.artifacts[1].sha256 = sha256(bundle.artifacts[1].content.as_bytes());

        let error = apply_bundle_file_with_bundle(&repository, &bundle)
            .unwrap_err()
            .to_string();
        assert!(error.contains("parent-child collision"));
        assert!(!repository.join(".codex/agents/collision").exists());
        assert!(!repository.join(MANIFEST_PATH).exists());
    }

    #[test]
    fn lifecycle_update_and_rollback_are_manifest_aware() {
        let repository = temp_repo("update-rollback");
        let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &original).unwrap();

        let mut updated = original.clone();
        updated.bundle_id = "balanced-codex-openai@updated".to_string();
        updated.artifacts[0].content.push_str("\n# updated\n");
        updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
        let bundle_file = write_bundle_file(&repository, "updated-bundle.json", &updated);

        let update = update_bundle_file(&repository, &bundle_file).unwrap();
        assert_eq!(update.action, "update");
        assert!(
            update
                .artifacts
                .iter()
                .any(|artifact| artifact.status == "update")
        );
        assert_eq!(
            sha256(&fs::read(repository.join(&updated.artifacts[0].path)).unwrap()),
            updated.artifacts[0].sha256
        );

        let rollback = rollback_repository(&repository).unwrap();
        assert_eq!(rollback.action, "rollback");
        assert!(
            rollback
                .artifacts
                .iter()
                .any(|artifact| artifact.status == "rollback")
        );
        assert_eq!(
            sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
            original.artifacts[0].sha256
        );
    }

    #[test]
    fn lifecycle_codex_config_merges_unrelated_entries_update_rollback_and_uninstall() {
        let repository = temp_repo("codex-config-ownership");
        fs::create_dir_all(repository.join(".codex")).unwrap();
        fs::write(
            repository.join(CODEX_CONFIG_PATH),
            "[agents.local_reviewer]\nconfig_file = \"./agents/local-reviewer.toml\"\n\n[features]\nlocal = true\n",
        )
        .unwrap();

        let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        let applied = apply_bundle_file_with_bundle(&repository, &codex).unwrap();
        assert!(
            applied.artifacts.iter().any(|artifact| {
                artifact.path == CODEX_CONFIG_PATH && artifact.status == "update"
            })
        );
        let config_after_apply = fs::read_to_string(repository.join(CODEX_CONFIG_PATH)).unwrap();
        assert_codex_config_entry(
            &config_after_apply,
            "local_reviewer",
            "./agents/local-reviewer.toml",
        );
        assert_codex_config_entry(
            &config_after_apply,
            "model_routing_terra_high",
            "./agents/model-routing-terra-high.toml",
        );
        assert_codex_config_entry(
            &config_after_apply,
            "model_routing_sol_high",
            "./agents/model-routing-sol-high.toml",
        );
        assert!(config_after_apply.contains("[features]"));

        let mixed = compile_policy("balanced", "mixed-host", Integration::Standalone).unwrap();
        let mixed_file = write_bundle_file(&repository, "mixed.json", &mixed);
        let update = update_bundle_file(&repository, &mixed_file).unwrap();
        assert!(
            update.artifacts.iter().any(|artifact| {
                artifact.path == CODEX_CONFIG_PATH && artifact.status == "update"
            })
        );
        let config_after_update = fs::read_to_string(repository.join(CODEX_CONFIG_PATH)).unwrap();
        assert_codex_config_entry(
            &config_after_update,
            "local_reviewer",
            "./agents/local-reviewer.toml",
        );
        assert_codex_config_entry(
            &config_after_update,
            "model_routing_terra_high",
            "./agents/model-routing-terra-high.toml",
        );
        assert_no_codex_config_entry(&config_after_update, "model_routing_sol_medium");
        assert_no_codex_config_entry(&config_after_update, "model_routing_sol_ultra");

        let rollback = rollback_repository(&repository).unwrap();
        assert!(rollback.artifacts.iter().any(|artifact| {
            artifact.path == CODEX_CONFIG_PATH && artifact.status == "rollback"
        }));
        let config_after_rollback = fs::read_to_string(repository.join(CODEX_CONFIG_PATH)).unwrap();
        assert_codex_config_entry(
            &config_after_rollback,
            "local_reviewer",
            "./agents/local-reviewer.toml",
        );
        assert_codex_config_entry(
            &config_after_rollback,
            "model_routing_sol_medium",
            "./agents/model-routing-sol-medium.toml",
        );
        assert_codex_config_entry(
            &config_after_rollback,
            "model_routing_sol_ultra",
            "./agents/model-routing-sol-ultra.toml",
        );

        let uninstall = uninstall_repository(&repository).unwrap();
        assert!(uninstall.artifacts.iter().any(|artifact| {
            artifact.path == CODEX_CONFIG_PATH && artifact.status == "removed"
        }));
        let config_after_uninstall =
            fs::read_to_string(repository.join(CODEX_CONFIG_PATH)).unwrap();
        assert_codex_config_entry(
            &config_after_uninstall,
            "local_reviewer",
            "./agents/local-reviewer.toml",
        );
        assert_no_codex_config_entry(&config_after_uninstall, "model_routing_terra_high");
        assert_no_codex_config_entry(&config_after_uninstall, "model_routing_sol_high");
        assert!(config_after_uninstall.contains("[features]"));
        assert!(!repository.join(MANIFEST_PATH).exists());
    }

    #[test]
    fn lifecycle_codex_config_modified_managed_entry_is_preserved_with_repair() {
        let repository = temp_repo("codex-config-modified-entry");
        let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &codex).unwrap();
        fs::write(
            repository.join(CODEX_CONFIG_PATH),
            "[agents.model_routing_terra_high]\nconfig_file = \"./agents/hacked.toml\"\n",
        )
        .unwrap();

        let status = status_repository(&repository).unwrap();
        assert!(status.artifacts.iter().any(|artifact| {
            artifact.path == CODEX_CONFIG_PATH
                && artifact.status == "modified"
                && artifact.repair.is_some()
        }));

        let uninstall = uninstall_repository(&repository).unwrap();
        assert!(uninstall.artifacts.iter().any(|artifact| {
            artifact.path == CODEX_CONFIG_PATH
                && artifact.status == "preserved-modified"
                && artifact.repair.is_some()
        }));
        assert!(repository.join(CODEX_CONFIG_PATH).exists());
        assert!(repository.join(MANIFEST_PATH).exists());
    }

    #[test]
    fn lifecycle_recovers_interrupted_transaction_before_next_entrypoint() {
        let repository = temp_repo("interrupted-transaction");
        let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &original).unwrap();

        let mut updated = original.clone();
        updated.bundle_id = "balanced-codex-openai@interrupted".to_string();
        updated.artifacts[0]
            .content
            .push_str("\n# interrupted update\n");
        updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
        let updated_file = write_bundle_file(&repository, "interrupted.json", &updated);

        TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(1));
        let interrupted = std::panic::catch_unwind(|| {
            update_bundle_file(&repository, &updated_file).unwrap();
        });
        TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(0));
        assert!(interrupted.is_err());
        assert!(has_transaction_directory(&repository));

        let status = status_repository(&repository).unwrap();
        assert_eq!(
            status.bundle_id.as_deref(),
            Some(original.bundle_id.as_str())
        );
        assert_eq!(
            sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
            original.artifacts[0].sha256
        );
        assert!(
            status
                .artifacts
                .iter()
                .all(|artifact| artifact.status == "managed")
        );
        assert!(!has_transaction_directory(&repository));

        let update = update_bundle_file(&repository, &updated_file).unwrap();
        assert!(
            update
                .artifacts
                .iter()
                .any(|artifact| artifact.status == "update")
        );
    }

    #[test]
    fn lifecycle_recovers_interrupted_atomic_journal_replacement() {
        let repository = temp_repo("journal-replace");
        let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &original).unwrap();

        let mut updated = original.clone();
        updated.bundle_id = "balanced-codex-openai@journal-replace".to_string();
        updated.artifacts[0]
            .content
            .push_str("\n# journal replace interruption\n");
        updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
        let updated_file = write_bundle_file(&repository, "journal-replace.json", &updated);

        TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER.with(|fail_after| fail_after.set(2));
        let interrupted = std::panic::catch_unwind(|| {
            update_bundle_file(&repository, &updated_file).unwrap();
        });
        TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER.with(|fail_after| fail_after.set(0));
        assert!(interrupted.is_err());
        assert!(has_transaction_directory(&repository));

        let status = status_repository(&repository).unwrap();
        assert_eq!(
            status.bundle_id.as_deref(),
            Some(original.bundle_id.as_str())
        );
        assert_eq!(
            sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
            original.artifacts[0].sha256
        );
        assert!(!has_transaction_directory(&repository));
    }

    #[test]
    fn lifecycle_restore_failure_preserves_recoverable_transaction_data() {
        let repository = temp_repo("restore-failure");
        let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &original).unwrap();

        let mut updated = original.clone();
        updated.bundle_id = "balanced-codex-openai@restore-failure".to_string();
        updated.artifacts[0]
            .content
            .push_str("\n# restore failure\n");
        updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
        let updated_file = write_bundle_file(&repository, "restore-failure.json", &updated);

        TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(1));
        let interrupted = std::panic::catch_unwind(|| {
            update_bundle_file(&repository, &updated_file).unwrap();
        });
        TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(0));
        assert!(interrupted.is_err());
        assert!(has_transaction_directory(&repository));

        TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(true));
        let recovery_error = status_repository(&repository).unwrap_err().to_string();
        TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(false));
        assert!(recovery_error.contains("failed to recover transaction write"));
        assert!(has_transaction_directory(&repository));

        let status = status_repository(&repository).unwrap();
        assert_eq!(
            status.bundle_id.as_deref(),
            Some(original.bundle_id.as_str())
        );
        assert_eq!(
            sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
            original.artifacts[0].sha256
        );
        assert!(!has_transaction_directory(&repository));
    }

    #[test]
    fn lifecycle_returned_journal_error_retains_backup_when_immediate_rollback_fails() {
        let repository = temp_repo("rollback-retains-backup");
        let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &original).unwrap();

        let mut updated = original.clone();
        updated.bundle_id = "balanced-codex-openai@rollback-retains-backup".to_string();
        updated.artifacts[0]
            .content
            .push_str("\n# returned journal error\n");
        updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
        let updated_file = write_bundle_file(&repository, "rollback-retains-backup.json", &updated);

        TRANSACTION_RETURN_JOURNAL_ERROR_AFTER.with(|fail_after| fail_after.set(2));
        TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(true));
        let error = update_bundle_file(&repository, &updated_file)
            .unwrap_err()
            .to_string();
        TRANSACTION_RETURN_JOURNAL_ERROR_AFTER.with(|fail_after| fail_after.set(0));
        TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(false));
        assert!(error.contains("transaction rollback incomplete"));
        assert!(has_transaction_directory(&repository));
        assert!(has_transaction_backup(&repository));

        let status = status_repository(&repository).unwrap();
        assert_eq!(
            status.bundle_id.as_deref(),
            Some(original.bundle_id.as_str())
        );
        assert_eq!(
            sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
            original.artifacts[0].sha256
        );
        assert!(!has_transaction_directory(&repository));
    }

    #[test]
    fn lifecycle_staged_rename_error_restores_backup_before_commit_mark() {
        let repository = temp_repo("staged-rename");
        let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &original).unwrap();

        let mut updated = original.clone();
        updated.bundle_id = "balanced-codex-openai@staged-rename".to_string();
        updated.artifacts[0]
            .content
            .push_str("\n# staged rename failure\n");
        updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
        let updated_file = write_bundle_file(&repository, "staged-rename.json", &updated);

        TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER.with(|fail_after| fail_after.set(1));
        let error = update_bundle_file(&repository, &updated_file)
            .unwrap_err()
            .to_string();
        TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER.with(|fail_after| fail_after.set(0));
        assert!(error.contains("injected staged rename error after backup"));
        assert!(!has_transaction_directory(&repository));

        let status = status_repository(&repository).unwrap();
        assert_eq!(
            status.bundle_id.as_deref(),
            Some(original.bundle_id.as_str())
        );
        assert_eq!(
            sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
            original.artifacts[0].sha256
        );
    }

    #[test]
    fn lifecycle_preserves_modified_files_and_residual_manifest() {
        let repository = temp_repo("preserve-residual");
        let mut bundle =
            compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        bundle.artifacts.truncate(1);
        apply_bundle_file_with_bundle(&repository, &bundle).unwrap();

        let target = repository.join(&bundle.artifacts[0].path);
        fs::write(&target, "user modified").unwrap();
        let uninstall = uninstall_repository(&repository).unwrap();
        assert_eq!(uninstall.artifacts[0].status, "preserved-modified");
        assert!(uninstall.artifacts[0].repair.is_some());
        assert!(target.exists());
        assert!(repository.join(MANIFEST_PATH).exists());

        let status = status_repository(&repository).unwrap();
        assert_eq!(status.artifacts[0].status, "modified");
        assert!(status.artifacts[0].repair.is_some());
    }

    #[test]
    fn lifecycle_cross_host_update_and_rollback_remove_old_managed_artifacts() {
        let repository = temp_repo("cross-host");
        let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        let claude = compile_policy("balanced", "claude-native", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &codex).unwrap();
        let codex_artifact = repository.join(".codex/agents/model-routing-sol-medium.toml");
        assert!(codex_artifact.exists());

        let claude_file = write_bundle_file(&repository, "claude.json", &claude);
        let update = update_bundle_file(&repository, &claude_file).unwrap();
        assert!(
            update
                .artifacts
                .iter()
                .any(|artifact| artifact.mode == "delete" && artifact.status == "removed")
        );
        assert!(!codex_artifact.exists());
        let status = status_repository(&repository).unwrap();
        assert!(
            status
                .artifacts
                .iter()
                .all(|artifact| artifact.path.starts_with(".claude/"))
        );

        let claude_artifact = repository.join(".claude/agents/model-routing-preset-worker.md");
        assert!(claude_artifact.exists());
        let rollback = rollback_repository(&repository).unwrap();
        assert!(
            rollback
                .artifacts
                .iter()
                .any(|artifact| artifact.mode == "delete" && artifact.status == "removed")
        );
        assert!(!claude_artifact.exists());
        assert!(codex_artifact.exists());
        let status = status_repository(&repository).unwrap();
        assert!(
            status
                .artifacts
                .iter()
                .all(|artifact| artifact.path.starts_with(".codex/"))
        );

        uninstall_repository(&repository).unwrap();
        assert!(!repository.join(MANIFEST_PATH).exists());
        assert!(!codex_artifact.exists());
    }

    #[test]
    fn lifecycle_cross_host_update_preserves_modified_removed_paths() {
        let repository = temp_repo("cross-host-preserve");
        let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        let claude = compile_policy("balanced", "claude-native", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &codex).unwrap();
        let codex_artifact = repository.join(".codex/agents/model-routing-sol-medium.toml");
        fs::write(&codex_artifact, "user modified codex artifact").unwrap();

        let claude_file = write_bundle_file(&repository, "claude.json", &claude);
        let update = update_bundle_file(&repository, &claude_file).unwrap();
        let preserved = update
            .artifacts
            .iter()
            .find(|artifact| artifact.path == ".codex/agents/model-routing-sol-medium.toml")
            .unwrap();
        assert_eq!(preserved.mode, "delete");
        assert_eq!(preserved.status, "preserved-modified");
        assert!(preserved.repair.is_some());
        assert!(codex_artifact.exists());

        let status = status_repository(&repository).unwrap();
        assert!(status.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/agents/model-routing-sol-medium.toml"
                && artifact.status == "modified"
                && artifact.repair.is_some()
        }));
        assert!(
            status
                .artifacts
                .iter()
                .any(|artifact| artifact.path.starts_with(".claude/"))
        );
    }

    fn apply_bundle_file_with_bundle(
        repository: &Path,
        bundle: &RoutingBundleV1,
    ) -> Result<LifecycleReport> {
        let bundle_file = write_bundle_file(repository, "bundle.json", bundle);
        apply_bundle_file(repository, &bundle_file)
    }

    fn write_bundle_file(repository: &Path, name: &str, bundle: &RoutingBundleV1) -> PathBuf {
        let bundle_file = repository.join(name);
        fs::write(&bundle_file, serde_json::to_vec_pretty(bundle).unwrap()).unwrap();
        bundle_file
    }

    fn assert_codex_config_entry(content: &str, agent_type: &str, config_file: &str) {
        let parsed: toml::Value = toml::from_str(content).unwrap();
        assert_eq!(
            parsed["agents"][agent_type]["config_file"].as_str(),
            Some(config_file)
        );
    }

    fn assert_no_codex_config_entry(content: &str, agent_type: &str) {
        let parsed: toml::Value = toml::from_str(content).unwrap();
        assert!(parsed["agents"].get(agent_type).is_none());
    }

    fn has_transaction_directory(repository: &Path) -> bool {
        fs::read_dir(repository.join(".model-routing"))
            .unwrap()
            .any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("txn-"))
            })
    }

    fn has_transaction_backup(repository: &Path) -> bool {
        fs::read_dir(repository.join(".model-routing"))
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("txn-"))
                    && entry.path().join("backup").exists()
            })
    }

    #[test]
    fn codex_agent_types_match_registered_toml_names() {
        for host in ["codex-openai", "mixed-host"] {
            let bundle = compile_policy("balanced", host, Integration::Standalone).unwrap();
            let config = bundle
                .artifacts
                .iter()
                .find(|artifact| artifact.path == ".codex/config.toml")
                .expect("Codex role config should be generated");
            let parsed_config: toml::Value = toml::from_str(&config.content).unwrap();
            let config_agents = parsed_config["agents"].as_table().unwrap();
            let registered_names = bundle
                .artifacts
                .iter()
                .filter(|artifact| artifact.path.starts_with(".codex/agents/"))
                .map(|artifact| {
                    let agent_type =
                        toml::from_str::<toml::Value>(&artifact.content).unwrap()["name"]
                            .as_str()
                            .unwrap()
                            .to_string();
                    let relative_config_file =
                        artifact.path.strip_prefix(".codex/").unwrap().to_string();
                    assert_eq!(
                        config_agents[&agent_type]["config_file"].as_str(),
                        Some(format!("./{relative_config_file}").as_str())
                    );
                    agent_type
                })
                .collect::<std::collections::BTreeSet<_>>();
            for profile in bundle
                .profiles
                .values()
                .filter(|profile| profile.client == "codex")
            {
                let agent_type = profile.agent_type.as_deref().unwrap();
                assert!(registered_names.contains(agent_type));
            }
        }
    }

    #[test]
    fn fresh_repository_registers_codex_native_role_discovery_config() {
        let repository = temp_repo("codex-native-discovery-config");
        let bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
        apply_bundle_file_with_bundle(&repository, &bundle).unwrap();

        for role in ["model-routing-terra-high", "model-routing-sol-high"] {
            assert!(
                repository
                    .join(format!(".codex/agents/{role}.toml"))
                    .exists(),
                "generated native Codex role file {role} should exist"
            );
        }

        let config = bundle
            .artifacts
            .iter()
            .find(|artifact| artifact.path == ".codex/config.toml")
            .expect("repository-local Codex role discovery config should be generated");
        let parsed: toml::Value = toml::from_str(&config.content).unwrap();
        assert_eq!(
            parsed["agents"]["model_routing_terra_high"]["config_file"].as_str(),
            Some("./agents/model-routing-terra-high.toml")
        );
        assert_eq!(
            parsed["agents"]["model_routing_sol_high"]["config_file"].as_str(),
            Some("./agents/model-routing-sol-high.toml")
        );
        assert_eq!(
            fs::read_to_string(repository.join(".codex/config.toml")).unwrap(),
            config.content
        );
    }

    #[test]
    fn generated_registry_is_derived_from_binding_profiles_and_routes() {
        for host in BINDINGS.map(|(host, _)| host) {
            let bundle = compile_policy("balanced", host, Integration::Planr).unwrap();
            let registry = bundle
                .artifacts
                .iter()
                .find(|artifact| artifact.path == ".planr/agents.toml")
                .unwrap();
            let parsed: toml::Value = toml::from_str(&registry.content).unwrap();
            assert_eq!(
                parsed["profiles"].as_table().unwrap().len(),
                bundle.profiles.len()
            );
            assert_eq!(
                parsed["routes"].as_array().unwrap().len(),
                bundle.routes.len()
            );
        }
    }

    #[test]
    fn checked_in_planr_contract_fixtures_are_generated_outputs() {
        for (host, fixture) in [
            (
                "codex-openai",
                include_str!("../fixtures/routing-bundle-v1/valid-balanced-codex.json"),
            ),
            (
                "mixed-host",
                include_str!("../fixtures/routing-bundle-v1/valid-balanced-mixed.json"),
            ),
        ] {
            let generated: serde_json::Value =
                serde_json::from_str(&compile_json("balanced", host, Integration::Planr).unwrap())
                    .unwrap();
            let checked_in: serde_json::Value = serde_json::from_str(fixture).unwrap();
            assert_eq!(generated, checked_in, "regenerate fixture for {host}");
        }
    }

    #[test]
    fn offline_evaluation_never_claims_live_verification_or_recommendation() {
        let report = evaluate_policy("balanced", "codex-openai").unwrap();
        assert!(report.offline_reproducible);
        assert!(report.scenario_count >= 7);
        assert_eq!(report.status, "experimental");
        assert!(!report.recommended);
    }

    #[test]
    fn no_in_memory_claim_can_promote_offline_evaluation() {
        let report = evaluate_policy("balanced", "codex-openai").unwrap();
        assert!(report.live_evidence.is_none());
        assert_eq!(report.status, "experimental");
        assert!(!report.recommended);
    }

    #[test]
    fn catalog_is_reproducible_and_contains_the_full_pool() {
        let first = catalog_json().unwrap();
        let second = catalog_json().unwrap();
        assert_eq!(first, second);
        let value: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(value["compositions"].as_array().unwrap().len(), 20);
        assert!(
            value["compositions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["recommended"] == false)
        );
    }

    #[test]
    fn registry_signatures_are_content_bound() {
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let trusted_public_key = encode_hex(signing_key.verifying_key().as_bytes());
        let signature = sign_registry(b"catalog", "fixture", &"07".repeat(32)).unwrap();
        verify_registry_signature(b"catalog", &signature, "fixture", &trusted_public_key).unwrap();
        assert!(
            verify_registry_signature(b"tampered", &signature, "fixture", &trusted_public_key)
                .is_err()
        );
        let attacker_key = encode_hex(
            SigningKey::from_bytes(&[8_u8; 32])
                .verifying_key()
                .as_bytes(),
        );
        assert!(
            verify_registry_signature(b"catalog", &signature, "fixture", &attacker_key).is_err()
        );
        assert!(
            verify_registry_signature(b"catalog", &signature, "attacker", &trusted_public_key)
                .is_err()
        );
    }

    #[test]
    fn probe_does_not_infer_authentication_from_version_availability() {
        let report = probe_host(
            "codex-openai",
            Some("definitely-not-a-model-routing-host-command"),
        )
        .unwrap();
        assert!(!report.available);
        assert_eq!(report.authentication, "not_tested");
    }
}
