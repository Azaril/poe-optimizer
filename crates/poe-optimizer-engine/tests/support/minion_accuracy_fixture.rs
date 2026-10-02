//! Finite component world: real authored programs, explicit external assumptions,
//! complete test-only membership, and dynamic parent/actor/action identities.
#![allow(dead_code)]
#[path = "minion_attack_source_fixture.rs"]
pub mod intrinsic;
pub use intrinsic::{action_value, actor_slot, actor_value, def, key, known, quantity};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn asset<T: DeserializeOwned>(family: &str, name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68")
                .join(family)
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn finite<T: Serialize + DeserializeOwned>(value: &T) -> T {
    fn walk(v: &mut Value) {
        match v {
            Value::Object(o) => {
                if o.contains_key("members") && o.contains_key("closure") {
                    o.insert("closure".into(), serde_json::json!({"kind":"complete"}));
                }
                for value in o.values_mut() {
                    walk(value);
                }
            }
            Value::Array(a) => {
                for value in a {
                    walk(value);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    walk(&mut value);
    serde_json::from_value(value).unwrap()
}
pub fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
pub fn node(id: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(id),
        expression,
    }
}
pub struct World {
    pub intrinsic: intrinsic::World,
}
impl World {
    pub fn new(levels: [u16; 2], block: Option<f64>) -> Self {
        let mut intrinsic = intrinsic::World::new(levels);
        for family in ["configuration-block-inputs", "minion-accuracy"] {
            let dependencies: Vec<DefinitionDescriptor> = asset(family, "dependencies.json");
            for definition in finite(&dependencies) {
                if !intrinsic
                    .f
                    .schema
                    .definitions
                    .iter()
                    .any(|d| d.address() == definition.address())
                {
                    intrinsic.f.schema.definitions.push(definition);
                }
            }
            let extension: Value = asset(family, "extension.json");
            for row in extension["schema"].as_array().unwrap() {
                assert_eq!(row["kind"], "definition");
                let definition: DefinitionDescriptor =
                    serde_json::from_value(row["value"].clone()).unwrap();
                intrinsic
                    .f
                    .schema
                    .definitions
                    .retain(|d| d.address() != definition.address());
                intrinsic.f.schema.definitions.push(finite(&definition));
            }
            let owners: Vec<DefinitionRules> =
                serde_json::from_value(extension["owners"].clone()).unwrap();
            for owner in finite(&owners) {
                if let Some(existing) = intrinsic
                    .f
                    .owners
                    .iter_mut()
                    .find(|r| r.owner == owner.owner)
                {
                    existing.programs.members.extend(owner.programs.members);
                } else {
                    intrinsic.f.owners.push(owner);
                }
            }
            let tables: Vec<IntegerRuleTable> =
                serde_json::from_value(extension["tables"].clone()).unwrap();
            intrinsic.f.tables.extend(tables);
            assert!(extension["receivers"].as_array().unwrap().is_empty());
        }
        let routes: Vec<ActionOutputRoutes> = asset("minion-accuracy", "routes.json");
        for route in routes {
            intrinsic
                .f
                .routes
                .iter_mut()
                .find(|r| r.output == route.output)
                .unwrap()
                .routes
                .members
                .extend(route.routes.members);
        }
        intrinsic.f.scenario.enemy.encounter = def(0x31d1);
        let mut result = Self { intrinsic };
        result.set_block(Some(block.is_some()), block);
        result
    }
    pub fn set_block(&mut self, present: Option<bool>, raw: Option<f64>) {
        let assumptions = &mut self.intrinsic.f.scenario.assumptions;
        assumptions.retain(|a| a.input != def(0x3216) && a.input != def(0x3217));
        if let Some(present) = present {
            assumptions.push(ExternalAssumption {
                input: def(0x3216),
                target: AssumptionTarget::Enemy,
                value: ParameterValue::Boolean(present),
            });
        }
        if let Some(raw) = raw {
            assumptions.push(ExternalAssumption {
                input: def(0x3217),
                target: AssumptionTarget::Enemy,
                value: quantity(raw, 2),
            });
        }
    }
    pub fn compile(&self) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
        self.intrinsic.compile()
    }
    pub fn evaluate(&self) -> OwnedEffectsReport {
        self.intrinsic.evaluate()
    }
    pub fn contribute(&mut self, name: &str, entity: RuleEntity, stat: u64, value: ParameterValue) {
        // Explicit finite counterfactual sources, never source-authority defaults.
        let owner = SchemaSubject::Definition(DefinitionAddress::Class(
            self.intrinsic.f.build.character.class.clone(),
        ));
        self.intrinsic
            .f
            .owner_mut(&owner)
            .programs
            .members
            .push(RuleProgram {
                id: key(name),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![node("value", RuleExpression::Literal { value })],
                effects: vec![RuleEffect {
                    id: key("source"),
                    when: None,
                    effect: RuleEffectKind::Contribute {
                        entity,
                        stat: def(stat),
                        contribution: ContributionKind::Add,
                        value: key("value"),
                    },
                }],
            });
    }
    pub fn action_reduction(&mut self, first: f64, second: f64) {
        let owner = SchemaSubject::Definition(DefinitionAddress::Skill(def(0x21)));
        self.intrinsic
            .f
            .owner_mut(&owner)
            .programs
            .members
            .push(RuleProgram {
                id: key("fixture-action-reduction"),
                context: RuleEntityKind::Action,
                reads: vec![RuleRead {
                    id: key("level"),
                    value_type: ComputedValueType::Integer,
                    source: RuleReadSource::Stat {
                        entity: RuleEntity::Actor,
                        stat: def(0x1c),
                    },
                }],
                nodes: vec![
                    node(
                        "level",
                        RuleExpression::Read {
                            input: key("level"),
                        },
                    ),
                    node("forty-four", RuleExpression::Literal { value: integer(44) }),
                    node(
                        "first",
                        RuleExpression::Compare {
                            operation: RuleComparison::Equal,
                            left: key("level"),
                            right: key("forty-four"),
                        },
                    ),
                    node(
                        "a",
                        RuleExpression::Literal {
                            value: quantity(first, 2),
                        },
                    ),
                    node(
                        "b",
                        RuleExpression::Literal {
                            value: quantity(second, 2),
                        },
                    ),
                    node(
                        "amount",
                        RuleExpression::Select {
                            condition: key("first"),
                            when_true: key("a"),
                            when_false: key("b"),
                        },
                    ),
                ],
                effects: vec![RuleEffect {
                    id: key("reduction"),
                    when: None,
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: def(0x321d),
                        contribution: ContributionKind::Add,
                        value: key("amount"),
                    },
                }],
            });
    }
    pub fn restore_partial(&mut self) {
        let extension: Value = asset("minion-accuracy", "extension.json");
        let owners: Vec<DefinitionRules> =
            serde_json::from_value(extension["owners"].clone()).unwrap();
        for owner in owners {
            self.intrinsic.f.owner_mut(&owner.owner).programs.closure = owner.programs.closure;
        }
    }
}
