//! Owned timing is a typed atomic algorithm with independently classified outputs.
//! Synthetic inputs prove composition/availability, not a complete game timing branch.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod owned_plan_fixture;
use owned_plan_fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, RuleStorageLimits},
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::NumericalFailure};
use std::sync::Arc;

fn quantity(v: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, def(unit)).unwrap())
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn recipe(output: OrdinaryTimingChannel) -> OrdinaryTimingRecipe {
    OrdinaryTimingRecipe {
        base_time: key("base"),
        increased_percent: key("increased"),
        more_multiplier: key("more"),
        additional_attack_time: key("attack"),
        additional_cast_time: key("cast"),
        action_speed_multiplier: key("action-speed"),
        repeats: key("repeats"),
        server_tick_rate: key("tick"),
        speed_multiplier_rounding_precision: 2,
        units: ReciprocalTimingUnits {
            time: def("seconds"),
            rate: def("per-second"),
        },
        output,
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.add_action_route();
    f.routes[0].routes.members.clear();
    for (name, dimension) in [
        ("seconds", UnitDimension::Time),
        ("other-time", UnitDimension::Time),
        ("per-second", UnitDimension::Rate),
        ("factor", UnitDimension::DimensionlessFactor),
        ("other-factor", UnitDimension::DimensionlessFactor),
        ("percent", UnitDimension::PercentagePoints),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Unit(entry(
            def(name),
            UnitSchema { dimension },
        )));
    }
    let mut nodes = vec![];
    for (name, value, unit) in [
        ("base", 1.0, "seconds"),
        ("increased", 25.0, "percent"),
        ("more", 1.6, "factor"),
        ("attack", 0.25, "seconds"),
        ("cast", 0.25, "seconds"),
        ("action-speed", 1.5, "factor"),
        ("tick", 1.0, "per-second"),
    ] {
        nodes.push(node(
            name,
            RuleExpression::Literal {
                value: quantity(value, unit),
            },
        ));
    }
    nodes.push(literal("repeats", 1));
    let mut effects = vec![];
    for (name, unit, output) in [
        (
            "multiplier",
            "factor",
            OrdinaryTimingChannel::SpeedMultiplier,
        ),
        ("pre-cap", "per-second", OrdinaryTimingChannel::PreCapRate),
        ("rate", "per-second", OrdinaryTimingChannel::ActionRate),
        ("time", "seconds", OrdinaryTimingChannel::ActionTime),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
            def(name),
            StatSchema {
                value: ComputedValueType::Quantity { unit: def(unit) },
                targets: vec![RuleEntityKind::Action],
            },
        )));
        nodes.push(node(
            name,
            RuleExpression::OrdinaryTiming {
                recipe: Box::new(recipe(output)),
            },
        ));
        effects.push(derive(name, RuleEntity::Current, name, name));
    }
    f.owner_mut(&SchemaSubject::Slot(ActionOutputDefId::address(&output())))
        .programs
        .members
        .push(RuleProgram {
            id: key("timing"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes,
            effects,
        });
    f
}
fn program(f: &mut Fixture) -> &mut RuleProgram {
    &mut f
        .owner_mut(&SchemaSubject::Slot(ActionOutputDefId::address(&output())))
        .programs
        .members[0]
}
fn set(f: &mut Fixture, id: &str, value: ParameterValue) {
    program(f)
        .nodes
        .iter_mut()
        .find(|n| n.id == key(id))
        .unwrap()
        .expression = RuleExpression::Literal { value };
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn value<'a>(report: &'a OwnedEffectsReport, name: &str) -> &'a EffectValue {
    let wanted = PlanValueKey::Stat {
        entity: ConcreteEntity::Action(Box::new(action())),
        stat: def(name),
    };
    &report
        .values
        .iter()
        .find(|r| r.key == wanted)
        .unwrap()
        .value
}
fn known(report: &OwnedEffectsReport, name: &str, number: f64, unit: &str) {
    assert_eq!(
        value(report, name),
        &EffectValue::Known {
            value: quantity(number, unit)
        }
    );
}
fn uncapped(f: &mut Fixture) {
    for (name, number, unit) in [
        ("base", 0.0, "seconds"),
        ("increased", 0.0, "percent"),
        ("more", 1.0, "factor"),
        ("attack", 0.0, "seconds"),
        ("cast", 0.0, "seconds"),
        ("action-speed", 1.0, "factor"),
        ("tick", 30.0, "per-second"),
    ] {
        set(f, name, quantity(number, unit));
    }
}
#[test]
fn typed_timing_applies_action_speed_before_cap_and_retains_uncapped_rate() {
    let mut f = fixture();
    let a = evaluate(&f);
    assert!(a.gaps.is_empty());
    known(&a, "multiplier", 2.0, "factor");
    known(&a, "pre-cap", 1.5, "per-second");
    known(&a, "rate", 1.0, "per-second");
    known(&a, "time", 1.0, "seconds");
    set(&mut f, "tick", quantity(2.0, "per-second"));
    let b = evaluate(&f);
    known(&b, "rate", 1.5, "per-second");
    known(&b, "time", 2.0 / 3.0, "seconds");
    assert_ne!(a.identity, b.identity);
}
#[test]
fn nonfinite_channels_do_not_poison_finite_capped_outputs() {
    for mode in ["infinite-rate", "unordered-rate", "infinite-multiplier"] {
        let mut f = fixture();
        uncapped(&mut f);
        match mode {
            "unordered-rate" => set(&mut f, "more", quantity(0.0, "factor")),
            "infinite-multiplier" => {
                set(&mut f, "more", quantity(f64::MAX, "factor"));
                set(&mut f, "increased", quantity(f64::MAX, "percent"));
            }
            _ => {}
        }
        let r = evaluate(&f);
        assert!(
            matches!(
                value(&r, "pre-cap"),
                EffectValue::NumericalError {
                    reason: NumericalFailure::NonFinite,
                    ..
                }
            ),
            "{mode}: {r:?}"
        );
        known(&r, "rate", 30.0, "per-second");
        known(&r, "time", 1.0 / 30.0, "seconds");
        if mode == "infinite-multiplier" {
            assert!(matches!(
                value(&r, "multiplier"),
                EffectValue::NumericalError {
                    reason: NumericalFailure::NonFinite,
                    ..
                }
            ));
        }
    }
}
#[test]
fn ordinary_algorithm_domain_does_not_silently_enforce_gameplay_ranges() {
    let mut f = fixture();
    set(&mut f, "tick", quantity(-2.0, "per-second"));
    let r = evaluate(&f);
    known(&r, "rate", -2.0, "per-second");
    known(&r, "time", -0.5, "seconds");
    set(&mut f, "repeats", integer(0));
    let r = evaluate(&f);
    known(&r, "rate", 0.0, "per-second");
    known(&r, "time", 0.0, "seconds");
    // Valid game tick/repeat ranges are injected input schemas/requirements.
    // This is computational evidence, not a legal character or eligibility claim.
}
#[test]
fn missing_atomic_input_is_not_zero_but_an_inactive_branch_does_not_demand_it() {
    let mut f = fixture();
    program(&mut f).reads.push(RuleRead {
        id: key("missing-time"),
        value_type: ComputedValueType::Quantity {
            unit: def("seconds"),
        },
        source: RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: def("time"),
        },
    });
    // Use a distinct declared stat with no producer, not a self-cycle.
    f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
        def("missing-base"),
        StatSchema {
            value: ComputedValueType::Quantity {
                unit: def("seconds"),
            },
            targets: vec![RuleEntityKind::Action],
        },
    )));
    program(&mut f).reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def("missing-base"),
    };
    program(&mut f)
        .nodes
        .iter_mut()
        .find(|n| n.id == key("base"))
        .unwrap()
        .expression = RuleExpression::Read {
        input: key("missing-time"),
    };
    let r = evaluate(&f);
    for name in ["multiplier", "pre-cap", "rate", "time"] {
        assert!(
            matches!(value(&r, name), EffectValue::Unresolved { .. }),
            "{name}: {r:?}"
        );
    }
    let p = program(&mut f);
    p.nodes.push(node(
        "disabled",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    ));
    for e in &mut p.effects {
        e.when = Some(key("disabled"));
    }
    let r = evaluate(&f);
    for name in ["multiplier", "pre-cap", "rate", "time"] {
        assert_eq!(value(&r, name), &EffectValue::Inactive);
    }
}
#[test]
fn preparation_checks_exact_units_integer_repeats_and_bounded_precision() {
    for case in [
        "time-unit",
        "rate-unit",
        "factor-unit",
        "repeats",
        "precision",
    ] {
        let mut f = fixture();
        match case {
            "time-unit" => set(&mut f, "attack", quantity(0.25, "other-time")),
            "rate-unit" => set(&mut f, "tick", quantity(1.0, "seconds")),
            "factor-unit" => set(&mut f, "action-speed", quantity(1.0, "other-factor")),
            "repeats" => set(&mut f, "repeats", quantity(1.0, "factor")),
            _ => {
                for n in &mut program(&mut f).nodes {
                    if let RuleExpression::OrdinaryTiming { recipe } = &mut n.expression {
                        recipe.speed_multiplier_rounding_precision = 13;
                    }
                }
            }
        }
        // Fixture::compile intentionally unwraps component compilation. Compile the
        // injected rule package explicitly here so rejection is the assertion.
        let schema =
            OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default())
                .unwrap();
        let rules = package(&f, &schema);
        assert!(
            poe_optimizer_engine::owned_rules::CompiledRulePackage::compile(
                &rules,
                &schema,
                Default::default()
            )
            .is_err(),
            "{case}"
        );
    }
}
fn package(f: &Fixture, schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    RulePackageInput {
        receivers: DeclaredSet::complete(vec![]),
        tables: vec![],
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("timing"),
        semantics_version: key("test"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: f.owners.clone(),
    }
}
#[test]
fn timing_references_are_validated_by_storage_and_execution_is_worker_local() {
    let f = fixture();
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let p = package(&f, &schema);
    let stored = OwnedRulePackage::new(p.clone(), &schema, RuleStorageLimits::default()).unwrap();
    assert_eq!(
        stored.input().operations_version.as_str(),
        OWNED_RULE_OPERATIONS_VERSION
    );
    let expression = RuleExpression::OrdinaryTiming {
        recipe: Box::new(recipe(OrdinaryTimingChannel::ActionRate)),
    };
    let encoded = serde_json::to_value(&expression).unwrap();
    assert_eq!(
        serde_json::from_value::<RuleExpression>(encoded.clone()).unwrap(),
        expression
    );
    for field in ["recipe", "units"] {
        let mut unknown = encoded.clone();
        if field == "recipe" {
            unknown["recipe"]["unexpected"] = true.into();
        } else {
            unknown["recipe"]["units"]["unexpected"] = true.into();
        }
        assert!(serde_json::from_value::<RuleExpression>(unknown).is_err());
    }
    let mut bad = p;
    for owner in &mut bad.owners {
        for program in &mut owner.programs.members {
            for n in &mut program.nodes {
                if let RuleExpression::OrdinaryTiming { recipe } = &mut n.expression {
                    recipe.server_tick_rate = key("missing-node");
                }
            }
        }
    }
    assert!(OwnedRulePackage::new(bad, &schema, RuleStorageLimits::default()).is_err());
    let a = Arc::new(f.compile().unwrap());
    let mut b = fixture();
    uncapped(&mut b);
    let b = b.compile().unwrap();
    let mut scratch = a.new_scratch();
    let expected = a.evaluate(&mut scratch).unwrap();
    b.evaluate(&mut scratch).unwrap();
    assert_eq!(expected, a.evaluate(&mut scratch).unwrap());
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let a = Arc::clone(&a);
            std::thread::spawn(move || a.evaluate(&mut a.new_scratch()).unwrap())
        })
        .collect();
    for worker in workers {
        assert_eq!(expected, worker.join().unwrap());
    }
}
