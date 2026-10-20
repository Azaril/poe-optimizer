//! Actual declaration path through the native occurrence binder; no evaluator/activation claim.
#[allow(dead_code)]
#[path = "support/owned_water_command_allocation.rs"]
mod packet;
use poe_optimizer_core::{
    build_identity::*, data::DataIdentity, owned_binding::*, owned_build::*, owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use std::collections::BTreeMap;
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(hex: &str) -> DefId<K> {
    DefId::parse(ns(), format!("def.{hex:0>16}")).unwrap()
}
fn id<K: BuildInstanceId>(n: u64) -> K {
    K::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([77; 16]), n).unwrap())
}
fn declaration<K: DefinitionDomain>(owner: SlotOwnerDefId, hex: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: def(hex),
    }
}
fn tree_grant() -> DeclaredSlot<GrantSlotDefId> {
    declaration(SlotOwnerDefId::PassiveNode(def("10d4")), "32d5")
}
fn command_grant() -> DeclaredSlot<GrantSlotDefId> {
    declaration(SlotOwnerDefId::Skill(def("032c")), "32bb")
}
fn command_supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    declaration(SlotOwnerDefId::Skill(def("032c")), "32ba")
}
fn tree_provider(n: u64) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::Allocation(id(n)),
        grant_path: vec![tree_grant(), command_grant()],
    }
}
fn manual_provider() -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(id(3)),
        grant_path: vec![command_grant()],
    }
}
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
    fn new(corrected: bool) -> Self {
        let deps = packet::dependencies();
        let definitions: Vec<DefinitionDescriptor> =
            serde_json::from_value(deps["definitions"].clone()).unwrap();
        let slots: Vec<SlotDescriptor> = serde_json::from_value(deps["slots"].clone()).unwrap();
        // A finite table exposing the actual captured rows, not a complete package claim.
        let mut this = Self {
            identity: serde_json::from_value(deps["source"]["definitions"].clone()).unwrap(),
            namespace: ns(),
            definitions: definitions.into_iter().map(|d| (d.address(), d)).collect(),
            slots: slots.into_iter().map(|s| (s.address(), s)).collect(),
        };
        if corrected {
            let m: OwnedReleaseMigrationInput = packet::read("migration.json");
            let SchemaExtensionEntry::Slot(s) = m.schema.into_iter().next().unwrap() else {
                panic!("slot")
            };
            assert!(this.slots.insert(s.address(), s).is_some());
        }
        this
    }
}
fn request(enabled: bool) -> OwnedEvaluationRequest {
    let limits = OwnedInputLimits::default();
    let build = BuildSpec::new(
        BuildInput {
            usage_preferences: vec![],
            generated_inputs: None,
            authored_support_order: None,
            allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([77; 16]), 100),
            revision: BuildRevision::from_u64(1),
            game_version: ns(),
            character: CharacterSpec {
                class: def("000a"),
                ascendancy: None,
                level: 90,
                rewards: vec![],
            },
            weapon_loadouts: vec![id(1)],
            active_weapon_loadout: id(1),
            items: vec![],
            gems: vec![],
            equipment: vec![],
            allocations: [2, 4]
                .map(|n| Allocation {
                    id: id(n),
                    node: def("10d4"),
                    pool: def("000c"),
                    scope: LoadoutScope::Shared,
                    access: AllocationAccess::Ordinary,
                    choices: vec![],
                })
                .to_vec(),
            skills: vec![SkillUse {
                id: id(3),
                source: AuthoredSkillSource::Direct(def("032c")),
                parameters: None,
                enabled,
                scope: LoadoutScope::Shared,
            }],
            supports: vec![],
            payload_links: vec![],
            choices: vec![],
        },
        limits,
    )
    .unwrap();
    let scenario = ScenarioSpec::new(
        ScenarioInput {
            game_version: ns(),
            enemy: EnemySpec {
                encounter: def("000b"),
                level: 90,
            },
            assumptions: vec![],
            usage: vec![],
            usage_intents: vec![],
        },
        limits,
    )
    .unwrap();
    let queries = QuerySpec::new(
        QueryInput {
            game_version: ns(),
            requests: vec![],
        },
        limits,
    )
    .unwrap();
    OwnedEvaluationRequest::new(build, scenario, queries, limits).unwrap()
}
fn assert_command(
    result: &OccurrenceResolution<ProviderOccurrence<'_>>,
    root: ProviderRoot,
    parent: Vec<DeclaredSlot<GrantSlotDefId>>,
) {
    assert_eq!(
        result.schema(),
        SchemaBindingStatus::Valid,
        "{:?}",
        result.issues()
    );
    assert_eq!(result.status(), SelectorBindingStatus::PendingResolution);
    let value = result.value().unwrap();
    assert_eq!(value.actor(), &ActorKey::Player);
    let ProviderExposure::Skill {
        key,
        owner,
        outputs,
        ..
    } = value.exposure()
    else {
        panic!("exact supplied Skill")
    };
    assert_eq!(owner, &SlotOwnerDefId::Skill(def("00d9")));
    assert_eq!(
        key,
        &GeneratedSkillKey {
            provider: ProviderKey {
                root,
                grant_path: parent
            },
            slot: command_supply()
        }
    );
    assert!(
        !outputs.is_complete(),
        "unreviewed output inventory is preserved"
    );
    assert_eq!(
        outputs.members,
        vec![declaration(SlotOwnerDefId::Skill(def("00d9")), "32bc")]
    );
}
#[test]
fn authenticated_delta_adds_only_the_allocation_role_and_keeps_source_joins() {
    packet::check_authored();
}
#[test]
fn native_tree_route_is_corrected_and_manual_source_remains_independent() {
    packet::check_authored();
    let old = Index::new(false);
    let next = Index::new(true);
    let request = request(true);
    let before = OwnedOccurrenceResolver::new(&old, &request, BindingLimits::default()).unwrap();
    let after = OwnedOccurrenceResolver::new(&next, &request, BindingLimits::default()).unwrap();
    let rejected = before.provider(&tree_provider(2)).unwrap();
    assert_eq!(rejected.schema(), SchemaBindingStatus::Invalid);
    assert!(rejected.value().is_none());
    assert!(
        rejected
            .issues()
            .iter()
            .any(|i| i.code == BindingIssueCode::IncompatibleRole
                && i.subject
                    == Some(SchemaSubject::Slot(GrantSlotDefId::address(
                        &command_grant()
                    ))))
    );
    for n in [2, 4] {
        let result = after.provider(&tree_provider(n)).unwrap();
        assert_eq!(result.value().unwrap().role(), ProviderRole::Allocation);
        assert_command(&result, ProviderRoot::Allocation(id(n)), vec![tree_grant()]);
    }
    for resolver in [&before, &after] {
        let result = resolver.provider(&manual_provider()).unwrap();
        assert_eq!(result.value().unwrap().role(), ProviderRole::SkillUse);
        assert_command(&result, ProviderRoot::SkillUse(id(3)), vec![]);
    }
    assert_ne!(
        after
            .provider(&tree_provider(2))
            .unwrap()
            .value()
            .unwrap()
            .key(),
        after
            .provider(&tree_provider(4))
            .unwrap()
            .value()
            .unwrap()
            .key()
    );
}
#[test]
fn native_path_does_not_admit_skipped_reordered_foreign_or_disabled_sources() {
    let index = Index::new(true);
    let request = request(false);
    let resolver =
        OwnedOccurrenceResolver::new(&index, &request, BindingLimits::default()).unwrap();
    for path in [
        vec![command_grant()],
        vec![command_grant(), tree_grant()],
        vec![
            tree_grant(),
            declaration(SlotOwnerDefId::Skill(def("0322")), "32a6"),
        ],
    ] {
        let result = resolver
            .provider(&ProviderKey {
                root: ProviderRoot::Allocation(id(2)),
                grant_path: path,
            })
            .unwrap();
        assert!(result.value().is_none());
        assert!(
            result
                .issues()
                .iter()
                .any(|i| i.code == BindingIssueCode::NotDeclared)
        );
    }
    let unrelated = ProviderKey {
        root: ProviderRoot::SkillUse(id(3)),
        grant_path: vec![tree_grant(), command_grant()],
    };
    assert!(
        resolver
            .structural_provider(&unrelated)
            .unwrap()
            .value()
            .is_none()
    );
    let unavailable = resolver.provider(&manual_provider()).unwrap();
    assert_eq!(unavailable.status(), SelectorBindingStatus::Unavailable);
    assert!(
        unavailable
            .issues()
            .iter()
            .any(|i| i.code == BindingIssueCode::DisabledProvider)
    );
    // Structural preparation still sees that disabled root; execution remains unavailable.
    assert_command(
        &resolver.structural_provider(&manual_provider()).unwrap(),
        ProviderRoot::SkillUse(id(3)),
        vec![],
    );
    assert_command(
        &resolver.provider(&tree_provider(2)).unwrap(),
        ProviderRoot::Allocation(id(2)),
        vec![tree_grant()],
    );
}
#[test]
fn mutation_of_target_or_extra_root_authority_is_rejected() {
    let m: OwnedReleaseMigrationInput = packet::read("migration.json");
    for extra in [
        ProviderRole::Character,
        ProviderRole::EquipmentUse,
        ProviderRole::SupportAssignment,
    ] {
        let mut bad = m.clone();
        let SchemaExtensionEntry::Slot(SlotDescriptor::Grant(row)) = &mut bad.schema[0] else {
            panic!("grant")
        };
        let SchemaState::Known(schema) = &mut row.schema else {
            panic!("known")
        };
        schema.provider_roles.push(extra);
        assert!(std::panic::catch_unwind(|| packet::check_delta(&bad)).is_err());
    }
    let mut bad = m;
    let SchemaExtensionEntry::Slot(SlotDescriptor::Grant(row)) = &mut bad.schema[0] else {
        panic!("grant")
    };
    let SchemaState::Known(schema) = &mut row.schema else {
        panic!("known")
    };
    schema.target = GrantTarget::Skill(declaration(SlotOwnerDefId::Skill(def("0322")), "32a5"));
    assert!(std::panic::catch_unwind(|| packet::check_delta(&bad)).is_err());
}
