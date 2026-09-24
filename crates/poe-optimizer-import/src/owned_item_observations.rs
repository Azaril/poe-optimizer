//! Finite offline facts for interpreting regenerated item display headers.
//!
//! These facts belong to source acquisition/import only. They establish neither
//! item membership nor numerical coverage, and never execute source code. A
//! structurally valid catalog remains untrusted until independently authenticated.
use crate::owned_mapping::{ExternalSourceSystem, SourcePin};
use serde::{Deserialize, Serialize};

pub const OWNED_ITEM_OBSERVATION_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemObservationCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    /// Canonical ascending exact source names; duplicate names are forbidden.
    pub bases: Vec<ItemObservationBaseRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemObservationBaseRow {
    pub source_base: String,
    /// The weapon branch precedes the armour branch in the reviewed source.
    pub weapon_branch: ItemObservationWeaponBranch,
    pub armour: ItemObservationArmourField,
    pub spirit: ItemObservationNumericField,
    pub charm_slots: ItemObservationNumericField,
    pub defence_base_retarget: ItemObservationRetarget,
}
impl ItemObservationBaseRow {
    /// Offline source fact only: compatible fresh-item defence recomputation.
    /// The caller must still establish source identity, preamble placement,
    /// exact template binding, numeric syntax and preceding-line attribution.
    pub fn can_observe_defences(&self) -> bool {
        matches!(
            self.weapon_branch,
            ItemObservationWeaponBranch::Absent | ItemObservationWeaponBranch::False
        ) && self.defence_base_retarget == ItemObservationRetarget::None
            && match &self.armour {
                ItemObservationArmourField::Table {
                    armour,
                    evasion,
                    energy_shield,
                    ward,
                    block_chance,
                    movement_penalty,
                } => [
                    armour,
                    evasion,
                    energy_shield,
                    ward,
                    block_chance,
                    movement_penalty,
                ]
                .iter()
                .all(|field| field.is_absent_or_finite()),
                _ => false,
            }
    }
    /// Offline source fact only; this does not calculate Spirit or emit an input.
    pub fn can_observe_spirit(&self) -> bool {
        self.spirit.is_finite()
    }
    /// Offline source fact only; zero is a present numeric base value.
    pub fn can_observe_charm_slots(&self) -> bool {
        self.charm_slots.is_finite()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemObservationWeaponBranch {
    Absent,
    False,
    /// Any present non-false source value selects the weapon branch. This says
    /// nothing about whether that value has a supported weapon-data shape.
    Truthy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemObservationArmourField {
    Absent,
    /// Only reviewed named numeric channels, with no unreviewed/indexed members.
    Table {
        armour: ItemObservationNumericField,
        evasion: ItemObservationNumericField,
        energy_shield: ItemObservationNumericField,
        ward: ItemObservationNumericField,
        block_chance: ItemObservationNumericField,
        movement_penalty: ItemObservationNumericField,
    },
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemObservationNumericField {
    Absent,
    Finite { value: f64 },
    Unsupported,
}
impl ItemObservationNumericField {
    fn is_finite(self) -> bool {
        matches!(self, Self::Finite { value } if value.is_finite())
    }
    fn is_absent_or_finite(self) -> bool {
        self == Self::Absent || self.is_finite()
    }
    fn validate(self) -> Result<()> {
        if matches!(self, Self::Finite { value } if !value.is_finite()) {
            return Err(ItemObservationError::Invalid("non-finite numeric field"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemObservationRetarget {
    None,
    /// A reviewed defence-header branch can change this exact selected base.
    Possible,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemObservationLimits {
    pub max_catalog_bytes: usize,
    pub max_bases: usize,
    pub max_source_files: usize,
    pub max_text_bytes: usize,
    pub max_work: usize,
}
impl Default for ItemObservationLimits {
    fn default() -> Self {
        Self {
            max_catalog_bytes: 4 * 1024 * 1024,
            max_bases: 4096,
            max_source_files: 256,
            max_text_bytes: 1024,
            max_work: 16 * 1024 * 1024,
        }
    }
}
impl ItemObservationLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, actual, maximum) in [
            (
                "catalog bytes",
                self.max_catalog_bytes,
                hard.max_catalog_bytes,
            ),
            ("bases", self.max_bases, hard.max_bases),
            ("source files", self.max_source_files, hard.max_source_files),
            ("text bytes", self.max_text_bytes, hard.max_text_bytes),
            ("work", self.max_work, hard.max_work),
        ] {
            if actual == 0 || actual > maximum {
                return Err(ItemObservationError::Limit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ItemObservationError {
    #[error("item observation catalog exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid item observation catalog: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, ItemObservationError>;
fn charge(work: &mut usize, amount: usize) -> Result<()> {
    *work = work
        .checked_sub(amount)
        .ok_or(ItemObservationError::Limit("work"))?;
    Ok(())
}
fn valid_text(value: &str, limits: ItemObservationLimits, work: &mut usize) -> Result<()> {
    if value.len() > limits.max_text_bytes {
        return Err(ItemObservationError::Limit("text bytes"));
    }
    charge(work, value.len().saturating_mul(3).saturating_add(1))?;
    if value.is_empty() || value.trim_ascii() != value || value.chars().any(char::is_control) {
        return Err(ItemObservationError::Invalid("text"));
    }
    Ok(())
}

/// Validate finite structure and canonical ordering with bounded work. This is
/// not source authentication: no revision string or supplied hash grants trust.
pub fn validate_item_observation_catalog(
    catalog: &ItemObservationCatalog,
    limits: ItemObservationLimits,
) -> Result<()> {
    limits.validate()?;
    if catalog.schema_version != OWNED_ITEM_OBSERVATION_VERSION
        || catalog.source.system != ExternalSourceSystem::PathOfBuilding2
    {
        return Err(ItemObservationError::Invalid("version or source system"));
    }
    if catalog.bases.is_empty() || catalog.bases.len() > limits.max_bases {
        return Err(ItemObservationError::Limit("bases"));
    }
    if catalog.source.files.is_empty() || catalog.source.files.len() > limits.max_source_files {
        return Err(ItemObservationError::Limit("source files"));
    }
    let mut work = limits.max_work;
    valid_text(&catalog.source.revision, limits, &mut work)?;
    let mut prior: Option<&str> = None;
    for pin in &catalog.source.files {
        valid_text(&pin.path, limits, &mut work)?;
        charge(&mut work, 64)?;
        if pin.sha256.len() != 64
            || !pin
                .sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(ItemObservationError::Invalid("source hash"));
        }
        if prior.is_some_and(|p| p >= pin.path.as_str()) {
            return Err(ItemObservationError::Invalid(
                "source file ordering or duplicate",
            ));
        }
        prior = Some(&pin.path);
    }
    prior = None;
    for row in &catalog.bases {
        valid_text(&row.source_base, limits, &mut work)?;
        charge(&mut work, 10)?;
        if prior.is_some_and(|p| p >= row.source_base.as_str()) {
            return Err(ItemObservationError::Invalid("base ordering or duplicate"));
        }
        prior = Some(&row.source_base);
        row.spirit.validate()?;
        row.charm_slots.validate()?;
        if let ItemObservationArmourField::Table {
            armour,
            evasion,
            energy_shield,
            ward,
            block_chance,
            movement_penalty,
        } = row.armour
        {
            for field in [
                armour,
                evasion,
                energy_shield,
                ward,
                block_chance,
                movement_penalty,
            ] {
                field.validate()?;
            }
        }
    }
    Ok(())
}

/// Decode bounded owned data without loading any upstream runtime or metadata.
pub fn decode_item_observation_catalog(
    bytes: &[u8],
    limits: ItemObservationLimits,
) -> Result<ItemObservationCatalog> {
    limits.validate()?;
    if bytes.len() > limits.max_catalog_bytes {
        return Err(ItemObservationError::Limit("catalog bytes"));
    }
    let catalog = serde_json::from_slice(bytes)?;
    validate_item_observation_catalog(&catalog, limits)?;
    Ok(catalog)
}
