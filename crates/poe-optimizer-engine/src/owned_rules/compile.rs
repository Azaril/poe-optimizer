use super::*;
use poe_optimizer_core::{
    owned_build::DeclaredSlot, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use std::collections::BTreeSet;

fn known<'a, T>(lookup: SchemaLookup<'a, T>, path: &str) -> Result<&'a T, RuleError> {
    match lookup {
        SchemaLookup::Known(v) => Ok(v),
        _ => Err(RuleError::new(
            path,
            "definition/schema is missing, unmapped, foreign or inconsistent",
        )),
    }
}
fn fail(path: &str, message: &str) -> RuleError {
    RuleError::new(path, message)
}
fn check(condition: bool, path: &str, message: &str) -> Result<(), RuleError> {
    if condition {
        Ok(())
    } else {
        Err(fail(path, message))
    }
}
fn add(total: &mut usize, n: usize, max: usize, path: &str) -> Result<(), RuleError> {
    *total = total
        .checked_add(n)
        .filter(|v| *v <= max)
        .ok_or_else(|| fail(path, "resource limit exceeded"))?;
    Ok(())
}
#[derive(Default)]
struct Budget {
    programs: usize,
    reads: usize,
    nodes: usize,
    edges: usize,
    effects: usize,
    work: usize,
}
impl Budget {
    fn work(&mut self, n: usize, l: RuleLimits, p: &str) -> Result<(), RuleError> {
        add(&mut self.work, n, l.max_work, p)
    }
}

pub(super) fn validate_value<I: DefinitionSchemaIndex>(
    v: &ParameterValue,
    index: &I,
    path: &str,
) -> Result<(), RuleError> {
    match v {
        ParameterValue::Quantity(v) => {
            known(index.definition(v.unit()), path)?;
        }
        ParameterValue::Option(v) => {
            known(index.definition(v), path)?;
        }
        ParameterValue::Boolean(_) | ParameterValue::Integer(_) => {}
    }
    Ok(())
}
fn validate_type<I: DefinitionSchemaIndex>(
    t: &ComputedValueType,
    index: &I,
    path: &str,
) -> Result<(), RuleError> {
    if let ComputedValueType::Quantity { unit } = t {
        known(index.definition(unit), path)?;
    }
    Ok(())
}
fn dimension<I: DefinitionSchemaIndex>(
    t: &ComputedValueType,
    expected: UnitDimension,
    index: &I,
    path: &str,
) -> Result<(), RuleError> {
    let ComputedValueType::Quantity { unit } = t else {
        return Err(fail(path, "quantity required"));
    };
    check(
        known(index.definition(unit), path)?.dimension == expected,
        path,
        "wrong unit dimension",
    )
}
fn schema_type<I: DefinitionSchemaIndex>(
    s: &ValueSchema,
    index: &I,
    path: &str,
    l: RuleLimits,
    b: &mut Budget,
) -> Result<ComputedValueType, RuleError> {
    Ok(match s {
        ValueSchema::Boolean => ComputedValueType::Boolean,
        ValueSchema::Integer(r) => {
            check(r.minimum <= r.maximum, path, "reversed integer range")?;
            ComputedValueType::Integer
        }
        ValueSchema::Quantity(r) => {
            validate_value(&ParameterValue::Quantity(r.minimum.clone()), index, path)?;
            validate_value(&ParameterValue::Quantity(r.maximum.clone()), index, path)?;
            check(
                r.minimum.unit() == r.maximum.unit() && r.minimum.value() <= r.maximum.value(),
                path,
                "invalid quantity range",
            )?;
            ComputedValueType::Quantity {
                unit: r.minimum.unit().clone(),
            }
        }
        ValueSchema::Option { allowed } => {
            b.work(allowed.members.len(), l, path)?;
            let mut seen = BTreeSet::new();
            for id in &allowed.members {
                known(index.definition(id), path)?;
                check(seen.insert(id), path, "duplicate option membership")?;
            }
            closure(&allowed.closure, index, path, l, b)?;
            ComputedValueType::Option
        }
    })
}
fn closure<I: DefinitionSchemaIndex>(
    value: &SchemaClosure,
    index: &I,
    path: &str,
    l: RuleLimits,
    b: &mut Budget,
) -> Result<(), RuleError> {
    if let SchemaClosure::Partial { gaps } = value {
        check(!gaps.is_empty(), path, "partial membership requires gaps")?;
        b.work(gaps.len(), l, path)?;
        for gap in gaps {
            subject_namespace(&gap.subject, index, path)?;
        }
    }
    Ok(())
}
fn program_closure<I: DefinitionSchemaIndex>(
    owner: &DefinitionRules,
    index: &I,
    l: RuleLimits,
    b: &mut Budget,
) -> Result<(), RuleError> {
    closure(&owner.programs.closure, index, "programs.closure", l, b)?;
    if let SchemaClosure::Partial { gaps } = &owner.programs.closure {
        let mut codes = BTreeSet::new();
        for gap in gaps {
            check(
                gap.subject == owner.owner && gap.facet == SchemaFacet::GameRules,
                "programs.closure",
                "program gap must name its exact owner and GameRules facet",
            )?;
            check(
                codes.insert(&gap.code),
                "programs.closure",
                "duplicate program gap code",
            )?;
        }
    }
    Ok(())
}
fn subject_namespace<I: DefinitionSchemaIndex>(
    s: &SchemaSubject,
    index: &I,
    path: &str,
) -> Result<(), RuleError> {
    match s {
        SchemaSubject::Definition(d) => check(
            d.namespace() == index.namespace(),
            path,
            "foreign definition namespace",
        ),
        SchemaSubject::Slot(s) => check(
            s.namespace() == index.namespace() && s.declaration().namespace() == index.namespace(),
            path,
            "foreign slot namespace",
        ),
    }
}
struct Ports<'a> {
    declarations: Option<&'a DeclaredSlots>,
    choices: &'a [DeclaredSlot<ChoiceSlotDefId>],
}
fn owner_ports<'a, I: DefinitionSchemaIndex>(
    owner: &SchemaSubject,
    index: &'a I,
    path: &str,
) -> Result<Ports<'a>, RuleError> {
    subject_namespace(owner, index, path)?;
    let mut ports = Ports {
        declarations: None,
        choices: &[],
    };
    macro_rules! state {
        ($e:expr) => {
            match &$e.schema {
                SchemaState::Known(v) => v,
                _ => return Err(fail(path, "owner schema is unmapped")),
            }
        };
    }
    match owner {
        SchemaSubject::Definition(a) => {
            let d = index
                .lookup_definition(a)
                .ok_or_else(|| fail(path, "owner definition is missing"))?;
            check(d.address() == *a, path, "inconsistent owner index")?;
            ports.declarations = match d {
                DefinitionDescriptor::Class(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::Ascendancy(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::Reward(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::ItemTemplate(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::Modifier(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::Gem(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::Skill(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::PassiveNode(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::UsagePolicy(e) => Some(&state!(e).declarations),
                DefinitionDescriptor::PointPool(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::EquipmentSlot(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::Encounter(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::Metric(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::Option(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::ActionPart(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::ActionMode(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::ActionStatSet(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::SkillLinkRole(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::SocketSlot(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::Unit(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::Quality(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::ExternalInput(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::Stat(e) => {
                    state!(e);
                    None
                }
                DefinitionDescriptor::Capability(e) => {
                    state!(e);
                    None
                }
            };
        }
        SchemaSubject::Slot(a) => {
            let d = index
                .lookup_slot(a)
                .ok_or_else(|| fail(path, "owner slot is missing"))?;
            check(d.address() == *a, path, "inconsistent owner slot index")?;
            match d {
                SlotDescriptor::Parameter(e) => {
                    state!(e);
                }
                SlotDescriptor::Choice(e) => {
                    state!(e);
                }
                SlotDescriptor::Grant(e) => {
                    state!(e);
                }
                SlotDescriptor::Actor(e) => {
                    state!(e);
                }
                SlotDescriptor::SkillGrant(e) => {
                    state!(e);
                }
                SlotDescriptor::ActionOutput(e) => {
                    ports.choices = &state!(e).choices.members;
                }
            }
        }
    }
    if let Some(d) = ports.declarations {
        ports.choices = &d.choices.members;
    }
    Ok(ports)
}
fn declaration_subject(d: &SlotOwnerDefId) -> SchemaSubject {
    SchemaSubject::Definition(match d {
        SlotOwnerDefId::Class(v) => DefinitionAddress::Class(v.clone()),
        SlotOwnerDefId::Ascendancy(v) => DefinitionAddress::Ascendancy(v.clone()),
        SlotOwnerDefId::Reward(v) => DefinitionAddress::Reward(v.clone()),
        SlotOwnerDefId::ItemTemplate(v) => DefinitionAddress::ItemTemplate(v.clone()),
        SlotOwnerDefId::Modifier(v) => DefinitionAddress::Modifier(v.clone()),
        SlotOwnerDefId::Gem(v) => DefinitionAddress::Gem(v.clone()),
        SlotOwnerDefId::Skill(v) => DefinitionAddress::Skill(v.clone()),
        SlotOwnerDefId::PassiveNode(v) => DefinitionAddress::PassiveNode(v.clone()),
        SlotOwnerDefId::UsagePolicy(v) => DefinitionAddress::UsagePolicy(v.clone()),
    })
}
fn membership<T: PartialEq>(
    slot: &T,
    members: &[T],
    b: &mut Budget,
    l: RuleLimits,
    path: &str,
) -> Result<(), RuleError> {
    b.work(members.len(), l, path)?;
    check(
        members.contains(slot),
        path,
        "slot is not a declared member of this owner",
    )
}
fn entity(e: RuleEntity, context: RuleEntityKind, path: &str) -> Result<RuleEntityKind, RuleError> {
    Ok(match e {
        RuleEntity::Current => context,
        RuleEntity::Player => RuleEntityKind::Actor,
        RuleEntity::Enemy => RuleEntityKind::Enemy,
        RuleEntity::Environment => RuleEntityKind::Environment,
        RuleEntity::Actor => {
            check(
                matches!(context, RuleEntityKind::Actor | RuleEntityKind::Action),
                path,
                "relative Actor requires actor/action context",
            )?;
            RuleEntityKind::Actor
        }
    })
}
fn stat<'a, I: DefinitionSchemaIndex>(
    id: &StatDefId,
    e: RuleEntity,
    c: RuleEntityKind,
    index: &'a I,
    path: &str,
) -> Result<&'a ComputedValueType, RuleError> {
    let s = known(index.definition(id), path)?;
    check(
        s.targets.contains(&entity(e, c, path)?),
        path,
        "stat target/context mismatch",
    )?;
    validate_type(&s.value, index, path)?;
    Ok(&s.value)
}
fn capability<I: DefinitionSchemaIndex>(
    id: &CapabilityDefId,
    e: RuleEntity,
    c: RuleEntityKind,
    index: &I,
    path: &str,
) -> Result<(), RuleError> {
    check(
        known(index.definition(id), path)?
            .targets
            .contains(&entity(e, c, path)?),
        path,
        "capability target/context mismatch",
    )
}
fn contribution<I: DefinitionSchemaIndex>(
    kind: ContributionKind,
    stat: &ComputedValueType,
    value: &ComputedValueType,
    index: &I,
    path: &str,
) -> Result<(), RuleError> {
    check(numeric(stat), path, "contribution requires numeric stat")?;
    match kind {
        ContributionKind::Add => check(
            stat == value,
            path,
            "Add requires exact stat value type/unit",
        ),
        ContributionKind::Increase => {
            dimension(value, UnitDimension::PercentagePoints, index, path)
        }
        ContributionKind::Multiply => {
            dimension(value, UnitDimension::DimensionlessFactor, index, path)
        }
    }
}
fn quality<I: DefinitionSchemaIndex>(
    owner: &SchemaSubject,
    id: &QualityDefId,
    item: bool,
    index: &I,
    path: &str,
    l: RuleLimits,
    b: &mut Budget,
) -> Result<ValueSchema, RuleError> {
    let policy = match (owner, item) {
        (SchemaSubject::Definition(DefinitionAddress::ItemTemplate(v)), true) => {
            &known(index.definition(v), path)?.quality
        }
        (SchemaSubject::Definition(DefinitionAddress::Gem(v)), false) => {
            &known(index.definition(v), path)?.quality
        }
        _ => return Err(fail(path, "quality read requires its exact item/gem owner")),
    };
    check(
        policy.presence != QualityPresence::Forbidden,
        path,
        "quality forbidden by owner",
    )?;
    membership(id, &policy.allowed_kinds.members, b, l, path)?;
    let s = ValueSchema::Quantity(known(index.definition(id), path)?.amount.clone());
    schema_type(&s, index, path, l, b)?;
    Ok(s)
}
fn read<I: DefinitionSchemaIndex>(
    r: &RuleRead,
    p: &RuleProgram,
    (owner, ports): (&SchemaSubject, &Ports<'_>),
    index: &I,
    path: &str,
    l: RuleLimits,
    b: &mut Budget,
) -> Result<CompiledRead, RuleError> {
    validate_type(&r.value_type, index, path)?;
    let mut constraint = None;
    let ty = match &r.source {
        RuleReadSource::Parameter { slot } => {
            check(
                declaration_subject(&slot.declaration) == *owner,
                path,
                "parameter declaration does not equal owner",
            )?;
            let members = ports
                .declarations
                .map(|v| v.parameters.members.as_slice())
                .unwrap_or(&[]);
            membership(slot, members, b, l, path)?;
            constraint = Some(known(index.slot(slot), path)?.value.clone());
            schema_type(constraint.as_ref().expect("set"), index, path, l, b)?
        }
        RuleReadSource::Choice { slot } => {
            if !matches!(owner, SchemaSubject::Slot(SlotAddress::ActionOutput(_))) {
                check(
                    declaration_subject(&slot.declaration) == *owner,
                    path,
                    "choice declaration does not equal owner",
                )?;
            }
            membership(slot, ports.choices, b, l, path)?;
            constraint = Some(known(index.slot(slot), path)?.value.clone());
            schema_type(constraint.as_ref().expect("set"), index, path, l, b)?
        }
        RuleReadSource::CharacterLevel => {
            if let SchemaSubject::Definition(DefinitionAddress::Class(id)) = owner {
                constraint = Some(ValueSchema::Integer(
                    known(index.definition(id), path)?.level.clone(),
                ));
            }
            ComputedValueType::Integer
        }
        RuleReadSource::GemLevel => {
            let SchemaSubject::Definition(DefinitionAddress::Gem(id)) = owner else {
                return Err(fail(path, "GemLevel requires gem owner"));
            };
            constraint = Some(ValueSchema::Integer(
                known(index.definition(id), path)?.level.clone(),
            ));
            ComputedValueType::Integer
        }
        RuleReadSource::ItemLevel => {
            let SchemaSubject::Definition(DefinitionAddress::ItemTemplate(id)) = owner else {
                return Err(fail(path, "ItemLevel requires item owner"));
            };
            constraint = Some(ValueSchema::Integer(
                known(index.definition(id), path)?.item_level.clone(),
            ));
            ComputedValueType::Integer
        }
        RuleReadSource::ItemQualityAmount { quality: id }
        | RuleReadSource::GemQualityAmount { quality: id } => {
            constraint = Some(quality(
                owner,
                id,
                matches!(r.source, RuleReadSource::ItemQualityAmount { .. }),
                index,
                path,
                l,
                b,
            )?);
            schema_type(constraint.as_ref().expect("set"), index, path, l, b)?
        }
        RuleReadSource::HasItemQuality { quality: id }
        | RuleReadSource::HasGemQuality { quality: id } => {
            quality(
                owner,
                id,
                matches!(r.source, RuleReadSource::HasItemQuality { .. }),
                index,
                path,
                l,
                b,
            )?;
            ComputedValueType::Boolean
        }
        RuleReadSource::Stat {
            entity: e,
            stat: id,
        } => stat(id, *e, p.context, index, path)?.clone(),
        RuleReadSource::Capability {
            entity: e,
            capability: id,
        } => {
            capability(id, *e, p.context, index, path)?;
            ComputedValueType::Boolean
        }
        RuleReadSource::External { entity: e, input } => {
            let s = known(index.definition(input), path)?;
            let target = match entity(*e, p.context, path)? {
                RuleEntityKind::Actor => AssumptionTargetKind::Actor,
                RuleEntityKind::Enemy => AssumptionTargetKind::Enemy,
                RuleEntityKind::Environment => AssumptionTargetKind::Environment,
                _ => return Err(fail(path, "external read has unsupported target context")),
            };
            check(
                s.targets.contains(&target),
                path,
                "external input target mismatch",
            )?;
            constraint = Some(s.value.clone());
            schema_type(&s.value, index, path, l, b)?
        }
        RuleReadSource::Contributions {
            entity: e,
            stat: id,
            contribution: kind,
            reduction,
            empty,
        } => {
            let st = stat(id, *e, p.context, index, path)?;
            validate_value(empty, index, path)?;
            contribution(*kind, st, &r.value_type, index, path)?;
            let product = *kind == ContributionKind::Multiply;
            check(
                *reduction
                    == if product {
                        ContributionReduction::Product
                    } else {
                        ContributionReduction::Sum
                    },
                path,
                "contribution reduction mismatch",
            )?;
            let identity = match empty {
                ParameterValue::Integer(v) => v.get() as f64,
                ParameterValue::Quantity(v) => v.value(),
                _ => return Err(fail(path, "numeric empty identity required")),
            };
            check(
                value_type(empty) == r.value_type && identity == if product { 1.0 } else { 0.0 },
                path,
                "wrong contribution empty identity/type",
            )?;
            r.value_type.clone()
        }
    };
    check(
        ty == r.value_type,
        path,
        "declared read type does not match source schema",
    )?;
    if let Some(s) = &constraint {
        schema_type(s, index, path, l, b)?;
    }
    Ok(CompiledRead {
        id: r.id.clone(),
        ty,
        schema: constraint,
    })
}
fn lower(
    e: &RuleExpression,
    nodes: &BTreeMap<OwnedDefinitionKey, usize>,
    reads: &BTreeMap<OwnedDefinitionKey, usize>,
    path: &str,
) -> Result<Op, RuleError> {
    let n = |id: &OwnedDefinitionKey| {
        nodes
            .get(id)
            .copied()
            .ok_or_else(|| fail(path, "unknown node reference"))
    };
    Ok(match e {
        RuleExpression::Literal { value } => Op::Literal(value.clone()),
        RuleExpression::Read { input } => Op::Read(
            *reads
                .get(input)
                .ok_or_else(|| fail(path, "unknown read reference"))?,
        ),
        RuleExpression::Add { left, right } => Op::Binary(Binary::Add, n(left)?, n(right)?),
        RuleExpression::Subtract { left, right } => {
            Op::Binary(Binary::Subtract, n(left)?, n(right)?)
        }
        RuleExpression::Minimum { left, right } => Op::Binary(Binary::Minimum, n(left)?, n(right)?),
        RuleExpression::Maximum { left, right } => Op::Binary(Binary::Maximum, n(left)?, n(right)?),
        RuleExpression::Scale { value, factor } => Op::Binary(Binary::Scale, n(value)?, n(factor)?),
        RuleExpression::ScaleInteger { value, count } => {
            Op::Binary(Binary::ScaleInteger, n(value)?, n(count)?)
        }
        RuleExpression::DivideFactor { value, divisor } => {
            Op::Binary(Binary::DivideFactor, n(value)?, n(divisor)?)
        }
        RuleExpression::Ratio {
            numerator,
            denominator,
            unit,
        } => Op::Ratio(n(numerator)?, n(denominator)?, unit.clone()),
        RuleExpression::PercentAsFactor { percent, unit } => Op::Percent(n(percent)?, unit.clone()),
        RuleExpression::Round {
            value,
            quantum,
            mode,
        } => Op::Round(n(value)?, quantum.clone(), *mode),
        RuleExpression::Compare {
            operation,
            left,
            right,
        } => Op::Compare(*operation, n(left)?, n(right)?),
        RuleExpression::Not { value } => Op::Not(n(value)?),
        RuleExpression::All { values } => Op::All(values.iter().map(n).collect::<Result<_, _>>()?),
        RuleExpression::Any { values } => Op::Any(values.iter().map(n).collect::<Result<_, _>>()?),
        RuleExpression::Select {
            condition,
            when_true,
            when_false,
        } => Op::Select(n(condition)?, n(when_true)?, n(when_false)?),
    })
}
fn infer<I: DefinitionSchemaIndex>(
    op: &Op,
    types: &[Option<ComputedValueType>],
    reads: &[CompiledRead],
    index: &I,
    path: &str,
) -> Result<ComputedValueType, RuleError> {
    let t = |i: usize| types[i].as_ref().expect("topological order");
    let same_numeric = |a: usize, b: usize| {
        check(
            numeric(t(a)) && t(a) == t(b),
            path,
            "numeric operands require identical types/units",
        )
    };
    Ok(match op {
        Op::Literal(v) => {
            validate_value(v, index, path)?;
            value_type(v)
        }
        Op::Read(i) => reads[*i].ty.clone(),
        Op::Binary(kind, a, b) => {
            match kind {
                Binary::Add | Binary::Subtract | Binary::Minimum | Binary::Maximum => {
                    same_numeric(*a, *b)?
                }
                Binary::Scale | Binary::DivideFactor => {
                    check(
                        matches!(t(*a), ComputedValueType::Quantity { .. }),
                        path,
                        "scaled value must be a quantity",
                    )?;
                    dimension(t(*b), UnitDimension::DimensionlessFactor, index, path)?;
                }
                Binary::ScaleInteger => check(
                    numeric(t(*a)) && *t(*b) == ComputedValueType::Integer,
                    path,
                    "ScaleInteger requires numeric value and integer count",
                )?,
            }
            t(*a).clone()
        }
        Op::Ratio(a, b, unit) => {
            same_numeric(*a, *b)?;
            check(
                matches!(t(*a), ComputedValueType::Quantity { .. }),
                path,
                "ratio requires quantities",
            )?;
            let out = ComputedValueType::Quantity { unit: unit.clone() };
            dimension(&out, UnitDimension::DimensionlessFactor, index, path)?;
            out
        }
        Op::Percent(a, unit) => {
            dimension(t(*a), UnitDimension::PercentagePoints, index, path)?;
            let out = ComputedValueType::Quantity { unit: unit.clone() };
            dimension(&out, UnitDimension::DimensionlessFactor, index, path)?;
            out
        }
        Op::Round(a, q, _) => {
            validate_value(&ParameterValue::Quantity(q.clone()), index, path)?;
            check(
                *t(*a) == value_type(&ParameterValue::Quantity(q.clone())) && q.value() > 0.0,
                path,
                "round quantum must be positive and share the exact value unit",
            )?;
            t(*a).clone()
        }
        Op::Compare(kind, a, b) => {
            check(t(*a) == t(*b), path, "comparison types/units differ")?;
            check(
                *kind == RuleComparison::Equal || numeric(t(*a)),
                path,
                "ordered comparison requires numeric values",
            )?;
            ComputedValueType::Boolean
        }
        Op::Not(a) => {
            check(
                *t(*a) == ComputedValueType::Boolean,
                path,
                "Not requires boolean",
            )?;
            ComputedValueType::Boolean
        }
        Op::All(v) | Op::Any(v) => {
            check(
                v.iter().all(|i| *t(*i) == ComputedValueType::Boolean),
                path,
                "All/Any require booleans",
            )?;
            ComputedValueType::Boolean
        }
        Op::Select(c, a, b) => {
            check(
                *t(*c) == ComputedValueType::Boolean && t(*a) == t(*b),
                path,
                "Select requires boolean condition and matching branch types",
            )?;
            t(*a).clone()
        }
    })
}
fn program<I: DefinitionSchemaIndex>(
    p: &RuleProgram,
    owner: &DefinitionRules,
    ports: &Ports<'_>,
    index: &I,
    l: RuleLimits,
    b: &mut Budget,
    path: &str,
) -> Result<CompiledProgram, RuleError> {
    let mut reads = Vec::with_capacity(p.reads.len());
    let mut read_index = BTreeMap::new();
    for r in &p.reads {
        check(
            read_index.insert(r.id.clone(), reads.len()).is_none(),
            path,
            "duplicate read ID",
        )?;
        reads.push(read(
            r,
            p,
            (&owner.owner, ports),
            index,
            &format!("{path}.reads.{}", r.id),
            l,
            b,
        )?);
    }
    let node_index = p
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect::<BTreeMap<_, _>>();
    check(node_index.len() == p.nodes.len(), path, "duplicate node ID")?;
    let mut ops = Vec::with_capacity(p.nodes.len());
    let mut indegree = vec![0usize; p.nodes.len()];
    let mut consumers = vec![Vec::new(); p.nodes.len()];
    for (i, n) in p.nodes.iter().enumerate() {
        let op = lower(&n.expression, &node_index, &read_index, path)?;
        let deps = op.dependencies();
        add(
            &mut b.edges,
            deps.len() + usize::from(matches!(op, Op::Read(_))),
            l.max_edges,
            path,
        )?;
        b.work(deps.len() + 1, l, path)?;
        indegree[i] = deps.len();
        for d in deps {
            consumers[d].push(i);
        }
        ops.push(op);
    }
    let mut ready = indegree
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect::<BTreeSet<_>>();
    let mut types = vec![None; p.nodes.len()];
    let mut visited = 0;
    while let Some(i) = ready.pop_first() {
        types[i] = Some(infer(
            &ops[i],
            &types,
            &reads,
            index,
            &format!("{path}.nodes.{}", p.nodes[i].id),
        )?);
        visited += 1;
        for &next in &consumers[i] {
            indegree[next] -= 1;
            if indegree[next] == 0 {
                ready.insert(next);
            }
        }
    }
    check(
        visited == p.nodes.len(),
        path,
        "expression graph contains a cycle",
    )?;
    let nodes = p
        .nodes
        .iter()
        .zip(ops)
        .zip(types)
        .map(|((n, expression), ty)| CompiledNode {
            id: n.id.clone(),
            expression,
            ty: ty.expect("all nodes typed"),
        })
        .collect::<Vec<_>>();
    let mut effects = Vec::with_capacity(p.effects.len());
    let mut ids = BTreeSet::new();
    for e in &p.effects {
        let ep = format!("{path}.effects.{}", e.id);
        check(ids.insert(&e.id), &ep, "duplicate effect ID")?;
        let resolve = |id: &OwnedDefinitionKey| {
            node_index
                .get(id)
                .copied()
                .ok_or_else(|| fail(&ep, "unknown effect node"))
        };
        let when = e.when.as_ref().map(resolve).transpose()?;
        if let Some(i) = when {
            check(
                nodes[i].ty == ComputedValueType::Boolean,
                &ep,
                "effect guard must be boolean",
            )?;
        }
        let (id, expected) = match &e.effect {
            RuleEffectKind::Contribute {
                entity: e,
                stat: id,
                contribution: kind,
                value,
            } => {
                let i = resolve(value)?;
                contribution(
                    *kind,
                    stat(id, *e, p.context, index, &ep)?,
                    &nodes[i].ty,
                    index,
                    &ep,
                )?;
                (value, nodes[i].ty.clone())
            }
            RuleEffectKind::Derive {
                entity: e,
                stat: id,
                value,
            } => (value, stat(id, *e, p.context, index, &ep)?.clone()),
            RuleEffectKind::Capability {
                entity: e,
                capability: id,
                enabled,
            } => {
                capability(id, *e, p.context, index, &ep)?;
                (enabled, ComputedValueType::Boolean)
            }
            RuleEffectKind::SupportApplicability { applicable } => {
                check(
                    p.context == RuleEntityKind::Action,
                    &ep,
                    "support applicability requires action context",
                )?;
                (applicable, ComputedValueType::Boolean)
            }
            RuleEffectKind::ActivateGrant { slot, enabled } => {
                check(
                    declaration_subject(&slot.declaration) == owner.owner,
                    &ep,
                    "grant declaration does not equal owner",
                )?;
                membership(
                    slot,
                    ports
                        .declarations
                        .map(|d| d.grants.members.as_slice())
                        .unwrap_or(&[]),
                    b,
                    l,
                    &ep,
                )?;
                known(index.slot(slot), &ep)?;
                (enabled, ComputedValueType::Boolean)
            }
            RuleEffectKind::Requirement { satisfied, .. } => {
                (satisfied, ComputedValueType::Boolean)
            }
        };
        let value = resolve(id)?;
        check(
            nodes[value].ty == expected,
            &ep,
            "effect value type/unit mismatch",
        )?;
        add(
            &mut b.edges,
            1 + usize::from(when.is_some()),
            l.max_edges,
            &ep,
        )?;
        effects.push(CompiledEffect {
            id: e.id.clone(),
            kind: e.effect.clone(),
            when,
            value,
        });
    }
    Ok(CompiledProgram {
        owner: owner.owner.clone(),
        id: p.id.clone(),
        closure: owner.programs.closure.clone(),
        reads,
        read_index,
        nodes,
        effects,
    })
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    l: RuleLimits,
) -> Result<CompiledRulePackage, RuleError> {
    l.validate()?;
    check(
        input.schema_version == OWNED_RULE_PACKAGE_VERSION,
        "schema_version",
        "unsupported rule package version",
    )?;
    check(
        input.operations_version.as_str() == OWNED_RULE_OPERATIONS_VERSION,
        "operations_version",
        "unsupported operation version",
    )?;
    input
        .definitions
        .validate()
        .map_err(|e| RuleError::new("definitions", e))?;
    check(
        &input.definitions == index.identity() && &input.namespace == index.namespace(),
        "definitions",
        "definition identity/namespace mismatch",
    )?;
    check(
        input.owners.len() <= l.max_owners,
        "owners",
        "owner count exceeds limit",
    )?;
    let mut b = Budget::default();
    for o in &input.owners {
        add(
            &mut b.programs,
            o.programs.members.len(),
            l.max_programs,
            "programs",
        )?;
        for p in &o.programs.members {
            add(&mut b.reads, p.reads.len(), l.max_reads, "reads")?;
            add(&mut b.nodes, p.nodes.len(), l.max_nodes, "nodes")?;
            add(&mut b.effects, p.effects.len(), l.max_effects, "effects")?;
            for n in &p.nodes {
                if let RuleExpression::All { values } | RuleExpression::Any { values } =
                    &n.expression
                {
                    check(
                        values.len() <= l.max_edges,
                        "nodes",
                        "list edge limit exceeded",
                    )?;
                }
            }
        }
    }
    // Streaming bound before cloning/index allocation. Source order of effects and
    // boolean operands remains meaningful; declaration tables are canonicalized.
    digest_owned("owned-rule-input-v1", input, l.max_wire_bytes)
        .map_err(|e| RuleError::new("wire", e.to_string()))?;
    let mut input = input.clone();
    input.owners.sort_by_key(|o| SubjectKey::from(&o.owner));
    let mut owners = BTreeSet::new();
    let mut programs = BTreeMap::new();
    for o in &mut input.owners {
        check(
            owners.insert(SubjectKey::from(&o.owner)),
            "owners",
            "duplicate owner",
        )?;
        let ports = owner_ports(&o.owner, index, "owner")?;
        program_closure(o, index, l, &mut b)?;
        o.programs.members.sort_by(|a, b| a.id.cmp(&b.id));
        for p in &mut o.programs.members {
            p.reads.sort_by(|a, b| a.id.cmp(&b.id));
            p.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        }
        for p in &o.programs.members {
            let key = (SubjectKey::from(&o.owner), p.id.clone());
            check(
                !programs.contains_key(&key),
                "programs",
                "duplicate program ID",
            )?;
            b.work(
                p.reads.len() + p.nodes.len() + p.effects.len() + 1,
                l,
                "program",
            )?;
            programs.insert(
                key,
                program(p, o, &ports, index, l, &mut b, &format!("program.{}", p.id))?,
            );
        }
    }
    let identity = digest_owned("owned-rule-programs-v1", &input, l.max_wire_bytes)
        .map_err(|e| RuleError::new("wire", e.to_string()))?;
    Ok(CompiledRulePackage {
        input,
        identity,
        programs,
        limits: l,
    })
}
