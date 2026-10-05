//! Selected raw inputs use the existing generated parameter graph. They neither
//! supply a Skill nor supersede a potential rule producer by evaluation order.
use super::*;

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn validate_generated_input_writers(&mut self) -> Result<()> {
        let Some(input) = &self.request.build().input().generated_inputs else {
            return Ok(());
        };
        if !self.operations.supports_preset_skill_inputs() {
            return Err(PlanError::Invalid(
                "preset generated inputs require operation v19".into(),
            ));
        }
        if input.schema_version != 1 {
            return Err(PlanError::Invalid(
                "unsupported generated input version".into(),
            ));
        }
        charge(&mut self.work, input.bindings.len())?;
        for binding in &input.bindings {
            charge(
                &mut self.work,
                binding.target.provider.grant_path.len() + binding.parameters.len() + 1,
            )?;
            let resolved = self
                .resolver
                .structural_skill(&SkillTarget::Generated(Box::new(binding.target.clone())))?;
            charge(&mut self.work, resolved.work_used())?;
            if resolved.into_value().is_none() {
                self.gap(
                    Some(binding.target.provider.clone()),
                    Some(SchemaSubject::Slot(SkillGrantSlotDefId::address(
                        &binding.target.slot,
                    ))),
                    PlanGapReason::UnresolvedTopology,
                )?;
            }
            let SchemaLookup::Known(supply) = self.index.slot(&binding.target.slot) else {
                return Err(PlanError::Invalid(
                    "generated input supply schema is unavailable".into(),
                ));
            };
            let Some(permission) = &supply.preset_inputs else {
                return Err(PlanError::Invalid(
                    "generated input supply has no preset permission".into(),
                ));
            };
            if permission.schema_version != 1 || !permission.parameters.is_complete() {
                return Err(PlanError::Invalid(
                    "generated input permission is not complete".into(),
                ));
            }
            let owner = owner_subject(&binding.target.slot.declaration);
            charge(&mut self.work, self.rules.input().owners.len())?;
            let programs = self
                .rules
                .input()
                .owners
                .iter()
                .find(|row| row.owner == owner);
            for parameter in &binding.parameters {
                charge(&mut self.work, permission.parameters.members.len() + 1)?;
                if !permission.parameters.members.contains(&parameter.slot) {
                    return Err(PlanError::Invalid(
                        "generated parameter is not preset-authorized".into(),
                    ));
                }
                // The ordinary rule validator requires the exact declaring
                // Definition owner for ProjectSkillParameter. Inspect all its
                // programs/effects, even false branches, inactive roots and
                // Action-context programs absent from the current query set.
                if let Some(programs) = programs {
                    charge(&mut self.work, programs.programs.members.len())?;
                    for program in &programs.programs.members {
                        charge(&mut self.work, program.effects.len())?;
                        if program.effects.iter().any(|effect| matches!(&effect.effect,
                            RuleEffectKind::ProjectSkillParameter { skill, parameter: target, .. }
                                if *skill == binding.target.slot && *target == parameter.slot
                        )) {
                            return Err(PlanError::Invalid("preset generated input competes with a potential provider projection".into()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn generated_inputs(&mut self) -> Result<()> {
        let Some(input) = &self.request.build().input().generated_inputs else {
            return Ok(());
        };
        charge(&mut self.work, input.bindings.len())?;
        for binding in &input.bindings {
            let parent = self.resolver.provider(&binding.target.provider)?;
            charge(
                &mut self.work,
                parent.work_used() + binding.target.provider.grant_path.len() + 1,
            )?;
            let status = parent.status();
            let target = SkillTarget::Generated(Box::new(binding.target.clone()));
            let origin = RuleOrigin::GeneratedInput {
                origin: binding.origin.clone(),
                target: Box::new(binding.target.clone()),
            };
            let gates = if let Some(parent) = parent.into_value() {
                let context = Context {
                    property_owner: None,
                    origin: origin.clone(),
                    provider: Some(binding.target.provider.clone()),
                    actor: parent.actor().clone(),
                    skill: match parent.exposure() {
                        ProviderExposure::Skill { key, .. } => Some(key.clone()),
                        _ => None,
                    },
                    receiving_skill: None,
                    assigned_skill: None,
                    entity: ConcreteEntity::Skill(Box::new(target.clone())),
                };
                // Exactly the supplying-parent gates, as for an ordinary
                // projection. Requiring the destination's own required inputs
                // would create a self-cycle. Its entering grant still gates all
                // ordinary consumers and never comes from this input binding.
                self.context_gates_at(&context, ReadinessPhase::Structural)?
            } else if status == SelectorBindingStatus::Unavailable {
                vec![PendingRead::Ready(ReadBinding::Constant(Some(
                    ParameterValue::Boolean(false),
                )))]
            } else {
                vec![missing(PlanGapReason::UnresolvedTopology)]
            };
            for parameter in &binding.parameters {
                charge(
                    &mut self.work,
                    gates.len() + binding.target.provider.grant_path.len() + 3,
                )?;
                self.add_effect(
                    EffectNode {
                        key: EffectOccurrenceKey {
                            invocation: ProgramOccurrenceKey {
                                origin: origin.clone(),
                                owner: SchemaSubject::Slot(SkillGrantSlotDefId::address(
                                    &binding.target.slot,
                                )),
                                program: OwnedDefinitionKey::new("preset-generated-input")
                                    .expect("static key"),
                                entity: ConcreteEntity::Skill(Box::new(target.clone())),
                            },
                            effect: parameter.slot.slot.key().clone(),
                        },
                        target: BoundEffectTarget::Value {
                            key: PlanValueKey::SkillParameter {
                                skill: Box::new(binding.target.clone()),
                                parameter: parameter.slot.clone(),
                            },
                        },
                        operation: EffectOperation::GeneratedInput {
                            value: parameter.value.clone(),
                        },
                        gates: vec![],
                        dependencies: vec![],
                    },
                    gates.clone(),
                )?;
            }
        }
        Ok(())
    }
}
