use crate::contracts::*;
use crate::error::Result;
use crate::{bail, product_error};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrySignature {
    pub algorithm: String,
    pub signer: String,
    pub content_sha256: String,
    pub value: String,
}

#[cfg(test)]
#[path = "tests/registry.rs"]
mod tests;

pub fn sign_registry(
    content: &[u8],
    signer: &str,
    private_key_hex: &str,
) -> Result<RegistrySignature> {
    if signer.trim().is_empty() {
        bail!("registry signer must not be blank");
    }
    let seed = decode_hex::<32>(private_key_hex.trim()).ok_or_else(|| {
        product_error!("private key file must contain exactly 64 hexadecimal characters")
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
        .ok_or_else(|| product_error!("trusted registry public key is invalid"))?;
    let signature_bytes = decode_hex::<64>(&signature.value)
        .ok_or_else(|| product_error!("registry signature value is invalid"))?;
    let key = VerifyingKey::from_bytes(&public_key)?;
    key.verify(content, &Signature::from_bytes(&signature_bytes))?;
    Ok(())
}
pub(crate) fn render_registry(source: &PolicySource) -> Result<String> {
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

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
        output
    })
}

pub(crate) fn decode_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2 {
        return None;
    }
    let mut decoded = [0_u8; N];
    for (index, output) in decoded.iter_mut().enumerate() {
        *output = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(decoded)
}
