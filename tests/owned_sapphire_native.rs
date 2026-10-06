//! Actual checked ring inputs and compiled cold programs in a finite topology.
//! This does not supply a final resistance contribution or a complete build.
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_sapphire_native_fixture.rs"]
mod shared;

use poe_optimizer_core::{owned_build::*, owned_definitions::FiniteQuantity, owned_rules::*};
use poe_optimizer_engine::owned_plan::*;
use shared::component;
use shared::{Fixture, Inputs, occurrence};

fn set_parameter(f: &mut Fixture, suffix: u64, value: ParameterValue) {
    f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot.slot.key().as_str() == format!("def.{suffix:016x}"))
        .unwrap()
        .value = value;
}
fn amount(f: &mut Fixture, value: f64) {
    let unit = f.family.unit.clone();
    set_parameter(
        f,
        0x09fa,
        ParameterValue::Quantity(FiniteQuantity::new(value, unit).unwrap()),
    );
}
fn kind(f: &mut Fixture, property: &str) {
    let owner = f.family_owner_mut();
    let program = owner
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "catalyst-scalar")
        .unwrap();
    let RuleExpression::Literal { value } = &program
        .nodes
        .iter()
        .find(|n| n.id.as_str() == format!("option-{property}"))
        .unwrap()
        .expression
    else {
        panic!()
    };
    let value = value.clone();
    set_parameter(f, 0x09f9, value);
}
fn evaluate(f: &Fixture, expected: f64) -> OwnedEffectsReport {
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for equipment in [6, 7] {
        assert_eq!(f.effective(&report, equipment, 4), &f.expected(expected));
    }
    assert!(
        !report
            .effects
            .iter()
            .any(|e| matches!(e.target, BoundEffectTarget::Contribution { .. })),
        "no final resistance contribution is authored"
    );
    assert!(
        !report.values.iter().any(|v| matches!(
            v.key,
            PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(_),
                ..
            }
        )),
        "no final resistance aggregate is supplied"
    );
    report
}

#[test]
#[ignore = "requires verified Sapphire endpoint and original-05 normalized inputs"]
fn exact_ring_inputs_feed_actual_cold_programs_for_two_receiving_uses() {
    let inputs = Inputs::load();
    let f = inputs.fixture(None);
    evaluate(&f, 25.0);
    for (property, percentage, expected) in [
        ("cold", 20.0, 30.0),
        ("cold", 0.0, 25.0),
        ("cold", 2.0, 25.0),
        ("cold", 4.0, 26.0),
        ("fire", 20.0, 25.0),
    ] {
        let mut f = inputs.fixture(None);
        kind(&mut f, property);
        amount(&mut f, percentage);
        evaluate(&f, expected);
    }
    for (raw, expected) in [(20.0, 20.0), (25.0, 25.0), (25.5, 26.0), (30.0, 30.0)] {
        let mut f = inputs.fixture(None);
        f.set_raw(0, 0, raw);
        evaluate(&f, expected);
    }
    let mut f = inputs.fixture(None);
    let factor = f.family.corrupted_base.clone();
    let unit = f.family.factor_unit.clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot == factor)
        .unwrap()
        .value = ParameterValue::Quantity(FiniteQuantity::new(1.5, unit).unwrap());
    // The numeric factor is supported independently of the Import profile,
    // which still rejects the source's corruptedRange construction.
    evaluate(&f, 38.0);
}

#[test]
#[ignore = "requires verified Sapphire endpoint and original-05 normalized inputs"]
fn shared_physical_edits_category_targeting_parallel_and_reused_scratch_are_isolated() {
    let inputs = Inputs::load();
    let mut f = inputs.fixture(None);
    let first = f.plan().unwrap();
    let before = first.evaluate(&mut first.new_scratch()).unwrap();
    kind(&mut f, "cold");
    let second = f.plan().unwrap();
    let after = second.evaluate(&mut second.new_scratch()).unwrap();
    assert_ne!(first.identity(), second.identity());
    for equipment in [6, 7] {
        assert_eq!(f.effective(&before, equipment, 4), &f.expected(25.0));
        assert_eq!(f.effective(&after, equipment, 4), &f.expected(30.0));
    }
    let mut scratch = first.new_scratch();
    for _ in 0..3 {
        assert_eq!(first.evaluate(&mut scratch).unwrap(), before);
        assert_eq!(second.evaluate(&mut scratch).unwrap(), after);
    }
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let (first, second, before, after) = (&first, &second, &before, &after);
            scope.spawn(move || {
                let mut scratch = first.new_scratch();
                assert_eq!(first.evaluate(&mut scratch).unwrap(), *before);
                assert_eq!(second.evaluate(&mut scratch).unwrap(), *after);
            });
        }
    });
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let third = f.plan().unwrap();
    let report = third.evaluate(&mut third.new_scratch()).unwrap();
    assert_eq!(f.effective(&report, 6, 4), &f.expected(30.0));
    assert!(!report.effects.iter().any(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if matches!(provider.root, ProviderRoot::EquipmentUse(use_id) | ProviderRoot::ItemModifier { equipment_use: use_id, .. } if use_id == occurrence(7)))));
    let categories = component::categories::bindings();
    for (target, expected) in [(categories.implicit, 37.0), (categories.explicit, 25.0)] {
        let f = inputs.fixture(Some(target));
        // The category producer is explicitly synthetic; the numeric consumer
        // is the unchanged authored ordered-magnitude-scalar program.
        evaluate(&f, expected);
    }
}

#[test]
#[ignore = "requires verified Sapphire endpoint and original-05 normalized inputs"]
fn missing_inputs_and_producers_remain_errors_or_unknown_values() {
    let inputs = Inputs::load();
    for slot in [0x2543, 0x2674, 0x316d] {
        let mut f = inputs.fixture(None);
        f.build.items[0].modifiers[0]
            .rolls
            .retain(|p| p.slot.slot.key().as_str() != format!("def.{slot:016x}"));
        assert!(
            matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    for slot in [0x09f9, 0x09fa] {
        let mut f = inputs.fixture(None);
        f.build.items[0]
            .parameters
            .retain(|p| p.slot.slot.key().as_str() != format!("def.{slot:016x}"));
        assert!(
            matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    let mut f = inputs.fixture(None);
    let raw = f.family.amount.clone();
    let wrong_unit = f.family.factor_unit.clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot == raw)
        .unwrap()
        .value = ParameterValue::Quantity(FiniteQuantity::new(25.0, wrong_unit).unwrap());
    assert!(
        matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
    for program in [
        "catalyst-inputs",
        "corrupted-base-factor",
        "ordered-magnitude-scalar",
    ] {
        let mut f = inputs.fixture(None);
        for owner in &mut f.recipe.rules.owners {
            owner.programs.members.retain(|p| p.id.as_str() != program);
        }
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        for equipment in [6, 7] {
            assert!(matches!(
                f.effective(&report, equipment, 4),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    let mut f = inputs.fixture(None);
    f.restore_partial_rules();
    f.restore_template_rules();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        !report.gaps.is_empty(),
        "actual owner incompleteness is not a complete build"
    );
}
