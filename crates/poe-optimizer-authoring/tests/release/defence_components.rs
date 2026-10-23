//! Decimal and integral defence components use the maintained DSL release.
//! Caller facts exercise component laws without establishing build coverage.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::{FiniteQuantity, ModifierDefId, ParameterSlotDefId, StatDefId, UnitDefId},
    owned_rules::{DefinitionRules, RuleEffectKind, RuleEntity, RuleProgram, RuleReadSource},
    owned_schema::{
        ComputedValueType, DefinitionSchemaIndex, RuleEntityKind, SchemaClosure,
        SchemaDefinitionId, SchemaLookup, SchemaSubject, UnitDimension,
    },
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact, RuleScratch};
use poe_optimizer_import::owned_recipe::CheckedOwnedRecipe;
use serde_json::Value;
use std::sync::OnceLock;

fn bindings() -> &'static Value {
    static BINDINGS: OnceLock<Value> = OnceLock::new();
    BINDINGS.get_or_init(|| {
        serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/owned/poe2/3887ae68/item-defence-inputs/bindings.json"
        )))
        .unwrap()
    })
}

fn number(program: &RuleProgram, name: &str, value: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|read| read.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("quantity input {name}")
    };
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap()),
    }
}

fn boolean(program: &RuleProgram, slot: &Value, value: bool) -> RuleFact {
    let slot: DeclaredSlot<ParameterSlotDefId> = serde_json::from_value(slot.clone()).unwrap();
    let read = program.reads.iter().find(|read| {
        matches!(&read.source, RuleReadSource::Parameter { slot: actual } if actual == &slot)
    }).unwrap();
    assert_eq!(read.value_type, ComputedValueType::Boolean);
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Boolean(value),
    }
}

fn evaluate(
    recipe: &CheckedOwnedRecipe,
    owner: &DefinitionRules,
    program: &RuleProgram,
    facts: &[RuleFact],
    scratch: &mut RuleScratch,
) -> EffectDisposition {
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
    assert_eq!(result.effects.len(), 1);
    result.effects[0].disposition.clone()
}

fn assert_amount(result: &EffectDisposition, expected: f64, unit: &UnitDefId) {
    let EffectDisposition::Applied {
        value: ParameterValue::Quantity(value),
    } = result
    else {
        panic!("expected defence component {expected}: {result:?}")
    };
    assert_eq!(value.unit(), unit);
    assert_eq!(value.value(), expected);
}

// Reviewed ItemTools.applyRange/applyValueScalar expectations. Decimal lexical
// syntax remains distinct from numeric integrality; an explicit base rounds
// before scaling even at 1, while an absent base skips that step.
const CASES: &[(f64, bool, bool, f64, f64, f64)] = &[
    (0.0, false, false, 1.0, 1.0, 0.0),
    (10.0, false, false, 1.0, 1.0, 10.0),
    (10.5, true, false, 1.0, 1.0, 10.5),
    (10.5, true, true, 1.0, 1.0, 10.5),
    (10.5, true, false, 99.0, 1.0, 10.5),
    (0.15, true, false, 1.0, 1.5, 0.2),
    (0.15, true, true, 1.0, 1.5, 0.3),
    (3.0, false, false, 1.0, 1.25, 3.0),
    (3.0, true, false, 1.0, 1.25, 3.7),
    (10.0, false, false, 1.0, 1.29995, 13.0),
    (10.0, true, false, 1.0, 1.29995, 12.9),
    (1.0, false, true, 1.49, 2.0, 2.0),
    (1.0, false, true, 1.5, 2.0, 4.0),
    (3.0, false, true, 2.0, 1.5, 9.0),
    (10.0, false, false, 1.0, 1.0, 10.0),
];

fn check_defence_components(recipe: &CheckedOwnedRecipe) {
    let families = bindings()["families"].as_array().unwrap();
    let mut scratch = recipe.compiled_rules().new_scratch();
    for name in [
        "armour-evasion-base",
        "armour-energy-shield-base",
        "triple-defence-base",
        "defences-increase",
    ] {
        let matching: Vec<_> = families
            .iter()
            .filter(|family| family["family"] == name)
            .collect();
        assert_eq!(matching.len(), 1, "{name}");
        let binding = matching[0];
        let increase = name == "defences-increase";
        assert_eq!(!binding["negative_input"].is_null(), increase);
        let modifier: ModifierDefId = serde_json::from_value(binding["canonical"].clone()).unwrap();
        let subject = SchemaSubject::Definition(modifier.address());
        let owner = recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|owner| owner.owner == subject)
            .unwrap();
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        // Other current programs deliver local evasion contributions. Only
        // these four scalar components promise Modifier-local derivation.
        for component in [
            "catalyst-scalar",
            "ordered-magnitude-scalar",
            "corrupted-base-factor",
            "effective-amount",
        ] {
            let program = owner
                .programs
                .members
                .iter()
                .find(|p| p.id.as_str() == component)
                .unwrap();
            assert_eq!(program.context, RuleEntityKind::EquipmentUse);
            assert_eq!(program.effects.len(), 1);
            assert!(matches!(
                &program.effects[0].effect,
                RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    ..
                }
            ));
            assert!(matches!(
                evaluate(recipe, owner, program, &[], &mut scratch),
                EffectDisposition::Unresolved { .. }
            ));
        }
        let program = owner
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "effective-amount")
            .unwrap();
        let output: StatDefId = serde_json::from_value(binding["output"].clone()).unwrap();
        let unit: UnitDefId = serde_json::from_value(binding["unit"].clone()).unwrap();
        let (expected_unit, expected_stat, dimension) = if increase {
            (
                "def.0000000000000002",
                "def.000000000000253e",
                UnitDimension::PercentagePoints,
            )
        } else {
            (
                "def.000000000000295a",
                "def.000000000000295b",
                UnitDimension::Count,
            )
        };
        assert_eq!(unit.key().as_str(), expected_unit);
        assert_eq!(output.key().as_str(), expected_stat);
        let SchemaLookup::Known(unit_schema) = recipe.schema().definition(&unit) else {
            panic!("declared defence unit")
        };
        assert_eq!(unit_schema.dimension, dimension);
        let SchemaLookup::Known(output_schema) = recipe.schema().definition(&output) else {
            panic!("declared Modifier output")
        };
        assert_eq!(output_schema.targets, [RuleEntityKind::Modifier]);
        assert_eq!(
            output_schema.value,
            ComputedValueType::Quantity { unit: unit.clone() }
        );
        assert!(matches!(&program.effects[0].effect,
            RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat, .. } if stat == &output));
        assert!(program.effects[0].when.is_none());
        for &(raw, decimal, base_present, base, magnitude, expected) in CASES {
            for negative in [false, true] {
                let component = if negative && !increase { -raw } else { raw };
                let expected = if negative { -expected } else { expected };
                let mut facts = vec![
                    number(program, "component", component),
                    boolean(program, &binding["decimal_quantization_input"], decimal),
                    boolean(
                        program,
                        &binding["base_rounding_present_input"],
                        base_present,
                    ),
                    number(program, "corruption-factor", base),
                    number(program, "magnitude-factor", magnitude),
                ];
                if increase {
                    facts.push(boolean(program, &binding["negative_input"], negative));
                }
                assert_eq!(program.reads.len(), facts.len());
                let complete = evaluate(recipe, owner, program, &facts, &mut scratch);
                assert_amount(&complete, expected, &unit);
                for omitted in 0..facts.len() {
                    let reduced: Vec<_> = facts
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index != omitted)
                        .map(|(_, fact)| fact.clone())
                        .collect();
                    let result = evaluate(recipe, owner, program, &reduced, &mut scratch);
                    let unchanged = magnitude == 1.0 && (!base_present || base == 1.0);
                    let unused = (omitted == 1 && unchanged) || (omitted == 3 && !base_present);
                    if unused {
                        assert_amount(&result, expected, &unit);
                    } else {
                        assert!(
                            matches!(&result,
                            EffectDisposition::Unresolved { input } if input == &facts[omitted].read),
                            "{name}: missing {} produced {result:?}",
                            facts[omitted].read.as_str()
                        );
                    }
                    // Missing or unused facts cannot leak into the next run.
                    assert_eq!(
                        evaluate(recipe, owner, program, &facts, &mut scratch),
                        complete
                    );
                }
            }
        }
    }
}

#[test]
fn tracked_defence_components_preserve_decimal_rounding_and_lazy_input_authority() {
    let recipe = super::tracked_release().release().assembled().checked();
    check_defence_components(recipe);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2)
            .map(|_| scope.spawn(|| check_defence_components(recipe)))
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}
