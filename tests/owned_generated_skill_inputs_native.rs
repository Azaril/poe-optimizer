//! Published V19 permissions and unchanged provider programs in a finite component.
//! Raw quality and occurrence-count transport are not final quality, FullDPS,
//! reservation, legality or whole-build parity.
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;

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
use std::{fs, path::Path, sync::Arc};

type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
type Checked<T> = std::result::Result<T, String>;
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn id<T: BuildInstanceId>(value: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([109; 16]), value).unwrap())
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
#[derive(Clone, Deserialize)]
struct Family {
    skill: SkillDefId,
    passive: Option<PassiveNodeDefId>,
    modifier: Option<ModifierDefId>,
    modifier_level: Option<DeclaredSlot<ParameterSlotDefId>>,
    raw_level: DeclaredSlot<ParameterSlotDefId>,
    raw_quality: DeclaredSlot<ParameterSlotDefId>,
    supply: DeclaredSlot<SkillGrantSlotDefId>,
    grant: DeclaredSlot<GrantSlotDefId>,
}
impl Family {
    fn owner(&self) -> SchemaSubject {
        SchemaSubject::Definition(
            self.passive
                .as_ref()
                .map(|d| d.address())
                .unwrap_or_else(|| self.modifier.as_ref().unwrap().address()),
        )
    }
}
#[derive(Clone)]
struct Fixture {
    families: Vec<Family>,
    schema: SchemaPackageInput,
    rules: RulePackageInput,
    original_owners: Vec<DefinitionRules>,
    build: BuildInput,
    scenario: ScenarioInput,
    quality_unit: UnitDefId,
}
impl Fixture {
    fn def<T: DefinitionDomain>(&self, name: &str) -> DefId<T> {
        DefId::parse(self.schema.namespace.clone(), name).unwrap()
    }
    fn quality(&self, value: f64) -> ParameterValue {
        ParameterValue::Quantity(FiniteQuantity::new(value, self.quality_unit.clone()).unwrap())
    }
    fn load() -> Self {
        let output = std::path::PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_INPUT_OUTPUT")
                .expect("set to the verified generated-input publication parent"),
        );
        let endpoint = release::load(&output.join("package"));
        assert!(
            endpoint.input().evaluation.is_none(),
            "published mechanics remain Partial"
        );
        let recipe = &endpoint.input().recipe;
        assert_eq!(recipe.schema.schema_version, 6);
        assert_eq!(
            recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V19
        );
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/generated-preset-inputs-v1");
        let bindings: Value = read(directory.join("bindings.json"));
        let families: Vec<Family> = typed(&bindings["families"]);
        assert_eq!(families.len(), 3);
        let quality_unit: UnitDefId = typed(&bindings["quality_unit"]);
        let dependencies: Value = read(directory.join("dependencies.json"));
        let original_owners: Vec<DefinitionRules> = typed(&dependencies["owners"]);
        assert_eq!(original_owners.len(), 3);
        for (family, owner) in families.iter().zip(&original_owners) {
            assert_eq!(family.owner(), owner.owner);
            assert!(!owner.programs.is_complete());
            assert_eq!(owner.programs.members.len(), 1);
            assert!(
                recipe.rules.owners.contains(owner),
                "provider body and Partial coverage unchanged"
            );
        }
        let migration: OwnedReleaseMigrationInput = read(directory.join("migration.json"));
        for row in migration.schema {
            match row {
                SchemaExtensionEntry::Definition(row) => {
                    assert!(recipe.schema.definitions.contains(&row))
                }
                SchemaExtensionEntry::Slot(row) => assert!(recipe.schema.slots.contains(&row)),
            }
        }

        // Reuse the real selected Staff's destination and level roll. The source
        // join and complete original preservation are independently checked by CLI.
        let draft: Value = read(output.join("original-05/draft.json"));
        let staff_modifier = families[2].modifier.as_ref().unwrap();
        let (item, rolled) = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|item| {
                item["modifiers"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|m| m["definition"]["value"] == serde_json::json!(staff_modifier))
                    .map(|m| (item, m))
            })
            .unwrap();
        let equipment = draft["draft"]["equipment"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["item"]["value"] == item["id"])
            .unwrap();
        assert_eq!(equipment["destination"]["kind"], "character_slot");
        let destination: EquipmentSlotDefId = typed(&equipment["destination"]["value"]["value"]);
        let template: ItemTemplateDefId = typed(&item["template"]["value"]);
        let rolls: Vec<ParameterAssignment> = rolled["rolls"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                assert_eq!(row["slot"]["kind"], "known");
                assert_eq!(row["value"]["kind"], "known");
                ParameterAssignment {
                    slot: typed(&row["slot"]["value"]),
                    value: typed(&row["value"]["value"]),
                }
            })
            .collect();
        assert_eq!(
            rolls,
            vec![ParameterAssignment {
                slot: families[2].modifier_level.clone().unwrap(),
                value: ParameterValue::Integer(BoundedInteger::new(11).unwrap()),
            }]
        );
        let prior: Vec<DefinitionDescriptor> = typed(&dependencies["definitions"]);
        let mut definitions: Vec<_> = recipe
            .schema
            .definitions
            .iter()
            .filter(|row| {
                prior.iter().any(|p| p.address() == row.address())
                    && !matches!(row, DefinitionDescriptor::Gem(_))
            })
            .cloned()
            .collect();
        let pool = definitions
            .iter()
            .find_map(|row| match row {
                DefinitionDescriptor::PassiveNode(DefinitionEntry {
                    schema: SchemaState::Known(s),
                    ..
                }) => Some(s.pools.members[0].clone()),
                _ => None,
            })
            .unwrap();
        for address in [pool.address(), destination.address()] {
            definitions.push(
                recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == address)
                    .unwrap()
                    .clone(),
            );
        }
        // Explicit finite topology: only the three raw supplies exist here.
        // Remove Djinn descendants and unrelated item inputs only in this fixture;
        // preserve actual parameter schemas, supply permissions and rule bodies.
        for row in &mut definitions {
            match row {
                DefinitionDescriptor::PassiveNode(DefinitionEntry {
                    schema: SchemaState::Known(s),
                    ..
                }) => {
                    s.adjacent = DeclaredSet::complete(vec![]);
                    let (grants, supplies) = (
                        s.declarations.grants.members.clone(),
                        s.declarations.skill_grants.members.clone(),
                    );
                    s.declarations = ports();
                    s.declarations.grants.members = grants;
                    s.declarations.skill_grants.members = supplies;
                }
                DefinitionDescriptor::Skill(DefinitionEntry {
                    schema: SchemaState::Known(s),
                    ..
                }) => {
                    let parameters = s.declarations.parameters.members.clone();
                    assert!(!s.declarations.parameters.is_complete());
                    s.declarations = ports();
                    s.declarations.parameters.members = parameters;
                }
                DefinitionDescriptor::Modifier(DefinitionEntry {
                    schema: SchemaState::Known(s),
                    ..
                }) => {
                    s.declarations.outputs = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                    schema: SchemaState::Known(s),
                    ..
                }) => {
                    s.equipment_slots = DeclaredSet::complete(vec![destination.clone()]);
                    s.socket_destinations = DeclaredSet::complete(vec![]);
                    s.modifiers = DeclaredSet::complete(vec![staff_modifier.clone()]);
                    s.quality = QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    };
                    s.declarations = ports();
                }
                _ => {}
            }
        }
        let prior_slots: Vec<SlotDescriptor> = typed(&dependencies["slots"]);
        let mut slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|row| {
                prior_slots.iter().any(|p| p.address() == row.address())
                    || row.address() == SlotAddress::Parameter(families[2].raw_quality.clone())
            })
            .cloned()
            .collect();
        for row in &mut slots {
            if let SlotDescriptor::SkillGrant(DefinitionEntry {
                id,
                schema: SchemaState::Known(s),
            }) = row
            {
                let family = families.iter().find(|f| &f.supply == id).unwrap();
                let permission = s.preset_inputs.as_ref().unwrap();
                assert!(permission.parameters.is_complete());
                assert_eq!(
                    permission.parameters.members,
                    vec![family.raw_quality.clone()]
                );
                assert!(!s.outputs.is_complete() && s.outputs.members.is_empty());
                s.outputs.closure = SchemaClosure::Complete;
            }
        }
        let namespace = recipe.schema.namespace.clone();
        let class: ClassDefId = DefId::parse(namespace.clone(), "component-class").unwrap();
        let encounter: EncounterDefId =
            DefId::parse(namespace.clone(), "component-encounter").unwrap();
        let range = IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        };
        definitions.extend([
            DefinitionDescriptor::Class(known(
                class.clone(),
                ClassSchema {
                    level: range.clone(),
                    ascendancies: DeclaredSet::complete(vec![]),
                    implicit_passives: DeclaredSet::complete(vec![]),
                    declarations: ports(),
                },
            )),
            DefinitionDescriptor::Encounter(known(
                encounter.clone(),
                EncounterSchema {
                    enemy_level: range,
                    external_inputs: DeclaredSet::complete(vec![]),
                },
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
            (
                "execution-probe",
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
            schema_version: 6,
            namespace: namespace.clone(),
            release: key("unpublished-generated-input-component"),
            semantics_version: recipe.schema.semantics_version.clone(),
            definitions,
            slots,
        };
        let mut owners = original_owners.clone();
        for owner in &mut owners {
            owner.programs.closure = SchemaClosure::Complete;
        }
        for family in &families {
            owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(family.skill.address()),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key("constant-readiness-probe"),
                    context: RuleEntityKind::Skill,
                    reads: vec![],
                    nodes: vec![RuleNode {
                        id: key("yes"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(true),
                        },
                    }],
                    effects: vec![RuleEffect {
                        id: key("probe"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat: DefId::parse(namespace.clone(), "execution-probe").unwrap(),
                            value: key("yes"),
                        },
                    }],
                }]),
            });
            for slot in [
                SlotAddress::Grant(family.grant.clone()),
                SlotAddress::SkillGrant(family.supply.clone()),
            ] {
                owners.push(DefinitionRules {
                    owner: SchemaSubject::Slot(slot),
                    programs: DeclaredSet::complete(vec![]),
                });
            }
        }
        for address in [class.address(), encounter.address(), template.address()] {
            owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(address),
                programs: DeclaredSet::complete(vec![]),
            });
        }
        let rules = RulePackageInput {
            support_discovery: Some(SupportDiscoveryInput {
                providers: owners
                    .iter()
                    .map(|row| SupportSourceDomainDeclaration {
                        owner: row.owner.clone(),
                        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
                    })
                    .collect(),
            }),
            existing_actor_rules: None,
            contribution_queries: None,
            schema_version: recipe.rules.schema_version,
            namespace: namespace.clone(),
            release: schema.release.clone(),
            semantics_version: recipe.rules.semantics_version.clone(),
            operations_version: recipe.rules.operations_version.clone(),
            definitions: recipe.rules.definitions.clone(),
            tables: vec![],
            owners,
            receivers: DeclaredSet::complete(vec![]),
            effect_applications: Some(DeclaredSet::complete(vec![])),
        };
        let build = BuildInput {
            generated_inputs: None,
            allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([109; 16]), 100),
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
            items: vec![ItemRecord {
                id: id(30),
                template,
                parameters: vec![],
                item_level: None,
                quality: None,
                modifier_order: vec![id(31)],
                modifiers: vec![RolledModifier {
                    id: id(31),
                    definition: staff_modifier.clone(),
                    rolls,
                }],
            }],
            equipment: vec![EquipmentUse {
                id: id(32),
                item: id(30),
                destination: EquipmentDestination::CharacterSlot(destination),
                scope: LoadoutScope::Selected {
                    loadouts: vec![id(1)],
                },
            }],
            gems: vec![],
            allocations: families
                .iter()
                .take(2)
                .enumerate()
                .map(|(i, f)| Allocation {
                    id: id(10 + i as u64),
                    node: f.passive.clone().unwrap(),
                    pool: pool.clone(),
                    scope: LoadoutScope::Shared,
                    access: AllocationAccess::Ordinary,
                    choices: vec![],
                })
                .collect(),
            skills: vec![],
            supports: vec![],
            support_origins: Some(vec![]),
            payload_links: vec![],
            choices: vec![],
        };
        let scenario = ScenarioInput {
            game_version: namespace,
            enemy: EnemySpec {
                encounter,
                level: 20,
            },
            assumptions: vec![],
            usage: vec![],
        };
        let mut f = Self {
            families,
            schema,
            rules,
            original_owners,
            build,
            scenario,
            quality_unit,
        };
        f.build.generated_inputs = Some(GeneratedSkillInputsV1 {
            schema_version: 1,
            bindings: (0..3).map(|i| f.binding(i, 0.0)).collect(),
        });
        f
    }
    fn provider(&self, i: usize) -> ProviderKey {
        ProviderKey {
            root: if i < 2 {
                ProviderRoot::Allocation(id(10 + i as u64))
            } else {
                ProviderRoot::ItemModifier {
                    equipment_use: id(if i == 2 { 32 } else { 33 }),
                    modifier: id(31),
                }
            },
            grant_path: vec![],
        }
    }
    fn target(&self, i: usize) -> GeneratedSkillKey {
        GeneratedSkillKey {
            provider: self.provider(i),
            slot: self.families[i.min(2)].supply.clone(),
        }
    }
    fn binding(&self, i: usize, quality: f64) -> SelectedGeneratedSkillInput {
        SelectedGeneratedSkillInput {
            target: self.target(i),
            parameters: vec![ParameterAssignment {
                slot: self.families[i.min(2)].raw_quality.clone(),
                value: self.quality(quality),
            }],
            origin: GeneratedSkillInputOrigin {
                skill_preset: id(90),
            },
        }
    }
    fn parameter(&self, i: usize, quality: bool) -> PlanValueKey {
        let family = &self.families[i.min(2)];
        PlanValueKey::SkillParameter {
            skill: Box::new(self.target(i)),
            parameter: if quality {
                family.raw_quality.clone()
            } else {
                family.raw_level.clone()
            },
        }
    }
    fn probe(&self, i: usize) -> PlanValueKey {
        PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(Box::new(SkillTarget::Generated(Box::new(
                self.target(i),
            )))),
            stat: self.def("execution-probe"),
        }
    }
    fn request(&self) -> OwnedEvaluationRequest {
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), Default::default()).unwrap(),
            ScenarioSpec::new(self.scenario.clone(), Default::default()).unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: self.schema.namespace.clone(),
                    requests: vec![],
                },
                Default::default(),
            )
            .unwrap(),
            Default::default(),
        )
        .unwrap()
    }
    fn plan(&self) -> Checked<Plan> {
        let definitions = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.schema.clone(), Default::default())
                .map_err(|e| e.to_string())?,
        );
        let request = Arc::new(self.request());
        let binding = bind_owned_request(definitions.as_ref(), &request, Default::default())
            .map_err(|e| e.to_string())?;
        if binding.schema() == SchemaBindingStatus::Invalid {
            return Err(format!(
                "component request bindings: {:?}",
                binding.issues()
            ));
        }
        let mut input = self.rules.clone();
        input.definitions = definitions.identity().clone();
        let stored = OwnedRulePackage::new(input, definitions.as_ref(), Default::default())
            .map_err(|e| e.to_string())?;
        let rules = Arc::new(
            CompiledRulePackage::compile_stored(&stored, definitions.as_ref(), Default::default())
                .map_err(|e| e.to_string())?,
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: 1,
                    namespace: self.schema.namespace.clone(),
                    release: key("empty-routing"),
                    definitions: definitions.identity().clone(),
                    outputs: vec![],
                },
                definitions.as_ref(),
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let programs: Vec<_> = stored
            .input()
            .owners
            .iter()
            .flat_map(|o| o.programs.members.iter().map(move |p| (o, p)))
            .collect();
        let stages = Arc::new(
            OwnedEvaluationStages::new(
                EvaluationStagesInput {
                    schema_version: OWNED_EVALUATION_STAGES_V3,
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
                                stage: key(if p.context == RuleEntityKind::Skill {
                                    "execute"
                                } else {
                                    "prepare"
                                }),
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
                        skills: self
                            .families
                            .iter()
                            .map(|f| SkillReadiness {
                                participation: None,
                                skill: f.skill.clone(),
                                parameters: DeclaredSet::complete(
                                    [f.raw_level.clone(), f.raw_quality.clone()]
                                        .into_iter()
                                        .map(|parameter| ParameterReadiness {
                                            parameter,
                                            phase: ReadinessPhase::Execution,
                                        })
                                        .collect(),
                                ),
                            })
                            .collect(),
                        programs: DeclaredSet::complete(
                            programs
                                .iter()
                                .map(|(o, p)| {
                                    let family =
                                        self.families.iter().find(|f| f.owner() == o.owner);
                                    ReadinessProgram {
                                        owner: o.owner.clone(),
                                        program: p.id.clone(),
                                        phase: if family.is_some() {
                                            ReadinessPhase::Structural
                                        } else {
                                            ReadinessPhase::Execution
                                        },
                                        role: if family.is_some() {
                                            ReadinessProgramRole::FinalInputAssembly
                                        } else {
                                            ReadinessProgramRole::Execution
                                        },
                                        outputs: family
                                            .map(|f| {
                                                vec![
                                                    StageChannel::Grant {
                                                        slot: f.grant.clone(),
                                                    },
                                                    StageChannel::SkillParameter {
                                                        parameter: f.raw_level.clone(),
                                                    },
                                                ]
                                            })
                                            .unwrap_or_default(),
                                    }
                                })
                                .collect(),
                        ),
                    }),
                },
                definitions.as_ref(),
                &stored,
                &routing,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let preparation = Arc::new(
            OwnedSupportPreparation::new(
                SupportPreparationInput {
                    schema_version: 1,
                    namespace: self.schema.namespace.clone(),
                    release: key("empty-supports"),
                    definitions: definitions.identity().clone(),
                    rules: *stored.identity(),
                    policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                    quality_unit: self.quality_unit.clone(),
                    types: vec![],
                    effects: vec![],
                    families: vec![],
                    supports: vec![],
                },
                definitions.as_ref(),
                &stored,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let flag = || self.def("unused-flag");
        let optional = || OptionalTypeInputs {
            present: flag(),
            members: vec![],
        };
        let inputs = Arc::new(
            OwnedSupportInputBindings::new(
                SupportInputBindingsInput {
                    schema_version: 1,
                    namespace: self.schema.namespace.clone(),
                    release: key("empty-inputs"),
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
                definitions.as_ref(),
                &stored,
                &preparation,
                &stages,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let receiving = Arc::new(
            OwnedSupportReceiving::new(
                SupportReceivingInput {
                    // This transport component has readiness but no source-property relation.
                    schema_version: OWNED_SUPPORT_RECEIVING_V2,
                    source_properties: None,
                    namespace: self.schema.namespace.clone(),
                    release: key("empty-receiving"),
                    definitions: definitions.identity().clone(),
                    rules: *stored.identity(),
                    preparation: *preparation.identity(),
                    inputs: *inputs.identity(),
                    stages: *stages.identity(),
                    roles: vec![],
                    targets: vec![],
                    supports: vec![],
                },
                definitions.as_ref(),
                &stored,
                &preparation,
                &inputs,
                &stages,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        Plan::compile(
            SupportEffectPlanInputs {
                request,
                definitions,
                rules,
                routing,
                stages,
                preparation,
                inputs,
                receiving,
            },
            Default::default(),
            Default::default(),
        )
        .map_err(|e| e.to_string())
    }
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("{report:?}")
    };
    effects
}
fn value<'a>(report: &'a SupportEffectsReport, key: &PlanValueKey) -> Option<&'a EffectValue> {
    let matches: Vec<_> = effects(report)
        .values
        .iter()
        .filter(|v| &v.key == key)
        .collect();
    assert!(matches.len() <= 1);
    matches.first().map(|v| &v.value)
}
fn known_value(value: ParameterValue) -> EffectValue {
    EffectValue::Known { value }
}

/// The published count channel is reused unchanged. This fixture supplies only
/// unrelated topology/readiness closure; it supplies no reservation/DPS formula.
struct RequestedCount {
    policy: UsagePolicyDefId,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    stat: StatDefId,
    unit: UnitDefId,
    program: OwnedDefinitionKey,
}
impl RequestedCount {
    fn load(f: &mut Fixture) -> Self {
        let output = std::path::PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_INPUT_OUTPUT").unwrap(),
        );
        let endpoint = release::load(&output.join("package"));
        let recipe = &endpoint.input().recipe;
        let policy: UsagePolicyDefId = f.def("def.000000000000326a");
        let stat: StatDefId = f.def("def.000000000000326c");
        let descriptor = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == policy.address())
            .unwrap();
        let DefinitionDescriptor::UsagePolicy(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = descriptor
        else {
            panic!("published requested-count policy must have a known schema")
        };
        assert!(schema.declarations.parameters.is_complete());
        assert_eq!(schema.declarations.parameters.members.len(), 1);
        let parameter = schema.declarations.parameters.members[0].clone();
        assert_eq!(parameter.slot, f.def("def.000000000000326b"));
        let parameter_descriptor = recipe
            .schema
            .slots
            .iter()
            .find(|d| d.address() == SlotAddress::Parameter(parameter.clone()))
            .unwrap();
        let stat_descriptor = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == stat.address())
            .unwrap();
        let DefinitionDescriptor::Stat(DefinitionEntry {
            schema:
                SchemaState::Known(StatSchema {
                    value: ComputedValueType::Quantity { unit },
                    ..
                }),
            ..
        }) = stat_descriptor
        else {
            panic!("published requested count must retain its quantity unit")
        };
        assert!(
            f.schema
                .definitions
                .iter()
                .any(|d| d.address() == unit.address())
        );
        let owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(policy.address()))
            .unwrap();
        assert!(owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        assert_eq!(owner.programs.members[0].context, RuleEntityKind::Skill);
        let program = owner.programs.members[0].id.clone();
        for descriptor in [descriptor, stat_descriptor] {
            assert!(
                f.schema
                    .definitions
                    .iter()
                    .all(|d| d.address() != descriptor.address())
            );
            f.schema.definitions.push(descriptor.clone());
        }
        f.schema.slots.push(parameter_descriptor.clone());
        f.rules.owners.push(owner.clone());
        // These equality checks intentionally include the complete original
        // declarations, integer bounds, program reads/nodes/effects and coverage.
        assert_eq!(f.schema.slots.last().unwrap(), parameter_descriptor);
        assert_eq!(f.rules.owners.last().unwrap(), owner);
        Self {
            policy,
            parameter,
            stat,
            unit: unit.clone(),
            program,
        }
    }
    fn selection(&self, target: SkillTarget, count: i64) -> UsagePolicySelection {
        UsagePolicySelection {
            policy: self.policy.clone(),
            target: UsageTarget::Skill(target),
            parameters: vec![ParameterAssignment {
                slot: self.parameter.clone(),
                value: ParameterValue::Integer(BoundedInteger::new(count).unwrap()),
            }],
        }
    }
    fn key(&self, target: &SkillTarget) -> PlanValueKey {
        PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(Box::new(target.clone())),
            stat: self.stat.clone(),
        }
    }
    fn assert_counts(
        &self,
        request: &OwnedEvaluationRequest,
        report: &SupportEffectsReport,
        targets: &[SkillTarget],
        counts: &[i64],
    ) {
        assert_eq!(targets.len(), counts.len());
        for (target, count) in targets.iter().zip(counts) {
            // ScenarioSpec canonicalizes usage by exact target/policy. The
            // engine's origin indexes that normalized request, not authored order.
            let selection = self.selection(target.clone(), *count);
            let indexes: Vec<_> = request
                .scenario()
                .input()
                .usage
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (row == &selection).then_some(index))
                .collect();
            assert_eq!(indexes.len(), 1, "one exact normalized usage selection");
            let index = indexes[0];
            let expected = known_value(ParameterValue::Quantity(
                FiniteQuantity::new(*count as f64, self.unit.clone()).unwrap(),
            ));
            assert_eq!(value(report, &self.key(target)), Some(&expected));
            let rows: Vec<_> = effects(report)
                .effects
                .iter()
                .filter(|row| {
                    row.key.invocation.program == self.program
                        && row.key.invocation.origin == RuleOrigin::Usage { index }
                })
                .collect();
            assert_eq!(
                rows.len(),
                1,
                "one count producer for each exact usage target"
            );
            assert_eq!(rows[0].value, expected);
        }
        assert_eq!(
            effects(report)
                .effects
                .iter()
                .filter(|row| row.key.invocation.program == self.program)
                .count(),
            targets.len(),
            "the usage policy cannot fan out to other copies of a Skill definition"
        );
    }
}

#[test]
#[ignore = "requires the verified generated-input publication"]
fn published_occurrence_counts_keep_direct_tree_and_item_targets_independent() {
    let mut f = Fixture::load();
    let count = RequestedCount::load(&mut f);
    // A Direct use of the same Skill definition as the first Tree supply must
    // retain its own count. Its actual raw parameter schemas permit both forms.
    f.build.skills.push(SkillUse {
        id: id(40),
        source: AuthoredSkillSource::Direct(f.families[0].skill.clone()),
        enabled: true,
        scope: LoadoutScope::Shared,
        parameters: Some(vec![
            ParameterAssignment {
                slot: f.families[0].raw_level.clone(),
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(1.0, count.unit.clone()).unwrap(),
                ),
            },
            ParameterAssignment {
                slot: f.families[0].raw_quality.clone(),
                value: f.quality(0.0),
            },
        ]),
    });
    let targets = vec![
        SkillTarget::Authored(id(40)),
        SkillTarget::Generated(Box::new(f.target(0))),
        SkillTarget::Generated(Box::new(f.target(1))),
        SkillTarget::Generated(Box::new(f.target(2))),
    ];
    let cases = [[0, 1, 3, 4], [4, 0, 1, 3], [3, 4, 0, 1], [1, 3, 4, 0]];
    let mut checked = Vec::new();
    for counts in cases {
        let mut case = f.clone();
        case.scenario.usage = targets
            .iter()
            .cloned()
            .zip(counts)
            .map(|(target, value)| count.selection(target, value))
            .collect();
        let plan = case.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        count.assert_counts(plan.request(), &report, &targets, &counts);
        assert_transport(&case, &report, &[0.0; 3]);
        checked.push((plan, report));
    }

    // Removing an explicit count supplies no implicit one and cannot change
    // another source. Raw provider levels/quality remain independently produced.
    let mut omitted = f.clone();
    omitted.scenario.usage = targets[..3]
        .iter()
        .cloned()
        .zip([0, 1, 3])
        .map(|(target, value)| count.selection(target, value))
        .collect();
    let plan = omitted.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    count.assert_counts(plan.request(), &report, &targets[..3], &[0, 1, 3]);
    assert_eq!(value(&report, &count.key(&targets[3])), None);
    assert_transport(&omitted, &report, &[0.0; 3]);

    let mut invalid = f.clone();
    invalid.scenario.usage = vec![count.selection(targets[0].clone(), 5)];
    assert!(
        invalid
            .plan()
            .err()
            .expect("out-of-domain count must reject")
            .contains("component request bindings"),
        "the published 0..4 input domain remains enforced"
    );

    let mut scratch = checked[0].0.new_scratch();
    for index in [0, 1, 0, 2, 3, 0] {
        let (plan, expected) = &checked[index];
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), expected);
    }
    let mut budget = 0;
    assert!(matches!(
        checked[0].0.evaluate_with_budget(&mut scratch, &mut budget),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(checked[0].0.evaluate(&mut scratch).unwrap(), checked[0].1);
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..32usize)
                .into_par_iter()
                .map_init(
                    || checked[0].0.new_scratch(),
                    |scratch, index| {
                        let (plan, expected) = &checked[index % checked.len()];
                        assert_eq!(&plan.evaluate(scratch).unwrap(), expected);
                    },
                )
                .count()
        });
}

fn assert_transport(f: &Fixture, report: &SupportEffectsReport, qualities: &[f64]) {
    for (i, quality) in qualities.iter().copied().enumerate() {
        let level = if i < 2 {
            ParameterValue::Quantity(
                FiniteQuantity::new(1.0, f.def("def.000000000000295a")).unwrap(),
            )
        } else {
            ParameterValue::Integer(BoundedInteger::new(11).unwrap())
        };
        assert_eq!(
            value(report, &f.parameter(i, false)),
            Some(&known_value(level))
        );
        assert_eq!(
            value(report, &f.parameter(i, true)),
            Some(&known_value(f.quality(quality)))
        );
        assert_eq!(
            value(report, &f.probe(i)),
            Some(&known_value(ParameterValue::Boolean(true)))
        );
        assert_eq!(
            value(
                report,
                &PlanValueKey::Grant {
                    provider: f.provider(i),
                    slot: f.families[i.min(2)].grant.clone()
                }
            ),
            Some(&known_value(ParameterValue::Boolean(true)))
        );
        let supplied: Vec<_> = effects(report).effects.iter().filter(|row| matches!(&row.key.invocation.origin, RuleOrigin::GeneratedInput { target, .. } if **target == f.target(i))).collect();
        assert_eq!(supplied.len(), 1);
        assert_eq!(supplied[0].value, known_value(f.quality(quality)));
        let RuleOrigin::GeneratedInput { origin, .. } = &supplied[0].key.invocation.origin else {
            unreachable!()
        };
        assert_eq!(origin.skill_preset, id(90));
        let owner = &f.original_owners[i.min(2)];
        let provider: Vec<_> = effects(report)
            .effects
            .iter()
            .filter(|row| {
                row.key.invocation.program == owner.programs.members[0].id
                    && row.key.invocation.origin
                        == RuleOrigin::Provider {
                            provider: f.provider(i),
                        }
            })
            .collect();
        assert_eq!(
            provider.len(),
            2,
            "unchanged level projection and activation retain provider provenance"
        );
    }
}

#[test]
#[ignore = "requires the verified generated-input publication"]
fn published_permissions_transport_raw_quality_without_overriding_provider_levels() {
    let f = Fixture::load();
    let plan = f.plan().unwrap();
    assert_transport(
        &f,
        &plan.evaluate(&mut plan.new_scratch()).unwrap(),
        &[0.0; 3],
    );
    let mut fractional = f.clone();
    for row in &mut fractional.build.generated_inputs.as_mut().unwrap().bindings {
        row.parameters[0].value = f.quality(12.5);
    }
    let plan = fractional.plan().unwrap();
    assert_transport(
        &fractional,
        &plan.evaluate(&mut plan.new_scratch()).unwrap(),
        &[12.5; 3],
    );
    assert!(
        f.build.gems.is_empty() && f.build.skills.is_empty(),
        "no replacement authored roots"
    );
}

#[test]
#[ignore = "requires the verified generated-input publication"]
fn repeated_item_provider_uses_keep_distinct_inputs_across_scratch_and_workers() {
    let mut f = Fixture::load();
    // One rolled record used twice is two exact providers. Physical availability
    // and simultaneous equipment legality are outside this transport component.
    let mut second = f.build.equipment[0].clone();
    second.id = id(33);
    let destination = f.def("component-second-destination");
    f.schema
        .definitions
        .push(DefinitionDescriptor::EquipmentSlot(known(
            destination.clone(),
            EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            },
        )));
    let template = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => Some(s),
            _ => None,
        })
        .unwrap();
    template.equipment_slots.members.push(destination.clone());
    second.destination = EquipmentDestination::CharacterSlot(destination);
    f.build.equipment.push(second);
    let extra = f.binding(3, 12.5);
    f.build
        .generated_inputs
        .as_mut()
        .unwrap()
        .bindings
        .push(extra);
    let a = f.plan().unwrap();
    let ra = a.evaluate(&mut a.new_scratch()).unwrap();
    assert_transport(&f, &ra, &[0.0, 0.0, 0.0, 12.5]);
    let mut changed = f.clone();
    changed.build.generated_inputs.as_mut().unwrap().bindings[2].parameters[0].value =
        f.quality(12.5);
    changed.build.generated_inputs.as_mut().unwrap().bindings[3].parameters[0].value =
        f.quality(0.0);
    let b = changed.plan().unwrap();
    let rb = b.evaluate(&mut b.new_scratch()).unwrap();
    assert_transport(&changed, &rb, &[0.0, 0.0, 12.5, 0.0]);
    let mut scratch = a.new_scratch();
    for (plan, expected) in [(&a, &ra), (&b, &rb), (&a, &ra)] {
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), expected);
    }
    let mut budget = 0;
    assert!(matches!(
        a.evaluate_with_budget(&mut scratch, &mut budget),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(a.evaluate(&mut scratch).unwrap(), ra);
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..32usize)
                .into_par_iter()
                .map_init(
                    || a.new_scratch(),
                    |scratch, i| {
                        let (plan, expected) = if i % 2 == 0 { (&a, &ra) } else { (&b, &rb) };
                        assert_eq!(&plan.evaluate(scratch).unwrap(), expected);
                    },
                )
                .count()
        });
}

#[test]
#[ignore = "requires the verified generated-input publication"]
fn missing_quality_forbidden_level_and_actual_partial_owners_keep_their_gates() {
    let f = Fixture::load();
    for i in 0..3 {
        let mut missing = f.clone();
        missing
            .build
            .generated_inputs
            .as_mut()
            .unwrap()
            .bindings
            .remove(i);
        let plan = missing.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(value(&report, &f.parameter(i, true)), None);
        assert_eq!(
            value(&report, &f.probe(i)),
            Some(&EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: None
            })
        );
        let mut invalid = f.clone();
        invalid.build.generated_inputs.as_mut().unwrap().bindings[i].parameters[0].slot =
            f.families[i].raw_level.clone();
        invalid.build.generated_inputs.as_mut().unwrap().bindings[i].parameters[0].value = if i < 2
        {
            ParameterValue::Quantity(
                FiniteQuantity::new(1.0, f.def("def.000000000000295a")).unwrap(),
            )
        } else {
            ParameterValue::Integer(BoundedInteger::new(11).unwrap())
        };
        let definitions =
            OwnedDefinitionSchemaPackage::new(invalid.schema.clone(), Default::default()).unwrap();
        assert_eq!(
            bind_owned_request(&definitions, &invalid.request(), Default::default())
                .unwrap()
                .schema(),
            SchemaBindingStatus::Invalid,
            "quality-only permission cannot replace a provider-owned level"
        );
        let mut partial = f.clone();
        partial
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == f.original_owners[i].owner)
            .unwrap()
            .programs
            .closure = f.original_owners[i].programs.closure.clone();
        let error = partial
            .plan()
            .err()
            .expect("actual Partial owner remains unavailable for early readiness");
        assert!(error.contains("complete owner programs"), "{error}");
    }
}
