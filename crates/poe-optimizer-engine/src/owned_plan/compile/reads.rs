//! Cold binding of exact owned input locations. No read chooses a source/default.
use super::*;

fn constant(value: Option<ParameterValue>) -> PendingRead {
    PendingRead::Ready(ReadBinding::Constant(value))
}
fn level(value: u16) -> Result<ParameterValue> {
    BoundedInteger::new(i64::from(value))
        .map(ParameterValue::Integer)
        .map_err(|e| PlanError::Invalid(e.to_string()))
}
fn row<'a, T>(
    rows: &'a [T],
    work: &mut usize,
    predicate: impl Fn(&T) -> bool,
) -> Result<Option<&'a T>> {
    charge(work, rows.len())?;
    Ok(rows.iter().find(|value| predicate(value)))
}
// These are the four existing logical aliases only. A path, generated skill,
// action, modifier or support never becomes its supplying/root choice owner.
fn canonical_choice(owner: &ChoiceOwner) -> ChoiceOwner {
    if let ChoiceOwner::Provider(provider) = owner
        && provider.grant_path.is_empty()
    {
        match provider.root {
            ProviderRoot::Character => return ChoiceOwner::Character,
            ProviderRoot::EquipmentUse(id) => return ChoiceOwner::EquipmentUse(id),
            ProviderRoot::Allocation(id) => return ChoiceOwner::Allocation(id),
            ProviderRoot::SkillUse(id) => return ChoiceOwner::Skill(SkillTarget::Authored(id)),
            _ => {}
        }
    }
    owner.clone()
}
fn choice_provider_role(root: &ProviderRoot) -> ProviderRole {
    match root {
        ProviderRoot::Character => ProviderRole::Character,
        ProviderRoot::EquipmentUse(_) => ProviderRole::EquipmentUse,
        ProviderRoot::ItemModifier { .. } => ProviderRole::ItemModifier,
        ProviderRoot::SkillUse(_) => ProviderRole::SkillUse,
        ProviderRoot::SupportAssignment(_) => ProviderRole::SupportAssignment,
        ProviderRoot::Allocation(_) => ProviderRole::Allocation,
        ProviderRoot::Reward(_) => ProviderRole::Reward,
    }
}
#[derive(Debug, Eq, PartialEq)]
enum GeneratedChoiceScope {
    Skill,
    Provider,
    Ambiguous,
    Unavailable,
}
fn generated_choice_scope(
    owners: &[ChoiceOwnerScope],
    role: Option<ProviderRole>,
) -> GeneratedChoiceScope {
    let mut skill = false;
    let mut provider = false;
    for owner in owners {
        match owner {
            ChoiceOwnerScope::Skill => skill = true,
            ChoiceOwnerScope::Provider(candidate) if Some(*candidate) == role => provider = true,
            _ => {}
        }
    }
    match (skill, provider) {
        (true, false) => GeneratedChoiceScope::Skill,
        (false, true) => GeneratedChoiceScope::Provider,
        (true, true) => GeneratedChoiceScope::Ambiguous,
        (false, false) => GeneratedChoiceScope::Unavailable,
    }
}
fn direct_root(c: &Context) -> Option<&ProviderRoot> {
    c.provider
        .as_ref()
        .filter(|p| p.grant_path.is_empty())
        .map(|p| &p.root)
}
fn select_parameter(
    rows: &[ParameterAssignment],
    slot: &DeclaredSlot<ParameterSlotDefId>,
    work: &mut usize,
) -> Result<Option<ParameterValue>> {
    Ok(row(rows, work, |v| &v.slot == slot)?.map(|v| v.value.clone()))
}
fn quality_value(
    quality: Option<&QualitySelection>,
    kind: &QualityDefId,
    presence: bool,
) -> PendingRead {
    let selected = quality.filter(|value| &value.kind == kind);
    constant(if presence {
        Some(ParameterValue::Boolean(selected.is_some()))
    } else {
        selected.map(|value| ParameterValue::Quantity(value.amount.clone()))
    })
}
impl<'a, I: DefinitionSchemaIndex> Builder<'a, I> {
    pub(super) fn item_read_record(&mut self, c: &Context) -> Result<Option<&'a ItemRecord>> {
        let id = match direct_root(c) {
            Some(ProviderRoot::EquipmentUse(id)) => *id,
            Some(ProviderRoot::ItemModifier { equipment_use, .. }) => *equipment_use,
            _ => return Ok(None),
        };
        let build = self.request.build().input();
        let Some(usage) = row(&build.equipment, &mut self.work, |value| value.id == id)? else {
            return Ok(None);
        };
        row(&build.items, &mut self.work, |value| value.id == usage.item)
    }
    fn template_read_record(
        &mut self,
        c: &Context,
        owner: &SchemaSubject,
    ) -> Result<Option<&'a ItemRecord>> {
        if !matches!(direct_root(c), Some(ProviderRoot::EquipmentUse(_))) {
            return Ok(None);
        }
        Ok(self
            .item_read_record(c)?
            .filter(|item| owner == &SchemaSubject::Definition(item.template.address())))
    }
    fn gem_read_record(
        &mut self,
        c: &Context,
        owner: &SchemaSubject,
    ) -> Result<Option<&'a GemInstance>> {
        let build = self.request.build().input();
        let gem = match direct_root(c) {
            Some(ProviderRoot::SkillUse(id)) => {
                let Some(skill) = row(&build.skills, &mut self.work, |value| value.id == *id)?
                else {
                    return Ok(None);
                };
                let AuthoredSkillSource::Gem(gem) = &skill.source else {
                    return Ok(None);
                };
                *gem
            }
            Some(ProviderRoot::SupportAssignment(id)) => {
                let Some(support) = row(&build.supports, &mut self.work, |value| value.id == *id)?
                else {
                    return Ok(None);
                };
                support.support
            }
            _ => return Ok(None),
        };
        Ok(row(&build.gems, &mut self.work, |value| value.id == gem)?
            .filter(|gem| owner == &SchemaSubject::Definition(gem.definition.address())))
    }
    fn generated_read_owner(
        &mut self,
        skill: &GeneratedSkillKey,
        definition: &SkillDefId,
    ) -> Result<bool> {
        charge(&mut self.work, 1)?;
        match self.index.slot(&skill.slot) {
            SchemaLookup::Known(schema) if &schema.skill == definition => Ok(true),
            SchemaLookup::Known(_) => Err(PlanError::Invalid(
                "generated read owner differs from supplied skill".into(),
            )),
            SchemaLookup::Missing | SchemaLookup::Unmapped(_) => Ok(false),
            SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                Err(PlanError::Invalid(
                    "generated read has foreign or inconsistent schema index".into(),
                ))
            }
        }
    }
    fn parameter_read(
        &mut self,
        slot: &DeclaredSlot<ParameterSlotDefId>,
        c: &Context,
        owner: &SchemaSubject,
    ) -> Result<PendingRead> {
        if owner_subject(&slot.declaration) != *owner {
            return Err(PlanError::Invalid(
                "parameter declaration differs from program owner".into(),
            ));
        }
        if let (SlotOwnerDefId::Skill(definition), Some(skill)) = (&slot.declaration, &c.skill) {
            if !self.generated_read_owner(skill, definition)? {
                return Ok(missing(PlanGapReason::SchemaUnresolved));
            }
            return Ok(PendingRead::Value(PlanValueKey::SkillParameter {
                skill: Box::new(skill.clone()),
                parameter: slot.clone(),
            }));
        }
        let parameters = match &slot.declaration {
            SlotOwnerDefId::ItemTemplate(_) => self
                .template_read_record(c, owner)?
                .map(|v| v.parameters.as_slice()),
            SlotOwnerDefId::Gem(_) => self
                .gem_read_record(c, owner)?
                .map(|v| v.parameters.as_slice()),
            SlotOwnerDefId::Modifier(definition) => {
                let Some(ProviderRoot::ItemModifier { modifier, .. }) = direct_root(c) else {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                };
                let Some(item) = self.item_read_record(c)? else {
                    return Ok(missing(PlanGapReason::MissingInput));
                };
                row(&item.modifiers, &mut self.work, |v| {
                    v.id == *modifier && &v.definition == definition
                })?
                .map(|v| v.rolls.as_slice())
            }
            SlotOwnerDefId::Reward(definition) => {
                let Some(ProviderRoot::Reward(id)) = direct_root(c) else {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                };
                row(
                    &self.request.build().input().character.rewards,
                    &mut self.work,
                    |v| v.id == *id && &v.definition == definition,
                )?
                .map(|v| v.parameters.as_slice())
            }
            SlotOwnerDefId::UsagePolicy(definition) => {
                let RuleOrigin::Usage { index } = &c.origin else {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                };
                charge(&mut self.work, 1)?;
                self.request
                    .scenario()
                    .input()
                    .usage
                    .get(*index)
                    .filter(|v| &v.policy == definition)
                    .map(|v| v.parameters.as_slice())
            }
            _ => None,
        };
        match parameters {
            Some(parameters) => Ok(constant(select_parameter(
                parameters,
                slot,
                &mut self.work,
            )?)),
            None => Ok(missing(PlanGapReason::MissingInput)),
        }
    }
    fn choice_read(
        &mut self,
        slot: &DeclaredSlot<ChoiceSlotDefId>,
        c: &Context,
        owner: &SchemaSubject,
    ) -> Result<PendingRead> {
        if !matches!(owner, SchemaSubject::Slot(SlotAddress::ActionOutput(_)))
            && owner_subject(&slot.declaration) != *owner
        {
            return Err(PlanError::Invalid(
                "choice declaration differs from program owner".into(),
            ));
        }
        // Action choices are consumed by explicit ActionOutput-owned programs.
        // Definition-owned programs retain their provider/skill choice scope
        // even when executed in an action context; no cross-scope fallback.
        let selected = if let SchemaSubject::Slot(SlotAddress::ActionOutput(output)) = owner {
            let ConcreteEntity::Action(action) = &c.entity else {
                return Ok(missing(PlanGapReason::UnsupportedContext));
            };
            if &action.action.output != output {
                return Err(PlanError::Invalid(
                    "choice output differs from current action".into(),
                ));
            }
            ChoiceOwner::Action(action.clone())
        } else if let (
            SchemaSubject::Definition(DefinitionAddress::Skill(definition)),
            Some(skill),
        ) = (owner, &c.skill)
        {
            if !self.generated_read_owner(skill, definition)? {
                return Ok(missing(PlanGapReason::SchemaUnresolved));
            }
            charge(&mut self.work, 1)?;
            let schema = match self.index.slot(slot) {
                SchemaLookup::Known(schema) => schema,
                SchemaLookup::Missing | SchemaLookup::Unmapped(_) => {
                    return Ok(missing(PlanGapReason::SchemaUnresolved));
                }
                SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                    return Err(PlanError::Invalid(
                        "choice read has foreign or inconsistent schema index".into(),
                    ));
                }
            };
            charge(&mut self.work, schema.owners.len())?;
            // An entered provider path and its parent+SkillSlot identity are not
            // aliases. The declared scope selects one; admitting both requires
            // an explicit read-scope contract that is not present yet.
            match generated_choice_scope(
                &schema.owners,
                c.provider.as_ref().map(|p| choice_provider_role(&p.root)),
            ) {
                GeneratedChoiceScope::Skill => {
                    ChoiceOwner::Skill(SkillTarget::Generated(Box::new(skill.clone())))
                }
                GeneratedChoiceScope::Provider => ChoiceOwner::Provider(
                    c.provider.as_ref().expect("matching provider role").clone(),
                ),
                GeneratedChoiceScope::Ambiguous => {
                    return Ok(missing(PlanGapReason::UnsupportedRelation));
                }
                GeneratedChoiceScope::Unavailable => {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                }
            }
        } else if let Some(provider) = &c.provider {
            canonical_choice(&ChoiceOwner::Provider(provider.clone()))
        } else {
            return Ok(missing(PlanGapReason::UnsupportedContext));
        };
        let build = self.request.build().input();
        charge(&mut self.work, build.choices.len())?;
        let mut value = None;
        for row in &build.choices {
            if row.choice.slot == *slot
                && canonical_choice(&row.owner) == selected
                && value.replace(row.choice.value.clone()).is_some()
            {
                return Err(PlanError::Invalid(
                    "multiple values for one logical choice".into(),
                ));
            }
        }
        if let ChoiceOwner::Allocation(id) = selected
            && let Some(allocation) = row(&build.allocations, &mut self.work, |v| v.id == id)?
        {
            charge(&mut self.work, allocation.choices.len())?;
            for choice in &allocation.choices {
                if choice.slot == *slot && value.replace(choice.value.clone()).is_some() {
                    return Err(PlanError::Invalid(
                        "multiple values for one logical allocation choice".into(),
                    ));
                }
            }
        }
        Ok(constant(value))
    }
    pub(super) fn read(
        &mut self,
        source: &RuleReadSource,
        c: &Context,
        owner: &SchemaSubject,
    ) -> Result<PendingRead> {
        charge(&mut self.work, 1)?;
        Ok(match source {
            RuleReadSource::Parameter { slot } => return self.parameter_read(slot, c, owner),
            RuleReadSource::Choice { slot } => return self.choice_read(slot, c, owner),
            RuleReadSource::CharacterLevel => {
                let character = &self.request.build().input().character;
                if let SchemaSubject::Definition(DefinitionAddress::Class(class)) = owner
                    && class != &character.class
                {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                }
                constant(Some(level(character.level)?))
            }
            RuleReadSource::CharacterClassIs { class } => constant(Some(ParameterValue::Boolean(
                &self.request.build().input().character.class == class,
            ))),
            RuleReadSource::CharacterAscendancyIs { ascendancy } => {
                constant(Some(ParameterValue::Boolean(
                    self.request.build().input().character.ascendancy.as_ref() == Some(ascendancy),
                )))
            }
            RuleReadSource::GemLevel => {
                let Some(gem) = self.gem_read_record(c, owner)? else {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                };
                constant(Some(level(gem.level)?))
            }
            RuleReadSource::ItemLevel => {
                let Some(item) = self.template_read_record(c, owner)? else {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                };
                constant(item.item_level.map(level).transpose()?)
            }
            RuleReadSource::ItemQualityAmount { quality }
            | RuleReadSource::HasItemQuality { quality } => {
                let Some(item) = self.template_read_record(c, owner)? else {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                };
                quality_value(
                    item.quality.as_ref(),
                    quality,
                    matches!(source, RuleReadSource::HasItemQuality { .. }),
                )
            }
            RuleReadSource::GemQualityAmount { quality }
            | RuleReadSource::HasGemQuality { quality } => {
                let Some(gem) = self.gem_read_record(c, owner)? else {
                    return Ok(missing(PlanGapReason::UnsupportedContext));
                };
                quality_value(
                    gem.quality.as_ref(),
                    quality,
                    matches!(source, RuleReadSource::HasGemQuality { .. }),
                )
            }
            RuleReadSource::Stat {
                entity: relative,
                stat,
            } => PendingRead::Value(PlanValueKey::Stat {
                entity: entity(*relative, c)?,
                stat: stat.clone(),
            }),
            RuleReadSource::Capability {
                entity: relative,
                capability,
            } => PendingRead::Value(PlanValueKey::Capability {
                entity: entity(*relative, c)?,
                capability: capability.clone(),
            }),
            RuleReadSource::External {
                entity: relative,
                input,
            } => {
                let target = match entity(*relative, c)? {
                    ConcreteEntity::Actor(actor) => AssumptionTarget::Actor(actor),
                    ConcreteEntity::Enemy => AssumptionTarget::Enemy,
                    ConcreteEntity::Environment => AssumptionTarget::Environment,
                    _ => return Ok(missing(PlanGapReason::UnsupportedContext)),
                };
                constant(
                    row(
                        &self.request.scenario().input().assumptions,
                        &mut self.work,
                        |v| &v.input == input && v.target == target,
                    )?
                    .map(|v| v.value.clone()),
                )
            }
            RuleReadSource::ModifierTransforms { stat, initial } => {
                let target = entity(RuleEntity::Modifier, c)?;
                PendingRead::ModifierTransforms {
                    key: PlanValueKey::Stat {
                        entity: target.clone(),
                        stat: stat.clone(),
                    },
                    initial: Box::new(PlanValueKey::Stat {
                        entity: target,
                        stat: initial.clone(),
                    }),
                }
            }
            RuleReadSource::Contributions {
                entity: relative,
                stat,
                contribution,
                reduction,
                empty,
            } => PendingRead::Contributions(
                ContributionKey {
                    entity: entity(*relative, c)?,
                    stat: stat.clone(),
                    kind: *contribution,
                },
                *reduction,
                empty.clone(),
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_and_entered_provider_choices_use_only_the_declared_scope() {
        let provider = ChoiceOwnerScope::Provider(ProviderRole::SkillUse);
        assert_eq!(
            generated_choice_scope(
                std::slice::from_ref(&provider),
                Some(ProviderRole::SkillUse)
            ),
            GeneratedChoiceScope::Provider
        );
        assert_eq!(
            generated_choice_scope(&[ChoiceOwnerScope::Skill], Some(ProviderRole::SkillUse)),
            GeneratedChoiceScope::Skill
        );
        for scopes in [
            vec![provider.clone(), ChoiceOwnerScope::Skill],
            vec![ChoiceOwnerScope::Skill, provider],
        ] {
            assert_eq!(
                generated_choice_scope(&scopes, Some(ProviderRole::SkillUse)),
                GeneratedChoiceScope::Ambiguous
            );
        }
        assert_eq!(
            generated_choice_scope(
                &[ChoiceOwnerScope::Provider(ProviderRole::SupportAssignment)],
                Some(ProviderRole::SkillUse)
            ),
            GeneratedChoiceScope::Unavailable
        );
        assert_eq!(
            generated_choice_scope(&[ChoiceOwnerScope::Action], Some(ProviderRole::SkillUse)),
            GeneratedChoiceScope::Unavailable
        );
        assert_eq!(
            generated_choice_scope(&[], Some(ProviderRole::SkillUse)),
            GeneratedChoiceScope::Unavailable
        );
        assert_eq!(
            generated_choice_scope(&[ChoiceOwnerScope::Provider(ProviderRole::SkillUse)], None),
            GeneratedChoiceScope::Unavailable
        );
    }
}
