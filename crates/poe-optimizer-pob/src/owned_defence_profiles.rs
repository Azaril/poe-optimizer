//! Authenticated offline raw numeric defence profiles from final constructed bases.
//!
//! Authored presence is preserved: absence is not zero, and a table is not proof
//! of equipped activation or assembled item defences. No source tables or callbacks
//! cross this finite owned-data boundary.
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
};

use poe_optimizer_data::item_loading::ItemMetadataValue;
use poe_optimizer_import::{
    owned_defence_profiles::{
        DefenceProfileCatalog, DefenceProfilePresence, DefenceProfileRow,
        OWNED_DEFENCE_PROFILE_VERSION,
    },
    owned_mapping::SourceFilePin,
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
pub struct DefenceProfileExportLimits {
    pub bases: ItemBaseExportLimits,
    pub max_profiles: usize,
    pub max_fields_per_profile: usize,
    pub max_total_fields: usize,
    pub max_catalog_bytes: usize,
}
impl Default for DefenceProfileExportLimits {
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
impl DefenceProfileExportLimits {
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
pub enum DefenceProfileExportError {
    #[error("owned defence-profile export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Base(#[from] ItemBaseExportError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, DefenceProfileExportError>;
fn invalid(message: impl Into<String>) -> DefenceProfileExportError {
    DefenceProfileExportError::Invalid(message.into())
}

/// Audit evidence is not an authentication token. Only fresh verified extraction
/// can construct the private authenticated result below.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DefenceProfileExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    pub extraction_source_files: Vec<SourceFilePin>,
    pub source_catalog_sha256: String,
    pub base_catalog_sha256: String,
    pub catalog_sha256: String,
    pub profiles: usize,
    pub table_profiles: usize,
    pub absent_profiles: usize,
    pub empty_profiles: usize,
    pub fields: usize,
    pub field_names: Vec<String>,
}

#[derive(Debug)]
pub struct AuthenticatedDefenceProfileExport {
    base_export: AuthenticatedItemBaseExport,
    catalog: DefenceProfileCatalog,
    catalog_bytes: Vec<u8>,
    evidence: DefenceProfileExportEvidence,
}
impl AuthenticatedDefenceProfileExport {
    pub fn base_export(&self) -> &AuthenticatedItemBaseExport {
        &self.base_export
    }
    pub fn catalog(&self) -> &DefenceProfileCatalog {
        &self.catalog
    }
    /// The exact identity-bearing bytes: pretty JSON and one trailing LF.
    pub fn catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }
    pub fn evidence(&self) -> &DefenceProfileExportEvidence {
        &self.evidence
    }
    pub fn validate_catalog(&self, candidate: &DefenceProfileCatalog) -> Result<()> {
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

/// Project every final base's raw defence-profile presence from one independently
/// authenticated construction, retaining the byte-identical base catalog. Empty
/// tables stay present; no source-shaped table or missing-field default escapes.
pub fn export_owned_defence_profiles(
    root: &Path,
    limits: DefenceProfileExportLimits,
) -> Result<AuthenticatedDefenceProfileExport> {
    limits.validate()?;
    let (base_export, constructed) = extract_owned_item_bases(root, limits.bases)?;
    let mut profiles = Vec::new();
    let mut total_fields = 0usize;
    let mut field_names = BTreeSet::new();
    let mut table_profiles = 0usize;
    let mut absent_profiles = 0usize;
    let mut empty_profiles = 0usize;
    for base in &base_export.catalog().bases {
        if profiles.len() >= limits.max_profiles {
            return Err(invalid("profile count limit"));
        }
        let original = constructed
            .base(&base.name)
            .ok_or_else(|| invalid("constructed base is missing"))?;
        let profile = match original.field("armour") {
            None => {
                absent_profiles += 1;
                DefenceProfilePresence::Absent
            }
            value => {
                let fields = numeric_fields(value, limits)?;
                total_fields = total_fields
                    .checked_add(fields.len())
                    .ok_or_else(|| invalid("total field count overflow"))?;
                if total_fields > limits.max_total_fields {
                    return Err(invalid("total field count limit"));
                }
                table_profiles += 1;
                empty_profiles += usize::from(fields.is_empty());
                field_names.extend(fields.keys().cloned());
                DefenceProfilePresence::Table { fields }
            }
        };
        profiles.push(DefenceProfileRow {
            base: base.name.clone(),
            profile,
        });
    }
    let catalog = DefenceProfileCatalog {
        schema_version: OWNED_DEFENCE_PROFILE_VERSION,
        source: base_export.catalog().source.clone(),
        base_catalog_sha256: base_export.evidence().catalog_sha256.clone(),
        profiles,
    };
    let catalog_bytes = catalog_bytes(&catalog, limits.max_catalog_bytes)?;
    let base_evidence = base_export.evidence();
    let evidence = DefenceProfileExportEvidence {
        schema_version: 1,
        upstream_revision: base_evidence.upstream_revision.clone(),
        source_manifest_sha256: base_evidence.source_manifest_sha256.clone(),
        extractor_sha256: game_data::hash(
            format!(
                "owned-defence-profile-export-v1\n{}\n{}",
                // This includes the shared constructor, base projection and all
                // upstream extractor code. The new projection has its own pin too.
                base_evidence.extractor_sha256,
                include_str!("owned_defence_profiles.rs").replace("\r\n", "\n"),
            )
            .as_bytes(),
        ),
        extraction_source_files: base_evidence.extraction_source_files.clone(),
        source_catalog_sha256: base_evidence.source_catalog_sha256.clone(),
        base_catalog_sha256: catalog.base_catalog_sha256.clone(),
        catalog_sha256: game_data::hash(&catalog_bytes),
        profiles: catalog.profiles.len(),
        table_profiles,
        absent_profiles,
        empty_profiles,
        fields: total_fields,
        field_names: field_names.into_iter().collect(),
    };
    Ok(AuthenticatedDefenceProfileExport {
        base_export,
        catalog,
        catalog_bytes,
        evidence,
    })
}

fn catalog_bytes(catalog: &DefenceProfileCatalog, limit: usize) -> Result<Vec<u8>> {
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
    limits: DefenceProfileExportLimits,
) -> Result<BTreeMap<String, f64>> {
    let table = match value {
        Some(ItemMetadataValue::Array(values)) if values.is_empty() => return Ok(BTreeMap::new()),
        Some(ItemMetadataValue::Table(table)) if table.indexed.is_empty() => table,
        _ => {
            return Err(invalid(
                "defence table contains unsupported indexed or scalar data",
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
            return Err(invalid("invalid or oversized defence field name"));
        }
        let ItemMetadataValue::Number(value) = value else {
            return Err(invalid("defence field is not numeric"));
        };
        if !value.is_finite() {
            return Err(invalid("defence field is not finite"));
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
