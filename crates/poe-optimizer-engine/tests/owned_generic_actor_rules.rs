//! One generic local mechanic on exact Player, supplied and ScenarioEnemy Actors.
//! Synthetic contracts only; no game data or source-engine assumptions.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_routing::*, owned_rules::*,
    owned_schema::*, owned_stages::*, owned_support_inputs::*, owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage, owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use std::{collections::BTreeSet, sync::Arc};
use support::*;

fn ports() -> DeclaredSlots {
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
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn common_owner() -> SchemaSubject {
    subject(def::<ActorDefinition>("shared-mechanic"))
}
fn encounter_owner(name: &str) -> SchemaSubject {
    subject(def::<EncounterDefinition>(name))
}
fn actor_stat<'a>(report: &'a OwnedEffectsReport, actor: ActorKey, name: &str) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: def(name),
                }
        })
        .expect("exact Actor result")
        .value
}
fn known(report: &OwnedEffectsReport, actor: ActorKey, name: &str, n: i64) {
    assert_eq!(
        actor_stat(report, actor, name),
        &EffectValue::Known { value: integer(n) }
    );
}
fn partial(owner: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::GameRules,
            code: key("remaining-source-domain"),
        }],
    }
}

struct World {
    f: Fixture,
    applications: DeclaredSet<ExistingActorRuleApplication>,
    queries: DeclaredSet<ContributionQuery>,
}
impl World {
    fn new() -> Self {
        let mut f = Fixture::new();
        f.add_generated_actors();
        for name in ["shared-mechanic", "enemy-query-reader", "supplied-profile"] {
            f.schema.definitions.push(DefinitionDescriptor::Actor(entry(
                def(name),
                ActorSchema {
                    declarations: ports(),
                },
            )));
            f.owners.push(DefinitionRules {
                owner: subject(def::<ActorDefinition>(name)),
                programs: DeclaredSet::complete(vec![]),
            });
        }
        let slot = f
            .schema
            .slots
            .iter_mut()
            .find_map(|s| match s {
                SlotDescriptor::Actor(a) if a.id == child_slot() => Some(a),
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(slot) = &mut slot.schema else {
            unreachable!()
        };
        slot.provider_definition = Some(def("supplied-profile"));
        for (name, value) in [
            ("shared-local", ComputedValueType::Integer),
            ("encounter-number", ComputedValueType::Integer),
            ("encounter-flag", ComputedValueType::Boolean),
            ("enemy-number-result", ComputedValueType::Integer),
            ("enemy-flag-result", ComputedValueType::Boolean),
        ] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
                def(name),
                StatSchema {
                    value,
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
        }
        for (name, value, targets) in [
            (
                "role-input",
                ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(-100).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                vec![
                    AssumptionTargetKind::PlayerActor,
                    AssumptionTargetKind::OwnedActor,
                    AssumptionTargetKind::ScenarioEnemyActor,
                ],
            ),
            (
                "enemy-amount",
                ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(-100).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                vec![AssumptionTargetKind::ScenarioEnemyActor],
            ),
            (
                "enemy-enabled",
                ValueSchema::Boolean,
                vec![AssumptionTargetKind::ScenarioEnemyActor],
            ),
            (
                "player-only",
                ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(-100).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                vec![AssumptionTargetKind::PlayerActor],
            ),
        ] {
            f.schema
                .definitions
                .push(DefinitionDescriptor::ExternalInput(entry(
                    def(name),
                    ExternalInputSchema { value, targets },
                )));
        }
        for (actor, n) in [
            (ActorKey::Player, 5),
            (child_actor(30), 11),
            (child_actor(31), 20),
            (ActorKey::ScenarioEnemy, 40),
        ] {
            f.scenario.assumptions.push(ExternalAssumption {
                input: def("role-input"),
                target: AssumptionTarget::Actor(actor),
                value: integer(n),
            });
        }
        f.scenario.assumptions.extend([
            ExternalAssumption {
                input: def("enemy-amount"),
                target: AssumptionTarget::Actor(ActorKey::ScenarioEnemy),
                value: integer(3),
            },
            ExternalAssumption {
                input: def("enemy-enabled"),
                target: AssumptionTarget::Actor(ActorKey::ScenarioEnemy),
                value: ParameterValue::Boolean(true),
            },
        ]);
        f.owner_mut(&common_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("local-law"),
                context: RuleEntityKind::Actor,
                reads: vec![read(
                    "input",
                    RuleReadSource::External {
                        entity: RuleEntity::Current,
                        input: def("role-input"),
                    },
                )],
                nodes: vec![
                    read_node("input", "input"),
                    literal("offset", 16),
                    node(
                        "total",
                        RuleExpression::Add {
                            left: key("input"),
                            right: key("offset"),
                        },
                    ),
                ],
                effects: vec![derive(
                    "local-result",
                    RuleEntity::Current,
                    "shared-local",
                    "total",
                )],
            });
        let inputs = vec![def("role-input"), def("enemy-amount"), def("enemy-enabled")];
        let encounter = f
            .schema
            .definitions
            .iter_mut()
            .find_map(|d| match d {
                DefinitionDescriptor::Encounter(e) if e.id == def("encounter") => Some(e),
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(s) = &mut encounter.schema else {
            unreachable!()
        };
        s.external_inputs = DeclaredSet::complete(inputs);
        let mut other = encounter.clone();
        other.id = def("other-encounter");
        f.schema
            .definitions
            .push(DefinitionDescriptor::Encounter(other));
        f.owners.push(DefinitionRules {
            owner: encounter_owner("other-encounter"),
            programs: DeclaredSet::complete(vec![]),
        });
        for name in ["encounter", "other-encounter"] {
            f.owner_mut(&encounter_owner(name))
                .programs
                .members
                .push(RuleProgram {
                    id: key("configured"),
                    context: RuleEntityKind::Actor,
                    reads: vec![
                        read(
                            "amount",
                            RuleReadSource::External {
                                entity: RuleEntity::Current,
                                input: def("enemy-amount"),
                            },
                        ),
                        RuleRead {
                            id: key("enabled"),
                            value_type: ComputedValueType::Boolean,
                            source: RuleReadSource::External {
                                entity: RuleEntity::Current,
                                input: def("enemy-enabled"),
                            },
                        },
                    ],
                    nodes: vec![
                        read_node("amount", "amount"),
                        read_node("enabled", "enabled"),
                    ],
                    effects: vec![
                        RuleEffect {
                            id: key("numeric"),
                            when: Some(key("enabled")),
                            effect: RuleEffectKind::Contribute {
                                entity: RuleEntity::Current,
                                stat: def("encounter-number"),
                                contribution: ContributionKind::Add,
                                value: key("amount"),
                            },
                        },
                        effect(
                            "flag",
                            RuleEffectKind::Contribute {
                                entity: RuleEntity::Current,
                                stat: def("encounter-flag"),
                                contribution: ContributionKind::Flag,
                                value: key("enabled"),
                            },
                        ),
                    ],
                });
        }
        let mut queries = vec![];
        for (id, stat, contribution, reduction, empty, effect_id) in [
            (
                "enemy-number",
                "encounter-number",
                ContributionKind::Add,
                ContributionReduction::ExactIntegerSum,
                integer(0),
                "numeric",
            ),
            (
                "enemy-flag",
                "encounter-flag",
                ContributionKind::Flag,
                ContributionReduction::Any,
                ParameterValue::Boolean(false),
                "flag",
            ),
        ] {
            queries.push(ContributionQuery {
                id: key(id),
                stat: def(stat),
                contribution,
                groups: vec![ContributionGroup {
                    id: key("sources"),
                    reduction,
                    ordering: ContributionOrdering::Unordered,
                    empty: Some(empty),
                    members: DeclaredSet::complete(
                        ["encounter", "other-encounter"]
                            .map(|name| ContributionMember {
                                producer: ContributionProducer::ProgramEffect(
                                    ProgramContributionProducer {
                                        owner: encounter_owner(name),
                                        program: key("configured"),
                                        effect: key(effect_id),
                                        origin: ContributionOrigin::Encounter,
                                    },
                                ),
                                order: None,
                            })
                            .to_vec(),
                    ),
                }],
            });
        }
        f.owner_mut(&subject(def::<ActorDefinition>("enemy-query-reader")))
            .programs
            .members
            .push(RuleProgram {
                id: key("checked-enemy"),
                context: RuleEntityKind::Actor,
                reads: vec![
                    read(
                        "number",
                        RuleReadSource::ContributionQuery {
                            entity: RuleEntity::Current,
                            query: key("enemy-number"),
                            group: key("sources"),
                        },
                    ),
                    RuleRead {
                        id: key("flag"),
                        value_type: ComputedValueType::Boolean,
                        source: RuleReadSource::ContributionQuery {
                            entity: RuleEntity::Current,
                            query: key("enemy-flag"),
                            group: key("sources"),
                        },
                    },
                ],
                nodes: vec![read_node("number", "number"), read_node("flag", "flag")],
                effects: vec![
                    derive(
                        "number",
                        RuleEntity::Current,
                        "enemy-number-result",
                        "number",
                    ),
                    derive("flag", RuleEntity::Current, "enemy-flag-result", "flag"),
                ],
            });
        Self {
            f,
            applications: DeclaredSet::complete(vec![
                ExistingActorRuleApplication {
                    id: key("common-local"),
                    owner: def("shared-mechanic"),
                    targets: vec![
                        ExistingActorRuleTarget::Player,
                        ExistingActorRuleTarget::OwnedSlot { slot: child_slot() },
                        ExistingActorRuleTarget::ScenarioEnemy,
                    ],
                },
                ExistingActorRuleApplication {
                    id: key("enemy-query"),
                    owner: def("enemy-query-reader"),
                    targets: vec![ExistingActorRuleTarget::ScenarioEnemy],
                },
            ]),
            queries: DeclaredSet::complete(queries),
        }
    }
    fn compile(&self) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
        let f = &self.f;
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap(),
        );
        let input = RulePackageInput {
            support_discovery: Some(assignment_only_domains(&f.owners)),
            existing_actor_rules: Some(self.applications.clone()),
            contribution_queries: Some(self.queries.clone()),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("generic-actor-test"),
            semantics_version: key("test"),
            operations_version: key(OWNED_RULE_OPERATIONS_V28),
            definitions: schema.identity().clone(),
            tables: f.tables.clone(),
            owners: f.owners.clone(),
            receivers: f.receivers.clone(),
        };
        staged_plan(
            f,
            input,
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: f.routes.clone(),
            },
        )
    }
    fn support_report(&self) -> SupportEffectsReport {
        let p = self.compile().unwrap();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn report(&self) -> OwnedEffectsReport {
        evaluated(&self.support_report()).clone()
    }

    fn assumption(&mut self, name: &str) -> &mut ExternalAssumption {
        self.f
            .scenario
            .assumptions
            .iter_mut()
            .find(|a| a.input == def(name))
            .unwrap()
    }
}

#[test]
fn one_local_rule_runs_once_per_exact_actor_with_role_inputs_and_provenance() {
    let mut w = World::new();
    let report = w.report();
    assert!(report.gaps.is_empty());
    let actors: BTreeSet<_> = [
        ActorKey::Player,
        child_actor(30),
        child_actor(31),
        ActorKey::ScenarioEnemy,
    ]
    .into_iter()
    .collect();
    let actual: Vec<_> = report
        .effects
        .iter()
        .filter(|e| e.key.invocation.owner == common_owner())
        .collect();
    assert_eq!(actual.len(), 4);
    let mut seen = BTreeSet::new();
    for e in actual {
        let RuleOrigin::ExistingActor { application, actor } = &e.key.invocation.origin else {
            panic!("shared invocation provenance")
        };
        assert_eq!(application, &key("common-local"));
        assert_eq!(
            e.key.invocation.entity,
            ConcreteEntity::Actor(actor.clone())
        );
        assert!(seen.insert(actor.clone()));
    }
    assert_eq!(seen, actors);
    for (actor, n) in [
        (ActorKey::Player, 21),
        (child_actor(30), 27),
        (child_actor(31), 36),
        (ActorKey::ScenarioEnemy, 56),
    ] {
        known(&report, actor, "shared-local", n);
    }
    w.f.queries.requests.clear();
    w.f.build.character.level = 99;
    let other = w.report();
    assert_eq!(
        other
            .effects
            .iter()
            .filter(|e| e.key.invocation.owner == common_owner())
            .count(),
        4
    );
    known(&other, ActorKey::ScenarioEnemy, "shared-local", 56);
}
#[test]
fn encounter_members_bind_only_selected_actor_and_preserve_zero_false_unknown() {
    let mut w = World::new();
    let report = w.report();
    known(&report, ActorKey::ScenarioEnemy, "enemy-number-result", 3);
    assert_eq!(
        actor_stat(&report, ActorKey::ScenarioEnemy, "enemy-flag-result"),
        &EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    let rows: Vec<_> = report
        .effects
        .iter()
        .filter(|e| e.key.invocation.origin == RuleOrigin::Encounter)
        .collect();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|e| e.key.invocation.owner == encounter_owner("encounter")
                && e.key.invocation.entity == ConcreteEntity::Actor(ActorKey::ScenarioEnemy))
    );
    w.f.scenario.enemy.encounter = def("other-encounter");
    let other = w.report();
    assert!(
        other
            .effects
            .iter()
            .filter(|e| e.key.invocation.origin == RuleOrigin::Encounter)
            .all(|e| e.key.invocation.owner == encounter_owner("other-encounter"))
    );
    w.assumption("enemy-amount").value = integer(0);
    known(
        &w.report(),
        ActorKey::ScenarioEnemy,
        "enemy-number-result",
        0,
    );
    w.assumption("enemy-enabled").value = ParameterValue::Boolean(false);
    let inactive = w.report();
    known(&inactive, ActorKey::ScenarioEnemy, "enemy-number-result", 0);
    assert_eq!(
        actor_stat(&inactive, ActorKey::ScenarioEnemy, "enemy-flag-result"),
        &EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    assert!(
        inactive
            .effects
            .iter()
            .any(|e| e.key.invocation.origin == RuleOrigin::Encounter
                && e.key.effect == key("numeric")
                && e.value == EffectValue::Inactive)
    );
    w.f.scenario
        .assumptions
        .retain(|a| a.input != def("enemy-enabled"));
    let unknown = w.report();
    assert!(matches!(
        actor_stat(&unknown, ActorKey::ScenarioEnemy, "enemy-number-result"),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingInput,
            ..
        }
    ));
}
#[test]
fn unused_role_and_provider_reads_are_denied_before_execution() {
    for source in [
        RuleReadSource::CharacterLevel,
        RuleReadSource::External {
            entity: RuleEntity::Current,
            input: def("player-only"),
        },
        RuleReadSource::Stat {
            entity: RuleEntity::Player,
            stat: def("shared-local"),
        },
        RuleReadSource::GemLevel,
    ] {
        let mut w = World::new();
        w.f.owner_mut(&common_owner()).programs.members[0]
            .reads
            .push(read("unused", source));
        assert!(
            w.compile().is_err(),
            "unused wrong-role/provider read cannot acquire authority"
        );
    }
    let mut wrong = World::new();
    wrong.f.scenario.assumptions.push(ExternalAssumption {
        input: def("player-only"),
        target: AssumptionTarget::Actor(ActorKey::ScenarioEnemy),
        value: integer(1),
    });
    assert!(
        wrong.compile().is_err(),
        "Actor-shaped identity does not erase input role"
    );
}
#[test]
fn ordinary_supplied_actor_rejects_unused_player_only_external_read() {
    let mut w = World::new();
    let owner = subject(def::<ActorDefinition>("supplied-profile"));
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(entry(
            def("profile-local"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor],
            },
        )));
    w.f.owner_mut(&owner).programs.members.push(RuleProgram {
        id: key("ordinary-profile-local"),
        context: RuleEntityKind::Actor,
        reads: vec![read(
            "unused",
            RuleReadSource::External {
                entity: RuleEntity::Current,
                input: def("role-input"),
            },
        )],
        nodes: vec![literal("one", 1)],
        effects: vec![derive(
            "local-result",
            RuleEntity::Current,
            "profile-local",
            "one",
        )],
    });
    assert!(
        w.applications
            .members
            .iter()
            .all(|a| subject(a.owner.clone()) != owner)
    );
    let report = w.report();
    assert_eq!(
        report
            .effects
            .iter()
            .filter(|e| e.key.invocation.owner == owner)
            .count(),
        2
    );
    for actor in [child_actor(30), child_actor(31)] {
        known(&report, actor, "profile-local", 1);
    }
    w.f.owner_mut(&owner).programs.members[0].reads[0].source = RuleReadSource::External {
        entity: RuleEntity::Current,
        input: def("player-only"),
    };
    match w.compile() {
        Err(PlanError::Invalid(message)) => assert_eq!(
            message,
            "external input is not admitted for the exact Actor role"
        ),
        _ => panic!("ordinary supplied-Actor binding must reject the unused wrong-role read"),
    }
}
#[test]
fn action_absolute_enemy_query_does_not_resolve_to_its_own_actor() {
    let mut w = World::new();
    w.f.add_action_route();
    for name in ["action-enemy-result", "action-actor-result"] {
        w.f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(entry(
                def(name),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Action],
                },
            )));
    }
    w.f.owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(output())))
        .programs
        .members
        .push(RuleProgram {
            id: key("absolute-enemy-query"),
            context: RuleEntityKind::Action,
            reads: vec![
                read(
                    "enemy",
                    RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Enemy,
                        query: key("enemy-number"),
                        group: key("sources"),
                    },
                ),
                read(
                    "actor",
                    RuleReadSource::Stat {
                        entity: RuleEntity::Actor,
                        stat: def("shared-local"),
                    },
                ),
            ],
            nodes: vec![read_node("enemy", "enemy"), read_node("actor", "actor")],
            effects: vec![
                derive("enemy", RuleEntity::Current, "action-enemy-result", "enemy"),
                derive("actor", RuleEntity::Current, "action-actor-result", "actor"),
            ],
        });
    for n in [3, 7] {
        w.assumption("enemy-amount").value = integer(n);
        let report = w.report();
        for (name, value) in [("action-enemy-result", n), ("action-actor-result", 21)] {
            let actual = report
                .values
                .iter()
                .find(|v| {
                    v.key
                        == PlanValueKey::Stat {
                            entity: ConcreteEntity::Action(Box::new(action())),
                            stat: def(name),
                        }
                })
                .unwrap();
            assert_eq!(
                actual.value,
                EffectValue::Known {
                    value: integer(value)
                }
            );
        }
        assert!(
            report
                .effects
                .iter()
                .filter(|e| e.key.invocation.program == key("absolute-enemy-query"))
                .all(|e| e.key.invocation.entity == ConcreteEntity::Action(Box::new(action())))
        );
    }
}
#[test]
fn duplicate_applicability_and_supply_bound_owner_are_not_deduplicated() {
    let mut duplicate = World::new();
    let mut second = duplicate.applications.members[0].clone();
    second.id = key("other-id");
    duplicate.applications.members.push(second);
    assert!(duplicate.compile().is_err());
    let mut supply = World::new();
    supply
        .applications
        .members
        .push(ExistingActorRuleApplication {
            id: key("duplicate-supply"),
            owner: def("supplied-profile"),
            targets: vec![ExistingActorRuleTarget::OwnedSlot { slot: child_slot() }],
        });
    assert!(
        supply.compile().is_err(),
        "shared owner must not repeat its actual provider-bound invocation"
    );
}
#[test]
fn supplied_ancestor_activation_and_missing_input_stay_local() {
    let mut w = World::new();
    w.f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let inactive = w.report();
    assert_eq!(
        actor_stat(&inactive, child_actor(30), "shared-local"),
        &EffectValue::Inactive
    );
    known(&inactive, child_actor(31), "shared-local", 36);
    known(&inactive, ActorKey::ScenarioEnemy, "shared-local", 56);
    let mut missing = World::new();
    missing
        .f
        .scenario
        .assumptions
        .retain(|a| a.target != AssumptionTarget::Actor(child_actor(30)));
    let report = missing.report();
    assert!(matches!(
        actor_stat(&report, child_actor(30), "shared-local"),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingInput,
            ..
        }
    ));
    known(&report, child_actor(31), "shared-local", 36);
    let mut absent = World::new();
    absent.f.build.skills.retain(|s| s.id != occurrence(30));
    absent
        .f
        .scenario
        .assumptions
        .retain(|a| a.target != AssumptionTarget::Actor(child_actor(30)));
    let report = absent.report();
    assert!(
        !report
            .effects
            .iter()
            .any(|e| e.key.invocation.owner == common_owner()
                && e.key.invocation.entity == ConcreteEntity::Actor(child_actor(30)))
    );
}
#[test]
fn partial_domains_and_unlisted_encounter_sources_never_become_empty() {
    let mut app = World::new();
    app.applications.closure = partial(common_owner());
    let report = app.support_report();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialExistingActorRules)
    );
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut owner = World::new();
    owner
        .f
        .owner_mut(&encounter_owner("encounter"))
        .programs
        .closure = partial(encounter_owner("encounter"));
    let report = owner.support_report();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    for active in [true, false] {
        let mut unlisted = World::new();
        unlisted.assumption("enemy-amount").value = integer(0);
        unlisted.assumption("enemy-enabled").value = ParameterValue::Boolean(active);
        unlisted.queries.members[0].groups[0]
            .members
            .members
            .clear();
        assert!(
            unlisted.compile().is_err(),
            "zero/inactive still needs declared membership"
        );
    }
    let mut query = World::new();
    query.queries.members[0].groups[0].members.closure =
        partial(subject(def::<StatDefinition>("encounter-number")));
    let report = query.support_report();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    let mut wrong = World::new();
    let ContributionProducer::ProgramEffect(p) =
        &mut wrong.queries.members[0].groups[0].members.members[0].producer
    else {
        unreachable!()
    };
    p.origin = ContributionOrigin::ExistingActor {
        application: key("common-local"),
    };
    assert!(wrong.compile().is_err());
}
#[test]
fn scenario_enemy_does_not_gain_usage_or_final_stat_receiver_authority() {
    assert!(serde_json::from_str::<StatReceiverTarget>("\"scenario_enemy\"").is_err());
    let mut w = World::new();
    w.f.schema
        .definitions
        .push(DefinitionDescriptor::UsagePolicy(entry(
            def("actor-usage"),
            UsagePolicySchema {
                targets: vec![UsageTargetKind::Actor],
                declarations: ports(),
            },
        )));
    w.f.owners.push(DefinitionRules {
        owner: subject(def::<UsagePolicyDefinition>("actor-usage")),
        programs: DeclaredSet::complete(vec![]),
    });
    w.f.scenario.usage.push(UsagePolicySelection {
        policy: def("actor-usage"),
        target: UsageTarget::Actor(ActorKey::ScenarioEnemy),
        parameters: vec![],
    });
    assert!(
        w.compile().is_err(),
        "generic Actor shape does not admit ScenarioEnemy usage"
    );
}
#[test]
fn role_inputs_encounter_selection_and_actor_identity_survive_reuse_and_rayon() {
    let a = World::new();
    let mut b = World::new();
    b.f.scenario.enemy.encounter = def("other-encounter");
    b.assumption("enemy-amount").value = integer(7);
    b.f.scenario
        .assumptions
        .iter_mut()
        .find(|x| {
            x.input == def("role-input") && x.target == AssumptionTarget::Actor(child_actor(31))
        })
        .unwrap()
        .value = integer(70);
    let pa = a.compile().unwrap();
    let pb = b.compile().unwrap();
    let ra = pa.evaluate(&mut pa.new_scratch()).unwrap();
    let rb = pb.evaluate(&mut pb.new_scratch()).unwrap();
    assert_ne!(ra.identity, rb.identity);
    known(evaluated(&rb), child_actor(31), "shared-local", 86);
    known(
        evaluated(&rb),
        ActorKey::ScenarioEnemy,
        "enemy-number-result",
        7,
    );
    let mut scratch = pa.new_scratch();
    for (p, r) in [(&pa, &ra), (&pb, &rb), (&pa, &ra)] {
        assert!(
            &p.evaluate(&mut scratch).unwrap() == r,
            "exact fresh/reused graph result"
        );
    }
    let actual: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || pa.new_scratch(),
            |s, n| {
                if n % 2 == 0 {
                    pa.evaluate(s).unwrap()
                } else {
                    pb.evaluate(s).unwrap()
                }
            },
        )
        .collect();
    for (n, r) in actual.iter().enumerate() {
        assert!(
            r == if n % 2 == 0 { &ra } else { &rb },
            "exact per-worker identity and result"
        );
    }
}

// The explicit finite support domain exists only to exercise the current
// readiness contract. It neither supplies game inputs nor closes a real owner.
fn staged_plan(
    f: &Fixture,
    mut rules_input: RulePackageInput,
    mut routing_input: ActionRoutingInput,
) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
    let namespace = f.schema.namespace.clone();
    let build = &f.build;
    let scenario = &f.scenario;
    let quality_unit = def("percent");
    let unused_input = |name: &str| -> StatDefId {
        DefId::parse(
            namespace.clone(),
            format!("fixture.empty-support-domain.{name}"),
        )
        .unwrap()
    };
    assert_eq!(
        rules_input.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V28
    );
    assert!(build.supports.is_empty());
    let applications = rules_input
        .effect_applications
        .clone()
        .expect("explicit application inventory");
    let unused = [
        (
            unused_input("support-level"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Integer,
        ),
        (
            unused_input("support-quality"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Quantity {
                unit: quality_unit.clone(),
            },
        ),
        (
            unused_input("target-presence"),
            RuleEntityKind::Skill,
            ComputedValueType::Boolean,
        ),
        (
            unused_input("target-player"),
            RuleEntityKind::Skill,
            ComputedValueType::Boolean,
        ),
    ];
    let mut schema_input = f.schema.clone();
    assert!(
        !schema_input
            .definitions
            .iter()
            .any(|row| row.address() == quality_unit.address())
    );
    schema_input
        .definitions
        .push(DefinitionDescriptor::Unit(entry(
            quality_unit.clone(),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )));
    for (id, scope, value) in &unused {
        assert!(
            !schema_input
                .definitions
                .iter()
                .any(|d| d.address() == id.address())
        );
        schema_input
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: value.clone(),
                    targets: vec![*scope],
                }),
            }));
    }
    // This fixture's only directly authored Skill belongs to Player. All
    // other SkillUse records expose complete-empty Gem Skill inventories.
    let facts = RuleProgram {
        id: key("direct-skill-preparation"),
        context: RuleEntityKind::Skill,
        reads: vec![],
        nodes: vec![
            RuleNode {
                id: key("absent"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(false),
                },
            },
            RuleNode {
                id: key("player"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(true),
                },
            },
        ],
        effects: vec![
            RuleEffect {
                id: key("absent"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: unused_input("target-presence"),
                    value: key("absent"),
                },
            },
            RuleEffect {
                id: key("player"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: unused_input("target-player"),
                    value: key("player"),
                },
            },
        ],
    };
    rules_input
        .owners
        .iter_mut()
        .find(|o| o.owner == subject(def::<SkillDefinition>("skill")))
        .unwrap()
        .programs
        .members
        .push(facts);
    let schema =
        Arc::new(OwnedDefinitionSchemaPackage::new(schema_input, Default::default()).unwrap());

    rules_input.definitions = schema.identity().clone();
    let stored = OwnedRulePackage::new(rules_input, schema.as_ref(), Default::default())
        .map_err(|e| PlanError::Invalid(e.to_string()))?;
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, schema.as_ref(), Default::default())
            .map_err(|e| PlanError::Invalid(e.to_string()))?,
    );

    routing_input.definitions = schema.identity().clone();
    let routing = Arc::new(
        OwnedActionRouting::new(routing_input, schema.as_ref(), Default::default()).unwrap(),
    );
    let programs: Vec<_> = stored
        .input()
        .owners
        .iter()
        .flat_map(|o| o.programs.members.iter().map(move |p| (o, p)))
        .collect();
    let stage_for = |o: &DefinitionRules, p: &RuleProgram| {
        if p.id == key("direct-skill-preparation") {
            key("prepare")
        } else if matches!(
            o.owner,
            SchemaSubject::Definition(DefinitionAddress::Encounter(_))
        ) {
            key("deliver")
        } else {
            key("execute")
        }
    };
    let stage_input = EvaluationStagesInput {
        schema_version: OWNED_EVALUATION_STAGES_V3,
        namespace: namespace.clone(),
        release: key("finite-empty-support-stages"),
        definitions: schema.identity().clone(),
        rules: *stored.identity(),
        routing: *routing.identity(),
        stages: vec![
            EvaluationStage {
                id: key("prepare"),
                predecessors: vec![],
            },
            EvaluationStage {
                id: key("deliver"),
                predecessors: vec![key("prepare")],
            },
            EvaluationStage {
                id: key("execute"),
                predecessors: vec![key("deliver")],
            },
        ],
        programs: DeclaredSet::complete(
            programs
                .iter()
                .map(|(o, p)| StagedRuleProgram {
                    owner: o.owner.clone(),
                    program: p.id.clone(),
                    stage: stage_for(o, p),
                })
                .collect(),
        ),
        effect_applications: Some(DeclaredSet {
            members: applications
                .members
                .iter()
                .map(|a| StagedEffectApplication {
                    application: a.id.clone(),
                    stage: key("execute"),
                })
                .collect(),
            closure: applications.closure,
        }),
        routing_stage: key("execute"),
        frozen_channels: unused
            .iter()
            .map(|(id, scope, _)| FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: *scope,
                    stat: id.clone(),
                },
                stage: key("prepare"),
            })
            .chain(
                [
                    ("encounter-number", ContributionKind::Add),
                    ("encounter-flag", ContributionKind::Flag),
                ]
                .into_iter()
                .map(|(name, contribution)| FrozenStageChannel {
                    channel: StageChannel::Contributions {
                        scope: RuleEntityKind::Actor,
                        stat: def(name),
                        contribution,
                    },
                    stage: key("deliver"),
                }),
            )
            .collect(),
        readiness: Some(ReadinessInput {
            skills: vec![SkillReadiness {
                skill: def("skill"),
                parameters: DeclaredSet::complete(vec![]),
                participation: None,
            }],
            programs: DeclaredSet::complete(
                programs
                    .iter()
                    .map(|(o, p)| ReadinessProgram {
                        owner: o.owner.clone(),
                        program: p.id.clone(),
                        phase: if p.id == key("direct-skill-preparation") {
                            ReadinessPhase::Preparation
                        } else {
                            ReadinessPhase::Execution
                        },
                        role: if p.id == key("direct-skill-preparation") {
                            ReadinessProgramRole::PreparationFacts
                        } else {
                            ReadinessProgramRole::Execution
                        },
                        outputs: if p.id == key("direct-skill-preparation") {
                            ["target-presence", "target-player"]
                                .into_iter()
                                .map(|name| StageChannel::Stat {
                                    scope: RuleEntityKind::Skill,
                                    stat: unused_input(name),
                                })
                                .collect()
                        } else {
                            vec![]
                        },
                    })
                    .collect(),
            ),
        }),
    };
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            stage_input,
            schema.as_ref(),
            &stored,
            &routing,
            Default::default(),
        )
        .map_err(|e| PlanError::Invalid(e.to_string()))?,
    );
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            SupportPreparationInput {
                schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
                namespace: namespace.clone(),
                release: key("finite-empty-support-no-supports"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                quality_unit: quality_unit.clone(),
                types: vec![],
                effects: vec![],
                families: vec![],
                supports: vec![],
            },
            schema.as_ref(),
            &stored,
            Default::default(),
        )
        .unwrap(),
    );
    let presence = unused_input("target-presence");
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            SupportInputBindingsInput {
                schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
                namespace: namespace.clone(),
                release: key("finite-empty-support-no-support-inputs"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                stages: *stages.identity(),
                preparation_stage: key("prepare"),
                effective_level: unused_input("support-level"),
                effective_quality: unused_input("support-quality"),
                target: SupportTargetInputBindings {
                    skill_types: vec![],
                    minion_types: OptionalTypeInputs {
                        present: presence.clone(),
                        members: vec![],
                    },
                    summoner: OptionalTypeContextInputs {
                        present: presence.clone(),
                        skill_types: vec![],
                        minion_types: OptionalTypeInputs {
                            present: presence.clone(),
                            members: vec![],
                        },
                    },
                    cannot_be_supported: presence.clone(),
                    has_gem: presence.clone(),
                    from_item: presence.clone(),
                    is_player_actor: unused_input("target-player"),
                },
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            SupportReceivingInput {
                schema_version: OWNED_SUPPORT_RECEIVING_V2,
                namespace: namespace.clone(),
                release: key("finite-empty-support-no-support-receivers"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                inputs: *inputs.identity(),
                stages: *stages.identity(),
                roles: vec![],
                targets: vec![],
                supports: vec![],
                source_properties: None,
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let mut build = build.clone();
    build.authored_support_order = Some(vec![]);
    build.generated_inputs = Some(GeneratedSkillInputsV1 {
        schema_version: 1,
        bindings: vec![],
    });
    let input_limits = OwnedInputLimits::default();
    let invalid =
        |e: poe_optimizer_core::owned_build::StructuralError| PlanError::Invalid(e.to_string());
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(build, input_limits).map_err(invalid)?,
        ScenarioSpec::new(scenario.clone(), input_limits).map_err(invalid)?,
        QuerySpec::new(f.queries.clone(), input_limits).map_err(invalid)?,
        input_limits,
    )
    .map_err(invalid)?;
    OwnedSupportEffectPlan::compile(
        SupportEffectPlanInputs {
            request: Arc::new(request),
            definitions: schema,
            rules,
            routing,
            stages,
            preparation,
            inputs,
            receiving,
        },
        Default::default(),
        Default::default(),
    )
}

fn evaluated(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!(
            "finite Actor fixture did not evaluate: {:?}",
            report.outcome
        )
    };
    effects
}
