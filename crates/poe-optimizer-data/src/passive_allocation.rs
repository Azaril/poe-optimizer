//! General physical passive allocations and capability-selected source views.
//! Structural presence alone never grants calculation or allocation admission.
use crate::{
    class_tree::ClassBaseAttributes,
    game_data::{ActorModifierRecord, PassiveEffect},
    tree_data::{TreeAscendancy, TreeClass, TreeNodeKind, TreePointCategory},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum AttributeOption {
    Strength,
    Dexterity,
    Intelligence,
}
impl AttributeOption {
    pub const ALL: [Self; 3] = [Self::Strength, Self::Dexterity, Self::Intelligence];
    pub const fn source_index(self) -> u32 {
        match self {
            Self::Strength => 1,
            Self::Dexterity => 2,
            Self::Intelligence => 3,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PassiveViewSelector {
    Base,
    Class { class_id: u32 },
    Ascendancy { ascendancy_id: String },
    Attribute { option: AttributeOption },
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct PassiveViewKey {
    pub physical_node_id: u32,
    pub selector: PassiveViewSelector,
}
/// Compact complete ordinary topology. Excluded views remain structural evidence.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AllocationNode {
    pub id: u32,
    pub name: String,
    pub kind: TreeNodeKind,
    pub point_category: TreePointCategory,
    pub source_default_point_cost: Option<u32>,
    pub adjacent: BTreeSet<u32>,
    pub attribute_options: BTreeMap<AttributeOption, u32>,
    pub automatic_selectors: BTreeSet<PassiveViewSelector>,
    pub unsupported_mechanics: BTreeSet<String>,
    pub source_sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PassiveNodeView {
    pub key: PassiveViewKey,
    pub effective_node_id: u32,
    pub name: String,
    pub stats: Vec<String>,
    pub source_sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExcludedPassiveView {
    pub key: PassiveViewKey,
    pub reason: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct PassiveAllocationSelection {
    pub class_id: u32,
    pub ascendancy_id: Option<String>,
    pub ordinary_nodes: BTreeSet<u32>,
    pub ascendancy_nodes: BTreeSet<u32>,
    pub attribute_options: BTreeMap<u32, AttributeOption>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedPassiveView {
    pub source: PassiveNodeView,
    pub effects: Vec<PassiveEffect>,
    pub actor_modifiers: Vec<ActorModifierRecord>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedPassiveAllocation {
    pub selection: PassiveAllocationSelection,
    pub class: TreeClass,
    pub ascendancy: Option<TreeAscendancy>,
    pub implicit_roots: BTreeSet<u32>,
    pub allocated_nodes: BTreeSet<u32>,
    pub base_attributes: ClassBaseAttributes,
    /// Physical-ID order is explicit. Source-order-sensitive modifier forms reject admission.
    pub views: Vec<ResolvedPassiveView>,
}

fn invalid(message: impl std::fmt::Display) -> crate::class_tree::ClassTreeError {
    crate::class_tree::ClassTreeError(message.to_string())
}
fn digest(value: &impl Serialize) -> Result<String, crate::class_tree::ClassTreeError> {
    use sha2::{Digest, Sha256};
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(invalid)?)
    ))
}
pub fn key_for_effective(view: &crate::tree_data::EffectiveTreeNode) -> PassiveViewKey {
    use crate::tree_data::OverrideProvenance;
    PassiveViewKey {
        physical_node_id: view.physical_node_id,
        selector: match &view.provenance {
            OverrideProvenance::Base => PassiveViewSelector::Base,
            OverrideProvenance::Class { class_id, .. } => PassiveViewSelector::Class {
                class_id: *class_id,
            },
            OverrideProvenance::Ascendancy { internal_id, .. } => PassiveViewSelector::Ascendancy {
                ascendancy_id: internal_id.clone(),
            },
        },
    }
}
fn view(
    key: PassiveViewKey,
    source: crate::tree_data::SourceTable,
) -> Result<PassiveNodeView, crate::class_tree::ClassTreeError> {
    Ok(PassiveNodeView {
        effective_node_id: source_u32(&source, "id")
            .map_err(invalid)?
            .unwrap_or(key.physical_node_id),
        name: source_string(&source, "name")
            .map_err(invalid)?
            .ok_or_else(|| invalid("passive view lacks source name"))?,
        stats: source_strings(&source, "stats")
            .map_err(invalid)?
            .unwrap_or_default(),
        source_sha256: digest(&source)?,
        key,
    })
}
pub(crate) fn source_allocation_views(
    snapshot: &crate::tree_data::TreeDataSnapshot,
    ascendancy_views: &BTreeMap<String, BTreeMap<u32, crate::tree_data::EffectiveTreeNode>>,
) -> Result<(BTreeMap<u32, AllocationNode>, Vec<PassiveNodeView>), crate::class_tree::ClassTreeError>
{
    use crate::tree_data::SourceValue;
    let mut nodes = BTreeMap::new();
    let mut views = Vec::new();
    for (id, node) in &snapshot.nodes {
        if node.point_category != TreePointCategory::Ordinary {
            continue;
        }
        let mut attribute_options = BTreeMap::new();
        let mut automatic_selectors = BTreeSet::new();
        let choices = match node.source.named.get("options") {
            Some(SourceValue::Table(table)) => Some(table),
            None => None,
            _ => return Err(invalid("source options must be a table")),
        };
        if node.unsupported_mechanics.contains("attribute_choice") {
            let options = choices.ok_or_else(|| invalid("attribute node lacks source options"))?;
            if options.indexed.len() != 3 || !options.named.is_empty() {
                return Err(invalid(
                    "attribute source options require exact three indexed choices",
                ));
            }
            for option in AttributeOption::ALL {
                let Some(SourceValue::Table(overrides)) =
                    options.indexed.get(&i64::from(option.source_index()))
                else {
                    return Err(invalid("missing source attribute choice"));
                };
                let mut effective = node.source.clone();
                effective.named.extend(overrides.named.clone());
                let key = PassiveViewKey {
                    physical_node_id: *id,
                    selector: PassiveViewSelector::Attribute { option },
                };
                let value = view(key, effective)?;
                attribute_options.insert(option, value.effective_node_id);
                views.push(value);
            }
        } else {
            if choices.is_some_and(|table| !table.indexed.is_empty()) {
                return Err(invalid("unmodeled numeric source options"));
            }
            views.push(view(
                PassiveViewKey {
                    physical_node_id: *id,
                    selector: PassiveViewSelector::Base,
                },
                node.source.clone(),
            )?);
        }
        for (name, overrides) in &node.automatic_overrides {
            let selector = if let Some(class) =
                snapshot.classes.values().find(|class| class.name == *name)
            {
                PassiveViewSelector::Class {
                    class_id: class.integer_id,
                }
            } else if let Some(asc) = snapshot.ascendancies.values().find(|asc| asc.name == *name) {
                PassiveViewSelector::Ascendancy {
                    ascendancy_id: asc.internal_id.clone(),
                }
            } else {
                return Err(invalid("unknown automatic passive source selector"));
            };
            let mut effective = node.source.clone();
            effective.named.extend(overrides.named.clone());
            effective.indexed.extend(overrides.indexed.clone());
            views.push(view(
                PassiveViewKey {
                    physical_node_id: *id,
                    selector: selector.clone(),
                },
                effective,
            )?);
            automatic_selectors.insert(selector);
        }
        nodes.insert(
            *id,
            AllocationNode {
                id: *id,
                name: node.name.clone(),
                kind: node.kind,
                point_category: node.point_category,
                source_default_point_cost: node.source_default_point_cost,
                adjacent: node.adjacent.clone(),
                attribute_options,
                automatic_selectors,
                unsupported_mechanics: node.unsupported_mechanics.clone(),
                source_sha256: digest(node)?,
            },
        );
    }
    for values in ascendancy_views.values() {
        for source in values.values() {
            views.push(view(key_for_effective(source), source.source.clone())?)
        }
    }
    views.sort_by(|a, b| a.key.cmp(&b.key));
    Ok((nodes, views))
}
pub(crate) fn validate_source_views(
    tree: &crate::bundled::BundledClassTree,
) -> Result<(), crate::class_tree::ClassTreeError> {
    let mut expected = BTreeSet::new();
    if tree.allocation_nodes.len() != 4109 {
        return Err(invalid(
            "complete pinned ordinary topology requires 4109 nodes",
        ));
    }
    for (id, node) in &tree.allocation_nodes {
        if *id != node.id
            || node.point_category != TreePointCategory::Ordinary
            || node.source_sha256.len() != 64
        {
            return Err(invalid("invalid complete ordinary source node"));
        }
        if node.attribute_options.is_empty() {
            expected.insert(PassiveViewKey {
                physical_node_id: *id,
                selector: PassiveViewSelector::Base,
            });
        } else {
            if node.attribute_options.keys().copied().collect::<Vec<_>>() != AttributeOption::ALL
                || !node.unsupported_mechanics.contains("attribute_choice")
            {
                return Err(invalid("incomplete explicit attribute source options"));
            }
            for option in AttributeOption::ALL {
                expected.insert(PassiveViewKey {
                    physical_node_id: *id,
                    selector: PassiveViewSelector::Attribute { option },
                });
            }
        }
        for selector in &node.automatic_selectors {
            match selector {
                PassiveViewSelector::Class { class_id } if tree.classes.contains_key(class_id) => {}
                PassiveViewSelector::Ascendancy { ascendancy_id }
                    if tree.ascendancies.contains_key(ascendancy_id) => {}
                _ => return Err(invalid("invalid automatic source selector")),
            }
            expected.insert(PassiveViewKey {
                physical_node_id: *id,
                selector: selector.clone(),
            });
        }
    }
    for values in tree.ascendancy_passives.values() {
        for source in values.values() {
            expected.insert(key_for_effective(source));
        }
    }
    let mut found = BTreeSet::new();
    let mut previous = None;
    for view in &tree.allocation_views {
        if view.source_sha256.len() != 64
            || !view
                .source_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || view.name.is_empty()
            || view.stats.len() > 64
            || view.stats.iter().any(|s| s.len() > 4096)
            || !found.insert(view.key.clone())
            || previous.is_some_and(|p| p >= &view.key)
        {
            return Err(invalid(
                "invalid, duplicate or unordered passive source view",
            ));
        }
        previous = Some(&view.key);
        if let PassiveViewSelector::Attribute { option } = view.key.selector
            && tree
                .allocation_nodes
                .get(&view.key.physical_node_id)
                .and_then(|node| node.attribute_options.get(&option))
                != Some(&view.effective_node_id)
        {
            return Err(invalid("attribute option effective source ID mismatch"));
        }
    }
    if found != expected {
        return Err(invalid("source passive view partition is incomplete"));
    }
    Ok(())
}
impl PassiveAllocationSelection {
    /// Resolve one ordinary capability for graph traversal. This checks context,
    /// source override and whole-effect admission; it does not assert connectivity.
    pub fn resolve_ordinary_node(
        &self,
        snapshot: &crate::game_data::GameDataSnapshot,
        physical_node_id: u32,
        option: Option<AttributeOption>,
    ) -> Result<ResolvedPassiveView, crate::class_tree::ClassTreeError> {
        let tree = snapshot.tree();
        tree.class(self.class_id).map_err(invalid)?;
        if let Some(id) = &self.ascendancy_id {
            tree.ascendancy(self.class_id, id).map_err(invalid)?;
        }
        let node = tree
            .allocation_nodes
            .get(&physical_node_id)
            .ok_or_else(|| invalid("unknown ordinary physical passive"))?;
        let selector = if !node.attribute_options.is_empty() {
            PassiveViewSelector::Attribute {
                option: option.ok_or_else(|| {
                    invalid("allocated attribute passive requires an explicit source option")
                })?,
            }
        } else {
            if option.is_some() {
                return Err(invalid(
                    "attribute override targets a non-attribute passive",
                ));
            }
            let class_selector = PassiveViewSelector::Class {
                class_id: self.class_id,
            };
            let asc_selector =
                self.ascendancy_id
                    .as_ref()
                    .map(|id| PassiveViewSelector::Ascendancy {
                        ascendancy_id: id.clone(),
                    });
            if node.automatic_selectors.contains(&class_selector) {
                class_selector
            } else if let Some(selector) =
                asc_selector.filter(|s| node.automatic_selectors.contains(s))
            {
                selector
            } else {
                PassiveViewSelector::Base
            }
        };
        let key = PassiveViewKey {
            physical_node_id,
            selector,
        };
        resolve_view(snapshot, &key)
    }
    /// Resolve one owned reviewed ascendancy capability without claiming connectivity.
    pub fn resolve_ascendancy_node(
        &self,
        snapshot: &crate::game_data::GameDataSnapshot,
        physical_node_id: u32,
    ) -> Result<ResolvedPassiveView, crate::class_tree::ClassTreeError> {
        let id = self
            .ascendancy_id
            .as_deref()
            .ok_or_else(|| invalid("ascendancy passive requires selected ascendancy"))?;
        let source = snapshot
            .tree()
            .ascendancy_passive(self.class_id, id, physical_node_id)
            .map_err(invalid)?;
        resolve_view(snapshot, &key_for_effective(source))
    }
    pub fn resolve(
        &self,
        snapshot: &crate::game_data::GameDataSnapshot,
    ) -> Result<ResolvedPassiveAllocation, crate::class_tree::ClassTreeError> {
        let tree = snapshot.tree();
        let class = tree.class(self.class_id).map_err(invalid)?;
        let ascendancy = self
            .ascendancy_id
            .as_deref()
            .map(|id| tree.ascendancy(self.class_id, id).map_err(invalid))
            .transpose()?;
        let mut roots = BTreeSet::from([class.start_node_id]);
        if let Some(asc) = ascendancy {
            roots.insert(asc.start_node_id);
        }
        if !self.ordinary_nodes.is_disjoint(&self.ascendancy_nodes)
            || self
                .ordinary_nodes
                .iter()
                .chain(&self.ascendancy_nodes)
                .any(|id| roots.contains(id))
            || self
                .attribute_options
                .keys()
                .any(|id| !self.ordinary_nodes.contains(id))
        {
            return Err(invalid(
                "paid physical selections/attribute options contain roots, duplicates or unallocated overrides",
            ));
        }
        let mut views = Vec::new();
        for id in &self.ordinary_nodes {
            views.push(self.resolve_ordinary_node(
                snapshot,
                *id,
                self.attribute_options.get(id).copied(),
            )?);
        }
        for id in &self.ascendancy_nodes {
            views.push(self.resolve_ascendancy_node(snapshot, *id)?);
        }
        let connected = |selected: &BTreeSet<u32>, root: u32, ordinary: bool| {
            let mut visited = BTreeSet::from([root]);
            let mut pending = vec![root];
            while let Some(id) = pending.pop() {
                let links = if id == root {
                    &tree.roots[&root].adjacent
                } else if ordinary {
                    &tree.allocation_nodes[&id].adjacent
                } else {
                    &tree.ascendancy_nodes[&id].adjacent
                };
                for next in links {
                    if selected.contains(next) && visited.insert(*next) {
                        pending.push(*next)
                    }
                }
            }
            selected.is_subset(&visited)
        };
        if !connected(&self.ordinary_nodes, class.start_node_id, true)
            || ascendancy
                .is_some_and(|asc| !connected(&self.ascendancy_nodes, asc.start_node_id, false))
        {
            return Err(invalid(
                "selected physical passives are disconnected from their owned root",
            ));
        }
        views.sort_by(|a, b| a.source.key.cmp(&b.source.key));
        let mut allocated = roots.clone();
        allocated.extend(&self.ordinary_nodes);
        allocated.extend(&self.ascendancy_nodes);
        Ok(ResolvedPassiveAllocation {
            selection: self.clone(),
            class: class.clone(),
            ascendancy: ascendancy.cloned(),
            implicit_roots: roots,
            allocated_nodes: allocated,
            base_attributes: ClassBaseAttributes {
                strength: class.base_strength,
                dexterity: class.base_dexterity,
                intelligence: class.base_intelligence,
            },
            views,
        })
    }
}
fn resolve_view(
    snapshot: &crate::game_data::GameDataSnapshot,
    key: &PassiveViewKey,
) -> Result<ResolvedPassiveView, crate::class_tree::ClassTreeError> {
    let source = snapshot
        .tree()
        .allocation_views
        .iter()
        .find(|view| &view.key == key)
        .ok_or_else(|| invalid("unknown passive source view"))?;
    let effects = snapshot
        .package()
        .passive_view_effects(key)
        .ok_or_else(|| {
            invalid(format!(
                "unsupported whole passive view: {}",
                snapshot
                    .package()
                    .passive_exclusions
                    .iter()
                    .find(|e| &e.key == key)
                    .map_or("missing numerical admission", |e| e.reason.as_str())
            ))
        })?;
    Ok(ResolvedPassiveView {
        source: source.clone(),
        effects: effects.effects.clone(),
        actor_modifiers: effects.actor_modifiers.clone(),
    })
}
impl From<crate::class_tree::ClassTreeSelection> for PassiveAllocationSelection {
    fn from(value: crate::class_tree::ClassTreeSelection) -> Self {
        Self {
            class_id: value.class_id,
            ascendancy_id: value.ascendancy_id,
            ordinary_nodes: value.entrance_node_id.into_iter().collect(),
            ascendancy_nodes: value.ascendancy_node_id.into_iter().collect(),
            attribute_options: BTreeMap::new(),
        }
    }
}

/// Union graph of admitted source views. A selected class/override still must pass resolve.
pub fn allocation_candidate_catalog(
    snapshot: &crate::game_data::GameDataSnapshot,
) -> Result<poe_optimizer_core::candidate::CandidateCatalog, crate::class_tree::ClassTreeError> {
    use poe_optimizer_core::candidate::{
        CandidateConstraints, CandidateDomain, PassiveKind, PassiveNode,
    };
    use sha2::{Digest, Sha256};
    let mut catalog = crate::class_tree::candidate_catalog(snapshot)?;
    let tree = snapshot.tree();
    let admitted: BTreeSet<_> = snapshot
        .package()
        .passive_effects
        .iter()
        .map(|entry| entry.key.physical_node_id)
        .collect();
    let retained: BTreeSet<_> = admitted
        .iter()
        .copied()
        .chain(tree.roots.keys().copied())
        .collect();
    for (id, node) in &tree.allocation_nodes {
        if !admitted.contains(id) {
            continue;
        }
        catalog.passive_nodes.insert(
            *id,
            PassiveNode {
                kind: PassiveKind::Ordinary,
                point_cost: node
                    .source_default_point_cost
                    .ok_or_else(|| invalid("admitted source node lacks point cost"))?,
                links: node.adjacent.intersection(&retained).copied().collect(),
                unsupported_mechanics: BTreeSet::new(),
            },
        );
    }
    for (id, node) in &mut catalog.passive_nodes {
        let links = tree
            .roots
            .get(id)
            .or_else(|| tree.ascendancy_nodes.get(id))
            .map(|node| &node.adjacent);
        if let Some(links) = links {
            node.links = links.intersection(&retained).copied().collect();
        }
    }
    catalog.identity.content_fingerprint = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                "capability-passive-graph-v1",
                snapshot.identity(),
                &catalog.passive_nodes,
                include_str!("passive_allocation.rs").replace("\r\n", "\n")
            ))
            .map_err(invalid)?
        )
    );
    CandidateDomain::new(catalog.clone(), CandidateConstraints::default()).map_err(invalid)?;
    Ok(catalog)
}

fn source_u32(
    table: &crate::tree_data::SourceTable,
    key: &str,
) -> Result<Option<u32>, crate::class_tree::ClassTreeError> {
    use crate::tree_data::SourceValue;
    match table.named.get(key) {
        None => Ok(None),
        Some(SourceValue::Integer(v)) => u32::try_from(*v).map(Some).map_err(invalid),
        Some(SourceValue::Number(v))
            if v.is_finite() && v.fract() == 0.0 && *v >= 0.0 && *v <= u32::MAX as f64 =>
        {
            Ok(Some(*v as u32))
        }
        _ => Err(invalid("source field must be an unsigned integer")),
    }
}
fn source_string(
    table: &crate::tree_data::SourceTable,
    key: &str,
) -> Result<Option<String>, crate::class_tree::ClassTreeError> {
    match table.named.get(key) {
        None => Ok(None),
        Some(crate::tree_data::SourceValue::String(s)) => Ok(Some(s.clone())),
        _ => Err(invalid("source field must be a string")),
    }
}
fn source_strings(
    table: &crate::tree_data::SourceTable,
    key: &str,
) -> Result<Option<Vec<String>>, crate::class_tree::ClassTreeError> {
    use crate::tree_data::SourceValue;
    let table = match table.named.get(key) {
        None => return Ok(None),
        Some(SourceValue::Table(table)) => table,
        _ => return Err(invalid("source strings must be an array")),
    };
    if !table.named.is_empty()
        || table.indexed.keys().copied().collect::<Vec<_>>()
            != (1..=table.indexed.len() as i64).collect::<Vec<_>>()
    {
        return Err(invalid("source strings must be dense"));
    }
    table
        .indexed
        .values()
        .map(|v| match v {
            SourceValue::String(s) => Ok(s.clone()),
            _ => Err(invalid("source string array contains non-string")),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
