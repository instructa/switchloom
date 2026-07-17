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
    let source = show_policy(policy, host)?;
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
    artifacts.extend(
        source
            .artifacts
            .iter()
            .filter(|artifact| include_artifact_for_integration(artifact, integration))
            .cloned()
            .map(|artifact| artifact_for_integration(artifact, integration))
            .map(bundle_artifact),
    );
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
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let planned = plan_artifacts(&repository, &bundle, None)?;
    ensure_apply_is_safe(&planned)?;
    let manifest = manifest_from_bundle(&bundle, sha256(bundle_input.as_bytes()), None);
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
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let current = read_manifest(&repository)?
        .ok_or_else(|| anyhow::anyhow!("no model-routing manifest found"))?;
    let planned = plan_artifacts(&repository, &bundle, Some(&current))?;
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
        let status = if !target.exists() {
            "missing"
        } else {
            let content = fs::read(&target)
                .with_context(|| format!("failed to read `{}`", target.display()))?;
            if sha256(&content) == artifact.sha256 {
                "managed"
            } else {
                "modified"
            }
        };
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
        let status = if !target.exists() {
            "missing"
        } else {
            let content = fs::read(&target)
                .with_context(|| format!("failed to read `{}`", target.display()))?;
            if sha256(&content) != artifact.sha256 {
                "preserved-modified"
            } else {
                fs::remove_file(&target)
                    .with_context(|| format!("failed to remove `{}`", target.display()))?;
                "removed"
            }
        };
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

#[derive(Debug)]
struct PlannedArtifact {
    path: String,
    target: PathBuf,
    mode: String,
    content: Option<String>,
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
            let status = preserved_or_removed_status(&target, artifact)?;
            planned.push(PlannedArtifact {
                path: artifact.path.clone(),
                target,
                mode: "delete".to_string(),
                content: artifact.content.clone(),
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
        let status = preserved_or_removed_status(&target, artifact)?;
        planned.push(PlannedArtifact {
            path: artifact.path.clone(),
            target,
            mode: "delete".to_string(),
            content: artifact.content.clone(),
            sha256: artifact.sha256.clone(),
            status,
        });
    }
    reject_parent_child_targets(&planned)?;
    Ok(planned)
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
    let current =
        fs::read(target).with_context(|| format!("failed to read `{}`", target.display()))?;
    if sha256(&current) == artifact.sha256 {
        Ok("removed".to_string())
    } else {
        Ok("preserved-modified".to_string())
    }
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
        let staged = if artifact.status == "removed" {
            None
        } else {
            let staged = stage_root.join(format!("artifact-{index}"));
            let content = artifact.content.as_ref().ok_or_else(|| {
                anyhow::anyhow!("artifact `{}` has no staged content", artifact.path)
            })?;
            fs::write(&staged, content.as_bytes())
                .with_context(|| format!("failed to stage `{}`", artifact.path))?;
            Some(staged)
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
    if normalized_text == ".codex/config.toml" || normalized_text.starts_with(".model-routing/") {
        bail!("artifact path `{artifact_path}` targets a reserved path");
    }
    if !allowed_repository_target(normalized_text) {
        bail!("artifact path `{artifact_path}` is not an allowed host artifact path");
    }
    Ok(repository.join(normalized))
}

fn allowed_repository_target(path: &str) -> bool {
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
                content: artifact.content.clone(),
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
        || artifact.content.contains("Normal implementation")
        || artifact.content.contains("Bounded checklist")
}

fn is_reviewer_role(artifact: &SourceArtifact) -> bool {
    artifact.path.contains("sol-high") || artifact.content.contains("Independent final review")
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
        assert_eq!(preview.artifacts.len(), 6);
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

        bundle.artifacts[0].path = ".codex/config.toml".to_string();
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
            let registered_names = bundle
                .artifacts
                .iter()
                .filter(|artifact| artifact.path.starts_with(".codex/agents/"))
                .map(|artifact| {
                    toml::from_str::<toml::Value>(&artifact.content).unwrap()["name"]
                        .as_str()
                        .unwrap()
                        .to_string()
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
