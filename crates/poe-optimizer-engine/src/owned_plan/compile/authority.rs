//! Cold permission checks cover the authored inventory, including unused reads.
//! Actor identity selects an input lane; it never grants source or target rights.
use super::*;

fn local(relative: RuleEntity, actor: Option<&ActorKey>) -> bool {
    match relative {
        RuleEntity::Current | RuleEntity::Actor => actor.is_some(),
        RuleEntity::Player => actor == Some(&ActorKey::Player),
        RuleEntity::Enemy => actor == Some(&ActorKey::ScenarioEnemy),
        _ => false,
    }
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn external_authority(
        &mut self,
        read: &RuleReadSource,
        context: &Context,
    ) -> Result<()> {
        let RuleReadSource::External {
            entity: relative,
            input,
        } = read
        else {
            return Ok(());
        };
        let target = match entity(*relative, context)? {
            ConcreteEntity::Actor(actor) => AssumptionTargetKind::for_actor(&actor),
            ConcreteEntity::Environment => AssumptionTargetKind::Environment,
            _ => {
                return Err(PlanError::Invalid(
                    "external read has no admitted role".into(),
                ));
            }
        };
        let SchemaLookup::Known(schema) = self.index.definition(input) else {
            return Err(PlanError::Invalid(
                "external input schema is not known".into(),
            ));
        };
        charge(&mut self.work, schema.targets.len() + 1)?;
        if !schema.targets.contains(&target) {
            return Err(PlanError::Invalid(
                "external input is not admitted for the exact Actor role".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn program_authority(
        &mut self,
        program: &RuleProgram,
        context: &Context,
    ) -> Result<()> {
        charge(
            &mut self.work,
            program.reads.len() + program.effects.len() + 1,
        )?;
        let shared = matches!(context.origin, RuleOrigin::ExistingActor { .. });
        let encounter = matches!(context.origin, RuleOrigin::Encounter);
        for read in &program.reads {
            self.external_authority(&read.source, context)?;
            if !shared && !encounter {
                continue;
            }
            let permitted = match &read.source {
                RuleReadSource::CharacterLevel | RuleReadSource::PlayerEquipmentSlot { .. } => {
                    shared && context.actor == Some(ActorKey::Player)
                }
                RuleReadSource::CharacterClassIs { .. }
                | RuleReadSource::CharacterAscendancyIs { .. } => {
                    encounter || (shared && context.actor == Some(ActorKey::Player))
                }
                RuleReadSource::EnemyLevel => {
                    encounter
                        || matches!(
                            context.actor,
                            Some(ActorKey::Player | ActorKey::ScenarioEnemy)
                        )
                }
                RuleReadSource::Stat { entity, .. }
                | RuleReadSource::Capability { entity, .. }
                | RuleReadSource::Contributions { entity, .. }
                | RuleReadSource::ContributionQuery { entity, .. }
                | RuleReadSource::ContributionSelection { entity, .. } => {
                    local(*entity, context.actor.as_ref())
                        || (encounter
                            && context.actor.is_none()
                            && matches!(entity, RuleEntity::Current | RuleEntity::Environment))
                }
                RuleReadSource::External { entity, .. } => {
                    local(*entity, context.actor.as_ref())
                        || (shared
                            && context.actor == Some(ActorKey::Player)
                            && matches!(entity, RuleEntity::Enemy | RuleEntity::Environment))
                        || (encounter
                            && (matches!(entity, RuleEntity::Enemy | RuleEntity::Environment)
                                || (context.actor.is_none()
                                    && matches!(entity, RuleEntity::Current))))
                }
                _ => false,
            };
            if !permitted {
                return Err(PlanError::Invalid(
                    "program read exceeds its bound Actor role authority".into(),
                ));
            }
        }
        if shared || encounter {
            for effect in &program.effects {
                let permitted = match &effect.effect {
                    RuleEffectKind::Contribute { entity, .. }
                    | RuleEffectKind::Derive { entity, .. }
                    | RuleEffectKind::Capability { entity, .. } => {
                        (!shared || !matches!(effect.effect, RuleEffectKind::Capability { .. }))
                            && (local(*entity, context.actor.as_ref())
                                || (encounter
                                    && context.actor.is_none()
                                    && matches!(
                                        entity,
                                        RuleEntity::Current | RuleEntity::Environment
                                    )))
                    }
                    RuleEffectKind::Requirement { .. } => encounter,
                    _ => false,
                };
                if !permitted {
                    return Err(PlanError::Invalid(
                        "program effect exceeds its bound Actor role authority".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}
