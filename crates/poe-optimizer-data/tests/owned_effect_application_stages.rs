#[allow(dead_code)]
#[path = "support/owned_effect_application_fixture.rs"]
mod fixture;

use fixture::*;
use poe_optimizer_core::{owned_routing::*, owned_rules::*, owned_schema::*, owned_stages::*};
use poe_optimizer_data::{owned_routing::*, owned_rules::*, owned_schema::*, owned_stages::*};

struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    rules: OwnedRulePackage,
    routing: OwnedActionRouting,
    stages: EvaluationStagesInput,
}
impl Fixture {
    fn new(source_stat: bool) -> Self {
        let schema = schema();
        let mut input = rules(&schema);
        if source_stat {
            input.effect_applications.as_mut().unwrap().members[0]
                .program
                .reads[1]
                .source = RuleReadSource::Stat {
                entity: RuleEntity::EffectSource,
                stat: id("source-scalar"),
            };
        }
        let mut second = input.effect_applications.as_ref().unwrap().members[0].clone();
        second.id = key("second");
        input
            .effect_applications
            .as_mut()
            .unwrap()
            .members
            .push(second);
        let rules = OwnedRulePackage::new(input, &schema, RuleStorageLimits::default()).unwrap();
        let routing = OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            &schema,
            RoutingLimits::default(),
        )
        .unwrap();
        let stages = EvaluationStagesInput {
            readiness: None,
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
                    id: key("apply"),
                    predecessors: vec![key("prepare")],
                },
                EvaluationStage {
                    id: key("finish"),
                    predecessors: vec![key("apply")],
                },
            ],
            programs: DeclaredSet::complete(vec![]),
            effect_applications: Some(DeclaredSet::complete(vec![
                StagedEffectApplication {
                    application: key("first"),
                    stage: key("apply"),
                },
                StagedEffectApplication {
                    application: key("second"),
                    stage: key("apply"),
                },
            ])),
            routing_stage: key("finish"),
            frozen_channels: vec![],
        };
        Self {
            schema,
            rules,
            routing,
            stages,
        }
    }
    fn load(
        &self,
        input: EvaluationStagesInput,
    ) -> Result<OwnedEvaluationStages, StageStorageError> {
        OwnedEvaluationStages::new(
            input,
            &self.schema,
            &self.rules,
            &self.routing,
            StageStorageLimits::default(),
        )
    }
}

#[test]
fn application_classification_is_explicit_canonical_and_bound_to_actual_rules() {
    let fixture = Fixture::new(false);
    let initial = fixture.load(fixture.stages.clone()).unwrap();
    assert!(initial.is_complete());
    assert_eq!(
        initial.stage_for_application(&key("first")),
        Some(&key("apply"))
    );
    assert_eq!(initial.stage_for_application(&key("absent")), None);
    let mut reordered = fixture.stages.clone();
    reordered
        .effect_applications
        .as_mut()
        .unwrap()
        .members
        .reverse();
    let reordered = fixture.load(reordered).unwrap();
    assert_eq!(initial.identity(), reordered.identity());
    let bytes = encode_evaluation_stages(&initial, StageStorageLimits::default()).unwrap();
    let decoded = decode_evaluation_stages(
        &bytes,
        &fixture.schema,
        &fixture.rules,
        &fixture.routing,
        StageStorageLimits::default(),
    )
    .unwrap();
    assert_eq!(initial.identity(), decoded.identity());
}

#[test]
fn missing_duplicate_foreign_and_cross_stage_application_groups_reject() {
    let fixture = Fixture::new(false);
    for case in 0..6 {
        let mut input = fixture.stages.clone();
        let expected = match case {
            0 => {
                input.effect_applications = None;
                "v15 requires explicit effect application stage membership"
            }
            1 => {
                input.effect_applications.as_mut().unwrap().members.pop();
                "complete stage partition omits effect applications"
            }
            2 => {
                input.effect_applications.as_mut().unwrap().members[1].application = key("first");
                "duplicate effect application stage classification"
            }
            3 => {
                input.effect_applications.as_mut().unwrap().members[1].application = key("absent");
                "unknown staged effect application"
            }
            4 => {
                input.effect_applications.as_mut().unwrap().members[1].stage = key("absent");
                "unknown effect application stage"
            }
            _ => {
                input.effect_applications.as_mut().unwrap().members[1].stage = key("finish");
                "effect stacking group spans incompatible stages"
            }
        };
        assert!(
            matches!(fixture.load(input), Err(StageStorageError::Invalid(message)) if message == expected),
            "expected {expected}"
        );
    }
}

#[test]
fn partial_application_partition_never_becomes_complete_through_owner_partition() {
    let fixture = Fixture::new(false);
    let mut input = fixture.stages.clone();
    input.effect_applications = Some(DeclaredSet::partial(vec![], vec![gap()]));
    let partial = fixture.load(input.clone()).unwrap();
    assert!(partial.input().programs.is_complete());
    assert!(!partial.is_complete());
    assert_eq!(partial.stage_for_application(&key("first")), None);
    input.effect_applications.as_mut().unwrap().closure = SchemaClosure::Partial { gaps: vec![] };
    assert!(matches!(
        fixture.load(input),
        Err(StageStorageError::Invalid(
            "partial effect application stages need gap evidence"
        ))
    ));
}

#[test]
fn source_parameters_and_source_stat_scope_participate_in_frozen_channel_checks() {
    for source_stat in [false, true] {
        let fixture = Fixture::new(source_stat);
        let mut input = fixture.stages.clone();
        let channel = if source_stat {
            StageChannel::Stat {
                scope: RuleEntityKind::Skill,
                stat: id("source-scalar"),
            }
        } else {
            StageChannel::SkillParameter {
                parameter: parameter(),
            }
        };
        input.frozen_channels.push(FrozenStageChannel {
            channel,
            stage: key("prepare"),
        });
        fixture.load(input.clone()).unwrap();
        input.frozen_channels[0].stage = key("finish");
        assert!(matches!(
            fixture.load(input),
            Err(StageStorageError::Invalid(
                "frozen channel read occurs before or outside frozen stage"
            ))
        ));
    }
}

#[test]
fn recipient_contribution_writes_cannot_bypass_a_frozen_channel() {
    let fixture = Fixture::new(false);
    let mut input = fixture.stages.clone();
    input.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: id("final"),
            contribution: ContributionKind::Increase,
        },
        stage: key("prepare"),
    });
    assert!(matches!(
        fixture.load(input),
        Err(StageStorageError::Invalid(
            "potential writer occurs after or outside frozen stage"
        ))
    ));
}

#[test]
fn legacy_stage_bytes_omit_new_inventory_and_do_not_accept_unversioned_applications() {
    let mut fixture = Fixture::new(false);
    let mut input = fixture.rules.input().clone();
    input.operations_version = key(OWNED_RULE_OPERATIONS_V14);
    input.effect_applications = None;
    fixture.rules =
        OwnedRulePackage::new(input, &fixture.schema, RuleStorageLimits::default()).unwrap();
    fixture.stages.rules = *fixture.rules.identity();
    let mut old = fixture.stages.clone();
    old.effect_applications = None;
    let old = fixture.load(old).unwrap();
    assert!(
        serde_json::to_value(old.input())
            .unwrap()
            .get("effect_applications")
            .is_none()
    );
    assert!(old.is_complete());
    assert!(matches!(
        fixture.load(fixture.stages.clone()),
        Err(StageStorageError::Invalid(
            "effect application stages require owned-domain-operations-v15"
        ))
    ));
}

#[test]
fn application_stage_work_is_counted_before_graph_access() {
    let fixture = Fixture::new(false);
    let used = fixture.load(fixture.stages.clone()).unwrap().resources();
    let limits = StageStorageLimits {
        max_work: used.work - 1,
        ..StageStorageLimits::default()
    };
    assert!(matches!(
        OwnedEvaluationStages::new(
            fixture.stages.clone(),
            &fixture.schema,
            &fixture.rules,
            &fixture.routing,
            limits
        ),
        Err(StageStorageError::Limit("work"))
    ));
}
