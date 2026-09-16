//! Authenticated offline projection of the final constructed item-base catalog.
//!
//! This adapter reuses the reviewed, bounded item-definition constructor. It
//! exports only finite base identity and weapon-field shape; no Lua metadata,
//! item callbacks, build/UI state, or evaluator execution enters the owned DTO.
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
    path::Path,
};

use poe_optimizer_data::item_loading::{ItemLoadingCatalog, ItemMetadataValue};
use poe_optimizer_import::{
    owned_item_bases::{
        ItemBaseCatalog, ItemBaseRow, ItemBaseWeaponField, OWNED_ITEM_BASE_VERSION,
    },
    owned_mapping::{ExternalSourceSystem, SourceFilePin, SourcePin},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{game_data, item_loading_extract, source};

const DATA_PATH: &str = "src/Modules/Data.lua";
const ITEM_PATH: &str = "src/Classes/Item.lua";
const BASE_PREFIX: &str = "src/Data/Bases/";

/// Projection limits. The reused constructor separately bounds Lua memory,
/// instructions, metadata depth, and metadata nodes before producing its result.
#[derive(Clone, Copy, Debug)]
pub struct ItemBaseExportLimits {
    pub max_source_files: usize,
    pub max_source_bytes: usize,
    pub max_bases: usize,
    pub max_text_bytes: usize,
    pub max_catalog_bytes: usize,
}
impl Default for ItemBaseExportLimits {
    fn default() -> Self {
        Self {
            max_source_files: 256,
            max_source_bytes: 64 * 1024 * 1024,
            max_bases: 4096,
            max_text_bytes: 1024,
            max_catalog_bytes: 2 * 1024 * 1024,
        }
    }
}
impl ItemBaseExportLimits {
    fn validate(self) -> Result<()> {
        let maximum = Self::default();
        for (name, actual, limit) in [
            (
                "source files",
                self.max_source_files,
                maximum.max_source_files,
            ),
            (
                "source bytes",
                self.max_source_bytes,
                maximum.max_source_bytes,
            ),
            ("bases", self.max_bases, maximum.max_bases),
            ("text bytes", self.max_text_bytes, maximum.max_text_bytes),
            (
                "catalog bytes",
                self.max_catalog_bytes,
                maximum.max_catalog_bytes,
            ),
        ] {
            if actual == 0 || actual > limit {
                return Err(invalid(format!("invalid {name} limit")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ItemBaseExportError {
    #[error("owned item-base export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Source(#[from] source::SourceError),
    #[error(transparent)]
    Extraction(#[from] game_data::GameDataExtractionError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, ItemBaseExportError>;
fn invalid(message: impl Into<String>) -> ItemBaseExportError {
    ItemBaseExportError::Invalid(message.into())
}

/// Provenance is evidence, not an authentication token. Deserializing or copying
/// these hashes cannot create an `AuthenticatedItemBaseExport`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    /// Every contributing source of the reused item constructor, including
    /// dependencies not retained in the narrower base-row source pin.
    pub extraction_source_files: Vec<SourceFilePin>,
    /// Compact serialization hash of the full intermediate ItemLoadingData.
    /// Its legacy tables are never published in the finite owned catalog.
    pub source_catalog_sha256: String,
    /// SHA-256 of precisely `catalog_bytes()`: pretty JSON followed by one LF.
    pub catalog_sha256: String,
    pub bases: usize,
    pub table_weapon_fields: usize,
    pub absent_weapon_fields: usize,
    pub unsupported_weapon_fields: usize,
}

/// Created only by fresh extraction from the independently authenticated source
/// root. Private fields prevent source-looking caller JSON from becoming proof.
#[derive(Debug)]
pub struct AuthenticatedItemBaseExport {
    catalog: ItemBaseCatalog,
    catalog_bytes: Vec<u8>,
    evidence: ItemBaseExportEvidence,
}
impl AuthenticatedItemBaseExport {
    pub fn catalog(&self) -> &ItemBaseCatalog {
        &self.catalog
    }
    pub fn catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }
    pub fn evidence(&self) -> &ItemBaseExportEvidence {
        &self.evidence
    }

    /// Verify all finite content against this independently extracted authority.
    /// A matching revision and source-file hashes alone are never sufficient.
    pub fn validate_catalog(&self, candidate: &ItemBaseCatalog) -> Result<()> {
        if candidate != &self.catalog {
            return Err(invalid(
                "catalog differs from independently authenticated extraction",
            ));
        }
        Ok(())
    }
}

/// Read the pinned source checkout, construct its final item catalog through the
/// existing audited extractor, and project a portable finite base-only catalog.
/// No caller-supplied catalog or provenance object is accepted as authority.
pub fn export_owned_item_bases(
    root: &Path,
    limits: ItemBaseExportLimits,
) -> Result<AuthenticatedItemBaseExport> {
    limits.validate()?;
    let source_manifest_sha256 = source::verify(root)?;
    let expected = game_data::expected_source_files()?;
    if expected.len() > limits.max_source_files {
        return Err(invalid("source file limit"));
    }
    let mut sources = BTreeMap::new();
    let mut bytes = 0usize;
    for (path, expected_hash) in expected {
        let text = source::read_verified_text(root, &path)?;
        bytes = bytes
            .checked_add(text.len())
            .ok_or_else(|| invalid("source byte count overflow"))?;
        if bytes > limits.max_source_bytes {
            return Err(invalid("source byte limit"));
        }
        if game_data::hash(text.as_bytes()) != expected_hash {
            return Err(invalid("verified source differs from extraction manifest"));
        }
        sources.insert(path, text);
    }
    let constructed = item_loading_extract::extract(&sources)?;
    let source_catalog_sha256 = serialized_sha256(&constructed)?;
    let constructed =
        ItemLoadingCatalog::new(constructed).map_err(|cause| invalid(cause.to_string()))?;
    if constructed.bases().len() > limits.max_bases {
        return Err(invalid("base count limit"));
    }
    let extraction_source_files: Vec<_> = constructed
        .data()
        .source
        .files
        .iter()
        .map(|(path, sha256)| SourceFilePin {
            path: path.clone(),
            sha256: sha256.clone(),
        })
        .collect();
    // Data.lua owns the finite module list and final-table construction. The
    // pinned Base modules are literal assignments and no later unique module
    // mutates itemBases. Item.lua pins the interpretation of base.weapon. All
    // broader constructor inputs remain recorded separately above.
    let mut base_sources = BTreeSet::from([DATA_PATH.to_owned(), ITEM_PATH.to_owned()]);
    for path in &constructed.data().source.module_order {
        if path.starts_with(BASE_PREFIX) {
            base_sources.insert(path.clone());
        }
    }
    let pins = base_sources
        .iter()
        .map(|path| {
            constructed
                .data()
                .source
                .files
                .get(path)
                .map(|sha256| SourceFilePin {
                    path: path.clone(),
                    sha256: sha256.clone(),
                })
                .ok_or_else(|| invalid("base source is absent from construction provenance"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut bases = Vec::with_capacity(constructed.bases().len());
    for base in constructed.bases() {
        for value in [&base.name, &base.item_type, &base.source_module] {
            if value.is_empty() || value.len() > limits.max_text_bytes || value.contains('\0') {
                return Err(invalid("invalid or oversized base text"));
            }
        }
        if !base.source_module.starts_with(BASE_PREFIX)
            || !base_sources.contains(&base.source_module)
        {
            return Err(invalid("base row has no authenticated base module"));
        }
        bases.push(ItemBaseRow {
            name: base.name.clone(),
            item_type: base.item_type.clone(),
            source_module: base.source_module.clone(),
            weapon_field: weapon_field(base.field("weapon")),
        });
    }
    bases.sort_by(|a, b| a.name.cmp(&b.name));
    let catalog = ItemBaseCatalog {
        schema_version: OWNED_ITEM_BASE_VERSION,
        source: SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: source::UPSTREAM_REVISION.into(),
            files: pins,
        },
        bases,
    };
    let mut output = BoundedBytes {
        bytes: Vec::new(),
        limit: limits.max_catalog_bytes,
    };
    serde_json::to_writer_pretty(&mut output, &catalog)?;
    output
        .write_all(b"\n")
        .map_err(|_| invalid("catalog byte limit"))?;
    let catalog_bytes = output.bytes;
    let evidence = ItemBaseExportEvidence {
        schema_version: 1,
        upstream_revision: source::UPSTREAM_REVISION.into(),
        source_manifest_sha256,
        extractor_sha256: game_data::hash(
            format!(
                "owned-item-base-export-v1\n{}\n{}",
                game_data::extractor_sha256(),
                include_str!("owned_item_bases.rs").replace("\r\n", "\n")
            )
            .as_bytes(),
        ),
        extraction_source_files,
        source_catalog_sha256,
        catalog_sha256: game_data::hash(&catalog_bytes),
        bases: catalog.bases.len(),
        table_weapon_fields: catalog
            .bases
            .iter()
            .filter(|b| b.weapon_field == ItemBaseWeaponField::Table)
            .count(),
        absent_weapon_fields: catalog
            .bases
            .iter()
            .filter(|b| b.weapon_field == ItemBaseWeaponField::Absent)
            .count(),
        unsupported_weapon_fields: catalog
            .bases
            .iter()
            .filter(|b| b.weapon_field == ItemBaseWeaponField::Unsupported)
            .count(),
    };
    Ok(AuthenticatedItemBaseExport {
        catalog,
        catalog_bytes,
        evidence,
    })
}

fn weapon_field(value: Option<&ItemMetadataValue>) -> ItemBaseWeaponField {
    match value {
        None => ItemBaseWeaponField::Absent,
        // Both encodings originate from Lua tables. Shape is not proof of any
        // numeric channel, martial type, action compatibility, or completeness.
        Some(ItemMetadataValue::Table(_) | ItemMetadataValue::Array(_)) => {
            ItemBaseWeaponField::Table
        }
        Some(_) => ItemBaseWeaponField::Unsupported,
    }
}

fn serialized_sha256(value: &impl Serialize) -> Result<String> {
    struct HashWriter(Sha256);
    impl Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut output = HashWriter(Sha256::new());
    serde_json::to_writer(&mut output, value)?;
    Ok(format!("{:x}", output.0.finalize()))
}
struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("catalog byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemOpaqueFunction, ItemSourceSpan};
    #[test]
    fn field_classification_never_uses_numeric_presence_or_scalar_truthiness() {
        assert_eq!(weapon_field(None), ItemBaseWeaponField::Absent);
        for value in [
            ItemMetadataValue::Table(ItemMetadataTable::default()),
            ItemMetadataValue::Array(vec![]),
        ] {
            assert_eq!(weapon_field(Some(&value)), ItemBaseWeaponField::Table);
        }
        for value in [
            ItemMetadataValue::Boolean(false),
            ItemMetadataValue::Boolean(true),
            ItemMetadataValue::Number(0.0),
            ItemMetadataValue::Text(String::new()),
            ItemMetadataValue::Callback(ItemOpaqueFunction {
                callback: ItemSourceSpan {
                    path: "callback".into(),
                    line: 1,
                    end_line: 1,
                    sha256: "0".repeat(64),
                },
            }),
        ] {
            assert_eq!(weapon_field(Some(&value)), ItemBaseWeaponField::Unsupported);
        }
    }
}
