//! Preset inputs join the ordinary generated-parameter graph without supplying
//! topology, replacing a rule writer, or granting complete-build coverage.
#[allow(dead_code)]
#[path = "support/owned_preparation_readiness_fixture.rs"]
mod support;

use poe_optimizer_core::{
    build_identity::InstanceAllocator, owned_build::*, owned_definitions::*, owned_readiness::*,
    owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_V6;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use support::delivery::fixture as base;
use support::*;

fn raw() -> DeclaredSlot<ParameterSlotDefId> {
    summon_parameter("preset-quality")
}
fn supplied(root: u64) -> GeneratedSkillKey {
    GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(root)),
            grant_path: vec![],
        },
        slot: summon_supply(),
    }
}
fn raw_key(root: u64) -> PlanValueKey {
    PlanValueKey::SkillParameter {
        skill: Box::new(supplied(root)),
        parameter: raw(),
    }
}
fn known(value: ParameterValue) -> EffectValue {
    EffectValue::Known { value }
}
fn declare(
    f: &mut Fixture,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    supplies: &[DeclaredSlot<SkillGrantSlotDefId>],
) {
    let SlotOwnerDefId::Skill(skill) = &parameter.declaration else {
        unreachable!()
    };
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(row) = definition
            && &row.id == skill
            && let SchemaState::Known(schema) = &mut row.schema
        {
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
                skill_input: Some(SkillInputAuthority::Projected),
                value: ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(-100.0, def("count")).unwrap(),
                    maximum: FiniteQuantity::new(200.0, def("count")).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            }),
        }));
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::SkillGrant(row) = slot
            && supplies.contains(&row.id)
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.preset_inputs = Some(PresetSkillInputPermission {
                schema_version: 1,
                parameters: DeclaredSet::complete(vec![parameter.clone()]),
            });
        }
    }
}
fn fixture_with_inputs() -> Fixture {
    let mut f = fixture();
    f.schema.schema_version = OWNED_SCHEMA_PACKAGE_V6;
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::Parameter(row) = slot
            && matches!(row.id.declaration, SlotOwnerDefId::Skill(_))
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.skill_input = Some(SkillInputAuthority::Projected);
        }
    }
    declare(&mut f, raw(), &[summon_supply()]);
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("preset-quality"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: def("count") },
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    let program = program_mut(&mut f, summon_owner(), "supply-child");
    program.reads.push(RuleRead {
        id: key("preset-quality"),
        value_type: ComputedValueType::Quantity { unit: def("count") },
        source: RuleReadSource::Parameter { slot: raw() },
    });
    program
        .nodes
        .push(base::read_node("preset-quality", "preset-quality"));
    program.effects.push(effect(
        "preset-quality",
        RuleEffectKind::ProjectActorStat {
            actor: actor_slot(),
            stat: def("preset-quality"),
            value: key("preset-quality"),
        },
    ));
    let mut allocator = InstanceAllocator::from_state(f.build.allocator);
    let preset = allocator.allocate().unwrap();
    f.build.allocator = allocator.state();
    f.build.generated_inputs = Some(GeneratedSkillInputsV1 {
        schema_version: 1,
        bindings: [(30, 12.5), (31, 20.25)]
            .into_iter()
            .map(|(root, value)| SelectedGeneratedSkillInput {
                target: supplied(root),
                parameters: vec![ParameterAssignment {
                    slot: raw(),
                    value: quantity(value),
                }],
                origin: GeneratedSkillInputOrigin {
                    skill_preset: preset,
                },
            })
            .collect(),
    });
    f
}
fn classify(stages: &mut EvaluationStagesInput) {
    // V19 includes V18's stage contract even when this fixture declares no
    // source-property relation. The reused base fixture intentionally uses V16.
    stages.schema_version = OWNED_EVALUATION_STAGES_V3;
    let readiness = stages.readiness.as_mut().unwrap();
    readiness
        .skills
        .iter_mut()
        .find(|row| row.skill == def::<SkillDefinition>("summon"))
        .unwrap()
        .parameters
        .members
        .push(ParameterReadiness {
            parameter: raw(),
            phase: ReadinessPhase::Structural,
        });
}
fn inputs_for(f: &Fixture, edit: impl FnOnce(&mut EvaluationStagesInput)) -> Checked<Inputs> {
    inputs_with_operations(
        f,
        false,
        OWNED_RULE_OPERATIONS_V19,
        |_| {},
        |stages| {
            classify(stages);
            edit(stages);
        },
        |_| {},
    )
}
fn plan(f: &Fixture) -> Checked<Effects> {
    inputs_for(f, |_| {}).and_then(compile_inputs)
}
fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|row| &row.key == key)
        .unwrap_or_else(|| panic!("missing {key:?}"))
        .value
}
fn rejected<T>(result: Checked<T>, text: &str) {
    let Err(error) = result else {
        panic!("expected {text}")
    };
    assert!(error.contains(text), "expected {text:?}, got {error:?}");
}

#[test]
fn exact_inputs_keep_preset_provenance_and_feed_real_preparation_graph() {
    let f = fixture_with_inputs();
    let plan = plan(&f).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let report = delivery::evaluated(&report);
    for (root, number) in [(30, 12.5), (31, 20.25)] {
        assert_eq!(*value(report, &raw_key(root)), known(quantity(number)));
        assert_eq!(
            *value(
                report,
                &PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor(root)),
                    stat: def("preset-quality"),
                }
            ),
            known(quantity(number))
        );
        let row = report
            .effects
            .iter()
            .find(|row| {
                matches!(&row.key.invocation.origin,
            RuleOrigin::GeneratedInput { target, .. } if **target == supplied(root))
            })
            .unwrap();
        let RuleOrigin::GeneratedInput { origin, target } = &row.key.invocation.origin else {
            unreachable!()
        };
        assert_eq!(
            origin,
            &f.build.generated_inputs.as_ref().unwrap().bindings[0].origin
        );
        assert_eq!(**target, supplied(root));
        assert_eq!(row.value, known(quantity(number)));
    }
    let metrics = metrics(plan);
    assert_eq!(
        numbers(&metrics.evaluate(&mut metrics.new_scratch()).unwrap()),
        [40.0, 40.0, 44.0]
    );
}

#[test]
fn absent_input_is_missing_producer_and_never_an_inferred_zero() {
    let mut f = fixture_with_inputs();
    f.build
        .generated_inputs
        .as_mut()
        .unwrap()
        .bindings
        .remove(0);
    let plan = plan(&f).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let SupportEffectsOutcome::Unavailable { cause, .. } = report.outcome else {
        panic!("{report:?}")
    };
    assert!(
        matches!(
            cause,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ),
        "{cause:?}"
    );
}

#[test]
fn potential_rule_writer_rejected_even_when_false_unused_and_inactive() {
    for disabled in [false, true] {
        let mut f = fixture_with_inputs();
        if disabled {
            f.build
                .skills
                .iter_mut()
                .for_each(|skill| skill.enabled = false);
            f.queries.requests.clear();
        }
        let program = program_mut(&mut f, physical_owner(), "supply-summon");
        program.nodes.extend([
            base::bool_node("never", false),
            base::node(
                "duplicate",
                RuleExpression::Literal {
                    value: quantity(99.0),
                },
            ),
        ]);
        let mut projection = effect(
            "duplicate-preset-input",
            RuleEffectKind::ProjectSkillParameter {
                skill: summon_supply(),
                parameter: raw(),
                value: key("duplicate"),
            },
        );
        projection.when = Some(key("never"));
        program.effects.push(projection);
        rejected(plan(&f), "competes with a potential provider projection");
    }
}

#[test]
fn explicit_permission_and_v19_are_required_without_promoting_legacy_packages() {
    let mut f = fixture_with_inputs();
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::SkillGrant(row) = slot
            && row.id == summon_supply()
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.preset_inputs = None;
        }
    }
    assert!(
        plan(&f).is_err(),
        "schema permission is independent of a supplied literal"
    );
    let f = fixture_with_inputs();
    let old = inputs_with_operations(
        &f,
        false,
        OWNED_RULE_OPERATIONS_V18,
        |_| {},
        classify,
        |_| {},
    );
    rejected(
        old.and_then(compile_inputs),
        "preset generated inputs require operation v19",
    );
}

fn nested_inputs(f: &mut Fixture) {
    let parameter = ability_parameter("preset-raw");
    declare(
        f,
        parameter.clone(),
        &[ability_supply("first"), ability_supply("second")],
    );
    let input = f.build.generated_inputs.as_mut().unwrap();
    let origin = input.bindings[0].origin.clone();
    for root in [30, 31] {
        for name in ["first", "second"] {
            let SkillTarget::Generated(target) = target(root, name) else {
                unreachable!()
            };
            input.bindings.push(SelectedGeneratedSkillInput {
                target: *target,
                parameters: vec![ParameterAssignment {
                    slot: parameter.clone(),
                    value: quantity(root as f64),
                }],
                origin: origin.clone(),
            });
        }
    }
}
fn classify_nested(stages: &mut EvaluationStagesInput) {
    stages
        .readiness
        .as_mut()
        .unwrap()
        .skills
        .iter_mut()
        .find(|row| row.skill == def::<SkillDefinition>("ability"))
        .unwrap()
        .parameters
        .members
        .push(ParameterReadiness {
            parameter: ability_parameter("preset-raw"),
            phase: ReadinessPhase::Structural,
        });
}

#[test]
fn nested_inputs_keep_inactive_and_unknown_parent_grants_distinct() {
    for unknown in [false, true] {
        let mut f = fixture_with_inputs();
        nested_inputs(&mut f);
        let program = program_mut(&mut f, summon_owner(), "supply-child");
        if unknown {
            program
                .effects
                .retain(|effect| effect.id != key("activate-actor"));
        } else {
            f.build
                .gems
                .iter_mut()
                .find(|row| row.id == occurrence(28))
                .unwrap()
                .parameters[0]
                .value = ParameterValue::Boolean(false);
        }
        let plan = inputs_for(&f, classify_nested)
            .and_then(compile_inputs)
            .unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        if unknown {
            let SupportEffectsOutcome::Unavailable { cause, .. } = report.outcome else {
                panic!("{report:?}")
            };
            assert!(
                matches!(
                    cause,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ),
                "{cause:?}"
            );
        } else {
            let report = delivery::evaluated(&report);
            let SkillTarget::Generated(skill) = target(30, "first") else {
                unreachable!()
            };
            assert_eq!(
                *value(
                    report,
                    &PlanValueKey::SkillParameter {
                        skill,
                        parameter: ability_parameter("preset-raw")
                    }
                ),
                EffectValue::Inactive
            );
            assert_eq!(
                *value(report, &raw_key(30)),
                known(quantity(12.5)),
                "the supplying physical parent is still active; only its child actor grant is false"
            );
        }
    }
}

#[test]
fn nested_raw_input_cannot_pull_a_late_parent_producer_into_preparation() {
    let mut f = fixture_with_inputs();
    nested_inputs(&mut f);
    rejected(
        inputs_for(&f, |stages| {
            classify_nested(stages);
            let declaration = stages
                .readiness
                .as_mut()
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|row| row.owner == summon_owner() && row.program == key("supply-child"))
                .unwrap();
            declaration.phase = ReadinessPhase::Execution;
            declaration.role = ReadinessProgramRole::Execution;
            declaration.outputs.clear();
            stages
                .programs
                .members
                .iter_mut()
                .find(|row| row.owner == summon_owner() && row.program == key("supply-child"))
                .unwrap()
                .stage = key("execute");
        })
        .and_then(compile_inputs),
        "later",
    );
}

#[test]
fn inputs_do_not_repair_partial_owner_coverage() {
    let mut f = fixture_with_inputs();
    f.owner_mut(&summon_owner()).programs.closure = partial(summon_owner(), SchemaFacet::GameRules);
    rejected(
        plan(&f),
        "early readiness needs an early phase and complete owner programs",
    );
}

#[test]
fn independent_occurrence_values_and_provenance_survive_scratch_reuse_and_rayon() {
    let a = fixture_with_inputs();
    let mut b = fixture_with_inputs();
    b.build.generated_inputs.as_mut().unwrap().bindings[0].parameters[0].value = quantity(-3.75);
    b.build.generated_inputs.as_mut().unwrap().bindings[1].parameters[0].value = quantity(75.5);
    let a = plan(&a).unwrap();
    let b = plan(&b).unwrap();
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    let changed = b.evaluate(&mut scratch).unwrap();
    assert_ne!(
        delivery::evaluated(&first).values,
        delivery::evaluated(&changed).values
    );
    assert_eq!(a.evaluate(&mut scratch).unwrap(), first);
    let parallel: Vec<_> = (0..12)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(parallel.iter().all(|report| report == &first));
}

#[test]
fn selected_inputs_are_not_query_driven_and_standalone_compile_keeps_readiness_requirement() {
    let mut f = fixture_with_inputs();
    f.queries.requests.clear();
    let input = inputs_for(&f, |_| {}).unwrap();
    let standalone = OwnedEffectPlan::compile(
        input.request.clone(),
        input.definitions.clone(),
        input.rules.clone(),
        input.routing.clone(),
        PlanLimits::default(),
    );
    let Err(error) = standalone else {
        panic!("v19 standalone plan must supply checked readiness")
    };
    assert!(
        error.to_string().contains("requires checked readiness"),
        "{error}"
    );
    let plan = compile_inputs(input).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let report = delivery::evaluated(&report);
    assert_eq!(*value(report, &raw_key(30)), known(quantity(12.5)));
    assert_eq!(*value(report, &raw_key(31)), known(quantity(20.25)));
}

#[test]
fn new_producers_remain_inside_existing_work_limits() {
    let f = fixture_with_inputs();
    let input = inputs_for(&f, |_| {}).unwrap();
    let result = Effects::compile(
        input,
        PlanLimits {
            max_work: 1,
            ..PlanLimits::default()
        },
        Default::default(),
    );
    assert!(
        matches!(result, Err(PlanError::Limit(_))),
        "bounded compile must stop before any graph escapes"
    );
    let plan = plan(&f).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(
        *value(delivery::evaluated(&report), &raw_key(30)),
        known(quantity(12.5))
    );
}
