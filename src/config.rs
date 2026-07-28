use crate::contracts::*;
use crate::error::{Result, ResultContext};
use crate::hosts::*;
use crate::{bail, product_error};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const SETUP_CONFIG_PATH: &str = ".switchloom/config.toml";
pub const SETUP_RECIPE_PREFIX: &str = "sw1_";
pub(crate) const MAX_SETUP_RECIPE_BYTES: usize = 65_536;
pub(crate) const MAX_SETUP_RECIPE_ENCODED_BYTES: usize =
    encoded_base64url_len(MAX_SETUP_RECIPE_BYTES);

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
        workflow: None,
    };
    validate_setup_spec(&spec)?;
    Ok(spec)
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;

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
    if let Some(workflow) = &spec.workflow {
        validate_workflow_request(workflow)?;
        let supported_adapter = matches!(
            (
                canonical_host,
                workflow.coding_agent,
                workflow.execution_path
            ),
            ("pi", CodingAgentRuntime::Pi, ExecutionPath::Extension)
                | (
                    "opencode",
                    CodingAgentRuntime::OpenCode,
                    ExecutionPath::Native
                )
        );
        if !supported_adapter {
            bail!(
                "workflow requests are currently supported only for Pi Subagents and OpenCode native"
            );
        }
        for (role, workflow_role) in &workflow.roles {
            let selection = spec.selected_roles.get(role).ok_or_else(|| {
                product_error!(
                    "workflow role `{role}` must have a matching setup selected_roles entry"
                )
            })?;
            if selection.model != format!("{}/{}", workflow_role.provider, workflow_role.model) {
                bail!("workflow role `{role}` provider/model must match its setup selection");
            }
            if selection.effort != workflow_role.thinking {
                bail!("workflow role `{role}` thinking must match its setup effort");
            }
        }
        if workflow.roles.len() != spec.selected_roles.len() {
            bail!("workflow roles must exactly match setup selected_roles");
        }
    }
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
        .ok_or_else(|| product_error!("setup recipe must start with `{SETUP_RECIPE_PREFIX}`"))?;
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

/// Validate the runtime-aware selection before an exporter is allowed to turn
/// it into repository artifacts. Certified tuples are actionable; experimental
/// and planned tuples remain catalog-visible but cannot be applied.
pub fn validate_workflow_request(request: &WorkflowRequestV1) -> Result<()> {
    if request.schema_version != 1 {
        bail!(
            "unsupported workflow schema_version {}",
            request.schema_version
        );
    }
    if request.roles.is_empty() {
        bail!("workflow roles must not be empty");
    }
    let expected = match (request.coding_agent, request.execution_path) {
        (CodingAgentRuntime::Codex, ExecutionPath::Native) => (
            ValidationStatus::Certified,
            ParentModelGuidance::CurrentSession,
            &[WorkflowTopology::RoleDispatch][..],
        ),
        (CodingAgentRuntime::Pi, ExecutionPath::Extension) => (
            ValidationStatus::Experimental,
            ParentModelGuidance::RuntimeManaged,
            &[WorkflowTopology::Sequential][..],
        ),
        (CodingAgentRuntime::Pi, ExecutionPath::Native) => {
            bail!("Pi native workflow is no longer supported; regenerate the recipe")
        }
        (CodingAgentRuntime::OpenCode, ExecutionPath::Native) => (
            ValidationStatus::Experimental,
            ParentModelGuidance::RuntimeManaged,
            &[WorkflowTopology::RoleDispatch][..],
        ),
        (CodingAgentRuntime::ClaudeCode, ExecutionPath::Sidecar) => (
            ValidationStatus::Planned,
            ParentModelGuidance::ExternalSetupRequired,
            &[WorkflowTopology::RoleDispatch][..],
        ),
        (CodingAgentRuntime::ClaudeCode, ExecutionPath::Native) => (
            ValidationStatus::Planned,
            ParentModelGuidance::RuntimeManaged,
            &[WorkflowTopology::RoleDispatch][..],
        ),
        (CodingAgentRuntime::Cursor, ExecutionPath::Native) => (
            ValidationStatus::Experimental,
            ParentModelGuidance::RuntimeManaged,
            &[WorkflowTopology::RoleDispatch][..],
        ),
        _ => bail!(
            "workflow runtime `{}` does not support execution path `{}`",
            workflow_runtime_id(request.coding_agent),
            workflow_path_id(request.execution_path)
        ),
    };
    if request.validation_status != expected.0 {
        bail!("workflow validation_status does not match the runtime capability");
    }
    if request.parent_model != expected.1 {
        bail!("workflow parent_model guidance does not match the runtime capability");
    }
    if !expected.2.contains(&request.topology) {
        bail!("workflow topology is not supported by the runtime capability");
    }
    if matches!(
        request.coding_agent,
        CodingAgentRuntime::Pi | CodingAgentRuntime::OpenCode
    ) && request.model_access.is_some()
    {
        bail!(
            "workflow model_access is no longer supported for Pi or OpenCode; regenerate the recipe"
        );
    }
    if let Some(access) = &request.model_access {
        validate_model_access_profile(request, access)?;
    }
    if request.coding_agent == CodingAgentRuntime::Pi {
        let child_roles = ["implementer", "reviewer", "verifier"];
        if request.roles.len() != child_roles.len()
            || child_roles
                .iter()
                .any(|role| !request.roles.contains_key(*role))
        {
            bail!(
                "Pi Subagents workflows must define exactly implementer, reviewer, and verifier child roles"
            );
        }
    }
    for (role, selection) in &request.roles {
        validate_setup_identifier("workflow role", role)?;
        if selection.provider.trim().is_empty() || selection.model.trim().is_empty() {
            bail!("workflow role `{role}` provider and model must not be blank");
        }
        reject_workflow_secret_like("provider", &selection.provider)?;
        reject_workflow_secret_like("model", &selection.model)?;
        if matches!(
            request.coding_agent,
            CodingAgentRuntime::Pi | CodingAgentRuntime::OpenCode
        ) && selection.selection_mode == ModelSelectionMode::GatewayAuto
        {
            bail!(
                "workflow gateway_auto selection is no longer supported for Pi or OpenCode; regenerate the recipe"
            );
        }
        if selection.selection_mode == ModelSelectionMode::GatewayAuto
            && (request
                .model_access
                .as_ref()
                .is_some_and(|access| access.kind != ModelAccessKind::HostedGateway)
                || selection.provider != "openrouter"
                || selection.model != "auto")
        {
            bail!("workflow gateway_auto selection requires the OpenRouter hosted gateway");
        }
        if selection.selection_mode == ModelSelectionMode::Fixed
            && selection.model == "auto"
            && !(request.coding_agent == CodingAgentRuntime::OpenCode
                && selection.provider == "openrouter")
        {
            bail!("workflow fixed selection must not use dynamic model `auto`");
        }
        if let Some(access) = &request.model_access {
            match access.kind {
                ModelAccessKind::HostedGateway | ModelAccessKind::SelfHostedProxy => {
                    if access.provider.as_deref() != Some(selection.provider.as_str()) {
                        bail!(
                            "workflow role `{role}` provider must match the selected model_access profile"
                        );
                    }
                }
                ModelAccessKind::RuntimeManaged | ModelAccessKind::Direct => {}
            }
        }
        let provider_models = workflow_provider_models(request.coding_agent);
        let Some((_, allowed_models)) = provider_models
            .iter()
            .find(|(provider, _)| *provider == selection.provider)
        else {
            bail!(
                "workflow role `{role}` provider `{}` is unsupported by {} {}",
                selection.provider,
                workflow_runtime_id(request.coding_agent),
                workflow_path_id(request.execution_path)
            );
        };
        if !allowed_models.contains(&selection.model.as_str()) {
            bail!(
                "workflow role `{role}` model `{}` is unsupported for provider `{}`",
                selection.model,
                selection.provider
            );
        }
        if request.coding_agent == CodingAgentRuntime::Codex
            && !selection.fallback_models.is_empty()
        {
            bail!("workflow role `{role}` fallbacks are not supported by Codex native");
        }
        for fallback in &selection.fallback_models {
            if fallback.trim().is_empty() {
                bail!("workflow role `{role}` fallback model must not be blank");
            }
            if workflow_qualified_model(request.coding_agent, fallback).is_none() {
                bail!(
                    "workflow role `{role}` fallback model `{fallback}` is unsupported by the runtime capability"
                );
            }
        }
    }
    if request.validation_status != ValidationStatus::Certified {
        bail!(
            "workflow is {} and cannot be applied as certified support",
            workflow_status_id(request.validation_status)
        );
    }
    Ok(())
}

fn validate_model_access_profile(
    request: &WorkflowRequestV1,
    access: &ModelAccessProfileV1,
) -> Result<()> {
    let Some((runtime, kind, status, provider)) = workflow_access_profile(&access.id) else {
        bail!(
            "workflow model_access profile `{}` is unsupported",
            access.id
        );
    };
    if runtime != request.coding_agent || kind != access.kind {
        bail!("workflow model_access profile does not match the runtime or access kind");
    }
    if status != request.validation_status {
        bail!("workflow model_access profile status does not match the runtime capability");
    }
    if let Some(required_provider) = provider {
        if access.provider.as_deref() != Some(required_provider) {
            bail!(
                "workflow model_access profile `{}` requires provider `{required_provider}`",
                access.id
            );
        }
    } else if access.provider.is_some() {
        bail!(
            "workflow model_access profile `{}` must not declare a provider",
            access.id
        );
    }
    reject_workflow_secret_like("model_access id", &access.id)?;
    if let Some(provider) = &access.provider {
        reject_workflow_secret_like("model_access provider", provider)?;
    }
    if let Some(reference) = &access.credential_reference {
        if !reference.starts_with("env:")
            || reference.len() == 4
            || !reference[4..].chars().all(|character| {
                character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
            })
        {
            bail!("workflow credential_reference must be an environment-variable reference");
        }
    }
    Ok(())
}

fn reject_workflow_secret_like(kind: &str, value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("bearer ")
        || lower.starts_with("sk-")
        || lower.contains("token=")
    {
        bail!("workflow {kind} must not contain a credential value");
    }
    Ok(())
}

fn workflow_access_profile(
    id: &str,
) -> Option<(
    CodingAgentRuntime,
    ModelAccessKind,
    ValidationStatus,
    Option<&'static str>,
)> {
    use CodingAgentRuntime::{ClaudeCode, Codex, Cursor};
    use ModelAccessKind::{Direct, HostedGateway, RuntimeManaged};
    use ValidationStatus::{Certified, Experimental, Planned};
    match id {
        "codex-runtime-managed" => Some((Codex, RuntimeManaged, Certified, None)),
        "cursor-direct" => Some((Cursor, Direct, Experimental, None)),
        "cursor-gateway" => Some((
            Cursor,
            HostedGateway,
            Experimental,
            Some("openai-compatible"),
        )),
        "claude-runtime-managed" => Some((ClaudeCode, RuntimeManaged, Planned, None)),
        "claude-compatible-gateway" => {
            Some((ClaudeCode, HostedGateway, Experimental, Some("anthropic")))
        }
        _ => None,
    }
}

pub fn workflow_capability_catalog_value() -> Value {
    json!({
        "schemaVersion": 1,
        "capabilities": [
            {"codingAgent": "codex", "executionPath": "native", "validationStatus": "certified", "parentModel": "current-session", "topologies": ["role-dispatch"], "providers": workflow_provider_capabilities(CodingAgentRuntime::Codex)},
            {"codingAgent": "pi", "executionPath": "extension", "validationStatus": "experimental", "parentModel": "runtime-managed", "topologies": ["sequential"], "providers": workflow_provider_capabilities(CodingAgentRuntime::Pi)},
            {"codingAgent": "opencode", "executionPath": "native", "validationStatus": "experimental", "parentModel": "runtime-managed", "topologies": ["role-dispatch"], "providers": workflow_provider_capabilities(CodingAgentRuntime::OpenCode)},
            {"codingAgent": "claude-code", "executionPath": "sidecar", "validationStatus": "planned", "parentModel": "external-setup-required", "topologies": ["role-dispatch"]},
            {"codingAgent": "claude-code", "executionPath": "native", "validationStatus": "planned", "parentModel": "runtime-managed", "topologies": ["role-dispatch"], "providers": workflow_provider_capabilities(CodingAgentRuntime::ClaudeCode)},
            {"codingAgent": "cursor", "executionPath": "native", "validationStatus": "experimental", "parentModel": "runtime-managed", "topologies": ["role-dispatch"], "providers": workflow_provider_capabilities(CodingAgentRuntime::Cursor)},
        ],
        "modelAccessProfiles": [
            {"id":"codex-runtime-managed","kind":"runtime_managed","status":"certified"},
            {"id":"cursor-direct","kind":"direct","status":"experimental"},
            {"id":"cursor-gateway","kind":"hosted_gateway","status":"experimental","provider":"openai-compatible"},
            {"id":"claude-runtime-managed","kind":"runtime_managed","status":"planned"},
            {"id":"claude-compatible-gateway","kind":"hosted_gateway","status":"experimental","provider":"anthropic"}
        ]
    })
}

fn workflow_provider_models(
    runtime: CodingAgentRuntime,
) -> &'static [(&'static str, &'static [&'static str])] {
    match runtime {
        CodingAgentRuntime::Codex => {
            &[("openai", &["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"])]
        }
        CodingAgentRuntime::Pi => &[
            (
                "openai-codex",
                &["gpt-5.6-luna", "gpt-5.6-terra", "gpt-5.6-sol"],
            ),
            (
                "anthropic",
                &["claude-sonnet-5", "claude-opus-5", "claude-fable-5"],
            ),
            (
                "openrouter",
                &[
                    "google/gemini-3.5-flash-lite",
                    "google/gemini-3.6-flash",
                    "x-ai/grok-4.5",
                    "moonshotai/kimi-k3",
                    "minimax/minimax-m3",
                    "z-ai/glm-5.2",
                    "stepfun/step-3.7-flash",
                    "xiaomi/mimo-v2.5",
                ],
            ),
        ],
        CodingAgentRuntime::OpenCode => &[
            ("openai", &["gpt-5-nano"]),
            (
                "openrouter",
                &["auto", "google/gemini-3.6-flash", "x-ai/grok-4.5"],
            ),
        ],
        CodingAgentRuntime::ClaudeCode => &[("anthropic", &["sonnet", "opus"])],
        CodingAgentRuntime::Cursor => &[("openai", &["gpt-5.5", "gpt-5.4-mini"])],
    }
}

pub(crate) fn workflow_qualified_model(runtime: CodingAgentRuntime, model: &str) -> Option<String> {
    workflow_provider_models(runtime)
        .iter()
        .find_map(|(provider, models)| {
            models
                .contains(&model)
                .then(|| format!("{provider}/{model}"))
        })
}

fn workflow_provider_capabilities(runtime: CodingAgentRuntime) -> Vec<Value> {
    workflow_provider_models(runtime)
        .iter()
        .map(|(provider, models)| json!({ "id": provider, "models": models }))
        .collect()
}

fn workflow_runtime_id(runtime: CodingAgentRuntime) -> &'static str {
    match runtime {
        CodingAgentRuntime::Codex => "codex",
        CodingAgentRuntime::Pi => "pi",
        CodingAgentRuntime::OpenCode => "opencode",
        CodingAgentRuntime::ClaudeCode => "claude-code",
        CodingAgentRuntime::Cursor => "cursor",
    }
}

fn workflow_path_id(path: ExecutionPath) -> &'static str {
    match path {
        ExecutionPath::Native => "native",
        ExecutionPath::Extension => "extension",
        ExecutionPath::Gateway => "gateway",
        ExecutionPath::Sidecar => "sidecar",
    }
}

fn workflow_status_id(status: ValidationStatus) -> &'static str {
    match status {
        ValidationStatus::Certified => "certified",
        ValidationStatus::Experimental => "experimental",
        ValidationStatus::Planned => "planned",
    }
}

pub fn setup_contract_catalog_value() -> Result<Value> {
    let hosts = [
        "codex",
        "claude-code",
        "cursor",
        "opencode",
        "pi",
        "mixed-host",
    ]
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
        "workflowCapabilities": workflow_capability_catalog_value(),
        "hosts": hosts,
    }))
}

pub fn setup_contract_catalog_json() -> Result<String> {
    let mut output = serde_json::to_string_pretty(&setup_contract_catalog_value()?)?;
    output.push('\n');
    Ok(output)
}

pub(crate) fn encode_base64url(bytes: &[u8]) -> String {
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

pub(crate) const fn encoded_base64url_len(decoded_len: usize) -> usize {
    let full_chunks = decoded_len / 3;
    match decoded_len % 3 {
        0 => full_chunks * 4,
        1 => full_chunks * 4 + 2,
        _ => full_chunks * 4 + 3,
    }
}

pub(crate) fn validate_base64url_payload_len(input: &str) -> Result<()> {
    if input.len() > MAX_SETUP_RECIPE_ENCODED_BYTES {
        bail!(
            "setup recipe payload exceeds {MAX_SETUP_RECIPE_ENCODED_BYTES} base64url characters for {MAX_SETUP_RECIPE_BYTES} decoded bytes"
        );
    }
    Ok(())
}

pub(crate) fn decode_base64url(input: &str) -> Result<Vec<u8>> {
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
            .ok_or_else(|| product_error!("invalid base64url payload"))?;
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
