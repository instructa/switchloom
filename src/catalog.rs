//! Canonical model defaults and capability instructions shared by routing and the website.
use crate::error::Result;
use crate::{bail, product_error};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const JUDGE_MODEL: &str = "jev-1.13.0";
pub const MISSING_CONTEXT_FLOOR: f64 = 0.7;
pub const ROUTINE_ACT_FLOOR: f64 = 0.7;
pub const ROUTINE_SUGGEST_FLOOR: f64 = 0.5;
pub const ELEVATED_ACT_FLOOR: f64 = 0.85;
pub const ELEVATED_SUGGEST_FLOOR: f64 = 0.7;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCriteria {
    pub what: String,
    pub not_for: String,
    pub examples: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub label: String,
    pub default_owner: Option<String>,
    pub routable: bool,
    pub stakes: String,
    pub instructions: String,
    pub criteria: CapabilityCriteria,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub model_id_pattern: String,
    pub reasoning_efforts: Vec<String>,
    pub completion_instructions: String,
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
        for capability in &self.capabilities {
            if !matches!(capability.stakes.as_str(), "routine" | "elevated") {
                bail!("capability stakes must be routine or elevated");
            }
            if capability.criteria.what.trim().is_empty()
                || capability.criteria.not_for.trim().is_empty()
                || capability.criteria.examples.is_empty()
                || capability.criteria.not_for == capability.instructions
            {
                bail!("classifier criteria must contrast the assignment instructions");
            }
        }
        Ok(())
    }
    pub fn routable<'a>(&'a self, profiles: &'a BTreeMap<String, Profile>) -> Vec<&'a Capability> {
        self.capabilities
            .iter()
            .filter(|cap| {
                cap.routable
                    && profiles
                        .values()
                        .any(|profile| profile.capabilities.contains(&cap.id))
            })
            .collect()
    }
    pub fn floors(&self, capability: &str) -> (f64, f64) {
        let stakes = self
            .capabilities
            .iter()
            .find(|cap| cap.id == capability)
            .map(|cap| cap.stakes.as_str());
        if stakes == Some("elevated") {
            (ELEVATED_ACT_FLOOR, ELEVATED_SUGGEST_FLOOR)
        } else {
            (ROUTINE_ACT_FLOOR, ROUTINE_SUGGEST_FLOOR)
        }
    }
    pub fn validate_settings(&self, settings: &ModelSettings) -> Result<()> {
        let pattern = regex_lite::Regex::new(&self.model_id_pattern)
            .map_err(|error| product_error!("invalid catalog model_id_pattern: {error}"))?;
        let full_match = pattern
            .find(&settings.model)
            .is_some_and(|found| found.start() == 0 && found.end() == settings.model.len());
        if settings.model.len() > 128 || !full_match {
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
