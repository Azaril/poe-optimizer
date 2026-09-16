//! Authenticated offline raw numeric weapon profiles from final constructed bases.
//!
//! Authored presence is preserved: absence is not zero, and a table is not proof
//! of action compatibility or final weapon stats. No source tables or callbacks
//! cross this finite owned-data boundary.
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
};

use poe_optimizer_data::item_loading::ItemMetadataValue;
use poe_optimizer_import::{
    owned_item_bases::ItemBaseWeaponField,
    owned_mapping::SourceFilePin,
    owned_weapon_profiles::{OWNED_WEAPON_PROFILE_VERSION, WeaponProfileCatalog, WeaponProfileRow},
};
use serde::Serialize;
use thiserror::Error;

use crate::{
    game_data,
    owned_item_bases::{
        AuthenticatedItemBaseExport, BoundedBytes, ItemBaseExportError, ItemBaseExportLimits,
        extract_owned_item_bases,
    },
};

#[derive(Clone, Copy, Debug)]
pub struct WeaponProfileExportLimits {
    pub bases: ItemBaseExportLimits,
    pub max_profiles: usize,
    pub max_fields_per_profile: usize,
    pub max_total_fields: usize,
    pub max_catalog_bytes: usize,
}
impl Default for WeaponProfileExportLimits {
    fn default() -> Self {
        Self {
            bases: Default::default(),
            max_profiles: 4096,
            max_fields_per_profile: 128,
            max_total_fields: 16_384,
            max_catalog_bytes: 2 * 1024 * 1024,
        }
    }
}
impl WeaponProfileExportLimits {
    fn validate(self) -> Result<()> {
        let maximum = Self::default();
        for (name, value, limit) in [
            ("profiles", self.max_profiles, maximum.max_profiles),
            (
                "fields per profile",
                self.max_fields_per_profile,
                maximum.max_fields_per_profile,
            ),
            (
                "total fields",
                self.max_total_fields,
                maximum.max_total_fields,
            ),
            (
                "catalog bytes",
                self.max_catalog_bytes,
                maximum.max_catalog_bytes,
            ),
        ] {
            if value == 0 || value > limit {
                return Err(invalid(format!("invalid {name} limit")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum WeaponProfileExportError {
    #[error("owned weapon-profile export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Base(#[from] ItemBaseExportError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, WeaponProfileExportError>;
fn invalid(message: impl Into<String>) -> WeaponProfileExportError {
    WeaponProfileExportError::Invalid(message.into())
}

/// Audit evidence is not an authentication token. Only fresh verified extraction
/// can construct the private authenticated result below.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponProfileExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    pub extraction_source_files: Vec<SourceFilePin>,
    pub source_catalog_sha256: String,
    pub base_catalog_sha256: String,
    pub catalog_sha256: String,
    pub profiles: usize,
    pub fields: usize,
    pub field_names: Vec<String>,
}

#[derive(Debug)]
pub struct AuthenticatedWeaponProfileExport {
    base_export: AuthenticatedItemBaseExport,
    catalog: WeaponProfileCatalog,
    catalog_bytes: Vec<u8>,
    evidence: WeaponProfileExportEvidence,
}
impl AuthenticatedWeaponProfileExport {
    pub fn base_export(&self) -> &AuthenticatedItemBaseExport {
        &self.base_export
    }
    pub fn catalog(&self) -> &WeaponProfileCatalog {
        &self.catalog
    }
    /// The exact identity-bearing bytes: pretty JSON and one trailing LF.
    pub fn catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }
    pub fn evidence(&self) -> &WeaponProfileExportEvidence {
        &self.evidence
    }
    pub fn validate_catalog(&self, candidate: &WeaponProfileCatalog) -> Result<()> {
        // Comparing bytes also distinguishes authored -0.0 from +0.0; floating
        // PartialEq alone would accept that content change. Bound serialization
        // even though authentication authority was obtained independently.
        let candidate_bytes = catalog_bytes(candidate, self.catalog_bytes.len())?;
        if candidate_bytes != self.catalog_bytes {
            return Err(invalid(
                "catalog differs from independently authenticated extraction",
            ));
        }
        Ok(())
    }
}

/// Project all final table-shaped weapon profiles from a single independently
/// authenticated construction, also retaining its byte-identical base catalog.
pub fn export_owned_weapon_profiles(
    root: &Path,
    limits: WeaponProfileExportLimits,
) -> Result<AuthenticatedWeaponProfileExport> {
    limits.validate()?;
    let (base_export, constructed) = extract_owned_item_bases(root, limits.bases)?;
    let mut profiles = Vec::new();
    let mut total_fields = 0usize;
    let mut field_names = BTreeSet::new();
    for base in &base_export.catalog().bases {
        if base.weapon_field != ItemBaseWeaponField::Table {
            continue;
        }
        if profiles.len() >= limits.max_profiles {
            return Err(invalid("profile count limit"));
        }
        let original = constructed
            .base(&base.name)
            .ok_or_else(|| invalid("constructed base is missing"))?;
        let fields = numeric_fields(original.field("weapon"), limits)?;
        total_fields = total_fields
            .checked_add(fields.len())
            .ok_or_else(|| invalid("total field count overflow"))?;
        if total_fields > limits.max_total_fields {
            return Err(invalid("total field count limit"));
        }
        field_names.extend(fields.keys().cloned());
        profiles.push(WeaponProfileRow {
            base: base.name.clone(),
            fields,
        });
    }
    let catalog = WeaponProfileCatalog {
        schema_version: OWNED_WEAPON_PROFILE_VERSION,
        source: base_export.catalog().source.clone(),
        base_catalog_sha256: base_export.evidence().catalog_sha256.clone(),
        profiles,
    };
    let catalog_bytes = catalog_bytes(&catalog, limits.max_catalog_bytes)?;
    let base_evidence = base_export.evidence();
    let evidence = WeaponProfileExportEvidence {
        schema_version: 1,
        upstream_revision: base_evidence.upstream_revision.clone(),
        source_manifest_sha256: base_evidence.source_manifest_sha256.clone(),
        extractor_sha256: game_data::hash(
            format!(
                "owned-weapon-profile-export-v1\n{}\n{}",
                // This includes the shared constructor, base projection and all
                // upstream extractor code. The new projection has its own pin too.
                base_evidence.extractor_sha256,
                include_str!("owned_weapon_profiles.rs").replace("\r\n", "\n"),
            )
            .as_bytes(),
        ),
        extraction_source_files: base_evidence.extraction_source_files.clone(),
        source_catalog_sha256: base_evidence.source_catalog_sha256.clone(),
        base_catalog_sha256: catalog.base_catalog_sha256.clone(),
        catalog_sha256: game_data::hash(&catalog_bytes),
        profiles: catalog.profiles.len(),
        fields: total_fields,
        field_names: field_names.into_iter().collect(),
    };
    Ok(AuthenticatedWeaponProfileExport {
        base_export,
        catalog,
        catalog_bytes,
        evidence,
    })
}

fn catalog_bytes(catalog: &WeaponProfileCatalog, limit: usize) -> Result<Vec<u8>> {
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

fn numeric_fields(
    value: Option<&ItemMetadataValue>,
    limits: WeaponProfileExportLimits,
) -> Result<BTreeMap<String, f64>> {
    let table = match value {
        Some(ItemMetadataValue::Array(values)) if values.is_empty() => return Ok(BTreeMap::new()),
        Some(ItemMetadataValue::Table(table)) if table.indexed.is_empty() => table,
        _ => {
            return Err(invalid(
                "weapon table contains unsupported indexed or scalar data",
            ));
        }
    };
    if table.fields.len() > limits.max_fields_per_profile {
        return Err(invalid("fields per profile limit"));
    }
    let mut fields = BTreeMap::new();
    for (key, value) in &table.fields {
        if key.is_empty()
            || key.len() > limits.bases.max_text_bytes
            || key.chars().any(char::is_control)
        {
            return Err(invalid("invalid or oversized weapon field name"));
        }
        let ItemMetadataValue::Number(value) = value else {
            return Err(invalid("weapon field is not numeric"));
        };
        if !value.is_finite() {
            return Err(invalid("weapon field is not finite"));
        }
        fields.insert(key.clone(), *value);
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_data::item_loading::ItemMetadataTable;

    #[test]
    fn raw_zero_unknown_numeric_fields_and_absence_remain_distinct() {
        let table = ItemMetadataValue::Table(ItemMetadataTable {
            fields: BTreeMap::from([
                ("PresentZero".into(), ItemMetadataValue::Number(0.0)),
                ("SignedZero".into(), ItemMetadataValue::Number(-0.0)),
                ("FutureNumericField".into(), ItemMetadataValue::Number(1.25)),
            ]),
            ..Default::default()
        });
        let fields = numeric_fields(Some(&table), Default::default()).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields["PresentZero"].to_bits(), 0.0f64.to_bits());
        assert_eq!(fields["SignedZero"].to_bits(), (-0.0f64).to_bits());
        assert_eq!(fields["FutureNumericField"], 1.25);
        assert!(!fields.contains_key("Missing"));
        assert!(numeric_fields(None, Default::default()).is_err());
        assert!(
            numeric_fields(Some(&ItemMetadataValue::Array(vec![])), Default::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn malformed_or_nonfinite_fields_are_rejected_without_partial_projection() {
        for value in [
            ItemMetadataValue::Boolean(false),
            ItemMetadataValue::Text("0".into()),
            ItemMetadataValue::Array(vec![]),
            ItemMetadataValue::Number(f64::NAN),
            ItemMetadataValue::Number(f64::INFINITY),
            ItemMetadataValue::Number(f64::NEG_INFINITY),
        ] {
            let table = ItemMetadataValue::Table(ItemMetadataTable {
                fields: BTreeMap::from([("Value".into(), value)]),
                ..Default::default()
            });
            assert!(numeric_fields(Some(&table), Default::default()).is_err());
        }
        for value in [
            ItemMetadataValue::Array(vec![ItemMetadataValue::Number(1.0)]),
            ItemMetadataValue::Table(ItemMetadataTable {
                indexed: BTreeMap::from([(1, ItemMetadataValue::Number(1.0))]),
                ..Default::default()
            }),
            ItemMetadataValue::Number(1.0),
        ] {
            assert!(numeric_fields(Some(&value), Default::default()).is_err());
        }
    }
}
