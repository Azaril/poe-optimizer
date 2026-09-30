//! Explicit final-type channels over injected support preparation data.
#[allow(dead_code)]
#[path = "owned_support_delivery_fixture.rs"]
pub mod delivery;
pub use delivery::{Fixture, child_actor, def, fixture, key, subject, target};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*, owned_stages::*,
    owned_support_inputs::*, owned_support_outputs::*, owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_data::{owned_rules::OwnedRulePackage, owned_support_outputs::*};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use std::sync::Arc;

pub fn output_bindings(
    args: &delivery::Inputs,
    stored: &OwnedRulePackage,
    output_stage: OwnedDefinitionKey,
    final_skill_types: Vec<SupportTypeStat>,
) -> Arc<OwnedSupportOutputBindings> {
    assert_eq!(args.rules.source_identity(), Some(*stored.identity()));
    Arc::new(
        OwnedSupportOutputBindings::new(
            SupportOutputBindingsInput {
                schema_version: OWNED_SUPPORT_OUTPUT_BINDINGS_VERSION,
                namespace: fixture::ns(),
                release: key("outputs"),
                definitions: args.definitions.identity().clone(),
                rules: *stored.identity(),
                preparation: *args.preparation.identity(),
                inputs: *args.inputs.identity(),
                receiving: *args.receiving.identity(),
                stages: *args.stages.identity(),
                output_stage,
                final_skill_types,
            },
            &SupportOutputDependencies {
                definitions: args.definitions.as_ref(),
                rules: stored,
                preparation: &args.preparation,
                inputs: &args.inputs,
                receiving: &args.receiving,
                stages: &args.stages,
            },
            SupportOutputStorageLimits::default(),
        )
        .unwrap(),
    )
}
pub fn source_fixture() -> Fixture {
    let mut f = delivery::source_fixture();
    for name in ["duration", "prepared-spell", "prepared-duration"] {
        f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: def(name),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Skill],
                }),
            }));
    }
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("prepared-result"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Action],
            }),
        }));
    let facts = f
        .owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("target-facts"))
        .unwrap();
    facts.nodes.push(fixture::bool_node("duration", false));
    facts.effects.push(fixture::derive(
        "duration",
        RuleEntity::Current,
        "duration",
        "duration",
    ));
    let downstream = &mut f
        .owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(
            delivery::output(),
        )))
        .programs
        .members[0];
    downstream.reads.push(RuleRead {
        id: key("prepared"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Stat {
            entity: RuleEntity::Skill,
            stat: def("prepared-duration"),
        },
    });
    downstream.nodes.extend([
        fixture::read_node("prepared", "prepared"),
        fixture::literal("empty", 0),
        fixture::node(
            "prepared-result",
            RuleExpression::Select {
                condition: key("prepared"),
                when_true: key("value"),
                when_false: key("empty"),
            },
        ),
    ]);
    downstream.effects.push(fixture::derive(
        "prepared-result",
        RuleEntity::Current,
        "prepared-result",
        "prepared-result",
    ));
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Metric(row) = definition {
            row.schema = SchemaState::Known(MetricSchema {
                targets: vec![MetricTargetKind::Actor, MetricTargetKind::Action],
                unit: def("count"),
                actor_roles: vec![MetricActorRole::Player, MetricActorRole::Owned],
                provider_roles: vec![ProviderRole::Character, ProviderRole::SkillUse],
            });
        }
    }
    f
}
pub fn action(use_id: u64, name: &str) -> ActionSelection {
    let SkillTarget::Generated(target) = target(use_id, name) else {
        unreachable!()
    };
    let mut provider = target.provider;
    provider.grant_path.push(DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def(name),
    });
    ActionSelection {
        action: ActionKey {
            actor: child_actor(use_id),
            provider,
            output: delivery::output(),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
pub fn inputs(f: &Fixture) -> (delivery::Inputs, Arc<OwnedSupportOutputBindings>) {
    inputs_with(f, |_| {}, |_| {}, |_| {}, |_| {})
}
pub fn inputs_with(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_preparation: impl FnOnce(&mut SupportPreparationInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit_receiving: impl FnOnce(&mut SupportReceivingInput),
) -> (delivery::Inputs, Arc<OwnedSupportOutputBindings>) {
    let (args, _, outputs) =
        authored_inputs_with(f, edit_rules, edit_preparation, edit_stages, edit_receiving);
    (args, outputs)
}
pub fn authored_inputs(
    f: &Fixture,
) -> (
    delivery::Inputs,
    OwnedRulePackage,
    Arc<OwnedSupportOutputBindings>,
) {
    authored_inputs_with(f, |_| {}, |_| {}, |_| {}, |_| {})
}
pub fn authored_inputs_with(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_preparation: impl FnOnce(&mut SupportPreparationInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit_receiving: impl FnOnce(&mut SupportReceivingInput),
) -> (
    delivery::Inputs,
    OwnedRulePackage,
    Arc<OwnedSupportOutputBindings>,
) {
    let mut authored = None;
    let args = delivery::inputs_with_all_packages(
        f,
        |rules| {
            edit_rules(rules);
            authored = Some(rules.clone());
        },
        |preparation| {
            preparation.types.push(key("duration"));
            let SchemaState::Known(definition) = &mut preparation.supports[0].preparation else {
                unreachable!()
            };
            definition.added_types.push(key("duration"));
            edit_preparation(preparation);
        },
        |stages| {
            stages.stages.push(EvaluationStage {
                id: key("prepared-types"),
                predecessors: vec![key("prepare")],
            });
            stages
                .stages
                .iter_mut()
                .find(|s| s.id == key("apply"))
                .unwrap()
                .predecessors = vec![key("prepared-types")];
            stages.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat: def("duration"),
                },
                stage: key("prepare"),
            });
            for name in ["prepared-spell", "prepared-duration"] {
                stages.frozen_channels.push(FrozenStageChannel {
                    channel: StageChannel::Stat {
                        scope: RuleEntityKind::Skill,
                        stat: def(name),
                    },
                    stage: key("prepared-types"),
                });
            }
            edit_stages(stages);
        },
        |inputs| {
            for rows in [
                &mut inputs.target.skill_types,
                &mut inputs.target.minion_types.members,
                &mut inputs.target.summoner.skill_types,
                &mut inputs.target.summoner.minion_types.members,
            ] {
                rows.push(SupportTypeStat {
                    support_type: key("duration"),
                    stat: def("duration"),
                });
            }
        },
        edit_receiving,
    );
    let stored = OwnedRulePackage::new(
        authored.unwrap(),
        args.definitions.as_ref(),
        Default::default(),
    )
    .unwrap();
    let outputs = output_bindings(
        &args,
        &stored,
        key("prepared-types"),
        vec![
            SupportTypeStat {
                support_type: key("spell"),
                stat: def("prepared-spell"),
            },
            SupportTypeStat {
                support_type: key("duration"),
                stat: def("prepared-duration"),
            },
        ],
    );
    (args, stored, outputs)
}
pub fn compile(f: &Fixture) -> delivery::Plan {
    let (args, outputs) = inputs(f);
    OwnedSupportEffectPlan::compile_with_outputs(
        args,
        outputs,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap()
}
pub fn type_value<'a>(
    report: &'a OwnedEffectsReport,
    target: &SkillTarget,
    stat: &str,
) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target.clone())),
                    stat: def(stat),
                }
        })
        .unwrap()
        .value
}
pub fn boolean(value: bool) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Boolean(value),
    }
}
