//! Actual published allocation producers in an explicitly finite V17 fixture.
//! Raw level transport is observable; missing quality still gates execution.
//! The synthetic constant consumer proves readiness, not Djinn numerical parity.
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
use poe_optimizer_import::owned_release_migration::OwnedReleaseMigrationInput;
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::Path, sync::Arc};

type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
fn read<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn id<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([92; 16]), n).unwrap())
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
#[derive(Clone, Deserialize)]
struct Family {
    passive: PassiveNodeDefId,
    skill: SkillDefId,
    raw_level: DeclaredSlot<ParameterSlotDefId>,
    raw_quality: DeclaredSlot<ParameterSlotDefId>,
    supply: DeclaredSlot<SkillGrantSlotDefId>,
    grant: DeclaredSlot<GrantSlotDefId>,
    program: OwnedDefinitionKey,
}
#[derive(Clone)]
struct Fixture {
    families: Vec<Family>,
    schema: SchemaPackageInput,
    rules: RulePackageInput,
    build: BuildInput,
    scenario: ScenarioInput,
    unit: UnitDefId,
    quality_unit: UnitDefId,
    published_closure: SchemaClosure,
}
impl Fixture {
    fn def<T: DefinitionDomain>(&self, name: &str) -> DefId<T> {
        DefId::parse(self.schema.namespace.clone(), name).unwrap()
    }
    fn quantity(&self, amount: f64) -> ParameterValue {
        ParameterValue::Quantity(FiniteQuantity::new(amount, self.unit.clone()).unwrap())
    }
    fn load() -> Self {
        let output = std::path::PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_DJINN_TREE_OUTPUT")
                .expect("set to the verified Djinn tree-grant publication parent"),
        );
        let endpoint = release::load(&output.join("package"));
        assert!(
            endpoint.input().evaluation.is_none(),
            "actual package is still data-only"
        );
        let packet = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/djinn-tree-grants");
        let bindings: Value = read(packet.join("bindings.json"));
        let families: Vec<Family> = serde_json::from_value(bindings["families"].clone()).unwrap();
        assert_eq!(families.len(), 2);
        let migration: OwnedReleaseMigrationInput = read(packet.join("migration.json"));
        let dependencies: Value = read(packet.join("dependencies.json"));
        let recipe = &endpoint.input().recipe;
        assert_eq!(recipe.schema.schema_version, 5);
        assert_eq!(
            recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V17
        );
        let prior_definitions: Vec<DefinitionDescriptor> =
            serde_json::from_value(dependencies["definitions"].clone()).unwrap();
        let raw_slots: Vec<SlotDescriptor> =
            serde_json::from_value(dependencies["slots"].clone()).unwrap();
        for row in &raw_slots {
            assert!(
                recipe.schema.slots.contains(row),
                "actual raw slot authority preserved"
            );
            let SlotDescriptor::Parameter(row) = row else {
                panic!("raw parameter")
            };
            let SchemaState::Known(schema) = &row.schema else {
                panic!("known raw parameter")
            };
            assert_eq!(schema.presence, SlotPresence::RequiredOnce);
            assert_eq!(
                schema.skill_input,
                Some(SkillInputAuthority::AuthoredOrProjected)
            );
        }
        assert_eq!(raw_slots.len(), 4);
        let unit = prior_definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::Unit(d) => Some(d.id.clone()),
                _ => None,
            })
            .unwrap();
        let pool = prior_definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::PointPool(d) => Some(d.id.clone()),
                _ => None,
            })
            .unwrap();
        let mut owners = Vec::new();
        for family in &families {
            let owner = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == SchemaSubject::Definition(family.passive.address()))
                .unwrap();
            assert!(
                migration.owners.contains(owner),
                "published producer equals exact authored body and closure"
            );
            assert!(!owner.programs.is_complete());
            assert_eq!(owner.programs.members.len(), 1);
            assert_eq!(owner.programs.members[0].id, family.program);
            assert_eq!(owner.programs.members[0].context, RuleEntityKind::Actor);
            assert_eq!(owner.programs.members[0].effects.len(), 2);
            owners.push(owner.clone());
        }
        let published_closure = owners[0].programs.closure.clone();
        let mut definitions: Vec<_> = recipe
            .schema
            .definitions
            .iter()
            .filter(|d| prior_definitions.iter().any(|p| p.address() == d.address()))
            .cloned()
            .collect();
        assert_eq!(definitions.len(), prior_definitions.len());
        // Isolate only these raw supplies. The actual adjacency, Command, actor,
        // final inputs and numerical mechanics remain Partial in the package.
        for d in &mut definitions {
            match d {
                DefinitionDescriptor::PassiveNode(d) => {
                    let SchemaState::Known(s) = &mut d.schema else {
                        panic!()
                    };
                    assert!(!s.declarations.grants.is_complete());
                    s.adjacent = DeclaredSet::complete(vec![]);
                    let grants = s.declarations.grants.members.clone();
                    let supplies = s.declarations.skill_grants.members.clone();
                    s.declarations = empty();
                    s.declarations.grants.members = grants;
                    s.declarations.skill_grants.members = supplies;
                }
                DefinitionDescriptor::Skill(d) => {
                    let SchemaState::Known(s) = &mut d.schema else {
                        panic!()
                    };
                    assert!(s.directly_selectable && !s.declarations.parameters.is_complete());
                    let parameters = s.declarations.parameters.members.clone();
                    s.declarations = empty();
                    s.declarations.parameters.members = parameters;
                }
                _ => assert!(prior_definitions.contains(d)),
            }
        }
        let quality_unit = match raw_slots
            .iter()
            .find(|row| row.address() == SlotAddress::Parameter(families[0].raw_quality.clone()))
            .unwrap()
        {
            SlotDescriptor::Parameter(DefinitionEntry {
                schema:
                    SchemaState::Known(ParameterSlotSchema {
                        value: ValueSchema::Quantity(range),
                        ..
                    }),
                ..
            }) => range.minimum.unit().clone(),
            _ => panic!("published quality quantity"),
        };
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
        let mut slots = raw_slots;
        for family in &families {
            for address in [
                SlotAddress::Grant(family.grant.clone()),
                SlotAddress::SkillGrant(family.supply.clone()),
            ] {
                let mut row = recipe
                    .schema
                    .slots
                    .iter()
                    .find(|r| r.address() == address)
                    .unwrap()
                    .clone();
                if let SlotDescriptor::SkillGrant(r) = &mut row {
                    let SchemaState::Known(s) = &mut r.schema else {
                        panic!()
                    };
                    assert!(s.outputs.members.is_empty() && !s.outputs.is_complete());
                    s.outputs.closure = SchemaClosure::Complete;
                }
                slots.push(row);
            }
        }
        let namespace = recipe.schema.namespace.clone();
        let def = |name: &str| DefId::parse(namespace.clone(), name).unwrap();
        let class: ClassDefId = def("component-class");
        let encounter: EncounterDefId =
            DefId::parse(namespace.clone(), "component-encounter").unwrap();
        let level = IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        };
        definitions.extend([
            DefinitionDescriptor::Class(known(
                class.clone(),
                ClassSchema {
                    level: level.clone(),
                    ascendancies: DeclaredSet::complete(vec![]),
                    implicit_passives: DeclaredSet::complete(vec![]),
                    declarations: empty(),
                },
            )),
            DefinitionDescriptor::Encounter(known(
                encounter.clone(),
                EncounterSchema {
                    enemy_level: level,
                    external_inputs: DeclaredSet::complete(vec![]),
                },
            )),
        ]);
        let mut schema = SchemaPackageInput {
            schema_version: 5,
            namespace: namespace.clone(),
            release: key("unpublished-tree-grant-component"),
            semantics_version: key("finite-raw-transport"),
            definitions,
            slots,
        };
        // These input channels only satisfy the empty support interface. No
        // support assignments request them and no game value is invented.
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
            schema.definitions.push(DefinitionDescriptor::Stat(known(
                DefId::parse(namespace.clone(), name).unwrap(),
                StatSchema {
                    value,
                    targets: vec![scope],
                },
            )));
        }
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
        for subject in [class.address(), encounter.address()] {
            owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(subject),
                programs: DeclaredSet::complete(vec![]),
            });
        }
        let checked =
            OwnedDefinitionSchemaPackage::new(schema.clone(), Default::default()).unwrap();
        let rules = RulePackageInput {
            schema_version: recipe.rules.schema_version,
            namespace: namespace.clone(),
            release: schema.release.clone(),
            semantics_version: schema.semantics_version.clone(),
            operations_version: recipe.rules.operations_version.clone(),
            definitions: checked.identity().clone(),
            tables: vec![],
            owners,
            receivers: DeclaredSet::complete(vec![]),
            effect_applications: Some(DeclaredSet::complete(vec![])),
        };
        let build = BuildInput {
            allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([92; 16]), 100),
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
            allocations: families
                .iter()
                .enumerate()
                .map(|(i, f)| Allocation {
                    id: id(10 + i as u64),
                    node: f.passive.clone(),
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
            build,
            scenario,
            unit,
            quality_unit: quality_unit.clone(),
            published_closure,
        };
        f.build.skills.push(SkillUse {
            id: id(20),
            source: AuthoredSkillSource::Direct(f.families[0].skill.clone()),
            parameters: Some(vec![
                ParameterAssignment {
                    slot: f.families[0].raw_level.clone(),
                    value: f.quantity(20.0),
                },
                ParameterAssignment {
                    slot: f.families[0].raw_quality.clone(),
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(12.5, quality_unit).unwrap(),
                    ),
                },
            ]),
            enabled: true,
            scope: LoadoutScope::Shared,
        });
        f
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
    fn plan(&self) -> Plan {
        self.checked_plan().unwrap()
    }
    fn checked_plan(&self) -> std::result::Result<Plan, String> {
        let definitions = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.schema.clone(), Default::default()).unwrap(),
        );
        let mut rules = self.rules.clone();
        rules.definitions = definitions.identity().clone();
        let stored =
            OwnedRulePackage::new(rules, definitions.as_ref(), Default::default()).unwrap();
        let rules = Arc::new(
            CompiledRulePackage::compile_stored(&stored, definitions.as_ref(), Default::default())
                .unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: 1,
                    namespace: self.schema.namespace.clone(),
                    release: self.schema.release.clone(),
                    definitions: definitions.identity().clone(),
                    outputs: vec![],
                },
                definitions.as_ref(),
                Default::default(),
            )
            .unwrap(),
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
                    schema_version: 2,
                    namespace: self.schema.namespace.clone(),
                    release: key("stages"),
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
                                stage: key(if p.context == RuleEntityKind::Actor {
                                    "prepare"
                                } else {
                                    "execute"
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
                            .map(|f| GeneratedSkillReadiness {
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
                                    let family = self.families.iter().find(|f| {
                                        o.owner == SchemaSubject::Definition(f.passive.address())
                                    });
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
            .map_err(|error| error.to_string())?,
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
            .unwrap(),
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
                    release: key("empty-support-inputs"),
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
            .unwrap(),
        );
        let receiving = Arc::new(
            OwnedSupportReceiving::new(
                SupportReceivingInput {
                    source_properties: None,
                    schema_version: 2,
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
            .unwrap(),
        );
        assert!(
            matches!(
                OwnedEffectPlan::compile(
                    Arc::new(self.request()),
                    definitions.clone(),
                    rules.clone(),
                    routing.clone(),
                    Default::default()
                ),
                Err(PlanError::Invalid(_))
            ),
            "V17 cannot bypass checked readiness"
        );
        Plan::compile(
            SupportEffectPlanInputs {
                request: Arc::new(self.request()),
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
        .map_err(|error| error.to_string())
    }
    fn provider(&self, index: usize) -> ProviderKey {
        ProviderKey {
            root: ProviderRoot::Allocation(id(10 + index as u64)),
            grant_path: vec![],
        }
    }
    fn generated(&self, index: usize) -> GeneratedSkillKey {
        GeneratedSkillKey {
            provider: self.provider(index),
            slot: self.families[index].supply.clone(),
        }
    }
    fn raw(&self, index: usize) -> PlanValueKey {
        PlanValueKey::SkillParameter {
            skill: Box::new(self.generated(index)),
            parameter: self.families[index].raw_level.clone(),
        }
    }
    fn probe(&self, target: SkillTarget) -> PlanValueKey {
        PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(Box::new(target)),
            stat: self.def("execution-probe"),
        }
    }
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite attempt unavailable: {:?}", report.outcome)
    };
    effects
}
fn value<'a>(report: &'a SupportEffectsReport, key: &PlanValueKey) -> Option<&'a EffectValue> {
    let found: Vec<_> = effects(report)
        .values
        .iter()
        .filter(|r| &r.key == key)
        .collect();
    assert!(found.len() <= 1);
    found.first().map(|r| &r.value)
}

#[test]
#[ignore = "requires the verified Djinn tree-grant publication"]
fn published_tree_producers_keep_exact_allocation_and_manual_occurrences_separate() {
    let f = Fixture::load();
    let plan = f.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for i in 0..2 {
        assert_eq!(
            value(&report, &f.raw(i)),
            Some(&EffectValue::Known {
                value: f.quantity(1.0)
            })
        );
        assert_eq!(
            value(
                &report,
                &PlanValueKey::Grant {
                    provider: f.provider(i),
                    slot: f.families[i].grant.clone()
                }
            ),
            Some(&EffectValue::Known {
                value: ParameterValue::Boolean(true)
            })
        );
        let target = SkillTarget::Generated(Box::new(f.generated(i)));
        assert_eq!(
            value(
                &report,
                &PlanValueKey::SkillParameter {
                    skill: Box::new(f.generated(i)),
                    parameter: f.families[i].raw_quality.clone(),
                },
            ),
            None,
            "no generated quality producer is fabricated"
        );
        assert_eq!(
            value(&report, &f.probe(target.clone())),
            Some(&EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: None,
            }),
            "missing quality gates even an input-free execution consumer"
        );
        let resolver =
            OwnedOccurrenceResolver::new(plan.definitions(), plan.request(), Default::default())
                .unwrap();
        let resolved = resolver.skill(&target).unwrap();
        assert_eq!(
            resolved.value().unwrap().definition(),
            Some(&f.families[i].skill)
        );
        assert_eq!(resolved.value().unwrap().provider().key(), &f.provider(i));
        let mut entered = f.provider(i);
        entered.grant_path.push(f.families[i].grant.clone());
        assert!(resolver.provider(&entered).unwrap().value().is_some());
        let mut wrong = f.generated(i);
        wrong.provider = f.provider(1 - i);
        assert!(
            resolver
                .skill(&SkillTarget::Generated(Box::new(wrong)))
                .unwrap()
                .value()
                .is_none()
        );
        let actual: Vec<_> = effects(&report)
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == f.families[i].program)
            .collect();
        assert_eq!(actual.len(), 2);
        assert!(actual.iter().all(|e| e.key.invocation.origin
            == RuleOrigin::Provider {
                provider: f.provider(i)
            }));
    }
    assert_eq!(
        value(&report, &f.probe(SkillTarget::Authored(id(20)))),
        Some(&EffectValue::Known {
            value: ParameterValue::Boolean(true)
        })
    );
    assert!(f.build.gems.is_empty());
}

#[test]
#[ignore = "requires the verified Djinn tree-grant publication"]
fn removing_an_allocation_and_inactive_manual_loadout_do_not_reuse_another_provider() {
    let original = Fixture::load();
    let a = original.plan();
    let mut changed = original.clone();
    changed.build.allocations.remove(0);
    changed.build.skills[0].scope = LoadoutScope::Selected {
        loadouts: vec![id(2)],
    };
    let b = changed.plan();
    let report_b = b.evaluate(&mut b.new_scratch()).unwrap();
    assert!(value(&report_b, &original.raw(0)).is_none());
    assert_eq!(
        value(&report_b, &original.raw(1)),
        Some(&EffectValue::Known {
            value: original.quantity(1.0)
        })
    );
    assert!(!matches!(
        value(&report_b, &original.probe(SkillTarget::Authored(id(20)))),
        Some(EffectValue::Known { .. })
    ));
    changed.build.active_weapon_loadout = id(2);
    let c = changed.plan();
    let report_c = c.evaluate(&mut c.new_scratch()).unwrap();
    assert_eq!(
        value(&report_c, &original.probe(SkillTarget::Authored(id(20)))),
        Some(&EffectValue::Known {
            value: ParameterValue::Boolean(true)
        })
    );
    let report_a = a.evaluate(&mut a.new_scratch()).unwrap();
    let mut scratch = a.new_scratch();
    for (plan, expected) in [
        (&a, &report_a),
        (&b, &report_b),
        (&c, &report_c),
        (&a, &report_a),
    ] {
        assert!(
            plan.evaluate(&mut scratch).unwrap() == *expected,
            "full report changed on A/B/C/A scratch reuse"
        );
    }
    let mut work = 0;
    assert!(matches!(
        a.evaluate_with_budget(&mut scratch, &mut work),
        Err(PlanError::Limit("work"))
    ));
    assert!(a.evaluate(&mut scratch).unwrap() == report_a);
    let plans = Arc::new([a, b, c]);
    let reports = [report_a, report_b, report_c];
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..48usize)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |scratch, i| (i % 3, plans[i % 3].evaluate(scratch).unwrap()),
                )
                .collect::<Vec<_>>()
        })
        .into_iter()
        .for_each(|(i, r)| assert!(r == reports[i], "private worker scratch differs"));
}

#[test]
#[ignore = "requires the verified Djinn tree-grant publication"]
fn actual_partial_coverage_and_shared_pool_authority_are_not_weakened() {
    let f = Fixture::load();
    let mut partial = f.clone();
    partial.rules.owners[0].programs.closure = f.published_closure.clone();
    let error = partial
        .checked_plan()
        .err()
        .expect("actual Partial owner cannot obtain early readiness");
    assert!(error.contains("complete owner programs"), "{error}");
    let mut invalid = f.clone();
    invalid.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![id(2)],
    };
    let definitions =
        OwnedDefinitionSchemaPackage::new(invalid.schema.clone(), Default::default()).unwrap();
    let report = bind_owned_request(&definitions, &invalid.request(), Default::default()).unwrap();
    assert_eq!(
        report.schema(),
        SchemaBindingStatus::Invalid,
        "actual ascendancy pool requires Shared allocation scope"
    );
}
