//! Typed selections and finite graph composition for the admitted class/entrance bundle.
//!
//! A selection contains paid physical IDs only. Roots are implicit, and an effective
//! source override never replaces the physical allocation ID. These helpers use an
//! already validated bundle; they do not authenticate arbitrary deserialized data or
//! certify point budgets, item requirements, or whole-game legality.

use crate::{
    bundled::BundledClassTree,
    game_data::GameDataSnapshot,
    tree_data::{EffectiveTreeNode, TreeAscendancy, TreeClass, TreeNodeKind},
};
use poe_optimizer_core::candidate::{
    AscendancyDefinition, CandidateCatalog, CandidateConstraints, CandidateDomain, CatalogIdentity,
    ClassDefinition, PassiveKind, PassiveNode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
#[error("invalid class/tree selection or catalog: {0}")]
pub struct ClassTreeError(pub String);

fn error(value: impl std::fmt::Display) -> ClassTreeError {
    ClassTreeError(value.to_string())
}

/// Zero or one ordinary entrance, with no paid ascendancy nodes. The caller must
/// supply progression budgets separately; choosing a node does not establish one.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClassTreeSelection {
    pub class_id: u32,
    pub ascendancy_id: Option<String>,
    pub entrance_node_id: Option<u32>,
}

/// Available base attributes from the selected class. Admitted roots are statless
/// and current entrance operations do not modify attributes. New attribute effects
/// require explicit requirement-resolution work before expanding this contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClassBaseAttributes {
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedClassTree {
    pub selection: ClassTreeSelection,
    pub class: TreeClass,
    pub ascendancy: Option<TreeAscendancy>,
    pub paid_node: Option<EffectiveTreeNode>,
    pub implicit_roots: BTreeSet<u32>,
    /// Canonical realized allocation: implicit roots plus the optional paid ID.
    pub allocated_nodes: BTreeSet<u32>,
    pub base_attributes: ClassBaseAttributes,
}

impl ClassTreeSelection {
    /// Resolve only within this bundle's admitted choices. Use the immutable tree
    /// from a `GameDataSnapshot` or the authenticated bundled tree; source labels
    /// alone do not establish a caller-created bundle's integrity.
    pub fn resolve(&self, tree: &BundledClassTree) -> Result<ResolvedClassTree, ClassTreeError> {
        let class = tree.class(self.class_id).map_err(error)?;
        let ascendancy = self
            .ascendancy_id
            .as_deref()
            .map(|id| tree.ascendancy(self.class_id, id).map_err(error))
            .transpose()?;
        let paid_node = self
            .entrance_node_id
            .map(|id| tree.entrance(self.class_id, id).map_err(error))
            .transpose()?;
        let mut implicit_roots = BTreeSet::from([class.start_node_id]);
        if let Some(ascendancy) = ascendancy {
            implicit_roots.insert(ascendancy.start_node_id);
        }
        let mut allocated_nodes = implicit_roots.clone();
        allocated_nodes.extend(self.entrance_node_id);
        Ok(ResolvedClassTree {
            selection: self.clone(),
            class: class.clone(),
            ascendancy: ascendancy.cloned(),
            paid_node: paid_node.cloned(),
            implicit_roots,
            allocated_nodes,
            base_attributes: ClassBaseAttributes {
                strength: class.base_strength,
                dexterity: class.base_dexterity,
                intelligence: class.base_intelligence,
            },
        })
    }
}

/// All admitted structural selections in class, ascendancy, entrance order, with
/// `None` before named/paid choices. The pinned bundle has 93; callers filter this
/// finite list with their own explicit budgets and locks before calculation.
pub fn selections(tree: &BundledClassTree) -> Result<Vec<ClassTreeSelection>, ClassTreeError> {
    tree.validate_scope().map_err(error)?;
    let mut result = Vec::new();
    for (class_id, class) in &tree.classes {
        for ascendancy_id in std::iter::once(None).chain(class.ascendancy_ids.iter().map(Some)) {
            for entrance_node_id in std::iter::once(None)
                .chain(tree.entrances(*class_id).map_err(error)?.keys().map(Some))
            {
                let selection = ClassTreeSelection {
                    class_id: *class_id,
                    ascendancy_id: ascendancy_id.cloned(),
                    entrance_node_id: entrance_node_id.copied(),
                };
                selection.resolve(tree)?;
                result.push(selection);
            }
        }
    }
    Ok(result)
}

/// Construct an empty non-tree catalog for composition from an explicitly partial
/// bundle. Only retained roots/ordinary entrances and edges between retained nodes
/// are included. Shared physical root owner sets remain intact. Coverage omissions
/// remain in `snapshot.tree().coverage`, outside selected-mechanic blockers.
///
/// The identity binds the selected full dataset and this projection implementation.
/// A composer must rebind its final catalog identity to all added payloads/rules.
/// No available points are inferred, and no full `TreeDataSnapshot` is synthesized.
pub fn candidate_catalog(snapshot: &GameDataSnapshot) -> Result<CandidateCatalog, ClassTreeError> {
    let tree = snapshot.tree();
    tree.validate_scope().map_err(error)?;
    let retained: BTreeSet<_> = tree
        .roots
        .keys()
        .chain(tree.ordinary_nodes.keys())
        .copied()
        .collect();
    let mut passive_nodes = BTreeMap::new();
    for (id, node) in tree.roots.iter().chain(&tree.ordinary_nodes) {
        let kind = match node.kind {
            TreeNodeKind::ClassStart => PassiveKind::ClassStart {
                class_ids: node.class_ids.iter().map(u32::to_string).collect(),
            },
            TreeNodeKind::AscendancyStart => PassiveKind::AscendancyStart {
                ascendancy_ids: node.ascendancy_ids.clone(),
            },
            TreeNodeKind::Normal => PassiveKind::Ordinary,
            _ => return Err(error("unsupported node reached admitted bundle projection")),
        };
        passive_nodes.insert(
            *id,
            PassiveNode {
                kind,
                point_cost: node
                    .source_default_point_cost
                    .ok_or_else(|| error("missing admitted node point cost"))?,
                links: node.adjacent.intersection(&retained).copied().collect(),
                unsupported_mechanics: node.unsupported_mechanics.clone(),
            },
        );
    }
    let projection_source_sha256 = format!(
        "{:x}",
        Sha256::digest(
            include_str!("class_tree.rs")
                .replace("\r\n", "\n")
                .as_bytes()
        )
    );
    let identity_bytes = serde_json::to_vec(&(
        "bundled-class-tree-catalog-v1",
        snapshot.identity(),
        projection_source_sha256,
    ))
    .map_err(error)?;
    let catalog = CandidateCatalog {
        identity: CatalogIdentity {
            schema_version: poe_optimizer_core::candidate::CANDIDATE_SCHEMA_VERSION,
            game: "path_of_exile_2".into(),
            rules_revision: tree.source.upstream_revision.clone(),
            content_fingerprint: format!("{:x}", Sha256::digest(identity_bytes)),
        },
        classes: tree
            .classes
            .iter()
            .map(|(id, class)| {
                (
                    id.to_string(),
                    ClassDefinition {
                        start_node_id: class.start_node_id,
                    },
                )
            })
            .collect(),
        ascendancies: tree
            .ascendancies
            .iter()
            .map(|(id, ascendancy)| {
                (
                    id.clone(),
                    AscendancyDefinition {
                        class_id: ascendancy.class_id.to_string(),
                        start_node_id: ascendancy.start_node_id,
                    },
                )
            })
            .collect(),
        passive_nodes,
        equipment_slots: BTreeSet::new(),
        skill_slots: BTreeSet::new(),
        items: BTreeMap::new(),
        active_skills: BTreeMap::new(),
        supports: BTreeMap::new(),
        support_definition_limits: BTreeMap::new(),
        unsupported_mechanics: BTreeSet::new(),
    };
    // This checks graph ownership/reference structure, not a run's available points.
    CandidateDomain::new(catalog.clone(), CandidateConstraints::default()).map_err(error)?;
    Ok(catalog)
}
