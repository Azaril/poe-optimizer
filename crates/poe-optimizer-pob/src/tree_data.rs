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
pub use poe_optimizer_data::tree_data::{
    DanglingConnection, EffectiveTreeNode, LOADER_PATH, OverrideProvenance, SPEC_PATH,
    SUPPORTED_TREE_VERSION, SourceTable, SourceValue, TREE_PATH, TREE_SNAPSHOT_SCHEMA,
    TreeAscendancy, TreeClass, TreeDataSnapshot, TreeNode, TreeNodeKind, TreePointCategory,
    TreeSourceIdentity,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::source;

const MAX_TREE_BYTES: usize = 8 * 1024 * 1024;
const MAX_NODES: usize = 20_000;
const MAX_CONNECTIONS: usize = 200_000;
const MAX_VALUES: usize = 1_000_000;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum TreeDataError {
    #[error(transparent)]
    Data(#[from] poe_optimizer_data::tree_data::TreeDataError),
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
    for (source_index, entry) in tree
        .get::<Table>("classes")?
        .sequence_values::<Table>()
        .enumerate()
    {
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
            source_index: source_index as u32 + 1,
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
    let identity = poe_optimizer_data::tree_data::expected_identity()?;
    let mut fingerprint = Sha256::new();
    fingerprint.update(b"poe-tree-extractor-and-model-v2");
    fingerprint.update(include_str!("tree_data.rs").replace("\r\n", "\n"));
    fingerprint
        .update(include_str!("../../poe-optimizer-data/src/tree_data.rs").replace("\r\n", "\n"));
    if identity.extractor_sha256 != format!("{:x}", fingerprint.finalize()) {
        return Err(invalid(
            "compiled tree extractor/model differ from the supported identity manifest",
        ));
    }
    Ok(identity)
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
