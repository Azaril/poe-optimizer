//! Authenticated offline projection for regenerated item display observations.
//! Source field names and their interpretation stop at this acquisition boundary.
//! The exported finite facts never contain callbacks, metadata tables, or UI state.
use crate::{
    game_data,
    owned_item_bases::{
        AuthenticatedItemBaseExport, BoundedBytes, ItemBaseExportError, ItemBaseExportLimits,
        extract_owned_item_bases,
    },
};
use poe_optimizer_data::item_loading::ItemMetadataValue;
use poe_optimizer_import::{
    owned_item_observations::{
        ItemObservationArmourField, ItemObservationBaseRow, ItemObservationCatalog,
        ItemObservationLimits, ItemObservationNumericField, ItemObservationRetarget,
        ItemObservationWeaponBranch, OWNED_ITEM_OBSERVATION_VERSION,
        validate_item_observation_catalog,
    },
    owned_mapping::SourceFilePin,
};
use serde::Serialize;
use std::{io::Write, path::Path};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Default)]
pub struct ItemObservationExportLimits {
    pub bases: ItemBaseExportLimits,
    pub catalog: ItemObservationLimits,
}
#[derive(Debug, Error)]
pub enum ItemObservationExportError {
    #[error("owned item-observation export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Base(#[from] ItemBaseExportError),
    #[error(transparent)]
    Catalog(#[from] poe_optimizer_import::owned_item_observations::ItemObservationError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, ItemObservationExportError>;
fn invalid(message: impl Into<String>) -> ItemObservationExportError {
    ItemObservationExportError::Invalid(message.into())
}

/// Evidence is inspectable; copied provenance cannot authenticate caller data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemObservationExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    pub extraction_source_files: Vec<SourceFilePin>,
    pub source_catalog_sha256: String,
    pub base_catalog_sha256: String,
    pub catalog_sha256: String,
    pub bases: usize,
    pub defence_observation_bases: usize,
    pub spirit_observation_bases: usize,
    pub charm_slot_observation_bases: usize,
    pub defence_retarget_bases: usize,
}
#[derive(Debug)]
pub struct AuthenticatedItemObservationExport {
    base_export: AuthenticatedItemBaseExport,
    catalog: ItemObservationCatalog,
    catalog_bytes: Vec<u8>,
    evidence: ItemObservationExportEvidence,
}
impl AuthenticatedItemObservationExport {
    pub fn base_export(&self) -> &AuthenticatedItemBaseExport {
        &self.base_export
    }
    pub fn catalog(&self) -> &ItemObservationCatalog {
        &self.catalog
    }
    /// Exact identity-bearing bytes: pretty JSON followed by one LF.
    pub fn catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }
    pub fn evidence(&self) -> &ItemObservationExportEvidence {
        &self.evidence
    }
    pub fn validate_catalog(&self, candidate: &ItemObservationCatalog) -> Result<()> {
        validate_item_observation_catalog(candidate, Default::default())?;
        if catalog_bytes(candidate, self.catalog_bytes.len())? != self.catalog_bytes {
            return Err(invalid(
                "catalog differs from independently authenticated extraction",
            ));
        }
        Ok(())
    }
}

/// Independently verify and construct the pinned source catalog once, then emit
/// finite facts. The original v1 base catalog is retained byte-for-byte.
pub fn export_owned_item_observations(
    root: &Path,
    limits: ItemObservationExportLimits,
) -> Result<AuthenticatedItemObservationExport> {
    // Validate caller limits before doing source work.
    let hard = ItemObservationLimits::default();
    for (name, value, maximum) in [
        (
            "catalog bytes",
            limits.catalog.max_catalog_bytes,
            hard.max_catalog_bytes,
        ),
        ("bases", limits.catalog.max_bases, hard.max_bases),
        (
            "source files",
            limits.catalog.max_source_files,
            hard.max_source_files,
        ),
        (
            "text bytes",
            limits.catalog.max_text_bytes,
            hard.max_text_bytes,
        ),
        ("work", limits.catalog.max_work, hard.max_work),
    ] {
        if value == 0 || value > maximum {
            return Err(invalid(format!("invalid {name} limit")));
        }
    }
    let (base_export, constructed) = extract_owned_item_bases(root, limits.bases)?;
    if base_export.catalog().bases.len() > limits.catalog.max_bases {
        return Err(invalid("base count limit"));
    }
    let mut bases = Vec::with_capacity(base_export.catalog().bases.len());
    for base in &base_export.catalog().bases {
        let original = constructed
            .base(&base.name)
            .ok_or_else(|| invalid("constructed base is missing"))?;
        bases.push(ItemObservationBaseRow {
            source_base: base.name.clone(),
            weapon_branch: weapon_branch(original.field("weapon")),
            armour: armour_field(original.field("armour")),
            spirit: numeric_field(original.field("spirit")),
            charm_slots: numeric_field(original.field("charmLimit")),
            defence_base_retarget: defence_retarget(&base.name),
        });
    }
    let catalog = ItemObservationCatalog {
        schema_version: OWNED_ITEM_OBSERVATION_VERSION,
        source: base_export.catalog().source.clone(),
        bases,
    };
    validate_item_observation_catalog(&catalog, limits.catalog)?;
    let catalog_bytes = catalog_bytes(&catalog, limits.catalog.max_catalog_bytes)?;
    let prior = base_export.evidence();
    let evidence = ItemObservationExportEvidence {
        schema_version: 1,
        upstream_revision: prior.upstream_revision.clone(),
        source_manifest_sha256: prior.source_manifest_sha256.clone(),
        extractor_sha256: game_data::hash(
            format!(
                "owned-item-observation-export-v1\n{}\n{}",
                prior.extractor_sha256,
                include_str!("owned_item_observations.rs").replace("\r\n", "\n")
            )
            .as_bytes(),
        ),
        extraction_source_files: prior.extraction_source_files.clone(),
        source_catalog_sha256: prior.source_catalog_sha256.clone(),
        base_catalog_sha256: prior.catalog_sha256.clone(),
        catalog_sha256: game_data::hash(&catalog_bytes),
        bases: catalog.bases.len(),
        defence_observation_bases: catalog
            .bases
            .iter()
            .filter(|b| b.can_observe_defences())
            .count(),
        spirit_observation_bases: catalog
            .bases
            .iter()
            .filter(|b| b.can_observe_spirit())
            .count(),
        charm_slot_observation_bases: catalog
            .bases
            .iter()
            .filter(|b| b.can_observe_charm_slots())
            .count(),
        defence_retarget_bases: catalog
            .bases
            .iter()
            .filter(|b| b.defence_base_retarget == ItemObservationRetarget::Possible)
            .count(),
    };
    Ok(AuthenticatedItemObservationExport {
        base_export,
        catalog,
        catalog_bytes,
        evidence,
    })
}
fn catalog_bytes(catalog: &ItemObservationCatalog, limit: usize) -> Result<Vec<u8>> {
    let mut output = BoundedBytes {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer_pretty(&mut output, catalog)?;
    output
        .write_all(b"\n")
        .map_err(|_| invalid("catalog byte limit"))?;
    Ok(output.bytes)
}
fn weapon_branch(value: Option<&ItemMetadataValue>) -> ItemObservationWeaponBranch {
    match value {
        None => ItemObservationWeaponBranch::Absent,
        Some(ItemMetadataValue::Boolean(false)) => ItemObservationWeaponBranch::False,
        Some(_) => ItemObservationWeaponBranch::Truthy,
    }
}
fn numeric_field(value: Option<&ItemMetadataValue>) -> ItemObservationNumericField {
    match value {
        None => ItemObservationNumericField::Absent,
        Some(ItemMetadataValue::Number(value)) if value.is_finite() => {
            ItemObservationNumericField::Finite { value: *value }
        }
        _ => ItemObservationNumericField::Unsupported,
    }
}
fn armour_field(value: Option<&ItemMetadataValue>) -> ItemObservationArmourField {
    let table = match value {
        None => return ItemObservationArmourField::Absent,
        Some(ItemMetadataValue::Table(table))
            if table.indexed.is_empty()
                && table.fields.keys().all(|key| {
                    matches!(
                        key.as_str(),
                        "Armour"
                            | "Evasion"
                            | "EnergyShield"
                            | "Ward"
                            | "BlockChance"
                            | "MovementPenalty"
                    )
                }) =>
        {
            Some(table)
        }
        // Empty arrays are the extractor's other encoding of an empty Lua table.
        Some(ItemMetadataValue::Array(values)) if values.is_empty() => None,
        _ => return ItemObservationArmourField::Unsupported,
    };
    let field = |name: &str| numeric_field(table.and_then(|t| t.fields.get(name)));
    ItemObservationArmourField::Table {
        armour: field("Armour"),
        evasion: field("Evasion"),
        energy_shield: field("EnergyShield"),
        ward: field("Ward"),
        block_chance: field("BlockChance"),
        movement_penalty: field("MovementPenalty"),
    }
}
fn defence_retarget(name: &str) -> ItemObservationRetarget {
    // These exact comparisons are part of pinned Item.lua's ordered display
    // dispatch, not general gameplay data or live base-name inference. Any new
    // source revision needs a reviewed extractor update and parity validation.
    match name {
        "Two-Toned Boots (Armour/Energy Shield)" | "Two-Toned Boots (Armour/Evasion)" => {
            ItemObservationRetarget::Possible
        }
        _ => ItemObservationRetarget::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_data::item_loading::ItemMetadataTable;
    #[test]
    fn source_shape_and_truthiness_are_distinct_and_fail_closed() {
        assert_eq!(weapon_branch(None), ItemObservationWeaponBranch::Absent);
        assert_eq!(
            weapon_branch(Some(&ItemMetadataValue::Boolean(false))),
            ItemObservationWeaponBranch::False
        );
        for value in [
            ItemMetadataValue::Number(0.0),
            ItemMetadataValue::Text(String::new()),
            ItemMetadataValue::Array(vec![]),
        ] {
            assert_eq!(
                weapon_branch(Some(&value)),
                ItemObservationWeaponBranch::Truthy
            );
        }
        assert_eq!(
            numeric_field(Some(&ItemMetadataValue::Number(0.0))),
            ItemObservationNumericField::Finite { value: 0.0 }
        );
        for value in [
            ItemMetadataValue::Boolean(false),
            ItemMetadataValue::Text("10".into()),
            ItemMetadataValue::Number(f64::NAN),
            ItemMetadataValue::Number(f64::INFINITY),
        ] {
            assert_eq!(
                numeric_field(Some(&value)),
                ItemObservationNumericField::Unsupported
            );
        }
        assert_eq!(armour_field(None), ItemObservationArmourField::Absent);
        for value in [
            ItemMetadataValue::Boolean(false),
            ItemMetadataValue::Number(5.0),
            ItemMetadataValue::Array(vec![ItemMetadataValue::Number(1.0)]),
        ] {
            assert_eq!(
                armour_field(Some(&value)),
                ItemObservationArmourField::Unsupported
            );
        }
        assert!(matches!(
            armour_field(Some(&ItemMetadataValue::Array(vec![]))),
            ItemObservationArmourField::Table { .. }
        ));
        let mut table = ItemMetadataTable::default();
        table
            .fields
            .insert("Armour".into(), ItemMetadataValue::Number(100.0));
        table
            .fields
            .insert("FutureChannel".into(), ItemMetadataValue::Number(1.0));
        assert_eq!(
            armour_field(Some(&ItemMetadataValue::Table(table.clone()))),
            ItemObservationArmourField::Unsupported
        );
        table.fields.remove("FutureChannel");
        table.indexed.insert(1, ItemMetadataValue::Number(5.0));
        assert_eq!(
            armour_field(Some(&ItemMetadataValue::Table(table))),
            ItemObservationArmourField::Unsupported
        );
        assert_eq!(
            defence_retarget("Two-Toned Boots (Armour/Energy Shield)"),
            ItemObservationRetarget::Possible
        );
        assert_eq!(
            defence_retarget("Two-Toned Boots (Armour/Evasion)"),
            ItemObservationRetarget::Possible
        );
        assert_eq!(
            defence_retarget("Two-Toned Boots (Evasion/Energy Shield)"),
            ItemObservationRetarget::None
        );
    }
}
