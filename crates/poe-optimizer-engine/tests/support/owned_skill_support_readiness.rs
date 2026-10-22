//! The same occurrence supplies preparation even when execution participation is false.
pub(super) use crate::engine_fixtures::participation;
use participation::{base, delivery, readiness, *};
use poe_optimizer_core::{
    owned_build::*, owned_readiness::*, owned_rules::*, owned_schema::*, owned_stages::*,
    owned_support_receiving::*,
};
use poe_optimizer_engine::owned_plan::*;
use std::collections::BTreeSet;

fn fixture() -> Fixture {
    let mut f = participation::fixture();
    for name in [
        "skill-prepared-property",
        "skill-executed-property",
        "observed-skill-property",
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            def(name),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Skill],
            },
        )));
    }
    f.owner_mut(&readiness::child_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("observe-skill-property"),
            context: RuleEntityKind::Skill,
            reads: vec![base::contributions(
                "properties",
                RuleEntity::Current,
                "skill-prepared-property",
            )],
            nodes: vec![base::read_node("value", "properties")],
            effects: vec![base::derive(
                "observed",
                RuleEntity::Current,
                "observed-skill-property",
                "value",
            )],
        });
    f
}
fn edit_rules(rules: &mut RulePackageInput) {
    let owner = rules
        .owners
        .iter_mut()
        .find(|o| o.owner == delivery::support_owner())
        .unwrap();
    let mut property = readiness::property_program();
    property.id = key("prepared-skill-property");
    property.context = RuleEntityKind::Skill;
    let RuleEffectKind::Contribute { stat, .. } = &mut property.effects[0].effect else {
        unreachable!()
    };
    *stat = def("skill-prepared-property");
    let mut delivery = delivery::support_program("skill-deliver", RuleEntityKind::Skill, None);
    let RuleEffectKind::Contribute { stat, .. } = &mut delivery.effects[0].effect else {
        unreachable!()
    };
    *stat = def("skill-executed-property");
    owner.programs.members.extend([
        delivery::support_program("prepared-skill-app", RuleEntityKind::Skill, Some(true)),
        property,
        delivery::support_program("skill-app", RuleEntityKind::Skill, Some(true)),
        delivery,
    ]);
}
fn property_channel() -> StageChannel {
    StageChannel::Contributions {
        scope: RuleEntityKind::Skill,
        stat: def("skill-prepared-property"),
        contribution: ContributionKind::Add,
    }
}
fn edit_stages(stages: &mut EvaluationStagesInput) {
    for (name, stage, phase, role, outputs) in [
        (
            "prepared-skill-app",
            "property-app",
            ReadinessPhase::Preparation,
            ReadinessProgramRole::SupportPreparationApplicability,
            vec![],
        ),
        (
            "prepared-skill-property",
            "properties",
            ReadinessPhase::Preparation,
            ReadinessProgramRole::SupportedPreparationProperty,
            vec![property_channel()],
        ),
        (
            "skill-app",
            "apply",
            ReadinessPhase::Execution,
            ReadinessProgramRole::Execution,
            vec![],
        ),
        (
            "skill-deliver",
            "deliver",
            ReadinessPhase::Execution,
            ReadinessProgramRole::Execution,
            vec![],
        ),
    ] {
        let staged = stages
            .programs
            .members
            .iter_mut()
            .find(|p| p.owner == delivery::support_owner() && p.program == key(name))
            .unwrap();
        staged.stage = key(stage);
        let readiness = stages
            .readiness
            .as_mut()
            .unwrap()
            .programs
            .members
            .iter_mut()
            .find(|p| p.owner == delivery::support_owner() && p.program == key(name))
            .unwrap();
        readiness.phase = phase;
        readiness.role = role;
        readiness.outputs = outputs;
    }
    assert!(
        !stages
            .frozen_channels
            .iter()
            .any(|row| row.channel == property_channel())
    );
    stages.frozen_channels.push(FrozenStageChannel {
        channel: property_channel(),
        stage: key("properties"),
    });
}
fn edit_receiving(receiving: &mut SupportReceivingInput) {
    receiving.roles.push(SupportReceivingRole {
        id: key("skill"),
        kind: SupportReceiverKind::Skill,
    });
    assert_eq!(receiving.targets.len(), 1);
    assert_eq!(
        receiving.targets[0].owner,
        SupportTargetDefinition::Skill(def("ability"))
    );
    receiving.targets[0]
        .roles
        .members
        .push(SupportReceivingRoleBinding {
            role: key("skill"),
            endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Skill {
                path: vec![],
                admission: SupportAdmissionContext::AssignedSkill,
            }]),
        });
    assert_eq!(receiving.supports.len(), 1);
    receiving.supports[0]
        .receivers
        .members
        .push(SupportRolePrograms {
            role: key("skill"),
            applicability: key("skill-app"),
            delivery: vec![key("skill-deliver")],
            preparation: Some(SupportPreparationPrograms {
                applicability: key("prepared-skill-app"),
                properties: vec![key("prepared-skill-property")],
            }),
        });
}
fn inputs(f: &Fixture, stages: impl FnOnce(&mut EvaluationStagesInput)) -> Checked<Inputs> {
    participation::inputs_with(
        f,
        edit_rules,
        |s| {
            edit_stages(s);
            stages(s);
        },
        edit_receiving,
    )
}
fn compile(f: &Fixture) -> Checked<Effects> {
    readiness::compile_inputs(inputs(f, |_| {})?)
}
fn rows<'a>(report: &'a OwnedEffectsReport, name: &str) -> Vec<&'a BoundEffectResult> {
    report
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(name))
        .collect()
}
fn receiver(effect: &BoundEffectResult) -> &SkillTarget {
    let RuleOrigin::SupportApplication { application } = &effect.key.invocation.origin else {
        panic!("exact support application origin");
    };
    let SupportReceiverKey::Skill(skill) = &application.receiver else {
        panic!("exact Skill receiver")
    };
    assert_eq!(
        effect.key.invocation.entity,
        ConcreteEntity::Skill(skill.clone())
    );
    skill
}
fn prepared(report: &OwnedEffectsReport) {
    let rows = rows(report, "prepared-skill-property");
    assert_eq!(rows.len(), 3);
    let mut targets = BTreeSet::new();
    let mut origins = BTreeSet::new();
    for effect in rows {
        let target = receiver(effect);
        let RuleOrigin::SupportApplication { application } = &effect.key.invocation.origin else {
            unreachable!()
        };
        assert_eq!(&application.prepared.target, target);
        origins.insert(application.prepared.origin.clone());
        targets.insert(target.clone());
        let value = if *target == readiness::target(31, "first") {
            23
        } else {
            assert!(
                [
                    readiness::target(30, "first"),
                    readiness::target(30, "second")
                ]
                .contains(target)
            );
            14
        };
        assert_eq!(effect.value, delivery::known(value));
    }
    assert_eq!(
        targets,
        [(30, "first"), (30, "second"), (31, "first")]
            .into_iter()
            .map(|(id, name)| readiness::target(id, name))
            .collect()
    );
    assert_eq!(
        origins,
        [61, 63, 65]
            .into_iter()
            .map(|id| SupportOrigin::Assignment(occurrence(id)))
            .collect()
    );
}

#[test]
fn skill_preparation_and_delivery_keep_exact_occurrences_and_source_assignments() {
    let f = fixture();
    let plan = compile(&f).unwrap();
    let result = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let report = delivery::evaluated(&result);
    prepared(report);
    let delivered = rows(report, "skill-deliver");
    assert_eq!(delivered.len(), 3);
    for effect in delivered {
        let target = receiver(effect);
        let RuleOrigin::SupportApplication { application } = &effect.key.invocation.origin else {
            unreachable!()
        };
        let SupportOrigin::Assignment(id) = &application.prepared.origin;
        let assignment = f.build.supports.iter().find(|row| row.id == *id).unwrap();
        let raw_level = f
            .build
            .gems
            .iter()
            .find(|gem| gem.id == assignment.support)
            .unwrap()
            .level;
        let expected = if *target == readiness::target(31, "first") {
            23
        } else {
            14
        };
        assert_eq!(
            effect.value,
            delivery::known(i64::from(raw_level) + expected)
        );
    }
    for (id, name, value) in [(30, "first", 14), (30, "second", 14), (31, "first", 23)] {
        let row = report
            .values
            .iter()
            .find(|row| {
                row.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Skill(Box::new(readiness::target(id, name))),
                        stat: def("observed-skill-property"),
                    }
            })
            .unwrap();
        assert_eq!(row.value, delivery::known(value));
    }
    // Existing Actor/Action work remains unchanged by adding a Skill role.
    let metrics = readiness::metrics(plan);
    assert_eq!(
        readiness::numbers(&metrics.evaluate(&mut metrics.new_scratch()).unwrap()),
        [40., 40., 44.]
    );
}

#[test]
fn false_or_unknown_participation_preserves_skill_preparation_but_gates_execution() {
    for requested in [Some(false), None] {
        let mut f = fixture();
        participation::set(&mut f, readiness::target(30, "first"), requested);
        let plan = compile(&f).unwrap();
        let result = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let report = delivery::evaluated(&result);
        prepared(report);
        assert_eq!(
            participation::final_level(report, 30, "first"),
            &delivery::known(40)
        );
        let delivered = rows(report, "skill-deliver");
        assert_eq!(delivered.len(), 3);
        for effect in delivered {
            if *receiver(effect) == readiness::target(30, "first") {
                if requested == Some(false) {
                    assert_eq!(effect.value, EffectValue::Inactive);
                } else {
                    assert!(
                        matches!(effect.value, EffectValue::Unresolved { .. }),
                        "{:?}",
                        effect.value
                    );
                }
            } else {
                assert!(matches!(effect.value, EffectValue::Known { .. }));
            }
        }
    }
}

#[test]
fn skill_receiver_keeps_preparation_and_execution_input_requirements_separate() {
    for early in [true, false] {
        let mut f = fixture();
        readiness::remove_projection(
            &mut f,
            readiness::actor_owner(),
            if early {
                "prepare-first"
            } else {
                "assemble-first"
            },
            if early {
                "preparation-level"
            } else {
                "final-quality"
            },
        );
        let plan = compile(&f).unwrap();
        let result = plan.evaluate(&mut plan.new_scratch()).unwrap();
        if early {
            assert_eq!(
                result.outcome,
                SupportEffectsOutcome::Unavailable {
                    cause: EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        read: None,
                    },
                    input: None,
                },
                "missing preparation input must prevent support admission"
            );
            continue;
        }
        let report = delivery::evaluated(&result);
        prepared(report);
        for effect in rows(report, "skill-deliver") {
            if [
                readiness::target(30, "first"),
                readiness::target(31, "first"),
            ]
            .contains(receiver(effect))
            {
                assert!(!matches!(effect.value, EffectValue::Known { .. }));
            } else {
                assert!(matches!(effect.value, EffectValue::Known { .. }));
            }
        }
    }
}

#[test]
fn exact_skill_contribution_freeze_cannot_follow_its_consumer() {
    let f = fixture();
    // The positive graph has already frozen this precise Skill channel at
    // properties; only its freeze changes in the negative graph.
    assert!(compile(&f).is_ok());
    let result = inputs(&f, |stages| {
        stages.stages.push(EvaluationStage {
            id: key("after-execute"),
            predecessors: vec![key("execute")],
        });
        let frozen = stages
            .frozen_channels
            .iter_mut()
            .find(|row| row.channel == property_channel())
            .unwrap();
        assert_eq!(frozen.stage, key("properties"));
        frozen.stage = key("after-execute");
    })
    .and_then(readiness::compile_inputs);
    participation::rejected(
        result,
        "frozen channel read occurs before or outside frozen stage",
    );
}
