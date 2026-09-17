//! Canonical model defaults and capability instructions shared by routing and the website.
use crate::bail;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSettings {
    pub model: String,
    pub effort: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub model: String,
    pub effort: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDefinition {
    pub id: String,
    pub label: String,
    pub efforts: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SlotDefinition {
    pub id: String,
    pub model: String,
    pub effort: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub label: String,
    pub default_owner: Option<String>,
    pub instructions: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub reasoning_efforts: Vec<String>,
    pub models: Vec<ModelDefinition>,
    pub slots: Vec<SlotDefinition>,
    pub capabilities: Vec<Capability>,
}
impl Catalog {
    pub fn default_profiles(&self) -> BTreeMap<String, Profile> {
        self.slots
            .iter()
            .map(|slot| {
                (
                    slot.id.clone(),
                    Profile {
                        model: slot.model.clone(),
                        effort: slot.effort.clone(),
                        capabilities: self
                            .capabilities
                            .iter()
                            .filter(|cap| cap.default_owner.as_deref() == Some(&slot.id))
                            .map(|cap| cap.id.clone())
                            .collect(),
                    },
                )
            })
            .collect()
    }
    pub fn validate_profiles(&self, profiles: &BTreeMap<String, Profile>) -> Result<()> {
        if profiles.is_empty() {
            bail!("provide at least one configured model");
        }
        let mut assigned = BTreeSet::new();
        for (id, profile) in profiles {
            if !self.slots.iter().any(|slot| &slot.id == id) {
                bail!("unknown model slot");
            }
            self.validate_settings(&ModelSettings {
                model: profile.model.clone(),
                effort: profile.effort.clone(),
            })?;
            for cap in &profile.capabilities {
                if !self.capabilities.iter().any(|known| &known.id == cap) {
                    bail!("unknown capability");
                }
                if !assigned.insert(cap) {
                    bail!("each capability must have exactly one owner");
                }
            }
        }
        if assigned.is_empty() {
            bail!("assign at least one capability");
        }
        Ok(())
    }
    pub fn validate_settings(&self, settings: &ModelSettings) -> Result<()> {
        let suffix = settings.model.strip_prefix("gpt-").unwrap_or("");
        if suffix.is_empty()
            || settings.model.len() > 128
            || !suffix.starts_with(|c: char| c.is_ascii_alphanumeric())
            || !suffix.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')
            })
        {
            bail!("provide a valid GPT model ID of at most 128 characters");
        }
        if !self.reasoning_efforts.contains(&settings.effort) {
            bail!("unsupported Codex reasoning effort");
        }
        if let Some(model) = self.models.iter().find(|model| model.id == settings.model) {
            if !model.efforts.contains(&settings.effort) {
                bail!("reasoning effort is not supported by the selected model");
            }
        }
        // New GPT IDs need no binary update. Codex validates actual availability.
        Ok(())
    }
}

pub fn load_catalog() -> Result<Catalog> {
    Ok(toml::from_str(include_str!("../catalog.toml"))?)
}
pub fn catalog_json() -> Result<String> {
    let catalog = load_catalog()?;
    catalog.validate_profiles(&catalog.default_profiles())?;
    Ok(serde_json::to_string_pretty(&catalog)? + "\n")
}
