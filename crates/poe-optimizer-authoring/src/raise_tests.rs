//! Raising is an offline representation change, not a new source of rule authority.
//! The comparator below rewrites only node references in independently held Core IR.
use crate::{lowering_tests as native, operation_tests as operations, program_tests, *};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits, UnsupportedDomainCause,
};
use std::collections::{BTreeMap, BTreeSet};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn id<K: DefinitionDomain>(value: &str) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("authored-rule-tests", "v1").unwrap(),
        value,
    )
    .unwrap()
}

fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}

fn quantity(value: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, id(unit)).unwrap())
}

fn node(name: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(name),
        expression,
    }
}

fn literal(name: &str, value: ParameterValue) -> RuleNode {
    node(name, RuleExpression::Literal { value })
}

// Exhaustive over Core variants: adding an operation requires extending this
// independent comparator. Static keys (reads, tables, units, targets, codes) are
// deliberately not node identities and are retained byte-for-byte/typed-exact.
fn node_references(
    expression: &mut RuleExpression,
    mut change: impl FnMut(&mut OwnedDefinitionKey),
) {
    match expression {
        RuleExpression::Literal { .. } | RuleExpression::Read { .. } => {}
        RuleExpression::CheckNumericDomain { value, .. }
        | RuleExpression::PercentAsFactor { percent: value, .. }
        | RuleExpression::Round { value, .. }
        | RuleExpression::QuantizeInteger { value, .. }
        | RuleExpression::Not { value } => change(value),
        RuleExpression::LookupIntegerTable { key, .. } => change(key),
        RuleExpression::Add { left, right }
        | RuleExpression::Subtract { left, right }
        | RuleExpression::Minimum { left, right }
        | RuleExpression::Maximum { left, right }
        | RuleExpression::Compare { left, right, .. }
        | RuleExpression::Scale {
            value: left,
            factor: right,
        }
        | RuleExpression::ScaleInteger {
            value: left,
            count: right,
        }
        | RuleExpression::DivideFactor {
            value: left,
            divisor: right,
        }
        | RuleExpression::Ratio {
            numerator: left,
            denominator: right,
            ..
        } => {
            change(left);
            change(right);
        }
        RuleExpression::All { values } | RuleExpression::Any { values } => {
            for value in values {
                change(value);
            }
        }
        RuleExpression::Select {
            condition,
            when_true,
            when_false,
        } => {
            change(condition);
            change(when_true);
            change(when_false);
        }
        RuleExpression::OrdinaryTiming { recipe } => {
            for value in [
                &mut recipe.base_time,
                &mut recipe.increased_percent,
                &mut recipe.more_multiplier,
                &mut recipe.additional_attack_time,
                &mut recipe.additional_cast_time,
                &mut recipe.action_speed_multiplier,
                &mut recipe.repeats,
                &mut recipe.server_tick_rate,
            ] {
                change(value);
            }
        }
    }
}

fn effect_references(effect: &mut RuleEffect, mut change: impl FnMut(&mut OwnedDefinitionKey)) {
    if let Some(when) = &mut effect.when {
        change(when);
    }
    let value = match &mut effect.effect {
        RuleEffectKind::Contribute { value, .. }
        | RuleEffectKind::ProjectModifierTransform { value, .. }
        | RuleEffectKind::Derive { value, .. }
        | RuleEffectKind::ProjectSkillParameter { value, .. }
        | RuleEffectKind::ProjectActorStat { value, .. } => value,
        RuleEffectKind::Capability { enabled, .. }
        | RuleEffectKind::ActivateGrant { enabled, .. } => enabled,
        RuleEffectKind::SupportApplicability { applicable } => applicable,
        RuleEffectKind::Requirement { satisfied, .. } => satisfied,
    };
    change(value);
}

pub(super) fn compare_graph(
    original: &RuleProgram,
    raised: &RaisedProgram,
    lowered: &LoweredProgram,
) {
    let original_ids: BTreeSet<_> = original.nodes.iter().map(|node| node.id.clone()).collect();
    assert_eq!(original_ids.len(), original.nodes.len());
    let names: BTreeMap<_, _> = raised
        .nodes
        .iter()
        .map(|node| (node.original.clone(), node.name.clone()))
        .collect();
    assert_eq!(names.len(), raised.nodes.len());
    assert_eq!(names.keys().cloned().collect::<BTreeSet<_>>(), original_ids);
    assert_eq!(names.values().collect::<BTreeSet<_>>().len(), names.len());
    let bindings: BTreeMap<_, _> = lowered
        .bindings
        .iter()
        .map(|binding| (binding.name.as_str(), binding.value.clone()))
        .collect();
    assert_eq!(bindings.len(), lowered.bindings.len());
    assert_eq!(
        names.values().map(String::as_str).collect::<BTreeSet<_>>(),
        bindings.keys().copied().collect()
    );
    let mapping: BTreeMap<_, _> = names
        .iter()
        .map(|(id, name)| (id.clone(), bindings[name.as_str()].clone()))
        .collect();
    assert_eq!(
        mapping.values().collect::<BTreeSet<_>>().len(),
        original.nodes.len(),
        "raising must not coalesce separate nodes"
    );

    // Also prove that the public mapping follows a topological emission order.
    let positions: BTreeMap<_, _> = raised
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.original.clone(), position))
        .collect();
    let read_ids: BTreeSet<_> = original.reads.iter().map(|read| read.id.clone()).collect();
    let mut expected = original.clone();
    for node in &mut expected.nodes {
        let original_id = node.id.clone();
        if let RuleExpression::Read { input } = &node.expression {
            assert!(read_ids.contains(input));
        }
        node_references(&mut node.expression, |reference| {
            assert!(positions[reference] < positions[&original_id]);
            *reference = mapping[reference].clone();
        });
        node.id = mapping[&original_id].clone();
    }
    for effect in &mut expected.effects {
        effect_references(effect, |reference| *reference = mapping[reference].clone());
    }
    assert_eq!(lowered.program.id, expected.id);
    assert_eq!(lowered.program.context, expected.context);
    let keyed_nodes = |nodes: &[RuleNode]| {
        nodes
            .iter()
            .map(|node| (node.id.clone(), node.expression.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(lowered.program.nodes.len(), expected.nodes.len());
    assert_eq!(
        keyed_nodes(&lowered.program.nodes),
        keyed_nodes(&expected.nodes)
    );
    expected.reads.sort_by(|left, right| left.id.cmp(&right.id));
    let mut actual_reads = lowered.program.reads.clone();
    actual_reads.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(actual_reads, expected.reads);
    assert_eq!(
        lowered.program.effects, expected.effects,
        "effect order is semantic and is never sorted"
    );
}

fn roundtrip(
    original: &RuleProgram,
    f: &native::fixture::Fixture,
    symbols: &SymbolRegistry,
) -> (RaisedProgram, LoweredProgram) {
    let raised = raise_program(original, symbols, RaiseLimits::default()).unwrap();
    let parsed = parse_program(&raised.source, ProgramParseLimits::default()).unwrap();
    let lowered = lower_program(
        &parsed,
        &f.schema,
        symbols,
        &f.rules.tables,
        ProgramLowerLimits::default(),
    )
    .unwrap();
    compare_graph(original, &raised, &lowered);
    (raised, lowered)
}

#[test]
fn every_existing_fixture_program_roundtrips_and_executes_independent_expected_cases() {
    let f = native::fixture::fixture();
    let symbols = program_tests::symbols(&f);
    let original =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    let mut after = f.rules.clone();
    for owner in &mut after.owners {
        for program in &mut owner.programs.members {
            *program = roundtrip(program, &f, &symbols).1.program;
        }
    }
    // Only programs change. Preserve every other package field and owner closure.
    let mut inverse = after.clone();
    for (owner, before) in inverse.owners.iter_mut().zip(&f.rules.owners) {
        owner.programs.members.clone_from(&before.programs.members);
    }
    assert_eq!(inverse, f.rules);
    let compiled = CompiledRulePackage::compile(&after, &f.schema, RuleLimits::default()).unwrap();
    for case in &f.cases {
        let facts: Vec<_> = case
            .facts
            .iter()
            .map(|(read, value)| RuleFact {
                read: read.clone(),
                value: value.clone(),
            })
            .collect();
        let before = original
            .evaluate(
                &case.owner,
                &case.program,
                &facts,
                &f.schema,
                &mut original.new_scratch(),
            )
            .unwrap();
        let after = compiled
            .evaluate(
                &case.owner,
                &case.program,
                &facts,
                &f.schema,
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(after.effects.len(), case.expected.len());
        for ((actual, previous), expected) in after
            .effects
            .iter()
            .zip(&before.effects)
            .zip(&case.expected)
        {
            assert_eq!(actual.id, expected.id, "{}", case.name);
            assert_eq!(actual.disposition, previous.disposition, "{}", case.name);
            assert_eq!(
                actual.disposition,
                expected
                    .value
                    .clone()
                    .map_or(EffectDisposition::Inactive, |value| {
                        EffectDisposition::Applied { value }
                    }),
                "{}",
                case.name
            );
        }
    }
}

#[test]
fn all_read_sources_preserve_exact_typed_metadata_without_claiming_context_authority() {
    let f = program_tests::enriched_fixture();
    let reads = program_tests::read_constructor_cases()
        .into_iter()
        .enumerate()
        .map(|(position, (_, _, value_type, source))| RuleRead {
            id: key(&format!("read-{position}")),
            value_type,
            source,
        })
        .collect();
    let program = RuleProgram {
        id: key("read-matrix"),
        context: RuleEntityKind::Actor,
        reads,
        nodes: vec![],
        effects: vec![],
    };
    let (_, lowered) = roundtrip(&program, &f, &program_tests::symbols(&f));
    assert!(lowered.program.nodes.is_empty());
    // These independent constructors cannot all be legal in one receiving context.
    let mut rules = f.rules.clone();
    rules.owners[0].programs.members.push(lowered.program);
    assert!(CompiledRulePackage::compile(&rules, &f.schema, RuleLimits::default()).is_err());
}

fn effect_matrix() -> RuleProgram {
    let item = SlotOwnerDefId::ItemTemplate(id("item.template"));
    let kinds = vec![
        RuleEffectKind::Contribute {
            entity: RuleEntity::Current,
            stat: id("stat.local-damage"),
            contribution: ContributionKind::Add,
            value: key("amount"),
        },
        RuleEffectKind::ProjectModifierTransform {
            stat: id("stat.factor"),
            targets: vec![
                ModifierTransformTarget {
                    definition: id("modifier.conditioned"),
                    when: None,
                },
                ModifierTransformTarget {
                    definition: id("modifier.other"),
                    when: Some(id("stat.eligible")),
                },
            ],
            order: BoundedInteger::new(2).unwrap(),
            operation: ModifierTransformOperation::Multiply,
            value: key("factor"),
        },
        RuleEffectKind::Derive {
            entity: RuleEntity::Current,
            stat: id("stat.local-damage"),
            value: key("amount"),
        },
        RuleEffectKind::Capability {
            entity: RuleEntity::Current,
            capability: id("capability.supported-action"),
            enabled: key("flag"),
        },
        RuleEffectKind::SupportApplicability {
            applicable: key("flag"),
        },
        RuleEffectKind::ActivateGrant {
            slot: DeclaredSlot {
                declaration: item.clone(),
                slot: id("grant.actor"),
            },
            enabled: key("flag"),
        },
        RuleEffectKind::ProjectSkillParameter {
            skill: DeclaredSlot {
                declaration: item.clone(),
                slot: id("supply.skill"),
            },
            parameter: DeclaredSlot {
                declaration: SlotOwnerDefId::Skill(id("skill.generated")),
                slot: id("parameter.skill"),
            },
            value: key("count"),
        },
        RuleEffectKind::ProjectActorStat {
            actor: DeclaredSlot {
                declaration: item,
                slot: id("actor.supplied"),
            },
            stat: id("stat.damage"),
            value: key("amount"),
        },
        RuleEffectKind::Requirement {
            satisfied: key("flag"),
            code: key("requirement-code"),
        },
    ];
    RuleProgram {
        id: key("effects"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![],
        nodes: vec![
            literal("flag", ParameterValue::Boolean(true)),
            literal("amount", quantity(2.0, "unit.damage")),
            literal("factor", quantity(1.5, "unit.factor")),
            literal("count", integer(2)),
        ],
        effects: kinds
            .into_iter()
            .enumerate()
            .map(|(position, effect)| RuleEffect {
                id: key(&format!("effect-{}", 100 - position)),
                when: Some(key("flag")),
                effect,
            })
            .collect(),
    }
}

#[test]
fn every_effect_preserves_order_guards_exact_targets_and_declared_slot_owners() {
    let f = program_tests::enriched_fixture();
    let program = effect_matrix();
    roundtrip(&program, &f, &program_tests::symbols(&f));
    for (position, entity) in [
        RuleEntity::Current,
        RuleEntity::Modifier,
        RuleEntity::Actor,
        RuleEntity::Player,
        RuleEntity::Enemy,
        RuleEntity::Environment,
        RuleEntity::SupportOrigin,
        RuleEntity::Skill,
        RuleEntity::AssignedSkill,
        RuleEntity::EffectSource,
        RuleEntity::PropertyOwner,
    ]
    .into_iter()
    .enumerate()
    {
        let mut changed = program.clone();
        changed.id = key(&format!("relative-{position}"));
        let RuleEffectKind::Contribute { entity: target, .. } = &mut changed.effects[0].effect
        else {
            unreachable!()
        };
        *target = entity;
        roundtrip(&changed, &f, &program_tests::symbols(&f));
    }
    for context in [
        RuleEntityKind::Actor,
        RuleEntityKind::Action,
        RuleEntityKind::EquipmentUse,
        RuleEntityKind::Environment,
        RuleEntityKind::SupportOrigin,
        RuleEntityKind::Skill,
    ] {
        let mut changed = program.clone();
        changed.context = context;
        roundtrip(&changed, &f, &program_tests::symbols(&f));
    }
}

fn expression_matrix() -> RuleProgram {
    let mut nodes = vec![
        literal("amount", quantity(5.0, "unit.damage")),
        literal("other", quantity(2.0, "unit.damage")),
        literal("factor", quantity(1.5, "unit.factor")),
        literal("percent", quantity(25.0, "unit.percent")),
        literal("count", integer(2)),
        literal("flag", ParameterValue::Boolean(true)),
        literal("choice", ParameterValue::Option(id("option.first"))),
        literal("time", quantity(0.5, "unit.time")),
        literal("zero-time", quantity(0.0, "unit.time")),
        literal("tick", quantity(4.0, "unit.rate")),
        node(
            "input",
            RuleExpression::Read {
                input: key("input"),
            },
        ),
        node(
            "input-copy",
            RuleExpression::Read {
                input: key("input"),
            },
        ),
        node(
            "sum",
            RuleExpression::Add {
                left: key("amount"),
                right: key("other"),
            },
        ),
        node(
            "difference",
            RuleExpression::Subtract {
                left: key("amount"),
                right: key("other"),
            },
        ),
        node(
            "minimum",
            RuleExpression::Minimum {
                left: key("amount"),
                right: key("other"),
            },
        ),
        node(
            "maximum",
            RuleExpression::Maximum {
                left: key("amount"),
                right: key("other"),
            },
        ),
        node(
            "scaled",
            RuleExpression::Scale {
                value: key("amount"),
                factor: key("factor"),
            },
        ),
        node(
            "counted",
            RuleExpression::ScaleInteger {
                value: key("amount"),
                count: key("count"),
            },
        ),
        node(
            "divided",
            RuleExpression::DivideFactor {
                value: key("amount"),
                divisor: key("factor"),
            },
        ),
        node(
            "ratio",
            RuleExpression::Ratio {
                numerator: key("amount"),
                denominator: key("other"),
                unit: id("unit.factor"),
            },
        ),
        node(
            "percent-factor",
            RuleExpression::PercentAsFactor {
                percent: key("percent"),
                unit: id("unit.factor"),
            },
        ),
        node("negated", RuleExpression::Not { value: key("flag") }),
        node(
            "all",
            RuleExpression::All {
                values: vec![key("flag"), key("negated"), key("flag")],
            },
        ),
        node(
            "any",
            RuleExpression::Any {
                values: vec![key("negated"), key("flag")],
            },
        ),
        node("all-empty", RuleExpression::All { values: vec![] }),
        node("any-empty", RuleExpression::Any { values: vec![] }),
        node(
            "select",
            RuleExpression::Select {
                condition: key("flag"),
                when_true: key("amount"),
                when_false: key("other"),
            },
        ),
        node(
            "integer-domain",
            RuleExpression::CheckNumericDomain {
                value: key("input"),
                domain: NumericDomain::Integer(IntegerRange {
                    minimum: BoundedInteger::new(-2).unwrap(),
                    maximum: BoundedInteger::new(3).unwrap(),
                }),
            },
        ),
        node(
            "quantity-domain",
            RuleExpression::CheckNumericDomain {
                value: key("amount"),
                domain: NumericDomain::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(-1.0, id("unit.damage")).unwrap(),
                    maximum: FiniteQuantity::new(5.0, id("unit.damage")).unwrap(),
                }),
            },
        ),
        node(
            "lookup",
            RuleExpression::LookupIntegerTable {
                table: key("signed-table"),
                key: key("input"),
            },
        ),
    ];
    for (position, mode) in [
        RuleRounding::Floor,
        RuleRounding::Ceiling,
        RuleRounding::Truncate,
        RuleRounding::NearestTiesPositive,
    ]
    .into_iter()
    .enumerate()
    {
        nodes.push(node(
            &format!("round-{position}"),
            RuleExpression::Round {
                value: key("amount"),
                quantum: FiniteQuantity::new(0.5, id("unit.damage")).unwrap(),
                mode,
            },
        ));
        nodes.push(node(
            &format!("quantize-{position}"),
            RuleExpression::QuantizeInteger {
                value: key("amount"),
                quantum: FiniteQuantity::new(0.5, id("unit.damage")).unwrap(),
                mode,
            },
        ));
    }
    for (position, operation) in [
        RuleComparison::Equal,
        RuleComparison::Less,
        RuleComparison::LessOrEqual,
        RuleComparison::Greater,
        RuleComparison::GreaterOrEqual,
    ]
    .into_iter()
    .enumerate()
    {
        nodes.push(node(
            &format!("compare-{position}"),
            RuleExpression::Compare {
                operation,
                left: key("amount"),
                right: key("other"),
            },
        ));
    }
    nodes.push(node(
        "option-equal",
        RuleExpression::Compare {
            operation: RuleComparison::Equal,
            left: key("choice"),
            right: key("choice"),
        },
    ));
    for (position, output) in [
        OrdinaryTimingChannel::SpeedMultiplier,
        OrdinaryTimingChannel::PreCapRate,
        OrdinaryTimingChannel::ActionRate,
        OrdinaryTimingChannel::ActionTime,
    ]
    .into_iter()
    .enumerate()
    {
        nodes.push(node(
            &format!("timing-{position}"),
            RuleExpression::OrdinaryTiming {
                recipe: Box::new(OrdinaryTimingRecipe {
                    base_time: key("time"),
                    increased_percent: key("percent"),
                    more_multiplier: key("factor"),
                    additional_attack_time: key("zero-time"),
                    additional_cast_time: key("zero-time"),
                    action_speed_multiplier: key("factor"),
                    repeats: key("count"),
                    server_tick_rate: key("tick"),
                    speed_multiplier_rounding_precision: 3,
                    units: ReciprocalTimingUnits {
                        time: id("unit.time"),
                        rate: id("unit.rate"),
                    },
                    output,
                }),
            },
        ));
    }
    RuleProgram {
        id: key("local-item"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![RuleRead {
            id: key("input"),
            value_type: ComputedValueType::Integer,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: id("stat.integer"),
            },
        }],
        nodes,
        effects: vec![RuleEffect {
            id: key("result"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id("stat.result"),
                value: key("select"),
            },
        }],
    }
}

#[test]
fn every_expression_variant_preserves_sharing_duplicate_edges_and_unused_nodes() {
    let mut f = operations::fixture(ComputedValueType::Quantity {
        unit: id("unit.damage"),
    });
    f.rules.tables.push(operations::table(
        ComputedValueType::Integer,
        vec![integer(11), integer(47), integer(-3)],
    ));
    let original = expression_matrix();
    let (_, lowered) = roundtrip(&original, &f, &operations::symbols(&f));
    for program in [original, lowered.program] {
        *f.rules.owners[0]
            .programs
            .members
            .iter_mut()
            .find(|program| program.id == key("local-item"))
            .unwrap() = program;
        // No input is supplied: every unused read/table/domain/timing node is
        // preserved and checked, but the independent selected effect stays lazy.
        assert_eq!(
            native::execute(&f, &[]),
            EffectDisposition::Applied {
                value: quantity(5.0, "unit.damage")
            }
        );
    }
}

#[test]
fn timing_roundtrip_retains_actual_engine_channels_and_missing_dependencies() {
    for (channel, unit, expected) in [
        ("speed_multiplier", "unit.factor", 1.0),
        ("pre_cap_rate", "unit.rate", 2.0),
        ("action_rate", "unit.rate", 2.0),
        ("action_time", "unit.time", 0.5),
    ] {
        let (mut f, _) = operations::timing_fixture(channel, unit, 2);
        let original = f.rules.owners[0]
            .programs
            .members
            .iter()
            .find(|program| program.id == key("local-item"))
            .unwrap()
            .clone();
        let (_, lowered) = roundtrip(&original, &f, &operations::symbols(&f));
        *f.rules.owners[0]
            .programs
            .members
            .iter_mut()
            .find(|program| program.id == key("local-item"))
            .unwrap() = lowered.program;
        assert_eq!(
            native::execute(&f, &operations::timing_facts()),
            EffectDisposition::Applied {
                value: quantity(expected, unit)
            }
        );
        for missing in ["base", "tick"] {
            let mut facts = operations::timing_facts();
            facts.retain(|(read, _)| *read != key(missing));
            assert_eq!(
                native::execute(&f, &facts),
                EffectDisposition::Unresolved {
                    input: key(missing)
                }
            );
        }
    }
}

#[test]
fn demanded_domain_table_and_option_results_retain_lazy_and_failure_semantics() {
    let mut f = operations::fixture(ComputedValueType::Option);
    let rows = vec![
        ParameterValue::Option(id("option.first")),
        ParameterValue::Option(id("option.second")),
        ParameterValue::Option(id("option.first")),
    ];
    f.rules
        .tables
        .push(operations::table(ComputedValueType::Option, rows.clone()));
    let domain = NumericDomain::Integer(IntegerRange {
        minimum: BoundedInteger::new(-3).unwrap(),
        maximum: BoundedInteger::new(1).unwrap(),
    });
    let original = RuleProgram {
        id: key("local-item"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![
            RuleRead {
                id: key("input"),
                value_type: ComputedValueType::Integer,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: id("stat.integer"),
                },
            },
            RuleRead {
                id: key("gate"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: id("stat.boolean"),
                },
            },
        ],
        nodes: vec![
            node(
                "input",
                RuleExpression::Read {
                    input: key("input"),
                },
            ),
            node("gate", RuleExpression::Read { input: key("gate") }),
            node(
                "checked",
                RuleExpression::CheckNumericDomain {
                    value: key("input"),
                    domain: domain.clone(),
                },
            ),
            node(
                "looked-up",
                RuleExpression::LookupIntegerTable {
                    table: key("signed-table"),
                    key: key("checked"),
                },
            ),
            literal("default", ParameterValue::Option(id("option.second"))),
            node(
                "selected",
                RuleExpression::Select {
                    condition: key("gate"),
                    when_true: key("looked-up"),
                    when_false: key("default"),
                },
            ),
        ],
        effects: vec![RuleEffect {
            id: key("result"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id("stat.result"),
                value: key("selected"),
            },
        }],
    };
    let (_, lowered) = roundtrip(&original, &f, &operations::symbols(&f));
    for program in [original, lowered.program] {
        let checked = program
            .nodes
            .iter()
            .find(|node| matches!(node.expression, RuleExpression::CheckNumericDomain { .. }))
            .unwrap()
            .id
            .clone();
        let lookup = program
            .nodes
            .iter()
            .find(|node| matches!(node.expression, RuleExpression::LookupIntegerTable { .. }))
            .unwrap()
            .id
            .clone();
        *f.rules.owners[0]
            .programs
            .members
            .iter_mut()
            .find(|program| program.id == key("local-item"))
            .unwrap() = program;
        assert_eq!(
            native::execute(&f, &[(key("gate"), ParameterValue::Boolean(false))]),
            EffectDisposition::Applied {
                value: ParameterValue::Option(id("option.second"))
            }
        );
        assert_eq!(
            native::execute(&f, &[(key("gate"), ParameterValue::Boolean(true))]),
            EffectDisposition::Unresolved {
                input: key("input")
            }
        );
        for (input, expected) in [-2, -1, 0].into_iter().zip(&rows) {
            assert_eq!(
                native::execute(
                    &f,
                    &[
                        (key("gate"), ParameterValue::Boolean(true)),
                        (key("input"), integer(input))
                    ]
                ),
                EffectDisposition::Applied {
                    value: expected.clone()
                }
            );
        }
        for input in [-3, 1] {
            assert_eq!(
                native::execute(
                    &f,
                    &[
                        (key("gate"), ParameterValue::Boolean(true)),
                        (key("input"), integer(input))
                    ]
                ),
                EffectDisposition::UnsupportedDomain {
                    node: lookup.clone(),
                    domain: UnsupportedDomainCause::IntegerTable {
                        table: key("signed-table"),
                        key: BoundedInteger::new(input).unwrap(),
                        minimum: BoundedInteger::new(-2).unwrap(),
                        maximum: BoundedInteger::new(0).unwrap()
                    },
                }
            );
        }
        assert_eq!(
            native::execute(
                &f,
                &[
                    (key("gate"), ParameterValue::Boolean(true)),
                    (key("input"), integer(2))
                ]
            ),
            EffectDisposition::UnsupportedDomain {
                node: checked,
                domain: UnsupportedDomainCause::NumericRange {
                    value: integer(2),
                    domain: Box::new(domain.clone())
                },
            }
        );
    }
}

#[test]
fn finite_literal_spelling_roundtrips_extremes_subnormals_and_bounded_integers_exactly() {
    let f = operations::fixture(ComputedValueType::Quantity {
        unit: id("unit.damage"),
    });
    let values = [
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
        1.0e100,
        1.0e-100,
        0.1,
        0.0,
        -0.0,
    ];
    let mut nodes: Vec<_> = values
        .into_iter()
        .enumerate()
        .map(|(position, value)| {
            literal(
                &format!("quantity-{position}"),
                quantity(value, "unit.damage"),
            )
        })
        .collect();
    for (position, value) in [-9_007_199_254_740_991, -1, 0, 1, 9_007_199_254_740_991]
        .into_iter()
        .enumerate()
    {
        nodes.push(literal(&format!("integer-{position}"), integer(value)));
    }
    let program = RuleProgram {
        id: key("literals"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![],
        nodes,
        effects: vec![],
    };
    let (raised, lowered) = roundtrip(&program, &f, &operations::symbols(&f));
    let bindings: BTreeMap<_, _> = lowered
        .bindings
        .iter()
        .map(|binding| (binding.name.as_str(), &binding.value))
        .collect();
    for (position, value) in values.into_iter().enumerate() {
        let name = &raised
            .nodes
            .iter()
            .find(|entry| entry.original == key(&format!("quantity-{position}")))
            .unwrap()
            .name;
        let actual = lowered
            .program
            .nodes
            .iter()
            .find(|node| &node.id == bindings[name.as_str()])
            .unwrap();
        let RuleExpression::Literal {
            value: ParameterValue::Quantity(actual),
        } = &actual.expression
        else {
            panic!("quantity literal retained")
        };
        let expected = FiniteQuantity::new(value, id("unit.damage")).unwrap();
        assert_eq!(actual.value().to_bits(), expected.value().to_bits());
        assert_eq!(actual.unit(), expected.unit());
    }
}

fn small_graph() -> RuleProgram {
    RuleProgram {
        id: key("limits"),
        context: RuleEntityKind::EquipmentUse,
        reads: ["left", "right"]
            .map(|name| RuleRead {
                id: key(name),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::HasItemQuality {
                    quality: id("quality.local"),
                },
            })
            .into(),
        nodes: vec![
            node("left", RuleExpression::Read { input: key("left") }),
            node(
                "right",
                RuleExpression::Read {
                    input: key("right"),
                },
            ),
            node("not", RuleExpression::Not { value: key("left") }),
            node(
                "all",
                RuleExpression::All {
                    values: vec![key("not"), key("right"), key("not")],
                },
            ),
        ],
        effects: [("left", "left", "not"), ("right", "right", "all")]
            .map(|(name, when, value)| RuleEffect {
                id: key(name),
                when: Some(key(when)),
                effect: RuleEffectKind::Requirement {
                    satisfied: key(value),
                    code: key("same-static-code"),
                },
            })
            .into(),
    }
}

#[test]
fn deterministic_source_ignores_read_node_storage_but_preserves_effect_order() {
    let f = native::fixture::fixture();
    let symbols = program_tests::symbols(&f);
    let original = small_graph();
    let baseline = roundtrip(&original, &f, &symbols).0;
    for shift in 0..original.nodes.len() {
        let mut permuted = original.clone();
        permuted.nodes.rotate_left(shift);
        permuted.nodes.reverse();
        permuted.reads.reverse();
        let actual = roundtrip(&permuted, &f, &symbols).0;
        assert_eq!(actual.source, baseline.source);
        assert_eq!(actual.nodes, baseline.nodes);
    }
    let mut swapped = original;
    swapped.effects.reverse();
    let actual = roundtrip(&swapped, &f, &symbols).0;
    assert_ne!(actual.source, baseline.source);
}

#[test]
fn duplicate_ids_dangling_references_and_unused_cycles_are_refused() {
    let f = native::fixture::fixture();
    let symbols = program_tests::symbols(&f);
    let original = small_graph();
    // Reusing a spelling across read/node/effect domains is valid; duplicates
    // within each domain are independently rejected.
    roundtrip(&original, &f, &symbols);
    for mutation in 0..9 {
        let mut bad = original.clone();
        match mutation {
            0 => bad.reads.push(bad.reads[0].clone()),
            1 => bad.nodes.push(bad.nodes[0].clone()),
            2 => bad.effects.push(bad.effects[0].clone()),
            3 => {
                bad.nodes[0].expression = RuleExpression::Read {
                    input: key("missing-read"),
                }
            }
            4 => {
                bad.nodes[2].expression = RuleExpression::Not {
                    value: key("missing-node"),
                }
            }
            5 => bad.effects[0].when = Some(key("missing-guard")),
            6 => {
                bad.effects[0].effect = RuleEffectKind::Requirement {
                    satisfied: key("missing-value"),
                    code: key("same-static-code"),
                }
            }
            7 => bad.nodes.extend([
                node(
                    "unused-a",
                    RuleExpression::Not {
                        value: key("unused-b"),
                    },
                ),
                node(
                    "unused-b",
                    RuleExpression::Not {
                        value: key("unused-a"),
                    },
                ),
            ]),
            8 => bad.nodes.push(node(
                "unused-self",
                RuleExpression::Not {
                    value: key("unused-self"),
                },
            )),
            _ => unreachable!(),
        }
        let (kind, location) = match mutation {
            0 => (
                RaiseErrorKind::DuplicateRead,
                RaiseLocation::Read(original.reads.len()),
            ),
            1 => (
                RaiseErrorKind::DuplicateNode,
                RaiseLocation::Node(original.nodes.len()),
            ),
            2 => (
                RaiseErrorKind::DuplicateEffect,
                RaiseLocation::Effect(original.effects.len()),
            ),
            3 => (RaiseErrorKind::MissingRead, RaiseLocation::Node(0)),
            4 => (RaiseErrorKind::MissingNode, RaiseLocation::Node(2)),
            5 | 6 => (RaiseErrorKind::MissingNode, RaiseLocation::Effect(0)),
            7 | 8 => (
                RaiseErrorKind::Cycle,
                RaiseLocation::Node(original.nodes.len()),
            ),
            _ => unreachable!(),
        };
        assert_eq!(
            raise_program(&bad, &symbols, RaiseLimits::default()).unwrap_err(),
            RaiseError { kind, location },
            "mutation {mutation}"
        );
    }
    let mut bad = original;
    bad.context = RuleEntityKind::Modifier;
    assert_eq!(
        raise_program(&bad, &symbols, RaiseLimits::default()).unwrap_err(),
        RaiseError {
            kind: RaiseErrorKind::InvalidContext,
            location: RaiseLocation::Program
        }
    );
}

#[test]
fn exact_raise_budgets_cover_all_lists_duplicate_edges_symbols_and_output_bytes() {
    let f = native::fixture::fixture();
    let symbols = program_tests::symbols(&f);
    let program = small_graph();
    let baseline = raise_program(&program, &symbols, RaiseLimits::default()).unwrap();
    let exact = RaiseLimits {
        max_reads: 2,
        max_nodes: 4,
        max_effects: 2,
        // Two Read.input + Not + three All operands + two guard/value pairs.
        max_references: 10,
        max_symbols: symbols.definitions().count() + symbols.slots().count(),
        max_source_bytes: baseline.source.len(),
    };
    assert_eq!(
        raise_program(&program, &symbols, exact).unwrap().source,
        baseline.source
    );
    for field in 0..6 {
        for zero in [false, true] {
            let mut limits = exact;
            let value = match field {
                0 => &mut limits.max_reads,
                1 => &mut limits.max_nodes,
                2 => &mut limits.max_effects,
                3 => &mut limits.max_references,
                4 => &mut limits.max_symbols,
                5 => &mut limits.max_source_bytes,
                _ => unreachable!(),
            };
            *value = if zero { 0 } else { *value - 1 };
            let (kind, location) = if zero {
                (RaiseErrorKind::InvalidLimits, RaiseLocation::Program)
            } else {
                let (kind, location) = match field {
                    0 => (RaiseLimit::Reads, RaiseLocation::Program),
                    1 => (RaiseLimit::Nodes, RaiseLocation::Program),
                    2 => (RaiseLimit::Effects, RaiseLocation::Program),
                    3 => (RaiseLimit::References, RaiseLocation::Effect(1)),
                    4 => (
                        RaiseLimit::Symbols,
                        RaiseLocation::Symbol(exact.max_symbols - 1),
                    ),
                    5 => (RaiseLimit::SourceBytes, RaiseLocation::Program),
                    _ => unreachable!(),
                };
                (RaiseErrorKind::LimitExceeded(kind), location)
            };
            assert_eq!(
                raise_program(&program, &symbols, limits).unwrap_err(),
                RaiseError { kind, location },
                "budget {field}, zero={zero}"
            );
        }
    }
}

#[test]
fn missing_symbols_refuse_instead_of_inventing_definition_or_slot_names() {
    let f = program_tests::enriched_fixture();
    let complete = program_tests::symbols(&f);
    let definitions: Vec<_> = complete
        .definitions()
        .map(|(name, address)| (name, address.clone()))
        .collect();
    let slots: Vec<_> = complete
        .slots()
        .map(|(name, address)| (name, address.clone()))
        .collect();
    let program = effect_matrix();
    for omit_slot in [false, true] {
        let mut definitions = definitions.clone();
        let mut slots = slots.clone();
        if omit_slot {
            slots.retain(|(_, address)| address.key().as_str() != "supply.skill");
        } else {
            definitions.retain(|(_, address)| address.key().as_str() != "unit.damage");
        }
        let registry =
            SymbolRegistry::new(&f.schema, &definitions, &slots, SymbolLimits::default()).unwrap();
        let (kind, location) = if omit_slot {
            (
                RaiseErrorKind::MissingSlotSymbol,
                RaiseLocation::Effect(
                    program
                        .effects
                        .iter()
                        .position(|effect| {
                            matches!(effect.effect, RuleEffectKind::ProjectSkillParameter { .. })
                        })
                        .unwrap(),
                ),
            )
        } else {
            (
                RaiseErrorKind::MissingDefinitionSymbol,
                RaiseLocation::Node(
                    program
                        .nodes
                        .iter()
                        .position(|node| node.id == key("amount"))
                        .unwrap(),
                ),
            )
        };
        assert_eq!(
            raise_program(&program, &registry, RaiseLimits::default()).unwrap_err(),
            RaiseError { kind, location }
        );
    }
}

#[test]
fn long_linear_graph_raises_lowers_compares_and_drops_on_one_mib_stack() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let f = native::fixture::fixture();
            let mut program = RuleProgram {
                id: key("linear"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![literal("node-0", ParameterValue::Boolean(true))],
                effects: vec![],
            };
            for position in 1..2048 {
                program.nodes.push(node(
                    &format!("node-{position}"),
                    RuleExpression::Not {
                        value: key(&format!("node-{}", position - 1)),
                    },
                ));
            }
            program.effects.push(RuleEffect {
                id: key("last"),
                when: None,
                effect: RuleEffectKind::Requirement {
                    satisfied: program.nodes.last().unwrap().id.clone(),
                    code: key("linear-result"),
                },
            });
            // Reverse storage deliberately; topological traversal must remain iterative.
            program.nodes.reverse();
            roundtrip(&program, &f, &program_tests::symbols(&f));
        })
        .unwrap()
        .join()
        .unwrap();
}
