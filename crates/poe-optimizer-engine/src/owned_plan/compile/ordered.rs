//! Exact candidate membership and optional semantic ordering. Workers receive reduction indices.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Sources<'a> {
    pub rules: &'a RulePackageInput,
    pub build: &'a BuildInput,
    pub effects: &'a [EffectOccurrenceKey],
    pub appended: &'a [EffectOccurrenceKey],
}
fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}
type Position = (u32, u32, usize, u32, u32);
fn owner(subject: &SchemaSubject) -> Result<&DefinitionAddress> {
    match subject {
        SchemaSubject::Definition(id) => Ok(id),
        SchemaSubject::Slot(_) => Err(invalid("contribution requires a direct definition owner")),
    }
}
impl Sources<'_> {
    fn effect(&self, index: usize) -> Result<&EffectOccurrenceKey> {
        if index < self.effects.len() {
            self.effects.get(index)
        } else {
            self.appended.get(index - self.effects.len())
        }
        .ok_or_else(|| invalid("contribution effect index is absent"))
    }
    fn provider<'a>(
        &self,
        effect: &'a EffectOccurrenceKey,
        origin: &ContributionOrigin,
        work: &mut usize,
    ) -> Result<&'a ProviderKey> {
        let RuleOrigin::Provider { provider } = &effect.invocation.origin else {
            return Err(invalid("contribution requires a reviewed provider origin"));
        };
        charge(work, provider.grant_path.len() + 1)?;
        if !provider.grant_path.is_empty() {
            return Err(invalid(
                "contribution membership does not admit generated provider paths",
            ));
        }
        match (origin, &provider.root) {
            (ContributionOrigin::Character, ProviderRoot::Character)
            | (ContributionOrigin::Allocation, ProviderRoot::Allocation(_)) => {}
            (ContributionOrigin::EquipmentUse { slots }, ProviderRoot::EquipmentUse(id)) => {
                self.equipment(*id, slots, work)?;
            }
            (
                ContributionOrigin::ItemModifier { slots },
                ProviderRoot::ItemModifier {
                    equipment_use,
                    modifier,
                },
            ) => {
                let (_, item) = self.equipment(*equipment_use, slots, work)?;
                charge(work, item.modifiers.len())?;
                if !item.modifiers.iter().any(|row| &row.id == modifier) {
                    return Err(invalid(
                        "contribution modifier is absent from its exact item",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    "contribution provider role differs from membership",
                ));
            }
        }
        Ok(provider)
    }
    fn position(
        &self,
        provider: &ProviderKey,
        origin: &ContributionOrigin,
        policy: &ContributionOrder,
        work: &mut usize,
    ) -> Result<Position> {
        let (slot, modifier) = match (origin, &provider.root) {
            (ContributionOrigin::Character, ProviderRoot::Character)
            | (ContributionOrigin::Allocation, ProviderRoot::Allocation(_)) => (0, 0),
            (ContributionOrigin::EquipmentUse { slots }, ProviderRoot::EquipmentUse(id)) => {
                let (slot, _) = self.equipment(*id, slots, work)?;
                (Self::slot_rank(slot, policy, work)?, 0)
            }
            (
                ContributionOrigin::ItemModifier { slots },
                ProviderRoot::ItemModifier {
                    equipment_use,
                    modifier,
                },
            ) => {
                let (slot, item) = self.equipment(*equipment_use, slots, work)?;
                charge(work, item.modifier_order.len())?;
                let position = item
                    .modifier_order
                    .iter()
                    .position(|id| id == modifier)
                    .ok_or_else(|| {
                        invalid("ordered contribution modifier has no explicit item order")
                    })?;
                (Self::slot_rank(slot, policy, work)?, position)
            }
            _ => {
                return Err(invalid(
                    "ordered contribution provider role differs from membership",
                ));
            }
        };
        Ok((
            policy.source_rank,
            slot,
            modifier,
            policy.program_rank,
            policy.effect_rank,
        ))
    }
    fn slot_rank(
        slot: &EquipmentSlotDefId,
        policy: &ContributionOrder,
        work: &mut usize,
    ) -> Result<u32> {
        charge(work, policy.slot_ranks.len())?;
        policy
            .slot_ranks
            .iter()
            .find(|entry| &entry.slot == slot)
            .map(|entry| entry.rank)
            .ok_or_else(|| invalid("ordered contribution equipment slot has no semantic rank"))
    }
    fn equipment<'a>(
        &'a self,
        id: ItemSlotUseId,
        slots: &[EquipmentSlotDefId],
        work: &mut usize,
    ) -> Result<(&'a EquipmentSlotDefId, &'a ItemRecord)> {
        charge(
            work,
            self.build.equipment.len() + self.build.items.len() + slots.len(),
        )?;
        let usage = self
            .build
            .equipment
            .iter()
            .find(|usage| usage.id == id)
            .ok_or_else(|| invalid("contribution equipment use is absent"))?;
        let EquipmentDestination::CharacterSlot(slot) = &usage.destination else {
            return Err(invalid(
                "contribution membership requires a character equipment slot",
            ));
        };
        if !slots.contains(slot) {
            return Err(invalid("contribution equipment slot is outside membership"));
        }
        let item = self
            .build
            .items
            .iter()
            .find(|item| item.id == usage.item)
            .ok_or_else(|| invalid("contribution equipment item is absent"))?;
        Ok((slot, item))
    }
    /// Check every concrete channel even when no program reads its query. This
    /// also runs after support suffix expansion; inactive effects still belong
    /// to the census and cannot disappear through lazy runtime evaluation.
    pub(super) fn validate_inventory(
        &self,
        contributions: &BTreeMap<ContributionKey, Vec<usize>>,
        complete: bool,
        work: &mut usize,
    ) -> Result<()> {
        let Some(registry) = &self.rules.contribution_queries else {
            return Ok(());
        };
        charge(work, registry.members.len())?;
        for query in &registry.members {
            let group = query
                .groups
                .first()
                .ok_or_else(|| invalid("contribution query has no group"))?;
            charge(work, contributions.len())?;
            for (key, indices) in contributions {
                if key.stat == query.stat && key.kind == query.contribution {
                    self.bind(key, &query.id, &group.id, indices, complete, work)?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn bind(
        &self,
        key: &ContributionKey,
        query: &OwnedDefinitionKey,
        group: &OwnedDefinitionKey,
        indices: &[usize],
        complete: bool,
        work: &mut usize,
    ) -> Result<ReadBinding> {
        let registry = self
            .rules
            .contribution_queries
            .as_ref()
            .ok_or_else(|| invalid("contribution query registry is absent"))?;
        charge(work, registry.members.len())?;
        let query = registry
            .members
            .iter()
            .find(|row| &row.id == query)
            .ok_or_else(|| invalid("contribution query is absent"))?;
        if query.stat != key.stat || query.contribution != key.kind {
            return Err(invalid("contribution channel differs from checked query"));
        }
        charge(work, query.groups.len())?;
        let selected = query
            .groups
            .iter()
            .find(|row| &row.id == group)
            .ok_or_else(|| invalid("contribution group is absent"))?;
        let mut policies = BTreeMap::new();
        for row in &query.groups {
            charge(work, row.members.members.len())?;
            for member in &row.members.members {
                let identity = (owner(&member.owner)?, &member.program, &member.effect);
                if policies.insert(identity, (row, member)).is_some() {
                    return Err(invalid(
                        "contribution effect occurs in multiple membership rows",
                    ));
                }
            }
        }
        let mut positions = BTreeMap::<&OwnedDefinitionKey, BTreeMap<Position, usize>>::new();
        let mut unordered = BTreeMap::new();
        let mut seen = BTreeSet::new();
        charge(work, indices.len())?;
        for index in indices {
            if !seen.insert(*index) {
                return Err(invalid("duplicate contribution occurrence"));
            }
            let effect = self.effect(*index)?;
            let invocation = &effect.invocation;
            let identity = (
                owner(&invocation.owner)?,
                &invocation.program,
                &effect.effect,
            );
            let (group, member) = policies
                .get(&identity)
                .ok_or_else(|| invalid("actual contribution has no declared membership"))?;
            let provider = self.provider(effect, &member.origin, work)?;
            match (group.ordering, &member.order) {
                (ContributionOrdering::Ordered, Some(policy)) => {
                    let position = self.position(provider, &member.origin, policy, work)?;
                    if positions
                        .entry(&group.id)
                        .or_default()
                        .insert(position, *index)
                        .is_some()
                    {
                        return Err(invalid("ordered contribution semantic positions are tied"));
                    }
                }
                (ContributionOrdering::Unordered, None) => {
                    // Stable identity order makes binding structure deterministic.
                    // It has no semantic rank or effect on Any or diagnostic priority.
                    let canonical = (identity, provider, &invocation.entity);
                    if unordered
                        .entry(&group.id)
                        .or_insert_with(BTreeMap::new)
                        .insert(canonical, *index)
                        .is_some()
                    {
                        return Err(invalid(
                            "duplicate unordered contribution occurrence identity",
                        ));
                    }
                }
                _ => {
                    return Err(invalid(
                        "contribution ordering policy differs from membership",
                    ));
                }
            }
        }
        let effects = match selected.ordering {
            ContributionOrdering::Ordered => positions
                .remove(&selected.id)
                .unwrap_or_default()
                .into_values()
                .collect(),
            ContributionOrdering::Unordered => unordered
                .remove(&selected.id)
                .unwrap_or_default()
                .into_values()
                .collect(),
        };
        Ok(ReadBinding::Reduction {
            effects,
            reduction: selected.reduction,
            empty: selected.empty.clone(),
            complete: complete
                && registry.is_complete()
                && query.groups.iter().all(|row| row.members.is_complete()),
        })
    }
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn ordered_coverage(&mut self) -> Result<()> {
        let rules = self.rules;
        let Some(registry) = &rules.input().contribution_queries else {
            return Ok(());
        };
        if !registry.is_complete() {
            self.gap(None, None, PlanGapReason::IncompleteContributors)?;
        }
        charge(&mut self.work, registry.members.len())?;
        for query in &registry.members {
            charge(&mut self.work, query.groups.len())?;
            if query
                .groups
                .iter()
                .any(|group| !group.members.is_complete())
            {
                self.gap(
                    None,
                    Some(SchemaSubject::Definition(query.stat.address())),
                    PlanGapReason::IncompleteContributors,
                )?;
            }
        }
        Ok(())
    }
}
