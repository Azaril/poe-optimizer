//! Conservative finite tree catalogs, produced from authenticated pinned snapshots.
//! This layer resolves data/provenance; it is not a whole-game legality certificate.

use crate::{
    tree_data::{
        SourceTable, SourceValue, TreeDataSnapshot, TreeNode, TreeNodeKind, TreePointCategory,
        TreeSourceIdentity,
    },
    tree_worker,
};
use poe_optimizer_core::candidate::{
    AscendancyDefinition, CandidateCatalog, CandidateConstraints, CandidateDomain, CatalogIdentity,
    ClassDefinition, PassiveKind, PassiveNode,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;

pub const TREE_PROJECTION_SCHEMA: u32 = 1;
pub const ELIGIBILITY_POLICY: &str = "normal_paid_nodes_v1";

#[derive(Debug, Error)]
pub enum TreeProjectionError {
    #[error("tree snapshot authentication failed: {0}")]
    Authentication(String),
    #[error("tree extraction failed: {0}")]
    Extraction(String),
    #[error("cannot project passive {node_id}: {reason}")]
    UnsupportedNode { node_id: u32, reason: String },
    #[error("invalid projected tree catalog: {0}")]
    InvalidCatalog(String),
}
type Result<T> = std::result::Result<T, TreeProjectionError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationProvenance {
    FreshPinnedExtraction,
    TrustedContentDigest,
}

/// Immutable after construction. No Deserialize implementation can bypass the
/// extraction/trusted-digest constructors. A digest's trusted origin is the
/// caller's responsibility, not something a matching source label can establish.
#[derive(Clone, Debug)]
pub struct AuthenticatedTreeSnapshot {
    snapshot: Arc<TreeDataSnapshot>,
    content_sha256: String,
    provenance: AuthenticationProvenance,
}
impl AuthenticatedTreeSnapshot {
    /// Use the application's supervised extraction worker, including its hard
    /// deadline and source checks. `executable` must be our trusted CLI binary.
    pub fn extract(executable: &Path, pob: &Path, timeout: Duration) -> Result<Self> {
        let snapshot = tree_worker::extract_tree(
            executable,
            pob,
            crate::tree_data::SUPPORTED_TREE_VERSION,
            timeout,
        )
        .map_err(|error| TreeProjectionError::Extraction(error.to_string()))?;
        let content_sha256 = snapshot
            .sha256()
            .map_err(|error| TreeProjectionError::Authentication(error.to_string()))?;
        Ok(Self {
            snapshot: Arc::new(snapshot),
            content_sha256,
            provenance: AuthenticationProvenance::FreshPinnedExtraction,
        })
    }

    /// `expected_sha256` must come from a trusted extraction or independent
    /// trusted manifest. Computing it from the same untrusted input supplies no
    /// authentication. It is the canonical snapshot digest, not a pretty-JSON
    /// file digest. Source identity is also checked for a supported pin/schema.
    pub fn from_trusted_digest(snapshot: TreeDataSnapshot, expected_sha256: &str) -> Result<Self> {
        if expected_sha256.len() != 64
            || !expected_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(TreeProjectionError::Authentication(
                "trusted digest must be lowercase SHA-256".into(),
            ));
        }
        snapshot
            .validate_source_identity()
            .map_err(|error| TreeProjectionError::Authentication(error.to_string()))?;
        let actual = snapshot
            .sha256()
            .map_err(|error| TreeProjectionError::Authentication(error.to_string()))?;
        if actual != expected_sha256 {
            return Err(TreeProjectionError::Authentication(
                "snapshot contents differ from the trusted digest".into(),
            ));
        }
        Ok(Self {
            snapshot: Arc::new(snapshot),
            content_sha256: actual,
            provenance: AuthenticationProvenance::TrustedContentDigest,
        })
    }

    pub fn snapshot(&self) -> &TreeDataSnapshot {
        &self.snapshot
    }
    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }
    pub fn provenance(&self) -> AuthenticationProvenance {
        self.provenance
    }
}

/// Kept outside CandidateCatalog's selected-mechanic blockers. It describes the
/// finite projection and source coverage without silently declaring it complete.
#[derive(Clone, Debug, Serialize)]
pub struct TreeProjectionCoverage {
    pub schema_version: u32,
    pub eligibility_policy: String,
    pub source: TreeSourceIdentity,
    pub snapshot_sha256: String,
    pub projection_source_sha256: String,
    pub authentication: AuthenticationProvenance,
    pub source_node_count: usize,
    pub projected_node_count: usize,
    pub allowed_paid_node_ids: BTreeSet<u32>,
    pub unselected_node_count: usize,
    /// Valid source edges removed because exactly one endpoint is projected.
    /// Endpoints are in ascending order, without inventing replacement paths.
    pub boundary_edges: BTreeSet<(u32, u32)>,
    pub source_dangling_connections: BTreeSet<crate::tree_data::DanglingConnection>,
    pub source_unsupported_mechanics: BTreeSet<String>,
    pub limitations: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct TreeProjection {
    source: AuthenticatedTreeSnapshot,
    catalog: CandidateCatalog,
    coverage: TreeProjectionCoverage,
}
impl TreeProjection {
    /// `allowed_paid_nodes` is a finite choice catalog, not an allocation. Every
    /// candidate still supplies its own connected subset and explicit budgets.
    /// All catalog classes/ascendancies remain present; unsupported choices fail
    /// construction rather than silently dropping classes or repairing paths.
    pub fn new(
        source: &AuthenticatedTreeSnapshot,
        allowed_paid_nodes: BTreeSet<u32>,
    ) -> Result<Self> {
        let snapshot = source.snapshot();
        let mut included = BTreeSet::new();
        for class in snapshot.classes.values() {
            included.insert(class.start_node_id);
        }
        for ascendancy in snapshot.ascendancies.values() {
            included.insert(ascendancy.start_node_id);
        }
        for root in &included {
            let node = snapshot
                .nodes
                .get(root)
                .ok_or_else(|| unsupported(*root, "catalog root is missing"))?;
            if !matches!(
                node.kind,
                TreeNodeKind::ClassStart | TreeNodeKind::AscendancyStart
            ) || node.point_category != TreePointCategory::ImplicitRoot
                || node.source_default_point_cost != Some(0)
            {
                return Err(unsupported(
                    *root,
                    "catalog root has unsupported type or point accounting",
                ));
            }
            validate_node_metadata(snapshot, node)?;
        }
        for id in &allowed_paid_nodes {
            let node = snapshot
                .nodes
                .get(id)
                .ok_or_else(|| unsupported(*id, "unknown or dangling node ID"))?;
            if included.contains(id) {
                return Err(unsupported(
                    *id,
                    "implicit roots cannot be selected as paid nodes",
                ));
            }
            if node.kind != TreeNodeKind::Normal {
                return Err(unsupported(
                    *id,
                    format!(
                        "policy {ELIGIBILITY_POLICY} excludes {:?}; notable, keystone, socket and image semantics need separate validation",
                        node.kind
                    ),
                ));
            }
            if !matches!(
                node.point_category,
                TreePointCategory::Ordinary | TreePointCategory::Ascendancy
            ) || node.source_default_point_cost != Some(1)
            {
                return Err(unsupported(
                    *id,
                    "unsupported paid-node point category or cost",
                ));
            }
            validate_node_metadata(snapshot, node)?;
        }
        included.extend(allowed_paid_nodes.iter().copied());
        let mut nodes = BTreeMap::new();
        let mut boundary_edges = BTreeSet::new();
        for id in &included {
            let node = &snapshot.nodes[id];
            let kind = match node.kind {
                TreeNodeKind::ClassStart => PassiveKind::ClassStart {
                    class_ids: node.class_ids.iter().map(u32::to_string).collect(),
                },
                TreeNodeKind::AscendancyStart => PassiveKind::AscendancyStart {
                    ascendancy_ids: node.ascendancy_ids.clone(),
                },
                TreeNodeKind::Normal if node.point_category == TreePointCategory::Ascendancy => {
                    PassiveKind::Ascendancy {
                        ascendancy_ids: node.ascendancy_ids.clone(),
                    }
                }
                TreeNodeKind::Normal => PassiveKind::Ordinary,
                _ => {
                    return Err(unsupported(
                        *id,
                        "unsupported node reached catalog construction",
                    ));
                }
            };
            let mut links = BTreeSet::new();
            for target in &node.adjacent {
                let other = snapshot.nodes.get(target).ok_or_else(|| {
                    unsupported(*id, "snapshot adjacency references a missing node")
                })?;
                if !other.adjacent.contains(id) || target == id {
                    return Err(unsupported(
                        *id,
                        "snapshot adjacency is not a simple symmetric graph",
                    ));
                }
                if included.contains(target) {
                    links.insert(*target);
                } else {
                    boundary_edges.insert(((*id).min(*target), (*id).max(*target)));
                }
            }
            nodes.insert(
                *id,
                PassiveNode {
                    kind,
                    point_cost: node.source_default_point_cost.unwrap_or(0),
                    links,
                    unsupported_mechanics: BTreeSet::new(),
                },
            );
        }
        let projection_source_sha256 = hash(
            include_str!("tree_projection.rs")
                .replace("\r\n", "\n")
                .as_bytes(),
        );
        let identity_bytes = serde_json::to_vec(&(
            "pob-tree-projection",
            TREE_PROJECTION_SCHEMA,
            ELIGIBILITY_POLICY,
            &projection_source_sha256,
            source.content_sha256(),
            &allowed_paid_nodes,
        ))
        .map_err(|error| TreeProjectionError::InvalidCatalog(error.to_string()))?;
        let catalog = CandidateCatalog {
            identity: CatalogIdentity {
                schema_version: poe_optimizer_core::candidate::CANDIDATE_SCHEMA_VERSION,
                game: "path_of_exile_2".into(),
                rules_revision: snapshot.identity.upstream_revision.clone(),
                content_fingerprint: hash(&identity_bytes),
            },
            classes: snapshot
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
            ascendancies: snapshot
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
            passive_nodes: nodes,
            equipment_slots: BTreeSet::new(),
            skill_slots: BTreeSet::new(),
            items: BTreeMap::new(),
            active_skills: BTreeMap::new(),
            supports: BTreeMap::new(),
            support_definition_limits: BTreeMap::new(),
            unsupported_mechanics: BTreeSet::new(),
        };
        // Reuse core validation for graph targets, shared owner sets and references.
        // These empty constraints do not become a run's actual progression budget.
        CandidateDomain::new(catalog.clone(), CandidateConstraints::default())
            .map_err(|error| TreeProjectionError::InvalidCatalog(format!("{error:?}")))?;
        let coverage = TreeProjectionCoverage {
            schema_version: TREE_PROJECTION_SCHEMA,
            eligibility_policy: ELIGIBILITY_POLICY.into(),
            source: snapshot.identity.clone(),
            snapshot_sha256: source.content_sha256().into(),
            projection_source_sha256,
            authentication: source.provenance(),
            source_node_count: snapshot.nodes.len(),
            projected_node_count: included.len(),
            allowed_paid_node_ids: allowed_paid_nodes,
            unselected_node_count: snapshot.nodes.len() - included.len(),
            boundary_edges,
            source_dangling_connections: snapshot.dangling_connections.clone(),
            source_unsupported_mechanics: snapshot.unsupported_mechanics.clone(),
            limitations: [
                "finite_normal_node_graph_only",
                "not_a_live_game_legality_certificate",
                "explicit_progression_budgets_required",
                "actual_effective_overrides_require_fresh_backend_observation",
                "stat_effects_require_the_selected_calculation_backend",
                "weapon_sets_jewels_grants_alternate_starts_and_choices_unmodeled",
                "notable_and_keystone_provider_semantics_unreviewed",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        };
        Ok(Self {
            source: source.clone(),
            catalog,
            coverage,
        })
    }

    pub fn catalog(&self) -> &CandidateCatalog {
        &self.catalog
    }
    pub fn snapshot(&self) -> &TreeDataSnapshot {
        self.source.snapshot()
    }
    pub fn allowed_paid_nodes(&self) -> &BTreeSet<u32> {
        &self.coverage.allowed_paid_node_ids
    }
    pub fn source_coverage(&self) -> &TreeProjectionCoverage {
        &self.coverage
    }
}

fn unsupported(node_id: u32, reason: impl Into<String>) -> TreeProjectionError {
    TreeProjectionError::UnsupportedNode {
        node_id,
        reason: reason.into(),
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_node_metadata(snapshot: &TreeDataSnapshot, node: &TreeNode) -> Result<()> {
    if !node.unsupported_mechanics.is_empty() {
        return Err(unsupported(
            node.id,
            format!(
                "unmodeled selected-node mechanics: {:?}",
                node.unsupported_mechanics
            ),
        ));
    }
    for field in [
        "isAttribute",
        "isJewelSocket",
        "containJewelSocket",
        "expansionJewel",
        "unlockConstraint",
        "isMastery",
        "isMultipleChoice",
        "isMultipleChoiceOption",
        "isFreeAllocate",
        "isGrantedPassive",
    ] {
        if node
            .source
            .named
            .get(field)
            .is_some_and(|value| !matches!(value, SourceValue::Boolean(false)))
        {
            return Err(unsupported(
                node.id,
                format!("unsupported source field {field}"),
            ));
        }
    }
    if !node.source.indexed.is_empty() {
        return Err(unsupported(
            node.id,
            "indexed node-record fields are unsupported",
        ));
    }
    for (selector, option) in &node.automatic_overrides {
        let class = snapshot
            .classes
            .values()
            .find(|class| class.name == *selector);
        let ascendancy = snapshot
            .ascendancies
            .values()
            .find(|asc| asc.name == *selector);
        if class.is_some() == ascendancy.is_some() {
            return Err(unsupported(
                node.id,
                format!("unknown or ambiguous automatic selector {selector}"),
            ));
        }
        if !option.indexed.is_empty() {
            return Err(unsupported(
                node.id,
                "indexed automatic override fields are unsupported",
            ));
        }
        for (field, value) in &option.named {
            match field.as_str() {
                "id" if unsigned_id(value).is_some() => {}
                "name" | "icon" if matches!(value, SourceValue::String(_)) => {}
                "stats" | "reminderText" if string_list(value) => {}
                "nodeOverlay" if display_overlay(value) => {}
                "ascendancyName" => {
                    let allowed = ascendancy.is_some_and(|asc| {
                        node.ascendancy_ids.contains(&asc.internal_id)
                            && matches!(value, SourceValue::String(name) if name == &asc.catalog_id)
                    });
                    if !allowed {
                        return Err(unsupported(
                            node.id,
                            "override changes unsupported ascendancy ownership",
                        ));
                    }
                }
                _ => {
                    return Err(unsupported(
                        node.id,
                        format!("unsupported effective overlay field {field}"),
                    ));
                }
            }
        }
        let (class_id, ascendancy_id) = match (class, ascendancy) {
            (Some(class), None) => (class.integer_id, None),
            (None, Some(ascendancy)) => {
                (ascendancy.class_id, Some(ascendancy.internal_id.as_str()))
            }
            _ => return Err(unsupported(node.id, "unresolved selector")),
        };
        snapshot
            .effective_node(class_id, ascendancy_id, node.id)
            .map_err(|error| {
                unsupported(node.id, format!("invalid effective override: {error}"))
            })?;
    }
    Ok(())
}
fn unsigned_id(value: &SourceValue) -> Option<u32> {
    match value {
        SourceValue::Integer(value) => u32::try_from(*value).ok(),
        SourceValue::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && *value >= 0.0
                && *value <= u32::MAX as f64 =>
        {
            Some(*value as u32)
        }
        _ => None,
    }
}
fn string_list(value: &SourceValue) -> bool {
    matches!(value, SourceValue::Table(SourceTable { named, indexed }) if named.is_empty()
        && indexed.keys().copied().eq(1..=indexed.len() as i64)
        && indexed.values().all(|value| matches!(value, SourceValue::String(_))))
}
fn display_overlay(value: &SourceValue) -> bool {
    matches!(value, SourceValue::Table(SourceTable { named, indexed }) if indexed.is_empty()
        && named.iter().all(|(key, value)| ["alloc", "path", "unalloc"].contains(&key.as_str()) && matches!(value, SourceValue::String(_))))
}
