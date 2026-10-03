//! The same injected raw slots serve independent authored and supplied occurrences.
#[allow(dead_code)]
#[path = "support/owned_preparation_readiness_fixture.rs"]
mod support;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_V5;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use support::delivery::integer;
use support::*;

fn strip_direct_path(provider: &mut ProviderKey) {
    if matches!(provider.root, ProviderRoot::SkillUse(id) if id == occurrence(30) || id == occurrence(31))
    {
        assert_eq!(provider.grant_path.remove(0), summon_grant());
    }
}
fn direct_action(action: &mut ActionSelection) {
    strip_direct_path(&mut action.action.provider);
    if let ActorKey::Owned(actor) = &mut action.action.actor {
        strip_direct_path(&mut actor.provider);
    }
}
fn mixed() -> Fixture {
    let mut f = fixture();
    f.schema.schema_version = OWNED_SCHEMA_PACKAGE_V5;
    for row in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(row) = row
            && row.id == def::<SkillDefinition>("summon")
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.directly_selectable = true;
        }
    }
    for row in &mut f.schema.slots {
        if let SlotDescriptor::Parameter(row) = row
            && row.id.declaration == SlotOwnerDefId::Skill(def("summon"))
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.skill_input = Some(SkillInputAuthority::AuthoredOrProjected);
            schema.sites = vec![ParameterSite::SkillParameter];
        }
    }
    for (id, level) in [(30, 11), (31, 20)] {
        let row = f
            .build
            .skills
            .iter_mut()
            .find(|s| s.id == occurrence(id))
            .unwrap();
        row.source = AuthoredSkillSource::Direct(def("summon"));
        row.parameters = Some(vec![
            ParameterAssignment {
                slot: summon_parameter("level"),
                value: integer(level),
            },
            ParameterAssignment {
                slot: summon_parameter("enabled"),
                value: ParameterValue::Boolean(true),
            },
        ]);
    }
    f.build.skills.push(SkillUse {
        id: occurrence(32),
        source: AuthoredSkillSource::Gem(occurrence(28)),
        parameters: None,
        enabled: true,
        scope: LoadoutScope::Shared,
    });
    for assignment in &mut f.build.supports {
        let SkillTarget::Generated(skill) = &mut assignment.target else {
            unreachable!()
        };
        strip_direct_path(&mut skill.provider);
    }
    for sequence in f.build.support_origins.as_mut().unwrap() {
        let SkillTarget::Generated(skill) = &mut sequence.target else {
            unreachable!()
        };
        strip_direct_path(&mut skill.provider);
    }
    for query in &mut f.queries.requests {
        let MetricTarget::Action(action) = &mut query.target else {
            unreachable!()
        };
        direct_action(action);
    }
    f.queries.requests.push(MetricRequest {
        id: QueryId::new("32-first").unwrap(),
        metric: def("requested"),
        target: MetricTarget::Action(Box::new(action(32, "first"))),
    });
    f.build
        .support_origins
        .as_mut()
        .unwrap()
        .push(SupportOriginSequence {
            target: target(32, "first"),
            origins: vec![],
        });
    f
}
fn checked(f: &Fixture) -> Checked<Effects> {
    compile_inputs(inputs_with_operations(
        f,
        false,
        OWNED_RULE_OPERATIONS_V17,
        |_| {},
        |_| {},
        |_| {},
    )?)
}
fn parameter_mut<'a>(f: &'a mut Fixture, name: &str) -> &'a mut ParameterSlotSchema {
    f.schema
        .slots
        .iter_mut()
        .find_map(|slot| match slot {
            SlotDescriptor::Parameter(row) if row.id == summon_parameter(name) => {
                match &mut row.schema {
                    SchemaState::Known(schema) => Some(schema),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap()
}
fn raw_level(f: &mut Fixture, id: u64, level: i64) {
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(id))
        .unwrap()
        .parameters
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p.slot == summon_parameter("level"))
        .unwrap()
        .value = integer(level);
}
fn report(f: &Fixture) -> OwnedSupportMetricReport {
    let plan = metrics(checked(f).unwrap());
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn rejected<T>(result: Checked<T>, expected: &str) {
    let Err(error) = result else {
        panic!("expected {expected}")
    };
    assert!(error.contains(expected), "expected {expected}, got {error}");
}

#[test]
fn shared_raw_slots_keep_two_direct_uses_and_generated_sibling_independent() {
    assert_eq!(numbers(&report(&mixed())), [40.0, 40.0, 44.0, 12.0]);
    let mut f = mixed();
    raw_level(&mut f, 30, 13);
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(28))
        .unwrap()
        .level = 17;
    assert_eq!(numbers(&report(&f)), [46.0, 46.0, 44.0, 18.0]);
    assert_eq!(numbers(&report(&mixed())), [40.0, 40.0, 44.0, 12.0]);
}

#[test]
fn explicit_authority_requires_v17_and_does_not_add_a_self_writer() {
    let f = mixed();
    for version in [
        OWNED_RULE_OPERATIONS_V14,
        OWNED_RULE_OPERATIONS_V15,
        OWNED_RULE_OPERATIONS_V16,
    ] {
        let error = inputs_with_operations(&f, false, version, |_| {}, |_| {}, |_| {});
        assert!(error.is_err(), "explicit authority admitted by {version}");
    }
    let mut f = mixed();
    parameter_mut(&mut f, "level").skill_input = Some(SkillInputAuthority::Authored);
    rejected(checked(&f), "does not permit provider projections");
    let mut f = mixed();
    parameter_mut(&mut f, "level").skill_input = Some(SkillInputAuthority::Projected);
    parameter_mut(&mut f, "level").sites.clear();
    rejected(checked(&f), "invalid schema bindings");
}

#[test]
fn missing_generated_writer_cannot_borrow_authored_sibling_values() {
    let mut f = mixed();
    remove_projection(&mut f, physical_owner(), "supply-summon", "summon-level");
    let report = report(&f);
    for (row, expected) in report.evaluation.results[..3]
        .iter()
        .zip([40.0, 40.0, 44.0])
    {
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(expected)
            }
        );
    }
    assert!(!matches!(
        report.evaluation.results[3].value,
        EffectValue::Known { .. }
    ));
}

#[test]
fn child_final_inputs_never_fall_back_to_direct_parent_raw_inputs() {
    let mut f = mixed();
    remove_projection(&mut f, actor_owner(), "assemble-first", "final-level");
    let report = report(&f);
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
    assert!(!matches!(
        report.evaluation.results[3].value,
        EffectValue::Known { .. }
    ));
}

#[test]
fn duplicate_generated_writers_still_reject_with_shared_authority() {
    let mut f = mixed();
    let program = program_mut(&mut f, physical_owner(), "supply-summon");
    let mut duplicate = program
        .effects
        .iter()
        .find(|e| e.id == key("summon-level"))
        .unwrap()
        .clone();
    duplicate.id = key("duplicate-level");
    program.effects.push(duplicate);
    assert!(checked(&f).is_err());
}

#[test]
fn unused_required_inputs_gate_only_their_declared_producer_domain() {
    use poe_optimizer_core::owned_readiness::{ParameterReadiness, ReadinessPhase};
    for authority in [
        SkillInputAuthority::Authored,
        SkillInputAuthority::Projected,
    ] {
        let mut f = mixed();
        let parameter = summon_parameter("domain-sentinel");
        for row in &mut f.schema.definitions {
            if let DefinitionDescriptor::Skill(row) = row
                && row.id == def::<SkillDefinition>("summon")
            {
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema
                    .declarations
                    .parameters
                    .members
                    .push(parameter.clone());
            }
        }
        f.schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: parameter.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: if authority == SkillInputAuthority::Authored {
                        vec![ParameterSite::SkillParameter]
                    } else {
                        vec![]
                    },
                    skill_input: Some(authority),
                }),
            }));
        if authority == SkillInputAuthority::Authored {
            for skill in &mut f.build.skills {
                if matches!(skill.source, AuthoredSkillSource::Direct(_)) {
                    skill
                        .parameters
                        .as_mut()
                        .unwrap()
                        .push(ParameterAssignment {
                            slot: parameter.clone(),
                            value: ParameterValue::Boolean(true),
                        });
                }
            }
        } else {
            program_mut(&mut f, physical_owner(), "supply-summon")
                .effects
                .push(effect(
                    "domain-sentinel",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: summon_supply(),
                        parameter: parameter.clone(),
                        value: key("present"),
                    },
                ));
        }
        let inputs = inputs_with_operations(
            &f,
            false,
            OWNED_RULE_OPERATIONS_V17,
            |_| {},
            |stages| {
                stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .skills
                    .iter_mut()
                    .find(|row| row.skill == def("summon"))
                    .unwrap()
                    .parameters
                    .members
                    .push(ParameterReadiness {
                        parameter: parameter.clone(),
                        phase: ReadinessPhase::Execution,
                    });
            },
            |_| {},
        )
        .unwrap();
        let plan = metrics(compile_inputs(inputs).unwrap());
        assert_eq!(
            numbers(&plan.evaluate(&mut plan.new_scratch()).unwrap()),
            [40.0, 40.0, 44.0, 12.0]
        );
    }
}

#[test]
fn inactive_generated_sibling_keeps_missing_projection_unavailable() {
    let mut f = mixed();
    remove_projection(&mut f, physical_owner(), "supply-summon", "summon-level");
    let program = program_mut(&mut f, physical_owner(), "supply-summon");
    program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("present"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    let report = report(&f);
    for (row, expected) in report.evaluation.results[..3]
        .iter()
        .zip([40.0, 40.0, 44.0])
    {
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(expected)
            }
        );
    }
    assert!(!matches!(
        report.evaluation.results[3].value,
        EffectValue::Known { .. }
    ));
}

#[test]
fn worker_scratch_reuse_parallel_runs_and_failed_attempts_keep_exact_inputs() {
    let a = metrics(checked(&mixed()).unwrap());
    let mut changed = mixed();
    raw_level(&mut changed, 30, 13);
    let b = metrics(checked(&changed).unwrap());
    let expected_a = a.evaluate(&mut a.new_scratch()).unwrap();
    let expected_b = b.evaluate(&mut b.new_scratch()).unwrap();
    assert_ne!(a.identity(), b.identity());
    assert_eq!(numbers(&expected_a), [40.0, 40.0, 44.0, 12.0]);
    assert_eq!(numbers(&expected_b), [46.0, 46.0, 44.0, 12.0]);
    let mut scratch = a.new_scratch();
    for (plan, expected) in [(&a, &expected_a), (&b, &expected_b), (&a, &expected_a)] {
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), expected);
    }
    assert!(a.evaluate_with_budget(&mut scratch, &mut 0).is_err());
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected_a);
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..48).into_par_iter().for_each_init(
                || a.new_scratch(),
                |scratch, i| {
                    let (plan, expected) = if i % 2 == 0 {
                        (&a, &expected_a)
                    } else {
                        (&b, &expected_b)
                    };
                    assert_eq!(&plan.evaluate(scratch).unwrap(), expected);
                },
            );
        });
}
