//! A finite native category boundary, separate from whole-build source coverage.
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod native;
use native::{Fixture, categories as category};
use poe_optimizer_core::owned_build::ParameterValue;
use poe_optimizer_engine::owned_plan::{EffectValue, PlanError};

#[test]
fn real_category_options_gate_actual_ordered_magnitude_and_numeric_recipes() {
    let binding = category::bindings();
    for target in [&binding.explicit, &binding.implicit, &binding.enchant] {
        let mut fixture = Fixture::with_categories(target.clone());
        fixture.complete_domain();
        let plan = fixture.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        for (equipment, modifier, actual) in [
            (6, 4, &binding.explicit),
            (6, 5, &binding.implicit),
            (7, 4, &binding.explicit),
            (7, 5, &binding.implicit),
            (32, 31, &binding.enchant),
        ] {
            // The test supplies only an explicit bounded +0.5 factor producer.
            // The actual authored fold and compiled numeric recipe produce the
            // source-observed Count7 (matching) or Count5 (nonmatching) output.
            assert_eq!(
                fixture.effective(&report, equipment, modifier),
                &fixture.expected(if actual == target { 7.0 } else { 5.0 })
            );
        }
        assert_eq!(
            fixture.total(&report),
            &fixture.expected(if target == &binding.enchant {
                27.0
            } else {
                29.0
            })
        );
    }
}

#[test]
fn category_changes_rebind_the_exact_modifier_without_reusing_old_scratch_values() {
    let binding = category::bindings();
    let mut fixture = Fixture::with_categories(binding.explicit.clone());
    fixture.complete_domain();
    let original = fixture.plan().unwrap();
    let mut scratch = original.new_scratch();
    let before = original.evaluate(&mut scratch).unwrap();
    assert_eq!(fixture.effective(&before, 6, 4), &fixture.expected(7.0));
    fixture.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot == binding.slot)
        .unwrap()
        .value = ParameterValue::Option(binding.enchant);
    let changed = fixture.plan().unwrap();
    assert_ne!(original.identity(), changed.identity());
    let after = changed.evaluate(&mut scratch).unwrap();
    assert_eq!(fixture.effective(&after, 6, 4), &fixture.expected(5.0));
    assert_eq!(fixture.effective(&after, 7, 4), &fixture.expected(5.0));
    assert_eq!(fixture.effective(&after, 6, 5), &fixture.expected(5.0));
    assert_eq!(fixture.effective(&after, 32, 31), &fixture.expected(5.0));
    assert_eq!(original.evaluate(&mut scratch).unwrap(), before);
}

#[test]
fn missing_required_category_is_invalid_and_real_partial_owner_stays_unresolved() {
    let binding = category::bindings();
    let mut missing = Fixture::with_categories(binding.explicit.clone());
    missing.complete_domain();
    missing.build.items[0].modifiers[0]
        .rolls
        .retain(|p| p.slot != binding.slot);
    assert!(
        matches!(missing.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
    let mut partial = Fixture::with_categories(binding.explicit);
    partial.complete_domain();
    partial.restore_partial_rules();
    let plan = partial.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(!report.gaps.is_empty());
    assert!(matches!(
        partial.total(&report),
        EffectValue::Unresolved { .. }
    ));
}
