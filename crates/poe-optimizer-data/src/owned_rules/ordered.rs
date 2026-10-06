//! Bounded definition-side checks. Concrete occurrence membership belongs to Engine.
use super::*;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{EquipmentSlotDefId, OwnedDefinitionKey},
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
    queries: BTreeMap<&'a OwnedDefinitionKey, &'a OrderedContributionQuery>,
    group_types: BTreeMap<(&'a OwnedDefinitionKey, &'a OwnedDefinitionKey), ComputedValueType>,
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
    query: &OrderedContributionQuery,
    group: &OrderedContributionGroup,
    stat: &ComputedValueType,
    index: &I,
) -> Result<ComputedValueType, RuleStorageError> {
    let invalid = RuleStorageError::Structure;
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
    member: &OrderedContributionMember,
    program: &RuleProgram,
    index: &I,
    usage: &mut RuleStorageUse,
    limits: RuleStorageLimits,
) -> Result<Option<EquipmentLane>, RuleStorageError> {
    let invalid = RuleStorageError::Structure;
    let definition = match &member.owner {
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
    let (valid, context, equipment) = match &member.order.origin {
        OrderedContributionOrigin::Character => (
            matches!(
                definition,
                DefinitionAddress::Class(_) | DefinitionAddress::Ascendancy(_)
            ),
            RuleEntityKind::Actor,
            None,
        ),
        OrderedContributionOrigin::Allocation => (
            matches!(definition, DefinitionAddress::PassiveNode(_)),
            RuleEntityKind::Actor,
            None,
        ),
        OrderedContributionOrigin::EquipmentUse { slots } => (
            matches!(definition, DefinitionAddress::ItemTemplate(_)),
            RuleEntityKind::EquipmentUse,
            Some((false, slots)),
        ),
        OrderedContributionOrigin::ItemModifier { slots } => (
            matches!(definition, DefinitionAddress::Modifier(_)),
            RuleEntityKind::EquipmentUse,
            Some((true, slots)),
        ),
    };
    if !valid || program.context != context {
        return Err(invalid(
            "ordered origin policy does not match its owner and program context",
        ));
    }
    let Some((modifier, slots)) = equipment else {
        return Ok(None);
    };
    add(&mut usage.ordered_slots, slots.len())?;
    work(usage, limits, slots.len())?;
    if slots.is_empty() {
        return Err(invalid(
            "ordered equipment policy needs explicit slot ranks",
        ));
    }
    let mut mapping = BTreeMap::new();
    let mut ranks = BTreeSet::new();
    for slot in slots {
        if !matches!(index.definition(&slot.slot), SchemaLookup::Known(_))
            || mapping.insert(slot.slot.clone(), slot.rank).is_some()
            || !ranks.insert(slot.rank)
        {
            return Err(invalid(
                "duplicate, unknown or ambiguously ranked ordered equipment slot",
            ));
        }
    }
    Ok(Some(EquipmentLane {
        modifier,
        source_rank: member.order.source_rank,
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
    let RuleReadSource::OrderedContributions {
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
        .is_some_and(RuleOperationsVersion::supports_ordered_contributions)
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
    // These queries initially consume direct actor/equipment contributions. In
    // particular they cannot bypass the separate source-property/Skill contracts.
    let valid = match entity {
        RuleEntity::Player => true,
        RuleEntity::Actor => matches!(p.context, RuleEntityKind::Actor | RuleEntityKind::Action),
        RuleEntity::Current => matches!(
            p.context,
            RuleEntityKind::Actor | RuleEntityKind::EquipmentUse
        ),
        _ => false,
    };
    if !valid {
        return Err(invalid(
            "ordered read has an unsupported relative recipient scope",
        ));
    }
    let target = if *entity == RuleEntity::Current {
        p.context
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
    let enabled = RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_ordered_contributions);
    let Some(registry) = &input.ordered_contributions else {
        if enabled {
            return Err(RuleStorageError::Structure(
                "ordered contributions require an explicit V21 inventory",
            ));
        }
        // No new counters or canonical fields enter historical receipts. Existing
        // package/program limits bound these scans; new reads still cannot hide in
        // unused programs or effect applications under an older version.
        let forbidden = |p: &RuleProgram| {
            p.reads
                .iter()
                .any(|r| matches!(r.source, RuleReadSource::OrderedContributions { .. }))
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
        if !matches!(
            stat.value,
            ComputedValueType::Integer | ComputedValueType::Quantity { .. }
        ) {
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
                if !members.insert((owner_key(&member.owner), &member.program, &member.effect)) {
                    return Err(RuleStorageError::Structure(
                        "ordered producer occurs more than once across query groups",
                    ));
                }
                if !subject_known(&member.owner, index) {
                    return Err(RuleStorageError::Structure(
                        "ordered member owner must be known",
                    ));
                }
                let Some(program) = programs.get(&(owner_key(&member.owner), &member.program))
                else {
                    return Err(RuleStorageError::Structure(
                        "ordered member references an unknown producer program",
                    ));
                };
                work(usage, limits, program.effects.len())?;
                let matching: Vec<_> = program
                    .effects
                    .iter()
                    .filter(|e| e.id == member.effect)
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
                let target = match entity {
                    RuleEntity::Player | RuleEntity::Actor => RuleEntityKind::Actor,
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
                if let Some(lane) = order(member, program, index, usage, limits)?
                    && let Some(prior) = lanes.insert(lane.modifier, lane.clone())
                    && prior != lane
                {
                    return Err(RuleStorageError::Structure(
                        "ordered equipment origin lane must share source and slot ranks",
                    ));
                }
                let order = (member.order.source_rank, member.order.program_rank);
                if let Some(prior) =
                    owner_ranks.insert(owner_key(&member.owner), member.order.source_rank)
                    && prior != member.order.source_rank
                {
                    return Err(RuleStorageError::Structure(
                        "ordered programs of one owner must share source rank",
                    ));
                }
                if let Some(prior) =
                    program_ranks.insert((owner_key(&member.owner), &member.program), order)
                    && prior != order
                {
                    return Err(RuleStorageError::Structure(
                        "ordered effects of one program must share source and program ranks",
                    ));
                }
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
