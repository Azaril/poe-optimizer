//! Source-free schedule, ownership, closure and bounded-storage laws.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_routing::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_data::{owned_routing::*, owned_rules::*, owned_schema::*, owned_stages::*};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("stage-tests", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn owner(name: &str) -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Stat(id(name)))
}
fn entry<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("skill")),
        slot: id("output"),
    }
}
fn stat(name: &str) -> StageChannel {
    StageChannel::Stat {
        scope: RuleEntityKind::Actor,
        stat: id(name),
    }
}
fn program(name: &str) -> RuleProgram {
    RuleProgram {
        id: key("same-local-id"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("one"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
            },
        }],
        effects: vec![RuleEffect {
            id: key("write"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id(name),
                value: key("one"),
            },
        }],
    }
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    rules: OwnedRulePackage,
    routing: OwnedActionRouting,
    input: EvaluationStagesInput,
}
impl Fixture {
    fn new() -> Self {
        let mut definitions = vec![];
        for (name, scope) in [
            ("a", RuleEntityKind::Actor),
            ("b", RuleEntityKind::Actor),
            ("action", RuleEntityKind::Action),
        ] {
            definitions.push(DefinitionDescriptor::Stat(entry(
                id(name),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![scope],
                },
            )));
        }
        definitions.extend([
            DefinitionDescriptor::Unit(entry(
                id("factor"),
                UnitSchema {
                    dimension: UnitDimension::DimensionlessFactor,
                },
            )),
            DefinitionDescriptor::Stat(entry(
                id("transform"),
                StatSchema {
                    value: ComputedValueType::Quantity { unit: id("factor") },
                    targets: vec![RuleEntityKind::Modifier],
                },
            )),
            DefinitionDescriptor::Capability(entry(
                id("eligibility"),
                CapabilitySchema {
                    targets: vec![RuleEntityKind::EquipmentUse],
                },
            )),
            DefinitionDescriptor::EquipmentSlot(entry(
                id("weapon"),
                EquipmentSlotSchema {
                    scope: ScopePolicy::Either,
                },
            )),
            DefinitionDescriptor::Skill(entry(
                id("skill"),
                SkillSchema {
                    directly_selectable: true,
                    declarations: DeclaredSlots {
                        parameters: empty(),
                        choices: empty(),
                        grants: empty(),
                        actors: empty(),
                        skill_grants: empty(),
                        sockets: empty(),
                        outputs: DeclaredSet::complete(vec![output()]),
                    },
                },
            )),
            DefinitionDescriptor::ActionPart(entry(id("part"), ActionPartSchema {})),
            DefinitionDescriptor::ActionMode(entry(id("mode"), ActionModeSchema {})),
            DefinitionDescriptor::ActionStatSet(entry(id("set"), ActionStatSetSchema {})),
        ]);
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns(),
                release: key("schema"),
                semantics_version: key("v1"),
                definitions,
                slots: vec![SlotDescriptor::ActionOutput(entry(
                    output(),
                    ActionOutputSchema {
                        actor_role: DeclaredActorRole::Player,
                        parts: DeclaredSet::complete(vec![id("part")]),
                        modes: DeclaredSet::complete(vec![id("mode")]),
                        stat_sets: DeclaredSet::complete(vec![id("set")]),
                        choices: empty(),
                    },
                ))],
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap();
        let mut b = program("b");
        b.reads.push(RuleRead {
            id: key("a-read"),
            value_type: ComputedValueType::Integer,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: id("a"),
            },
        });
        let rules = OwnedRulePackage::new(
            RulePackageInput {
                ordered_contributions: None,
                effect_applications: None,
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("rules"),
                semantics_version: key("v1"),
                operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
                definitions: schema.identity().clone(),
                tables: vec![],
                owners: vec![
                    DefinitionRules {
                        owner: owner("a"),
                        programs: DeclaredSet::complete(vec![program("a")]),
                    },
                    DefinitionRules {
                        owner: owner("b"),
                        programs: DeclaredSet::complete(vec![b]),
                    },
                ],
                receivers: empty(),
            },
            &schema,
            RuleStorageLimits::default(),
        )
        .unwrap();
        let routing = OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: vec![ActionOutputRoutes {
                    output: output(),
                    routes: DeclaredSet::complete(vec![ActionStatRoute {
                        id: key("actor"),
                        selection: ActionRouteSelection::All,
                        source: ActionStatRouteSource::ActionActor { stat: id("a") },
                        target: id("action"),
                    }]),
                    source_selectors: Some(empty()),
                }],
            },
            &schema,
            RoutingLimits::default(),
        )
        .unwrap();
        let input = EvaluationStagesInput {
            readiness: None,
            effect_applications: None,
            schema_version: OWNED_EVALUATION_STAGES_VERSION,
            namespace: ns(),
            release: key("stages"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            routing: *routing.identity(),
            stages: vec![
                EvaluationStage {
                    id: key("prepare"),
                    predecessors: vec![],
                },
                EvaluationStage {
                    id: key("delivery"),
                    predecessors: vec![key("prepare")],
                },
                EvaluationStage {
                    id: key("finish"),
                    predecessors: vec![key("delivery")],
                },
            ],
            programs: DeclaredSet::complete(vec![
                StagedRuleProgram {
                    owner: owner("a"),
                    program: key("same-local-id"),
                    stage: key("prepare"),
                },
                StagedRuleProgram {
                    owner: owner("b"),
                    program: key("same-local-id"),
                    stage: key("delivery"),
                },
            ]),
            routing_stage: key("delivery"),
            frozen_channels: vec![FrozenStageChannel {
                channel: stat("a"),
                stage: key("prepare"),
            }],
        };
        Self {
            schema,
            rules,
            routing,
            input,
        }
    }
    fn package(&self) -> Result<OwnedEvaluationStages, StageStorageError> {
        OwnedEvaluationStages::new(
            self.input.clone(),
            &self.schema,
            &self.rules,
            &self.routing,
            StageStorageLimits::default(),
        )
    }
    fn change_rules(&mut self, f: impl FnOnce(&mut RulePackageInput)) {
        let mut input = self.rules.input().clone();
        f(&mut input);
        self.rules =
            OwnedRulePackage::new(input, &self.schema, RuleStorageLimits::default()).unwrap();
        self.input.rules = *self.rules.identity();
    }
    fn change_routing(&mut self, f: impl FnOnce(&mut ActionRoutingInput)) {
        let mut input = self.routing.input().clone();
        f(&mut input);
        self.routing =
            OwnedActionRouting::new(input, &self.schema, RoutingLimits::default()).unwrap();
        self.input.routing = *self.routing.identity();
    }
}

#[test]
fn owner_qualified_partition_canonical_roundtrip_and_transitive_precedence() {
    let mut f = Fixture::new();
    let p = f.package().unwrap();
    assert_eq!(
        p.stage_for(&owner("a"), &key("same-local-id")),
        Some(&key("prepare"))
    );
    assert_eq!(
        p.stage_for(&owner("b"), &key("same-local-id")),
        Some(&key("delivery"))
    );
    assert!(p.precedes(&key("prepare"), &key("finish")));
    assert!(!p.precedes(&key("prepare"), &key("prepare")));
    assert!(!p.precedes(&key("finish"), &key("prepare")));
    assert!(!p.precedes(&key("unknown"), &key("finish")));
    assert_eq!(p.frozen_at(&stat("a")), Some(&key("prepare")));
    let limits = StageStorageLimits::default();
    let bytes = encode_evaluation_stages(&p, limits).unwrap();
    f.input.stages.reverse();
    f.input.programs.members.reverse();
    f.input.frozen_channels.reverse();
    assert_eq!(p.identity(), f.package().unwrap().identity());
    assert_eq!(
        p.identity(),
        decode_evaluation_stages(&bytes, &f.schema, &f.rules, &f.routing, limits)
            .unwrap()
            .identity()
    );
    p.verify_bindings(&f.schema, &f.rules, &f.routing).unwrap();
}

#[test]
fn partition_rejects_omissions_duplicates_foreign_programs_and_stages() {
    let original = Fixture::new().input;
    for mutation in 0..5 {
        let mut f = Fixture::new();
        f.input = original.clone();
        match mutation {
            0 => {
                f.input.programs.members.pop();
            }
            1 => f
                .input
                .programs
                .members
                .push(f.input.programs.members[0].clone()),
            2 => f.input.programs.members[0].owner = owner("unknown"),
            3 => f.input.programs.members[0].program = key("missing"),
            _ => f.input.programs.members[0].stage = key("missing"),
        }
        assert!(
            matches!(f.package(), Err(StageStorageError::Invalid(_))),
            "{mutation}"
        );
    }
}

#[test]
fn partial_partition_is_unavailable_and_never_repairs_rule_or_receiver_closure() {
    let mut f = Fixture::new();
    let gap = SchemaGap {
        subject: owner("a"),
        facet: SchemaFacet::GameRules,
        code: key("unconverted"),
    };
    f.input.programs.members.pop();
    f.input.programs.closure = SchemaClosure::Partial {
        gaps: vec![gap.clone()],
    };
    let p = f.package().unwrap();
    assert!(!p.is_complete());
    assert!(p.stage_for(&owner("b"), &key("same-local-id")).is_none());
    f.input.programs.closure = SchemaClosure::Partial { gaps: vec![] };
    assert!(f.package().is_err());
    let mut f = Fixture::new();
    f.change_rules(|rules| {
        rules.owners[0].programs.closure = SchemaClosure::Partial { gaps: vec![gap] }
    });
    let p = f.package().unwrap();
    assert!(p.is_complete());
    // This answers classification only; the authoritative rule closure is untouched.
    assert!(!f.rules.input().owners[0].programs.is_complete());
    f.change_rules(|rules| {
        rules.receivers.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: owner("b"),
                facet: SchemaFacet::GameRules,
                code: key("receiver-discovery"),
            }],
        };
    });
    assert!(f.package().unwrap().is_complete());
    assert!(!f.rules.input().receivers.is_complete());
}

#[test]
fn finite_dag_rejects_cycles_unknown_duplicate_and_incomparable_stages() {
    for mutation in 0..5 {
        let mut f = Fixture::new();
        match mutation {
            0 => f.input.stages[0].predecessors.push(key("finish")),
            1 => f.input.stages[0].predecessors.push(key("unknown")),
            2 => f.input.stages.push(f.input.stages[0].clone()),
            3 => f.input.stages[1].predecessors.push(key("prepare")),
            _ => f.input.stages[1].predecessors.clear(),
        }
        assert!(
            matches!(f.package(), Err(StageStorageError::Invalid(_))),
            "{mutation}"
        );
    }
}

#[test]
fn freeze_rejects_later_potential_writer_even_when_gate_is_false() {
    let mut f = Fixture::new();
    f.change_rules(|rules| {
        let p = &mut rules.owners[1].programs.members[0];
        p.nodes.push(RuleNode {
            id: key("false"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        });
        p.effects.push(RuleEffect {
            id: key("late"),
            when: Some(key("false")),
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Player,
                stat: id("a"),
                value: key("one"),
            },
        });
    });
    assert!(matches!(
        f.package(),
        Err(StageStorageError::Invalid(
            "potential writer occurs after or outside frozen stage"
        ))
    ));
}

#[test]
fn final_values_and_contribution_streams_are_distinct_channels() {
    let mut f = Fixture::new();
    f.change_rules(|rules| {
        rules.owners[1].programs.members[0]
            .effects
            .push(RuleEffect {
                id: key("late-contribution"),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Actor,
                    stat: id("a"),
                    contribution: ContributionKind::Add,
                    value: key("one"),
                },
            })
    });
    f.package().unwrap();
    f.input.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: id("a"),
            contribution: ContributionKind::Add,
        },
        stage: key("prepare"),
    });
    assert!(f.package().is_err());
    f.input.frozen_channels.last_mut().unwrap().stage = key("delivery");
    f.package().unwrap();
}

#[test]
fn frozen_reads_cannot_run_before_their_stage_even_when_not_demanded_yet() {
    let mut f = Fixture::new();
    f.input.frozen_channels[0].stage = key("finish");
    assert!(matches!(
        f.package(),
        Err(StageStorageError::Invalid(
            "frozen channel read occurs before or outside frozen stage"
        ))
    ));
    let mut f = Fixture::new();
    f.input.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::ModifierTransforms {
            stat: id("transform"),
        },
        stage: key("finish"),
    });
    f.change_rules(|rules| {
        rules.owners[1].programs.members[0].reads.push(RuleRead {
            id: key("transform-read"),
            value_type: ComputedValueType::Quantity { unit: id("factor") },
            source: RuleReadSource::ModifierTransforms {
                stat: id("transform"),
                initial: id("transform"),
            },
        })
    });
    assert!(f.package().is_err());
}

#[test]
fn routing_writes_and_eligibility_selectors_participate_in_frozen_channels() {
    let mut f = Fixture::new();
    f.input.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Stat {
            scope: RuleEntityKind::Action,
            stat: id("action"),
        },
        stage: key("prepare"),
    });
    assert!(f.package().is_err());
    let mut f = Fixture::new();
    f.change_routing(|routing| {
        routing.outputs[0].source_selectors =
            Some(DeclaredSet::complete(vec![ActionSourceSelector {
                id: key("weapon-selector"),
                selection: ActionRouteSelection::All,
                sources: vec![NamedActionSource {
                    id: key("weapon"),
                    origin: ActionSourceOrigin::PlayerEquipment { slot: id("weapon") },
                }],
                policy: ActionSourcePolicy::EquipmentEligibility {
                    source: key("weapon"),
                    capability: id("eligibility"),
                    when_empty: ActionSourceOutcome::Unavailable,
                    when_ineligible: ActionSourceOutcome::Unavailable,
                },
            }]))
    });
    f.input.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Capability {
            scope: RuleEntityKind::EquipmentUse,
            capability: id("eligibility"),
        },
        stage: key("finish"),
    });
    assert!(f.package().is_err());
}

#[test]
fn channel_types_duplicates_and_binding_changes_are_rejected() {
    for mutation in 0..6 {
        let mut f = Fixture::new();
        match mutation {
            0 => f
                .input
                .frozen_channels
                .push(f.input.frozen_channels[0].clone()),
            1 => {
                f.input.frozen_channels[0].channel = StageChannel::Stat {
                    scope: RuleEntityKind::Enemy,
                    stat: id("a"),
                }
            }
            2 => {
                f.input.frozen_channels[0].channel =
                    StageChannel::ModifierTransforms { stat: id("a") }
            }
            3 => f.input.routing = f.input.rules,
            4 => f.input.definitions.release = "changed".into(),
            _ => f.input.routing_stage = key("unknown"),
        }
        assert!(f.package().is_err(), "{mutation}");
    }
}

#[test]
fn storage_bounds_wire_fields_and_exact_resource_replay() {
    let f = Fixture::new();
    let p = f.package().unwrap();
    let use_ = p.resources();
    let exact = StageStorageLimits {
        max_stages: use_.stages,
        max_entries: use_.entries,
        max_work: use_.work,
        ..Default::default()
    };
    OwnedEvaluationStages::new(f.input.clone(), &f.schema, &f.rules, &f.routing, exact).unwrap();
    for short in [
        StageStorageLimits {
            max_stages: 2,
            ..Default::default()
        },
        StageStorageLimits {
            max_entries: use_.entries - 1,
            ..Default::default()
        },
        StageStorageLimits {
            max_work: use_.work - 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            OwnedEvaluationStages::new(f.input.clone(), &f.schema, &f.rules, &f.routing, short),
            Err(StageStorageError::Limit(_))
        ));
    }
    assert!(
        OwnedEvaluationStages::new(
            f.input.clone(),
            &f.schema,
            &f.rules,
            &f.routing,
            StageStorageLimits {
                max_wire_bytes: 1,
                ..Default::default()
            },
        )
        .is_err()
    );
    for (field, value) in [
        ("unknown", serde_json::json!(true)),
        ("schema_version", serde_json::json!(2)),
        ("programs", serde_json::Value::Null),
    ] {
        let mut wire = serde_json::to_value(&f.input).unwrap();
        wire[field] = value;
        assert!(
            decode_evaluation_stages(
                &serde_json::to_vec(&wire).unwrap(),
                &f.schema,
                &f.rules,
                &f.routing,
                StageStorageLimits::default()
            )
            .is_err()
        );
    }
}

#[path = "support/owned_readiness.rs"]
mod readiness;

#[test]
fn ordered_group_reads_obey_the_same_frozen_contribution_channel() {
    use poe_optimizer_core::owned_readiness::*;
    let mut f = Fixture::new();
    f.change_rules(|rules| {
        rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
        rules.effect_applications = Some(empty());
        // This storage-only fixture checks scheduling. An actual candidate with
        // an unlisted contributor still fails Engine's whole-query binding.
        rules.ordered_contributions = Some(DeclaredSet::complete(vec![OrderedContributionQuery {
            id: key("ordered"),
            stat: id("a"),
            contribution: ContributionKind::Add,
            groups: vec![OrderedContributionGroup {
                id: key("group"),
                reduction: ContributionReduction::Sum,
                empty: ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
                members: empty(),
            }],
        }]));
        rules.owners[1].programs.members[0].reads[0].source =
            RuleReadSource::OrderedContributions {
                entity: RuleEntity::Current,
                query: key("ordered"),
                group: key("group"),
            };
    });
    // V21 retains V18+'s readiness/stage envelope, with an explicitly empty
    // effect-application partition for this finite fixture.
    f.input.effect_applications = Some(empty());
    f.input.schema_version = OWNED_EVALUATION_STAGES_V3;
    f.input.readiness = Some(ReadinessInput {
        skills: vec![GeneratedSkillReadiness {
            skill: id("skill"),
            parameters: empty(),
        }],
        programs: DeclaredSet::complete(
            ["a", "b"]
                .into_iter()
                .map(|name| ReadinessProgram {
                    owner: owner(name),
                    program: key("same-local-id"),
                    phase: ReadinessPhase::Execution,
                    role: ReadinessProgramRole::Execution,
                    outputs: vec![],
                })
                .collect(),
        ),
    });
    f.input.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: id("a"),
            contribution: ContributionKind::Add,
        },
        stage: key("prepare"),
    });
    // Both programs retain ordinary execution readiness. The ordered read
    // itself confers no early preparation authority.
    f.package().unwrap();
    f.input.frozen_channels.last_mut().unwrap().stage = key("finish");
    assert!(matches!(
        f.package(),
        Err(StageStorageError::Invalid(
            "frozen channel read occurs before or outside frozen stage"
        ))
    ));
}
