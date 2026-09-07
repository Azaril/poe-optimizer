//! Versioned, data-only extraction of the pinned PoB passive tree.
//!
//! Call `extract_pinned_tree` only in an isolated offline extraction worker.
//! It evaluates a hash-verified data literal in a fresh, bounded Lua state;
//! snapshot consumers use owned Rust/serde values and never receive Lua handles.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::source;

pub const TREE_SNAPSHOT_SCHEMA: u32 = 1;
pub const SUPPORTED_TREE_VERSION: &str = "0_5";
const TREE_PATH: &str = "src/TreeData/0_5/tree.lua";
const LOADER_PATH: &str = "src/Classes/PassiveTree.lua";
const SPEC_PATH: &str = "src/Classes/PassiveSpec.lua";
const MANIFEST: &str = include_str!("../data/pob-source-manifest.json");
const MAX_TREE_BYTES: usize = 8 * 1024 * 1024;
const MAX_NODES: usize = 20_000;
const MAX_CONNECTIONS: usize = 200_000;
const MAX_VALUES: usize = 1_000_000;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum TreeDataError {
    #[error(transparent)]
    Source(#[from] source::SourceError),
    #[error("passive tree extraction failed: {0}")]
    Invalid(String),
    #[error("passive tree Lua extraction failed: {0}")]
    Lua(#[from] mlua::Error),
    #[error("passive tree input read failed: {0}")]
    Io(#[from] std::io::Error),
}

fn invalid(message: impl Into<String>) -> TreeDataError {
    TreeDataError::Invalid(message.into())
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
}

/// Extract only the committed 0_5 tree. This synchronous worker-side function
/// requires process supervision for a wall-clock deadline, including verification
/// and Lua parsing. It performs no network access and writes no files.
pub fn extract_pinned_tree(root: &Path, version: &str) -> Result<TreeDataSnapshot, TreeDataError> {
    if version != SUPPORTED_TREE_VERSION {
        return Err(invalid(format!(
            "unsupported tree version {version:?}; expected {SUPPORTED_TREE_VERSION}"
        )));
    }
    let identity = expected_identity()?;
    let actual_manifest = source::verify(root)?;
    if actual_manifest != identity.source_manifest_sha256 {
        return Err(invalid(
            "verified source manifest differs from extraction identity",
        ));
    }
    // Re-read the actual bytes that Lua will execute and verify them against the
    // embedded manifest; a change between source::verify and this read is rejected.
    let file = File::open(root.join(TREE_PATH))?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > (MAX_TREE_BYTES * 2) as u64 {
        return Err(invalid("tree must be an ordinary bounded file"));
    }
    let mut bytes = Vec::new();
    file.take((MAX_TREE_BYTES * 2 + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_TREE_BYTES * 2 {
        return Err(invalid("tree file exceeds byte limit"));
    }
    let text = String::from_utf8(bytes)
        .map_err(|error| invalid(error.to_string()))?
        .replace("\r\n", "\n");
    if text.len() > MAX_TREE_BYTES
        || hash(text.as_bytes()) != identity.source_files_sha256[TREE_PATH]
    {
        return Err(invalid("tree bytes changed after source verification"));
    }
    let lua = Lua::new_with(StdLib::NONE, LuaOptions::default())?;
    lua.set_memory_limit(128 * 1024 * 1024)?;
    let ticks = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(10_000),
        move |_, _| {
            if ticks.fetch_add(1, Ordering::Relaxed) >= 1_000 {
                return Err(mlua::Error::RuntimeError(
                    "tree extraction instruction limit".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    // No filesystem loader, require, OS, package, FFI, or application modules.
    let tree: Table = lua
        .load(&text)
        .set_name(TREE_PATH)
        .set_mode(mlua::chunk::ChunkMode::Text)
        .set_environment(lua.create_table()?)
        .eval()?;
    extract_table(tree, identity)
}

fn extract_table(
    tree: Table,
    identity: TreeSourceIdentity,
) -> Result<TreeDataSnapshot, TreeDataError> {
    let mut budget = ValueBudget::default();
    let mut classes = BTreeMap::new();
    let mut ascendancies = BTreeMap::new();
    let mut class_names = BTreeMap::new();
    let mut asc_names = BTreeMap::new();
    for entry in tree.get::<Table>("classes")?.sequence_values::<Table>() {
        let entry = entry?;
        let id: u32 = entry.get("integerId")?;
        let name: String = entry.get("name")?;
        let mut asc_ids = BTreeSet::new();
        for (index, asc) in entry
            .get::<Table>("ascendancies")?
            .sequence_values::<Table>()
            .enumerate()
        {
            let asc = asc?;
            let internal_id: String = asc.get("internalId")?;
            let catalog_id: String = asc.get("id")?;
            let name: String = asc.get("name")?;
            if asc_names
                .insert(catalog_id.clone(), internal_id.clone())
                .is_some()
            {
                return Err(invalid("duplicate ascendancy catalog ID"));
            }
            asc_ids.insert(internal_id.clone());
            let definition = TreeAscendancy {
                internal_id: internal_id.clone(),
                catalog_id,
                name,
                class_id: id,
                class_index: (index + 1) as u32,
                start_node_id: 0,
                replaces: asc.get("replace")?,
                replaced_by: asc.get("replaceBy")?,
                source: copy_table(&asc, &mut budget, 0)?,
            };
            if ascendancies.insert(internal_id, definition).is_some() {
                return Err(invalid("duplicate ascendancy internal ID"));
            }
        }
        let definition = TreeClass {
            integer_id: id,
            name: name.clone(),
            start_node_id: 0,
            ascendancy_ids: asc_ids,
            base_strength: entry.get("base_str")?,
            base_dexterity: entry.get("base_dex")?,
            base_intelligence: entry.get("base_int")?,
            source: copy_table(&entry, &mut budget, 0)?,
        };
        if class_names.insert(name, id).is_some() || classes.insert(id, definition).is_some() {
            return Err(invalid("duplicate class name or integer ID"));
        }
    }
    if classes.is_empty() || classes.len() > 64 || ascendancies.len() > 256 {
        return Err(invalid("unexpected class/ascendancy catalog size"));
    }
    let mut nodes = BTreeMap::new();
    let mut connection_count = 0;
    for pair in tree.get::<Table>("nodes")?.pairs::<Value, Table>() {
        let (key, entry) = pair?;
        if matches!(key, Value::String(ref key) if key.as_bytes().as_ref() == b"root") {
            continue;
        }
        let id: u32 = entry.get("skill")?;
        let kind = node_kind(&entry)?;
        let class_start_labels = string_sequence(entry.get("classesStart")?)?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let class_ids = class_start_labels
            .iter()
            .filter_map(|label| class_names.get(label).copied())
            .collect::<BTreeSet<_>>();
        for class_id in &class_ids {
            let class = classes
                .get_mut(class_id)
                .ok_or_else(|| invalid("unresolved class root"))?;
            if class.start_node_id != 0 {
                return Err(invalid("class has several physical roots"));
            }
            class.start_node_id = id;
        }
        let asc_name: Option<String> = entry.get("ascendancyName")?;
        let mut ascendancy_ids = BTreeSet::new();
        if let Some(name) = &asc_name {
            ascendancy_ids.insert(
                asc_names
                    .get(name)
                    .ok_or_else(|| invalid(format!("unknown ascendancy label {name}")))?
                    .clone(),
            );
        }
        let mut automatic_overrides = BTreeMap::new();
        let mut unsupported = BTreeSet::new();
        if flag(&entry, "isSwitchable")? {
            for option in entry.get::<Table>("options")?.pairs::<String, Table>() {
                let (selector, option) = option?;
                if !class_names.contains_key(&selector) && !asc_names.contains_key(&selector) {
                    unsupported.insert(format!("unknown_automatic_selector:{selector}"));
                }
                if asc_name.is_some()
                    && let Some(asc_id) = asc_names.get(&selector)
                {
                    ascendancy_ids.insert(asc_id.clone());
                }
                automatic_overrides.insert(selector, copy_table(&option, &mut budget, 0)?);
            }
        }
        if kind == TreeNodeKind::AscendancyStart {
            for asc_id in &ascendancy_ids {
                let asc = ascendancies
                    .get_mut(asc_id)
                    .ok_or_else(|| invalid("unresolved ascendancy root"))?;
                if asc.start_node_id != 0 {
                    return Err(invalid("ascendancy has several physical roots"));
                }
                asc.start_node_id = id;
            }
        }
        for (field, diagnostic) in [
            ("isAttribute", "attribute_choice"),
            ("isJewelSocket", "jewel_socket"),
            ("containJewelSocket", "embedded_jewel_socket"),
            ("expansionJewel", "generated_subgraph"),
            ("unlockConstraint", "unlock_constraint"),
            ("isMastery", "mastery_choice"),
            ("isMultipleChoice", "multiple_choice"),
            ("isMultipleChoiceOption", "multiple_choice_option"),
        ] {
            let value: Value = entry.get(field)?;
            if !matches!(value, Value::Nil | Value::Boolean(false)) {
                unsupported.insert(diagnostic.into());
            }
        }
        let (point_category, source_default_point_cost) = match kind {
            TreeNodeKind::ClassStart | TreeNodeKind::AscendancyStart => {
                (TreePointCategory::ImplicitRoot, Some(0))
            }
            TreeNodeKind::ImageOnly => (TreePointCategory::NonAllocatable, None),
            _ if flag(&entry, "isMultipleChoiceOption")? => {
                (TreePointCategory::UnsupportedChoice, None)
            }
            _ if asc_name.is_some() => (TreePointCategory::Ascendancy, Some(1)),
            _ => (TreePointCategory::Ordinary, Some(1)),
        };
        let mut raw_connections = BTreeSet::new();
        if let Some(connections) = entry.get::<Option<Table>>("connections")? {
            for connection in connections.sequence_values::<Table>() {
                connection_count += 1;
                if connection_count > MAX_CONNECTIONS {
                    return Err(invalid("connection count limit"));
                }
                raw_connections.insert(connection?.get("id")?);
            }
        }
        let node = TreeNode {
            id,
            string_id: entry.get("stringId")?,
            name: entry.get("name")?,
            kind,
            point_category,
            source_default_point_cost,
            class_start_labels,
            class_ids,
            ascendancy_ids,
            stats: string_sequence(entry.get("stats")?)?,
            raw_connections,
            adjacent: BTreeSet::new(),
            automatic_overrides,
            unsupported_mechanics: unsupported,
            source: copy_table(&entry, &mut budget, 0)?,
        };
        if nodes.insert(id, node).is_some() || nodes.len() > MAX_NODES {
            return Err(invalid("duplicate node ID or node count limit"));
        }
    }
    if classes.values().any(|class| class.start_node_id == 0)
        || ascendancies.values().any(|asc| asc.start_node_id == 0)
    {
        return Err(invalid("catalog class or ascendancy has no physical root"));
    }
    let mut dangling_connections = BTreeSet::new();
    let mut ignored_image_connections = BTreeSet::new();
    let mut ignored_self_connections = BTreeSet::new();
    let mut edges = BTreeSet::new();
    for node in nodes.values() {
        for target in &node.raw_connections {
            let Some(other) = nodes.get(target) else {
                dangling_connections.insert(DanglingConnection {
                    from: node.id,
                    missing: *target,
                });
                continue;
            };
            if node.kind == TreeNodeKind::ImageOnly || other.kind == TreeNodeKind::ImageOnly {
                ignored_image_connections.insert((node.id, *target));
            } else if node.id == *target {
                ignored_self_connections.insert(node.id);
            } else {
                edges.insert((node.id.min(*target), node.id.max(*target)));
            }
        }
    }
    for (left, right) in edges {
        nodes
            .get_mut(&left)
            .ok_or_else(|| invalid("missing edge source"))?
            .adjacent
            .insert(right);
        nodes
            .get_mut(&right)
            .ok_or_else(|| invalid("missing edge target"))?
            .adjacent
            .insert(left);
    }
    Ok(TreeDataSnapshot {
        identity,
        classes,
        ascendancies,
        nodes,
        dangling_connections,
        ignored_image_connections,
        ignored_self_connections,
        unsupported_mechanics: [
            "live_game_topology_completeness",
            "stat_and_modifier_translation",
            "weapon_set_allocations",
            "granted_and_free_allocations",
            "alternate_and_radius_starts",
            "jewels_and_generated_subgraphs",
            "explicit_hash_overrides",
            "attribute_and_mastery_choices",
            "unlock_constraints",
            "progression_and_derived_point_budgets",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    })
}

fn expected_identity() -> Result<TreeSourceIdentity, TreeDataError> {
    let manifest: serde_json::Value =
        serde_json::from_str(MANIFEST).map_err(|error| invalid(error.to_string()))?;
    let mut files = BTreeMap::new();
    for entry in manifest["files"]
        .as_array()
        .ok_or_else(|| invalid("invalid source manifest"))?
    {
        let path = entry["path"]
            .as_str()
            .ok_or_else(|| invalid("invalid manifest path"))?;
        if [TREE_PATH, LOADER_PATH, SPEC_PATH].contains(&path) {
            files.insert(
                path.to_owned(),
                entry["sha256"]
                    .as_str()
                    .ok_or_else(|| invalid("invalid manifest hash"))?
                    .to_owned(),
            );
        }
    }
    if files.len() != 3 {
        return Err(invalid("tree or loader absent from embedded manifest"));
    }
    Ok(TreeSourceIdentity {
        schema_version: TREE_SNAPSHOT_SCHEMA,
        upstream_revision: source::UPSTREAM_REVISION.into(),
        tree_version: SUPPORTED_TREE_VERSION.into(),
        source_manifest_sha256: hash(MANIFEST.replace("\r\n", "\n").as_bytes()),
        source_files_sha256: files,
        extractor_sha256: hash(
            include_str!("tree_data.rs")
                .replace("\r\n", "\n")
                .as_bytes(),
        ),
    })
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn flag(table: &Table, key: &str) -> Result<bool, mlua::Error> {
    Ok(table.get::<Option<bool>>(key)?.unwrap_or(false))
}
fn node_kind(table: &Table) -> Result<TreeNodeKind, TreeDataError> {
    Ok(if table.contains_key("classesStart")? {
        TreeNodeKind::ClassStart
    } else if flag(table, "isAscendancyStart")? {
        TreeNodeKind::AscendancyStart
    } else if flag(table, "isOnlyImage")? {
        TreeNodeKind::ImageOnly
    } else if flag(table, "isJewelSocket")? {
        TreeNodeKind::Socket
    } else if flag(table, "ks")? || flag(table, "isKeystone")? {
        TreeNodeKind::Keystone
    } else if flag(table, "not")? || flag(table, "isNotable")? {
        TreeNodeKind::Notable
    } else {
        TreeNodeKind::Normal
    })
}
fn string_sequence(table: Option<Table>) -> Result<Vec<String>, TreeDataError> {
    match table {
        Some(table) => table
            .sequence_values::<String>()
            .collect::<Result<_, _>>()
            .map_err(Into::into),
        None => Ok(Vec::new()),
    }
}

#[derive(Default)]
struct ValueBudget {
    values: usize,
    text_bytes: usize,
}

fn copy_table(
    table: &Table,
    budget: &mut ValueBudget,
    depth: usize,
) -> Result<SourceTable, TreeDataError> {
    if depth > 32 || table.metatable().is_some() {
        return Err(invalid("raw source table depth/metatable unsupported"));
    }
    let mut result = SourceTable::default();
    for pair in table.pairs::<Value, Value>() {
        let (key, value) = pair?;
        budget.values += 1;
        if budget.values > MAX_VALUES {
            return Err(invalid("source value count limit"));
        }
        let value = match value {
            Value::Boolean(value) => SourceValue::Boolean(value),
            Value::Integer(value) => SourceValue::Integer(value),
            Value::Number(value) if value.is_finite() => SourceValue::Number(value),
            Value::String(value) => SourceValue::String(copy_text(value, budget)?),
            Value::Table(value) => SourceValue::Table(copy_table(&value, budget, depth + 1)?),
            _ => return Err(invalid("source contains unsupported non-primitive value")),
        };
        match key {
            Value::String(key) => {
                result.named.insert(copy_text(key, budget)?, value);
            }
            Value::Integer(key) => {
                result.indexed.insert(key, value);
            }
            Value::Number(key)
                if key.is_finite()
                    && key.fract() == 0.0
                    && key.abs() <= 9_007_199_254_740_991.0 =>
            {
                result.indexed.insert(key as i64, value);
            }
            _ => return Err(invalid("source contains unsupported table key")),
        }
    }
    Ok(result)
}
fn copy_text(text: mlua::LuaString, budget: &mut ValueBudget) -> Result<String, TreeDataError> {
    budget.text_bytes += text.as_bytes().len();
    if budget.text_bytes > MAX_TEXT_BYTES {
        return Err(invalid("source text byte limit"));
    }
    Ok(text.to_str()?.to_owned())
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
