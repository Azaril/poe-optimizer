//! Authenticated, explicitly partial class/root/ordinary-entrance data for native hosts.
//! The artifact is generated from a freshly verified full snapshot. Runtime readers
//! authenticate its complete bytes against a compiled trusted digest, not its labels.
use crate::{
    tree_data::*,
    tree_projection::{AuthenticatedTreeSnapshot, TreeProjection},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
use thiserror::Error;

pub const BUNDLE_SCHEMA: u32 = 1;
pub const BUNDLE_POLICY: &str = "all_class_ascendancy_roots_and_ordinary_entrances_v1";
const BUNDLE_BYTES: &[u8] = include_bytes!("../data/class-tree.json");
const MAX_BUNDLE_BYTES: usize = 1024 * 1024;
#[derive(Debug, Clone, Error)]
#[error("invalid bundled class tree: {0}")]
pub struct BundleError(pub String);
type Result<T> = std::result::Result<T, BundleError>;
fn error(value: impl std::fmt::Display) -> BundleError {
    BundleError(value.to_string())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundledTreeCoverage {
    pub source_node_count: usize,
    pub retained_node_count: usize,
    pub excluded_node_count: usize,
    pub class_entrance_view_count: usize,
    /// Class-only plus class/ascendancy choices whose implicit roots have zero stats.
    pub no_effect_implicit_root_selections: usize,
    /// Source edges crossing from a retained node to an excluded one.
    pub boundary_edges: BTreeSet<(u32, u32)>,
    pub source_dangling_connections: BTreeSet<DanglingConnection>,
    pub source_unsupported_mechanics: BTreeSet<String>,
    pub limitations: BTreeSet<String>,
}
/// A subset, never a partial object masquerading as a complete TreeDataSnapshot.
/// Raw retained node adjacency may point outside this subset; coverage preserves
/// those omitted boundaries. Only class_entrances is an admitted native choice map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundledClassTree {
    pub schema_version: u32,
    pub policy: String,
    pub source: TreeSourceIdentity,
    pub full_snapshot_sha256: String,
    pub classes: BTreeMap<u32, TreeClass>,
    pub ascendancies: BTreeMap<String, TreeAscendancy>,
    pub roots: BTreeMap<u32, TreeNode>,
    pub ordinary_nodes: BTreeMap<u32, TreeNode>,
    pub class_entrances: BTreeMap<u32, BTreeMap<u32, EffectiveTreeNode>>,
    pub coverage: BundledTreeCoverage,
}
impl BundledClassTree {
    pub fn class(&self, class_id: u32) -> Result<&TreeClass> {
        self.classes
            .get(&class_id)
            .ok_or_else(|| error("unknown class integer ID"))
    }
    pub fn ascendancy(&self, class_id: u32, internal_id: &str) -> Result<&TreeAscendancy> {
        let value = self
            .ascendancies
            .get(internal_id)
            .ok_or_else(|| error("unknown ascendancy internal ID"))?;
        if value.class_id != class_id {
            return Err(error("ascendancy does not belong to selected class"));
        }
        Ok(value)
    }
    pub fn entrances(&self, class_id: u32) -> Result<&BTreeMap<u32, EffectiveTreeNode>> {
        self.class_entrances
            .get(&class_id)
            .ok_or_else(|| error("unknown class integer ID"))
    }
    pub fn entrance(&self, class_id: u32, node_id: u32) -> Result<&EffectiveTreeNode> {
        self.entrances(class_id)?
            .get(&node_id)
            .ok_or_else(|| error("node is not an admitted ordinary entrance for selected class"))
    }
    /// Deterministic generation only from a caller-authenticated complete source.
    /// The caller must preserve its trust boundary; provenance labels alone do not suffice.
    pub fn from_authenticated_snapshot(source: &AuthenticatedTreeSnapshot) -> Result<Self> {
        let snapshot = source.snapshot();
        if snapshot.classes.len() != 8
            || snapshot.ascendancies.len() != 23
            || snapshot.nodes.len() != 4914
        {
            return Err(error("unexpected pinned full snapshot cardinality"));
        }
        let indices: BTreeSet<_> = snapshot
            .classes
            .values()
            .map(|value| value.source_index)
            .collect();
        if indices != (1..=8).collect() {
            return Err(error(
                "class source-array indices are not a complete one-based permutation",
            ));
        }
        let mut class_entrances = BTreeMap::new();
        let mut paid = BTreeSet::new();
        let mut roots = BTreeMap::new();
        for class in snapshot.classes.values() {
            roots.insert(
                class.start_node_id,
                snapshot
                    .nodes
                    .get(&class.start_node_id)
                    .ok_or_else(|| error("missing class root"))?
                    .clone(),
            );
            let entrances = snapshot
                .ordinary_entrances(class.integer_id)
                .map_err(error)?;
            if entrances.len() != 2 {
                return Err(error(
                    "each pinned class must have exactly two ordinary entrances",
                ));
            }
            let mut views = BTreeMap::new();
            for node_id in entrances {
                let view = snapshot
                    .effective_node(class.integer_id, None, node_id)
                    .map_err(error)?;
                for ascendancy in &class.ascendancy_ids {
                    if snapshot
                        .effective_node(class.integer_id, Some(ascendancy), node_id)
                        .map_err(error)?
                        != view
                    {
                        return Err(error(
                            "an ascendancy changes an admitted class entrance; bundle policy must expand explicitly",
                        ));
                    }
                }
                views.insert(node_id, view);
                paid.insert(node_id);
            }
            class_entrances.insert(class.integer_id, views);
        }
        for asc in snapshot.ascendancies.values() {
            roots.insert(
                asc.start_node_id,
                snapshot
                    .nodes
                    .get(&asc.start_node_id)
                    .ok_or_else(|| error("missing ascendancy root"))?
                    .clone(),
            );
        }
        let projection = TreeProjection::new(source, paid.clone()).map_err(error)?;
        let retained_node_count = roots.len() + paid.len();
        let result = Self {
            schema_version: BUNDLE_SCHEMA,
            policy: BUNDLE_POLICY.into(),
            source: snapshot.identity.clone(),
            full_snapshot_sha256: source.content_sha256().into(),
            classes: snapshot.classes.clone(),
            ascendancies: snapshot.ascendancies.clone(),
            roots,
            ordinary_nodes: paid
                .into_iter()
                .map(|id| (id, snapshot.nodes[&id].clone()))
                .collect(),
            class_entrances,
            coverage: BundledTreeCoverage {
                source_node_count: snapshot.nodes.len(),
                retained_node_count,
                excluded_node_count: snapshot.nodes.len() - retained_node_count,
                class_entrance_view_count: 16,
                no_effect_implicit_root_selections: 31,
                boundary_edges: projection.source_coverage().boundary_edges.clone(),
                source_dangling_connections: snapshot.dangling_connections.clone(),
                source_unsupported_mechanics: snapshot.unsupported_mechanics.clone(),
                limitations: [
                    "strict_subset_not_full_tree",
                    "raw_adjacency_includes_excluded_endpoints",
                    "ordinary_entrances_only",
                    "no_allocated_ascendancy_effects",
                    "explicit_ordinary_point_budget_required",
                    "stats_are_exact_source_strings_not_generic_modifier_translation",
                    "source_views_do_not_certify_live_pointer_or_game_legality",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            },
        };
        result.validate_scope()?;
        Ok(result)
    }
    /// Validate the admitted subset's semantics. This does not authenticate arbitrary data.
    pub fn validate_scope(&self) -> Result<()> {
        if self.schema_version != BUNDLE_SCHEMA
            || self.policy != BUNDLE_POLICY
            || self.classes.len() != 8
            || self.ascendancies.len() != 23
            || self.class_entrances.len() != 8
            || self.coverage.class_entrance_view_count != 16
            || self.coverage.no_effect_implicit_root_selections != 31
            || self.coverage.retained_node_count != self.roots.len() + self.ordinary_nodes.len()
            || self.coverage.excluded_node_count + self.coverage.retained_node_count
                != self.coverage.source_node_count
        {
            return Err(error("subset scope/count metadata is inconsistent"));
        }
        let root_fields = [
            "ascendancyName",
            "classesStart",
            "connections",
            "group",
            "icon",
            "isAscendancyStart",
            "isSwitchable",
            "name",
            "nodeOverlay",
            "options",
            "orbit",
            "orbitIndex",
            "skill",
            "stats",
            "stringId",
        ];
        for root in self.roots.values() {
            if !matches!(
                root.kind,
                TreeNodeKind::ClassStart | TreeNodeKind::AscendancyStart
            ) || root.point_category != TreePointCategory::ImplicitRoot
                || root.source_default_point_cost != Some(0)
                || !root.stats.is_empty()
                || root
                    .source
                    .named
                    .keys()
                    .any(|key| !root_fields.contains(&key.as_str()))
            {
                return Err(error(format!(
                    "implicit root {} has stats, unsupported fields or point semantics",
                    root.id
                )));
            }
            crate::tree_projection::validate_node_metadata_for_catalog(
                &self.classes,
                &self.ascendancies,
                root,
            )
            .map_err(error)?;
        }
        for (class_id, class) in &self.classes {
            if class.integer_id != *class_id {
                return Err(error("class map key disagrees with integer ID"));
            }
            let root = self
                .roots
                .get(&class.start_node_id)
                .ok_or_else(|| error("class root absent from subset"))?;
            if !root.class_ids.contains(class_id) {
                return Err(error("class root owner missing"));
            }
            let views = self.entrances(*class_id)?;
            if views.len() != 2 {
                return Err(error("class requires exactly two entrance views"));
            }
            let mut selections = vec![None];
            for id in &class.ascendancy_ids {
                selections.push(Some(self.ascendancy(*class_id, id)?));
            }
            for ascendancy in selections {
                if !crate::tree_data::effective_source_node(class, ascendancy, root)
                    .map_err(error)?
                    .stats
                    .is_empty()
                {
                    return Err(error("effective class root adds unmodeled stats"));
                }
                if let Some(asc) = ascendancy {
                    let asc_root = self
                        .roots
                        .get(&asc.start_node_id)
                        .ok_or_else(|| error("ascendancy root absent from subset"))?;
                    if !asc_root.ascendancy_ids.contains(&asc.internal_id)
                        || !crate::tree_data::effective_source_node(class, Some(asc), asc_root)
                            .map_err(error)?
                            .stats
                            .is_empty()
                    {
                        return Err(error(
                            "effective ascendancy root adds stats or loses ownership",
                        ));
                    }
                }
                for (id, view) in views {
                    let node = self
                        .ordinary_nodes
                        .get(id)
                        .ok_or_else(|| error("entrance raw record missing"))?;
                    crate::tree_projection::validate_node_metadata_for_catalog(
                        &self.classes,
                        &self.ascendancies,
                        node,
                    )
                    .map_err(error)?;
                    if node.kind != TreeNodeKind::Normal
                        || node.point_category != TreePointCategory::Ordinary
                        || node.source_default_point_cost != Some(1)
                        || !root.adjacent.contains(id)
                        || !node.adjacent.contains(&root.id)
                        || &crate::tree_data::effective_source_node(class, ascendancy, node)
                            .map_err(error)?
                            != view
                    {
                        return Err(error(
                            "entrance effect, owner connection or normal-node semantics changed",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(error)
    }
    pub fn sha256(&self) -> Result<String> {
        Ok(hash(&self.canonical_bytes()?))
    }
}
/// Canonical artifact digest from the checked-in trusted manifest.
pub fn content_sha256() -> &'static str {
    include_str!("../data/class-tree.sha256").trim()
}
/// Authenticate arbitrary bytes only against the compiled expected bundle digest.
/// This never accepts a caller-supplied self-asserted source/digest label.
pub fn authenticate_bundle(bytes: &[u8]) -> Result<BundledClassTree> {
    if bytes.len() > MAX_BUNDLE_BYTES || hash(bytes) != content_sha256() {
        return Err(error(
            "bundle bytes differ from compiled trusted digest or exceed size limit",
        ));
    }
    let value: BundledClassTree = serde_json::from_slice(bytes).map_err(error)?;
    if value.schema_version != BUNDLE_SCHEMA
        || value.policy != BUNDLE_POLICY
        || value.source != expected_identity().map_err(error)?
    {
        return Err(error("bundle source, schema or policy is stale"));
    }
    value.validate_scope()?;
    Ok(value)
}
/// Once-parsed authenticated bundle; callers borrow records without snapshot cloning.
pub fn class_tree() -> Result<&'static BundledClassTree> {
    static BUNDLE: OnceLock<Result<BundledClassTree>> = OnceLock::new();
    BUNDLE
        .get_or_init(|| authenticate_bundle(BUNDLE_BYTES))
        .as_ref()
        .map_err(Clone::clone)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
