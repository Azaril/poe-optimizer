//! Full-session intent proof, before selected records are projected.
use super::*;
use crate::{
    build_identity::*,
    data::DataIdentity,
    owned_binding::{BindingLimits, IssueClass},
    owned_build::*,
    owned_preset_intent::{
        IntentDiagnostic, IntentError, IntentProofBudget, IntentSchemaIssues,
        compose_proven_preset, usage_sources,
    },
    owned_schema::DefinitionSchemaIndex,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct DraftIntentProof {
    digest: OwnedContentDigest,
    data: DataIdentity,
    schema: Vec<IntentSchemaIssues>,
    obligations: BTreeMap<SkillPresetId, Vec<DraftIssue>>,
    work_used: usize,
}
impl DraftIntentProof {
    pub fn draft_digest(&self) -> OwnedContentDigest {
        self.digest
    }
    pub fn data_identity(&self) -> &DataIdentity {
        &self.data
    }
    pub fn schema_issues(&self) -> &[IntentSchemaIssues] {
        &self.schema
    }
    pub fn unresolved_dependencies(&self) -> &BTreeMap<SkillPresetId, Vec<DraftIssue>> {
        &self.obligations
    }
    pub fn work_used(&self) -> usize {
        self.work_used
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CheckedDraftFinalization {
    draft_digest: OwnedContentDigest,
    data: DataIdentity,
    finalization: DraftFinalization,
    diagnostics: Vec<IntentDiagnostic>,
    schema_issues: Vec<IntentSchemaIssues>,
}
impl CheckedDraftFinalization {
    pub fn draft_digest(&self) -> OwnedContentDigest {
        self.draft_digest
    }
    pub fn data_identity(&self) -> &DataIdentity {
        &self.data
    }
    pub fn finalization(&self) -> &DraftFinalization {
        &self.finalization
    }
    pub fn into_finalization(self) -> DraftFinalization {
        self.finalization
    }
    pub fn diagnostics(&self) -> &[IntentDiagnostic] {
        &self.diagnostics
    }
    pub fn schema_issues(&self) -> &[IntentSchemaIssues] {
        &self.schema_issues
    }
}

/// Only genuinely resolved records enter the schema resolver. The dependency
/// graph below keeps omitted unresolved records as obligations, not absent roots.
#[derive(Default)]
struct ResolvedRecords {
    rewards: Vec<RewardSelection>,
    items: Vec<ItemRecord>,
    gems: Vec<GemInstance>,
    equipment: Vec<EquipmentUse>,
    allocations: Vec<Allocation>,
    skills: Vec<SkillUse>,
    supports: Vec<SupportAssignment>,
    payload_links: Vec<PayloadLink>,
}
impl ResolvedRecords {
    fn new(input: &DraftSessionInput) -> Self {
        Self {
            rewards: resolved_members(&input.rewards),
            items: resolved_members(&input.items),
            gems: resolved_members(&input.gems),
            equipment: resolved_members(&input.equipment),
            allocations: resolved_members(&input.allocations),
            skills: resolved_members(&input.skills),
            supports: resolved_members(&input.supports),
            payload_links: resolved_members(&input.payload_links),
        }
    }
    fn tables<'a>(&'a self, input: &'a DraftSessionInput) -> RecordTables<'a> {
        RecordTables {
            weapon_loadouts: &input.weapon_loadouts.members,
            rewards: &self.rewards,
            items: &self.items,
            gems: &self.gems,
            equipment: &self.equipment,
            allocations: &self.allocations,
            skills: &self.skills,
            supports: &self.supports,
            payload_links: &self.payload_links,
        }
    }
}
fn resolved_members<T: ResolveDraft>(rows: &DraftList<T>) -> Vec<T::Resolved> {
    rows.members
        .iter()
        .filter_map(ResolveDraft::to_resolved)
        .collect()
}
fn root_id(root: &ProviderRoot) -> Option<InstanceId> {
    match root {
        ProviderRoot::Allocation(id) => Some(id.instance_id()),
        ProviderRoot::EquipmentUse(id)
        | ProviderRoot::ItemModifier {
            equipment_use: id, ..
        } => Some(id.instance_id()),
        ProviderRoot::SkillUse(id) => Some(id.instance_id()),
        ProviderRoot::SupportAssignment(id) => Some(id.instance_id()),
        _ => None,
    }
}
fn dependencies(input: &DraftSessionInput) -> BTreeMap<InstanceId, Vec<InstanceId>> {
    let mut edges = BTreeMap::new();
    for row in &input.items.members {
        edges.insert(
            row.id.instance_id(),
            row.modifiers
                .members
                .iter()
                .map(|m| m.id.instance_id())
                .collect(),
        );
    }
    for row in &input.equipment.members {
        let mut children: Vec<_> = row
            .item
            .to_resolved()
            .map(BuildInstanceId::instance_id)
            .into_iter()
            .collect();
        match &row.destination {
            DraftEquipmentDestination::ItemSocket { container, .. } => {
                children.extend(container.to_resolved().map(BuildInstanceId::instance_id))
            }
            DraftEquipmentDestination::PassiveSocket { allocation, .. } => {
                children.extend(allocation.to_resolved().map(BuildInstanceId::instance_id))
            }
            _ => {}
        }
        edges.insert(row.id.instance_id(), children);
    }
    for row in &input.allocations.members {
        let mut children = vec![];
        if let DraftAllocationAccess::Granted(provider) = &row.access {
            children.extend(provider.root.to_resolved().as_ref().and_then(root_id));
        }
        edges.insert(row.id.instance_id(), children);
    }
    for row in &input.skills.members {
        let children = match &row.source {
            DraftAuthoredSkillSource::Gem(id) => id
                .to_resolved()
                .map(BuildInstanceId::instance_id)
                .into_iter()
                .collect(),
            _ => vec![],
        };
        edges.insert(row.id.instance_id(), children);
    }
    for row in &input.supports.members {
        let mut children: Vec<_> = row
            .support
            .to_resolved()
            .map(BuildInstanceId::instance_id)
            .into_iter()
            .collect();
        match &row.target {
            DraftSkillTarget::Authored(id) => {
                children.extend(id.to_resolved().map(BuildInstanceId::instance_id))
            }
            DraftSkillTarget::Generated(key) => {
                children.extend(key.provider.root.to_resolved().as_ref().and_then(root_id))
            }
            _ => {}
        }
        edges.insert(row.id.instance_id(), children);
    }
    edges
}
fn dependency_issues(
    roots: &[ProviderRoot],
    edges: &BTreeMap<InstanceId, Vec<InstanceId>>,
    issues: &BTreeMap<InstanceId, Vec<DraftIssue>>,
    budget: &mut IntentProofBudget,
) -> Result<Vec<DraftIssue>, IntentError> {
    budget.charge(roots.len())?;
    let mut pending: Vec<_> = roots.iter().filter_map(root_id).collect();
    let mut seen = BTreeSet::new();
    let mut result = BTreeMap::new();
    while let Some(id) = pending.pop() {
        budget.charge(1)?;
        if !seen.insert(id) {
            continue;
        }
        if let Some(rows) = issues.get(&id) {
            budget.charge(rows.len())?;
            for issue in rows {
                result.insert(issue.id, issue.clone());
            }
        }
        if let Some(children) = edges.get(&id) {
            budget.charge(children.len())?;
            pending.extend(children);
        }
    }
    Ok(result.into_values().collect())
}
impl DraftSession {
    /// Prove every resolved intent row, including unselected alternatives. Known
    /// malformed rows fail; unresolved fields and source records retain their
    /// original issue IDs. Required selected choices remain a final binding duty.
    pub fn prove_intent<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
        draft_limits: DraftLimits,
        limits: BindingLimits,
    ) -> Result<DraftIntentProof, IntentError> {
        self.validate_limits(draft_limits)?;
        let (validation, entries) = super::structure::validate_with_entries(
            self.input(),
            DraftLimits {
                input: limits.input,
                ..draft_limits
            },
        )?;
        let input = self.input();
        let mut budget = IntentProofBudget::new(limits);
        budget.check(
            index,
            &input.game_version,
            ResolvedRecords::default().tables(input),
            &[],
            &[],
        )?;
        // Structural traversal counted all nested fields/collections before these
        // complete-record clones and the known-relationship graph are allocated.
        budget.charge(entries)?;
        let records = ResolvedRecords::new(input);
        let edges = dependencies(input);
        let mut issues_by_owner: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for issue in validation.issues {
            if let Some(owner) = issue.owner {
                issues_by_owner.entry(owner).or_default().push(issue);
            }
        }
        let mut schema = vec![];
        let mut obligations = BTreeMap::new();
        for preset in &input.skill_presets.members {
            let Some(intent) = &preset.intent else {
                continue;
            };
            let mut pending: BTreeMap<_, _> = issues_by_owner
                .get(&preset.id.instance_id())
                .into_iter()
                .flatten()
                .map(|v| (v.id, v.clone()))
                .collect();
            let mut usage = vec![];
            let mut inputs = vec![];
            for row in &intent.usage.members {
                let Some(row) = row.to_resolved() else {
                    continue;
                };
                let gaps = dependency_issues(
                    &usage_sources(&row.selection.target),
                    &edges,
                    &issues_by_owner,
                    &mut budget,
                )?;
                if gaps.is_empty() {
                    usage.push(row.selection);
                } else {
                    pending.extend(gaps.into_iter().map(|v| (v.id, v)));
                }
            }
            for row in &intent.generated_inputs.members {
                let Some(row) = row.to_resolved() else {
                    continue;
                };
                let gaps = dependency_issues(
                    std::slice::from_ref(&row.target.provider.root),
                    &edges,
                    &issues_by_owner,
                    &mut budget,
                )?;
                if gaps.is_empty() {
                    inputs.push(SelectedGeneratedSkillInput {
                        target: row.target,
                        parameters: row.parameters,
                        origin: GeneratedSkillInputOrigin {
                            skill_preset: preset.id,
                        },
                    });
                } else {
                    pending.extend(gaps.into_iter().map(|v| (v.id, v)));
                }
            }
            let found = budget.check(
                index,
                &input.game_version,
                records.tables(input),
                &usage,
                &inputs,
            )?;
            if !found.is_empty() {
                schema.push(IntentSchemaIssues {
                    skill_preset: Some(preset.id),
                    issues: found,
                });
            }
            if !pending.is_empty() {
                obligations.insert(preset.id, pending.into_values().collect());
            }
        }
        if schema
            .iter()
            .any(|r| r.issues.iter().any(|i| i.class == IssueClass::Invalid))
        {
            return Err(IntentError::Schema(schema));
        }
        Ok(DraftIntentProof {
            digest: self.digest(limits.input.max_wire_bytes)?,
            data: index.identity().clone(),
            schema,
            obligations,
            work_used: budget.work_used(),
        })
    }

    pub fn finalize_selection_checked<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
        selection: EvaluationSelection,
        proof: &DraftIntentProof,
        draft_limits: DraftLimits,
        limits: BindingLimits,
    ) -> Result<CheckedDraftFinalization, FinalizationError> {
        // Pending is a successful checked result too: validate the caller's
        // current index and limits before any obligation-only return can bypass
        // the selected-request callback below.
        IntentProofBudget::new(limits).check(
            index,
            &self.input().game_version,
            ResolvedRecords::default().tables(self.input()),
            &[],
            &[],
        )?;
        if proof.digest != self.digest(limits.input.max_wire_bytes)?
            || proof.data != *index.identity()
        {
            return Err(IntentError::ProofMismatch.into());
        }
        if proof.work_used > limits.max_work {
            return Err(IntentError::Binding(crate::owned_binding::BindingError::WorkLimit).into());
        }
        if proof.schema.iter().map(|r| r.issues.len()).sum::<usize>() > limits.max_issues {
            return Err(
                IntentError::Binding(crate::owned_binding::BindingError::IssueLimit).into(),
            );
        }
        self.validate_limits(DraftLimits {
            input: limits.input,
            ..draft_limits
        })?;
        let schema_issues: Vec<_> = proof
            .schema
            .iter()
            .filter(|r| r.skill_preset == Some(selection.build.skills))
            .cloned()
            .collect();
        let extra = proof
            .obligations
            .get(&selection.build.skills)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let (finalization, diagnostics) = self.finalize_with(
            selection,
            draft_limits,
            extra,
            !schema_issues.is_empty(),
            |build, preset, scenario, queries| {
                let request = OwnedEvaluationRequest::new(
                    build.clone(),
                    scenario.clone(),
                    queries.clone(),
                    limits.input,
                )?;
                let checked =
                    crate::owned_binding::validate_selected_usage(index, &request, limits)
                        .map_err(IntentError::from)?;
                if !checked.issues.is_empty() {
                    return Err(IntentError::Schema(vec![IntentSchemaIssues {
                        skill_preset: None,
                        issues: checked.issues,
                    }])
                    .into());
                }
                Ok(compose_proven_preset(
                    build,
                    preset,
                    scenario,
                    queries,
                    limits.input,
                )?)
            },
        )?;
        Ok(CheckedDraftFinalization {
            draft_digest: proof.digest,
            data: proof.data.clone(),
            finalization,
            diagnostics,
            schema_issues,
        })
    }
}
