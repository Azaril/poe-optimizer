//! Authored copy and Passive programs in the existing finite Minion component.
//! No real contributor closure, canonical unscalable admission or V18 relation
//! is certified by these ordinary-rule numerical controls.
#[path = "support/owned_amulet_level_copy_native.rs"]
mod native;
use native::{Fixture, occurrence};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;

fn assert_copy(f: &Fixture, report: &OwnedEffectsReport, modifier: u64, value: f64) {
    let rows: Vec<_> = f.contributions(report, true).into_iter().filter(|e| matches!(&e.key.invocation.origin,
        RuleOrigin::Provider { provider } if provider.root == ProviderRoot::ItemModifier { equipment_use: occurrence(6), modifier: occurrence(modifier) } && provider.grant_path.is_empty())).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, f.expected(value));
    assert!(
        matches!(&rows[0].target, BoundEffectTarget::Contribution { key } if key.entity == ConcreteEntity::Actor(ActorKey::Player) && key.stat == f.bindings.minion_level && key.kind == ContributionKind::Add)
    );
}

fn assert_missing_copy_producer(
    f: &Fixture,
    report: &OwnedEffectsReport,
    read_name: &str,
    relative: RuleEntity,
    concrete: ConcreteEntity,
    stat: &StatDefId,
) {
    let read_id: OwnedDefinitionKey = read_name.parse().unwrap();
    let authored_read = f
        .authored_copy
        .reads
        .iter()
        .find(|r| r.id == read_id)
        .unwrap();
    assert_eq!(
        authored_read.source,
        RuleReadSource::Stat {
            entity: relative,
            stat: stat.clone(),
        },
        "the diagnostic read must identify the exact authored channel"
    );
    let absent = PlanValueKey::Stat {
        entity: concrete,
        stat: stat.clone(),
    };
    assert!(
        !report.values.iter().any(|v| v.key == absent),
        "the negative removes this exact producer"
    );
    for modifier in [4, 5] {
        let rows: Vec<_> = f.contributions(report, true).into_iter().filter(|e| matches!(&e.key.invocation.origin,
            RuleOrigin::Provider { provider } if provider.root == ProviderRoot::ItemModifier { equipment_use: occurrence(6), modifier: occurrence(modifier) } && provider.grant_path.is_empty())).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].value,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: Some(read_id.clone()),
            }
        );
    }
}

#[test]
fn selected_crown_and_solar_minion_values_keep_two_direct_records_and_zero_copy() {
    let mut f = Fixture::new();
    f.original_pair();
    f.boundary_factor(0.0);
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(f.total(&report), &f.expected(2.0));
    assert_eq!(f.effective(&report, 6, 4), &f.expected(1.0));
    assert_eq!(f.effective(&report, 32, 31), &f.expected(1.0));
    assert_eq!(f.contributions(&report, false).len(), 2);
    assert_eq!(f.contributions(&report, true).len(), 1);
    assert_copy(&f, &report, 4, 0.0);
    let authored_copy = f.authored_copy.clone();
    assert!(
        f.family_owner_mut()
            .programs
            .members
            .contains(&authored_copy)
    );
    assert!(report.effects.iter().any(|e|e.key.invocation.program==f.bindings.programs.copy && e.value==EffectValue::Inactive && matches!(&e.key.invocation.origin,RuleOrigin::Provider {provider} if provider.root==ProviderRoot::ItemModifier {equipment_use:occurrence(32),modifier:occurrence(31)})));
}

#[test]
fn known_non_amulet_needs_no_amulet_snapshot() {
    let mut f = Fixture::new();
    f.build.items.remove(0);
    f.build.equipment.retain(|u| u.id == occurrence(32));
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(f.total(&report), &f.expected(1.0));
    assert_eq!(f.contributions(&report, false).len(), 1);
    assert!(f.contributions(&report, true).is_empty());
    assert!(
        report
            .effects
            .iter()
            .any(|e| e.key.invocation.program == f.bindings.programs.copy
                && e.value == EffectValue::Inactive)
    );
}

#[test]
fn explicit_factor_boundaries_floor_each_copy_without_changing_direct_values() {
    for (percent, first, second, total) in [
        (0.0, 0.0, 0.0, 5.0),
        (25.0, 0.0, 0.0, 5.0),
        (50.0, 0.0, 1.0, 6.0),
        (100.0, 1.0, 3.0, 9.0),
    ] {
        let mut f = Fixture::new();
        f.boundary_factor(percent);
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.gaps.is_empty(), "{percent}: {:?}", report.gaps);
        assert_eq!(f.effective(&report, 6, 4), &f.expected(1.0));
        assert_eq!(f.effective(&report, 6, 5), &f.expected(3.0));
        assert_copy(&f, &report, 4, first);
        assert_copy(&f, &report, 5, second);
        assert_eq!(f.total(&report), &f.expected(total));
        assert_eq!(f.contributions(&report, false).len(), 3);
        assert_eq!(f.contributions(&report, true).len(), 2);
    }
}

#[test]
fn actual_passive_produces_twenty_five_before_the_finite_snapshot_and_copy() {
    let mut f = Fixture::new();
    assert_eq!(f.bindings.passive.percent, 25.0);
    f.set_raw(0, 0, 4.0);
    f.set_raw(0, 1, 8.0);
    f.actual_passive();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(f.factor(&report), &f.expected_percent(25.0));
    let passive: Vec<_> = report
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == f.bindings.programs.passive)
        .collect();
    assert_eq!(passive.len(), 1);
    assert_eq!(passive[0].value, f.expected_percent(25.0));
    assert!(
        matches!(&passive[0].key.invocation.origin,RuleOrigin::Provider {provider} if provider.root==ProviderRoot::Allocation(occurrence(50)) && provider.grant_path.is_empty())
    );
    assert_copy(&f, &report, 4, 1.0);
    assert_copy(&f, &report, 5, 2.0);
    assert_eq!(f.total(&report), &f.expected(16.0));
    assert!(
        f.recipe
            .rules
            .owners
            .iter()
            .any(|o| o.programs.members.contains(&f.authored_passive))
    );
    f.build.allocations.clear();
    let without = f.plan().unwrap();
    let empty = without.evaluate(&mut without.new_scratch()).unwrap();
    assert_eq!(
        f.factor(&empty),
        &f.expected_percent(0.0),
        "only this explicitly finite contributor world proves empty zero"
    );
    assert_eq!(f.total(&empty), &f.expected(13.0));
}

#[test]
fn unscalable_is_a_test_boundary_and_bypasses_only_its_exact_modifier_copy() {
    for percent in [0.0, 25.0, 50.0] {
        let mut f = Fixture::new();
        f.boundary_factor(percent);
        f.set_unscalable_boundary(true);
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.gaps.is_empty());
        assert_copy(&f, &report, 4, 1.0);
        assert_copy(&f, &report, 5, if percent == 50.0 { 1.0 } else { 0.0 });
        assert_eq!(f.effective(&report, 6, 4), &f.expected(1.0));
    }
}

#[test]
fn actual_placement_rejects_wrong_slot_and_scope_instead_of_acquiring_copy_authority() {
    let mut f = Fixture::new();
    f.boundary_factor(50.0);
    f.build.equipment[0].destination =
        EquipmentDestination::CharacterSlot(f.bindings.placement_slots.helmet.clone());
    assert!(matches!(f.plan(), Err(PlanError::Invalid(_))));
    let mut f = Fixture::new();
    f.boundary_factor(50.0);
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    assert!(
        matches!(f.plan(), Err(PlanError::Invalid(_))),
        "actual Amulet slot is Shared, so selected-scope source is invalid"
    );
}

#[test]
fn complete_empty_socket_destination_rejects_an_otherwise_valid_item_socket() {
    let mut f = Fixture::new();
    f.boundary_factor(50.0);
    let crown = f.build.items[1].template.clone();
    let solar = f.build.items[0].template.clone();
    let slot: SocketSlotDefId =
        DefId::parse(crown.namespace().clone(), "fixture-known-item-socket").unwrap();
    f.recipe
        .schema
        .definitions
        .push(DefinitionDescriptor::SocketSlot(DefinitionEntry {
            id: slot.clone(),
            schema: SchemaState::Known(SocketSlotSchema {
                owner: SlotOwnerDefId::ItemTemplate(crown.clone()),
                kind: SocketKind::Item,
                scope: ScopePolicy::Shared,
            }),
        }));
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(parent),
        ..
    }) = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == crown.address())
        .unwrap()
    else {
        panic!()
    };
    parent.declarations.sockets.members.push(slot.clone());
    f.build.equipment[0].destination = EquipmentDestination::ItemSocket {
        container: occurrence(32),
        slot: slot.clone(),
    };
    f.complete_domain();
    assert!(matches!(f.plan(), Err(PlanError::Invalid(_))));

    // This counterfactual membership is test-only: proving the parent/slot are
    // otherwise valid isolates the actual Complete([]) destination rejection.
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(child),
        ..
    }) = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == solar.address())
        .unwrap()
    else {
        panic!()
    };
    assert!(
        child.socket_destinations.is_complete() && child.socket_destinations.members.is_empty()
    );
    child.socket_destinations.members.push(slot);
    f.complete_domain();
    assert!(f.plan().is_ok());
}

#[test]
fn missing_snapshot_eligibility_and_partial_coverage_fail_closed() {
    let f = Fixture::new();
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(f.total(&report), EffectValue::Unresolved { .. }));
    assert_missing_copy_producer(
        &f,
        &report,
        "pre-amulet-percent",
        RuleEntity::Player,
        ConcreteEntity::Actor(ActorKey::Player),
        &f.bindings.channels.pre_amulet_percent,
    );
    for missing in ["eligibility", "actual-family-coverage", "incoming"] {
        let mut f = Fixture::new();
        f.actual_passive();
        match missing {
            "eligibility" => {
                let program = f.bindings.programs.eligibility.clone();
                for owner in &mut f.recipe.rules.owners {
                    owner.programs.members.retain(|p| p.id != program);
                }
            }
            "actual-family-coverage" => f.restore_actual_family_gap(),
            "incoming" => f.incomplete_incoming(),
            _ => unreachable!(),
        }
        let plan = f.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(
            matches!(f.total(&report), EffectValue::Unresolved { .. }),
            "{missing}"
        );
        if missing == "eligibility" {
            assert_missing_copy_producer(
                &f,
                &report,
                "eligible",
                RuleEntity::Current,
                ConcreteEntity::EquipmentUse(occurrence(6)),
                &f.bindings.channels.eligibility,
            );
        }
    }
}

#[test]
fn changed_equipment_occurrence_and_factors_are_isolated_under_scratch_and_rayon_reuse() {
    let mut f = Fixture::new();
    f.boundary_factor(50.0);
    let a = f.plan().unwrap();
    let expected_a = a.evaluate(&mut a.new_scratch()).unwrap();
    f.boundary_factor(100.0);
    f.build.equipment[0].id = occurrence(7);
    let b = f.plan().unwrap();
    let expected_b = b.evaluate(&mut b.new_scratch()).unwrap();
    assert_ne!(a.identity(), b.identity());
    assert_eq!(f.total(&expected_a), &f.expected(6.0));
    assert_eq!(f.total(&expected_b), &f.expected(9.0));
    assert!(!expected_b.effects.iter().any(|e|matches!(&e.key.invocation.origin,RuleOrigin::Provider {provider} if matches!(provider.root,ProviderRoot::EquipmentUse(id)|ProviderRoot::ItemModifier{equipment_use:id,..} if id==occurrence(6)))));
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected_a);
    assert_eq!(b.evaluate(&mut scratch).unwrap(), expected_b);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected_a);
    (0..48).into_par_iter().for_each_init(
        || a.new_scratch(),
        |scratch, i| {
            let (plan, expected) = if i % 2 == 0 {
                (&a, &expected_a)
            } else {
                (&b, &expected_b)
            };
            assert_eq!(&plan.evaluate(scratch).unwrap(), expected);
        },
    );
}
