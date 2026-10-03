//! Private discovery boundary regressions; no public receiver feature is added.
use super::*;
use crate::owned_rules::RuleLimits;
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
    SchemaPackageInput,
};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("deferred-owners", "v1").unwrap()
}
fn def<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(ns(), name).unwrap()
}
fn id<T: BuildInstanceId>(value: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([41; 16]), value).unwrap())
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
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
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn output(gem: bool) -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: if gem {
            SlotOwnerDefId::Gem(def("gem"))
        } else {
            SlotOwnerDefId::Skill(def("skill"))
        },
        slot: def(if gem { "gem-output" } else { "skill-output" }),
    }
}
fn action(gem: bool) -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: root(ProviderRoot::SkillUse(id(if gem { 20 } else { 10 }))),
            output: output(gem),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
fn subject(gem: bool) -> SchemaSubject {
    if gem {
        SchemaSubject::Definition(def::<GemDefinition>("gem").address())
    } else {
        SchemaSubject::Definition(def::<SkillDefinition>("skill").address())
    }
}
fn program(gem: bool) -> RuleProgram {
    RuleProgram {
        id: key("action-program"),
        context: RuleEntityKind::Action,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("literal"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Integer(
                    BoundedInteger::new(if gem { 23 } else { 17 }).unwrap(),
                ),
            },
        }],
        effects: vec![RuleEffect {
            id: key("value"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: def("value"),
                value: key("literal"),
            },
        }],
    }
}
struct Fixture {
    request: OwnedEvaluationRequest,
    schema: OwnedDefinitionSchemaPackage,
    rules: CompiledRulePackage,
    operations: RuleOperationsVersion,
}
impl Fixture {
    fn new(operations: RuleOperationsVersion) -> Self {
        let mut skill_ports = ports();
        skill_ports.outputs.members.push(output(false));
        let mut gem_ports = ports();
        gem_ports.outputs.members.push(output(true));
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns(),
                release: key("schema"),
                semantics_version: key("v1"),
                definitions: vec![
                    DefinitionDescriptor::Class(entry(
                        def("class"),
                        ClassSchema {
                            implicit_passives: empty(),
                            level: range(),
                            ascendancies: empty(),
                            declarations: ports(),
                        },
                    )),
                    DefinitionDescriptor::Encounter(entry(
                        def("encounter"),
                        EncounterSchema {
                            enemy_level: range(),
                            external_inputs: empty(),
                        },
                    )),
                    DefinitionDescriptor::Skill(entry(
                        def("skill"),
                        SkillSchema {
                            directly_selectable: true,
                            declarations: skill_ports,
                        },
                    )),
                    DefinitionDescriptor::Gem(entry(
                        def("gem"),
                        GemSchema {
                            level: range(),
                            roles: vec![AuthoredGemRole::SkillUse],
                            skills: empty(),
                            quality: QualityUseSchema {
                                presence: QualityPresence::Forbidden,
                                allowed_kinds: empty(),
                            },
                            declarations: gem_ports,
                        },
                    )),
                    DefinitionDescriptor::ActionPart(entry(def("part"), ActionPartSchema {})),
                    DefinitionDescriptor::ActionMode(entry(def("mode"), ActionModeSchema {})),
                    DefinitionDescriptor::ActionStatSet(entry(def("set"), ActionStatSetSchema {})),
                    DefinitionDescriptor::Stat(entry(
                        def("value"),
                        StatSchema {
                            value: ComputedValueType::Integer,
                            targets: vec![RuleEntityKind::Action],
                        },
                    )),
                ],
                slots: [false, true]
                    .into_iter()
                    .map(|gem| {
                        SlotDescriptor::ActionOutput(entry(
                            output(gem),
                            ActionOutputSchema {
                                actor_role: DeclaredActorRole::ProviderActor,
                                parts: DeclaredSet::complete(vec![def("part")]),
                                modes: DeclaredSet::complete(vec![def("mode")]),
                                stat_sets: DeclaredSet::complete(vec![def("set")]),
                                choices: empty(),
                            },
                        ))
                    })
                    .collect(),
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap();
        let rules = CompiledRulePackage::compile(
            &RulePackageInput {
                effect_applications: None,
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("rules"),
                semantics_version: key("v1"),
                operations_version: key(&format!(
                    "owned-domain-operations-v{}",
                    operations.revision()
                )),
                definitions: schema.identity().clone(),
                tables: vec![],
                receivers: empty(),
                owners: vec![
                    DefinitionRules {
                        owner: SchemaSubject::Definition(def::<ClassDefinition>("class").address()),
                        programs: empty(),
                    },
                    DefinitionRules {
                        owner: subject(false),
                        programs: DeclaredSet::complete(vec![program(false)]),
                    },
                    DefinitionRules {
                        owner: subject(true),
                        programs: DeclaredSet::complete(vec![program(true)]),
                    },
                ],
            },
            &schema,
            RuleLimits::default(),
        )
        .unwrap();
        let build = BuildSpec::new(
            BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([41; 16]),
                    100,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class: def("class"),
                    ascendancy: None,
                    level: 50,
                    rewards: vec![],
                },
                weapon_loadouts: vec![id(1)],
                active_weapon_loadout: id(1),
                items: vec![],
                equipment: vec![],
                allocations: vec![],
                supports: vec![],
                support_origins: None,
                payload_links: vec![],
                choices: vec![],
                gems: vec![GemInstance {
                    id: id(30),
                    definition: def("gem"),
                    parameters: vec![],
                    level: 1,
                    quality: None,
                }],
                skills: vec![
                    SkillUse {
                        parameters: None,
                        id: id(10),
                        source: AuthoredSkillSource::Direct(def("skill")),
                        enabled: true,
                        scope: LoadoutScope::Shared,
                    },
                    SkillUse {
                        parameters: None,
                        id: id(20),
                        source: AuthoredSkillSource::Gem(id(30)),
                        enabled: true,
                        scope: LoadoutScope::Shared,
                    },
                ],
            },
            OwnedInputLimits::default(),
        )
        .unwrap();
        let scenario = ScenarioSpec::new(
            ScenarioInput {
                game_version: ns(),
                enemy: EnemySpec {
                    encounter: def("encounter"),
                    level: 50,
                },
                assumptions: vec![],
                usage: vec![],
            },
            OwnedInputLimits::default(),
        )
        .unwrap();
        let queries = QuerySpec::new(
            QueryInput {
                game_version: ns(),
                requests: vec![],
            },
            OwnedInputLimits::default(),
        )
        .unwrap();
        let request =
            OwnedEvaluationRequest::new(build, scenario, queries, OwnedInputLimits::default())
                .unwrap();
        Self {
            request,
            schema,
            rules,
            operations,
        }
    }
    fn builder(&self, limits: PlanLimits) -> Builder<'_, OwnedDefinitionSchemaPackage> {
        Builder::new(
            &self.request,
            &self.schema,
            &self.rules,
            OwnedOccurrenceResolver::new(&self.schema, &self.request, limits.binding).unwrap(),
            self.operations,
            limits,
        )
    }
}

#[test]
fn explicit_actions_registered_after_topology_reach_both_gem_and_skill_owners() {
    for operations in [
        RuleOperationsVersion::V6,
        RuleOperationsVersion::V7,
        RuleOperationsVersion::V8,
        RuleOperationsVersion::V9,
        RuleOperationsVersion::V10,
        RuleOperationsVersion::V11,
    ] {
        let fixture = Fixture::new(operations);
        let mut builder = fixture.builder(PlanLimits::default());
        let owners = builder.discover().unwrap();
        assert!(builder.invocations.is_empty());
        assert!(builder.effects.is_empty());
        assert!(builder.actions.is_empty());
        assert_eq!(owners.len(), 3);
        builder.actions.insert(action(true));
        builder.actions.insert(action(false));
        builder.instantiate_discovered_owners(owners).unwrap();
        builder.check_skill_supply_coverage().unwrap();
        assert!(builder.gaps.is_empty(), "{:?}", builder.gaps);
        assert_eq!(builder.invocations.len(), 2);
        assert_eq!(builder.effects.len(), 2);
        assert_eq!(builder.invocations[0].key.owner, subject(false));
        assert_eq!(builder.invocations[1].key.owner, subject(true));
        let mut scratch = fixture.rules.new_scratch();
        for (index, gem) in [false, true].into_iter().enumerate() {
            assert_eq!(
                builder.effects[index].target,
                BoundEffectTarget::Value {
                    key: PlanValueKey::Stat {
                        entity: ConcreteEntity::Action(Box::new(action(gem))),
                        stat: def("value")
                    }
                }
            );
            assert_eq!(
                builder.invocations[index]
                    .program
                    .evaluate_effect_indexed(0, &[], &mut scratch, 100)
                    .unwrap(),
                EffectDisposition::Applied {
                    value: ParameterValue::Integer(
                        BoundedInteger::new(if gem { 23 } else { 17 }).unwrap()
                    )
                }
            );
        }
    }
}

#[test]
fn deferred_diagnostics_preserve_historical_first_occurrence_order() {
    let fixture = Fixture::new(RuleOperationsVersion::V11);
    let mut builder = fixture.builder(PlanLimits::default());
    let mut owners = Vec::new();
    let provider = root(ProviderRoot::SkillUse(id(10)));
    // This missing owner writes the same diagnostic later encountered by
    // topology. Its earlier occurrence must retain precedence after replay.
    let missing_owner = SchemaSubject::Definition(def::<SkillDefinition>("unmapped").address());
    builder
        .gap(None, None, PlanGapReason::SchemaUnresolved)
        .unwrap();
    builder
        .defer_owner(
            &mut owners,
            missing_owner.clone(),
            &provider,
            &ActorKey::Player,
            None,
        )
        .unwrap();
    builder
        .gap(
            Some(provider.clone()),
            Some(subject(false)),
            PlanGapReason::PartialDeclarations,
        )
        .unwrap();
    builder
        .gap(
            Some(provider.clone()),
            Some(missing_owner.clone()),
            PlanGapReason::MissingPrograms,
        )
        .unwrap();
    builder
        .defer_owner(
            &mut owners,
            subject(false),
            &provider,
            &ActorKey::Player,
            None,
        )
        .unwrap();
    builder
        .gap(None, None, PlanGapReason::UnsupportedRelation)
        .unwrap();
    builder.instantiate_discovered_owners(owners).unwrap();
    assert_eq!(
        builder.gaps,
        vec![
            PlanGap {
                provider: None,
                subject: None,
                reason: PlanGapReason::SchemaUnresolved
            },
            PlanGap {
                provider: Some(provider.clone()),
                subject: Some(missing_owner),
                reason: PlanGapReason::MissingPrograms
            },
            PlanGap {
                provider: Some(provider.clone()),
                subject: Some(subject(false)),
                reason: PlanGapReason::PartialDeclarations
            },
            PlanGap {
                provider: Some(provider),
                subject: Some(subject(false)),
                reason: PlanGapReason::UnsupportedContext
            },
            PlanGap {
                provider: None,
                subject: None,
                reason: PlanGapReason::UnsupportedRelation
            },
        ]
    );
}

#[test]
fn empty_owner_bindings_are_bounded_before_inventory_allocation() {
    let fixture = Fixture::new(RuleOperationsVersion::V11);
    let mut builder = fixture.builder(PlanLimits {
        max_owner_bindings: 1,
        ..Default::default()
    });
    assert!(matches!(
        builder.discover(),
        Err(PlanError::Limit("owner bindings"))
    ));
    assert!(builder.invocations.is_empty());
    assert!(builder.effects.is_empty());
}

#[test]
fn empty_owner_bindings_do_not_consume_the_rule_invocation_limit() {
    let fixture = Fixture::new(RuleOperationsVersion::V11);
    let limits = PlanLimits {
        max_owner_bindings: 3,
        max_invocations: 2,
        ..Default::default()
    };
    limits.validate().unwrap();
    let mut builder = fixture.builder(limits);
    let owners = builder.discover().unwrap();
    // The class has an empty program collection; Skill and Gem each invoke one.
    assert_eq!(owners.len(), 3);
    builder.actions.insert(action(false));
    builder.actions.insert(action(true));
    builder.instantiate_discovered_owners(owners).unwrap();
    assert_eq!(builder.invocations.len(), 2);
    assert_eq!(builder.effects.len(), 2);
    assert!(builder.gaps.is_empty());
}

#[test]
fn owner_binding_limit_is_validated_independently() {
    for max_owner_bindings in [0, PlanLimits::default().max_owner_bindings + 1] {
        assert!(matches!(
            PlanLimits {
                max_owner_bindings,
                ..Default::default()
            }
            .validate(),
            Err(PlanError::Invalid(message)) if message == "invalid owner bindings limit"
        ));
    }
}

#[test]
fn additional_actions_remain_bounded_before_owner_instantiation() {
    let fixture = Fixture::new(RuleOperationsVersion::V11);
    let mut builder = fixture.builder(PlanLimits {
        max_providers: 3,
        ..Default::default()
    });
    let owners = builder.discover().unwrap();
    for mode in ["mode", "extra-a", "extra-b", "extra-c"] {
        let mut selected = action(false);
        selected.mode = def(mode);
        builder.actions.insert(selected);
    }
    assert!(matches!(
        builder.instantiate_discovered_owners(owners),
        Err(PlanError::Limit("actions"))
    ));
    assert!(builder.invocations.is_empty());
}
