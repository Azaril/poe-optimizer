//! Input contracts over injected data, with no source format or real-build special cases.
use poe_optimizer_core::{
    build_identity::*, data::DataIdentity, owned_binding::*, owned_build::*, owned_definitions::*,
    owned_schema::*,
};
use std::collections::BTreeMap;
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("record-game", "v1").unwrap()
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn id<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([37; 16]), n).unwrap())
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
fn range(a: i64, b: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(a).unwrap(),
        maximum: BoundedInteger::new(b).unwrap(),
    }
}
fn quality() -> QualityUseSchema {
    QualityUseSchema {
        presence: QualityPresence::Forbidden,
        allowed_kinds: empty(),
    }
}
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::InputSchema,
        code: OwnedDefinitionKey::new("pending").unwrap(),
    }
}
#[derive(Clone)]
struct Index {
    namespace: GameVersionNamespace,
    identity: DataIdentity,
    definitions: BTreeMap<DefinitionAddress, DefinitionDescriptor>,
    slots: BTreeMap<SlotAddress, SlotDescriptor>,
}
impl DefinitionSchemaIndex for Index {
    fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    fn lookup_definition(&self, a: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        self.definitions.get(a)
    }
    fn lookup_slot(&self, a: &SlotAddress) -> Option<&SlotDescriptor> {
        self.slots.get(a)
    }
}
impl Index {
    fn put(&mut self, d: DefinitionDescriptor) {
        self.definitions.insert(d.address(), d);
    }
    fn slots(&mut self, owner: &SlotOwnerDefId) -> &mut DeclaredSlots {
        let a = match owner {
            SlotOwnerDefId::Reward(id) => id.address(),
            SlotOwnerDefId::ItemTemplate(id) => id.address(),
            SlotOwnerDefId::Modifier(id) => id.address(),
            SlotOwnerDefId::Gem(id) => id.address(),
            SlotOwnerDefId::UsagePolicy(id) => id.address(),
            _ => panic!("fixture owner"),
        };
        match self.definitions.get_mut(&a).unwrap() {
            DefinitionDescriptor::Reward(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => &mut s.declarations,
            DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => &mut s.declarations,
            DefinitionDescriptor::Modifier(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => &mut s.declarations,
            DefinitionDescriptor::Gem(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => &mut s.declarations,
            DefinitionDescriptor::UsagePolicy(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => &mut s.declarations,
            _ => panic!("known fixture"),
        }
    }
    fn parameter(
        &mut self,
        owner: SlotOwnerDefId,
        site: ParameterSite,
        value: ValueSchema,
    ) -> DeclaredSlot<ParameterSlotDefId> {
        let slot = DeclaredSlot {
            declaration: owner.clone(),
            slot: def("parameter"),
        };
        self.slots(&owner).parameters.members.push(slot.clone());
        let row = SlotDescriptor::Parameter(known(
            slot.clone(),
            ParameterSlotSchema {
                value,
                presence: SlotPresence::RequiredOnce,
                sites: vec![site],
            },
        ));
        self.slots.insert(row.address(), row);
        slot
    }
}
#[derive(Clone)]
struct Fixture {
    index: Index,
    build: BuildInput,
    scenario: ScenarioInput,
}
impl Fixture {
    fn new() -> Self {
        let mut index = Index {
            namespace: ns(),
            identity: DataIdentity {
                game: "record-game".into(),
                release: "test".into(),
                schema_version: 1,
                content_sha256: "a".repeat(64),
                semantics_version: "v1".into(),
            },
            definitions: BTreeMap::new(),
            slots: BTreeMap::new(),
        };
        index.put(DefinitionDescriptor::Class(known(
            def("class"),
            ClassSchema {
                level: range(1, 100),
                ascendancies: empty(),
                declarations: declarations(),
            },
        )));
        index.put(DefinitionDescriptor::Encounter(known(
            def("encounter"),
            EncounterSchema {
                enemy_level: range(1, 100),
                external_inputs: DeclaredSet::complete(vec![def("external")]),
            },
        )));
        index.put(DefinitionDescriptor::ExternalInput(known(
            def("external"),
            ExternalInputSchema {
                value: ValueSchema::Boolean,
                targets: vec![AssumptionTargetKind::Enemy],
            },
        )));
        index.put(DefinitionDescriptor::Reward(known(
            def("reward"),
            RewardSchema {
                declarations: declarations(),
            },
        )));
        index.put(DefinitionDescriptor::ItemTemplate(known(
            def("item"),
            ItemTemplateSchema {
                item_level: range(1, 100),
                equipment_slots: DeclaredSet::complete(vec![def("equipment")]),
                socket_destinations: empty(),
                modifiers: DeclaredSet::complete(vec![def("modifier")]),
                quality: quality(),
                declarations: declarations(),
            },
        )));
        index.put(DefinitionDescriptor::Modifier(known(
            def("modifier"),
            ModifierSchema {
                declarations: declarations(),
            },
        )));
        index.put(DefinitionDescriptor::Gem(known(
            def("gem"),
            GemSchema {
                level: range(1, 20),
                roles: vec![
                    AuthoredGemRole::SkillUse,
                    AuthoredGemRole::SupportAssignment,
                ],
                skills: empty(),
                quality: quality(),
                declarations: declarations(),
            },
        )));
        index.put(DefinitionDescriptor::UsagePolicy(known(
            def("usage"),
            UsagePolicySchema {
                targets: vec![UsageTargetKind::Actor],
                declarations: declarations(),
            },
        )));
        index.put(DefinitionDescriptor::EquipmentSlot(known(
            def("equipment"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Shared,
            },
        )));
        Self {
            index,
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([37; 16]),
                    100,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class: def("class"),
                    ascendancy: None,
                    level: 20,
                    rewards: vec![RewardSelection {
                        id: id(2),
                        definition: def("reward"),
                        parameters: vec![],
                    }],
                },
                weapon_loadouts: vec![id(1)],
                active_weapon_loadout: id(1),
                items: vec![ItemRecord {
                    id: id(3),
                    template: def("item"),
                    parameters: vec![],
                    item_level: 20,
                    quality: None,
                    modifiers: vec![RolledModifier {
                        id: id(4),
                        definition: def("modifier"),
                        rolls: vec![],
                    }],
                }],
                gems: vec![GemInstance {
                    id: id(5),
                    definition: def("gem"),
                    parameters: vec![],
                    level: 10,
                    quality: None,
                }],
                equipment: vec![EquipmentUse {
                    id: id(6),
                    item: id(3),
                    destination: EquipmentDestination::CharacterSlot(def("equipment")),
                    scope: LoadoutScope::Shared,
                }],
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                payload_links: vec![],
                choices: vec![],
            },
            scenario: ScenarioInput {
                game_version: ns(),
                enemy: EnemySpec {
                    encounter: def("encounter"),
                    level: 20,
                },
                assumptions: vec![ExternalAssumption {
                    input: def("external"),
                    target: AssumptionTarget::Enemy,
                    value: ParameterValue::Boolean(false),
                }],
                usage: vec![UsagePolicySelection {
                    policy: def("usage"),
                    target: UsageTarget::Actor(ActorKey::Player),
                    parameters: vec![],
                }],
            },
        }
    }
    fn owned(&self) -> OwnedEvaluationRequest {
        let l = OwnedInputLimits::default();
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), l).unwrap(),
            ScenarioSpec::new(self.scenario.clone(), l).unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: ns(),
                    requests: vec![],
                },
                l,
            )
            .unwrap(),
            l,
        )
        .unwrap()
    }
    fn bind(&self) -> DefinitionBindingReport {
        bind_owned_request(&self.index, &self.owned(), BindingLimits::default()).unwrap()
    }
    fn parameter_rows(&mut self, n: usize) -> &mut Vec<ParameterAssignment> {
        match n {
            0 => &mut self.build.character.rewards[0].parameters,
            1 => &mut self.build.items[0].parameters,
            2 => &mut self.build.items[0].modifiers[0].rolls,
            3 => &mut self.build.gems[0].parameters,
            4 => &mut self.scenario.usage[0].parameters,
            _ => panic!(),
        }
    }
    fn parameters(&mut self) -> Vec<DeclaredSlot<ParameterSlotDefId>> {
        let sites = [
            (
                SlotOwnerDefId::Reward(def("reward")),
                ParameterSite::RewardParameter,
            ),
            (
                SlotOwnerDefId::ItemTemplate(def("item")),
                ParameterSite::ItemParameter,
            ),
            (
                SlotOwnerDefId::Modifier(def("modifier")),
                ParameterSite::ModifierRoll,
            ),
            (SlotOwnerDefId::Gem(def("gem")), ParameterSite::GemParameter),
            (
                SlotOwnerDefId::UsagePolicy(def("usage")),
                ParameterSite::UsagePolicyParameter,
            ),
        ];
        sites
            .into_iter()
            .enumerate()
            .map(|(n, (owner, site))| {
                let slot = self.index.parameter(owner, site, ValueSchema::Boolean);
                self.parameter_rows(n).push(ParameterAssignment {
                    slot: slot.clone(),
                    value: ParameterValue::Boolean(false),
                });
                slot
            })
            .collect()
    }
}
fn valid(r: &DefinitionBindingReport) {
    assert_eq!(r.schema(), SchemaBindingStatus::Valid, "{:?}", r.issues());
}
fn has(r: &DefinitionBindingReport, code: BindingIssueCode) -> bool {
    r.issues().iter().any(|v| v.code == code)
}
#[test]
fn all_five_parameter_sites_preserve_false_and_enforce_kind_presence_and_declaring_site() {
    let mut f = Fixture::new();
    let slots = f.parameters();
    valid(&f.bind());
    for (n, slot) in slots.iter().enumerate() {
        let mut bad = f.clone();
        bad.parameter_rows(n).clear();
        assert!(has(&bad.bind(), BindingIssueCode::RequiredValueMissing));
        let mut bad = f.clone();
        bad.parameter_rows(n)[0].value = ParameterValue::Integer(BoundedInteger::new(0).unwrap());
        assert!(has(&bad.bind(), BindingIssueCode::ValueKindMismatch));
        let mut bad = f.clone();
        let SlotDescriptor::Parameter(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) = bad
            .index
            .slots
            .get_mut(&ParameterSlotDefId::address(slot))
            .unwrap()
        else {
            panic!()
        };
        s.sites.clear();
        assert!(has(&bad.bind(), BindingIssueCode::IncompatibleRole));
    }
}
#[test]
fn integer_and_quantity_bounds_are_inclusive_and_units_are_exact_not_dimensions() {
    let mut f = Fixture::new();
    let slot = f.index.parameter(
        SlotOwnerDefId::Reward(def("reward")),
        ParameterSite::RewardParameter,
        ValueSchema::Integer(range(-2, 0)),
    );
    for n in [-2, 0] {
        f.build.character.rewards[0].parameters = vec![ParameterAssignment {
            slot: slot.clone(),
            value: ParameterValue::Integer(BoundedInteger::new(n).unwrap()),
        }];
        valid(&f.bind());
    }
    f.build.character.rewards[0].parameters[0].value =
        ParameterValue::Integer(BoundedInteger::new(1).unwrap());
    assert!(has(&f.bind(), BindingIssueCode::OutOfRange));
    for key in ["a", "b"] {
        f.index.put(DefinitionDescriptor::Unit(known(
            def(key),
            UnitSchema {
                dimension: UnitDimension::Count,
            },
        )));
    }
    let SlotDescriptor::Parameter(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = f
        .index
        .slots
        .get_mut(&ParameterSlotDefId::address(&slot))
        .unwrap()
    else {
        panic!()
    };
    s.value = ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(-1.5, def("a")).unwrap(),
        maximum: FiniteQuantity::new(0.0, def("a")).unwrap(),
    });
    f.build.character.rewards[0].parameters[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(0.0, def("a")).unwrap());
    valid(&f.bind());
    f.build.character.rewards[0].parameters[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(0.0, def("b")).unwrap());
    assert!(has(&f.bind(), BindingIssueCode::UnitMismatch));
}
#[test]
fn option_membership_and_partial_required_collections_never_silently_default() {
    let mut f = Fixture::new();
    for key in ["yes", "no"] {
        f.index.put(DefinitionDescriptor::Option(known(
            def(key),
            OptionSchema {},
        )));
    }
    let owner = SlotOwnerDefId::Reward(def("reward"));
    let slot = f.index.parameter(
        owner.clone(),
        ParameterSite::RewardParameter,
        ValueSchema::Option {
            allowed: DeclaredSet::complete(vec![def("yes")]),
        },
    );
    f.build.character.rewards[0].parameters = vec![ParameterAssignment {
        slot: slot.clone(),
        value: ParameterValue::Option(def("yes")),
    }];
    valid(&f.bind());
    f.build.character.rewards[0].parameters[0].value = ParameterValue::Option(def("no"));
    assert!(has(&f.bind(), BindingIssueCode::NotDeclared));
    f.build.character.rewards[0].parameters[0].value = ParameterValue::Option(def("yes"));
    f.index.slots(&owner).parameters.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Slot(ParameterSlotDefId::address(&slot)))],
    };
    assert_eq!(f.bind().schema(), SchemaBindingStatus::Unresolved);
}
#[test]
fn quality_none_is_distinct_from_present_zero_and_data_controls_limits() {
    let mut f = Fixture::new();
    f.index.put(DefinitionDescriptor::Unit(known(
        def("quality-unit"),
        UnitSchema {
            dimension: UnitDimension::Count,
        },
    )));
    f.index.put(DefinitionDescriptor::Quality(known(
        def("quality"),
        QualitySchema {
            amount: QuantityRange {
                minimum: FiniteQuantity::new(0.0, def("quality-unit")).unwrap(),
                maximum: FiniteQuantity::new(20.0, def("quality-unit")).unwrap(),
            },
        },
    )));
    let DefinitionDescriptor::Gem(DefinitionEntry {
        schema: SchemaState::Known(gem),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<GemDefinition>("gem").address())
        .unwrap()
    else {
        panic!()
    };
    gem.quality = QualityUseSchema {
        presence: QualityPresence::Required,
        allowed_kinds: DeclaredSet::complete(vec![def("quality")]),
    };
    assert!(has(&f.bind(), BindingIssueCode::RequiredValueMissing));
    f.build.gems[0].quality = Some(QualitySelection {
        kind: def("quality"),
        amount: FiniteQuantity::new(0.0, def("quality-unit")).unwrap(),
    });
    valid(&f.bind());
    f.build.gems[0].level = 21;
    assert!(has(&f.bind(), BindingIssueCode::OutOfRange));
    f.build.gems[0].level = 20;
    valid(&f.bind());
    f.build.items[0].quality = f.build.gems[0].quality.clone();
    assert!(has(&f.bind(), BindingIssueCode::ValueForbidden));
}
#[test]
fn missing_unmapped_malformed_indexes_and_resource_limits_are_distinct() {
    let f = Fixture::new();
    let request = f.owned();
    let mut missing = f.clone();
    missing
        .index
        .definitions
        .remove(&def::<GemDefinition>("gem").address());
    assert_eq!(missing.bind().schema(), SchemaBindingStatus::Unresolved);
    let mut unmapped = f.clone();
    unmapped
        .index
        .put(DefinitionDescriptor::Gem(DefinitionEntry {
            id: def("gem"),
            schema: SchemaState::Unmapped {
                gaps: vec![gap(SchemaSubject::Definition(
                    def::<GemDefinition>("gem").address(),
                ))],
            },
        }));
    assert!(has(&unmapped.bind(), BindingIssueCode::UnmappedSchema));
    let mut corrupt = f.clone();
    let DefinitionDescriptor::Class(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = corrupt
        .index
        .definitions
        .get_mut(&def::<ClassDefinition>("class").address())
        .unwrap()
    else {
        panic!()
    };
    s.level = range(10, 1);
    assert!(matches!(
        bind_owned_request(&corrupt.index, &request, BindingLimits::default()),
        Err(BindingError::Index {
            fault: IndexFault::InvalidSchema,
            ..
        })
    ));
    let mut corrupt = f.clone();
    corrupt.index.definitions.insert(
        def::<ClassDefinition>("class").address(),
        DefinitionDescriptor::Class(known(
            def("other-class"),
            ClassSchema {
                level: range(1, 100),
                ascendancies: empty(),
                declarations: declarations(),
            },
        )),
    );
    assert!(matches!(
        bind_owned_request(&corrupt.index, &request, BindingLimits::default()),
        Err(BindingError::Index {
            fault: IndexFault::InconsistentLookup,
            ..
        })
    ));
    assert!(matches!(
        bind_owned_request(
            &f.index,
            &request,
            BindingLimits {
                max_work: 1,
                ..BindingLimits::default()
            }
        ),
        Err(BindingError::WorkLimit)
    ));
    let mut missing = f.clone();
    missing.index.definitions.clear();
    assert!(matches!(
        bind_owned_request(
            &missing.index,
            &request,
            BindingLimits {
                max_issues: 1,
                ..BindingLimits::default()
            }
        ),
        Err(BindingError::IssueLimit)
    ));
}
#[test]
fn scenario_target_value_and_item_destination_scope_are_independent_of_structure() {
    let mut f = Fixture::new();
    valid(&f.bind());
    f.scenario.assumptions[0].target = AssumptionTarget::Environment;
    f.scenario.assumptions[0].value = ParameterValue::Integer(BoundedInteger::new(0).unwrap());
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![id(1)],
    };
    let report = f.bind();
    assert!(has(&report, BindingIssueCode::IncompatibleRole));
    assert!(has(&report, BindingIssueCode::ValueKindMismatch));
    assert!(has(&report, BindingIssueCode::IncompatibleScope));
}

#[test]
fn ascendancies_and_separate_pools_are_data_membership_without_invented_connectivity() {
    let mut f = Fixture::new();
    f.build.character.ascendancy = Some(def("ascendancy"));
    f.index.put(DefinitionDescriptor::Ascendancy(known(
        def("ascendancy"),
        AscendancySchema {
            classes: DeclaredSet::complete(vec![def("class")]),
            declarations: declarations(),
        },
    )));
    let DefinitionDescriptor::Class(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<ClassDefinition>("class").address())
        .unwrap()
    else {
        panic!()
    };
    s.ascendancies = DeclaredSet::complete(vec![def("ascendancy")]);
    for (key, scope) in [
        ("ordinary", PointPoolScope::Shared),
        ("ascendancy", PointPoolScope::Shared),
        ("weapon", PointPoolScope::PerLoadout),
    ] {
        f.index.put(DefinitionDescriptor::PointPool(known(
            def(key),
            PointPoolSchema { scope },
        )));
    }
    f.index.put(DefinitionDescriptor::PassiveNode(known(
        def("disconnected"),
        PassiveNodeSchema {
            pools: DeclaredSet::complete(vec![def("ascendancy")]),
            adjacent: empty(),
            declarations: declarations(),
        },
    )));
    f.build.allocations = vec![Allocation {
        id: id(7),
        node: def("disconnected"),
        pool: def("ascendancy"),
        scope: LoadoutScope::Shared,
        access: AllocationAccess::Ordinary,
        choices: vec![],
    }];
    valid(&f.bind());
    f.build.allocations[0].pool = def("ordinary");
    assert!(has(&f.bind(), BindingIssueCode::NotDeclared));
    let DefinitionDescriptor::PassiveNode(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<PassiveNodeDefinition>("disconnected").address())
        .unwrap()
    else {
        panic!()
    };
    s.pools = DeclaredSet::complete(vec![def("weapon")]);
    f.build.allocations[0].pool = def("weapon");
    assert!(has(&f.bind(), BindingIssueCode::IncompatibleScope));
    f.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![id(1)],
    };
    valid(&f.bind());
    let DefinitionDescriptor::Ascendancy(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<AscendancyDefinition>("ascendancy").address())
        .unwrap()
    else {
        panic!()
    };
    s.classes = empty();
    assert!(has(&f.bind(), BindingIssueCode::NotDeclared));
}
#[test]
fn payload_membership_is_a_possible_intersection_not_all_companion_effects() {
    let mut f = Fixture::new();
    for key in ["offered", "companion", "other"] {
        f.index.put(DefinitionDescriptor::Skill(known(
            def(key),
            SkillSchema {
                directly_selectable: true,
                declarations: declarations(),
            },
        )));
    }
    let DefinitionDescriptor::Gem(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<GemDefinition>("gem").address())
        .unwrap()
    else {
        panic!()
    };
    s.skills = DeclaredSet::complete(vec![def("offered"), def("companion")]);
    f.index.put(DefinitionDescriptor::SkillLinkRole(known(
        def("payload-role"),
        SkillLinkRoleSchema {
            containers: DeclaredSet::complete(vec![def("offered")]),
            payloads: DeclaredSet::complete(vec![def("other")]),
        },
    )));
    f.build.skills = vec![
        SkillUse {
            id: id(7),
            source: AuthoredSkillSource::Gem(id(5)),
            enabled: true,
            scope: LoadoutScope::Shared,
        },
        SkillUse {
            id: id(8),
            source: AuthoredSkillSource::Direct(def("other")),
            enabled: true,
            scope: LoadoutScope::Shared,
        },
    ];
    f.build.payload_links = vec![PayloadLink {
        id: id(9),
        container: id(7),
        payload: id(8),
        role: def("payload-role"),
    }];
    valid(&f.bind());
    let DefinitionDescriptor::Gem(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<GemDefinition>("gem").address())
        .unwrap()
    else {
        panic!()
    };
    s.skills = DeclaredSet::complete(vec![def("companion")]);
    assert!(has(&f.bind(), BindingIssueCode::IncompatibleRole));
    let DefinitionDescriptor::Gem(DefinitionEntry {
        schema: SchemaState::Known(s),
        ..
    }) = f
        .index
        .definitions
        .get_mut(&def::<GemDefinition>("gem").address())
        .unwrap()
    else {
        panic!()
    };
    s.skills = DeclaredSet::partial(
        vec![def("companion")],
        vec![gap(SchemaSubject::Definition(
            def::<GemDefinition>("gem").address(),
        ))],
    );
    assert_eq!(f.bind().schema(), SchemaBindingStatus::Unresolved);
}
