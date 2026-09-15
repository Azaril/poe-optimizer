//! End-to-end owned occurrence binding: build data -> rules -> resolved effects.
//! This proves component resolution, not game metric coverage or build legality.
#[path = "support/owned_plan_fixture.rs"]
mod owned_plan_fixture;
use owned_plan_fixture::*;
use poe_optimizer_core::{build_identity::*, owned_build::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use std::{collections::BTreeSet, sync::Arc};

fn stat(entity: ConcreteEntity, name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity,
        stat: def(name),
    }
}
fn equipment(n: u64) -> ConcreteEntity {
    ConcreteEntity::EquipmentUse(occurrence(n))
}
fn player() -> ConcreteEntity {
    ConcreteEntity::Actor(ActorKey::Player)
}
fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    let rows: Vec<_> = report.values.iter().filter(|row| &row.key == key).collect();
    assert_eq!(
        rows.len(),
        1,
        "expected one final value for {key:?}; report: {report:?}"
    );
    &rows[0].value
}
fn known(report: &OwnedEffectsReport, key: &PlanValueKey, n: i64) {
    assert_eq!(
        value(report, key),
        &EffectValue::Known { value: integer(n) }
    );
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn unresolved(report: &OwnedEffectsReport, key: &PlanValueKey, reason: PlanGapReason) {
    assert!(
        matches!(value(report,key),EffectValue::Unresolved {reason:r,..} if *r==reason),
        "{report:?}"
    );
}

#[test]
fn repeated_modifier_definitions_and_shared_item_uses_keep_exact_occurrences() {
    let fixture = Fixture::new();
    let report = evaluate(&fixture);
    known(&report, &stat(equipment(6), "local"), 18);
    known(&report, &stat(equipment(7), "local"), 18);
    known(&report, &stat(player(), "actor-total"), 16);
    assert!(report.gaps.is_empty(), "{report:?}");
    let mut actual = BTreeSet::new();
    for row in &report.effects {
        if row.key.effect != key("local") {
            continue;
        }
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            continue;
        };
        let ProviderRoot::ItemModifier {
            equipment_use,
            modifier,
        } = &provider.root
        else {
            continue;
        };
        assert!(provider.grant_path.is_empty());
        let magnitude = if *modifier == occurrence::<ModifierInstanceId>(4) {
            3
        } else {
            assert_eq!(*modifier, occurrence(5));
            5
        };
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: integer(magnitude)
            }
        );
        assert_eq!(
            row.key.invocation.entity,
            ConcreteEntity::EquipmentUse(*equipment_use)
        );
        assert!(
            actual.insert((*equipment_use, *modifier)),
            "duplicate effect occurrence"
        );
    }
    assert_eq!(
        actual,
        BTreeSet::from([
            (occurrence(6), occurrence(4)),
            (occurrence(6), occurrence(5)),
            (occurrence(7), occurrence(4)),
            (occurrence(7), occurrence(5))
        ])
    );
    // Local base Contribute and its reducer are in one program. Their valid
    // dependency is effect-level; a whole-program self-edge would reject it.
    assert_eq!(
        report
            .effects
            .iter()
            .filter(|r| r.key.invocation.program == key("local"))
            .count(),
        4
    );
}

#[test]
fn inactive_loadout_use_does_not_contribute_or_gain_a_final_value() {
    let mut f = Fixture::new();
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let report = evaluate(&f);
    known(&report, &stat(equipment(6), "local"), 18);
    known(&report, &stat(player(), "actor-total"), 8);
    assert!(
        !report
            .values
            .iter()
            .any(|r| r.key == stat(equipment(7), "local"))
    );
    f.build.active_weapon_loadout = occurrence(2);
    let active = evaluate(&f);
    known(&active, &stat(equipment(7), "local"), 18);
    known(&active, &stat(player(), "actor-total"), 16);
}

#[test]
fn complete_empty_contributions_use_identity_but_partial_membership_never_does() {
    let mut empty = Fixture::new();
    empty.build.items[0].modifiers.clear();
    empty.build.items[0].modifier_order.clear();
    let report = evaluate(&empty);
    known(&report, &stat(player(), "actor-total"), 0);
    known(&report, &stat(equipment(6), "local"), 10);
    let mut partial = Fixture::new();
    let owner = modifier_owner();
    partial.owner_mut(&owner).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("unknown-modifier-programs"),
        }],
    };
    let report = evaluate(&partial);
    unresolved(
        &report,
        &stat(player(), "actor-total"),
        PlanGapReason::IncompleteContributors,
    );
    unresolved(
        &report,
        &stat(equipment(6), "local"),
        PlanGapReason::IncompleteContributors,
    );
    unresolved(
        &report,
        &stat(equipment(7), "local"),
        PlanGapReason::IncompleteContributors,
    );
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms)
    );
    let local: Vec<_> = report
        .effects
        .iter()
        .filter(|r| r.key.invocation.owner == owner && r.key.effect == key("local"))
        .collect();
    assert_eq!(local.len(), 4);
    assert!(
        local
            .iter()
            .all(|r| matches!(r.value, EffectValue::Known { .. })),
        "{report:?}"
    );
}

#[test]
fn absent_program_owner_is_unknown_even_when_other_contributors_are_known() {
    let mut f = Fixture::new();
    f.owners.retain(|o| o.owner != modifier_owner());
    let report = evaluate(&f);
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::MissingPrograms)
    );
    unresolved(
        &report,
        &stat(player(), "actor-total"),
        PlanGapReason::IncompleteContributors,
    );
    unresolved(
        &report,
        &stat(equipment(6), "local"),
        PlanGapReason::IncompleteContributors,
    );
}

#[test]
fn final_producers_that_alias_current_actor_or_player_are_rejected() {
    for alias in [RuleEntity::Current, RuleEntity::Actor, RuleEntity::Player] {
        let mut f = Fixture::new();
        f.owner_mut(&class_owner()).programs.members[0]
            .effects
            .push(derive("competing", alias, "actor-total", "total"));
        let error = f
            .compile()
            .err()
            .expect("two normalized final producers must reject")
            .to_string();
        assert!(
            error.to_lowercase().contains("producer") || error.to_lowercase().contains("duplicate"),
            "{error}"
        );
    }
}

#[test]
fn cross_program_final_dependency_cycle_is_rejected() {
    let mut f = Fixture::new();
    f.owner_mut(&item_owner()).programs.members = [("left", "right"), ("right", "left")]
        .into_iter()
        .map(|(target, source)| RuleProgram {
            id: key(target),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![read(
                "upstream",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def(source),
                },
            )],
            nodes: vec![read_node("value", "upstream")],
            effects: vec![derive("final", RuleEntity::Current, target, "value")],
        })
        .collect();
    let error = f
        .compile()
        .err()
        .expect("dependency cycle must reject")
        .to_string();
    assert!(error.to_lowercase().contains("cycl"), "{error}");
}

#[test]
fn unspecified_item_level_is_missing_only_when_lazy_expression_demands_it() {
    let mut f = Fixture::new();
    f.add_level_program();
    f.build.items[0].item_level = None;
    let report = evaluate(&f);
    known(&report, &stat(equipment(6), "level"), 17);
    known(&report, &stat(equipment(7), "level"), 17);
    f.build.items[0].parameters[0].value = ParameterValue::Boolean(true);
    let missing = evaluate(&f);
    for use_id in [6, 7] {
        assert!(
            matches!(value(&missing,&stat(equipment(use_id),"level")),EffectValue::Unresolved {reason:PlanGapReason::MissingInput,read:Some(read)} if read==&key("level")),
            "{missing:?}"
        );
    }
    f.build.items[0].item_level = Some(23);
    let known_level = evaluate(&f);
    known(&known_level, &stat(equipment(6), "level"), 23);
}

#[test]
fn exact_player_equipment_route_reads_selected_owned_local_stat() {
    let mut f = Fixture::new();
    f.add_action_route();
    let report = evaluate(&f);
    let routed = stat(ConcreteEntity::Action(Box::new(action())), "routed");
    known(&report, &routed, 18);
    let route:Vec<_>=report.effects.iter().filter(|r|matches!(&r.key.invocation.origin,RuleOrigin::Route {action:a,route} if a.as_ref()==&action()&&route==&key("weapon-local"))).collect();
    assert_eq!(route.len(), 1);
    assert_eq!(
        route[0].target,
        BoundEffectTarget::Value {
            key: routed.clone()
        }
    );
    // Data is read from the exact shared record, not a fixture-name result.
    f.build.items[0].modifiers[0].rolls[0].value = integer(31);
    let changed = evaluate(&f);
    known(&changed, &routed, 46);
    assert_ne!(report.identity, changed.identity);
    // Requests remain metric selectors; this report only exposes semantic stats.
    assert_eq!(
        f.request().queries().input().requests[0].metric,
        def("requested")
    );
}

#[test]
fn scratch_reuse_across_plans_and_parallel_workers_preserves_values_and_identity() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<OwnedEffectPlan<poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage>>();
    let a = Arc::new(Fixture::new().compile().unwrap());
    let mut changed = Fixture::new();
    changed.build.items[0].modifiers[0].rolls[0].value = integer(7);
    changed.add_level_program();
    let b = changed.compile().unwrap();
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    known(
        &b.evaluate(&mut scratch).unwrap(),
        &stat(equipment(6), "local"),
        22,
    );
    assert_eq!(first, a.evaluate(&mut scratch).unwrap());
    assert_ne!(a.identity(), b.identity());
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let plan = Arc::clone(&a);
            std::thread::spawn(move || plan.evaluate(&mut plan.new_scratch()).unwrap())
        })
        .collect();
    for handle in handles {
        assert_eq!(first, handle.join().unwrap());
    }
}

#[test]
fn plan_limits_bound_occurrence_expansion_and_effect_rows() {
    let f = Fixture::new();
    for limits in [
        PlanLimits {
            max_providers: 1,
            ..PlanLimits::default()
        },
        PlanLimits {
            max_effects: 1,
            ..PlanLimits::default()
        },
        PlanLimits {
            max_work: 1,
            ..PlanLimits::default()
        },
    ] {
        assert!(f.compile_with(limits).is_err());
    }
}

#[test]
fn generated_actor_projection_uses_parent_provider_and_consumes_own_stat() {
    let mut f = Fixture::new();
    f.add_generated_actors();
    let report = evaluate(&f);
    for (use_id, level) in [(30, 11), (31, 20)] {
        let entity = ConcreteEntity::Actor(child_actor(use_id));
        known(&report, &stat(entity.clone(), "child-level"), level);
        known(&report, &stat(entity.clone(), "child-plus"), level + 1);
        assert_eq!(
            value(
                &report,
                &PlanValueKey::Grant {
                    provider: summoner_provider(use_id),
                    slot: child_grant()
                }
            ),
            &EffectValue::Known {
                value: ParameterValue::Boolean(true)
            }
        );
        let consumers: Vec<_> = report
            .effects
            .iter()
            .filter(|row| {
                row.key.invocation.program == key("child-consumer")
                    && row.key.invocation.entity == entity
            })
            .collect();
        assert_eq!(consumers.len(), 1, "{report:?}");
        assert_eq!(
            consumers[0].key.invocation.owner,
            SchemaSubject::Slot(SlotAddress::Actor(child_slot()))
        );
        let mut entered = summoner_provider(use_id);
        entered.grant_path.push(child_grant());
        assert_eq!(
            consumers[0].key.invocation.origin,
            RuleOrigin::Provider { provider: entered }
        );
        let ActorKey::Owned(actor) = child_actor(use_id) else {
            panic!()
        };
        // Actor identity is based on the supplying prefix. Appending its own
        // activation grant here would silently create a different actor.
        assert_eq!(actor.provider, summoner_provider(use_id));
        assert!(actor.provider.grant_path.is_empty());
    }
    assert!(
        !report
            .values
            .iter()
            .any(|row| row.key == stat(player(), "child-level")
                || row.key == stat(player(), "child-plus"))
    );
    assert!(report.gaps.is_empty(), "{report:?}");
}

#[test]
fn false_grant_deactivates_only_its_exact_child_and_missing_activation_is_unresolved() {
    let mut f = Fixture::new();
    f.add_generated_actors();
    f.build
        .gems
        .iter_mut()
        .find(|gem| gem.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let report = evaluate(&f);
    assert_eq!(
        value(
            &report,
            &PlanValueKey::Grant {
                provider: summoner_provider(30),
                slot: child_grant()
            }
        ),
        &EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    known(
        &report,
        &stat(ConcreteEntity::Actor(child_actor(30)), "child-level"),
        11,
    );
    {
        let name = "child-plus";
        assert_eq!(
            value(&report, &stat(ConcreteEntity::Actor(child_actor(30)), name)),
            &EffectValue::Inactive,
            "{report:?}"
        );
    }
    known(
        &report,
        &stat(ConcreteEntity::Actor(child_actor(31)), "child-level"),
        20,
    );
    known(
        &report,
        &stat(ConcreteEntity::Actor(child_actor(31)), "child-plus"),
        21,
    );
    f.owner_mut(&summoner_owner()).programs.members[0]
        .effects
        .retain(|effect| effect.id != key("activate"));
    let missing = evaluate(&f);
    for (use_id, level) in [(30, 11), (31, 20)] {
        known(
            &missing,
            &stat(ConcreteEntity::Actor(child_actor(use_id)), "child-level"),
            level,
        );
        {
            let name = "child-plus";
            assert!(
                matches!(
                    value(
                        &missing,
                        &stat(ConcreteEntity::Actor(child_actor(use_id)), name)
                    ),
                    EffectValue::Unresolved { .. }
                ),
                "no activation producer may imply neither true nor false: {missing:?}"
            );
        }
    }
}

#[test]
fn complete_actor_skill_membership_is_not_activation_authority() {
    let mut f = Fixture::new();
    f.add_generated_actors();
    for descriptor in &mut f.schema.slots {
        if let SlotDescriptor::Actor(entry) = descriptor
            && entry.id == child_slot()
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.skills.members.push(def("skill"));
        }
    }
    let report = evaluate(&f);
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::UnresolvedActivation)
    );
    unresolved(
        &report,
        &stat(ConcreteEntity::Actor(child_actor(30)), "child-plus"),
        PlanGapReason::IncompleteContributors,
    );
}
