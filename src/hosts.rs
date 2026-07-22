use crate::contracts::*;
use crate::error::{OptionContext, Result, ResultContext};
use crate::evidence::*;
use crate::{bail, product_error};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

pub(crate) const NPM_PACKAGE_JSON: &str = include_str!("../package.json");
const CODEX_PROJECT_CONFIG_PATH: &str = ".codex/config.toml";
pub(crate) const BINDINGS: [(&str, &str); 7] = [
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
        "opencode-native",
        include_str!("../host-bindings/opencode-native.toml"),
    ),
    (
        "pi-external",
        include_str!("../host-bindings/pi-external.toml"),
    ),
    (
        "mixed-host",
        include_str!("../host-bindings/mixed-host.toml"),
    ),
];

pub fn probe_host(host: &str, command_override: Option<&str>) -> Result<ProbeReport> {
    probe_host_with_repository(host, command_override, Path::new("."))
}

pub fn probe_host_with_repository(
    host: &str,
    command_override: Option<&str>,
    repository: &Path,
) -> Result<ProbeReport> {
    let binding = binding_for_selector(host)?;
    let default_command = match binding.host.as_str() {
        "codex" => Some("codex"),
        "cursor" => Some("cursor-agent"),
        "claude-code" => Some("claude"),
        "opencode" => Some("opencode"),
        "pi" => Some("pi"),
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
    let diagnostics =
        probe_diagnostics_for_binding(&binding, available, version.as_deref(), repository)?;
    Ok(ProbeReport {
        host: host.to_string(),
        command: command.map(ToOwned::to_owned),
        available,
        version,
        capabilities: requirement_capabilities_for_binding(&binding),
        authentication: "not_tested".to_string(),
        limitation,
        diagnostics,
    })
}

pub(crate) fn probe_diagnostics_for_binding(
    binding: &HostBinding,
    available: bool,
    version: Option<&str>,
    repository: &Path,
) -> Result<Vec<ProbeDiagnostic>> {
    if binding.host != "codex" {
        return Ok(Vec::new());
    }

    let mut diagnostics = Vec::new();
    match version.and_then(parse_codex_semver) {
        Some([0, 145, 0]) => diagnostics.push(probe_diagnostic(
            "codex_exact_version_ready",
            "info",
            "Codex reports exact version 0.145.0 for the certified V2 contract.",
            "No version action required for Switchloom v0.3.1 certification.",
        )),
        Some([major, minor, patch]) if available => diagnostics.push(probe_diagnostic(
            "codex_exact_version_mismatch",
            "error",
            format!(
                "Codex reports {major}.{minor}.{patch}; this Switchloom contract is certified only for Codex 0.145.0."
            ),
            "Install or select Codex CLI 0.145.0 before claiming certified native V2 routing; newer or older versions must be re-certified.",
        )),
        _ => diagnostics.push(probe_diagnostic(
            "codex_unavailable",
            "error",
            "Codex version could not be read.",
            "Install Codex CLI 0.145.0 or pass --command with a compatible Codex binary.",
        )),
    }
    diagnostics.push(probe_diagnostic(
        "codex_luna_experimental_unverified",
        "warning",
        "Luna is available only as an explicit experimental/unverified Codex role; certified default routing uses Terra for mechanical work.",
        "Use Terra defaults for certified Codex V2 routing; select Luna only manually and do not claim deterministic Luna certification until exact-version live evidence is independently reviewed.",
    ));

    diagnostics.extend(codex_repository_diagnostics(repository)?);
    Ok(diagnostics)
}

pub(crate) fn codex_repository_diagnostics(repository: &Path) -> Result<Vec<ProbeDiagnostic>> {
    let config_path = repository.join(CODEX_PROJECT_CONFIG_PATH);
    if !config_path.exists() {
        return Ok(vec![probe_diagnostic(
            "codex_project_config_missing",
            "warning",
            "Repository-local .codex/config.toml is missing, so Codex cannot discover Switchloom roles from this repository yet.",
            "Run switchloom preview/apply for a Codex bundle, then trust the repository and reload or restart Codex.",
        )]);
    }

    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read `{}`", config_path.display()))?;
    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(parsed) => parsed,
        Err(error) => {
            return Ok(vec![probe_diagnostic(
                "codex_project_config_invalid",
                "error",
                format!("Repository-local .codex/config.toml is not valid TOML: {error}"),
                "Fix the TOML syntax, then rerun switchloom doctor codex.",
            )]);
        }
    };
    let multi_agent_v2 = parsed
        .get("features")
        .and_then(toml::Value::as_table)
        .and_then(|features| features.get("multi_agent_v2"))
        .and_then(toml::Value::as_table);
    let mut diagnostics = Vec::new();
    match multi_agent_v2.and_then(|table| table.get("enabled")).and_then(toml::Value::as_bool) {
        Some(true) => {}
        Some(false) => diagnostics.push(probe_diagnostic(
            "codex_v2_activation_conflict",
            "error",
            "Repository-local features.multi_agent_v2.enabled is false; Switchloom will not override explicit user-owned disabled V2 state.",
            "Change .codex/config.toml to enabled = true intentionally, or remove the conflicting user-owned setting before applying certified Codex V2 routing.",
        )),
        None => diagnostics.push(probe_diagnostic(
            "codex_v2_activation_missing",
            "warning",
            "Repository-local features.multi_agent_v2.enabled is missing, so Codex V2 role dispatch is not activated for this project.",
            "Run switchloom update/apply for a Codex 0.145 bundle to add enabled = true, then reload or restart Codex.",
        )),
    }
    match multi_agent_v2
        .and_then(|table| table.get("hide_spawn_agent_metadata"))
        .and_then(toml::Value::as_bool)
    {
        Some(false) => {}
        Some(true) => diagnostics.push(probe_diagnostic(
            "codex_v2_metadata_conflict",
            "error",
            "Repository-local features.multi_agent_v2.hide_spawn_agent_metadata is true; certified Switchloom evidence expects visible spawn metadata.",
            "Set hide_spawn_agent_metadata = false for certified evidence capture, then reload or restart Codex.",
        )),
        None => diagnostics.push(probe_diagnostic(
            "codex_v2_metadata_missing",
            "warning",
            "Repository-local features.multi_agent_v2.hide_spawn_agent_metadata is missing.",
            "Run switchloom update/apply for a Codex 0.145 bundle to add hide_spawn_agent_metadata = false.",
        )),
    }

    let agents = parsed.get("agents").and_then(toml::Value::as_table);
    for agent in [
        "model_routing_terra_high",
        "model_routing_terra_mechanical",
        "model_routing_sol_high",
    ] {
        if agents.and_then(|agents| agents.get(agent)).is_none() {
            diagnostics.push(probe_diagnostic(
                "codex_role_registration_missing",
                "warning",
                format!("Repository-local .codex/config.toml does not register required role `{agent}`."),
                "Run switchloom update/apply for a Codex bundle, then reload or restart Codex before dispatching by agent_type.",
            ));
        }
    }
    diagnostics.push(probe_diagnostic(
        "codex_trust_reload_required",
        "info",
        "Codex project trust and agent discovery after reload/restart are host-owned and cannot be mutated by Switchloom doctor.",
        "Trust this repository in Codex if prompted, then reload or restart the Codex session before relying on newly applied roles.",
    ));
    Ok(diagnostics)
}

fn probe_diagnostic(
    code: impl Into<String>,
    severity: impl Into<String>,
    message: impl Into<String>,
    repair: impl Into<String>,
) -> ProbeDiagnostic {
    ProbeDiagnostic {
        code: code.into(),
        severity: severity.into(),
        message: message.into(),
        repair: repair.into(),
    }
}

fn parse_codex_semver(value: &str) -> Option<[u64; 3]> {
    let version = value
        .trim()
        .strip_prefix("codex ")
        .or_else(|| value.trim().strip_prefix("codex-cli "))?;
    let mut parts = version.split(['-', '+']).next()?.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some([major, minor, patch])
}

#[cfg(test)]
#[path = "tests/hosts.rs"]
mod tests;

pub(crate) fn validate_host_adapter(binding: &HostBinding) -> Result<()> {
    if binding.id.trim().is_empty()
        || binding.version.trim().is_empty()
        || binding.host.trim().is_empty()
    {
        bail!("host adapter id, version, and host must not be blank");
    }
    if binding.profiles.is_empty() {
        bail!("host adapter `{}` must declare profiles", binding.id);
    }
    if binding.default_role.is_none() && binding.routes.is_empty() {
        bail!(
            "host adapter `{}` must declare routes or a default role",
            binding.id
        );
    }
    if let Some(default_role) = &binding.default_role {
        binding_profile_id(binding, default_role)?;
    }
    let mut profile_ids = BTreeMap::<String, String>::new();
    for (role, profile) in &binding.profiles {
        validate_setup_identifier("binding role", role)?;
        if profile.profile.trim().is_empty()
            || profile.client.trim().is_empty()
            || profile.model.trim().is_empty()
        {
            bail!(
                "host adapter `{}` profile `{role}` has blank identity fields",
                binding.id
            );
        }
        if let Some(existing) = profile_ids.insert(profile.profile.clone(), role.clone()) {
            bail!(
                "host adapter `{}` roles `{existing}` and `{role}` both normalize to profile `{}`",
                binding.id,
                profile.profile
            );
        }
        if profile.client == "codex" && profile.agent_type.is_none() {
            bail!(
                "host adapter `{}` Codex profile `{role}` must declare agent_type",
                binding.id
            );
        }
    }
    for route in &binding.routes {
        if route.work_type.trim().is_empty() {
            bail!(
                "host adapter `{}` route work_type must not be blank",
                binding.id
            );
        }
        binding_profile_id(binding, &route.role)?;
        for fallback in &route.fallback_roles {
            binding_profile_id(binding, fallback)?;
        }
    }
    let mut artifact_paths = BTreeMap::<String, String>::new();
    let mut codex_agent_types = BTreeMap::<String, String>::new();
    for artifact in &binding.artifacts {
        if artifact.path.trim().is_empty() || artifact.kind.trim().is_empty() {
            bail!(
                "host adapter `{}` artifacts must declare path and kind",
                binding.id
            );
        }
        if let Some(existing) = artifact_paths.insert(artifact.path.clone(), artifact.kind.clone())
        {
            bail!(
                "host adapter `{}` artifacts `{existing}` and `{}` both emit path `{}`",
                binding.id,
                artifact.kind,
                artifact.path
            );
        }
        if artifact.content.trim().is_empty() {
            bail!(
                "host adapter `{}` artifact `{}` must not be empty",
                binding.id,
                artifact.path
            );
        }
        if artifact.path.starts_with(".codex/agents/") {
            let parsed: toml::Value = toml::from_str(&artifact.content).with_context(|| {
                format!(
                    "host adapter `{}` artifact `{}` must be TOML",
                    binding.id, artifact.path
                )
            })?;
            let agent_type = parsed
                .get("name")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    product_error!(
                        "host adapter `{}` artifact `{}` must declare name",
                        binding.id,
                        artifact.path
                    )
                })?;
            if let Some(existing) =
                codex_agent_types.insert(agent_type.to_string(), artifact.path.clone())
            {
                bail!(
                    "host adapter `{}` artifacts `{existing}` and `{}` both declare Codex agent_type `{agent_type}`",
                    binding.id,
                    artifact.path
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn requirement_capabilities_for_binding(binding: &HostBinding) -> Vec<String> {
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
    capabilities
}

pub(crate) fn artifacts_for_binding(binding: &HostBinding) -> Vec<SourceArtifact> {
    binding
        .artifacts
        .iter()
        .map(|artifact| SourceArtifact {
            media_type: media_type_for(&artifact.path, &artifact.kind),
            path: artifact.path.clone(),
            mode: "create".to_string(),
            content: artifact.content.clone(),
        })
        .collect()
}

pub(crate) fn npm_package_identity() -> Result<String> {
    let package: Value = serde_json::from_str(NPM_PACKAGE_JSON)?;
    let name = package
        .get("name")
        .and_then(Value::as_str)
        .context("package.json must declare string name")?;
    let version = package
        .get("version")
        .and_then(Value::as_str)
        .context("package.json must declare string version")?;
    Ok(format!("{name}@{version}"))
}

pub(crate) fn control_level(supported: bool, deterministic: bool) -> GuaranteeLevel {
    match (supported, deterministic) {
        (true, true) => GuaranteeLevel::Deterministic,
        (true, false) => GuaranteeLevel::Advisory,
        (false, _) => GuaranteeLevel::Unsupported,
    }
}

pub(crate) fn max_parallel_children_for_binding(binding: &HostBinding) -> Result<u32> {
    if binding.host == "codex" {
        Ok(codex_v2_runtime_evidence()?
            .parallelism
            .max_parallel_children)
    } else {
        Ok(1)
    }
}

pub(crate) fn host_version_constraints_for_binding(
    binding: &HostBinding,
) -> Result<HostVersionConstraints> {
    if binding.host == "codex" {
        let evidence = codex_v2_runtime_evidence()?;
        let host_version = codex_v2_host_version(&evidence);
        return Ok(HostVersionConstraints {
            minimum: Some(host_version.clone()),
            maximum: Some(host_version),
            evidence_max_age_seconds: binding.verification.max_age_seconds.unwrap_or(0),
        });
    }
    Ok(HostVersionConstraints {
        minimum: None,
        maximum: None,
        evidence_max_age_seconds: binding.verification.max_age_seconds.unwrap_or(0),
    })
}

pub(crate) fn runtime_behavior_for_binding(binding: &HostBinding) -> Result<RuntimeBehaviorV1> {
    if binding.host == "codex" {
        let evidence = codex_v2_runtime_evidence()?;
        return Ok(RuntimeBehaviorV1 {
            capability_version: evidence.evidence_id,
            installed_host_version_source: format!(
                "{} via {}",
                evidence.installed_version.stdout, evidence.installed_version.command
            ),
            backend_selection_source: evidence.backend_selection_owner,
            trust_boundary: evidence.trust_and_discovery.trust_boundary,
            discovery_behavior: evidence.trust_and_discovery.discovery_behavior,
            role_precedence: evidence.role_precedence,
            shared_filesystem: evidence.shared_filesystem,
            delegation_modes: evidence.delegation_modes,
            source_references: vec![codex_v2_runtime_evidence_reference()],
        });
    }

    let source_references = if binding.capability_evidence.is_empty() {
        vec![format!("host-binding:{}", binding.id)]
    } else {
        binding.capability_evidence.clone()
    };

    Ok(RuntimeBehaviorV1 {
        capability_version: binding.verification.id.clone(),
        installed_host_version_source: format!("{} --version", binding.host),
        backend_selection_source: "host account, workspace, provider, or runner configuration outside Switchloom ownership".to_string(),
        trust_boundary: "repository-local generated artifacts are Switchloom-managed; host authentication, account policy, and execution state are host-owned".to_string(),
        discovery_behavior: "host-specific project artifact discovery".to_string(),
        role_precedence: vec![
            "Switchloom declares requested semantic role, profile, model, effort, and artifacts".to_string(),
            "the host runtime remains the authority for effective execution".to_string(),
        ],
        shared_filesystem: binding.runtime_class == RuntimeClass::NativeSubagent,
        delegation_modes: DelegationModesV1 {
            explicit_agent_type_dispatch: binding.capabilities.fork_none,
            ultra_auto_delegation: false,
            automatic_delegation_requires_ultra: false,
        },
        source_references,
    })
}

pub(crate) fn dispatch_fields_for_binding(binding: &HostBinding) -> Vec<String> {
    let mut fields = vec!["profile".to_string(), "model".to_string()];
    if binding.runtime_class == RuntimeClass::ExternalRunner {
        fields.push("provider".to_string());
    }
    if binding.capabilities.effort_override {
        fields.push("effort".to_string());
    }
    if binding
        .profiles
        .values()
        .any(|profile| profile.agent_type.is_some())
    {
        fields.push("agent_type".to_string());
    }
    if binding.capabilities.fork_none || binding.capabilities.fork_all {
        fields.push("fork_turns".to_string());
    }
    if binding.runtime_class == RuntimeClass::ExternalRunner {
        fields.push("isolation".to_string());
        fields.push("task".to_string());
    }
    fields
}

pub(crate) fn capability_guarantees_for_binding(
    binding: &HostBinding,
) -> BTreeMap<String, CapabilityGuarantee> {
    BTreeMap::from([
        (
            "artifact_lifecycle".to_string(),
            CapabilityGuarantee {
                level: GuaranteeLevel::Deterministic,
                reason: "Switchloom owns preview/apply/update/rollback/uninstall for managed artifacts.".to_string(),
                evidence_required: false,
            },
        ),
        (
            "dispatch_identity".to_string(),
            CapabilityGuarantee {
                level: if binding.capabilities.fork_none {
                    GuaranteeLevel::Deterministic
                } else {
                    GuaranteeLevel::Unsupported
                },
                reason: if binding.capabilities.fork_none {
                    "Adapter can emit explicit local child identity and non-all context policy.".to_string()
                } else {
                    "Host binding has no explicit non-all child dispatch contract.".to_string()
                },
                evidence_required: binding.capabilities.fork_none,
            },
        ),
        (
            "model_selection".to_string(),
            CapabilityGuarantee {
                level: if binding.capabilities.model_override && binding.host == "codex" {
                    GuaranteeLevel::Deterministic
                } else if binding.capabilities.model_override {
                    GuaranteeLevel::Advisory
                } else {
                    GuaranteeLevel::Unsupported
                },
                reason: if binding.capabilities.model_override && binding.host == "codex" {
                    "Codex project agent files declare the child model; live evidence is still required for certification.".to_string()
                } else if binding.capabilities.model_override {
                    "Host accepts a requested model but may apply account, workspace, or runtime precedence.".to_string()
                } else {
                    "Host binding exposes no model override control.".to_string()
                },
                evidence_required: binding.capabilities.model_override,
            },
        ),
        (
            "effort_selection".to_string(),
            CapabilityGuarantee {
                level: if binding.capabilities.effort_override && binding.host == "codex" {
                    GuaranteeLevel::Deterministic
                } else if binding.capabilities.effort_override {
                    GuaranteeLevel::Advisory
                } else {
                    GuaranteeLevel::Unsupported
                },
                reason: if binding.capabilities.effort_override && binding.host == "codex" {
                    "Codex project agent files declare model_reasoning_effort for role-local child dispatch.".to_string()
                } else if binding.capabilities.effort_override {
                    "Host accepts an effort-like field but effective precedence must be proven separately.".to_string()
                } else {
                    "Host binding exposes no effort override control.".to_string()
                },
                evidence_required: binding.capabilities.effort_override,
            },
        ),
        (
            "effective_runtime_evidence".to_string(),
            CapabilityGuarantee {
                level: GuaranteeLevel::Advisory,
                reason: "Generated bundles declare requested routing; certification must persist requested-versus-effective host evidence.".to_string(),
                evidence_required: true,
            },
        ),
    ])
}

pub(crate) fn binding_artifact_for_role(
    binding: &HostBinding,
    artifacts: &[SourceArtifact],
    role: &str,
) -> Result<SourceArtifact> {
    let profile = binding
        .profiles
        .get(role)
        .ok_or_else(|| product_error!("setup role `{role}` is missing from binding"))?;
    let agent_type = profile
        .agent_type
        .as_ref()
        .ok_or_else(|| product_error!("setup role `{role}` has no binding agent_type"))?;
    artifacts
        .iter()
        .find(|artifact| {
            artifact
                .content
                .contains(&format!("name = \"{agent_type}\""))
        })
        .cloned()
        .ok_or_else(|| product_error!("binding role `{role}` has no generated host artifact"))
}

pub(crate) fn binding_artifact_path_for_role(binding: &HostBinding, role: &str) -> Result<String> {
    let profile = binding
        .profiles
        .get(role)
        .ok_or_else(|| product_error!("setup role `{role}` is missing from binding"))?;
    let agent_type = profile
        .agent_type
        .as_ref()
        .ok_or_else(|| product_error!("setup role `{role}` has no binding agent_type"))?;
    binding
        .artifacts
        .iter()
        .find(|artifact| {
            artifact
                .content
                .contains(&format!("name = \"{agent_type}\""))
        })
        .map(|artifact| artifact.path.clone())
        .ok_or_else(|| product_error!("binding role `{role}` has no generated host artifact"))
}

pub(crate) fn binding_profile_id<'a>(binding: &'a HostBinding, role: &str) -> Result<&'a str> {
    binding
        .profiles
        .get(role)
        .map(|profile| profile.profile.as_str())
        .ok_or_else(|| product_error!("binding route references unknown role `{role}`"))
}

pub(crate) fn binding_for_selector(selector: &str) -> Result<HostBinding> {
    let binding_id = canonical_binding_id(selector);
    let raw = BINDINGS
        .iter()
        .find(|(id, _)| *id == binding_id)
        .map(|(_, raw)| *raw)
        .ok_or_else(|| product_error!("unknown setup host `{selector}`"))?;
    Ok(toml::from_str(raw)?)
}

pub(crate) fn canonical_binding_id(selector: &str) -> &str {
    match selector {
        "codex" => "codex-openai",
        "claude-code" => "claude-native",
        "cursor" => "cursor-openai",
        "opencode" => "opencode-native",
        "pi" => "pi-external",
        other => other,
    }
}

pub(crate) fn setup_runtime_host(binding: &HostBinding) -> &str {
    binding.host.as_str()
}

pub(crate) fn setup_model_catalog(host: &str) -> Vec<SetupModelOption> {
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
        "opencode" => vec![
            SetupModelOption {
                id: "opencode/gpt-5-nano",
                efforts: &["low", "medium", "high", "max"],
                tier: "standard",
            },
            SetupModelOption {
                id: "anthropic/claude-sonnet-4-5",
                efforts: &["low", "medium", "high"],
                tier: "standard",
            },
            SetupModelOption {
                id: "anthropic/claude-opus-4-5",
                efforts: &["high", "max"],
                tier: "premium",
            },
        ],
        "pi" => vec![
            SetupModelOption {
                id: "openai/gpt-4o-mini",
                efforts: &["low", "medium", "high", "xhigh"],
                tier: "standard",
            },
            SetupModelOption {
                id: "google/gemini-2.5-flash",
                efforts: &["low", "medium", "high", "xhigh"],
                tier: "standard",
            },
            SetupModelOption {
                id: "anthropic/claude-sonnet-4-5",
                efforts: &["low", "medium", "high", "xhigh"],
                tier: "premium",
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

pub(crate) fn validate_model_effort(
    host: &str,
    role: &str,
    selection: &SetupRoleSelection,
    catalog: &[SetupModelOption],
) -> Result<()> {
    let option = catalog
        .iter()
        .find(|option| option.id == selection.model)
        .ok_or_else(|| {
            product_error!(
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

pub(crate) fn selection_matches_binding_profile(
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

pub(crate) fn setup_spawn_policy_for_binding_role(
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

pub(crate) fn selection_spawn_matches_binding(
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

pub(crate) fn validate_setup_spawn_policy(
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

pub(crate) fn validate_setup_snake_identifier(kind: &str, value: &str) -> Result<()> {
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

pub(crate) fn validate_setup_identifier(kind: &str, value: &str) -> Result<()> {
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

pub(crate) fn validate_setup_identity_collisions(
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

pub(crate) fn setup_artifact_path(
    runtime_host: &str,
    role: &str,
    selection: &SetupRoleSelection,
) -> Result<String> {
    let file_role = identifier_token(role);
    Ok(match runtime_host {
        "codex" => {
            let spawn = selection.spawn.as_ref().ok_or_else(|| {
                product_error!("setup role `{role}` must declare Codex spawn policy")
            })?;
            format!(".codex/agents/{}.toml", spawn.agent_type)
        }
        "claude-code" => format!(".claude/agents/switchloom-{file_role}.md"),
        "cursor" => format!(".cursor/agents/switchloom-{file_role}.md"),
        "opencode" => format!(".opencode/agents/switchloom-{file_role}.md"),
        "pi" => format!(".pi/workflows/switchloom-{file_role}.json"),
        "mixed-host" => format!(".model-routing/roles/{file_role}.toml"),
        other => bail!("unsupported setup runtime host `{other}`"),
    })
}

pub(crate) fn validate_setup_route_role(
    roles: &BTreeMap<String, SetupRoleSelection>,
    role: &str,
) -> Result<()> {
    if !roles.contains_key(role) {
        bail!("setup route references unknown role `{role}`");
    }
    Ok(())
}

pub(crate) fn reject_setup_secret_like(kind: &str, value: &str) -> Result<()> {
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

pub(crate) fn identifier_token(value: &str) -> String {
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

pub(crate) fn media_type_for(path: &str, kind: &str) -> String {
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

pub(crate) fn render_codex_agent_registration_artifact(
    artifacts: &[SourceArtifact],
) -> Result<Option<SourceArtifact>> {
    #[derive(Serialize)]
    struct CodexConfig {
        agents: BTreeMap<String, CodexAgentRegistration>,
        features: CodexFeaturesConfig,
    }

    #[derive(Serialize)]
    struct CodexAgentRegistration {
        config_file: String,
    }

    #[derive(Serialize)]
    struct CodexFeaturesConfig {
        multi_agent_v2: CodexMultiAgentV2Config,
    }

    #[derive(Serialize)]
    struct CodexMultiAgentV2Config {
        enabled: bool,
        hide_spawn_agent_metadata: bool,
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
                product_error!("Codex agent artifact `{}` must declare name", artifact.path)
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
    let mut content = toml::to_string_pretty(&CodexConfig {
        agents,
        features: CodexFeaturesConfig {
            multi_agent_v2: CodexMultiAgentV2Config {
                enabled: true,
                hide_spawn_agent_metadata: false,
            },
        },
    })?;
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
