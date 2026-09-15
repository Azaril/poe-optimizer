//! Directly authored receiver definitions; no provider instances or source data.
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("receiver-game", "v1").unwrap()
}
pub fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
pub fn owner() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Stat(id("final")))
}
pub fn slot(s: &str) -> DeclaredSlot<ActorSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Reward(id("supplier")),
        slot: id(s),
    }
}
pub fn parameter() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Reward(id("supplier")),
        slot: id("parameter"),
    }
}
pub fn value(v: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, id("points")).unwrap())
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
pub fn schema_input() -> SchemaPackageInput {
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("schema"),
        semantics_version: key("schema-v1"),
        definitions: vec![
            DefinitionDescriptor::Unit(known(
                id("points"),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
            DefinitionDescriptor::Unit(known(
                id("factor"),
                UnitSchema {
                    dimension: UnitDimension::DimensionlessFactor,
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("final"),
                StatSchema {
                    value: ComputedValueType::Quantity { unit: id("points") },
                    targets: vec![RuleEntityKind::Actor],
                },
            )),
            DefinitionDescriptor::Reward(known(
                id("supplier"),
                RewardSchema {
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(vec![parameter()]),
                        choices: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        actors: DeclaredSet::complete(vec![slot("first"), slot("second")]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                },
            )),
        ],
        slots: vec![
            SlotDescriptor::Actor(known(
                slot("first"),
                ActorSlotSchema {
                    skills: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                },
            )),
            SlotDescriptor::Actor(known(
                slot("second"),
                ActorSlotSchema {
                    skills: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                },
            )),
            SlotDescriptor::Parameter(known(
                parameter(),
                ParameterSlotSchema {
                    value: ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(-100.0, id("points")).unwrap(),
                        maximum: FiniteQuantity::new(100.0, id("points")).unwrap(),
                    }),
                    presence: SlotPresence::OptionalOnce,
                    sites: vec![ParameterSite::RewardParameter],
                },
            )),
        ],
    }
}
pub fn schema() -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(schema_input(), OwnedSchemaLimits::default()).unwrap()
}
pub fn input(s: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    let program = |name: &str, n: f64| RuleProgram {
        id: key(name),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("value"),
            expression: RuleExpression::Literal { value: value(n) },
        }],
        effects: vec![RuleEffect {
            id: key("final"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id("final"),
                value: key("value"),
            },
        }],
    };
    RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("receiver-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: s.identity().clone(),
        tables: vec![],
        owners: vec![DefinitionRules {
            owner: owner(),
            programs: DeclaredSet::complete(vec![program("player", 7.0), program("owned", 11.0)]),
        }],
        receivers: DeclaredSet::complete(vec![
            ActorStatReceiver {
                id: key("player"),
                stat: id("final"),
                program: key("player"),
                targets: vec![ActorReceiverTarget::Player],
            },
            ActorStatReceiver {
                id: key("owned"),
                stat: id("final"),
                program: key("owned"),
                targets: vec![
                    ActorReceiverTarget::OwnedSlot {
                        slot: slot("first"),
                    },
                    ActorReceiverTarget::OwnedSlot {
                        slot: slot("second"),
                    },
                ],
            },
        ]),
    }
}
pub fn gap(code: &str) -> SchemaGap {
    SchemaGap {
        subject: owner(),
        facet: SchemaFacet::GameRules,
        code: key(code),
    }
}
