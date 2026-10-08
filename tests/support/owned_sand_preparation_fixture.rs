//! Finite component world: actual Sand rules, slots and table; no build-completeness claim.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_readiness::*, owned_routing::*,
    owned_rules::*, owned_schema::*, owned_source_properties::*, owned_stages::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_mapping::RegistryInput,
    owned_recipe::OwnedRecipeInput,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

#[path = "owned_sand_preparation_runtime.rs"]
mod runtime;
pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
pub fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
pub fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
pub fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
pub fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([0x6b; 16]), n).unwrap())
}
pub fn slot<K: DefinitionDomain>(n: u64, owner: u64) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def(owner)),
        slot: def(n),
    }
}
pub fn quantity(n: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
pub fn integer(n: i64) -> BoundedInteger {
    BoundedInteger::new(n).unwrap()
}
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/sand-preparation")
                .join(file),
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
pub fn manual() -> SkillTarget {
    SkillTarget::Authored(occurrence(3))
}
pub fn tree() -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::Allocation(occurrence(2)),
            grant_path: vec![],
        },
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::PassiveNode(def(0xb34)),
            slot: def(0x32d2),
        },
    }))
}
pub fn provider(target: &SkillTarget) -> ProviderKey {
    match target {
        SkillTarget::Authored(id) => ProviderKey {
            root: ProviderRoot::SkillUse(*id),
            grant_path: vec![],
        },
        SkillTarget::Generated(g) => {
            let mut p = g.provider.clone();
            p.grant_path.push(DeclaredSlot {
                declaration: SlotOwnerDefId::PassiveNode(def(0xb34)),
                slot: def(0x32d3),
            });
            p
        }
    }
}
pub fn command(target: &SkillTarget) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: provider(target),
        slot: slot(0x32a5, 0x322),
    }))
}
pub fn command_parameter(target: &SkillTarget, n: u64) -> PlanValueKey {
    let SkillTarget::Generated(skill) = command(target) else {
        unreachable!()
    };
    PlanValueKey::SkillParameter {
        skill,
        parameter: slot(n, 0xd8),
    }
}
pub fn actor(target: &SkillTarget) -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: provider(target),
        slot: slot(0x32a3, 0x322),
    }))
}
pub fn stat_key(target: SkillTarget, n: u64) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Skill(Box::new(target)),
        stat: def(n),
    }
}
pub fn actor_level(target: &SkillTarget) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Actor(actor(target)),
        stat: def(0x1c),
    }
}

pub struct World {
    pub recipe: OwnedRecipeInput,
    pub build: BuildInput,
    pub scenario: ScenarioInput,
    pub relations: SourcePropertyPreparationInput,
}
impl World {
    pub fn new(raw: f64, quality: f64, tree_quality: f64, ordinary: f64) -> Self {
        let extension: OwnedRecipeExtension = read("extension.json");
        let dependencies: Value = read("dependencies.json");
        let mut definitions: Vec<DefinitionDescriptor> =
            serde_json::from_value(dependencies["definitions"].clone()).unwrap();
        let mut slots: Vec<SlotDescriptor> =
            serde_json::from_value(dependencies["slots"].clone()).unwrap();
        // Explicit source-proven grant-role corrections use the existing offline
        // release migration path; never hide them in the fixture's closure step.
        for correction in read::<Vec<SlotDescriptor>>("corrections.json") {
            let prior = slots
                .iter_mut()
                .find(|s| s.address() == correction.address())
                .unwrap();
            *prior = correction;
        }
        for entry in extension.schema {
            match entry {
                SchemaExtensionEntry::Definition(d) => {
                    definitions.retain(|old| old.address() != d.address());
                    definitions.push(d)
                }
                SchemaExtensionEntry::Slot(s) => slots.push(s),
            }
        }
        // This finite world contains the two root forms, their Command and their
        // population. It deliberately excludes actor attack/damage execution.
        for d in &mut definitions {
            match d {
                DefinitionDescriptor::Skill(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        unreachable!()
                    };
                    s.declarations.outputs = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::Actor(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        unreachable!()
                    };
                    s.declarations = ports();
                }
                DefinitionDescriptor::PassiveNode(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        unreachable!()
                    };
                    s.adjacent = DeclaredSet::complete(vec![]);
                    s.pools = DeclaredSet::complete(vec![def(0x1bf1)]);
                }
                DefinitionDescriptor::PointPool(row) => {
                    row.schema = SchemaState::Known(PointPoolSchema {
                        scope: PointPoolScope::Shared,
                    });
                }
                _ => {}
            }
        }
        for s in &mut slots {
            match s {
                SlotDescriptor::Actor(row) => {
                    let SchemaState::Known(v) = &mut row.schema else {
                        unreachable!()
                    };
                    v.skills = DeclaredSet::complete(vec![]);
                    v.outputs = DeclaredSet::complete(vec![]);
                }
                SlotDescriptor::SkillGrant(row) => {
                    let SchemaState::Known(v) = &mut row.schema else {
                        unreachable!()
                    };
                    v.outputs = DeclaredSet::complete(vec![]);
                }
                _ => {}
            }
        }
        // Close only the explicitly enumerated finite fixture membership. Authored
        // packet data remains Partial and cannot be installed as a checked sidecar.
        fn close(value: &mut Value) {
            match value {
                Value::Object(map) => {
                    if map.contains_key("members") && map.contains_key("closure") {
                        map.insert("closure".into(), serde_json::json!({"kind":"complete"}));
                    }
                    for v in map.values_mut() {
                        close(v)
                    }
                }
                Value::Array(a) => {
                    for v in a {
                        close(v)
                    }
                }
                _ => {}
            }
        }
        let mut finite = serde_json::to_value((&definitions, &slots)).unwrap();
        close(&mut finite);
        (definitions, slots) = serde_json::from_value(finite).unwrap();
        definitions.extend([
            DefinitionDescriptor::Class(known(
                def(0xf1001),
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
                def(0xf1002),
                EncounterSchema {
                    enemy_level: IntegerRange {
                        minimum: integer(1),
                        maximum: integer(100),
                    },
                    external_inputs: DeclaredSet::complete(vec![]),
                },
            )),
        ]);
        let schema = SchemaPackageInput {
            schema_version: 6,
            namespace: ns(),
            release: key("finite-sand-preparation"),
            semantics_version: key("owned-mechanics-skill-inputs-v1"),
            definitions,
            slots,
        };
        let checked =
            OwnedDefinitionSchemaPackage::new(schema.clone(), Default::default()).unwrap();
        let mut owners: Vec<DefinitionRules> =
            serde_json::from_value(dependencies["owners"].clone()).unwrap();
        for appended in extension.owners {
            owners
                .iter_mut()
                .find(|o| o.owner == appended.owner)
                .unwrap()
                .programs
                .members
                .extend(appended.programs.members);
        }
        for o in &mut owners {
            o.programs.closure = SchemaClosure::Complete;
        }
        for d in &schema.definitions {
            let owner = SchemaSubject::Definition(d.address());
            if !owners.iter().any(|o| o.owner == owner) {
                owners.push(DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(vec![]),
                })
            }
        }
        // The finite population has no attack programs. Actor-slot coverage is
        // independent of its Actor definition; neither is closed in production.
        owners.push(DefinitionRules {
            owner: SchemaSubject::Slot(SlotAddress::Actor(slot(0x32a3, 0x322))),
            programs: DeclaredSet::complete(vec![]),
        });
        owners
            .iter_mut()
            .find(|o| o.owner == subject(def::<ClassDefinition>(0xf1001)))
            .unwrap()
            .programs
            .members
            .push(RuleProgram {
                id: key("finite-ordinary-level"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![RuleNode {
                    id: key("amount"),
                    expression: RuleExpression::Literal {
                        value: quantity(ordinary, 0x295a),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("amount"),
                    when: None,
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Player,
                        stat: def(0x30ab),
                        contribution: ContributionKind::Add,
                        value: key("amount"),
                    },
                }],
            });
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
                release: key("finite-sand-preparation"),
                semantics_version: key("finite-sand-preparation"),
                operations_version: key(OWNED_RULE_OPERATIONS_V22),
                definitions: checked.identity().clone(),
                tables: serde_json::from_value(dependencies["tables"].clone()).unwrap(),
                owners,
                receivers: DeclaredSet::complete(vec![]),
                effect_applications: Some(DeclaredSet::complete(vec![])),
                existing_actor_rules: None,
                contribution_queries: Some(DeclaredSet::complete(vec![])),
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("finite-sand-preparation"),
                definitions: checked.identity().clone(),
                outputs: vec![],
            },
        };
        let SkillTarget::Generated(generated) = tree() else {
            unreachable!()
        };
        let build = BuildInput {
            allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([0x6b; 16]), 4),
            revision: BuildRevision::from_u64(1),
            game_version: ns(),
            character: CharacterSpec {
                class: def(0xf1001),
                ascendancy: None,
                level: 95,
                rewards: vec![],
            },
            weapon_loadouts: vec![occurrence(1)],
            active_weapon_loadout: occurrence(1),
            items: vec![],
            gems: vec![],
            equipment: vec![],
            allocations: vec![Allocation {
                id: occurrence(2),
                node: def(0xb34),
                pool: def(0x1bf1),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            }],
            skills: vec![SkillUse {
                id: occurrence(3),
                source: AuthoredSkillSource::Direct(def(0x322)),
                enabled: true,
                scope: LoadoutScope::Shared,
                parameters: Some(vec![
                    ParameterAssignment {
                        slot: slot(0x3261, 0x322),
                        value: quantity(raw, 0x295a),
                    },
                    ParameterAssignment {
                        slot: slot(0x3262, 0x322),
                        value: quantity(quality, 2),
                    },
                ]),
            }],
            supports: vec![],
            support_origins: Some(vec![
                SupportOriginSequence {
                    target: manual(),
                    origins: vec![],
                },
                SupportOriginSequence {
                    target: tree(),
                    origins: vec![],
                },
            ]),
            generated_inputs: Some(GeneratedSkillInputsV1 {
                schema_version: 1,
                bindings: vec![SelectedGeneratedSkillInput {
                    target: *generated,
                    parameters: vec![ParameterAssignment {
                        slot: slot(0x3262, 0x322),
                        value: quantity(tree_quality, 2),
                    }],
                    origin: GeneratedSkillInputOrigin {
                        skill_preset: occurrence(4),
                    },
                }],
            }),
            payload_links: vec![],
            choices: vec![],
        };
        let scenario = ScenarioInput {
            game_version: ns(),
            enemy: EnemySpec {
                encounter: def(0xf1002),
                level: 82,
            },
            assumptions: vec![],
            usage: vec![],
        };
        let mut relations: Value = read("source-properties.json");
        close(&mut relations);
        Self {
            recipe,
            build,
            scenario,
            relations: serde_json::from_value(relations).unwrap(),
        }
    }
    /// A finite counterfactual, not a claimed real supported-property producer.
    pub fn add_supported_properties(&mut self, level: f64, quality: f64) {
        let owner = subject(def::<ClassDefinition>(0xf1001));
        let program = key("finite-source-properties");
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == owner)
            .unwrap()
            .programs
            .members
            .push(RuleProgram {
                id: program.clone(),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![
                    RuleNode {
                        id: key("level"),
                        expression: RuleExpression::Literal {
                            value: quantity(level, 0x295a),
                        },
                    },
                    RuleNode {
                        id: key("quality"),
                        expression: RuleExpression::Literal {
                            value: quantity(quality, 2),
                        },
                    },
                ],
                effects: [("level", 0x32e1), ("quality", 0x3352)]
                    .into_iter()
                    .map(|(name, stat)| RuleEffect {
                        id: key(name),
                        when: None,
                        effect: RuleEffectKind::Contribute {
                            entity: RuleEntity::PropertyOwner,
                            stat: def(stat),
                            contribution: ContributionKind::Add,
                            value: key(name),
                        },
                    })
                    .collect(),
            });
        for relation in &mut self.relations.relations.members {
            relation
                .external
                .members
                .push(SourcePropertyExternalProgram {
                    owner: owner.clone(),
                    program: program.clone(),
                });
        }
    }
    pub fn compile(
        &self,
    ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, String> {
        runtime::compile(self)
    }
    pub fn compile_configured(
        &self,
        configure: impl FnOnce(&mut EvaluationStagesInput),
    ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, String> {
        runtime::compile_configured(self, configure)
    }
}

pub fn classification(program: &RuleProgram) -> (ReadinessProgramRole, &'static str) {
    match program.id.as_str() {
        "finite-source-properties" => (
            ReadinessProgramRole::SourceExternalProperty,
            "source-properties",
        ),
        "sand-final-preparation" => (
            ReadinessProgramRole::SourceFinalInputAssembly,
            "source-assembly",
        ),
        "sand-command-preparation" => (ReadinessProgramRole::FinalInputAssembly, "descendants"),
        "sand-tree-raw-supply" => (ReadinessProgramRole::FinalInputAssembly, "prepare"),
        "sand-population-level" => (ReadinessProgramRole::PreparationFacts, "descendants"),
        _ => (ReadinessProgramRole::PreparationFacts, "prepare"),
    }
}
pub fn outputs(program: &RuleProgram) -> Vec<StageChannel> {
    let scope = |entity| match entity {
        RuleEntity::Current => program.context,
        RuleEntity::Player | RuleEntity::Actor => RuleEntityKind::Actor,
        RuleEntity::Modifier => RuleEntityKind::Modifier,
        RuleEntity::PropertyOwner => RuleEntityKind::Skill,
        _ => panic!("unreviewed finite preparation destination"),
    };
    program
        .effects
        .iter()
        .map(|effect| match &effect.effect {
            RuleEffectKind::Derive { entity, stat, .. } => StageChannel::Stat {
                scope: scope(*entity),
                stat: stat.clone(),
            },
            RuleEffectKind::ProjectActorStat { stat, .. } => StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: stat.clone(),
            },
            RuleEffectKind::Contribute {
                stat,
                contribution,
                entity,
                ..
            } => StageChannel::Contributions {
                scope: scope(*entity),
                stat: stat.clone(),
                contribution: *contribution,
            },
            RuleEffectKind::ActivateGrant { slot, .. } => {
                StageChannel::Grant { slot: slot.clone() }
            }
            RuleEffectKind::ProjectSkillParameter { parameter, .. } => {
                StageChannel::SkillParameter {
                    parameter: parameter.clone(),
                }
            }
            _ => panic!("unexpected finite preparation effect"),
        })
        .collect()
}
pub fn evaluated(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("{report:?}")
    };
    effects
}
pub fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> Option<&'a EffectValue> {
    report
        .values
        .iter()
        .find(|r| &r.key == key)
        .map(|r| &r.value)
}
