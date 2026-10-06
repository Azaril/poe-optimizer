//! Finite component world executing the authored incoming-hit programs unchanged.
//! Closing this fixture's incoming membership does not close the real Encounter.
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
use std::{fs, path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub encounter: EncounterDefId,
    pub damage_unit: UnitDefId,
    pub percent_unit: UnitDefId,
    pub factor_unit: UnitDefId,
    pub category_input: ExternalInputDefId,
    pub categories: Vec<Category>,
    pub inputs: Vec<Input>,
    pub damage: Vec<Damage>,
    pub total_stat: StatDefId,
    pub penetration: Vec<Penetration>,
    pub program: OwnedDefinitionKey,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub name: String,
    pub option: OptionDefId,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub source_name: String,
    pub presence_input: ExternalInputDefId,
    pub value_input: ExternalInputDefId,
    pub unit: UnitDefId,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Damage {
    pub name: String,
    pub minimum_stat: StatDefId,
    pub maximum_stat: StatDefId,
    pub output_stat: StatDefId,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Penetration {
    pub source_name: String,
    pub output_stat: StatDefId,
}

/// Measured inputs and outputs for this bounded stage only. Query sums are
/// explicit finite component inputs, not a claim about original owner closure.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeasuredCase {
    pub name: String,
    pub xml_sha256: String,
    pub mode: String,
    pub enemy_level: u16,
    pub category: String,
    pub inputs: [Option<f64>; 8],
    pub minimum: Option<[f64; 5]>,
    pub maximum: Option<[f64; 5]>,
    pub damage: [f64; 5],
    pub total: f64,
    pub penetration: [f64; 3],
}

pub fn measured_cases(vectors: &Value) -> Vec<MeasuredCase> {
    typed(&vectors["cases"])
}

pub fn replay(case: &MeasuredCase) {
    assert!(matches!(case.mode.as_str(), "MAIN" | "CALCS"));
    let mut fixture = Fixture::new(true);
    fixture.scenario.enemy.level = case.enemy_level;
    fixture.category(Some(&case.category));
    for (index, raw) in case.inputs.iter().copied().enumerate() {
        fixture.set(index, Some(raw.is_some()), raw);
    }
    if case.category == "DamageOverTime" {
        assert!(case.minimum.is_none() && case.maximum.is_none());
    } else {
        let minimum = case.minimum.expect("measured hit minimum queries");
        let maximum = case.maximum.expect("measured hit maximum queries");
        for index in 0..5 {
            fixture.contribute(
                &format!("measured-contributor-{index}"),
                index,
                minimum[index],
                maximum[index],
            );
        }
    }
    let report = fixture.evaluate();
    fixture.assert_damage(&report, case.damage, case.total);
    fixture.assert_penetration(&report, case.penetration);
}

/// Copy observed operands/results only; no source arithmetic is recreated here.
pub fn project_source_case(case: &Value, mode: &str, inputs: &Inputs) -> MeasuredCase {
    let view = &case["state"][mode];
    let category = view["category"].as_str().unwrap();
    let damage_over_time = category == "DamageOverTime";
    let rows: &[Value] = match &view["rows"] {
        Value::Array(rows) => rows,
        Value::Object(rows) if damage_over_time && rows.is_empty() => &[],
        _ => panic!("expected ordered hit rows or an empty DoT table"),
    };
    assert_eq!(rows.len(), if damage_over_time { 0 } else { 5 });
    for (row, descriptor) in rows.iter().zip(&inputs.damage) {
        assert_eq!(row["damage_type"], descriptor.name);
        for bound in ["minimum", "maximum"] {
            assert_eq!(row[bound]["operation"], "BASE");
            assert_eq!(
                row[bound]["cfg"]["keywordFlags"],
                case["state"]["enemy_keyword_flags"]
            );
        }
    }
    MeasuredCase {
        name: case["name"].as_str().unwrap().to_owned(),
        xml_sha256: case["xml_sha256"].as_str().unwrap().to_owned(),
        mode: view["mode"].as_str().unwrap().to_owned(),
        enemy_level: u16::try_from(view["enemy_level"].as_u64().unwrap()).unwrap(),
        category: category.to_owned(),
        inputs: std::array::from_fn(|index| {
            let value = &view["input"][&inputs.inputs[index].source_name];
            (!value.is_null()).then(|| value.as_f64().unwrap())
        }),
        minimum: (!damage_over_time).then(|| {
            std::array::from_fn(|index| rows[index]["minimum"]["result"].as_f64().unwrap())
        }),
        maximum: (!damage_over_time).then(|| {
            std::array::from_fn(|index| rows[index]["maximum"]["result"].as_f64().unwrap())
        }),
        damage: if damage_over_time {
            let measured = std::array::from_fn(|index| {
                view["player"][format!("{}EnemyDamage", inputs.damage[index].name)]
                    .as_f64()
                    .unwrap()
            });
            assert_eq!(measured, [0.; 5]);
            measured
        } else {
            std::array::from_fn(|index| rows[index]["preconversion_amount"].as_f64().unwrap())
        },
        total: view["total_in"].as_f64().unwrap(),
        penetration: std::array::from_fn(|index| {
            let damage_type = inputs.penetration[index]
                .source_name
                .strip_prefix("enemy")
                .unwrap()
                .strip_suffix("Pen")
                .unwrap();
            view["player"][format!("{damage_type}EnemyPen")]
                .as_f64()
                .unwrap()
        }),
    }
}

pub fn asset(family: &str, name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68")
        .join(family)
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
pub fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
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
    visit(&mut value, &json!(namespace));
    typed(&value)
}

pub struct Fixture {
    pub inputs: Inputs,
    pub scenario: ScenarioInput,
    pub tables: Vec<IntegerRuleTable>,
    base: base::Fixture,
    authored: DefinitionRules,
    operations: OwnedDefinitionKey,
}

impl Fixture {
    pub fn new(finite: bool) -> Self {
        let inputs: Inputs = typed(&asset("incoming-damage-inputs", "native-inputs.json"));
        let extension = asset("incoming-damage-inputs", "extension.json");
        assert_eq!(inputs.schema_version, 1);
        assert_eq!(extension["schema_version"], 1);
        assert_eq!(inputs.inputs.len(), 8);
        assert_eq!(inputs.damage.len(), 5);
        assert_eq!(inputs.penetration.len(), 3);
        assert_eq!(inputs.categories.len(), 7);
        assert_eq!(
            inputs
                .damage
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>(),
            ["Physical", "Lightning", "Cold", "Fire", "Chaos"]
        );
        let owners: Vec<DefinitionRules> = typed(&extension["owners"]);
        assert_eq!(owners.len(), 1);
        let authored = owners[0].clone();
        assert_eq!(
            authored.owner,
            SchemaSubject::Definition(inputs.encounter.address())
        );
        assert!(!authored.programs.is_complete());
        assert_eq!(authored.programs.members.len(), 1);
        assert_eq!(authored.programs.members[0].id, inputs.program);
        assert!(extension["receivers"].as_array().unwrap().is_empty());

        let mut base = base::Fixture::new();
        base.schema = rebase(&base.schema, &inputs.namespace);
        base.build = rebase(&base.build, &inputs.namespace);
        base.scenario = rebase(&base.scenario, &inputs.namespace);
        base.queries = rebase(&base.queries, &inputs.namespace);
        base.owners = rebase(&base.owners, &inputs.namespace);
        base.schema.schema_version = 4;
        base.schema
            .definitions
            .retain(|d| !matches!(d, DefinitionDescriptor::Encounter(_)));
        // The real Encounter preserves these prior external-input declarations.
        // Include their exact descriptors; this finite world selects only the
        // incoming-hit program, not their unrelated numerical producers.
        for family in [
            "configuration-resistance-inputs",
            "configuration-rating-inputs",
            "configuration-block-inputs",
            "incoming-damage-inputs",
        ] {
            for row in asset(family, "extension.json")["schema"]
                .as_array()
                .unwrap()
            {
                assert_eq!(row["kind"], "definition");
                let definition: DefinitionDescriptor = typed(&row["value"]);
                base.schema
                    .definitions
                    .retain(|d| d.address() != definition.address());
                base.schema.definitions.push(definition);
            }
        }
        let rating = DefId::parse(inputs.namespace.clone(), "def.00000000000029ee").unwrap();
        for (id, dimension) in [
            (inputs.damage_unit.clone(), UnitDimension::Damage),
            (inputs.percent_unit.clone(), UnitDimension::PercentagePoints),
            (
                inputs.factor_unit.clone(),
                UnitDimension::DimensionlessFactor,
            ),
            (rating, UnitDimension::Rating),
        ] {
            base.schema
                .definitions
                .push(DefinitionDescriptor::Unit(DefinitionEntry {
                    id,
                    schema: SchemaState::Known(UnitSchema { dimension }),
                }));
        }
        base.owners.retain(|owner| {
            !matches!(
                owner.owner,
                SchemaSubject::Definition(DefinitionAddress::Encounter(_))
            )
        });
        let mut owner = authored.clone();
        if finite {
            // Explicit finite-test membership only; no production descriptor or
            // authored program body is promoted to complete.
            owner.programs.closure = SchemaClosure::Complete;
        }
        base.owners.push(owner);
        base.scenario.enemy.encounter = inputs.encounter.clone();
        base.scenario.enemy.level = 82;
        let scenario = base.scenario.clone();
        let mut result = Self {
            inputs,
            scenario,
            tables: typed(&extension["tables"]),
            base,
            authored,
            operations: key(extension["operations_version"].as_str().unwrap()),
        };
        for index in 0..8 {
            result.set(index, Some(false), None);
        }
        result.category(Some("Average"));
        result
    }

    pub fn quantity(&self, value: f64, unit: &UnitDefId) -> ParameterValue {
        ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
    }

    pub fn set(&mut self, index: usize, present: Option<bool>, raw: Option<f64>) {
        let input = &self.inputs.inputs[index];
        self.scenario
            .assumptions
            .retain(|a| a.input != input.presence_input && a.input != input.value_input);
        if let Some(value) = present {
            self.scenario.assumptions.push(ExternalAssumption {
                input: input.presence_input.clone(),
                target: AssumptionTarget::Enemy,
                value: ParameterValue::Boolean(value),
            });
        }
        if let Some(value) = raw {
            self.scenario.assumptions.push(ExternalAssumption {
                input: input.value_input.clone(),
                target: AssumptionTarget::Enemy,
                value: self.quantity(value, &input.unit),
            });
        }
    }

    pub fn category(&mut self, name: Option<&str>) {
        self.scenario
            .assumptions
            .retain(|a| a.input != self.inputs.category_input);
        if let Some(name) = name {
            let category = self
                .inputs
                .categories
                .iter()
                .find(|c| c.name == name)
                .unwrap();
            self.scenario.assumptions.push(ExternalAssumption {
                input: self.inputs.category_input.clone(),
                target: AssumptionTarget::Enemy,
                value: ParameterValue::Option(category.option.clone()),
            });
        }
    }

    pub fn contribute(&mut self, name: &str, index: usize, minimum: f64, maximum: f64) {
        let damage = &self.inputs.damage[index];
        let nodes = [("min", minimum), ("max", maximum)]
            .into_iter()
            .map(|(id, value)| RuleNode {
                id: key(id),
                expression: RuleExpression::Literal {
                    value: self.quantity(value, &self.inputs.damage_unit),
                },
            })
            .collect();
        let effects = [("min", &damage.minimum_stat), ("max", &damage.maximum_stat)]
            .into_iter()
            .map(|(id, stat)| RuleEffect {
                id: key(id),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: stat.clone(),
                    contribution: ContributionKind::Add,
                    value: key(id),
                },
            })
            .collect();
        self.base
            .owners
            .last_mut()
            .unwrap()
            .programs
            .members
            .push(RuleProgram {
                id: key(name),
                context: RuleEntityKind::Enemy,
                reads: vec![],
                nodes,
                effects,
            });
    }

    pub fn plan(
        &self,
    ) -> std::result::Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>, String> {
        assert_eq!(
            &self.base.owners.last().unwrap().programs.members[..1],
            self.authored.programs.members
        );
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(
                self.base.schema.clone(),
                OwnedSchemaLimits::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let input = RulePackageInput {
            ordered_contributions: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: self.inputs.namespace.clone(),
            release: key("incoming-hit-component"),
            semantics_version: self.base.schema.semantics_version.clone(),
            operations_version: self.operations.clone(),
            definitions: schema.identity().clone(),
            tables: self.tables.clone(),
            owners: self.base.owners.clone(),
            receivers: self.base.receivers.clone(),
            effect_applications: (self.operations.as_str() == OWNED_RULE_OPERATIONS_V15)
                .then(|| DeclaredSet::complete(vec![])),
        };
        let rules = Arc::new(
            CompiledRulePackage::compile(&input, schema.as_ref(), RuleLimits::default())
                .map_err(|e| e.to_string())?,
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: self.inputs.namespace.clone(),
                    release: key("incoming-hit-routing"),
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
    pub fn value<'a>(&self, report: &'a OwnedEffectsReport, stat: &StatDefId) -> &'a EffectValue {
        let key = PlanValueKey::Stat {
            entity: ConcreteEntity::Enemy,
            stat: stat.clone(),
        };
        let rows: Vec<_> = report.values.iter().filter(|row| row.key == key).collect();
        assert_eq!(rows.len(), 1, "expected one exact Enemy stat: {stat:?}");
        &rows[0].value
    }
    pub fn assert_stat(
        &self,
        report: &OwnedEffectsReport,
        stat: &StatDefId,
        unit: &UnitDefId,
        expected: f64,
    ) {
        assert_eq!(
            self.value(report, stat),
            &EffectValue::Known {
                value: self.quantity(expected, unit)
            }
        );
    }
    pub fn assert_damage(&self, report: &OwnedEffectsReport, expected: [f64; 5], total: f64) {
        for (damage, value) in self.inputs.damage.iter().zip(expected) {
            self.assert_stat(report, &damage.output_stat, &self.inputs.damage_unit, value);
        }
        self.assert_stat(
            report,
            &self.inputs.total_stat,
            &self.inputs.damage_unit,
            total,
        );
    }
    pub fn assert_penetration(&self, report: &OwnedEffectsReport, expected: [f64; 3]) {
        for (row, value) in self.inputs.penetration.iter().zip(expected) {
            self.assert_stat(report, &row.output_stat, &self.inputs.percent_unit, value);
        }
    }
}
