//! Shared Player rules use the ordinary evaluator, coverage and dependency graph.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use std::sync::Arc;
use support::*;

fn actor_owner() -> SchemaSubject {
    subject(def::<ActorDefinition>("shared-player"))
}
fn empty_declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
struct World {
    fixture: Fixture,
    applications: Option<DeclaredSet<ExistingActorRuleApplication>>,
}
impl World {
    fn new() -> Self {
        let mut f = Fixture::new();
        f.schema
            .definitions
            .push(DefinitionDescriptor::Actor(DefinitionEntry {
                id: def("shared-player"),
                schema: SchemaState::Known(ActorSchema {
                    declarations: empty_declarations(),
                }),
            }));
        for name in ["shared-base", "shared-final"] {
            f.schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: def(name),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }));
        }
        f.owners.push(DefinitionRules {
            owner: actor_owner(),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("intrinsic"),
                context: RuleEntityKind::Actor,
                reads: vec![read("level", RuleReadSource::CharacterLevel)],
                nodes: vec![
                    read_node("level", "level"),
                    literal("base", 16),
                    node(
                        "amount",
                        RuleExpression::Add {
                            left: key("level"),
                            right: key("base"),
                        },
                    ),
                ],
                effects: vec![effect(
                    "base",
                    RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: def("shared-base"),
                        contribution: ContributionKind::Add,
                        value: key("amount"),
                    },
                )],
            }]),
        });
        f.owners.push(DefinitionRules {
            owner: subject(def::<StatDefinition>("shared-final")),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("total"),
                context: RuleEntityKind::Actor,
                reads: vec![contributions("base", RuleEntity::Current, "shared-base")],
                nodes: vec![read_node("base", "base")],
                effects: vec![derive("total", RuleEntity::Current, "shared-final", "base")],
            }]),
        });
        f.receivers.members.push(StatReceiver {
            id: key("shared-final"),
            stat: def("shared-final"),
            program: key("total"),
            targets: vec![StatReceiverTarget::Player],
        });
        Self {
            fixture: f,
            applications: Some(DeclaredSet::complete(vec![ExistingActorRuleApplication {
                id: key("player-intrinsics"),
                owner: def("shared-player"),
                targets: vec![ExistingActorRuleTarget::Player],
            }])),
        }
    }
    fn compile(&self, limits: PlanLimits) -> Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>> {
        let f = &self.fixture;
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap(),
        );
        let rules = RulePackageInput {
            support_discovery: Some(SupportDiscoveryInput {
                providers: f
                    .owners
                    .iter()
                    .map(|row| SupportSourceDomainDeclaration {
                        owner: row.owner.clone(),
                        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
                    })
                    .collect(),
            }),
            existing_actor_rules: self.applications.clone(),
            contribution_queries: None,
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("test"),
            operations_version: key(OWNED_RULE_OPERATIONS_V14),
            definitions: schema.identity().clone(),
            tables: f.tables.clone(),
            owners: f.owners.clone(),
            receivers: f.receivers.clone(),
        };
        let rules = Arc::new(CompiledRulePackage::compile(
            &rules,
            schema.as_ref(),
            Default::default(),
        )?);
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: ns(),
                    release: key("routing"),
                    definitions: schema.identity().clone(),
                    outputs: f.routes.clone(),
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(Arc::new(f.request()), schema, rules, routing, limits)
    }
    fn report(&self) -> OwnedEffectsReport {
        let p = self.compile(Default::default()).unwrap();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
}
fn final_value(report: &OwnedEffectsReport) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def("shared-final"),
                }
        })
        .unwrap()
        .value
}
fn partial() -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: actor_owner(),
            facet: SchemaFacet::GameRules,
            code: key("remaining-intrinsics"),
        }],
    }
}
fn count_shared(report: &OwnedEffectsReport) -> usize {
    report
        .effects
        .iter()
        .filter(|e| matches!(e.key.invocation.origin, RuleOrigin::ExistingActor { .. }))
        .count()
}

#[test]
fn one_actual_player_application_is_independent_of_class_equipment_and_query_count() {
    let mut w = World::new();
    w.fixture.build.character.level = 40;
    let baseline = w.report();
    assert!(baseline.gaps.is_empty());
    assert_eq!(
        final_value(&baseline),
        &EffectValue::Known { value: integer(56) }
    );
    assert_eq!(count_shared(&baseline), 1);
    let e = baseline
        .effects
        .iter()
        .find(|e| e.key.invocation.owner == actor_owner())
        .unwrap();
    assert_eq!(
        e.key.invocation.origin,
        RuleOrigin::ExistingActor {
            application: key("player-intrinsics"),
            actor: ActorKey::Player
        }
    );
    assert_eq!(
        e.key.invocation.entity,
        ConcreteEntity::Actor(ActorKey::Player)
    );
    let original_class = w
        .fixture
        .schema
        .definitions
        .iter()
        .find_map(|d| {
            if let DefinitionDescriptor::Class(e) = d {
                Some(e.clone())
            } else {
                None
            }
        })
        .unwrap();
    let mut other_class = original_class;
    other_class.id = def("other-class");
    w.fixture
        .schema
        .definitions
        .push(DefinitionDescriptor::Class(other_class));
    let mut other_owner = w.fixture.owner_mut(&class_owner()).clone();
    other_owner.owner = subject(def::<ClassDefinition>("other-class"));
    w.fixture.owners.push(other_owner);
    w.fixture.build.character.class = def("other-class");
    w.fixture.build.active_weapon_loadout = occurrence(2);
    let changed = w.report();
    assert_eq!(final_value(&changed), final_value(&baseline));
    assert_eq!(count_shared(&changed), 1);
    w.fixture.queries.requests.clear();
    assert_eq!(count_shared(&w.report()), 1);
}

#[test]
fn partial_registry_and_owner_refuse_even_when_known_effect_is_not_demanded() {
    let mut registry = World::new();
    registry.applications.as_mut().unwrap().closure = partial();
    let r = registry.report();
    assert!(
        r.gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialExistingActorRules)
    );
    assert!(matches!(
        final_value(&r),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
    registry.applications.as_mut().unwrap().members.clear();
    registry
        .fixture
        .receivers
        .members
        .retain(|r| r.id != key("shared-final"));
    assert!(
        registry
            .report()
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialExistingActorRules)
    );
    let mut owner = World::new();
    owner.fixture.owner_mut(&actor_owner()).programs.closure = partial();
    let r = owner.report();
    assert!(
        r.gaps
            .iter()
            .any(|g| g.subject.as_ref() == Some(&actor_owner())
                && g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(matches!(
        final_value(&r),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}

#[test]
fn duplicate_applications_missing_owners_and_foreign_contexts_fail_raw_compilation() {
    let mut duplicate = World::new();
    let mut second = duplicate.applications.as_ref().unwrap().members[0].clone();
    second.id = key("other-name");
    duplicate
        .applications
        .as_mut()
        .unwrap()
        .members
        .push(second);
    assert!(matches!(
        duplicate.compile(Default::default()),
        Err(PlanError::Rule(_))
    ));
    let mut missing = World::new();
    missing.fixture.owners.retain(|o| o.owner != actor_owner());
    assert!(matches!(
        missing.compile(Default::default()),
        Err(PlanError::Rule(_))
    ));
    let mut wrong = World::new();
    wrong.fixture.owner_mut(&actor_owner()).programs.members[0]
        .reads
        .push(read("unowned-item", RuleReadSource::ItemLevel));
    assert!(matches!(
        wrong.compile(Default::default()),
        Err(PlanError::Rule(_))
    ));
}

#[test]
fn same_graph_rejects_final_producer_collision_and_real_dependency_cycles() {
    let mut duplicate = World::new();
    duplicate.fixture.owner_mut(&actor_owner()).programs.members[0]
        .effects
        .push(derive(
            "duplicate",
            RuleEntity::Current,
            "shared-final",
            "amount",
        ));
    assert!(
        matches!(duplicate.compile(Default::default()), Err(PlanError::Invalid(message)) if message.contains("producer"))
    );
    let mut cycle = World::new();
    let p = &mut cycle.fixture.owner_mut(&actor_owner()).programs.members[0];
    p.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def("shared-final"),
    };
    assert!(
        matches!(cycle.compile(Default::default()), Err(PlanError::Invalid(message)) if message.contains("cycle"))
    );
    let mut missing = World::new();
    missing.fixture.owner_mut(&actor_owner()).programs.members[0].reads[0].source =
        RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: def("shared-base"),
        };
    let report = missing.report();
    assert!(matches!(
        final_value(&report),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
}

#[test]
fn empty_actor_owners_still_consume_the_total_owner_binding_budget() {
    let mut w = World::new();
    w.fixture.owner_mut(&actor_owner()).programs.members.clear();
    let mut without = World::new();
    without.applications = None;
    let minimum = (1..30)
        .find(|n| {
            without
                .compile(PlanLimits {
                    max_owner_bindings: *n,
                    ..Default::default()
                })
                .is_ok()
        })
        .unwrap();
    assert!(matches!(
        w.compile(PlanLimits {
            max_owner_bindings: minimum,
            ..Default::default()
        }),
        Err(PlanError::Limit("owner bindings"))
    ));
    assert!(
        w.compile(PlanLimits {
            max_owner_bindings: minimum + 1,
            ..Default::default()
        })
        .is_ok()
    );
}

#[test]
fn no_applicability_is_explicitly_empty_and_does_not_guess_a_class_default() {
    let mut w = World::new();
    w.applications = None;
    let report = w.report();
    assert_eq!(count_shared(&report), 0);
    assert_eq!(
        final_value(&report),
        &EffectValue::Known { value: integer(0) }
    );
    // This zero is the receiver's authored complete-empty Sum identity, not an
    // inferred intrinsic amount. A PoE migration must publish its applicability.
}

#[test]
fn existing_actor_effects_are_deterministic_across_edits_reused_scratch_and_workers() {
    let mut a = World::new();
    a.fixture.build.character.level = 1;
    let mut b = World::new();
    b.fixture.build.character.level = 99;
    let pa = a.compile(Default::default()).unwrap();
    let pb = b.compile(Default::default()).unwrap();
    let va = a.report();
    let vb = b.report();
    assert_ne!(va.identity, vb.identity);
    let mut scratch = pa.new_scratch();
    for (plan, expected) in [(&pa, &va), (&pb, &vb), (&pa, &va)] {
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), expected);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let actual: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                let p = if i % 2 == 0 { &pa } else { &pb };
                p.evaluate(&mut p.new_scratch()).unwrap()
            })
            .collect()
    });
    for (i, report) in actual.iter().enumerate() {
        assert_eq!(report, if i % 2 == 0 { &va } else { &vb });
    }
}
