//! Actual authored passive flag producers and the existing Strength-Life receiver.
//! Only this finite test domain closes the flag inventories. Final Strength is
//! supplied as a test input; this does not claim final Life or full-build parity.
#[allow(dead_code)]
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{owned_mapping::RegistryInput, owned_recipe::OwnedRecipeInput};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([0x6a; 16]), n).unwrap())
}
fn integer(n: i64) -> BoundedInteger {
    BoundedInteger::new(n).unwrap()
}
fn read<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68")
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
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
#[derive(Deserialize)]
struct Producer {
    owner: SchemaSubject,
    program: RuleProgram,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authored {
    schema_version: u32,
    producers: Vec<Producer>,
    owners: Vec<DefinitionRules>,
    receivers: Vec<StatReceiver>,
    queries: Vec<ContributionQuery>,
}
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
    scenario: ScenarioInput,
}
impl World {
    fn new(halving: bool, doubling: bool, close_finite_domain: bool) -> Self {
        let authored: Authored = read("inherent-attribute-flags/rules.json");
        assert_eq!(authored.schema_version, 1);
        assert_eq!(authored.producers.len(), 2);
        let migration: Value = read("strength-life/migration.json");
        let dependencies: Value = read("strength-life/dependencies.json");
        let mut definitions: Vec<DefinitionDescriptor> = migration["schema"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| serde_json::from_value(row["value"].clone()).unwrap())
            .collect();
        definitions.extend(
            serde_json::from_value::<Vec<DefinitionDescriptor>>(
                dependencies["definitions"].clone(),
            )
            .unwrap(),
        );
        definitions.extend([
            DefinitionDescriptor::Class(known(
                def(0xf0001),
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
            DefinitionDescriptor::Encounter(known(
                def(0xf0002),
                EncounterSchema {
                    enemy_level: IntegerRange {
                        minimum: integer(1),
                        maximum: integer(100),
                    },
                    external_inputs: DeclaredSet::complete(vec![]),
                },
            )),
            DefinitionDescriptor::PointPool(known(
                def(0xf0003),
                PointPoolSchema {
                    scope: PointPoolScope::Shared,
                },
            )),
            DefinitionDescriptor::Unit(known(
                def(2),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
        ]);
        for n in [0x10ac, 0x18d9] {
            definitions.push(DefinitionDescriptor::PassiveNode(known(
                def(n),
                PassiveNodeSchema {
                    pools: DeclaredSet::complete(vec![def(0xf0003)]),
                    adjacent: DeclaredSet::complete(vec![]),
                    declarations: ports(),
                },
            )));
        }
        let schema = SchemaPackageInput {
            schema_version: 6,
            namespace: ns(),
            release: key("finite-inherent-flags"),
            semantics_version: key("finite-inherent-flags"),
            definitions,
            slots: vec![],
        };
        let checked =
            OwnedDefinitionSchemaPackage::new(schema.clone(), Default::default()).unwrap();
        let mut owners = authored.owners;
        owners.extend(
            serde_json::from_value::<Vec<DefinitionRules>>(migration["owners"].clone()).unwrap(),
        );
        owners.extend(authored.producers.into_iter().map(|p| DefinitionRules {
            owner: p.owner,
            programs: DeclaredSet::complete(vec![p.program]),
        }));
        for d in &schema.definitions {
            let owner = SchemaSubject::Definition(d.address());
            if !owners.iter().any(|row| row.owner == owner) {
                owners.push(DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(vec![]),
                });
            }
        }
        owners
            .iter_mut()
            .find(|row| row.owner == subject(def::<ClassDefinition>(0xf0001)))
            .unwrap()
            .programs
            .members
            .push(RuleProgram {
                id: key("fixture-final-strength"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Integer(integer(27)),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("strength"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Player,
                        stat: def(0x1d2e),
                        value: key("value"),
                    },
                }],
            });
        let mut receivers = authored.receivers;
        receivers.extend(
            serde_json::from_value::<Vec<StatReceiver>>(migration["receivers"].clone()).unwrap(),
        );
        let mut queries = authored.queries;
        assert!(
            queries
                .iter()
                .all(|q| q.groups.iter().all(|g| !g.members.is_complete()))
        );
        if close_finite_domain {
            for group in queries.iter_mut().flat_map(|q| &mut q.groups) {
                group.members.closure = SchemaClosure::Complete;
            }
        }
        let recipe = OwnedRecipeInput {
            schema_version: 1,
            // This fixture invokes the evaluator helper, not release assembly;
            // the acquisition registry is not an input to native evaluation.
            registry: RegistryInput {
                schema_version: 1,
                namespace: ns(),
                revision: integer(0),
                last_issued: integer(0),
                entries: vec![],
            },
            schema,
            rules: RulePackageInput {
                support_discovery: Some(SupportDiscoveryInput {
                    providers: owners
                        .iter()
                        .map(|row| SupportSourceDomainDeclaration {
                            owner: row.owner.clone(),
                            domain: SchemaState::Known(
                                SupportSourceDomain::AuthoredAssignmentsOnly,
                            ),
                        })
                        .collect(),
                }),
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("finite-inherent-flags"),
                semantics_version: key("finite-inherent-flags"),
                operations_version: key(OWNED_RULE_OPERATIONS_V22),
                definitions: checked.identity().clone(),
                tables: vec![],
                owners,
                receivers: DeclaredSet::complete(receivers),
                effect_applications: Some(DeclaredSet::complete(vec![])),
                contribution_queries: Some(DeclaredSet::complete(queries)),
                existing_actor_rules: None,
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("finite-inherent-flags"),
                definitions: checked.identity().clone(),
                outputs: vec![],
            },
        };
        let allocations = [(halving, 0x10ac), (doubling, 0x18d9)]
            .into_iter()
            .enumerate()
            .filter(|(_, (selected, _))| *selected)
            .map(|(i, (_, n))| Allocation {
                id: occurrence(i as u64 + 2),
                node: def(n),
                pool: def(0xf0003),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            })
            .collect();
        Self {
            recipe,
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([0x6a; 16]),
                    4,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class: def(0xf0001),
                    ascendancy: None,
                    level: 20,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                gems: vec![],
                equipment: vec![],
                allocations,
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
                    encounter: def(0xf0002),
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
        }
    }
    fn plan(&self) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
        empty_support::compile(&self.recipe, &self.build, &self.scenario, def(2)).unwrap()
    }
}
fn amount(report: &SupportEffectsReport) -> &EffectValue {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("{report:?}")
    };
    assert!(report.gaps.is_empty());
    assert!(
        !effects
            .values
            .iter()
            .any(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if *stat == def(0x311a))),
        "no final Life producer was supplied"
    );
    &effects
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(0x331a),
                }
        })
        .unwrap()
        .value
}
#[test]
fn actual_passive_flags_feed_existing_strength_life_receiver_with_all_allocation_combinations() {
    let mut first = None;
    let mut scratch = None;
    for (halving, doubling, expected) in [
        (false, false, 54.),
        (true, false, 27.),
        (false, true, 108.),
        (true, true, 54.),
        (false, false, 54.),
    ] {
        let world = World::new(halving, doubling, true);
        let plan = world.plan();
        let fresh = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(
            amount(&fresh),
            &EffectValue::Known {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(expected, def(0x3119)).unwrap()
                )
            }
        );
        let reused = plan
            .evaluate(scratch.get_or_insert_with(|| plan.new_scratch()))
            .unwrap();
        assert_eq!(fresh, reused);
        if first.is_none() {
            first = Some(fresh.clone());
        }
        if !halving && !doubling {
            assert_eq!(first.as_ref().unwrap(), &fresh);
        }
    }
}
#[test]
fn authored_partial_flag_inventories_remain_unavailable_even_when_selected_flags_are_true() {
    let world = World::new(true, true, false);
    let plan = world.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::IncompleteContributors)
    );
}
