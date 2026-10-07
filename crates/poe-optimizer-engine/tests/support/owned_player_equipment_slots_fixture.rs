//! Synthetic equipment and existing Player Actor over the public staged engine.
#![allow(dead_code)]
#[path = "owned_preparation_readiness_fixture.rs"]
pub mod readiness;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_source_properties::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
pub use readiness::delivery::{self, fixture as base};
pub use readiness::{Checked, Effects, Fixture, Inputs, def, effect, key, occurrence, subject};

pub fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
pub fn actor_owner() -> SchemaSubject {
    subject(def::<ActorDefinition>("slot-player"))
}
pub fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
pub fn slot_read(
    name: &str,
    slot: &str,
    read: PlayerEquipmentSlotRead,
    value_type: ComputedValueType,
) -> RuleRead {
    RuleRead {
        id: key(name),
        source: RuleReadSource::PlayerEquipmentSlot {
            slot: def(slot),
            read,
        },
        value_type,
    }
}
pub fn fixture() -> Fixture {
    let mut f = readiness::fixture();
    let original = Fixture::new();
    f.build.items = original.build.items;
    f.build.equipment = original.build.equipment;
    f.build.items[0].item_level = Some(27);
    f.schema.definitions.extend([
        DefinitionDescriptor::Actor(known(
            def("slot-player"),
            ActorSchema {
                declarations: ports(),
            },
        )),
        DefinitionDescriptor::Stat(known(
            def("slot-value"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::EquipmentUse],
            },
        )),
        DefinitionDescriptor::Capability(known(
            def("slot-capability"),
            CapabilitySchema {
                targets: vec![RuleEntityKind::EquipmentUse],
            },
        )),
    ]);
    f.owner_mut(&base::item_owner()).programs.members.extend([
        RuleProgram {
            id: key("slot-value"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![base::read("level", RuleReadSource::ItemLevel)],
            nodes: vec![base::read_node("level", "level")],
            effects: vec![base::derive(
                "value",
                RuleEntity::Current,
                "slot-value",
                "level",
            )],
        },
        RuleProgram {
            id: key("slot-capability"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![RuleRead {
                id: key("capability"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Parameter {
                    slot: base::parameter(SlotOwnerDefId::ItemTemplate(def("item")), "needs-level"),
                },
            }],
            nodes: vec![base::read_node("capability", "capability")],
            effects: vec![effect(
                "capability",
                RuleEffectKind::Capability {
                    entity: RuleEntity::Current,
                    capability: def("slot-capability"),
                    enabled: key("capability"),
                },
            )],
        },
    ]);
    let mut programs = vec![];
    for slot in ["weapon", "other"] {
        for (suffix, value) in [
            ("occupied", ComputedValueType::Boolean),
            ("capability", ComputedValueType::Boolean),
            ("value", ComputedValueType::Integer),
            ("lazy", ComputedValueType::Integer),
        ] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(&format!("{slot}-{suffix}")),
                StatSchema {
                    value,
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
        }
        programs.push(RuleProgram {
            id: key(&format!("read-{slot}")),
            context: RuleEntityKind::Actor,
            reads: vec![
                slot_read(
                    "occupied",
                    slot,
                    PlayerEquipmentSlotRead::Occupied,
                    ComputedValueType::Boolean,
                ),
                slot_read(
                    "value",
                    slot,
                    PlayerEquipmentSlotRead::Stat {
                        stat: def("slot-value"),
                    },
                    ComputedValueType::Integer,
                ),
                slot_read(
                    "capability",
                    slot,
                    PlayerEquipmentSlotRead::Capability {
                        capability: def("slot-capability"),
                    },
                    ComputedValueType::Boolean,
                ),
            ],
            nodes: vec![
                base::read_node("occupied", "occupied"),
                base::read_node("value", "value"),
                base::read_node("capability", "capability"),
                base::literal("empty", 17),
                base::node(
                    "lazy",
                    RuleExpression::Select {
                        condition: key("occupied"),
                        when_true: key("value"),
                        when_false: key("empty"),
                    },
                ),
            ],
            effects: ["occupied", "value", "capability", "lazy"]
                .into_iter()
                .map(|s| base::derive(s, RuleEntity::Current, &format!("{slot}-{s}"), s))
                .collect(),
        });
    }
    f.owners.push(DefinitionRules {
        owner: actor_owner(),
        programs: DeclaredSet::complete(programs),
    });
    f
}
pub fn registry() -> DeclaredSet<ExistingActorRuleApplication> {
    DeclaredSet::complete(vec![ExistingActorRuleApplication {
        id: key("slot-player"),
        owner: def("slot-player"),
        targets: vec![ExistingActorRuleTarget::Player],
    }])
}
pub fn inputs_with(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
) -> Checked<Inputs> {
    readiness::inputs_with_operations(
        f,
        false,
        OWNED_RULE_OPERATIONS_V21,
        |rules| {
            rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
            rules.existing_actor_rules = Some(registry());
            edit_rules(rules);
        },
        |stages| {
            stages.schema_version = 4;
            edit_stages(stages);
        },
        |receiving| {
            receiving.schema_version = 3;
            receiving.source_properties = Some(SourcePropertyPreparationInput {
                relations: DeclaredSet::complete(vec![]),
            });
        },
    )
}
pub fn inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(f, |_| {}, |_| {})
}
pub fn compile(f: &Fixture) -> Checked<Effects> {
    readiness::compile_inputs(inputs(f)?)
}
pub fn evaluate(f: &Fixture) -> SupportEffectsReport {
    let p = compile(f).unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
pub fn value<'a>(r: &'a OwnedEffectsReport, name: &str) -> &'a EffectValue {
    &r.values
        .iter()
        .find(|r| {
            r.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(name),
                }
        })
        .unwrap()
        .value
}
pub fn count(r: &OwnedEffectsReport) -> usize {
    r.effects
        .iter()
        .filter(|e| matches!(e.key.invocation.origin, RuleOrigin::ExistingActor { .. }))
        .count()
}
pub fn boolean(v: bool) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Boolean(v),
    }
}
pub fn edit_early(
    stages: &mut EvaluationStagesInput,
    owner: &SchemaSubject,
    program: &str,
    phase: ReadinessPhase,
    outputs: Vec<StageChannel>,
) {
    let row = stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|r| &r.owner == owner && r.program == key(program))
        .unwrap();
    row.phase = phase;
    row.role = ReadinessProgramRole::PreparationFacts;
    row.outputs = outputs;
    stages
        .programs
        .members
        .iter_mut()
        .find(|r| &r.owner == owner && r.program == key(program))
        .unwrap()
        .stage = key("prepare");
}
pub fn rejected<T>(result: Checked<T>, needle: &str) {
    let Err(e) = result else {
        panic!("expected {needle}")
    };
    assert!(e.contains(needle), "expected {needle:?}, got {e:?}");
}
