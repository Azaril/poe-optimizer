//! Synthetic persisted recipe IDs and coefficients, not a game catalog.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{owned_mapping::*, owned_recipe::*};
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
pub fn recipe(coefficient: i64) -> OwnedRecipeInput {
    let namespace = GameVersionNamespace::new("recipe-test", "v1").unwrap();
    let mut registry =
        OwnedIdRegistry::empty(namespace.clone(), OwnedMappingLimits::default()).unwrap();
    let owner: ModifierDefId = registry.allocate_definition().unwrap();
    let stat: StatDefId = registry.allocate_definition().unwrap();
    let declarations = DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    };
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace.clone(),
            release: key("schema-release"),
            semantics_version: key("input-semantics"),
            definitions: vec![
                DefinitionDescriptor::Modifier(DefinitionEntry {
                    id: owner.clone(),
                    schema: SchemaState::Known(ModifierSchema { declarations }),
                }),
                DefinitionDescriptor::Stat(DefinitionEntry {
                    id: stat.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }),
            ],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let owner = SchemaSubject::Definition(owner.address());
    OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: registry.input().clone(),
        schema: schema.input().clone(),
        rules: RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: namespace.clone(),
            release: key("independent-rule-release"),
            semantics_version: key("rule-semantics"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![IntegerRuleTable {
                id: key("coefficients"),
                minimum: BoundedInteger::new(1).unwrap(),
                maximum: BoundedInteger::new(3).unwrap(),
                value_type: ComputedValueType::Integer,
                rows: [0, coefficient, 9]
                    .into_iter()
                    .map(|v| ParameterValue::Integer(BoundedInteger::new(v).unwrap()))
                    .collect(),
            }],
            owners: vec![DefinitionRules {
                owner: owner.clone(),
                programs: DeclaredSet {
                    closure: SchemaClosure::Partial {
                        gaps: vec![SchemaGap {
                            subject: owner,
                            facet: SchemaFacet::GameRules,
                            code: key("other-effects"),
                        }],
                    },
                    members: vec![RuleProgram {
                        id: key("component"),
                        context: RuleEntityKind::Actor,
                        reads: vec![],
                        nodes: vec![
                            RuleNode {
                                id: key("key"),
                                expression: RuleExpression::Literal {
                                    value: ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
                                },
                            },
                            RuleNode {
                                id: key("value"),
                                expression: RuleExpression::LookupIntegerTable {
                                    table: key("coefficients"),
                                    key: key("key"),
                                },
                            },
                        ],
                        effects: vec![RuleEffect {
                            id: key("derive"),
                            when: None,
                            effect: RuleEffectKind::Derive {
                                entity: RuleEntity::Current,
                                stat,
                                value: key("value"),
                            },
                        }],
                    }],
                },
            }],
        },
        routing: ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace,
            release: key("routing-release"),
            definitions: schema.identity().clone(),
            outputs: vec![],
        },
    }
}
