//! Source/definition-bound prerequisites for the general native preparation path.
//!
//! This is diagnostic evidence, not an admission token or a claim that selected
//! authored records are enabled/effective actors, skills, equipment or modifiers.
use poe_optimizer_core::{
    evaluation::{EvaluationError, EvaluationErrorKind},
    metrics::MetricQuery,
    options::EvaluationOptions,
};
use poe_optimizer_data::skill_identities::GemIdentityResolutionStatus;
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, SourceOccurrenceId, SourceRole},
    item_source::ItemSourceUse,
    selected_view::{DomainSelection, SelectedView, SelectedViewReport, SelectionProblem},
    skill_definitions::InstanceIdentityResolution,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const PREPARATION_REPORT_SCHEMA: u32 = 3;
const MAX_ISSUES: usize = 65_536;
const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationIssueKind {
    UnresolvedIdentity,
    AmbiguousIdentity,
    DeferredProducer,
    SourceError,
    Unsupported,
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparationIssue {
    pub kind: PreparationIssueKind,
    pub stage: &'static str,
    pub instance: Option<AuthoredInstanceId>,
    pub source: Option<SourceOccurrenceId>,
    pub message: String,
}
/// Requested diagnostic/evaluation inputs. These are neither effective scenario
/// outputs nor computed metric measurements.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreparationRequest {
    pub options: EvaluationOptions,
    pub metric_queries: Vec<MetricQuery>,
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparationReport {
    pub schema_version: u32,
    pub requested: PreparationRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored_skills: Option<crate::skills::SkillPreparationReport>,
    /// Loader-local state; this does not claim effective callbacks or root setup ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored_configuration: Option<crate::configuration::ConfigurationPreparationReport>,
    /// Independent authored selections and identity evidence, not completed LoadDB.
    pub view: SelectedViewReport,
    pub issues: Vec<PreparationIssue>,
    /// The current narrow calculation adapter's separate rejection, if any.
    /// A successful adapter does not establish the general producers below.
    pub legacy_adapter_error: Option<String>,
}

struct Issues {
    values: Vec<PreparationIssue>,
    records_left: usize,
    text_left: usize,
}
impl Issues {
    fn new(records: usize, text: usize) -> Self {
        Self {
            values: vec![],
            records_left: records,
            text_left: text,
        }
    }
    fn push(
        &mut self,
        kind: PreparationIssueKind,
        stage: &'static str,
        instance: Option<AuthoredInstanceId>,
        source: Option<SourceOccurrenceId>,
        message: &str,
    ) -> Result<(), EvaluationError> {
        self.records_left = self
            .records_left
            .checked_sub(1)
            .ok_or_else(|| resource("issue count"))?;
        self.text_left = self
            .text_left
            .checked_sub(message.len())
            .ok_or_else(|| resource("message bytes"))?;
        self.values.push(PreparationIssue {
            kind,
            stage,
            instance,
            source,
            message: message.into(),
        });
        Ok(())
    }
}
fn resource(limit: &'static str) -> EvaluationError {
    EvaluationError::new(
        EvaluationErrorKind::InvalidRequest,
        format!(
            "native preparation diagnostics exceed {limit} limit; no partial report was returned"
        ),
    )
}
fn invariant() -> EvaluationError {
    EvaluationError::new(
        EvaluationErrorKind::BackendContract,
        "selected preparation occurrence is absent from its bound imported build",
    )
}
fn repaired_problem(problem: &SelectionProblem, selections: &[&DomainSelection]) -> bool {
    selections.iter().any(|selection| {
        selection.override_instance.is_some()
            && selection.problem.is_none()
            && selection.selected.is_some()
            && selection
                .saved_selection_problem
                .as_ref()
                .is_some_and(|saved| {
                    saved.code == problem.code
                        && saved.source == problem.source
                        && saved.message == problem.message
                })
    })
}
fn problem_kind(problem: &SelectionProblem) -> PreparationIssueKind {
    if matches!(
        problem.code,
        "namespace_context" | "unavailable_override" | "shadowed_override"
    ) {
        PreparationIssueKind::Unsupported
    } else {
        PreparationIssueKind::SourceError
    }
}

/// Enumerate independent preparation prerequisites without invoking the narrow
/// legacy profile adapter. Work is bounded by imported/view collections; expanded
/// diagnostics additionally fail as a whole at 65,536 issues or 4 MiB of messages.
pub(crate) fn collect(
    view: &SelectedView<'_>,
    options: &EvaluationOptions,
    metrics: &[MetricQuery],
) -> Result<PreparationReport, EvaluationError> {
    collect_with_limits(view, options, metrics, MAX_ISSUES, MAX_MESSAGE_BYTES)
}

/// Incorporate only independently executed skill-loading facts. The privately
/// constructed stage carries its own ownership proof; reports cannot admit it.
pub(crate) fn collect_with_skills(
    view: &SelectedView<'_>,
    options: &EvaluationOptions,
    metrics: &[MetricQuery],
    skills: &crate::skills::PreparedSkills,
) -> Result<PreparationReport, EvaluationError> {
    use crate::skills::{SkillFailureKind, SkillIdentityStatus as S};
    use PreparationIssueKind as K;
    let mut result = collect(view, options, metrics)?;
    let selected: BTreeSet<_> = view
        .report()
        .skills
        .selected
        .iter()
        .flat_map(|set| &set.members)
        .copied()
        .collect();
    let stage = skills.report();
    for group in &stage.groups {
        if !selected.contains(&AuthoredInstanceId::SkillGroup(group.instance)) {
            continue;
        }
        for gem in &group.gems {
            if !gem.processed {
                continue;
            }
            let instance = AuthoredInstanceId::SkillEntry(gem.instance);
            result.issues.retain(|issue| {
                !(issue.instance == Some(instance)
                    && matches!(
                        issue.stage,
                        "skill_identity"
                            | "skill_primary_gem_owner"
                            | "skill_name_resolution"
                            | "skill_effect_producers"
                            | "support_compatibility"
                            | "tree_skill_provider"
                    ))
            });
            let entry_issue = match gem.identity_status {
                S::ResolvedGem | S::ResolvedEffect => Some((K::DeferredProducer, "skill_effect_producers",
                    "Authored identity and level processing completed. Effective stat sets, support application, provider availability and actor/action ownership still require preparation.".to_owned())),
                S::Empty => None,
                S::UnresolvedName => Some((K::UnresolvedIdentity, "skill_name_resolution",
                    gem.text("errMsg").unwrap_or("Source name lookup did not resolve a gem").to_owned())),
                S::AmbiguousName => Some((K::AmbiguousIdentity, "skill_name_resolution",
                    gem.text("errMsg").unwrap_or("Source name lookup has multiple matches").to_owned())),
                S::AmbiguousDefinition => Some((K::AmbiguousIdentity, "skill_definition",
                    "Definition construction has source-order-sensitive candidates without a portable unique owner.".to_owned())),
                S::HiddenGem => Some((K::SourceError, "skill_visibility",
                    gem.text("errMsg").unwrap_or("Source processing rejects this gem as an active skill").to_owned())),
                S::UnresolvedDefinition => Some((K::UnresolvedIdentity, "skill_definition",
                    "Source processing did not obtain an available gem/effect definition.".to_owned())),
                S::NotProcessed => return Err(invariant()),
            };
            if let Some((kind, stage, message)) = entry_issue {
                result.issues.push(PreparationIssue {
                    kind,
                    stage,
                    instance: Some(instance),
                    source: Some(gem.source),
                    message,
                });
            }
        }
        if group.attached {
            for issue in &mut result.issues {
                if issue.instance == Some(AuthoredInstanceId::SkillGroup(group.instance))
                    && issue.stage == "skill_group_producers"
                {
                    issue.message = "The authored group was processed and attached by skill loading. Effective support/global effects and actor/action ownership remain pending; loading does not prove activation.".into();
                }
            }
        }
    }
    if let Some(failure) = &stage.failure {
        result.issues.push(PreparationIssue {
            kind: match failure.kind {
                SkillFailureKind::SourceRuntime => K::SourceError,
                SkillFailureKind::UnsupportedSource => K::Unsupported,
                SkillFailureKind::AmbiguousDefinition => K::AmbiguousIdentity,
            },
            stage: failure.stage,
            instance: failure.instance,
            source: failure.source,
            message: failure.message.clone(),
        });
    }
    if result.issues.len() > MAX_ISSUES {
        return Err(resource("issue count"));
    }
    let mut bytes_left = MAX_MESSAGE_BYTES;
    for issue in &result.issues {
        bytes_left = bytes_left
            .checked_sub(issue.message.len())
            .ok_or_else(|| resource("message bytes"))?;
    }
    result.authored_skills = Some(stage.clone());
    Ok(result)
}

/// Attach the independently prepared configuration prefix without retiring its
/// pending activation or the root/provider prerequisites.
pub(crate) fn collect_with_stages(
    view: &SelectedView<'_>,
    options: &EvaluationOptions,
    metrics: &[MetricQuery],
    skills: &crate::skills::PreparedSkills,
    configuration: &crate::configuration::PreparedConfiguration,
) -> Result<PreparationReport, EvaluationError> {
    use crate::configuration::ConfigurationPrefixStatus;
    let mut result = collect_with_skills(view, options, metrics, skills)?;
    let stage = configuration.report();
    if stage.status == ConfigurationPrefixStatus::Prepared {
        for issue in &mut result.issues {
            if issue.stage == "configuration_effects" {
                issue.message = "The first configuration loader prefix has prepared defaults, authored writes and compatibility migrations. Activation controls, callbacks, effective enemy/scenario state and later configuration sections remain pending; this is not completed root setup or effective configuration.".into();
            }
        }
    }
    if let Some(failure) = &stage.failure {
        result.issues.push(PreparationIssue {
            kind: if stage.status == ConfigurationPrefixStatus::SourceFailure {
                PreparationIssueKind::SourceError
            } else {
                PreparationIssueKind::Unsupported
            },
            stage: failure.stage,
            instance: None,
            source: failure.source,
            message: failure.message.clone(),
        });
    }
    if result.issues.len() > MAX_ISSUES {
        return Err(resource("issue count"));
    }
    let mut bytes_left = MAX_MESSAGE_BYTES;
    for issue in &result.issues {
        bytes_left = bytes_left
            .checked_sub(issue.message.len())
            .ok_or_else(|| resource("message bytes"))?;
    }
    result.authored_configuration = Some(stage.clone());
    Ok(result)
}

fn collect_with_limits(
    view: &SelectedView<'_>,
    options: &EvaluationOptions,
    metrics: &[MetricQuery],
    max_issues: usize,
    max_text: usize,
) -> Result<PreparationReport, EvaluationError> {
    use PreparationIssueKind as K;
    let report = view.report();
    let build = view.build();
    let membership: BTreeMap<_, _> = build
        .instances()
        .iter()
        .map(|binding| (binding.source(), binding.instance()))
        .collect();
    let selections = [
        &report.skills,
        &report.items,
        &report.passives,
        &report.configuration,
    ];
    let mut issues = Issues::new(max_issues, max_text);
    let mut seen_problems = BTreeSet::new();
    for problem in report.load_problems.iter().chain(
        selections
            .iter()
            .filter_map(|selection| selection.problem.as_ref()),
    ) {
        if repaired_problem(problem, &selections)
            || !seen_problems.insert((problem.code, problem.source, problem.message.as_str()))
        {
            continue;
        }
        issues.push(
            problem_kind(problem),
            "source_selection",
            problem
                .source
                .and_then(|source| membership.get(&source).copied()),
            problem.source,
            &problem.message,
        )?;
    }
    if let Some(problem) = &report.skill_identities.problem {
        issues.push(
            K::Unsupported,
            "skill_identity_projection",
            report
                .skills
                .selected
                .as_ref()
                .and_then(|set| set.origin.instance()),
            report.skills.container,
            problem,
        )?;
    }
    for entry in &report.skill_identities.entries {
        let (kind, stage, message) = match &entry.resolution {
            InstanceIdentityResolution::ExternalGem {
                status: GemIdentityResolutionStatus::Missing,
                ..
            } => (
                K::UnresolvedIdentity,
                "skill_identity",
                "The authored external gem ID has no matching definition. A supplied skill ID does not replace this source lookup branch.",
            ),
            InstanceIdentityResolution::ExternalGem {
                status: GemIdentityResolutionStatus::Ambiguous,
                ..
            } => (
                K::AmbiguousIdentity,
                "skill_identity",
                "The external gem lookup has multiple candidates; an effective variant winner has not been established.",
            ),
            InstanceIdentityResolution::ExternalGem { .. } => (
                K::DeferredProducer,
                "skill_effect_producers",
                "Resolved gem identity still requires effect levels/stat sets, support applicability, enabled-state processing and actor/action construction.",
            ),
            InstanceIdentityResolution::ExplicitEffect { matched: None, .. } => (
                K::UnresolvedIdentity,
                "skill_identity",
                "The authored effect ID has no matching definition; name-based fallback processing has not established an effective skill.",
            ),
            InstanceIdentityResolution::ExplicitEffect {
                possible_primary_gem_keys,
                ..
            } if possible_primary_gem_keys.len() > 1 => (
                K::AmbiguousIdentity,
                "skill_primary_gem_owner",
                "The effect identity is known but has multiple possible primary gem owners; original construction order must establish the gemForSkill winner.",
            ),
            InstanceIdentityResolution::ExplicitEffect {
                matched: Some(effect),
                ..
            } if effect.support == Some(true) => (
                K::DeferredProducer,
                "support_compatibility",
                "The definition identifies a support; its applicability, restrictions and interactions with the selected group require support producers.",
            ),
            InstanceIdentityResolution::ExplicitEffect {
                matched: Some(effect),
                ..
            } if effect.from_tree == Some(true) => (
                K::DeferredProducer,
                "tree_skill_provider",
                "The definition identifies a tree-derived effect; passive providers and effective actor/action construction have not established its availability.",
            ),
            InstanceIdentityResolution::ExplicitEffect { .. } => (
                K::DeferredProducer,
                "skill_effect_producers",
                "Resolved effect identity still requires levels/stat sets, enabled-state processing, support applicability and actor/action construction.",
            ),
            InstanceIdentityResolution::NameOnlyNotResolved => (
                K::DeferredProducer,
                "skill_name_resolution",
                "This authored name requires the original name-matching and socket-group processing rules; catalog absence has not been established.",
            ),
            InstanceIdentityResolution::MissingIdentity => (
                K::UnresolvedIdentity,
                "skill_identity",
                "The authored entry supplies no resolvable gem/effect/name identity. Socket-group processing must establish whether it is an empty placeholder or an effective entry.",
            ),
        };
        issues.push(
            kind,
            stage,
            Some(entry.instance),
            Some(entry.source),
            message,
        )?;
    }
    if let Some(set) = &report.skills.selected {
        for &instance in &set.members {
            let source = build.binding(instance).map_err(|_| invariant())?.source();
            issues.push(K::DeferredProducer, "skill_group_producers", Some(instance), Some(source), "This group belongs to the selected authored set. Its effective entries, support interactions, global effects and actor/action ownership still require producer execution; authored membership does not prove activation.")?;
        }
    }
    if let Some(set) = &report.items.selected {
        issues.push(K::DeferredProducer, "item_registration", set.origin.instance(), set.origin.source(), "Inventory records require ParseRaw, base validation and item assembly before source-ID registration and duplicate winners are known. The selected set does not prove which inventory records become equipped.")?;
        for &instance in &set.members {
            let source = build.binding(instance).map_err(|_| invariant())?.source();
            let occurrence = build.occurrence(source).map_err(|_| invariant())?;
            let (stage, message) = match occurrence.role() {
                SourceRole::Item {
                    usage: ItemSourceUse::CharacterRuneSlot,
                    ..
                } => (
                    "character_rune_assignment",
                    "This authored rune-slot use requires rune definition resolution and character-slot effects before it becomes effective.",
                ),
                SourceRole::Item {
                    usage: ItemSourceUse::EquipmentSlot | ItemSourceUse::LegacyEquipmentSlot,
                    ..
                } => (
                    "equipment_assignment",
                    "This authored slot use requires registered item resolution, slot validity, weapon-set participation and activation rules. Its raw item reference is not proof of an equipped item.",
                ),
                _ => return Err(invariant()),
            };
            issues.push(
                K::DeferredProducer,
                stage,
                Some(instance),
                Some(source),
                message,
            )?;
        }
    }
    if let Some(set) = &report.passives.selected {
        issues.push(K::DeferredProducer, "passive_version", set.origin.instance(), set.origin.source(), "The selected authored spec requires tree-version validation, migrations and class/ascendancy start resolution against injected definitions.")?;
        issues.push(K::DeferredProducer, "passive_allocation", set.origin.instance(), set.origin.source(), "Passive connectivity, allocation budgets, separate ascendancy points, mastery choices and passive providers have not been prepared for this selected spec.")?;
        for &instance in &set.members {
            let source = build.binding(instance).map_err(|_| invariant())?.source();
            let occurrence = build.occurrence(source).map_err(|_| invariant())?;
            if !matches!(
                occurrence.role(),
                SourceRole::Item {
                    usage: ItemSourceUse::JewelAssignment,
                    ..
                }
            ) {
                return Err(invariant());
            }
            issues.push(K::DeferredProducer, "passive_jewel_assignment", Some(instance), Some(source), "This selected-spec jewel use requires registered item resolution, socket allocation, radius/provider effects and assignment lifecycle processing before any item is effective.")?;
        }
    }
    if let Some(set) = &report.configuration.selected {
        issues.push(K::DeferredProducer, "configuration_effects", set.origin.instance(), set.origin.source(), "Configuration defaults, placeholder precedence, migrations and modifier generation must run against the injected definitions. The selected authored set alone is not an effective scenario.")?;
    }
    for frontier in &report.frontiers {
        // The concrete selected records above explain these stages with their
        // instances. Retain root/namespace and any future unmatched frontiers.
        if matches!(
            frontier.stage,
            "skill_processing" | "item_preparation" | "passive_loading" | "configuration_effects"
        ) {
            continue;
        }
        issues.push(
            if frontier.stage == "namespace_context" {
                K::Unsupported
            } else {
                K::DeferredProducer
            },
            frontier.stage,
            frontier
                .source
                .and_then(|source| membership.get(&source).copied()),
            frontier.source,
            frontier.reason,
        )?;
    }
    Ok(PreparationReport {
        schema_version: PREPARATION_REPORT_SCHEMA,
        requested: PreparationRequest {
            options: options.clone(),
            metric_queries: metrics.to_vec(),
        },
        authored_skills: None,
        authored_configuration: None,
        view: report.clone(),
        issues: issues.values,
        legacy_adapter_error: None,
    })
}

#[cfg(test)]
mod tests {
    mod preparation_edits {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/support/skill_preparation_edits.rs"
        ));
    }
    use super::*;
    use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
    use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
    use poe_optimizer_import::{
        build_instance::{ImportedBuildInstance, InstanceImportLimits},
        decode_build,
        selected_view::{ResolveLimits, resolve_view},
    };
    use std::sync::OnceLock;

    fn collect(view: &SelectedView<'_>) -> Result<PreparationReport, EvaluationError> {
        super::collect(view, &EvaluationOptions::default(), &[])
    }
    fn collect_with_limits(
        view: &SelectedView<'_>,
        max_issues: usize,
        max_text: usize,
    ) -> Result<PreparationReport, EvaluationError> {
        super::collect_with_limits(
            view,
            &EvaluationOptions::default(),
            &[],
            max_issues,
            max_text,
        )
    }
    fn data() -> &'static GameDataSnapshot {
        static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
        DATA.get_or_init(|| bundled_snapshot().unwrap())
    }
    fn imported(sections: &str) -> ImportedBuildInstance {
        let xml = format!("<PathOfBuilding2>{sections}</PathOfBuilding2>");
        ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([35; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap()
    }
    fn report(sections: &str) -> PreparationReport {
        let build = imported(sections);
        let view = resolve_view(
            &build,
            data(),
            &ViewRequest::default(),
            ResolveLimits::default(),
        )
        .unwrap();
        collect(&view).unwrap()
    }
    #[test]
    fn selected_instances_have_independent_prerequisites_without_inventory_admission() {
        let build = imported(
            "<Skills activeSkillSet='2'><SkillSet id='1'><Skill><Gem gemId='inactive-missing'/></Skill></SkillSet><SkillSet id='2'><Skill><Gem gemId='missing' skillId='FireballPlayer'/><Gem nameSpec='provided name'/></Skill></SkillSet></Skills><Items><Item id='3'>unknown inventory</Item><ItemSet id='1'><Slot name='Weapon 1' itemId='3'/><RuneSlot name='rune' runeId='unknown'/></ItemSet></Items><Tree><Spec treeVersion='0_5'><Sockets><Socket nodeId='1' itemId='3'/></Sockets></Spec></Tree><Config/>",
        );
        let view = resolve_view(
            &build,
            data(),
            &ViewRequest::default(),
            ResolveLimits::default(),
        )
        .unwrap();
        let result = collect(&view).unwrap();
        let stages: BTreeSet<_> = result.issues.iter().map(|issue| issue.stage).collect();
        for stage in [
            "skill_identity",
            "skill_name_resolution",
            "skill_group_producers",
            "item_registration",
            "equipment_assignment",
            "character_rune_assignment",
            "passive_version",
            "passive_allocation",
            "passive_jewel_assignment",
            "configuration_effects",
        ] {
            assert!(stages.contains(stage), "{stage}");
        }
        assert_eq!(
            result
                .issues
                .iter()
                .filter(|issue| issue.kind == PreparationIssueKind::UnresolvedIdentity)
                .count(),
            1
        );
        assert!(
            !result
                .issues
                .iter()
                .any(|issue| matches!(issue.instance, Some(AuthoredInstanceId::ItemRecord(_))))
        );
        for issue in &result.issues {
            if let Some(instance) = issue.instance {
                assert_eq!(
                    Some(build.binding(instance).unwrap().source()),
                    issue.source
                );
            }
        }
        assert_eq!(result.view.source_sha256, build.source_sha256());
        assert_eq!(result.view.data, *data().identity());
        assert!(result.legacy_adapter_error.is_none());
    }
    #[test]
    fn identity_absence_and_source_failure_have_different_issue_kinds() {
        let result = report(
            "<Skills><SkillSet id='1'><Skill><Gem skillId='definitely-unknown'/><Gem/></Skill></SkillSet></Skills><Config><ConfigSet id='nan'/></Config>",
        );
        assert_eq!(
            result
                .issues
                .iter()
                .filter(|issue| issue.kind == PreparationIssueKind::UnresolvedIdentity)
                .count(),
            2
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::SourceError
                    && issue.stage == "source_selection")
        );
        assert!(
            !result
                .issues
                .iter()
                .any(|issue| issue.stage == "configuration_effects")
        );
    }
    #[test]
    fn known_effects_require_producers_instead_of_claiming_calculation_support() {
        let effect = data()
            .package()
            .skill_identities
            .skills
            .iter()
            .find(|effect| effect.support == Some(true))
            .unwrap();
        let result = report(&format!(
            "<Skills><Skill><Gem skillId='{}'/></Skill></Skills>",
            effect.id
        ));
        assert!(result.issues.iter().any(|issue| matches!(
            issue.stage,
            "support_compatibility" | "skill_primary_gem_owner"
        )));
        assert!(
            !result
                .issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::UnresolvedIdentity)
        );
    }
    #[test]
    fn namespace_frontiers_and_report_limits_do_not_masquerade_as_source_errors() {
        let result = report("<Skills xmlns='urn:unprepared'><Skill/></Skills>");
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::Unsupported
                    && issue.stage == "namespace_context")
        );
        assert!(
            !result
                .issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::SourceError)
        );
        let build = imported("");
        let view = resolve_view(
            &build,
            data(),
            &ViewRequest::default(),
            ResolveLimits::default(),
        )
        .unwrap();
        let full = collect(&view).unwrap();
        let exact_records = full.issues.len();
        let exact_text = full
            .issues
            .iter()
            .map(|issue| issue.message.len())
            .sum::<usize>();
        assert_eq!(
            collect_with_limits(&view, exact_records, exact_text)
                .unwrap()
                .issues
                .len(),
            exact_records
        );
        for (records, text) in [
            (0, MAX_MESSAGE_BYTES),
            (MAX_ISSUES, 0),
            (exact_records - 1, exact_text),
            (exact_records, exact_text - 1),
        ] {
            let error = collect_with_limits(&view, records, text).unwrap_err();
            assert_eq!(error.kind, EvaluationErrorKind::InvalidRequest);
            assert!(error.message.contains("no partial report"));
        }
    }
    #[test]
    fn injected_catalog_ambiguity_changes_bound_diagnostics_without_changing_source() {
        use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
        let mut package = data().package().clone();
        for gem in &mut package.skill_identities.gems[..2] {
            gem.game_id = "Injected/Preparation/Ambiguous".into();
            gem.variant_id = "shared".into();
            let declaration = &mut package.skill_identities.gem_declarations
                [gem.winning_declaration as usize - 1];
            declaration.identity.game_id = gem.game_id.clone();
            declaration.identity.variant_id = gem.variant_id.clone();
        }
        preparation_edits::refresh_lookups(&mut package);
        package.refresh_section_digests().unwrap();
        let injected = GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap();
        let build = imported(
            "<Skills><Skill><Gem gemId='Injected/Preparation/Ambiguous' variantId='shared'/></Skill></Skills>",
        );
        let base = collect(
            &resolve_view(
                &build,
                data(),
                &ViewRequest::default(),
                ResolveLimits::default(),
            )
            .unwrap(),
        )
        .unwrap();
        let custom = collect(
            &resolve_view(
                &build,
                &injected,
                &ViewRequest::default(),
                ResolveLimits::default(),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(
            base.issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::UnresolvedIdentity)
        );
        assert!(
            custom
                .issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::AmbiguousIdentity)
        );
        assert!(
            !custom
                .issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::UnresolvedIdentity)
        );
        assert_eq!(base.view.source_sha256, custom.view.source_sha256);
        assert_ne!(
            base.view.data.content_sha256,
            custom.view.data.content_sha256
        );
    }
    #[test]
    fn successful_explicit_selector_repair_preserves_audit_without_blocking_issue() {
        use poe_optimizer_core::build_view::SelectionRequest;
        let build = imported("<Tree activeSpec='-1'><Spec treeVersion='0_5'/></Tree>");
        let id = build
            .instances()
            .iter()
            .find_map(|binding| match binding.instance() {
                AuthoredInstanceId::PassiveSpec(id) => Some(id),
                _ => None,
            })
            .unwrap();
        let request = ViewRequest {
            passives: SelectionRequest::Instance(id),
            ..ViewRequest::default()
        };
        let result =
            collect(&resolve_view(&build, data(), &request, ResolveLimits::default()).unwrap())
                .unwrap();
        assert!(result.view.passives.saved_selection_problem.is_some());
        assert!(!result.view.load_problems.is_empty());
        assert!(
            !result
                .issues
                .iter()
                .any(|issue| issue.kind == PreparationIssueKind::SourceError)
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.stage == "passive_allocation")
        );
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn incomplete_public_native_preparation_retains_requested_options_and_metric_queries() {
        use poe_optimizer_core::{
            evaluation::{BuildDocument, BuildFormat, EvaluationRequest},
            metrics::{ActorScope, MetricQuery},
            options::{BossKind, DamageAmounts, EncounterOverrides, SkillSelection},
        };
        let backend = crate::NativeBackend::new();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/builds/breadth-20260908/build-02.xml");
        let mut request = EvaluationRequest {
            build: BuildDocument {
                format: BuildFormat::PathOfBuilding2Xml,
                content: std::fs::read_to_string(path).unwrap(),
            },
            options: EvaluationOptions {
                selection: Some(SkillSelection {
                    socket_group: 2,
                    active_skill: Some(3),
                    minion_skill: Some(1),
                }),
                encounter: Some(EncounterOverrides {
                    name: "Caller diagnostics".into(),
                    enemy_level: Some(84),
                    boss: Some(BossKind::Pinnacle),
                    incoming_hit: Some(DamageAmounts {
                        physical: 1234.5,
                        fire: 67.25,
                        ..DamageAmounts::default()
                    }),
                }),
            },
            metrics: vec![
                MetricQuery {
                    actor: ActorScope::Player,
                    id: "selected_hit_dps".into(),
                },
                MetricQuery {
                    actor: ActorScope::Player,
                    id: "life".into(),
                },
            ],
        };
        let expected = PreparationRequest {
            options: request.options.clone(),
            metric_queries: request.metrics.clone(),
        };
        let crate::PreparationOutcome::Incomplete(report) = backend
            .prepare_request_with_lineage(&request, BuildLineage::from_bytes([42; 16]))
            .unwrap()
        else {
            panic!("original build must remain incomplete");
        };
        assert_eq!(report.requested, expected);
        assert!(!report.issues.is_empty());
        assert!(report.legacy_adapter_error.is_some());
        request.options.encounter.as_mut().unwrap().name = "later request".into();
        request.metrics.reverse();
        assert_eq!(
            report.requested, expected,
            "report owns the original request context"
        );
        let serialized = serde_json::to_value(&report).unwrap();
        assert_eq!(
            serialized["requested"],
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(
            serialized["requested"]["metric_queries"][0]["id"],
            "selected_hit_dps"
        );
        assert!(serialized.get("metrics").is_none());
        assert!(serialized.get("measurements").is_none());
        assert!(
            serialized["requested"]["metric_queries"][0]
                .get("value")
                .is_none()
        );
    }
    #[test]
    fn all_five_originals_have_bound_general_prerequisites() {
        for index in 1..=5 {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../tests/fixtures/builds/breadth-20260908/build-{index:02}.xml"
            ));
            let build = ImportedBuildInstance::from_decoded(
                decode_build(&std::fs::read(path).unwrap()).unwrap(),
                BuildLineage::from_bytes([index; 16]),
                InstanceImportLimits::default(),
            )
            .unwrap();
            let view = resolve_view(
                &build,
                data(),
                &ViewRequest::default(),
                ResolveLimits::default(),
            )
            .unwrap();
            let result = collect(&view).unwrap();
            for stage in [
                "skill_group_producers",
                "item_registration",
                "equipment_assignment",
                "passive_allocation",
                "configuration_effects",
            ] {
                assert!(
                    result.issues.iter().any(|issue| issue.stage == stage),
                    "build{index}:{stage}"
                );
            }
            assert!(
                result
                    .issues
                    .iter()
                    .any(|issue| issue.instance.is_some() && issue.source.is_some())
            );
            assert_eq!(result.view.source_sha256, build.source_sha256());
            eprintln!(
                "build {index}: {} selected identity entries, {} preparation issues",
                result.view.skill_identities.entries.len(),
                result.issues.len()
            );
        }
    }
}
