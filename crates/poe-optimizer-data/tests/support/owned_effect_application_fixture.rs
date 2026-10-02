//! Direct-authored effect application data; no source game or build assumptions.
#[allow(dead_code)]
#[path = "owned_rule_receiver_fixture.rs"]
mod receiver;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
pub use receiver::{id, key, ns, slot};

pub fn value(amount: f64) -> ParameterValue {
    receiver::value(amount)
}

pub fn parameter() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("source")),
        slot: id("strength"),
    }
}
pub fn choice() -> DeclaredSlot<ChoiceSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("source")),
        slot: id("enabled"),
    }
}
pub fn schema() -> OwnedDefinitionSchemaPackage {
    let mut input = receiver::schema_input();
    let mut declarations = DeclaredSlots {
        parameters: DeclaredSet::complete(vec![parameter()]),
        choices: DeclaredSet::complete(vec![choice()]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    };
    input
        .definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: id("source"),
            schema: SchemaState::Known(SkillSchema {
                directly_selectable: true,
                declarations: declarations.clone(),
            }),
        }));
    declarations.parameters = DeclaredSet::complete(vec![]);
    declarations.choices = DeclaredSet::complete(vec![]);
    input
        .definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: id("other-source"),
            schema: SchemaState::Known(SkillSchema {
                directly_selectable: true,
                declarations,
            }),
        }));
    input
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: id("source-scalar"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: id("points") },
                targets: vec![RuleEntityKind::Skill],
            }),
        }));
    for name in ["final", "other"] {
        if let Some(DefinitionDescriptor::Stat(entry)) = input
            .definitions
            .iter_mut()
            .find(|v| v.address() == id::<StatDefinition>(name).address())
        {
            let SchemaState::Known(schema) = &mut entry.schema else {
                unreachable!()
            };
            schema.targets.push(RuleEntityKind::Enemy);
        } else {
            input
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: id(name),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Quantity { unit: id("factor") },
                        targets: vec![RuleEntityKind::Actor, RuleEntityKind::Enemy],
                    }),
                }));
        }
    }
    input.slots.push(SlotDescriptor::Parameter(DefinitionEntry {
        id: parameter(),
        schema: SchemaState::Known(ParameterSlotSchema {
            value: ValueSchema::Quantity(QuantityRange {
                minimum: FiniteQuantity::new(-100.0, id("points")).unwrap(),
                maximum: FiniteQuantity::new(100.0, id("points")).unwrap(),
            }),
            presence: SlotPresence::OptionalOnce,
            sites: vec![],
        }),
    }));
    input.slots.push(SlotDescriptor::Choice(DefinitionEntry {
        id: choice(),
        schema: SchemaState::Known(ChoiceSlotSchema {
            value: ValueSchema::Boolean,
            presence: SlotPresence::RequiredOnce,
            owners: vec![ChoiceOwnerScope::Skill],
        }),
    }));
    OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap()
}
pub fn application(name: &str) -> EffectApplicationRule {
    EffectApplicationRule {
        id: key(name),
        source: EffectApplicationSource::Skill {
            skill: id("source"),
        },
        targets: vec![
            EffectApplicationTarget::Player,
            EffectApplicationTarget::OwnedSlot {
                slot: slot("first"),
            },
        ],
        activation: key("enabled"),
        program: RuleProgram {
            id: key("deliver"),
            context: RuleEntityKind::Actor,
            reads: vec![
                RuleRead {
                    id: key("active-input"),
                    source: RuleReadSource::EffectSourceChoice { slot: choice() },
                    value_type: ComputedValueType::Boolean,
                },
                RuleRead {
                    id: key("amount-input"),
                    source: RuleReadSource::EffectSourceParameter { slot: parameter() },
                    value_type: ComputedValueType::Quantity { unit: id("points") },
                },
            ],
            nodes: vec![
                RuleNode {
                    id: key("enabled"),
                    expression: RuleExpression::Read {
                        input: key("active-input"),
                    },
                },
                RuleNode {
                    id: key("amount"),
                    expression: RuleExpression::Read {
                        input: key("amount-input"),
                    },
                },
            ],
            effects: vec![RuleEffect {
                id: key("damage"),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: id("final"),
                    contribution: ContributionKind::Increase,
                    value: key("amount"),
                },
            }],
        },
        stacking: vec![EffectStackingRule {
            effect: key("damage"),
            family: key("family"),
            modifier: key("damage"),
            reduction: EffectStackingReduction::Maximum,
        }],
    }
}
pub fn rules(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    RulePackageInput {
        effect_applications: Some(DeclaredSet::complete(vec![application("first")])),
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("applications"),
        semantics_version: key("applications-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_V15),
        definitions: schema.identity().clone(),
        tables: vec![],
        owners: vec![],
        receivers: DeclaredSet::complete(vec![]),
    }
}
pub fn gap() -> SchemaGap {
    SchemaGap {
        subject: SchemaSubject::Definition(id::<SkillDefinition>("source").address()),
        facet: SchemaFacet::GameRules,
        code: key("unconverted-effects"),
    }
}
