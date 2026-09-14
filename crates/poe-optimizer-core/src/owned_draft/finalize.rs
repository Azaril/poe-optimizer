//! Selected draft projection. Complete constructors remain the final boundary.
use super::*;
use crate::{build_identity::*, owned_build::*, owned_content::*, owned_project::*};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

/// A complete structural request or the exact selected uncertainty that blocks it.
/// Pending queries retain every authored row and never acquire a substitute target.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DraftFinalization {
    Ready(Box<FinalizedDraft>),
    Pending {
        draft_digest: OwnedContentDigest,
        selection: Box<EvaluationSelection>,
        issues: Vec<DraftIssue>,
        queries: QueryDraft,
    },
}
/// Private construction binds the request to the exact selected authoring snapshot.
/// This is structural provenance, not a definition, legality or calculation result.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FinalizedDraft {
    draft_digest: OwnedContentDigest,
    selection: EvaluationSelection,
    request_digest: OwnedContentDigest,
    request: OwnedEvaluationRequest,
}
impl FinalizedDraft {
    pub fn draft_digest(&self) -> OwnedContentDigest {
        self.draft_digest
    }
    pub fn selection(&self) -> EvaluationSelection {
        self.selection
    }
    pub fn request_digest(&self) -> OwnedContentDigest {
        self.request_digest
    }
    pub fn request(&self) -> &OwnedEvaluationRequest {
        &self.request
    }
    pub fn into_request(self) -> OwnedEvaluationRequest {
        self.request
    }
}
#[derive(Debug)]
pub enum FinalizationError {
    Structure(StructuralError),
    Project(ProjectError),
    Digest(ContentDigestError),
    IncompleteWithoutIssue { path: &'static str },
}
impl fmt::Display for FinalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(e) => e.fmt(f),
            Self::Project(e) => e.fmt(f),
            Self::Digest(e) => e.fmt(f),
            Self::IncompleteWithoutIssue { path } => {
                write!(f, "{path}: incomplete draft has no selected issue")
            }
        }
    }
}
impl std::error::Error for FinalizationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(e) => Some(e),
            Self::Project(e) => Some(e),
            Self::Digest(e) => Some(e),
            _ => None,
        }
    }
}
impl From<StructuralError> for FinalizationError {
    fn from(e: StructuralError) -> Self {
        Self::Structure(e)
    }
}
impl From<ProjectError> for FinalizationError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}
impl From<ContentDigestError> for FinalizationError {
    fn from(e: ContentDigestError) -> Self {
        Self::Digest(e)
    }
}

fn index<T, I: Ord>(rows: &[T], key: impl Fn(&T) -> I) -> BTreeMap<I, &T> {
    rows.iter().map(|row| (key(row), row)).collect()
}
fn selected<'a, T, I: BuildInstanceId + Ord>(
    rows: &BTreeMap<I, &'a T>,
    id: I,
    kind: OccurrenceKind,
    path: &str,
    allocator: InstanceAllocatorState,
) -> Result<&'a T, FinalizationError> {
    let instance = id.instance_id();
    let error = if instance.lineage() != allocator.lineage() {
        Some(StructuralErrorKind::ForeignLineage)
    } else if instance.local() > allocator.last_issued() {
        Some(StructuralErrorKind::BeyondWatermark)
    } else if !rows.contains_key(&id) {
        Some(StructuralErrorKind::MissingReference {
            expected: kind,
            id: instance,
        })
    } else {
        None
    };
    if let Some(kind) = error {
        return Err(StructuralError {
            path: path.into(),
            kind,
        }
        .into());
    }
    Ok(rows[&id])
}
fn resolved<T: ResolveDraft>(
    value: &T,
    path: &'static str,
) -> Result<T::Resolved, FinalizationError> {
    value
        .to_resolved()
        .ok_or(FinalizationError::IncompleteWithoutIssue { path })
}
fn resolved_rows<T: ResolveDraft>(
    rows: &[&T],
    path: &'static str,
) -> Result<Vec<T::Resolved>, FinalizationError> {
    rows.iter().map(|row| resolved(*row, path)).collect()
}
impl DraftSession {
    /// Select explicit independent presets. Only backing item/gem records are
    /// followed automatically; omitted providers are never selected from another
    /// preset. Disabled or off-loadout selected rows still need complete inputs.
    /// Global registry closures and unselected alternatives remain in this session.
    /// Inventory adoption/copy assignment is a separate operation on the result.
    pub fn finalize_selection(
        &self,
        selection: EvaluationSelection,
        limits: DraftLimits,
    ) -> Result<DraftFinalization, FinalizationError> {
        let validation = self.validate_limits(limits)?;
        let input = self.input();
        let allocator = input.allocator;
        let draft_digest = self.digest(limits.input.max_wire_bytes)?;
        let mut selected_owners = BTreeSet::new();
        macro_rules! preset {
            ($table:ident,$id:expr,$kind:ident,$path:literal) => {{
                let rows = index(&input.$table.members, |row| row.id);
                let row = selected(&rows, $id, OccurrenceKind::$kind, $path, allocator)?;
                selected_owners.insert(row.id.instance_id());
                row
            }};
        }
        let character = preset!(
            character_presets,
            selection.build.character,
            CharacterPreset,
            "selection.character"
        );
        let equipment_preset = preset!(
            equipment_presets,
            selection.build.equipment,
            EquipmentPreset,
            "selection.equipment"
        );
        let allocation_preset = preset!(
            allocation_presets,
            selection.build.allocations,
            AllocationPreset,
            "selection.allocations"
        );
        let skill_preset = preset!(
            skill_presets,
            selection.build.skills,
            SkillPreset,
            "selection.skills"
        );
        let choice_preset = preset!(
            choice_presets,
            selection.build.choices,
            ChoicePreset,
            "selection.choices"
        );
        let scenario = preset!(
            scenario_presets,
            selection.scenario,
            ScenarioPreset,
            "selection.scenario"
        );
        let queries = preset!(
            query_presets,
            selection.queries,
            QueryPreset,
            "selection.queries"
        );
        let loadouts = index(&input.weapon_loadouts.members, |id| *id);
        selected(
            &loadouts,
            selection.build.active_weapon_loadout,
            OccurrenceKind::Loadout,
            "selection.active_weapon_loadout",
            allocator,
        )?;
        macro_rules! rows {
            ($table:ident,$ids:expr,$kind:ident) => {{
                let by_id = index(&input.$table.members, |row| row.id);
                let mut rows = Vec::new();
                for id in $ids {
                    let row = selected(
                        &by_id,
                        *id,
                        OccurrenceKind::$kind,
                        stringify!($table),
                        allocator,
                    )?;
                    selected_owners.insert(row.id.instance_id());
                    rows.push(row);
                }
                rows
            }};
        }
        let rewards = rows!(rewards, &character.rewards.members, Reward);
        let equipment = rows!(equipment, &equipment_preset.equipment.members, EquipmentUse);
        let allocations = rows!(
            allocations,
            &allocation_preset.allocations.members,
            Allocation
        );
        let skills = rows!(skills, &skill_preset.skills.members, SkillUse);
        let supports = rows!(supports, &skill_preset.supports.members, SupportAssignment);
        let payload_links = rows!(
            payload_links,
            &skill_preset.payload_links.members,
            PayloadLink
        );
        // Unknown receiving records block their owner; candidates are never followed.
        let item_ids: BTreeSet<_> = equipment
            .iter()
            .filter_map(|row| row.item.to_resolved())
            .collect();
        let gem_ids: BTreeSet<_> = skills
            .iter()
            .filter_map(|row| match row.source.to_resolved() {
                Some(AuthoredSkillSource::Gem(id)) => Some(id),
                _ => None,
            })
            .chain(supports.iter().filter_map(|row| row.support.to_resolved()))
            .collect();
        let items = rows!(items, &item_ids, Item);
        let gems = rows!(gems, &gem_ids, Gem);
        let issues: Vec<_> = validation
            .issues
            .into_iter()
            .filter(|issue| {
                issue
                    .owner
                    .is_some_and(|owner| selected_owners.contains(&owner))
            })
            .collect();
        if !issues.is_empty() {
            return Ok(DraftFinalization::Pending {
                draft_digest,
                selection: Box::new(selection),
                issues,
                queries: queries.queries.clone(),
            });
        }
        // Run the existing complete project/composition/request validators. This
        // catches selected combinations that omit a required supplying occurrence.
        let project = BuildProject::new(
            ProjectInput {
                allocator,
                revision: input.revision,
                game_version: input.game_version.clone(),
                weapon_loadouts: input.weapon_loadouts.members.clone(),
                items: resolved_rows(&items, "items")?,
                gems: resolved_rows(&gems, "gems")?,
                rewards: resolved_rows(&rewards, "rewards")?,
                equipment: resolved_rows(&equipment, "equipment")?,
                allocations: resolved_rows(&allocations, "allocations")?,
                skills: resolved_rows(&skills, "skills")?,
                supports: resolved_rows(&supports, "supports")?,
                payload_links: resolved_rows(&payload_links, "payload_links")?,
                character_presets: vec![resolved(character, "character")?],
                equipment_presets: vec![resolved(equipment_preset, "equipment_preset")?],
                allocation_presets: vec![resolved(allocation_preset, "allocation_preset")?],
                skill_presets: vec![resolved(skill_preset, "skill_preset")?],
                choice_presets: vec![resolved(choice_preset, "choice_preset")?],
                saved_variants: vec![],
            },
            limits.input,
        )?;
        let request = OwnedEvaluationRequest::new(
            compose(&project, &selection.build, None, limits.input)?,
            ScenarioSpec::new(resolved(&scenario.scenario, "scenario")?, limits.input)?,
            QuerySpec::new(resolved(&queries.queries, "queries")?, limits.input)?,
            limits.input,
        )?;
        let request_digest =
            digest_owned("owned-request-v1", &request, limits.input.max_wire_bytes)?;
        Ok(DraftFinalization::Ready(Box::new(FinalizedDraft {
            draft_digest,
            selection,
            request_digest,
            request,
        })))
    }
}
