//! Native retained support applications close the ordinary graph in one attempt.
#[allow(dead_code)]
#[path = "owned_computed_support_fixture.rs"]
pub mod fixture;
pub use fixture::{Fixture, child_actor, def, effect, integer, key, occurrence, subject, target};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
    owned_stages::*, owned_support_receiving::*, owned_supports::SupportPreparationInput,
};
use poe_optimizer_data::{
    owned_rules::OwnedRulePackage, owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::OwnedEvaluationStages, owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::*, owned_supports::*};
use std::sync::Arc;

pub type Inputs = SupportEffectPlanInputs<OwnedDefinitionSchemaPackage>;
pub type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;

pub fn support_owner() -> SchemaSubject {
    subject(def::<GemDefinition>("support"))
}
pub fn output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("ability")),
        slot: def("receiving-output"),
    }
}
pub fn final_program(name: &str, context: RuleEntityKind) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context,
        reads: vec![fixture::contributions(
            "incoming",
            RuleEntity::Current,
            "support-total",
        )],
        nodes: vec![fixture::read_node("value", "incoming")],
        effects: vec![fixture::derive(
            "total",
            RuleEntity::Current,
            "support-total",
            "value",
        )],
    }
}
pub fn source_fixture() -> Fixture {
    let mut f = fixture::generated_fixture();
    f.schema.definitions.extend([
        DefinitionDescriptor::ActionPart(DefinitionEntry {
            id: def("part-two"),
            schema: SchemaState::Known(ActionPartSchema {}),
        }),
        DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("support-total"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor, RuleEntityKind::Action],
            }),
        }),
    ]);
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(DefinitionEntry {
            id,
            schema: SchemaState::Known(schema),
        }) = definition
            && *id == def::<SkillDefinition>("ability")
        {
            schema.declarations.outputs.members.push(output());
        }
    }
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::SkillGrant(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = slot
            && schema.skill == def::<SkillDefinition>("ability")
        {
            schema.outputs.members.push(output());
        }
    }
    f.schema
        .slots
        .push(SlotDescriptor::ActionOutput(DefinitionEntry {
            id: output(),
            schema: SchemaState::Known(ActionOutputSchema {
                actor_role: DeclaredActorRole::ProviderActor,
                parts: DeclaredSet::complete(vec![def("part"), def("part-two")]),
                modes: DeclaredSet::complete(vec![def("mode")]),
                stat_sets: DeclaredSet::complete(vec![def("set")]),
                choices: DeclaredSet::complete(vec![]),
            }),
        }));
    f.routes.push(ActionOutputRoutes {
        output: output(),
        routes: DeclaredSet::complete(vec![]),
        source_selectors: Some(DeclaredSet::complete(vec![])),
    });
    f.owner_mut(&subject(def::<ActorDefinition>("family")))
        .programs
        .members
        .push(final_program("actor-final", RuleEntityKind::Actor));
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(SlotAddress::ActionOutput(output())),
        programs: DeclaredSet::complete(vec![final_program(
            "action-final",
            RuleEntityKind::Action,
        )]),
    });
    f
}
pub fn support_program(
    name: &str,
    context: RuleEntityKind,
    applicability: Option<bool>,
) -> RuleProgram {
    if let Some(value) = applicability {
        return RuleProgram {
            id: key(name),
            context,
            reads: vec![],
            nodes: vec![fixture::bool_node("answer", value)],
            effects: vec![effect(
                "applicable",
                RuleEffectKind::SupportApplicability {
                    applicable: key("answer"),
                },
            )],
        };
    }
    RuleProgram {
        id: key(name),
        context,
        // Both reads remain physically assignment-owned even on generated receivers.
        reads: vec![
            fixture::read("raw", RuleReadSource::GemLevel),
            fixture::read(
                "effective",
                RuleReadSource::Stat {
                    entity: RuleEntity::SupportOrigin,
                    stat: def("effective-level"),
                },
            ),
        ],
        nodes: vec![
            fixture::read_node("raw", "raw"),
            fixture::read_node("effective", "effective"),
            fixture::node(
                "value",
                RuleExpression::Add {
                    left: key("raw"),
                    right: key("effective"),
                },
            ),
        ],
        effects: vec![effect(
            "contribution",
            RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def("support-total"),
                contribution: ContributionKind::Add,
                value: key("value"),
            },
        )],
    }
}
pub fn inputs(f: &Fixture, edit: impl FnOnce(&mut SupportReceivingInput)) -> Inputs {
    inputs_with_packages(f, |_| {}, |_| {}, edit)
}
pub fn inputs_with_rules(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit: impl FnOnce(&mut SupportReceivingInput),
) -> Inputs {
    inputs_with_packages(f, edit_rules, |_| {}, edit)
}
pub fn inputs_with_preparation(
    f: &Fixture,
    edit_preparation: impl FnOnce(&mut SupportPreparationInput),
    edit: impl FnOnce(&mut SupportReceivingInput),
) -> Inputs {
    inputs_with_packages(f, |_| {}, edit_preparation, edit)
}
pub fn inputs_with_packages(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_preparation: impl FnOnce(&mut SupportPreparationInput),
    edit: impl FnOnce(&mut SupportReceivingInput),
) -> Inputs {
    inputs_with_stage_packages(f, edit_rules, edit_preparation, |_| {}, edit)
}
pub fn inputs_with_stage_packages(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_preparation: impl FnOnce(&mut SupportPreparationInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit: impl FnOnce(&mut SupportReceivingInput),
) -> Inputs {
    let base = fixture::compile_inputs(f, target(30, "first"));
    let mut rule_input = base.rules.input().clone();
    rule_input.operations_version = key(OWNED_RULE_OPERATIONS_V13);
    rule_input
        .owners
        .iter_mut()
        .find(|row| row.owner == support_owner())
        .unwrap()
        .programs
        .members
        .extend([
            support_program("actor-app", RuleEntityKind::Actor, Some(true)),
            support_program("action-app", RuleEntityKind::Action, Some(true)),
            support_program("actor-deliver", RuleEntityKind::Actor, None),
            support_program("action-deliver", RuleEntityKind::Action, None),
        ]);
    edit_rules(&mut rule_input);
    let stored =
        OwnedRulePackage::new(rule_input, base.definitions.as_ref(), Default::default()).unwrap();
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(
            &stored,
            base.definitions.as_ref(),
            RuleLimits::default(),
        )
        .unwrap(),
    );
    let mut stage_input = base.stages.input().clone();
    stage_input.rules = *stored.identity();
    stage_input.stages.extend([
        EvaluationStage {
            id: key("apply"),
            predecessors: vec![key("prepare")],
        },
        EvaluationStage {
            id: key("deliver"),
            predecessors: vec![key("apply")],
        },
        EvaluationStage {
            id: key("final"),
            predecessors: vec![key("deliver")],
        },
    ]);
    for row in &mut stage_input.programs.members {
        if [key("actor-final"), key("action-final")].contains(&row.program) {
            row.stage = key("final");
        }
    }
    for (name, stage) in [
        ("actor-app", "apply"),
        ("action-app", "apply"),
        ("actor-deliver", "deliver"),
        ("action-deliver", "deliver"),
    ] {
        stage_input.programs.members.push(StagedRuleProgram {
            owner: support_owner(),
            program: key(name),
            stage: key(stage),
        });
    }
    edit_stages(&mut stage_input);
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            stage_input,
            base.definitions.as_ref(),
            &stored,
            &base.routing,
            Default::default(),
        )
        .unwrap(),
    );
    let mut prep_input = base.preparation.input().clone();
    prep_input.rules = *stored.identity();
    edit_preparation(&mut prep_input);
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            prep_input,
            base.definitions.as_ref(),
            &stored,
            Default::default(),
        )
        .unwrap(),
    );
    let mut input_input = base.inputs.input().clone();
    input_input.rules = *stored.identity();
    input_input.preparation = *preparation.identity();
    input_input.stages = *stages.identity();
    let bindings = Arc::new(
        OwnedSupportInputBindings::new(
            input_input,
            base.definitions.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let mut receiving_input = SupportReceivingInput {
        schema_version: OWNED_SUPPORT_RECEIVING_VERSION,
        namespace: fixture::ns(),
        release: key("receiving"),
        definitions: base.definitions.identity().clone(),
        rules: *stored.identity(),
        preparation: *preparation.identity(),
        inputs: *bindings.identity(),
        stages: *stages.identity(),
        roles: vec![
            SupportReceivingRole {
                id: key("actor"),
                kind: SupportReceiverKind::Actor,
            },
            SupportReceivingRole {
                id: key("action"),
                kind: SupportReceiverKind::Action,
            },
        ],
        targets: vec![SupportTargetReceivingRoles {
            owner: SupportTargetDefinition::Skill(def("ability")),
            roles: DeclaredSet::complete(vec![
                SupportReceivingRoleBinding {
                    role: key("actor"),
                    endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Actor {
                        path: vec![],
                        admission: SupportAdmissionContext::AssignedSkill,
                    }]),
                },
                SupportReceivingRoleBinding {
                    role: key("action"),
                    endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Action {
                        path: vec![],
                        output: output(),
                        selection: SupportActionSelection::AllDeclared,
                        admission: SupportAdmissionContext::AssignedSkill,
                    }]),
                },
            ]),
        }],
        supports: vec![SupportReceivingEntry {
            gem: def("support"),
            receivers: DeclaredSet::complete(vec![
                SupportRolePrograms {
                    role: key("actor"),
                    applicability: key("actor-app"),
                    delivery: vec![key("actor-deliver")],
                },
                SupportRolePrograms {
                    role: key("action"),
                    applicability: key("action-app"),
                    delivery: vec![key("action-deliver")],
                },
            ]),
        }],
    };
    edit(&mut receiving_input);
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            receiving_input,
            base.definitions.as_ref(),
            &stored,
            &preparation,
            &bindings,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    Inputs {
        request: base.request,
        definitions: base.definitions,
        rules,
        routing: base.routing,
        stages,
        preparation,
        inputs: bindings,
        receiving,
    }
}
pub fn compile(f: &Fixture) -> Plan {
    Plan::compile(
        inputs(f, |_| {}),
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap()
}
pub fn evaluated(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("expected complete native effects: {report:?}")
    };
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    effects
}
pub fn application(effect: &BoundEffectResult) -> Option<&SupportApplicationKey> {
    if let RuleOrigin::SupportApplication { application } = &effect.key.invocation.origin {
        Some(application)
    } else {
        None
    }
}
pub fn known(value: i64) -> EffectValue {
    EffectValue::Known {
        value: integer(value),
    }
}
pub fn final_value<'a>(report: &'a OwnedEffectsReport, entity: &ConcreteEntity) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: entity.clone(),
                    stat: def("support-total"),
                }
        })
        .unwrap()
        .value
}
pub fn partial(owner: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::GameRules,
            code: key("pending"),
        }],
    }
}
