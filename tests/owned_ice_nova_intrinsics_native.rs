//! Actual Ice intrinsic packet in an explicitly finite unpublished component.
//! The original table tests retain an explicit final-input boundary. The new
//! source-input path instead executes authored raw-to-final rules into that slot.
//! Both are finite unpublished components; the real package remains Partial.
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_ice_nova_source_native.rs"]
mod source_inputs;

use poe_optimizer_core::{
    build_identity::*, owned_binding::*, owned_build::*, owned_definitions::*, owned_readiness::*,
    owned_routing::*, owned_rules::*, owned_schema::*, owned_stages::*, owned_support_inputs::*,
    owned_support_receiving::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_rules::OwnedRulePackage,
    owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput},
    owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving,
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
fn id<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([103; 16]), n).unwrap())
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn empty() -> DeclaredSlots {
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
fn finite(slots: &mut DeclaredSlots) {
    slots.parameters.closure = SchemaClosure::Complete;
    slots.choices.closure = SchemaClosure::Complete;
    slots.grants.closure = SchemaClosure::Complete;
    slots.actors.closure = SchemaClosure::Complete;
    slots.skill_grants.closure = SchemaClosure::Complete;
    slots.outputs.closure = SchemaClosure::Complete;
    slots.sockets.closure = SchemaClosure::Complete;
}
#[derive(Clone, Deserialize)]
struct Channels {
    cold_min: StatDefId,
    cold_max: StatDefId,
    radius: StatDefId,
}
impl Channels {
    fn all(&self) -> [&StatDefId; 3] {
        [&self.cold_min, &self.cold_max, &self.radius]
    }
}
#[derive(Clone, Deserialize)]
struct StatSet {
    source_index: u32,
    stat_set: ActionStatSetDefId,
}
#[derive(Clone, Deserialize)]
struct Bindings {
    physical_gem: GemDefId,
    primary_skill: SkillDefId,
    primary_supply: DeclaredSlot<SkillGrantSlotDefId>,
    entering_grant: DeclaredSlot<GrantSlotDefId>,
    output: DeclaredSlot<ActionOutputDefId>,
    part: ActionPartDefId,
    mode: ActionModeDefId,
    final_level: DeclaredSlot<ParameterSlotDefId>,
    channels: Channels,
    stat_sets: Vec<StatSet>,
    program: OwnedDefinitionKey,
}
#[derive(Clone)]
struct Fixture {
    b: Bindings,
    schema: SchemaPackageInput,
    rules: RulePackageInput,
    routing: ActionRoutingInput,
    build: BuildInput,
    scenario: ScenarioInput,
    queries: QueryInput,
    quality_unit: UnitDefId,
    fixture_level: DeclaredSlot<ParameterSlotDefId>,
    actual_owner: DefinitionRules,
    actual_routes: ActionOutputRoutes,
}
impl Fixture {
    fn def<K: DefinitionDomain>(&self, name: &str) -> DefId<K> {
        DefId::parse(self.schema.namespace.clone(), name).unwrap()
    }
    fn provider(copy: usize) -> ProviderKey {
        ProviderKey {
            root: ProviderRoot::SkillUse(id(1000 + copy as u64)),
            grant_path: vec![],
        }
    }
    fn generated(&self, copy: usize) -> GeneratedSkillKey {
        GeneratedSkillKey {
            provider: Self::provider(copy),
            slot: self.b.primary_supply.clone(),
        }
    }
    fn action(&self, copy: usize, set: usize) -> ActionSelection {
        let mut provider = Self::provider(copy);
        provider.grant_path.push(self.b.entering_grant.clone());
        ActionSelection {
            action: ActionKey {
                actor: ActorKey::Player,
                provider,
                output: self.b.output.clone(),
            },
            part: self.b.part.clone(),
            mode: self.b.mode.clone(),
            stat_set: self.b.stat_sets[set].stat_set.clone(),
        }
    }
    fn load() -> Self {
        static FIXTURE: OnceLock<Fixture> = OnceLock::new();
        FIXTURE.get_or_init(Self::from_publication).clone()
    }
    fn from_publication() -> Self {
        let output = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ICE_INTRINSICS_OUTPUT")
                .expect("verified Ice intrinsic publication parent"),
        );
        let endpoint = release::load(&output.join("package"));
        assert!(endpoint.input().evaluation.is_none());
        let packet = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/ice-nova-intrinsics");
        let b: Bindings = read(packet.join("bindings.json"));
        assert_eq!(
            b.stat_sets
                .iter()
                .map(|s| s.source_index)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        let migration: OwnedReleaseMigrationInput = read(packet.join("migration.json"));
        let dependencies: Value = read(packet.join("dependencies.json"));
        let recipe = &endpoint.input().recipe;
        assert_eq!(recipe.schema.schema_version, 5);
        assert_eq!(
            recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V17
        );
        let mut definitions: Vec<DefinitionDescriptor> =
            serde_json::from_value(dependencies["definitions"].clone()).unwrap();
        let mut slots: Vec<SlotDescriptor> =
            serde_json::from_value(dependencies["slots"].clone()).unwrap();
        // Dependencies are historical immutable rows. A migrated row supersedes
        // its prior copy below, before the full actual-schema equality check.
        for row in &migration.schema {
            match row {
                SchemaExtensionEntry::Definition(row) => {
                    definitions.retain(|d| d.address() != row.address());
                    definitions.push(row.clone());
                }
                SchemaExtensionEntry::Slot(row) => {
                    slots.retain(|d| d.address() != row.address());
                    slots.push(row.clone());
                }
            }
        }
        for row in &definitions {
            assert!(
                recipe.schema.definitions.contains(row),
                "actual declared definition retained"
            );
        }
        for row in &slots {
            assert!(
                recipe.schema.slots.contains(row),
                "actual declared slot retained"
            );
        }
        for (index, stat) in b.channels.all().into_iter().enumerate() {
            let unit = definitions
                .iter()
                .find_map(|d| match d {
                    DefinitionDescriptor::Stat(DefinitionEntry {
                        id,
                        schema:
                            SchemaState::Known(StatSchema {
                                value: ComputedValueType::Quantity { unit },
                                ..
                            }),
                    }) if id == stat => Some(unit),
                    _ => None,
                })
                .unwrap();
            let dimension = definitions
                .iter()
                .find_map(|d| match d {
                    DefinitionDescriptor::Unit(DefinitionEntry {
                        id,
                        schema: SchemaState::Known(UnitSchema { dimension }),
                    }) if id == unit => Some(dimension),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                *dimension,
                if index == 2 {
                    UnitDimension::Distance
                } else {
                    UnitDimension::Damage
                }
            );
        }
        let final_slot = slots
            .iter()
            .find(|s| s.address() == SlotAddress::Parameter(b.final_level.clone()))
            .unwrap();
        let SlotDescriptor::Parameter(DefinitionEntry {
            schema: SchemaState::Known(final_schema),
            ..
        }) = final_slot
        else {
            panic!("known final slot")
        };
        assert_eq!(final_schema.presence, SlotPresence::RequiredOnce);
        assert_eq!(
            final_schema.skill_input,
            Some(SkillInputAuthority::Projected)
        );
        assert!(final_schema.sites.is_empty());
        assert_eq!(
            final_schema.value,
            ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(1).unwrap(),
                maximum: BoundedInteger::new(40).unwrap()
            })
        );
        let actual_owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(b.primary_skill.address()))
            .unwrap()
            .clone();
        assert!(migration.owners.contains(&actual_owner));
        assert!(!actual_owner.programs.is_complete());
        assert_eq!(actual_owner.programs.members.len(), 1);
        assert_eq!(actual_owner.programs.members[0].id, b.program);
        assert_eq!(
            actual_owner.programs.members[0].context,
            RuleEntityKind::Action
        );
        assert_eq!(migration.tables.len(), 4);
        for table in &migration.tables {
            assert!(recipe.rules.tables.contains(table));
            assert_eq!(table.rows.len(), 40);
        }
        let actual_routes = recipe
            .routing
            .outputs
            .iter()
            .find(|r| r.output == b.output)
            .unwrap()
            .clone();
        assert!(!actual_routes.routes.is_complete());
        assert_eq!(actual_routes.routes.members.len(), 6);
        assert!(
            actual_routes
                .source_selectors
                .as_ref()
                .is_some_and(|s| !s.is_complete())
        );
        let authored_routing: ActionRoutingInput = read(packet.join("routing.json"));
        assert_eq!(recipe.routing.schema_version, 2);
        let mut authored_routes = authored_routing
            .outputs
            .iter()
            .find(|r| r.output == b.output)
            .unwrap()
            .clone();
        authored_routes
            .routes
            .members
            .sort_by(|a, b| a.id.cmp(&b.id));
        authored_routes
            .source_selectors
            .as_mut()
            .unwrap()
            .members
            .sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(actual_routes, authored_routes);
        let gem_owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(b.physical_gem.address()))
            .unwrap()
            .clone();
        assert!(!gem_owner.programs.is_complete());
        assert_eq!(gem_owner.programs.members.len(), 1);
        assert!(gem_owner.programs.members[0].effects.iter().all(|e|matches!(&e.effect,RuleEffectKind::ActivateGrant{slot,..} if *slot==b.entering_grant)),"production primary remains activation only");
        // Select exact required units from actual schema. No synthetic replacement
        // of the packet's Damage/Distance or physical input units is permitted.
        let quality_unit = recipe
            .schema
            .definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::Unit(DefinitionEntry {
                    id,
                    schema:
                        SchemaState::Known(UnitSchema {
                            dimension: UnitDimension::PercentagePoints,
                        }),
                }) => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        if !definitions
            .iter()
            .any(|d| d.address() == quality_unit.address())
        {
            definitions.push(
                recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == quality_unit.address())
                    .unwrap()
                    .clone(),
            );
        }
        let namespace = recipe.schema.namespace.clone();
        let class: ClassDefId =
            DefId::parse(namespace.clone(), "intrinsic-component-class").unwrap();
        let encounter: EncounterDefId =
            DefId::parse(namespace.clone(), "intrinsic-component-encounter").unwrap();
        let requested: MetricDefId =
            DefId::parse(namespace.clone(), "intrinsic-component-request").unwrap();
        let fixture_level = DeclaredSlot {
            declaration: SlotOwnerDefId::Gem(b.physical_gem.clone()),
            slot: DefId::parse(namespace.clone(), "fixture-final-level").unwrap(),
        };
        for d in &mut definitions {
            match d {
                DefinitionDescriptor::Gem(e) if e.id == b.physical_gem => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    assert!(!s.declarations.parameters.is_complete());
                    s.skills.closure = SchemaClosure::Complete;
                    s.quality.allowed_kinds.closure = SchemaClosure::Complete;
                    finite(&mut s.declarations);
                    s.declarations
                        .parameters
                        .members
                        .push(fixture_level.clone());
                }
                DefinitionDescriptor::Skill(e) if e.id == b.primary_skill => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    finite(&mut s.declarations);
                }
                _ => {}
            }
        }
        for row in &mut slots {
            match row {
                SlotDescriptor::SkillGrant(e) if e.id == b.primary_supply => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.outputs.closure = SchemaClosure::Complete;
                }
                SlotDescriptor::ActionOutput(e) if e.id == b.output => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.choices.closure = SchemaClosure::Complete;
                }
                _ => {}
            }
        }
        slots.push(SlotDescriptor::Parameter(known(
            fixture_level.clone(),
            ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(41).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::GemParameter],
                skill_input: None,
            },
        )));
        let range = IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        };
        let damage_unit = definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::Stat(DefinitionEntry {
                    id,
                    schema:
                        SchemaState::Known(StatSchema {
                            value: ComputedValueType::Quantity { unit },
                            ..
                        }),
                }) if *id == b.channels.cold_min => Some(unit.clone()),
                _ => None,
            })
            .unwrap();
        definitions.extend([
            DefinitionDescriptor::Class(known(
                class.clone(),
                ClassSchema {
                    level: range.clone(),
                    ascendancies: DeclaredSet::complete(vec![]),
                    implicit_passives: DeclaredSet::complete(vec![]),
                    declarations: empty(),
                },
            )),
            DefinitionDescriptor::Encounter(known(
                encounter.clone(),
                EncounterSchema {
                    enemy_level: range,
                    external_inputs: DeclaredSet::complete(vec![]),
                },
            )),
            DefinitionDescriptor::Metric(known(
                requested.clone(),
                MetricSchema {
                    targets: vec![MetricTargetKind::Action],
                    unit: damage_unit,
                    actor_roles: vec![MetricActorRole::Player],
                    provider_roles: vec![ProviderRole::SkillUse],
                },
            )),
            DefinitionDescriptor::ActionStatSet(known(
                DefId::parse(namespace.clone(), "foreign-set").unwrap(),
                ActionStatSetSchema {},
            )),
        ]);
        for (name, scope, value) in [
            (
                "unused-level",
                RuleEntityKind::SupportOrigin,
                ComputedValueType::Integer,
            ),
            (
                "unused-quality",
                RuleEntityKind::SupportOrigin,
                ComputedValueType::Quantity {
                    unit: quality_unit.clone(),
                },
            ),
            (
                "unused-flag",
                RuleEntityKind::Skill,
                ComputedValueType::Boolean,
            ),
        ] {
            definitions.push(DefinitionDescriptor::Stat(known(
                DefId::parse(namespace.clone(), name).unwrap(),
                StatSchema {
                    value,
                    targets: vec![scope],
                },
            )));
        }
        let schema = SchemaPackageInput {
            schema_version: 5,
            namespace: namespace.clone(),
            release: key("unpublished-ice-intrinsic-component"),
            semantics_version: key("intrinsic-component-only"),
            definitions,
            slots,
        };
        let mut owners = vec![gem_owner, actual_owner.clone()];
        for owner in &mut owners {
            owner.programs.closure = SchemaClosure::Complete;
        }
        owners[0].programs.members.push(RuleProgram {
            id: key("fixture-final-input-only"),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("explicit-final"),
                value_type: ComputedValueType::Integer,
                source: RuleReadSource::Parameter {
                    slot: fixture_level.clone(),
                },
            }],
            nodes: vec![RuleNode {
                id: key("final"),
                expression: RuleExpression::Read {
                    input: key("explicit-final"),
                },
            }],
            effects: vec![RuleEffect {
                id: key("fixture-project-final"),
                when: None,
                effect: RuleEffectKind::ProjectSkillParameter {
                    skill: b.primary_supply.clone(),
                    parameter: b.final_level.clone(),
                    value: key("final"),
                },
            }],
        });
        for owner in [
            SchemaSubject::Definition(class.address()),
            SchemaSubject::Definition(encounter.address()),
            SchemaSubject::Slot(SlotAddress::ActionOutput(b.output.clone())),
            SchemaSubject::Slot(SlotAddress::Grant(b.entering_grant.clone())),
            SchemaSubject::Slot(SlotAddress::SkillGrant(b.primary_supply.clone())),
        ] {
            owners.push(DefinitionRules {
                owner,
                programs: DeclaredSet::complete(vec![]),
            });
        }
        let mut rules = recipe.rules.clone();
        rules.release = schema.release.clone();
        rules.semantics_version = schema.semantics_version.clone();
        rules.tables = migration.tables;
        rules.owners = owners;
        rules.receivers = DeclaredSet::complete(vec![]);
        rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        let mut routes = actual_routes.clone();
        routes.routes.closure = SchemaClosure::Complete;
        routes.source_selectors.as_mut().unwrap().closure = SchemaClosure::Complete;
        let routing = ActionRoutingInput {
            schema_version: 2,
            namespace: namespace.clone(),
            release: schema.release.clone(),
            definitions: recipe.routing.definitions.clone(),
            outputs: vec![routes],
        };
        let build = BuildInput {
            generated_inputs: None,
            allocator: InstanceAllocatorState::from_parts(
                BuildLineage::from_bytes([103; 16]),
                10000,
            ),
            revision: BuildRevision::from_u64(1),
            game_version: namespace.clone(),
            character: CharacterSpec {
                class,
                ascendancy: None,
                level: 20,
                rewards: vec![],
            },
            weapon_loadouts: vec![id(1), id(2)],
            active_weapon_loadout: id(1),
            items: vec![],
            equipment: vec![],
            gems: vec![],
            allocations: vec![],
            skills: vec![],
            supports: vec![],
            support_origins: Some(vec![]),
            payload_links: vec![],
            choices: vec![],
        };
        let scenario = ScenarioInput {
            game_version: namespace.clone(),
            enemy: EnemySpec {
                encounter,
                level: 20,
            },
            assumptions: vec![],
            usage: vec![],
        };
        let queries = QueryInput {
            game_version: namespace,
            requests: vec![],
        };
        let mut f = Self {
            b,
            schema,
            rules,
            routing,
            build,
            scenario,
            queries,
            quality_unit,
            fixture_level,
            actual_owner,
            actual_routes,
        };
        f.levels(&[17, 1]);
        f
    }
    fn levels(&mut self, levels: &[i64]) {
        self.build.gems.clear();
        self.build.skills.clear();
        self.queries.requests.clear();
        for (copy, &level) in levels.iter().enumerate() {
            let mut parameters = vec![];
            for row in &self.schema.slots {
                let SlotDescriptor::Parameter(e) = row else {
                    continue;
                };
                if e.id.declaration != SlotOwnerDefId::Gem(self.b.physical_gem.clone()) {
                    continue;
                }
                let SchemaState::Known(s) = &e.schema else {
                    panic!()
                };
                let value = if e.id == self.fixture_level {
                    integer(level)
                } else {
                    match &s.value {
                        ValueSchema::Boolean => ParameterValue::Boolean(false),
                        ValueSchema::Quantity(range) => ParameterValue::Quantity(
                            FiniteQuantity::new(0., range.minimum.unit().clone()).unwrap(),
                        ),
                        _ => panic!("unexpected actual physical slot"),
                    }
                };
                parameters.push(ParameterAssignment {
                    slot: e.id.clone(),
                    value,
                });
            }
            self.build.gems.push(GemInstance {
                id: id(100 + copy as u64),
                definition: self.b.physical_gem.clone(),
                parameters,
                level: 17,
                quality: None,
            });
            self.build.skills.push(SkillUse {
                id: id(1000 + copy as u64),
                source: AuthoredSkillSource::Gem(id(100 + copy as u64)),
                enabled: true,
                scope: LoadoutScope::Shared,
                parameters: None,
            });
            for set in 0..2 {
                self.queries.requests.push(MetricRequest {
                    id: QueryId::new(format!("copy-{copy}-set-{set}")).unwrap(),
                    metric: self.def("intrinsic-component-request"),
                    target: MetricTarget::Action(Box::new(self.action(copy, set))),
                });
            }
        }
    }
    fn request(&self) -> OwnedEvaluationRequest {
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), Default::default()).unwrap(),
            ScenarioSpec::new(self.scenario.clone(), Default::default()).unwrap(),
            QuerySpec::new(self.queries.clone(), Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap()
    }
    fn plan(&self) -> Plan {
        self.plan_with_source(None)
    }
    fn plan_with_source(&self, source: Option<&source_inputs::Configuration>) -> Plan {
        self.try_plan_with_source(source).unwrap()
    }
    fn try_plan_with_source(
        &self,
        source: Option<&source_inputs::Configuration>,
    ) -> std::result::Result<Plan, Box<dyn std::error::Error>> {
        let definitions = Arc::new(OwnedDefinitionSchemaPackage::new(
            self.schema.clone(),
            Default::default(),
        )?);
        let mut rules = self.rules.clone();
        rules.definitions = definitions.identity().clone();
        let stored = OwnedRulePackage::new(rules, definitions.as_ref(), Default::default())?;
        let compiled = Arc::new(CompiledRulePackage::compile_stored(
            &stored,
            definitions.as_ref(),
            Default::default(),
        )?);
        let mut routing = self.routing.clone();
        routing.definitions = definitions.identity().clone();
        let routing = Arc::new(OwnedActionRouting::new(
            routing,
            definitions.as_ref(),
            Default::default(),
        )?);
        let programs: Vec<_> = stored
            .input()
            .owners
            .iter()
            .flat_map(|o| o.programs.members.iter().map(move |p| (o, p)))
            .collect();
        let early = |p: &RuleProgram| p.context != RuleEntityKind::Action;
        let stages = Arc::new(OwnedEvaluationStages::new(
            source.map_or_else(
                || EvaluationStagesInput {
                    schema_version: 2,
                    namespace: self.schema.namespace.clone(),
                    release: key("component-stages"),
                    definitions: definitions.identity().clone(),
                    rules: *stored.identity(),
                    routing: *routing.identity(),
                    stages: vec![
                        EvaluationStage {
                            id: key("prepare"),
                            predecessors: vec![],
                        },
                        EvaluationStage {
                            id: key("execute"),
                            predecessors: vec![key("prepare")],
                        },
                    ],
                    programs: DeclaredSet::complete(
                        programs
                            .iter()
                            .map(|(o, p)| StagedRuleProgram {
                                owner: o.owner.clone(),
                                program: p.id.clone(),
                                stage: key(if early(p) { "prepare" } else { "execute" }),
                            })
                            .collect(),
                    ),
                    effect_applications: Some(DeclaredSet::complete(vec![])),
                    routing_stage: key("execute"),
                    frozen_channels: [
                        ("unused-level", RuleEntityKind::SupportOrigin),
                        ("unused-quality", RuleEntityKind::SupportOrigin),
                        ("unused-flag", RuleEntityKind::Skill),
                    ]
                    .into_iter()
                    .map(|(name, scope)| FrozenStageChannel {
                        channel: StageChannel::Stat {
                            scope,
                            stat: self.def(name),
                        },
                        stage: key("prepare"),
                    })
                    .collect(),
                    readiness: Some(ReadinessInput {
                        skills: vec![SkillReadiness {
                            participation: None,
                            skill: self.b.primary_skill.clone(),
                            parameters: DeclaredSet::complete(vec![ParameterReadiness {
                                parameter: self.b.final_level.clone(),
                                phase: ReadinessPhase::Execution,
                            }]),
                        }],
                        programs: DeclaredSet::complete(
                            programs
                                .iter()
                                .map(|(o, p)| {
                                    let projection = p.id == key("fixture-final-input-only");
                                    ReadinessProgram {
                                        owner: o.owner.clone(),
                                        program: p.id.clone(),
                                        phase: if early(p) {
                                            ReadinessPhase::Structural
                                        } else {
                                            ReadinessPhase::Execution
                                        },
                                        role: if projection {
                                            ReadinessProgramRole::FinalInputAssembly
                                        } else if early(p) {
                                            ReadinessProgramRole::PreparationFacts
                                        } else {
                                            ReadinessProgramRole::Execution
                                        },
                                        outputs: if projection {
                                            vec![StageChannel::SkillParameter {
                                                parameter: self.b.final_level.clone(),
                                            }]
                                        } else if early(p) {
                                            vec![StageChannel::Grant {
                                                slot: self.b.entering_grant.clone(),
                                            }]
                                        } else {
                                            vec![]
                                        },
                                    }
                                })
                                .collect(),
                        ),
                    }),
                },
                |source| source.stages(self, &definitions, &stored, &routing),
            ),
            definitions.as_ref(),
            &stored,
            &routing,
            Default::default(),
        )?);
        let preparation = Arc::new(OwnedSupportPreparation::new(
            source.map_or_else(
                || SupportPreparationInput {
                    schema_version: 1,
                    namespace: self.schema.namespace.clone(),
                    release: key("component-no-supports"),
                    definitions: definitions.identity().clone(),
                    rules: *stored.identity(),
                    policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                    quality_unit: self.quality_unit.clone(),
                    types: vec![],
                    effects: vec![],
                    families: vec![],
                    supports: vec![],
                },
                |source| source.preparation(self, &definitions, &stored),
            ),
            definitions.as_ref(),
            &stored,
            Default::default(),
        )?);
        let flag = || self.def("unused-flag");
        let optional = || OptionalTypeInputs {
            present: flag(),
            members: vec![],
        };
        let inputs = Arc::new(OwnedSupportInputBindings::new(
            source.map_or_else(
                || SupportInputBindingsInput {
                    schema_version: 1,
                    namespace: self.schema.namespace.clone(),
                    release: key("component-support-inputs"),
                    definitions: definitions.identity().clone(),
                    rules: *stored.identity(),
                    preparation: *preparation.identity(),
                    stages: *stages.identity(),
                    preparation_stage: key("prepare"),
                    effective_level: self.def("unused-level"),
                    effective_quality: self.def("unused-quality"),
                    target: SupportTargetInputBindings {
                        skill_types: vec![],
                        minion_types: optional(),
                        summoner: OptionalTypeContextInputs {
                            present: flag(),
                            skill_types: vec![],
                            minion_types: optional(),
                        },
                        cannot_be_supported: flag(),
                        has_gem: flag(),
                        from_item: flag(),
                        is_player_actor: flag(),
                    },
                },
                |source| source.inputs(self, &definitions, &stored, &preparation, &stages),
            ),
            definitions.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )?);
        let receiving = Arc::new(OwnedSupportReceiving::new(
            source.map_or_else(
                || SupportReceivingInput {
                    source_properties: None,
                    schema_version: 2,
                    namespace: self.schema.namespace.clone(),
                    release: key("component-no-receivers"),
                    definitions: definitions.identity().clone(),
                    rules: *stored.identity(),
                    preparation: *preparation.identity(),
                    inputs: *inputs.identity(),
                    stages: *stages.identity(),
                    roles: vec![],
                    targets: vec![],
                    supports: vec![],
                },
                |source| {
                    source.receiving(self, &definitions, &stored, &preparation, &inputs, &stages)
                },
            ),
            definitions.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )?);
        assert!(
            matches!(
                OwnedEffectPlan::compile(
                    Arc::new(self.request()),
                    definitions.clone(),
                    compiled.clone(),
                    routing.clone(),
                    Default::default()
                ),
                Err(PlanError::Invalid(_))
            ),
            "V17 requires checked readiness"
        );
        Ok(Plan::compile(
            SupportEffectPlanInputs {
                request: Arc::new(self.request()),
                definitions,
                rules: compiled,
                routing,
                stages,
                preparation,
                inputs,
                receiving,
            },
            Default::default(),
            Default::default(),
        )?)
    }
}

fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!(
            "component unavailable: {:?}; gaps: {:?}",
            report.outcome, report.gaps
        )
    };
    effects
}
fn value<'a>(report: &'a SupportEffectsReport, key: &PlanValueKey) -> Option<&'a EffectValue> {
    let found: Vec<_> = effects(report)
        .values
        .iter()
        .filter(|row| &row.key == key)
        .collect();
    assert!(found.len() <= 1);
    found.first().map(|row| &row.value)
}
fn action_value<'a>(
    f: &Fixture,
    report: &'a SupportEffectsReport,
    copy: usize,
    set: usize,
    stat: &StatDefId,
) -> Option<&'a EffectValue> {
    value(
        report,
        &PlanValueKey::Stat {
            entity: ConcreteEntity::Action(Box::new(f.action(copy, set))),
            stat: stat.clone(),
        },
    )
}
fn expected() -> &'static BTreeMap<(u32, i64), [f64; 3]> {
    static EXPECTED: OnceLock<BTreeMap<(u32, i64), [f64; 3]>> = OnceLock::new();
    EXPECTED.get_or_init(|| {
        let base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("runs/owned-ice-nova-intrinsic-source-01");
        let off = fs::read(base.join("source-jit-off.json")).unwrap();
        let on = fs::read(base.join("source-jit-on.json")).unwrap();
        assert_eq!(off.len(), 6_919_663);
        assert_eq!(off, on);
        assert_eq!(
            format!("{:x}", Sha256::digest(&off)),
            "dbbb23258ce9cd3d2c90340ed0bafd1a956cd06df9779e800265c68b9598ee56"
        );
        let report: Value = serde_json::from_slice(&off).unwrap();
        assert_eq!(report["native_build_parity"], false);
        assert_eq!(report["final_input_authority"], false);
        let cases = report["cases"].as_array().unwrap();
        let original = cases.iter().find(|c| c["name"] == "original-05").unwrap();
        let probes = original["states"]["fresh"]["intrinsic"]["admitted"]
            .as_array()
            .unwrap();
        assert_eq!(probes.len(), 80);
        let mut results = BTreeMap::new();
        for probe in probes {
            assert_eq!(probe["success"], true);
            assert_eq!(probe["instance"]["quality"], 0);
            let k = (
                probe["stat_set"].as_u64().unwrap() as u32,
                probe["instance"]["level"].as_i64().unwrap(),
            );
            let values = [
                "spell_minimum_base_cold_damage",
                "spell_maximum_base_cold_damage",
                "active_skill_base_area_of_effect_radius",
            ]
            .map(|name| probe["stats"][name].as_f64().unwrap());
            assert!(results.insert(k, values).is_none());
        }
        assert_eq!(results.len(), 80);
        results
    })
}
fn assert_numbers(f: &Fixture, report: &SupportEffectsReport, copy: usize, level: i64) {
    for set in 0..2 {
        for (stat, number) in
            f.b.channels
                .all()
                .into_iter()
                .zip(expected()[&(f.b.stat_sets[set].source_index, level)])
        {
            let Some(EffectValue::Known {
                value: ParameterValue::Quantity(value),
            }) = action_value(f, report, copy, set, stat)
            else {
                panic!("missing intrinsic copy{copy}/set{set}/{stat:?}")
            };
            assert_eq!(value.value(), number);
        }
    }
}

#[test]
#[ignore = "requires the checked Ice intrinsic publication and authenticated source reports"]
fn actual_packet_matches_all_eighty_original_helper_vectors() {
    let mut f = Fixture::load();
    f.levels(&(1..=40).collect::<Vec<_>>());
    assert!(
        f.build.gems.iter().all(|g| g.level == 17),
        "physical and explicit final inputs are separate"
    );
    let plan = f.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(plan.binding_report().schema(), SchemaBindingStatus::Valid);
    for copy in 0..40 {
        assert_numbers(&f, &report, copy, copy as i64 + 1);
    }
}

#[test]
#[ignore = "requires the checked Ice intrinsic publication and authenticated source reports"]
fn repeated_occurrences_activation_and_parallel_scratch_keep_inputs_separate() {
    let original = Fixture::load();
    let a = original.plan();
    let report_a = a.evaluate(&mut a.new_scratch()).unwrap();
    assert_numbers(&original, &report_a, 0, 17);
    assert_numbers(&original, &report_a, 1, 1);
    assert_eq!(
        original.build.gems[1].level, 17,
        "physical level does not supply the final input1"
    );
    assert_eq!(original.action(0, 0).action, original.action(0, 1).action);
    assert_ne!(
        original.action(0, 0).stat_set,
        original.action(0, 1).stat_set
    );
    assert_ne!(
        original.action(0, 0).action.provider,
        original.action(1, 0).action.provider
    );
    let mut changed = original.clone();
    changed.levels(&[1, 40]);
    let b = changed.plan();
    let report_b = b.evaluate(&mut b.new_scratch()).unwrap();
    assert_numbers(&changed, &report_b, 0, 1);
    assert_numbers(&changed, &report_b, 1, 40);
    // Explicitly requesting an unavailable action invalidates the whole request;
    // it is not a request to silently omit that action's contributors.
    let mut disabled = changed.clone();
    disabled.build.skills[0].enabled = false;
    let requested = disabled.plan();
    let rejected = requested.evaluate(&mut requested.new_scratch()).unwrap();
    assert!(matches!(
        rejected.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert!(rejected.gaps.iter().any(|gap| {
        gap.reason == PlanGapReason::UnresolvedTopology
            && gap.provider.as_ref() == Some(&disabled.action(0, 0).action.provider)
            && gap.subject.as_ref()
                == Some(&SchemaSubject::Slot(SlotAddress::ActionOutput(
                    disabled.b.output.clone(),
                )))
    }));
    // Retain the disabled physical occurrence but ask only about its live sibling.
    // This changes the query explicitly and does not bypass the completeness gate.
    let disabled_provider = disabled.action(0, 0).action.provider;
    disabled.queries.requests.retain(|query| {
        !matches!(&query.target, MetricTarget::Action(action)
            if action.action.provider == disabled_provider)
    });
    assert_eq!(disabled.queries.requests.len(), 2);
    let live = disabled.plan();
    let live_report = live.evaluate(&mut live.new_scratch()).unwrap();
    assert_numbers(&disabled, &live_report, 1, 40);
    for set in 0..2 {
        for stat in disabled.b.channels.all() {
            assert!(action_value(&disabled, &live_report, 0, set, stat).is_none());
        }
    }
    let mut scratch = a.new_scratch();
    for (plan, want) in [(&a, &report_a), (&b, &report_b), (&a, &report_a)] {
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), want);
    }
    let mut work = 0;
    assert!(matches!(
        a.evaluate_with_budget(&mut scratch, &mut work),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(a.evaluate(&mut scratch).unwrap(), report_a);
    let plans = Arc::new([a, b]);
    let reports = [report_a, report_b];
    let actual = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..48usize)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |scratch, i| (i % 2, plans[i % 2].evaluate(scratch).unwrap()),
                )
                .collect::<Vec<_>>()
        });
    for (i, report) in actual {
        assert_eq!(report, reports[i]);
    }
}

#[test]
#[ignore = "requires the checked Ice intrinsic publication and authenticated source reports"]
fn missing_and_invalid_final_inputs_cannot_use_source_helper_fallbacks() {
    let mut missing = Fixture::load();
    missing
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == SchemaSubject::Definition(missing.b.physical_gem.address()))
        .unwrap()
        .programs
        .members
        .retain(|p| p.id != key("fixture-final-input-only"));
    let plan = missing.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for copy in 0..2 {
        assert!(
            value(
                &report,
                &PlanValueKey::SkillParameter {
                    skill: Box::new(missing.generated(copy)),
                    parameter: missing.b.final_level.clone()
                }
            )
            .is_none(),
            "an absent producer has no fabricated projected value row"
        );
        for set in 0..2 {
            for stat in missing.b.channels.all() {
                assert!(
                    matches!(
                        action_value(&missing, &report, copy, set, stat),
                        Some(EffectValue::Unresolved {
                            reason: PlanGapReason::MissingProducer,
                            ..
                        })
                    ),
                    "missing final producer must remain the exact routed cause"
                );
            }
        }
    }
    for invalid in [0, 41] {
        let mut f = Fixture::load();
        f.levels(&[invalid, 17]);
        let plan = f.plan();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let projected = value(
            &report,
            &PlanValueKey::SkillParameter {
                skill: Box::new(f.generated(0)),
                parameter: f.b.final_level.clone(),
            },
        );
        assert!(
            matches!(projected,Some(EffectValue::UnsupportedValue{value}) if *value==integer(invalid)),
            "actual final slot rejects invalid value: {projected:?}"
        );
        for set in 0..2 {
            for stat in f.b.channels.all() {
                assert!(!matches!(
                    action_value(&f, &report, 0, set, stat),
                    Some(EffectValue::Known { .. })
                ));
            }
        }
        assert_numbers(&f, &report, 1, 17);
    }
}

#[test]
#[ignore = "requires the checked Ice intrinsic publication and authenticated source reports"]
fn foreign_selection_unknown_activation_and_actual_partial_coverage_remain_unresolved() {
    let original = Fixture::load();
    let definitions =
        OwnedDefinitionSchemaPackage::new(original.schema.clone(), Default::default()).unwrap();
    let request = original.request();
    let resolver =
        OwnedOccurrenceResolver::new(&definitions, &request, Default::default()).unwrap();
    let mut foreign = original.action(0, 0);
    foreign.stat_set = original.def("foreign-set");
    let report = resolver.action(&foreign).unwrap();
    assert_eq!(report.schema(), SchemaBindingStatus::Invalid);
    assert!(report.value().is_none());
    let mut inactive = original.clone();
    inactive
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == SchemaSubject::Definition(inactive.b.physical_gem.address()))
        .unwrap()
        .programs
        .members
        .retain(|p| p.id == key("fixture-final-input-only"));
    let plan = inactive.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    for copy in 0..2 {
        assert!(
            report.gaps.iter().any(|gap| {
                gap.reason == PlanGapReason::UnresolvedActivation
                    && gap.provider.as_ref() == Some(&Fixture::provider(copy))
                    && gap.subject.as_ref()
                        == Some(&SchemaSubject::Definition(
                            inactive.b.primary_skill.address(),
                        ))
            }),
            "missing activation retains exact occurrence diagnostic"
        );
    }
    let mut partial = original.clone();
    *partial
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == partial.actual_owner.owner)
        .unwrap() = partial.actual_owner.clone();
    let plan = partial.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    let mut partial = original.clone();
    partial.routing.outputs[0] = partial.actual_routes.clone();
    let plan = partial.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        matches!(report.outcome, SupportEffectsOutcome::Unavailable { .. }),
        "actual Partial routing cannot become executable"
    );
}

#[test]
#[ignore = "requires both checked Ice publications; finite already-admitted support positions"]
fn authored_source_inputs_produce_final_levels_and_both_intrinsic_alternatives() {
    // Original selected/archived compositions, the real Exodus threshold, and
    // the item-free empty case. Type matching and ordinary delivery remain out
    // of this finite boundary; all numerical programs come from the packet.
    for (supports, count, final_level) in [
        (vec![0, 1, 2], 3, 17),
        (vec![0, 1, 3], 3, 17),
        (vec![0, 1, 3, 4], 4, 17),
        (vec![5], 1, 20),
        (vec![5, 1], 2, 17),
        (vec![], 0, 17),
    ] {
        let mut source = source_inputs::SourceFixture::load();
        source.raw(&[17]);
        source.supports(0, &supports.iter().map(|i| (*i, 0.)).collect::<Vec<_>>());
        assert!(source.f.build.items.is_empty() && source.f.build.equipment.is_empty());
        let plan = source.plan();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(
            value(&report, &source.final_key(0)),
            Some(&EffectValue::Known {
                value: integer(final_level)
            })
        );
        assert_eq!(
            value(&report, &source.stat_key(0, "count")),
            Some(&EffectValue::Known {
                value: integer(count)
            })
        );
        assert_numbers(&source.f, &report, 0, final_level);
        for position in 0..supports.len() {
            assert_eq!(
                value(&report, &source.support_stat_key(0, position, false)),
                Some(&EffectValue::Known { value: integer(1) })
            );
        }
        let assemblies: Vec<_> = effects(&report)
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key("ice-nova-final-level"))
            .collect();
        assert_eq!(
            assemblies.len(),
            1,
            "two Action alternatives share one exact source assembly"
        );
        assert!(matches!(&assemblies[0].key.invocation.origin,
            RuleOrigin::SourceProperty { owner, producer, position:None, .. }
            if owner.as_ref()==&source_inputs::SourceFixture::owner(0) && *producer==Fixture::provider(0)));
    }
}

#[test]
#[ignore = "requires both checked Ice publications; real raw inputs and final validation"]
fn authored_raw_corruption_quality_and_dense_validation_remain_separate() {
    let mut source = source_inputs::SourceFixture::load();
    source.raw(&[1, 39, 12, 12]);
    source.delta(0, -4.);
    source.delta(1, 5.);
    source.delta(2, 0.5);
    source.quality(3, 12.5);
    let plan = source.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for (copy, pre, final_level) in [(0, 1., 1), (1, 44., 40), (2, 12.5, 20), (3, 12., 12)] {
        assert!(
            matches!(value(&report,&source.stat_key(copy,"level")),Some(EffectValue::Known{value:ParameterValue::Quantity(q)}) if q.value()==pre)
        );
        assert_eq!(
            value(&report, &source.final_key(copy)),
            Some(&EffectValue::Known {
                value: integer(final_level)
            })
        );
        assert_numbers(&source.f, &report, copy, final_level);
    }
    assert!(
        matches!(value(&report,&source.stat_key(3,"quality")),Some(EffectValue::Known{value:ParameterValue::Quantity(q)}) if q.value()==12.5 && q.unit()==&source.f.quality_unit)
    );
    assert!(
        matches!(value(&report,&source.stat_key(2,"quality")),Some(EffectValue::Known{value:ParameterValue::Quantity(q)}) if q.value()==0.)
    );
    assert_eq!(
        source.f.build.gems[2].level, 12,
        "fractional corruption is not a rewritten raw level"
    );
}

#[test]
#[ignore = "requires both checked Ice publications; source occurrence and worker isolation"]
fn real_source_programs_preserve_duplicate_winners_copies_and_worker_reuse() {
    let mut a = source_inputs::SourceFixture::load();
    a.raw(&[17, 12]);
    a.supports(0, &[(5, 0.), (5, 15.)]);
    a.supports(1, &[(1, 0.)]);
    a.quality(1, 12.5);
    let plan_a = a.plan();
    let report_a = plan_a.evaluate(&mut plan_a.new_scratch()).unwrap();
    assert_numbers(&a.f, &report_a, 0, 20);
    assert_numbers(&a.f, &report_a, 1, 12);
    assert_eq!(
        value(&report_a, &a.stat_key(0, "count")),
        Some(&EffectValue::Known { value: integer(1) })
    );
    let properties: Vec<_> = effects(&report_a)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("exodus-source-level"))
        .collect();
    assert_eq!(properties.len(), 1);
    assert!(
        matches!(&properties[0].key.invocation.origin,RuleOrigin::SourceProperty{producer,owner,..}
        if producer.root==ProviderRoot::SupportAssignment(id(4001)) && owner.as_ref()==&source_inputs::SourceFixture::owner(0)),
        "prepared quality15 chooses the second exact source"
    );
    assert!(
        matches!(value(&report_a,&a.support_stat_key(0,1,true)),Some(EffectValue::Known{value:ParameterValue::Quantity(q)}) if q.value()==15.)
    );
    let mut b = a.clone();
    b.raw(&[11, 39]);
    b.supports(0, &[(5, 0.)]);
    b.supports(1, &[(5, 0.)]);
    let plan_b = b.plan();
    let report_b = plan_b.evaluate(&mut plan_b.new_scratch()).unwrap();
    assert_numbers(&b.f, &report_b, 0, 14);
    assert_numbers(&b.f, &report_b, 1, 40);
    let mut scratch = plan_a.new_scratch();
    for (plan, expected) in [
        (&plan_a, &report_a),
        (&plan_b, &report_b),
        (&plan_a, &report_a),
    ] {
        assert!(plan.evaluate(&mut scratch).unwrap() == *expected);
    }
    let mut zero = 0;
    assert!(matches!(
        plan_a.evaluate_with_budget(&mut scratch, &mut zero),
        Err(PlanError::Limit("work"))
    ));
    assert!(plan_a.evaluate(&mut scratch).unwrap() == report_a);
    let plans = [plan_a, plan_b];
    let expected = [report_a, report_b];
    let reports = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..24usize)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |scratch, n| (n % 2, plans[n % 2].evaluate(scratch).unwrap()),
                )
                .collect::<Vec<_>>()
        });
    for (i, report) in reports {
        assert!(report == expected[i]);
    }
}

#[test]
#[ignore = "requires both checked Ice publications; no raw/final or completeness defaults"]
fn real_source_inputs_keep_missing_raw_final_and_actual_owner_gaps() {
    let mut missing_quality = source_inputs::SourceFixture::load();
    missing_quality.raw(&[17]);
    missing_quality.f.build.gems[0].quality = None;
    let plan = missing_quality.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        matches!(
            value(&report, &missing_quality.stat_key(0, "quality")),
            Some(EffectValue::Unresolved {
                reason: PlanGapReason::MissingInput,
                ..
            })
        ),
        "missing selected quality must not be replaced with zero"
    );
    let mut incoming = source_inputs::SourceFixture::load();
    incoming.raw(&[17]);
    incoming.incomplete_ordinary_incoming();
    let plan = incoming.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialReceivers)
    );
    let mut missing = source_inputs::SourceFixture::load();
    missing.raw(&[17]);
    missing.remove_raw_delta(0);
    let error = match missing.f.try_plan_with_source(Some(&missing.config)) {
        Ok(_) => panic!("missing required raw corruption input was defaulted"),
        Err(error) => error,
    };
    assert!(
        matches!(error.downcast_ref::<PlanError>(), Some(PlanError::Invalid(message))
            if message == "owned request has invalid schema bindings"),
        "missing required raw delta must fail binding: {error}"
    );
    let mut missing = source_inputs::SourceFixture::load();
    missing.raw(&[17]);
    missing.remove_final();
    let plan = missing.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(value(&report, &missing.final_key(0)).is_none());
    for set in 0..2 {
        for stat in missing.f.b.channels.all() {
            assert!(matches!(
                action_value(&missing.f, &report, 0, set, stat),
                Some(EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                })
            ));
        }
    }
    let mut actual = source_inputs::SourceFixture::load();
    *actual
        .f
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actual.config.actual_gem_owner.owner)
        .unwrap() = actual.config.actual_gem_owner.clone();
    let error = match actual.f.try_plan_with_source(Some(&actual.config)) {
        Ok(_) => panic!("actual Partial owner became executable"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("complete owner programs"),
        "{error}"
    );
}
