//! Structural admission of finite effect-application declarations. Runtime
//! occurrence discovery, activation, types and graph cycles remain Engine work.
use super::*;
use poe_optimizer_core::{owned_definitions::*, owned_schema::RuleEntityKind};
use std::collections::BTreeMap;

fn work(
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
    amount: usize,
) -> Result<(), RuleStorageError> {
    add(&mut usage.effect_application_work, amount)?;
    usage.check(limits)
}

pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    limits: RuleStorageLimits,
    usage: &mut RuleStorageUse,
    tables: &BTreeSet<&OwnedDefinitionKey>,
) -> Result<(), RuleStorageError> {
    let invalid = RuleStorageError::Structure;
    let supported = RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_effect_applications);
    let Some(applications) = &input.effect_applications else {
        return if supported {
            Err(invalid(
                "v15 requires an explicit effect application inventory",
            ))
        } else {
            Ok(())
        };
    };
    if !supported {
        return Err(invalid(
            "effect applications require owned-domain-operations-v15",
        ));
    }
    add(&mut usage.effect_applications, applications.members.len())?;
    add(&mut usage.programs, applications.members.len())?;
    for row in &applications.members {
        add(&mut usage.effect_application_targets, row.targets.len())?;
        add(&mut usage.effect_stacking_rules, row.stacking.len())?;
        usage.check(limits)?;
    }
    if let SchemaClosure::Partial { gaps } = &applications.closure {
        add(&mut usage.gaps, gaps.len())?;
        usage.check(limits)?;
        if gaps.is_empty() {
            return Err(invalid("partial effect applications need gap evidence"));
        }
        let mut seen = BTreeSet::new();
        for gap in gaps {
            work(usage, limits, 1)?;
            let valid = match &gap.subject {
                SchemaSubject::Definition(id) => {
                    id.namespace() == &input.namespace
                        && index
                            .lookup_definition(id)
                            .is_some_and(|d| d.address() == *id)
                }
                SchemaSubject::Slot(id) => {
                    id.namespace() == &input.namespace
                        && id.declaration().namespace() == &input.namespace
                        && index.lookup_slot(id).is_some_and(|d| d.address() == *id)
                }
            };
            if !valid
                || gap.facet != SchemaFacet::GameRules
                || !seen.insert((owner_key(&gap.subject), &gap.code))
            {
                return Err(invalid("invalid effect application inventory gap"));
            }
        }
    }
    usage.check(limits)?;
    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::new();
    for row in &applications.members {
        work(usage, limits, 1)?;
        if !ids.insert(&row.id) {
            return Err(invalid("duplicate effect application ID"));
        }
        match &row.source {
            EffectApplicationSource::Skill { skill } => {
                if skill.namespace() != &input.namespace
                    || !matches!(index.definition(skill), SchemaLookup::Known(_))
                {
                    return Err(invalid(
                        "effect source Skill must be known in this namespace",
                    ));
                }
            }
            EffectApplicationSource::OwnedSlot { slot } => {
                if !known_actor_slot(slot, input, index) {
                    return Err(invalid(
                        "effect source actor slot must be known in this namespace",
                    ));
                }
            }
        }
        let context = row.program.context;
        if !matches!(context, RuleEntityKind::Actor | RuleEntityKind::Enemy)
            || row.targets.is_empty()
            || row.program.effects.is_empty()
        {
            return Err(invalid(
                "effect application needs recipient context, targets and effects",
            ));
        }
        let mut targets = BTreeSet::new();
        for target in &row.targets {
            work(usage, limits, 1)?;
            if !targets.insert(target) {
                return Err(invalid("duplicate effect application target"));
            }
            let valid = match target {
                EffectApplicationTarget::Player => context == RuleEntityKind::Actor,
                EffectApplicationTarget::Enemy => context == RuleEntityKind::Enemy,
                EffectApplicationTarget::OwnedSlot { slot } => {
                    context == RuleEntityKind::Actor && known_actor_slot(slot, input, index)
                }
            };
            if !valid {
                return Err(invalid(
                    "effect target is foreign, unknown or context-incompatible",
                ));
            }
        }
        let mut transforms = BTreeSet::new();
        if row.program.uses_source_property_scopes() {
            return Err(invalid(
                "effect applications cannot acquire source property authority",
            ));
        }
        validate_program(
            input,
            index,
            limits,
            usage,
            tables,
            &row.program,
            &mut transforms,
        )?;
        work(usage, limits, row.program.nodes.len() + 1)?;
        add(&mut usage.edges, 1)?;
        usage.check(limits)?;
        if !row
            .program
            .nodes
            .iter()
            .any(|node| node.id == row.activation)
        {
            return Err(invalid("effect activation references a missing node"));
        }
        for read in &row.program.reads {
            work(usage, limits, 1)?;
            match &read.source {
                RuleReadSource::EffectSourceParameter { slot } => {
                    let EffectApplicationSource::Skill { skill } = &row.source else {
                        return Err(invalid(
                            "effect source parameter requires an exact Skill source",
                        ));
                    };
                    let SchemaLookup::Known(schema) = index.definition(skill) else {
                        unreachable!()
                    };
                    work(
                        usage,
                        limits,
                        schema.declarations.parameters.members.len() + 1,
                    )?;
                    if slot.declaration != SlotOwnerDefId::Skill(skill.clone())
                        || !schema.declarations.parameters.members.contains(slot)
                        || !matches!(index.slot(slot), SchemaLookup::Known(_))
                    {
                        return Err(invalid(
                            "effect source parameter is not declared by its exact Skill",
                        ));
                    }
                }
                RuleReadSource::EffectSourceChoice { slot } => {
                    let EffectApplicationSource::Skill { skill } = &row.source else {
                        return Err(invalid(
                            "effect source choice requires an exact Skill source",
                        ));
                    };
                    let SchemaLookup::Known(schema) = index.definition(skill) else {
                        unreachable!()
                    };
                    work(usage, limits, schema.declarations.choices.members.len() + 1)?;
                    if slot.declaration != SlotOwnerDefId::Skill(skill.clone())
                        || !schema.declarations.choices.members.contains(slot)
                        || !matches!(index.slot(slot), SchemaLookup::Known(_))
                    {
                        return Err(invalid(
                            "effect source choice is not declared by its exact Skill",
                        ));
                    }
                }
                RuleReadSource::Parameter { .. }
                | RuleReadSource::Choice { .. }
                | RuleReadSource::GemLevel
                | RuleReadSource::ItemLevel
                | RuleReadSource::ItemQualityAmount { .. }
                | RuleReadSource::HasItemQuality { .. }
                | RuleReadSource::GemQualityAmount { .. }
                | RuleReadSource::HasGemQuality { .. }
                | RuleReadSource::ModifierTransforms { .. } => {
                    return Err(invalid(
                        "effect applications require explicitly source-scoped occurrence reads",
                    ));
                }
                RuleReadSource::Stat { entity, .. }
                | RuleReadSource::Capability { entity, .. }
                | RuleReadSource::External { entity, .. }
                | RuleReadSource::Contributions { entity, .. }
                    if matches!(
                        entity,
                        RuleEntity::Modifier
                            | RuleEntity::SupportOrigin
                            | RuleEntity::Skill
                            | RuleEntity::AssignedSkill
                    ) =>
                {
                    return Err(invalid(
                        "effect application read has no declared relative scope",
                    ));
                }
                _ => {}
            }
        }
        let mut effects = BTreeMap::new();
        for effect in &row.program.effects {
            work(usage, limits, 1)?;
            let RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat,
                contribution,
                ..
            } = &effect.effect
            else {
                return Err(invalid(
                    "effect applications only contribute to the exact current recipient",
                ));
            };
            let SchemaLookup::Known(schema) = index.definition(stat) else {
                return Err(invalid("effect application stat must be known"));
            };
            work(usage, limits, schema.targets.len() + 1)?;
            if stat.namespace() != &input.namespace
                || !schema.targets.contains(&context)
                || !matches!(
                    schema.value,
                    ComputedValueType::Integer | ComputedValueType::Quantity { .. }
                )
            {
                return Err(invalid(
                    "effect application contribution must be numeric in its recipient scope",
                ));
            }
            if let ComputedValueType::Quantity { unit } = &schema.value
                && !matches!(index.definition(unit), SchemaLookup::Known(_))
            {
                return Err(invalid("effect application unit must be known"));
            }
            effects.insert(&effect.id, (stat, contribution, &schema.value));
        }
        let mut mapped = BTreeSet::new();
        let mut local_groups = BTreeSet::new();
        for stacking in &row.stacking {
            work(usage, limits, 1)?;
            let Some((stat, kind, value_type)) = effects.get(&stacking.effect) else {
                return Err(invalid(
                    "stacking references an unknown contribution effect",
                ));
            };
            let group = (&stacking.family, &stacking.modifier);
            if !mapped.insert(&stacking.effect) || !local_groups.insert(group) {
                return Err(invalid(
                    "duplicate effect stacking mapping or local modifier group",
                ));
            }
            let signature = (*stat, *kind, *value_type, stacking.reduction);
            if let Some(previous) = groups.insert(group, signature)
                && previous != signature
            {
                return Err(invalid(
                    "effect stacking group stat, contribution, type or unit disagrees",
                ));
            }
        }
        if mapped.len() != effects.len() {
            return Err(invalid(
                "every effect contribution requires explicit stacking",
            ));
        }
    }
    Ok(())
}

fn known_actor_slot<I: DefinitionSchemaIndex>(
    slot: &poe_optimizer_core::owned_build::DeclaredSlot<ActorSlotDefId>,
    input: &RulePackageInput,
    index: &I,
) -> bool {
    slot.slot.namespace() == &input.namespace
        && slot.declaration.namespace() == &input.namespace
        && matches!(index.slot(slot), SchemaLookup::Known(_))
}
