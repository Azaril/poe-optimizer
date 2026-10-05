//! Versioned preset intent with one exact-source applicability boundary.
//!
//! Stored schema proof precedes selection. It is neither activation nor numerical
//! readiness, and cannot be reconstructed from serialized diagnostic records.
//! Stored proof deliberately defers selected required-choice satisfaction. The
//! composed request still requires ordinary definition/schema binding before use.
use crate::{
    build_identity::*,
    data::DataIdentity,
    owned_binding::{
        BindingError, BindingIssue, BindingLimits, IssueClass, validate_stored_intent,
    },
    owned_build::*,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_inventory::InventorySnapshot,
    owned_project::{BuildProject, ProjectError, ProjectInput, SkillPreset, VariantSelection},
    owned_schema::DefinitionSchemaIndex,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetApplicability {
    Required,
    WhenExactSourceSelected,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetUsageBinding {
    pub selection: UsagePolicySelection,
    pub applicability: PresetApplicability,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSkillInputBinding {
    pub target: GeneratedSkillKey,
    pub parameters: Vec<ParameterAssignment>,
    pub applicability: PresetApplicability,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPresetIntentV1 {
    pub schema_version: u32,
    pub usage: Vec<PresetUsageBinding>,
    pub generated_inputs: Vec<GeneratedSkillInputBinding>,
}
impl SkillPresetIntentV1 {
    /// Explicit authoring conversion, never an implicit codec migration.
    pub fn from_legacy(usage: Vec<UsagePolicySelection>) -> Self {
        Self {
            schema_version: 1,
            usage: usage
                .into_iter()
                .map(|selection| PresetUsageBinding {
                    selection,
                    applicability: PresetApplicability::Required,
                })
                .collect(),
            generated_inputs: vec![],
        }
    }
    pub(crate) fn canonicalize(&mut self) {
        self.usage.sort_by(|a, b| {
            (&a.selection.policy, &a.selection.target)
                .cmp(&(&b.selection.policy, &b.selection.target))
        });
        for row in &mut self.usage {
            row.selection.parameters.sort_by(|a, b| a.slot.cmp(&b.slot));
        }
        self.generated_inputs
            .sort_by(|a, b| a.target.cmp(&b.target));
        for row in &mut self.generated_inputs {
            row.parameters.sort_by(|a, b| a.slot.cmp(&b.slot));
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IntentRecord {
    Usage {
        policy: UsagePolicyDefId,
        target: UsageTarget,
    },
    GeneratedInput {
        target: GeneratedSkillKey,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IntentDisposition {
    Applied,
    Overridden,
    NotApplicable { excluded_sources: Vec<ProviderRoot> },
    RequiredSourceExcluded { excluded_sources: Vec<ProviderRoot> },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntentDiagnostic {
    pub skill_preset: SkillPresetId,
    pub record: IntentRecord,
    pub disposition: IntentDisposition,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntentSchemaIssues {
    pub skill_preset: Option<SkillPresetId>,
    pub issues: Vec<BindingIssue>,
}

#[derive(Debug)]
pub enum IntentError {
    Structure(StructuralError),
    Project(ProjectError),
    Binding(BindingError),
    Digest(ContentDigestError),
    ProofMismatch,
    Schema(Vec<IntentSchemaIssues>),
    RequiredSources(Vec<IntentDiagnostic>),
}
impl fmt::Display for IntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(e) => e.fmt(f),
            Self::Project(e) => e.fmt(f),
            Self::Binding(e) => e.fmt(f),
            Self::Digest(e) => e.fmt(f),
            Self::ProofMismatch => {
                f.write_str("preset intent proof differs from current content or data")
            }
            Self::Schema(_) => f.write_str("stored preset intent schema is invalid or unresolved"),
            Self::RequiredSources(_) => {
                f.write_str("required preset intent source is not selected")
            }
        }
    }
}
impl std::error::Error for IntentError {}
impl From<StructuralError> for IntentError {
    fn from(e: StructuralError) -> Self {
        Self::Structure(e)
    }
}
impl From<ProjectError> for IntentError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}
impl From<BindingError> for IntentError {
    fn from(e: BindingError) -> Self {
        Self::Binding(e)
    }
}
impl From<ContentDigestError> for IntentError {
    fn from(e: ContentDigestError) -> Self {
        Self::Digest(e)
    }
}

/// A checked full-project schema census. Alternative unresolved records remain
/// explicit; malformed resolved records cannot acquire this proof.
#[derive(Clone, Debug)]
pub struct ProjectIntentProof {
    content: OwnedContentDigest,
    data: DataIdentity,
    schema_issues: Vec<IntentSchemaIssues>,
    work_used: usize,
}
impl ProjectIntentProof {
    pub fn content_digest(&self) -> OwnedContentDigest {
        self.content
    }
    pub fn data_identity(&self) -> &DataIdentity {
        &self.data
    }
    pub fn schema_issues(&self) -> &[IntentSchemaIssues] {
        &self.schema_issues
    }
    pub fn work_used(&self) -> usize {
        self.work_used
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ComposedRequest {
    request: OwnedEvaluationRequest,
    diagnostics: Vec<IntentDiagnostic>,
    receipt: IntentCompositionReceipt,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct IntentCompositionReceipt {
    pub project_digest: OwnedContentDigest,
    pub data: DataIdentity,
    pub selection: VariantSelection,
    pub scenario_digest: OwnedContentDigest,
    pub request_digest: OwnedContentDigest,
}
pub struct IntentCompositionInputs<'a> {
    pub selection: &'a VariantSelection,
    pub inventory: Option<&'a InventorySnapshot>,
    pub scenario: ScenarioSpec,
    pub queries: QuerySpec,
}
impl ComposedRequest {
    pub fn request(&self) -> &OwnedEvaluationRequest {
        &self.request
    }
    pub fn diagnostics(&self) -> &[IntentDiagnostic] {
        &self.diagnostics
    }
    pub fn receipt(&self) -> &IntentCompositionReceipt {
        &self.receipt
    }
    pub fn into_request(self) -> OwnedEvaluationRequest {
        self.request
    }
}

fn structure(path: &str, kind: StructuralErrorKind) -> StructuralError {
    StructuralError {
        path: path.into(),
        kind,
    }
}
pub(crate) fn has_intent(project: &ProjectInput) -> bool {
    project.skill_presets.iter().any(|p| p.intent.is_some())
}
pub(crate) fn require_legacy(project: &ProjectInput) -> Result<(), StructuralError> {
    if has_intent(project) {
        return Err(structure(
            "project.skill_presets.intent.requires_data_proof",
            StructuralErrorKind::WrongProviderOwner,
        ));
    }
    Ok(())
}

pub(crate) fn usage_sources(target: &UsageTarget) -> Vec<ProviderRoot> {
    let mut roots = BTreeSet::new();
    match target {
        UsageTarget::Skill(SkillTarget::Authored(id)) => {
            roots.insert(ProviderRoot::SkillUse(*id));
        }
        UsageTarget::Skill(SkillTarget::Generated(key)) => {
            roots.insert(key.provider.root.clone());
        }
        UsageTarget::Actor(ActorKey::Owned(actor)) => {
            roots.insert(actor.provider.root.clone());
        }
        UsageTarget::Actor(ActorKey::Player) => {}
        UsageTarget::Action(action) => {
            roots.insert(action.action.provider.root.clone());
            if let ActorKey::Owned(actor) = &action.action.actor {
                roots.insert(actor.provider.root.clone());
            }
        }
    }
    roots.into_iter().collect()
}

pub(crate) fn validate_source_scope(
    path: &str,
    roots: &[ProviderRoot],
    skills: &BTreeSet<SkillUseId>,
    supports: &BTreeSet<SupportAssignmentId>,
    applicability: PresetApplicability,
) -> Result<(), StructuralError> {
    if roots.is_empty() {
        return Err(structure(path, StructuralErrorKind::WrongProviderOwner));
    }
    for root in roots {
        let local = match root {
            ProviderRoot::SkillUse(id) => Some(skills.contains(id)),
            ProviderRoot::SupportAssignment(id) => Some(supports.contains(id)),
            ProviderRoot::Allocation(_)
            | ProviderRoot::EquipmentUse(_)
            | ProviderRoot::ItemModifier { .. } => None,
            _ => return Err(structure(path, StructuralErrorKind::WrongProviderOwner)),
        };
        if let Some(member) = local
            && (!member || applicability != PresetApplicability::Required)
        {
            return Err(structure(path, StructuralErrorKind::WrongProviderOwner));
        }
    }
    Ok(())
}

/// Reuse complete structural leaves over real global records, without selecting
/// alternatives or fabricating a combined build.
pub(crate) fn validate_project_intent_structure(
    project: &ProjectInput,
    members: &[(InstanceId, OccurrenceKind)],
    limits: OwnedInputLimits,
) -> Result<usize, StructuralError> {
    if !has_intent(project) {
        return Ok(0);
    }
    let mut check = StructuralCheck::new(&project.game_version, limits, Some(project.allocator))?;
    check.begin_membership();
    for (id, kind) in members {
        check.register("intent.registry", *id, *kind)?;
    }
    for item in &project.items {
        for modifier in &item.modifiers {
            check.seed_modifier_item("intent.registry", modifier.id, item.id)?;
        }
    }
    for equipment in &project.equipment {
        check.seed_equipment_item("intent.registry", equipment.id, equipment.item)?;
    }
    for preset in &project.skill_presets {
        let Some(intent) = &preset.intent else {
            continue;
        };
        let skills = preset.skills.iter().copied().collect();
        let supports = preset.supports.iter().copied().collect();
        if preset.usage_preferences.is_some() || intent.schema_version != 1 {
            return Err(structure(
                "skill_presets.intent",
                StructuralErrorKind::WrongDeclaration,
            ));
        }
        check.collection("skill_presets.intent.usage", intent.usage.len())?;
        let mut usage = BTreeSet::new();
        for row in &intent.usage {
            if !usage.insert((&row.selection.policy, &row.selection.target)) {
                return Err(structure(
                    "skill_presets.intent.usage",
                    StructuralErrorKind::DuplicateAssignment,
                ));
            }
            check.usage(
                "skill_presets.intent.usage",
                std::slice::from_ref(&row.selection),
            )?;
            validate_source_scope(
                "skill_presets.intent.usage.target",
                &usage_sources(&row.selection.target),
                &skills,
                &supports,
                row.applicability,
            )?;
        }
        check.collection(
            "skill_presets.intent.generated_inputs",
            intent.generated_inputs.len(),
        )?;
        let mut targets = BTreeSet::new();
        for row in &intent.generated_inputs {
            if !targets.insert(&row.target) {
                return Err(structure(
                    "skill_presets.intent.generated_inputs",
                    StructuralErrorKind::DuplicateAssignment,
                ));
            }
            check.skill(
                "skill_presets.intent.generated_inputs.target",
                &SkillTarget::Generated(Box::new(row.target.clone())),
            )?;
            validate_source_scope(
                "skill_presets.intent.generated_inputs.target",
                std::slice::from_ref(&row.target.provider.root),
                &skills,
                &supports,
                row.applicability,
            )?;
            check.collection(
                "skill_presets.intent.generated_inputs.parameters",
                row.parameters.len(),
            )?;
            let mut parameters = BTreeSet::new();
            for parameter in &row.parameters {
                if !matches!(parameter.slot.declaration, SlotOwnerDefId::Skill(_)) {
                    return Err(structure(
                        "skill_presets.intent.generated_inputs.parameters",
                        StructuralErrorKind::WrongDeclaration,
                    ));
                }
                if !parameters.insert(&parameter.slot) {
                    return Err(structure(
                        "skill_presets.intent.generated_inputs.parameters",
                        StructuralErrorKind::DuplicateAssignment,
                    ));
                }
                check.slot(
                    "skill_presets.intent.generated_inputs.parameters",
                    &parameter.slot,
                )?;
                check.value(
                    "skill_presets.intent.generated_inputs.parameters",
                    &parameter.value,
                )?;
            }
        }
    }
    Ok(check.entries_used())
}

fn selected_inputs(
    preset: &SkillPreset,
    intent: &SkillPresetIntentV1,
) -> Vec<SelectedGeneratedSkillInput> {
    intent
        .generated_inputs
        .iter()
        .map(|row| SelectedGeneratedSkillInput {
            target: row.target.clone(),
            parameters: row.parameters.clone(),
            origin: GeneratedSkillInputOrigin {
                skill_preset: preset.id,
            },
        })
        .collect()
}

/// One budget across every preset, including schema checks of dormant rows.
pub(crate) struct IntentProofBudget {
    limits: BindingLimits,
    work: usize,
    issues: usize,
}
impl IntentProofBudget {
    pub(crate) fn new(limits: BindingLimits) -> Self {
        Self {
            limits,
            work: 0,
            issues: 0,
        }
    }
    pub(crate) fn work_used(&self) -> usize {
        self.work
    }
    pub(crate) fn charge(&mut self, amount: usize) -> Result<(), IntentError> {
        self.work = self
            .work
            .checked_add(amount)
            .filter(|v| *v <= self.limits.max_work)
            .ok_or(BindingError::WorkLimit)?;
        Ok(())
    }
    pub(crate) fn check<I: DefinitionSchemaIndex>(
        &mut self,
        index: &I,
        namespace: &GameVersionNamespace,
        tables: RecordTables<'_>,
        usage: &[UsagePolicySelection],
        inputs: &[SelectedGeneratedSkillInput],
    ) -> Result<Vec<BindingIssue>, IntentError> {
        if self.limits.max_work == 0
            || self.limits.max_work > 100_000_000
            || self.limits.max_issues == 0
            || self.limits.max_issues > 65_536
        {
            return Err(BindingError::InvalidLimit.into());
        }
        let work = self
            .limits
            .max_work
            .checked_sub(self.work)
            .filter(|v| *v > 0)
            .ok_or(BindingError::WorkLimit)?;
        let issues = self
            .limits
            .max_issues
            .checked_sub(self.issues)
            .ok_or(BindingError::IssueLimit)?;
        // The underlying checker requires a positive limit. At zero remaining
        // issues, permit only an issue-free check, then reject its first issue.
        let report = validate_stored_intent(
            index,
            namespace,
            tables,
            usage,
            inputs,
            BindingLimits {
                max_work: work,
                max_issues: if self.limits.max_issues == 0 {
                    0
                } else {
                    issues.max(1)
                },
                ..self.limits
            },
        )?;
        self.work = self
            .work
            .checked_add(report.work_used)
            .ok_or(BindingError::WorkLimit)?;
        if report.issues.len() > issues {
            return Err(BindingError::IssueLimit.into());
        }
        self.issues += report.issues.len();
        Ok(report.issues)
    }
}

pub fn prove_project_intent<I: DefinitionSchemaIndex>(
    index: &I,
    project: &BuildProject,
    limits: BindingLimits,
) -> Result<ProjectIntentProof, IntentError> {
    project.validate_limits(limits.input)?;
    let content = digest_owned(
        "owned-project-intent-v1",
        project,
        limits.input.max_wire_bytes,
    )?;
    let mut budget = IntentProofBudget::new(limits);
    budget.check(
        index,
        &project.input().game_version,
        project.input().tables(),
        &[],
        &[],
    )?;
    let mut schema_issues = vec![];
    for preset in &project.input().skill_presets {
        let Some(intent) = &preset.intent else {
            continue;
        };
        let usage: Vec<_> = intent.usage.iter().map(|r| r.selection.clone()).collect();
        let inputs = selected_inputs(preset, intent);
        let issues = budget.check(
            index,
            &project.input().game_version,
            project.input().tables(),
            &usage,
            &inputs,
        )?;
        if !issues.is_empty() {
            schema_issues.push(IntentSchemaIssues {
                skill_preset: Some(preset.id),
                issues,
            });
        }
    }
    if schema_issues
        .iter()
        .any(|row| row.issues.iter().any(|i| i.class == IssueClass::Invalid))
    {
        return Err(IntentError::Schema(schema_issues));
    }
    Ok(ProjectIntentProof {
        content,
        data: index.identity().clone(),
        schema_issues,
        work_used: budget.work_used(),
    })
}

struct SelectedSources {
    allocations: BTreeSet<AllocationId>,
    equipment: BTreeSet<ItemSlotUseId>,
    skills: BTreeSet<SkillUseId>,
    supports: BTreeSet<SupportAssignmentId>,
}
impl SelectedSources {
    fn new(build: &BuildInput) -> Self {
        Self {
            allocations: build.allocations.iter().map(|r| r.id).collect(),
            equipment: build.equipment.iter().map(|r| r.id).collect(),
            skills: build.skills.iter().map(|r| r.id).collect(),
            supports: build.supports.iter().map(|r| r.id).collect(),
        }
    }
    fn excluded(&self, roots: &[ProviderRoot]) -> Vec<ProviderRoot> {
        roots
            .iter()
            .filter(|root| !match root {
                ProviderRoot::Allocation(id) => self.allocations.contains(id),
                ProviderRoot::EquipmentUse(id)
                | ProviderRoot::ItemModifier {
                    equipment_use: id, ..
                } => self.equipment.contains(id),
                ProviderRoot::SkillUse(id) => self.skills.contains(id),
                ProviderRoot::SupportAssignment(id) => self.supports.contains(id),
                _ => false,
            })
            .cloned()
            .collect()
    }
}

/// The only selection decision shared by raw bindings and usage preferences.
pub(crate) fn disposition(
    applicability: PresetApplicability,
    excluded: Vec<ProviderRoot>,
) -> IntentDisposition {
    if excluded.is_empty() {
        IntentDisposition::Applied
    } else if applicability == PresetApplicability::WhenExactSourceSelected {
        IntentDisposition::NotApplicable {
            excluded_sources: excluded,
        }
    } else {
        IntentDisposition::RequiredSourceExcluded {
            excluded_sources: excluded,
        }
    }
}

pub fn compose_request_checked<I: DefinitionSchemaIndex>(
    index: &I,
    project: &BuildProject,
    inputs: IntentCompositionInputs<'_>,
    proof: &ProjectIntentProof,
    limits: BindingLimits,
) -> Result<ComposedRequest, IntentError> {
    let IntentCompositionInputs {
        selection,
        inventory,
        scenario,
        queries,
    } = inputs;
    if proof.data != *index.identity()
        || proof.content
            != digest_owned(
                "owned-project-intent-v1",
                project,
                limits.input.max_wire_bytes,
            )?
    {
        return Err(IntentError::ProofMismatch);
    }
    // Revalidate tighter caller input limits before cloning selected records.
    project.validate_limits(limits.input)?;
    if proof.work_used > limits.max_work {
        return Err(BindingError::WorkLimit.into());
    }
    if proof
        .schema_issues
        .iter()
        .map(|r| r.issues.len())
        .sum::<usize>()
        > limits.max_issues
    {
        return Err(BindingError::IssueLimit.into());
    }
    let build =
        crate::owned_project::compose_validated(project, selection, inventory, limits.input)?;
    let preset = project
        .input()
        .skill_presets
        .iter()
        .find(|p| p.id == selection.skills)
        .expect("validated selection");
    let selected_issues: Vec<_> = proof
        .schema_issues
        .iter()
        .filter(|r| r.skill_preset == Some(preset.id))
        .cloned()
        .collect();
    if !selected_issues.is_empty() {
        return Err(IntentError::Schema(selected_issues));
    }
    let temporary = OwnedEvaluationRequest::new(
        build.clone(),
        scenario.clone(),
        queries.clone(),
        limits.input,
    )?;
    let report = crate::owned_binding::validate_selected_usage(index, &temporary, limits)?;
    if !report.issues.is_empty() {
        return Err(IntentError::Schema(vec![IntentSchemaIssues {
            skill_preset: None,
            issues: report.issues,
        }]));
    }
    let scenario_digest = digest_owned(
        "owned-preset-intent-scenario-v1",
        &scenario,
        limits.input.max_wire_bytes,
    )?;
    let (request, diagnostics) =
        compose_proven_preset(build, preset, scenario, queries, limits.input)?;
    let receipt = IntentCompositionReceipt {
        project_digest: proof.content,
        data: proof.data.clone(),
        selection: *selection,
        scenario_digest,
        request_digest: digest_owned("owned-request-v1", &request, limits.input.max_wire_bytes)?,
    };
    Ok(ComposedRequest {
        request,
        diagnostics,
        receipt,
    })
}

/// Called only after the full stored authoring graph and selected scenario have
/// been checked. Selection and activation remain deliberately separate.
pub(crate) fn compose_proven_preset(
    build: BuildSpec,
    preset: &SkillPreset,
    scenario: ScenarioSpec,
    queries: QuerySpec,
    limits: OwnedInputLimits,
) -> Result<(OwnedEvaluationRequest, Vec<IntentDiagnostic>), IntentError> {
    let Some(intent) = &preset.intent else {
        return compose_selected(
            build,
            preset.usage_preferences.as_deref().unwrap_or_default(),
            None,
            scenario,
            queries,
            vec![],
            limits,
        );
    };
    let selected = SelectedSources::new(build.input());
    let overrides: BTreeSet<_> = scenario
        .input()
        .usage
        .iter()
        .map(|v| (&v.policy, &v.target))
        .collect();
    let mut diagnostics = vec![];
    let mut usage = vec![];
    for row in &intent.usage {
        let mut result = disposition(
            row.applicability,
            selected.excluded(&usage_sources(&row.selection.target)),
        );
        if result == IntentDisposition::Applied {
            if overrides.contains(&(&row.selection.policy, &row.selection.target)) {
                result = IntentDisposition::Overridden;
            }
            usage.push(row.selection.clone());
        }
        diagnostics.push(IntentDiagnostic {
            skill_preset: preset.id,
            record: IntentRecord::Usage {
                policy: row.selection.policy.clone(),
                target: row.selection.target.clone(),
            },
            disposition: result,
        });
    }
    let mut inputs = vec![];
    for row in &intent.generated_inputs {
        let result = disposition(
            row.applicability,
            selected.excluded(std::slice::from_ref(&row.target.provider.root)),
        );
        if result == IntentDisposition::Applied {
            inputs.push(SelectedGeneratedSkillInput {
                target: row.target.clone(),
                parameters: row.parameters.clone(),
                origin: GeneratedSkillInputOrigin {
                    skill_preset: preset.id,
                },
            });
        }
        diagnostics.push(IntentDiagnostic {
            skill_preset: preset.id,
            record: IntentRecord::GeneratedInput {
                target: row.target.clone(),
            },
            disposition: result,
        });
    }
    if diagnostics.iter().any(|d| {
        matches!(
            d.disposition,
            IntentDisposition::RequiredSourceExcluded { .. }
        )
    }) {
        return Err(IntentError::RequiredSources(diagnostics));
    }
    compose_selected(
        build,
        &usage,
        Some(GeneratedSkillInputsV1 {
            schema_version: 1,
            bindings: inputs,
        }),
        scenario,
        queries,
        diagnostics,
        limits,
    )
}

fn compose_selected(
    build: BuildSpec,
    preferences: &[UsagePolicySelection],
    inputs: Option<GeneratedSkillInputsV1>,
    scenario: ScenarioSpec,
    queries: QuerySpec,
    diagnostics: Vec<IntentDiagnostic>,
    limits: OwnedInputLimits,
) -> Result<(OwnedEvaluationRequest, Vec<IntentDiagnostic>), IntentError> {
    OwnedEvaluationRequest::new(build.clone(), scenario.clone(), queries.clone(), limits)?;
    let mut merged = scenario.into_input();
    validate_usage_layers(&merged.game_version, preferences, &merged.usage, limits)?;
    let mut usage: BTreeMap<_, _> = preferences
        .iter()
        .map(|row| ((row.policy.clone(), row.target.clone()), row.clone()))
        .collect();
    for row in merged.usage {
        usage.insert((row.policy.clone(), row.target.clone()), row);
    }
    merged.usage = usage.into_values().collect();
    let mut build = build.into_input();
    build.generated_inputs = inputs;
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(build, limits)?,
        ScenarioSpec::new(merged, limits)?,
        queries,
        limits,
    )?;
    Ok((request, diagnostics))
}
