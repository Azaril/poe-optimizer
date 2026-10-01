//! Authored movement inputs/components; this is not final movement-speed parity.
#[path = "support/owned_movement_speed_native.rs"]
mod native;
use native::{categories, family, fixture, occurrence, with_categories};
use poe_optimizer_core::{owned_build::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;

#[test]
fn actual_programs_and_numeric_compiler_preserve_every_modifier_and_equipment_use() {
    let mut f = fixture();
    f.set_raw(0, 0, 10.0);
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    // The fixture supplies Speed catalyst20 at one item and zero at the other.
    // Actual ordered-magnitude and numeric programs perform the truncation.
    for (equipment, modifier, expected) in [
        (6, 4, 12.0),
        (6, 5, 6.0),
        (7, 4, 12.0),
        (7, 5, 6.0),
        (32, 31, 7.0),
    ] {
        assert_eq!(
            f.effective(&report, equipment, modifier),
            &f.expected(expected)
        );
    }
    assert_eq!(f.total(&report), &f.expected(43.0));
    let direct: Vec<_> = report
        .effects
        .iter()
        .filter(|effect| {
            matches!(
                &effect.target,
                BoundEffectTarget::Contribution { key }
                    if key.entity == ConcreteEntity::Actor(ActorKey::Player)
                        && key.stat == f.family.contribution
                        && key.kind == ContributionKind::Add
            )
        })
        .collect();
    assert_eq!(direct.len(), 5, "one contribution per exact modifier/use");
}

#[test]
fn untagged_saved_line_does_not_acquire_the_crafting_catalogue_speed_property() {
    let mut f = fixture();
    f.set_raw(0, 0, 10.0);
    let speed = f.family.properties.get("speed").unwrap().clone();
    f.build.items[0].modifiers[0]
        .rolls
        .iter_mut()
        .find(|roll| roll.slot == speed)
        .unwrap()
        .value = ParameterValue::Boolean(false);
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty());
    assert_eq!(f.effective(&report, 6, 4), &f.expected(10.0));
    assert_eq!(f.effective(&report, 7, 4), &f.expected(10.0));
    assert_eq!(f.effective(&report, 6, 5), &f.expected(6.0));
}

#[test]
fn category_transform_uses_exact_required_options_and_actual_numeric_programs() {
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
            &f.expected(if target == &category.enchant {
                55.0
            } else {
                60.0
            })
        );
    }
}

#[test]
fn changed_rolls_inactive_uses_and_worker_scratch_do_not_share_values() {
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
    assert_eq!(f.total(&after), &f.expected(17.0));
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
        &f.expected(27.0)
    );
}

#[test]
fn missing_required_rolls_scalars_and_real_partial_owners_remain_unavailable() {
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
