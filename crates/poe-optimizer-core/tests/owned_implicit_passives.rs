//! Direct-owned implicit roots: no allocation IDs, source syntax or execution.
use poe_optimizer_core::{
    build_identity::*, data::DataIdentity, owned_binding::*, owned_build::*, owned_definitions::*,
    owned_schema::*,
};
use std::collections::BTreeMap;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("implicit-game", "v1").unwrap()
}
fn id<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn instance<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([29; 16]), n).unwrap())
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
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
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
fn root() -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::Character,
        grant_path: vec![],
    }
}
fn owner() -> SlotOwnerDefId {
    SlotOwnerDefId::PassiveNode(id("class-root"))
}
fn slot<K: DefinitionDomain>(name: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner(),
        slot: id(name),
    }
}
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::StaticLinks,
        code: OwnedDefinitionKey::new("remaining-roots").unwrap(),
    }
}
#[derive(Clone)]
struct Index {
    identity: DataIdentity,
    namespace: GameVersionNamespace,
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
    fn lookup_definition(&self, id: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        self.definitions.get(id)
    }
    fn lookup_slot(&self, id: &SlotAddress) -> Option<&SlotDescriptor> {
        self.slots.get(id)
    }
}
impl Index {
    fn put(&mut self, d: DefinitionDescriptor) {
        self.definitions.insert(d.address(), d);
    }
    fn class(&mut self) -> &mut ClassSchema {
        match self
            .definitions
            .get_mut(&id::<ClassDefinition>("class").address())
            .unwrap()
        {
            DefinitionDescriptor::Class(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => s,
            _ => panic!("class"),
        }
    }
    fn passive(&mut self) -> &mut PassiveNodeSchema {
        match self
            .definitions
            .get_mut(&id::<PassiveNodeDefinition>("class-root").address())
            .unwrap()
        {
            DefinitionDescriptor::PassiveNode(DefinitionEntry {
                schema: SchemaState::Known(s),
                ..
            }) => s,
            _ => panic!("root"),
        }
    }
}
#[derive(Clone)]
struct Fixture {
    index: Index,
    build: BuildInput,
}
impl Fixture {
    fn new() -> Self {
        let mut index = Index {
            identity: DataIdentity {
                game: "implicit-game".into(),
                release: "test".into(),
                schema_version: 2,
                content_sha256: "d".repeat(64),
                semantics_version: "v1".into(),
            },
            namespace: ns(),
            definitions: BTreeMap::new(),
            slots: BTreeMap::new(),
        };
        for (name, roots) in [
            ("class", vec![id("shared-root"), id("class-root")]),
            ("other-class", vec![id("other-root")]),
        ] {
            index.put(DefinitionDescriptor::Class(known(
                id(name),
                ClassSchema {
                    level: range(),
                    ascendancies: DeclaredSet::complete(vec![id("ascendancy")]),
                    implicit_passives: DeclaredSet::complete(roots),
                    declarations: declarations(),
                },
            )));
        }
        index.put(DefinitionDescriptor::Ascendancy(known(
            id("ascendancy"),
            AscendancySchema {
                classes: DeclaredSet::complete(vec![id("class"), id("other-class")]),
                implicit_passives: DeclaredSet::complete(vec![id("shared-root")]),
                declarations: declarations(),
            },
        )));
        for name in ["class-root", "shared-root", "other-root"] {
            index.put(DefinitionDescriptor::PassiveNode(known(
                id(name),
                PassiveNodeSchema {
                    pools: empty(),
                    adjacent: empty(),
                    declarations: declarations(),
                },
            )));
        }
        index.put(DefinitionDescriptor::Encounter(known(
            id("encounter"),
            EncounterSchema {
                enemy_level: range(),
                external_inputs: empty(),
            },
        )));
        index.put(DefinitionDescriptor::PointPool(known(
            id("pool"),
            PointPoolSchema {
                scope: PointPoolScope::Shared,
            },
        )));
        let build = BuildInput {
            game_version: ns(),
            allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([29; 16]), 10),
            revision: BuildRevision::from_u64(1),
            character: CharacterSpec {
                class: id("class"),
                ascendancy: Some(id("ascendancy")),
                level: 20,
                rewards: vec![],
            },
            weapon_loadouts: vec![instance(1)],
            active_weapon_loadout: instance(1),
            items: vec![],
            gems: vec![],
            equipment: vec![],
            allocations: vec![],
            skills: vec![],
            supports: vec![],
            payload_links: vec![],
            choices: vec![],
        };
        Self { index, build }
    }
    fn request(&self) -> OwnedEvaluationRequest {
        let limits = OwnedInputLimits::default();
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), limits).unwrap(),
            ScenarioSpec::new(
                ScenarioInput {
                    game_version: ns(),
                    enemy: EnemySpec {
                        encounter: id("encounter"),
                        level: 20,
                    },
                    assumptions: vec![],
                    usage: vec![],
                },
                limits,
            )
            .unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: ns(),
                    requests: vec![],
                },
                limits,
            )
            .unwrap(),
            limits,
        )
        .unwrap()
    }
    fn bind(&self) -> DefinitionBindingReport {
        bind_owned_request(&self.index, &self.request(), BindingLimits::default()).unwrap()
    }
    fn choice(&mut self) {
        self.index
            .passive()
            .declarations
            .choices
            .members
            .push(slot("choice"));
        let d = SlotDescriptor::Choice(known(
            slot("choice"),
            ChoiceSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::RequiredOnce,
                owners: vec![ChoiceOwnerScope::Character],
            },
        ));
        self.index.slots.insert(d.address(), d);
    }
    fn grant(&mut self) {
        self.index
            .passive()
            .declarations
            .actors
            .members
            .push(slot("actor"));
        self.index
            .passive()
            .declarations
            .grants
            .members
            .push(slot("grant"));
        let actor = SlotDescriptor::Actor(known(
            slot("actor"),
            ActorSlotSchema {
                skills: empty(),
                outputs: empty(),
            },
        ));
        self.index.slots.insert(actor.address(), actor);
        let grant = SlotDescriptor::Grant(known(
            slot("grant"),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::Character],
                target: GrantTarget::Actor(slot("actor")),
            },
        ));
        self.index.slots.insert(grant.address(), grant);
    }
}

#[test]
fn shared_roots_expose_exact_character_owners_without_allocating_instances() {
    let f = Fixture::new();
    let request = f.request();
    let before = serde_json::to_vec(&request).unwrap();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let resolution = resolver.provider(&root()).unwrap();
    assert_eq!(resolution.schema(), SchemaBindingStatus::Valid);
    let ProviderExposure::Root {
        owners,
        implicit_passives,
        ..
    } = resolution.value().unwrap().exposure()
    else {
        panic!("root exposure")
    };
    assert_eq!(
        owners
            .iter()
            .map(|o| o.definition().clone())
            .collect::<Vec<_>>(),
        vec![
            SlotOwnerDefId::Class(id("class")),
            SlotOwnerDefId::Ascendancy(id("ascendancy")),
            SlotOwnerDefId::PassiveNode(id("shared-root")),
            owner()
        ]
    );
    assert_eq!(implicit_passives.len(), 2);
    assert_eq!(
        implicit_passives[0].owner(),
        &SlotOwnerDefId::Class(id("class"))
    );
    assert_eq!(implicit_passives[1].nodes().len(), 1);
    assert!(request.build().input().allocations.is_empty());
    assert_eq!(serde_json::to_vec(&request).unwrap(), before);
    assert_eq!(f.bind().schema(), SchemaBindingStatus::Valid);
}

#[test]
fn implicit_root_choices_are_required_at_character_scope_and_exact_owner() {
    let mut f = Fixture::new();
    f.choice();
    let report = f.bind();
    assert_eq!(report.schema(), SchemaBindingStatus::Invalid);
    assert!(
        report
            .issues()
            .iter()
            .any(|i| i.code == BindingIssueCode::RequiredValueMissing
                && i.site.location == BindingLocation::Character)
    );
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Character,
        choice: ChoiceSelection {
            slot: slot("choice"),
            value: ParameterValue::Boolean(false),
        },
    });
    assert_eq!(f.bind().schema(), SchemaBindingStatus::Valid);
    f.build.choices[0].choice.slot.declaration = SlotOwnerDefId::Class(id("class"));
    assert_eq!(f.bind().schema(), SchemaBindingStatus::Invalid);
}

#[test]
fn class_changes_remove_old_passive_choices_and_grants_without_retargeting() {
    let mut f = Fixture::new();
    f.grant();
    let old = f.request();
    let resolver = OwnedOccurrenceResolver::new(&f.index, &old, BindingLimits::default()).unwrap();
    let granted = ProviderKey {
        root: ProviderRoot::Character,
        grant_path: vec![slot("grant")],
    };
    let result = resolver.provider(&granted).unwrap();
    let value = result.value().unwrap();
    assert_eq!(value.key(), &granted);
    assert_eq!(value.role(), ProviderRole::Character);
    let ProviderExposure::Actor {
        key, parent_actor, ..
    } = value.exposure()
    else {
        panic!("actor")
    };
    assert_eq!(key.provider, root());
    assert_eq!(key.slot, slot("actor"));
    assert_eq!(*parent_actor, ActorKey::Player);
    f.build.character.class = id("other-class");
    let next = f.request();
    let next_resolver =
        OwnedOccurrenceResolver::new(&f.index, &next, BindingLimits::default()).unwrap();
    assert_ne!(resolver.request_digest(), next_resolver.request_digest());
    let missing = next_resolver.provider(&granted).unwrap();
    assert_eq!(missing.status(), SelectorBindingStatus::Unavailable);
    assert!(missing.value().is_none());
    f.choice();
    f.build.choices.push(MechanicChoice {
        owner: ChoiceOwner::Character,
        choice: ChoiceSelection {
            slot: slot("choice"),
            value: ParameterValue::Boolean(true),
        },
    });
    assert_eq!(f.bind().schema(), SchemaBindingStatus::Invalid);
}

#[test]
fn partial_root_and_pool_closures_remain_explicit_without_erasing_known_owners() {
    let mut f = Fixture::new();
    f.index.class().implicit_passives.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            id::<ClassDefinition>("class").address(),
        ))],
    };
    f.index.passive().pools.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            id::<PassiveNodeDefinition>("class-root").address(),
        ))],
    };
    let report = f.bind();
    assert_eq!(report.schema(), SchemaBindingStatus::Unresolved);
    assert!(report.issues().iter().any(
        |i| i.code == BindingIssueCode::PartialMembership && i.site.facet == BindingFacet::Pool
    ));
    let request = f.request();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    let resolution = resolver.provider(&root()).unwrap();
    let ProviderExposure::Root {
        owners,
        implicit_passives,
        ..
    } = resolution.value().unwrap().exposure()
    else {
        panic!("root exposure")
    };
    assert!(owners.iter().any(|o| o.definition() == &owner()));
    assert!(!implicit_passives[0].declaration().is_complete());
    let (_, SchemaLookup::Known(schema)) = implicit_passives[0]
        .nodes()
        .iter()
        .find(|(node, _)| **node == id("class-root"))
        .unwrap()
    else {
        panic!("known root")
    };
    assert!(!schema.pools.is_complete());
}

#[test]
fn missing_and_unmapped_root_schemas_stay_in_evidence_and_do_not_become_owners() {
    for unmapped in [false, true] {
        let mut f = Fixture::new();
        f.index
            .definitions
            .remove(&id::<PassiveNodeDefinition>("class-root").address());
        if unmapped {
            f.index
                .put(DefinitionDescriptor::PassiveNode(DefinitionEntry {
                    id: id("class-root"),
                    schema: SchemaState::Unmapped {
                        gaps: vec![gap(SchemaSubject::Definition(
                            id::<PassiveNodeDefinition>("class-root").address(),
                        ))],
                    },
                }));
        }
        assert_eq!(f.bind().schema(), SchemaBindingStatus::Unresolved);
        let request = f.request();
        let resolver =
            OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
        let resolution = resolver.provider(&root()).unwrap();
        let ProviderExposure::Root {
            owners,
            implicit_passives,
            ..
        } = resolution.value().unwrap().exposure()
        else {
            panic!("root exposure")
        };
        assert!(!owners.iter().any(|o| o.definition() == &owner()));
        assert!(
            owners
                .iter()
                .any(|o| o.definition() == &SlotOwnerDefId::PassiveNode(id("shared-root")))
        );
        let (_, lookup) = implicit_passives[0]
            .nodes()
            .iter()
            .find(|(node, _)| **node == id("class-root"))
            .unwrap();
        assert_eq!(matches!(lookup, SchemaLookup::Unmapped(_)), unmapped);
        assert_eq!(matches!(lookup, SchemaLookup::Missing), !unmapped);
    }
}

#[test]
fn custom_index_root_contradictions_and_unused_root_work_are_rejected() {
    let base = Fixture::new();
    for kind in 0..4 {
        let mut f = base.clone();
        match kind {
            0 => f.index.class().implicit_passives.members.push(
                DefId::parse(
                    GameVersionNamespace::new("foreign-game", "v1").unwrap(),
                    "foreign-root",
                )
                .unwrap(),
            ),
            1 => f
                .index
                .class()
                .implicit_passives
                .members
                .push(id("class-root")),
            2 => f.index.passive().pools.members.push(id("pool")),
            _ => {
                f.index.class().implicit_passives.closure = SchemaClosure::Partial { gaps: vec![] }
            }
        }
        assert!(bind_owned_request(&f.index, &f.request(), BindingLimits::default()).is_err());
    }
    let request = base.request();
    let tight = OwnedOccurrenceResolver::new(
        &base.index,
        &request,
        BindingLimits {
            max_work: 3,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        tight.provider(&root()),
        Err(BindingError::WorkLimit)
    ));
    let mut paid = base;
    paid.build.allocations.push(Allocation {
        id: instance(2),
        node: id("class-root"),
        pool: id("pool"),
        scope: LoadoutScope::Shared,
        access: AllocationAccess::Ordinary,
        choices: vec![],
    });
    let report = paid.bind();
    assert_eq!(report.schema(), SchemaBindingStatus::Invalid);
    assert!(
        report
            .issues()
            .iter()
            .any(|i| i.code == BindingIssueCode::NotDeclared
                && i.site.location == BindingLocation::Allocation(instance(2)))
    );
}

#[test]
fn implicit_passive_fields_are_required_and_duplicate_wire_fields_reject() {
    let f = Fixture::new();
    for address in [
        id::<ClassDefinition>("class").address(),
        id::<AscendancyDefinition>("ascendancy").address(),
    ] {
        let raw = serde_json::to_string(&f.index.definitions[&address]).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        value["value"]["schema"]["value"]
            .as_object_mut()
            .unwrap()
            .remove("implicit_passives");
        assert!(serde_json::from_value::<DefinitionDescriptor>(value).is_err());
        let duplicate=raw.replacen("\"implicit_passives\":", "\"implicit_passives\":{\"members\":[],\"closure\":{\"kind\":\"complete\"}},\"implicit_passives\":",1);
        assert_ne!(duplicate, raw);
        assert!(serde_json::from_str::<DefinitionDescriptor>(&duplicate).is_err());
    }
}

#[test]
fn one_pool_can_admit_both_scopes_without_changing_restrictive_pool_contracts() {
    for eligibility in [
        PointPoolScope::Shared,
        PointPoolScope::PerLoadout,
        PointPoolScope::Either,
    ] {
        for selected in [false, true] {
            let mut f = Fixture::new();
            f.index.put(DefinitionDescriptor::PointPool(known(
                id("pool"),
                PointPoolSchema { scope: eligibility },
            )));
            f.index.put(DefinitionDescriptor::PassiveNode(known(
                id("paid"),
                PassiveNodeSchema {
                    pools: DeclaredSet::complete(vec![id("pool")]),
                    adjacent: empty(),
                    declarations: declarations(),
                },
            )));
            f.build.allocations.push(Allocation {
                id: instance(2),
                node: id("paid"),
                pool: id("pool"),
                scope: if selected {
                    LoadoutScope::Selected {
                        loadouts: vec![instance(1)],
                    }
                } else {
                    LoadoutScope::Shared
                },
                access: AllocationAccess::Ordinary,
                choices: vec![],
            });
            let admitted = matches!(
                (eligibility, selected),
                (PointPoolScope::Either, _)
                    | (PointPoolScope::Shared, false)
                    | (PointPoolScope::PerLoadout, true)
            );
            let report = f.bind();
            assert_eq!(
                report.schema(),
                if admitted {
                    SchemaBindingStatus::Valid
                } else {
                    SchemaBindingStatus::Invalid
                }
            );
            assert_eq!(
                report
                    .issues()
                    .iter()
                    .any(|i| i.code == BindingIssueCode::IncompatibleScope),
                !admitted
            );
        }
    }
}
