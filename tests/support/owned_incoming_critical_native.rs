//! Finite component world executing the authored incoming-critical programs.
//! Fixture literals stand for resolved dependencies; only this test world closes
//! source inventories. No literal here enters the release or import path.
#[allow(dead_code)]
#[path = "owned_empty_support_domain.rs"]
mod empty_support;

use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_mapping::RegistryInput,
    owned_recipe::OwnedRecipeInput,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
};
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};

pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68/incoming-critical-effect")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
pub fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([0x6b; 16]), n).unwrap())
}
fn integer(n: i64) -> BoundedInteger {
    BoundedInteger::new(n).unwrap()
}
fn quantity(n: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn ports() -> DeclaredSlots {
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
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}

/// Explicit finite resolved inputs, in the same order as native-inputs.json.
#[derive(Clone, Debug)]
pub struct Case {
    pub category: u64,
    pub raw: [Option<f64>; 2],
    pub presence: [Option<bool>; 2],
    pub flags: [Option<bool>; 3],
    // Actor chance INC; Enemy chance INC; configured evade;
    // Enemy critical bonus BASE; Enemy Hit critical bonus INC; reduction.
    pub dependencies: [Option<f64>; 6],
}
impl Default for Case {
    fn default() -> Self {
        Self {
            category: 0x3252,
            raw: [None; 2],
            presence: [Some(false); 2],
            flags: [Some(false); 3],
            dependencies: [Some(0.); 6],
        }
    }
}
impl Case {
    pub fn raw(mut self, chance: Option<f64>, damage: Option<f64>) -> Self {
        self.raw = [chance, damage];
        self.presence = [Some(chance.is_some()), Some(damage.is_some())];
        self
    }
}

pub struct World {
    pub recipe: OwnedRecipeInput,
    pub build: BuildInput,
    pub scenario: ScenarioInput,
}
impl World {
    pub fn new(case: &Case, close_finite_domain: bool) -> Self {
        let extension: OwnedRecipeExtension = read("extension.json");
        let mut definitions: Vec<_> = extension
            .schema
            .into_iter()
            .map(|entry| match entry {
                SchemaExtensionEntry::Definition(d) => d,
                SchemaExtensionEntry::Slot(_) => panic!("no critical calculation slots"),
            })
            .collect();
        let encounter = definitions
            .iter_mut()
            .find_map(|d| match d {
                DefinitionDescriptor::Encounter(e) => Some(e),
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(encounter) = &mut encounter.schema else {
            panic!()
        };
        // The finite world includes only the new raw ports and the shared category.
        encounter.external_inputs =
            DeclaredSet::complete([0x3251, 0x3353, 0x3354, 0x3355, 0x3356].map(def).to_vec());
        definitions.extend([
            DefinitionDescriptor::Class(known(
                def(0xf0101),
                ClassSchema {
                    level: IntegerRange {
                        minimum: integer(1),
                        maximum: integer(100),
                    },
                    ascendancies: DeclaredSet::complete(vec![]),
                    implicit_passives: DeclaredSet::complete(vec![]),
                    declarations: ports(),
                },
            )),
            DefinitionDescriptor::Actor(known(
                def(0x332a),
                ActorSchema {
                    declarations: ports(),
                },
            )),
            DefinitionDescriptor::Unit(known(
                def(1),
                UnitSchema {
                    dimension: UnitDimension::DimensionlessFactor,
                },
            )),
            DefinitionDescriptor::Unit(known(
                def(2),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
            DefinitionDescriptor::ExternalInput(known(
                def(0x3251),
                ExternalInputSchema {
                    value: ValueSchema::Option {
                        allowed: DeclaredSet::complete((0x3252..=0x3258).map(def).collect()),
                    },
                    targets: vec![AssumptionTargetKind::Enemy],
                },
            )),
        ]);
        for n in 0x3252..=0x3258 {
            definitions.push(DefinitionDescriptor::Option(known(def(n), OptionSchema {})));
        }
        let schema = SchemaPackageInput {
            schema_version: 6,
            namespace: ns(),
            release: key("finite-incoming-critical"),
            semantics_version: key("finite-incoming-critical"),
            definitions,
            slots: vec![],
        };
        let checked =
            OwnedDefinitionSchemaPackage::new(schema.clone(), Default::default()).unwrap();
        let mut owners = extension.owners;
        for owner in &mut owners {
            assert!(
                !owner.programs.is_complete(),
                "production closure must remain Partial"
            );
            owner.programs.closure = SchemaClosure::Complete;
        }
        for d in &schema.definitions {
            let owner = SchemaSubject::Definition(d.address());
            if !owners.iter().any(|row| row.owner == owner) {
                owners.push(DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(vec![]),
                });
            }
        }
        for (index, value) in case.dependencies.iter().enumerate() {
            if let Some(value) = value {
                let actor = matches!(index, 0 | 2 | 5);
                let owner = if actor {
                    subject(def::<ActorDefinition>(0x332a))
                } else {
                    subject(def::<EncounterDefinition>(0x31d1))
                };
                owners
                    .iter_mut()
                    .find(|o| o.owner == owner)
                    .unwrap()
                    .programs
                    .members
                    .push(RuleProgram {
                        id: key(&format!("fixture-resolved-{index}")),
                        context: if actor {
                            RuleEntityKind::Actor
                        } else {
                            RuleEntityKind::Enemy
                        },
                        reads: vec![],
                        nodes: vec![RuleNode {
                            id: key("value"),
                            expression: RuleExpression::Literal {
                                value: quantity(*value, 2),
                            },
                        }],
                        effects: vec![RuleEffect {
                            id: key("value"),
                            when: None,
                            effect: RuleEffectKind::Derive {
                                entity: RuleEntity::Current,
                                stat: def(0x335c + index as u64),
                                value: key("value"),
                            },
                        }],
                    });
            }
        }
        let mut queries: Vec<ContributionQuery> = read("queries.json");
        for (index, query) in queries.iter_mut().enumerate() {
            assert!(!query.groups[0].members.is_complete());
            if close_finite_domain {
                query.groups[0].members.closure = SchemaClosure::Complete;
            }
            if let Some(value) = case.flags[index] {
                let program = key(&format!("fixture-flag-{index}"));
                let owner = subject(def::<ClassDefinition>(0xf0101));
                owners
                    .iter_mut()
                    .find(|o| o.owner == owner)
                    .unwrap()
                    .programs
                    .members
                    .push(RuleProgram {
                        id: program.clone(),
                        context: RuleEntityKind::Actor,
                        reads: vec![],
                        nodes: vec![RuleNode {
                            id: key("value"),
                            expression: RuleExpression::Literal {
                                value: ParameterValue::Boolean(value),
                            },
                        }],
                        effects: vec![RuleEffect {
                            id: key("flag"),
                            when: None,
                            effect: RuleEffectKind::Contribute {
                                entity: if index < 2 {
                                    RuleEntity::Enemy
                                } else {
                                    RuleEntity::Player
                                },
                                stat: query.stat.clone(),
                                contribution: ContributionKind::Flag,
                                value: key("value"),
                            },
                        }],
                    });
                query.groups[0].members.members.push(ContributionMember {
                    owner,
                    program,
                    effect: key("flag"),
                    origin: ContributionOrigin::Character,
                    order: None,
                });
            }
        }
        let recipe = OwnedRecipeInput {
            schema_version: 1,
            registry: RegistryInput {
                schema_version: 1,
                namespace: ns(),
                revision: integer(0),
                last_issued: integer(0),
                entries: vec![],
            },
            schema,
            rules: RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("finite-incoming-critical"),
                semantics_version: key("finite-incoming-critical"),
                operations_version: key(OWNED_RULE_OPERATIONS_V22),
                definitions: checked.identity().clone(),
                tables: vec![],
                owners,
                receivers: DeclaredSet::complete(vec![]),
                effect_applications: Some(DeclaredSet::complete(vec![])),
                contribution_queries: Some(DeclaredSet::complete(queries)),
                existing_actor_rules: Some(DeclaredSet::complete(vec![
                    ExistingActorRuleApplication {
                        id: key("shared-player-initialization"),
                        owner: def(0x332a),
                        targets: vec![ExistingActorRuleTarget::Player],
                    },
                ])),
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("finite-incoming-critical"),
                definitions: checked.identity().clone(),
                outputs: vec![],
            },
        };
        let mut assumptions = vec![ExternalAssumption {
            input: def(0x3251),
            target: AssumptionTarget::Enemy,
            value: ParameterValue::Option(def(case.category)),
        }];
        for index in 0..2 {
            if let Some(presence) = case.presence[index] {
                assumptions.push(ExternalAssumption {
                    input: def(0x3353 + 2 * index as u64),
                    target: AssumptionTarget::Enemy,
                    value: ParameterValue::Boolean(presence),
                });
            }
            if let Some(raw) = case.raw[index] {
                assumptions.push(ExternalAssumption {
                    input: def(0x3354 + 2 * index as u64),
                    target: AssumptionTarget::Enemy,
                    value: quantity(raw, 2),
                });
            }
        }
        Self {
            recipe,
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([0x6b; 16]),
                    2,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class: def(0xf0101),
                    ascendancy: None,
                    level: 20,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                gems: vec![],
                equipment: vec![],
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                support_origins: Some(vec![]),
                generated_inputs: Some(GeneratedSkillInputsV1 {
                    schema_version: 1,
                    bindings: vec![],
                }),
                payload_links: vec![],
                choices: vec![],
            },
            scenario: ScenarioInput {
                game_version: ns(),
                enemy: EnemySpec {
                    encounter: def(0x31d1),
                    level: 82,
                },
                assumptions,
                usage: vec![],
            },
        }
    }
    pub fn plan(&self) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
        empty_support::compile(&self.recipe, &self.build, &self.scenario, def(2)).unwrap()
    }
    pub fn evaluate(&self) -> SupportEffectsReport {
        let plan = self.plan();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
}

pub fn output(report: &SupportEffectsReport, stat: u64) -> Option<&EffectValue> {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        return None;
    };
    effects
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(stat),
                }
        })
        .map(|v| &v.value)
}
pub fn assert_outputs(report: &SupportEffectsReport, expected: [f64; 3]) {
    assert!(report.gaps.is_empty(), "{report:?}");
    for (index, expected) in expected.into_iter().enumerate() {
        assert_eq!(
            output(report, 0x3362 + index as u64),
            Some(&EffectValue::Known {
                value: quantity(expected, if index == 2 { 1 } else { 2 }),
            }),
            "{report:?}"
        );
    }
}
