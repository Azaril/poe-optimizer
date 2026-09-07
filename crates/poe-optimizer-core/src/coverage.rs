//! Observed skill resolution and selection after evaluation, not a legality certificate.
//!
//! Group and gem indices are one-based positions in the evaluated active skill set.
//! Catalog IDs are stable only within the evaluator's pinned data revision.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCoverage {
    pub schema_version: u32,
    pub active_skill_set_id: Option<u32>,
    pub groups: Vec<SkillGroupCoverage>,
    pub selected_player: Option<SelectedSkillContext>,
    pub selected_minion: Option<SelectedSkillContext>,
    pub full_dps: FullDpsCoverage,
    pub unresolved_entry_count: usize,
    pub tree_connections: Vec<TreeConnectionIssue>,
    /// Optional for saved coverage schema 1 reports produced before passive evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passives: Option<PassiveCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillGroupCoverage {
    pub index: usize,
    pub label: Option<String>,
    pub enabled: bool,
    pub slot: Option<String>,
    pub provenance: SkillProvenance,
    pub include_in_full_dps: bool,
    pub group_count: Option<f64>,
    pub main_active_skill: Option<usize>,
    pub gems: Vec<GemCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProvenance {
    pub kind: SkillOrigin,
    pub source: Option<String>,
    pub item_id: Option<u32>,
    pub item_name: Option<String>,
    pub node_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillOrigin {
    Manual,
    ItemGranted,
    TreeGranted,
    OtherGenerated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemCoverage {
    pub index: usize,
    pub name: Option<String>,
    /// Evaluator catalog key, which can differ from the imported game's gem ID.
    pub gem_id: Option<String>,
    pub gem_game_id: Option<String>,
    pub variant_id: Option<String>,
    pub skill_id: Option<String>,
    pub enabled: bool,
    pub count: Option<f64>,
    pub level: Option<f64>,
    pub quality: Option<f64>,
    pub is_support: Option<bool>,
    pub resolution: SkillResolution,
    pub diagnostic: Option<String>,
    pub hint: ResolutionHint,
    pub related_candidates: Vec<RelatedSkillCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillResolution {
    ResolvedGem,
    ResolvedGrantedEffect,
    Unresolved,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionHint {
    None,
    MinionActionNameMatch,
    SpectreNameMatch,
    AmbiguousSpectreNameMatch,
    NoCatalogNameMatch,
}

/// A catalog relationship to review; it does not resolve the imported entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedSkillCandidate {
    pub kind: RelatedSkillKind,
    pub minion_id: String,
    pub minion_name: String,
    pub skill_id: Option<String>,
    /// Position in the catalog skill list, not necessarily the runtime action index.
    pub catalog_action_index: Option<usize>,
    pub active_parent_groups: Vec<usize>,
    pub currently_selected_action: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelatedSkillKind {
    MinionAction,
    SpectreMonster,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedSkillContext {
    pub actor: SkillActor,
    pub skill_id: Option<String>,
    pub skill_name: Option<String>,
    /// For a minion action these identify the owning summoning skill's entry.
    pub group_index: Option<usize>,
    pub gem_index: Option<usize>,
    /// Position in this actor's evaluated active skill list.
    pub actor_skill_index: Option<usize>,
    pub minion_id: Option<String>,
    pub part_index: Option<usize>,
    pub part_name: Option<String>,
    pub stat_set_index: Option<usize>,
    pub stat_set_label: Option<String>,
    pub show_average: bool,
    /// Identifies upstream's fallback unarmed player action without a socket group.
    pub synthesized_default_attack: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillActor {
    Player,
    Minion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDpsCoverage {
    pub included_group_count: usize,
    pub selected_group_included: bool,
    /// MAIN skill/count metadata, not a claim that each entry contributed damage.
    pub active_skills: Vec<FullDpsSkillCoverage>,
    /// Rows returned by the evaluator's Full DPS calculation, without recomputation.
    pub reported_contributions: Vec<FullDpsContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDpsSkillCoverage {
    pub group_index: Option<usize>,
    pub gem_index: Option<usize>,
    pub skill_id: Option<String>,
    pub skill_name: Option<String>,
    pub included: bool,
    pub count: Option<f64>,
    pub count_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDpsContribution {
    pub name: Option<String>,
    pub dps: Option<f64>,
    pub count: Option<f64>,
    pub skill_part: Option<String>,
    pub trigger: Option<String>,
    pub source: Option<String>,
}

/// An upstream tree edge omitted during loading because its target is absent.
/// This concerns global loaded-tree topology, even when neither endpoint is allocated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeConnectionIssue {
    pub tree_version: String,
    pub source_node_id: u32,
    pub missing_target_id: u32,
    pub is_build_tree: bool,
    pub source_allocated: bool,
    pub missing_target_allocated: bool,
}

/// Owned observation of the evaluator's live allocated tree. This never certifies
/// requested-node preservation or legality; consumers compare their requested state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveCoverage {
    pub schema_version: u32,
    pub tree_version: String,
    pub class: PassiveClassCoverage,
    pub ascendancy: Option<PassiveAscendancyCoverage>,
    pub secondary_ascendancy: Option<PassiveAscendancyCoverage>,
    pub allocation_counts: PassiveAllocationCounts,
    /// Strictly increasing physical allocation IDs, including implicit roots.
    pub allocated_nodes: Vec<PassiveNodeCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveClassCoverage {
    pub index: u32,
    pub internal_id: u32,
    pub name: String,
    pub start_node_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveAscendancyCoverage {
    pub index: u32,
    /// Missing identity is preserved when a backend falls back from an invalid selection.
    pub internal_id: Option<String>,
    pub catalog_id: Option<String>,
    pub name: String,
    pub start_node_id: Option<u32>,
}

/// Direct observed CountAllocNodes buckets. Weapon counts overlap the other
/// categories; sockets are a subset, so these six values must not be added.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveAllocationCounts {
    pub ordinary: u32,
    pub ascendancy: u32,
    pub secondary_ascendancy: u32,
    pub sockets: u32,
    pub weapon_set_1: u32,
    pub weapon_set_2: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveNodeCoverage {
    pub physical_node_id: u32,
    pub node_type: String,
    /// Runtime node.name can retain its inherited base name after replacement.
    pub name: String,
    /// Runtime node.dn is the actual display name replaced by the pinned host.
    pub display_name: String,
    pub stats: Vec<String>,
    pub allocation_mode: u32,
    pub implicit_roots: Vec<PassiveRootRole>,
    /// None and Some(false) differ in the upstream allocation counter.
    pub free_allocation: Option<bool>,
    pub ascendancy_name: Option<String>,
    pub is_multiple_choice_option: bool,
    pub is_granted_passive: bool,
    pub is_attribute: bool,
    pub is_conquered: bool,
    pub has_hash_override: bool,
    pub is_switchable: bool,
    pub switch: Option<PassiveSwitchCoverage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassiveRootRole {
    Class,
    Ascendancy,
    SecondaryAscendancy,
}

/// Derived from the live reverse switch map, not an extracted source projection.
/// A later jewel/override may change stats after this automatic source selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveSwitchCoverage {
    pub selected_source_id: u32,
    pub kind: PassiveSwitchKind,
    pub selector: Option<String>,
    pub stats_reference_matches_selected_source: bool,
    pub display_name_matches_selected_source: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassiveSwitchKind {
    Base,
    Class,
    Ascendancy,
    Unclassified,
}

impl PassiveCoverage {
    /// Structural consistency only. Does not authenticate records or reconstruct
    /// game topology, switches, allocation costs, or progression legality.
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::BTreeSet;
        fn text(value: &str, field: &str) -> Result<(), String> {
            if value.trim().is_empty() || value.len() > 4096 {
                Err(format!(
                    "passive {field} must be nonblank and at most 4096 bytes"
                ))
            } else {
                Ok(())
            }
        }
        if self.schema_version != 1 {
            return Err("unsupported passive coverage schema version".into());
        }
        text(&self.tree_version, "tree_version")?;
        text(&self.class.name, "class.name")?;
        if self.allocated_nodes.is_empty() || self.allocated_nodes.len() > 20_000 {
            return Err("passive allocated node count must be 1..20000".into());
        }
        let count = self.allocated_nodes.len() as u32;
        let buckets = &self.allocation_counts;
        if [
            buckets.ordinary,
            buckets.ascendancy,
            buckets.secondary_ascendancy,
            buckets.sockets,
            buckets.weapon_set_1,
            buckets.weapon_set_2,
        ]
        .iter()
        .any(|value| *value > count)
        {
            return Err("passive allocation bucket exceeds the observed node count".into());
        }
        let mut expected_roots = vec![(self.class.start_node_id, PassiveRootRole::Class)];
        for (ascendancy, role) in [
            (&self.ascendancy, PassiveRootRole::Ascendancy),
            (
                &self.secondary_ascendancy,
                PassiveRootRole::SecondaryAscendancy,
            ),
        ] {
            if let Some(value) = ascendancy {
                if value.index == 0 {
                    return Err("passive ascendancy index must be positive when present".into());
                }
                text(&value.name, "ascendancy.name")?;
                for value in [&value.internal_id, &value.catalog_id]
                    .into_iter()
                    .flatten()
                {
                    text(value, "ascendancy identity")?;
                }
                if let Some(id) = value.start_node_id {
                    expected_roots.push((id, role));
                }
            }
        }
        let mut previous = None;
        let mut found_roots = BTreeSet::new();
        let mut text_bytes = 0usize;
        for node in &self.allocated_nodes {
            if previous.is_some_and(|id| id >= node.physical_node_id) {
                return Err("passive physical node IDs must be unique and sorted".into());
            }
            previous = Some(node.physical_node_id);
            text(&node.node_type, "node_type")?;
            text(&node.name, "name")?;
            text(&node.display_name, "display_name")?;
            if node.allocation_mode > 2 {
                return Err("passive allocation mode must be 0, 1, or 2".into());
            }
            if node.stats.len() > 1024 {
                return Err("passive stat line count exceeds 1024".into());
            }
            for stat in &node.stats {
                if stat.len() > 16 * 1024 {
                    return Err("passive stat line exceeds 16 KiB".into());
                }
                text_bytes = text_bytes.saturating_add(stat.len());
            }
            if text_bytes > 16 * 1024 * 1024 {
                return Err("passive stat text exceeds 16 MiB".into());
            }
            if let Some(name) = &node.ascendancy_name {
                text(name, "ascendancy_name")?;
            }
            let mut roles = BTreeSet::new();
            for role in &node.implicit_roots {
                if !roles.insert(*role) || !expected_roots.contains(&(node.physical_node_id, *role))
                {
                    return Err(
                        "passive implicit root role disagrees with selected identity".into(),
                    );
                }
                found_roots.insert((node.physical_node_id, *role));
            }
            if let Some(switch) = &node.switch {
                if !node.is_switchable {
                    return Err("passive switch evidence requires a switchable node".into());
                }
                match (&switch.kind, &switch.selector) {
                    (PassiveSwitchKind::Class | PassiveSwitchKind::Ascendancy, Some(selector)) => {
                        text(selector, "switch selector")?
                    }
                    (PassiveSwitchKind::Base | PassiveSwitchKind::Unclassified, None) => {}
                    _ => return Err("passive switch kind and selector disagree".into()),
                }
                if switch.kind == PassiveSwitchKind::Class
                    && switch.selector.as_deref() != Some(&self.class.name)
                {
                    return Err(
                        "passive class switch selector differs from the selected class".into(),
                    );
                }
                if switch.kind == PassiveSwitchKind::Ascendancy
                    && switch.selector.as_deref()
                        != self.ascendancy.as_ref().map(|value| value.name.as_str())
                {
                    return Err(
                        "passive ascendancy switch selector differs from the selected ascendancy"
                            .into(),
                    );
                }
            }
        }
        if expected_roots
            .iter()
            .any(|root| !found_roots.contains(root))
        {
            return Err("passive selected implicit root is not allocated".into());
        }
        Ok(())
    }
}
