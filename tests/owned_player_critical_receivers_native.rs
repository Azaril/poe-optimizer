//! Native receiver authority, independently of the PoE formula/data publication.
//! This finite world explicitly supplies all operands; it proves no real-build coverage.
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_empty_support_contribution_fixture.rs"]
mod empty_support;
#[allow(dead_code)]
#[path = "support/owned_player_critical_receivers.rs"]
mod family;
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_plan_fixture.rs"]
mod support;
#[path = "support/owned_player_critical_receivers_vectors.rs"]
mod vectors;

use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::{owned_plan::*, owned_rules::*};
use rayon::prelude::*;
use std::sync::Arc;
use support::*;

fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn stat(name: &str) -> SchemaSubject {
    subject(def::<StatDefinition>(name))
}
fn encounter() -> SchemaSubject {
    subject(def::<EncounterDefinition>("encounter"))
}
fn shared() -> SchemaSubject {
    subject(def::<ActorDefinition>("shared-intrinsics"))
}
fn partial(owner: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::GameRules,
            code: key("untranslated-source-domain"),
        }],
    }
}
fn bool_node(name: &str, value: bool) -> RuleNode {
    node(
        name,
        RuleExpression::Literal {
            value: ParameterValue::Boolean(value),
        },
    )
}
struct World {
    f: Fixture,
    queries: DeclaredSet<ContributionQuery>,
}
impl World {
    fn new(enemy_level: u16, never: bool) -> Self {
        let mut f = Fixture::new();
        f.build.items.clear();
        f.build.equipment.clear();
        f.schema
            .definitions
            .retain(|d| !matches!(d, DefinitionDescriptor::Skill(_)));
        f.schema
            .slots
            .retain(|d| !matches!(d, SlotDescriptor::ActionOutput(_)));
        f.owners.retain(|o| {
            !matches!(
                o.owner,
                SchemaSubject::Definition(DefinitionAddress::Skill(_)) | SchemaSubject::Slot(_)
            )
        });
        for owner in &mut f.owners {
            owner.programs.members.clear();
        }
        f.scenario.enemy.level = enemy_level;
        f.schema.definitions.push(DefinitionDescriptor::Unit(known(
            def("percent"),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )));
        for (name, value, targets) in [
            (
                "enemy-operand",
                ComputedValueType::Integer,
                vec![RuleEntityKind::Actor, RuleEntityKind::Enemy],
            ),
            (
                "never",
                ComputedValueType::Boolean,
                vec![RuleEntityKind::Actor, RuleEntityKind::Enemy],
            ),
            (
                "selected",
                ComputedValueType::Integer,
                vec![RuleEntityKind::Actor],
            ),
            (
                "final",
                ComputedValueType::Integer,
                vec![RuleEntityKind::Actor],
            ),
        ] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema { value, targets },
            )));
        }
        f.schema.definitions.push(DefinitionDescriptor::Actor(known(
            def("shared-intrinsics"),
            ActorSchema {
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            },
        )));
        f.owners.push(DefinitionRules {
            owner: shared(),
            programs: DeclaredSet::complete(vec![]),
        });
        f.owner_mut(&encounter())
            .programs
            .members
            .push(RuleProgram {
                id: key("enemy-operand"),
                context: RuleEntityKind::Enemy,
                reads: vec![read("level", RuleReadSource::EnemyLevel)],
                nodes: vec![read_node("level", "level")],
                effects: vec![derive(
                    "operand",
                    RuleEntity::Current,
                    "enemy-operand",
                    "level",
                )],
            });
        // Deliberately contradictory Player values expose accidental recipient aliasing.
        f.owner_mut(&class_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("source-flags"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![
                    bool_node("enemy", never),
                    bool_node("player", !never),
                    literal("decoy", 99),
                ],
                effects: vec![
                    effect(
                        "enemy-flag",
                        RuleEffectKind::Contribute {
                            entity: RuleEntity::Enemy,
                            stat: def("never"),
                            contribution: ContributionKind::Flag,
                            value: key("enemy"),
                        },
                    ),
                    effect(
                        "player-flag",
                        RuleEffectKind::Contribute {
                            entity: RuleEntity::Current,
                            stat: def("never"),
                            contribution: ContributionKind::Flag,
                            value: key("player"),
                        },
                    ),
                    derive("decoy", RuleEntity::Current, "enemy-operand", "decoy"),
                ],
            });
        let queries = DeclaredSet::complete(vec![ContributionQuery {
            id: key("never"),
            stat: def("never"),
            contribution: ContributionKind::Flag,
            groups: vec![ContributionGroup {
                id: key("all"),
                reduction: ContributionReduction::Any,
                ordering: ContributionOrdering::Unordered,
                empty: Some(ParameterValue::Boolean(false)),
                members: DeclaredSet::complete(
                    ["enemy-flag", "player-flag"]
                        .into_iter()
                        .map(|name| ContributionMember {
                            producer: ContributionProducer::ProgramEffect(
                                ProgramContributionProducer {
                                    owner: class_owner(),
                                    program: key("source-flags"),
                                    effect: key(name),
                                    origin: ContributionOrigin::Character,
                                },
                            ),
                            order: None,
                        })
                        .collect(),
                ),
            }],
        }]);
        for (name, reads, nodes) in [
            (
                "selected",
                vec![
                    RuleRead {
                        id: key("never"),
                        value_type: ComputedValueType::Boolean,
                        source: RuleReadSource::ContributionQuery {
                            entity: RuleEntity::Enemy,
                            query: key("never"),
                            group: key("all"),
                        },
                    },
                    read(
                        "enemy",
                        RuleReadSource::Stat {
                            entity: RuleEntity::Enemy,
                            stat: def("enemy-operand"),
                        },
                    ),
                ],
                vec![
                    read_node("never", "never"),
                    read_node("enemy", "enemy"),
                    literal("zero", 0),
                    node(
                        "result",
                        RuleExpression::Select {
                            condition: key("never"),
                            when_true: key("zero"),
                            when_false: key("enemy"),
                        },
                    ),
                ],
            ),
            (
                "final",
                vec![read(
                    "selected",
                    RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def("selected"),
                    },
                )],
                vec![
                    read_node("selected", "selected"),
                    literal("one", 1),
                    node(
                        "result",
                        RuleExpression::Add {
                            left: key("selected"),
                            right: key("one"),
                        },
                    ),
                ],
            ),
        ] {
            f.owners.push(DefinitionRules {
                owner: stat(name),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key(name),
                    context: RuleEntityKind::Actor,
                    reads,
                    nodes,
                    effects: vec![derive("result", RuleEntity::Current, name, "result")],
                }]),
            });
            f.receivers.members.push(StatReceiver {
                id: key(name),
                stat: def(name),
                program: key(name),
                targets: vec![StatReceiverTarget::Player],
            });
        }
        Self { f, queries }
    }
    fn schema(&self) -> Arc<OwnedDefinitionSchemaPackage> {
        Arc::new(
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap(),
        )
    }
    fn rules(&self, schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
        RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("receiver-contract"),
            semantics_version: key("finite-component-v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V27),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: self.f.owners.clone(),
            receivers: self.f.receivers.clone(),
            support_discovery: Some(assignment_only_domains(&self.f.owners)),
            existing_actor_rules: Some(DeclaredSet::complete(vec![ExistingActorRuleApplication {
                id: key("shared-player"),
                owner: def("shared-intrinsics"),
                targets: vec![ExistingActorRuleTarget::Player],
            }])),
            contribution_queries: Some(self.queries.clone()),
            effect_applications: Some(DeclaredSet::complete(vec![])),
        }
    }
    fn plan_with(
        &self,
        edit: impl FnOnce(&mut EvaluationStagesInput),
    ) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
        let schema = self.schema();
        empty_support::checked_plan_with_stages(
            &self.f,
            self.rules(&schema),
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            Default::default(),
            edit,
        )
    }
    fn plan(&self) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
        self.plan_with(|_| {}).unwrap()
    }
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("{:?}", report.outcome)
    };
    effects
}
fn value<'a>(report: &'a OwnedEffectsReport, name: &str) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(name),
                }
        })
        .unwrap()
        .value
}
fn number(report: &OwnedEffectsReport, name: &str, expected: i64) {
    assert_eq!(
        value(report, name),
        &EffectValue::Known {
            value: integer(expected)
        }
    );
}
fn incomplete(report: &SupportEffectsReport) {
    assert!(
        matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                },
                ..
            }
        ),
        "{:?}",
        report.outcome
    );
}

#[test]
fn player_receivers_bind_exact_enemy_stats_and_checked_enemy_flags() {
    for (level, never) in [(7, false), (33, true), (81, false)] {
        let world = World::new(level, never);
        let plan = world.plan();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let bound = effects(&report);
        assert!(bound.gaps.is_empty(), "{:?}", bound.gaps);
        let expected = if never { 0 } else { i64::from(level) };
        number(bound, "selected", expected);
        number(bound, "final", expected + 1);
        let receivers: Vec<_> = bound
            .effects
            .iter()
            .filter(|row| matches!(row.key.invocation.origin, RuleOrigin::Receiver { .. }))
            .collect();
        assert_eq!(receivers.len(), 2);
        assert!(receivers.iter().all(|row| row.key.invocation.entity
            == ConcreteEntity::Actor(ActorKey::Player)
            && matches!(
                row.key.invocation.owner,
                SchemaSubject::Definition(DefinitionAddress::Stat(_))
            )));
        // The ordinary expression entry point agrees with the bound/prepared one.
        let schema = world.schema();
        let rules = CompiledRulePackage::compile(
            &world.rules(&schema),
            schema.as_ref(),
            Default::default(),
        )
        .unwrap();
        let ordinary = rules
            .evaluate(
                &stat("selected"),
                &key("selected"),
                &[
                    RuleFact {
                        read: key("never"),
                        value: ParameterValue::Boolean(never),
                    },
                    RuleFact {
                        read: key("enemy"),
                        value: integer(i64::from(level)),
                    },
                ],
                schema.as_ref(),
                &mut rules.new_scratch(),
            )
            .unwrap();
        assert_eq!(
            ordinary.effects[0].disposition,
            EffectDisposition::Applied {
                value: integer(expected)
            }
        );
    }
}

#[test]
fn missing_enemy_operands_are_lazy_but_partial_membership_is_never_false() {
    for never in [false, true] {
        let mut world = World::new(7, never);
        world.f.owner_mut(&encounter()).programs.members.clear();
        let plan = world.plan();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        if never {
            number(effects(&report), "selected", 0);
            number(effects(&report), "final", 1);
        } else {
            assert!(matches!(
                value(effects(&report), "selected"),
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ));
            assert!(!matches!(
                value(effects(&report), "final"),
                EffectValue::Known { .. }
            ));
        }
    }
    let mut world = World::new(7, false);
    world.queries.members[0].groups[0].members.closure = partial(stat("never"));
    let plan = world.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    incomplete(&report);
}

#[test]
fn receiver_results_do_not_close_untranslated_shared_actor_coverage() {
    let mut world = World::new(7, false);
    world.f.owner_mut(&shared()).programs.closure = partial(shared());
    let plan = world.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        report.gaps.iter().any(
            |gap| gap.subject == Some(shared()) && gap.reason == PlanGapReason::PartialPrograms
        )
    );
    incomplete(&report);
}

#[test]
fn receiver_contract_keeps_existing_type_scope_and_write_restrictions() {
    for case in 0..5 {
        let world = World::new(7, false);
        let schema = world.schema();
        let mut input = world.rules(&schema);
        let p = &mut input
            .owners
            .iter_mut()
            .find(|o| o.owner == stat("selected"))
            .unwrap()
            .programs
            .members[0];
        match case {
            0 => p.reads[1].value_type = ComputedValueType::Boolean,
            1 => {
                p.effects[0].effect = RuleEffectKind::Derive {
                    entity: RuleEntity::Enemy,
                    stat: def("selected"),
                    value: key("result"),
                }
            }
            2 => {
                p.effects[0].effect = RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: def("final"),
                    value: key("result"),
                }
            }
            3 => {
                p.reads[0].source = RuleReadSource::Contributions {
                    entity: RuleEntity::Enemy,
                    stat: def("never"),
                    contribution: ContributionKind::Flag,
                    reduction: ContributionReduction::Any,
                    empty: ParameterValue::Boolean(false),
                }
            }
            _ => {
                p.reads[1].source = RuleReadSource::Stat {
                    entity: RuleEntity::Enemy,
                    stat: def("final"),
                }
            }
        }
        assert!(
            CompiledRulePackage::compile(&input, schema.as_ref(), Default::default()).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn receiver_dependency_cycles_competing_producers_and_frozen_stages_still_refuse() {
    let mut world = World::new(7, false);
    world.f.owner_mut(&stat("selected")).programs.members[0].reads[1].source =
        RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: def("final"),
        };
    assert!(
        world.plan_with(|_| {}).is_err(),
        "final -> selected -> final must not iterate"
    );
    let mut world = World::new(7, false);
    let duplicate = world.f.owner_mut(&stat("selected")).programs.members[0].clone();
    world
        .f
        .owner_mut(&class_owner())
        .programs
        .members
        .push(duplicate);
    assert!(
        world.plan_with(|_| {}).is_err(),
        "receiver and class cannot compete for Player's same final Stat"
    );
    let world = World::new(7, false);
    assert!(
        world
            .plan_with(|stages| stages.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Enemy,
                    stat: def("enemy-operand")
                },
                stage: key("prepare"),
            }))
            .is_err(),
        "later Enemy producer cannot write an already frozen dependency"
    );
}

#[test]
fn fresh_reused_and_parallel_workers_keep_receiver_values_and_diagnostics_exact() {
    let good = World::new(7, false).plan();
    let changed = World::new(33, true).plan();
    let mut incomplete = World::new(81, false);
    incomplete.queries.members[0].groups[0].members.closure = partial(stat("never"));
    let unavailable = incomplete.plan();
    let plans = [&good, &changed, &unavailable];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = good.new_scratch();
    for n in [0, 2, 1, 0, 2, 0] {
        assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), expected[n]);
    }
    let actual = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..18)
                .into_par_iter()
                .map_init(
                    || good.new_scratch(),
                    |s, n| plans[n % 3].evaluate(s).unwrap(),
                )
                .collect::<Vec<_>>()
        });
    for (n, report) in actual.into_iter().enumerate() {
        assert_eq!(report, expected[n % 3]);
    }
}
