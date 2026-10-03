//! Preparation readiness is proved through a closed public support/metric plan.
#[allow(dead_code)]
#[path = "support/owned_preparation_readiness_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use std::collections::BTreeSet;
use support::delivery::{fixture as base, integer};
use support::*;

fn rejected<T>(result: Checked<T>, expected: &str) {
    let Err(error) = result else {
        panic!("expected rejection containing {expected}")
    };
    assert!(
        error.contains(expected),
        "expected {expected:?}, got {error:?}"
    );
}
fn no_known_metrics(report: &OwnedSupportMetricReport) {
    assert_eq!(report.evaluation.results.len(), 3);
    assert!(
        report
            .evaluation
            .results
            .iter()
            .all(|row| !matches!(row.value, EffectValue::Known { .. })),
        "{report:?}"
    );
}

#[test]
fn admitted_typed_properties_assemble_required_child_inputs_before_execution_metrics() {
    let f = fixture();
    let effects = compile(&f).expect("closed preparation/property/assembly graph");
    for name in ["level", "quality"] {
        let SchemaLookup::Known(schema) = effects.definitions().slot(&ability_parameter(name))
        else {
            panic!("child input must be declared");
        };
        assert_eq!(schema.presence, SlotPresence::RequiredOnce);
        assert!(
            schema.sites.is_empty(),
            "final inputs are projected, not caller supplied"
        );
    }
    let report = effects.evaluate(&mut effects.new_scratch()).unwrap();
    let evaluated = delivery::evaluated(&report);
    let properties: Vec<_> = evaluated
        .effects
        .iter()
        .filter(|effect| effect.key.invocation.program == key("prepared-actor-property"))
        .collect();
    assert_eq!(
        properties.len(),
        3,
        "one actor property for each retained exact assignment"
    );
    let mut origins = BTreeSet::new();
    for property in properties {
        let RuleOrigin::SupportApplication { application } = &property.key.invocation.origin else {
            panic!("property must be produced by retained native admission: {property:?}");
        };
        origins.insert(application.prepared.origin.clone());
        let SupportReceiverKey::Actor(receiver) = &application.receiver else {
            panic!("property belongs to an exact actor receiver");
        };
        let expected = if *receiver == actor(30) {
            assert!(
                [target(30, "first"), target(30, "second")].contains(&application.prepared.target)
            );
            14
        } else {
            assert_eq!(*receiver, actor(31));
            assert_eq!(application.prepared.target, target(31, "first"));
            23
        };
        assert_eq!(
            property.value,
            EffectValue::Known {
                value: integer(expected)
            }
        );
    }
    assert_eq!(
        origins,
        [61, 63, 65]
            .into_iter()
            .map(|id| SupportOrigin::Assignment(occurrence(id)))
            .collect()
    );
    for (use_id, name, expected) in [(30, "first", 40), (30, "second", 40), (31, "first", 44)] {
        let SkillTarget::Generated(skill) = target(use_id, name) else {
            unreachable!()
        };
        assert_eq!(
            skill.provider.grant_path.len(),
            2,
            "physical root → summon → actor"
        );
        let key = PlanValueKey::SkillParameter {
            skill,
            parameter: ability_parameter("level"),
        };
        let value = evaluated
            .values
            .iter()
            .find(|row| row.key == key)
            .expect("ordinary actor-to-child final projection");
        assert_eq!(
            value.value,
            EffectValue::Known {
                value: integer(expected)
            }
        );
    }
    let metrics = metrics(effects);
    let report = metrics.evaluate(&mut metrics.new_scratch()).unwrap();
    assert_eq!(numbers(&report), [40.0, 40.0, 44.0]);
    assert_eq!(
        report
            .evaluation
            .results
            .iter()
            .map(|row| row.request.clone())
            .collect::<Vec<_>>(),
        f.queries.requests
    );
}

#[test]
fn descendant_final_values_do_not_bypass_missing_ancestor_execution_input() {
    for produced in [true, false] {
        let mut f = fixture();
        ancestor_execution_input(&mut f, produced);
        let plan = compile_inputs(
            inputs_with(&f, false, |_| {}, classify_ancestor_execution, |_| {}).unwrap(),
        )
        .unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let values = delivery::evaluated(&report);
        for (root, name, expected) in [(30, "first", 40), (30, "second", 40), (31, "first", 44)] {
            let SkillTarget::Generated(skill) = target(root, name) else {
                unreachable!()
            };
            let wanted = PlanValueKey::SkillParameter {
                skill,
                parameter: ability_parameter("level"),
            };
            assert_eq!(
                values
                    .values
                    .iter()
                    .find(|row| row.key == wanted)
                    .unwrap()
                    .value,
                EffectValue::Known {
                    value: integer(expected)
                },
                "child assembly itself remains available"
            );
        }
        let metrics = metrics(plan);
        let report = metrics.evaluate(&mut metrics.new_scratch()).unwrap();
        if produced {
            assert_eq!(numbers(&report), [40.0, 40.0, 44.0]);
        } else {
            no_known_metrics(&report);
        }
    }
}

#[test]
fn missing_preparation_input_blocks_admission_and_missing_final_input_blocks_execution() {
    let mut early = fixture();
    remove_projection(
        &mut early,
        actor_owner(),
        "prepare-first",
        "preparation-level",
    );
    let plan = metrics(compile(&early).unwrap());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    no_known_metrics(&report);
    assert!(!matches!(report.support, SupportMetricStatus::Evaluated));

    let mut final_input = fixture();
    remove_projection(
        &mut final_input,
        actor_owner(),
        "assemble-first",
        "final-quality",
    );
    let plan = metrics(compile(&final_input).unwrap());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        matches!(report.support, SupportMetricStatus::Evaluated),
        "preparation remains independently available: {report:?}"
    );
    assert!(!matches!(
        report.evaluation.results[0].value,
        EffectValue::Known { .. }
    ));
    assert_eq!(
        report.evaluation.results[1].value,
        EffectValue::Known {
            value: quantity(40.0)
        }
    );
    assert!(!matches!(
        report.evaluation.results[2].value,
        EffectValue::Known { .. }
    ));
}

#[test]
fn inactive_root_does_not_mask_another_roots_missing_execution_input() {
    let mut f = fixture();
    f.build
        .gems
        .iter_mut()
        .find(|row| row.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    remove_projection(&mut f, actor_owner(), "assemble-first", "final-quality");
    let plan = metrics(compile(&f).unwrap());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    no_known_metrics(&report);
    assert_eq!(report.evaluation.results[0].value, EffectValue::Inactive);
    assert_eq!(report.evaluation.results[1].value, EffectValue::Inactive);
    assert!(
        matches!(
            report.evaluation.results[2].value,
            EffectValue::Unresolved { .. }
        ),
        "{report:?}"
    );
}

#[test]
fn early_reads_and_late_structural_activation_are_rejected_before_evaluation() {
    let mut f = fixture();
    program_mut(&mut f, child_owner(), "target-facts").reads[0].source =
        RuleReadSource::Parameter {
            slot: ability_parameter("level"),
        };
    rejected(compile(&f), "later");

    let f = fixture();
    rejected(
        inputs_with(
            &f,
            false,
            |_| {},
            |stages| {
                let row = stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .programs
                    .members
                    .iter_mut()
                    .find(|row| row.owner == actor_owner() && row.program == key("activate-first"))
                    .unwrap();
                row.phase = ReadinessPhase::Execution;
                row.role = ReadinessProgramRole::Execution;
                row.outputs.clear();
                stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|row| row.owner == actor_owner() && row.program == key("activate-first"))
                    .unwrap()
                    .stage = key("execute");
            },
            |_| {},
        )
        .and_then(compile_inputs),
        "later",
    );
}

#[test]
fn false_branch_cannot_hide_a_final_parameter_from_cold_preparation_proof() {
    let mut f = fixture();
    let facts = program_mut(&mut f, child_owner(), "target-facts");
    facts.reads.push(base::read(
        "late",
        RuleReadSource::Parameter {
            slot: ability_parameter("quality"),
        },
    ));
    facts.nodes.extend([
        base::read_node("late", "late"),
        base::bool_node("never", false),
        base::read_node("early", "level"),
    ]);
    facts
        .nodes
        .iter_mut()
        .find(|node| node.id == key("base"))
        .unwrap()
        .expression = RuleExpression::Select {
        condition: key("never"),
        when_true: key("late"),
        when_false: key("early"),
    };
    rejected(compile(&f), "later");
}

#[test]
fn lower_priority_origin_cannot_hide_a_late_property_read_in_an_unretained_branch() {
    let f = fixture();
    // The positive witness proves every retained origin has physical level two.
    // This late branch would only execute for level-one origins that selection replaces.
    rejected(
        inputs_with(
            &f,
            false,
            |rules| {
                let property = package_program_mut(
                    rules,
                    &delivery::support_owner(),
                    "prepared-actor-property",
                );
                property.reads.extend([
                    base::read("raw", RuleReadSource::GemLevel),
                    base::contributions("late", RuleEntity::Current, "support-total"),
                ]);
                property.nodes.extend([
                    base::read_node("raw", "raw"),
                    base::read_node("late", "late"),
                    base::literal("one", 1),
                    base::node(
                        "replaced-origin",
                        RuleExpression::Compare {
                            operation: RuleComparison::Equal,
                            left: key("raw"),
                            right: key("one"),
                        },
                    ),
                    base::node(
                        "guarded",
                        RuleExpression::Select {
                            condition: key("replaced-origin"),
                            when_true: key("late"),
                            when_false: key("value"),
                        },
                    ),
                ]);
                let RuleEffectKind::Contribute { value, .. } = &mut property.effects[0].effect
                else {
                    unreachable!()
                };
                *value = key("guarded");
            },
            |_| {},
            |_| {},
        )
        .and_then(compile_inputs),
        "later",
    );
}

#[test]
fn partial_input_program_and_receiving_inventories_cannot_certify_preparation() {
    let f = fixture();
    rejected(
        inputs_with(
            &f,
            false,
            |_| {},
            |stages| {
                stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .skills
                    .iter_mut()
                    .find(|row| row.skill == def::<SkillDefinition>("ability"))
                    .unwrap()
                    .parameters
                    .closure = partial(child_owner(), SchemaFacet::InputSchema);
            },
            |_| {},
        ),
        "complete required-input",
    );
    rejected(
        inputs_with(
            &f,
            false,
            |_| {},
            |stages| {
                stages.readiness.as_mut().unwrap().programs.closure =
                    partial(child_owner(), SchemaFacet::GameRules);
            },
            |_| {},
        ),
        "complete program",
    );
    let plan = compile_inputs(
        inputs_with(
            &f,
            false,
            |_| {},
            |_| {},
            |receiving| {
                receiving.supports[0].receivers.closure =
                    partial(delivery::support_owner(), SchemaFacet::GameRules);
            },
        )
        .unwrap(),
    )
    .unwrap();
    let plan = metrics(plan);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    no_known_metrics(&report);
    assert!(!matches!(report.support, SupportMetricStatus::Evaluated));
}

#[test]
fn typed_output_roles_reject_undeclared_writes_and_early_ordinary_delivery() {
    let f = fixture();
    rejected(
        inputs_with(
            &f,
            false,
            |_| {},
            |stages| {
                stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .programs
                    .members
                    .iter_mut()
                    .find(|row| row.program == key("target-facts"))
                    .unwrap()
                    .outputs
                    .clear();
            },
            |_| {},
        ),
        "potential writes",
    );
    rejected(
        inputs_with(
            &f,
            false,
            |rules| {
                package_program_mut(rules, &delivery::support_owner(), "prepared-actor-property")
                    .effects[0]
                    .effect = RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: def("prepared-property"),
                    value: key("value"),
                };
            },
            |_| {},
            |_| {},
        ),
        "preparation output role",
    );
    rejected(
        inputs_with(
            &f,
            false,
            |_| {},
            |stages| {
                let row = stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .programs
                    .members
                    .iter_mut()
                    .find(|row| row.program == key("actor-deliver"))
                    .unwrap();
                row.phase = ReadinessPhase::Preparation;
                row.role = ReadinessProgramRole::SupportedPreparationProperty;
                row.outputs = vec![StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: def("support-total"),
                    contribution: ContributionKind::Add,
                }];
            },
            |_| {},
        ),
        "readiness role mismatch",
    );
}

#[test]
fn false_guards_do_not_hide_duplicate_early_or_final_input_writers() {
    for (owner, source, duplicate) in [
        (child_owner(), "target-facts", "target-facts-duplicate"),
        (actor_owner(), "assemble-first", "assemble-duplicate"),
    ] {
        let mut f = fixture();
        let mut copy = program_mut(&mut f, owner.clone(), source).clone();
        copy.id = key(duplicate);
        copy.nodes.push(base::bool_node("never", false));
        for effect in &mut copy.effects {
            effect.when = Some(key("never"));
        }
        f.owner_mut(&owner).programs.members.push(copy);
        rejected(compile(&f), "competing potential");
    }
}

#[test]
fn repeated_preparation_contributions_remain_a_legal_ordered_stream() {
    let f = fixture();
    let plan = compile_inputs(
        inputs_with(
            &f,
            false,
            |rules| {
                let property = package_program_mut(
                    rules,
                    &delivery::support_owner(),
                    "prepared-actor-property",
                );
                let mut second = property.effects[0].clone();
                second.id = key("second-property");
                property.effects.push(second);
            },
            |_| {},
            |_| {},
        )
        .unwrap(),
    )
    .unwrap();
    let plan = metrics(plan);
    assert_eq!(
        numbers(&plan.evaluate(&mut plan.new_scratch()).unwrap()),
        [68.0, 68.0, 67.0]
    );
}

#[test]
fn ordinary_execution_final_writers_still_resolve_after_native_selection() {
    let final_delivery = |rules: &mut RulePackageInput| {
        package_program_mut(rules, &delivery::support_owner(), "actor-deliver").effects[0].effect =
            RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: def("support-total"),
                value: key("value"),
            };
    };
    let mut f = fixture();
    f.build
        .supports
        .retain(|row| row.target != target(30, "second"));
    f.build
        .support_origins
        .as_mut()
        .unwrap()
        .retain(|row| row.target != target(30, "second"));
    let plan =
        compile_inputs(inputs_with(&f, false, final_delivery, |_| {}, |_| {}).unwrap()).unwrap();
    let plan = metrics(plan);
    assert_eq!(
        numbers(&plan.evaluate(&mut plan.new_scratch()).unwrap()),
        [26.0, 26.0, 44.0]
    );

    // Two retained applications now really share one actor destination. Cold
    // compilation preserves legacy selection semantics; the retained graph fails.
    let f = fixture();
    let plan =
        compile_inputs(inputs_with(&f, false, final_delivery, |_| {}, |_| {}).unwrap()).unwrap();
    rejected(
        plan.evaluate(&mut plan.new_scratch())
            .map_err(|e| e.to_string()),
        "competing final producers",
    );
}

#[test]
fn v15_cannot_use_ordinary_delivery_to_supply_its_own_admission_inputs() {
    let f = fixture();
    let result = inputs_with(
        &f,
        true,
        |rules| {
            let mut property = property_program();
            property.id = key("actor-deliver");
            *package_program_mut(rules, &delivery::support_owner(), "actor-deliver") = property;
        },
        |stages| {
            // Keep base preparation in one stage so this tests the actual deferred
            // delivery cycle, not an unrelated stage-precedence violation.
            for row in &mut stages.programs.members {
                if row.program.as_str().starts_with("assemble-") {
                    row.stage = key("prepare");
                }
            }
        },
        |_| {},
    )
    .and_then(compile_inputs);
    rejected(
        result,
        "preparation prefix depends on a potential support delivery channel",
    );
}

#[test]
fn legacy_v15_packages_keep_absent_readiness_and_stable_plan_identity() {
    let f = delivery::source_fixture();
    let legacy = || {
        delivery::inputs_with_all_packages(
            &f,
            |rules| {
                rules.operations_version = key(OWNED_RULE_OPERATIONS_V15);
                rules.effect_applications = Some(DeclaredSet::complete(vec![]));
            },
            |_| {},
            |stages| {
                stages.effect_applications = Some(DeclaredSet::complete(vec![]));
            },
            |_| {},
            |_| {},
        )
    };
    let input = legacy();
    assert_eq!(input.stages.input().schema_version, 1);
    assert!(input.stages.input().readiness.is_none());
    assert_eq!(input.receiving.input().schema_version, 1);
    assert!(
        input
            .receiving
            .input()
            .supports
            .iter()
            .flat_map(|row| &row.receivers.members)
            .all(|row| row.preparation.is_none())
    );
    let first = compile_inputs(input).unwrap();
    let second = compile_inputs(legacy()).unwrap();
    assert_eq!(first.identity(), second.identity());
    assert_eq!(
        first.evaluate(&mut first.new_scratch()).unwrap(),
        second.evaluate(&mut second.new_scratch()).unwrap()
    );
}
