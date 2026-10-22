//! Lowered expressions execute only through the existing checked Engine.
use crate::*;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{BoundedInteger, FiniteQuantity, OwnedDefinitionKey, UnitDefId},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
    SchemaPackageInput,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, NumericalFailure, RuleFact, RuleLimits,
};

#[allow(dead_code)]
#[path = "../../poe-optimizer-engine/tests/support/owned_rule_fixture.rs"]
pub(super) mod fixture;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn local(f: &fixture::Fixture) -> &RuleProgram {
    f.rules.owners[0]
        .programs
        .members
        .iter()
        .find(|program| program.id == key("local-item"))
        .unwrap()
}

pub(super) fn bindings(f: &fixture::Fixture) -> ExpressionBindings {
    let units = [
        ("damage", "unit.damage"),
        ("percent", "unit.percent"),
        ("%", "unit.percent"),
        ("factor", "unit.factor"),
        ("units.e2", "unit.damage"),
    ]
    .map(|(name, value)| {
        (
            name,
            UnitDefId::parse(f.rules.namespace.clone(), value).unwrap(),
        )
    });
    let names: Vec<_> = local(f)
        .reads
        .iter()
        .map(|read| read.id.as_str().replace('-', "_"))
        .collect();
    let reads: Vec<_> = names
        .iter()
        .zip(&local(f).reads)
        .map(|(name, read)| (name.as_str(), read.clone()))
        .collect();
    ExpressionBindings::from_inputs(
        &f.schema,
        &units,
        &reads,
        &[],
        &[],
        BindingLimits::default(),
    )
    .unwrap()
}

fn lower(source: &str, bindings: &ExpressionBindings) -> LoweredExpression {
    lower_expression(
        &parse_expression(source, ParseLimits::default()).unwrap(),
        bindings,
        LowerLimits::default(),
    )
    .unwrap()
}

pub(super) fn install(f: &mut fixture::Fixture, lowered: LoweredExpression) {
    let program = f.rules.owners[0]
        .programs
        .members
        .iter_mut()
        .find(|program| program.id == key("local-item"))
        .unwrap();
    let RuleEffectKind::Derive { value, .. } = &mut program.effects[0].effect else {
        panic!("fixture declares a derived local result")
    };
    *value = lowered.root;
    program.nodes = lowered.nodes;
}

pub(super) fn execute(
    f: &fixture::Fixture,
    facts: &[(OwnedDefinitionKey, ParameterValue)],
) -> EffectDisposition {
    let compiled =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    let facts: Vec<_> = facts
        .iter()
        .map(|(read, value)| RuleFact {
            read: read.clone(),
            value: value.clone(),
        })
        .collect();
    let report = compiled
        .evaluate(
            &f.rules.owners[0].owner,
            &key("local-item"),
            &facts,
            &f.schema,
            &mut compiled.new_scratch(),
        )
        .unwrap();
    report.effects.into_iter().next().unwrap().disposition
}

pub(super) fn damage(f: &fixture::Fixture, value: f64) -> EffectDisposition {
    EffectDisposition::Applied {
        value: ParameterValue::Quantity(
            FiniteQuantity::new(
                value,
                UnitDefId::parse(f.rules.namespace.clone(), "unit.damage").unwrap(),
            )
            .unwrap(),
        ),
    }
}

#[test]
fn arithmetic_formula_matches_independently_authored_ir_and_expected_values() {
    let mut f = fixture::fixture();
    let expected: Vec<_> = f.cases[..2]
        .iter()
        .map(|case| execute(&f, &case.facts))
        .collect();
    let source = "round((100damage + flat) * (1factor + pct(increase, factor)) * \
        (if has_quality { 1factor + pct(quality, factor) } else { 1factor }), \
        1damage, nearest_ties_positive)";
    let lowered = lower(source, &bindings(&f));
    install(&mut f, lowered);
    for (index, value) in [165.0, 132.0].into_iter().enumerate() {
        let actual = execute(&f, &f.cases[index].facts);
        assert_eq!(actual, expected[index]);
        assert_eq!(actual, damage(&f, value));
    }
}

#[test]
fn explicit_numeric_builtins_execute_with_independent_units_and_rounding_expectations() {
    let mut f = fixture::fixture();
    let names = bindings(&f);
    for (source, expected) in [
        ("4damage * ratio(10damage, 4damage, factor)", 10.0),
        ("100damage * pct(25percent, factor)", 25.0),
        ("scale(8damage, 1.5factor)", 12.0),
        ("scale_integer(3damage, 4)", 12.0),
        ("divide_factor(9damage, 3factor)", 3.0),
        ("max(min(8damage, 6damage), 4damage)", 6.0),
        ("if all() { 100damage } else { 0damage }", 100.0),
        ("if any() { 100damage } else { 0damage }", 0.0),
        ("round(-1.5damage, 1damage, floor)", -2.0),
        ("round(-1.5damage, 1damage, ceiling)", -1.0),
        ("round(-1.5damage, 1damage, truncate)", -1.0),
        (
            "round(-1.5damage, quantity(1, units.e2), nearest_ties_positive)",
            -1.0,
        ),
        (
            "scale_integer(1damage, quantize_integer(-1.5damage, 1damage, floor))",
            -2.0,
        ),
        (
            "scale_integer(1damage, quantize_integer(-1.5damage, 1damage, ceiling))",
            -1.0,
        ),
        (
            "scale_integer(1damage, quantize_integer(-1.5damage, 1damage, truncate))",
            -1.0,
        ),
        (
            "scale_integer(1damage, quantize_integer(-1.5damage, 1damage, nearest_ties_positive))",
            -1.0,
        ),
    ] {
        install(&mut f, lower(source, &names));
        assert_eq!(execute(&f, &[]), damage(&f, expected), "{source}");
    }
    install(&mut f, lower("-flat", &names));
    assert_eq!(execute(&f, &f.cases[0].facts), damage(&f, -10.0));
}

#[test]
fn lazy_select_preserves_missing_inputs_and_numerical_failures() {
    let mut f = fixture::fixture();
    let names = bindings(&f);
    let mut facts = f.cases[1].facts.clone();
    install(
        &mut f,
        lower(
            "100damage * (1factor + pct(if has_quality { quality } else { 0percent }, factor))",
            &names,
        ),
    );
    assert_eq!(execute(&f, &facts), damage(&f, 100.0));
    facts
        .iter_mut()
        .find(|(id, _)| *id == key("has-quality"))
        .unwrap()
        .1 = ParameterValue::Boolean(true);
    assert_eq!(
        execute(&f, &facts),
        EffectDisposition::Unresolved {
            input: key("quality")
        }
    );
    install(
        &mut f,
        lower(
            "if has_quality { 100damage / 0factor } else { 100damage }",
            &names,
        ),
    );
    assert!(matches!(
        execute(&f, &facts),
        EffectDisposition::NumericalError {
            reason: NumericalFailure::DivisionByZero,
            ..
        }
    ));
    facts
        .iter_mut()
        .find(|(id, _)| *id == key("has-quality"))
        .unwrap()
        .1 = ParameterValue::Boolean(false);
    assert_eq!(execute(&f, &facts), damage(&f, 100.0));
}

#[test]
fn boolean_order_preserves_existing_short_circuit_and_failure_precedence() {
    let mut f = fixture::fixture();
    let names = bindings(&f);
    for (operator, flag, value) in [("any", true, 100.0), ("all", false, 0.0)] {
        let mut facts = f.cases[1].facts.clone();
        facts
            .iter_mut()
            .find(|(id, _)| *id == key("has-quality"))
            .unwrap()
            .1 = ParameterValue::Boolean(flag);
        let source = format!(
            "if {operator}(has_quality, quality > 0percent) {{ 100damage }} else {{ 0damage }}"
        );
        install(&mut f, lower(&source, &names));
        assert_eq!(execute(&f, &facts), damage(&f, value));
        let source = format!(
            "if {operator}(quality > 0percent, has_quality) {{ 100damage }} else {{ 0damage }}"
        );
        install(&mut f, lower(&source, &names));
        assert_eq!(
            execute(&f, &facts),
            EffectDisposition::Unresolved {
                input: key("quality")
            }
        );
    }
}

#[test]
fn repeated_reads_and_boolean_association_are_not_deduplicated_or_flattened() {
    let f = fixture::fixture();
    let names = bindings(&f);
    let nested = lower("has_quality or has_quality or has_quality", &names);
    let variadic = lower("any(has_quality, has_quality, has_quality)", &names);
    assert_eq!(
        nested
            .nodes
            .iter()
            .filter(|node| matches!(node.expression, RuleExpression::Read { .. }))
            .count(),
        3
    );
    let nested_folds: Vec<_> = nested
        .nodes
        .iter()
        .filter_map(|node| match &node.expression {
            RuleExpression::Any { values } => Some(values),
            _ => None,
        })
        .collect();
    assert_eq!(nested_folds.len(), 2);
    assert!(nested_folds.iter().all(|values| values.len() == 2));
    let RuleExpression::Any { values } = &variadic.nodes.last().unwrap().expression else {
        panic!()
    };
    assert_eq!(
        values,
        &variadic.nodes[..3]
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>()
    );
    let arithmetic = lower("flat - 2damage - 3damage", &names);
    let RuleExpression::Subtract { left, right } = &arithmetic.nodes.last().unwrap().expression
    else {
        panic!()
    };
    assert!(matches!(
        arithmetic
            .nodes
            .iter()
            .find(|node| &node.id == left)
            .unwrap()
            .expression,
        RuleExpression::Subtract { .. }
    ));
    assert!(matches!(
        arithmetic
            .nodes
            .iter()
            .find(|node| &node.id == right)
            .unwrap()
            .expression,
        RuleExpression::Literal { .. }
    ));
}

#[test]
fn invalid_symbols_units_types_and_nonfinite_literals_fail_before_execution() {
    let f = fixture::fixture();
    let names = bindings(&f);
    for source in [
        "missing",
        "self.life",
        "1unknown",
        "1.5",
        "1e2",
        "1e309damage",
        "9007199254740992",
        "-9007199254740992",
        "flat + 1percent",
        "1factor * flat",
        "flat / 1damage",
        "has_quality + 1",
        "pct(flat, factor)",
        "if has_quality { flat } else { 1factor }",
        "min(flat, 1percent)",
        "round(flat, flat, floor)",
        "round(flat, 1damage + 1damage, floor)",
        "round(flat, 0damage, floor)",
        "round(flat, 1damage, guessed_mode)",
        "round(flat, 1damage)",
        "ratio(1damage, 2damage)",
        "pct(25percent)",
        "min(flat)",
        "unknown(flat)",
        "self.stat(flat)",
        "quantity(flat, damage)",
    ] {
        let parsed = parse_expression(source, ParseLimits::default()).unwrap();
        assert!(
            lower_expression(&parsed, &names, LowerLimits::default()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn exact_literal_domains_and_explicit_quantity_constructor_use_core_semantics() {
    let f = fixture::fixture();
    let names = bindings(&f);
    for source in ["-0damage", "quantity(-0, units.e2)"] {
        let lowered = lower(source, &names);
        let RuleExpression::Literal {
            value: ParameterValue::Quantity(value),
        } = &lowered.nodes.last().unwrap().expression
        else {
            panic!("{source}")
        };
        assert_eq!(value.value().to_bits(), 0.0f64.to_bits());
    }
    for value in [BoundedInteger::MIN, BoundedInteger::MAX] {
        let lowered = lower(&value.to_string(), &names);
        let RuleExpression::Literal {
            value: ParameterValue::Integer(actual),
        } = &lowered.nodes.last().unwrap().expression
        else {
            panic!()
        };
        assert_eq!(actual.get(), value);
    }
    let lowered = lower("quantity(1.25e2, units.e2)", &names);
    let RuleExpression::Literal {
        value: ParameterValue::Quantity(value),
    } = &lowered.nodes.last().unwrap().expression
    else {
        panic!()
    };
    assert_eq!(value.value(), 125.0);
    assert_eq!(
        value.unit(),
        names.unit("damage").map(|binding| &binding.unit).unwrap()
    );
}

#[test]
fn lowering_limits_bound_expansion_and_long_syntax_trees() {
    let f = fixture::fixture();
    let names = bindings(&f);
    let parsed = parse_expression("flat != 1damage", ParseLimits::default()).unwrap();
    let lowered = lower_expression(&parsed, &names, LowerLimits::default()).unwrap();
    assert!(matches!(
        lowered.nodes.last().unwrap().expression,
        RuleExpression::Not { .. }
    ));
    assert!(
        lower_expression(
            &parsed,
            &names,
            LowerLimits {
                max_nodes: lowered.nodes.len() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        lower_expression(
            &parsed,
            &names,
            LowerLimits {
                max_syntax_nodes: parsed.nodes().len() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || {
            let source = std::iter::repeat_n("1", 3000)
                .collect::<Vec<_>>()
                .join(" + ");
            let parsed = parse_expression(&source, ParseLimits::default()).unwrap();
            let lowered = lower_expression(&parsed, &names, LowerLimits::default()).unwrap();
            assert_eq!(lowered.nodes.len(), 5999);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn frontend_type_information_does_not_replace_engine_read_authority() {
    let mut f = fixture::fixture();
    let program = f.rules.owners[0]
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("local-item"))
        .unwrap();
    // A forged declaration claims the damage contribution is an integer.
    program
        .reads
        .iter_mut()
        .find(|read| read.id == key("flat"))
        .unwrap()
        .value_type = ComputedValueType::Integer;
    let lowered = lower("flat + 1", &bindings(&f));
    install(&mut f, lowered);
    assert!(CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).is_err());
}

#[test]
fn retained_life_guard_matches_exact_ir_shape_and_source_emission_vectors() {
    // The packet is the authored source. The old JSON is a historical comparison
    // fixture; source vectors independently constrain behavior. Neither grants
    // complete Life coverage or admission for the original builds.
    let dependencies: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/owned/poe2/3887ae68/inherent-life-contribution/dependencies.json"
    ))
    .unwrap();
    let original: RuleProgram = serde_json::from_str(include_str!(
        "../../../data/owned/poe2/3887ae68/inherent-life-contribution/program.json"
    ))
    .unwrap();
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/owned/poe2/3887ae68/inherent-life-contribution/source-vectors.json"
    ))
    .unwrap();
    let ComputedValueType::Quantity { unit } = &original.reads[0].value_type else {
        panic!()
    };
    let unit = unit.clone();
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: unit.namespace().clone(),
            release: key("retained-life-guard"),
            semantics_version: key("retained-life-guard-v1"),
            definitions: serde_json::from_value(dependencies["definitions"].clone()).unwrap(),
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let symbols = load_names(
        include_str!("../../../data/owned/poe2/3887ae68/inherent-life-contribution/names.kdl"),
        &schema,
        NamesLimits::default(),
    )
    .unwrap();
    let manifest = parse_packet(
        include_str!("../../../data/owned/poe2/3887ae68/inherent-life-contribution/packet.kdl"),
        &symbols,
        ManifestLimits::default(),
    )
    .unwrap();
    let packet = lower_packet(
        &manifest,
        &[RuleSource {
            name: "inherent-life.rule",
            text: include_str!(
                "../../../data/owned/poe2/3887ae68/inherent-life-contribution/inherent-life.rule"
            ),
        }],
        &schema,
        &symbols,
        &[],
        PacketLimits::default(),
    )
    .unwrap();
    assert_eq!(packet.programs().len(), 1);
    let lowered = &packet.programs()[0].lowered;
    let bindings: std::collections::BTreeMap<_, _> = lowered
        .bindings
        .iter()
        .map(|binding| (binding.name.as_str(), binding.value.clone()))
        .collect();
    assert_eq!(bindings.len(), lowered.bindings.len());
    assert_eq!(bindings.len(), original.nodes.len());
    let map: std::collections::BTreeMap<_, _> = original
        .nodes
        .iter()
        .map(|node| {
            let name = node.id.as_str().replace('-', "_");
            (node.id.clone(), bindings[name.as_str()].clone())
        })
        .collect();
    assert_eq!(
        map.values()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        original.nodes.len(),
        "the authored program preserves distinct historical nodes"
    );
    // Remap only node identities in the independent historical program. Exact
    // equality retains every read, operand order, effect ID, target and guard.
    let mut expected = original.clone();
    for node in &mut expected.nodes {
        node.id = map[&node.id].clone();
        match &mut node.expression {
            RuleExpression::Read { .. } => {}
            RuleExpression::Any { values } => {
                for value in values {
                    *value = map[value].clone();
                }
            }
            RuleExpression::Not { value } => *value = map[value].clone(),
            other => panic!("retained bridge acquired an unreviewed operation: {other:?}"),
        }
    }
    for effect in &mut expected.effects {
        if let Some(guard) = &mut effect.when {
            *guard = map[guard].clone();
        }
        let RuleEffectKind::Contribute { value, .. } = &mut effect.effect else {
            panic!("retained bridge must remain a contribution")
        };
        *value = map[value].clone();
    }
    assert_eq!(lowered.program, expected);
    let migrated = lowered.program.clone();
    let owner: SchemaSubject =
        serde_json::from_value(dependencies["owner_before"]["owner"].clone()).unwrap();
    let closure: SchemaClosure =
        serde_json::from_value(dependencies["owner_before"]["programs"]["closure"].clone())
            .unwrap();
    assert!(!matches!(closure, SchemaClosure::Complete));
    let mut input = fixture::fixture().rules;
    input.namespace = unit.namespace().clone();
    input.definitions = schema.identity().clone();
    input.operations_version = key(OWNED_RULE_OPERATIONS_VERSION);
    // Current operations require explicit inventories. This isolated guard has
    // no application families or contribution-query readers; omitted inventories
    // inherited from the older fixture would not be a valid current package.
    input.effect_applications = Some(DeclaredSet::complete(vec![]));
    input.contribution_queries = Some(DeclaredSet::complete(vec![]));
    input.existing_actor_rules =
        Some(serde_json::from_value(dependencies["existing_actor_rules"].clone()).unwrap());
    input.owners = vec![DefinitionRules {
        owner: owner.clone(),
        programs: DeclaredSet {
            closure: closure.clone(),
            members: vec![original.clone()],
        },
    }];
    let reference = CompiledRulePackage::compile(&input, &schema, RuleLimits::default()).unwrap();
    input.owners[0].programs.members.clear();
    let before = input.clone();
    packet.apply_to(&mut input).unwrap();
    assert_eq!(input.owners[0].programs.closure, closure);
    assert_eq!(input.owners[0].programs.members, [migrated]);
    let mut inverse = input.clone();
    inverse.owners[0].programs.members.clear();
    assert_eq!(inverse, before, "only the declared membership was appended");
    let compiled = CompiledRulePackage::compile(&input, &schema, RuleLimits::default()).unwrap();
    for vector in vectors["emission_cases"].as_array().unwrap() {
        let mut facts = vec![RuleFact {
            read: key("amount"),
            value: ParameterValue::Quantity(
                FiniteQuantity::new(vector["inherent_life"].as_f64().unwrap(), unit.clone())
                    .unwrap(),
            ),
        }];
        for (read, flag) in [
            ("all-disabled", "NoAttributeBonuses"),
            ("strength-disabled", "NoStrengthAttributeBonuses"),
            ("life-disabled", "NoStrBonusToLife"),
        ] {
            facts.push(RuleFact {
                read: key(read),
                value: ParameterValue::Boolean(vector["flags"][flag].as_bool().unwrap()),
            });
        }
        let expected = reference
            .evaluate(
                &owner,
                &original.id,
                &facts,
                &schema,
                &mut reference.new_scratch(),
            )
            .unwrap();
        let actual = compiled
            .evaluate(
                &owner,
                &original.id,
                &facts,
                &schema,
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(
            actual.effects[0].disposition, expected.effects[0].disposition,
            "{}",
            vector["name"]
        );
        if vector["emitted_count"] == 0 {
            assert_eq!(actual.effects[0].disposition, EffectDisposition::Inactive);
        } else {
            assert_eq!(
                actual.effects[0].disposition,
                EffectDisposition::Applied {
                    value: facts[0].value.clone()
                }
            );
        }
    }
}
