//! Bounded definition-side checks. Concrete occurrence membership belongs to Engine.
use super::*;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{EquipmentSlotDefId, OwnedDefinitionKey, SlotOwnerDefId, StatDefId},
    owned_schema::{SchemaDefinitionId, UnitDimension},
};
use std::collections::BTreeMap;

#[derive(Clone, Eq, PartialEq)]
struct EquipmentLane {
    modifier: bool,
    source_rank: u32,
    slots: BTreeMap<EquipmentSlotDefId, u32>,
}

#[derive(Default)]
struct QueryIndex<'a> {
    queries: BTreeMap<&'a OwnedDefinitionKey, &'a ContributionQuery>,
    group_types: BTreeMap<(&'a OwnedDefinitionKey, &'a OwnedDefinitionKey), ComputedValueType>,
}

struct ApplicationGroup<'a> {
    stat: &'a StatDefId,
    kind: ContributionKind,
    reduction: EffectStackingReduction,
    declarations: BTreeSet<(&'a OwnedDefinitionKey, &'a OwnedDefinitionKey)>,
}
type ApplicationGroups<'a> =
    BTreeMap<(&'a OwnedDefinitionKey, &'a OwnedDefinitionKey), ApplicationGroup<'a>>;

/// Census declarations before any candidate is selected or evaluated. This is
/// shared by storage and standalone compilation, including unread query groups.
fn application_groups<'a, I: DefinitionSchemaIndex>(
    input: &'a RulePackageInput,
    index: &I,
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
) -> Result<ApplicationGroups<'a>, RuleStorageError> {
    let invalid = RuleStorageError::Structure;
    let mut groups: ApplicationGroups<'a> = BTreeMap::new();
    let Some(registry) = &input.effect_applications else {
        return Ok(groups);
    };
    work(usage, limits, registry.members.len())?;
    let mut ids = BTreeSet::new();
    for application in &registry.members {
        if !ids.insert(&application.id) {
            return Err(invalid("duplicate application in contribution census"));
        }
        work(usage, limits, application.program.effects.len())?;
        let mut effects = BTreeMap::new();
        for effect in &application.program.effects {
            if effects.insert(&effect.id, effect).is_some() {
                return Err(invalid("duplicate application contribution effect"));
            }
        }
        work(usage, limits, application.stacking.len())?;
        let mut mapped = BTreeSet::new();
        for mapping in &application.stacking {
            let Some(effect) = effects.get(&mapping.effect) else {
                return Err(invalid(
                    "application membership references an unknown effect",
                ));
            };
            if !mapped.insert(&mapping.effect) {
                return Err(invalid("duplicate application contribution mapping"));
            }
            let RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat,
                contribution,
                ..
            } = &effect.effect
            else {
                return Err(invalid(
                    "application group requires an exact-recipient contribution",
                ));
            };
            let SchemaLookup::Known(schema) = index.definition(stat) else {
                return Err(invalid("application group requires a known stat"));
            };
            work(
                usage,
                limits,
                schema.targets.len() + application.targets.len(),
            )?;
            let context = application.program.context;
            if !schema.targets.contains(&context)
                || !matches!(
                    schema.value,
                    ComputedValueType::Integer | ComputedValueType::Quantity { .. }
                )
                || application.targets.is_empty()
                || application.targets.iter().any(|target| match target {
                    EffectApplicationTarget::Player => context != RuleEntityKind::Actor,
                    EffectApplicationTarget::Enemy => context != RuleEntityKind::Enemy,
                    EffectApplicationTarget::OwnedSlot { slot } => {
                        context != RuleEntityKind::Actor
                            || !matches!(index.slot(slot), SchemaLookup::Known(_))
                    }
                })
            {
                return Err(invalid(
                    "application group recipient or value type differs from its stat",
                ));
            }
            let group = groups
                .entry((&mapping.family, &mapping.modifier))
                .or_insert_with(|| ApplicationGroup {
                    stat,
                    kind: *contribution,
                    reduction: mapping.reduction,
                    declarations: BTreeSet::new(),
                });
            if group.stat != stat
                || group.kind != *contribution
                || group.reduction != mapping.reduction
            {
                return Err(invalid(
                    "application group contribution channel or reduction disagrees",
                ));
            }
            if !group
                .declarations
                .insert((&application.id, &mapping.effect))
            {
                return Err(invalid("duplicate application group declaration"));
            }
        }
        if mapped.len() != effects.len() {
            return Err(invalid(
                "application contribution has no stacking declaration",
            ));
        }
    }
    Ok(groups)
}

fn work(
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
    count: usize,
) -> Result<(), RuleStorageError> {
    add(&mut usage.ordered_work, count)?;
    usage.check(limits)
}
fn subject_known<I: DefinitionSchemaIndex>(subject: &SchemaSubject, index: &I) -> bool {
    match subject {
        SchemaSubject::Definition(id) => index
            .lookup_definition(id)
            .is_some_and(|d| d.address() == *id),
        SchemaSubject::Slot(id) => index.lookup_slot(id).is_some_and(|d| d.address() == *id),
    }
}
fn closure<I: DefinitionSchemaIndex>(
    value: &SchemaClosure,
    required_subject: Option<&SchemaSubject>,
    index: &I,
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
) -> Result<(), RuleStorageError> {
    if let SchemaClosure::Partial { gaps } = value {
        add(&mut usage.gaps, gaps.len())?;
        work(usage, limits, gaps.len())?;
        if gaps.is_empty() {
            return Err(RuleStorageError::Structure(
                "partial ordered inventory needs gap evidence",
            ));
        }
        let mut seen = BTreeSet::new();
        for gap in gaps {
            if gap.facet != SchemaFacet::GameRules
                || !subject_known(&gap.subject, index)
                || required_subject.is_some_and(|s| s != &gap.subject)
                || !seen.insert((owner_key(&gap.subject), &gap.code))
            {
                return Err(RuleStorageError::Structure("invalid ordered inventory gap"));
            }
        }
    }
    Ok(())
}
fn value_type(value: &ParameterValue) -> Option<ComputedValueType> {
    match value {
        ParameterValue::Integer(_) => Some(ComputedValueType::Integer),
        ParameterValue::Quantity(q) => Some(ComputedValueType::Quantity {
            unit: q.unit().clone(),
        }),
        _ => None,
    }
}
fn group_type<I: DefinitionSchemaIndex>(
    query: &ContributionQuery,
    group: &ContributionGroup,
    stat: &ComputedValueType,
    index: &I,
) -> Result<ComputedValueType, RuleStorageError> {
    let invalid = RuleStorageError::Structure;
    if query.contribution == ContributionKind::Flag {
        if *stat != ComputedValueType::Boolean
            || group.empty != ParameterValue::Boolean(false)
            || group.reduction != ContributionReduction::Any
            || group.ordering != ContributionOrdering::Unordered
        {
            return Err(invalid(
                "flag group requires Boolean stat, false identity and unordered Any",
            ));
        }
        return Ok(ComputedValueType::Boolean);
    }
    if group.ordering != ContributionOrdering::Ordered {
        return Err(invalid(
            "numeric contribution group requires semantic ordering",
        ));
    }
    let ty = value_type(&group.empty).ok_or(invalid("ordered group needs a numeric identity"))?;
    let product = query.contribution == ContributionKind::Multiply;
    let expected = if product { 1.0 } else { 0.0 };
    let actual = match &group.empty {
        ParameterValue::Integer(v) => v.get() as f64,
        ParameterValue::Quantity(v) => v.value(),
        _ => unreachable!(),
    };
    if actual != expected
        || group.reduction
            != if product {
                ContributionReduction::Product
            } else {
                ContributionReduction::Sum
            }
    {
        return Err(invalid(
            "ordered group reduction or identity differs from its contribution kind",
        ));
    }
    let dimension = if let ComputedValueType::Quantity { unit } = &ty {
        let SchemaLookup::Known(schema) = index.definition(unit) else {
            return Err(invalid("ordered group unit must be known"));
        };
        Some(schema.dimension)
    } else {
        None
    };
    let valid = match query.contribution {
        ContributionKind::Flag => unreachable!("flag handled above"),
        ContributionKind::Add => ty == *stat,
        ContributionKind::Increase => dimension == Some(UnitDimension::PercentagePoints),
        ContributionKind::Multiply => dimension == Some(UnitDimension::DimensionlessFactor),
    };
    if !valid {
        return Err(invalid(
            "ordered group identity has the wrong numeric type or unit",
        ));
    }
    Ok(ty)
}
fn order<I: DefinitionSchemaIndex>(
    member: &ContributionMember,
    ordering: ContributionOrdering,
    program: &RuleProgram,
    input: &RulePackageInput,
    index: &I,
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
) -> Result<Option<EquipmentLane>, RuleStorageError> {
    let invalid = RuleStorageError::Structure;
    let ContributionProducer::ProgramEffect(producer) = &member.producer else {
        return Err(invalid(
            "ordinary contribution requires a program-effect producer",
        ));
    };
    if (ordering == ContributionOrdering::Ordered) != member.order.is_some() {
        return Err(invalid(
            "contribution member ordering differs from its group",
        ));
    }
    if let ContributionOrigin::Action { authored, supplies } = &producer.origin {
        if !RuleOperationsVersion::parse(input.operations_version.as_str())
            .is_some_and(RuleOperationsVersion::supports_action_contribution_queries)
        {
            return Err(invalid(
                "Action contribution origins require owned-domain-operations-v26",
            ));
        }
        if program.context != RuleEntityKind::Action
            || member
                .order
                .as_ref()
                .is_some_and(|o| !o.slot_ranks.is_empty())
        {
            return Err(invalid(
                "Action contribution requires Action context and no equipment ranks",
            ));
        }
        let (skill, output) = match &producer.owner {
            SchemaSubject::Definition(DefinitionAddress::Skill(skill)) => (skill, None),
            SchemaSubject::Slot(SlotAddress::ActionOutput(output)) => {
                let SlotOwnerDefId::Skill(skill) = &output.declaration else {
                    return Err(invalid(
                        "Action output contribution requires a Skill declaration",
                    ));
                };
                if !matches!(index.slot(output), SchemaLookup::Known(_)) {
                    return Err(invalid("Action contribution requires a known output"));
                }
                (skill, Some(output))
            }
            _ => {
                return Err(invalid(
                    "Action contribution requires a Skill or Action-output owner",
                ));
            }
        };
        let SchemaLookup::Known(schema) = index.definition(skill) else {
            return Err(invalid("Action contribution requires a known Skill"));
        };
        if *authored {
            if !schema.directly_selectable {
                return Err(invalid(
                    "authored Action contribution requires a directly selectable Skill",
                ));
            }
            work(usage, limits, schema.declarations.outputs.members.len())?;
            if output.is_some_and(|o| !schema.declarations.outputs.members.contains(o)) {
                return Err(invalid(
                    "authored Action output is not declared by its Skill",
                ));
            }
        }
        add(&mut usage.ordered_slots, supplies.len())?;
        work(usage, limits, supplies.len())?;
        if !authored && supplies.is_empty() {
            return Err(invalid(
                "Action contribution requires authored permission or explicit supply membership",
            ));
        }
        let mut seen = BTreeSet::new();
        for slot in supplies {
            if !seen.insert(slot) {
                return Err(invalid("duplicate supplied Action contribution slot"));
            }
            let SchemaLookup::Known(supply) = index.slot(slot) else {
                return Err(invalid("supplied Action contribution slot must be known"));
            };
            if &supply.skill != skill {
                return Err(invalid(
                    "supplied Action contribution owner differs from its Skill slot",
                ));
            }
            work(usage, limits, supply.outputs.members.len())?;
            if output.is_some_and(|o| !supply.outputs.members.contains(o)) {
                return Err(invalid(
                    "supplied Action output is absent from its Skill supply",
                ));
            }
        }
        return Ok(None);
    }
    if let ContributionOrigin::Skill { authored, supplies } = &producer.origin {
        if !RuleOperationsVersion::parse(input.operations_version.as_str())
            .is_some_and(RuleOperationsVersion::supports_skill_contribution_queries)
        {
            return Err(invalid(
                "Skill contribution origins require owned-domain-operations-v24",
            ));
        }
        if program.context != RuleEntityKind::Skill
            || member
                .order
                .as_ref()
                .is_some_and(|o| !o.slot_ranks.is_empty())
        {
            return Err(invalid(
                "Skill contribution origins require Skill context and no equipment ranks",
            ));
        }
        let SchemaSubject::Definition(DefinitionAddress::Skill(skill)) = &producer.owner else {
            return Err(invalid(
                "Skill contribution requires a Skill definition owner",
            ));
        };
        let SchemaLookup::Known(schema) = index.definition(skill) else {
            return Err(invalid("Skill contribution requires a known Skill schema"));
        };
        if *authored && !schema.directly_selectable {
            return Err(invalid(
                "authored Skill contribution requires a directly selectable Skill",
            ));
        }
        add(&mut usage.ordered_slots, supplies.len())?;
        work(usage, limits, supplies.len())?;
        if !authored && supplies.is_empty() {
            return Err(invalid(
                "Skill contribution requires authored permission or explicit supply membership",
            ));
        }
        let mut seen = BTreeSet::new();
        for slot in supplies {
            if !seen.insert(slot) {
                return Err(invalid("duplicate supplied Skill contribution slot"));
            }
            let SchemaLookup::Known(supply) = index.slot(slot) else {
                return Err(invalid("supplied Skill contribution slot must be known"));
            };
            if &supply.skill != skill {
                return Err(invalid(
                    "supplied Skill contribution owner differs from its slot",
                ));
            }
        }
        return Ok(None);
    }
    let extended = matches!(
        producer.origin,
        ContributionOrigin::ExistingActor { .. }
            | ContributionOrigin::Reward
            | ContributionOrigin::SuppliedActor { .. }
    );
    if extended {
        if !RuleOperationsVersion::parse(input.operations_version.as_str())
            .is_some_and(RuleOperationsVersion::supports_actor_reward_contributions)
        {
            return Err(invalid(
                "Actor/reward contribution origins require owned-domain-operations-v23",
            ));
        }
        if program.context != RuleEntityKind::Actor
            || member
                .order
                .as_ref()
                .is_some_and(|o| !o.slot_ranks.is_empty())
        {
            return Err(invalid(
                "Actor/reward contribution origins require Actor context and no equipment ranks",
            ));
        }
        match &producer.origin {
            ContributionOrigin::ExistingActor { application } => {
                let SchemaSubject::Definition(DefinitionAddress::Actor(actor)) = &producer.owner
                else {
                    return Err(invalid(
                        "existing Actor contribution requires an Actor definition owner",
                    ));
                };
                let registry = input.existing_actor_rules.as_ref().ok_or(invalid(
                    "existing Actor contribution requires declared applicability",
                ))?;
                work(usage, limits, registry.members.len())?;
                let row = registry
                    .members
                    .iter()
                    .find(|r| &r.id == application)
                    .ok_or(invalid("existing Actor contribution application is absent"))?;
                if &row.owner != actor || !matches!(index.definition(actor), SchemaLookup::Known(_))
                {
                    return Err(invalid(
                        "existing Actor contribution application has a different owner",
                    ));
                }
            }
            ContributionOrigin::Reward => {
                let SchemaSubject::Definition(DefinitionAddress::Reward(reward)) = &producer.owner
                else {
                    return Err(invalid(
                        "reward contribution requires a Reward definition owner",
                    ));
                };
                if !matches!(index.definition(reward), SchemaLookup::Known(_)) {
                    return Err(invalid(
                        "reward contribution requires a known Reward schema",
                    ));
                }
            }
            ContributionOrigin::SuppliedActor { slots } => {
                add(&mut usage.ordered_slots, slots.len())?;
                work(usage, limits, slots.len())?;
                if slots.is_empty() {
                    return Err(invalid(
                        "supplied Actor contribution requires explicit slot membership",
                    ));
                }
                let mut seen = BTreeSet::new();
                for slot in slots {
                    if !seen.insert(slot) {
                        return Err(invalid("duplicate supplied Actor contribution slot"));
                    }
                    let SchemaLookup::Known(schema) = index.slot(slot) else {
                        return Err(invalid("supplied Actor contribution slot must be known"));
                    };
                    let Some(actor) = &schema.provider_definition else {
                        return Err(invalid(
                            "supplied Actor contribution requires an explicit provider definition",
                        ));
                    };
                    if !matches!(index.definition(actor), SchemaLookup::Known(_)) {
                        return Err(invalid(
                            "supplied Actor contribution provider must be known",
                        ));
                    }
                    let valid = match &producer.owner {
                        SchemaSubject::Definition(DefinitionAddress::Actor(id)) => id == actor,
                        SchemaSubject::Slot(SlotAddress::Actor(id)) => id == slot,
                        _ => false,
                    };
                    if !valid {
                        return Err(invalid(
                            "supplied Actor contribution owner differs from its slot",
                        ));
                    }
                }
            }
            _ => unreachable!(),
        }
        return Ok(None);
    }
    let definition = match &producer.owner {
        SchemaSubject::Definition(id) => id,
        SchemaSubject::Slot(_) => {
            return Err(invalid("ordered member requires a direct definition owner"));
        }
    };
    let known = match definition {
        DefinitionAddress::Class(id) => matches!(index.definition(id), SchemaLookup::Known(_)),
        DefinitionAddress::Ascendancy(id) => matches!(index.definition(id), SchemaLookup::Known(_)),
        DefinitionAddress::PassiveNode(id) => {
            matches!(index.definition(id), SchemaLookup::Known(_))
        }
        DefinitionAddress::ItemTemplate(id) => {
            matches!(index.definition(id), SchemaLookup::Known(_))
        }
        DefinitionAddress::Modifier(id) => matches!(index.definition(id), SchemaLookup::Known(_)),
        _ => false,
    };
    if !known {
        return Err(invalid(
            "ordered producer definition must have a known schema",
        ));
    }
    let (valid, context, equipment) = match &producer.origin {
        ContributionOrigin::Character => (
            matches!(
                definition,
                DefinitionAddress::Class(_) | DefinitionAddress::Ascendancy(_)
            ),
            RuleEntityKind::Actor,
            None,
        ),
        ContributionOrigin::Allocation => (
            matches!(definition, DefinitionAddress::PassiveNode(_)),
            RuleEntityKind::Actor,
            None,
        ),
        ContributionOrigin::EquipmentUse { slots } => (
            matches!(definition, DefinitionAddress::ItemTemplate(_)),
            RuleEntityKind::EquipmentUse,
            Some((false, slots)),
        ),
        ContributionOrigin::ItemModifier { slots } => (
            matches!(definition, DefinitionAddress::Modifier(_)),
            RuleEntityKind::EquipmentUse,
            Some((true, slots)),
        ),
        ContributionOrigin::ExistingActor { .. }
        | ContributionOrigin::Reward
        | ContributionOrigin::SuppliedActor { .. }
        | ContributionOrigin::Skill { .. }
        | ContributionOrigin::Action { .. } => {
            unreachable!("extended origins checked above")
        }
    };
    if !valid || program.context != context {
        return Err(invalid(
            "ordered origin policy does not match its owner and program context",
        ));
    }
    let Some((modifier, slots)) = equipment else {
        if member
            .order
            .as_ref()
            .is_some_and(|o| !o.slot_ranks.is_empty())
        {
            return Err(invalid(
                "non-equipment contribution order forbids slot ranks",
            ));
        }
        return Ok(None);
    };
    add(&mut usage.ordered_slots, slots.len())?;
    work(usage, limits, slots.len())?;
    if slots.is_empty() {
        return Err(invalid(
            "contribution equipment origin needs explicit slot membership",
        ));
    }
    let mut membership = BTreeSet::new();
    for slot in slots {
        if !matches!(index.definition(slot), SchemaLookup::Known(_))
            || !membership.insert(slot.clone())
        {
            return Err(invalid("duplicate or unknown contribution equipment slot"));
        }
    }
    let Some(order) = &member.order else {
        return Ok(None);
    };
    work(usage, limits, order.slot_ranks.len())?;
    let mut mapping = BTreeMap::new();
    let mut ranks = BTreeSet::new();
    for slot in &order.slot_ranks {
        if !membership.contains(&slot.slot)
            || mapping.insert(slot.slot.clone(), slot.rank).is_some()
            || !ranks.insert(slot.rank)
        {
            return Err(invalid(
                "duplicate, unknown or ambiguously ranked ordered equipment slot",
            ));
        }
    }
    if mapping.len() != membership.len() {
        return Err(invalid(
            "ordered slot ranks must exactly cover origin membership",
        ));
    }
    Ok(Some(EquipmentLane {
        modifier,
        source_rank: order.source_rank,
        slots: mapping,
    }))
}
fn read<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    p: &RuleProgram,
    r: &RuleRead,
    catalog: &QueryIndex<'_>,
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
) -> Result<(), RuleStorageError> {
    let RuleReadSource::ContributionQuery {
        entity,
        query,
        group,
    } = &r.source
    else {
        return Ok(());
    };
    work(usage, limits, 1)?;
    let invalid = RuleStorageError::Structure;
    if !RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_contribution_queries)
    {
        return Err(invalid(
            "ordered contributions require owned-domain-operations-v21",
        ));
    }
    if !catalog.queries.contains_key(query)
        || catalog.group_types.get(&(query, group)) != Some(&r.value_type)
    {
        return Err(invalid(
            "ordered read has an unknown query/group or mismatched type",
        ));
    }
    // Skill reads address the exact current occurrence; source-property and
    // cross-Skill relations require their own authority and remain excluded.
    let skill_allowed = RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_skill_contribution_queries);
    let action_allowed = RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_action_contribution_queries);
    let enemy_allowed = catalog.queries[query].contribution == ContributionKind::Flag
        && RuleOperationsVersion::parse(input.operations_version.as_str())
            .is_some_and(RuleOperationsVersion::supports_boolean_contributions);
    let valid = match entity {
        RuleEntity::Player => true,
        RuleEntity::Enemy => enemy_allowed,
        RuleEntity::Actor => matches!(p.context, RuleEntityKind::Actor | RuleEntityKind::Action),
        RuleEntity::Current => {
            matches!(
                p.context,
                RuleEntityKind::Actor | RuleEntityKind::EquipmentUse
            ) || (p.context == RuleEntityKind::Enemy && enemy_allowed)
                || (p.context == RuleEntityKind::Skill && skill_allowed)
                || (p.context == RuleEntityKind::Action && action_allowed)
        }
        _ => false,
    };
    if !valid {
        return Err(invalid(
            "ordered read has an unsupported relative recipient scope",
        ));
    }
    let target = if *entity == RuleEntity::Current {
        p.context
    } else if *entity == RuleEntity::Enemy {
        RuleEntityKind::Enemy
    } else {
        RuleEntityKind::Actor
    };
    let SchemaLookup::Known(stat) = index.definition(&catalog.queries[query].stat) else {
        unreachable!("query checked")
    };
    if !stat.targets.contains(&target) {
        return Err(invalid(
            "ordered read recipient is not admitted by its stat",
        ));
    }
    Ok(())
}
pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    limits: RuleStorageLimits,
    usage: &mut RuleStorageUse,
) -> Result<(), RuleStorageError> {
    let boolean_enabled = RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_boolean_contributions);
    // Bound the public raw-input entry point before any program traversal. This
    // is traversal work, independent of the package's ordinary object counts.
    work(usage, limits, input.owners.len())?;
    for owner in &input.owners {
        work(usage, limits, owner.programs.members.len())?;
    }
    if let Some(applications) = &input.effect_applications {
        work(usage, limits, applications.members.len())?;
    }
    // Raw compilation calls this validator too. Public direct reductions cannot
    // bypass exact membership; the compiler's private typing adapter is separate.
    for p in input.owners.iter().flat_map(|o| &o.programs.members).chain(
        input
            .effect_applications
            .iter()
            .flat_map(|a| a.members.iter().map(|a| &a.program)),
    ) {
        work(usage, limits, p.reads.len().saturating_add(p.effects.len()))?;
        for read in &p.reads {
            if let RuleReadSource::Contributions {
                contribution,
                reduction,
                ..
            } = &read.source
                && (*contribution == ContributionKind::Flag
                    || *reduction == ContributionReduction::Any
                    || read.value_type == ComputedValueType::Boolean)
            {
                return Err(RuleStorageError::Structure(
                    "Boolean contributions require checked query membership",
                ));
            }
        }
        if !boolean_enabled
            && p.effects.iter().any(|e| {
                matches!(
                    e.effect,
                    RuleEffectKind::Contribute {
                        contribution: ContributionKind::Flag,
                        ..
                    }
                )
            })
        {
            return Err(RuleStorageError::Structure(
                "Boolean contributions require owned-domain-operations-v22",
            ));
        }
    }
    let enabled = RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_contribution_queries);
    let Some(registry) = &input.contribution_queries else {
        if enabled {
            return Err(RuleStorageError::Structure(
                "ordered contributions require an explicit V21 inventory",
            ));
        }
        // Older operation capabilities still reject query reads. The current
        // schema and work receipt apply to every operation subset, including
        // unused programs and effect applications.
        let forbidden = |p: &RuleProgram| {
            p.reads
                .iter()
                .any(|r| matches!(r.source, RuleReadSource::ContributionQuery { .. }))
        };
        if input
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .any(forbidden)
            || input
                .effect_applications
                .as_ref()
                .is_some_and(|a| a.members.iter().any(|a| forbidden(&a.program)))
        {
            return Err(RuleStorageError::Structure(
                "ordered contributions require owned-domain-operations-v21",
            ));
        }
        return Ok(());
    };
    if !enabled {
        return Err(RuleStorageError::Structure(
            "ordered contributions require owned-domain-operations-v21",
        ));
    }
    add(&mut usage.ordered_queries, registry.members.len())?;
    work(usage, limits, registry.members.len())?;
    closure(&registry.closure, None, index, usage, limits)?;
    // Bound scans before allocating the exact program/effect index.
    work(usage, limits, input.owners.len())?;
    let mut programs = BTreeMap::new();
    for owner in &input.owners {
        work(usage, limits, owner.programs.members.len())?;
        for program in &owner.programs.members {
            work(
                usage,
                limits,
                program.effects.len().saturating_add(program.reads.len()),
            )?;
            if programs
                .insert((owner_key(&owner.owner), &program.id), program)
                .is_some()
            {
                return Err(RuleStorageError::Structure(
                    "duplicate ordered producer program",
                ));
            }
        }
    }
    let applications = application_groups(input, index, usage, limits)?;
    let mut catalog = QueryIndex::default();
    for query in &registry.members {
        if catalog.queries.insert(&query.id, query).is_some() {
            return Err(RuleStorageError::Structure("duplicate ordered query"));
        }
        let SchemaLookup::Known(stat) = index.definition(&query.stat) else {
            return Err(RuleStorageError::Structure(
                "ordered query stat must be known",
            ));
        };
        if query.contribution == ContributionKind::Flag && !boolean_enabled {
            return Err(RuleStorageError::Structure(
                "Boolean contributions require owned-domain-operations-v22",
            ));
        }
        if !matches!(
            stat.value,
            ComputedValueType::Integer | ComputedValueType::Quantity { .. }
        ) && !(query.contribution == ContributionKind::Flag
            && stat.value == ComputedValueType::Boolean)
        {
            return Err(RuleStorageError::Structure(
                "ordered query stat must be numeric",
            ));
        }
        if let ComputedValueType::Quantity { unit } = &stat.value
            && !matches!(index.definition(unit), SchemaLookup::Known(_))
        {
            return Err(RuleStorageError::Structure(
                "ordered query stat unit must be known",
            ));
        }
        add(&mut usage.ordered_groups, query.groups.len())?;
        work(usage, limits, query.groups.len())?;
        if query.groups.is_empty() {
            return Err(RuleStorageError::Structure(
                "ordered query requires named groups",
            ));
        }
        let mut members = BTreeSet::new();
        let mut application_members = BTreeSet::new();
        for group in &query.groups {
            let ty = group_type(query, group, &stat.value, index)?;
            if catalog
                .group_types
                .insert((&query.id, &group.id), ty)
                .is_some()
            {
                return Err(RuleStorageError::Structure("duplicate ordered group"));
            }
            let subject = SchemaSubject::Definition(query.stat.address());
            closure(&group.members.closure, Some(&subject), index, usage, limits)?;
            add(&mut usage.ordered_members, group.members.members.len())?;
            work(usage, limits, group.members.members.len())?;
            let mut lanes = BTreeMap::new();
            let mut owner_ranks = BTreeMap::new();
            let mut program_ranks = BTreeMap::new();
            for member in &group.members.members {
                let producer = match &member.producer {
                    ContributionProducer::ProgramEffect(producer) => producer,
                    ContributionProducer::ApplicationGroup(producer) => {
                        if !RuleOperationsVersion::parse(input.operations_version.as_str())
                            .is_some_and(
                                RuleOperationsVersion::supports_application_group_contributions,
                            )
                        {
                            return Err(RuleStorageError::Structure(
                                "application-group contributions require owned-domain-operations-v25",
                            ));
                        }
                        let address = (&producer.family, &producer.modifier);
                        if !application_members.insert(address) {
                            return Err(RuleStorageError::Structure(
                                "application group occurs more than once across query groups",
                            ));
                        }
                        let Some(actual) = applications.get(&address) else {
                            return Err(RuleStorageError::Structure(
                                "unknown application group producer",
                            ));
                        };
                        if actual.stat != &query.stat || actual.kind != query.contribution {
                            return Err(RuleStorageError::Structure(
                                "application group contribution channel differs from query",
                            ));
                        }
                        add(&mut usage.ordered_members, producer.declarations.len())?;
                        work(
                            usage,
                            limits,
                            producer.declarations.len() + actual.declarations.len(),
                        )?;
                        let mut declared = BTreeSet::new();
                        for declaration in &producer.declarations {
                            if !declared.insert((&declaration.application, &declaration.effect)) {
                                return Err(RuleStorageError::Structure(
                                    "duplicate application producer declaration",
                                ));
                            }
                        }
                        if declared != actual.declarations {
                            return Err(RuleStorageError::Structure(
                                "application producer declarations differ from complete potential census",
                            ));
                        }
                        if (group.ordering == ContributionOrdering::Ordered)
                            != member.order.is_some()
                            || member
                                .order
                                .as_ref()
                                .is_some_and(|o| !o.slot_ranks.is_empty())
                        {
                            return Err(RuleStorageError::Structure(
                                "application group ordering requires the group's policy and no equipment ranks",
                            ));
                        }
                        continue;
                    }
                };
                if !members.insert((
                    owner_key(&producer.owner),
                    &producer.program,
                    &producer.effect,
                )) {
                    return Err(RuleStorageError::Structure(
                        "ordered producer occurs more than once across query groups",
                    ));
                }
                if !subject_known(&producer.owner, index) {
                    return Err(RuleStorageError::Structure(
                        "ordered member owner must be known",
                    ));
                }
                let Some(program) = programs.get(&(owner_key(&producer.owner), &producer.program))
                else {
                    return Err(RuleStorageError::Structure(
                        "ordered member references an unknown producer program",
                    ));
                };
                work(usage, limits, program.effects.len())?;
                let matching: Vec<_> = program
                    .effects
                    .iter()
                    .filter(|e| e.id == producer.effect)
                    .collect();
                if matching.len() != 1 {
                    return Err(RuleStorageError::Structure(
                        "ordered member references an unknown or duplicate effect",
                    ));
                }
                let RuleEffectKind::Contribute {
                    entity,
                    stat: channel,
                    contribution,
                    ..
                } = &matching[0].effect
                else {
                    return Err(RuleStorageError::Structure(
                        "ordered member must select a contribution effect",
                    ));
                };
                if channel != &query.stat || contribution != &query.contribution {
                    return Err(RuleStorageError::Structure(
                        "ordered member contribution channel differs from query",
                    ));
                }
                if matches!(producer.origin, ContributionOrigin::Skill { .. })
                    && *entity != RuleEntity::Current
                {
                    return Err(RuleStorageError::Structure(
                        "Skill contribution membership admits only the exact current Skill recipient",
                    ));
                }
                if matches!(producer.origin, ContributionOrigin::Action { .. })
                    && *entity != RuleEntity::Current
                {
                    return Err(RuleStorageError::Structure(
                        "Action contribution membership admits only the exact current Action recipient",
                    ));
                }
                let target = match entity {
                    RuleEntity::Player | RuleEntity::Actor => RuleEntityKind::Actor,
                    RuleEntity::Enemy
                        if boolean_enabled && query.contribution == ContributionKind::Flag =>
                    {
                        RuleEntityKind::Enemy
                    }
                    RuleEntity::Current => program.context,
                    _ => {
                        return Err(RuleStorageError::Structure(
                            "ordered producer has unsupported recipient scope",
                        ));
                    }
                };
                if !stat.targets.contains(&target) {
                    return Err(RuleStorageError::Structure(
                        "ordered producer recipient is not admitted by its stat",
                    ));
                }
                if let Some(lane) =
                    order(member, group.ordering, program, input, index, usage, limits)?
                    && let Some(prior) = lanes.insert(lane.modifier, lane.clone())
                    && prior != lane
                {
                    return Err(RuleStorageError::Structure(
                        "ordered equipment origin lane must share source and slot ranks",
                    ));
                }
                let Some(member_order) = &member.order else {
                    continue;
                };
                let order = (member_order.source_rank, member_order.program_rank);
                if let Some(prior) =
                    owner_ranks.insert(owner_key(&producer.owner), member_order.source_rank)
                    && prior != member_order.source_rank
                {
                    return Err(RuleStorageError::Structure(
                        "ordered programs of one owner must share source rank",
                    ));
                }
                if let Some(prior) =
                    program_ranks.insert((owner_key(&producer.owner), &producer.program), order)
                    && prior != order
                {
                    return Err(RuleStorageError::Structure(
                        "ordered effects of one program must share source and program ranks",
                    ));
                }
            }
        }
        if RuleOperationsVersion::parse(input.operations_version.as_str())
            .is_some_and(RuleOperationsVersion::supports_action_contribution_queries)
        {
            // Potential Action writers must be reviewed even if no request
            // selects their output, or their value/activation is neutral.
            work(usage, limits, programs.len())?;
            for ((owner, id), program) in &programs {
                if program.context != RuleEntityKind::Action {
                    continue;
                }
                work(usage, limits, program.effects.len())?;
                for effect in &program.effects {
                    if matches!(&effect.effect, RuleEffectKind::Contribute {entity: RuleEntity::Current, stat, contribution, ..} if stat == &query.stat && contribution == &query.contribution)
                        && !members.contains(&(*owner, *id, &effect.id))
                    {
                        return Err(RuleStorageError::Structure(
                            "potential Action contribution has no declared membership",
                        ));
                    }
                }
            }
        }
        work(usage, limits, applications.len())?;
        for (address, application) in &applications {
            if application.stat == &query.stat
                && application.kind == query.contribution
                && !application_members.contains(address)
            {
                return Err(RuleStorageError::Structure(
                    "potential application contribution has no declared membership",
                ));
            }
        }
    }
    for owner in &input.owners {
        for p in &owner.programs.members {
            for r in &p.reads {
                read(input, index, p, r, &catalog, usage, limits)?;
            }
        }
    }
    if let Some(applications) = &input.effect_applications {
        work(usage, limits, applications.members.len())?;
        for application in &applications.members {
            work(usage, limits, application.program.reads.len())?;
            for r in &application.program.reads {
                read(
                    input,
                    index,
                    &application.program,
                    r,
                    &catalog,
                    usage,
                    limits,
                )?;
            }
        }
    }
    Ok(())
}
