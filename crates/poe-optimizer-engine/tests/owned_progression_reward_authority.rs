//! Progression facts keep exact authority after Character/Choice composition.
//! Metric assessments summarize exact requirements without claiming game legality.
#[allow(dead_code)]
#[path = "support/owned_empty_support_contribution_fixture.rs"]
mod empty_support;
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;

use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_project::*, owned_routing::*,
    owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_data::{
    owned_metrics::OwnedMetricMapping, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::owned_plan::*;
use std::{collections::BTreeSet, sync::Arc};
use support::*;

fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}

fn reward() -> RewardDefId {
    def("progression-fact")
}

fn fact_slot() -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Reward(reward()), "count")
}

fn contribution_query(
    name: &str,
    contribution: ContributionKind,
    reduction: ContributionReduction,
    empty: Option<ParameterValue>,
) -> ContributionQuery {
    ContributionQuery {
        id: key(name),
        stat: def(name),
        contribution,
        groups: vec![ContributionGroup {
            id: key("all"),
            reduction,
            ordering: ContributionOrdering::Unordered,
            empty,
            members: DeclaredSet::complete(vec![ContributionMember {
                producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                    owner: subject(reward()),
                    program: key("produce"),
                    effect: key(name),
                    origin: ContributionOrigin::Reward,
                }),
                order: None,
            }]),
        }],
    }
}

struct World {
    f: Fixture,
    queries: DeclaredSet<ContributionQuery>,
}

impl World {
    fn new(values: &[(u64, i64)], character: &[u64], choices: &[u64]) -> Self {
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
        f.schema.definitions.push(DefinitionDescriptor::Unit(known(
            def("percent"),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )));
        f.schema
            .definitions
            .push(DefinitionDescriptor::Reward(known(
                reward(),
                RewardSchema {
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(vec![fact_slot()]),
                        actors: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        choices: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                },
            )));
        f.schema.slots.push(SlotDescriptor::Parameter(known(
            fact_slot(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::RewardParameter],
            },
        )));
        for name in ["fact", "authority", "selected-fact", "authority-count"] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
        }
        f.owners.push(DefinitionRules {
            owner: subject(reward()),
            programs: DeclaredSet::complete(vec![
                RuleProgram {
                    id: key("produce"),
                    context: RuleEntityKind::Actor,
                    reads: vec![read(
                        "count",
                        RuleReadSource::Parameter { slot: fact_slot() },
                    )],
                    nodes: vec![read_node("count", "count"), literal("one", 1)],
                    effects: [
                        ("fact", ContributionKind::Override, "count"),
                        ("authority", ContributionKind::Add, "one"),
                    ]
                    .into_iter()
                    .map(|(name, contribution, value)| {
                        effect(
                            name,
                            RuleEffectKind::Contribute {
                                entity: RuleEntity::Current,
                                stat: def(name),
                                contribution,
                                value: key(value),
                            },
                        )
                    })
                    .collect(),
                },
                RuleProgram {
                    id: key("validate-authority"),
                    context: RuleEntityKind::Actor,
                    reads: vec![read("count", authority_read())],
                    nodes: vec![
                        read_node("count", "count"),
                        literal("one", 1),
                        node(
                            "unique",
                            RuleExpression::Compare {
                                operation: RuleComparison::Equal,
                                left: key("count"),
                                right: key("one"),
                            },
                        ),
                    ],
                    effects: vec![effect(
                        "unique-authority",
                        RuleEffectKind::Requirement {
                            satisfied: key("unique"),
                            code: key("exactly-one-progression-authority"),
                        },
                    )],
                },
            ]),
        });
        for (name, source) in [
            (
                "selected-fact",
                RuleReadSource::ContributionSelection {
                    entity: RuleEntity::Current,
                    query: key("fact"),
                    group: key("all"),
                    projection: ContributionSelectionProjection::Value,
                },
            ),
            ("authority-count", authority_read()),
        ] {
            f.owners.push(DefinitionRules {
                owner: subject(def::<StatDefinition>(name)),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key(name),
                    context: RuleEntityKind::Actor,
                    reads: vec![read("value", source)],
                    nodes: vec![read_node("value", "value")],
                    effects: vec![derive("result", RuleEntity::Current, name, "value")],
                }]),
            });
            f.receivers.members.push(StatReceiver {
                id: key(name),
                stat: def(name),
                program: key(name),
                targets: vec![StatReceiverTarget::Player],
            });
        }
        let project = project(&f.build, values, character, choices);
        f.build = compose(&project, &selection(), None, Default::default())
            .unwrap()
            .input()
            .clone();
        Self {
            f,
            queries: DeclaredSet::complete(vec![
                contribution_query(
                    "fact",
                    ContributionKind::Override,
                    ContributionReduction::RequireAgreement,
                    None,
                ),
                contribution_query(
                    "authority",
                    ContributionKind::Add,
                    ContributionReduction::ExactIntegerSum,
                    Some(integer(0)),
                ),
            ]),
        }
    }

    fn plan(&self) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap();
        let rules = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("progression-authority"),
            semantics_version: key("test-v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V27),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: self.f.owners.clone(),
            receivers: self.f.receivers.clone(),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            contribution_queries: Some(self.queries.clone()),
            existing_actor_rules: None,
            support_discovery: Some(assignment_only_domains(&self.f.owners)),
        };
        empty_support::checked_plan_with_stages(
            &self.f,
            rules,
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("progression-authority-routes"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            Default::default(),
            |stages| {
                for program in &mut stages.programs.members {
                    if program.owner == subject(reward()) && program.program == key("produce") {
                        program.stage = key("prepare");
                    }
                }
                for (name, contribution) in [
                    ("fact", ContributionKind::Override),
                    ("authority", ContributionKind::Add),
                ] {
                    stages.frozen_channels.push(FrozenStageChannel {
                        channel: StageChannel::Contributions {
                            scope: RuleEntityKind::Actor,
                            stat: def(name),
                            contribution,
                        },
                        stage: key("prepare"),
                    });
                }
            },
        )
        .unwrap()
    }

    fn evaluate(&self) -> SupportEffectsReport {
        let plan = self.plan();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }

    fn assess(&self) -> OwnedSupportMetricReport {
        let effects = Arc::new(self.plan());
        let mapping = Arc::new(
            OwnedMetricMapping::new(
                MetricMappingInput {
                    schema_version: OWNED_METRIC_MAPPING_VERSION,
                    namespace: ns(),
                    release: key("progression-no-metric-projection"),
                    definitions: effects.definitions().identity().clone(),
                    bindings: vec![],
                },
                effects.definitions(),
                Default::default(),
            )
            .unwrap(),
        );
        let plan = OwnedSupportMetricPlan::compile(effects, mapping).unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(report.evaluation.results.is_empty());
        report
    }
}

fn authority_read() -> RuleReadSource {
    RuleReadSource::ContributionQuery {
        entity: RuleEntity::Current,
        query: key("authority"),
        group: key("all"),
    }
}

fn selection() -> VariantSelection {
    VariantSelection {
        character: occurrence(50),
        equipment: occurrence(51),
        allocations: occurrence(52),
        skills: occurrence(53),
        choices: occurrence(54),
        active_weapon_loadout: occurrence(1),
    }
}

fn project(
    build: &BuildInput,
    values: &[(u64, i64)],
    character: &[u64],
    choices: &[u64],
) -> BuildProject {
    let selection = selection();
    BuildProject::new(
        ProjectInput {
            allocator: build.allocator,
            revision: build.revision,
            game_version: build.game_version.clone(),
            weapon_loadouts: build.weapon_loadouts.clone(),
            items: vec![],
            gems: vec![],
            rewards: values
                .iter()
                .map(|&(id, count)| RewardSelection {
                    id: occurrence(id),
                    definition: reward(),
                    parameters: vec![ParameterAssignment {
                        slot: fact_slot(),
                        value: integer(count),
                    }],
                })
                .collect(),
            equipment: vec![],
            allocations: vec![],
            skills: vec![],
            supports: vec![],
            payload_links: vec![],
            character_presets: vec![CharacterPreset {
                id: selection.character,
                class: build.character.class.clone(),
                ascendancy: None,
                level: build.character.level,
                rewards: character.iter().copied().map(occurrence).collect(),
            }],
            equipment_presets: vec![EquipmentPreset {
                id: selection.equipment,
                equipment: vec![],
            }],
            allocation_presets: vec![AllocationPreset {
                id: selection.allocations,
                allocations: vec![],
                equipment: vec![],
            }],
            skill_presets: vec![SkillPreset {
                id: selection.skills,
                skills: vec![],
                supports: vec![],
                authored_support_order: None,
                payload_links: vec![],
                usage_preferences: None,
                intent: None,
            }],
            choice_presets: vec![ChoicePreset {
                id: selection.choices,
                choices: vec![],
                rewards: choices.iter().copied().map(occurrence).collect(),
            }],
            saved_variants: vec![],
        },
        Default::default(),
    )
    .unwrap()
}

fn evaluated(report: SupportEffectsReport) -> OwnedEffectsReport {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
        panic!(
            "expected evaluated diagnostic effects: {:?}",
            report.outcome
        )
    };
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
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

fn assert_requirements(report: &OwnedEffectsReport, occurrences: &[u64], satisfied: bool) {
    let requirements: Vec<_> = report
        .effects
        .iter()
        .filter(|row| matches!(row.target, BoundEffectTarget::Requirement { .. }))
        .collect();
    assert_eq!(requirements.len(), occurrences.len());
    let mut origins = BTreeSet::new();
    for row in requirements {
        assert_eq!(row.key.invocation.owner, subject(reward()));
        assert_eq!(row.key.invocation.program, key("validate-authority"));
        assert_eq!(
            row.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Player)
        );
        assert_eq!(
            row.target,
            BoundEffectTarget::Requirement {
                code: key("exactly-one-progression-authority")
            }
        );
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(satisfied)
            }
        );
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            panic!("requirement must retain its exact Reward origin")
        };
        assert!(provider.grant_path.is_empty());
        let ProviderRoot::Reward(id) = provider.root else {
            panic!("expected Reward occurrence")
        };
        origins.insert(id);
    }
    assert_eq!(
        origins,
        occurrences.iter().copied().map(occurrence).collect()
    );
}

fn assert_assessment(world: &World, expected: &[u64], satisfied: bool) {
    let report = world.assess();
    assert!(matches!(report.support, SupportMetricStatus::Evaluated));
    let assessment = report.evaluation.requirements;
    assert_eq!(assessment.coverage, RequirementCoverage::Complete);
    assert_eq!(assessment.is_satisfied(), satisfied);
    assert_eq!(assessment.has_violations(), !satisfied);
    assert_eq!(assessment.results.len(), expected.len());
    let mut origins = BTreeSet::new();
    for row in assessment.results {
        assert_eq!(row.key.invocation.owner, subject(reward()));
        assert_eq!(row.key.invocation.program, key("validate-authority"));
        assert_eq!(row.code, key("exactly-one-progression-authority"));
        assert_eq!(
            row.outcome,
            if satisfied {
                RequirementOutcome::Satisfied
            } else {
                RequirementOutcome::Violated
            }
        );
        let RuleOrigin::Provider { provider } = row.key.invocation.origin else {
            panic!("assessment must retain Reward source identity")
        };
        assert!(provider.grant_path.is_empty());
        let ProviderRoot::Reward(id) = provider.root else {
            panic!("assessment must retain exact Reward occurrence")
        };
        origins.insert(id);
    }
    assert_eq!(origins, expected.iter().copied().map(occurrence).collect());
}

#[test]
fn same_reward_selected_by_character_and_choice_has_one_authority_and_preserves_zero() {
    for count in [0, 56] {
        let world = World::new(&[(40, count)], &[40], &[40]);
        assert_eq!(world.f.build.character.rewards.len(), 1);
        let report = evaluated(world.evaluate());
        assert_eq!(
            value(&report, "selected-fact"),
            &EffectValue::Known {
                value: integer(count)
            }
        );
        assert_eq!(
            value(&report, "authority-count"),
            &EffectValue::Known { value: integer(1) }
        );
        assert_requirements(&report, &[40], true);
        assert_assessment(&world, &[40], true);
    }
}

#[test]
fn distinct_equal_and_conflicting_rewards_fail_exact_requirements_after_composition() {
    for second in [56, 57] {
        let world = World::new(&[(40, 56), (41, second)], &[40], &[41]);
        assert_eq!(world.f.build.character.rewards.len(), 2);
        // Evaluated currently means execution completed, not build legality.
        // The whole diagnostic report retains both false Requirements even when
        // the numeric selection agrees. Complete-build admission must consume them.
        let report = evaluated(world.evaluate());
        assert_eq!(
            value(&report, "authority-count"),
            &EffectValue::Known { value: integer(2) }
        );
        assert_requirements(&report, &[40, 41], false);
        assert_assessment(&world, &[40, 41], false);
        if second == 56 {
            assert_eq!(
                value(&report, "selected-fact"),
                &EffectValue::Known { value: integer(56) }
            );
        } else {
            assert!(matches!(
                value(&report, "selected-fact"),
                EffectValue::Unresolved {
                    reason: PlanGapReason::ConflictingContributors,
                    ..
                }
            ));
        }
    }
}

#[test]
fn incomplete_authority_or_fact_membership_cannot_prove_a_progression_value() {
    for query in 0..2 {
        let mut world = World::new(&[(40, 56)], &[], &[40]);
        let stat = world.queries.members[query].stat.clone();
        world.queries.members[query].groups[0].members.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: subject(stat),
                facet: SchemaFacet::GameRules,
                code: key("unknown-authority-membership"),
            }],
        };
        assert!(matches!(
            world.evaluate().outcome,
            SupportEffectsOutcome::Unavailable { .. }
        ));
        let metrics = world.assess();
        assert!(matches!(
            metrics.support,
            SupportMetricStatus::Unavailable { .. }
        ));
        assert!(matches!(
            metrics.evaluation.requirements.coverage,
            RequirementCoverage::Incomplete { .. }
        ));
        assert!(!metrics.evaluation.requirements.is_satisfied());
    }
}

#[test]
fn missing_reward_is_absent_fact_not_an_authored_zero() {
    let report = evaluated(World::new(&[], &[], &[]).evaluate());
    assert_eq!(
        value(&report, "authority-count"),
        &EffectValue::Known { value: integer(0) }
    );
    assert!(matches!(
        value(&report, "selected-fact"),
        EffectValue::Unresolved {
            reason: PlanGapReason::AbsentSelection,
            ..
        }
    ));
    assert_requirements(&report, &[], false);
}
