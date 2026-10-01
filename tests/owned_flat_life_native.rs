//! Authored flat-Life components; the synthetic sum is not final maximum Life.
#[path = "support/owned_flat_life_native.rs"]
mod native;
use native::{categories, family, fixture, occurrence, with_categories};
use poe_optimizer_core::{owned_build::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;

#[test]
fn authored_count_to_life_projection_preserves_each_modifier_and_equipment_use() {
    let mut f = fixture();
    for (item, modifier, raw) in [(0, 0, 17.0), (0, 1, 16.0), (1, 0, 10.0)] {
        f.set_raw(item, modifier, raw);
    }
    for item in &mut f.build.items {
        for modifier in &mut item.modifiers {
            for roll in &mut modifier.rolls {
                if let ParameterValue::Boolean(value) = &mut roll.value {
                    *value = false;
                }
            }
        }
    }
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_ne!(f.expected(10.0), f.expected_total(10.0));
    for (equipment, modifier, expected) in [
        (6, 4, 17.0),
        (6, 5, 16.0),
        (7, 4, 17.0),
        (7, 5, 16.0),
        (32, 31, 10.0),
    ] {
        assert_eq!(
            f.effective(&report, equipment, modifier),
            &f.expected(expected)
        );
        let direct: Vec<_> = report.effects.iter().filter(|effect| matches!(
            (&effect.key.invocation.origin, &effect.target),
            (RuleOrigin::Provider { provider }, BoundEffectTarget::Contribution { key })
                if provider.root == ProviderRoot::ItemModifier { equipment_use: occurrence(equipment), modifier: occurrence(modifier) }
                    && provider.grant_path.is_empty()
                    && key.entity == ConcreteEntity::Actor(ActorKey::Player)
                    && key.stat == f.family.contribution && key.kind == ContributionKind::Add
        )).collect();
        assert_eq!(direct.len(), 1, "one authored projection per modifier/use");
        assert_eq!(direct[0].value, f.expected_total(expected));
    }
    // Each raw value is an actual source witness; repeated equipment uses and
    // this finite sum belong only to the explicitly synthetic component.
    assert_eq!(f.total(&report), &f.expected_total(76.0));
}

#[test]
fn source_property_and_category_inputs_control_actual_numeric_programs() {
    let mut f = fixture();
    f.set_raw(0, 0, 10.0);
    f.complete_domain();
    let tagged = f.plan().unwrap();
    let tagged_report = tagged.evaluate(&mut tagged.new_scratch()).unwrap();
    assert!(tagged_report.gaps.is_empty());
    assert_eq!(f.effective(&tagged_report, 6, 4), &f.expected(12.0));
    let life = f.family.properties.get("life").unwrap().clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|roll| roll.slot == life)
        .unwrap()
        .value = ParameterValue::Boolean(false);
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty());
    // An English Life modifier does not imply the source's life property tag.
    assert_eq!(f.effective(&report, 6, 4), &f.expected(10.0));
    assert_eq!(f.effective(&report, 7, 4), &f.expected(10.0));
    assert_eq!(f.total(&report), &f.expected_total(39.0));
    let category = categories::bindings();
    for target in [&category.explicit, &category.implicit, &category.enchant] {
        let mut f = with_categories(target.clone());
        for (item, modifier) in [(0, 0), (0, 1), (1, 0)] {
            f.set_raw(item, modifier, 10.0);
        }
        f.complete_domain();
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        for (equipment, modifier, actual) in [
            (6, 4, &category.explicit),
            (6, 5, &category.implicit),
            (7, 4, &category.explicit),
            (7, 5, &category.implicit),
            (32, 31, &category.enchant),
        ] {
            assert_eq!(
                f.effective(&report, equipment, modifier),
                &f.expected(if actual == target { 15.0 } else { 10.0 })
            );
        }
        assert_eq!(
            f.total(&report),
            &f.expected_total(if target == &category.enchant {
                55.0
            } else {
                60.0
            })
        );
    }
}

#[test]
fn one_physical_life_roll_projects_once_per_selected_equipment_use() {
    let mut f = fixture();
    f.set_raw(0, 0, 10.0);
    f.build.items.truncate(1);
    f.build.items[0].modifiers.truncate(1);
    f.build.items[0].modifier_order.truncate(1);
    f.build.equipment.truncate(2);
    for roll in &mut f.build.items[0].modifiers[0].rolls {
        if let ParameterValue::Boolean(value) = &mut roll.value {
            *value = false;
        }
    }
    assert_eq!(f.build.equipment[0].item, f.build.equipment[1].item);
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(f.effective(&report, 6, 4), &f.expected(10.0));
    assert_eq!(f.effective(&report, 7, 4), &f.expected(10.0));
    let direct: Vec<_> = report
        .effects
        .iter()
        .filter(|effect| {
            matches!(
                &effect.target, BoundEffectTarget::Contribution { key }
                    if key.stat == f.family.contribution && key.kind == ContributionKind::Add
            )
        })
        .collect();
    assert_eq!(direct.len(), 2);
    assert!(
        direct
            .iter()
            .all(|effect| effect.value == f.expected_total(10.0))
    );
    assert_eq!(f.total(&report), &f.expected_total(20.0));
    f.set_raw(0, 0, 0.0);
    let zero = f.plan().unwrap();
    let zero_report = zero.evaluate(&mut zero.new_scratch()).unwrap();
    assert!(zero_report.gaps.is_empty());
    assert_eq!(f.effective(&zero_report, 6, 4), &f.expected(0.0));
    assert_eq!(f.effective(&zero_report, 7, 4), &f.expected(0.0));
    assert_eq!(f.total(&zero_report), &f.expected_total(0.0));
}

#[test]
fn changed_rolls_inactive_uses_and_parallel_scratch_preserve_life_units() {
    let mut f = fixture();
    f.set_raw(0, 0, 10.0);
    f.complete_domain();
    let first = f.plan().unwrap();
    let before = first.evaluate(&mut first.new_scratch()).unwrap();
    f.set_raw(0, 0, 4.0);
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let second = f.plan().unwrap();
    let after = second.evaluate(&mut second.new_scratch()).unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_eq!(f.total(&after), &f.expected_total(17.0));
    assert!(!after.effects.iter().any(|effect| matches!(&effect.key.invocation.origin, RuleOrigin::Provider { provider } if matches!(provider.root, ProviderRoot::EquipmentUse(id) | ProviderRoot::ItemModifier { equipment_use: id, .. } if id == occurrence(7)))));
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
    f.build.active_weapon_loadout = occurrence(2);
    let plan = f.plan().unwrap();
    assert_eq!(
        f.total(&plan.evaluate(&mut plan.new_scratch()).unwrap()),
        &f.expected_total(27.0)
    );
}

#[test]
fn missing_required_inputs_scalars_and_partial_coverage_stay_unresolved() {
    let binding = family::bindings();
    for slot in [binding.amount, binding.category] {
        let mut f = fixture();
        f.complete_domain();
        f.build.items[0].modifiers[0]
            .rolls
            .retain(|roll| roll.slot != slot);
        assert!(
            matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    for missing in [
        "fixture-catalyst-amount",
        "corrupted-base-factor",
        "ordered-magnitude-scalar",
    ] {
        let mut f = fixture();
        f.complete_domain();
        for owner in &mut f.recipe.rules.owners {
            owner
                .programs
                .members
                .retain(|program| program.id.as_str() != missing);
        }
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(
            matches!(f.total(&report), EffectValue::Unresolved { .. }),
            "{missing}: {report:?}"
        );
        for (equipment, modifier) in [(6, 4), (6, 5), (7, 4), (7, 5), (32, 31)] {
            assert!(matches!(
                f.effective(&report, equipment, modifier),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    let mut f = fixture();
    f.complete_domain();
    f.restore_partial_rules();
    assert!(matches!(
        f.family_owner_mut().programs.closure,
        SchemaClosure::Partial { .. }
    ));
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(!report.gaps.is_empty());
    assert_eq!(
        f.total(&report),
        &EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            read: None
        }
    );
}
