//! Canonical scalar and Actor attribute laws over the maintained DSL release.
//! Caller-supplied component facts never establish complete build coverage.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::{
        BoundedInteger, FiniteQuantity, ModifierDefId, ParameterSlotDefId, StatDefId, UnitDefId,
    },
    owned_rules::{
        ContributionKind, DefinitionRules, RuleEffectKind, RuleEntity, RuleExpression, RuleProgram,
        RuleReadSource, RuleRounding,
    },
    owned_schema::{
        ComputedValueType, DefinitionSchemaIndex, RuleEntityKind, SchemaClosure,
        SchemaDefinitionId, SchemaLookup, SchemaSubject, UnitDimension,
    },
};
use poe_optimizer_engine::owned_rules::{
    EffectDisposition, NumericalFailure, RuleFact, RuleScratch,
};
use poe_optimizer_import::owned_recipe::CheckedOwnedRecipe;
use serde_json::Value;
use std::{collections::BTreeSet, sync::OnceLock};

struct Bindings {
    attributes: Value,
    rarity_chaos: Value,
    defence: Value,
    actor_attributes: Value,
}
fn bindings() -> &'static Bindings {
    static BINDINGS: OnceLock<Bindings> = OnceLock::new();
    BINDINGS.get_or_init(|| {
        macro_rules! load {
            ($name:literal) => {
                serde_json::from_slice(include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../data/owned/poe2/3887ae68/",
                    $name,
                    "/bindings.json"
                )))
                .unwrap()
            };
        }
        Bindings {
            attributes: load!("item-attribute-inputs"),
            rarity_chaos: load!("item-rarity-chaos-inputs"),
            defence: load!("item-defence-inputs"),
            actor_attributes: load!("actor-attribute-inputs"),
        }
    })
}
fn subject(family: &Value, field: &str) -> SchemaSubject {
    let modifier: ModifierDefId = serde_json::from_value(family[field].clone()).unwrap();
    SchemaSubject::Definition(modifier.address())
}
fn owner<'a>(recipe: &'a CheckedOwnedRecipe, subject: &SchemaSubject) -> &'a DefinitionRules {
    recipe
        .rules()
        .input()
        .owners
        .iter()
        .find(|row| &row.owner == subject)
        .unwrap()
}
fn program<'a>(owner: &'a DefinitionRules, name: &str) -> &'a RuleProgram {
    owner
        .programs
        .members
        .iter()
        .find(|program| program.id.as_str() == name)
        .unwrap()
}
fn quantity(program: &RuleProgram, name: &str, amount: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|r| r.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("quantity input {name}")
    };
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(amount, unit.clone()).unwrap()),
    }
}
fn evaluate(
    recipe: &CheckedOwnedRecipe,
    owner: &DefinitionRules,
    program: &RuleProgram,
    facts: &[RuleFact],
    scratch: &mut RuleScratch,
) -> Vec<EffectDisposition> {
    let result = recipe
        .compiled_rules()
        .evaluate(
            &owner.owner,
            &program.id,
            facts,
            recipe.schema().as_ref(),
            scratch,
        )
        .unwrap();
    assert_eq!(result.owner_programs_closure, owner.programs.closure);
    assert_eq!(result.effects.len(), program.effects.len());
    result
        .effects
        .into_iter()
        .map(|effect| effect.disposition)
        .collect()
}
fn applied(disposition: &EffectDisposition) -> &ParameterValue {
    let EffectDisposition::Applied { value } = disposition else {
        panic!("expected applied component: {disposition:?}")
    };
    value
}
fn assert_missing(results: &[EffectDisposition], expected: &str) {
    assert!(!results.is_empty());
    assert!(
        results.iter().all(|result| {
            matches!(result, EffectDisposition::Unresolved { input } if input.as_str() == expected)
        }),
        "expected missing {expected}: {results:?}"
    );
}

const POSITIVE: &[(f64, f64, f64, f64)] = &[
    (0.0, 1.0, 1.0, 0.0),
    (10.0, 1.0, 1.0, 10.0),
    (10.0, 2.0, 1.5, 30.0),
    (10.49, 1.0, 1.0, 10.0),
    (10.5, 1.0, 1.0, 11.0),
    (3.0, 1.0, 1.2, 3.0),
    (10.0, 1.0, 1.0, 10.0),
];
const REDUCED: &[(f64, f64, f64, f64)] = &[
    (0.0, 1.0, 1.0, 0.0),
    (10.0, 1.0, 1.0, -10.0),
    (10.0, 2.0, 1.5, -30.0),
    (10.5, 1.0, 1.0, -11.0),
    (3.0, 1.0, 1.2, -3.0),
];
const SIGNED: &[(f64, f64, f64, f64)] = &[(-10.0, 2.0, 1.5, -28.0), (-10.5, 1.0, 1.0, -11.0)];

fn check_scalar_family(
    recipe: &CheckedOwnedRecipe,
    family: &Value,
    unit: &UnitDefId,
    stat: &StatDefId,
    scratch: &mut RuleScratch,
) {
    let subject = subject(family, "canonical");
    let owner = owner(recipe, &subject);
    assert!(matches!(
        owner.programs.closure,
        SchemaClosure::Partial { .. }
    ));
    // Later Actor projections are valid additional programs on these owners.
    // Only these four scalar components promise Modifier-local outputs.
    for name in [
        "catalyst-scalar",
        "ordered-magnitude-scalar",
        "corrupted-base-factor",
        "effective-amount",
    ] {
        let program = program(owner, name);
        assert_eq!(program.effects.len(), 1);
        assert!(matches!(
            &program.effects[0].effect,
            RuleEffectKind::Derive {
                entity: RuleEntity::Modifier,
                ..
            }
        ));
        assert!(matches!(
            &evaluate(recipe, owner, program, &[], scratch)[0],
            EffectDisposition::Unresolved { .. }
        ));
    }
    let effective = program(owner, "effective-amount");
    assert!(matches!(&effective.effects[0].effect,
        RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat: output, .. } if output == stat));
    let negative = if family["negative_input"].is_null() {
        None
    } else {
        let slot: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(family["negative_input"].clone()).unwrap();
        let read = effective.reads.iter().find(|read| {
            matches!(&read.source, RuleReadSource::Parameter { slot: current } if current == &slot)
        }).unwrap();
        assert_eq!(read.value_type, ComputedValueType::Boolean);
        Some(read.id.clone())
    };
    for (cases, reduced) in [
        (POSITIVE, false),
        (if negative.is_some() { REDUCED } else { SIGNED }, true),
    ] {
        for &(raw, corruption, magnitude, expected) in cases {
            let mut facts = vec![
                quantity(effective, "component", raw),
                quantity(effective, "corruption-factor", corruption),
                quantity(effective, "magnitude-factor", magnitude),
            ];
            if let Some(read) = &negative {
                facts.push(RuleFact {
                    read: read.clone(),
                    value: ParameterValue::Boolean(reduced),
                });
            }
            assert_eq!(effective.reads.len(), facts.len());
            let result = evaluate(recipe, owner, effective, &facts, scratch);
            let ParameterValue::Quantity(value) = applied(&result[0]) else {
                panic!("effective scalar quantity")
            };
            assert_eq!(value.unit(), unit);
            assert_eq!(
                value.value(),
                expected,
                "{}: {raw}/{corruption}/{magnitude}",
                family["family"]
            );
            for omitted in 0..facts.len() {
                let incomplete: Vec<_> = facts
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != omitted)
                    .map(|(_, fact)| fact.clone())
                    .collect();
                assert_missing(
                    &evaluate(recipe, owner, effective, &incomplete, scratch),
                    facts[omitted].read.as_str(),
                );
            }
            // The same scratch must recover after every missing-input attempt.
            assert_eq!(evaluate(recipe, owner, effective, &facts, scratch), result);
        }
    }
}

fn check_scalar_families(recipe: &CheckedOwnedRecipe) {
    let b = bindings();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let attributes = b.attributes["families"].as_array().unwrap();
    assert_eq!(attributes.len(), 4);
    assert_eq!(
        attributes
            .iter()
            .map(|f| f["family"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        ["strength", "dexterity", "intelligence", "all-attributes"]
            .into_iter()
            .collect()
    );
    let unit = serde_json::from_value(b.attributes["attribute_unit"].clone()).unwrap();
    let stat = serde_json::from_value(b.attributes["attribute_stat"].clone()).unwrap();
    for family in attributes {
        assert!(family["negative_input"].is_null());
        check_scalar_family(recipe, family, &unit, &stat, &mut scratch);
    }
    let rarity_chaos = b.rarity_chaos["families"].as_array().unwrap();
    assert_eq!(rarity_chaos.len(), 2);
    assert_eq!(
        rarity_chaos
            .iter()
            .map(|f| f["family"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        ["item-rarity", "chaos-resistance"].into_iter().collect()
    );
    let unit = serde_json::from_value(b.rarity_chaos["scalar_unit"].clone()).unwrap();
    let stat = serde_json::from_value(b.rarity_chaos["effective_stat"].clone()).unwrap();
    for family in rarity_chaos {
        assert_eq!(
            !family["negative_input"].is_null(),
            family["family"] == "item-rarity"
        );
        check_scalar_family(recipe, family, &unit, &stat, &mut scratch);
    }
    // These are the fifteen canonical families covered by the former shared
    // helper. The four separate historical defence families retain their tests.
    let names = [
        "armour-base",
        "evasion-base",
        "energy-shield-base",
        "ward-base",
        "evasion-energy-shield-base",
        "block-chance-base",
        "armour-increase",
        "evasion-increase",
        "energy-shield-increase",
        "ward-increase",
        "armour-evasion-increase",
        "armour-energy-shield-increase",
        "evasion-energy-shield-increase",
        "triple-defence-increase",
        "block-chance-increase",
    ];
    let families = b.defence["families"].as_array().unwrap();
    let mut tested = BTreeSet::new();
    for name in names {
        let matching: Vec<_> = families.iter().filter(|f| f["family"] == name).collect();
        assert_eq!(matching.len(), 1, "{name}");
        let family = matching[0];
        assert_eq!(
            !family["negative_input"].is_null(),
            name.ends_with("-increase")
        );
        let (expected_unit, expected_stat) =
            if name.ends_with("-increase") || name == "block-chance-base" {
                (
                    &b.rarity_chaos["scalar_unit"],
                    &b.rarity_chaos["effective_stat"],
                )
            } else {
                (
                    &b.attributes["attribute_unit"],
                    &b.attributes["attribute_stat"],
                )
            };
        assert_eq!(&family["unit"], expected_unit);
        assert_eq!(&family["output"], expected_stat);
        let unit = serde_json::from_value(expected_unit.clone()).unwrap();
        let stat = serde_json::from_value(expected_stat.clone()).unwrap();
        check_scalar_family(recipe, family, &unit, &stat, &mut scratch);
        assert!(tested.insert(name));
    }
    assert_eq!(tested.len(), 15);
}

fn expression<'a>(
    program: &'a RuleProgram,
    id: &poe_optimizer_core::owned_definitions::OwnedDefinitionKey,
) -> &'a RuleExpression {
    &program
        .nodes
        .iter()
        .find(|node| &node.id == id)
        .unwrap()
        .expression
}
fn integral_formatter_tail(
    program: &RuleProgram,
    unit: &UnitDefId,
    value: &poe_optimizer_core::owned_definitions::OwnedDefinitionKey,
) {
    // Follow semantic edges rather than historical SSA names or declaration order.
    let RuleExpression::DivideFactor { value, divisor } = expression(program, value) else {
        panic!("formatter division tail")
    };
    assert!(
        matches!(expression(program, divisor), RuleExpression::Literal {
        value: ParameterValue::Quantity(factor)
    } if factor.value() == 1.0 && factor.unit().key().as_str() == "def.0000000000000001")
    );
    let RuleExpression::Select {
        when_true,
        when_false,
        ..
    } = expression(program, value)
    else {
        panic!("signed formatter rounding")
    };
    for (branch, rounding) in [
        (when_true, RuleRounding::Ceiling),
        (when_false, RuleRounding::Floor),
    ] {
        assert!(
            matches!(expression(program, branch), RuleExpression::Round { quantum, mode, .. }
            if quantum.value() == 1.0 && quantum.unit() == unit && mode == &rounding)
        );
    }
}

fn check_actor_attributes(recipe: &CheckedOwnedRecipe) {
    let b = &bindings().actor_attributes;
    let unit: UnitDefId = serde_json::from_value(b["effective_unit"].clone()).unwrap();
    let stat: StatDefId = serde_json::from_value(b["effective_stat"].clone()).unwrap();
    let SchemaLookup::Known(unit_schema) = recipe.schema().definition(&unit) else {
        panic!("Count unit")
    };
    assert_eq!(unit_schema.dimension, UnitDimension::Count);
    let SchemaLookup::Known(stat_schema) = recipe.schema().definition(&stat) else {
        panic!("Modifier output")
    };
    assert_eq!(stat_schema.targets, [RuleEntityKind::Modifier]);
    assert_eq!(
        stat_schema.value,
        ComputedValueType::Quantity { unit: unit.clone() }
    );
    let expected_channels: [(&str, &[&str]); 4] = [
        (
            "strength",
            &["def.000000000000331b", "def.000000000000331e"],
        ),
        (
            "dexterity",
            &["def.000000000000331c", "def.000000000000331f"],
        ),
        (
            "intelligence",
            &["def.000000000000331d", "def.0000000000003320"],
        ),
        (
            "all-attributes",
            &[
                "def.000000000000331b",
                "def.000000000000331e",
                "def.000000000000331c",
                "def.000000000000331f",
                "def.000000000000331d",
                "def.0000000000003320",
            ],
        ),
    ];
    let families = b["families"].as_array().unwrap();
    assert_eq!(families.len(), expected_channels.len());
    let mut scratch = recipe.compiled_rules().new_scratch();
    for (family, (name, expected)) in families.iter().zip(expected_channels) {
        assert_eq!(family["family"], name);
        let subject = subject(family, "modifier");
        let owner = owner(recipe, &subject);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        let formatter = program(owner, family["formatter_program"].as_str().unwrap());
        let projection = program(owner, family["projection_program"].as_str().unwrap());
        assert_eq!(formatter.effects.len(), 1);
        assert_eq!(
            formatter.effects[0].id.as_str(),
            family["formatter_effect"].as_str().unwrap()
        );
        assert!(formatter.effects[0].when.is_none());
        let RuleEffectKind::Derive {
            entity: RuleEntity::Modifier,
            stat: output,
            value,
        } = &formatter.effects[0].effect
        else {
            panic!("Modifier formatter output")
        };
        assert_eq!(output, &stat);
        integral_formatter_tail(formatter, &unit, value);
        assert_eq!(projection.context, RuleEntityKind::EquipmentUse);
        assert_eq!(projection.reads.len(), 1);
        assert_eq!(projection.reads[0].id.as_str(), "effective");
        assert_eq!(projection.reads[0].value_type, stat_schema.value);
        assert!(matches!(&projection.reads[0].source,
            RuleReadSource::Stat { entity: RuleEntity::Modifier, stat: input } if input == &stat));
        // The maintained release feeds both attribute calculation passes with
        // Count quantities; the old binding's Integer receivers were retired.
        let channels: Vec<_> = projection
            .effects
            .iter()
            .map(|effect| {
                assert!(effect.when.is_none());
                let RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat,
                    contribution: ContributionKind::Add,
                    ..
                } = &effect.effect
                else {
                    panic!("Player attribute contribution")
                };
                stat
            })
            .collect();
        assert_eq!(
            channels
                .iter()
                .map(|id| id.key().as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            channels.iter().collect::<BTreeSet<_>>().len(),
            channels.len()
        );
        assert_eq!(projection.effects.len(), expected.len());
        for (effect, channel) in projection.effects.iter().zip(&channels) {
            let SchemaLookup::Known(schema) = recipe.schema().definition(*channel) else {
                panic!("Actor channel")
            };
            assert_eq!(schema.targets, [RuleEntityKind::Actor]);
            assert_eq!(
                schema.value,
                ComputedValueType::Quantity { unit: unit.clone() }
            );
            assert!(effect.when.is_none());
            let RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: output,
                contribution: ContributionKind::Add,
                value,
            } = &effect.effect
            else {
                panic!("Player attribute contribution")
            };
            assert_eq!(output, *channel);
            let RuleExpression::ScaleInteger { value, count } = expression(projection, value)
            else {
                panic!("checked integral Count contribution")
            };
            assert!(
                matches!(expression(projection, value), RuleExpression::Literal {
                value: ParameterValue::Quantity(quantum)
            } if quantum.value() == 1.0 && quantum.unit() == &unit)
            );
            let RuleExpression::QuantizeInteger {
                value,
                quantum,
                mode: RuleRounding::Floor,
            } = expression(projection, count)
            else {
                panic!("integer projection")
            };
            assert_eq!(quantum.value(), 1.0);
            assert_eq!(quantum.unit(), &unit);
            assert!(
                matches!(expression(projection, value), RuleExpression::Read { input } if input == &projection.reads[0].id)
            );
        }
        for (raw, corruption, magnitude, expected) in [
            (0.0, 1.0, 1.0, 0),
            (27.0, 1.0, 1.0, 27),
            (-27.0, 1.0, 1.0, -27),
            (10.49, 1.0, 1.0, 10),
            (10.5, 1.0, 1.0, 11),
            (-10.5, 1.0, 1.0, -11),
            (10.0, 2.0, 1.5, 30),
            (-10.0, 2.0, 1.5, -28),
            (3.0, 1.0, 1.2, 3),
            (1_000_000.0, 1.0, 1.0, 1_000_000),
            (0.0, 1.0, 1.0, 0),
        ] {
            let facts = [
                quantity(formatter, "component", raw),
                quantity(formatter, "corruption-factor", corruption),
                quantity(formatter, "magnitude-factor", magnitude),
            ];
            assert_eq!(formatter.reads.len(), facts.len());
            let formatted = evaluate(recipe, owner, formatter, &facts, &mut scratch);
            let ParameterValue::Quantity(amount) = applied(&formatted[0]) else {
                panic!("formatted Count")
            };
            assert_eq!(amount.unit(), &unit);
            assert_eq!(amount.value(), f64::from(expected));
            assert_eq!(amount.value().fract(), 0.0);
            let projected = evaluate(
                recipe,
                owner,
                projection,
                &[RuleFact {
                    read: projection.reads[0].id.clone(),
                    value: ParameterValue::Quantity(amount.clone()),
                }],
                &mut scratch,
            );
            let expected = EffectDisposition::Applied {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(f64::from(expected), unit.clone()).unwrap(),
                ),
            };
            assert!(projected.iter().all(|value| value == &expected));
            assert_missing(
                &evaluate(recipe, owner, projection, &[], &mut scratch),
                "effective",
            );
        }
        for exact in [BoundedInteger::MIN, BoundedInteger::MAX] {
            let projected = evaluate(
                recipe,
                owner,
                projection,
                &[quantity(projection, "effective", exact as f64)],
                &mut scratch,
            );
            assert!(projected.iter().all(|value| value
                == &EffectDisposition::Applied {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(exact as f64, unit.clone()).unwrap()
                    ),
                }));
        }
        assert_missing(
            &evaluate(recipe, owner, projection, &[], &mut scratch),
            "effective",
        );
        let wrong_unit: UnitDefId =
            serde_json::from_value(bindings().rarity_chaos["scalar_unit"].clone()).unwrap();
        assert_ne!(wrong_unit, unit);
        for value in [
            ParameterValue::Boolean(true),
            ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
            ParameterValue::Quantity(FiniteQuantity::new(2.0, wrong_unit).unwrap()),
        ] {
            assert!(
                recipe
                    .compiled_rules()
                    .evaluate(
                        &subject,
                        &projection.id,
                        &[RuleFact {
                            read: projection.reads[0].id.clone(),
                            value
                        }],
                        recipe.schema().as_ref(),
                        &mut scratch
                    )
                    .is_err()
            );
        }
        for overflow in [
            BoundedInteger::MAX as f64 + 1.0,
            BoundedInteger::MIN as f64 - 1.0,
        ] {
            let result = evaluate(
                recipe,
                owner,
                projection,
                &[quantity(projection, "effective", overflow)],
                &mut scratch,
            );
            assert!(result.iter().all(|effect| matches!(effect,
                EffectDisposition::NumericalError { node, reason: NumericalFailure::IntegerOverflow }
                if matches!(expression(projection, node), RuleExpression::QuantizeInteger { .. }))));
        }
        // A failed projection cannot poison the next valid occurrence.
        let recovered = evaluate(
            recipe,
            owner,
            projection,
            &[quantity(projection, "effective", 27.0)],
            &mut scratch,
        );
        assert!(recovered.iter().all(|value| value
            == &EffectDisposition::Applied {
                value: ParameterValue::Quantity(FiniteQuantity::new(27.0, unit.clone()).unwrap()),
            }));
    }
}

#[test]
fn tracked_scalar_families_preserve_units_rounding_missing_inputs_and_worker_scratch() {
    let recipe = super::tracked_release().release().assembled().checked();
    check_scalar_families(recipe);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2)
            .map(|_| scope.spawn(|| check_scalar_families(recipe)))
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

#[test]
fn tracked_actor_attributes_preserve_exact_channels_integer_bounds_and_worker_scratch() {
    let recipe = super::tracked_release().release().assembled().checked();
    check_actor_attributes(recipe);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2)
            .map(|_| scope.spawn(|| check_actor_attributes(recipe)))
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}
