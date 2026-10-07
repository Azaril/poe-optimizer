//! Exact generated ownership runs through the same source census and graph.
#[allow(dead_code)]
#[path = "support/owned_source_properties_fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/owned_generated_source_properties_fixture.rs"]
mod generated;
use generated::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::SlotOwnerDefId, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;

fn number(report: &OwnedEffectsReport, target: SkillTarget, expected: i64) {
    assert_eq!(
        fixture::value(report, &final_key(target)),
        &EffectValue::Known {
            value: base::integer(expected)
        }
    );
}
fn relation_count(report: &OwnedEffectsReport, target: SkillTarget, expected: i64) {
    let key = PlanValueKey::Stat {
        entity: ConcreteEntity::Skill(Box::new(target)),
        stat: def("source-count"),
    };
    assert_eq!(
        fixture::value(report, &key),
        &EffectValue::Known {
            value: base::integer(expected)
        }
    );
}
fn evaluate(plan: &Effects) -> SupportEffectsReport {
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}

#[test]
fn generated_sources_keep_declaring_providers_and_independent_item_and_tree_occurrences() {
    let f = generated::fixture();
    let plan = compile(&f).expect("generated source owners bind exact supplies");
    let report = evaluate(&plan);
    let values = fixture::evaluated(&report);
    for (target, final_value, count) in [
        (fixture::member(30), 33, 1),
        (fixture::member(31), 42, 1),
        (item_target(6, 4), 23, 0),
        (item_target(6, 5), 25, 0),
        (item_target(7, 4), 23, 0),
        (item_target(7, 5), 25, 0),
        (node_target(80), 40, 0),
        (node_target(81), 40, 0),
    ] {
        number(values, target.clone(), final_value);
        relation_count(values, target, count);
    }
    let assemblies: Vec<_> = values
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("source-assembly"))
        .collect();
    assert_eq!(assemblies.len(), 8);
    for effect in assemblies {
        let invocation = &effect.key.invocation;
        let RuleOrigin::SourceProperty {
            owner,
            producer,
            position,
            ..
        } = &invocation.origin
        else {
            panic!("assembly ran through ordinary invocation")
        };
        let SkillTarget::Generated(owner) = owner.as_ref() else {
            panic!("fabricated authored source")
        };
        assert_eq!(
            producer, &owner.provider,
            "declaring parent, not entered child"
        );
        assert_eq!(position, &None);
        match producer.root {
            ProviderRoot::ItemModifier { equipment_use, .. } => {
                assert_eq!(
                    invocation.entity,
                    ConcreteEntity::EquipmentUse(equipment_use)
                );
                assert_eq!(invocation.owner, base::modifier_owner());
            }
            ProviderRoot::Allocation(_) => {
                assert_eq!(invocation.entity, ConcreteEntity::Actor(ActorKey::Player));
                assert_eq!(
                    invocation.owner,
                    subject(def::<
                        poe_optimizer_core::owned_definitions::PassiveNodeDefinition,
                    >("source-node"))
                );
            }
            ProviderRoot::SkillUse(_) => assert_eq!(invocation.owner, readiness::physical_owner()),
            _ => panic!("unexpected producer"),
        }
    }
}

#[test]
fn generated_source_properties_run_without_queries_or_support_assignments() {
    let mut f = generated::fixture();
    fixture::without_supports(&mut f);
    f.queries.requests.clear();
    let report = evaluate(&compile(&f).unwrap());
    let values = fixture::evaluated(&report);
    number(values, fixture::member(30), 31);
    number(values, fixture::member(31), 40);
    number(values, node_target(80), 40);
    number(values, item_target(7, 5), 25);
    for target in [fixture::member(30), node_target(80), item_target(7, 5)] {
        relation_count(values, target, 0);
    }
    assert_eq!(
        values
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key("source-external"))
            .count(),
        8
    );
}

#[test]
fn absent_and_off_loadout_providers_do_not_invoke_assembly_or_retarget_another_copy() {
    let mut f = generated::fixture();
    f.build.allocations[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let report = evaluate(&compile(&f).unwrap());
    let values = fixture::evaluated(&report);
    for target in [node_target(81), item_target(7, 4), item_target(7, 5)] {
        assert!(
            !values
                .values
                .iter()
                .any(|row| row.key == final_key(target.clone()))
        );
    }
    number(values, node_target(80), 40);
    number(values, item_target(6, 5), 25);

    f.build.equipment.clear();
    f.build.allocations.clear();
    f.build.skills.clear();
    f.build.supports.clear();
    f.build.support_origins = Some(vec![]);
    f.queries.requests.clear();
    let report = evaluate(&compile(&f).unwrap());
    assert!(
        !fixture::evaluated(&report)
            .effects
            .iter()
            .any(|e| e.key.invocation.program == key("source-assembly")
                || e.key.invocation.program == key("source-external"))
    );
}

#[test]
fn authored_and_generated_forms_share_final_skill_stats_without_sharing_source_ownership() {
    let mut f = generated::fixture();
    f.schema.schema_version = poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_V5;
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(row) = descriptor
            && row.id == def::<poe_optimizer_core::owned_definitions::SkillDefinition>("summon")
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.directly_selectable = true;
        }
    }
    for descriptor in &mut f.schema.slots {
        if let SlotDescriptor::Parameter(row) = descriptor
            && [
                readiness::summon_parameter("level"),
                readiness::summon_parameter("enabled"),
            ]
            .contains(&row.id)
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.sites = vec![ParameterSite::SkillParameter];
            schema.skill_input = Some(SkillInputAuthority::AuthoredOrProjected);
        }
    }
    f.build.skills.push(SkillUse {
        id: occurrence(82),
        source: AuthoredSkillSource::Direct(def("summon")),
        enabled: true,
        scope: LoadoutScope::Shared,
        parameters: Some(vec![
            ParameterAssignment {
                slot: readiness::summon_parameter("level"),
                value: base::integer(7),
            },
            ParameterAssignment {
                slot: readiness::summon_parameter("enabled"),
                value: ParameterValue::Boolean(true),
            },
        ]),
    });
    add_shared_final_stat(&mut f);
    let report = evaluate(&readiness::compile_inputs(shared_final_inputs(&f).unwrap()).unwrap());
    let values = fixture::evaluated(&report);
    let manual = SkillTarget::Authored(occurrence(82));
    let manual_origins: Vec<_> = values
        .effects
        .iter()
        .filter_map(|effect| match &effect.key.invocation.origin {
            RuleOrigin::SourceProperty {
                relation, owner, ..
            }
            | RuleOrigin::SourcePropertyCensus { relation, owner }
                if owner.as_ref() == &manual =>
            {
                Some(relation)
            }
            _ => None,
        })
        .collect();
    assert!(!manual_origins.is_empty());
    assert!(
        manual_origins
            .iter()
            .all(|relation| **relation == key("manual-shared-source"))
    );
    for (target, expected) in [
        (manual.clone(), 27),
        (fixture::member(30), 33),
        (node_target(80), 40),
        (item_target(6, 4), 23),
    ] {
        assert_eq!(
            fixture::value(
                values,
                &PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target.clone())),
                    stat: def("direct-final"),
                }
            ),
            &EffectValue::Known {
                value: base::integer(expected)
            }
        );
        assert_eq!(
            fixture::value(
                values,
                &PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target)),
                    stat: def("readiness-metric"),
                }
            ),
            &EffectValue::Known {
                value: readiness::quantity(expected as f64)
            }
        );
    }
    number(values, node_target(80), 40);
    number(values, fixture::member(30), 33);
    relation_count(values, manual, 0);
}

#[test]
fn generated_assembly_keeps_the_natural_skill_parent_context() {
    let mut f = generated::fixture();
    add_skill_parent(&mut f);
    let parent = SlotOwnerDefId::Skill(def("skill"));
    let child = target(ProviderRoot::SkillUse(occurrence(90)), parent.clone());
    let input = inputs_with(
        &f,
        |_| {},
        |_| {},
        |receiving| {
            receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members
                .push(relation(supply(parent), "generated-skill-parent"));
        },
    )
    .unwrap();
    let report = evaluate(&readiness::compile_inputs(input).unwrap());
    let values = fixture::evaluated(&report);
    number(values, child.clone(), 34);
    relation_count(values, child.clone(), 0);
    let assembly: Vec<_> = values
        .effects
        .iter()
        .filter(|effect| {
            effect.key.invocation.program == key("source-assembly")
                && matches!(&effect.key.invocation.origin,
            RuleOrigin::SourceProperty { owner, .. } if owner.as_ref() == &child)
        })
        .collect();
    assert_eq!(assembly.len(), 1);
    assert_eq!(
        assembly[0].key.invocation.entity,
        ConcreteEntity::Skill(Box::new(SkillTarget::Authored(occurrence(90))))
    );
    assert_eq!(
        assembly[0].key.invocation.owner,
        subject(def::<poe_optimizer_core::owned_definitions::SkillDefinition>("skill"))
    );
    let RuleOrigin::SourceProperty { producer, .. } = &assembly[0].key.invocation.origin else {
        unreachable!()
    };
    assert_eq!(
        producer,
        &ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(90)),
            grant_path: vec![]
        }
    );
}

#[test]
fn disabled_generated_participation_retains_source_preparation_and_gates_execution() {
    let mut f = generated::fixture();
    let input = participation_inputs(&mut f).unwrap();
    let report = evaluate(&readiness::compile_inputs(input).unwrap());
    let values = fixture::evaluated(&report);
    number(values, node_target(80), 40);
    number(values, node_target(81), 40);
    relation_count(values, node_target(80), 0);
    assert_eq!(
        fixture::value(
            values,
            &PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(Box::new(node_target(80))),
                stat: def("requested-participation"),
            }
        ),
        &EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    assert_eq!(
        fixture::value(
            values,
            &PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(Box::new(node_target(80))),
                stat: def("readiness-metric"),
            }
        ),
        &EffectValue::Inactive
    );
    assert_eq!(
        fixture::value(
            values,
            &PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(Box::new(node_target(81))),
                stat: def("readiness-metric"),
            }
        ),
        &EffectValue::Known {
            value: readiness::quantity(40.0)
        }
    );
}

#[test]
fn duplicate_relation_claims_and_sibling_parameter_projection_are_rejected() {
    let f = generated::fixture();
    let duplicate = inputs_with(
        &f,
        |_| {},
        |_| {},
        |receiving| {
            let relations = &mut receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members;
            let mut duplicate = relations[1].clone();
            duplicate.id = key("duplicate-item-source");
            relations.push(duplicate);
        },
    )
    .and_then(readiness::compile_inputs);
    assert!(
        duplicate.is_err(),
        "one concrete owner cannot belong to two relations"
    );

    let mut sibling = generated::fixture();
    fixture::two_effects(&mut sibling);
    assert!(
        inputs(&sibling)
            .and_then(readiness::compile_inputs)
            .is_err(),
        "the exact-provider program now projects to a sibling as well as its admitted source"
    );
}

#[test]
fn missing_generated_final_producer_remains_unresolved_instead_of_using_raw_input() {
    let mut f = generated::fixture();
    f.owner_mut(&subject(def::<
        poe_optimizer_core::owned_definitions::PassiveNodeDefinition,
    >("source-node")))
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
                .members[2]
                .assembly
                .members
                .clear();
        },
    )
    .unwrap();
    let report = evaluate(&readiness::compile_inputs(inputs).unwrap());
    let values = fixture::evaluated(&report);
    assert!(
        !values
            .values
            .iter()
            .any(|row| row.key == final_key(node_target(80)))
    );
    assert!(matches!(
        fixture::value(
            values,
            &PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(Box::new(node_target(80))),
                stat: def("readiness-metric"),
            }
        ),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
    number(values, item_target(6, 4), 23);
}

#[test]
fn generated_property_cycles_remain_rejected_even_with_no_supports_or_queries() {
    let mut f = generated::fixture();
    fixture::without_supports(&mut f);
    f.queries.requests.clear();
    readiness::program_mut(&mut f, base::class_owner(), "source-external").reads[0] =
        base::contributions("input", RuleEntity::PropertyOwner, "source-add");
    let error = match compile(&f) {
        Ok(_) => panic!("source property self-cycle accepted"),
        Err(error) => error,
    };
    assert!(error.to_lowercase().contains("cycle"), "{error}");
}

#[test]
fn generated_sources_preserve_serial_parallel_and_reused_scratch_results() {
    let f = generated::fixture();
    let a = compile(&f).unwrap();
    let mut changed = generated::fixture();
    changed.build.items[0].modifiers[0].rolls[0].value = base::integer(9);
    let b = compile(&changed).unwrap();
    let mut scratch = a.new_scratch();
    let expected = a.evaluate(&mut scratch).unwrap();
    let changed_report = b.evaluate(&mut scratch).unwrap();
    number(fixture::evaluated(&changed_report), item_target(6, 4), 29);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    for threads in [1, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        let rows = pool.install(|| {
            (0..16)
                .into_par_iter()
                .map_init(
                    || a.new_scratch(),
                    |scratch, _| {
                        b.evaluate(scratch).unwrap();
                        a.evaluate(scratch).unwrap()
                    },
                )
                .collect::<Vec<_>>()
        });
        assert!(rows.iter().all(|actual| actual == &expected));
    }
}

#[test]
fn preset_generated_quality_feeds_source_assembly_without_an_invented_stage() {
    use poe_optimizer_core::{
        build_identity::InstanceAllocator, owned_definitions::*, owned_readiness::*,
        owned_source_properties::*, owned_stages::*,
    };
    let mut f = generated::fixture();
    add_shared_final_stat(&mut f);
    f.schema.schema_version = poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_V6;
    let raw = readiness::summon_parameter("preset-quality");
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(row) = definition
            && row.id == def::<SkillDefinition>("summon")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.declarations.parameters.members.push(raw.clone());
        }
    }
    f.schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: raw.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                skill_input: Some(SkillInputAuthority::Projected),
                value: ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(0.0, def("count")).unwrap(),
                    maximum: FiniteQuantity::new(100.0, def("count")).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            }),
        }));
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::SkillGrant(row) = slot
            && [
                readiness::summon_supply(),
                supply(modifier()),
                supply(passive()),
            ]
            .contains(&row.id)
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.preset_inputs = Some(PresetSkillInputPermission {
                schema_version: 1,
                parameters: DeclaredSet::complete(vec![raw.clone()]),
            });
        }
    }
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("prepared-quality"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: def("count") },
                targets: vec![RuleEntityKind::Skill],
            }),
        }));
    let assembly =
        readiness::program_mut(&mut f, readiness::summon_owner(), "source-direct-assembly");
    assembly.reads.push(RuleRead {
        id: key("quality"),
        value_type: ComputedValueType::Quantity { unit: def("count") },
        source: RuleReadSource::Parameter { slot: raw.clone() },
    });
    assembly.nodes.extend([
        base::read_node("quality", "quality"),
        base::node(
            "quality-unit",
            RuleExpression::Literal {
                value: readiness::quantity(1.0),
            },
        ),
        base::node(
            "quality-properties",
            RuleExpression::ScaleInteger {
                value: key("quality-unit"),
                count: key("properties"),
            },
        ),
        base::node(
            "prepared-quality",
            RuleExpression::Add {
                left: key("quality"),
                right: key("quality-properties"),
            },
        ),
    ]);
    assembly.effects.push(base::derive(
        "quality",
        RuleEntity::Current,
        "prepared-quality",
        "prepared-quality",
    ));
    let mut allocator = InstanceAllocator::from_state(f.build.allocator);
    let preset = allocator.allocate().unwrap();
    f.build.allocator = allocator.state();
    let targets = [
        fixture::member(30),
        fixture::member(31),
        item_target(6, 4),
        item_target(6, 5),
        item_target(7, 4),
        item_target(7, 5),
        node_target(80),
        node_target(81),
    ];
    f.build.generated_inputs = Some(GeneratedSkillInputsV1 {
        schema_version: 1,
        bindings: targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let SkillTarget::Generated(target) = target else {
                    unreachable!()
                };
                SelectedGeneratedSkillInput {
                    target: *target.clone(),
                    parameters: vec![ParameterAssignment {
                        slot: raw.clone(),
                        value: readiness::quantity(12.5 + index as f64),
                    }],
                    origin: GeneratedSkillInputOrigin {
                        skill_preset: preset,
                    },
                }
            })
            .collect(),
    });
    let checked = inputs_with_operations(
        &f,
        OWNED_RULE_OPERATIONS_V19,
        |_| {},
        |stages| {
            let ready = stages.readiness.as_mut().unwrap();
            ready
                .skills
                .iter_mut()
                .find(|row| row.skill == def::<SkillDefinition>("summon"))
                .unwrap()
                .parameters
                .members
                .push(ParameterReadiness {
                    parameter: raw.clone(),
                    phase: ReadinessPhase::Structural,
                });
            ready
                .programs
                .members
                .iter_mut()
                .find(|row| {
                    row.owner == readiness::summon_owner()
                        && row.program == key("source-direct-assembly")
                })
                .unwrap()
                .outputs
                .push(StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat: def("prepared-quality"),
                });
        },
        |receiving| {
            for relation in &mut receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members
            {
                relation.inputs.push(def("direct-final"));
                relation
                    .assembly
                    .members
                    .push(SourcePropertyAssemblyProgram {
                        program: key("source-direct-assembly"),
                        binding: SourcePropertyAssemblyBinding::InputOwner,
                    });
            }
        },
    )
    .unwrap();
    let plan = readiness::compile_inputs(checked)
        .expect("preset inputs retain intrinsic Structural readiness");
    let report = evaluate(&plan);
    let values = fixture::evaluated(&report);
    for (index, target) in targets.iter().enumerate() {
        // The two supported sources add 2; the item/tree sources add the common 20.
        let properties = if index < 2 { 22.0 } else { 20.0 };
        assert_eq!(
            fixture::value(
                values,
                &PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target.clone())),
                    stat: def("prepared-quality"),
                }
            ),
            &EffectValue::Known {
                value: readiness::quantity(12.5 + index as f64 + properties)
            }
        );
    }
    assert_eq!(
        values
            .effects
            .iter()
            .filter(|effect| matches!(
                effect.key.invocation.origin,
                RuleOrigin::GeneratedInput { .. }
            ))
            .count(),
        8
    );
    let mut scratch = plan.new_scratch();
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), report);
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), report);
}
