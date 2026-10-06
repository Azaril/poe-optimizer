//! Selection identity is a cold, typed read of an already established Action.
//! Synthetic definitions exercise the public preparation and receiving graph;
//! these tests do not classify game mechanics or manufacture action availability.
#[allow(dead_code)]
#[path = "support/owned_preparation_readiness_fixture.rs"]
mod support;

use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_core::{
    owned_stages::OWNED_EVALUATION_STAGES_V3, owned_support_receiving::OWNED_SUPPORT_RECEIVING_V3,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::{owned_plan::*, owned_rules::*};
use rayon::prelude::*;
use std::collections::BTreeMap;
use support::delivery::fixture as base;
use support::*;

fn output_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::ActionOutput(delivery::output()))
}

fn observer() -> RuleProgram {
    let reads = [
        ("part", RuleReadSource::ActionPartIs { part: def("part") }),
        ("mode", RuleReadSource::ActionModeIs { mode: def("mode") }),
        (
            "set",
            RuleReadSource::ActionStatSetIs {
                stat_set: def("set"),
            },
        ),
    ];
    RuleProgram {
        id: key("selection-observer"),
        context: RuleEntityKind::Action,
        reads: reads
            .into_iter()
            .map(|(id, source)| RuleRead {
                id: key(id),
                value_type: ComputedValueType::Boolean,
                source,
            })
            .collect(),
        nodes: ["part", "mode", "set"]
            .into_iter()
            .map(|name| base::read_node(name, name))
            .collect(),
        effects: ["part", "mode", "set"]
            .into_iter()
            .map(|name| {
                effect(
                    name,
                    RuleEffectKind::Requirement {
                        satisfied: key(name),
                        code: key(name),
                    },
                )
            })
            .collect(),
    }
}

fn selection_fixture() -> Fixture {
    let mut f = fixture();
    f.schema.definitions.extend([
        DefinitionDescriptor::ActionMode(DefinitionEntry {
            id: def("mode-two"),
            schema: SchemaState::Known(ActionModeSchema {}),
        }),
        DefinitionDescriptor::ActionStatSet(DefinitionEntry {
            id: def("set-two"),
            schema: SchemaState::Known(ActionStatSetSchema {}),
        }),
    ]);
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::ActionOutput(row) = slot
            && row.id == delivery::output()
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.modes.members.push(def("mode-two"));
            schema.stat_sets.members.push(def("set-two"));
        }
    }
    f.owner_mut(&output_owner())
        .programs
        .members
        .push(observer());
    f
}

fn plan(f: &Fixture, edit: impl FnOnce(&mut RulePackageInput)) -> Effects {
    compile_inputs(
        inputs_with_operations(
            f,
            false,
            OWNED_RULE_OPERATIONS_V20,
            edit,
            |stages| stages.schema_version = OWNED_EVALUATION_STAGES_V3,
            |receiving| {
                receiving.schema_version = OWNED_SUPPORT_RECEIVING_V3;
                receiving.source_properties = Some(
                    poe_optimizer_core::owned_source_properties::SourcePropertyPreparationInput {
                        relations: DeclaredSet::complete(vec![]),
                    },
                );
            },
        )
        .expect("typed V20 packages"),
    )
    .expect("closed readiness graph")
}

fn projection(
    report: &SupportEffectsReport,
) -> BTreeMap<(ActionSelection, OwnedDefinitionKey), EffectValue> {
    delivery::evaluated(report)
        .effects
        .iter()
        .filter(|row| row.key.invocation.program == key("selection-observer"))
        .map(|row| {
            let ConcreteEntity::Action(action) = &row.key.invocation.entity else {
                panic!("selection observer must retain its Action");
            };
            (
                (action.as_ref().clone(), row.key.effect.clone()),
                row.value.clone(),
            )
        })
        .collect()
}

fn expected(action: &ActionSelection, axis: &OwnedDefinitionKey) -> bool {
    match axis.as_str() {
        "part" => action.part == def("part"),
        "mode" => action.mode == def("mode"),
        "set" => action.stat_set == def("set"),
        _ => panic!("unknown observer axis"),
    }
}

#[test]
fn exact_action_axes_remain_independent_across_receivers_and_query_selection() {
    let mut f = selection_fixture();
    let p = plan(&f, |_| {});
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    let values = projection(&report);
    assert_eq!(values.len(), 3 * 2 * 2 * 2 * 3);
    for ((action, axis), value) in &values {
        assert_eq!(
            *value,
            EffectValue::Known {
                value: ParameterValue::Boolean(expected(action, axis))
            }
        );
        assert!(
            [30, 31]
                .into_iter()
                .any(|id| action.action.provider.root == ProviderRoot::SkillUse(occurrence(id)))
        );
    }
    f.queries.requests.reverse();
    let reordered = plan(&f, |_| {});
    let reordered_report = reordered.evaluate(&mut reordered.new_scratch()).unwrap();
    // QuerySpec deliberately preserves measurement order. A different request
    // has a different plan identity while the exact Action facts stay equal.
    assert_ne!(reordered_report.identity, report.identity);
    assert!(
        projection(&reordered_report) == values,
        "query order changed Action facts"
    );
    f.queries.requests.truncate(1);
    let subset = plan(&f, |_| {});
    assert_eq!(
        projection(&subset.evaluate(&mut subset.new_scratch()).unwrap()),
        values,
        "declared support receivers do not depend on requested metrics"
    );
}

#[test]
fn support_applicability_reads_the_exact_receiving_action_not_the_assigned_skill() {
    let f = selection_fixture();
    let p = plan(&f, |rules| {
        let program = package_program_mut(rules, &delivery::support_owner(), "action-app");
        let reads = observer();
        program.reads = reads.reads;
        program.nodes = reads.nodes;
        program.nodes.push(base::node(
            "selected",
            RuleExpression::All {
                values: ["part", "mode", "set"].into_iter().map(key).collect(),
            },
        ));
        program.effects[0].effect = RuleEffectKind::SupportApplicability {
            applicable: key("selected"),
        };
    });
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    let mut seen = 0;
    let mut admitted = 0;
    for row in &delivery::evaluated(&report).values {
        let PlanValueKey::SupportApplicability { application } = &row.key else {
            continue;
        };
        let SupportReceiverKey::Action(action) = &application.receiver else {
            continue;
        };
        let selected = action.part == def("part")
            && action.mode == def("mode")
            && action.stat_set == def("set");
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(selected)
            }
        );
        assert!(matches!(
            application.prepared.origin,
            SupportOrigin::Assignment(_)
        ));
        seen += 1;
        admitted += usize::from(selected);
    }
    assert_eq!((seen, admitted), (24, 3));
    let deliveries: Vec<_> = delivery::evaluated(&report)
        .effects
        .iter()
        .filter(|row| row.key.invocation.program == key("action-deliver"))
        .collect();
    assert_eq!(deliveries.len(), 24);
    for row in deliveries {
        let application = delivery::application(row).unwrap();
        let SupportReceiverKey::Action(action) = &application.receiver else {
            unreachable!()
        };
        let selected = action.part == def("part")
            && action.mode == def("mode")
            && action.stat_set == def("set");
        if selected {
            assert!(matches!(row.value, EffectValue::Known { .. }));
        } else {
            assert_eq!(row.value, EffectValue::Inactive);
        }
    }
}

fn semantic(
    f: &Fixture,
    p: RuleProgram,
    operations: &str,
) -> std::result::Result<CompiledRulePackage, RuleError> {
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut input = base::raw_rules(f, &schema, operations);
    input.owners = vec![DefinitionRules {
        owner: output_owner(),
        programs: DeclaredSet::complete(vec![p]),
    }];
    input.tables.clear();
    input.receivers = DeclaredSet::complete(vec![]);
    input.effect_applications =
        (operations != OWNED_RULE_OPERATIONS_V14).then(|| DeclaredSet::complete(vec![]));
    CompiledRulePackage::compile(&input, &schema, Default::default())
}

#[test]
fn semantic_validation_requires_known_typed_definitions_action_context_boolean_and_v20() {
    let f = selection_fixture();
    assert!(semantic(&f, observer(), OWNED_RULE_OPERATIONS_V20).is_ok());
    for operations in [
        OWNED_RULE_OPERATIONS_V14,
        OWNED_RULE_OPERATIONS_V19,
        "owned-domain-operations-v999",
    ] {
        assert!(
            semantic(&f, observer(), operations).is_err(),
            "{operations}"
        );
    }
    for axis in 0..3 {
        let mut p = observer();
        p.reads[axis].value_type = ComputedValueType::Integer;
        assert!(semantic(&f, p, OWNED_RULE_OPERATIONS_V20).is_err());
        let mut p = observer();
        p.reads[axis].source = match axis {
            0 => RuleReadSource::ActionPartIs {
                part: def("missing"),
            },
            1 => RuleReadSource::ActionModeIs {
                mode: def("missing"),
            },
            _ => RuleReadSource::ActionStatSetIs {
                stat_set: def("missing"),
            },
        };
        assert!(
            semantic(&f, p, OWNED_RULE_OPERATIONS_V20)
                .unwrap_err()
                .to_string()
                .contains("missing, unmapped, foreign")
        );
        let foreign = GameVersionNamespace::new("foreign", "v1").unwrap();
        let mut p = observer();
        p.reads[axis].source = match axis {
            0 => RuleReadSource::ActionPartIs {
                part: DefId::parse(foreign, "part").unwrap(),
            },
            1 => RuleReadSource::ActionModeIs {
                mode: DefId::parse(foreign, "mode").unwrap(),
            },
            _ => RuleReadSource::ActionStatSetIs {
                stat_set: DefId::parse(foreign, "set").unwrap(),
            },
        };
        assert!(semantic(&f, p, OWNED_RULE_OPERATIONS_V20).is_err());
    }
    let mut unmapped = selection_fixture();
    let unresolved: ActionStatSetDefId = def("unresolved-set");
    unmapped
        .schema
        .definitions
        .push(DefinitionDescriptor::ActionStatSet(DefinitionEntry {
            id: unresolved.clone(),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: subject(unresolved.clone()),
                    facet: SchemaFacet::InputSchema,
                    code: key("unreviewed"),
                }],
            },
        }));
    let mut p = observer();
    p.reads[2].source = RuleReadSource::ActionStatSetIs {
        stat_set: unresolved,
    };
    assert!(
        semantic(&unmapped, p, OWNED_RULE_OPERATIONS_V20)
            .unwrap_err()
            .to_string()
            .contains("missing, unmapped, foreign")
    );
    for context in [
        RuleEntityKind::Actor,
        RuleEntityKind::Enemy,
        RuleEntityKind::Environment,
        RuleEntityKind::EquipmentUse,
        RuleEntityKind::Skill,
        RuleEntityKind::SupportOrigin,
    ] {
        let mut p = observer();
        p.context = context;
        assert!(
            semantic(&f, p, OWNED_RULE_OPERATIONS_V20).is_err(),
            "{context:?}"
        );
    }
}

#[test]
fn action_queries_do_not_bypass_disabled_roots_or_missing_execution_inputs() {
    let mut f = selection_fixture();
    f.build
        .gems
        .iter_mut()
        .find(|row| row.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let effects = plan(&f, |_| {});
    let observed = effects.evaluate(&mut effects.new_scratch()).unwrap();
    for ((action, axis), value) in projection(&observed) {
        if action.action.provider.root == ProviderRoot::SkillUse(occurrence(30)) {
            assert_eq!(value, EffectValue::Inactive);
        } else {
            assert_eq!(
                value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(expected(&action, &axis))
                }
            );
        }
    }
    let p = metrics(effects);
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(report.evaluation.results[0].value, EffectValue::Inactive);
    assert_eq!(report.evaluation.results[1].value, EffectValue::Inactive);
    assert!(matches!(
        report.evaluation.results[2].value,
        EffectValue::Known { .. }
    ));

    remove_projection(&mut f, actor_owner(), "assemble-first", "final-quality");
    let p = metrics(plan(&f, |_| {}));
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(report.evaluation.results[0].value, EffectValue::Inactive);
    assert_eq!(report.evaluation.results[1].value, EffectValue::Inactive);
    assert!(matches!(
        report.evaluation.results[2].value,
        EffectValue::Unresolved { .. }
    ));
}

#[test]
fn scratch_rebinding_and_parallel_evaluation_preserve_selection_identity() {
    let f = selection_fixture();
    let a = plan(&f, |_| {});
    let b = plan(&f, |rules| {
        package_program_mut(rules, &output_owner(), "selection-observer").reads[2].source =
            RuleReadSource::ActionStatSetIs {
                stat_set: def("set-two"),
            };
    });
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    let middle = b.evaluate(&mut scratch).unwrap();
    assert_ne!(projection(&first), projection(&middle));
    assert!(
        a.evaluate(&mut scratch).unwrap() == first,
        "same-request A/B/A replay changed"
    );
    let results: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(results.iter().all(|value| value == &first));
}
