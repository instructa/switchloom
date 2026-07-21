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
