//! Generated skill readiness requires every computed RequiredOnce input, even
//! for constant consumers. Projection values and activation are separate facts.
#[allow(dead_code)] // The shared fixture also serves actor, routing and worker tests.
#[path = "support/owned_plan_fixture.rs"]
mod owned_plan_fixture;
use owned_plan_fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::ActionOutputRoutes, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;

fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn generated_owner() -> SchemaSubject {
    subject(def::<SkillDefinition>("generated"))
}
fn generated_parameter(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Skill(def("generated")), name)
}
fn generated_output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("generated")),
        slot: def("generated-output"),
    }
}
fn skill_slot() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(def("item")),
        slot: def("generated-skill"),
    }
}
fn activation() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(def("item")),
        slot: def("activate-generated"),
    }
}
fn supplying_provider(use_id: u64) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::EquipmentUse(occurrence(use_id)),
        grant_path: vec![],
    }
}
fn generated_key(use_id: u64) -> GeneratedSkillKey {
    GeneratedSkillKey {
        provider: supplying_provider(use_id),
        slot: skill_slot(),
    }
}
fn generated_action(use_id: u64) -> ActionSelection {
    let mut provider = supplying_provider(use_id);
    provider.grant_path.push(activation());
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider,
            output: generated_output(),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
fn required_key(use_id: u64, name: &str) -> PlanValueKey {
    PlanValueKey::SkillParameter {
        skill: Box::new(generated_key(use_id)),
        parameter: generated_parameter(name),
    }
}
fn consumer_key(use_id: u64, name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Action(Box::new(generated_action(use_id))),
        stat: def(name),
    }
}
fn constants(owner: SchemaSubject, id: &str, stat: &str, n: i64) -> DefinitionRules {
    DefinitionRules {
        owner,
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key(id),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![literal("constant", n)],
            effects: vec![derive("constant", RuleEntity::Current, stat, "constant")],
        }]),
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let item = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::ItemTemplate(e) if e.id == def("item") => match &mut e.schema {
                SchemaState::Known(v) => Some(v),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    item.declarations.grants.members.push(activation());
    item.declarations.skill_grants.members.push(skill_slot());
    let mut declarations = ports();
    declarations.parameters.members = vec![
        generated_parameter("required-level"),
        generated_parameter("required-false"),
        generated_parameter("optional"),
    ];
    declarations.outputs.members.push(generated_output());
    f.schema.definitions.push(DefinitionDescriptor::Skill(entry(
        def("generated"),
        SkillSchema {
            directly_selectable: false,
            declarations,
        },
    )));
    for name in ["skill-constant", "output-constant"] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
            def(name),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Action],
            },
        )));
    }
    let metric = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Metric(e) => match &mut e.schema {
                SchemaState::Known(v) => Some(v),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    metric.provider_roles.push(ProviderRole::EquipmentUse);
    f.schema.slots.extend([
        SlotDescriptor::Grant(entry(
            activation(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::EquipmentUse],
                target: GrantTarget::Skill(skill_slot()),
            },
        )),
        SlotDescriptor::SkillGrant(entry(
            skill_slot(),
            SkillGrantSlotSchema {
                skill: def("generated"),
                outputs: DeclaredSet::complete(vec![generated_output()]),
            },
        )),
        SlotDescriptor::Parameter(entry(
            generated_parameter("required-level"),
            ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(20).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::Parameter(entry(
            generated_parameter("required-false"),
            ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::Parameter(entry(
            generated_parameter("optional"),
            ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::OptionalOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::ActionOutput(entry(
            generated_output(),
            ActionOutputSchema {
                actor_role: DeclaredActorRole::ProviderActor,
                parts: DeclaredSet::complete(vec![def("part")]),
                modes: DeclaredSet::complete(vec![def("mode")]),
                stat_sets: DeclaredSet::complete(vec![def("set")]),
                choices: empty(),
            },
        )),
    ]);
    f.owner_mut(&item_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("project-required"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![read("level", RuleReadSource::ItemLevel)],
            nodes: vec![
                read_node("level", "level"),
                node(
                    "present-false",
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(false),
                    },
                ),
                node(
                    "emit-level",
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    },
                ),
                node(
                    "active",
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    },
                ),
            ],
            effects: vec![
                RuleEffect {
                    id: key("project-level"),
                    when: Some(key("emit-level")),
                    effect: RuleEffectKind::ProjectSkillParameter {
                        skill: skill_slot(),
                        parameter: generated_parameter("required-level"),
                        value: key("level"),
                    },
                },
                effect(
                    "project-false",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: skill_slot(),
                        parameter: generated_parameter("required-false"),
                        value: key("present-false"),
                    },
                ),
                effect(
                    "activate",
                    RuleEffectKind::ActivateGrant {
                        slot: activation(),
                        enabled: key("active"),
                    },
                ),
            ],
        });
    f.owners.push(constants(
        generated_owner(),
        "generated-constant",
        "skill-constant",
        71,
    ));
    f.owners.push(constants(
        SchemaSubject::Slot(SlotAddress::ActionOutput(generated_output())),
        "output-constant",
        "output-constant",
        72,
    ));
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(SlotAddress::Grant(activation())),
        programs: empty(),
    });
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(SlotAddress::SkillGrant(skill_slot())),
        programs: empty(),
    });
    f.routes.push(ActionOutputRoutes {
        output: generated_output(),
        routes: empty(),
    });
    for use_id in [6, 7] {
        f.queries.requests.push(MetricRequest {
            id: QueryId::new(format!("generated-{use_id}")).unwrap(),
            metric: def("requested"),
            target: MetricTarget::Action(Box::new(generated_action(use_id))),
        });
    }
    f
}
fn projection(f: &mut Fixture) -> &mut RuleProgram {
    f.owner_mut(&item_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("project-required"))
        .unwrap()
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    let mut rows = report.values.iter().filter(|row| &row.key == key);
    let row = rows
        .next()
        .unwrap_or_else(|| panic!("missing {key:?}: {report:?}"));
    assert!(rows.next().is_none());
    &row.value
}
fn consumers(report: &OwnedEffectsReport, expected: impl Fn(&EffectValue) -> bool) {
    for use_id in [6, 7] {
        for name in ["skill-constant", "output-constant"] {
            assert!(
                expected(value(report, &consumer_key(use_id, name))),
                "{report:?}"
            );
        }
    }
    // Both the generated Skill program and exact output program must exist, so
    // passing by omitting a consumer cannot satisfy this test.
    assert_eq!(
        report
            .effects
            .iter()
            .filter(
                |row| row.key.invocation.program == key("generated-constant")
                    || row.key.invocation.program == key("output-constant")
            )
            .count(),
        4
    );
}

#[test]
fn all_required_inputs_gate_constant_consumers_and_known_false_is_present() {
    let f = fixture();
    for owner in [
        generated_owner(),
        SchemaSubject::Slot(SlotAddress::ActionOutput(generated_output())),
    ] {
        assert!(
            f.owners
                .iter()
                .find(|r| r.owner == owner)
                .unwrap()
                .programs
                .members
                .iter()
                .all(|p| p.reads.is_empty())
        );
    }
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{report:?}");
    for use_id in [6, 7] {
        assert_eq!(
            value(&report, &required_key(use_id, "required-level")),
            &EffectValue::Known { value: integer(20) }
        );
        assert_eq!(
            value(&report, &required_key(use_id, "required-false")),
            &EffectValue::Known {
                value: ParameterValue::Boolean(false)
            }
        );
        assert!(
            !report
                .values
                .iter()
                .any(|row| row.key == required_key(use_id, "optional"))
        );
        assert_eq!(
            value(&report, &consumer_key(use_id, "skill-constant")),
            &EffectValue::Known { value: integer(71) }
        );
        assert_eq!(
            value(&report, &consumer_key(use_id, "output-constant")),
            &EffectValue::Known { value: integer(72) }
        );
    }
    consumers(&report, |value| matches!(value, EffectValue::Known { .. }));
}

#[test]
fn each_missing_required_projection_blocks_even_unused_inputs() {
    for removed in ["project-level", "project-false"] {
        let mut f = fixture();
        projection(&mut f)
            .effects
            .retain(|effect| effect.id != key(removed));
        // The complete owner-program membership remains explicit. The missing
        // required producer is not disguised as a generally unmapped package.
        assert!(f.owner_mut(&item_owner()).programs.is_complete());
        let report = evaluate(&f);
        consumers(&report, |value| {
            matches!(value, EffectValue::Unresolved { .. })
        });
        let missing = if removed == "project-level" {
            "required-level"
        } else {
            "required-false"
        };
        for use_id in [6, 7] {
            assert!(
                !report
                    .values
                    .iter()
                    .any(|row| row.key == required_key(use_id, missing))
            );
        }
    }
}

#[test]
fn inactive_or_out_of_range_projection_does_not_satisfy_required_input() {
    let mut guarded = fixture();
    projection(&mut guarded)
        .nodes
        .iter_mut()
        .find(|n| n.id == key("emit-level"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    let report = evaluate(&guarded);
    for use_id in [6, 7] {
        assert_eq!(
            value(&report, &required_key(use_id, "required-level")),
            &EffectValue::Inactive
        );
    }
    consumers(&report, |value| {
        matches!(value, EffectValue::Unresolved { .. })
    });
    let mut invalid = fixture();
    invalid.build.items[0].item_level = Some(21); // Valid item input; generated parameter permits only 1..20.
    let report = evaluate(&invalid);
    for use_id in [6, 7] {
        assert_eq!(
            value(&report, &required_key(use_id, "required-level")),
            &EffectValue::UnsupportedValue { value: integer(21) }
        );
    }
    consumers(&report, |value| {
        matches!(
            value,
            EffectValue::Unresolved {
                reason: PlanGapReason::UpstreamUnavailable,
                ..
            }
        )
    });
}

#[test]
fn false_activation_deactivates_child_with_missing_required_producer() {
    let mut f = fixture();
    let parent = projection(&mut f);
    parent
        .effects
        .retain(|effect| effect.id != key("project-level"));
    parent
        .nodes
        .iter_mut()
        .find(|n| n.id == key("active"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    let report = evaluate(&f);
    for use_id in [6, 7] {
        assert_eq!(
            value(
                &report,
                &PlanValueKey::Grant {
                    provider: supplying_provider(use_id),
                    slot: activation()
                }
            ),
            &EffectValue::Known {
                value: ParameterValue::Boolean(false)
            }
        );
    }
    consumers(&report, |value| matches!(value, EffectValue::Inactive));
}
