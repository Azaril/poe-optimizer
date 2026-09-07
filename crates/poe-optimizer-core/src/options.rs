//! Backend-neutral selection and external encounter inputs.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationOptions {
    #[serde(default)]
    pub selection: Option<SkillSelection>,
    #[serde(default)]
    pub encounter: Option<EncounterOverrides>,
}

/// One-based imported group/action indexes for diagnostic evaluation.
/// Search will resolve stable skill requirements to these selectors per candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillSelection {
    pub socket_group: u32,
    #[serde(default)]
    pub active_skill: Option<u32>,
    #[serde(default)]
    pub minion_skill: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterOverrides {
    pub name: String,
    #[serde(default)]
    pub enemy_level: Option<u32>,
    #[serde(default)]
    pub boss: Option<BossKind>,
    #[serde(default)]
    pub incoming_hit: Option<DamageAmounts>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BossKind {
    Normal,
    Standard,
    Pinnacle,
    Uber,
}

/// Explicit incoming damage amounts. Supplying this replaces all five components.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageAmounts {
    pub physical: f64,
    pub fire: f64,
    pub cold: f64,
    pub lightning: f64,
    pub chaos: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    Boolean(bool),
    Number(f64),
    Text(String),
}

/// Effective inputs are evidence for interpreting metrics, never candidate mutations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationContext {
    pub requested: EvaluationOptions,
    pub calculation_mode: String,
    pub enemy_level: u32,
    pub config_inputs: BTreeMap<String, Scalar>,
    pub config_placeholders: BTreeMap<String, Scalar>,
    pub player_conditions: BTreeMap<String, bool>,
    pub enemy_conditions: BTreeMap<String, bool>,
}

impl EvaluationOptions {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(selection) = &self.selection
            && (selection.socket_group == 0
                || selection.active_skill == Some(0)
                || selection.minion_skill == Some(0))
        {
            return Err("Skill selection indexes must be positive and one-based".into());
        }
        if let Some(encounter) = &self.encounter {
            if encounter.name.trim().is_empty() || encounter.name.len() > 128 {
                return Err("Encounter name must contain 1..128 bytes of nonblank text".into());
            }
            if encounter.enemy_level == Some(0) {
                return Err("Enemy level must be positive".into());
            }
            if let Some(hit) = &encounter.incoming_hit {
                let values = [hit.physical, hit.fire, hit.cold, hit.lightning, hit.chaos];
                if values
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.0)
                {
                    return Err("Incoming damage components must be finite and nonnegative".into());
                }
                if values.iter().all(|value| *value == 0.0) {
                    return Err("Incoming hit must have at least one positive component".into());
                }
            }
        }
        Ok(())
    }
}
