//! Requested participation shares the existing occurrence and preparation graph.
#[path = "support/owned_skill_participation_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

#[test]
fn false_parent_keeps_supply_and_preparation_but_gates_all_descendant_execution() {
    let mut f = fixture();
    set(&mut f, sources::member(30), Some(false));
    let plan = compile(&f).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let r = delivery::evaluated(&report);
    assert_eq!(activation(r, &sources::member(30)), &bool_value(false));
    assert_eq!(activation(r, &sources::member(31)), &bool_value(true));
    for name in ["first", "second"] {
        assert_eq!(
            activation(r, &readiness::target(30, name)),
            &bool_value(true),
            "child preference stays independently computable"
        );
        assert_eq!(
            final_level(r, 30, name),
            &delivery::known(40),
            "nonparticipation cannot remove assembly inputs"
        );
        let rows: Vec<_> = r
            .effects
            .iter()
            .filter(|e| {
                e.key.invocation.program == key("execution-value")
                    && e.key.invocation.entity
                        == ConcreteEntity::Skill(Box::new(readiness::target(30, name)))
            })
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, EffectValue::Inactive);
    }
    let prepared: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("prepared-actor-property"))
        .collect();
    assert_eq!(prepared.len(), 3);
    assert!(
        prepared
            .iter()
            .all(|e| matches!(e.value, EffectValue::Known { .. }))
    );
    for e in &r.effects {
        if let Some(application) = delivery::application(e)
            && e.key.invocation.program == key("actor-deliver")
            && application.receiver == SupportReceiverKey::Actor(readiness::actor(30))
        {
            assert_eq!(e.value, EffectValue::Inactive);
        }
    }
    let metric = readiness::metrics(plan);
    let r = metric.evaluate(&mut metric.new_scratch()).unwrap();
    assert_eq!(
        r.evaluation
            .results
            .iter()
            .map(|r| r.value.clone())
            .collect::<Vec<_>>(),
        vec![
            EffectValue::Inactive,
            EffectValue::Inactive,
            EffectValue::Known {
                value: readiness::quantity(44.)
            },
        ]
    );
}

#[test]
fn sibling_requests_are_independent_and_unknown_is_not_false_or_a_default() {
    let mut f = fixture();
    set(&mut f, readiness::target(30, "first"), Some(false));
    let p = readiness::metrics(compile(&f).unwrap());
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(r.evaluation.results[0].value, EffectValue::Inactive);
    assert_eq!(
        r.evaluation.results[1].value,
        EffectValue::Known {
            value: readiness::quantity(40.)
        }
    );
    assert_eq!(
        r.evaluation.results[2].value,
        EffectValue::Known {
            value: readiness::quantity(44.)
        }
    );
    for absent_record in [false, true] {
        let mut f = fixture();
        if absent_record {
            f.scenario
                .usage
                .retain(|r| r.target != UsageTarget::Skill(sources::member(30)));
        } else {
            set(&mut f, sources::member(30), None);
        }
        let p = compile(&f).unwrap();
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        let e = delivery::evaluated(&r);
        assert_eq!(final_level(e, 30, "first"), &delivery::known(40));
        let p = readiness::metrics(p);
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        assert!(matches!(
            r.evaluation.results[0].value,
            EffectValue::Unresolved { .. }
        ));
        assert!(matches!(
            r.evaluation.results[1].value,
            EffectValue::Unresolved { .. }
        ));
        assert_eq!(
            r.evaluation.results[2].value,
            EffectValue::Known {
                value: readiness::quantity(44.)
            }
        );
    }
}

#[test]
fn participation_cannot_enable_a_disabled_mechanical_provider_or_hide_partial_coverage() {
    let mut f = fixture();
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let p = readiness::metrics(compile(&f).unwrap());
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(r.evaluation.results[0].value, EffectValue::Inactive);
    assert_eq!(r.evaluation.results[1].value, EffectValue::Inactive);
    assert_eq!(
        r.evaluation.results[2].value,
        EffectValue::Known {
            value: readiness::quantity(44.)
        }
    );
    let mut f = fixture();
    set(&mut f, sources::member(30), Some(false));
    f.owner_mut(&readiness::summon_owner()).programs.closure =
        readiness::partial(readiness::summon_owner(), SchemaFacet::GameRules);
    rejected(compile(&f), "early phase and complete owner programs");
    let mut f = fixture();
    set(&mut f, sources::member(30), Some(false));
    let owner = SchemaSubject::Slot(SlotAddress::ActionOutput(delivery::output()));
    f.owner_mut(&owner).programs.closure =
        readiness::partial(owner.clone(), SchemaFacet::GameRules);
    let p = compile(&f).unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(
        r.gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(matches!(
        r.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}

#[test]
fn participation_producer_reads_keep_cold_late_cycle_and_competing_writer_checks() {
    let mut f = fixture();
    let policy = f
        .owner_mut(&policy_owner())
        .programs
        .members
        .first_mut()
        .unwrap();
    policy.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def("requested-participation"),
    };
    rejected(compile(&f), "cycle");
    let mut f = fixture();
    let mut copy = f.owner_mut(&policy_owner()).programs.members[0].clone();
    copy.id = key("requested-duplicate");
    copy.nodes.push(base::bool_node("never", false));
    copy.effects[0].when = Some(key("never"));
    f.owner_mut(&policy_owner()).programs.members.push(copy);
    rejected(compile(&f), "competing");
    let mut f = fixture();
    f.schema.definitions.push(DefinitionDescriptor::Stat(known(
        def("late-participation"),
        StatSchema {
            value: ComputedValueType::Boolean,
            targets: vec![RuleEntityKind::Skill],
        },
    )));
    f.owner_mut(&readiness::summon_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("late-value"),
            context: RuleEntityKind::Skill,
            reads: vec![],
            nodes: vec![base::bool_node("late", true)],
            effects: vec![base::derive(
                "late",
                RuleEntity::Current,
                "late-participation",
                "late",
            )],
        });
    f.owner_mut(&policy_owner()).programs.members[0].reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def("late-participation"),
    };
    rejected(compile(&f), "later");
}

#[test]
fn applications_gate_both_the_exact_source_and_the_recipient_ancestry() {
    let mut f = fixture();
    add_application(&mut f);
    set(&mut f, sources::member(30), Some(false));
    let p = readiness::compile_inputs(application_inputs(&f).unwrap()).unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    let r = delivery::evaluated(&r);
    let rows: Vec<_> = r
        .effects
        .iter()
        .filter(|e| {
            matches!(
                e.key.invocation.origin,
                RuleOrigin::EffectApplication { .. }
            ) && e.key.invocation.program == key("buff")
        })
        .collect();
    assert_eq!(rows.len(), 8, "four sources, two independent recipients");
    let mut known = 0;
    for row in rows {
        let RuleOrigin::EffectApplication {
            source, recipient, ..
        } = &row.key.invocation.origin
        else {
            unreachable!()
        };
        let inactive = matches!(source,ConcreteEntity::Skill(s) if **s==readiness::target(30,"first") || **s==readiness::target(30,"second"))
            || *recipient == ConcreteEntity::Actor(readiness::actor(30));
        if inactive {
            assert_eq!(row.value, EffectValue::Inactive);
        } else {
            assert_eq!(row.value, delivery::known(5));
            known += 1;
        }
    }
    assert_eq!(known, 2);
}

#[test]
fn source_assembly_keeps_mechanical_members_with_mixed_requested_participation() {
    let mut f = sources::fixture();
    install(&mut f);
    set(&mut f, sources::member(30), Some(false));
    let input = sources::inputs_with(
        &f,
        |r| {
            r.operations_version = key(OWNED_RULE_OPERATIONS_V21);
            r.contribution_queries = Some(DeclaredSet::complete(vec![]));
        },
        configure_stages,
        |_| {},
    )
    .unwrap();
    let p = readiness::compile_inputs(input).unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    let r = delivery::evaluated(&r);
    for (id, expected) in [(30, 33), (31, 42)] {
        assert_eq!(
            sources::value(r, &sources::final_key(id)),
            &delivery::known(expected)
        );
        assert_eq!(
            sources::value(r, &sources::count_key(id)),
            &delivery::known(1)
        );
    }
    let p = sources::metrics(p);
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(r.evaluation.results[0].value, EffectValue::Inactive);
    assert_eq!(r.evaluation.results[1].value, EffectValue::Inactive);
    assert_eq!(
        r.evaluation.results[2].value,
        EffectValue::Known {
            value: readiness::quantity(42.)
        }
    );
    assert_eq!(
        r.evaluation.results[3].value,
        EffectValue::Known {
            value: readiness::quantity(42.)
        }
    );
}

#[test]
fn participation_is_query_independent_and_deterministic_across_scratch_and_rayon() {
    let a = fixture();
    let mut b = fixture();
    set(&mut b, sources::member(30), Some(false));
    let plans = [
        Arc::new(readiness::metrics(compile(&a).unwrap())),
        Arc::new(readiness::metrics(compile(&b).unwrap())),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = plans[0].new_scratch();
    for i in [0, 1, 0] {
        assert_eq!(plans[i].evaluate(&mut scratch).unwrap(), expected[i]);
    }
    let parallel: Vec<_> = (0..12)
        .into_par_iter()
        .map_init(
            || plans[0].new_scratch(),
            |s, i| (i % 2, plans[i % 2].evaluate(s).unwrap()),
        )
        .collect();
    for (i, r) in parallel {
        assert_eq!(r, expected[i]);
    }
    let values: BTreeMap<_, _> = expected[1]
        .evaluation
        .results
        .iter()
        .map(|r| (r.request.id.clone(), r.value.clone()))
        .collect();
    for subset in [false, true] {
        let mut f = fixture();
        set(&mut f, sources::member(30), Some(false));
        f.queries.requests.reverse();
        if subset {
            f.queries.requests.truncate(1);
        }
        let p = readiness::metrics(compile(&f).unwrap());
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        for row in r.evaluation.results {
            assert_eq!(row.value, values[&row.request.id]);
        }
    }
}

#[test]
fn item_tree_and_authored_requests_preserve_exact_occurrence_and_availability() {
    let targets = root_targets();
    let mut f = roots_fixture();
    for (target, active) in targets.iter().zip([false, true, false, true, false]) {
        set(&mut f, target.clone(), Some(active));
    }
    let p = readiness::compile_inputs(roots_inputs(&f).unwrap()).unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    let r = delivery::evaluated(&r);
    for (target, active) in targets.iter().zip([false, true, false, true, false]) {
        assert_eq!(activation(r, target), &bool_value(active));
        assert_eq!(
            root_value(r, target),
            Some(&if active {
                delivery::known(9)
            } else {
                EffectValue::Inactive
            })
        );
    }
    for unavailable in ["disabled", "loadout"] {
        let mut f = roots_fixture();
        let skill = f
            .build
            .skills
            .iter_mut()
            .find(|s| s.id == occurrence(81))
            .unwrap();
        if unavailable == "disabled" {
            skill.enabled = false;
        } else {
            skill.scope = LoadoutScope::Selected {
                loadouts: vec![occurrence(2)],
            };
        }
        f.build
            .equipment
            .iter_mut()
            .find(|e| e.id == occurrence(6))
            .unwrap()
            .scope = LoadoutScope::Selected {
            loadouts: vec![occurrence(2)],
        };
        f.build.allocations[0].scope = LoadoutScope::Selected {
            loadouts: vec![occurrence(2)],
        };
        let p = readiness::compile_inputs(roots_inputs(&f).unwrap()).unwrap();
        let r = p.evaluate(&mut p.new_scratch()).unwrap();
        let r = delivery::evaluated(&r);
        for i in [0, 2, 3] {
            assert!(!matches!(
                root_value(r, &targets[i]),
                Some(EffectValue::Known { .. })
            ));
        }
        for i in [1, 4] {
            assert_eq!(root_value(r, &targets[i]), Some(&delivery::known(9)));
        }
    }
}

#[test]
fn native_final_support_membership_cannot_impersonate_requested_participation() {
    use poe_optimizer_core::{owned_support_inputs::SupportTypeStat, owned_support_outputs::*};
    use poe_optimizer_data::{owned_rules::OwnedRulePackage, owned_support_outputs::*};
    let mut f = fixture();
    f.owner_mut(&policy_owner()).programs.members.clear();
    let mut authored = None;
    let input = inputs_with(
        &f,
        |rules| authored = Some(rules.clone()),
        |stages| {
            stages
                .frozen_channels
                .push(poe_optimizer_core::owned_stages::FrozenStageChannel {
                    channel: poe_optimizer_core::owned_stages::StageChannel::Stat {
                        scope: RuleEntityKind::Skill,
                        stat: def("requested-participation"),
                    },
                    stage: key("assemble"),
                });
        },
        |_| {},
    )
    .unwrap();
    let stored = OwnedRulePackage::new(
        authored.unwrap(),
        input.definitions.as_ref(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(input.rules.source_identity(), Some(*stored.identity()));
    let outputs = OwnedSupportOutputBindings::new(
        SupportOutputBindingsInput {
            schema_version: OWNED_SUPPORT_OUTPUT_BINDINGS_VERSION,
            namespace: base::ns(),
            release: key("participation-impersonation"),
            definitions: input.definitions.identity().clone(),
            rules: *stored.identity(),
            preparation: *input.preparation.identity(),
            inputs: *input.inputs.identity(),
            receiving: *input.receiving.identity(),
            stages: *input.stages.identity(),
            output_stage: key("assemble"),
            final_skill_types: vec![SupportTypeStat {
                support_type: key("spell"),
                stat: def("requested-participation"),
            }],
        },
        &SupportOutputDependencies {
            definitions: input.definitions.as_ref(),
            rules: &stored,
            preparation: &input.preparation,
            inputs: &input.inputs,
            receiving: &input.receiving,
            stages: &input.stages,
        },
        Default::default(),
    )
    .unwrap();
    rejected(
        OwnedSupportEffectPlan::compile_with_outputs(
            input,
            Arc::new(outputs),
            Default::default(),
            Default::default(),
        )
        .map_err(|e| e.to_string()),
        "participation cannot be supplied by native support output",
    );
}
