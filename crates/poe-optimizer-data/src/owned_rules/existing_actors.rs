//! Applicability is checked once; evaluation uses existing typed actor programs.
use super::*;
use std::collections::BTreeMap;

pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    limits: RuleStorageLimits,
    usage: &mut RuleStorageUse,
) -> Result<(), RuleStorageError> {
    if let Some(registry) = &input.existing_actor_rules {
        add(
            &mut usage.existing_actor_applications,
            registry.members.len(),
        )?;
        usage.check(limits)?;
    }
    slot_reads(input, index, limits, usage)?;
    let Some(registry) = &input.existing_actor_rules else {
        return Ok(());
    };
    let invalid = RuleStorageError::Structure;
    work(usage, limits, input.owners.len() + registry.members.len())?;
    let mut owners = BTreeMap::new();
    for row in &input.owners {
        if let SchemaSubject::Definition(DefinitionAddress::Actor(actor)) = &row.owner
            && owners.insert(actor.clone(), row).is_some()
        {
            return Err(invalid("duplicate actor rule owner"));
        }
    }
    let mut ids = BTreeSet::new();
    let mut bound = BTreeSet::new();
    for application in &registry.members {
        if !ids.insert(&application.id)
            || application.owner.namespace() != &input.namespace
            || application.targets.is_empty()
        {
            return Err(invalid("invalid existing actor rule application"));
        }
        add(&mut usage.existing_actor_targets, application.targets.len())?;
        work(usage, limits, application.targets.len() + 1)?;
        let SchemaLookup::Known(schema) = index.definition(&application.owner) else {
            return Err(invalid(
                "existing actor rules require a known Actor definition",
            ));
        };
        let declarations = &schema.declarations;
        macro_rules! empty {
            ($($field:ident),+ $(,)?) => {
                $(if !declarations.$field.is_complete() || !declarations.$field.members.is_empty() {
                    return Err(invalid("existing actor rules require complete empty declarations"));
                })+
            };
        }
        empty!(
            parameters,
            choices,
            grants,
            actors,
            skill_grants,
            outputs,
            sockets
        );
        for target in &application.targets {
            if !bound.insert((&application.owner, target)) {
                return Err(invalid("duplicate existing actor owner applicability"));
            }
        }
        let owner = owners
            .get(&application.owner)
            .ok_or(invalid("existing actor rules require an owner"))?;
        work(usage, limits, owner.programs.members.len())?;
        for program in &owner.programs.members {
            if program.context != RuleEntityKind::Actor {
                return Err(invalid("existing actor rules require Actor context"));
            }
            work(usage, limits, program.effects.len() + program.reads.len())?;
            for effect in &program.effects {
                let entity = match &effect.effect {
                    RuleEffectKind::Contribute { entity, .. }
                    | RuleEffectKind::Derive { entity, .. } => entity,
                    _ => return Err(invalid("unsupported existing actor rule effect")),
                };
                if !actor_entity(*entity) {
                    return Err(invalid(
                        "existing actor rules cannot write outside their actor",
                    ));
                }
            }
            for read in &program.reads {
                let admitted = match &read.source {
                    RuleReadSource::CharacterLevel
                    | RuleReadSource::EnemyLevel
                    | RuleReadSource::CharacterClassIs { .. }
                    | RuleReadSource::CharacterAscendancyIs { .. }
                    | RuleReadSource::PlayerEquipmentSlot { .. } => true,
                    RuleReadSource::Stat { entity, .. }
                    | RuleReadSource::Capability { entity, .. }
                    | RuleReadSource::Contributions { entity, .. }
                    | RuleReadSource::ContributionQuery { entity, .. }
                    | RuleReadSource::ContributionSelection { entity, .. } => actor_entity(*entity),
                    RuleReadSource::External { entity, .. } => {
                        actor_entity(*entity)
                            || matches!(entity, RuleEntity::Enemy | RuleEntity::Environment)
                    }
                    _ => false,
                };
                if !admitted {
                    return Err(invalid(
                        "existing actor read requires unsupported provider authority",
                    ));
                }
            }
        }
    }
    if let SchemaClosure::Partial { gaps } = &registry.closure {
        work(usage, limits, gaps.len())?;
        if gaps.is_empty() {
            return Err(invalid("partial existing actor inventory needs gaps"));
        }
        let mut seen = BTreeSet::new();
        for gap in gaps {
            let SchemaSubject::Definition(DefinitionAddress::Actor(actor)) = &gap.subject else {
                return Err(invalid("existing actor inventory gap needs an Actor owner"));
            };
            if actor.namespace() != &input.namespace
                || !matches!(index.definition(actor), SchemaLookup::Known(_))
                || gap.facet != SchemaFacet::GameRules
                || !seen.insert((actor, &gap.code))
            {
                return Err(invalid("invalid existing actor inventory gap"));
            }
        }
        add(&mut usage.gaps, gaps.len())?;
    }
    usage.check(limits)
}

/// Scan even an absent application registry. A new read cannot acquire Player
/// authority by being placed on an ordinary Actor/provider or an unused owner.
fn slot_reads<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    limits: RuleStorageLimits,
    usage: &mut RuleStorageUse,
) -> Result<(), RuleStorageError> {
    let invalid = RuleStorageError::Structure;
    let mut player_owners = BTreeSet::new();
    if let Some(registry) = &input.existing_actor_rules {
        work(usage, limits, registry.members.len())?;
        for application in &registry.members {
            if application.targets == [ExistingActorRuleTarget::Player] {
                player_owners.insert(&application.owner);
            }
        }
    }
    work(usage, limits, input.owners.len())?;
    for owner in &input.owners {
        work(usage, limits, owner.programs.members.len())?;
        for program in &owner.programs.members {
            work(usage, limits, program.reads.len())?;
            for row in &program.reads {
                let RuleReadSource::PlayerEquipmentSlot { slot, read } = &row.source else {
                    continue;
                };
                if !RuleOperationsVersion::parse(input.operations_version.as_str())
                    .is_some_and(RuleOperationsVersion::supports_player_equipment_slots)
                {
                    return Err(invalid(
                        "Player equipment slot reads require operations V21",
                    ));
                }
                if program.context != RuleEntityKind::Actor
                    || !matches!(&owner.owner,
                        SchemaSubject::Definition(DefinitionAddress::Actor(actor))
                        if player_owners.contains(actor))
                {
                    return Err(invalid(
                        "Player equipment slot reads require an existing Player Actor application",
                    ));
                }
                if slot.namespace() != &input.namespace
                    || !matches!(index.definition(slot), SchemaLookup::Known(_))
                {
                    return Err(invalid(
                        "Player equipment slot must be known and in the package namespace",
                    ));
                }
                let valid = match read {
                    PlayerEquipmentSlotRead::Occupied => {
                        row.value_type == ComputedValueType::Boolean
                    }
                    PlayerEquipmentSlotRead::Stat { stat } => {
                        let SchemaLookup::Known(schema) = index.definition(stat) else {
                            return Err(invalid("Player equipment stat must be known"));
                        };
                        work(usage, limits, schema.targets.len())?;
                        stat.namespace() == &input.namespace
                            && schema.targets.contains(&RuleEntityKind::EquipmentUse)
                            && schema.value == row.value_type
                    }
                    PlayerEquipmentSlotRead::Capability { capability } => {
                        let SchemaLookup::Known(schema) = index.definition(capability) else {
                            return Err(invalid("Player equipment capability must be known"));
                        };
                        work(usage, limits, schema.targets.len())?;
                        capability.namespace() == &input.namespace
                            && schema.targets.contains(&RuleEntityKind::EquipmentUse)
                            && row.value_type == ComputedValueType::Boolean
                    }
                };
                if !valid {
                    return Err(invalid(
                        "Player equipment output must match its declared EquipmentUse type and scope",
                    ));
                }
            }
        }
    }
    if let Some(applications) = &input.effect_applications {
        work(usage, limits, applications.members.len())?;
        for application in &applications.members {
            work(usage, limits, application.program.reads.len())?;
            if application
                .program
                .reads
                .iter()
                .any(|row| matches!(row.source, RuleReadSource::PlayerEquipmentSlot { .. }))
            {
                return Err(invalid(
                    "effect applications cannot read Player equipment slots",
                ));
            }
        }
    }
    Ok(())
}

fn actor_entity(entity: RuleEntity) -> bool {
    // The admitted target is exactly Player. Future owned-actor applicability
    // must separately establish cross-actor read/write authority.
    matches!(
        entity,
        RuleEntity::Current | RuleEntity::Actor | RuleEntity::Player
    )
}

fn work(
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
    count: usize,
) -> Result<(), RuleStorageError> {
    add(&mut usage.existing_actor_work, count)?;
    usage.check(limits)
}
