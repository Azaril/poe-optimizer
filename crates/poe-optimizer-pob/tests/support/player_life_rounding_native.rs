//! Native scalar-operation control for the source witness. Synthetic definitions
//! supply only an explicit quantity; this is not a resource formula, a build
//! evaluator, or evidence that any game contribution inventory is complete.

use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{
        DefId, DefinitionDomain, FiniteQuantity, GameVersionNamespace, OwnedDefinitionKey,
        StatDefId, UnitDefId,
    },
    owned_rules::{
        DefinitionRules, OWNED_RULE_OPERATIONS_V22, OWNED_RULE_PACKAGE_VERSION, RuleEffect,
        RuleEffectKind, RuleEntity, RuleExpression, RuleNode, RulePackageInput, RuleProgram,
        RuleRead, RuleReadSource, RuleRounding,
    },
    owned_schema::{
        ComputedValueType, DeclaredSet, DefinitionAddress, DefinitionDescriptor, DefinitionEntry,
        RuleEntityKind, SchemaClosure, SchemaState, SchemaSubject, StatSchema, UnitDimension,
        UnitSchema,
    },
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleScratch,
};

fn key(name: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(name).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("native-rounding-test", "v1").unwrap()
}
fn id<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(namespace(), name).unwrap()
}
fn quantity(value: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, id("count")).unwrap())
}
fn stat(name: &str) -> DefinitionDescriptor {
    DefinitionDescriptor::Stat(DefinitionEntry {
        id: id(name),
        schema: SchemaState::Known(StatSchema {
            value: ComputedValueType::Quantity { unit: id("count") },
            targets: vec![RuleEntityKind::Actor],
        }),
    })
}

/// Compiled once; scratch is reset and reused by the production executor on each
/// call. Direct facts here test scalar operations, not source resolution.
pub struct NativeLifeRounding {
    schema: OwnedDefinitionSchemaPackage,
    compiled: CompiledRulePackage,
    scratch: RuleScratch,
    owner: SchemaSubject,
}

impl NativeLifeRounding {
    pub fn new() -> Self {
        let unit: UnitDefId = id("count");
        let owner = SchemaSubject::Definition(DefinitionAddress::Stat(id("rounded")));
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: namespace(),
                release: key("native-rounding-test"),
                semantics_version: key("input-and-computed-schema-v1"),
                definitions: vec![
                    DefinitionDescriptor::Unit(DefinitionEntry {
                        id: unit.clone(),
                        schema: SchemaState::Known(UnitSchema {
                            dimension: UnitDimension::Count,
                        }),
                    }),
                    stat("input"),
                    stat("rounded"),
                    stat("clamped"),
                ],
                slots: vec![],
            },
            Default::default(),
        )
        .unwrap();
        let program = RuleProgram {
            id: key("round-and-lower-bound"),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("input"),
                value_type: ComputedValueType::Quantity { unit: unit.clone() },
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: id("input"),
                },
            }],
            nodes: vec![
                RuleNode {
                    id: key("input"),
                    expression: RuleExpression::Read {
                        input: key("input"),
                    },
                },
                RuleNode {
                    id: key("rounded"),
                    expression: RuleExpression::Round {
                        value: key("input"),
                        quantum: FiniteQuantity::new(1.0, unit).unwrap(),
                        mode: RuleRounding::NearestTiesPositive,
                    },
                },
                RuleNode {
                    id: key("one"),
                    expression: RuleExpression::Literal {
                        value: quantity(1.0),
                    },
                },
                RuleNode {
                    id: key("clamped"),
                    expression: RuleExpression::Maximum {
                        left: key("rounded"),
                        right: key("one"),
                    },
                },
            ],
            effects: ["rounded", "clamped"]
                .into_iter()
                .map(|name| RuleEffect {
                    id: key(name),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: id(name),
                        value: key(name),
                    },
                })
                .collect(),
        };
        let input = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("native-rounding-test"),
            semantics_version: key("native-scalar-rounding-control"),
            operations_version: key(OWNED_RULE_OPERATIONS_V22),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: vec![DefinitionRules {
                owner: owner.clone(),
                programs: DeclaredSet::complete(vec![program]),
            }],
            receivers: DeclaredSet::complete(vec![]),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            contribution_queries: Some(DeclaredSet::complete(vec![])),
            existing_actor_rules: Some(DeclaredSet::complete(vec![])),
        };
        let compiled = CompiledRulePackage::compile(&input, &schema, Default::default()).unwrap();
        let scratch = compiled.new_scratch();
        Self {
            schema,
            compiled,
            scratch,
            owner,
        }
    }

    /// Returns `(rounded, maximum_of_rounded_and_one)` for a finite input.
    pub fn evaluate(&mut self, value: f64) -> (f64, f64) {
        let report = self
            .compiled
            .evaluate(
                &self.owner,
                &key("round-and-lower-bound"),
                &[RuleFact {
                    read: key("input"),
                    value: quantity(value),
                }],
                &self.schema,
                &mut self.scratch,
            )
            .unwrap();
        assert_eq!(report.owner_programs_closure, SchemaClosure::Complete);
        assert_eq!(report.effects.len(), 2);
        let output = |name| {
            let effects: Vec<_> = report
                .effects
                .iter()
                .filter(|e| e.id == key(name))
                .collect();
            assert_eq!(effects.len(), 1);
            let stat: StatDefId = id(name);
            assert_eq!(
                effects[0].effect,
                RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat,
                    value: key(name),
                }
            );
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(value),
            } = &effects[0].disposition
            else {
                panic!("expected applied native quantity: {:?}", effects[0]);
            };
            assert_eq!(
                value.unit(),
                &id::<poe_optimizer_core::owned_definitions::UnitDefinition>("count")
            );
            value.value()
        };
        (output("rounded"), output("clamped"))
    }
}

#[cfg(test)]
mod tests {
    use super::NativeLifeRounding;

    #[test]
    fn native_zero_and_signed_half_ties_use_positive_tie_direction() {
        let mut native = NativeLifeRounding::new();
        for (input, rounded, clamped) in [
            (0.0, 0.0, 1.0),
            (0.5, 1.0, 1.0),
            (-0.5, 0.0, 1.0),
            (1.5, 2.0, 2.0),
            (-1.5, -1.0, 1.0),
            (2.5, 3.0, 3.0),
            (-2.5, -2.0, 1.0),
        ] {
            assert_eq!(native.evaluate(input), (rounded, clamped), "{input}");
        }
    }

    #[test]
    fn native_adjacent_half_values_do_not_become_false_ties() {
        let mut native = NativeLifeRounding::new();
        let below = f64::from_bits(0.5f64.to_bits() - 1);
        let above = f64::from_bits(0.5f64.to_bits() + 1);
        assert_eq!(native.evaluate(below), (0.0, 1.0));
        assert_eq!(native.evaluate(0.5), (1.0, 1.0));
        assert_eq!(native.evaluate(above), (1.0, 1.0));
    }

    #[test]
    fn native_large_integer_is_preserved_across_reused_scratch() {
        let mut native = NativeLifeRounding::new();
        let large = 4_503_599_627_370_497.0; // 2^52 + 1, exactly represented.
        assert_eq!(native.evaluate(large), (large, large));
        assert_eq!(native.evaluate(-2.5), (-2.0, 1.0));
        assert_eq!(native.evaluate(large), (large, large));
        assert_eq!(NativeLifeRounding::new().evaluate(large), (large, large));
    }
}
