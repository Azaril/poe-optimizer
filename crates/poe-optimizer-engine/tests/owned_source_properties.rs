//! Source aggregation executes through the public single-attempt support graph.
#[allow(dead_code)]
#[path = "support/owned_source_properties_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_readiness::*, owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::collections::BTreeSet;

#[test]
fn physical_sources_preserve_producer_inputs_and_share_properties_across_action_variants() {
    let f = fixture();
    let plan = compile(&f).expect("complete native source relation");
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let evaluated = evaluated(&report);
    assert_ne!(owner(30), member(30));
    assert_ne!(owner(31), member(31));
    for (id, final_value) in [(30, 33), (31, 42)] {
        assert_eq!(
            value(evaluated, &final_key(id)),
            &EffectValue::Known {
                value: base::integer(final_value)
            }
        );
        assert_eq!(
            value(evaluated, &count_key(id)),
            &EffectValue::Known {
                value: base::integer(1)
            }
        );
    }
    let contributions: Vec<_> = evaluated
        .effects
        .iter()
        .filter(|row| row.key.invocation.program == key("source-supported"))
        .collect();
    assert_eq!(
        contributions.len(),
        2,
        "two physical owners, independent of four Action requests"
    );
    let mut owners = BTreeSet::new();
    let mut producers = BTreeSet::new();
    for row in contributions {
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: base::integer(2)
            },
            "read the actual support Gem, not its effective level or receiving Skill"
        );
        let RuleOrigin::SourceProperty {
            relation,
            owner,
            producer,
            position,
        } = &row.key.invocation.origin
        else {
            panic!("source property lost its retained-position provenance")
        };
        assert_eq!(relation, &key("source"));
        assert!(position.is_some());
        owners.insert(owner.as_ref().clone());
        producers.insert(producer.clone());
    }
    assert_eq!(owners, [owner(30), owner(31)].into_iter().collect());
    assert_eq!(producers.len(), 2);
    let metric = metrics(plan);
    let result = metric.evaluate(&mut metric.new_scratch()).unwrap();
    assert_eq!(readiness::numbers(&result), [33.0, 33.0, 42.0, 42.0]);
}

#[test]
fn no_assignments_or_queries_still_execute_nonzero_external_source_properties() {
    let mut f = fixture();
    without_supports(&mut f);
    f.queries.requests.clear();
    assert!(f.build.supports.is_empty());
    let plan = compile(&f).expect("source owners are independent of optional consumers");
    assert!(plan.request().queries().input().requests.is_empty());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let evaluated = evaluated(&report);
    for (id, final_value) in [(30, 31), (31, 40)] {
        assert_eq!(
            value(evaluated, &final_key(id)),
            &EffectValue::Known {
                value: base::integer(final_value)
            }
        );
        assert_eq!(
            value(evaluated, &count_key(id)),
            &EffectValue::Known {
                value: base::integer(0)
            }
        );
    }
    let properties: Vec<_> = evaluated
        .effects
        .iter()
        .filter(|row| row.key.invocation.program == key("source-external"))
        .collect();
    assert_eq!(properties.len(), 2);
    assert!(properties.iter().all(|row| row.value
        == EffectValue::Known {
            value: base::integer(20)
        }));
    assert!(
        !evaluated
            .effects
            .iter()
            .any(|row| row.key.invocation.program == key("source-supported"))
    );
}

#[test]
fn direct_sources_keep_authored_raw_inputs_and_derive_a_distinct_final_stat() {
    let f = direct_fixture();
    let original = f.build.skills[0].parameters.clone();
    let plan = readiness::compile_inputs(direct_inputs(&f).unwrap()).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let evaluated = evaluated(&report);
    let final_value = PlanValueKey::Stat {
        entity: ConcreteEntity::Skill(Box::new(owner(32))),
        stat: def("direct-final"),
    };
    assert_eq!(
        value(evaluated, &final_value),
        &EffectValue::Known {
            value: base::integer(34)
        }
    );
    assert_eq!(
        value(evaluated, &count_key(32)),
        &EffectValue::Known {
            value: base::integer(1)
        }
    );
    assert_eq!(
        plan.request().build().input().skills[0].parameters,
        original
    );
    assert_eq!(original.unwrap()[0].value, base::integer(12));
    assert!(
        !evaluated
            .values
            .iter()
            .any(|row| matches!(row.key, PlanValueKey::SkillParameter { .. })),
        "Direct final values do not create self-projection parameters"
    );
}

#[test]
fn absent_input_owners_never_turn_relation_external_programs_into_ordinary_invocations() {
    let mut f = fixture();
    f.build.skills.clear();
    f.build.supports.clear();
    f.build.support_origins = Some(vec![]);
    f.queries.requests.clear();
    let plan = compile(&f).expect("zero matching source owners is a complete empty relation");
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let report = evaluated(&report);
    assert!(!report.effects.iter().any(|row| matches!(
        row.key.invocation.origin,
        RuleOrigin::SourceProperty { .. } | RuleOrigin::SourcePropertyCensus { .. }
    )));
    assert!(
        !report
            .effects
            .iter()
            .any(|row| row.key.invocation.program == key("source-external"))
    );
    assert!(
        report
            .effects
            .iter()
            .any(|row| row.key.invocation.program == key("target-facts-player")),
        "the real external producer's Player owner still exists"
    );
}

fn rejected<T>(result: Checked<T>, fragment: &str) {
    let Err(error) = result else {
        panic!("expected rejection containing {fragment}")
    };
    assert!(
        error.to_lowercase().contains(&fragment.to_lowercase()),
        "expected {fragment:?}, got {error:?}"
    );
}

#[test]
fn aliases_are_rejected_from_the_native_build_even_when_the_extra_root_is_disabled() {
    let mut f = fixture();
    let mut alias = f.build.skills[0].clone();
    alias.id = occurrence(99);
    alias.enabled = false;
    f.build.skills.push(alias);
    rejected(compile(&f), "alias");

    let mut f = fixture();
    let support = f.build.supports[0].support;
    f.build.supports[1].support = support;
    rejected(compile(&f), "alias");
}

#[test]
fn every_source_inventory_must_be_complete_including_empty_external_and_support_rows() {
    for part in [
        "relations",
        "effects",
        "channels",
        "external",
        "supports",
        "programs",
        "assembly",
    ] {
        let f = fixture();
        rejected(
            inputs_with(
                &f,
                |_| {},
                |_| {},
                |receiving| {
                    let source = receiving.source_properties.as_mut().unwrap();
                    let partial =
                        readiness::partial(readiness::physical_owner(), SchemaFacet::GameRules);
                    if part == "relations" {
                        source.relations.closure = partial;
                        return;
                    }
                    let relation = &mut source.relations.members[0];
                    match part {
                        "effects" => relation.effects.closure = partial,
                        "channels" => relation.channels.closure = partial,
                        "external" => {
                            relation.external.members.clear();
                            relation.external.closure = partial
                        }
                        "supports" => relation.supports.closure = partial,
                        "programs" => relation.supports.members[0].programs.closure = partial,
                        "assembly" => relation.assembly.closure = partial,
                        _ => unreachable!(),
                    }
                },
            ),
            "Complete",
        );
    }
}

#[test]
fn source_properties_require_their_preparation_phase_and_census_dependency() {
    let f = fixture();
    rejected(
        inputs_with(
            &f,
            |_| {},
            |stages| {
                stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .programs
                    .members
                    .iter_mut()
                    .find(|row| row.program == key("source-supported"))
                    .unwrap()
                    .phase = ReadinessPhase::Structural;
            },
            |_| {},
        ),
        "preparation",
    );
    rejected(
        inputs_with(
            &f,
            |_| {},
            |_| {},
            |receiving| {
                receiving
                    .source_properties
                    .as_mut()
                    .unwrap()
                    .relations
                    .members[0]
                    .census_stage = key("assemble");
            },
        ),
        "follow census",
    );
}

#[test]
fn final_input_is_missing_when_no_assembly_producer_is_declared() {
    let mut f = fixture();
    f.owner_mut(&readiness::physical_owner())
        .programs
        .members
        .retain(|program| program.id != key("source-assembly"));
    let inputs = inputs_with(
        &f,
        |_| {},
        |_| {},
        |receiving| {
            receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members[0]
                .assembly
                .members
                .clear()
        },
    )
    .unwrap();
    let plan = readiness::compile_inputs(inputs)
        .expect("an honest missing producer remains a readiness gap");
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let report = evaluated(&report);
    for id in [30, 31] {
        assert!(
            !report.values.iter().any(|row| row.key == final_key(id)),
            "an absent assembly must not manufacture a final parameter row"
        );
        assert!(matches!(
            value(
                report,
                &PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(member(id))),
                    stat: def("readiness-metric"),
                }
            ),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
    }
    let metrics = metrics(plan);
    let report = metrics.evaluate(&mut metrics.new_scratch()).unwrap();
    assert_eq!(report.evaluation.results.len(), 4);
    assert!(
        report.evaluation.results.iter().all(|result| matches!(
            result.value,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        )),
        "every Action query must retain the missing final producer: {report:?}"
    );
}

fn measure(plan: &Effects, scratch: &mut OwnedPlanScratch) -> (SupportEffectsReport, usize) {
    let allowance = PlanLimits::default().max_work;
    let mut remaining = allowance;
    let report = plan.evaluate_with_budget(scratch, &mut remaining).unwrap();
    (report, allowance - remaining)
}

#[test]
fn one_decreasing_budget_covers_source_census_properties_and_assembly_and_recovers() {
    let plan = compile(&fixture()).unwrap();
    let mut scratch = plan.new_scratch();
    let (expected, used) = measure(&plan, &mut scratch);
    assert!(used > 1);
    let mut remaining = used * 2;
    for after in [used, 0] {
        assert!(
            plan.evaluate_with_budget(&mut scratch, &mut remaining)
                .unwrap()
                == expected
        );
        assert_eq!(remaining, after);
    }
    let mut short = used - 1;
    assert!(matches!(
        plan.evaluate_with_budget(&mut scratch, &mut short),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(short, 0);
    assert!(measure(&plan, &mut scratch) == (expected, used));
}

#[test]
fn source_copies_and_changed_inputs_survive_a_b_a_and_private_rayon_reuse() {
    let a = fixture();
    let mut b = fixture();
    b.build.character.level = 23;
    b.build
        .gems
        .iter_mut()
        .find(|gem| gem.id == occurrence(51))
        .unwrap()
        .level = 4;
    b.build
        .gems
        .iter_mut()
        .find(|gem| gem.id == occurrence(28))
        .unwrap()
        .level = 12;
    let plans = [compile(&a).unwrap(), compile(&b).unwrap()];
    let expected = plans
        .each_ref()
        .map(|plan| measure(plan, &mut plan.new_scratch()));
    assert_ne!(plans[0].identity(), plans[1].identity());
    for (id, number) in [(30, 39), (31, 45)] {
        assert_eq!(
            value(evaluated(&expected[1].0), &final_key(id)),
            &EffectValue::Known {
                value: base::integer(number)
            }
        );
    }
    let mut scratch = plans[0].new_scratch();
    for index in [0, 1, 0] {
        assert!(measure(&plans[index], &mut scratch) == expected[index]);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results: Vec<_> = pool.install(|| {
        (0..8)
            .into_par_iter()
            .map_init(
                || plans[0].new_scratch(),
                |scratch, batch| {
                    (0..6)
                        .map(|step| {
                            let selected = (batch + step) % 2;
                            (selected, measure(&plans[selected], scratch))
                        })
                        .collect::<Vec<_>>()
                },
            )
            .collect()
    });
    for batch in results {
        for (selected, result) in batch {
            assert!(
                result == expected[selected],
                "worker reused stale source channels"
            );
        }
    }
}

#[test]
fn repeated_selected_positions_count_twice_but_admission_by_two_effects_does_not() {
    let f = family_fixture();
    let plan = readiness::compile_inputs(family_inputs(&f).unwrap()).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let report = evaluated(&report);
    assert_eq!(
        value(report, &count_key(30)),
        &EffectValue::Known {
            value: base::integer(2)
        }
    );
    let SkillTarget::Generated(second) = second_member(30) else {
        unreachable!()
    };
    for key in [
        final_key(30),
        PlanValueKey::SkillParameter {
            skill: second,
            parameter: final_parameter(),
        },
    ] {
        assert_eq!(
            value(report, &key),
            &EffectValue::Known {
                value: base::integer(35)
            }
        );
    }
    let properties: Vec<_> = report
        .effects
        .iter()
        .filter(|row| row.key.invocation.program == key("source-supported"))
        .collect();
    assert_eq!(
        properties.len(),
        2,
        "two retained positions, not two effects multiplied by two positions"
    );
    let mut positions = BTreeSet::new();
    let mut producers = BTreeSet::new();
    for property in properties {
        assert_eq!(
            property.value,
            EffectValue::Known {
                value: base::integer(2)
            }
        );
        let RuleOrigin::SourceProperty {
            owner: source,
            producer,
            position,
            ..
        } = &property.key.invocation.origin
        else {
            panic!("exact position origin required")
        };
        assert_eq!(source.as_ref(), &owner(30));
        positions.insert(position.unwrap());
        producers.insert(producer);
    }
    assert_eq!(positions, [0, 1].into_iter().collect());
    assert_eq!(
        producers.len(),
        1,
        "both retained slots intentionally refer to one physical assignment"
    );
    assert_eq!(
        value(report, &count_key(31)),
        &EffectValue::Known {
            value: base::integer(0)
        }
    );
}

#[test]
fn known_empty_support_properties_keep_external_values_while_a_missing_inventory_stays_unavailable()
{
    let mut f = fixture();
    f.owner_mut(&readiness::delivery::support_owner())
        .programs
        .members
        .retain(|program| program.id != key("source-supported"));
    let known = inputs_with(
        &f,
        |_| {},
        |_| {},
        |receiving| {
            receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members[0]
                .supports
                .members[0]
                .programs
                .members
                .clear()
        },
    )
    .unwrap();
    let known = readiness::compile_inputs(known).unwrap();
    let report = known.evaluate(&mut known.new_scratch()).unwrap();
    assert_eq!(
        value(evaluated(&report), &final_key(30)),
        &EffectValue::Known {
            value: base::integer(31)
        }
    );
    assert_eq!(
        value(evaluated(&report), &count_key(30)),
        &EffectValue::Known {
            value: base::integer(1)
        }
    );
    let missing = inputs_with(
        &f,
        |_| {},
        |_| {},
        |receiving| {
            receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members[0]
                .supports
                .members
                .clear()
        },
    )
    .unwrap();
    let missing = readiness::compile_inputs(missing).unwrap();
    let report = missing.evaluate(&mut missing.new_scratch()).unwrap();
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
        "{report:?}"
    );
}

#[test]
fn partial_real_program_ownership_cannot_be_promoted_by_a_complete_relation_inventory() {
    let mut f = fixture();
    f.owner_mut(&readiness::physical_owner()).programs.closure =
        readiness::partial(readiness::physical_owner(), SchemaFacet::GameRules);
    rejected(inputs(&f), "Complete");
}

fn channel_chain(cyclic: bool) -> Checked<Inputs> {
    let mut f = fixture();
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("source-other"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Skill],
            }),
        }));
    let external = readiness::program_mut(&mut f, base::class_owner(), "source-external");
    let RuleEffectKind::Contribute { stat, .. } = &mut external.effects[0].effect else {
        unreachable!()
    };
    *stat = def("source-other");
    if cyclic {
        external.reads[0] = base::contributions("input", RuleEntity::PropertyOwner, "source-add");
    }
    let support = readiness::program_mut(
        &mut f,
        readiness::delivery::support_owner(),
        "source-supported",
    );
    support.reads[0] = base::contributions("input", RuleEntity::PropertyOwner, "source-other");
    if cyclic {
        for support in &mut f.build.supports {
            support.enabled = false;
        }
    }
    inputs_with(
        &f,
        |_| {},
        |stages| {
            let external = stages
                .readiness
                .as_mut()
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|row| row.program == key("source-external"))
                .unwrap();
            external.outputs = vec![StageChannel::Contributions {
                scope: RuleEntityKind::Skill,
                stat: def("source-other"),
                contribution: ContributionKind::Add,
            }];
        },
        |receiving| {
            receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members[0]
                .channels
                .members
                .push(
                    poe_optimizer_core::owned_source_properties::SourcePropertyChannel {
                        stat: def("source-other"),
                        contribution: ContributionKind::Add,
                    },
                );
        },
    )
}

#[test]
fn same_phase_channel_dependencies_are_ordered_and_cycles_in_unselected_programs_are_rejected() {
    let plan = readiness::compile_inputs(channel_chain(false).unwrap()).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(
        value(evaluated(&report), &final_key(30)),
        &EffectValue::Known {
            value: base::integer(31)
        }
    );
    assert_eq!(
        value(evaluated(&report), &final_key(31)),
        &EffectValue::Known {
            value: base::integer(40)
        }
    );
    rejected(
        channel_chain(true).and_then(readiness::compile_inputs),
        "cycle",
    );
}

#[test]
fn disabled_sources_are_inactive_while_unknown_activation_blocks_the_attempt_and_clears_scratch() {
    let baseline = compile(&fixture()).unwrap();
    let expected = baseline.evaluate(&mut baseline.new_scratch()).unwrap();
    let mut disabled = fixture();
    disabled
        .build
        .skills
        .iter_mut()
        .find(|skill| skill.id == occurrence(30))
        .unwrap()
        .enabled = false;
    // A disabled authored root has no discovered Action topology. Query the
    // independent active copy while checking the disabled source's own facts.
    disabled.queries.requests.retain(|request| {
        matches!(&request.target, MetricTarget::Action(action)
            if action.action.provider.root == ProviderRoot::SkillUse(occurrence(31)))
    });
    assert_eq!(disabled.queries.requests.len(), 2);
    let disabled = compile(&disabled).unwrap();
    let report = disabled.evaluate(&mut disabled.new_scratch()).unwrap();
    let report = evaluated(&report);
    assert_eq!(value(report, &count_key(30)), &EffectValue::Inactive);
    assert_eq!(value(report, &final_key(30)), &EffectValue::Inactive);
    assert_eq!(
        value(report, &final_key(31)),
        &EffectValue::Known {
            value: base::integer(42)
        }
    );
    for effect in report.effects.iter().filter(|effect|matches!(&effect.key.invocation.origin,RuleOrigin::SourceProperty{owner:source,..} if source.as_ref()==&owner(30))) {
        assert_eq!(effect.value,EffectValue::Inactive,"an inactive source must not receive its Player's external property");
    }

    let mut unknown = fixture();
    readiness::remove_projection(
        &mut unknown,
        readiness::physical_owner(),
        "supply-summon",
        "activate-summon",
    );
    let unknown = compile(&unknown).unwrap();
    let mut scratch = baseline.new_scratch();
    let report = unknown.evaluate(&mut scratch).unwrap();
    assert!(
        matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved { .. },
                ..
            }
        ),
        "missing activation is unknown, never an inactive source: {report:?}"
    );
    assert!(
        baseline.evaluate(&mut scratch).unwrap() == expected,
        "unresolved source admission leaked into a later successful attempt"
    );
}
