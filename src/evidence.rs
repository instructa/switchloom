use crate::contracts::*;
use crate::error::{Result, ResultContext};
use crate::registry::*;
use crate::{bail, product_error};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) const CODEX_V2_RUNTIME_EVIDENCE_JSON: &str =
    include_str!("../docs/codex-v2-runtime-evidence.json");

pub(crate) fn codex_v2_runtime_evidence() -> Result<CodexV2RuntimeEvidence> {
    let evidence: CodexV2RuntimeEvidence = serde_json::from_str(CODEX_V2_RUNTIME_EVIDENCE_JSON)
        .context("Codex V2 runtime evidence must be valid JSON")?;
    validate_codex_v2_runtime_evidence(&evidence)?;
    Ok(evidence)
}

#[cfg(test)]
#[path = "tests/evidence.rs"]
mod tests;

pub(crate) fn validate_codex_v2_runtime_evidence(evidence: &CodexV2RuntimeEvidence) -> Result<()> {
    if evidence.schema_version != 1 {
        bail!("unsupported Codex V2 runtime evidence schema_version");
    }
    if evidence.evidence_id.trim().is_empty()
        || evidence.observed_at.trim().is_empty()
        || evidence.installed_version.command != "codex --version"
        || evidence.installed_version.stdout.trim().is_empty()
        || evidence.backend_selection_owner.trim().is_empty()
        || evidence
            .trust_and_discovery
            .trust_boundary
            .trim()
            .is_empty()
        || evidence
            .trust_and_discovery
            .discovery_behavior
            .trim()
            .is_empty()
        || evidence.parallelism.source.trim().is_empty()
    {
        bail!("Codex V2 runtime evidence fields must not be blank");
    }
    if evidence.installed_version.stdout_sha256
        != sha256(format!("{}\n", evidence.installed_version.stdout).as_bytes())
    {
        bail!("Codex V2 runtime evidence installed_version stdout digest mismatch");
    }
    if evidence.runtime_class != RuntimeClass::NativeSubagent {
        bail!("Codex V2 runtime evidence must describe native-subagent");
    }
    if evidence.parallelism.max_parallel_children != 3 {
        bail!("Codex V2 runtime evidence must record three parallel child slots");
    }
    if !evidence.shared_filesystem
        || !evidence.delegation_modes.explicit_agent_type_dispatch
        || !evidence.delegation_modes.ultra_auto_delegation
        || !evidence
            .delegation_modes
            .automatic_delegation_requires_ultra
    {
        bail!("Codex V2 runtime evidence must record filesystem and delegation guarantees");
    }
    for (field, values) in [
        ("switchloom_ownership", &evidence.switchloom_ownership),
        ("codex_ownership", &evidence.codex_ownership),
        ("role_precedence", &evidence.role_precedence),
        ("negative_contracts", &evidence.negative_contracts),
    ] {
        if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
            bail!("Codex V2 runtime evidence must record {field}");
        }
    }
    for claim in [
        "installed_version",
        "backend_selection_owner",
        "trust_and_discovery",
        "parallelism",
        "role_precedence",
        "shared_filesystem",
        "delegation_modes",
    ] {
        let Some(records) = evidence.claim_provenance.get(claim) else {
            bail!("Codex V2 runtime evidence missing provenance for {claim}");
        };
        if records.is_empty() {
            bail!("Codex V2 runtime evidence has incomplete provenance for {claim}");
        }
        for record in records {
            validate_codex_claim_provenance_record(evidence, claim, record)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_codex_claim_provenance_record(
    evidence: &CodexV2RuntimeEvidence,
    claim: &str,
    record: &CodexClaimProvenance,
) -> Result<()> {
    if record.kind.trim().is_empty()
        || record.source.trim().is_empty()
        || record.observed_at.trim().is_empty()
        || record.codex_version != evidence.installed_version.stdout
    {
        bail!("Codex V2 runtime evidence has incomplete provenance for {claim}");
    }
    if record.source_url.as_deref().unwrap_or("").trim().is_empty()
        && record
            .source_path
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        bail!(
            "Codex V2 runtime evidence provenance for {claim} must include source_url or source_path"
        );
    }
    let Some(raw_output) = record.raw_output.as_deref() else {
        bail!("Codex V2 runtime evidence provenance for {claim} must include raw output");
    };
    let Some(raw_output_sha256) = record.raw_output_sha256.as_deref() else {
        bail!("Codex V2 runtime evidence provenance for {claim} must include raw output digest");
    };
    if raw_output_sha256 != sha256(raw_output.as_bytes()) {
        bail!("Codex V2 runtime evidence provenance raw output digest mismatch for {claim}");
    }
    let expected_value = codex_claim_observed_value(evidence, claim)?;
    if record.observed_value != expected_value {
        bail!("Codex V2 runtime evidence provenance observed value mismatch for {claim}");
    }
    if record.required_raw_fragments.is_empty()
        || record
            .required_raw_fragments
            .iter()
            .any(|fragment| fragment.trim().is_empty())
    {
        bail!("Codex V2 runtime evidence provenance for {claim} must bind raw fragments");
    }
    for fragment in codex_claim_required_raw_fragments(evidence, claim)? {
        if !record
            .required_raw_fragments
            .iter()
            .any(|recorded| recorded == &fragment)
        {
            bail!("Codex V2 runtime evidence provenance for {claim} missing required raw fragment");
        }
        if !raw_output.contains(&fragment) {
            bail!("Codex V2 runtime evidence raw capture does not support {claim}");
        }
    }
    for fragment in &record.required_raw_fragments {
        if !raw_output.contains(fragment) {
            bail!(
                "Codex V2 runtime evidence raw capture does not contain declared fragment for {claim}"
            );
        }
    }
    match record.kind.as_str() {
        "host-command" => {
            if claim != "installed_version"
                || record.source != evidence.installed_version.command
                || raw_output != format!("{}\n", evidence.installed_version.stdout)
                || raw_output_sha256 != evidence.installed_version.stdout_sha256
            {
                bail!("Codex V2 runtime evidence installed_version provenance mismatch");
            }
        }
        "source-document" => {
            if record.source_url.as_deref().unwrap_or("").trim().is_empty() {
                bail!(
                    "Codex V2 runtime evidence source-document provenance for {claim} must include source_url"
                );
            }
        }
        "session-runtime-contract" => {
            if record
                .source_path
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                bail!(
                    "Codex V2 runtime evidence session-runtime provenance for {claim} must include source_path"
                );
            }
        }
        other => {
            bail!("Codex V2 runtime evidence unsupported provenance kind `{other}` for {claim}")
        }
    }
    validate_codex_claim_source_identity(claim, record)?;
    Ok(())
}

pub(crate) fn validate_codex_claim_source_identity(
    claim: &str,
    record: &CodexClaimProvenance,
) -> Result<()> {
    let source_url = record.source_url.as_deref();
    let source_path = record.source_path.as_deref();
    let matches = match claim {
        "installed_version" => {
            record.kind == "host-command"
                && record.source == "codex --version"
                && source_path == Some("local-shell:codex --version")
                && source_url.is_none()
        }
        "backend_selection_owner" => {
            record.kind == "source-document"
                && source_url == Some("https://developers.openai.com/codex/config-reference")
                && source_path.is_none()
        }
        "trust_and_discovery" => {
            record.kind == "source-document"
                && source_url == Some("https://developers.openai.com/codex/config-reference")
                && source_path == Some("https://developers.openai.com/codex/subagents")
        }
        "parallelism" => {
            record.kind == "session-runtime-contract"
                && source_path == Some("current-session:developer-collaboration-runtime")
                && source_url.is_none()
        }
        "role_precedence" => {
            record.kind == "source-document"
                && source_url == Some("https://developers.openai.com/codex/subagents")
                && source_path.is_none()
        }
        "shared_filesystem" => {
            record.kind == "session-runtime-contract"
                && source_path == Some("current-session:developer-collaboration-runtime")
                && source_url.is_none()
        }
        "delegation_modes" => {
            record.kind == "source-document"
                && source_url == Some("https://developers.openai.com/codex/subagents")
                && source_path == Some("https://developers.openai.com/codex/models")
        }
        _ => false,
    };
    if !matches {
        bail!("Codex V2 runtime evidence provenance source identity mismatch for {claim}");
    }
    Ok(())
}

pub(crate) fn codex_claim_observed_value(
    evidence: &CodexV2RuntimeEvidence,
    claim: &str,
) -> Result<Value> {
    match claim {
        "installed_version" => Ok(json!(evidence.installed_version.stdout)),
        "backend_selection_owner" => Ok(json!(evidence.backend_selection_owner)),
        "trust_and_discovery" => Ok(json!({
            "trust_boundary": evidence.trust_and_discovery.trust_boundary,
            "discovery_behavior": evidence.trust_and_discovery.discovery_behavior,
        })),
        "parallelism" => Ok(json!({
            "max_parallel_children": evidence.parallelism.max_parallel_children,
            "source": evidence.parallelism.source,
        })),
        "role_precedence" => Ok(json!(evidence.role_precedence)),
        "shared_filesystem" => Ok(json!(evidence.shared_filesystem)),
        "delegation_modes" => Ok(json!(evidence.delegation_modes)),
        _ => bail!("Codex V2 runtime evidence unknown provenance claim `{claim}`"),
    }
}

pub(crate) fn codex_claim_required_raw_fragments(
    evidence: &CodexV2RuntimeEvidence,
    claim: &str,
) -> Result<Vec<String>> {
    match claim {
        "installed_version" => Ok(vec![evidence.installed_version.stdout.clone()]),
        "backend_selection_owner" => Ok(vec![
            "Project-scoped config cannot override machine-local provider, auth".to_string(),
            "configuration profile selection".to_string(),
        ]),
        "trust_and_discovery" => Ok(vec![
            "project-scoped config files only when you trust the project".to_string(),
            "standalone TOML files under .codex/agents/".to_string(),
        ]),
        "parallelism" => Ok(vec![
            "4 available concurrency slots".to_string(),
            "including the root thread".to_string(),
            "at most 3 parallel child agents".to_string(),
        ]),
        "role_precedence" => Ok(vec![
            "reapplies the parent turn live runtime overrides".to_string(),
            "sandbox and approval choices".to_string(),
            "model_reasoning_effort inherit from the parent session when omitted".to_string(),
        ]),
        "shared_filesystem" => Ok(vec![
            "All agents share the same container and filesystem".to_string(),
            "edits made by one agent are immediately visible to all other agents".to_string(),
        ]),
        "delegation_modes" => Ok(vec![
            "With Ultra, ChatGPT can proactively delegate work".to_string(),
            "At most intelligence levels, ask for delegation explicitly".to_string(),
        ]),
        _ => bail!("Codex V2 runtime evidence unknown provenance claim `{claim}`"),
    }
}

pub(crate) fn codex_v2_host_version(evidence: &CodexV2RuntimeEvidence) -> String {
    evidence.installed_version.stdout.clone()
}

pub fn validate_dispatch_evidence(evidence: &DispatchEvidenceV1) -> Result<()> {
    if evidence.schema_version != 1 {
        bail!("unsupported dispatch evidence schema_version");
    }
    if evidence.package_digest.trim().is_empty()
        || evidence.host_version.trim().is_empty()
        || evidence.nonce.trim().is_empty()
    {
        bail!("dispatch evidence package_digest, host_version, and nonce must not be blank");
    }
    if evidence.requested_dispatch.semantic_role.trim().is_empty()
        || evidence.requested_dispatch.profile.trim().is_empty()
        || evidence.requested_dispatch.model.trim().is_empty()
    {
        bail!("dispatch evidence requested dispatch must name role, profile, and model");
    }
    if evidence.child_identity.host.trim().is_empty()
        || evidence.child_identity.role.trim().is_empty()
        || evidence.child_identity.agent_role.trim().is_empty()
    {
        bail!("dispatch evidence child identity must name host, role, and agent_role");
    }
    if evidence.raw_evidence_refs.is_empty()
        || evidence
            .raw_evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        bail!("dispatch evidence must include raw evidence references");
    }
    if evidence.verdict == GuaranteeLevel::Deterministic {
        let effective_model = evidence.effective_model.as_deref().ok_or_else(|| {
            product_error!("deterministic dispatch evidence must include observed effective_model")
        })?;
        if effective_model != evidence.requested_dispatch.model {
            bail!(
                "deterministic dispatch evidence effective_model `{effective_model}` does not match requested model `{}`",
                evidence.requested_dispatch.model
            );
        }
        if let Some(requested_effort) = evidence.requested_dispatch.effort.as_deref() {
            let effective_effort = evidence.effective_effort.as_deref().ok_or_else(|| {
                product_error!(
                    "deterministic dispatch evidence must include observed effective_effort"
                )
            })?;
            if effective_effort != requested_effort {
                bail!(
                    "deterministic dispatch evidence effective_effort `{effective_effort}` does not match requested effort `{requested_effort}`"
                );
            }
        }
    }
    Ok(())
}

pub fn validate_dispatch_evidence_for_adapter(
    evidence: &DispatchEvidenceV1,
    contract: &AdapterContractV1,
) -> Result<()> {
    validate_adapter_contract(contract)?;
    validate_dispatch_evidence(evidence)?;
    if evidence.child_identity.host != contract.capability.host {
        bail!(
            "dispatch evidence host `{}` does not match adapter host `{}`",
            evidence.child_identity.host,
            contract.capability.host
        );
    }
    if !contract
        .dispatch_evidence
        .required_verdicts
        .contains(&evidence.verdict)
    {
        bail!("dispatch evidence verdict is not allowed by adapter contract");
    }
    let request = contract
        .routing_intent
        .role_requests
        .iter()
        .find(|request| request.semantic_role == evidence.requested_dispatch.semantic_role)
        .ok_or_else(|| {
            product_error!(
                "dispatch evidence role `{}` is not declared by adapter contract",
                evidence.requested_dispatch.semantic_role
            )
        })?;
    if evidence.child_identity.role != evidence.requested_dispatch.semantic_role {
        bail!(
            "dispatch evidence child role `{}` does not match requested semantic role `{}`",
            evidence.child_identity.role,
            evidence.requested_dispatch.semantic_role
        );
    }
    if evidence.requested_dispatch.model != request.requested_model {
        bail!(
            "dispatch evidence requested model `{}` does not match adapter role request `{}`",
            evidence.requested_dispatch.model,
            request.requested_model
        );
    }
    if evidence.requested_dispatch.effort != request.requested_effort {
        bail!("dispatch evidence requested effort does not match adapter role request");
    }
    if evidence.verdict == GuaranteeLevel::Deterministic {
        require_deterministic_observation(evidence, contract)?;
        require_live_nonce_observation(evidence, contract)?;
    }
    Ok(())
}

pub(crate) fn require_deterministic_observation(
    evidence: &DispatchEvidenceV1,
    contract: &AdapterContractV1,
) -> Result<()> {
    if contract.capability.observability.effective_model != GuaranteeLevel::Deterministic {
        bail!(
            "deterministic dispatch evidence for adapter `{}` is not allowed because effective model observability is {:?}",
            contract.adapter.adapter_id,
            contract.capability.observability.effective_model
        );
    }
    if evidence.requested_dispatch.effort.is_some()
        && contract.capability.effort_control.level != GuaranteeLevel::Deterministic
    {
        bail!(
            "deterministic dispatch evidence for adapter `{}` is not allowed because effective effort control is {:?}",
            contract.adapter.adapter_id,
            contract.capability.effort_control.level
        );
    }
    Ok(())
}

pub(crate) fn require_live_nonce_observation(
    evidence: &DispatchEvidenceV1,
    contract: &AdapterContractV1,
) -> Result<()> {
    if contract.capability.observability.effective_model == GuaranteeLevel::Deterministic
        && !evidence.raw_evidence_refs.iter().any(|reference| {
            reference.starts_with("host-output:") || reference.starts_with("codex-session:")
        })
    {
        bail!(
            "deterministic dispatch evidence for adapter `{}` requires a live host output reference",
            contract.adapter.adapter_id
        );
    }
    if evidence
        .raw_evidence_refs
        .iter()
        .any(|reference| reference.contains("status:not-run"))
    {
        bail!(
            "dispatch evidence for adapter `{}` cannot cite a not-run host output",
            contract.adapter.adapter_id
        );
    }
    Ok(())
}

pub(crate) fn validate_adapter_contract(contract: &AdapterContractV1) -> Result<()> {
    if contract.schema_version != 1
        || contract.routing_intent.schema_version != 1
        || contract.capability.schema_version != 1
        || contract.adapter.schema_version != 1
        || contract.dispatch_evidence.schema_version != 1
        || contract.planr_handoff.schema_version != 1
    {
        bail!("unsupported adapter contract schema_version");
    }
    if contract.routing_intent.semantic_roles.is_empty() {
        bail!("adapter contract must declare semantic roles");
    }
    if contract.routing_intent.role_requests.is_empty() {
        bail!("adapter contract must declare role requests");
    }
    let semantic_roles = contract
        .routing_intent
        .semantic_roles
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for request in &contract.routing_intent.role_requests {
        if !semantic_roles.contains(&request.semantic_role) {
            bail!(
                "adapter contract role request references unknown semantic role `{}`",
                request.semantic_role
            );
        }
        if request.requested_model.trim().is_empty() || request.instructions.trim().is_empty() {
            bail!("adapter contract role requests must include model and instructions");
        }
    }
    if contract.capability.host.trim().is_empty() || contract.adapter.adapter_id.trim().is_empty() {
        bail!("adapter contract host and adapter_id must not be blank");
    }
    if contract.capability.runtime_class != contract.adapter.runtime_class {
        bail!("adapter contract runtime_class mismatch");
    }
    validate_runtime_behavior(&contract.capability)?;
    if contract.adapter.accepts_intent_schema != "RoutingIntentV1" {
        bail!("adapter contract must accept RoutingIntentV1");
    }
    if contract.capability.model_control.field.trim().is_empty()
        || contract.capability.effort_control.field.trim().is_empty()
    {
        bail!("adapter contract control fields must not be blank");
    }
    if contract
        .adapter
        .dispatch_recipe
        .invocation
        .trim()
        .is_empty()
    {
        bail!("adapter contract dispatch recipe invocation must not be blank");
    }
    if contract.adapter.dispatch_recipe.required_fields.is_empty() {
        bail!("adapter contract dispatch recipe must declare required fields");
    }
    for required in &contract.routing_intent.required_guarantees {
        let Some(guarantee) = contract.capability.guarantees.get(required) else {
            bail!("adapter contract requires undeclared guarantee `{required}`");
        };
        if guarantee.level == GuaranteeLevel::Unsupported {
            bail!("adapter contract required guarantee `{required}` is unsupported");
        }
    }
    for (name, guarantee) in &contract.capability.guarantees {
        if name.trim().is_empty() || guarantee.reason.trim().is_empty() {
            bail!("adapter contract guarantee names and reasons must not be blank");
        }
    }
    let verdicts = contract
        .dispatch_evidence
        .required_verdicts
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for verdict in [
        GuaranteeLevel::Deterministic,
        GuaranteeLevel::Advisory,
        GuaranteeLevel::Unsupported,
    ] {
        if !verdicts.contains(&verdict) {
            bail!("adapter contract dispatch evidence must enumerate all guarantee verdicts");
        }
    }
    if contract.dispatch_evidence.receipt_schema != "DispatchEvidenceV1" {
        bail!("adapter contract dispatch evidence must reference DispatchEvidenceV1");
    }
    Ok(())
}

pub(crate) fn validate_runtime_behavior(capability: &HostCapabilityV1) -> Result<()> {
    let behavior = &capability.runtime_behavior;
    if behavior.capability_version.trim().is_empty()
        || behavior.installed_host_version_source.trim().is_empty()
        || behavior.backend_selection_source.trim().is_empty()
        || behavior.trust_boundary.trim().is_empty()
        || behavior.discovery_behavior.trim().is_empty()
    {
        bail!("adapter contract runtime behavior fields must not be blank");
    }
    if behavior.role_precedence.is_empty()
        || behavior
            .role_precedence
            .iter()
            .any(|entry| entry.trim().is_empty())
    {
        bail!("adapter contract runtime behavior must declare role precedence");
    }
    if behavior.source_references.is_empty()
        || behavior
            .source_references
            .iter()
            .any(|entry| entry.trim().is_empty())
    {
        bail!("adapter contract runtime behavior must declare source references");
    }
    if capability.host == "codex" {
        let evidence = codex_v2_runtime_evidence()?;
        let expected_source_reference = codex_v2_runtime_evidence_reference();
        let expected_host_version = codex_v2_host_version(&evidence);
        if behavior.capability_version != evidence.evidence_id {
            bail!("Codex V2 runtime capability_version must match parsed evidence_id");
        }
        if behavior.installed_host_version_source
            != format!(
                "{} via {}",
                evidence.installed_version.stdout, evidence.installed_version.command
            )
        {
            bail!(
                "Codex V2 runtime installed host version must match parsed evidence command output"
            );
        }
        if capability.host_version_constraints.minimum.as_deref()
            != Some(expected_host_version.as_str())
            || capability.host_version_constraints.maximum.as_deref()
                != Some(expected_host_version.as_str())
        {
            bail!("Codex V2 host_version_constraints must freeze the parsed evidence version");
        }
        if !capability
            .discovery_artifacts
            .iter()
            .any(|artifact| artifact == &evidence.evidence_id)
        {
            bail!("Codex V2 discovery artifacts must include the parsed evidence id");
        }
        if behavior.source_references != vec![expected_source_reference] {
            bail!(
                "Codex V2 runtime source reference must match the digest-bound evidence artifact"
            );
        }
        if capability.parallelism.max_parallel_children
            != evidence.parallelism.max_parallel_children
        {
            bail!("Codex V2 runtime must declare exactly the parsed evidence child slots");
        }
        if behavior.backend_selection_source != evidence.backend_selection_owner {
            bail!("Codex V2 backend selection source must match parsed evidence");
        }
        if behavior.trust_boundary != evidence.trust_and_discovery.trust_boundary
            || behavior.discovery_behavior != evidence.trust_and_discovery.discovery_behavior
        {
            bail!("Codex V2 trust and discovery behavior must match parsed evidence");
        }
        if behavior.role_precedence != evidence.role_precedence {
            bail!("Codex V2 role precedence must match parsed evidence");
        }
        if behavior.shared_filesystem != evidence.shared_filesystem {
            bail!("Codex V2 shared filesystem flag must match parsed evidence");
        }
        if behavior.delegation_modes != evidence.delegation_modes {
            bail!("Codex V2 delegation modes must match parsed evidence");
        }
        if !evidence
            .codex_ownership
            .iter()
            .any(|owner| owner.contains("execution timing and orchestration"))
            || !evidence
                .switchloom_ownership
                .iter()
                .any(|owner| owner.contains("semantic role compilation"))
        {
            bail!("Codex V2 ownership boundaries must be recorded in parsed evidence");
        }
        if capability.runtime_class != RuntimeClass::NativeSubagent {
            bail!("Codex V2 runtime must be a native-subagent contract");
        }
        if !behavior.shared_filesystem {
            bail!("Codex V2 runtime must declare shared filesystem behavior");
        }
        if !behavior.delegation_modes.explicit_agent_type_dispatch {
            bail!("Codex V2 runtime must declare explicit agent_type dispatch");
        }
        if !behavior.delegation_modes.ultra_auto_delegation
            || !behavior
                .delegation_modes
                .automatic_delegation_requires_ultra
        {
            bail!("Codex V2 runtime must declare Ultra automatic delegation boundaries");
        }
    }
    Ok(())
}

pub(crate) fn codex_v2_runtime_evidence_reference() -> String {
    format!(
        "docs/codex-v2-runtime-evidence.json#sha256:{}",
        sha256(CODEX_V2_RUNTIME_EVIDENCE_JSON.as_bytes())
    )
}
