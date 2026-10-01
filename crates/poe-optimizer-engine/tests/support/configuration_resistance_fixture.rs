//! Finite component harness. Only fixture identities are rebased; authored
//! resistance declarations and programs are loaded without modification.
#[allow(dead_code)]
#[path = "owned_plan_fixture.rs"]
mod base;

use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, RoutingLimits},
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
};
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{CompiledRulePackage, RuleLimits},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub encounter: EncounterDefId,
    pub unit: UnitDefId,
    pub inputs: Vec<Input>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub source_name: String,
    pub presence_input: ExternalInputDefId,
    pub value_input: ExternalInputDefId,
    pub contribution_stat: StatDefId,
    pub program: OwnedDefinitionKey,
    pub default_value: ParameterValue,
}

pub fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn asset(name: &str) -> Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/configuration-resistance-inputs");
    serde_json::from_slice(&std::fs::read(root.join(name)).unwrap()).unwrap()
}

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}

fn rebase<T: Serialize + DeserializeOwned>(value: &T, namespace: &GameVersionNamespace) -> T {
    fn visit(value: &mut Value, namespace: &Value) {
        if *value == json!({"game":"owned-plan-test","version":"v1"}) {
            *value = namespace.clone();
            return;
        }
        match value {
            Value::Object(fields) => fields.values_mut().for_each(|v| visit(v, namespace)),
            Value::Array(values) => values.iter_mut().for_each(|v| visit(v, namespace)),
            _ => (),
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    visit(&mut value, &serde_json::to_value(namespace).unwrap());
    typed(&value)
}

pub struct Fixture {
    pub inputs: Inputs,
    pub scenario: ScenarioInput,
    base: base::Fixture,
    authored: DefinitionRules,
}

impl Fixture {
    pub fn new(finite: bool) -> Self {
        let inputs: Inputs = typed(&asset("native-inputs.json"));
        assert_eq!(inputs.schema_version, 1);
        assert_eq!(inputs.inputs.len(), 4);
        let extension = asset("extension.json");
        assert_eq!(extension["schema_version"], 1);
        assert!(extension["tables"].as_array().unwrap().is_empty());
        assert!(extension["receivers"].as_array().unwrap().is_empty());
        let definitions: Vec<DefinitionDescriptor> = extension["schema"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                assert_eq!(row["kind"], "definition");
                typed(&row["value"])
            })
            .collect();
        assert_eq!(definitions.len(), 13);
        let owners: Vec<DefinitionRules> = typed(&extension["owners"]);
        assert_eq!(owners.len(), 1);
        let authored = owners[0].clone();
        assert_eq!(
            authored.owner,
            SchemaSubject::Definition(inputs.encounter.address())
        );
        assert!(!authored.programs.is_complete());
        assert_eq!(authored.programs.members.len(), 4);
        let mut base = base::Fixture::new();
        base.schema = rebase(&base.schema, &inputs.namespace);
        base.build = rebase(&base.build, &inputs.namespace);
        base.scenario = rebase(&base.scenario, &inputs.namespace);
        base.queries = rebase(&base.queries, &inputs.namespace);
        base.owners = rebase(&base.owners, &inputs.namespace);
        base.schema.schema_version = 4;
        base.schema
            .definitions
            .retain(|row| !matches!(row, DefinitionDescriptor::Encounter(_)));
        base.schema.definitions.extend(definitions);
        base.schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id: inputs.unit.clone(),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                }),
            }));
        base.owners.retain(|row| {
            !matches!(
                row.owner,
                SchemaSubject::Definition(DefinitionAddress::Encounter(_))
            )
        });
        let mut owner = authored.clone();
        if finite {
            // Test-only authority covers these four source-owned contributions;
            // it does not repair the production Encounter owner.
            owner.programs.closure = SchemaClosure::Complete;
        }
        for row in &inputs.inputs {
            assert!(owner.programs.members.iter().any(|p| p.id == row.program));
            owner.programs.members.push(RuleProgram {
                id: key(&format!("component-sum-{}", row.program.as_str())),
                context: RuleEntityKind::Enemy,
                reads: vec![RuleRead {
                    id: key("sum"),
                    value_type: ComputedValueType::Quantity {
                        unit: inputs.unit.clone(),
                    },
                    source: RuleReadSource::Contributions {
                        entity: RuleEntity::Current,
                        stat: row.contribution_stat.clone(),
                        contribution: ContributionKind::Add,
                        reduction: ContributionReduction::Sum,
                        empty: ParameterValue::Quantity(
                            FiniteQuantity::new(0., inputs.unit.clone()).unwrap(),
                        ),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("sum"),
                    expression: RuleExpression::Read { input: key("sum") },
                }],
                effects: vec![RuleEffect {
                    id: key("sum"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: row.contribution_stat.clone(),
                        value: key("sum"),
                    },
                }],
            });
        }
        assert_eq!(&owner.programs.members[..4], authored.programs.members);
        base.owners.push(owner);
        base.scenario.enemy.encounter = inputs.encounter.clone();
        base.scenario.enemy.level = 82;
        let scenario = base.scenario.clone();
        let mut result = Self {
            inputs,
            scenario,
            base,
            authored,
        };
        for index in 0..4 {
            result.set(index, Some(false), None);
        }
        result
    }

    pub fn quantity(&self, value: f64) -> ParameterValue {
        ParameterValue::Quantity(FiniteQuantity::new(value, self.inputs.unit.clone()).unwrap())
    }

    pub fn set(&mut self, index: usize, present: Option<bool>, raw: Option<f64>) {
        let row = &self.inputs.inputs[index];
        self.scenario
            .assumptions
            .retain(|value| value.input != row.presence_input && value.input != row.value_input);
        if let Some(present) = present {
            self.scenario.assumptions.push(ExternalAssumption {
                input: row.presence_input.clone(),
                target: AssumptionTarget::Enemy,
                value: ParameterValue::Boolean(present),
            });
        }
        if let Some(raw) = raw {
            self.scenario.assumptions.push(ExternalAssumption {
                input: row.value_input.clone(),
                target: AssumptionTarget::Enemy,
                value: self.quantity(raw),
            });
        }
    }

    pub fn plan(
        &self,
    ) -> std::result::Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>, String> {
        assert_eq!(
            &self.base.owners.last().unwrap().programs.members[..4],
            self.authored.programs.members
        );
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(
                self.base.schema.clone(),
                OwnedSchemaLimits::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let rules = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: self.inputs.namespace.clone(),
            release: key("configuration-component"),
            semantics_version: self.base.schema.semantics_version.clone(),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: self.base.owners.clone(),
            receivers: self.base.receivers.clone(),
        };
        let rules = Arc::new(
            CompiledRulePackage::compile(&rules, schema.as_ref(), RuleLimits::default())
                .map_err(|e| e.to_string())?,
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: self.inputs.namespace.clone(),
                    release: key("component-routing"),
                    definitions: schema.identity().clone(),
                    outputs: vec![],
                },
                schema.as_ref(),
                RoutingLimits::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let limits = OwnedInputLimits::default();
        let request = OwnedEvaluationRequest::new(
            BuildSpec::new(self.base.build.clone(), limits).map_err(|e| e.to_string())?,
            ScenarioSpec::new(self.scenario.clone(), limits).map_err(|e| e.to_string())?,
            QuerySpec::new(self.base.queries.clone(), limits).map_err(|e| e.to_string())?,
            limits,
        )
        .map_err(|e| e.to_string())?;
        OwnedEffectPlan::compile(
            Arc::new(request),
            schema,
            rules,
            routing,
            PlanLimits::default(),
        )
        .map_err(|e| e.to_string())
    }

    pub fn evaluate(&self) -> OwnedEffectsReport {
        let plan = self.plan().unwrap();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }

    pub fn contribution<'a>(
        &self,
        report: &'a OwnedEffectsReport,
        index: usize,
    ) -> &'a EffectValue {
        let row = &self.inputs.inputs[index];
        let effects: Vec<_> = report
            .effects
            .iter()
            .filter(|effect| effect.key.invocation.program == row.program)
            .collect();
        assert_eq!(effects.len(), 1);
        let effect = effects[0];
        assert_eq!(effect.key.invocation.origin, RuleOrigin::Encounter);
        assert_eq!(effect.key.invocation.entity, ConcreteEntity::Enemy);
        assert_eq!(
            effect.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Enemy,
                    stat: row.contribution_stat.clone(),
                    kind: ContributionKind::Add,
                }
            }
        );
        &effect.value
    }

    pub fn sum<'a>(&self, report: &'a OwnedEffectsReport, index: usize) -> &'a EffectValue {
        let key = PlanValueKey::Stat {
            entity: ConcreteEntity::Enemy,
            stat: self.inputs.inputs[index].contribution_stat.clone(),
        };
        let values: Vec<_> = report.values.iter().filter(|row| row.key == key).collect();
        assert_eq!(values.len(), 1);
        &values[0].value
    }

    pub fn assert_known(&self, report: &OwnedEffectsReport, index: usize, value: f64) {
        let expected = EffectValue::Known {
            value: self.quantity(value),
        };
        assert_eq!(self.contribution(report, index), &expected);
        assert_eq!(self.sum(report, index), &expected);
    }
}
