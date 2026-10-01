#![allow(dead_code)]
//! Native recipe components; complete source/build coverage is deliberately absent.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::owned_effective_gem_recipe::*;

pub fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("recipe-fixture", "v1").unwrap()
}
pub fn id<K: DefinitionDomain>(v: &str) -> DefId<K> {
    DefId::parse(ns(), v).unwrap()
}
pub fn int(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
pub fn qty(v: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, unit.clone()).unwrap())
}
fn range(a: f64, b: f64, unit: &UnitDefId) -> QuantityRange {
    QuantityRange {
        minimum: FiniteQuantity::new(a, unit.clone()).unwrap(),
        maximum: FiniteQuantity::new(b, unit.clone()).unwrap(),
    }
}
fn slots() -> DeclaredSlots {
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
pub fn gap(gem: &GemDefId) -> SchemaGap {
    SchemaGap {
        subject: SchemaSubject::Definition(gem.address()),
        facet: SchemaFacet::InputSchema,
        code: key("other-inputs-unconverted"),
    }
}
pub struct Fixture {
    pub schema: OwnedDefinitionSchemaPackage,
    pub input: EffectiveGemRecipeInput,
    pub count: UnitDefId,
    pub percent: UnitDefId,
}
pub struct Values {
    pub raw_level: i64,
    pub corruption: f64,
    pub external_level: Option<f64>,
    pub external_quality: Option<f64>,
    pub quality: Option<f64>,
}
impl Fixture {
    pub fn new() -> Self {
        let count: UnitDefId = id("count");
        let percent: UnitDefId = id("percent");
        let quality: QualityDefId = id("ordinary-quality");
        let active: GemDefId = id("active");
        let support: GemDefId = id("support");
        let corruption = DeclaredSlot {
            declaration: SlotOwnerDefId::Gem(active.clone()),
            slot: id::<ParameterSlotDefinition>("corruption"),
        };
        let mut definitions = vec![
            DefinitionDescriptor::Unit(DefinitionEntry {
                id: count.clone(),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::Count,
                }),
            }),
            DefinitionDescriptor::Unit(DefinitionEntry {
                id: percent.clone(),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                }),
            }),
            DefinitionDescriptor::Quality(DefinitionEntry {
                id: quality.clone(),
                schema: SchemaState::Known(QualitySchema {
                    amount: range(0.0, 1000.0, &percent),
                }),
            }),
        ];
        for (gem, role) in [
            (&active, AuthoredGemRole::SkillUse),
            (&support, AuthoredGemRole::SupportAssignment),
        ] {
            let mut declarations = slots();
            declarations.parameters = DeclaredSet::partial(
                if gem == &active {
                    vec![corruption.clone()]
                } else {
                    vec![]
                },
                vec![gap(gem)],
            );
            definitions.push(DefinitionDescriptor::Gem(DefinitionEntry {
                id: gem.clone(),
                schema: SchemaState::Known(GemSchema {
                    level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(65535).unwrap(),
                    },
                    roles: vec![role],
                    skills: DeclaredSet::complete(vec![]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Optional,
                        allowed_kinds: DeclaredSet::complete(vec![quality.clone()]),
                    },
                    declarations,
                }),
            }));
        }
        let level_output: StatDefId = id("active-level-output");
        let quality_output: StatDefId = id("quality-output");
        for (name, value, targets) in [
            (
                "active-level-output",
                ComputedValueType::Quantity {
                    unit: count.clone(),
                },
                vec![RuleEntityKind::Skill],
            ),
            (
                "support-level-output",
                ComputedValueType::Integer,
                vec![RuleEntityKind::SupportOrigin],
            ),
            (
                "quality-output",
                ComputedValueType::Quantity {
                    unit: percent.clone(),
                },
                vec![RuleEntityKind::Skill, RuleEntityKind::SupportOrigin],
            ),
            (
                "external-level",
                ComputedValueType::Quantity {
                    unit: count.clone(),
                },
                vec![RuleEntityKind::Actor],
            ),
            (
                "external-quality",
                ComputedValueType::Quantity {
                    unit: percent.clone(),
                },
                vec![RuleEntityKind::Actor],
            ),
        ] {
            definitions.push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: id(name),
                schema: SchemaState::Known(StatSchema { value, targets }),
            }));
        }
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: 4,
                namespace: ns(),
                release: key("schema"),
                semantics_version: key("semantics"),
                definitions,
                slots: vec![SlotDescriptor::Parameter(DefinitionEntry {
                    id: corruption.clone(),
                    schema: SchemaState::Known(ParameterSlotSchema {
                        value: ValueSchema::Quantity(range(-1000.0, 1000.0, &count)),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::GemParameter],
                    }),
                })],
            },
            Default::default(),
        )
        .unwrap();
        let bindings = [
            (
                active,
                EffectiveGemRecipeRole::ActivePreSupport { corruption },
            ),
            (
                support,
                EffectiveGemRecipeRole::SupportPreparation {
                    levels: DenseGemLevelPolicy {
                        maximum: 40,
                        natural_maximum: 20,
                    },
                },
            ),
        ]
        .into_iter()
        .map(|(gem, role)| EffectiveGemRecipeBinding {
            level_output: if matches!(role, EffectiveGemRecipeRole::ActivePreSupport { .. }) {
                level_output.clone()
            } else {
                id("support-level-output")
            },
            gem,
            program: key("effective-inputs"),
            role,
            quality: quality.clone(),
            quality_absence: QualityAbsencePolicy::ZeroWhenProvenSingleton,
            level_unit: count.clone(),
            external_level: vec![id("external-level")],
            external_quality: vec![id("external-quality")],
            quality_output: quality_output.clone(),
        })
        .collect();
        let input = EffectiveGemRecipeInput {
            schema_version: 1,
            version: key("recipe"),
            definitions: schema.identity().clone(),
            bindings,
        };
        Self {
            schema,
            input,
            count,
            percent,
        }
    }
    pub fn compile(&self) -> EffectiveGemRecipeOutput {
        compile_effective_gem_recipe(&self.input, &self.schema, Default::default()).unwrap()
    }
    pub fn executable(&self, output: &EffectiveGemRecipeOutput) -> CompiledRulePackage {
        CompiledRulePackage::compile(
            &RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("component"),
                semantics_version: key("component"),
                operations_version: key(OWNED_RULE_OPERATIONS_V13),
                definitions: self.schema.identity().clone(),
                tables: vec![],
                owners: output
                    .programs
                    .iter()
                    .map(|row| {
                        let owner = SchemaSubject::Definition(row.gem.address());
                        DefinitionRules {
                            owner: owner.clone(),
                            programs: DeclaredSet::partial(
                                vec![row.program.clone()],
                                vec![SchemaGap {
                                    subject: owner,
                                    facet: SchemaFacet::GameRules,
                                    code: key("unconverted-game-rules"),
                                }],
                            ),
                        }
                    })
                    .collect(),
                receivers: DeclaredSet::complete(vec![]),
            },
            &self.schema,
            Default::default(),
        )
        .unwrap()
    }
    pub fn evaluate(
        &self,
        row: usize,
        raw: i64,
        corruption: f64,
        external: Option<f64>,
        quality: Option<f64>,
    ) -> Vec<EffectDisposition> {
        self.evaluate_values(
            row,
            Values {
                raw_level: raw,
                corruption,
                external_level: external,
                external_quality: Some(3.0),
                quality,
            },
        )
    }
    pub fn evaluate_values(&self, row: usize, values: Values) -> Vec<EffectDisposition> {
        let output = self.compile();
        let compiled = self.executable(&output);
        let row = &output.programs[row];
        let facts = row
            .program
            .reads
            .iter()
            .filter_map(|read| {
                let value = match &read.source {
                    RuleReadSource::GemLevel => Some(int(values.raw_level)),
                    RuleReadSource::Parameter { .. } => Some(qty(values.corruption, &self.count)),
                    RuleReadSource::HasGemQuality { .. } => {
                        Some(ParameterValue::Boolean(values.quality.is_some()))
                    }
                    RuleReadSource::GemQualityAmount { .. } => {
                        values.quality.map(|v| qty(v, &self.percent))
                    }
                    RuleReadSource::Contributions { stat, .. }
                        if stat == &id::<StatDefinition>("external-level") =>
                    {
                        values.external_level.map(|v| qty(v, &self.count))
                    }
                    RuleReadSource::Contributions { .. } => {
                        values.external_quality.map(|v| qty(v, &self.percent))
                    }
                    _ => panic!("unexpected generated read"),
                }?;
                Some(RuleFact {
                    read: read.id.clone(),
                    value,
                })
            })
            .collect::<Vec<_>>();
        let report = compiled
            .evaluate(
                &SchemaSubject::Definition(row.gem.address()),
                &row.program.id,
                &facts,
                &self.schema,
                &mut compiled.new_scratch(),
            )
            .unwrap();
        let mut effects = report.effects;
        effects.sort_by(|a, b| a.id.cmp(&b.id));
        effects.into_iter().map(|e| e.disposition).collect()
    }
    pub fn edit_schema(&mut self, f: impl FnOnce(&mut SchemaPackageInput)) {
        let mut schema = self.schema.input().clone();
        f(&mut schema);
        self.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
        self.input.definitions = self.schema.identity().clone();
    }
}
