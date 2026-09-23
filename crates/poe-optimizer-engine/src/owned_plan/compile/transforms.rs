//! Cold binding of ordered sibling-modifier scalar projections.
use super::*;

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn modifier_transform(
        &mut self,
        context: &Context,
        key: &ProgramOccurrenceKey,
        definition: &RuleEffect,
        invocation: usize,
        effect: usize,
    ) -> Result<()> {
        let RuleEffectKind::ProjectModifierTransform {
            stat,
            targets,
            order,
            operation,
            ..
        } = &definition.effect
        else {
            return Err(PlanError::Invalid(
                "expected modifier transform projection".into(),
            ));
        };
        let ConcreteEntity::Modifier(producer) = entity(RuleEntity::Modifier, context)? else {
            return Err(PlanError::Invalid(
                "transform requires a direct modifier producer".into(),
            ));
        };
        let ProviderRoot::ItemModifier {
            equipment_use,
            modifier,
        } = producer.root
        else {
            return Err(PlanError::Invalid(
                "transform requires an item-modifier root".into(),
            ));
        };
        let item = self
            .item_read_record(context)?
            .ok_or_else(|| PlanError::Invalid("transform producer has no owning item".into()))?;
        charge(&mut self.work, item.modifier_order.len())?;
        let position = item
            .modifier_order
            .iter()
            .position(|id| *id == modifier)
            .ok_or_else(|| {
                PlanError::Invalid(
                    "transform producer is absent from semantic modifier order".into(),
                )
            })?;
        // Target definitions select potential sibling occurrences. Their Boolean
        // predicates are ordinary native stat dependencies, not a second selector VM.
        charge(
            &mut self.work,
            targets.len().saturating_add(item.modifiers.len()),
        )?;
        let targets: BTreeMap<_, _> = targets
            .iter()
            .map(|target| (&target.definition, &target.when))
            .collect();
        for sibling in &item.modifiers {
            let Some(predicate) = targets.get(&sibling.definition) else {
                continue;
            };
            if self.effects.len() >= self.limits.max_effects {
                return Err(PlanError::Limit("effects"));
            }
            charge(&mut self.work, 1)?;
            let recipient = root(ProviderRoot::ItemModifier {
                equipment_use,
                modifier: sibling.id,
            });
            let recipient_context = Context {
                origin: RuleOrigin::Provider {
                    provider: recipient.clone(),
                },
                provider: Some(recipient.clone()),
                actor: context.actor.clone(),
                skill: None,
                entity: ConcreteEntity::EquipmentUse(equipment_use),
            };
            let recipient_entity = entity(RuleEntity::Modifier, &recipient_context)?;
            let channel = PlanValueKey::Stat {
                entity: recipient_entity.clone(),
                stat: stat.clone(),
            };
            let sequence = (position, order.get());
            if self
                .transforms
                .get(&channel)
                .is_some_and(|steps| steps.contains_key(&sequence))
            {
                return Err(PlanError::Invalid(
                    "competing transforms at the same producer/channel step".into(),
                ));
            }
            let mut gates = self.context_gates(context)?;
            gates.extend(self.context_gates(&recipient_context)?);
            if let Some(predicate) = predicate {
                gates.push(PendingRead::Value(PlanValueKey::Stat {
                    entity: recipient_entity,
                    stat: predicate.clone(),
                }));
            }
            let index = self.effects.len();
            let mut projection = key.clone();
            projection.origin = RuleOrigin::ModifierTransform {
                producer: producer.clone(),
                recipient,
            };
            self.add_effect(
                EffectNode {
                    key: EffectOccurrenceKey {
                        invocation: projection,
                        effect: definition.id.clone(),
                    },
                    target: BoundEffectTarget::ModifierTransform {
                        key: channel.clone(),
                        operation: *operation,
                        order: *order,
                    },
                    operation: EffectOperation::Program { invocation, effect },
                    gates: vec![],
                    dependencies: vec![],
                },
                gates,
            )?;
            self.transforms.entry(channel).or_default().insert(
                sequence,
                BoundModifierTransform {
                    effect: index,
                    operation: *operation,
                },
            );
        }
        Ok(())
    }
}
