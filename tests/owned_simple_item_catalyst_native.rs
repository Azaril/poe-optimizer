//! Actual template transport + Life programs; only the surrounding finite
//! contributor domain is synthetic. No whole-build Life coverage is asserted.
#[path = "support/owned_simple_item_catalyst_native.rs"]
mod native;
use native::{fixture, occurrence};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;

#[test]
fn actual_producers_transport_imported_absence_values_per_use_before_life_projection() {
    let mut f = fixture();
    f.set_raw(0, 0, 17.0);
    f.set_raw(1, 0, 16.0);
    for item in 0..2 {
        let ParameterValue::Option(none) = f.parameter(item, false) else {
            panic!()
        };
        assert_eq!(none.key().as_str(), "def.00000000000009eb");
        let ParameterValue::Quantity(amount) = f.parameter(item, true) else {
            panic!()
        };
        assert_eq!(amount.value(), 20.0);
        assert_eq!(amount.unit().key().as_str(), "def.0000000000000002");
    }
    // These are already the exact source-policy assignments. The Engine has
    // no knowledge of a missing raw header or a helper's fallback value.
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for (item, equipment, modifier, amount) in [(0, 6, 4, 17.0), (0, 7, 4, 17.0), (1, 32, 31, 16.0)]
    {
        for is_amount in [false, true] {
            assert_eq!(
                f.input(&report, equipment, is_amount),
                &EffectValue::Known {
                    value: f.parameter(item, is_amount).clone()
                }
            );
        }
        assert_eq!(
            f.factor(&report, equipment, modifier),
            &f.expected_factor(1.0)
        );
        assert_eq!(
            f.effective(&report, equipment, modifier),
            &f.expected(amount)
        );
        let contributions:Vec<_>=report.effects.iter().filter(|e|matches!((&e.key.invocation.origin,&e.target),
            (RuleOrigin::Provider{provider},BoundEffectTarget::Contribution{key}) if provider.root==ProviderRoot::ItemModifier{equipment_use:occurrence(equipment),modifier:occurrence(modifier)} && provider.grant_path.is_empty() && key.stat==f.family.contribution && key.kind==ContributionKind::Add && key.entity==ConcreteEntity::Actor(ActorKey::Player))).collect();
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].value, f.expected_total(amount));
    }
    assert!(!report.effects.iter().any(|e| {
        e.key
            .invocation
            .program
            .as_str()
            .starts_with("fixture-catalyst")
    }));
    assert_eq!(f.total(&report), &f.expected_total(50.0));
}

#[test]
fn zero_kind_only_amount_only_and_property_matching_keep_their_distinct_meanings() {
    let seed = fixture();
    let none = match seed.parameter(0, false) {
        ParameterValue::Option(v) => v.clone(),
        _ => panic!(),
    };
    let life = seed.option("life");
    let other = seed.option("mana");
    for (label, kind, amount, factor, effective) in [
        ("absent-kind-and-amount", none.clone(), 20.0, 1.0, 10.0),
        ("amount-only", none, 37.0, 1.0, 10.0),
        ("kind-only-default-amount", life.clone(), 20.0, 1.2, 12.0),
        ("explicit-zero", life.clone(), 0.0, 1.0, 10.0),
        ("mismatched-kind", other, 20.0, 1.0, 10.0),
    ] {
        let mut f = fixture();
        f.set_kind(0, kind);
        f.set_amount(0, amount);
        f.set_life_tag(0, true);
        f.complete_domain();
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.gaps.is_empty(), "{label}: {:?}", report.gaps);
        for equipment in [6, 7] {
            assert_eq!(
                f.input(&report, equipment, false),
                &EffectValue::Known {
                    value: f.parameter(0, false).clone()
                },
                "{label}"
            );
            assert_eq!(
                f.input(&report, equipment, true),
                &EffectValue::Known {
                    value: f.parameter(0, true).clone()
                },
                "{label}"
            );
            assert_eq!(
                f.factor(&report, equipment, 4),
                &f.expected_factor(factor),
                "{label}"
            );
            assert_eq!(
                f.effective(&report, equipment, 4),
                &f.expected(effective),
                "{label}"
            );
        }
        assert_eq!(f.factor(&report, 32, 31), &f.expected_factor(1.0));
        assert_eq!(f.total(&report), &f.expected_total(2.0 * effective + 10.0));
    }
    let mut f = fixture();
    f.set_kind(0, life);
    f.set_amount(0, 20.0);
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty());
    assert_eq!(
        f.factor(&report, 6, 4),
        &f.expected_factor(1.0),
        "English Life text does not manufacture a source property"
    );
    assert_eq!(f.effective(&report, 6, 4), &f.expected(10.0));
}

#[test]
fn exact_owners_and_equipment_uses_remain_isolated_across_changes_and_scratch_reuse() {
    let mut f = fixture();
    let kind = f.option("life");
    f.set_kind(0, kind);
    f.set_life_tag(0, true);
    f.set_amount(1, 37.0);
    f.complete_domain();
    let first = f.plan().unwrap();
    let before = first.evaluate(&mut first.new_scratch()).unwrap();
    assert_eq!(f.total(&before), &f.expected_total(34.0));
    assert_ne!(f.input(&before, 6, true), f.input(&before, 32, true));
    f.set_amount(0, 0.0);
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let second = f.plan().unwrap();
    let after = second.evaluate(&mut second.new_scratch()).unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_eq!(f.total(&after), &f.expected_total(20.0));
    assert_eq!(f.input(&after, 32, true), f.input(&before, 32, true));
    assert!(!after.effects.iter().any(|e|matches!(&e.key.invocation.origin,RuleOrigin::Provider{provider} if matches!(provider.root,ProviderRoot::EquipmentUse(id)|ProviderRoot::ItemModifier{equipment_use:id,..} if id==occurrence(7)))));
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
}

#[test]
fn missing_or_invalid_canonical_parameters_never_acquire_import_defaults_at_runtime() {
    for (item, amount) in [(0, false), (0, true), (1, false), (1, true)] {
        let mut f = fixture();
        f.complete_domain();
        let slot = if amount {
            f.bindings[item].amount.clone()
        } else {
            f.bindings[item].selection.clone()
        };
        f.build.items[item].parameters.retain(|p| p.slot != slot);
        assert!(
            matches!(f.plan(),Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    for item in 0..2 {
        let mut f = fixture();
        f.complete_domain();
        let own = f.bindings[item].selection.clone();
        let foreign = f.bindings[1 - item].selection.clone();
        f.build.items[item]
            .parameters
            .iter_mut()
            .find(|p| p.slot == own)
            .unwrap()
            .slot = foreign;
        assert_eq!(
            BuildSpec::new(f.build.clone(), OwnedInputLimits::default())
                .unwrap_err()
                .kind,
            StructuralErrorKind::WrongDeclaration
        );
    }
    let mut f = fixture();
    f.complete_domain();
    let unknown = OptionDefId::parse(
        f.family.modifier.namespace().clone(),
        "def.00000000000030e2",
    )
    .unwrap();
    f.set_kind(0, unknown); // Known category Option, but not a permitted catalyst Option.
    assert!(
        matches!(f.plan(),Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
    let mut f = fixture();
    f.complete_domain();
    let slot = f.bindings[0].amount.clone();
    let wrong = ParameterValue::Quantity(FiniteQuantity::new(20.0, f.family.unit.clone()).unwrap());
    f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot == slot)
        .unwrap()
        .value = wrong;
    assert!(
        matches!(f.plan(),Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
}

#[test]
fn absent_producers_and_actual_partial_owner_coverage_remain_unresolved() {
    let mut f = fixture();
    f.complete_domain();
    let owner = SchemaSubject::Definition(f.bindings[0].template.address());
    f.recipe
        .rules
        .owners
        .iter_mut()
        .find(|v| v.owner == owner)
        .unwrap()
        .programs
        .members
        .clear();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for equipment in [6, 7] {
        // No producer means no direct output row. Its consumers must remain
        // unresolved rather than acquiring the importer's source defaults.
        assert!(f.input_if_produced(&report, equipment, false).is_none());
        assert!(f.input_if_produced(&report, equipment, true).is_none());
        assert!(matches!(
            f.factor(&report, equipment, 4),
            EffectValue::Unresolved { .. }
        ));
        assert!(matches!(
            f.effective(&report, equipment, 4),
            EffectValue::Unresolved { .. }
        ));
    }
    assert!(matches!(f.total(&report), EffectValue::Unresolved { .. }));
    for template in [false, true] {
        let mut f = fixture();
        f.complete_domain();
        if template {
            f.restore_template_rules();
        } else {
            f.restore_partial_rules();
        }
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(!report.gaps.is_empty());
        assert!(matches!(
            f.total(&report),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
    }
}
