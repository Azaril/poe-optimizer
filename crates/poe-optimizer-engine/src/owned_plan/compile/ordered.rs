//! Candidate-local semantic ordering. Workers receive only ordinary reduction indices.
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
        SchemaSubject::Slot(_) => Err(invalid(
            "ordered contribution requires a direct definition owner",
        )),
    }
}
impl Sources<'_> {
    fn effect(&self, index: usize) -> Result<&EffectOccurrenceKey> {
        if index < self.effects.len() {
            self.effects.get(index)
        } else {
            self.appended.get(index - self.effects.len())
        }
        .ok_or_else(|| invalid("ordered contribution effect index is absent"))
    }
    fn position(
        &self,
        effect: &EffectOccurrenceKey,
        policy: &OrderedContributionOrder,
        work: &mut usize,
    ) -> Result<Position> {
        let RuleOrigin::Provider { provider } = &effect.invocation.origin else {
            return Err(invalid(
                "ordered contribution requires a reviewed provider origin",
            ));
        };
        charge(work, provider.grant_path.len() + 1)?;
        if !provider.grant_path.is_empty() {
            return Err(invalid(
                "ordered contribution policy does not admit generated provider paths",
            ));
        }
        let (slot, modifier) = match (&policy.origin, &provider.root) {
            (OrderedContributionOrigin::Character, ProviderRoot::Character)
            | (OrderedContributionOrigin::Allocation, ProviderRoot::Allocation(_)) => (0, 0),
            (OrderedContributionOrigin::EquipmentUse { slots }, ProviderRoot::EquipmentUse(id)) => {
                (self.equipment(*id, slots, work)?.0, 0)
            }
            (
                OrderedContributionOrigin::ItemModifier { slots },
                ProviderRoot::ItemModifier {
                    equipment_use,
                    modifier,
                },
            ) => {
                let (rank, item) = self.equipment(*equipment_use, slots, work)?;
                charge(work, item.modifier_order.len())?;
                let position = item
                    .modifier_order
                    .iter()
                    .position(|id| id == modifier)
                    .ok_or_else(|| {
                        invalid("ordered contribution modifier has no explicit item order")
                    })?;
                (rank, position)
            }
            _ => {
                return Err(invalid(
                    "ordered contribution provider role differs from policy",
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
    fn equipment<'a>(
        &'a self,
        id: ItemSlotUseId,
        slots: &[OrderedEquipmentSlot],
        work: &mut usize,
    ) -> Result<(u32, &'a ItemRecord)> {
        charge(
            work,
            self.build.equipment.len() + self.build.items.len() + slots.len(),
        )?;
        let usage = self
            .build
            .equipment
            .iter()
            .find(|usage| usage.id == id)
            .ok_or_else(|| invalid("ordered contribution equipment use is absent"))?;
        let EquipmentDestination::CharacterSlot(slot) = &usage.destination else {
            return Err(invalid(
                "ordered contribution policy requires a character equipment slot",
            ));
        };
        let rank = slots
            .iter()
            .find(|entry| &entry.slot == slot)
            .ok_or_else(|| invalid("ordered contribution equipment slot has no semantic rank"))?
            .rank;
        let item = self
            .build
            .items
            .iter()
            .find(|item| item.id == usage.item)
            .ok_or_else(|| invalid("ordered contribution equipment item is absent"))?;
        Ok((rank, item))
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
            .ordered_contributions
            .as_ref()
            .ok_or_else(|| invalid("ordered contribution registry is absent"))?;
        charge(work, registry.members.len())?;
        let query = registry
            .members
            .iter()
            .find(|row| &row.id == query)
            .ok_or_else(|| invalid("ordered contribution query is absent"))?;
        if query.stat != key.stat || query.contribution != key.kind {
            return Err(invalid(
                "ordered contribution channel differs from checked query",
            ));
        }
        charge(work, query.groups.len())?;
        let selected = query
            .groups
            .iter()
            .find(|row| &row.id == group)
            .ok_or_else(|| invalid("ordered contribution group is absent"))?;
        let mut policies = BTreeMap::new();
        for row in &query.groups {
            charge(work, row.members.members.len())?;
            for member in &row.members.members {
                let identity = (owner(&member.owner)?, &member.program, &member.effect);
                if policies
                    .insert(identity, (&row.id, &member.order))
                    .is_some()
                {
                    return Err(invalid(
                        "ordered contribution effect occurs in multiple membership rows",
                    ));
                }
            }
        }
        let mut positions = BTreeMap::<&OwnedDefinitionKey, BTreeMap<Position, usize>>::new();
        let mut seen = BTreeSet::new();
        charge(work, indices.len())?;
        for index in indices {
            if !seen.insert(*index) {
                return Err(invalid("duplicate ordered contribution occurrence"));
            }
            let effect = self.effect(*index)?;
            let invocation = &effect.invocation;
            let identity = (
                owner(&invocation.owner)?,
                &invocation.program,
                &effect.effect,
            );
            let (group, policy) = policies
                .get(&identity)
                .ok_or_else(|| invalid("actual contribution has no ordered membership"))?;
            let position = self.position(effect, policy, work)?;
            if positions
                .entry(group)
                .or_default()
                .insert(position, *index)
                .is_some()
            {
                return Err(invalid("ordered contribution semantic positions are tied"));
            }
        }
        let effects = positions
            .remove(&selected.id)
            .unwrap_or_default()
            .into_values()
            .collect();
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
        let Some(registry) = &rules.input().ordered_contributions else {
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
