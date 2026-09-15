//! Stat-owned final calculations over actual owned requests and metric queries.
//! No source importer, UI, per-skill dispatch, or externally supplied final totals.
#[allow(dead_code)]
#[path = "support/owned_metric_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_metrics::*, owned_schema::OwnedDefinitionSchemaPackage};
use poe_optimizer_engine::owned_plan::*;
use std::sync::Arc;
use support::*;

fn stat_owner(name: &str) -> SchemaSubject {
    subject(def::<StatDefinition>(name))
}
fn quantity(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def("count")).unwrap())
}
fn receiver(
    f: &mut Fixture,
    id: &str,
    stat: &str,
    program: RuleProgram,
    targets: Vec<ActorReceiverTarget>,
) {
    let owner = stat_owner(stat);
    f.receivers.members.push(ActorStatReceiver {
        id: key(id),
        stat: def(stat),
        program: program.id.clone(),
        targets,
    });
    if let Some(row) = f.owners.iter_mut().find(|r| r.owner == owner) {
        row.programs.members.push(program);
    } else {
        f.owners.push(DefinitionRules {
            owner,
            programs: DeclaredSet::complete(vec![program]),
        });
    }
}
fn receivers_fixture() -> Fixture {
    let mut f = fixture();
    // This fixture's owned receiver replaces the unused actor-slot final consumer.
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::Actor(child_slot())))
        .programs
        .members
        .clear();
    // Move shared player formulas off the class; the class retains no final formulas.
    let programs = std::mem::take(&mut f.owner_mut(&class_owner()).programs.members);
    for program in programs {
        let stat = match &program.effects[0].effect {
            RuleEffectKind::Derive { stat, .. } => stat.key().as_str().to_owned(),
            _ => panic!("single derived fixture result"),
        };
        receiver(
            &mut f,
            &stat,
            &stat,
            program,
            vec![ActorReceiverTarget::Player],
        );
    }
    // The same semantic final stat has a different explicit recipe for this owned slot.
    receiver(
        &mut f,
        "owned-quantity",
        "player-quantity",
        RuleProgram {
            id: key("owned-quantity"),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("incoming"),
                value_type: ComputedValueType::Quantity { unit: def("count") },
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("child-quantity"),
                },
            }],
            nodes: vec![
                read_node("incoming", "incoming"),
                node(
                    "one",
                    RuleExpression::Literal {
                        value: quantity(1.0),
                    },
                ),
                node(
                    "final",
                    RuleExpression::Add {
                        left: key("incoming"),
                        right: key("one"),
                    },
                ),
            ],
            effects: vec![derive(
                "measure",
                RuleEntity::Actor,
                "player-quantity",
                "final",
            )],
        },
        vec![ActorReceiverTarget::OwnedSlot { slot: child_slot() }],
    );
    f
}
fn metric_plan(f: &Fixture) -> OwnedMetricPlan<OwnedDefinitionSchemaPackage> {
    let effects = Arc::new(f.compile().unwrap());
    let mut input = mapping_input(effects.definitions());
    input
        .bindings
        .iter_mut()
        .find(|b| b.role == MetricBindingRole::OwnedActor)
        .unwrap()
        .stat = def("player-quantity");
    let mapping = Arc::new(
        OwnedMetricMapping::new(input, effects.definitions(), MetricMappingLimits::default())
            .unwrap(),
    );
    OwnedMetricPlan::compile(effects, mapping).unwrap()
}
fn measured(f: &Fixture) -> Vec<f64> {
    let plan = metric_plan(f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{report:?}");
    report.results.iter().map(number).collect()
}
fn final_key(actor: ActorKey, name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Actor(actor),
        stat: def(name),
    }
}
fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    &report.values.iter().find(|r| &r.key == key).unwrap().value
}

#[test]
fn stat_receivers_use_actual_contributions_and_distinct_actor_recipes() {
    let mut f = receivers_fixture();
    assert_eq!(measured(&f), [12.0, 16.0, 18.0, 21.0, 12.0]);
    let plan = metric_plan(&f);
    let report = plan
        .effect_plan()
        .evaluate(&mut plan.new_scratch())
        .unwrap();
    let roots: Vec<_> = report
        .effects
        .iter()
        .filter(|r| matches!(r.key.invocation.origin, RuleOrigin::Receiver { .. }))
        .collect();
    assert_eq!(
        roots.len(),
        4,
        "two player roots and one per real owned actor"
    );
    assert!(roots.iter().all(|r| matches!(
        r.key.invocation.owner,
        SchemaSubject::Definition(DefinitionAddress::Stat(_))
    )));
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(28))
        .unwrap()
        .level = 37;
    f.build.items[0].modifiers[0].rolls[0].value = integer(7);
    assert_eq!(measured(&f), [38.0, 24.0, 22.0, 21.0, 38.0]);
}

#[test]
fn changing_class_does_not_select_a_different_common_receiver() {
    let mut f = receivers_fixture();
    let expected = measured(&f);
    let mut alternative = f
        .schema
        .definitions
        .iter()
        .find_map(|d| {
            if let DefinitionDescriptor::Class(c) = d {
                Some(c.clone())
            } else {
                None
            }
        })
        .unwrap();
    alternative.id = def("alternative-class");
    f.schema
        .definitions
        .push(DefinitionDescriptor::Class(alternative));
    f.owner_mut(&class_owner()).owner = subject(def::<ClassDefinition>("alternative-class"));
    f.build.character.class = def("alternative-class");
    assert_eq!(measured(&f), expected);
}

#[test]
fn receiver_discovery_is_independent_of_query_order_and_unused_queries() {
    let mut f = receivers_fixture();
    let baseline = f.compile().unwrap();
    let report = baseline.evaluate(&mut baseline.new_scratch()).unwrap();
    f.queries.requests.reverse();
    assert_eq!(measured(&f), [12.0, 21.0, 18.0, 16.0, 12.0]);
    f.queries
        .requests
        .retain(|q| matches!(q.target, MetricTarget::Action(_)));
    let no_queries = f.compile().unwrap();
    let without = no_queries.evaluate(&mut no_queries.new_scratch()).unwrap();
    // Remove every actor query while retaining the fixture's explicitly selected
    // action context. Actual actor receivers still serve transitive dependencies.
    for actor in [ActorKey::Player, child_actor(30), child_actor(31)] {
        let key = final_key(actor, "player-quantity");
        assert_eq!(value(&without, &key), value(&report, &key));
    }
}

#[test]
fn absent_owned_applicability_never_inherits_player_defaults() {
    let mut f = receivers_fixture();
    f.receivers
        .members
        .retain(|r| r.id != key("owned-quantity"));
    let plan = metric_plan(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(number(&report.results[1]), 16.0);
    for i in [0, 3, 4] {
        assert!(matches!(
            report.results[i].value,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
    }
}

#[test]
fn false_missing_and_ambiguous_actor_grants_gate_receivers_without_erasing_parent_evidence() {
    for mode in ["false", "missing", "ambiguous"] {
        let mut f = receivers_fixture();
        if mode == "false" {
            f.build
                .gems
                .iter_mut()
                .find(|g| g.id == occurrence(28))
                .unwrap()
                .parameters[0]
                .value = ParameterValue::Boolean(false);
        } else if mode == "missing" {
            f.owner_mut(&summoner_owner()).programs.members[0]
                .effects
                .retain(|e| e.id != key("activate"));
        } else {
            let mut grant = f
                .schema
                .slots
                .iter()
                .find_map(|s| {
                    if let SlotDescriptor::Grant(r) = s {
                        Some(r.clone())
                    } else {
                        None
                    }
                })
                .unwrap();
            grant.id.slot = def("second-child-grant");
            let slot = grant.id.clone();
            f.schema.slots.push(SlotDescriptor::Grant(grant));
            for d in &mut f.schema.definitions {
                if let DefinitionDescriptor::Gem(g) = d
                    && g.id == def("summoner")
                    && let SchemaState::Known(schema) = &mut g.schema
                {
                    schema.declarations.grants.members.push(slot.clone());
                }
            }
            f.owner_mut(&summoner_owner()).programs.members[0]
                .effects
                .push(effect(
                    "second-activation",
                    RuleEffectKind::ActivateGrant {
                        slot,
                        enabled: key("enabled"),
                    },
                ));
        }
        let plan = metric_plan(&f);
        let mut scratch = plan.new_scratch();
        let diagnostic = plan.effect_plan().evaluate(&mut scratch).unwrap();
        assert_eq!(
            value(&diagnostic, &final_key(child_actor(30), "child-quantity")),
            &EffectValue::Known {
                value: quantity(11.0)
            }
        );
        let report = plan.evaluate(&mut scratch).unwrap();
        for i in if mode == "false" {
            vec![0, 4]
        } else {
            vec![0, 3, 4]
        } {
            match mode {
                "false" => assert_eq!(report.results[i].value, EffectValue::Inactive),
                "missing" => assert!(matches!(
                    report.results[i].value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                )),
                _ => assert!(matches!(
                    report.results[i].value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::UnresolvedActivation,
                        ..
                    }
                )),
            }
        }
        if mode == "false" {
            assert_eq!(number(&report.results[3]), 21.0);
        }
    }
}

#[test]
fn complete_empty_contributors_and_partial_registry_or_programs_are_distinct() {
    let mut empty = receivers_fixture();
    empty.build.items[0].modifiers.clear();
    empty.build.items[0].modifier_order.clear();
    assert_eq!(measured(&empty)[1], 0.0);
    for registry in [false, true] {
        let mut f = receivers_fixture();
        let closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: stat_owner("actor-total"),
                facet: SchemaFacet::GameRules,
                code: key("unconverted"),
            }],
        };
        if registry {
            f.receivers.closure = closure;
        } else {
            f.owner_mut(&stat_owner("actor-total")).programs.closure = closure;
        }
        let plan = metric_plan(&f);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.gaps.iter().any(|g| g.reason
            == if registry {
                PlanGapReason::PartialReceivers
            } else {
                PlanGapReason::PartialPrograms
            }));
        assert!(report.results.iter().all(|r| matches!(
            r.value,
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        )));
    }
}

#[test]
fn receiver_and_provider_final_collisions_and_receiver_cycles_are_rejected() {
    let mut f = receivers_fixture();
    let mut competing = f.owner_mut(&stat_owner("actor-total")).programs.members[0].clone();
    competing.id = key("competing");
    if let RuleEffectKind::Derive { entity, .. } = &mut competing.effects[0].effect {
        *entity = RuleEntity::Actor;
    }
    f.owner_mut(&class_owner()).programs.members.push(competing);
    assert!(
        matches!(f.compile(), Err(PlanError::Invalid(s)) if s.contains("competing final producers"))
    );
    let mut f = receivers_fixture();
    f.owner_mut(&stat_owner("actor-total")).programs.members[0].reads[0].source =
        RuleReadSource::Stat {
            entity: RuleEntity::Actor,
            stat: def("actor-total"),
        };
    assert!(matches!(f.compile(), Err(PlanError::Invalid(s)) if s.contains("cycle")));
}

#[test]
fn receiver_limits_charge_expansion_and_workers_keep_independent_scratch() {
    let f = receivers_fixture();
    assert!(matches!(
        f.compile_with(PlanLimits {
            max_invocations: 1,
            ..PlanLimits::default()
        }),
        Err(PlanError::Limit("invocations"))
    ));
    let plan = Arc::new(metric_plan(&f));
    let reference = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let plan = plan.clone();
            std::thread::spawn(move || {
                let mut scratch = plan.new_scratch();
                (0..4)
                    .map(|_| plan.evaluate(&mut scratch).unwrap())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for handle in handles {
        for report in handle.join().unwrap() {
            assert_eq!(report, reference);
        }
    }
    let mut changed = receivers_fixture();
    changed.build.items[0].modifiers[0].rolls[0].value = integer(7);
    let other = metric_plan(&changed);
    let mut scratch = plan.new_scratch();
    assert_ne!(
        other.evaluate(&mut scratch).unwrap().results,
        reference.results
    );
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), reference);
}

#[test]
fn an_unlisted_actual_actor_slot_does_not_match_a_different_owned_slot() {
    let mut f = receivers_fixture();
    let other = DeclaredSlot {
        declaration: child_slot().declaration,
        slot: def("other-child"),
    };
    let grant = DeclaredSlot {
        declaration: child_grant().declaration,
        slot: def("other-child-grant"),
    };
    let mut actor_schema = f
        .schema
        .slots
        .iter()
        .find_map(|s| match s {
            SlotDescriptor::Actor(row) if row.id == child_slot() => Some(row.clone()),
            _ => None,
        })
        .unwrap();
    actor_schema.id = other.clone();
    let mut grant_schema = f
        .schema
        .slots
        .iter()
        .find_map(|s| match s {
            SlotDescriptor::Grant(row) if row.id == child_grant() => Some(row.clone()),
            _ => None,
        })
        .unwrap();
    grant_schema.id = grant.clone();
    let SchemaState::Known(schema) = &mut grant_schema.schema else {
        panic!()
    };
    schema.target = GrantTarget::Actor(other.clone());
    f.schema.slots.extend([
        SlotDescriptor::Actor(actor_schema),
        SlotDescriptor::Grant(grant_schema),
    ]);
    for d in &mut f.schema.definitions {
        if let DefinitionDescriptor::Gem(g) = d
            && g.id == def("summoner")
            && let SchemaState::Known(schema) = &mut g.schema
        {
            schema.declarations.actors.members.push(other.clone());
            schema.declarations.grants.members.push(grant.clone());
        }
    }
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(SlotAddress::Actor(other.clone())),
        programs: DeclaredSet::complete(vec![]),
    });
    let program = &mut f.owner_mut(&summoner_owner()).programs.members[0];
    program.effects.push(effect(
        "other-active",
        RuleEffectKind::ActivateGrant {
            slot: grant,
            enabled: key("enabled"),
        },
    ));
    program.effects.push(effect(
        "other-value",
        RuleEffectKind::ProjectActorStat {
            actor: other.clone(),
            stat: def("child-quantity"),
            value: key("measurement"),
        },
    ));
    let actor = ActorKey::Owned(Box::new(OwnedActorKey {
        provider: summoner_provider(30),
        slot: other,
    }));
    f.queries
        .requests
        .push(query("unlisted", MetricTarget::Actor(actor.clone())));
    let plan = metric_plan(&f);
    let mut scratch = plan.new_scratch();
    let diagnostic = plan.effect_plan().evaluate(&mut scratch).unwrap();
    assert_eq!(
        value(&diagnostic, &final_key(actor, "child-quantity")),
        &EffectValue::Known {
            value: quantity(11.0)
        }
    );
    let report = plan.evaluate(&mut scratch).unwrap();
    assert!(report.gaps.is_empty(), "{report:?}");
    assert_eq!(number(&report.results[0]), 12.0);
    assert!(matches!(
        report.results.last().unwrap().value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
}

#[test]
fn receiver_to_receiver_collision_and_indirect_cycle_are_rejected() {
    let mut collision = receivers_fixture();
    let mut program = collision
        .owner_mut(&stat_owner("actor-total"))
        .programs
        .members[0]
        .clone();
    program.id = key("alternative-total");
    receiver(
        &mut collision,
        "competing-total",
        "actor-total",
        program,
        vec![ActorReceiverTarget::Player],
    );
    assert!(
        matches!(collision.compile(), Err(PlanError::Invalid(s)) if s.contains("competing final producers"))
    );
    let mut cycle = receivers_fixture();
    let mut definition = cycle
        .schema
        .definitions
        .iter()
        .find_map(|d| match d {
            DefinitionDescriptor::Stat(row) if row.id == def("actor-total") => Some(row.clone()),
            _ => None,
        })
        .unwrap();
    definition.id = def("other-total");
    cycle
        .schema
        .definitions
        .push(DefinitionDescriptor::Stat(definition));
    let mut program = cycle.owner_mut(&stat_owner("actor-total")).programs.members[0].clone();
    program.id = key("other-total");
    program.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def("actor-total"),
    };
    let RuleEffectKind::Derive { stat, .. } = &mut program.effects[0].effect else {
        panic!()
    };
    *stat = def("other-total");
    receiver(
        &mut cycle,
        "other-total",
        "other-total",
        program,
        vec![ActorReceiverTarget::Player],
    );
    cycle.owner_mut(&stat_owner("actor-total")).programs.members[0].reads[0].source =
        RuleReadSource::Stat {
            entity: RuleEntity::Actor,
            stat: def("other-total"),
        };
    assert!(matches!(cycle.compile(), Err(PlanError::Invalid(s)) if s.contains("cycle")));
}
