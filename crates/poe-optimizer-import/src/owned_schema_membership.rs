//! Typed, monotonic additions to partial schema membership. This is an offline
//! authoring operation; it never closes coverage or changes scalar descriptors.
use super::{Result, SuccessorBundleError, charge};
use crate::owned_recipe::StagedOwnedRecipe;
use poe_optimizer_core::{data::DataIdentity, owned_schema::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Explicit permission to add members to existing Partial facets of these Known
/// subjects. Endpoints commit the full canonical schemas; no listed subject may
/// be a no-op. Existing coverage gaps, scalar fields and descriptor types remain
/// unchanged. This policy grants neither activation nor numerical completeness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaMembershipRefinement {
    pub schema_version: u32,
    pub before: DataIdentity,
    pub after: DataIdentity,
    pub subjects: Vec<SchemaSubject>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Subject {
    Definition(DefinitionAddress),
    Slot(SlotAddress),
}
impl From<&SchemaSubject> for Subject {
    fn from(value: &SchemaSubject) -> Self {
        match value {
            SchemaSubject::Definition(value) => Self::Definition(value.clone()),
            SchemaSubject::Slot(value) => Self::Slot(value.clone()),
        }
    }
}
fn invalid(message: &'static str) -> SuccessorBundleError {
    SuccessorBundleError::Refinement(message)
}
fn subjects(policy: &SchemaMembershipRefinement) -> Result<BTreeSet<Subject>> {
    let mut subjects = BTreeSet::new();
    for subject in &policy.subjects {
        if !subjects.insert(subject.into()) {
            return Err(invalid("duplicate membership subject"));
        }
    }
    Ok(subjects)
}
fn known<'a, 'b, T>(
    old: &'a mut SchemaState<T>,
    new: &'b SchemaState<T>,
) -> Result<(&'a mut T, &'b T)> {
    match (old, new) {
        (SchemaState::Known(old), SchemaState::Known(new)) => Ok((old, new)),
        _ => Err(invalid("membership refinement requires Known descriptors")),
    }
}

struct Check<'a> {
    left: &'a mut usize,
    changed: bool,
    eligible: bool,
}
impl Check<'_> {
    fn set<T: Clone + Ord>(
        &mut self,
        old: &mut DeclaredSet<T>,
        new: &DeclaredSet<T>,
    ) -> Result<()> {
        charge(self.left, old.members.len(), "membership entries")?;
        charge(self.left, new.members.len(), "membership entries")?;
        if matches!(new.closure, SchemaClosure::Partial { .. }) && !new.members.is_empty() {
            self.eligible = true;
        }
        if old == new {
            return Ok(());
        }
        if old.closure != new.closure || old.is_complete() {
            return Err(invalid(
                "membership refinement changed closure or a Complete set",
            ));
        }
        if new.members.len() <= old.members.len() {
            return Err(invalid("membership refinement is not an addition"));
        }
        let previous: BTreeSet<_> = old.members.iter().collect();
        if previous.len() != old.members.len() {
            return Err(invalid("duplicate previous member"));
        }
        let mut expected = old.members.iter();
        let mut seen = BTreeSet::new();
        for member in &new.members {
            if !seen.insert(member) {
                return Err(invalid("duplicate added member"));
            }
            if previous.contains(member) && expected.next() != Some(member) {
                return Err(invalid("membership refinement reordered existing members"));
            }
        }
        if expected.next().is_some() {
            return Err(invalid("membership refinement removed existing members"));
        }
        old.members.clone_from(&new.members);
        self.changed = true;
        Ok(())
    }
    fn ports(&mut self, old: &mut DeclaredSlots, new: &DeclaredSlots) -> Result<()> {
        self.set(&mut old.parameters, &new.parameters)?;
        self.set(&mut old.choices, &new.choices)?;
        self.set(&mut old.grants, &new.grants)?;
        self.set(&mut old.actors, &new.actors)?;
        self.set(&mut old.skill_grants, &new.skill_grants)?;
        self.set(&mut old.outputs, &new.outputs)?;
        self.set(&mut old.sockets, &new.sockets)
    }
    fn value(&mut self, old: &mut ValueSchema, new: &ValueSchema) -> Result<()> {
        if let (ValueSchema::Option { allowed: old }, ValueSchema::Option { allowed: new }) =
            (old, new)
        {
            self.set(old, new)?;
        }
        Ok(())
    }
}
fn definition(
    old: &DefinitionDescriptor,
    new: &DefinitionDescriptor,
    check: &mut Check<'_>,
) -> Result<()> {
    let mut expected = old.clone();
    match (&mut expected, new) {
        (DefinitionDescriptor::Class(a), DefinitionDescriptor::Class(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.ascendancies, &b.ascendancies)?;
            check.set(&mut a.implicit_passives, &b.implicit_passives)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::Ascendancy(a), DefinitionDescriptor::Ascendancy(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.classes, &b.classes)?;
            check.set(&mut a.implicit_passives, &b.implicit_passives)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::ItemTemplate(a), DefinitionDescriptor::ItemTemplate(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.equipment_slots, &b.equipment_slots)?;
            check.set(&mut a.socket_destinations, &b.socket_destinations)?;
            check.set(&mut a.modifiers, &b.modifiers)?;
            check.set(&mut a.quality.allowed_kinds, &b.quality.allowed_kinds)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::Gem(a), DefinitionDescriptor::Gem(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.skills, &b.skills)?;
            check.set(&mut a.quality.allowed_kinds, &b.quality.allowed_kinds)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::PassiveNode(a), DefinitionDescriptor::PassiveNode(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.pools, &b.pools)?;
            check.set(&mut a.adjacent, &b.adjacent)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::Reward(a), DefinitionDescriptor::Reward(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::Modifier(a), DefinitionDescriptor::Modifier(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::Skill(a), DefinitionDescriptor::Skill(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::UsagePolicy(a), DefinitionDescriptor::UsagePolicy(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.ports(&mut a.declarations, &b.declarations)?;
        }
        (DefinitionDescriptor::Encounter(a), DefinitionDescriptor::Encounter(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.external_inputs, &b.external_inputs)?;
        }
        (DefinitionDescriptor::SkillLinkRole(a), DefinitionDescriptor::SkillLinkRole(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.containers, &b.containers)?;
            check.set(&mut a.payloads, &b.payloads)?;
        }
        (DefinitionDescriptor::ExternalInput(a), DefinitionDescriptor::ExternalInput(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.value(&mut a.value, &b.value)?;
        }
        // These descriptors contain no DeclaredSet: selecting one cannot justify
        // any mutation, even if it currently has the same scalar values.
        (DefinitionDescriptor::PointPool(a), DefinitionDescriptor::PointPool(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::EquipmentSlot(a), DefinitionDescriptor::EquipmentSlot(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::Metric(a), DefinitionDescriptor::Metric(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::Option(a), DefinitionDescriptor::Option(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::ActionPart(a), DefinitionDescriptor::ActionPart(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::ActionMode(a), DefinitionDescriptor::ActionMode(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::ActionStatSet(a), DefinitionDescriptor::ActionStatSet(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::SocketSlot(a), DefinitionDescriptor::SocketSlot(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::Unit(a), DefinitionDescriptor::Unit(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::Quality(a), DefinitionDescriptor::Quality(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::Stat(a), DefinitionDescriptor::Stat(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        (DefinitionDescriptor::Capability(a), DefinitionDescriptor::Capability(b)) => {
            known(&mut a.schema, &b.schema)?;
        }
        _ => return Err(invalid("membership descriptor kind changed")),
    }
    if expected != *new {
        return Err(invalid(
            "membership refinement changed non-membership fields",
        ));
    }
    Ok(())
}
fn slot(old: &SlotDescriptor, new: &SlotDescriptor, check: &mut Check<'_>) -> Result<()> {
    let mut expected = old.clone();
    match (&mut expected, new) {
        (SlotDescriptor::Parameter(a), SlotDescriptor::Parameter(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.value(&mut a.value, &b.value)?;
        }
        (SlotDescriptor::Choice(a), SlotDescriptor::Choice(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.value(&mut a.value, &b.value)?;
        }
        (SlotDescriptor::Grant(a), SlotDescriptor::Grant(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            if let (
                GrantTarget::AllocationAccess { pools: a },
                GrantTarget::AllocationAccess { pools: b },
            ) = (&mut a.target, &b.target)
            {
                check.set(a, b)?;
            }
        }
        (SlotDescriptor::Actor(a), SlotDescriptor::Actor(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.skills, &b.skills)?;
            check.set(&mut a.outputs, &b.outputs)?;
        }
        (SlotDescriptor::SkillGrant(a), SlotDescriptor::SkillGrant(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.outputs, &b.outputs)?;
        }
        (SlotDescriptor::ActionOutput(a), SlotDescriptor::ActionOutput(b)) => {
            let (a, b) = known(&mut a.schema, &b.schema)?;
            check.set(&mut a.parts, &b.parts)?;
            check.set(&mut a.modes, &b.modes)?;
            check.set(&mut a.stat_sets, &b.stat_sets)?;
            check.set(&mut a.choices, &b.choices)?;
        }
        _ => return Err(invalid("membership slot kind changed")),
    }
    if expected != *new {
        return Err(invalid(
            "membership refinement changed non-membership fields",
        ));
    }
    Ok(())
}

pub(super) fn preserve(
    policy: &SchemaMembershipRefinement,
    before: &StagedOwnedRecipe,
    after: &StagedOwnedRecipe,
    left: &mut usize,
) -> Result<(usize, usize)> {
    let mut selected = subjects(policy)?;
    let mut counts = (0, 0);
    for previous in &before.schema().input().definitions {
        let address = previous.address();
        let Some(next) = after.schema().lookup_definition(&address) else {
            return Err(invalid("missing successor definition"));
        };
        if selected.remove(&Subject::Definition(address)) {
            let mut check = Check {
                left,
                changed: false,
                eligible: false,
            };
            definition(previous, next, &mut check)?;
            if !check.changed {
                return Err(invalid("membership subject has no additions"));
            }
            counts.0 += 1;
        } else if previous != next {
            return Err(SuccessorBundleError::ChangedDeclaration);
        }
    }
    for previous in &before.schema().input().slots {
        let address = previous.address();
        let Some(next) = after.schema().lookup_slot(&address) else {
            return Err(invalid("missing successor slot"));
        };
        if selected.remove(&Subject::Slot(address)) {
            let mut check = Check {
                left,
                changed: false,
                eligible: false,
            };
            slot(previous, next, &mut check)?;
            if !check.changed {
                return Err(invalid("membership subject has no additions"));
            }
            counts.1 += 1;
        } else if previous != next {
            return Err(SuccessorBundleError::ChangedDeclaration);
        }
    }
    if !selected.is_empty() {
        return Err(invalid("foreign or absent membership subject"));
    }
    Ok(counts)
}

pub(super) fn validate_current<I: DefinitionSchemaIndex>(
    policy: &SchemaMembershipRefinement,
    index: &I,
) -> Result<()> {
    let mut left = super::SuccessorBundleLimits::default().max_validation_entries;
    charge(&mut left, policy.subjects.len(), "membership subjects")?;
    let selected = subjects(policy)?;
    for subject in selected {
        let mut check = Check {
            left: &mut left,
            changed: false,
            eligible: false,
        };
        match subject {
            Subject::Definition(address) => {
                if address.namespace() != index.namespace() {
                    return Err(invalid("foreign current membership subject"));
                }
                let row = index
                    .lookup_definition(&address)
                    .filter(|row| row.address() == address)
                    .ok_or_else(|| invalid("unknown current membership subject"))?;
                definition(row, row, &mut check)?;
            }
            Subject::Slot(address) => {
                if address.namespace() != index.namespace() {
                    return Err(invalid("foreign current membership subject"));
                }
                let row = index
                    .lookup_slot(&address)
                    .filter(|row| row.address() == address)
                    .ok_or_else(|| invalid("unknown current membership subject"))?;
                slot(row, row, &mut check)?;
            }
        }
        if !check.eligible {
            return Err(invalid(
                "current membership subject has no nonempty Partial facet",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_core::owned_definitions::{
        GameVersionNamespace, OwnedDefinitionKey, SkillDefId,
    };
    fn partial(members: Vec<u32>) -> DeclaredSet<u32> {
        DeclaredSet::partial(
            members,
            vec![SchemaGap {
                subject: SchemaSubject::Definition(
                    SkillDefId::parse(GameVersionNamespace::new("poe2", "test").unwrap(), "skill")
                        .unwrap()
                        .address(),
                ),
                facet: SchemaFacet::InputSchema,
                code: OwnedDefinitionKey::new("unconverted").unwrap(),
            }],
        )
    }
    fn check(
        old: &mut DeclaredSet<u32>,
        new: &DeclaredSet<u32>,
        budget: usize,
    ) -> Result<(bool, bool)> {
        let mut left = budget;
        let mut check = Check {
            left: &mut left,
            changed: false,
            eligible: false,
        };
        check.set(old, new)?;
        Ok((check.changed, check.eligible))
    }
    #[test]
    fn generic_membership_requires_additions_preserves_order_and_never_closes_gaps() {
        let original = partial(vec![2, 4]);
        let mut old = original.clone();
        let expected = partial(vec![1, 2, 3, 4, 5]);
        assert_eq!(check(&mut old, &expected, 100).unwrap(), (true, true));
        assert_eq!(old, expected);
        for members in [
            vec![4, 2, 5],
            vec![2, 5, 6],
            vec![2, 4, 4],
            vec![2],
            vec![4, 2],
        ] {
            assert!(check(&mut original.clone(), &partial(members), 100).is_err());
        }
        let mut changed_gap = expected.clone();
        let SchemaClosure::Partial { gaps } = &mut changed_gap.closure else {
            panic!()
        };
        gaps[0].code = OwnedDefinitionKey::new("different").unwrap();
        assert!(check(&mut original.clone(), &changed_gap, 100).is_err());
        assert!(
            check(
                &mut original.clone(),
                &DeclaredSet::complete(vec![2, 4, 5]),
                100
            )
            .is_err()
        );
        assert!(
            check(
                &mut DeclaredSet::complete(vec![2, 4]),
                &DeclaredSet::complete(vec![2, 4, 5]),
                100
            )
            .is_err()
        );
        assert_eq!(
            check(
                &mut DeclaredSet::complete(vec![2, 4]),
                &DeclaredSet::complete(vec![2, 4]),
                100
            )
            .unwrap(),
            (false, false)
        );
    }
    #[test]
    fn set_work_is_bounded_before_indexing_or_cloning_members() {
        let old = partial(vec![2, 4]);
        let new = partial(vec![1, 2, 3, 4, 5]);
        for budget in [0, 1, 2, 6] {
            let mut candidate = old.clone();
            assert!(check(&mut candidate, &new, budget).is_err());
            assert_eq!(candidate, old);
        }
        check(&mut old.clone(), &new, 7).unwrap();
    }
}
