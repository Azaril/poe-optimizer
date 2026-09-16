//! Portable raw actor facts acquired offline, independent of creating actions.
//!
//! Curves are alternatives: profile provenance does not select one. Raw zero
//! attack time remains zero, and missing facts remain missing. A native compiler
//! must bind an exact creating occurrence and retain the unconverted coverage.
use crate::owned_mapping::SourcePin;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use std::{collections::BTreeMap, fmt};

pub const OWNED_ACTOR_BASELINE_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBaselineCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub profiles: Vec<ActorBaselineProfile>,
    pub summon_levels: ActorSummonLevelTable,
    pub allied_damage: ActorDamageLevelTable,
    pub hostile_damage: ActorDamageLevelTable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBaselineProfile {
    /// Offline source identity for an explicit mapping to an owned actor slot.
    pub key: String,
    pub name: String,
    pub source_module: String,
    #[serde(deserialize_with = "optional_finite")]
    pub attack_time: Option<f64>,
    #[serde(deserialize_with = "optional_finite")]
    pub damage_scale: Option<f64>,
    #[serde(deserialize_with = "optional_finite")]
    pub damage_spread: Option<f64>,
    #[serde(deserialize_with = "optional_finite")]
    pub critical_chance: Option<f64>,
    #[serde(deserialize_with = "optional_finite")]
    pub attack_range: Option<f64>,
    pub weapon_family: Option<String>,
    pub hostile: Option<bool>,
    pub base_damage_ignores_attack_speed: Option<bool>,
    /// Authored ordering is meaningful for action selection and is preserved.
    pub child_skills: Vec<String>,
    /// Lossless additional scalar facts (including health and defences). Their
    /// interpretation is still unconverted until an explicit policy consumes it.
    #[serde(deserialize_with = "unique_facts")]
    pub extra_facts: BTreeMap<String, ActorScalarFact>,
    /// Every other present top-level field, in canonical order. These are gaps,
    /// not an assertion that unrelated actor mechanics have been implemented.
    pub unconverted_fields: Vec<String>,
    /// None means absent; Some(empty) means an explicitly empty source list.
    /// Descriptors identify unconverted effects, never executable source logic.
    pub unconverted_modifiers: Option<Vec<ActorModifierCoverage>>,
    /// Finite source flags retained without interpreting their actor semantics.
    #[serde(deserialize_with = "optional_unique_flags")]
    pub unconverted_flags: Option<BTreeMap<String, bool>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorModifierConstructor {
    Modifier,
    Flag,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ActorScalarFact {
    Number(#[serde(deserialize_with = "finite")] f64),
    Boolean(bool),
    Text(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorModifierCoverage {
    pub constructor: ActorModifierConstructor,
    pub name: String,
    pub operation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSummonLevelTable {
    pub first_level: u32,
    pub rows: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorDamageLevelTable {
    pub first_level: u32,
    #[serde(deserialize_with = "finite_rows")]
    pub rows: Vec<f64>,
}

fn optional_finite<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    let value = Option::<f64>::deserialize(d)?;
    if value.is_some_and(|n| !n.is_finite()) {
        return Err(serde::de::Error::custom("actor fact must be finite"));
    }
    Ok(value)
}

fn finite_rows<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<f64>, D::Error> {
    let values = Vec::<f64>::deserialize(d)?;
    if values.iter().any(|n| !n.is_finite()) {
        return Err(serde::de::Error::custom("actor curve must be finite"));
    }
    Ok(values)
}

fn finite<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let value = f64::deserialize(d)?;
    if !value.is_finite() {
        return Err(serde::de::Error::custom("actor fact must be finite"));
    }
    Ok(value)
}

fn unique_facts<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, ActorScalarFact>, D::Error> {
    struct Facts;
    impl<'de> Visitor<'de> for Facts {
        type Value = BTreeMap<String, ActorScalarFact>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unique finite actor scalar facts")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut facts = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, ActorScalarFact>()? {
                if facts.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate actor scalar fact"));
                }
            }
            Ok(facts)
        }
    }
    d.deserialize_map(Facts)
}
fn optional_unique_flags<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<BTreeMap<String, bool>>, D::Error> {
    #[derive(Deserialize)]
    struct Flags(#[serde(deserialize_with = "unique_flags")] BTreeMap<String, bool>);
    Ok(Option::<Flags>::deserialize(d)?.map(|f| f.0))
}
fn unique_flags<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, bool>, D::Error> {
    struct Flags;
    impl<'de> Visitor<'de> for Flags {
        type Value = BTreeMap<String, bool>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unique actor flags")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut flags = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, bool>()? {
                if flags.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate actor flag"));
                }
            }
            Ok(flags)
        }
    }
    d.deserialize_map(Flags)
}
