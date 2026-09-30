//! Finite declarative receiving relations; no test claims numeric delivery coverage.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_routing::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
    owned_support_inputs::*,
    owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::*, owned_rules::*, owned_schema::*, owned_stages::*, owned_support_inputs::*,
    owned_support_receiving::*, owned_supports::*,
};

fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("receiving-tests", "v1").unwrap()
}
fn id<K: DefinitionDomain>(v: &str) -> DefId<K> {
    DefId::new(ns(), key(v))
}
fn complete<T>(v: Vec<T>) -> DeclaredSet<T> {
    DeclaredSet::complete(v)
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: complete(vec![]),
        choices: complete(vec![]),
        grants: complete(vec![]),
        actors: complete(vec![]),
        skill_grants: complete(vec![]),
        outputs: complete(vec![]),
        sockets: complete(vec![]),
    }
}
fn slot<K: DefinitionDomain>(owner: SlotOwnerDefId, name: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: id(name),
    }
}
fn parent() -> SlotOwnerDefId {
    SlotOwnerDefId::Skill(id("parent"))
}
fn child() -> SlotOwnerDefId {
    SlotOwnerDefId::Skill(id("child"))
}
fn actor() -> SlotOwnerDefId {
    SlotOwnerDefId::Actor(id("minion"))
}
fn active() -> SlotOwnerDefId {
    SlotOwnerDefId::Gem(id("active"))
}
fn actor_grant() -> DeclaredSlot<GrantSlotDefId> {
    slot(parent(), "actor-grant")
}
fn skill_grant() -> DeclaredSlot<GrantSlotDefId> {
    slot(actor(), "skill-grant")
}
fn child_path() -> Vec<DeclaredSlot<GrantSlotDefId>> {
    vec![actor_grant(), skill_grant()]
}
fn owner(gem: &str) -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Gem(id(gem)))
}
fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::GameRules,
            code: key("unconverted"),
        }],
    }
}
fn quality() -> QualityUseSchema {
    QualityUseSchema {
        presence: QualityPresence::Forbidden,
        allowed_kinds: complete(vec![]),
    }
}
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn program(name: &str, context: RuleEntityKind, applicability: bool) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("value"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(true),
            },
        }],
        effects: vec![RuleEffect {
            id: key("effect"),
            when: None,
            effect: if applicability {
                RuleEffectKind::SupportApplicability {
                    applicable: key("value"),
                }
            } else {
                RuleEffectKind::Requirement {
                    satisfied: key("value"),
                    code: key("requirement"),
                }
            },
        }],
    }
}
fn action_endpoint(
    path: Vec<DeclaredSlot<GrantSlotDefId>>,
    output: DeclaredSlot<ActionOutputDefId>,
    admission: SupportAdmissionContext,
) -> SupportReceiverEndpoint {
    SupportReceiverEndpoint::Action {
        path,
        output,
        selection: SupportActionSelection::AllDeclared,
        admission,
    }
}
fn target(
    owner: SupportTargetDefinition,
    endpoints: Vec<SupportReceiverEndpoint>,
) -> SupportTargetReceivingRoles {
    SupportTargetReceivingRoles {
        owner,
        roles: complete(vec![SupportReceivingRoleBinding {
            role: key("actions"),
            endpoints: complete(endpoints),
        }]),
    }
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    rules: OwnedRulePackage,
    preparation: OwnedSupportPreparation,
    stages: OwnedEvaluationStages,
    inputs: OwnedSupportInputBindings,
    input: SupportReceivingInput,
}
impl Fixture {
    fn new() -> Self {
        Self::with(|_| {}, |_| {}, |_| {})
    }
    fn with(
        edit_schema: impl FnOnce(&mut SchemaPackageInput),
        edit_rules: impl FnOnce(&mut RulePackageInput),
        edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    ) -> Self {
        let mut parent_ports = ports();
        parent_ports.grants.members.push(actor_grant());
        parent_ports.actors.members.push(slot(parent(), "actor"));
        parent_ports
            .outputs
            .members
            .push(slot(parent(), "parent-output"));
        let mut child_ports = ports();
        child_ports
            .outputs
            .members
            .push(slot(child(), "child-output"));
        let mut actor_ports = ports();
        actor_ports.grants.members.push(skill_grant());
        actor_ports
            .skill_grants
            .members
            .push(slot(actor(), "ability"));
        let mut active_ports = ports();
        active_ports
            .outputs
            .members
            .push(slot(active(), "active-output"));
        // Explicit cross-owner grant targets are allowed by the Core resolver.
        active_ports
            .grants
            .members
            .push(slot(active(), "actor-crosslink"));
        let mut definitions = vec![
            DefinitionDescriptor::Skill(entry(
                id("parent"),
                SkillSchema {
                    directly_selectable: true,
                    declarations: parent_ports,
                },
            )),
            DefinitionDescriptor::Skill(entry(
                id("child"),
                SkillSchema {
                    directly_selectable: false,
                    declarations: child_ports,
                },
            )),
            DefinitionDescriptor::Actor(entry(
                id("minion"),
                ActorSchema {
                    declarations: actor_ports,
                },
            )),
            DefinitionDescriptor::Gem(entry(
                id("active"),
                GemSchema {
                    level: range(),
                    roles: vec![AuthoredGemRole::SkillUse],
                    skills: complete(vec![]),
                    quality: quality(),
                    declarations: active_ports,
                },
            )),
            DefinitionDescriptor::Unit(entry(
                id("quality-unit"),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
            DefinitionDescriptor::Stat(entry(
                id("level"),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::SupportOrigin],
                },
            )),
            DefinitionDescriptor::Stat(entry(
                id("quality"),
                StatSchema {
                    value: ComputedValueType::Quantity {
                        unit: id("quality-unit"),
                    },
                    targets: vec![RuleEntityKind::SupportOrigin],
                },
            )),
            DefinitionDescriptor::Stat(entry(
                id("flag"),
                StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Skill],
                },
            )),
            DefinitionDescriptor::ActionPart(entry(id("part"), ActionPartSchema {})),
            DefinitionDescriptor::ActionMode(entry(id("mode"), ActionModeSchema {})),
            DefinitionDescriptor::ActionStatSet(entry(id("stat-set"), ActionStatSetSchema {})),
        ];
        for name in ["support-a", "support-b"] {
            definitions.push(DefinitionDescriptor::Gem(entry(
                id(name),
                GemSchema {
                    level: range(),
                    roles: vec![AuthoredGemRole::SupportAssignment],
                    skills: complete(vec![]),
                    quality: quality(),
                    declarations: ports(),
                },
            )));
        }
        let mut slots = vec![
            SlotDescriptor::Grant(entry(
                actor_grant(),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Actor(slot(parent(), "actor")),
                },
            )),
            SlotDescriptor::Grant(entry(
                slot(active(), "actor-crosslink"),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Actor(slot(parent(), "actor")),
                },
            )),
            SlotDescriptor::Actor(entry(
                slot(parent(), "actor"),
                ActorSlotSchema {
                    skills: complete(vec![id("child")]),
                    outputs: complete(vec![]),
                    provider_definition: Some(id("minion")),
                },
            )),
            SlotDescriptor::Grant(entry(
                skill_grant(),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Skill(slot(actor(), "ability")),
                },
            )),
            SlotDescriptor::SkillGrant(entry(
                slot(actor(), "ability"),
                SkillGrantSlotSchema {
                    skill: id("child"),
                    outputs: complete(vec![slot(child(), "child-output")]),
                },
            )),
        ];
        for output in [
            slot(parent(), "parent-output"),
            slot(child(), "child-output"),
            slot(active(), "active-output"),
        ] {
            slots.push(SlotDescriptor::ActionOutput(entry(
                output,
                ActionOutputSchema {
                    actor_role: DeclaredActorRole::ProviderActor,
                    parts: complete(vec![id("part")]),
                    modes: complete(vec![id("mode")]),
                    stat_sets: complete(vec![id("stat-set")]),
                    choices: complete(vec![]),
                },
            )));
        }
        let mut schema = SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_V4,
            namespace: ns(),
            release: key("schema"),
            semantics_version: key("v1"),
            definitions,
            slots,
        };
        edit_schema(&mut schema);
        let schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
        let mut rule_input = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V13),
            definitions: schema.identity().clone(),
            tables: vec![],
            receivers: complete(vec![]),
            owners: ["support-a", "support-b"]
                .into_iter()
                .map(|gem| DefinitionRules {
                    owner: owner(gem),
                    programs: complete(vec![
                        program("actor-app", RuleEntityKind::Actor, true),
                        program("actor-delivery", RuleEntityKind::Actor, false),
                        program("action-app", RuleEntityKind::Action, true),
                        program("action-delivery", RuleEntityKind::Action, false),
                    ]),
                })
                .collect(),
        };
        edit_rules(&mut rule_input);
        let rules = OwnedRulePackage::new(rule_input, &schema, Default::default()).unwrap();
        let routing = OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            &schema,
            Default::default(),
        )
        .unwrap();
        let preparation = OwnedSupportPreparation::new(
            SupportPreparationInput {
                schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
                namespace: ns(),
                release: key("preparation"),
                definitions: schema.identity().clone(),
                rules: *rules.identity(),
                policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                quality_unit: id("quality-unit"),
                types: vec![],
                effects: vec![],
                families: vec![],
                supports: vec![],
            },
            &schema,
            &rules,
            Default::default(),
        )
        .unwrap();
        let mut stages_input = EvaluationStagesInput {
            schema_version: OWNED_EVALUATION_STAGES_VERSION,
            namespace: ns(),
            release: key("stages"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            routing: *routing.identity(),
            stages: vec![
                EvaluationStage {
                    id: key("prepare"),
                    predecessors: vec![],
                },
                EvaluationStage {
                    id: key("applicable"),
                    predecessors: vec![key("prepare")],
                },
                EvaluationStage {
                    id: key("delivery"),
                    predecessors: vec![key("applicable")],
                },
            ],
            programs: complete(
                rules
                    .input()
                    .owners
                    .iter()
                    .flat_map(|owner| {
                        owner.programs.members.iter().map(|p| StagedRuleProgram {
                            owner: owner.owner.clone(),
                            program: p.id.clone(),
                            stage: key(if p.id.as_str().ends_with("app") {
                                "applicable"
                            } else {
                                "delivery"
                            }),
                        })
                    })
                    .collect(),
            ),
            routing_stage: key("delivery"),
            frozen_channels: [
                ("flag", RuleEntityKind::Skill),
                ("level", RuleEntityKind::SupportOrigin),
                ("quality", RuleEntityKind::SupportOrigin),
            ]
            .into_iter()
            .map(|(name, scope)| FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope,
                    stat: id(name),
                },
                stage: key("prepare"),
            })
            .collect(),
        };
        edit_stages(&mut stages_input);
        let stages =
            OwnedEvaluationStages::new(stages_input, &schema, &rules, &routing, Default::default())
                .unwrap();
        let inputs = OwnedSupportInputBindings::new(
            SupportInputBindingsInput {
                schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
                namespace: ns(),
                release: key("inputs"),
                definitions: schema.identity().clone(),
                rules: *rules.identity(),
                preparation: *preparation.identity(),
                stages: *stages.identity(),
                preparation_stage: key("prepare"),
                effective_level: id("level"),
                effective_quality: id("quality"),
                target: SupportTargetInputBindings {
                    skill_types: vec![],
                    minion_types: OptionalTypeInputs {
                        present: id("flag"),
                        members: vec![],
                    },
                    summoner: OptionalTypeContextInputs {
                        present: id("flag"),
                        skill_types: vec![],
                        minion_types: OptionalTypeInputs {
                            present: id("flag"),
                            members: vec![],
                        },
                    },
                    cannot_be_supported: id("flag"),
                    has_gem: id("flag"),
                    from_item: id("flag"),
                    is_player_actor: id("flag"),
                },
            },
            &schema,
            &rules,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap();
        let mut parent_target = target(
            SupportTargetDefinition::Skill(id("parent")),
            vec![
                action_endpoint(
                    vec![],
                    slot(parent(), "parent-output"),
                    SupportAdmissionContext::AssignedSkill,
                ),
                action_endpoint(
                    child_path(),
                    slot(child(), "child-output"),
                    SupportAdmissionContext::ReceivingSkill {
                        summoner_path: Some(vec![]),
                    },
                ),
            ],
        );
        parent_target
            .roles
            .members
            .push(SupportReceivingRoleBinding {
                role: key("actors"),
                endpoints: complete(vec![SupportReceiverEndpoint::Actor {
                    path: vec![actor_grant()],
                    admission: SupportAdmissionContext::AssignedSkill,
                }]),
            });
        let input = SupportReceivingInput {
            schema_version: OWNED_SUPPORT_RECEIVING_VERSION,
            namespace: ns(),
            release: key("receiving"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            preparation: *preparation.identity(),
            inputs: *inputs.identity(),
            stages: *stages.identity(),
            roles: vec![
                SupportReceivingRole {
                    id: key("actions"),
                    kind: SupportReceiverKind::Action,
                },
                SupportReceivingRole {
                    id: key("actors"),
                    kind: SupportReceiverKind::Actor,
                },
            ],
            targets: vec![
                parent_target,
                target(
                    SupportTargetDefinition::Gem(id("active")),
                    vec![action_endpoint(
                        vec![],
                        slot(active(), "active-output"),
                        SupportAdmissionContext::AssignedSkill,
                    )],
                ),
            ],
            supports: ["support-a", "support-b"]
                .into_iter()
                .map(|gem| SupportReceivingEntry {
                    gem: id(gem),
                    receivers: complete(vec![
                        SupportRolePrograms {
                            role: key("actions"),
                            applicability: key("action-app"),
                            delivery: vec![key("action-delivery")],
                        },
                        SupportRolePrograms {
                            role: key("actors"),
                            applicability: key("actor-app"),
                            delivery: vec![key("actor-delivery")],
                        },
                    ]),
                })
                .collect(),
        };
        Self {
            schema,
            rules,
            preparation,
            stages,
            inputs,
            input,
        }
    }
    fn build(
        &self,
        input: SupportReceivingInput,
    ) -> Result<OwnedSupportReceiving, SupportReceivingStorageError> {
        self.build_with(input, Default::default())
    }
    fn build_with(
        &self,
        input: SupportReceivingInput,
        limits: SupportReceivingStorageLimits,
    ) -> Result<OwnedSupportReceiving, SupportReceivingStorageError> {
        OwnedSupportReceiving::new(
            input,
            &self.schema,
            &self.rules,
            &self.preparation,
            &self.inputs,
            &self.stages,
            limits,
        )
    }
    fn bad(&self, input: SupportReceivingInput, text: &str) {
        let err = self.build(input).unwrap_err();
        assert!(err.to_string().contains(text), "{err}");
    }
}

#[test]
fn factored_roles_roundtrip_and_gem_roots_never_infer_potential_skill() {
    let f = Fixture::new();
    let package = f.build(f.input.clone()).unwrap();
    assert!(package.declarations_complete());
    assert_eq!(package.input().targets.len(), 2);
    assert_eq!(package.input().supports.len(), 2);
    assert_eq!(package.resources().expanded_endpoints, 4);
    assert!(
        package
            .target_for(&SupportTargetDefinition::Gem(id("active")))
            .is_some()
    );
    assert!(package.support_for(&id("support-a")).is_some());
    assert_eq!(
        package.role(&key("actions")).unwrap().kind,
        SupportReceiverKind::Action
    );
    let mut reversed = f.input.clone();
    reversed.targets.reverse();
    reversed.supports.reverse();
    reversed.roles.reverse();
    for target in &mut reversed.targets {
        target.roles.members.reverse();
        for role in &mut target.roles.members {
            role.endpoints.members.reverse();
        }
    }
    for support in &mut reversed.supports {
        support.receivers.members.reverse();
    }
    assert_eq!(package.identity(), f.build(reversed).unwrap().identity());
    let bytes = encode_support_receiving(&package, Default::default()).unwrap();
    let decoded = decode_support_receiving(
        &bytes,
        &f.schema,
        &f.rules,
        &f.preparation,
        &f.inputs,
        &f.stages,
        Default::default(),
    )
    .unwrap();
    assert_eq!(package.input(), decoded.input());
    assert_eq!(package.identity(), decoded.identity());
}

#[test]
fn exact_paths_allow_declared_crosslinks_but_reject_unrelated_steps() {
    let f = Fixture::new();
    let mut input = f.input.clone();
    input.targets[1].roles.members[0]
        .endpoints
        .members
        .push(action_endpoint(
            vec![slot(active(), "actor-crosslink"), skill_grant()],
            slot(child(), "child-output"),
            SupportAdmissionContext::ReceivingSkill {
                summoner_path: Some(vec![]),
            },
        ));
    assert!(f.build(input).is_ok());
    let mut bad = f.input.clone();
    let SupportReceiverEndpoint::Action { path, .. } =
        &mut bad.targets[0].roles.members[0].endpoints.members[1]
    else {
        panic!()
    };
    path.reverse();
    f.bad(bad, "unrelated declaration");
}

#[test]
fn gem_root_explicit_skill_declarations_do_not_select_a_generated_skill() {
    let f = Fixture::with(
        |schema| {
            for descriptor in &mut schema.definitions {
                if let DefinitionDescriptor::Gem(DefinitionEntry {
                    id: gem,
                    schema: SchemaState::Known(gem_schema),
                }) = descriptor
                    && *gem == id("active")
                {
                    gem_schema.skills.members.push(id("parent"));
                }
            }
        },
        |_| {},
        |_| {},
    );
    let mut input = f.input.clone();
    input.targets[1].roles.members[0].endpoints.members = vec![
        action_endpoint(
            vec![],
            slot(parent(), "parent-output"),
            SupportAdmissionContext::AssignedSkill,
        ),
        action_endpoint(
            child_path(),
            slot(child(), "child-output"),
            SupportAdmissionContext::ReceivingSkill {
                summoner_path: Some(vec![]),
            },
        ),
    ];
    assert!(f.build(input.clone()).is_ok());
    // Potential membership alone never establishes a selected supplied Skill.
    let SupportReceiverEndpoint::Action { admission, .. } =
        &mut input.targets[1].roles.members[0].endpoints.members[0]
    else {
        panic!()
    };
    *admission = SupportAdmissionContext::ReceivingSkill {
        summoner_path: None,
    };
    f.bad(input, "exact generated skill");
    let unrelated = Fixture::new();
    let mut input = unrelated.input.clone();
    input.targets[1].roles.members[0].endpoints.members = vec![action_endpoint(
        vec![],
        slot(parent(), "parent-output"),
        SupportAdmissionContext::AssignedSkill,
    )];
    unrelated.bad(input, "not declared");
}

#[test]
fn normalized_actor_aliases_and_overlapping_action_variants_reject() {
    let f = Fixture::new();
    let mut input = f.input.clone();
    input.targets[0].roles.members[1]
        .endpoints
        .members
        .push(SupportReceiverEndpoint::Actor {
            path: child_path(),
            admission: SupportAdmissionContext::AssignedSkill,
        });
    f.bad(input, "duplicate normalized");
    let mut input = f.input.clone();
    let mut exact = input.targets[0].roles.members[0].endpoints.members[0].clone();
    let SupportReceiverEndpoint::Action { selection, .. } = &mut exact else {
        panic!()
    };
    *selection = SupportActionSelection::Exact(Box::new(SupportActionVariant {
        part: id("part"),
        mode: id("mode"),
        stat_set: id("stat-set"),
    }));
    let wire = serde_json::to_value(&selection).unwrap();
    assert_eq!(wire["kind"], "exact");
    assert!(wire.get("part").is_some());
    assert!(wire.get("mode").is_some());
    assert!(wire.get("stat_set").is_some());
    assert_eq!(
        serde_json::from_value::<SupportActionSelection>(wire).unwrap(),
        *selection
    );
    input.targets[0].roles.members[0]
        .endpoints
        .members
        .push(exact);
    f.bad(input, "duplicate normalized");
}

#[test]
fn partial_local_and_schema_membership_remain_diagnostic() {
    let f = Fixture::new();
    let mut input = f.input.clone();
    input.targets[0].roles.closure = partial(SchemaSubject::Definition(DefinitionAddress::Skill(
        id("parent"),
    )));
    input.supports[0].receivers.members.pop();
    input.supports[0].receivers.closure = partial(owner("support-a"));
    let package = f.build(input).unwrap();
    assert!(!package.declarations_complete());
    assert!(
        package
            .target_for(&SupportTargetDefinition::Skill(id("absent")))
            .is_none()
    );
    let f = Fixture::with(
        |schema| {
            for descriptor in &mut schema.definitions {
                if let DefinitionDescriptor::Skill(DefinitionEntry {
                    id: skill,
                    schema: SchemaState::Known(skill_schema),
                }) = descriptor
                    && *skill == id("parent")
                {
                    skill_schema.declarations.grants.closure =
                        partial(SchemaSubject::Definition(skill.address()));
                }
            }
        },
        |_| {},
        |_| {},
    );
    assert!(!f.build(f.input.clone()).unwrap().declarations_complete());
}

#[test]
fn admission_is_explicit_and_does_not_infer_summoner_or_actor_skill() {
    let f = Fixture::new();
    for path in [child_path(), vec![actor_grant()]] {
        let mut input = f.input.clone();
        let SupportReceiverEndpoint::Action { admission, .. } =
            &mut input.targets[0].roles.members[0].endpoints.members[1]
        else {
            panic!()
        };
        *admission = SupportAdmissionContext::ReceivingSkill {
            summoner_path: Some(path),
        };
        f.bad(input, "summoner");
    }
    let mut input = f.input.clone();
    let SupportReceiverEndpoint::Actor { admission, .. } =
        &mut input.targets[0].roles.members[1].endpoints.members[0]
    else {
        panic!()
    };
    *admission = SupportAdmissionContext::ReceivingSkill {
        summoner_path: None,
    };
    f.bad(input, "exact generated skill");
    let omitted = serde_json::json!({"kind":"receiving_skill"});
    assert!(serde_json::from_value::<SupportAdmissionContext>(omitted).is_err());
    let explicit = serde_json::json!({"kind":"receiving_skill", "summoner_path":null});
    assert_eq!(
        serde_json::from_value::<SupportAdmissionContext>(explicit).unwrap(),
        SupportAdmissionContext::ReceivingSkill {
            summoner_path: None
        }
    );
}

#[test]
fn complete_inventory_must_classify_known_owner_programs() {
    let f = Fixture::new();
    let mut input = f.input.clone();
    input.supports[0].receivers.members.pop();
    f.bad(input, "omits known");
    let mut input = f.input.clone();
    input.supports[0].receivers.members[0]
        .delivery
        .push(key("action-delivery"));
    f.bad(input, "duplicate delivery");
    let mut input = f.input.clone();
    input.supports[0].receivers.members[0].applicability = key("actor-app");
    f.bad(input, "receiving context");
}

#[test]
fn applicability_and_delivery_programs_retain_scope_and_stage_contracts() {
    let f = Fixture::with(
        |_| {},
        |rules| {
            rules.owners[0].programs.members[0].reads.push(RuleRead {
                id: key("ambiguous-skill"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Skill,
                    stat: id("flag"),
                },
            });
        },
        |_| {},
    );
    f.bad(f.input.clone(), "no unique receiving skill");
    let f = Fixture::with(
        |_| {},
        |rules| {
            rules.owners[0].programs.members[2].effects[0].when = Some(key("value"));
        },
        |_| {},
    );
    f.bad(f.input.clone(), "unguarded");
    let f = Fixture::with(
        |_| {},
        |rules| {
            rules.owners[0].programs.members[3].effects[0].effect = RuleEffectKind::Derive {
                entity: RuleEntity::Actor,
                stat: id("flag"),
                value: key("value"),
            };
        },
        |_| {},
    );
    f.bad(f.input.clone(), "receiving scope");
    let f = Fixture::with(
        |_| {},
        |_| {},
        |stages| {
            stages
                .programs
                .members
                .iter_mut()
                .find(|p| p.owner == owner("support-a") && p.program == key("action-delivery"))
                .unwrap()
                .stage = key("applicable");
        },
    );
    f.bad(f.input.clone(), "delivery must follow");
}

#[test]
fn exact_bindings_and_frozen_operation_versions_are_checked() {
    let f = Fixture::new();
    for which in 0..3 {
        let mut input = f.input.clone();
        match which {
            0 => input.rules = *f.stages.identity(),
            1 => input.inputs = *f.preparation.identity(),
            _ => input.preparation = *f.inputs.identity(),
        }
        f.bad(input, "binding mismatch");
    }
    let f = Fixture::with(
        |_| {},
        |rules| rules.operations_version = key(OWNED_RULE_OPERATIONS_V12),
        |_| {},
    );
    f.bad(f.input.clone(), "v13");
}

#[test]
fn allocation_expansion_and_wire_limits_are_checked_on_new_decode_and_encode() {
    let f = Fixture::new();
    let package = f.build(f.input.clone()).unwrap();
    for limits in [
        SupportReceivingStorageLimits {
            max_entries: 1,
            ..Default::default()
        },
        SupportReceivingStorageLimits {
            max_path_depth: 1,
            ..Default::default()
        },
        SupportReceivingStorageLimits {
            max_expanded_endpoints: 1,
            ..Default::default()
        },
        SupportReceivingStorageLimits {
            max_work: 1,
            ..Default::default()
        },
        SupportReceivingStorageLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(f.build_with(f.input.clone(), limits).is_err());
        assert!(encode_support_receiving(&package, limits).is_err());
    }
    let bytes = encode_support_receiving(&package, Default::default()).unwrap();
    assert!(
        decode_support_receiving(
            &bytes,
            &f.schema,
            &f.rules,
            &f.preparation,
            &f.inputs,
            &f.stages,
            SupportReceivingStorageLimits {
                max_wire_bytes: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn empty_cartesian_dimension_has_no_unbounded_iteration_or_fabricated_receiver() {
    let f = Fixture::with(
        |schema| {
            for slot in &mut schema.slots {
                if let SlotDescriptor::ActionOutput(DefinitionEntry {
                    schema: SchemaState::Known(output),
                    ..
                }) = slot
                {
                    output.stat_sets.members.clear();
                }
            }
        },
        |_| {},
        |_| {},
    );
    let package = f.build(f.input.clone()).unwrap();
    assert_eq!(package.resources().expanded_endpoints, 1); // the exact Actor only
}
