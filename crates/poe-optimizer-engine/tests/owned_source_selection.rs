//! Synthetic native requests exercise source selection, not full build parity.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;

fn selector(f: &mut Fixture) -> &mut ActionSourceSelector {
    &mut f.routes[0].source_selectors.as_mut().unwrap().members[0]
}
fn constant_program(name: &str, context: RuleEntityKind, prefix: &str, base: i64) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context,
        reads: vec![],
        nodes: (0..3)
            .map(|i| literal(&format!("v{i}"), base + i))
            .collect(),
        effects: (0..3)
            .map(|i| {
                derive(
                    &format!("e{i}"),
                    RuleEntity::Current,
                    &format!("{prefix}-{i}"),
                    &format!("v{i}"),
                )
            })
            .collect(),
    }
}
fn source_fixture(eligible: bool) -> Fixture {
    let mut f = Fixture::new();
    f.add_action_route();
    f.build.items[0].parameters[0].value = ParameterValue::Boolean(eligible);
    f.schema
        .definitions
        .push(DefinitionDescriptor::Capability(DefinitionEntry {
            id: def("eligible"),
            schema: SchemaState::Known(CapabilitySchema {
                targets: vec![RuleEntityKind::EquipmentUse],
            }),
        }));
    for (prefix, target) in [
        ("weapon", RuleEntityKind::EquipmentUse),
        ("intrinsic", RuleEntityKind::Actor),
        ("replacement", RuleEntityKind::Action),
        ("selected", RuleEntityKind::Action),
    ] {
        for i in 0..3 {
            f.schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: def(&format!("{prefix}-{i}")),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![target],
                    }),
                }));
        }
    }
    f.owner_mut(&item_owner()).programs.members.extend([
        constant_program("weapon", RuleEntityKind::EquipmentUse, "weapon", 101),
        RuleProgram {
            id: key("eligibility"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![RuleRead {
                id: key("eligible"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Parameter {
                    slot: parameter(SlotOwnerDefId::ItemTemplate(def("item")), "needs-level"),
                },
            }],
            nodes: vec![read_node("eligible", "eligible")],
            effects: vec![effect(
                "eligible",
                RuleEffectKind::Capability {
                    entity: RuleEntity::Current,
                    capability: def("eligible"),
                    enabled: key("eligible"),
                },
            )],
        },
    ]);
    f.owner_mut(&class_owner())
        .programs
        .members
        .push(constant_program(
            "intrinsic",
            RuleEntityKind::Actor,
            "intrinsic",
            11,
        ));
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(output())))
        .programs
        .members
        .push(constant_program(
            "replacement",
            RuleEntityKind::Action,
            "replacement",
            201,
        ));
    let selection = f.routes[0].routes.members[0].selection.clone();
    f.routes[0].source_selectors = Some(DeclaredSet::complete(vec![ActionSourceSelector {
        id: key("attack"),
        selection: selection.clone(),
        sources: vec![
            NamedActionSource {
                id: key("equipped"),
                origin: ActionSourceOrigin::PlayerEquipment {
                    slot: def("weapon"),
                },
            },
            NamedActionSource {
                id: key("baseline"),
                origin: ActionSourceOrigin::ActionActor,
            },
        ],
        policy: ActionSourcePolicy::EquipmentEligibility {
            source: key("equipped"),
            capability: def("eligible"),
            when_empty: ActionSourceOutcome::Use {
                source: key("baseline"),
            },
            when_ineligible: ActionSourceOutcome::Use {
                source: key("baseline"),
            },
        },
    }]));
    f.routes[0].routes.members = (0..3)
        .map(|i| ActionStatRoute {
            id: key(&format!("channel-{i}")),
            selection: selection.clone(),
            target: def(&format!("selected-{i}")),
            source: ActionStatRouteSource::Selected {
                selector: key("attack"),
                stats: vec![
                    ActionSourceStat {
                        source: key("equipped"),
                        stat: def(&format!("weapon-{i}")),
                    },
                    ActionSourceStat {
                        source: key("baseline"),
                        stat: def(&format!("intrinsic-{i}")),
                    },
                ],
            },
        })
        .collect();
    f
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let p = f.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn selected<'a>(
    report: &'a OwnedEffectsReport,
    action: &ActionSelection,
    index: usize,
) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|r| {
            r.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Action(Box::new(action.clone())),
                    stat: def(&format!("selected-{index}")),
                }
        })
        .unwrap()
        .value
}
fn assert_values(report: &OwnedEffectsReport, action: &ActionSelection, base: i64) {
    for i in 0..3 {
        assert_eq!(
            selected(report, action, i),
            &EffectValue::Known {
                value: integer(base + i as i64)
            }
        );
    }
}
fn decision_count(report: &OwnedEffectsReport) -> usize {
    report
        .effects
        .iter()
        .filter(|e| matches!(e.target, BoundEffectTarget::SourceSelection { .. }))
        .count()
}
#[test]
fn one_shared_decision_routes_three_channels_and_distinguishes_equipped_false_from_empty() {
    let mut f = source_fixture(true);
    let equipped = evaluate(&f);
    assert_eq!(decision_count(&equipped), 1);
    assert_values(&equipped, &action(), 101);
    f.build.items[0].parameters[0].value = ParameterValue::Boolean(false);
    let caster = evaluate(&f);
    assert_values(&caster, &action(), 11);
    assert_eq!(f.build.equipment.len(), 2);
    // A distinct known-empty outcome demonstrates that false is not absence.
    let s = selector(&mut f);
    s.sources.push(NamedActionSource {
        id: key("empty-source"),
        origin: ActionSourceOrigin::CurrentAction,
    });
    if let ActionSourcePolicy::EquipmentEligibility { when_empty, .. } = &mut s.policy {
        *when_empty = ActionSourceOutcome::Use {
            source: key("empty-source"),
        };
    }
    for (i, route) in f.routes[0].routes.members.iter_mut().enumerate() {
        if let ActionStatRouteSource::Selected { stats, .. } = &mut route.source {
            stats.push(ActionSourceStat {
                source: key("empty-source"),
                stat: def(&format!("replacement-{i}")),
            });
        }
    }
    assert_values(&evaluate(&f), &action(), 11);
    f.build
        .equipment
        .retain(|e| e.destination != EquipmentDestination::CharacterSlot(def("weapon")));
    assert_values(&evaluate(&f), &action(), 201);
}
#[test]
fn unknown_eligibility_or_selected_stat_never_reselects_available_baseline() {
    let mut f = source_fixture(true);
    f.owner_mut(&item_owner())
        .programs
        .members
        .retain(|p| p.id != key("eligibility"));
    let report = evaluate(&f);
    for i in 0..3 {
        assert!(matches!(
            selected(&report, &action(), i),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
    }
    let mut f = source_fixture(true);
    f.owner_mut(&item_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("weapon"))
        .unwrap()
        .effects
        .remove(1);
    let report = evaluate(&f);
    assert_eq!(
        selected(&report, &action(), 0),
        &EffectValue::Known {
            value: integer(101)
        }
    );
    assert!(matches!(
        selected(&report, &action(), 1),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
    assert_eq!(
        selected(&report, &action(), 2),
        &EffectValue::Known {
            value: integer(103)
        }
    );
    // An unselected missing producer does not contaminate a known false branch.
    f.build.items[0].parameters[0].value = ParameterValue::Boolean(false);
    assert_values(&evaluate(&f), &action(), 11);
}
#[test]
fn raw_occupancy_keeps_unresolved_provider_distinct_from_absence_and_respects_loadout() {
    let mut f = source_fixture(true);
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(1)],
    };
    assert_values(&evaluate(&f), &action(), 101);
    f.build.active_weapon_loadout = occurrence(2);
    assert_values(&evaluate(&f), &action(), 11);
    f.build.active_weapon_loadout = occurrence(1);
    for d in &mut f.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(e) = d {
            e.schema = SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: item_owner(),
                    facet: SchemaFacet::InputSchema,
                    code: key("unknown-item"),
                }],
            };
        }
    }
    // Unknown item schema cannot carry compiled known-owner rules.
    f.owners.retain(|owner| owner.owner != item_owner());
    let report = evaluate(&f);
    let decision = report
        .effects
        .iter()
        .find(|e| matches!(e.target, BoundEffectTarget::SourceSelection { .. }))
        .unwrap();
    assert!(matches!(
        decision.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::UnresolvedTopology,
            ..
        }
    ));
    assert!(!matches!(
        selected(&report, &action(), 0),
        EffectValue::Known { .. }
    ));
}
#[test]
fn fixed_replacement_does_not_probe_equipment_and_self_dependencies_are_cycles() {
    let mut f = source_fixture(true);
    let s = selector(&mut f);
    s.sources = vec![NamedActionSource {
        id: key("replacement"),
        origin: ActionSourceOrigin::CurrentAction,
    }];
    s.policy = ActionSourcePolicy::Fixed {
        source: key("replacement"),
    };
    for (i, r) in f.routes[0].routes.members.iter_mut().enumerate() {
        if let ActionStatRouteSource::Selected { stats, .. } = &mut r.source {
            *stats = vec![ActionSourceStat {
                source: key("replacement"),
                stat: def(&format!("replacement-{i}")),
            }];
        }
    }
    f.owner_mut(&item_owner())
        .programs
        .members
        .retain(|p| p.id != key("eligibility"));
    assert_values(&evaluate(&f), &action(), 201);
    if let ActionStatRouteSource::Selected { stats, .. } = &mut f.routes[0].routes.members[0].source
    {
        stats[0].stat = def("selected-0");
    }
    assert!(f.compile().err().unwrap().to_string().contains("cycle"));
}
#[test]
fn unavailable_inactive_and_partial_selection_remain_distinct() {
    let mut f = source_fixture(false);
    if let ActionSourcePolicy::EquipmentEligibility {
        when_ineligible, ..
    } = &mut selector(&mut f).policy
    {
        *when_ineligible = ActionSourceOutcome::Unavailable;
    }
    let report = evaluate(&f);
    for i in 0..3 {
        assert_eq!(selected(&report, &action(), i), &EffectValue::Inactive);
    }
    let mut f = source_fixture(true);
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(20))
        .unwrap()
        .enabled = false;
    let report = evaluate(&f);
    assert_eq!(decision_count(&report), 0);
    assert!(!report.values.iter().any(|r| matches!(
        &r.key,
        PlanValueKey::Stat {
            entity: ConcreteEntity::Action(_),
            ..
        }
    )));
    let mut f = source_fixture(true);
    f.routes[0].source_selectors.as_mut().unwrap().closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Slot(SlotAddress::ActionOutput(output())),
            facet: SchemaFacet::GameRules,
            code: key("unknown-selector"),
        }],
    };
    let report = evaluate(&f);
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialRouting)
    );
    assert!(matches!(
        selected(&report, &action(), 0),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}
#[test]
fn intrinsic_uses_exact_owned_action_actor_and_retains_its_grant() {
    let mut f = source_fixture(true);
    f.add_generated_actors();
    for slot in &mut f.schema.slots {
        match slot {
            SlotDescriptor::Actor(e) if e.id == child_slot() => {
                if let SchemaState::Known(s) = &mut e.schema {
                    s.outputs.members.push(output());
                }
            }
            SlotDescriptor::ActionOutput(e) if e.id == output() => {
                if let SchemaState::Known(s) = &mut e.schema {
                    s.actor_role = DeclaredActorRole::ProviderActor;
                }
            }
            _ => {}
        }
    }
    for d in &mut f.schema.definitions {
        if let DefinitionDescriptor::Metric(e) = d
            && let SchemaState::Known(s) = &mut e.schema
        {
            s.actor_roles.push(MetricActorRole::Owned);
        }
    }
    let mut a = action();
    a.action.actor = child_actor(30);
    a.action.provider = summoner_provider(30);
    a.action.provider.grant_path.push(child_grant());
    f.queries.requests[0].target = MetricTarget::Action(Box::new(a.clone()));
    selector(&mut f).sources = vec![NamedActionSource {
        id: key("child"),
        origin: ActionSourceOrigin::ActionActor,
    }];
    selector(&mut f).policy = ActionSourcePolicy::Fixed {
        source: key("child"),
    };
    for r in &mut f.routes[0].routes.members {
        if let ActionStatRouteSource::Selected { stats, .. } = &mut r.source {
            *stats = vec![ActionSourceStat {
                source: key("child"),
                stat: def("child-level"),
            }];
        }
    }
    let report = evaluate(&f);
    for i in 0..3 {
        assert_eq!(
            selected(&report, &a, i),
            &EffectValue::Known { value: integer(11) }
        );
    }
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let report = evaluate(&f);
    for i in 0..3 {
        assert_eq!(selected(&report, &a, i), &EffectValue::Inactive);
    }
}
#[test]
fn shared_plan_identity_query_order_and_worker_scratch_are_stable() {
    let mut f = source_fixture(true);
    let mut second = f.queries.requests[0].clone();
    second.id = QueryId::new("second-action-query").unwrap();
    f.queries.requests.push(second);
    let a = f.compile().unwrap();
    let mut scratch = a.new_scratch();
    let expected = a.evaluate(&mut scratch).unwrap();
    assert_eq!(decision_count(&expected), 1);
    f.queries.requests.reverse();
    let same = f.compile().unwrap();
    let reordered = same.evaluate(&mut same.new_scratch()).unwrap();
    assert_eq!(reordered.effects, expected.effects);
    assert_eq!(reordered.values, expected.values);
    assert_eq!(same.bindings().routing, a.bindings().routing);
    assert_eq!(same.bindings().rules, a.bindings().rules);
    f.build.items[0].parameters[0].value = ParameterValue::Boolean(false);
    let b = f.compile().unwrap();
    assert_ne!(a.identity(), b.identity());
    assert_values(&b.evaluate(&mut scratch).unwrap(), &action(), 11);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    use rayon::prelude::*;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        (0..64).into_par_iter().for_each_init(
            || a.new_scratch(),
            |scratch, _| {
                assert_eq!(a.evaluate(scratch).unwrap(), expected);
                assert_values(&b.evaluate(scratch).unwrap(), &action(), 11);
                assert_eq!(a.evaluate(scratch).unwrap(), expected);
            },
        )
    });
    std::thread::scope(|scope| {
        let threads = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let mut scratch = a.new_scratch();
                    for _ in 0..16 {
                        assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
                    }
                })
            })
            .collect::<Vec<_>>();
        for t in threads {
            t.join().unwrap();
        }
    });
}
