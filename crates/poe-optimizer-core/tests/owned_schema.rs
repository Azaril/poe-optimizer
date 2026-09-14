use poe_optimizer_core::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::{cell::Cell, collections::BTreeMap, fmt::Debug};

fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("authored-game", "schema-test").unwrap()
}

fn id<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}

fn slot<S>(owner: &str, slot: S) -> DeclaredSlot<S> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id(owner)),
        slot,
    }
}

fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::InputSchema,
        code: OwnedDefinitionKey::new("schema.unmapped").unwrap(),
    }
}

fn roundtrip<T: Serialize + DeserializeOwned + Debug + PartialEq>(value: &T) {
    let bytes = serde_json::to_vec(value).unwrap();
    let decoded: T = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(&decoded, value);
}

// This test index is deliberately just storage, not a validated production package.
struct TestIndex {
    namespace: GameVersionNamespace,
    identity: DataIdentity,
    definitions: BTreeMap<DefinitionAddress, DefinitionDescriptor>,
    slots: BTreeMap<SlotAddress, SlotDescriptor>,
    definition_reads: Cell<usize>,
    slot_reads: Cell<usize>,
}
impl TestIndex {
    fn new() -> Self {
        Self {
            namespace: namespace(),
            identity: DataIdentity {
                game: "authored-game".into(),
                release: "test".into(),
                schema_version: 1,
                content_sha256: "a".repeat(64),
                semantics_version: "schema-test".into(),
            },
            definitions: BTreeMap::new(),
            slots: BTreeMap::new(),
            definition_reads: Cell::new(0),
            slot_reads: Cell::new(0),
        }
    }
}
impl DefinitionSchemaIndex for TestIndex {
    fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        self.definition_reads.set(self.definition_reads.get() + 1);
        self.definitions.get(address)
    }
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        self.slot_reads.set(self.slot_reads.get() + 1);
        self.slots.get(address)
    }
}

#[test]
fn typed_lookup_returns_stored_data_with_one_index_hook() {
    let mut index = TestIndex::new();
    for n in 0..100 {
        let entry = DefinitionDescriptor::Unit(DefinitionEntry {
            id: id(&format!("unit.{n}")),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        });
        index.definitions.insert(entry.address(), entry);
    }
    let requested: UnitDefId = id("unit.42");
    let SchemaLookup::Known(found) = index.definition(&requested) else {
        panic!("expected a known unit");
    };
    assert_eq!(index.definition_reads.get(), 1);
    let DefinitionDescriptor::Unit(entry) = index.definitions.get(&requested.address()).unwrap()
    else {
        panic!("expected the typed unit entry");
    };
    let SchemaState::Known(stored) = &entry.schema else {
        panic!("expected known stored data");
    };
    assert!(std::ptr::eq(found, stored));
    assert_eq!(index.identity(), &index.identity);

    assert_eq!(
        index.definition(&id::<UnitDefinition>("missing")),
        SchemaLookup::Missing
    );
    let before = index.definition_reads.get();
    let foreign = UnitDefId::parse(
        GameVersionNamespace::new("other-game", "schema-test").unwrap(),
        "unit.42",
    )
    .unwrap();
    assert_eq!(index.definition(&foreign), SchemaLookup::NamespaceMismatch);
    assert_eq!(index.definition_reads.get(), before);
}

#[test]
fn slot_lookup_uses_the_full_declaration_and_checks_both_namespaces() {
    let mut index = TestIndex::new();
    let shared_slot: ParameterSlotDefId = id("parameter.shared");
    let first = slot("template.first", shared_slot.clone());
    let second = slot("template.second", shared_slot);
    for (key, minimum) in [(&first, 1), (&second, 20)] {
        let entry = SlotDescriptor::Parameter(DefinitionEntry {
            id: key.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(minimum).unwrap(),
                    maximum: BoundedInteger::new(minimum + 5).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ItemParameter],
            }),
        });
        index.slots.insert(entry.address(), entry);
    }
    let SchemaLookup::Known(first_schema) = index.slot(&first) else {
        panic!("expected first declaration");
    };
    let SchemaLookup::Known(second_schema) = index.slot(&second) else {
        panic!("expected second declaration");
    };
    assert_ne!(first_schema, second_schema);
    assert_eq!(index.slot_reads.get(), 2);
    assert_eq!(
        index.slot(&slot("template.missing", first.slot.clone())),
        SchemaLookup::Missing,
    );

    let foreign = GameVersionNamespace::new("foreign-game", "schema-test").unwrap();
    let wrong_owner = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(
            ItemTemplateDefId::parse(foreign.clone(), "template.first").unwrap(),
        ),
        slot: first.slot.clone(),
    };
    let wrong_slot = DeclaredSlot {
        declaration: first.declaration.clone(),
        slot: ParameterSlotDefId::parse(foreign, "parameter.shared").unwrap(),
    };
    let before = index.slot_reads.get();
    assert_eq!(index.slot(&wrong_owner), SchemaLookup::NamespaceMismatch);
    assert_eq!(index.slot(&wrong_slot), SchemaLookup::NamespaceMismatch);
    assert_eq!(index.slot_reads.get(), before);
}

#[test]
fn inconsistent_hook_results_cannot_be_silently_retargeted() {
    let mut index = TestIndex::new();
    let requested: UnitDefId = id("unit.requested");
    index.definitions.insert(
        requested.address(),
        DefinitionDescriptor::Unit(DefinitionEntry {
            id: id("unit.other"),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        }),
    );
    assert_eq!(
        index.definition(&requested),
        SchemaLookup::InconsistentIndex
    );
    index.definitions.insert(
        requested.address(),
        DefinitionDescriptor::Option(DefinitionEntry {
            id: id("unit.requested"),
            schema: SchemaState::Known(OptionSchema {}),
        }),
    );
    assert_eq!(
        index.definition(&requested),
        SchemaLookup::InconsistentIndex
    );

    let requested = slot(
        "template.first",
        id::<ChoiceSlotDefinition>("choice.shared"),
    );
    let wrong = slot("template.second", requested.slot.clone());
    index.slots.insert(
        ChoiceSlotDefId::address(&requested),
        SlotDescriptor::Choice(DefinitionEntry {
            id: wrong,
            schema: SchemaState::Known(ChoiceSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::OptionalOnce,
                owners: vec![ChoiceOwnerScope::Action],
            }),
        }),
    );
    assert_eq!(index.slot(&requested), SchemaLookup::InconsistentIndex);
}

#[test]
fn incomplete_and_unmapped_membership_never_become_known_empty() {
    let issue = gap(SchemaSubject::Definition(DefinitionAddress::Option(id(
        "option.test",
    ))));
    let empty: DeclaredSet<OptionDefId> = DeclaredSet::complete(vec![]);
    let partial: DeclaredSet<OptionDefId> = DeclaredSet::partial(vec![], vec![issue.clone()]);
    assert!(empty.is_complete());
    assert!(!partial.is_complete());
    assert_ne!(empty, partial);
    roundtrip(&empty);
    roundtrip(&partial);
    roundtrip(&ValueSchema::Option { allowed: partial });
    let unmapped: SchemaState<OptionSchema> = SchemaState::Unmapped { gaps: vec![issue] };
    assert!(matches!(unmapped.as_lookup(), SchemaLookup::Unmapped(gaps) if gaps.len() == 1));
    roundtrip(&unmapped);
    assert!(serde_json::from_value::<DeclaredSet<OptionDefId>>(json!({"members":[]})).is_err());
    assert!(
        serde_json::from_value::<SchemaState<OptionSchema>>(json!({"kind":"unmapped","value":{}}))
            .is_err()
    );
}

#[test]
fn every_existing_definition_and_slot_family_has_a_typed_projection() {
    let mut index = TestIndex::new();
    macro_rules! check_definitions {
        ($($variant:ident: $id:ty),+ $(,)?) => { $(
            let id = <$id>::parse(namespace(), "same.symbol").unwrap();
            let issue = gap(SchemaSubject::Definition(id.address()));
            let entry = DefinitionDescriptor::$variant(DefinitionEntry {
                id: id.clone(), schema: SchemaState::Unmapped { gaps: vec![issue] },
            });
            roundtrip(&entry);
            roundtrip(&entry.address());
            index.definitions.insert(entry.address(), entry);
            assert!(matches!(index.definition(&id), SchemaLookup::Unmapped(gaps) if gaps.len() == 1));
        )+ };
    }
    check_definitions! {
        Class: ClassDefId, Ascendancy: AscendancyDefId, Reward: RewardDefId,
        ItemTemplate: ItemTemplateDefId, Modifier: ModifierDefId, Gem: GemDefId,
        Skill: SkillDefId, PassiveNode: PassiveNodeDefId, PointPool: PointPoolDefId,
        EquipmentSlot: EquipmentSlotDefId, Encounter: EncounterDefId, Metric: MetricDefId,
        Option: OptionDefId, ActionPart: ActionPartDefId, ActionMode: ActionModeDefId,
        ActionStatSet: ActionStatSetDefId, UsagePolicy: UsagePolicyDefId,
        SkillLinkRole: SkillLinkRoleDefId, SocketSlot: SocketSlotDefId,
        Unit: UnitDefId, Quality: QualityDefId, ExternalInput: ExternalInputDefId,
    }
    assert_eq!(index.definitions.len(), 22);

    macro_rules! check_slots {
        ($($variant:ident: $id:ty),+ $(,)?) => { $(
            let key = slot("template.test", <$id>::parse(namespace(), "same.slot").unwrap());
            let issue = gap(SchemaSubject::Slot(<$id>::address(&key)));
            let entry = SlotDescriptor::$variant(DefinitionEntry {
                id: key.clone(), schema: SchemaState::Unmapped { gaps: vec![issue] },
            });
            roundtrip(&entry);
            roundtrip(&entry.address());
            index.slots.insert(entry.address(), entry);
            assert!(matches!(index.slot(&key), SchemaLookup::Unmapped(gaps) if gaps.len() == 1));
        )+ };
    }
    check_slots! {
        Parameter: ParameterSlotDefId, Choice: ChoiceSlotDefId, Grant: GrantSlotDefId,
        Actor: ActorSlotDefId, SkillGrant: SkillGrantSlotDefId, ActionOutput: ActionOutputDefId,
    }
    assert_eq!(index.slots.len(), 6);
}

#[test]
fn schema_wire_rejects_wrong_kinds_unknown_fields_and_implicit_defaults() {
    let descriptor = DefinitionDescriptor::Unit(DefinitionEntry {
        id: id("unit.test"),
        schema: SchemaState::Known(UnitSchema {
            dimension: UnitDimension::Time,
        }),
    });
    let mut value = serde_json::to_value(&descriptor).unwrap();
    value["value"]["id"]["kind"] = json!("gem");
    assert!(serde_json::from_value::<DefinitionDescriptor>(value).is_err());

    let mut value = serde_json::to_value(&descriptor).unwrap();
    value["value"]["schema"]["value"]["source_program"] = json!("unexpected");
    assert!(serde_json::from_value::<DefinitionDescriptor>(value).is_err());
    assert!(serde_json::from_value::<OptionSchema>(json!({"label":"unexpected"})).is_err());
    assert!(serde_json::from_value::<QualityUseSchema>(json!({"presence":"optional"})).is_err());

    let action_choice = ChoiceSlotSchema {
        value: ValueSchema::Boolean,
        presence: SlotPresence::RequiredOnce,
        owners: vec![
            ChoiceOwnerScope::Action,
            ChoiceOwnerScope::Provider(ProviderRole::ItemModifier),
        ],
    };
    roundtrip(&action_choice);
    let mut value = serde_json::to_value(&action_choice).unwrap();
    value.as_object_mut().unwrap().remove("owners");
    assert!(serde_json::from_value::<ChoiceSlotSchema>(value).is_err());
}

#[test]
fn potential_topology_retains_explicit_cross_declaration_links() {
    let provider_owner = SlotOwnerDefId::Modifier(id("modifier.provider"));
    let skill_owner = SlotOwnerDefId::Skill(id("skill.supplied"));
    let actor = DeclaredSlot {
        declaration: provider_owner,
        slot: id("actor.population"),
    };
    let output = DeclaredSlot {
        declaration: skill_owner,
        slot: id("action.output"),
    };
    let grant = GrantSlotSchema {
        provider_roles: vec![ProviderRole::ItemModifier],
        target: GrantTarget::Actor(actor.clone()),
    };
    let actor_schema = ActorSlotSchema {
        skills: DeclaredSet::complete(vec![id("skill.supplied")]),
        outputs: DeclaredSet::complete(vec![output.clone()]),
    };
    let action_schema = ActionOutputSchema {
        actor_role: DeclaredActorRole::OwnedSlot(actor),
        parts: DeclaredSet::complete(vec![id("part.primary")]),
        modes: DeclaredSet::complete(vec![id("mode.default")]),
        stat_sets: DeclaredSet::complete(vec![id("stats.primary")]),
        choices: DeclaredSet::complete(vec![DeclaredSlot {
            declaration: output.declaration.clone(),
            slot: id("choice.action"),
        }]),
    };
    roundtrip(&grant);
    roundtrip(&actor_schema);
    roundtrip(&action_schema);
    assert_ne!(
        actor_schema.outputs.members[0].declaration,
        match &grant.target {
            GrantTarget::Actor(actor) => actor.declaration.clone(),
            _ => unreachable!(),
        },
    );
    // Serialization of potential topology grants no activation/legality authority.
}
