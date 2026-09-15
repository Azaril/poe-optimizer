//! Synthetic owned inputs only: no import adapter, profiles, or supplied RuleFact values.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, RoutingLimits},
    owned_schema::{
        OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
        SchemaPackageInput,
    },
};
use poe_optimizer_engine::{
    owned_plan::{OwnedEffectPlan, PlanError, PlanLimits},
    owned_rules::{CompiledRulePackage, RuleLimits},
};
use std::sync::Arc;

pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("owned-plan-test", "v1").unwrap()
}
pub fn def<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(ns(), s).unwrap()
}
pub fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([61; 16]), n).unwrap())
}
pub fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
pub fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
pub fn item_owner() -> SchemaSubject {
    subject(def::<ItemTemplateDefinition>("item"))
}
pub fn modifier_owner() -> SchemaSubject {
    subject(def::<ModifierDefinition>("modifier"))
}
pub fn class_owner() -> SchemaSubject {
    subject(def::<ClassDefinition>("class"))
}
pub fn parameter(owner: SlotOwnerDefId, name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: def(name),
    }
}
pub fn output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("skill")),
        slot: def("output"),
    }
}
pub fn action() -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(occurrence(20)),
                grant_path: vec![],
            },
            output: output(),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn range(min: i64, max: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(min).unwrap(),
        maximum: BoundedInteger::new(max).unwrap(),
    }
}
pub fn read(id: &str, source: RuleReadSource) -> RuleRead {
    RuleRead {
        id: key(id),
        value_type: ComputedValueType::Integer,
        source,
    }
}
pub fn node(id: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(id),
        expression,
    }
}
pub fn literal(id: &str, n: i64) -> RuleNode {
    node(id, RuleExpression::Literal { value: integer(n) })
}
pub fn read_node(id: &str, input: &str) -> RuleNode {
    node(id, RuleExpression::Read { input: key(input) })
}
pub fn effect(id: &str, effect: RuleEffectKind) -> RuleEffect {
    RuleEffect {
        id: key(id),
        when: None,
        effect,
    }
}
pub fn derive(id: &str, entity: RuleEntity, stat: &str, value: &str) -> RuleEffect {
    effect(
        id,
        RuleEffectKind::Derive {
            entity,
            stat: def(stat),
            value: key(value),
        },
    )
}
pub fn contributions(id: &str, entity: RuleEntity, stat: &str) -> RuleRead {
    read(
        id,
        RuleReadSource::Contributions {
            entity,
            stat: def(stat),
            contribution: ContributionKind::Add,
            reduction: ContributionReduction::Sum,
            empty: integer(0),
        },
    )
}
fn local_program() -> RuleProgram {
    RuleProgram {
        id: key("local"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![contributions("incoming", RuleEntity::Current, "local")],
        nodes: vec![literal("base", 10), read_node("total", "incoming")],
        effects: vec![
            effect(
                "base",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def("local"),
                    contribution: ContributionKind::Add,
                    value: key("base"),
                },
            ),
            derive("final", RuleEntity::Current, "local", "total"),
        ],
    }
}
fn modifier_program() -> RuleProgram {
    RuleProgram {
        id: key("roll"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![read(
            "magnitude",
            RuleReadSource::Parameter {
                slot: parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll"),
            },
        )],
        nodes: vec![read_node("value", "magnitude")],
        effects: vec![
            effect(
                "local",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def("local"),
                    contribution: ContributionKind::Add,
                    value: key("value"),
                },
            ),
            effect(
                "player",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat: def("actor-total"),
                    contribution: ContributionKind::Add,
                    value: key("value"),
                },
            ),
        ],
    }
}
fn class_program() -> RuleProgram {
    RuleProgram {
        id: key("actor"),
        context: RuleEntityKind::Actor,
        reads: vec![contributions(
            "incoming",
            RuleEntity::Current,
            "actor-total",
        )],
        nodes: vec![read_node("total", "incoming")],
        effects: vec![derive("final", RuleEntity::Current, "actor-total", "total")],
    }
}
fn rules_for(owner: SchemaSubject, programs: Vec<RuleProgram>) -> DefinitionRules {
    DefinitionRules {
        owner,
        programs: DeclaredSet::complete(programs),
    }
}

pub struct Fixture {
    pub schema: SchemaPackageInput,
    pub build: BuildInput,
    pub scenario: ScenarioInput,
    pub queries: QueryInput,
    pub owners: Vec<DefinitionRules>,
    pub tables: Vec<IntegerRuleTable>,
    pub receivers: DeclaredSet<ActorStatReceiver>,
    pub routes: Vec<ActionOutputRoutes>,
}
impl Fixture {
    pub fn new() -> Self {
        let roll = parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll");
        let needed = parameter(SlotOwnerDefId::ItemTemplate(def("item")), "needs-level");
        let mut item_declarations = declarations();
        item_declarations.parameters.members.push(needed.clone());
        let mut modifier_declarations = declarations();
        modifier_declarations.parameters.members.push(roll.clone());
        let mut skill_declarations = declarations();
        skill_declarations.outputs.members.push(output());
        let mut definitions = vec![
            DefinitionDescriptor::Class(known(
                def("class"),
                ClassSchema {
                    implicit_passives: poe_optimizer_core::owned_schema::DeclaredSet::complete(
                        vec![],
                    ),
                    level: range(1, 100),
                    ascendancies: empty(),
                    declarations: declarations(),
                },
            )),
            DefinitionDescriptor::Encounter(known(
                def("encounter"),
                EncounterSchema {
                    enemy_level: range(1, 100),
                    external_inputs: empty(),
                },
            )),
            DefinitionDescriptor::ItemTemplate(known(
                def("item"),
                ItemTemplateSchema {
                    item_level: range(1, 100),
                    equipment_slots: DeclaredSet::complete(vec![def("weapon"), def("other")]),
                    socket_destinations: empty(),
                    modifiers: DeclaredSet::complete(vec![def("modifier")]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: empty(),
                    },
                    declarations: item_declarations,
                },
            )),
            DefinitionDescriptor::Modifier(known(
                def("modifier"),
                ModifierSchema {
                    declarations: modifier_declarations,
                },
            )),
            DefinitionDescriptor::EquipmentSlot(known(
                def("weapon"),
                EquipmentSlotSchema {
                    scope: ScopePolicy::Either,
                },
            )),
            DefinitionDescriptor::EquipmentSlot(known(
                def("other"),
                EquipmentSlotSchema {
                    scope: ScopePolicy::Either,
                },
            )),
            DefinitionDescriptor::Skill(known(
                def("skill"),
                SkillSchema {
                    directly_selectable: true,
                    declarations: skill_declarations,
                },
            )),
            DefinitionDescriptor::ActionPart(known(def("part"), ActionPartSchema {})),
            DefinitionDescriptor::ActionMode(known(def("mode"), ActionModeSchema {})),
            DefinitionDescriptor::ActionStatSet(known(def("set"), ActionStatSetSchema {})),
            DefinitionDescriptor::Unit(known(
                def("count"),
                UnitSchema {
                    dimension: UnitDimension::Count,
                },
            )),
            DefinitionDescriptor::Metric(known(
                def("requested"),
                MetricSchema {
                    targets: vec![MetricTargetKind::Action],
                    unit: def("count"),
                    actor_roles: vec![MetricActorRole::Player],
                    provider_roles: vec![ProviderRole::SkillUse],
                },
            )),
        ];
        for (name, kind) in [
            ("local", RuleEntityKind::EquipmentUse),
            ("actor-total", RuleEntityKind::Actor),
            ("level", RuleEntityKind::EquipmentUse),
            ("left", RuleEntityKind::EquipmentUse),
            ("right", RuleEntityKind::EquipmentUse),
            ("routed", RuleEntityKind::Action),
        ] {
            definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![kind],
                },
            )));
        }
        let schema = SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("schema"),
            semantics_version: key("test-v1"),
            definitions,
            slots: vec![
                SlotDescriptor::Parameter(known(
                    roll.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Integer(range(-100, 100)),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ModifierRoll],
                    },
                )),
                SlotDescriptor::Parameter(known(
                    needed.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Boolean,
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ItemParameter],
                    },
                )),
                SlotDescriptor::ActionOutput(known(
                    output(),
                    ActionOutputSchema {
                        actor_role: DeclaredActorRole::ProviderActor,
                        parts: DeclaredSet::complete(vec![def("part")]),
                        modes: DeclaredSet::complete(vec![def("mode")]),
                        stat_sets: DeclaredSet::complete(vec![def("set")]),
                        choices: empty(),
                    },
                )),
            ],
        };
        let build = BuildInput {
            allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([61; 16]), 100),
            revision: BuildRevision::from_u64(1),
            game_version: ns(),
            character: CharacterSpec {
                class: def("class"),
                ascendancy: None,
                level: 20,
                rewards: vec![],
            },
            weapon_loadouts: vec![occurrence(1), occurrence(2)],
            active_weapon_loadout: occurrence(1),
            items: vec![ItemRecord {
                id: occurrence(3),
                template: def("item"),
                parameters: vec![ParameterAssignment {
                    slot: needed,
                    value: ParameterValue::Boolean(false),
                }],
                item_level: Some(20),
                quality: None,
                modifier_order: vec![occurrence(4), occurrence(5)],
                modifiers: vec![
                    RolledModifier {
                        id: occurrence(4),
                        definition: def("modifier"),
                        rolls: vec![ParameterAssignment {
                            slot: roll.clone(),
                            value: integer(3),
                        }],
                    },
                    RolledModifier {
                        id: occurrence(5),
                        definition: def("modifier"),
                        rolls: vec![ParameterAssignment {
                            slot: roll,
                            value: integer(5),
                        }],
                    },
                ],
            }],
            equipment: vec![
                EquipmentUse {
                    id: occurrence(6),
                    item: occurrence(3),
                    destination: EquipmentDestination::CharacterSlot(def("weapon")),
                    scope: LoadoutScope::Shared,
                },
                EquipmentUse {
                    id: occurrence(7),
                    item: occurrence(3),
                    destination: EquipmentDestination::CharacterSlot(def("other")),
                    scope: LoadoutScope::Shared,
                },
            ],
            gems: vec![],
            allocations: vec![],
            skills: vec![],
            supports: vec![],
            payload_links: vec![],
            choices: vec![],
        };
        Self {
            schema,
            build,
            scenario: ScenarioInput {
                game_version: ns(),
                enemy: EnemySpec {
                    encounter: def("encounter"),
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
            queries: QueryInput {
                game_version: ns(),
                requests: vec![],
            },
            owners: vec![
                rules_for(class_owner(), vec![class_program()]),
                rules_for(item_owner(), vec![local_program()]),
                rules_for(modifier_owner(), vec![modifier_program()]),
                rules_for(subject(def::<EncounterDefinition>("encounter")), vec![]),
                rules_for(subject(def::<SkillDefinition>("skill")), vec![]),
                rules_for(
                    SchemaSubject::Slot(SlotAddress::ActionOutput(output())),
                    vec![],
                ),
            ],
            tables: vec![],
            receivers: DeclaredSet::complete(vec![]),
            routes: vec![],
        }
    }
    pub fn owner_mut(&mut self, subject: &SchemaSubject) -> &mut DefinitionRules {
        self.owners
            .iter_mut()
            .find(|o| &o.owner == subject)
            .unwrap()
    }
    pub fn request(&self) -> OwnedEvaluationRequest {
        let limits = OwnedInputLimits::default();
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), limits).unwrap(),
            ScenarioSpec::new(self.scenario.clone(), limits).unwrap(),
            QuerySpec::new(self.queries.clone(), limits).unwrap(),
            limits,
        )
        .unwrap()
    }
    pub fn compile(&self) -> Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
        self.compile_with(PlanLimits::default())
    }
    pub fn compile_with(
        &self,
        limits: PlanLimits,
    ) -> Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.schema.clone(), OwnedSchemaLimits::default())
                .unwrap(),
        );
        let rules = RulePackageInput {
            receivers: self.receivers.clone(),
            tables: self.tables.clone(),
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("test-v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            owners: self.owners.clone(),
        };
        let rules = Arc::new(
            CompiledRulePackage::compile(&rules, schema.as_ref(), RuleLimits::default()).unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: ns(),
                    release: key("routing"),
                    definitions: schema.identity().clone(),
                    outputs: self.routes.clone(),
                },
                schema.as_ref(),
                RoutingLimits::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(Arc::new(self.request()), schema, rules, routing, limits)
    }
    pub fn add_level_program(&mut self) {
        self.owner_mut(&item_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("level"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![
                    read("level", RuleReadSource::ItemLevel),
                    RuleRead {
                        id: key("needed"),
                        value_type: ComputedValueType::Boolean,
                        source: RuleReadSource::Parameter {
                            slot: parameter(
                                SlotOwnerDefId::ItemTemplate(def("item")),
                                "needs-level",
                            ),
                        },
                    },
                ],
                nodes: vec![
                    read_node("level", "level"),
                    read_node("needed", "needed"),
                    literal("fallback", 17),
                    node(
                        "selected",
                        RuleExpression::Select {
                            condition: key("needed"),
                            when_true: key("level"),
                            when_false: key("fallback"),
                        },
                    ),
                ],
                effects: vec![derive("final", RuleEntity::Current, "level", "selected")],
            });
    }
    pub fn add_action_route(&mut self) {
        self.build.skills.push(SkillUse {
            id: occurrence(20),
            source: AuthoredSkillSource::Direct(def("skill")),
            enabled: true,
            scope: LoadoutScope::Shared,
        });
        self.queries.requests.push(MetricRequest {
            id: QueryId::new("requested-action").unwrap(),
            metric: def("requested"),
            target: MetricTarget::Action(Box::new(action())),
        });
        self.routes.push(ActionOutputRoutes {
            output: output(),
            routes: DeclaredSet::complete(vec![ActionStatRoute {
                id: key("weapon-local"),
                selection: ActionRouteSelection::Exact(Box::new(ActionRouteSelector {
                    part: def("part"),
                    mode: def("mode"),
                    stat_set: def("set"),
                })),
                source: ActionStatRouteSource::PlayerEquipment {
                    slot: def("weapon"),
                    stat: def("local"),
                },
                target: def("routed"),
            }]),
        });
    }
}

pub fn summoner_owner() -> SchemaSubject {
    subject(def::<GemDefinition>("summoner"))
}
pub fn child_slot() -> DeclaredSlot<ActorSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("child"),
    }
}
pub fn child_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("child-grant"),
    }
}
pub fn summoner_provider(use_id: u64) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(use_id)),
        grant_path: vec![],
    }
}
pub fn child_actor(use_id: u64) -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: summoner_provider(use_id),
        slot: child_slot(),
    }))
}
impl Fixture {
    pub fn add_generated_actors(&mut self) {
        let enabled = parameter(SlotOwnerDefId::Gem(def("summoner")), "enabled");
        let mut ports = declarations();
        ports.parameters.members.push(enabled.clone());
        ports.actors.members.push(child_slot());
        ports.grants.members.push(child_grant());
        self.schema
            .definitions
            .push(DefinitionDescriptor::Gem(known(
                def("summoner"),
                GemSchema {
                    level: range(1, 100),
                    roles: vec![AuthoredGemRole::SkillUse],
                    skills: empty(),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: empty(),
                    },
                    declarations: ports,
                },
            )));
        for name in ["child-level", "child-plus"] {
            self.schema
                .definitions
                .push(DefinitionDescriptor::Stat(known(
                    def(name),
                    StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    },
                )));
        }
        self.schema.slots.extend([
            SlotDescriptor::Parameter(known(
                enabled.clone(),
                ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                },
            )),
            SlotDescriptor::Actor(known(
                child_slot(),
                ActorSlotSchema {
                    skills: empty(),
                    outputs: empty(),
                },
            )),
            SlotDescriptor::Grant(known(
                child_grant(),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Actor(child_slot()),
                },
            )),
        ]);
        for (gem, use_id, level) in [(28, 30, 11), (29, 31, 20)] {
            self.build.gems.push(GemInstance {
                id: occurrence(gem),
                definition: def("summoner"),
                parameters: vec![ParameterAssignment {
                    slot: enabled.clone(),
                    value: ParameterValue::Boolean(true),
                }],
                level,
                quality: None,
            });
            self.build.skills.push(SkillUse {
                id: occurrence(use_id),
                source: AuthoredSkillSource::Gem(occurrence(gem)),
                enabled: true,
                scope: LoadoutScope::Shared,
            });
        }
        self.owners.push(rules_for(
            summoner_owner(),
            vec![RuleProgram {
                id: key("supply-child"),
                context: RuleEntityKind::Actor,
                reads: vec![
                    read("level", RuleReadSource::GemLevel),
                    RuleRead {
                        id: key("enabled"),
                        value_type: ComputedValueType::Boolean,
                        source: RuleReadSource::Parameter { slot: enabled },
                    },
                ],
                nodes: vec![read_node("level", "level"), read_node("enabled", "enabled")],
                effects: vec![
                    effect(
                        "project",
                        RuleEffectKind::ProjectActorStat {
                            actor: child_slot(),
                            stat: def("child-level"),
                            value: key("level"),
                        },
                    ),
                    effect(
                        "activate",
                        RuleEffectKind::ActivateGrant {
                            slot: child_grant(),
                            enabled: key("enabled"),
                        },
                    ),
                ],
            }],
        ));
        self.owners.push(rules_for(
            SchemaSubject::Slot(SlotAddress::Actor(child_slot())),
            vec![RuleProgram {
                id: key("child-consumer"),
                context: RuleEntityKind::Actor,
                reads: vec![read(
                    "own-level",
                    RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def("child-level"),
                    },
                )],
                nodes: vec![
                    read_node("own-level", "own-level"),
                    literal("one", 1),
                    node(
                        "plus",
                        RuleExpression::Add {
                            left: key("own-level"),
                            right: key("one"),
                        },
                    ),
                ],
                effects: vec![derive("final", RuleEntity::Current, "child-plus", "plus")],
            }],
        ));
        self.owners.push(rules_for(
            SchemaSubject::Slot(SlotAddress::Grant(child_grant())),
            vec![],
        ));
    }
}
