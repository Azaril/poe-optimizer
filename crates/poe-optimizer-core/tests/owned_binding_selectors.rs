use poe_optimizer_core::{
    build_identity::*, data::DataIdentity, owned_binding::*, owned_build::*, owned_definitions::*,
    owned_schema::*,
};
use std::collections::BTreeMap;

fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("selector-game", "v1").unwrap()
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([19; 16]), n).unwrap())
}
fn declared<K: DefinitionDomain>(owner: SlotOwnerDefId, key: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: def(key),
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
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn quality() -> QualityUseSchema {
    QualityUseSchema {
        presence: QualityPresence::Forbidden,
        allowed_kinds: empty(),
    }
}
fn root() -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(2)),
        grant_path: vec![],
    }
}
fn skill_owner() -> SlotOwnerDefId {
    SlotOwnerDefId::Skill(def("skill"))
}
fn action(
    provider: ProviderKey,
    actor: ActorKey,
    output: DeclaredSlot<ActionOutputDefId>,
) -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor,
            provider,
            output,
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("stats"),
    }
}
fn request(id: &str, target: MetricTarget) -> MetricRequest {
    MetricRequest {
        id: QueryId::new(id).unwrap(),
        metric: def("metric"),
        target,
    }
}
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::StaticLinks,
        code: OwnedDefinitionKey::new("pending").unwrap(),
    }
}

struct Index {
    namespace: GameVersionNamespace,
    identity: DataIdentity,
    definitions: BTreeMap<DefinitionAddress, DefinitionDescriptor>,
    slots: BTreeMap<SlotAddress, SlotDescriptor>,
}
impl DefinitionSchemaIndex for Index {
    fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        self.definitions.get(address)
    }
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        self.slots.get(address)
    }
}
impl Index {
    fn put_definition(&mut self, value: DefinitionDescriptor) {
        self.definitions.insert(value.address(), value);
    }
    fn put_slot(&mut self, value: SlotDescriptor) {
        self.slots.insert(value.address(), value);
    }
    fn declarations(&mut self, owner: &SlotOwnerDefId) -> &mut DeclaredSlots {
        let address = match owner {
            SlotOwnerDefId::Class(id) => id.address(),
            SlotOwnerDefId::Skill(id) => id.address(),
            SlotOwnerDefId::ItemTemplate(id) => id.address(),
            SlotOwnerDefId::Gem(id) => id.address(),
            SlotOwnerDefId::PassiveNode(id) => id.address(),
            _ => panic!("fixture owner"),
        };
        match self.definitions.get_mut(&address).unwrap() {
            DefinitionDescriptor::Class(DefinitionEntry {
                schema: SchemaState::Known(value),
                ..
            }) => &mut value.declarations,
            DefinitionDescriptor::Skill(DefinitionEntry {
                schema: SchemaState::Known(value),
                ..
            }) => &mut value.declarations,
            DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                schema: SchemaState::Known(value),
                ..
            }) => &mut value.declarations,
            DefinitionDescriptor::Gem(DefinitionEntry {
                schema: SchemaState::Known(value),
                ..
            }) => &mut value.declarations,
            DefinitionDescriptor::PassiveNode(DefinitionEntry {
                schema: SchemaState::Known(value),
                ..
            }) => &mut value.declarations,
            _ => panic!("known declaring fixture"),
        }
    }
    fn output(
        &mut self,
        owner: SlotOwnerDefId,
        name: &str,
        actor_role: DeclaredActorRole,
    ) -> DeclaredSlot<ActionOutputDefId> {
        let slot = declared(owner.clone(), name);
        self.declarations(&owner).outputs.members.push(slot.clone());
        self.put_slot(SlotDescriptor::ActionOutput(known(
            slot.clone(),
            ActionOutputSchema {
                actor_role,
                parts: DeclaredSet::complete(vec![def("part")]),
                modes: DeclaredSet::complete(vec![def("mode")]),
                stat_sets: DeclaredSet::complete(vec![def("stats")]),
                choices: empty(),
            },
        )));
        slot
    }
    fn choice(
        &mut self,
        owner: SlotOwnerDefId,
        name: &str,
        scopes: Vec<ChoiceOwnerScope>,
        presence: SlotPresence,
    ) -> DeclaredSlot<ChoiceSlotDefId> {
        let slot = declared(owner.clone(), name);
        self.declarations(&owner).choices.members.push(slot.clone());
        self.put_slot(SlotDescriptor::Choice(known(
            slot.clone(),
            ChoiceSlotSchema {
                value: ValueSchema::Boolean,
                presence,
                owners: scopes,
            },
        )));
        slot
    }
    fn actor_grant(
        &mut self,
        owner: SlotOwnerDefId,
        name: &str,
        outputs: Vec<DeclaredSlot<ActionOutputDefId>>,
    ) -> (DeclaredSlot<GrantSlotDefId>, DeclaredSlot<ActorSlotDefId>) {
        let actor = declared(owner.clone(), &format!("{name}.actor"));
        self.declarations(&owner).actors.members.push(actor.clone());
        self.put_slot(SlotDescriptor::Actor(known(
            actor.clone(),
            ActorSlotSchema {
                skills: empty(),
                outputs: DeclaredSet::complete(outputs),
            },
        )));
        let grant = declared(owner.clone(), &format!("{name}.grant"));
        self.declarations(&owner).grants.members.push(grant.clone());
        self.put_slot(SlotDescriptor::Grant(known(
            grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Actor(actor.clone()),
            },
        )));
        (grant, actor)
    }
}
struct Fixture {
    index: Index,
    build: BuildInput,
    scenario: ScenarioInput,
    queries: Vec<MetricRequest>,
}
impl Fixture {
    fn new() -> Self {
        let mut index = Index {
            namespace: namespace(),
            identity: DataIdentity {
                game: "selector-game".into(),
                release: "fixture".into(),
                schema_version: 1,
                content_sha256: "d".repeat(64),
                semantics_version: "v1".into(),
            },
            definitions: BTreeMap::new(),
            slots: BTreeMap::new(),
        };
        index.put_definition(DefinitionDescriptor::Class(known(
            def("class"),
            ClassSchema {
                level: range(),
                ascendancies: empty(),
                declarations: declarations(),
            },
        )));
        index.put_definition(DefinitionDescriptor::Skill(known(
            def("skill"),
            SkillSchema {
                directly_selectable: true,
                declarations: declarations(),
            },
        )));
        index.put_definition(DefinitionDescriptor::Encounter(known(
            def("encounter"),
            EncounterSchema {
                enemy_level: range(),
                external_inputs: empty(),
            },
        )));
        index.put_definition(DefinitionDescriptor::Unit(known(
            def("unit"),
            UnitSchema {
                dimension: UnitDimension::Count,
            },
        )));
        index.put_definition(DefinitionDescriptor::Metric(known(
            def("metric"),
            MetricSchema {
                targets: vec![MetricTargetKind::Actor, MetricTargetKind::Action],
                unit: def("unit"),
                actor_roles: vec![MetricActorRole::Player, MetricActorRole::Owned],
                provider_roles: vec![
                    ProviderRole::Character,
                    ProviderRole::EquipmentUse,
                    ProviderRole::SkillUse,
                    ProviderRole::SupportAssignment,
                    ProviderRole::Allocation,
                ],
            },
        )));
        index.put_definition(DefinitionDescriptor::ActionPart(known(
            def("part"),
            ActionPartSchema {},
        )));
        index.put_definition(DefinitionDescriptor::ActionMode(known(
            def("mode"),
            ActionModeSchema {},
        )));
        index.put_definition(DefinitionDescriptor::ActionStatSet(known(
            def("stats"),
            ActionStatSetSchema {},
        )));
        Self {
            index,
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([19; 16]),
                    100,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: namespace(),
                character: CharacterSpec {
                    class: def("class"),
                    ascendancy: None,
                    level: 20,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1), occurrence(9)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                gems: vec![],
                equipment: vec![],
                allocations: vec![],
                skills: vec![SkillUse {
                    id: occurrence(2),
                    source: AuthoredSkillSource::Direct(def("skill")),
                    enabled: true,
                    scope: LoadoutScope::Shared,
                }],
                supports: vec![],
                payload_links: vec![],
                choices: vec![],
            },
            scenario: ScenarioInput {
                game_version: namespace(),
                enemy: EnemySpec {
                    encounter: def("encounter"),
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
            queries: vec![],
        }
    }
    fn owned(&self) -> OwnedEvaluationRequest {
        let limits = OwnedInputLimits::default();
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), limits).unwrap(),
            ScenarioSpec::new(self.scenario.clone(), limits).unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: namespace(),
                    requests: self.queries.clone(),
                },
                limits,
            )
            .unwrap(),
            limits,
        )
        .unwrap()
    }
    fn bind(&self) -> DefinitionBindingReport {
        bind_owned_request(&self.index, &self.owned(), BindingLimits::default()).unwrap()
    }
    fn equipment(&mut self) {
        self.index
            .put_definition(DefinitionDescriptor::EquipmentSlot(known(
                def("equipment"),
                EquipmentSlotSchema {
                    scope: ScopePolicy::Either,
                },
            )));
        self.index
            .put_definition(DefinitionDescriptor::ItemTemplate(known(
                def("item"),
                ItemTemplateSchema {
                    item_level: range(),
                    equipment_slots: DeclaredSet::complete(vec![def("equipment")]),
                    socket_destinations: empty(),
                    modifiers: empty(),
                    quality: quality(),
                    declarations: declarations(),
                },
            )));
        self.build.items.push(ItemRecord {
            id: occurrence(3),
            template: def("item"),
            parameters: vec![],
            item_level: Some(20),
            quality: None,
            modifier_order: vec![],
            modifiers: vec![],
        });
        self.build.equipment.push(EquipmentUse {
            id: occurrence(4),
            item: occurrence(3),
            destination: EquipmentDestination::CharacterSlot(def("equipment")),
            scope: LoadoutScope::Shared,
        });
    }
    fn allocation(&mut self) {
        self.index
            .put_definition(DefinitionDescriptor::PointPool(known(
                def("pool"),
                PointPoolSchema {
                    scope: PointPoolScope::Shared,
                },
            )));
        self.index
            .put_definition(DefinitionDescriptor::PassiveNode(known(
                def("node"),
                PassiveNodeSchema {
                    pools: DeclaredSet::complete(vec![def("pool")]),
                    adjacent: empty(),
                    declarations: declarations(),
                },
            )));
        self.build.allocations.push(Allocation {
            id: occurrence(5),
            node: def("node"),
            pool: def("pool"),
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        });
    }
}
fn assert_valid(report: &DefinitionBindingReport) {
    assert_eq!(
        report.schema(),
        SchemaBindingStatus::Valid,
        "{:?}",
        report.issues()
    );
}

#[test]
fn ordered_queries_keep_schema_and_resolution_separate_and_player_role_is_character() {
    let mut f = Fixture::new();
    let output = f
        .index
        .output(skill_owner(), "output", DeclaredActorRole::Player);
    f.queries = vec![
        request("z-player", MetricTarget::Actor(ActorKey::Player)),
        request(
            "a-action",
            MetricTarget::Action(Box::new(action(root(), ActorKey::Player, output))),
        ),
    ];
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report
            .queries()
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["z-player", "a-action"]
    );
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::SchemaBound
    );
    assert_eq!(
        report.queries()[1].selector,
        SelectorBindingStatus::PendingResolution
    );
    let DefinitionDescriptor::Metric(DefinitionEntry {
        schema: SchemaState::Known(metric),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<MetricDefinition>("metric").address())
        .unwrap()
    else {
        panic!()
    };
    metric.provider_roles = vec![ProviderRole::Character];
    let report = f.bind();
    assert_eq!(report.queries()[0].schema, SchemaBindingStatus::Valid);
    assert_eq!(report.queries()[1].schema, SchemaBindingStatus::Invalid);
    assert_eq!(
        report.queries()[1].selector,
        SelectorBindingStatus::PendingResolution
    );
    assert!(report.issues().iter().any(|issue| issue.site.location
        == BindingLocation::Query(QueryId::new("a-action").unwrap())
        && issue.code == BindingIssueCode::IncompatibleRole));
}

#[test]
fn entered_actor_keeps_parent_identity_and_does_not_reopen_siblings() {
    let mut f = Fixture::new();
    let output = f.index.output(
        skill_owner(),
        "owned-output",
        DeclaredActorRole::ProviderActor,
    );
    let sibling = f.index.output(
        skill_owner(),
        "sibling-output",
        DeclaredActorRole::ProviderActor,
    );
    let (grant, actor) = f
        .index
        .actor_grant(skill_owner(), "summon", vec![output.clone()]);
    let parent = root();
    let entered = ProviderKey {
        root: parent.root.clone(),
        grant_path: vec![grant],
    };
    let owned = ActorKey::Owned(Box::new(OwnedActorKey {
        provider: parent,
        slot: actor.clone(),
    }));
    f.queries = vec![
        request("actor", MetricTarget::Actor(owned.clone())),
        request(
            "action",
            MetricTarget::Action(Box::new(action(entered.clone(), owned.clone(), output))),
        ),
        request(
            "sibling",
            MetricTarget::Action(Box::new(action(entered.clone(), owned, sibling))),
        ),
        request(
            "rebased",
            MetricTarget::Actor(ActorKey::Owned(Box::new(OwnedActorKey {
                provider: entered,
                slot: actor,
            }))),
        ),
    ];
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report
            .queries()
            .iter()
            .map(|row| row.selector)
            .collect::<Vec<_>>(),
        vec![
            SelectorBindingStatus::PendingResolution,
            SelectorBindingStatus::PendingResolution,
            SelectorBindingStatus::Unavailable,
            SelectorBindingStatus::Unavailable
        ]
    );
}

#[test]
fn skill_then_actor_grant_preserves_root_role_and_explicit_cross_declaration_output() {
    let mut f = Fixture::new();
    f.equipment();
    let item_owner = SlotOwnerDefId::ItemTemplate(def("item"));
    f.index.put_definition(DefinitionDescriptor::Skill(known(
        def("granted-skill"),
        SkillSchema {
            directly_selectable: false,
            declarations: declarations(),
        },
    )));
    let cross_output = f.index.output(
        skill_owner(),
        "cross-output",
        DeclaredActorRole::ProviderActor,
    );
    let (actor_grant, actor) = f.index.actor_grant(
        SlotOwnerDefId::Skill(def("granted-skill")),
        "nested",
        vec![cross_output.clone()],
    );
    let skill_slot = declared(item_owner.clone(), "generated-skill");
    f.index
        .declarations(&item_owner)
        .skill_grants
        .members
        .push(skill_slot.clone());
    f.index.put_slot(SlotDescriptor::SkillGrant(known(
        skill_slot.clone(),
        SkillGrantSlotSchema {
            skill: def("granted-skill"),
            outputs: empty(),
        },
    )));
    let grant = declared(item_owner.clone(), "skill-grant");
    f.index
        .declarations(&item_owner)
        .grants
        .members
        .push(grant.clone());
    f.index.put_slot(SlotDescriptor::Grant(known(
        grant.clone(),
        GrantSlotSchema {
            provider_roles: vec![ProviderRole::EquipmentUse],
            target: GrantTarget::Skill(skill_slot),
        },
    )));
    let SlotDescriptor::Grant(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .slots
        .get_mut(&GrantSlotDefId::address(&actor_grant))
        .unwrap()
    else {
        panic!()
    };
    schema.provider_roles = vec![ProviderRole::EquipmentUse];
    let parent = ProviderKey {
        root: ProviderRoot::EquipmentUse(occurrence(4)),
        grant_path: vec![grant.clone()],
    };
    let entered = ProviderKey {
        root: ProviderRoot::EquipmentUse(occurrence(4)),
        grant_path: vec![grant, actor_grant],
    };
    let actor = ActorKey::Owned(Box::new(OwnedActorKey {
        provider: parent,
        slot: actor,
    }));
    f.queries.push(request(
        "nested",
        MetricTarget::Action(Box::new(action(entered, actor, cross_output))),
    ));
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::PendingResolution
    );
}

#[test]
fn entered_skill_exposes_only_its_explicit_output_subset() {
    let mut f = Fixture::new();
    let allowed = f
        .index
        .output(skill_owner(), "allowed", DeclaredActorRole::Player);
    let other = f
        .index
        .output(skill_owner(), "other", DeclaredActorRole::Player);
    let skill = declared(skill_owner(), "generated");
    f.index
        .declarations(&skill_owner())
        .skill_grants
        .members
        .push(skill.clone());
    f.index.put_slot(SlotDescriptor::SkillGrant(known(
        skill.clone(),
        SkillGrantSlotSchema {
            skill: def("skill"),
            outputs: DeclaredSet::complete(vec![allowed.clone()]),
        },
    )));
    let grant = declared(skill_owner(), "grant");
    f.index
        .declarations(&skill_owner())
        .grants
        .members
        .push(grant.clone());
    f.index.put_slot(SlotDescriptor::Grant(known(
        grant.clone(),
        GrantSlotSchema {
            provider_roles: vec![ProviderRole::SkillUse],
            target: GrantTarget::Skill(skill),
        },
    )));
    let provider = ProviderKey {
        root: root().root,
        grant_path: vec![grant],
    };
    f.queries = vec![
        request(
            "allowed",
            MetricTarget::Action(Box::new(action(
                provider.clone(),
                ActorKey::Player,
                allowed,
            ))),
        ),
        request(
            "other",
            MetricTarget::Action(Box::new(action(provider, ActorKey::Player, other))),
        ),
    ];
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::PendingResolution
    );
    assert_eq!(
        report.queries()[1].selector,
        SelectorBindingStatus::Unavailable
    );
}

#[test]
fn saved_absence_partial_membership_and_unmapped_schema_remain_distinct() {
    let mut f = Fixture::new();
    let output = f
        .index
        .output(skill_owner(), "output", DeclaredActorRole::Player);
    f.queries.push(request(
        "selected",
        MetricTarget::Action(Box::new(action(root(), ActorKey::Player, output.clone()))),
    ));
    f.index.declarations(&skill_owner()).outputs.members.clear();
    assert_eq!(
        f.bind().queries()[0].selector,
        SelectorBindingStatus::Unavailable
    );
    f.index.declarations(&skill_owner()).outputs.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            def::<SkillDefinition>("skill").address(),
        ))],
    };
    let report = f.bind();
    assert_eq!(report.queries()[0].schema, SchemaBindingStatus::Unresolved);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::Unresolved
    );
    f.index.declarations(&skill_owner()).outputs = DeclaredSet::complete(vec![output.clone()]);
    let SlotDescriptor::ActionOutput(entry) = f
        .index
        .slots
        .get_mut(&ActionOutputDefId::address(&output))
        .unwrap()
    else {
        panic!()
    };
    entry.schema = SchemaState::Unmapped {
        gaps: vec![gap(SchemaSubject::Slot(ActionOutputDefId::address(
            &output,
        )))],
    };
    let report = f.bind();
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::Unresolved
    );
    f.build.skills.clear();
    let report = f.bind();
    assert_eq!(report.queries()[0].schema, SchemaBindingStatus::Valid);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::Unavailable
    );
}

#[test]
fn disabled_and_inactive_roots_are_unavailable_without_erasing_authored_choices() {
    let mut f = Fixture::new();
    let output = f
        .index
        .output(skill_owner(), "output", DeclaredActorRole::Player);
    let choice = f.index.choice(
        skill_owner(),
        "choice",
        vec![ChoiceOwnerScope::Skill],
        SlotPresence::RequiredOnce,
    );
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Skill(SkillTarget::Authored(occurrence(2))),
        choice: ChoiceSelection {
            slot: choice,
            value: ParameterValue::Boolean(false),
        },
    });
    f.queries.push(request(
        "selected",
        MetricTarget::Action(Box::new(action(root(), ActorKey::Player, output))),
    ));
    f.build.skills[0].enabled = false;
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::Unavailable
    );
    f.build.skills[0].enabled = true;
    f.build.skills[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(9)],
    };
    let report = f.bind();
    assert_valid(&report);
    assert!(
        report
            .issues()
            .iter()
            .any(|issue| issue.code == BindingIssueCode::InactiveLoadout)
    );
}

#[test]
fn empty_provider_choice_aliases_have_one_required_instance_and_one_scope() {
    for variant in 0..4 {
        for use_provider in [false, true] {
            for provider_scope_only in [false, true] {
                let mut f = Fixture::new();
                f.equipment();
                f.allocation();
                let (direct, provider, declaration, direct_scope, provider_scope) = match variant {
                    0 => (
                        ChoiceOwner::Character,
                        ProviderRoot::Character,
                        SlotOwnerDefId::Class(def("class")),
                        ChoiceOwnerScope::Character,
                        ProviderRole::Character,
                    ),
                    1 => (
                        ChoiceOwner::EquipmentUse(occurrence(4)),
                        ProviderRoot::EquipmentUse(occurrence(4)),
                        SlotOwnerDefId::ItemTemplate(def("item")),
                        ChoiceOwnerScope::EquipmentUse,
                        ProviderRole::EquipmentUse,
                    ),
                    2 => (
                        ChoiceOwner::Allocation(occurrence(5)),
                        ProviderRoot::Allocation(occurrence(5)),
                        SlotOwnerDefId::PassiveNode(def("node")),
                        ChoiceOwnerScope::Allocation,
                        ProviderRole::Allocation,
                    ),
                    _ => (
                        ChoiceOwner::Skill(SkillTarget::Authored(occurrence(2))),
                        root().root,
                        skill_owner(),
                        ChoiceOwnerScope::Skill,
                        ProviderRole::SkillUse,
                    ),
                };
                let slot = f.index.choice(
                    declaration,
                    "choice",
                    vec![if provider_scope_only {
                        ChoiceOwnerScope::Provider(provider_scope)
                    } else {
                        direct_scope
                    }],
                    SlotPresence::RequiredOnce,
                );
                let alias = ChoiceOwner::Provider(ProviderKey {
                    root: provider,
                    grant_path: vec![],
                });
                f.build.choices.push(MechanicChoice {
                    owner: if use_provider {
                        alias.clone()
                    } else {
                        direct.clone()
                    },
                    choice: ChoiceSelection {
                        slot: slot.clone(),
                        value: ParameterValue::Boolean(false),
                    },
                });
                assert_valid(&f.bind());
                for value in [false, true] {
                    let mut duplicate = f.build.clone();
                    duplicate.choices.push(MechanicChoice {
                        owner: if use_provider {
                            direct.clone()
                        } else {
                            alias.clone()
                        },
                        choice: ChoiceSelection {
                            slot: slot.clone(),
                            value: ParameterValue::Boolean(value),
                        },
                    });
                    assert!(BuildSpec::new(duplicate, OwnedInputLimits::default()).is_err());
                }
            }
        }
    }
}

#[test]
fn allocation_local_choice_satisfies_provider_alias_and_duplicate_is_rejected() {
    let mut f = Fixture::new();
    f.allocation();
    let slot = f.index.choice(
        SlotOwnerDefId::PassiveNode(def("node")),
        "choice",
        vec![ChoiceOwnerScope::Provider(ProviderRole::Allocation)],
        SlotPresence::RequiredOnce,
    );
    f.build.allocations[0].choices.push(ChoiceSelection {
        slot: slot.clone(),
        value: ParameterValue::Boolean(false),
    });
    assert_valid(&f.bind());
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Provider(ProviderKey {
            root: ProviderRoot::Allocation(occurrence(5)),
            grant_path: vec![],
        }),
        choice: ChoiceSelection {
            slot,
            value: ParameterValue::Boolean(false),
        },
    });
    assert!(BuildSpec::new(f.build, OwnedInputLimits::default()).is_err());
}

#[test]
fn selected_action_requires_its_own_choice_but_unselected_outputs_do_not() {
    let mut f = Fixture::new();
    let output = f
        .index
        .output(skill_owner(), "output", DeclaredActorRole::Player);
    let slot = f.index.choice(
        skill_owner(),
        "action-choice",
        vec![ChoiceOwnerScope::Action],
        SlotPresence::RequiredOnce,
    );
    let SlotDescriptor::ActionOutput(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .slots
        .get_mut(&ActionOutputDefId::address(&output))
        .unwrap()
    else {
        panic!()
    };
    schema.choices.members.push(slot.clone());
    assert_valid(&f.bind());
    let selected = action(root(), ActorKey::Player, output);
    f.queries.push(request(
        "selected",
        MetricTarget::Action(Box::new(selected.clone())),
    ));
    assert!(
        f.bind()
            .issues()
            .iter()
            .any(|issue| issue.code == BindingIssueCode::RequiredValueMissing)
    );
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Action(Box::new(selected.clone())),
        choice: ChoiceSelection {
            slot,
            value: ParameterValue::Boolean(false),
        },
    });
    assert_valid(&f.bind());
    f.index
        .put_definition(DefinitionDescriptor::ActionMode(known(
            def("other-mode"),
            ActionModeSchema {},
        )));
    let SlotDescriptor::ActionOutput(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .slots
        .get_mut(&ActionOutputDefId::address(&selected.action.output))
        .unwrap()
    else {
        panic!()
    };
    schema.modes.members.push(def("other-mode"));
    let MetricTarget::Action(action) = &mut f.queries[0].target else {
        panic!()
    };
    action.mode = def("other-mode");
    assert!(f.bind().issues().iter().any(|issue| issue.site.location
        == BindingLocation::Query(QueryId::new("selected").unwrap())
        && issue.code == BindingIssueCode::RequiredValueMissing));
}

#[test]
fn partial_required_lists_and_wrong_supplied_value_are_not_silently_complete() {
    let mut f = Fixture::new();
    let slot = f.index.choice(
        skill_owner(),
        "choice",
        vec![ChoiceOwnerScope::Skill],
        SlotPresence::RequiredOnce,
    );
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Skill(SkillTarget::Authored(occurrence(2))),
        choice: ChoiceSelection {
            slot,
            value: ParameterValue::Boolean(false),
        },
    });
    f.index.declarations(&skill_owner()).choices.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            def::<SkillDefinition>("skill").address(),
        ))],
    };
    let report = f.bind();
    assert_eq!(report.schema(), SchemaBindingStatus::Unresolved);
    assert!(
        !report
            .issues()
            .iter()
            .any(|issue| issue.code == BindingIssueCode::RequiredValueMissing)
    );
    f.build.choices[0].choice.value = ParameterValue::Integer(BoundedInteger::new(0).unwrap());
    let report = f.bind();
    assert_eq!(report.schema(), SchemaBindingStatus::Invalid);
    assert!(
        report
            .issues()
            .iter()
            .any(|issue| issue.code == BindingIssueCode::ValueKindMismatch)
    );
}

#[test]
fn unsorted_membership_and_tight_work_issue_limits_are_respected() {
    let mut f = Fixture::new();
    let output = f
        .index
        .output(skill_owner(), "output", DeclaredActorRole::Player);
    f.index
        .output(skill_owner(), "z-output", DeclaredActorRole::Player);
    f.index
        .output(skill_owner(), "a-output", DeclaredActorRole::Player);
    f.index
        .declarations(&skill_owner())
        .outputs
        .members
        .reverse();
    f.queries.push(request(
        "selected",
        MetricTarget::Action(Box::new(action(root(), ActorKey::Player, output))),
    ));
    assert_valid(&f.bind());
    let limits = BindingLimits {
        max_work: 1,
        ..BindingLimits::default()
    };
    assert!(matches!(
        bind_owned_request(&f.index, &f.owned(), limits),
        Err(BindingError::WorkLimit)
    ));
    f.index.choice(
        skill_owner(),
        "first-required",
        vec![ChoiceOwnerScope::Skill],
        SlotPresence::RequiredOnce,
    );
    f.index.choice(
        skill_owner(),
        "second-required",
        vec![ChoiceOwnerScope::Skill],
        SlotPresence::RequiredOnce,
    );
    let limits = BindingLimits {
        max_issues: 1,
        ..BindingLimits::default()
    };
    assert!(matches!(
        bind_owned_request(&f.index, &f.owned(), limits),
        Err(BindingError::IssueLimit)
    ));
}

#[test]
fn nested_equipment_and_support_targets_keep_explicit_activity_dependencies() {
    let mut f = Fixture::new();
    f.equipment();
    let item_owner = SlotOwnerDefId::ItemTemplate(def("item"));
    let socket = def::<SocketSlotDefinition>("socket");
    f.index
        .put_definition(DefinitionDescriptor::SocketSlot(known(
            socket.clone(),
            SocketSlotSchema {
                owner: item_owner.clone(),
                kind: SocketKind::Item,
                scope: ScopePolicy::Either,
            },
        )));
    f.index
        .declarations(&item_owner)
        .sockets
        .members
        .push(socket.clone());
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<ItemTemplateDefinition>("item").address())
        .unwrap()
    else {
        panic!()
    };
    schema.socket_destinations.members.push(socket.clone());
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(9)],
    };
    f.build.equipment.push(EquipmentUse {
        id: occurrence(6),
        item: occurrence(3),
        destination: EquipmentDestination::ItemSocket {
            container: occurrence(4),
            slot: socket,
        },
        scope: LoadoutScope::Shared,
    });
    let output = f
        .index
        .output(item_owner, "socketed-output", DeclaredActorRole::Player);
    f.queries.push(request(
        "socketed",
        MetricTarget::Action(Box::new(action(
            ProviderKey {
                root: ProviderRoot::EquipmentUse(occurrence(6)),
                grant_path: vec![],
            },
            ActorKey::Player,
            output,
        ))),
    ));
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::Unavailable
    );
    assert!(
        report
            .issues()
            .iter()
            .any(|issue| issue.code == BindingIssueCode::InactiveLoadout)
    );

    let mut f = Fixture::new();
    let owner = SlotOwnerDefId::Gem(def("support"));
    f.index.put_definition(DefinitionDescriptor::Gem(known(
        def("support"),
        GemSchema {
            level: range(),
            roles: vec![AuthoredGemRole::SupportAssignment],
            skills: empty(),
            quality: quality(),
            declarations: declarations(),
        },
    )));
    f.build.gems.push(GemInstance {
        id: occurrence(7),
        definition: def("support"),
        parameters: vec![],
        level: 1,
        quality: None,
    });
    f.build.supports.push(SupportAssignment {
        id: occurrence(8),
        support: occurrence(7),
        target: SkillTarget::Authored(occurrence(2)),
        enabled: true,
    });
    let output = f
        .index
        .output(owner, "support-output", DeclaredActorRole::Player);
    f.queries.push(request(
        "support",
        MetricTarget::Action(Box::new(action(
            ProviderKey {
                root: ProviderRoot::SupportAssignment(occurrence(8)),
                grant_path: vec![],
            },
            ActorKey::Player,
            output,
        ))),
    ));
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::PendingResolution
    );
    f.build.skills[0].enabled = false;
    let report = f.bind();
    assert_valid(&report);
    assert_eq!(
        report.queries()[0].selector,
        SelectorBindingStatus::Unavailable
    );
}

#[test]
fn equal_definition_choices_on_distinct_equipment_uses_remain_independent() {
    let mut f = Fixture::new();
    f.equipment();
    f.index
        .put_definition(DefinitionDescriptor::EquipmentSlot(known(
            def("second-equipment"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            },
        )));
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<ItemTemplateDefinition>("item").address())
        .unwrap()
    else {
        panic!()
    };
    schema.equipment_slots.members.push(def("second-equipment"));
    f.build.equipment.push(EquipmentUse {
        id: occurrence(6),
        item: occurrence(3),
        destination: EquipmentDestination::CharacterSlot(def("second-equipment")),
        scope: LoadoutScope::Shared,
    });
    let choice = f.index.choice(
        SlotOwnerDefId::ItemTemplate(def("item")),
        "choice",
        vec![ChoiceOwnerScope::EquipmentUse],
        SlotPresence::RequiredOnce,
    );
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::EquipmentUse(occurrence(4)),
        choice: ChoiceSelection {
            slot: choice.clone(),
            value: ParameterValue::Boolean(false),
        },
    });
    let report = f.bind();
    assert_eq!(report.schema(), SchemaBindingStatus::Invalid);
    assert!(report.issues().iter().any(|issue| issue.site.location
        == BindingLocation::Equipment(occurrence(6))
        && issue.code == BindingIssueCode::RequiredValueMissing));
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::EquipmentUse(occurrence(6)),
        choice: ChoiceSelection {
            slot: choice,
            value: ParameterValue::Boolean(true),
        },
    });
    assert_valid(&f.bind());
}

#[test]
fn a_direct_scope_does_not_expand_to_nonempty_provider_paths() {
    let mut f = Fixture::new();
    let choice = f.index.choice(
        skill_owner(),
        "choice",
        vec![ChoiceOwnerScope::Skill],
        SlotPresence::OptionalOnce,
    );
    let skill = declared(skill_owner(), "generated");
    f.index
        .declarations(&skill_owner())
        .skill_grants
        .members
        .push(skill.clone());
    f.index.put_slot(SlotDescriptor::SkillGrant(known(
        skill.clone(),
        SkillGrantSlotSchema {
            skill: def("skill"),
            outputs: empty(),
        },
    )));
    let grant = declared(skill_owner(), "grant");
    f.index
        .declarations(&skill_owner())
        .grants
        .members
        .push(grant.clone());
    f.index.put_slot(SlotDescriptor::Grant(known(
        grant.clone(),
        GrantSlotSchema {
            provider_roles: vec![ProviderRole::SkillUse],
            target: GrantTarget::Skill(skill),
        },
    )));
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Provider(ProviderKey {
            root: root().root,
            grant_path: vec![grant],
        }),
        choice: ChoiceSelection {
            slot: choice.clone(),
            value: ParameterValue::Boolean(false),
        },
    });
    let report = f.bind();
    assert_eq!(report.schema(), SchemaBindingStatus::Invalid);
    assert!(
        report
            .issues()
            .iter()
            .any(|issue| issue.code == BindingIssueCode::IncompatibleScope)
    );
    let SlotDescriptor::Choice(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .slots
        .get_mut(&ChoiceSlotDefId::address(&choice))
        .unwrap()
    else {
        panic!()
    };
    schema.owners = vec![ChoiceOwnerScope::Provider(ProviderRole::SkillUse)];
    assert_valid(&f.bind());
}

#[test]
fn selected_gem_skill_requires_only_its_exact_provider_choices() {
    let mut f = Fixture::new();
    f.index.put_definition(DefinitionDescriptor::Skill(known(
        def("companion"),
        SkillSchema {
            directly_selectable: false,
            declarations: declarations(),
        },
    )));
    f.index.put_definition(DefinitionDescriptor::Gem(known(
        def("active-gem"),
        GemSchema {
            level: range(),
            roles: vec![AuthoredGemRole::SkillUse],
            skills: DeclaredSet::complete(vec![def("companion"), def("skill")]),
            quality: quality(),
            declarations: declarations(),
        },
    )));
    f.build.gems.push(GemInstance {
        id: occurrence(3),
        definition: def("active-gem"),
        parameters: vec![],
        level: 1,
        quality: None,
    });
    f.build.skills[0].source = AuthoredSkillSource::Gem(occurrence(3));
    let companion_owner = SlotOwnerDefId::Skill(def("companion"));
    let required = f.index.choice(
        companion_owner.clone(),
        "companion-choice",
        vec![ChoiceOwnerScope::Skill],
        SlotPresence::RequiredOnce,
    );
    let ordinary = f
        .index
        .output(skill_owner(), "ordinary-output", DeclaredActorRole::Player);
    let companion = f.index.output(
        companion_owner,
        "companion-output",
        DeclaredActorRole::Player,
    );
    f.queries.push(request(
        "ordinary",
        MetricTarget::Action(Box::new(action(root(), ActorKey::Player, ordinary))),
    ));
    assert_valid(&f.bind());
    f.queries.push(request(
        "companion",
        MetricTarget::Action(Box::new(action(root(), ActorKey::Player, companion))),
    ));
    let report = f.bind();
    assert_eq!(report.queries()[0].schema, SchemaBindingStatus::Valid);
    assert_eq!(report.queries()[1].schema, SchemaBindingStatus::Invalid);
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Provider(root()),
        choice: ChoiceSelection {
            slot: required,
            value: ParameterValue::Boolean(false),
        },
    });
    assert_valid(&f.bind());
}

#[test]
fn entered_skill_choice_is_required_at_the_exact_selected_provider_path() {
    let mut f = Fixture::new();
    let generated_owner = SlotOwnerDefId::Skill(def("generated-skill"));
    f.index.put_definition(DefinitionDescriptor::Skill(known(
        def("generated-skill"),
        SkillSchema {
            directly_selectable: false,
            declarations: declarations(),
        },
    )));
    let required = f.index.choice(
        generated_owner.clone(),
        "generated-choice",
        vec![ChoiceOwnerScope::Provider(ProviderRole::SkillUse)],
        SlotPresence::RequiredOnce,
    );
    let output = f.index.output(
        generated_owner,
        "generated-output",
        DeclaredActorRole::Player,
    );
    let skill = declared(skill_owner(), "generated");
    f.index
        .declarations(&skill_owner())
        .skill_grants
        .members
        .push(skill.clone());
    f.index.put_slot(SlotDescriptor::SkillGrant(known(
        skill.clone(),
        SkillGrantSlotSchema {
            skill: def("generated-skill"),
            outputs: DeclaredSet::complete(vec![output.clone()]),
        },
    )));
    let grant = declared(skill_owner(), "grant");
    f.index
        .declarations(&skill_owner())
        .grants
        .members
        .push(grant.clone());
    f.index.put_slot(SlotDescriptor::Grant(known(
        grant.clone(),
        GrantSlotSchema {
            provider_roles: vec![ProviderRole::SkillUse],
            target: GrantTarget::Skill(skill),
        },
    )));
    assert_valid(&f.bind());
    let selected = ProviderKey {
        root: root().root,
        grant_path: vec![grant],
    };
    f.queries.push(request(
        "generated",
        MetricTarget::Action(Box::new(action(selected.clone(), ActorKey::Player, output))),
    ));
    let report = f.bind();
    assert_eq!(report.queries()[0].schema, SchemaBindingStatus::Invalid);
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Provider(selected),
        choice: ChoiceSelection {
            slot: required,
            value: ParameterValue::Boolean(false),
        },
    });
    assert_valid(&f.bind());
}

#[test]
fn occurrence_resolver_reuses_actor_prefix_and_action_binding_without_activation_claims() {
    let mut f = Fixture::new();
    let output = f.index.output(
        skill_owner(),
        "actor-output",
        DeclaredActorRole::ProviderActor,
    );
    let (grant, actor_slot) = f
        .index
        .actor_grant(skill_owner(), "summon", vec![output.clone()]);
    let actor = ActorKey::Owned(Box::new(OwnedActorKey {
        provider: root(),
        slot: actor_slot.clone(),
    }));
    let provider = ProviderKey {
        root: root().root,
        grant_path: vec![grant],
    };
    let selection = action(provider.clone(), actor.clone(), output);
    f.queries.push(request(
        "same-action",
        MetricTarget::Action(Box::new(selection.clone())),
    ));
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let resolved = resolver.provider(&provider).unwrap();
    assert_eq!(resolved.schema(), SchemaBindingStatus::Valid);
    assert_eq!(resolved.status(), SelectorBindingStatus::PendingResolution);
    let context = resolved.value().unwrap();
    assert_eq!(context.actor(), &actor);
    assert_eq!(context.role(), ProviderRole::SkillUse);
    let ProviderExposure::Actor {
        key,
        parent_actor,
        schema,
    } = context.exposure()
    else {
        panic!("actor exposure")
    };
    assert_eq!(&key.provider, &root());
    assert_eq!(parent_actor, &ActorKey::Player);
    assert_eq!(
        schema.outputs.members,
        vec![selection.action.output.clone()]
    );
    let action = resolver.action(&selection).unwrap();
    let report = bind_owned_request(&f.index, &request, BindingLimits::default()).unwrap();
    assert_eq!(action.request_digest(), report.request_digest());
    assert_eq!(action.data_identity(), report.data_identity());
    assert_eq!(action.schema(), report.queries()[0].schema);
    assert_eq!(action.status(), report.queries()[0].selector);
    assert_eq!(action.value().unwrap().expected_actor(), &actor);
    assert_eq!(action.value().unwrap().selection(), &selection);
    let owned = resolver.actor(&actor).unwrap();
    assert!(matches!(
        owned.value(),
        Some(ActorOccurrence::Owned {
            parent_actor: ActorKey::Player,
            ..
        })
    ));
    let fabricated_child = ActorKey::Owned(Box::new(OwnedActorKey {
        provider,
        slot: actor_slot,
    }));
    let absent = resolver.actor(&fabricated_child).unwrap();
    assert_eq!(absent.status(), SelectorBindingStatus::Unavailable);
    assert!(absent.value().is_none());
    let player = resolver.actor(&ActorKey::Player).unwrap();
    assert_eq!(player.status(), SelectorBindingStatus::SchemaBound);
    assert!(matches!(player.value(), Some(ActorOccurrence::Player)));
}

#[test]
fn occurrence_resolver_preserves_generated_skill_parent_and_explicit_output_restriction() {
    let mut f = Fixture::new();
    let owner = SlotOwnerDefId::Skill(def("generated"));
    f.index.put_definition(DefinitionDescriptor::Skill(known(
        def("generated"),
        SkillSchema {
            directly_selectable: false,
            declarations: declarations(),
        },
    )));
    let allowed = f
        .index
        .output(owner.clone(), "allowed", DeclaredActorRole::Player);
    let sibling = f
        .index
        .output(owner.clone(), "sibling", DeclaredActorRole::Player);
    let slot = declared(skill_owner(), "generated-slot");
    f.index
        .declarations(&skill_owner())
        .skill_grants
        .members
        .push(slot.clone());
    f.index.put_slot(SlotDescriptor::SkillGrant(known(
        slot.clone(),
        SkillGrantSlotSchema {
            skill: def("generated"),
            outputs: DeclaredSet::complete(vec![allowed.clone()]),
        },
    )));
    let grant = declared(skill_owner(), "enter-generated");
    f.index
        .declarations(&skill_owner())
        .grants
        .members
        .push(grant.clone());
    f.index.put_slot(SlotDescriptor::Grant(known(
        grant.clone(),
        GrantSlotSchema {
            provider_roles: vec![ProviderRole::SkillUse],
            target: GrantTarget::Skill(slot.clone()),
        },
    )));
    let generated = GeneratedSkillKey {
        provider: root(),
        slot,
    };
    let entered = ProviderKey {
        root: root().root,
        grant_path: vec![grant],
    };
    let (actor_grant, actor_slot) = f.index.actor_grant(owner, "nested-actor", vec![]);
    let mut nested = entered.clone();
    nested.grant_path.push(actor_grant);
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let direct = resolver
        .skill(&SkillTarget::Generated(Box::new(generated.clone())))
        .unwrap();
    assert_eq!(
        direct.value().unwrap().definition(),
        Some(&def("generated"))
    );
    let via_grant = resolver.provider(&entered).unwrap();
    for context in [
        direct.value().unwrap().provider(),
        via_grant.value().unwrap(),
    ] {
        let ProviderExposure::Skill { key, outputs, .. } = context.exposure() else {
            panic!("generated exposure")
        };
        assert_eq!(key, &generated);
        assert_eq!(outputs.members, vec![allowed.clone()]);
    }
    assert!(
        resolver
            .action(&action(entered.clone(), ActorKey::Player, allowed))
            .unwrap()
            .value()
            .is_some()
    );
    assert_eq!(
        resolver
            .action(&action(entered.clone(), ActorKey::Player, sibling))
            .unwrap()
            .status(),
        SelectorBindingStatus::Unavailable
    );
    let nested = resolver.provider(&nested).unwrap();
    let ProviderExposure::Actor {
        key, parent_actor, ..
    } = nested.value().unwrap().exposure()
    else {
        panic!("nested actor")
    };
    assert_eq!(
        key,
        &OwnedActorKey {
            provider: entered,
            slot: actor_slot
        }
    );
    assert_eq!(parent_actor, &ActorKey::Player);
}

#[test]
fn occurrence_resolver_does_not_choose_one_potential_gem_skill() {
    let mut f = Fixture::new();
    f.index.put_definition(DefinitionDescriptor::Skill(known(
        def("companion"),
        SkillSchema {
            directly_selectable: false,
            declarations: declarations(),
        },
    )));
    f.index.put_definition(DefinitionDescriptor::Gem(known(
        def("gem"),
        GemSchema {
            level: range(),
            roles: vec![AuthoredGemRole::SkillUse],
            skills: DeclaredSet::complete(vec![def("companion"), def("skill")]),
            quality: quality(),
            declarations: declarations(),
        },
    )));
    f.build.gems.push(GemInstance {
        id: occurrence(3),
        definition: def("gem"),
        parameters: vec![],
        level: 1,
        quality: None,
    });
    f.build.skills[0].source = AuthoredSkillSource::Gem(occurrence(3));
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let result = resolver
        .skill(&SkillTarget::Authored(occurrence(2)))
        .unwrap();
    let skill = result.value().unwrap();
    assert!(skill.definition().is_none());
    let ProviderExposure::Root {
        owners,
        skills: Some(skills),
    } = skill.provider().exposure()
    else {
        panic!("gem potential")
    };
    assert_eq!(owners[0].definition(), &SlotOwnerDefId::Gem(def("gem")));
    assert_eq!(skills.members, vec![def("companion"), def("skill")]);
}

#[test]
fn occurrence_resolver_keeps_invalid_unavailable_and_partial_separate() {
    let mut f = Fixture::new();
    let output = f
        .index
        .output(skill_owner(), "output", DeclaredActorRole::Player);
    f.index
        .put_definition(DefinitionDescriptor::ActionMode(known(
            def("other-mode"),
            ActionModeSchema {},
        )));
    let mut selection = action(root(), ActorKey::Player, output.clone());
    selection.mode = def("other-mode");
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let invalid = resolver.action(&selection).unwrap();
    assert_eq!(invalid.schema(), SchemaBindingStatus::Invalid);
    assert_eq!(invalid.status(), SelectorBindingStatus::PendingResolution);
    assert!(invalid.value().is_none());
    assert!(
        invalid
            .issues()
            .iter()
            .any(|i| i.code == BindingIssueCode::NotDeclared)
    );
    let removed = resolver
        .provider(&ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(20)),
            grant_path: vec![],
        })
        .unwrap();
    assert_eq!(removed.schema(), SchemaBindingStatus::Valid);
    assert_eq!(removed.status(), SelectorBindingStatus::Unavailable);
    assert!(removed.value().is_none());
    let declarations = f.index.declarations(&skill_owner());
    declarations.outputs = DeclaredSet::partial(
        vec![],
        vec![gap(SchemaSubject::Definition(
            def::<SkillDefinition>("skill").address(),
        ))],
    );
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let partial = resolver
        .action(&action(root(), ActorKey::Player, output))
        .unwrap();
    assert_eq!(partial.schema(), SchemaBindingStatus::Unresolved);
    assert_eq!(partial.status(), SelectorBindingStatus::Unresolved);
    assert!(partial.value().is_none());
}

#[test]
fn occurrence_resolver_checks_arbitrary_selector_structure_and_reports_repeatable_work() {
    let f = Fixture::new();
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let first = resolver.provider(&root()).unwrap();
    let second = resolver.provider(&root()).unwrap();
    assert!(first.work_used() > 0);
    assert_eq!(first.work_used(), second.work_used());
    assert_eq!(first.request_digest(), second.request_digest());
    let tight = OwnedOccurrenceResolver::new(
        &f.index,
        &request,
        BindingLimits {
            max_work: first.work_used() - 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        tight.provider(&root()),
        Err(BindingError::WorkLimit)
    ));
    let wrong_domain = ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(1)),
        grant_path: vec![],
    };
    assert!(matches!(
        resolver.provider(&wrong_domain),
        Err(BindingError::Structure(StructuralError {
            kind: StructuralErrorKind::MissingReference { .. },
            ..
        }))
    ));
    let mut foreign = root();
    foreign.root = ProviderRoot::SkillUse(SkillUseId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([55; 16]), 2).unwrap(),
    ));
    assert!(matches!(
        resolver.provider(&foreign),
        Err(BindingError::Structure(StructuralError {
            kind: StructuralErrorKind::ForeignLineage,
            ..
        }))
    ));
    let mut changed = Fixture::new();
    changed.build.revision = BuildRevision::from_u64(2);
    let changed_request = changed.owned();
    let changed_resolver =
        OwnedOccurrenceResolver::new(&changed.index, &changed_request, BindingLimits::default())
            .unwrap();
    assert_ne!(first.request_digest(), changed_resolver.request_digest());
    assert_eq!(first.data_identity(), changed_resolver.data_identity());
}

#[test]
fn occurrence_resolver_keeps_containment_activity_and_modifier_record_ownership() {
    let mut f = Fixture::new();
    f.equipment();
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(9)],
    };
    f.build.equipment.push(EquipmentUse {
        id: occurrence(6),
        item: occurrence(3),
        destination: EquipmentDestination::ItemSocket {
            container: occurrence(4),
            slot: def("socket"),
        },
        scope: LoadoutScope::Shared,
    });
    let provider = ProviderKey {
        root: ProviderRoot::EquipmentUse(occurrence(6)),
        grant_path: vec![],
    };
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let inactive = resolver.provider(&provider).unwrap();
    assert_eq!(inactive.status(), SelectorBindingStatus::Unavailable);
    assert!(
        inactive
            .issues()
            .iter()
            .any(|i| i.code == BindingIssueCode::InactiveLoadout)
    );
    f.build.equipment[0].scope = LoadoutScope::Shared;
    f.build.items[0].modifier_order.push(occurrence(7));
    f.build.items[0].modifiers.push(RolledModifier {
        id: occurrence(7),
        definition: def("modifier"),
        rolls: vec![],
    });
    f.build.items.push(ItemRecord {
        id: occurrence(10),
        template: def("item"),
        parameters: vec![],
        item_level: None,
        quality: None,
        modifier_order: vec![],
        modifiers: vec![],
    });
    f.build.equipment.push(EquipmentUse {
        id: occurrence(11),
        item: occurrence(10),
        destination: EquipmentDestination::CharacterSlot(def("equipment")),
        scope: LoadoutScope::Shared,
    });
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    assert!(resolver.provider(&provider).unwrap().value().is_some());
    let wrong_owner = ProviderKey {
        root: ProviderRoot::ItemModifier {
            equipment_use: occurrence(11),
            modifier: occurrence(7),
        },
        grant_path: vec![],
    };
    assert!(matches!(
        resolver.provider(&wrong_owner),
        Err(BindingError::Structure(StructuralError {
            kind: StructuralErrorKind::WrongProviderOwner,
            ..
        }))
    ));
}
