//! Explicit usage selections bind one ordinary rule invocation to one occurrence.
//! Policy meanings and values remain in the injected schema and rule package.
use super::*;

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    fn usage_context(
        &mut self,
        index: usize,
        target: &UsageTarget,
    ) -> Result<(SelectorBindingStatus, Option<Context>)> {
        let (status, mut context) = match target {
            UsageTarget::Actor(actor) => self.actor_context(actor)?,
            UsageTarget::Action(action) => {
                let resolved = self.resolver.action(action)?;
                charge(&mut self.work, resolved.work_used())?;
                let status = resolved.status();
                let context = resolved.into_value().map(|resolved| Context {
                    origin: RuleOrigin::Usage { index },
                    provider: Some(action.action.provider.clone()),
                    actor: resolved.expected_actor().clone(),
                    skill: match resolved.provider().exposure() {
                        ProviderExposure::Skill { key, .. } => Some(key.clone()),
                        _ => None,
                    },
                    receiving_skill: None,
                    assigned_skill: None,
                    entity: ConcreteEntity::Action(action.clone()),
                });
                (status, context)
            }
            UsageTarget::Skill(target) => {
                let resolved = self.resolver.skill(target)?;
                charge(&mut self.work, resolved.work_used())?;
                let status = resolved.status();
                let Some(resolved) = resolved.into_value() else {
                    return Ok((status, None));
                };
                let (provider, skill) = match target {
                    SkillTarget::Authored(_) => (resolved.provider().key().clone(), None),
                    SkillTarget::Generated(skill) => {
                        charge(&mut self.work, 1)?;
                        // The saved skill key names its supplying parent and slot.
                        // Gates need the discovered entering grant path as well;
                        // selecting a declared slot cannot activate an occurrence.
                        let Some(provider) = self.skill_supplies.get(skill.as_ref()) else {
                            self.gap(
                                Some(skill.provider.clone()),
                                Some(SchemaSubject::Slot(SkillGrantSlotDefId::address(
                                    &skill.slot,
                                ))),
                                PlanGapReason::UnresolvedActivation,
                            )?;
                            return Ok((SelectorBindingStatus::Unresolved, None));
                        };
                        (provider.clone(), Some(skill.as_ref().clone()))
                    }
                };
                (
                    status,
                    Some(Context {
                        origin: RuleOrigin::Usage { index },
                        provider: Some(provider),
                        actor: resolved.provider().actor().clone(),
                        skill,
                        receiving_skill: None,
                        assigned_skill: None,
                        entity: ConcreteEntity::Skill(Box::new(target.clone())),
                    }),
                )
            }
        };
        if let Some(context) = &mut context {
            context.origin = RuleOrigin::Usage { index };
        }
        Ok((status, context))
    }

    pub(super) fn usage_programs(&mut self) -> Result<()> {
        let selections = &self.request.scenario().input().usage;
        charge(&mut self.work, selections.len())?;
        for (index, selection) in selections.iter().enumerate() {
            let subject = SchemaSubject::Definition(selection.policy.address());
            charge(&mut self.work, self.rules.input().owners.len() + 1)?;
            let Some(owner) = self
                .rules
                .input()
                .owners
                .iter()
                .find(|r| r.owner == subject)
            else {
                self.gap(None, Some(subject), PlanGapReason::MissingPrograms)?;
                continue;
            };
            if !owner.programs.is_complete() {
                self.gap(None, Some(subject.clone()), PlanGapReason::PartialPrograms)?;
            }
            let kind = match &selection.target {
                UsageTarget::Actor(_) => RuleEntityKind::Actor,
                UsageTarget::Action(_) => RuleEntityKind::Action,
                UsageTarget::Skill(_) => RuleEntityKind::Skill,
            };
            let (status, context) = self.usage_context(index, &selection.target)?;
            if context.is_none() && status != SelectorBindingStatus::Unavailable {
                self.gap(
                    None,
                    Some(subject.clone()),
                    PlanGapReason::UnresolvedTopology,
                )?;
            }
            charge(&mut self.work, owner.programs.members.len())?;
            for program in &owner.programs.members {
                // No implicit fanout or coercion between an actor, a skill and
                // its possible outputs. Each program uses the saved target kind.
                if program.context != kind {
                    self.gap(
                        None,
                        Some(subject.clone()),
                        PlanGapReason::UnsupportedContext,
                    )?;
                    continue;
                }
                charge(&mut self.work, program.effects.len())?;
                if program.effects.iter().any(|effect| {
                    !matches!(
                        effect.effect,
                        RuleEffectKind::Derive { .. }
                            | RuleEffectKind::Contribute { .. }
                            | RuleEffectKind::Capability { .. }
                            | RuleEffectKind::Requirement { .. }
                    )
                }) {
                    // A policy is not a supplying provider. Its declared child
                    // slots cannot be attached to the selected occurrence, nor
                    // can it synthesize support or modifier-transform delivery.
                    self.gap(
                        None,
                        Some(subject.clone()),
                        PlanGapReason::UnsupportedRelation,
                    )?;
                    continue;
                }
                if let Some(context) = &context {
                    self.instantiate(subject.clone(), program, context)?;
                }
            }
        }
        Ok(())
    }
}
