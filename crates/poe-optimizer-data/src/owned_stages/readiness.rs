//! Checked readiness authority is distinct from scheduling and runtime coverage.
use super::*;
use poe_optimizer_core::owned_build::DeclaredSlot;

#[derive(Clone, Debug, Default)]
pub(super) struct ReadinessIndex {
    pub programs: BTreeMap<(OwnerKey, OwnedDefinitionKey), usize>,
    pub skills: BTreeMap<SkillDefId, usize>,
    pub parameters: BTreeMap<(SkillDefId, DeclaredSlot<ParameterSlotDefId>), ReadinessPhase>,
}
pub(super) fn charge(
    input: &EvaluationStagesInput,
    used: &mut StageStorageUse,
    limits: StageStorageLimits,
) -> Result<()> {
    if let Some(readiness) = &input.readiness {
        used.entries(readiness.skills.len(), limits)?;
        used.entries(readiness.programs.members.len(), limits)?;
        for skill in &readiness.skills {
            used.entries(skill.parameters.members.len(), limits)?;
        }
        for program in &readiness.programs.members {
            used.entries(program.outputs.len(), limits)?;
        }
    }
    Ok(())
}
pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &mut EvaluationStagesInput,
    index: &I,
    rules: &OwnedRulePackage,
    used: &mut StageStorageUse,
    limits: StageStorageLimits,
) -> Result<ReadinessIndex> {
    let Some(readiness) = &mut input.readiness else {
        return Ok(ReadinessIndex::default());
    };
    if !input.programs.is_complete() || !readiness.programs.is_complete() {
        return Err(StageStorageError::Invalid(
            "readiness requires complete program classification",
        ));
    }

    let mut result = ReadinessIndex::default();
    readiness.skills.sort_by(|a, b| a.skill.cmp(&b.skill));
    for (i, row) in readiness.skills.iter_mut().enumerate() {
        if result.skills.insert(row.skill.clone(), i).is_some() {
            return Err(StageStorageError::Invalid(
                "duplicate generated skill readiness",
            ));
        }
        let schema = known(index.definition(&row.skill))?;
        if !schema.declarations.parameters.is_complete() || !row.parameters.is_complete() {
            return Err(StageStorageError::Invalid(
                "readiness requires complete required-input inventory",
            ));
        }
        used.work(schema.declarations.parameters.members.len(), limits)?;
        let mut required = BTreeSet::new();
        for parameter in &schema.declarations.parameters.members {
            let declaration = known(index.slot(parameter))?;
            if declaration.presence == SlotPresence::RequiredOnce {
                required.insert(parameter.clone());
            }
        }
        row.parameters
            .members
            .sort_by(|a, b| a.parameter.cmp(&b.parameter));
        for parameter in &row.parameters.members {
            if parameter.parameter.declaration != SlotOwnerDefId::Skill(row.skill.clone())
                || !required.remove(&parameter.parameter)
            {
                return Err(StageStorageError::Invalid(
                    "readiness parameter is duplicate, foreign, optional or undeclared",
                ));
            }
            result.parameters.insert(
                (row.skill.clone(), parameter.parameter.clone()),
                parameter.phase,
            );
        }
        if !required.is_empty() {
            return Err(StageStorageError::Invalid(
                "readiness omits a required parameter",
            ));
        }
    }
    let mut known_programs = BTreeMap::new();
    used.work(rules.input().owners.len(), limits)?;
    for owner in &rules.input().owners {
        used.work(owner.programs.members.len(), limits)?;
        for p in &owner.programs.members {
            known_programs.insert(
                (owner_key(&owner.owner), p.id.clone()),
                (p, owner.programs.is_complete()),
            );
        }
    }
    readiness
        .programs
        .members
        .sort_by(|a, b| (owner_key(&a.owner), &a.program).cmp(&(owner_key(&b.owner), &b.program)));
    // Final writes retain their full declared destination. Two sibling actor or
    // Skill supplies may legitimately project the same stat/parameter family.
    let mut writers = BTreeMap::new();
    let mut early_channels = BTreeSet::new();
    let mut execution_channels = BTreeSet::new();
    for (i, row) in readiness.programs.members.iter_mut().enumerate() {
        let key = (owner_key(&row.owner), row.program.clone());
        let Some((program, complete)) = known_programs.get(&key) else {
            return Err(StageStorageError::Invalid("unknown readiness program"));
        };
        let source_role = matches!(
            row.role,
            ReadinessProgramRole::SourceSupportedProperty
                | ReadinessProgramRole::SourceExternalProperty
                | ReadinessProgramRole::SourceFinalInputAssembly
        );
        if (source_role && input.schema_version != OWNED_EVALUATION_STAGES_V3)
            || (program.uses_source_property_scopes() && !source_role)
        {
            return Err(StageStorageError::Invalid(
                "source property scope requires an explicit V3 source role",
            ));
        }
        if source_role && row.phase != ReadinessPhase::Preparation {
            return Err(StageStorageError::Invalid(
                "source property roles require preparation readiness",
            ));
        }
        if result.programs.insert(key.clone(), i).is_some() {
            return Err(StageStorageError::Invalid("duplicate readiness program"));
        }
        if row.role == ReadinessProgramRole::Execution {
            if row.phase != ReadinessPhase::Execution || !row.outputs.is_empty() {
                return Err(StageStorageError::Invalid(
                    "execution programs cannot request earlier readiness or output overrides",
                ));
            }
        } else if row.phase == ReadinessPhase::Execution || !complete {
            return Err(StageStorageError::Invalid(
                "early readiness needs an early phase and complete owner programs",
            ));
        }
        if matches!(
            row.role,
            ReadinessProgramRole::SupportPreparationApplicability
                | ReadinessProgramRole::SupportedPreparationProperty
                | ReadinessProgramRole::SourceSupportedProperty
        ) {
            if row.phase != ReadinessPhase::Preparation {
                return Err(StageStorageError::Invalid(
                    "support preparation roles require preparation readiness",
                ));
            }
            let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) = &row.owner else {
                return Err(StageStorageError::Invalid(
                    "support preparation role requires support Gem owner",
                ));
            };
            let schema = known(index.definition(gem))?;
            used.work(schema.roles.len(), limits)?;
            if !schema.roles.contains(&AuthoredGemRole::SupportAssignment) {
                return Err(StageStorageError::Invalid(
                    "support preparation role requires support assignment schema",
                ));
            }
        }
        used.work(program.effects.len(), limits)?;
        let mut actual = BTreeSet::new();
        for effect in &program.effects {
            let channel = effect_channel(&effect.effect, program.context)?;
            if row.role != ReadinessProgramRole::Execution {
                validate_effect(row.role, &effect.effect, program.context, index)?;
            }
            if let Some(channel) = channel {
                if row.role == ReadinessProgramRole::Execution {
                    execution_channels.insert(channel.clone());
                } else {
                    early_channels.insert(channel.clone());
                }
                actual.insert(channel);
            }
            // Conditions never make competing potential final producers safe.
            if let Some(destination) = final_destination(&effect.effect, program.context)? {
                let early = row.phase != ReadinessPhase::Execution;
                if let Some(previous_early) = writers.insert((key.0.clone(), destination), early)
                    && (previous_early || early)
                {
                    return Err(StageStorageError::Invalid(
                        "competing potential early final producers",
                    ));
                }
            }
        }
        if row.role == ReadinessProgramRole::SupportPreparationApplicability
            && (program.effects.len() != 1 || program.effects[0].when.is_some())
        {
            return Err(StageStorageError::Invalid(
                "preparation applicability must be one unguarded final producer",
            ));
        }
        row.outputs.sort();
        if row.outputs.windows(2).any(|v| v[0] == v[1]) {
            return Err(StageStorageError::Invalid(
                "duplicate readiness output channel",
            ));
        }
        for output in &row.outputs {
            check_channel(output, index, used, limits)?;
        }
        if row.role != ReadinessProgramRole::Execution
            && actual != row.outputs.iter().cloned().collect()
        {
            return Err(StageStorageError::Invalid(
                "readiness outputs do not exactly describe potential writes",
            ));
        }
    }
    if result.programs.len() != known_programs.len() {
        return Err(StageStorageError::Invalid("readiness omits known programs"));
    }
    // A channel classified as an ordinary execution output cannot acquire early
    // authority by another producer relabeling it as a preparation fact.
    used.work(early_channels.len(), limits)?;
    if early_channels
        .iter()
        .any(|c| execution_channels.contains(c))
    {
        return Err(StageStorageError::Invalid(
            "preparation and execution output roles overlap",
        ));
    }
    Ok(result)
}
fn effect_channel(
    effect: &RuleEffectKind,
    context: RuleEntityKind,
) -> Result<Option<StageChannel>> {
    Ok(Some(match effect {
        RuleEffectKind::Derive { entity, stat, .. } => StageChannel::Stat {
            scope: scope(*entity, context, None)?,
            stat: stat.clone(),
        },
        RuleEffectKind::Contribute {
            entity,
            stat,
            contribution,
            ..
        } => StageChannel::Contributions {
            scope: scope(*entity, context, None)?,
            stat: stat.clone(),
            contribution: *contribution,
        },
        RuleEffectKind::Capability {
            entity, capability, ..
        } => StageChannel::Capability {
            scope: scope(*entity, context, None)?,
            capability: capability.clone(),
        },
        RuleEffectKind::ActivateGrant { slot, .. } => StageChannel::Grant { slot: slot.clone() },
        RuleEffectKind::ProjectSkillParameter { parameter, .. } => StageChannel::SkillParameter {
            parameter: parameter.clone(),
        },
        RuleEffectKind::ProjectActorStat { stat, .. } => StageChannel::Stat {
            scope: RuleEntityKind::Actor,
            stat: stat.clone(),
        },
        RuleEffectKind::ProjectModifierTransform { stat, .. } => {
            StageChannel::ModifierTransforms { stat: stat.clone() }
        }
        RuleEffectKind::SupportApplicability { .. } | RuleEffectKind::Requirement { .. } => {
            return Ok(None);
        }
    }))
}
#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum FinalDestination {
    Channel(RuleEntity, StageChannel),
    Actor(DeclaredSlot<ActorSlotDefId>, StatDefId),
    Skill(
        DeclaredSlot<SkillGrantSlotDefId>,
        DeclaredSlot<ParameterSlotDefId>,
    ),
}
fn final_destination(
    effect: &RuleEffectKind,
    context: RuleEntityKind,
) -> Result<Option<FinalDestination>> {
    Ok(match effect {
        RuleEffectKind::Derive { entity, .. } | RuleEffectKind::Capability { entity, .. } => {
            effect_channel(effect, context)?
                .map(|channel| FinalDestination::Channel(*entity, channel))
        }
        RuleEffectKind::ActivateGrant { .. } => effect_channel(effect, context)?
            .map(|channel| FinalDestination::Channel(RuleEntity::Current, channel)),
        RuleEffectKind::ProjectActorStat { actor, stat, .. } => {
            Some(FinalDestination::Actor(actor.clone(), stat.clone()))
        }
        RuleEffectKind::ProjectSkillParameter {
            skill, parameter, ..
        } => Some(FinalDestination::Skill(skill.clone(), parameter.clone())),
        _ => None,
    })
}
fn validate_effect<I: DefinitionSchemaIndex>(
    role: ReadinessProgramRole,
    effect: &RuleEffectKind,
    context: RuleEntityKind,
    index: &I,
) -> Result<()> {
    let allowed = match role {
        ReadinessProgramRole::Execution => true,
        ReadinessProgramRole::PreparationFacts => match effect {
            RuleEffectKind::Derive { entity, .. }
            | RuleEffectKind::Contribute { entity, .. }
            | RuleEffectKind::Capability { entity, .. } => matches!(
                scope(*entity, context, None)?,
                RuleEntityKind::Actor | RuleEntityKind::Skill | RuleEntityKind::SupportOrigin
            ),
            RuleEffectKind::ActivateGrant { .. } | RuleEffectKind::ProjectActorStat { .. } => true,
            _ => false,
        },
        ReadinessProgramRole::FinalInputAssembly => matches!(
            effect,
            RuleEffectKind::ProjectSkillParameter { .. } | RuleEffectKind::ActivateGrant { .. }
        ),
        ReadinessProgramRole::SupportPreparationApplicability => {
            matches!(effect, RuleEffectKind::SupportApplicability { .. })
                && matches!(context, RuleEntityKind::Actor | RuleEntityKind::Action)
        }
        ReadinessProgramRole::SupportedPreparationProperty => match effect {
            RuleEffectKind::Contribute { entity, stat, .. }
                if (*entity == RuleEntity::Current
                    || (*entity == RuleEntity::Actor && context == RuleEntityKind::Actor))
                    && matches!(context, RuleEntityKind::Actor | RuleEntityKind::Action) =>
            {
                matches!(
                    known(index.definition(stat))?.value,
                    ComputedValueType::Integer | ComputedValueType::Quantity { .. }
                )
            }
            _ => false,
        },
        ReadinessProgramRole::SourceSupportedProperty
        | ReadinessProgramRole::SourceExternalProperty => {
            matches!(effect, RuleEffectKind::Contribute { entity: RuleEntity::PropertyOwner, stat, .. }
                if matches!(known(index.definition(stat))?.value, ComputedValueType::Integer | ComputedValueType::Quantity { .. }))
                && if role == ReadinessProgramRole::SourceSupportedProperty {
                    context == RuleEntityKind::SupportOrigin
                } else {
                    matches!(
                        context,
                        RuleEntityKind::Actor | RuleEntityKind::EquipmentUse
                    )
                }
        }
        ReadinessProgramRole::SourceFinalInputAssembly => match effect {
            RuleEffectKind::ProjectSkillParameter { .. } => {
                matches!(context, RuleEntityKind::Actor | RuleEntityKind::Skill)
            }
            RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat,
                ..
            } => {
                context == RuleEntityKind::Skill
                    && matches!(
                        known(index.definition(stat))?.value,
                        ComputedValueType::Integer | ComputedValueType::Quantity { .. }
                    )
            }
            _ => false,
        },
    };
    if !allowed {
        return Err(StageStorageError::Invalid(
            "effect is not authorized by preparation output role",
        ));
    }
    Ok(())
}
