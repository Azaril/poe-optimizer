//! Finite contribution tests with actual checked readiness and an empty support domain.
use super::support::*;
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
use std::sync::Arc;
// Same checked empty-support construction used by the publication component
// fixtures: all real programs execute, with no support/input values invented.
pub fn checked_plan(
    f: &Fixture,
    mut rules_input: RulePackageInput,
    mut routing_input: ActionRoutingInput,
    limits: PlanLimits,
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
    assert!(
        [OWNED_RULE_OPERATIONS_V22, OWNED_RULE_OPERATIONS_V23]
            .contains(&rules_input.operations_version.as_str())
    );
    // Actor-supplying Gems may have complete empty Skill exposure. They create
    // real provider/Actor occurrences but no support recipient in this fixture.
    for gem in &build.gems {
        let row = f
            .schema
            .definitions
            .iter()
            .find_map(|row| match row {
                DefinitionDescriptor::Gem(row) if row.id == gem.definition => Some(row),
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(schema) = &row.schema else {
            panic!()
        };
        assert!(schema.skills.is_complete() && schema.skills.members.is_empty());
    }
    for skill in &build.skills {
        assert!(
            matches!(skill.source, AuthoredSkillSource::Gem(id) if build.gems.iter().any(|g| g.id == id))
        );
    }
    assert!(build.supports.is_empty());
    assert!(
        build
            .generated_inputs
            .as_ref()
            .is_none_or(|v| v.schema_version == 1 && v.bindings.is_empty())
    );
    assert!(build.support_origins.as_ref().is_none_or(Vec::is_empty));
    assert_eq!(
        rules_input.effect_applications,
        Some(DeclaredSet::complete(vec![]))
    );
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
    ];
    let mut schema_input = f.schema.clone();
    assert!(
        !schema_input
            .definitions
            .iter()
            .any(|d| matches!(d, DefinitionDescriptor::Skill(_)))
    );
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
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            EvaluationStagesInput {
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
                        id: key("execute"),
                        predecessors: vec![key("prepare")],
                    },
                ],
                programs: DeclaredSet::complete(
                    programs
                        .iter()
                        .map(|(o, p)| StagedRuleProgram {
                            owner: o.owner.clone(),
                            program: p.id.clone(),
                            stage: key("execute"),
                        })
                        .collect(),
                ),
                effect_applications: Some(DeclaredSet::complete(vec![])),
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
                    .collect(),
                readiness: Some(ReadinessInput {
                    skills: vec![],
                    programs: DeclaredSet::complete(
                        programs
                            .iter()
                            .map(|(o, p)| ReadinessProgram {
                                owner: o.owner.clone(),
                                program: p.id.clone(),
                                phase: ReadinessPhase::Execution,
                                role: ReadinessProgramRole::Execution,
                                outputs: vec![],
                            })
                            .collect(),
                    ),
                }),
            },
            schema.as_ref(),
            &stored,
            &routing,
            Default::default(),
        )
        .unwrap(),
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
                    is_player_actor: presence,
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
    build.support_origins = Some(vec![]);
    build.generated_inputs = Some(GeneratedSkillInputsV1 {
        schema_version: 1,
        bindings: vec![],
    });
    let input_limits = OwnedInputLimits::default();
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(build, input_limits).unwrap(),
        ScenarioSpec::new(scenario.clone(), input_limits).unwrap(),
        QuerySpec::new(
            QueryInput {
                game_version: namespace.clone(),
                requests: vec![],
            },
            input_limits,
        )
        .unwrap(),
        input_limits,
    )
    .unwrap();
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
        limits,
        Default::default(),
    )
}
