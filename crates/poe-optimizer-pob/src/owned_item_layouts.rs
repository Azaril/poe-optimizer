//! Authenticated, finite evidence of base-generated modifier prefixes.
//!
//! The reviewed Item.lua constructor prepends flask/charm buff entries. This
//! projection inspects the final constructed fields rather than inferring their
//! absence from a weapon kind. Buff text and source tables stay in the adapter.
use std::{io::Write, path::Path};

use poe_optimizer_data::item_loading::ItemMetadataValue;
use poe_optimizer_import::{
    owned_item_layouts::{
        ItemBaseGeneratedPrefix, ItemBaseLayoutCatalog, ItemBaseLayoutRow,
        OWNED_ITEM_LAYOUT_VERSION,
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
pub struct ItemLayoutExportLimits {
    pub bases: ItemBaseExportLimits,
    pub max_layouts: usize,
    pub max_buff_lines_per_base: usize,
    pub max_total_buff_lines: usize,
    pub max_catalog_bytes: usize,
}
impl Default for ItemLayoutExportLimits {
    fn default() -> Self {
        Self {
            bases: Default::default(),
            max_layouts: 4096,
            max_buff_lines_per_base: 128,
            max_total_buff_lines: 16_384,
            max_catalog_bytes: 2 * 1024 * 1024,
        }
    }
}
impl ItemLayoutExportLimits {
    fn validate(self) -> Result<()> {
        let maximum = Self::default();
        for (name, actual, limit) in [
            ("layouts", self.max_layouts, maximum.max_layouts),
            (
                "buff lines per base",
                self.max_buff_lines_per_base,
                maximum.max_buff_lines_per_base,
            ),
            (
                "total buff lines",
                self.max_total_buff_lines,
                maximum.max_total_buff_lines,
            ),
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
pub enum ItemLayoutExportError {
    #[error("owned item-layout export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Base(#[from] ItemBaseExportError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, ItemLayoutExportError>;
fn invalid(message: impl Into<String>) -> ItemLayoutExportError {
    ItemLayoutExportError::Invalid(message.into())
}

/// Evidence is inspectable but does not authenticate caller-supplied content.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLayoutExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    pub extraction_source_files: Vec<SourceFilePin>,
    pub source_catalog_sha256: String,
    pub base_catalog_sha256: String,
    pub catalog_sha256: String,
    pub bases: usize,
    pub absent_prefixes: usize,
    pub present_prefixes: usize,
    pub unsupported_prefixes: usize,
    pub buff_lines: usize,
}

/// Only fresh verified source construction can create this authority.
#[derive(Debug)]
pub struct AuthenticatedItemLayoutExport {
    base_export: AuthenticatedItemBaseExport,
    catalog: ItemBaseLayoutCatalog,
    catalog_bytes: Vec<u8>,
    evidence: ItemLayoutExportEvidence,
}
impl AuthenticatedItemLayoutExport {
    pub fn base_export(&self) -> &AuthenticatedItemBaseExport {
        &self.base_export
    }
    pub fn catalog(&self) -> &ItemBaseLayoutCatalog {
        &self.catalog
    }
    /// Exact identity-bearing bytes: pretty JSON followed by one LF.
    pub fn catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }
    pub fn evidence(&self) -> &ItemLayoutExportEvidence {
        &self.evidence
    }
    pub fn validate_catalog(&self, candidate: &ItemBaseLayoutCatalog) -> Result<()> {
        let bytes = catalog_bytes(candidate, self.catalog_bytes.len())?;
        if bytes != self.catalog_bytes {
            return Err(invalid(
                "catalog differs from independently authenticated extraction",
            ));
        }
        Ok(())
    }
}

/// Project reviewed generated-prefix presence for every final constructed base.
/// The existing v1 item-base export remains byte-identical and is retained for
/// the caller's separately authenticated policy binding.
pub fn export_owned_item_layouts(
    root: &Path,
    limits: ItemLayoutExportLimits,
) -> Result<AuthenticatedItemLayoutExport> {
    limits.validate()?;
    let (base_export, constructed) = extract_owned_item_bases(root, limits.bases)?;
    if base_export.catalog().bases.len() > limits.max_layouts {
        return Err(invalid("layout count limit"));
    }
    let mut bases = Vec::with_capacity(base_export.catalog().bases.len());
    let mut buff_lines = 0usize;
    let mut absent_prefixes = 0;
    let mut present_prefixes = 0;
    let mut unsupported_prefixes = 0;
    for base in &base_export.catalog().bases {
        let original = constructed
            .base(&base.name)
            .ok_or_else(|| invalid("constructed base is missing"))?;
        let mut per_base = 0usize;
        let flask = parent_prefix(
            original.field("flask"),
            limits,
            &mut per_base,
            &mut buff_lines,
        )?;
        let charm = parent_prefix(
            original.field("charm"),
            limits,
            &mut per_base,
            &mut buff_lines,
        )?;
        let prefix = merge_prefix(flask, charm);
        match prefix {
            ItemBaseGeneratedPrefix::Absent => absent_prefixes += 1,
            ItemBaseGeneratedPrefix::Present => present_prefixes += 1,
            ItemBaseGeneratedPrefix::Unsupported => unsupported_prefixes += 1,
        }
        bases.push(ItemBaseLayoutRow {
            source_base: base.name.clone(),
            prefix,
        });
    }
    let catalog = ItemBaseLayoutCatalog {
        schema_version: OWNED_ITEM_LAYOUT_VERSION,
        source: base_export.catalog().source.clone(),
        bases,
    };
    let catalog_bytes = catalog_bytes(&catalog, limits.max_catalog_bytes)?;
    let base_evidence = base_export.evidence();
    let evidence = ItemLayoutExportEvidence {
        schema_version: 1,
        upstream_revision: base_evidence.upstream_revision.clone(),
        source_manifest_sha256: base_evidence.source_manifest_sha256.clone(),
        extractor_sha256: game_data::hash(
            format!(
                "owned-item-layout-export-v1\n{}\n{}",
                base_evidence.extractor_sha256,
                include_str!("owned_item_layouts.rs").replace("\r\n", "\n"),
            )
            .as_bytes(),
        ),
        extraction_source_files: base_evidence.extraction_source_files.clone(),
        source_catalog_sha256: base_evidence.source_catalog_sha256.clone(),
        base_catalog_sha256: base_evidence.catalog_sha256.clone(),
        catalog_sha256: game_data::hash(&catalog_bytes),
        bases: catalog.bases.len(),
        absent_prefixes,
        present_prefixes,
        unsupported_prefixes,
        buff_lines,
    };
    Ok(AuthenticatedItemLayoutExport {
        base_export,
        catalog,
        catalog_bytes,
        evidence,
    })
}

fn catalog_bytes(catalog: &ItemBaseLayoutCatalog, limit: usize) -> Result<Vec<u8>> {
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

fn merge_prefix(a: ItemBaseGeneratedPrefix, b: ItemBaseGeneratedPrefix) -> ItemBaseGeneratedPrefix {
    use ItemBaseGeneratedPrefix::{Absent, Present, Unsupported};
    match (a, b) {
        (Unsupported, _) | (_, Unsupported) => Unsupported,
        (Present, _) | (_, Present) => Present,
        (Absent, Absent) => Absent,
    }
}

fn parent_prefix(
    parent: Option<&ItemMetadataValue>,
    limits: ItemLayoutExportLimits,
    per_base: &mut usize,
    total: &mut usize,
) -> Result<ItemBaseGeneratedPrefix> {
    use ItemBaseGeneratedPrefix::{Absent, Unsupported};
    let buff = match parent {
        None => return Ok(Absent),
        // The extractor rejects metatables, so a missing named field is exact.
        Some(ItemMetadataValue::Table(table)) if table.indexed.is_empty() => {
            table.fields.get("buff")
        }
        Some(ItemMetadataValue::Array(values)) if values.is_empty() => return Ok(Absent),
        _ => return Ok(Unsupported),
    };
    buff_prefix(buff, limits, per_base, total)
}

fn buff_prefix(
    buff: Option<&ItemMetadataValue>,
    limits: ItemLayoutExportLimits,
    per_base: &mut usize,
    total: &mut usize,
) -> Result<ItemBaseGeneratedPrefix> {
    use ItemBaseGeneratedPrefix::{Absent, Present, Unsupported};
    let count = match buff {
        None => return Ok(Absent),
        Some(ItemMetadataValue::Array(values)) => values.len(),
        Some(ItemMetadataValue::Table(table)) if table.fields.is_empty() => table.indexed.len(),
        _ => return Ok(Unsupported),
    };
    *per_base = per_base
        .checked_add(count)
        .ok_or_else(|| invalid("buff count overflow"))?;
    *total = total
        .checked_add(count)
        .ok_or_else(|| invalid("buff count overflow"))?;
    if *per_base > limits.max_buff_lines_per_base {
        return Err(invalid("buff lines per base limit"));
    }
    if *total > limits.max_total_buff_lines {
        return Err(invalid("total buff lines limit"));
    }
    // Charge the full visited list before inspection; unknown shapes stay blocked.
    let supported = match buff {
        Some(ItemMetadataValue::Array(values)) => values
            .iter()
            .all(|v| matches!(v, ItemMetadataValue::Text(_))),
        Some(ItemMetadataValue::Table(table)) => {
            table.indexed.iter().enumerate().all(|(i, (key, value))| {
                i64::try_from(i + 1).ok() == Some(*key)
                    && matches!(value, ItemMetadataValue::Text(_))
            })
        }
        _ => unreachable!("classified above"),
    };
    if !supported {
        return Ok(Unsupported);
    }
    // Empty string is still an ipairs member, hence still a generated prefix.
    Ok(if count == 0 { Absent } else { Present })
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemOpaqueFunction, ItemSourceSpan};
    use std::collections::BTreeMap;

    fn prefix(value: Option<&ItemMetadataValue>) -> ItemBaseGeneratedPrefix {
        buff_prefix(value, Default::default(), &mut 0, &mut 0).unwrap()
    }
    fn parent(value: Option<&ItemMetadataValue>) -> ItemBaseGeneratedPrefix {
        parent_prefix(value, Default::default(), &mut 0, &mut 0).unwrap()
    }
    fn table(buff: ItemMetadataValue) -> ItemMetadataValue {
        ItemMetadataValue::Table(ItemMetadataTable {
            fields: BTreeMap::from([("buff".into(), buff)]),
            ..Default::default()
        })
    }

    #[test]
    fn exact_absence_and_empty_tables_are_distinct_from_empty_string_entries() {
        use ItemBaseGeneratedPrefix::{Absent, Present};
        assert_eq!(parent(None), Absent);
        assert_eq!(prefix(None), Absent);
        for empty in [
            ItemMetadataValue::Array(vec![]),
            ItemMetadataValue::Table(Default::default()),
        ] {
            assert_eq!(parent(Some(&empty)), Absent);
            assert_eq!(prefix(Some(&empty)), Absent);
            assert_eq!(parent(Some(&table(empty))), Absent);
        }
        for text in ["", "Immune to Freeze"] {
            let buff = ItemMetadataValue::Array(vec![ItemMetadataValue::Text(text.into())]);
            assert_eq!(prefix(Some(&buff)), Present);
            assert_eq!(parent(Some(&table(buff))), Present);
        }
        let dense = ItemMetadataValue::Table(ItemMetadataTable {
            fields: Default::default(),
            indexed: BTreeMap::from([(1, ItemMetadataValue::Text(String::new()))]),
        });
        assert_eq!(prefix(Some(&dense)), Present);
    }

    #[test]
    fn scalar_callbacks_mixed_and_sparse_fields_never_establish_absence() {
        use ItemBaseGeneratedPrefix::Unsupported;
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
            assert_eq!(parent(Some(&value)), Unsupported);
            assert_eq!(prefix(Some(&value)), Unsupported);
        }
        for buff in [
            ItemMetadataValue::Array(vec![ItemMetadataValue::Number(0.0)]),
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: BTreeMap::from([("future".into(), ItemMetadataValue::Text("x".into()))]),
                indexed: Default::default(),
            }),
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: Default::default(),
                indexed: BTreeMap::from([(2, ItemMetadataValue::Text("x".into()))]),
            }),
        ] {
            assert_eq!(prefix(Some(&buff)), Unsupported);
            assert_eq!(parent(Some(&table(buff))), Unsupported);
        }
    }

    #[test]
    fn both_construction_branches_and_resource_budgets_are_preserved() {
        use ItemBaseGeneratedPrefix::{Absent, Present, Unsupported};
        for (a, b, want) in [
            (Absent, Absent, Absent),
            (Absent, Present, Present),
            (Present, Absent, Present),
            (Present, Unsupported, Unsupported),
            (Unsupported, Absent, Unsupported),
        ] {
            assert_eq!(merge_prefix(a, b), want);
        }
        let buff = ItemMetadataValue::Array(vec![
            ItemMetadataValue::Text("a".into()),
            ItemMetadataValue::Text("b".into()),
        ]);
        assert!(
            buff_prefix(
                Some(&buff),
                ItemLayoutExportLimits {
                    max_buff_lines_per_base: 1,
                    ..Default::default()
                },
                &mut 0,
                &mut 0
            )
            .is_err()
        );
        assert!(
            buff_prefix(
                Some(&buff),
                ItemLayoutExportLimits {
                    max_total_buff_lines: 1,
                    ..Default::default()
                },
                &mut 0,
                &mut 0
            )
            .is_err()
        );
        let mut per_base = 1;
        assert!(
            buff_prefix(
                Some(&buff),
                ItemLayoutExportLimits {
                    max_buff_lines_per_base: 2,
                    ..Default::default()
                },
                &mut per_base,
                &mut 0
            )
            .is_err()
        );
    }
}
