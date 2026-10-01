//! Actual authored family programs in a closed, explicitly synthetic component.
#[path = "support/owned_global_minion_level_native.rs"]
mod native;
use native::{Fixture, occurrence};
use poe_optimizer_core::{owned_build::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;

#[test]
fn actual_authored_family_and_numeric_compiler_bind_every_modifier_and_use() {
    let mut f = Fixture::new();
    f.complete_domain();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for (equipment, modifier, expected) in [
        (6, 4, 3.0),
        (6, 5, 6.0),
        (7, 4, 3.0),
        (7, 5, 6.0),
        (32, 31, 7.0),
    ] {
        assert_eq!(
            f.effective(&report, equipment, modifier),
            &f.expected(expected)
        );
    }
    assert_eq!(f.total(&report), &f.expected(25.0));
    let direct: Vec<_> = report
        .effects
        .iter()
        .filter(|e| e.key.effect.as_str() == "direct-minion-gem-level")
        .collect();
    assert_eq!(
        direct.len(),
        5,
        "one contribution per modifier/use, not item definition"
    );
    assert!(direct.iter().all(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.entity == ConcreteEntity::Actor(ActorKey::Player) && key.stat == f.family.contribution && key.kind == ContributionKind::Add)));
}

#[test]
fn inactive_equipment_and_mutated_rolls_do_not_leak_across_plans_or_workers() {
    let mut f = Fixture::new();
    f.complete_domain();
    let first = f.plan().unwrap();
    let a = first.evaluate(&mut first.new_scratch()).unwrap();
    f.set_raw(0, 0, 4.0);
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let second = f.plan().unwrap();
    let b = second.evaluate(&mut second.new_scratch()).unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_eq!(f.total(&b), &f.expected(18.0));
    assert_eq!(f.effective(&b, 6, 4), &f.expected(5.0));
    assert_eq!(f.effective(&b, 32, 31), &f.expected(7.0));
    assert!(!b.effects.iter().any(|e| matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if matches!(provider.root, ProviderRoot::EquipmentUse(id) | ProviderRoot::ItemModifier {equipment_use: id, ..} if id == occurrence(7)))));
    let mut scratch = first.new_scratch();
    for _ in 0..3 {
        assert_eq!(first.evaluate(&mut scratch).unwrap(), a);
        assert_eq!(second.evaluate(&mut scratch).unwrap(), b);
    }
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let (first, second, a, b) = (&first, &second, &a, &b);
            scope.spawn(move || {
                let mut scratch = first.new_scratch();
                assert_eq!(first.evaluate(&mut scratch).unwrap(), *a);
                assert_eq!(second.evaluate(&mut scratch).unwrap(), *b);
            });
        }
    });
    f.build.active_weapon_loadout = occurrence(2);
    let active = f.plan().unwrap();
    assert_eq!(
        f.total(&active.evaluate(&mut active.new_scratch()).unwrap()),
        &f.expected(29.0)
    );
}

#[test]
fn missing_scalar_producer_never_becomes_unity_or_a_partial_sum() {
    for missing in [
        "fixture-catalyst-amount",
        "corrupted-base-factor",
        "ordered-magnitude-scalar",
    ] {
        let mut f = Fixture::new();
        f.complete_domain();
        for owner in &mut f.recipe.rules.owners {
            owner.programs.members.retain(|p| p.id.as_str() != missing);
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
}

#[test]
fn missing_required_input_is_rejected_and_real_partial_coverage_stays_unresolved() {
    let mut f = Fixture::new();
    f.complete_domain();
    f.build.items[0].modifiers[0]
        .rolls
        .retain(|r| r.slot != f.family.amount);
    assert!(
        matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
    );
    let mut f = Fixture::new();
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
