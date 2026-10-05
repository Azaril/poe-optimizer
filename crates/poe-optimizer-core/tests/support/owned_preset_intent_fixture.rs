use super::*;

pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("intent-game", "v1").unwrap()
}
pub fn id<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([91; 16]), n).unwrap())
}
pub fn def<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(ns(), s).unwrap()
}
pub fn slot<K: DefinitionDomain>(owner: SlotOwnerDefId, key: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: def(key),
    }
}
pub fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(40).unwrap(),
    }
}
fn complete<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: complete(),
        choices: complete(),
        grants: complete(),
        actors: complete(),
        skill_grants: complete(),
        outputs: complete(),
        sockets: complete(),
    }
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
#[derive(Clone)]
pub struct Index {
    pub identity: DataIdentity,
    pub definitions: BTreeMap<DefinitionAddress, DefinitionDescriptor>,
    pub slots: BTreeMap<SlotAddress, SlotDescriptor>,
    pub namespace: GameVersionNamespace,
}
impl DefinitionSchemaIndex for Index {
    fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    fn lookup_definition(&self, id: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        self.definitions.get(id)
    }
    fn lookup_slot(&self, id: &SlotAddress) -> Option<&SlotDescriptor> {
        self.slots.get(id)
    }
}
impl Index {
    fn put_definition(&mut self, row: DefinitionDescriptor) {
        self.definitions.insert(row.address(), row);
    }
    fn put_slot(&mut self, row: SlotDescriptor) {
        self.slots.insert(row.address(), row);
    }
}
pub fn target() -> GeneratedSkillKey {
    GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::EquipmentUse(id(11)),
            grant_path: vec![],
        },
        slot: slot(SlotOwnerDefId::ItemTemplate(def("item")), "supply"),
    }
}
pub fn parameter() -> DeclaredSlot<ParameterSlotDefId> {
    slot(SlotOwnerDefId::Skill(def("child")), "quality")
}
pub fn usage(n: i64) -> UsagePolicySelection {
    UsagePolicySelection {
        policy: def("usage"),
        target: UsageTarget::Skill(SkillTarget::Generated(Box::new(target()))),
        parameters: vec![ParameterAssignment {
            slot: slot(SlotOwnerDefId::UsagePolicy(def("usage")), "count"),
            value: integer(n),
        }],
    }
}
pub fn intent(applicability: PresetApplicability) -> SkillPresetIntentV1 {
    SkillPresetIntentV1 {
        schema_version: 1,
        usage: vec![PresetUsageBinding {
            selection: usage(3),
            applicability,
        }],
        generated_inputs: vec![GeneratedSkillInputBinding {
            target: target(),
            parameters: vec![ParameterAssignment {
                slot: parameter(),
                value: integer(12),
            }],
            applicability,
        }],
    }
}
pub fn schema() -> Index {
    let mut index = Index {
        identity: DataIdentity {
            game: "intent-game".into(),
            release: "fixture".into(),
            schema_version: 6,
            content_sha256: "a".repeat(64),
            semantics_version: "v1".into(),
        },
        namespace: ns(),
        definitions: BTreeMap::new(),
        slots: BTreeMap::new(),
    };
    let mut item = declarations();
    item.skill_grants.members.push(target().slot.clone());
    index.put_definition(DefinitionDescriptor::ItemTemplate(known(
        def("item"),
        ItemTemplateSchema {
            item_level: range(),
            equipment_slots: complete(),
            socket_destinations: complete(),
            modifiers: complete(),
            quality: QualityUseSchema {
                presence: QualityPresence::Forbidden,
                allowed_kinds: complete(),
            },
            declarations: item,
        },
    )));
    let mut child = declarations();
    child.parameters.members.push(parameter());
    index.put_definition(DefinitionDescriptor::Skill(known(
        def("child"),
        SkillSchema {
            directly_selectable: false,
            declarations: child,
        },
    )));
    index.put_slot(SlotDescriptor::Parameter(known(
        parameter(),
        ParameterSlotSchema {
            value: ValueSchema::Integer(range()),
            presence: SlotPresence::RequiredOnce,
            sites: vec![],
            skill_input: Some(SkillInputAuthority::AuthoredOrProjected),
        },
    )));
    index.put_slot(SlotDescriptor::SkillGrant(known(
        target().slot,
        SkillGrantSlotSchema {
            skill: def("child"),
            outputs: complete(),
            preset_inputs: Some(PresetSkillInputPermission {
                schema_version: 1,
                parameters: DeclaredSet::complete(vec![parameter()]),
            }),
        },
    )));
    let mut policy = declarations();
    let count = slot(SlotOwnerDefId::UsagePolicy(def("usage")), "count");
    policy.parameters.members.push(count.clone());
    index.put_definition(DefinitionDescriptor::UsagePolicy(known(
        def("usage"),
        UsagePolicySchema {
            targets: vec![UsageTargetKind::Skill],
            declarations: policy,
        },
    )));
    index.put_slot(SlotDescriptor::Parameter(known(
        count,
        ParameterSlotSchema {
            value: ValueSchema::Integer(range()),
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::UsagePolicyParameter],
            skill_input: None,
        },
    )));
    index
}
pub fn project_input() -> ProjectInput {
    ProjectInput {
        allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([91; 16]), 1000),
        revision: BuildRevision::from_u64(1),
        game_version: ns(),
        weapon_loadouts: vec![id(1), id(2)],
        items: vec![ItemRecord {
            id: id(10),
            template: def("item"),
            item_level: None,
            quality: None,
            parameters: vec![],
            modifiers: vec![],
            modifier_order: vec![],
        }],
        gems: vec![],
        rewards: vec![],
        equipment: vec![EquipmentUse {
            id: id(11),
            item: id(10),
            destination: EquipmentDestination::CharacterSlot(def("slot")),
            scope: LoadoutScope::Selected {
                loadouts: vec![id(2)],
            },
        }],
        allocations: vec![],
        skills: vec![],
        supports: vec![],
        payload_links: vec![],
        character_presets: vec![CharacterPreset {
            id: id(100),
            class: def("class"),
            ascendancy: None,
            level: 20,
            rewards: vec![],
        }],
        equipment_presets: vec![
            EquipmentPreset {
                id: id(110),
                equipment: vec![id(11)],
            },
            EquipmentPreset {
                id: id(111),
                equipment: vec![],
            },
        ],
        allocation_presets: vec![AllocationPreset {
            id: id(120),
            allocations: vec![],
            equipment: vec![],
        }],
        skill_presets: vec![SkillPreset {
            id: id(130),
            skills: vec![],
            supports: vec![],
            support_origins: None,
            payload_links: vec![],
            usage_preferences: None,
            intent: Some(intent(PresetApplicability::WhenExactSourceSelected)),
        }],
        choice_presets: vec![ChoicePreset {
            id: id(140),
            choices: vec![],
            rewards: vec![],
        }],
        saved_variants: vec![],
    }
}
pub fn selection() -> VariantSelection {
    VariantSelection {
        character: id(100),
        equipment: id(110),
        allocations: id(120),
        skills: id(130),
        choices: id(140),
        active_weapon_loadout: id(1),
    }
}
pub fn scenario(rows: Vec<UsagePolicySelection>) -> ScenarioSpec {
    ScenarioSpec::new(
        ScenarioInput {
            game_version: ns(),
            enemy: EnemySpec {
                encounter: def("enemy"),
                level: 20,
            },
            assumptions: vec![],
            usage: rows,
        },
        OwnedInputLimits::default(),
    )
    .unwrap()
}
pub fn queries() -> QuerySpec {
    QuerySpec::new(
        QueryInput {
            game_version: ns(),
            requests: vec![],
        },
        OwnedInputLimits::default(),
    )
    .unwrap()
}
pub fn compose(
    index: &Index,
    project: &BuildProject,
    selected: &VariantSelection,
    proof: &ProjectIntentProof,
    overrides: Vec<UsagePolicySelection>,
) -> Result<ComposedRequest, IntentError> {
    compose_request_checked(
        index,
        project,
        IntentCompositionInputs {
            selection: selected,
            inventory: None,
            scenario: scenario(overrides),
            queries: queries(),
        },
        proof,
        BindingLimits::default(),
    )
}
pub fn draft(raw: ProjectInput) -> DraftSessionInput {
    DraftSessionInput {
        allocator: raw.allocator,
        revision: raw.revision,
        game_version: raw.game_version,
        weapon_loadouts: raw.weapon_loadouts.into(),
        items: raw.items.into(),
        gems: raw.gems.into(),
        rewards: raw.rewards.into(),
        equipment: raw.equipment.into(),
        allocations: raw.allocations.into(),
        skills: raw.skills.into(),
        supports: raw.supports.into(),
        payload_links: raw.payload_links.into(),
        character_presets: raw.character_presets.into(),
        equipment_presets: raw.equipment_presets.into(),
        allocation_presets: raw.allocation_presets.into(),
        skill_presets: raw.skill_presets.into(),
        choice_presets: raw.choice_presets.into(),
        scenario_presets: vec![ScenarioPresetDraft {
            id: id(150),
            scenario: scenario(vec![]).into_input().into(),
        }]
        .into(),
        query_presets: vec![QueryPresetDraft {
            id: id(160),
            queries: queries().into_input().into(),
        }]
        .into(),
        saved_variants: vec![].into(),
    }
}
pub fn evaluation() -> EvaluationSelection {
    EvaluationSelection {
        build: selection(),
        scenario: id(150),
        queries: id(160),
    }
}
pub fn pending<T>(n: u64) -> DraftField<T> {
    DraftField::Pending(PendingValue {
        id: id(n),
        code: OwnedDefinitionKey::new("unresolved").unwrap(),
        candidates: vec![],
    })
}
