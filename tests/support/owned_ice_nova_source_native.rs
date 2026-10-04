//! Real source-input programs inside the existing finite intrinsic fixture.
//! The item-free component supplies already-admitted support positions. Its
//! preparation metadata does not claim general Ice type matching or delivery.
use super::*;
use poe_optimizer_core::owned_source_properties::*;

#[derive(Clone, Deserialize)]
struct SourceChannels {
    pre_support_level: StatDefId,
    pre_support_quality: StatDefId,
    prepared_support_level: StatDefId,
    prepared_support_quality: StatDefId,
    non_hidden_count: StatDefId,
    supported_level: StatDefId,
    global_spell_level: StatDefId,
}
#[derive(Clone, Deserialize)]
struct SourcePrograms {
    pre_support: OwnedDefinitionKey,
    assembly: OwnedDefinitionKey,
    exodus: OwnedDefinitionKey,
}
#[derive(Clone, Deserialize)]
pub(super) struct SupportRow {
    pub gem: GemDefId,
    preparation_program: OwnedDefinitionKey,
    property_programs: Vec<OwnedDefinitionKey>,
    counted: bool,
}
#[derive(Clone, Deserialize)]
struct SourceBindings {
    physical_gem: GemDefId,
    primary_skill: SkillDefId,
    final_level: DeclaredSlot<ParameterSlotDefId>,
    corruption: DeclaredSlot<ParameterSlotDefId>,
    quality: QualityDefId,
    channels: SourceChannels,
    programs: SourcePrograms,
    supports: Vec<SupportRow>,
}
#[derive(Clone)]
pub(super) struct Configuration {
    b: SourceBindings,
    pub actual_gem_owner: DefinitionRules,
}
#[derive(Clone)]
pub(super) struct SourceFixture {
    pub f: Fixture,
    pub config: Configuration,
}
fn definition(
    recipe: &poe_optimizer_import::owned_recipe::OwnedRecipeInput,
    address: &DefinitionAddress,
) -> DefinitionDescriptor {
    recipe
        .schema
        .definitions
        .iter()
        .find(|row| &row.address() == address)
        .unwrap()
        .clone()
}
fn add_definition(f: &mut Fixture, row: DefinitionDescriptor) {
    if let Some(prior) = f
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == row.address())
    {
        assert_eq!(
            *prior, row,
            "shared dependency must be the actual published descriptor"
        );
    } else {
        f.schema.definitions.push(row);
    }
}
impl SourceFixture {
    pub fn load() -> Self {
        static SOURCE: OnceLock<SourceFixture> = OnceLock::new();
        SOURCE.get_or_init(Self::from_publication).clone()
    }
    fn from_publication() -> Self {
        let mut f = Fixture::load();
        let output = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ICE_SOURCE_INPUTS_OUTPUT")
                .expect("checked source-input publication parent"),
        );
        let endpoint = release::load(&output.join("package"));
        assert!(
            endpoint.evaluation().is_none(),
            "actual Partial release has no fabricated evaluation bundle"
        );
        let recipe = &endpoint.input().recipe;
        assert_eq!(
            recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V18
        );
        let packet = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/ice-nova-source-inputs");
        let b: SourceBindings = read(packet.join("bindings.json"));
        let extension: OwnedReleaseMigrationInput = read(packet.join("migration.json"));
        assert_eq!(b.physical_gem, f.b.physical_gem);
        assert_eq!(b.primary_skill, f.b.primary_skill);
        assert_eq!(b.final_level, f.b.final_level);
        assert!(
            extension
                .owners
                .iter()
                .flat_map(|o| &o.programs.members)
                .any(|p| p.id == b.programs.pre_support
                    && serde_json::to_string(p)
                        .unwrap()
                        .contains(b.channels.global_spell_level.key().as_str()))
        );
        assert_eq!(
            b.supports.len(),
            6,
            "include every authored support, including archived variants"
        );
        for entry in &extension.schema {
            match entry {
                SchemaExtensionEntry::Definition(row) => {
                    assert!(recipe.schema.definitions.contains(row));
                    add_definition(&mut f, row.clone());
                }
                SchemaExtensionEntry::Slot(_) => panic!("this packet allocates no input slots"),
            }
        }
        for stat in [
            &b.channels.pre_support_level,
            &b.channels.pre_support_quality,
            &b.channels.prepared_support_level,
            &b.channels.prepared_support_quality,
        ] {
            add_definition(&mut f, definition(recipe, &stat.address()));
        }
        for authored in &extension.owners {
            let actual = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == authored.owner)
                .unwrap();
            assert_eq!(actual.programs.closure, authored.programs.closure);
            assert!(!actual.programs.is_complete());
            for p in &authored.programs.members {
                assert!(
                    actual.programs.members.contains(p),
                    "execute exact published program bytes"
                );
            }
        }
        let actual_gem_owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(b.physical_gem.address()))
            .unwrap()
            .clone();
        let mut owner = actual_gem_owner.clone();
        owner.programs.closure = SchemaClosure::Complete;
        let target = f
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == owner.owner)
            .unwrap();
        *target = owner;
        // Remove the old explicit-final input boundary entirely on this path.
        f.schema
            .slots
            .retain(|row| row.address() != SlotAddress::Parameter(f.fixture_level.clone()));
        for row in &mut f.schema.definitions {
            if let DefinitionDescriptor::Gem(DefinitionEntry {
                id,
                schema: SchemaState::Known(gem),
            }) = row
                && *id == b.physical_gem
            {
                gem.declarations
                    .parameters
                    .members
                    .retain(|p| p != &f.fixture_level);
            }
        }
        assert!(
            !f.rules
                .owners
                .iter()
                .flat_map(|o| &o.programs.members)
                .any(|p| p.id == key("fixture-final-input-only"))
        );
        for support in &b.supports {
            let actual = definition(recipe, &support.gem.address());
            let DefinitionDescriptor::Gem(mut row) = actual else {
                unreachable!()
            };
            let SchemaState::Known(gem) = &mut row.schema else {
                panic!("known physical support")
            };
            for skill in &gem.skills.members {
                add_definition(&mut f, definition(recipe, &skill.address()));
            }
            for parameter in &gem.declarations.parameters.members {
                let slot = recipe
                    .schema
                    .slots
                    .iter()
                    .find(|s| s.address() == SlotAddress::Parameter(parameter.clone()))
                    .unwrap()
                    .clone();
                assert!(!f.schema.slots.iter().any(|s| s.address() == slot.address()));
                f.schema.slots.push(slot);
            }
            // The finite component keeps raw support inputs and property rules,
            // but excludes implicit catalog Skill topology and ordinary delivery.
            // The actual descriptor remains authenticated above and in the
            // publication dependency receipt; it is not changed in the package.
            gem.skills = DeclaredSet::complete(vec![]);
            gem.quality.allowed_kinds.closure = SchemaClosure::Complete;
            finite(&mut gem.declarations);
            f.schema.definitions.push(DefinitionDescriptor::Gem(row));
            let mut owner = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == SchemaSubject::Definition(support.gem.address()))
                .unwrap()
                .clone();
            assert!(!owner.programs.is_complete());
            assert!(
                owner
                    .programs
                    .members
                    .iter()
                    .any(|p| p.id == support.preparation_program)
            );
            owner.programs.closure = SchemaClosure::Complete;
            f.rules.owners.push(owner);
        }
        // Admission is an explicit finite boundary. No synthetic numerical
        // property, raw input, prepared level or final level is supplied here.
        let true_stat: StatDefId = f.def("admitted-boundary-true");
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            true_stat.clone(),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Skill],
            },
        )));
        for subject in [
            SchemaSubject::Definition(b.physical_gem.address()),
            SchemaSubject::Definition(b.primary_skill.address()),
        ] {
            let false_stat = f.def("unused-flag");
            let owner = f
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == subject)
                .unwrap();
            owner.programs.members.push(RuleProgram {
                id: key("finite-admitted-position-facts"),
                context: RuleEntityKind::Skill,
                reads: vec![],
                nodes: [false, true]
                    .into_iter()
                    .map(|v| RuleNode {
                        id: key(if v { "true" } else { "false" }),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(v),
                        },
                    })
                    .collect(),
                effects: [("false", false_stat), ("true", true_stat.clone())]
                    .into_iter()
                    .map(|(name, stat)| RuleEffect {
                        id: key(name),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat,
                            value: key(name),
                        },
                    })
                    .collect(),
            });
        }
        f.rules.operations_version = key(OWNED_RULE_OPERATIONS_V18);
        let mut result = Self {
            f,
            config: Configuration {
                b,
                actual_gem_owner,
            },
        };
        result.raw(&[17, 12]);
        result
    }
    pub fn raw(&mut self, levels: &[u16]) {
        self.f.levels(&vec![17; levels.len()]);
        self.f.build.supports.clear();
        self.f.build.support_origins = Some(vec![]);
        for (copy, level) in levels.iter().copied().enumerate() {
            self.f.build.gems[copy].level = level;
            self.f.build.gems[copy].quality = Some(QualitySelection {
                kind: self.config.b.quality.clone(),
                amount: FiniteQuantity::new(0., self.f.quality_unit.clone()).unwrap(),
            });
            self.f
                .build
                .support_origins
                .as_mut()
                .unwrap()
                .push(SupportOriginSequence {
                    target: Self::owner(copy),
                    origins: vec![],
                });
        }
        assert!(
            self.f
                .build
                .gems
                .iter()
                .all(|g| g.parameters.iter().all(|p| p.slot != self.f.fixture_level))
        );
    }
    pub fn owner(copy: usize) -> SkillTarget {
        SkillTarget::Authored(id(1000 + copy as u64))
    }
    pub fn delta(&mut self, copy: usize, amount: f64) {
        let p = self.f.build.gems[copy]
            .parameters
            .iter_mut()
            .find(|p| p.slot == self.config.b.corruption)
            .unwrap();
        let ParameterValue::Quantity(q) = &p.value else {
            panic!()
        };
        p.value = ParameterValue::Quantity(FiniteQuantity::new(amount, q.unit().clone()).unwrap());
    }
    pub fn quality(&mut self, copy: usize, amount: f64) {
        self.f.build.gems[copy].quality.as_mut().unwrap().amount =
            FiniteQuantity::new(amount, self.f.quality_unit.clone()).unwrap();
    }
    pub fn supports(&mut self, copy: usize, rows: &[(usize, f64)]) {
        let mut origins = vec![];
        for (position, (which, quality)) in rows.iter().copied().enumerate() {
            let row = &self.config.b.supports[which];
            let gem = id(2000 + copy as u64 * 100 + position as u64);
            let assignment = id(4000 + copy as u64 * 100 + position as u64);
            let mut parameters = vec![];
            for slot in &self.f.schema.slots {
                if let SlotDescriptor::Parameter(DefinitionEntry {
                    id,
                    schema: SchemaState::Known(s),
                }) = slot
                    && id.declaration == SlotOwnerDefId::Gem(row.gem.clone())
                {
                    let value = match &s.value {
                        ValueSchema::Boolean => ParameterValue::Boolean(false),
                        ValueSchema::Quantity(r) => ParameterValue::Quantity(
                            FiniteQuantity::new(0., r.minimum.unit().clone()).unwrap(),
                        ),
                        _ => panic!("unexpected actual support raw parameter"),
                    };
                    parameters.push(ParameterAssignment {
                        slot: id.clone(),
                        value,
                    });
                }
            }
            self.f.build.gems.push(GemInstance {
                id: gem,
                definition: row.gem.clone(),
                parameters,
                level: 1,
                quality: Some(QualitySelection {
                    kind: self.config.b.quality.clone(),
                    amount: FiniteQuantity::new(quality, self.f.quality_unit.clone()).unwrap(),
                }),
            });
            self.f.build.supports.push(SupportAssignment {
                id: assignment,
                support: gem,
                target: Self::owner(copy),
                enabled: true,
            });
            origins.push(SupportOrigin::Assignment(assignment));
        }
        self.f
            .build
            .support_origins
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s.target == Self::owner(copy))
            .unwrap()
            .origins = origins;
    }
    pub fn plan(&self) -> Plan {
        self.f.plan_with_source(Some(&self.config))
    }
    pub fn final_key(&self, copy: usize) -> PlanValueKey {
        PlanValueKey::SkillParameter {
            skill: Box::new(self.f.generated(copy)),
            parameter: self.f.b.final_level.clone(),
        }
    }
    pub fn stat_key(&self, copy: usize, which: &str) -> PlanValueKey {
        let stat = match which {
            "count" => &self.config.b.channels.non_hidden_count,
            "level" => &self.config.b.channels.pre_support_level,
            "quality" => &self.config.b.channels.pre_support_quality,
            _ => panic!(),
        };
        PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(Box::new(Self::owner(copy))),
            stat: stat.clone(),
        }
    }
    pub fn remove_final(&mut self) {
        self.f
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == SchemaSubject::Definition(self.f.b.physical_gem.address()))
            .unwrap()
            .programs
            .members
            .retain(|p| p.id != self.config.b.programs.assembly);
    }
    pub fn remove_raw_delta(&mut self, copy: usize) {
        self.f.build.gems[copy]
            .parameters
            .retain(|p| p.slot != self.config.b.corruption);
    }
    pub fn incomplete_ordinary_incoming(&mut self) {
        // Synthetic negative of this fixture's explicitly closed item-free frame.
        // The real packet's top-level receiver inventory is currently Complete.
        self.f.rules.receivers.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(
                    self.config.b.channels.global_spell_level.address(),
                ),
                facet: SchemaFacet::GameRules,
                code: key("fixture-incomplete-ordinary-incoming"),
            }],
        };
    }
    pub fn support_stat_key(&self, copy: usize, position: usize, quality: bool) -> PlanValueKey {
        PlanValueKey::Stat {
            entity: ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(id(4000
                + copy as u64 * 100
                + position as u64))),
            stat: if quality {
                self.config.b.channels.prepared_support_quality.clone()
            } else {
                self.config.b.channels.prepared_support_level.clone()
            },
        }
    }
}

impl Configuration {
    pub fn stages(
        &self,
        f: &Fixture,
        defs: &OwnedDefinitionSchemaPackage,
        rules: &OwnedRulePackage,
        routing: &OwnedActionRouting,
    ) -> EvaluationStagesInput {
        let mut classified = vec![];
        let mut scheduled = vec![];
        for o in &rules.input().owners {
            for p in &o.programs.members {
                let (stage, phase, role) = if p.id == self.b.programs.assembly {
                    (
                        "assemble",
                        ReadinessPhase::Preparation,
                        ReadinessProgramRole::SourceFinalInputAssembly,
                    )
                } else if p.id == self.b.programs.exodus {
                    (
                        "properties",
                        ReadinessPhase::Preparation,
                        ReadinessProgramRole::SourceSupportedProperty,
                    )
                } else if p.context == RuleEntityKind::Action {
                    (
                        "execute",
                        ReadinessPhase::Execution,
                        ReadinessProgramRole::Execution,
                    )
                } else {
                    (
                        "prepare",
                        ReadinessPhase::Structural,
                        ReadinessProgramRole::PreparationFacts,
                    )
                };
                let outputs = if phase == ReadinessPhase::Execution {
                    vec![]
                } else {
                    p.effects
                        .iter()
                        .map(|e| match &e.effect {
                            RuleEffectKind::Derive {
                                entity: RuleEntity::Current,
                                stat,
                                ..
                            } => StageChannel::Stat {
                                scope: p.context,
                                stat: stat.clone(),
                            },
                            RuleEffectKind::Contribute {
                                entity: RuleEntity::PropertyOwner,
                                stat,
                                contribution,
                                ..
                            } => StageChannel::Contributions {
                                scope: RuleEntityKind::Skill,
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
                            _ => panic!("unexpected authored source program output"),
                        })
                        .collect()
                };
                classified.push(ReadinessProgram {
                    owner: o.owner.clone(),
                    program: p.id.clone(),
                    phase,
                    role,
                    outputs,
                });
                scheduled.push(StagedRuleProgram {
                    owner: o.owner.clone(),
                    program: p.id.clone(),
                    stage: key(stage),
                });
            }
        }
        let mut frozen = vec![];
        for (stat, scope) in [
            (&self.b.channels.pre_support_level, RuleEntityKind::Skill),
            (&self.b.channels.pre_support_quality, RuleEntityKind::Skill),
            (
                &self.b.channels.prepared_support_level,
                RuleEntityKind::SupportOrigin,
            ),
            (
                &self.b.channels.prepared_support_quality,
                RuleEntityKind::SupportOrigin,
            ),
        ] {
            frozen.push(FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope,
                    stat: stat.clone(),
                },
                stage: key("prepare"),
            });
        }
        for stat in [f.def("unused-flag"), f.def("admitted-boundary-true")] {
            frozen.push(FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat,
                },
                stage: key("prepare"),
            });
        }
        EvaluationStagesInput {
            schema_version: 3,
            namespace: f.schema.namespace.clone(),
            release: key("finite-real-source-stages"),
            definitions: defs.identity().clone(),
            rules: *rules.identity(),
            routing: *routing.identity(),
            stages: [
                ("prepare", None),
                ("census", Some("prepare")),
                ("properties", Some("census")),
                ("assemble", Some("properties")),
                ("execute", Some("assemble")),
            ]
            .into_iter()
            .map(|(id, before)| EvaluationStage {
                id: key(id),
                predecessors: before.map(key).into_iter().collect(),
            })
            .collect(),
            programs: DeclaredSet::complete(scheduled),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            routing_stage: key("execute"),
            frozen_channels: frozen,
            readiness: Some(ReadinessInput {
                skills: vec![GeneratedSkillReadiness {
                    skill: f.b.primary_skill.clone(),
                    parameters: DeclaredSet::complete(vec![ParameterReadiness {
                        parameter: f.b.final_level.clone(),
                        phase: ReadinessPhase::Execution,
                    }]),
                }],
                programs: DeclaredSet::complete(classified),
            }),
        }
    }
    pub fn preparation(
        &self,
        f: &Fixture,
        defs: &OwnedDefinitionSchemaPackage,
        rules: &OwnedRulePackage,
    ) -> SupportPreparationInput {
        // This fixture's selected frame admits these source-observed positions.
        // Separate metadata conversion must prove type predicates before shipping.
        let symbols: Vec<_> = self
            .b
            .supports
            .iter()
            .map(|s| s.gem.key().clone())
            .collect();
        SupportPreparationInput {
            schema_version: 1,
            namespace: f.schema.namespace.clone(),
            release: key("finite-already-admitted-positions"),
            definitions: defs.identity().clone(),
            rules: *rules.identity(),
            policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
            quality_unit: f.quality_unit.clone(),
            types: vec![],
            effects: symbols.clone(),
            families: symbols,
            supports: self
                .b
                .supports
                .iter()
                .map(|s| SupportPreparationEntry {
                    gem: s.gem.clone(),
                    preparation: SchemaState::Known(SupportPreparationDefinition {
                        effect: s.gem.key().clone(),
                        families: Some(vec![s.gem.key().clone()]),
                        plus_version_of: None,
                        requires: None,
                        excludes: None,
                        added_types: vec![],
                        gems_only: false,
                        from_item: false,
                        is_support: true,
                        is_trigger: false,
                        ignore_minion_types: false,
                    }),
                })
                .collect(),
        }
    }
    pub fn inputs(
        &self,
        f: &Fixture,
        defs: &OwnedDefinitionSchemaPackage,
        rules: &OwnedRulePackage,
        preparation: &OwnedSupportPreparation,
        stages: &OwnedEvaluationStages,
    ) -> SupportInputBindingsInput {
        let absent = || OptionalTypeInputs {
            present: f.def("unused-flag"),
            members: vec![],
        };
        SupportInputBindingsInput {
            schema_version: 1,
            namespace: f.schema.namespace.clone(),
            release: key("finite-real-source-inputs"),
            definitions: defs.identity().clone(),
            rules: *rules.identity(),
            preparation: *preparation.identity(),
            stages: *stages.identity(),
            preparation_stage: key("prepare"),
            effective_level: self.b.channels.prepared_support_level.clone(),
            effective_quality: self.b.channels.prepared_support_quality.clone(),
            target: SupportTargetInputBindings {
                skill_types: vec![],
                minion_types: absent(),
                summoner: OptionalTypeContextInputs {
                    present: f.def("unused-flag"),
                    skill_types: vec![],
                    minion_types: absent(),
                },
                cannot_be_supported: f.def("unused-flag"),
                has_gem: f.def("admitted-boundary-true"),
                from_item: f.def("unused-flag"),
                is_player_actor: f.def("admitted-boundary-true"),
            },
        }
    }
    pub fn receiving(
        &self,
        f: &Fixture,
        defs: &OwnedDefinitionSchemaPackage,
        rules: &OwnedRulePackage,
        preparation: &OwnedSupportPreparation,
        inputs: &OwnedSupportInputBindings,
        stages: &OwnedEvaluationStages,
    ) -> SupportReceivingInput {
        let assembly = rules
            .input()
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .any(|p| p.id == self.b.programs.assembly)
            .then(|| self.b.programs.assembly.clone())
            .into_iter()
            .collect();
        SupportReceivingInput {
            schema_version: 3,
            namespace: f.schema.namespace.clone(),
            release: key("finite-real-source-receiving"),
            definitions: defs.identity().clone(),
            rules: *rules.identity(),
            preparation: *preparation.identity(),
            inputs: *inputs.identity(),
            stages: *stages.identity(),
            roles: vec![],
            targets: [
                SupportTargetDefinition::Gem(f.b.physical_gem.clone()),
                SupportTargetDefinition::Skill(f.b.primary_skill.clone()),
            ]
            .into_iter()
            .map(|owner| SupportTargetReceivingRoles {
                owner,
                roles: DeclaredSet::complete(vec![]),
            })
            .collect(),
            // Ordinary delivery is outside this finite component. Its absence
            // must be declared per support; omitted inventories remain unknown.
            supports: self
                .b
                .supports
                .iter()
                .map(|s| SupportReceivingEntry {
                    gem: s.gem.clone(),
                    receivers: DeclaredSet::complete(vec![]),
                })
                .collect(),
            source_properties: Some(SourcePropertyPreparationInput {
                relations: DeclaredSet::complete(vec![SourcePropertyRelation {
                    id: key("ice-source-inputs"),
                    owner: SupportTargetDefinition::Gem(f.b.physical_gem.clone()),
                    occurrence: SourcePropertyOccurrence::AuthoredSkillUseV1,
                    aliases: SourcePropertyAliasPolicy::RejectSharedBackingGemV1,
                    context: SourcePropertyContext::PlayerScenarioV1,
                    census: SourcePropertyCensus::ExactSelectedPositionV1,
                    census_stage: key("census"),
                    effects: DeclaredSet::complete(vec![SourcePropertyEffect {
                        endpoint: SourcePropertyEffectEndpoint::Generated {
                            path: vec![],
                            skill_supply: f.b.primary_supply.clone(),
                        },
                        admission: SupportAdmissionContext::ReceivingSkill {
                            summoner_path: None,
                        },
                    }]),
                    inputs: vec![
                        self.b.channels.pre_support_level.clone(),
                        self.b.channels.pre_support_quality.clone(),
                    ],
                    channels: DeclaredSet::complete(vec![SourcePropertyChannel {
                        stat: self.b.channels.supported_level.clone(),
                        contribution: ContributionKind::Add,
                    }]),
                    external: DeclaredSet::complete(vec![]),
                    supports: DeclaredSet::complete(
                        self.b
                            .supports
                            .iter()
                            .map(|s| SourcePropertySupportPrograms {
                                gem: s.gem.clone(),
                                counted: s.counted,
                                programs: DeclaredSet::complete(s.property_programs.clone()),
                            })
                            .collect(),
                    ),
                    assembly: DeclaredSet::complete(assembly),
                    non_hidden_count: self.b.channels.non_hidden_count.clone(),
                }]),
            }),
        }
    }
}
