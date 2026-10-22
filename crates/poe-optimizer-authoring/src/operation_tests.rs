//! The remaining expression operations lower into existing Core operations and
//! execute through Engine. These finite component fixtures grant no game coverage.
use crate::{lowering_tests as native, *};
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, NumericalFailure, RuleLimits, UnsupportedDomainCause,
};

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

fn quantity_type(unit: &str) -> ComputedValueType {
    ComputedValueType::Quantity { unit: id(unit) }
}

fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}

pub(super) fn fixture(result_type: ComputedValueType) -> native::fixture::Fixture {
    let mut f = native::fixture::fixture();
    let mut schema = f.schema.input().clone();
    for (name, dimension) in [
        ("unit.time", UnitDimension::Time),
        ("unit.rate", UnitDimension::Rate),
        ("unit.other-time", UnitDimension::Time),
        ("unit.other-factor", UnitDimension::DimensionlessFactor),
    ] {
        schema.definitions.push(DefinitionDescriptor::Unit(known(
            id(name),
            UnitSchema { dimension },
        )));
    }
    for name in ["option.first", "option.second"] {
        schema.definitions.push(DefinitionDescriptor::Option(known(
            id(name),
            OptionSchema {},
        )));
    }
    for (name, value) in [
        ("stat.result", result_type),
        ("stat.integer", ComputedValueType::Integer),
        ("stat.number", quantity_type("unit.damage")),
        ("stat.boolean", ComputedValueType::Boolean),
        ("stat.option", ComputedValueType::Option),
        ("stat.base", quantity_type("unit.time")),
        ("stat.increased", quantity_type("unit.percent")),
        ("stat.more", quantity_type("unit.factor")),
        ("stat.attack", quantity_type("unit.time")),
        ("stat.cast", quantity_type("unit.time")),
        ("stat.action", quantity_type("unit.factor")),
        ("stat.repeats", ComputedValueType::Integer),
        ("stat.tick", quantity_type("unit.rate")),
    ] {
        schema.definitions.push(DefinitionDescriptor::Stat(known(
            id(name),
            StatSchema {
                value,
                targets: vec![RuleEntityKind::EquipmentUse],
            },
        )));
    }
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    f.rules.definitions = f.schema.identity().clone();
    f.rules.operations_version = key(OWNED_RULE_OPERATIONS_V27);
    f.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    f.rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
    f
}

pub(super) fn symbols(f: &native::fixture::Fixture) -> SymbolRegistry {
    let owned: Vec<_> = f
        .schema
        .input()
        .definitions
        .iter()
        .map(|entry| {
            let address = entry.address();
            (address.key().as_str().replace('-', "_"), address)
        })
        .collect();
    let names: Vec<_> = owned
        .iter()
        .map(|(name, address)| (name.as_str(), address.clone()))
        .collect();
    SymbolRegistry::new(&f.schema, &names, &[], SymbolLimits::default()).unwrap()
}

fn program(reads: &str, expression: &str) -> String {
    format!(
        "program \"local-item\" context equipment_use {{ {reads}
         let result = {expression};
         effect \"result\" = derive(current, stat.result, result); }}"
    )
}

fn lower(source: &str, f: &native::fixture::Fixture) -> LoweredProgram {
    lower_program(
        &parse_program(source, ProgramParseLimits::default()).unwrap(),
        &f.schema,
        &symbols(f),
        &f.rules.tables,
        ProgramLowerLimits::default(),
    )
    .unwrap()
}

fn install(f: &mut native::fixture::Fixture, lowered: &LoweredProgram) {
    *f.rules.owners[0]
        .programs
        .members
        .iter_mut()
        .find(|program| program.id == key("local-item"))
        .unwrap() = lowered.program.clone();
}

fn applied(value: ParameterValue) -> EffectDisposition {
    EffectDisposition::Applied { value }
}

fn rejected(source: &str, f: &native::fixture::Fixture) -> Diagnostic {
    let error = match parse_program(source, ProgramParseLimits::default()) {
        Ok(parsed) => lower_program(
            &parsed,
            &f.schema,
            &symbols(f),
            &f.rules.tables,
            ProgramLowerLimits::default(),
        )
        .unwrap_err(),
        Err(error) => error,
    };
    assert!(error.span.text(source).is_some(), "{source}: {error}");
    error
}

const INTEGER_READ: &str = "read \"input\" as input: integer = stat(current, stat.integer);";
const NUMBER_READ: &str =
    "read \"input\" as input: quantity(unit.damage) = stat(current, stat.number);";
const FLAG_READ: &str = "read \"flag\" as flag: boolean = stat(current, stat.boolean);";

#[test]
fn integer_domain_preserves_exact_input_and_reports_inclusive_boundary_failures() {
    let mut f = fixture(ComputedValueType::Integer);
    let source = program(INTEGER_READ, "check_domain(input, -2, 3)");
    let lowered = lower(&source, &f);
    let domain = NumericDomain::Integer(IntegerRange {
        minimum: BoundedInteger::new(-2).unwrap(),
        maximum: BoundedInteger::new(3).unwrap(),
    });
    let [read, checked] = lowered.program.nodes.as_slice() else {
        panic!("one read and one domain operation; static endpoints emit no nodes")
    };
    assert_eq!(
        read.expression,
        RuleExpression::Read {
            input: key("input")
        }
    );
    assert_eq!(
        checked.expression,
        RuleExpression::CheckNumericDomain {
            value: read.id.clone(),
            domain: domain.clone(),
        }
    );
    install(&mut f, &lowered);
    for value in [-2, -1, 0, 2, 3] {
        assert_eq!(
            native::execute(&f, &[(key("input"), integer(value))]),
            applied(integer(value))
        );
    }
    for value in [-3, 4] {
        assert_eq!(
            native::execute(&f, &[(key("input"), integer(value))]),
            EffectDisposition::UnsupportedDomain {
                node: checked.id.clone(),
                domain: UnsupportedDomainCause::NumericRange {
                    value: integer(value),
                    domain: Box::new(domain.clone()),
                },
            }
        );
    }
    assert_eq!(
        native::execute(&f, &[]),
        EffectDisposition::Unresolved {
            input: key("input")
        }
    );
}

#[test]
fn quantity_domain_checks_adjacent_floats_and_remains_lazy() {
    let mut f = fixture(quantity_type("unit.damage"));
    let source = program(
        &format!("{NUMBER_READ} {FLAG_READ}"),
        "if flag { check_domain(input, quantity(1, unit.damage), quantity(2, unit.damage)) } else { quantity(9, unit.damage) }",
    );
    let lowered = lower(&source, &f);
    let checked = lowered
        .program
        .nodes
        .iter()
        .find(|node| matches!(node.expression, RuleExpression::CheckNumericDomain { .. }))
        .unwrap();
    let RuleExpression::CheckNumericDomain { domain, .. } = &checked.expression else {
        unreachable!()
    };
    install(&mut f, &lowered);
    for value in [
        1.0,
        f64::from_bits(1.0_f64.to_bits() + 1),
        f64::from_bits(2.0_f64.to_bits() - 1),
        2.0,
    ] {
        let input = quantity(value, "unit.damage");
        assert_eq!(
            native::execute(
                &f,
                &[
                    (key("flag"), ParameterValue::Boolean(true)),
                    (key("input"), input.clone())
                ]
            ),
            applied(input)
        );
    }
    for value in [
        f64::from_bits(1.0_f64.to_bits() - 1),
        f64::from_bits(2.0_f64.to_bits() + 1),
    ] {
        let input = quantity(value, "unit.damage");
        assert_eq!(
            native::execute(
                &f,
                &[
                    (key("flag"), ParameterValue::Boolean(true)),
                    (key("input"), input.clone())
                ]
            ),
            EffectDisposition::UnsupportedDomain {
                node: checked.id.clone(),
                domain: UnsupportedDomainCause::NumericRange {
                    value: input,
                    domain: Box::new(domain.clone())
                },
            }
        );
    }
    assert_eq!(
        native::execute(&f, &[(key("flag"), ParameterValue::Boolean(true))]),
        EffectDisposition::Unresolved {
            input: key("input")
        }
    );
    for facts in [
        vec![(key("flag"), ParameterValue::Boolean(false))],
        vec![
            (key("flag"), ParameterValue::Boolean(false)),
            (key("input"), quantity(100.0, "unit.damage")),
        ],
    ] {
        assert_eq!(
            native::execute(&f, &facts),
            applied(quantity(9.0, "unit.damage"))
        );
    }
}

pub(super) fn table(value_type: ComputedValueType, rows: Vec<ParameterValue>) -> IntegerRuleTable {
    IntegerRuleTable {
        id: key("signed-table"),
        minimum: BoundedInteger::new(-2).unwrap(),
        maximum: BoundedInteger::new(0).unwrap(),
        value_type,
        rows,
    }
}

#[test]
fn signed_dense_tables_preserve_all_scalar_types_and_exact_domain() {
    for (ty, rows) in [
        (
            ComputedValueType::Integer,
            vec![integer(11), integer(47), integer(-3)],
        ),
        (
            ComputedValueType::Boolean,
            vec![
                ParameterValue::Boolean(true),
                ParameterValue::Boolean(false),
                ParameterValue::Boolean(true),
            ],
        ),
        (
            ComputedValueType::Option,
            vec![
                ParameterValue::Option(id("option.second")),
                ParameterValue::Option(id("option.first")),
                ParameterValue::Option(id("option.second")),
            ],
        ),
        (
            quantity_type("unit.damage"),
            vec![
                quantity(0.25, "unit.damage"),
                quantity(8.5, "unit.damage"),
                quantity(-4.0, "unit.damage"),
            ],
        ),
    ] {
        let mut f = fixture(ty.clone());
        f.rules.tables.push(table(ty, rows.clone()));
        let lowered = lower(
            &program(INTEGER_READ, "lookup(input, \"signed-table\")"),
            &f,
        );
        let [read, lookup] = lowered.program.nodes.as_slice() else {
            panic!("lookup has one key dependency; table metadata emits no nodes")
        };
        assert_eq!(
            lookup.expression,
            RuleExpression::LookupIntegerTable {
                table: key("signed-table"),
                key: read.id.clone()
            }
        );
        install(&mut f, &lowered);
        for (value, expected) in [-2, -1, 0].into_iter().zip(rows) {
            assert_eq!(
                native::execute(&f, &[(key("input"), integer(value))]),
                applied(expected)
            );
        }
        for value in [-3, 1] {
            assert_eq!(
                native::execute(&f, &[(key("input"), integer(value))]),
                EffectDisposition::UnsupportedDomain {
                    node: lookup.id.clone(),
                    domain: UnsupportedDomainCause::IntegerTable {
                        table: key("signed-table"),
                        key: BoundedInteger::new(value).unwrap(),
                        minimum: BoundedInteger::new(-2).unwrap(),
                        maximum: BoundedInteger::new(0).unwrap()
                    },
                }
            );
        }
        assert_eq!(
            native::execute(&f, &[]),
            EffectDisposition::Unresolved {
                input: key("input")
            }
        );
        assert_eq!(
            rejected(
                &program("", "lookup(quantity(-1.5, unit.damage), \"signed-table\")"),
                &f
            )
            .code,
            DiagnosticCode::TypeMismatch
        );
    }
}

#[test]
fn table_signatures_do_not_bypass_engine_row_domain_or_runtime_authority() {
    let original = table(
        ComputedValueType::Integer,
        vec![integer(11), integer(47), integer(-3)],
    );
    for mutation in 0..5 {
        let mut f = fixture(ComputedValueType::Integer);
        let mut bad = original.clone();
        match mutation {
            0 => {
                bad.rows.pop();
            }
            1 => bad.maximum = BoundedInteger::new(-3).unwrap(),
            2 => bad.rows[1] = ParameterValue::Boolean(true),
            3 => {
                bad.rows.push(integer(9));
            }
            4 => {
                bad.minimum = BoundedInteger::new(-4).unwrap();
            }
            _ => unreachable!(),
        }
        f.rules.tables.push(bad);
        // Authoring binds only the typed signature; it must not certify rows.
        let lowered = lower(
            &program(INTEGER_READ, "lookup(input, \"signed-table\")"),
            &f,
        );
        install(&mut f, &lowered);
        assert!(
            CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).is_err(),
            "malformed table {mutation}"
        );
    }
    let mut f = fixture(ComputedValueType::Integer);
    f.rules.tables.push(original);
    let lowered = lower(
        &program(INTEGER_READ, "lookup(input, \"signed-table\")"),
        &f,
    );
    install(&mut f, &lowered);
    assert!(CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).is_ok());
    let tables = std::mem::take(&mut f.rules.tables);
    assert!(CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).is_err());
    f.rules.tables = tables;
    f.rules.tables[0].value_type = ComputedValueType::Boolean;
    f.rules.tables[0].rows = vec![ParameterValue::Boolean(true); 3];
    // A well-formed replacement table cannot launder a changed derive type.
    assert!(CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).is_err());

    let mut f = fixture(ComputedValueType::Integer);
    f.rules.tables.push(table(
        ComputedValueType::Integer,
        vec![integer(11), integer(47), integer(-3)],
    ));
    let source = program(
        "read \"input\" as input: integer = stat(current, stat.number);",
        "lookup(input, \"signed-table\")",
    );
    let lowered = lower(&source, &f);
    install(&mut f, &lowered);
    let error =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("declared read type does not match source schema")
    );
}

#[test]
fn option_identity_equality_and_lazy_table_selection_survive_lowering() {
    let mut f = fixture(ComputedValueType::Option);
    f.rules.tables.push(table(
        ComputedValueType::Option,
        vec![ParameterValue::Option(id("option.second")); 3],
    ));
    let reads = format!("{INTEGER_READ} {FLAG_READ}");
    let lowered = lower(
        &program(
            &reads,
            "if flag { option(option.first) } else { lookup(input, \"signed-table\") }",
        ),
        &f,
    );
    assert!(lowered.program.nodes.iter().any(|node| node.expression
        == RuleExpression::Literal {
            value: ParameterValue::Option(id("option.first"))
        }));
    install(&mut f, &lowered);
    assert_eq!(
        native::execute(&f, &[(key("flag"), ParameterValue::Boolean(true))]),
        applied(ParameterValue::Option(id("option.first")))
    );
    assert_eq!(
        native::execute(&f, &[(key("flag"), ParameterValue::Boolean(false))]),
        EffectDisposition::Unresolved {
            input: key("input")
        }
    );
    assert_eq!(
        native::execute(
            &f,
            &[
                (key("flag"), ParameterValue::Boolean(false)),
                (key("input"), integer(-1))
            ]
        ),
        applied(ParameterValue::Option(id("option.second")))
    );
    assert!(matches!(
        native::execute(
            &f,
            &[
                (key("flag"), ParameterValue::Boolean(false)),
                (key("input"), integer(1))
            ]
        ),
        EffectDisposition::UnsupportedDomain {
            domain: UnsupportedDomainCause::IntegerTable { .. },
            ..
        }
    ));

    let mut f = fixture(ComputedValueType::Boolean);
    let lowered = lower(
        &program(
            "read \"choice\" as choice: option = stat(current, stat.option);",
            "choice == option(option.first)",
        ),
        &f,
    );
    install(&mut f, &lowered);
    for (name, expected) in [("option.first", true), ("option.second", false)] {
        assert_eq!(
            native::execute(&f, &[(key("choice"), ParameterValue::Option(id(name)))]),
            applied(ParameterValue::Boolean(expected))
        );
    }
    assert_eq!(
        native::execute(&f, &[]),
        EffectDisposition::Unresolved {
            input: key("choice")
        }
    );
}

const TIMING_READS: &str = r#"
    read "base" as base: quantity(unit.time) = stat(current, stat.base);
    read "increased" as increased: quantity(unit.percent) = stat(current, stat.increased);
    read "more" as more: quantity(unit.factor) = stat(current, stat.more);
    read "attack" as attack: quantity(unit.time) = stat(current, stat.attack);
    read "cast" as cast: quantity(unit.time) = stat(current, stat.cast);
    read "action" as action: quantity(unit.factor) = stat(current, stat.action);
    read "repeats" as repeats: integer = stat(current, stat.repeats);
    read "tick" as tick: quantity(unit.rate) = stat(current, stat.tick);
"#;

fn timing_expression(precision: u32, channel: &str) -> String {
    format!(
        "ordinary_timing(base, increased, more, attack, cast, action, repeats, tick, {precision}, unit.time, unit.rate, {channel})"
    )
}

pub(super) fn timing_facts() -> Vec<(OwnedDefinitionKey, ParameterValue)> {
    vec![
        (key("base"), quantity(0.5, "unit.time")),
        (key("increased"), quantity(0.0, "unit.percent")),
        (key("more"), quantity(1.0, "unit.factor")),
        (key("attack"), quantity(0.0, "unit.time")),
        (key("cast"), quantity(0.0, "unit.time")),
        (key("action"), quantity(1.0, "unit.factor")),
        (key("repeats"), integer(1)),
        (key("tick"), quantity(4.0, "unit.rate")),
    ]
}

fn set(facts: &mut [(OwnedDefinitionKey, ParameterValue)], name: &str, value: ParameterValue) {
    facts
        .iter_mut()
        .find(|(read, _)| *read == key(name))
        .unwrap()
        .1 = value;
}

pub(super) fn timing_fixture(
    channel: &str,
    unit: &str,
    precision: u32,
) -> (native::fixture::Fixture, LoweredProgram) {
    let mut f = fixture(quantity_type(unit));
    let lowered = lower(
        &program(TIMING_READS, &timing_expression(precision, channel)),
        &f,
    );
    install(&mut f, &lowered);
    (f, lowered)
}

#[test]
fn ordinary_timing_is_one_atomic_operation_with_all_eight_ordered_dependencies() {
    for (channel, unit, output, expected) in [
        (
            "speed_multiplier",
            "unit.factor",
            OrdinaryTimingChannel::SpeedMultiplier,
            1.0,
        ),
        (
            "pre_cap_rate",
            "unit.rate",
            OrdinaryTimingChannel::PreCapRate,
            2.0,
        ),
        (
            "action_rate",
            "unit.rate",
            OrdinaryTimingChannel::ActionRate,
            2.0,
        ),
        (
            "action_time",
            "unit.time",
            OrdinaryTimingChannel::ActionTime,
            0.5,
        ),
    ] {
        let (f, lowered) = timing_fixture(channel, unit, 2);
        let [
            base,
            increased,
            more,
            attack,
            cast,
            action,
            repeats,
            tick,
            timing,
        ] = lowered.program.nodes.as_slice()
        else {
            panic!("eight dynamic reads and one atomic timing operation")
        };
        for (node, name) in [base, increased, more, attack, cast, action, repeats, tick]
            .into_iter()
            .zip([
                "base",
                "increased",
                "more",
                "attack",
                "cast",
                "action",
                "repeats",
                "tick",
            ])
        {
            assert_eq!(node.expression, RuleExpression::Read { input: key(name) });
        }
        assert_eq!(
            timing.expression,
            RuleExpression::OrdinaryTiming {
                recipe: Box::new(OrdinaryTimingRecipe {
                    base_time: base.id.clone(),
                    increased_percent: increased.id.clone(),
                    more_multiplier: more.id.clone(),
                    additional_attack_time: attack.id.clone(),
                    additional_cast_time: cast.id.clone(),
                    action_speed_multiplier: action.id.clone(),
                    repeats: repeats.id.clone(),
                    server_tick_rate: tick.id.clone(),
                    speed_multiplier_rounding_precision: 2,
                    units: ReciprocalTimingUnits {
                        time: id("unit.time"),
                        rate: id("unit.rate")
                    },
                    output,
                })
            }
        );
        assert_eq!(
            native::execute(&f, &timing_facts()),
            applied(quantity(expected, unit)),
            "{channel}"
        );
        for missing in [
            "base",
            "increased",
            "more",
            "attack",
            "cast",
            "action",
            "repeats",
            "tick",
        ] {
            let mut facts = timing_facts();
            facts.retain(|(name, _)| *name != key(missing));
            assert_eq!(
                native::execute(&f, &facts),
                EffectDisposition::Unresolved {
                    input: key(missing)
                },
                "{channel} must demand {missing}"
            );
        }
    }
}

#[test]
fn timing_capped_channels_survive_nonfinite_pre_cap_and_preserve_lazy_failure() {
    for base in [0.0, 1.0e-310] {
        for (channel, unit, expected) in [
            ("speed_multiplier", "unit.factor", 1.0),
            ("action_rate", "unit.rate", 4.0),
            ("action_time", "unit.time", 0.25),
        ] {
            let (f, _) = timing_fixture(channel, unit, 2);
            let mut facts = timing_facts();
            set(&mut facts, "base", quantity(base, "unit.time"));
            assert_eq!(
                native::execute(&f, &facts),
                applied(quantity(expected, unit)),
                "{channel} at {base}"
            );
        }
        let (f, lowered) = timing_fixture("pre_cap_rate", "unit.rate", 2);
        let mut facts = timing_facts();
        set(&mut facts, "base", quantity(base, "unit.time"));
        let timing = lowered
            .program
            .nodes
            .iter()
            .find(|node| matches!(node.expression, RuleExpression::OrdinaryTiming { .. }))
            .unwrap();
        assert_eq!(
            native::execute(&f, &facts),
            EffectDisposition::NumericalError {
                node: timing.id.clone(),
                reason: NumericalFailure::NonFinite
            }
        );
    }
    let mut f = fixture(quantity_type("unit.rate"));
    let expression = format!(
        "if flag {{ {} }} else {{ quantity(7, unit.rate) }}",
        timing_expression(2, "pre_cap_rate")
    );
    let lowered = lower(
        &program(&format!("{FLAG_READ} {TIMING_READS}"), &expression),
        &f,
    );
    install(&mut f, &lowered);
    assert_eq!(
        native::execute(&f, &[(key("flag"), ParameterValue::Boolean(false))]),
        applied(quantity(7.0, "unit.rate"))
    );
    let mut facts = timing_facts();
    set(&mut facts, "base", quantity(0.0, "unit.time"));
    facts.push((key("flag"), ParameterValue::Boolean(true)));
    assert!(matches!(
        native::execute(&f, &facts),
        EffectDisposition::NumericalError {
            reason: NumericalFailure::NonFinite,
            ..
        }
    ));

    let guarded = format!(
        "program \"local-item\" context equipment_use {{ {FLAG_READ} {TIMING_READS}
         let permitted = flag; let result = {};
         effect \"result\" when permitted = derive(current, stat.result, result); }}",
        timing_expression(2, "pre_cap_rate")
    );
    let lowered = lower(&guarded, &f);
    install(&mut f, &lowered);
    assert_eq!(
        native::execute(&f, &[(key("flag"), ParameterValue::Boolean(false))]),
        EffectDisposition::Inactive
    );
    assert!(matches!(
        native::execute(&f, &facts),
        EffectDisposition::NumericalError {
            reason: NumericalFailure::NonFinite,
            ..
        }
    ));
}

#[test]
fn timing_precision_caps_repeats_and_denominator_association_have_independent_results() {
    for (precision, increased, expected) in [
        (0, 13.555, 1.0),
        (2, 13.555, 1.14),
        (3, 13.555, 1.136),
        (0, -250.0, -1.0),
        (12, 0.0, 1.0),
    ] {
        let (f, _) = timing_fixture("speed_multiplier", "unit.factor", precision);
        let mut facts = timing_facts();
        set(&mut facts, "increased", quantity(increased, "unit.percent"));
        assert_eq!(
            native::execute(&f, &facts),
            applied(quantity(expected, "unit.factor"))
        );
    }
    for (channel, unit, repeat, expected) in [
        ("pre_cap_rate", "unit.rate", 1, 6.0),
        ("action_rate", "unit.rate", 1, 4.0),
        ("action_time", "unit.time", 1, 0.25),
        ("action_rate", "unit.rate", 2, 6.0),
        ("action_time", "unit.time", 2, 1.0 / 6.0),
    ] {
        let (f, _) = timing_fixture(channel, unit, 2);
        let mut facts = timing_facts();
        set(&mut facts, "action", quantity(3.0, "unit.factor"));
        set(&mut facts, "repeats", integer(repeat));
        assert_eq!(
            native::execute(&f, &facts),
            applied(quantity(expected, unit))
        );
    }
    for (channel, unit) in [
        ("pre_cap_rate", "unit.rate"),
        ("action_rate", "unit.rate"),
        ("action_time", "unit.time"),
    ] {
        let (f, _) = timing_fixture(channel, unit, 2);
        let mut facts = timing_facts();
        set(&mut facts, "base", quantity(1.0e16, "unit.time"));
        set(&mut facts, "attack", quantity(-1.0e16, "unit.time"));
        set(&mut facts, "cast", quantity(1.0, "unit.time"));
        assert_eq!(
            native::execute(&f, &facts),
            applied(quantity(1.0, unit)),
            "left-associated denominator: {channel}"
        );
    }
}

#[test]
fn operation_metadata_and_dynamic_types_reject_malformed_calls_without_evaluation() {
    let mut f = fixture(ComputedValueType::Integer);
    f.rules
        .tables
        .push(table(ComputedValueType::Integer, vec![integer(1); 3]));
    for expression in [
        "check_domain(input, 3, -2)",
        "check_domain(input, true, false)",
        "check_domain(input, -2, quantity(3, unit.damage))",
        "check_domain(input, input, 3)",
        "check_domain(input, -2 + 0, 3)",
        "check_domain(input, -2)",
        "check_domain(input, -2, 3, 4)",
        "lookup(input, signed_table)",
        "lookup(input, \"unknown-table\")",
        "lookup(input)",
        "lookup(true, \"signed-table\")",
        "option(option.unknown)",
        "option(stat.integer)",
        "option(\"option.first\")",
        "option(option.first, option.second)",
    ] {
        rejected(&program(INTEGER_READ, expression), &f);
    }
    let f = fixture(quantity_type("unit.time"));
    let good = timing_expression(2, "action_time");
    for expression in [
        good.replace(", 2,", ", 13,"),
        good.replace(", 2,", ", -1,"),
        good.replace(", 2,", ", 2.5,"),
        good.replace(", 2,", ", repeats,"),
        good.replace(", 2,", ", 1 + 1,"),
        good.replace("action_time)", "unknown_channel)"),
        good.replace("unit.time, unit.rate", "unit.damage, unit.rate"),
        good.replace("unit.time, unit.rate", "unit.time, unit.time"),
        good.replace("base, increased", "quantity(1, unit.other_time), increased"),
        good.replace("cast, action", "quantity(0, unit.other_time), action"),
        good.replace("cast, action", "cast, quantity(1, unit.other_factor)"),
        good.replace("repeats, tick", "quantity(1, unit.factor), tick"),
        good.replace("repeats, tick", "repeats, quantity(4, unit.time)"),
        good.replace("base, increased", "base, quantity(25, unit.factor)"),
        good.replace("increased, more", "increased, quantity(1, unit.percent)"),
        good.replace(", unit.rate, action_time", ", unit.rate"),
    ] {
        rejected(&program(TIMING_READS, &expression), &f);
    }
    assert_eq!(
        rejected(
            &program(
                NUMBER_READ,
                "check_domain(input, quantity(1, unit.damage), quantity(2, unit.time))"
            ),
            &f
        )
        .code,
        DiagnosticCode::TypeMismatch
    );
}

#[test]
fn static_operation_metadata_is_bounded_but_does_not_become_runtime_nodes() {
    let mut f = fixture(ComputedValueType::Integer);
    f.rules
        .tables
        .push(table(ComputedValueType::Integer, vec![integer(1); 3]));
    for source in [
        program(INTEGER_READ, "check_domain(input, -2, 3)"),
        program(INTEGER_READ, "lookup(input, \"signed-table\")"),
        program("", "option(option.first)"),
        program(TIMING_READS, &timing_expression(12, "action_rate")),
    ] {
        let parsed = parse_program(&source, ProgramParseLimits::default()).unwrap();
        let registry = symbols(&f);
        let actual = lower(&source, &f);
        let count = actual.program.nodes.len();
        let mut limits = ProgramLowerLimits::default();
        limits.formulas.max_nodes = count;
        assert_eq!(
            lower_program(&parsed, &f.schema, &registry, &f.rules.tables, limits).unwrap(),
            actual
        );
        if count > 1 {
            limits.formulas.max_nodes = count - 1;
            assert_eq!(
                lower_program(&parsed, &f.schema, &registry, &f.rules.tables, limits)
                    .unwrap_err()
                    .code,
                DiagnosticCode::LimitExceeded(LimitKind::LoweredNodes)
            );
        }
        limits = ProgramLowerLimits::default();
        limits.formulas.max_syntax_nodes = 1;
        assert_eq!(
            lower_program(&parsed, &f.schema, &registry, &f.rules.tables, limits)
                .unwrap_err()
                .code,
            DiagnosticCode::LimitExceeded(LimitKind::Nodes)
        );
        limits = ProgramLowerLimits::default();
        let symbolic_literals = f
            .schema
            .input()
            .definitions
            .iter()
            .filter(|definition| {
                matches!(
                    definition,
                    DefinitionDescriptor::Unit(_) | DefinitionDescriptor::Option(_)
                )
            })
            .count();
        limits.bindings.max_bindings =
            symbolic_literals + parsed.reads().len() + f.rules.tables.len();
        assert_eq!(
            lower_program(&parsed, &f.schema, &registry, &f.rules.tables, limits).unwrap(),
            actual
        );
        limits.bindings.max_bindings -= 1;
        assert_eq!(
            lower_program(&parsed, &f.schema, &registry, &f.rules.tables, limits)
                .unwrap_err()
                .code,
            DiagnosticCode::LimitExceeded(LimitKind::Bindings)
        );
    }
}
