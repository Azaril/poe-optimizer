//! Exact candidate membership and optional semantic ordering. Workers receive reduction indices.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Sources<'a> {
    pub rules: &'a RulePackageInput,
    pub build: &'a BuildInput,
    pub actor_supplies: &'a BTreeMap<OwnedActorKey, ProviderKey>,
    pub skill_supplies: &'a BTreeMap<GeneratedSkillKey, ProviderKey>,
    pub effects: &'a [EffectOccurrenceKey],
    pub appended: &'a [EffectOccurrenceKey],
}
fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}
type Position = (u32, u32, usize, u32, u32);
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum Owner<'a> {
    Definition(&'a DefinitionAddress),
    Slot(&'a SlotAddress),
}
fn owner(subject: &SchemaSubject) -> Owner<'_> {
    match subject {
        SchemaSubject::Definition(id) => Owner::Definition(id),
        SchemaSubject::Slot(id) => Owner::Slot(id),
    }
}
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum Source<'a> {
    Provider(&'a ProviderKey),
    ExistingActor(&'a OwnedDefinitionKey, &'a ActorKey),
    ApplicationGroup(
        &'a ConcreteEntity,
        &'a OwnedDefinitionKey,
        &'a OwnedDefinitionKey,
    ),
}
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum ProducerAddress<'a> {
    Program(Owner<'a>, &'a OwnedDefinitionKey, &'a OwnedDefinitionKey),
    ApplicationGroup(&'a OwnedDefinitionKey, &'a OwnedDefinitionKey),
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
    fn source<'a>(
        &self,
        effect: &'a EffectOccurrenceKey,
        member: &ProgramContributionProducer,
        work: &mut usize,
    ) -> Result<Source<'a>> {
        let origin = &member.origin;
        if let ContributionOrigin::ExistingActor { application } = origin {
            let RuleOrigin::ExistingActor {
                application: actual,
                actor,
            } = &effect.invocation.origin
            else {
                return Err(invalid(
                    "contribution requires its exact existing Actor application",
                ));
            };
            if actual != application
                || actor != &ActorKey::Player
                || effect.invocation.entity != ConcreteEntity::Actor(actor.clone())
            {
                return Err(invalid(
                    "existing Actor contribution occurrence differs from membership",
                ));
            }
            let registry = self
                .rules
                .existing_actor_rules
                .as_ref()
                .ok_or_else(|| invalid("existing Actor contribution registry is absent"))?;
            charge(work, registry.members.len())?;
            let row = registry
                .members
                .iter()
                .find(|r| &r.id == actual)
                .ok_or_else(|| invalid("existing Actor contribution application is absent"))?;
            charge(work, row.targets.len())?;
            if member.owner != SchemaSubject::Definition(row.owner.address())
                || !row.targets.contains(&ExistingActorRuleTarget::Player)
            {
                return Err(invalid(
                    "existing Actor contribution applicability differs from membership",
                ));
            }
            return Ok(Source::ExistingActor(actual, actor));
        }
        let RuleOrigin::Provider { provider } = &effect.invocation.origin else {
            return Err(invalid("contribution requires a reviewed provider origin"));
        };
        charge(work, provider.grant_path.len() + 1)?;
        if let ContributionOrigin::Action { authored, supplies } = origin {
            let ConcreteEntity::Action(action) = &effect.invocation.entity else {
                return Err(invalid(
                    "Action contribution requires its exact Action occurrence",
                ));
            };
            if &action.action.provider != provider {
                return Err(invalid(
                    "Action contribution provider differs from its exact Action",
                ));
            }
            let SlotOwnerDefId::Skill(definition) = &action.action.output.declaration else {
                return Err(invalid("Action contribution requires a Skill-owned output"));
            };
            if member.owner != SchemaSubject::Definition(definition.address())
                && member.owner
                    != SchemaSubject::Slot(ActionOutputDefId::address(&action.action.output))
            {
                return Err(invalid(
                    "Action contribution owner differs from its exact output",
                ));
            }
            charge(work, self.skill_supplies.len())?;
            let mut bound = self.skill_supplies.iter().filter(|(_, p)| *p == provider);
            if let Some((skill, _)) = bound.next() {
                if bound.next().is_some() {
                    return Err(invalid("Action contribution has ambiguous Skill supply"));
                }
                charge(work, supplies.len() + skill.provider.grant_path.len() + 1)?;
                if !supplies.contains(&skill.slot) {
                    return Err(invalid(
                        "Action contribution differs from its validated Skill supply membership",
                    ));
                }
            } else {
                let ProviderRoot::SkillUse(id) = provider.root else {
                    return Err(invalid(
                        "Action contribution has no authored or supplied Skill source",
                    ));
                };
                if !authored || !provider.grant_path.is_empty() {
                    return Err(invalid(
                        "authored Action contribution requires permission for its exact use",
                    ));
                }
                charge(work, self.build.skills.len())?;
                let skill = self
                    .build
                    .skills
                    .iter()
                    .find(|s| s.id == id)
                    .ok_or_else(|| invalid("contribution Action Skill use is absent"))?;
                let AuthoredSkillSource::Direct(actual) = &skill.source else {
                    return Err(invalid(
                        "authored Action contribution requires a direct Skill source",
                    ));
                };
                if actual != definition {
                    return Err(invalid(
                        "authored Action contribution Skill differs from membership",
                    ));
                }
                // Gem-backed skills use their explicit generated supply above;
                // a root selector cannot stand in for that child occurrence.
            }
            return Ok(Source::Provider(provider));
        }
        if let ContributionOrigin::Skill { authored, supplies } = origin {
            let ConcreteEntity::Skill(target) = &effect.invocation.entity else {
                return Err(invalid(
                    "Skill contribution requires its exact Skill occurrence",
                ));
            };
            match target.as_ref() {
                SkillTarget::Generated(skill) => {
                    charge(work, supplies.len() + skill.provider.grant_path.len() + 1)?;
                    if !supplies.contains(&skill.slot)
                        || self.skill_supplies.get(skill.as_ref()) != Some(provider)
                    {
                        return Err(invalid(
                            "supplied Skill contribution differs from the validated skill supply",
                        ));
                    }
                }
                SkillTarget::Authored(id) => {
                    if !authored
                        || !provider.grant_path.is_empty()
                        || provider.root != ProviderRoot::SkillUse(*id)
                    {
                        return Err(invalid(
                            "authored Skill contribution requires permission for its exact direct use",
                        ));
                    }
                    charge(work, self.build.skills.len())?;
                    let skill = self
                        .build
                        .skills
                        .iter()
                        .find(|s| &s.id == id)
                        .ok_or_else(|| invalid("contribution Skill use is absent"))?;
                    let AuthoredSkillSource::Direct(definition) = &skill.source else {
                        return Err(invalid(
                            "authored Skill contribution requires a direct Skill source",
                        ));
                    };
                    if member.owner != SchemaSubject::Definition(definition.address()) {
                        return Err(invalid(
                            "authored Skill contribution definition differs from membership",
                        ));
                    }
                }
            }
            return Ok(Source::Provider(provider));
        }
        if let ContributionOrigin::SuppliedActor { slots } = origin {
            let ConcreteEntity::Actor(ActorKey::Owned(actor)) = &effect.invocation.entity else {
                return Err(invalid(
                    "supplied Actor contribution requires its exact Actor occurrence",
                ));
            };
            charge(work, slots.len() + actor.provider.grant_path.len() + 1)?;
            if !slots.contains(&actor.slot)
                || self.actor_supplies.get(actor.as_ref()) != Some(provider)
            {
                return Err(invalid(
                    "supplied Actor contribution differs from the validated actor supply",
                ));
            }
            // ActorKey retains the parent address. Use the discovered grant
            // relation; raw path equality or assuming a fixed depth is wrong.
            return Ok(Source::Provider(provider));
        }
        if !provider.grant_path.is_empty() {
            return Err(invalid(
                "contribution membership does not admit generated provider paths",
            ));
        }
        match (origin, &provider.root) {
            (ContributionOrigin::Character, ProviderRoot::Character)
            | (ContributionOrigin::Allocation, ProviderRoot::Allocation(_)) => {}
            (ContributionOrigin::Reward, ProviderRoot::Reward(id)) => {
                charge(work, self.build.character.rewards.len())?;
                let reward = self
                    .build
                    .character
                    .rewards
                    .iter()
                    .find(|r| &r.id == id)
                    .ok_or_else(|| invalid("contribution reward selection is absent"))?;
                if member.owner != SchemaSubject::Definition(reward.definition.address()) {
                    return Err(invalid(
                        "contribution reward definition differs from membership",
                    ));
                }
            }
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
        Ok(Source::Provider(provider))
    }
    fn position(
        &self,
        source: Source<'_>,
        origin: &ContributionOrigin,
        policy: &ContributionOrder,
        work: &mut usize,
    ) -> Result<Position> {
        let (slot, modifier) = match (origin, source) {
            (ContributionOrigin::ExistingActor { .. }, Source::ExistingActor(_, _))
            | (ContributionOrigin::SuppliedActor { .. }, Source::Provider(_))
            | (ContributionOrigin::Skill { .. }, Source::Provider(_)) => (0, 0),
            (ContributionOrigin::Action { .. }, Source::Provider(_)) => (0, 0),
            (_, Source::Provider(provider)) => match (origin, &provider.root) {
                (ContributionOrigin::Character, ProviderRoot::Character)
                | (ContributionOrigin::Allocation, ProviderRoot::Allocation(_))
                | (ContributionOrigin::Reward, ProviderRoot::Reward(_)) => (0, 0),
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
            },
            _ => {
                return Err(invalid(
                    "ordered contribution source differs from membership",
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
                let identity = match &member.producer {
                    ContributionProducer::ProgramEffect(p) => {
                        ProducerAddress::Program(owner(&p.owner), &p.program, &p.effect)
                    }
                    ContributionProducer::ApplicationGroup(p) => {
                        ProducerAddress::ApplicationGroup(&p.family, &p.modifier)
                    }
                };
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
            let identity = match &invocation.origin {
                RuleOrigin::EffectApplicationGroup {
                    family, modifier, ..
                } => ProducerAddress::ApplicationGroup(family, modifier),
                _ => ProducerAddress::Program(
                    owner(&invocation.owner),
                    &invocation.program,
                    &effect.effect,
                ),
            };
            let (group, member) = policies
                .get(&identity)
                .ok_or_else(|| invalid("actual contribution has no declared membership"))?;
            let (source, origin) = match &member.producer {
                ContributionProducer::ProgramEffect(p) => {
                    if matches!(p.origin, ContributionOrigin::Action { .. })
                        && invocation.entity != key.entity
                    {
                        return Err(invalid(
                            "Action contribution recipient differs from its exact Action",
                        ));
                    }
                    (self.source(effect, p, work)?, Some(&p.origin))
                }
                ContributionProducer::ApplicationGroup(p) => {
                    let RuleOrigin::EffectApplicationGroup {
                        recipient,
                        family,
                        modifier,
                    } = &invocation.origin
                    else {
                        return Err(invalid(
                            "application membership requires a post-stacking group",
                        ));
                    };
                    if recipient != &key.entity
                        || recipient != &invocation.entity
                        || family != &p.family
                        || modifier != &p.modifier
                    {
                        return Err(invalid(
                            "application contribution differs from its exact recipient group",
                        ));
                    }
                    (Source::ApplicationGroup(recipient, family, modifier), None)
                }
            };
            match (group.ordering, &member.order) {
                (ContributionOrdering::Ordered, Some(policy)) => {
                    let position = if let Some(origin) = origin {
                        self.position(source, origin, policy, work)?
                    } else {
                        if !policy.slot_ranks.is_empty() {
                            return Err(invalid("application groups cannot have equipment ranks"));
                        }
                        (
                            policy.source_rank,
                            0,
                            0,
                            policy.program_rank,
                            policy.effect_rank,
                        )
                    };
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
                    let canonical = (identity, source, &invocation.entity);
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
