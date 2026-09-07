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
