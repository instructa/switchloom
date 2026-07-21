use crate::contracts::*;
use crate::error::{Result, ResultContext};
use crate::evidence::{validate_adapter_contract, validate_dispatch_evidence_for_adapter};
use crate::integrations::render_planr_native_role;
use crate::{bail, product_error};
use crate::{config::*, hosts::*, registry::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const GENERATED_AT: &str = "2026-07-16T00:00:00Z";
pub(crate) const GENERATED_AT_UNIX: i64 = 1_784_160_000;
pub(crate) const EVALUATION_SUITE: &str = include_str!("../evaluations/preset-suite-v1.toml");
pub(crate) const POLICIES: [(&str, &str); 4] = [
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

#[cfg(test)]
#[path = "tests/routing.rs"]
mod tests;

pub fn show_policy(policy: &str, host: &str) -> Result<PolicySource> {
    let binding_id = canonical_binding_id(host);
    let policy_raw = POLICIES
        .iter()
        .find(|(id, _)| *id == policy)
        .map(|(_, raw)| *raw)
        .ok_or_else(|| product_error!("unknown routing policy `{policy}`"))?;
    let binding_raw = BINDINGS
        .iter()
        .find(|(id, _)| *id == binding_id)
        .map(|(_, raw)| *raw)
        .ok_or_else(|| product_error!("unknown routing host `{host}`"))?;
    let policy_contract: PolicyContract = toml::from_str(policy_raw)?;
    validate_policy_contract(&policy_contract)?;
    let binding: HostBinding = toml::from_str(binding_raw)?;
    let adapter = compile_host_adapter(
        policy_contract.id.as_str(),
        &binding,
        Integration::Standalone,
    )?;
    Ok(PolicySource {
        policy_id: policy_contract.id.clone(),
        host: host.to_string(),
        policy_version: policy_contract.version.clone(),
        binding_id: binding.id.clone(),
        binding_version: binding.version.clone(),
        generated_at: GENERATED_AT.to_string(),
        requirements: adapter.requirements,
        profiles: adapter.profiles,
        routes: adapter.routes,
        route_default: adapter.route_default,
        artifacts: adapter.artifacts,
        evidence: EvaluationEvidence {
            evaluation_ids: vec![binding.verification.id.clone()],
            status: "experimental".to_string(),
        },
        adapter_contract: adapter.adapter_contract,
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
pub(crate) fn compile_builtin_policy_direct(
    policy: &str,
    host: &str,
    integration: Integration,
) -> Result<RoutingBundleV1> {
    compile_source(show_policy(policy, host)?, integration)
}

pub(crate) fn compile_source(
    source: PolicySource,
    integration: Integration,
) -> Result<RoutingBundleV1> {
    validate_source(&source)?;
    let mut adapter_contract = adapter_contract_for_source(&source, integration)?;
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
    adapter_contract.adapter.emitted_artifact_modes = artifacts
        .iter()
        .map(|artifact| artifact.mode.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    adapter_contract.adapter.dispatch_recipe.artifact_paths = artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect();
    validate_adapter_contract(&adapter_contract)?;

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
        adapter_contract: Some(adapter_contract),
    })
}

pub fn compile_json(policy: &str, host: &str, integration: Integration) -> Result<String> {
    let mut json = serde_json::to_string_pretty(&compile_policy(policy, host, integration)?)?;
    json.push('\n');
    Ok(json)
}

pub fn evaluate_policy(policy: &str, host: &str) -> Result<EvaluationReport> {
    let suite: toml::Value = toml::from_str(EVALUATION_SUITE)?;
    let suite_id = suite
        .get("id")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| product_error!("evaluation suite is missing id"))?;
    let suite_version = suite
        .get("version")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| product_error!("evaluation suite is missing version"))?;
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
        product_error!("bundle does not match RoutingBundle v1 schema: {error}")
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
    if let Some(contract) = &bundle.adapter_contract {
        validate_adapter_contract(contract)?;
    }
    Ok(())
}

pub(crate) fn validate_raw_bundle_shape(value: &Value) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| product_error!("bundle root must be a JSON object"))?;
    let schema_version = object
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| product_error!("bundle schema_version must be an integer"))?;
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
        "adapter_contract",
    ]);
    for key in object.keys() {
        if !allowed_root.contains(key.as_str()) {
            bail!("unknown bundle field `{key}`");
        }
    }
    let source = object
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(|| product_error!("bundle source must be an object"))?;
    if !source.contains_key("integration") {
        bail!("bundle source.integration is required");
    }
    let artifacts = object
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| product_error!("bundle artifacts must be an array"))?;
    let allowed_artifact = BTreeSet::from(["path", "media_type", "mode", "content", "sha256"]);
    for artifact in artifacts {
        let artifact_object = artifact
            .as_object()
            .ok_or_else(|| product_error!("bundle artifact must be an object"))?;
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

pub(crate) fn validate_policy_contract(policy: &PolicyContract) -> Result<()> {
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

pub(crate) fn validate_source(source: &PolicySource) -> Result<()> {
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
    validate_adapter_contract(&source.adapter_contract)?;
    validate_policy_contract(&source.policy)?;
    Ok(())
}

pub(crate) fn bundle_artifact(source: SourceArtifact) -> BundleArtifact {
    BundleArtifact {
        sha256: sha256(source.content.as_bytes()),
        path: source.path,
        media_type: source.media_type,
        mode: source.mode,
        content: source.content,
    }
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

pub(crate) fn source_from_setup_spec(spec: &SetupSpecV1) -> Result<PolicySource> {
    validate_setup_spec(spec)?;
    let binding = binding_for_selector(&spec.host)?;
    let mut source = show_policy(&spec.usage_policy, &binding.id)?;
    if setup_matches_binding(spec, &binding)? {
        return Ok(source);
    }
    let adapter =
        compile_setup_adapter(source.policy_id.as_str(), &binding, spec, &source.artifacts)?;
    source.requirements = adapter.requirements;
    source.profiles = adapter.profiles;
    source.routes = adapter.routes;
    source.route_default = adapter.route_default;
    source.artifacts = adapter.artifacts;
    source.adapter_contract = adapter.adapter_contract;
    source.evidence = EvaluationEvidence {
        evaluation_ids: Vec::new(),
        status: "custom-unverified".to_string(),
    };
    Ok(source)
}

pub(crate) fn compile_setup_adapter(
    policy_id: &str,
    binding: &HostBinding,
    spec: &SetupSpecV1,
    binding_artifacts: &[SourceArtifact],
) -> Result<CompiledHostAdapter> {
    validate_host_adapter(binding)?;
    let runtime_host = setup_runtime_host(binding);
    let profiles = setup_profiles_from_intent(spec, binding)?;
    let routes = setup_routes_from_intent(spec);
    let route_default = setup_default_route_from_intent(spec);
    let artifacts = setup_artifacts_from_intent(
        runtime_host,
        &spec.selected_roles,
        binding,
        binding_artifacts,
    )?;
    let mut adapter_contract = adapter_contract_for_binding(policy_id, binding, spec.integration)?;
    adapter_contract.routing_intent.semantic_roles = profiles.keys().cloned().collect();
    adapter_contract.routing_intent.role_requests = role_intents_for_profiles(&profiles);
    adapter_contract.adapter.emitted_artifact_modes = artifacts
        .iter()
        .map(|artifact| artifact.mode.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    adapter_contract.adapter.dispatch_recipe.artifact_paths = artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect();
    validate_adapter_contract(&adapter_contract)?;
    Ok(CompiledHostAdapter {
        requirements: vec![HostRequirement {
            host: binding.host.clone(),
            capabilities: requirement_capabilities_for_binding(binding),
        }],
        profiles,
        routes,
        route_default,
        artifacts,
        adapter_contract,
    })
}

pub(crate) fn setup_profiles_from_intent(
    spec: &SetupSpecV1,
    binding: &HostBinding,
) -> Result<BTreeMap<String, Profile>> {
    let runtime_host = setup_runtime_host(binding);
    let model_catalog = setup_model_catalog(runtime_host);
    spec.selected_roles
        .iter()
        .map(|(role, selection)| {
            let option = model_catalog
                .iter()
                .find(|option| option.id == selection.model)
                .ok_or_else(|| {
                    product_error!(
                        "setup role `{role}` model `{}` is not supported by host `{}`",
                        selection.model,
                        spec.host
                    )
                })?;
            if runtime_host == "codex"
                && selection.spawn.is_none()
                && selection_matches_binding_profile(role, selection, binding)
            {
                return Ok((
                    role.clone(),
                    profile_from_binding_profile(binding.profiles.get(role).ok_or_else(|| {
                        product_error!("setup role `{role}` is missing from binding")
                    })?),
                ));
            }
            let agent_type = if runtime_host == "codex" {
                Some(
                    selection
                        .spawn
                        .as_ref()
                        .ok_or_else(|| {
                            product_error!("setup role `{role}` must declare Codex spawn policy")
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
        .collect()
}

pub(crate) fn setup_routes_from_intent(spec: &SetupSpecV1) -> Vec<Route> {
    spec.routes
        .iter()
        .map(|route| Route {
            selector: RouteSelector {
                work_type: Some(route.work_type.clone()),
                plan: None,
            },
            profile: route.role.clone(),
            fallbacks: route.fallbacks.clone(),
        })
        .collect()
}

pub(crate) fn setup_default_route_from_intent(spec: &SetupSpecV1) -> Option<DefaultRoute> {
    spec.route_default.as_ref().map(|default| DefaultRoute {
        profile: default.role.clone(),
        fallbacks: default.fallbacks.clone(),
    })
}

pub(crate) fn setup_matches_binding(spec: &SetupSpecV1, binding: &HostBinding) -> Result<bool> {
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

pub(crate) fn setup_artifacts_from_intent(
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
                        product_error!("setup role `{role}` must declare Codex spawn policy")
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
                "opencode" => {
                    let effort = selection
                        .effort
                        .clone()
                        .unwrap_or_else(|| "medium".to_string());
                    (
                        "opencode_agent",
                        format!(
                            "---\ndescription: Switchloom custom {role} role.\nmode: subagent\nmodel: {}\nvariant: {effort}\npermission:\n  edit: allow\n  bash: ask\n  task:\n    \"*\": deny\n---\nFollow the repository-local Switchloom setup role `{role}` and preserve routing evidence.\n",
                            selection.model
                        ),
                    )
                }
                "pi" => {
                    let effort = selection
                        .effort
                        .clone()
                        .unwrap_or_else(|| "medium".to_string());
                    let (provider, model) = selection.model.split_once('/').ok_or_else(|| {
                        product_error!(
                            "setup role `{role}` Pi model `{}` must use provider/model form",
                            selection.model
                        )
                    })?;
                    let agent_type = format!("switchloom-pi-{file_role}");
                    (
                        "pi_workflow",
                        format!(
                            "{{\n  \"schema_version\": 1,\n  \"workflow\": \"switchloom-{file_role}\",\n  \"runner\": \"pi\",\n  \"runtime_class\": \"external-runner\",\n  \"arguments\": {{\n    \"agent_type\": \"{agent_type}\",\n    \"provider_model\": \"{}\",\n    \"thinking\": \"{effort}\",\n    \"isolation\": {{\n      \"session\": \"none\",\n      \"tools\": \"none\",\n      \"extensions\": \"none\",\n      \"skills\": \"none\",\n      \"agent_dir\": \"report-workdir/.pi-agent\"\n    }},\n    \"task\": {{\n      \"semantic_role\": \"{role}\",\n      \"profile\": \"{agent_type}\",\n      \"returns\": \"nonce-only\"\n    }}\n  }},\n  \"process\": {{\n    \"argv\": [\"pi\", \"--print\", \"--no-session\", \"--no-tools\", \"--no-extensions\", \"--no-skills\", \"--provider\", \"{provider}\", \"--model\", \"{model}\", \"--thinking\", \"{effort}\"],\n    \"state_boundary\": \"PI_CODING_AGENT_DIR is set to a report-local directory for every certification run\"\n  }},\n  \"security\": {{\n    \"filesystem_tools\": \"disabled\",\n    \"session_persistence\": \"disabled\",\n    \"native_subagents\": \"not used\",\n    \"certification_requirement\": \"A persisted workflow receipt must include the dynamic nonce returned by a live Pi child process before advisory runtime evidence is accepted.\"\n  }}\n}}\n",
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

pub(crate) fn compile_host_adapter(
    policy_id: &str,
    binding: &HostBinding,
    integration: Integration,
) -> Result<CompiledHostAdapter> {
    validate_host_adapter(binding)?;
    Ok(CompiledHostAdapter {
        requirements: vec![HostRequirement {
            host: binding.host.clone(),
            capabilities: requirement_capabilities_for_binding(binding),
        }],
        profiles: profiles_for_binding(binding),
        routes: routes_for_binding(binding)?,
        route_default: default_route_for_binding(binding)?,
        artifacts: artifacts_for_binding(binding),
        adapter_contract: adapter_contract_for_binding(policy_id, binding, integration)?,
    })
}

pub(crate) fn profiles_for_binding(binding: &HostBinding) -> BTreeMap<String, Profile> {
    binding
        .profiles
        .values()
        .map(|profile| {
            (
                profile.profile.clone(),
                profile_from_binding_profile(profile),
            )
        })
        .collect()
}

pub(crate) fn routes_for_binding(binding: &HostBinding) -> Result<Vec<Route>> {
    binding
        .routes
        .iter()
        .map(|route| {
            Ok(Route {
                selector: RouteSelector {
                    work_type: Some(route.work_type.clone()),
                    plan: None,
                },
                profile: binding_profile_id(binding, &route.role)?.to_string(),
                fallbacks: route
                    .fallback_roles
                    .iter()
                    .map(|role| binding_profile_id(binding, role).map(ToOwned::to_owned))
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect()
}

pub(crate) fn default_route_for_binding(binding: &HostBinding) -> Result<Option<DefaultRoute>> {
    binding
        .default_role
        .as_deref()
        .map(|role| -> Result<DefaultRoute> {
            Ok(DefaultRoute {
                profile: binding_profile_id(binding, role)?.to_string(),
                fallbacks: Vec::new(),
            })
        })
        .transpose()
}

pub(crate) fn adapter_contract_for_binding(
    policy_id: &str,
    binding: &HostBinding,
    integration: Integration,
) -> Result<AdapterContractV1> {
    let runtime_class = binding.runtime_class;
    let semantic_roles = binding.profiles.keys().cloned().collect::<Vec<_>>();
    let artifact_modes = if binding.artifacts.is_empty() {
        Vec::new()
    } else {
        vec!["create".to_string(), "replace".to_string()]
    };
    let dispatch_fields = dispatch_fields_for_binding(binding);
    let artifact_paths = binding
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect::<Vec<_>>();
    Ok(AdapterContractV1 {
        schema_version: 1,
        routing_intent: RoutingIntentV1 {
            schema_version: 1,
            integration,
            semantic_roles,
            role_requests: role_intents_for_binding(binding),
            required_guarantees: vec![
                "artifact_lifecycle".to_string(),
                "dispatch_identity".to_string(),
            ],
        },
        capability: HostCapabilityV1 {
            schema_version: 1,
            host: binding.host.clone(),
            host_version_constraints: host_version_constraints_for_binding(binding)?,
            runtime_class,
            runtime_behavior: runtime_behavior_for_binding(binding)?,
            discovery_artifacts: binding.capability_evidence.clone(),
            dispatch_fields: dispatch_fields.clone(),
            model_control: ControlCapability {
                level: control_level(binding.capabilities.model_override, binding.host == "codex"),
                field: "model".to_string(),
                evidence_required: binding.capabilities.model_override,
            },
            effort_control: ControlCapability {
                level: control_level(binding.capabilities.effort_override, binding.host == "codex"),
                field: "effort".to_string(),
                evidence_required: binding.capabilities.effort_override,
            },
            context_semantics: ContextSemantics {
                supports_fork_none: binding.capabilities.fork_none,
                supports_fork_all: binding.capabilities.fork_all,
                requires_bounded_context_for_overrides: binding.host == "codex",
            },
            nesting: NestingCapability {
                max_depth: 1,
                level: if binding.capabilities.fork_none {
                    GuaranteeLevel::Deterministic
                } else {
                    GuaranteeLevel::Unsupported
                },
            },
            parallelism: ParallelismCapability {
                max_parallel_children: max_parallel_children_for_binding(binding)?,
                level: GuaranteeLevel::Advisory,
            },
            observability: ObservabilityCapability {
                requested_dispatch: GuaranteeLevel::Deterministic,
                effective_identity: if binding.host == "codex" {
                    GuaranteeLevel::Deterministic
                } else {
                    GuaranteeLevel::Advisory
                },
                effective_model: if binding.host == "codex" {
                    GuaranteeLevel::Deterministic
                } else {
                    GuaranteeLevel::Advisory
                },
                raw_evidence_refs: binding.capability_evidence.clone(),
            },
            guarantees: capability_guarantees_for_binding(binding),
            known_limitations: binding.known_limitations.clone(),
        },
        adapter: HostAdapterV1 {
            schema_version: 1,
            adapter_id: binding.id.clone(),
            adapter_version: binding.version.clone(),
            runtime_class,
            accepts_intent_schema: "RoutingIntentV1".to_string(),
            emitted_artifact_modes: artifact_modes,
            dispatch_recipe: DispatchRecipeV1 {
                invocation: match runtime_class {
                    RuntimeClass::NativeSubagent => "host-native-subagent".to_string(),
                    RuntimeClass::ExternalRunner => "external-runner-process".to_string(),
                },
                required_fields: dispatch_fields,
                artifact_paths,
            },
            lifecycle_owner: "switchloom-managed".to_string(),
        },
        dispatch_evidence: DispatchEvidenceContractV1 {
            schema_version: 1,
            required_verdicts: vec![
                GuaranteeLevel::Deterministic,
                GuaranteeLevel::Advisory,
                GuaranteeLevel::Unsupported,
            ],
            receipt_schema: "DispatchEvidenceV1".to_string(),
        },
        planr_handoff: PlanrHandoffV1 {
            schema_version: 1,
            switchloom_package: npm_package_identity()?,
            semantic_role_contract: format!(
                "Planr supplies usage policy `{policy_id}`, work_type routes, and semantic roles; Switchloom owns the selected host binding, model, effort, fork/context policy, and generated dispatch artifacts."
            ),
            required_consumer_behavior: vec![
                "Consume RoutingIntentV1 as the only source for semantic_role, work_type, selected profile, model, effort, agent_type, and fork_turns inputs.".to_string(),
                "Use the CLI lifecycle or SetupSpecV1 recipe commands to preview, apply, update, status, rollback, and uninstall repository-local artifacts.".to_string(),
                "Record Switchloom package version, package digest, bundle_id, host version, requested dispatch, effective child identity, nonce, and receipt paths before claiming certification.".to_string(),
                "Treat advisory or unsupported guarantees as uncertified until nonce-bearing live host evidence upgrades them.".to_string(),
                "For the available-host release gate, Codex may be certified from deterministic effective-routing evidence; Cursor profiles may only claim advisory nonce-correlated requested-routing evidence unless the host exposes authenticated effective role/model telemetry. Claude Code, OpenCode, and Pi remain unavailable or unverified until authentic receipts exist.".to_string(),
            ],
            forbidden_duplicate_ownership: vec![
                "Do not maintain a Planr-side model catalog, effort catalog, preset compiler, host adapter, or fork policy normalizer for Switchloom-owned inputs.".to_string(),
                "Do not re-normalize Switchloom model, effort, role, agent_type, profile, or fork policy identifiers in Planr.".to_string(),
                "Do not overwrite Switchloom-managed artifacts outside preview/apply/update/rollback/uninstall.".to_string(),
                "Do not mark Claude Code, OpenCode, Pi, or any advisory receipt as certified without live nonce-bearing child evidence.".to_string(),
            ],
            certification_report_reference:
                "reports/native-host-certification/<host>/<timestamp>/workdir/dispatch-evidence.json plus the matching bundle.json, invocation receipt, package digest, and validator stdout".to_string(),
        },
    })
}

pub(crate) fn adapter_contract_for_source(
    source: &PolicySource,
    integration: Integration,
) -> Result<AdapterContractV1> {
    let mut contract = source.adapter_contract.clone();
    contract.routing_intent.integration = integration;
    contract.adapter.emitted_artifact_modes = source
        .artifacts
        .iter()
        .map(|artifact| artifact.mode.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    validate_adapter_contract(&contract)?;
    Ok(contract)
}

pub(crate) fn role_intents_for_binding(binding: &HostBinding) -> Vec<RoutingRoleIntentV1> {
    binding
        .profiles
        .iter()
        .map(|(role, profile)| RoutingRoleIntentV1 {
            semantic_role: role.clone(),
            requested_model: profile.model.clone(),
            requested_effort: profile.effort.clone(),
            instructions: format!("Route `{role}` through `{}`.", profile.profile),
        })
        .collect()
}

pub(crate) fn role_intents_for_profiles(
    profiles: &BTreeMap<String, Profile>,
) -> Vec<RoutingRoleIntentV1> {
    profiles
        .iter()
        .map(|(role, profile)| RoutingRoleIntentV1 {
            semantic_role: role.clone(),
            requested_model: profile.model.clone(),
            requested_effort: profile.effort.clone(),
            instructions: format!("Route `{role}` through `{}`.", profile.client),
        })
        .collect()
}

pub(crate) fn profile_from_binding_profile(profile: &BindingProfile) -> Profile {
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

pub(crate) fn validate_profile_fork_policy(profile: &Profile) -> Result<()> {
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

pub(crate) fn include_artifact_for_integration(
    artifact: &SourceArtifact,
    integration: Integration,
) -> bool {
    if artifact.path.contains("/skills/")
        || artifact
            .content
            .contains("name: model-routing-native-routing")
    {
        return false;
    }
    integration == Integration::Planr || !artifact.path.starts_with(".planr/")
}

pub(crate) fn artifact_for_integration(
    mut artifact: SourceArtifact,
    integration: Integration,
) -> SourceArtifact {
    if integration == Integration::Planr {
        artifact.content = render_planr_native_role(&artifact);
    }
    artifact
}

pub fn validate_dispatch_evidence_json_for_bundle(
    evidence_json: &str,
    bundle_json: &str,
) -> Result<()> {
    let bundle: RoutingBundleV1 = serde_json::from_str(bundle_json)?;
    validate_bundle(&bundle)?;
    let contract = bundle
        .adapter_contract
        .as_ref()
        .ok_or_else(|| product_error!("bundle is missing adapter_contract"))?;
    let evidence: DispatchEvidenceV1 = serde_json::from_str(evidence_json)?;
    validate_dispatch_evidence_for_adapter(&evidence, contract)
}
