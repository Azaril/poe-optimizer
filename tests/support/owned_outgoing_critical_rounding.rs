//! Executes the six authored decimal-rounding fragments in a finite rule graph.
//! This does not admit the pending Encounter consumer or supply any game input.
use super::family;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits,
};
use serde_json::{Value, json};

pub const SITES: [&str; 6] = [
    "chance-rounded-more-current",
    "chance-rounded-more-actor",
    "bonus-rounded-more-current",
    "bonus-rounded-more-actor",
    "rounded-chance",
    "rounded-bonus",
];
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn stat() -> StatDefId {
    DefId::parse(namespace(), "test.critical-rounding").unwrap()
}
fn value(v: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, unit.clone()).unwrap())
}
fn node(id: String, expression: Value) -> Value {
    json!({"id":id,"expression":expression})
}

/// Fixed semantic shape, not a second evaluation of the full critical formula.
pub fn review_nodes(consumer: &Value) -> Result<(), String> {
    let nodes = consumer["owners"][0]["programs"]["members"][0]["nodes"]
        .as_array()
        .ok_or("nodes")?;
    for site in SITES {
        let unit = if site == "rounded-chance" { 2 } else { 1 };
        let unit = json!({"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":format!("def.{unit:016x}")});
        let quantity = |n| json!({"value":n,"unit":unit});
        let half = if site == "rounded-chance" {
            "round-half-percent"
        } else {
            "round-half-factor"
        };
        let expected = [
            node(
                format!("{site}-below-half"),
                json!({"kind":"compare","operation":"less","left":format!("{site}-scaled"),"right":half}),
            ),
            node(
                format!("{site}-lower-domain"),
                json!({"kind":"check_numeric_domain","value":format!("{site}-scaled"),"domain":{"kind":"quantity","value":{"minimum":quantity(-4503599627370496.0),"maximum":quantity(0.4999999999999999)}}}),
            ),
            node(
                format!("{site}-upper-domain"),
                json!({"kind":"check_numeric_domain","value":format!("{site}-scaled"),"domain":{"kind":"quantity","value":{"minimum":quantity(0.5),"maximum":quantity(4503599627370496.0)}}}),
            ),
            node(
                format!("{site}-bounded"),
                json!({"kind":"select","condition":format!("{site}-below-half"),"when_true":format!("{site}-lower-domain"),"when_false":format!("{site}-upper-domain")}),
            ),
            node(
                format!("{site}-integer"),
                json!({"kind":"round","value":format!("{site}-bounded"),"quantum":quantity(1.0),"mode":"nearest_ties_positive"}),
            ),
            node(
                site.to_owned(),
                json!({"kind":"divide_factor","value":format!("{site}-integer"),"divisor":"round-hundred"}),
            ),
        ];
        for expected in expected {
            let matches: Vec<_> = nodes.iter().filter(|n| n["id"] == expected["id"]).collect();
            if matches.len() != 1 {
                return Err("rounding node multiplicity".into());
            }
            let actual: RuleNode =
                serde_json::from_value(matches[0].clone()).map_err(|e| e.to_string())?;
            let expected: RuleNode = serde_json::from_value(expected).map_err(|e| e.to_string())?;
            if actual != expected {
                return Err("semantic rounding/domain shape".into());
            }
        }
    }
    if nodes
        .iter()
        .any(|n| n["id"].as_str().is_some_and(|s| s.ends_with("-half-added")))
    {
        return Err("source add-half operation is not native rounding".into());
    }
    Ok(())
}

pub struct Fragment {
    compiled: CompiledRulePackage,
    schema: OwnedDefinitionSchemaPackage,
    owner: SchemaSubject,
    unit: UnitDefId,
}
pub fn fragment(site: &str, bypass: bool) -> Fragment {
    assert!(SITES.contains(&site));
    let c = family::read("consumer.json");
    review_nodes(&c).unwrap();
    let source = c["owners"][0]["programs"]["members"][0]["nodes"]
        .as_array()
        .unwrap();
    let unit_number = if site == "rounded-chance" { 2 } else { 1 };
    let unit: UnitDefId = DefId::parse(namespace(), format!("def.{unit_number:016x}")).unwrap();
    let mut descriptors: Vec<DefinitionDescriptor> = (1..=2).map(|n| serde_json::from_value(json!({"kind":"unit","value":{"id":{"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":format!("def.{n:016x}")},"schema":{"kind":"known","value":{"dimension":if n==1 {"dimensionless_factor"} else {"percentage_points"}}}}})).unwrap()).collect();
    descriptors.push(DefinitionDescriptor::Stat(DefinitionEntry {
        id: stat(),
        schema: SchemaState::Known(StatSchema {
            value: ComputedValueType::Quantity { unit: unit.clone() },
            targets: vec![RuleEntityKind::Actor],
        }),
    }));
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("critical-rounding-fragment"),
            semantics_version: key("input-and-computed-schema-v1"),
            definitions: descriptors,
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    let mut nodes: Vec<RuleNode> = source
        .iter()
        .filter(|n| {
            let id = n["id"].as_str().unwrap();
            id == site
                || id.starts_with(&format!("{site}-"))
                || ["round-half-factor", "round-half-percent", "round-hundred"].contains(&id)
        })
        .map(|n| serde_json::from_value(n.clone()).unwrap())
        .collect();
    let input = nodes
        .iter_mut()
        .find(|n| n.id.as_str() == format!("{site}-scaled"))
        .unwrap();
    // Only the scaled operand is injected; every guard/round/division node is
    // the exact authored consumer node. No game query is made Complete here.
    input.expression = RuleExpression::Read {
        input: key("operand"),
    };
    nodes.push(RuleNode {
        id: key("bypass"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(bypass),
        },
    });
    nodes.push(RuleNode {
        id: key("bypass-value"),
        expression: RuleExpression::Literal {
            value: value(7.0, &unit),
        },
    });
    nodes.push(RuleNode {
        id: key("result"),
        expression: RuleExpression::Select {
            condition: key("bypass"),
            when_true: key("bypass-value"),
            when_false: key(site),
        },
    });
    let owner = SchemaSubject::Definition(stat().address());
    let program = RuleProgram {
        id: key("rounding"),
        context: RuleEntityKind::Actor,
        reads: vec![RuleRead {
            id: key("operand"),
            value_type: ComputedValueType::Quantity { unit: unit.clone() },
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: stat(),
            },
        }],
        nodes,
        effects: vec![RuleEffect {
            id: key("result"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: stat(),
                value: key("result"),
            },
        }],
    };
    let rules = RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: namespace(),
        release: key("critical-rounding-fragment"),
        semantics_version: key("checked-decimal-rounding"),
        operations_version: key(OWNED_RULE_OPERATIONS_V27),
        definitions: schema.identity().clone(),
        tables: vec![],
        owners: vec![DefinitionRules {
            owner: owner.clone(),
            programs: DeclaredSet::complete(vec![program]),
        }],
        receivers: DeclaredSet::complete(vec![]),
        support_discovery: None,
        existing_actor_rules: None,
        contribution_queries: Some(DeclaredSet::complete(vec![])),
        effect_applications: Some(DeclaredSet::complete(vec![])),
    };
    let compiled = CompiledRulePackage::compile(&rules, &schema, RuleLimits::default()).unwrap();
    Fragment {
        compiled,
        schema,
        owner,
        unit,
    }
}
impl Fragment {
    pub fn evaluate(&self, operands: &[Option<f64>]) -> Vec<EffectDisposition> {
        let mut scratch = self.compiled.new_scratch();
        operands
            .iter()
            .map(|v| {
                let facts: Vec<_> = v
                    .iter()
                    .map(|v| RuleFact {
                        read: key("operand"),
                        value: value(*v, &self.unit),
                    })
                    .collect();
                let report = self
                    .compiled
                    .evaluate(
                        &self.owner,
                        &key("rounding"),
                        &facts,
                        &self.schema,
                        &mut scratch,
                    )
                    .unwrap();
                assert_eq!(report.effects.len(), 1);
                report.effects[0].disposition.clone()
            })
            .collect()
    }
}
