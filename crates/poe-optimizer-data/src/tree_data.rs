//! Portable, owned tree snapshot data and explicit source-identity validation.
//! Matching labels validates supported provenance claims; only a separately trusted
//! complete-content digest can authenticate externally supplied snapshot contents.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
pub const TREE_SNAPSHOT_SCHEMA: u32 = 2;
pub const SUPPORTED_TREE_VERSION: &str = "0_5";
pub const TREE_PATH: &str = "src/TreeData/0_5/tree.lua";
pub const LOADER_PATH: &str = "src/Classes/PassiveTree.lua";
pub const SPEC_PATH: &str = "src/Classes/PassiveSpec.lua";
#[derive(Debug, Error)]
pub enum TreeDataError {
    #[error("invalid passive tree data: {0}")]
    Invalid(String),
}
fn invalid(message: impl Into<String>) -> TreeDataError {
    TreeDataError::Invalid(message.into())
}
/// Compiled, source-reviewed provenance manifest. Not proof of arbitrary input contents.
pub fn expected_identity() -> Result<TreeSourceIdentity, TreeDataError> {
    serde_json::from_str(include_str!("../data/tree-source-identity.json"))
        .map_err(|error| invalid(error.to_string()))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Lua table key types remain distinct, including numeric attribute choices.
/// Empty tables remain tables; they are never guessed to be JSON arrays.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTable {
    pub named: BTreeMap<String, SourceValue>,
    pub indexed: BTreeMap<i64, SourceValue>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SourceValue {
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Table(SourceTable),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeSourceIdentity {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub tree_version: String,
    pub source_manifest_sha256: String,
    pub source_files_sha256: BTreeMap<String, String>,
    pub extractor_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeClass {
    /// One-based source classes-array position, distinct from canonical integer_id.
    pub source_index: u32,
    pub integer_id: u32,
    pub name: String,
    pub start_node_id: u32,
    pub ascendancy_ids: BTreeSet<String>,
    pub base_strength: u32,
    pub base_dexterity: u32,
    pub base_intelligence: u32,
    pub source: SourceTable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAscendancy {
    pub internal_id: String,
    pub catalog_id: String,
    pub name: String,
    pub class_id: u32,
    pub class_index: u32,
    pub start_node_id: u32,
    pub replaces: Option<String>,
    pub replaced_by: Option<String>,
    pub source: SourceTable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeNodeKind {
    ClassStart,
    AscendancyStart,
    ImageOnly,
    Socket,
    Keystone,
    Notable,
    Normal,
}

/// Source-default accounting only. Runtime-granted/free allocations and weapon
/// sets can change costs and need their own validation before search uses them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreePointCategory {
    ImplicitRoot,
    Ordinary,
    Ascendancy,
    NonAllocatable,
    UnsupportedChoice,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNode {
    pub id: u32,
    pub string_id: Option<String>,
    pub name: String,
    pub kind: TreeNodeKind,
    pub point_category: TreePointCategory,
    pub source_default_point_cost: Option<u32>,
    pub class_start_labels: BTreeSet<String>,
    pub class_ids: BTreeSet<u32>,
    pub ascendancy_ids: BTreeSet<String>,
    pub stats: Vec<String>,
    pub raw_connections: BTreeSet<u32>,
    pub adjacent: BTreeSet<u32>,
    pub automatic_overrides: BTreeMap<String, SourceTable>,
    pub unsupported_mechanics: BTreeSet<String>,
    pub source: SourceTable,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DanglingConnection {
    pub from: u32,
    pub missing: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeDataSnapshot {
    pub identity: TreeSourceIdentity,
    pub classes: BTreeMap<u32, TreeClass>,
    pub ascendancies: BTreeMap<String, TreeAscendancy>,
    pub nodes: BTreeMap<u32, TreeNode>,
    pub dangling_connections: BTreeSet<DanglingConnection>,
    pub ignored_image_connections: BTreeSet<(u32, u32)>,
    pub ignored_self_connections: BTreeSet<u32>,
    pub unsupported_mechanics: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OverrideProvenance {
    Base,
    Class {
        class_id: u32,
        selector: String,
    },
    Ascendancy {
        internal_id: String,
        selector: String,
    },
}

/// The chosen source record with PoB's shallow fallback to the base record.
/// `physical_node_id` remains the allocation/graph key. This is a source-data
/// view, not a claim that every field is replaced on PoB's live spec node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveTreeNode {
    pub physical_node_id: u32,
    pub effective_source_id: u32,
    pub name: String,
    pub stats: Vec<String>,
    pub provenance: OverrideProvenance,
    pub override_fields: BTreeSet<String>,
    pub source: SourceTable,
}

impl TreeDataSnapshot {
    /// Reject a stale schema, pin, loader or extractor identity. This checks
    /// provenance claims only; arbitrary deserialized snapshot contents are not
    /// authenticated merely because these fields match.
    pub fn validate_source_identity(&self) -> Result<(), TreeDataError> {
        if self.identity != expected_identity()? {
            return Err(invalid("snapshot source or extractor identity is stale"));
        }
        Ok(())
    }

    /// A stable digest of the complete serialized snapshot, including evidence.
    pub fn sha256(&self) -> Result<String, TreeDataError> {
        let bytes = serde_json::to_vec(self).map_err(|error| invalid(error.to_string()))?;
        Ok(hash(&bytes))
    }

    pub fn ordinary_entrances(&self, class_id: u32) -> Result<BTreeSet<u32>, TreeDataError> {
        let class = self
            .classes
            .get(&class_id)
            .ok_or_else(|| invalid("unknown class ID"))?;
        let root = self
            .nodes
            .get(&class.start_node_id)
            .ok_or_else(|| invalid("missing class root"))?;
        Ok(root
            .adjacent
            .iter()
            .copied()
            .filter(|id| {
                self.nodes
                    .get(id)
                    .is_some_and(|node| node.point_category == TreePointCategory::Ordinary)
            })
            .collect())
    }

    /// Resolve automatic class/ascendancy switches, preserving base graph IDs.
    /// Class selectors take precedence, exactly as BuildAllDependsAndPaths.
    /// Attribute choices, jewels and explicit hash overrides remain unsupported.
    pub fn effective_node(
        &self,
        class_id: u32,
        ascendancy_internal_id: Option<&str>,
        physical_node_id: u32,
    ) -> Result<EffectiveTreeNode, TreeDataError> {
        let class = self
            .classes
            .get(&class_id)
            .ok_or_else(|| invalid("unknown class ID"))?;
        let ascendancy = ascendancy_internal_id
            .map(|id| {
                let asc = self
                    .ascendancies
                    .get(id)
                    .ok_or_else(|| invalid("unknown ascendancy internal ID"))?;
                if asc.class_id != class_id {
                    return Err(invalid("ascendancy does not belong to the selected class"));
                }
                Ok(asc)
            })
            .transpose()?;
        let node = self
            .nodes
            .get(&physical_node_id)
            .ok_or_else(|| invalid("unknown physical node ID"))?;
        effective_source_node(class, ascendancy, node)
    }
}

pub(crate) fn effective_source_node(
    class: &TreeClass,
    ascendancy: Option<&TreeAscendancy>,
    node: &TreeNode,
) -> Result<EffectiveTreeNode, TreeDataError> {
    let physical_node_id = node.id;
    let class_id = class.integer_id;
    let selected = if let Some(option) = node.automatic_overrides.get(&class.name) {
        Some((
            option,
            OverrideProvenance::Class {
                class_id,
                selector: class.name.clone(),
            },
        ))
    } else {
        ascendancy.and_then(|asc| {
            node.automatic_overrides.get(&asc.name).map(|option| {
                (
                    option,
                    OverrideProvenance::Ascendancy {
                        internal_id: asc.internal_id.clone(),
                        selector: asc.name.clone(),
                    },
                )
            })
        })
    };
    let mut effective = node.source.clone();
    let mut override_fields = BTreeSet::new();
    let provenance = match selected {
        Some((option, provenance)) => {
            for (key, value) in &option.named {
                override_fields.insert(key.clone());
                effective.named.insert(key.clone(), value.clone());
            }
            effective.indexed.extend(option.indexed.clone());
            provenance
        }
        None => OverrideProvenance::Base,
    };
    Ok(EffectiveTreeNode {
        physical_node_id,
        effective_source_id: source_u32(&effective, "id")?.unwrap_or(physical_node_id),
        name: source_string(&effective, "name")?.unwrap_or_else(|| node.name.clone()),
        stats: source_strings(&effective, "stats")?.unwrap_or_default(),
        provenance,
        override_fields,
        source: effective,
    })
}

fn source_u32(table: &SourceTable, field: &str) -> Result<Option<u32>, TreeDataError> {
    match table.named.get(field) {
        None => Ok(None),
        Some(SourceValue::Integer(value)) => u32::try_from(*value)
            .map(Some)
            .map_err(|_| invalid("source integer out of range")),
        Some(SourceValue::Number(value))
            if value.fract() == 0.0 && *value >= 0.0 && *value <= u32::MAX as f64 =>
        {
            Ok(Some(*value as u32))
        }
        _ => Err(invalid(format!(
            "source field {field} must be an unsigned integer"
        ))),
    }
}
fn source_string(table: &SourceTable, field: &str) -> Result<Option<String>, TreeDataError> {
    match table.named.get(field) {
        None => Ok(None),
        Some(SourceValue::String(value)) => Ok(Some(value.clone())),
        _ => Err(invalid(format!("source field {field} must be a string"))),
    }
}
fn source_strings(table: &SourceTable, field: &str) -> Result<Option<Vec<String>>, TreeDataError> {
    let Some(value) = table.named.get(field) else {
        return Ok(None);
    };
    let SourceValue::Table(table) = value else {
        return Err(invalid("source string list is not a table"));
    };
    if !table.named.is_empty()
        || !table
            .indexed
            .keys()
            .copied()
            .eq(1..=table.indexed.len() as i64)
    {
        return Err(invalid(
            "source string list is not a contiguous one-based sequence",
        ));
    }
    table
        .indexed
        .values()
        .map(|value| match value {
            SourceValue::String(value) => Ok(value.clone()),
            _ => Err(invalid("source string list contains non-string")),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
