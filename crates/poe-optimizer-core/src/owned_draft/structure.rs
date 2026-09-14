//! Bounded structural validation of partial, source-independent records.
//! Candidates are checked individually; no candidate is selected or installed.
use super::{records::*, session::*};
use crate::{build_identity::*, owned_build::*, owned_definitions::*};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

type Result<T = ()> = std::result::Result<T, StructuralError>;
fn error(path: &str, kind: StructuralErrorKind) -> StructuralError {
    StructuralError {
        path: path.into(),
        kind,
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DraftLimits {
    pub input: OwnedInputLimits,
    pub max_issues: usize,
    pub max_candidates_per_field: usize,
}
impl Default for DraftLimits {
    fn default() -> Self {
        Self {
            input: OwnedInputLimits::default(),
            max_issues: 16_384,
            max_candidates_per_field: 64,
        }
    }
}
impl DraftLimits {
    pub(crate) fn validate(self) -> Result {
        let hard = DraftLimits::default();
        for (path, value, ceiling) in [
            ("max_issues", self.max_issues, hard.max_issues),
            (
                "max_candidates_per_field",
                self.max_candidates_per_field,
                hard.max_candidates_per_field,
            ),
        ] {
            if value == 0 || value > ceiling {
                return Err(error(path, StructuralErrorKind::InvalidLimit));
            }
        }
        self.input.validate()
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DraftIssue {
    pub id: DraftIssueId,
    /// Top-level row identity; None denotes a global registry's open completion.
    pub owner: Option<InstanceId>,
    pub path: String,
    pub code: OwnedDefinitionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DraftValidation {
    pub issues: Vec<DraftIssue>,
}

pub fn validate_draft(input: &DraftSessionInput, limits: DraftLimits) -> Result<DraftValidation> {
    limits.validate()?;
    let mut visitor = Visit {
        check: StructuralCheck::new(&input.game_version, limits.input, Some(input.allocator))?,
        limits,
        gathering: true,
        historical: false,
        owner: None,
        issues: Vec::new(),
    };
    visitor.check.begin_membership();
    // All live rows and issues enter one registry before any reference is checked.
    visitor.session(input)?;
    visitor.gathering = false;
    for (i, item) in input.items.members.iter().enumerate() {
        for (j, modifier) in item.modifiers.members.iter().enumerate() {
            visitor.check.seed_modifier_item(
                &format!("items.members[{i}].modifiers.members[{j}]"),
                modifier.id,
                item.id,
            )?;
        }
    }
    for (i, equipment) in input.equipment.members.iter().enumerate() {
        if let DraftField::Known { value } = equipment.item {
            visitor.check.seed_equipment_item(
                &format!("equipment.members[{i}].item"),
                equipment.id,
                value,
            )?;
        }
    }
    visitor.session(input)?;
    known_containment(&input.equipment.members)?;
    Ok(DraftValidation {
        issues: visitor.issues,
    })
}

struct Visit<'a> {
    check: StructuralCheck<'a>,
    limits: DraftLimits,
    gathering: bool,
    historical: bool,
    owner: Option<InstanceId>,
    issues: Vec<DraftIssue>,
}
impl Visit<'_> {
    fn issue(&mut self, path: &str, id: DraftIssueId, code: &OwnedDefinitionKey) -> Result {
        if self.gathering {
            if self.issues.len() >= self.limits.max_issues {
                return Err(error(path, StructuralErrorKind::LimitExceeded));
            }
            self.check.collection(path, 1)?;
            self.check
                .register(&format!("{path}.id"), id, OccurrenceKind::DraftIssue)?;
            self.issues.push(DraftIssue {
                id,
                owner: self.owner,
                path: path.into(),
                code: code.clone(),
            });
        }
        Ok(())
    }
    fn pending<T>(
        &mut self,
        path: &str,
        value: &PendingValue<T>,
        mut validate: impl FnMut(&mut Self, &str, &T) -> Result,
    ) -> Result {
        self.issue(path, value.id, &value.code)?;
        if self.gathering {
            if value.candidates.len() > self.limits.max_candidates_per_field {
                return Err(error(path, StructuralErrorKind::LimitExceeded));
            }
            self.check
                .collection(&format!("{path}.candidates"), value.candidates.len())?;
        } else {
            for (i, candidate) in value.candidates.iter().enumerate() {
                validate(self, &format!("{path}.candidates[{i}]"), candidate)?;
            }
        }
        Ok(())
    }
    fn field<T>(
        &mut self,
        path: &str,
        value: &DraftField<T>,
        mut validate: impl FnMut(&mut Self, &str, &T) -> Result,
    ) -> Result {
        match value {
            DraftField::Known { value } => {
                if self.gathering {
                    Ok(())
                } else {
                    validate(self, &format!("{path}.value"), value)
                }
            }
            DraftField::Pending(value) => self.pending(path, value, validate),
        }
    }
    fn list<T>(
        &mut self,
        path: &str,
        value: &DraftList<T>,
        mut member: impl FnMut(&mut Self, &str, &T) -> Result,
    ) -> Result {
        if self.gathering {
            self.check
                .collection(&format!("{path}.members"), value.members.len())?;
        }
        if let DraftListCompletion::Pending { id, code } = &value.completion {
            self.issue(&format!("{path}.completion"), *id, code)?;
        }
        for (i, value) in value.members.iter().enumerate() {
            member(self, &format!("{path}.members[{i}]"), value)?;
        }
        Ok(())
    }
    fn row<T: BuildInstanceId>(&mut self, path: &str, id: T, kind: OccurrenceKind) -> Result {
        self.owner = Some(id.instance_id());
        if self.gathering {
            self.check.register(&format!("{path}.id"), id, kind)?;
        }
        Ok(())
    }
    fn checked(&mut self, validate: impl FnOnce(&mut StructuralCheck<'_>) -> Result) -> Result {
        if self.historical {
            self.check.with_query_references(validate)
        } else {
            validate(&mut self.check)
        }
    }
    fn reference<T: BuildInstanceId + Copy>(
        &mut self,
        path: &str,
        value: &T,
        kind: OccurrenceKind,
    ) -> Result {
        self.checked(|check| check.reference(path, *value, kind))
    }
    fn ref_field<T: BuildInstanceId + Copy>(
        &mut self,
        path: &str,
        value: &DraftField<T>,
        kind: OccurrenceKind,
    ) -> Result {
        self.field(path, value, |v, p, id| v.reference(p, id, kind))
    }
    fn definition<K: DefinitionDomain>(
        &mut self,
        path: &str,
        value: &DraftField<DefId<K>>,
    ) -> Result {
        self.field(path, value, |v, p, id| v.check.definition(p, id))
    }
    fn scalar<T>(&mut self, path: &str, value: &DraftField<T>) -> Result {
        self.field(path, value, |_, _, _| Ok(()))
    }
    fn slot<K: DefinitionDomain>(
        &mut self,
        path: &str,
        value: &DraftField<DeclaredSlot<DefId<K>>>,
    ) -> Result {
        self.field(path, value, |v, p, slot| v.check.slot(p, slot))
    }
    fn references<T: BuildInstanceId + Copy + Ord>(
        &mut self,
        path: &str,
        values: &DraftList<T>,
        kind: OccurrenceKind,
    ) -> Result {
        let mut seen = BTreeSet::new();
        self.list(path, values, |v, p, id| {
            if !v.gathering {
                v.reference(p, id, kind)?;
                if !seen.insert(*id) {
                    return Err(error(p, StructuralErrorKind::DuplicateAssignment));
                }
            }
            Ok(())
        })
    }
    fn parameters(
        &mut self,
        path: &str,
        values: &DraftList<ParameterDraft>,
        declaration: Option<SlotOwnerDefId>,
        owner_family: fn(&SlotOwnerDefId) -> bool,
    ) -> Result {
        let mut seen = BTreeSet::new();
        self.list(path, values, |v, p, value| {
            v.field(&format!("{p}.slot"), &value.slot, |v, p, slot| {
                v.check.slot(p, slot)?;
                if !owner_family(&slot.declaration)
                    || declaration
                        .as_ref()
                        .is_some_and(|owner| owner != &slot.declaration)
                {
                    return Err(error(p, StructuralErrorKind::WrongDeclaration));
                }
                Ok(())
            })?;
            v.field(&format!("{p}.value"), &value.value, |v, p, value| {
                v.check.value(p, value)
            })?;
            if !v.gathering
                && let DraftField::Known { value: slot } = &value.slot
                && !seen.insert(slot.clone())
            {
                return Err(error(p, StructuralErrorKind::DuplicateAssignment));
            }
            Ok(())
        })
    }
    fn quality(&mut self, path: &str, value: &DraftQuality) -> Result {
        match value {
            DraftQuality::Known { value: None } => Ok(()),
            DraftQuality::Known { value: Some(value) } => {
                self.definition(&format!("{path}.value.kind"), &value.kind)?;
                self.field(&format!("{path}.value.amount"), &value.amount, |v, p, q| {
                    v.check.definition(p, q.unit())
                })
            }
            DraftQuality::Pending(value) => {
                self.pending(path, value, |v, p, q| v.check.quality(p, q))
            }
        }
    }
    fn choice_selection(&mut self, path: &str, value: &ChoiceSelectionDraft) -> Result {
        self.slot(&format!("{path}.slot"), &value.slot)?;
        self.field(&format!("{path}.value"), &value.value, |v, p, value| {
            v.check.value(p, value)
        })
    }
    fn scope(&mut self, path: &str, value: &DraftField<LoadoutScope>) -> Result {
        self.field(path, value, |v, p, scope| v.check.scope(p, scope))
    }

    fn destination(&mut self, path: &str, value: &DraftEquipmentDestination) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftEquipmentDestination::CharacterSlot(slot) => self.definition(&p, slot),
            DraftEquipmentDestination::ItemSocket { container, slot } => {
                self.ref_field(
                    &format!("{p}.container"),
                    container,
                    OccurrenceKind::EquipmentUse,
                )?;
                self.definition(&format!("{p}.slot"), slot)
            }
            DraftEquipmentDestination::PassiveSocket { allocation, slot } => {
                self.ref_field(
                    &format!("{p}.allocation"),
                    allocation,
                    OccurrenceKind::Allocation,
                )?;
                self.definition(&format!("{p}.slot"), slot)
            }
            DraftEquipmentDestination::Pending(value) => {
                self.pending(&p, value, |v, p, value| match value {
                    EquipmentDestination::CharacterSlot(slot) => v.check.definition(p, slot),
                    EquipmentDestination::ItemSocket { container, slot } => {
                        v.reference(p, container, OccurrenceKind::EquipmentUse)?;
                        v.check.definition(p, slot)
                    }
                    EquipmentDestination::PassiveSocket { allocation, slot } => {
                        v.reference(p, allocation, OccurrenceKind::Allocation)?;
                        v.check.definition(p, slot)
                    }
                })
            }
        }
    }
    fn source(&mut self, path: &str, value: &DraftAuthoredSkillSource) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftAuthoredSkillSource::Gem(id) => self.ref_field(&p, id, OccurrenceKind::Gem),
            DraftAuthoredSkillSource::Direct(id) => self.definition(&p, id),
            DraftAuthoredSkillSource::Pending(value) => {
                self.pending(&p, value, |v, p, value| match value {
                    AuthoredSkillSource::Gem(id) => v.reference(p, id, OccurrenceKind::Gem),
                    AuthoredSkillSource::Direct(id) => v.check.definition(p, id),
                })
            }
        }
    }
    fn access(&mut self, path: &str, value: &DraftAllocationAccess) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftAllocationAccess::Ordinary => Ok(()),
            DraftAllocationAccess::Granted(value) => self.provider(&p, value),
            DraftAllocationAccess::Pending(value) => {
                self.pending(&p, value, |v, p, value| match value {
                    AllocationAccess::Ordinary => Ok(()),
                    AllocationAccess::Granted(value) => v.checked(|c| c.provider(p, value)),
                })
            }
        }
    }
    fn provider_root_value(&mut self, path: &str, value: &ProviderRoot) -> Result {
        // Empty path here only validates this already-typed root and its ownership.
        self.checked(|c| {
            c.provider(
                path,
                &ProviderKey {
                    root: value.clone(),
                    grant_path: Vec::new(),
                },
            )
        })
    }
    fn provider_root(&mut self, path: &str, value: &DraftProviderRoot) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftProviderRoot::Character => Ok(()),
            DraftProviderRoot::ItemModifier {
                equipment_use,
                modifier,
            } => {
                self.field(&format!("{p}.equipment_use"), equipment_use, |v, p, id| {
                    if let DraftField::Known { value: modifier } = modifier {
                        v.provider_root_value(
                            p,
                            &ProviderRoot::ItemModifier {
                                equipment_use: *id,
                                modifier: *modifier,
                            },
                        )
                    } else {
                        v.reference(p, id, OccurrenceKind::EquipmentUse)
                    }
                })?;
                self.field(&format!("{p}.modifier"), modifier, |v, p, id| {
                    if let DraftField::Known {
                        value: equipment_use,
                    } = equipment_use
                    {
                        v.provider_root_value(
                            p,
                            &ProviderRoot::ItemModifier {
                                equipment_use: *equipment_use,
                                modifier: *id,
                            },
                        )
                    } else {
                        v.reference(p, id, OccurrenceKind::Modifier)
                    }
                })
            }
            DraftProviderRoot::SkillUse(id) => self.ref_field(&p, id, OccurrenceKind::SkillUse),
            DraftProviderRoot::SupportAssignment(id) => {
                self.ref_field(&p, id, OccurrenceKind::SupportAssignment)
            }
            DraftProviderRoot::EquipmentUse(id) => {
                self.ref_field(&p, id, OccurrenceKind::EquipmentUse)
            }
            DraftProviderRoot::Allocation(id) => self.ref_field(&p, id, OccurrenceKind::Allocation),
            DraftProviderRoot::Reward(id) => self.ref_field(&p, id, OccurrenceKind::Reward),
            DraftProviderRoot::Pending(value) => self.pending(&p, value, Self::provider_root_value),
        }
    }
    fn provider(&mut self, path: &str, value: &ProviderKeyDraft) -> Result {
        self.provider_root(&format!("{path}.root"), &value.root)?;
        if value.grant_path.members.len() > self.limits.input.max_provider_steps {
            return Err(error(
                &format!("{path}.grant_path"),
                StructuralErrorKind::LimitExceeded,
            ));
        }
        self.list(&format!("{path}.grant_path"), &value.grant_path, Self::slot)
    }
    fn skill_target(&mut self, path: &str, value: &DraftSkillTarget) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftSkillTarget::Authored(id) => self.ref_field(&p, id, OccurrenceKind::SkillUse),
            DraftSkillTarget::Generated(value) => {
                self.provider(&format!("{p}.provider"), &value.provider)?;
                self.slot(&format!("{p}.slot"), &value.slot)
            }
            DraftSkillTarget::Pending(value) => {
                self.pending(&p, value, |v, p, value| v.checked(|c| c.skill(p, value)))
            }
        }
    }
    fn actor(&mut self, path: &str, value: &DraftActorKey) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftActorKey::Player => Ok(()),
            DraftActorKey::Owned(value) => {
                self.provider(&format!("{p}.provider"), &value.provider)?;
                self.slot(&format!("{p}.slot"), &value.slot)
            }
            DraftActorKey::Pending(value) => {
                self.pending(&p, value, |v, p, value| v.checked(|c| c.actor(p, value)))
            }
        }
    }
    fn action(&mut self, path: &str, value: &ActionSelectionDraft) -> Result {
        self.actor(&format!("{path}.action.actor"), &value.action.actor)?;
        self.provider(&format!("{path}.action.provider"), &value.action.provider)?;
        self.slot(&format!("{path}.action.output"), &value.action.output)?;
        self.definition(&format!("{path}.part"), &value.part)?;
        self.definition(&format!("{path}.mode"), &value.mode)?;
        self.definition(&format!("{path}.stat_set"), &value.stat_set)
    }
    fn choice_owner(&mut self, path: &str, value: &DraftChoiceOwner) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftChoiceOwner::Character => Ok(()),
            DraftChoiceOwner::EquipmentUse(id) => {
                self.ref_field(&p, id, OccurrenceKind::EquipmentUse)
            }
            DraftChoiceOwner::Allocation(id) => self.ref_field(&p, id, OccurrenceKind::Allocation),
            DraftChoiceOwner::Skill(value) => self.skill_target(&p, value),
            DraftChoiceOwner::Action(value) => self.action(&p, value),
            DraftChoiceOwner::Provider(value) => self.provider(&p, value),
            DraftChoiceOwner::Pending(value) => self.pending(&p, value, |v, p, value| {
                v.checked(|c| c.choice_owner(p, value))
            }),
        }
    }
    fn assumption_target(&mut self, path: &str, value: &DraftAssumptionTarget) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftAssumptionTarget::Environment | DraftAssumptionTarget::Enemy => Ok(()),
            DraftAssumptionTarget::Actor(value) => self.actor(&p, value),
            DraftAssumptionTarget::Skill(value) => self.skill_target(&p, value),
            DraftAssumptionTarget::Pending(value) => {
                self.pending(&p, value, |v, p, value| match value {
                    AssumptionTarget::Environment | AssumptionTarget::Enemy => Ok(()),
                    AssumptionTarget::Actor(value) => v.checked(|c| c.actor(p, value)),
                    AssumptionTarget::Skill(value) => v.checked(|c| c.skill(p, value)),
                })
            }
        }
    }
    fn usage_target(&mut self, path: &str, value: &DraftUsageTarget) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftUsageTarget::Actor(value) => self.actor(&p, value),
            DraftUsageTarget::Action(value) => self.action(&p, value),
            DraftUsageTarget::Skill(value) => self.skill_target(&p, value),
            DraftUsageTarget::Pending(value) => {
                self.pending(&p, value, |v, p, value| match value {
                    UsageTarget::Actor(value) => v.checked(|c| c.actor(p, value)),
                    UsageTarget::Action(value) => v.checked(|c| c.action(p, value)),
                    UsageTarget::Skill(value) => v.checked(|c| c.skill(p, value)),
                })
            }
        }
    }
    fn metric_target(&mut self, path: &str, value: &DraftMetricTarget) -> Result {
        let p = format!("{path}.value");
        match value {
            DraftMetricTarget::Actor(value) => self.actor(&p, value),
            DraftMetricTarget::Action(value) => self.action(&p, value),
            DraftMetricTarget::Pending(value) => {
                self.pending(&p, value, |v, p, value| match value {
                    MetricTarget::Actor(value) => v.checked(|c| c.actor(p, value)),
                    MetricTarget::Action(value) => v.checked(|c| c.action(p, value)),
                })
            }
        }
    }

    fn item(&mut self, path: &str, value: &ItemDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::Item)?;
        self.definition(&format!("{path}.template"), &value.template)?;
        self.parameters(
            &format!("{path}.parameters"),
            &value.parameters,
            value
                .template
                .to_resolved()
                .map(SlotOwnerDefId::ItemTemplate),
            |owner| matches!(owner, SlotOwnerDefId::ItemTemplate(_)),
        )?;
        self.scalar(&format!("{path}.item_level"), &value.item_level)?;
        self.quality(&format!("{path}.quality"), &value.quality)?;
        self.list(&format!("{path}.modifiers"), &value.modifiers, |v, p, m| {
            if v.gathering {
                v.check
                    .register(&format!("{p}.id"), m.id, OccurrenceKind::Modifier)?;
            }
            v.definition(&format!("{p}.definition"), &m.definition)?;
            v.parameters(
                &format!("{p}.rolls"),
                &m.rolls,
                m.definition.to_resolved().map(SlotOwnerDefId::Modifier),
                |owner| matches!(owner, SlotOwnerDefId::Modifier(_)),
            )
        })
    }
    fn gem(&mut self, path: &str, value: &GemDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::Gem)?;
        self.definition(&format!("{path}.definition"), &value.definition)?;
        self.parameters(
            &format!("{path}.parameters"),
            &value.parameters,
            value.definition.to_resolved().map(SlotOwnerDefId::Gem),
            |owner| matches!(owner, SlotOwnerDefId::Gem(_)),
        )?;
        self.scalar(&format!("{path}.level"), &value.level)?;
        self.quality(&format!("{path}.quality"), &value.quality)
    }
    fn reward(&mut self, path: &str, value: &RewardDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::Reward)?;
        self.definition(&format!("{path}.definition"), &value.definition)?;
        self.parameters(
            &format!("{path}.parameters"),
            &value.parameters,
            value.definition.to_resolved().map(SlotOwnerDefId::Reward),
            |owner| matches!(owner, SlotOwnerDefId::Reward(_)),
        )
    }
    fn equipment(&mut self, path: &str, value: &EquipmentDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::EquipmentUse)?;
        self.ref_field(&format!("{path}.item"), &value.item, OccurrenceKind::Item)?;
        self.destination(&format!("{path}.destination"), &value.destination)?;
        self.scope(&format!("{path}.scope"), &value.scope)
    }
    fn allocation(&mut self, path: &str, value: &AllocationDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::Allocation)?;
        self.definition(&format!("{path}.node"), &value.node)?;
        self.definition(&format!("{path}.pool"), &value.pool)?;
        self.scope(&format!("{path}.scope"), &value.scope)?;
        self.access(&format!("{path}.access"), &value.access)?;
        let mut seen = BTreeSet::new();
        self.list(&format!("{path}.choices"), &value.choices, |v, p, value| {
            v.choice_selection(p, value)?;
            if !v.gathering
                && let DraftField::Known { value: slot } = &value.slot
                && !seen.insert(slot.clone())
            {
                return Err(error(p, StructuralErrorKind::DuplicateAssignment));
            }
            Ok(())
        })
    }
    fn skill(&mut self, path: &str, value: &SkillDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::SkillUse)?;
        self.source(&format!("{path}.source"), &value.source)?;
        self.scalar(&format!("{path}.enabled"), &value.enabled)?;
        self.scope(&format!("{path}.scope"), &value.scope)
    }
    fn support(&mut self, path: &str, value: &SupportDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::SupportAssignment)?;
        self.ref_field(
            &format!("{path}.support"),
            &value.support,
            OccurrenceKind::Gem,
        )?;
        self.skill_target(&format!("{path}.target"), &value.target)?;
        self.scalar(&format!("{path}.enabled"), &value.enabled)
    }
    fn payload(&mut self, path: &str, value: &PayloadDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::PayloadLink)?;
        self.ref_field(
            &format!("{path}.container"),
            &value.container,
            OccurrenceKind::SkillUse,
        )?;
        self.ref_field(
            &format!("{path}.payload"),
            &value.payload,
            OccurrenceKind::SkillUse,
        )?;
        self.definition(&format!("{path}.role"), &value.role)
    }
    fn choices(&mut self, path: &str, values: &DraftList<ChoiceDraft>) -> Result {
        let mut seen = BTreeSet::new();
        self.list(path, values, |v, p, value| {
            v.choice_owner(&format!("{p}.owner"), &value.owner)?;
            v.choice_selection(&format!("{p}.choice"), &value.choice)?;
            if !v.gathering
                && let Some(owner) = value.owner.to_resolved()
                && let DraftField::Known { value: slot } = &value.choice.slot
                && !seen.insert((canonical_choice_owner(&owner), slot.clone()))
            {
                return Err(error(p, StructuralErrorKind::DuplicateAssignment));
            }
            Ok(())
        })
    }
    fn character_preset(&mut self, path: &str, value: &CharacterPresetDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::CharacterPreset)?;
        self.definition(&format!("{path}.class"), &value.class)?;
        self.field(
            &format!("{path}.ascendancy"),
            &value.ascendancy,
            |v, p, id| {
                if let Some(id) = id {
                    v.check.definition(p, id)?;
                }
                Ok(())
            },
        )?;
        self.scalar(&format!("{path}.level"), &value.level)?;
        self.references(
            &format!("{path}.rewards"),
            &value.rewards,
            OccurrenceKind::Reward,
        )
    }
    fn equipment_preset(&mut self, path: &str, value: &EquipmentPresetDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::EquipmentPreset)?;
        self.references(
            &format!("{path}.equipment"),
            &value.equipment,
            OccurrenceKind::EquipmentUse,
        )
    }
    fn allocation_preset(&mut self, path: &str, value: &AllocationPresetDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::AllocationPreset)?;
        self.references(
            &format!("{path}.allocations"),
            &value.allocations,
            OccurrenceKind::Allocation,
        )?;
        self.references(
            &format!("{path}.equipment"),
            &value.equipment,
            OccurrenceKind::EquipmentUse,
        )
    }
    fn skill_preset(&mut self, path: &str, value: &SkillPresetDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::SkillPreset)?;
        self.references(
            &format!("{path}.skills"),
            &value.skills,
            OccurrenceKind::SkillUse,
        )?;
        self.references(
            &format!("{path}.supports"),
            &value.supports,
            OccurrenceKind::SupportAssignment,
        )?;
        self.references(
            &format!("{path}.payload_links"),
            &value.payload_links,
            OccurrenceKind::PayloadLink,
        )
    }
    fn choice_preset(&mut self, path: &str, value: &ChoicePresetDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::ChoicePreset)?;
        self.choices(&format!("{path}.choices"), &value.choices)?;
        self.references(
            &format!("{path}.rewards"),
            &value.rewards,
            OccurrenceKind::Reward,
        )
    }
    fn scenario_preset(&mut self, path: &str, value: &ScenarioPresetDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::ScenarioPreset)?;
        self.historical = true;
        let result = self.scenario(&format!("{path}.scenario"), &value.scenario);
        self.historical = false;
        result
    }
    fn scenario(&mut self, path: &str, value: &ScenarioDraft) -> Result {
        self.check
            .namespace(&format!("{path}.game_version"), &value.game_version)?;
        self.definition(&format!("{path}.enemy.encounter"), &value.enemy.encounter)?;
        self.scalar(&format!("{path}.enemy.level"), &value.enemy.level)?;
        let mut seen = BTreeSet::new();
        self.list(
            &format!("{path}.assumptions"),
            &value.assumptions,
            |v, p, value| {
                v.definition(&format!("{p}.input"), &value.input)?;
                v.assumption_target(&format!("{p}.target"), &value.target)?;
                v.field(&format!("{p}.value"), &value.value, |v, p, value| {
                    v.check.value(p, value)
                })?;
                if !v.gathering
                    && let Some(target) = value.target.to_resolved()
                    && let Some(input) = value.input.to_resolved()
                    && !seen.insert((target, input))
                {
                    return Err(error(p, StructuralErrorKind::DuplicateAssignment));
                }
                Ok(())
            },
        )?;
        let mut seen = BTreeSet::new();
        self.list(&format!("{path}.usage"), &value.usage, |v, p, value| {
            v.definition(&format!("{p}.policy"), &value.policy)?;
            v.usage_target(&format!("{p}.target"), &value.target)?;
            v.parameters(
                &format!("{p}.parameters"),
                &value.parameters,
                value.policy.to_resolved().map(SlotOwnerDefId::UsagePolicy),
                |owner| matches!(owner, SlotOwnerDefId::UsagePolicy(_)),
            )?;
            if !v.gathering
                && let Some(target) = value.target.to_resolved()
                && let Some(policy) = value.policy.to_resolved()
                && !seen.insert((target, policy))
            {
                return Err(error(p, StructuralErrorKind::DuplicateAssignment));
            }
            Ok(())
        })
    }
    fn query_preset(&mut self, path: &str, value: &QueryPresetDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::QueryPreset)?;
        self.historical = true;
        let result = self.queries(&format!("{path}.queries"), &value.queries);
        self.historical = false;
        result
    }
    fn queries(&mut self, path: &str, value: &QueryDraft) -> Result {
        self.check
            .namespace(&format!("{path}.game_version"), &value.game_version)?;
        let mut seen = BTreeSet::new();
        self.list(
            &format!("{path}.requests"),
            &value.requests,
            |v, p, value| {
                v.definition(&format!("{p}.metric"), &value.metric)?;
                v.metric_target(&format!("{p}.target"), &value.target)?;
                if !v.gathering && !seen.insert(value.id.clone()) {
                    return Err(error(p, StructuralErrorKind::DuplicateAssignment));
                }
                Ok(())
            },
        )
    }
    fn saved_variant(&mut self, path: &str, value: &SavedVariantDraft) -> Result {
        self.row(path, value.id, OccurrenceKind::SavedVariant)?;
        self.ref_field(
            &format!("{path}.selection.character"),
            &value.selection.character,
            OccurrenceKind::CharacterPreset,
        )?;
        self.ref_field(
            &format!("{path}.selection.equipment"),
            &value.selection.equipment,
            OccurrenceKind::EquipmentPreset,
        )?;
        self.ref_field(
            &format!("{path}.selection.allocations"),
            &value.selection.allocations,
            OccurrenceKind::AllocationPreset,
        )?;
        self.ref_field(
            &format!("{path}.selection.skills"),
            &value.selection.skills,
            OccurrenceKind::SkillPreset,
        )?;
        self.ref_field(
            &format!("{path}.selection.choices"),
            &value.selection.choices,
            OccurrenceKind::ChoicePreset,
        )?;
        self.ref_field(
            &format!("{path}.selection.active_weapon_loadout"),
            &value.selection.active_weapon_loadout,
            OccurrenceKind::Loadout,
        )?;
        self.ref_field(
            &format!("{path}.selection.scenario"),
            &value.selection.scenario,
            OccurrenceKind::ScenarioPreset,
        )?;
        self.ref_field(
            &format!("{path}.selection.queries"),
            &value.selection.queries,
            OccurrenceKind::QueryPreset,
        )?;
        Ok(())
    }
    fn session(&mut self, value: &DraftSessionInput) -> Result {
        self.owner = None;
        self.list("weapon_loadouts", &value.weapon_loadouts, |v, p, id| {
            if v.gathering {
                v.check.register(p, *id, OccurrenceKind::Loadout)?;
            }
            Ok(())
        })?;
        self.owner = None;
        self.list("items", &value.items, Self::item)?;
        self.owner = None;
        self.list("gems", &value.gems, Self::gem)?;
        self.owner = None;
        self.list("rewards", &value.rewards, Self::reward)?;
        self.owner = None;
        self.list("equipment", &value.equipment, Self::equipment)?;
        self.owner = None;
        self.list("allocations", &value.allocations, Self::allocation)?;
        self.owner = None;
        self.list("skills", &value.skills, Self::skill)?;
        self.owner = None;
        self.list("supports", &value.supports, Self::support)?;
        self.owner = None;
        self.list("payload_links", &value.payload_links, Self::payload)?;
        self.owner = None;
        self.list(
            "character_presets",
            &value.character_presets,
            Self::character_preset,
        )?;
        self.owner = None;
        self.list(
            "equipment_presets",
            &value.equipment_presets,
            Self::equipment_preset,
        )?;
        self.owner = None;
        self.list(
            "allocation_presets",
            &value.allocation_presets,
            Self::allocation_preset,
        )?;
        self.owner = None;
        self.list("skill_presets", &value.skill_presets, Self::skill_preset)?;
        self.owner = None;
        self.list("choice_presets", &value.choice_presets, Self::choice_preset)?;
        self.owner = None;
        self.list(
            "scenario_presets",
            &value.scenario_presets,
            Self::scenario_preset,
        )?;
        self.owner = None;
        self.list("query_presets", &value.query_presets, Self::query_preset)?;
        self.owner = None;
        self.list("saved_variants", &value.saved_variants, Self::saved_variant)?;
        Ok(())
    }
}

fn known_containment(equipment: &[EquipmentDraft]) -> Result {
    // A known container edge remains meaningful when its socket or item is pending.
    let parents: BTreeMap<_, _> = equipment
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match &item.destination {
            DraftEquipmentDestination::ItemSocket {
                container: DraftField::Known { value },
                ..
            } => Some((item.id, (*value, i))),
            _ => None,
        })
        .collect();
    let mut complete = BTreeSet::new();
    for start in parents.keys() {
        let mut path = BTreeSet::new();
        let mut cursor = *start;
        while !complete.contains(&cursor) {
            if !path.insert(cursor) {
                let index = parents[&cursor].1;
                return Err(error(
                    &format!("equipment.members[{index}].destination.value.container"),
                    StructuralErrorKind::ContainmentCycle,
                ));
            }
            let Some((parent, _)) = parents.get(&cursor) else {
                break;
            };
            cursor = *parent;
        }
        complete.extend(path);
    }
    Ok(())
}
