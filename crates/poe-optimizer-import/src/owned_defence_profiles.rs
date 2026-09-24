//! Finite raw armour/defence profiles with explicit whole-profile absence.
//! Source tables are raw EquipmentUse facts, not final defences or activation.
pub use crate::owned_item_profiles::{
    ItemFieldAbsence as DefenceFieldAbsence, ItemProfileField as DefenceProfileField,
    ItemProfileLimits as DefenceProfileLimits, ItemProfilePolicy as DefenceProfilePolicy,
    ItemProfileReceipt as DefenceProfileReceipt,
    StagedItemProfileRecipe as StagedDefenceProfileRecipe,
};
use crate::{
    owned_item_profiles::{self, ItemProfileCatalog, ItemProfileRow, ProfileFamily, ProfileFormat},
    owned_mapping::{OwnedMappingError, OwnedMappingIndex, SourcePin},
    owned_recipe::{OwnedRecipeError, StagedOwnedRecipe},
};
use poe_optimizer_core::owned_content::ContentDigestError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
pub const OWNED_DEFENCE_PROFILE_VERSION: u32 = 1;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefenceProfileCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub base_catalog_sha256: String,
    pub profiles: Vec<DefenceProfileRow>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefenceProfileRow {
    pub base: String,
    pub profile: DefenceProfilePresence,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DefenceProfilePresence {
    Absent,
    Table {
        #[serde(deserialize_with = "unique_fields")]
        fields: BTreeMap<String, f64>,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum DefenceProfileError {
    #[error("defence profile exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid defence profile catalog or policy: {0}")]
    Invalid(&'static str),
    #[error("defence profile artifact, source or predecessor binding differs")]
    Binding,
    #[error("defence profile baseline preservation: {0}")]
    Preservation(String),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
impl From<owned_item_profiles::ItemProfileError> for DefenceProfileError {
    fn from(error: owned_item_profiles::ItemProfileError) -> Self {
        use owned_item_profiles::ItemProfileError as E;
        match error {
            E::Limit(value) => Self::Limit(value),
            E::Invalid(value) => Self::Invalid(value),
            E::Binding => Self::Binding,
            E::Preservation(value) => Self::Preservation(value),
            E::Mapping(value) => Self::Mapping(value),
            E::Recipe(value) => Self::Recipe(value),
            E::Digest(value) => Self::Digest(value),
            E::Json(value) => Self::Json(value),
        }
    }
}
fn unique_fields<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<String, f64>, D::Error> {
    owned_item_profiles::unique_fields(d, "defence")
}
/// Compile every base's explicit raw profile; absent profiles publish no facts.
/// Table-field fallbacks remain caller-authored, including for empty tables.
pub fn compile_owned_defence_profiles(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    base_catalog_bytes: &[u8],
    catalog_bytes: &[u8],
    policy: &DefenceProfilePolicy,
    limits: DefenceProfileLimits,
) -> std::result::Result<StagedDefenceProfileRecipe, DefenceProfileError> {
    owned_item_profiles::compile_item_profiles(
        base,
        mapping,
        base_catalog_bytes,
        catalog_bytes,
        policy,
        limits,
        ProfileFormat {
            family: ProfileFamily::Defence,
            decode: |bytes: &[u8]| {
                let catalog: DefenceProfileCatalog = serde_json::from_slice(bytes)?;
                Ok(ItemProfileCatalog {
                    schema_version: catalog.schema_version,
                    source: catalog.source,
                    base_catalog_sha256: catalog.base_catalog_sha256,
                    profiles: catalog
                        .profiles
                        .into_iter()
                        .map(|row| ItemProfileRow {
                            base: row.base,
                            fields: match row.profile {
                                DefenceProfilePresence::Absent => None,
                                DefenceProfilePresence::Table { fields } => Some(fields),
                            },
                        })
                        .collect(),
                })
            },
        },
    )
    .map_err(Into::into)
}
