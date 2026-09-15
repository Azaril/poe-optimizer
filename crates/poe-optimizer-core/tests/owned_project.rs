//! Directly authored project laws; no source checkout, game catalog, XML or UI.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_inventory::*, owned_project::*,
};
use serde_json::{Value, json};

fn limits() -> OwnedInputLimits {
    OwnedInputLimits::default()
}
fn lineage() -> BuildLineage {
    BuildLineage::from_bytes([0x72; 16])
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("project-game", "v1").unwrap()
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(lineage(), local).unwrap())
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn slot<K: DefinitionDomain>(key: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("gem")),
        slot: def(key),
    }
}
fn choice(owner: ChoiceOwner, key: &str) -> MechanicChoice {
    MechanicChoice {
        owner,
        choice: ChoiceSelection {
            slot: slot(key),
            value: ParameterValue::Boolean(true),
        },
    }
}
fn provider(root: ProviderRoot) -> ProviderKey {
    ProviderKey {
        root,
        grant_path: vec![],
    }
}
fn item(local: u64) -> ItemRecord {
    ItemRecord {
        id: id(local),
        template: def("item"),
        item_level: Some(40),
        quality: None,
        parameters: vec![ParameterAssignment {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::ItemTemplate(def("item")),
                slot: def("intrinsic"),
            },
            value: ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
        }],
        modifiers: vec![RolledModifier {
            id: id(local + 1),
            definition: def("modifier"),
            rolls: vec![],
        }],
    }
}
fn gem(local: u64) -> GemInstance {
    GemInstance {
        id: id(local),
        definition: def("gem"),
        level: 12,
        parameters: vec![],
        quality: None,
    }
}
fn selection() -> VariantSelection {
    VariantSelection {
        character: id(100),
        equipment: id(110),
        allocations: id(120),
        skills: id(130),
        choices: id(140),
        active_weapon_loadout: id(2),
    }
}
fn input() -> ProjectInput {
    let generated = SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: provider(ProviderRoot::EquipmentUse(id(40))),
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::ItemTemplate(def("item")),
            slot: def("granted-skill"),
        },
    }));
    ProjectInput {
        allocator: InstanceAllocatorState::from_parts(lineage(), 200),
        revision: BuildRevision::from_u64(8),
        game_version: namespace(),
        weapon_loadouts: vec![id(2), id(1)],
        items: vec![item(14), item(12), item(10)],
        gems: vec![gem(22), gem(21), gem(20)],
        rewards: vec![
            RewardSelection {
                id: id(31),
                definition: def("reward"),
                parameters: vec![],
            },
            RewardSelection {
                id: id(30),
                definition: def("reward"),
                parameters: vec![],
            },
        ],
        equipment: vec![
            EquipmentUse {
                id: id(40),
                item: id(10),
                destination: EquipmentDestination::CharacterSlot(def("ring-a")),
                scope: LoadoutScope::Shared,
            },
            EquipmentUse {
                id: id(41),
                item: id(10),
                destination: EquipmentDestination::CharacterSlot(def("ring-b")),
                scope: LoadoutScope::Shared,
            },
            EquipmentUse {
                id: id(42),
                item: id(12),
                destination: EquipmentDestination::CharacterSlot(def("weapon")),
                scope: LoadoutScope::Selected {
                    loadouts: vec![id(2), id(1)],
                },
            },
            EquipmentUse {
                id: id(43),
                item: id(12),
                destination: EquipmentDestination::ItemSocket {
                    container: id(40),
                    slot: def("socket"),
                },
                scope: LoadoutScope::Shared,
            },
        ],
        allocations: vec![
            Allocation {
                id: id(50),
                node: def("ordinary-node"),
                pool: def("ordinary-pool"),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            },
            Allocation {
                id: id(51),
                node: def("ascendancy-node"),
                pool: def("ascendancy-pool"),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            },
            Allocation {
                id: id(52),
                node: def("granted-node"),
                pool: def("ordinary-pool"),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Granted(provider(ProviderRoot::SupportAssignment(id(
                    70,
                )))),
                choices: vec![],
            },
        ],
        skills: vec![
            SkillUse {
                id: id(60),
                source: AuthoredSkillSource::Gem(id(20)),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
            SkillUse {
                id: id(61),
                source: AuthoredSkillSource::Gem(id(20)),
                enabled: false,
                scope: LoadoutScope::Shared,
            },
            SkillUse {
                id: id(62),
                source: AuthoredSkillSource::Direct(def("direct-skill")),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
        ],
        supports: vec![
            SupportAssignment {
                id: id(70),
                support: id(21),
                target: SkillTarget::Authored(id(60)),
                enabled: true,
            },
            SupportAssignment {
                id: id(71),
                support: id(21),
                target: generated,
                enabled: true,
            },
        ],
        payload_links: vec![PayloadLink {
            id: id(80),
            container: id(60),
            payload: id(61),
            role: def("payload"),
        }],
        character_presets: vec![
            CharacterPreset {
                id: id(101),
                class: def("second-class"),
                ascendancy: Some(def("ascendancy")),
                level: 70,
                rewards: vec![id(31)],
            },
            CharacterPreset {
                id: id(100),
                class: def("first-class"),
                ascendancy: None,
                level: 50,
                rewards: vec![id(30)],
            },
        ],
        equipment_presets: vec![
            EquipmentPreset {
                id: id(112),
                equipment: vec![id(43), id(40)],
            },
            EquipmentPreset {
                id: id(110),
                equipment: vec![id(41), id(40)],
            },
            EquipmentPreset {
                id: id(111),
                equipment: vec![id(42)],
            },
        ],
        allocation_presets: vec![
            AllocationPreset {
                id: id(121),
                allocations: vec![id(52)],
                equipment: vec![],
            },
            AllocationPreset {
                id: id(120),
                allocations: vec![id(51), id(50)],
                equipment: vec![],
            },
        ],
        skill_presets: vec![
            SkillPreset {
                id: id(131),
                skills: vec![id(62)],
                supports: vec![],
                payload_links: vec![],
            },
            SkillPreset {
                id: id(130),
                skills: vec![id(61), id(60)],
                supports: vec![id(70)],
                payload_links: vec![id(80)],
            },
            SkillPreset {
                id: id(132),
                skills: vec![id(60)],
                supports: vec![id(71)],
                payload_links: vec![],
            },
        ],
        choice_presets: vec![
            ChoicePreset {
                id: id(141),
                rewards: vec![],
                choices: vec![choice(
                    ChoiceOwner::Provider(provider(ProviderRoot::SupportAssignment(id(70)))),
                    "support-choice",
                )],
            },
            ChoicePreset {
                id: id(140),
                rewards: vec![],
                choices: vec![choice(ChoiceOwner::Character, "character-choice")],
            },
            ChoicePreset {
                id: id(149),
                rewards: vec![],
                choices: vec![],
            },
        ],
        saved_variants: vec![SavedVariant {
            id: id(150),
            selection: selection(),
        }],
    }
}
fn project() -> BuildProject {
    BuildProject::new(input(), limits()).unwrap()
}
fn stock(items: Vec<ItemRecord>, copies: Vec<InventoryItem>) -> InventorySnapshot {
    InventorySnapshot::new(
        InventoryInput {
            allocator: InstanceAllocatorState::from_parts(lineage(), 250),
            revision: BuildRevision::from_u64(20),
            game_version: namespace(),
            items,
            copies,
            completeness: InventoryCompleteness::Partial,
        },
        limits(),
    )
    .unwrap()
}
fn structure_error(error: ProjectError) -> StructuralErrorKind {
    match error {
        ProjectError::Structure(error) => {
            assert!(!error.path.is_empty());
            error.kind
        }
        other => panic!("expected structural error, got {other:?}"),
    }
}
fn reverse_arrays(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values.iter_mut() {
                reverse_arrays(value);
            }
            values.reverse();
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                reverse_arrays(value);
            }
        }
        _ => {}
    }
}
fn array_stats(value: &Value) -> (usize, usize) {
    match value {
        Value::Array(values) => values
            .iter()
            .map(array_stats)
            .fold((values.len(), values.len()), |(sum, max), (s, m)| {
                (sum + s, max.max(m))
            }),
        Value::Object(values) => values
            .values()
            .map(array_stats)
            .fold((0, 0), |(sum, max), (s, m)| (sum + s, max.max(m))),
        _ => (0, 0),
    }
}

#[test]
fn explicit_independent_selection_preserves_shared_records_distinct_uses_and_payloads() {
    let project = project();
    let build = compose(&project, &selection(), None, limits()).unwrap();
    let selected = build.input();
    assert_eq!(
        selected.character.class,
        def::<ClassDefinition>("first-class")
    );
    assert_eq!(selected.character.level, 50);
    assert_eq!(selected.character.rewards[0].id, id(30));
    assert_eq!(selected.active_weapon_loadout, id(2));
    assert_eq!(
        selected.equipment.iter().map(|v| v.id).collect::<Vec<_>>(),
        vec![id(40), id(41)]
    );
    assert_eq!(selected.items.len(), 1);
    assert_eq!(selected.items[0].id, id(10));
    assert!(selected.equipment.iter().all(|v| v.item == id(10)));
    assert_eq!(
        selected.gems.iter().map(|v| v.id).collect::<Vec<_>>(),
        vec![id(20), id(21)]
    );
    assert_eq!(
        selected.skills.iter().map(|v| v.id).collect::<Vec<_>>(),
        vec![id(60), id(61)]
    );
    assert_eq!(selected.skills[0].source, selected.skills[1].source);
    assert!(!selected.skills[1].enabled);
    assert_eq!(selected.supports[0].target, SkillTarget::Authored(id(60)));
    assert_eq!(selected.payload_links[0].container, id(60));
    assert_eq!(selected.payload_links[0].payload, id(61));
    assert_eq!(
        selected
            .allocations
            .iter()
            .map(|v| v.pool.clone())
            .collect::<Vec<_>>(),
        vec![
            def::<PointPoolDefinition>("ordinary-pool"),
            def("ascendancy-pool")
        ]
    );
    assert_eq!(selected.allocator, project.input().allocator);
    assert_eq!(selected.revision, project.input().revision);
    // The final build roundtrips through the standalone codec after project removal.
    let bytes = encode_owned(&OwnedDocument::Build(Box::new(build.clone())), limits()).unwrap();
    assert_eq!(
        decode_owned(&bytes, limits()).unwrap(),
        OwnedDocument::Build(Box::new(build))
    );
}

#[test]
fn changing_one_preset_never_changes_unselected_preset_domains_or_reuses_supply_ids() {
    let project = project();
    let first = compose(&project, &selection(), None, limits()).unwrap();
    let changed = VariantSelection {
        equipment: id(111),
        ..selection()
    };
    let second = compose(&project, &changed, None, limits()).unwrap();
    assert_eq!(first.input().character, second.input().character);
    assert_eq!(first.input().skills, second.input().skills);
    assert_eq!(first.input().allocations, second.input().allocations);
    assert_eq!(first.input().choices, second.input().choices);
    assert_eq!(second.input().equipment[0].id, id(42));
    assert_eq!(second.input().equipment[0].item, id(12));
    assert_eq!(second.input().items[0].id, id(12));
    assert_ne!(
        build_content_binding(&first, limits()).unwrap(),
        build_content_binding(&second, limits()).unwrap()
    );
    assert_eq!(
        project.saved_variant(id(150)).unwrap().selection,
        selection()
    );
    assert!(project.saved_variant(id(199)).is_none());
}

#[test]
fn canonical_project_wire_is_stable_under_independent_preset_and_record_permutations() {
    let raw = input();
    let first = BuildProject::new(raw.clone(), limits()).unwrap();
    let mut value = serde_json::to_value(raw).unwrap();
    reverse_arrays(&mut value);
    let second = BuildProject::new(serde_json::from_value(value).unwrap(), limits()).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    assert_eq!(
        compose(&first, &selection(), None, limits()).unwrap(),
        compose(&second, &selection(), None, limits()).unwrap()
    );
    let decoded: ProjectInput =
        serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
    assert_eq!(BuildProject::new(decoded, limits()).unwrap(), first);
}

#[test]
fn preset_and_occurrence_ids_are_global_domains_with_no_position_or_cast_fallback() {
    let mut duplicate = input();
    duplicate.character_presets[0].id = id(100);
    assert!(matches!(
        structure_error(BuildProject::new(duplicate, limits()).unwrap_err()),
        StructuralErrorKind::DuplicateIdentity { .. }
    ));
    let mut collision = input();
    collision.equipment_presets[0].id = id(10);
    assert!(matches!(
        structure_error(BuildProject::new(collision, limits()).unwrap_err()),
        StructuralErrorKind::DuplicateIdentity { .. }
    ));
    let mut wrong_domain = input();
    wrong_domain.skill_presets[0].skills = vec![id(40)];
    assert!(matches!(
        structure_error(BuildProject::new(wrong_domain, limits()).unwrap_err()),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::SkillUse,
            ..
        }
    ));
    let project = project();
    for changed in [
        VariantSelection {
            skills: id(100),
            ..selection()
        },
        VariantSelection {
            skills: id(199),
            ..selection()
        },
    ] {
        assert!(matches!(
            structure_error(compose(&project, &changed, None, limits()).unwrap_err()),
            StructuralErrorKind::MissingReference {
                expected: OccurrenceKind::SkillPreset,
                ..
            }
        ));
    }
    let mut wrong_saved = input();
    wrong_saved.saved_variants[0].selection.allocations = id(110);
    assert!(matches!(
        structure_error(BuildProject::new(wrong_saved, limits()).unwrap_err()),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::AllocationPreset,
            ..
        }
    ));
    let mut duplicate_ref = input();
    duplicate_ref.equipment_presets[0].equipment.push(id(40));
    assert_eq!(
        structure_error(BuildProject::new(duplicate_ref, limits()).unwrap_err()),
        StructuralErrorKind::DuplicateAssignment
    );
}

#[test]
fn lineage_namespace_watermark_and_unselected_corruption_reject() {
    let mut wrong = input();
    wrong.character_presets[0].id = CharacterPresetId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([9; 16]), 101).unwrap(),
    );
    assert_eq!(
        structure_error(BuildProject::new(wrong, limits()).unwrap_err()),
        StructuralErrorKind::ForeignLineage
    );
    let mut wrong = input();
    wrong.allocator = InstanceAllocatorState::from_parts(lineage(), 149);
    assert_eq!(
        structure_error(BuildProject::new(wrong, limits()).unwrap_err()),
        StructuralErrorKind::BeyondWatermark
    );
    let mut wrong = input();
    wrong.character_presets[0].class =
        ClassDefId::parse(GameVersionNamespace::new("foreign", "v1").unwrap(), "class").unwrap();
    assert_eq!(
        structure_error(BuildProject::new(wrong, limits()).unwrap_err()),
        StructuralErrorKind::ForeignNamespace
    );
    let mut wrong = input();
    wrong.items[0].modifiers[0].id = id(11);
    assert!(matches!(
        structure_error(BuildProject::new(wrong, limits()).unwrap_err()),
        StructuralErrorKind::DuplicateIdentity { .. }
    ));
    let mut wrong = input();
    wrong.items[0].parameters[0].slot.declaration = SlotOwnerDefId::Gem(def("gem"));
    assert_eq!(
        structure_error(BuildProject::new(wrong, limits()).unwrap_err()),
        StructuralErrorKind::WrongDeclaration
    );
}

#[test]
fn composition_checks_cross_preset_provider_and_container_membership() {
    let project = project();
    let supported = VariantSelection {
        allocations: id(121),
        choices: id(141),
        ..selection()
    };
    let result = compose(&project, &supported, None, limits()).unwrap();
    assert_eq!(
        result.input().allocations[0].access,
        AllocationAccess::Granted(provider(ProviderRoot::SupportAssignment(id(70))))
    );
    assert_eq!(
        result.input().choices[0].owner,
        ChoiceOwner::Provider(provider(ProviderRoot::SupportAssignment(id(70))))
    );
    let missing_support = VariantSelection {
        skills: id(131),
        ..supported
    };
    assert!(matches!(
        structure_error(compose(&project, &missing_support, None, limits()).unwrap_err()),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::SupportAssignment,
            ..
        }
    ));
    let generated = VariantSelection {
        skills: id(132),
        ..selection()
    };
    assert!(compose(&project, &generated, None, limits()).is_ok());
    let missing_equipment = VariantSelection {
        equipment: id(111),
        ..generated
    };
    assert!(matches!(
        structure_error(compose(&project, &missing_equipment, None, limits()).unwrap_err()),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::EquipmentUse,
            ..
        }
    ));
    let socket = compose(
        &project,
        &VariantSelection {
            equipment: id(112),
            ..selection()
        },
        None,
        limits(),
    )
    .unwrap();
    assert!(
        matches!(socket.input().equipment[1].destination,EquipmentDestination::ItemSocket{container,..} if container==id(40))
    );
    let mut raw = project.into_input();
    raw.equipment_presets
        .iter_mut()
        .find(|p| p.id == id(112))
        .unwrap()
        .equipment = vec![id(43)];
    let partial = BuildProject::new(raw, limits()).unwrap();
    assert!(matches!(
        structure_error(
            compose(
                &partial,
                &VariantSelection {
                    equipment: id(112),
                    ..selection()
                },
                None,
                limits()
            )
            .unwrap_err()
        ),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::EquipmentUse,
            ..
        }
    ));
}

#[test]
fn alternative_choices_are_separate_but_selected_allocation_assignments_must_not_overlap() {
    let mut raw = input();
    raw.choice_presets[0].choices = vec![choice(ChoiceOwner::Character, "character-choice")];
    raw.choice_presets[0].choices[0].choice.value = ParameterValue::Boolean(false);
    let alternative = BuildProject::new(raw.clone(), limits()).unwrap();
    assert!(
        compose(
            &alternative,
            &VariantSelection {
                choices: id(141),
                ..selection()
            },
            None,
            limits()
        )
        .is_ok()
    );
    let duplicate = raw.choice_presets[0].choices[0].clone();
    raw.choice_presets[0].choices.push(duplicate);
    assert_eq!(
        structure_error(BuildProject::new(raw, limits()).unwrap_err()),
        StructuralErrorKind::DuplicateAssignment
    );
    let mut raw = input();
    let chosen = choice(ChoiceOwner::Allocation(id(50)), "node-choice");
    raw.allocations[0].choices.push(chosen.choice.clone());
    raw.choice_presets[1].choices = vec![chosen];
    let project = BuildProject::new(raw, limits()).unwrap();
    assert_eq!(
        structure_error(compose(&project, &selection(), None, limits()).unwrap_err()),
        StructuralErrorKind::DuplicateAssignment
    );
    // Selecting another allocation preset does not activate an absent allocation's choice.
    assert!(
        compose(
            &project,
            &VariantSelection {
                choices: id(149),
                ..selection()
            },
            None,
            limits()
        )
        .is_ok()
    );
}

#[test]
fn registry_reuses_exact_modifier_provider_ownership_and_containment_checks() {
    let mut raw = input();
    raw.choice_presets[0].choices = vec![choice(
        ChoiceOwner::Provider(provider(ProviderRoot::ItemModifier {
            equipment_use: id(40),
            modifier: id(13),
        })),
        "choice",
    )];
    assert_eq!(
        structure_error(BuildProject::new(raw, limits()).unwrap_err()),
        StructuralErrorKind::WrongProviderOwner
    );
    let mut raw = input();
    raw.equipment[0].destination = EquipmentDestination::ItemSocket {
        container: id(43),
        slot: def("socket"),
    };
    assert_eq!(
        structure_error(BuildProject::new(raw, limits()).unwrap_err()),
        StructuralErrorKind::ContainmentCycle
    );
    let mut raw = input();
    raw.skill_presets[1].skills = vec![id(60)];
    let project = BuildProject::new(raw, limits()).unwrap();
    assert!(matches!(
        structure_error(compose(&project, &selection(), None, limits()).unwrap_err()),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::SkillUse,
            ..
        }
    ));
}

#[test]
fn inventory_union_checks_all_records_and_copy_domains_without_adopting_stock() {
    let project = project();
    let inventory = stock(
        vec![item(14), item(160)],
        vec![InventoryItem {
            id: id(170),
            item: id(160),
        }],
    );
    let selected = compose(&project, &selection(), Some(&inventory), limits()).unwrap();
    assert_eq!(selected.input().items.len(), 1);
    assert_eq!(selected.input().items[0].id, id(10));
    assert_eq!(selected.input().allocator.last_issued(), 250);
    assert_eq!(selected.input().revision, project.input().revision);
    let mut changed = item(14);
    *changed.item_level.as_mut().unwrap() += 1;
    let conflict = stock(vec![changed], vec![]);
    assert!(
        matches!(compose(&project,&selection(),Some(&conflict),limits()),Err(ProjectError::Inventory(InventoryError::ConflictingItemRecord(record))) if record==id(14))
    );
    let collision = stock(
        vec![item(160)],
        vec![InventoryItem {
            id: id(100),
            item: id(160),
        }],
    );
    assert!(matches!(
        structure_error(compose(&project, &selection(), Some(&collision), limits()).unwrap_err()),
        StructuralErrorKind::DuplicateIdentity { .. }
    ));
    let mut raw = input();
    raw.equipment[0].item = id(160);
    assert!(matches!(
        structure_error(BuildProject::new(raw, limits()).unwrap_err()),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::Item,
            ..
        }
    ));
}

#[test]
fn inventory_union_rejects_foreign_snapshots_and_modifier_collisions_even_unselected() {
    let project = project();
    let mut other = stock(vec![], vec![]).into_input();
    other.game_version = GameVersionNamespace::new("other-game", "v1").unwrap();
    let other = InventorySnapshot::new(other, limits()).unwrap();
    assert!(matches!(
        compose(&project, &selection(), Some(&other), limits()),
        Err(ProjectError::Inventory(InventoryError::ForeignNamespace))
    ));
    let mut other = stock(vec![], vec![]).into_input();
    other.allocator = InstanceAllocatorState::from_parts(BuildLineage::from_bytes([8; 16]), 300);
    let other = InventorySnapshot::new(other, limits()).unwrap();
    assert!(matches!(
        compose(&project, &selection(), Some(&other), limits()),
        Err(ProjectError::Inventory(InventoryError::ForeignLineage))
    ));
    let mut collision = item(160);
    collision.modifiers[0].id = id(11);
    let other = stock(vec![collision], vec![]);
    assert!(matches!(
        structure_error(compose(&project, &selection(), Some(&other), limits()).unwrap_err()),
        StructuralErrorKind::DuplicateIdentity { .. }
    ));
}

#[test]
fn project_aggregate_collection_and_provider_path_limits_cover_all_presets() {
    let raw = input();
    let (entries, collection) = array_stats(&serde_json::to_value(&raw).unwrap());
    let exact = OwnedInputLimits {
        max_entries: entries,
        max_collection_entries: collection,
        ..limits()
    };
    let project = BuildProject::new(raw.clone(), exact).unwrap();
    project.validate_limits(exact).unwrap();
    assert!(
        BuildProject::new(
            raw.clone(),
            OwnedInputLimits {
                max_entries: entries - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(
        BuildProject::new(
            raw.clone(),
            OwnedInputLimits {
                max_collection_entries: collection - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(
        project
            .validate_limits(OwnedInputLimits {
                max_entries: entries - 1,
                ..exact
            })
            .is_err()
    );
    let mut path = raw;
    let ChoiceOwner::Provider(provider) = &mut path.choice_presets[0].choices[0].owner else {
        panic!("fixture provider")
    };
    provider.grant_path = vec![slot("grant"), slot("grant")];
    assert_eq!(
        structure_error(
            BuildProject::new(
                path,
                OwnedInputLimits {
                    max_provider_steps: 1,
                    ..limits()
                }
            )
            .unwrap_err()
        ),
        StructuralErrorKind::LimitExceeded
    );
}

#[test]
fn raw_project_wire_has_no_defaults_unknown_fields_or_duplicate_field_escape() {
    let raw = input();
    let wire = serde_json::to_value(&raw).unwrap();
    for pointer in ["", "/character_presets/0", "/saved_variants/0/selection"] {
        let mut unknown = wire.clone();
        unknown
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("source_set_index".into(), json!(1));
        assert!(serde_json::from_value::<ProjectInput>(unknown).is_err());
    }
    let mut missing = wire.clone();
    missing["character_presets"][1]
        .as_object_mut()
        .unwrap()
        .remove("ascendancy");
    assert!(serde_json::from_value::<ProjectInput>(missing).is_err());
    let mut missing = wire;
    missing["saved_variants"][0]["selection"]
        .as_object_mut()
        .unwrap()
        .remove("active_weapon_loadout");
    assert!(serde_json::from_value::<ProjectInput>(missing).is_err());
    let encoded = serde_json::to_string(&selection()).unwrap();
    let duplicated = encoded.replacen(
        "{",
        &format!(
            "{{\"character\":{},",
            serde_json::to_string(&selection().character).unwrap()
        ),
        1,
    );
    assert!(serde_json::from_str::<VariantSelection>(&duplicated).is_err());
}

#[test]
fn every_preset_kind_allocates_independently_in_the_shared_monotonic_domain() {
    let mut allocator = InstanceAllocator::new(lineage());
    let values = [
        allocator
            .allocate::<CharacterPresetId>()
            .unwrap()
            .instance_id(),
        allocator
            .allocate::<EquipmentPresetId>()
            .unwrap()
            .instance_id(),
        allocator
            .allocate::<AllocationPresetId>()
            .unwrap()
            .instance_id(),
        allocator.allocate::<SkillPresetId>().unwrap().instance_id(),
        allocator
            .allocate::<ChoicePresetId>()
            .unwrap()
            .instance_id(),
        allocator
            .allocate::<SavedVariantId>()
            .unwrap()
            .instance_id(),
    ];
    assert_eq!(
        values.iter().map(|id| id.local()).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn exact_empty_provider_choice_aliases_reject_equal_and_conflicting_assignments() {
    let pairs = [
        (ChoiceOwner::Character, ProviderRoot::Character),
        (
            ChoiceOwner::EquipmentUse(id(40)),
            ProviderRoot::EquipmentUse(id(40)),
        ),
        (
            ChoiceOwner::Allocation(id(50)),
            ProviderRoot::Allocation(id(50)),
        ),
        (
            ChoiceOwner::Skill(SkillTarget::Authored(id(60))),
            ProviderRoot::SkillUse(id(60)),
        ),
    ];
    for (direct, root) in pairs {
        for same_value in [true, false] {
            let mut raw = input();
            let first = choice(direct.clone(), "same-port");
            let mut second = choice(ChoiceOwner::Provider(provider(root.clone())), "same-port");
            second.choice.value = ParameterValue::Boolean(same_value);
            raw.choice_presets[1].choices = vec![first.clone(), second.clone()];
            assert_eq!(
                structure_error(BuildProject::new(raw, limits()).unwrap_err()),
                StructuralErrorKind::DuplicateAssignment
            );
            // The shared check also protects directly authored standalone builds.
            let mut build = compose(&project(), &selection(), None, limits())
                .unwrap()
                .into_input();
            build.choices = vec![first, second];
            assert_eq!(
                BuildSpec::new(build, limits()).unwrap_err().kind,
                StructuralErrorKind::DuplicateAssignment
            );
        }
    }
}

#[test]
fn allocation_local_choice_and_empty_allocation_provider_alias_share_one_port() {
    for value in [true, false] {
        let mut raw = input();
        let local = choice(ChoiceOwner::Allocation(id(50)), "same-port");
        raw.allocations[0].choices = vec![local.choice];
        let mut alias = choice(
            ChoiceOwner::Provider(provider(ProviderRoot::Allocation(id(50)))),
            "same-port",
        );
        alias.choice.value = ParameterValue::Boolean(value);
        raw.choice_presets[1].choices = vec![alias];
        // Independent groups store a well-formed alternative; selection checks
        // it against the selected allocation-local assignment without merging.
        let project = BuildProject::new(raw, limits()).unwrap();
        assert_eq!(
            structure_error(compose(&project, &selection(), None, limits()).unwrap_err()),
            StructuralErrorKind::DuplicateAssignment
        );
    }
}

#[test]
fn authored_choice_owner_survives_and_nonempty_paths_or_other_roots_stay_distinct() {
    let mut raw = input();
    let root = ChoiceOwner::Provider(provider(ProviderRoot::Character));
    raw.choice_presets[1].choices = vec![choice(root.clone(), "same-port")];
    let project = BuildProject::new(raw, limits()).unwrap();
    let build = compose(&project, &selection(), None, limits()).unwrap();
    assert_eq!(build.input().choices[0].owner, root);
    let mut raw = project.into_input();
    let mut nested = provider(ProviderRoot::Character);
    nested.grant_path = vec![slot("grant")];
    raw.choice_presets
        .iter_mut()
        .find(|preset| preset.id == id(140))
        .unwrap()
        .choices = vec![
        choice(ChoiceOwner::Character, "same-port"),
        choice(ChoiceOwner::Provider(nested), "same-port"),
        choice(
            ChoiceOwner::Provider(provider(ProviderRoot::SupportAssignment(id(70)))),
            "same-port",
        ),
        choice(
            ChoiceOwner::Skill(SkillTarget::Authored(id(60))),
            "same-port",
        ),
    ];
    let project = BuildProject::new(raw, limits()).unwrap();
    assert_eq!(
        compose(&project, &selection(), None, limits())
            .unwrap()
            .input()
            .choices
            .len(),
        4
    );
}

fn contribution_input() -> ProjectInput {
    let mut raw = input();
    for (use_id, item_id, allocation_id) in [(44, 10, 50), (45, 12, 51)] {
        raw.equipment.push(EquipmentUse {
            id: id(use_id),
            item: id(item_id),
            destination: EquipmentDestination::PassiveSocket {
                allocation: id(allocation_id),
                slot: def("passive-socket"),
            },
            scope: LoadoutScope::Shared,
        });
    }
    for (preset_id, allocation_id, use_id) in [(120, 50, 44), (121, 51, 45)] {
        let preset = raw
            .allocation_presets
            .iter_mut()
            .find(|row| row.id == id(preset_id))
            .unwrap();
        preset.allocations = vec![id(allocation_id)];
        preset.equipment = vec![id(use_id)];
    }
    raw
}

#[test]
fn equipment_and_allocation_contributions_compose_all_four_independent_selections() {
    let raw = contribution_input();
    let project = BuildProject::new(raw.clone(), limits()).unwrap();
    let before = serde_json::to_vec(&project).unwrap();
    let document = OwnedDocument::Project(Box::new(project.clone()));
    assert_eq!(
        decode_owned(&encode_owned(&document, limits()).unwrap(), limits()).unwrap(),
        document
    );
    for (equipment_id, ordinary_ids) in [(110, vec![40, 41]), (111, vec![42])] {
        for (allocation_preset, allocation_id, socket_id) in [(120, 50, 44), (121, 51, 45)] {
            let selected = VariantSelection {
                equipment: id(equipment_id),
                allocations: id(allocation_preset),
                ..selection()
            };
            let actual = compose(&project, &selected, None, limits()).unwrap();
            let expected_use_ids: Vec<ItemSlotUseId> = ordinary_ids
                .iter()
                .copied()
                .chain([socket_id])
                .map(id)
                .collect();
            let equipment: Vec<_> = raw
                .equipment
                .iter()
                .filter(|row| expected_use_ids.contains(&row.id))
                .cloned()
                .collect();
            let expected = BuildSpec::new(
                BuildInput {
                    allocator: raw.allocator,
                    revision: raw.revision,
                    game_version: namespace(),
                    character: CharacterSpec {
                        class: def("first-class"),
                        ascendancy: None,
                        level: 50,
                        rewards: vec![
                            raw.rewards
                                .iter()
                                .find(|row| row.id == id(30))
                                .unwrap()
                                .clone(),
                        ],
                    },
                    weapon_loadouts: raw.weapon_loadouts.clone(),
                    active_weapon_loadout: id(2),
                    items: raw
                        .items
                        .iter()
                        .filter(|item| equipment.iter().any(|row| row.item == item.id))
                        .cloned()
                        .collect(),
                    equipment,
                    gems: vec![gem(20), gem(21)],
                    allocations: vec![
                        raw.allocations
                            .iter()
                            .find(|row| row.id == id(allocation_id))
                            .unwrap()
                            .clone(),
                    ],
                    skills: raw
                        .skills
                        .iter()
                        .filter(|row| [id(60), id(61)].contains(&row.id))
                        .cloned()
                        .collect(),
                    supports: vec![
                        raw.supports
                            .iter()
                            .find(|row| row.id == id(70))
                            .unwrap()
                            .clone(),
                    ],
                    payload_links: raw.payload_links.clone(),
                    choices: vec![choice(ChoiceOwner::Character, "character-choice")],
                },
                limits(),
            )
            .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(
                build_content_binding(&actual, limits()).unwrap(),
                build_content_binding(&expected, limits()).unwrap()
            );
            assert_eq!(
                actual
                    .input()
                    .equipment
                    .iter()
                    .map(|row| row.id)
                    .collect::<Vec<_>>(),
                expected_use_ids
            );
            assert_eq!(actual.input().allocator, raw.allocator);
            assert_eq!(actual.input().revision, raw.revision);
        }
    }
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
}

#[test]
fn shared_contribution_selects_one_use_but_distinct_uses_of_same_item_and_socket_survive() {
    let mut raw = contribution_input();
    let base = BuildProject::new(raw.clone(), limits()).unwrap();
    let before = compose(&base, &selection(), None, limits()).unwrap();
    raw.equipment_presets
        .iter_mut()
        .find(|p| p.id == id(110))
        .unwrap()
        .equipment
        .push(id(44));
    let overlap = BuildProject::new(raw.clone(), limits()).unwrap();
    let after = compose(&overlap, &selection(), None, limits()).unwrap();
    assert_eq!(before, after);
    assert_eq!(
        build_content_binding(&before, limits()).unwrap(),
        build_content_binding(&after, limits()).unwrap()
    );
    let mut second = raw
        .equipment
        .iter()
        .find(|r| r.id == id(44))
        .unwrap()
        .clone();
    second.id = id(46);
    raw.equipment.push(second);
    raw.allocation_presets
        .iter_mut()
        .find(|p| p.id == id(120))
        .unwrap()
        .equipment
        .push(id(46));
    let project = BuildProject::new(raw, limits()).unwrap();
    let result = compose(&project, &selection(), None, limits()).unwrap();
    assert_eq!(
        result
            .input()
            .equipment
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![id(40), id(41), id(44), id(46)]
    );
    assert_eq!(result.input().items.len(), 1);
    assert!(result.input().equipment.iter().all(|r| r.item == id(10)));
    assert_eq!(
        result.input().equipment[2].destination,
        result.input().equipment[3].destination
    );
}

#[test]
fn allocation_equipment_references_reject_duplicates_wrong_domains_and_unselected_dangling_ids() {
    for (ids, expected) in [
        (
            vec![id(44), id(44)],
            StructuralErrorKind::DuplicateAssignment,
        ),
        (
            vec![id(50)],
            StructuralErrorKind::MissingReference {
                expected: OccurrenceKind::EquipmentUse,
                id: id::<AllocationId>(50).instance_id(),
            },
        ),
        (
            vec![id(199)],
            StructuralErrorKind::MissingReference {
                expected: OccurrenceKind::EquipmentUse,
                id: id::<ItemSlotUseId>(199).instance_id(),
            },
        ),
    ] {
        let mut raw = contribution_input();
        raw.allocation_presets
            .iter_mut()
            .find(|p| p.id == id(121))
            .unwrap()
            .equipment = ids;
        assert_eq!(
            structure_error(BuildProject::new(raw, limits()).unwrap_err()),
            expected
        );
    }
}

#[test]
fn contribution_never_adopts_an_omitted_passive_container_or_inventory_only_item() {
    for (contribution, expected) in [
        (44, OccurrenceKind::Allocation),
        (43, OccurrenceKind::EquipmentUse),
    ] {
        let mut raw = contribution_input();
        raw.allocation_presets
            .iter_mut()
            .find(|p| p.id == id(121))
            .unwrap()
            .equipment = vec![id(contribution)];
        let project = BuildProject::new(raw, limits()).unwrap();
        let selected = VariantSelection {
            equipment: id(111),
            allocations: id(121),
            ..selection()
        };
        assert!(
            matches!(structure_error(compose(&project,&selected,None,limits()).unwrap_err()),StructuralErrorKind::MissingReference{expected:kind,..} if kind==expected)
        );
    }
    let mut raw = contribution_input();
    let stock = stock(vec![item(160)], vec![]);
    raw.equipment
        .iter_mut()
        .find(|row| row.id == id(44))
        .unwrap()
        .item = stock.input().items[0].id;
    assert!(matches!(
        structure_error(BuildProject::new(raw, limits()).unwrap_err()),
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::Item,
            ..
        }
    ));
}

#[test]
fn contribution_budget_counts_both_original_lists_before_union_and_canonicalizes_members() {
    let mut raw = contribution_input();
    raw.allocation_presets
        .iter_mut()
        .find(|p| p.id == id(120))
        .unwrap()
        .equipment = vec![id(44), id(41), id(40)];
    let (count, max_collection) = array_stats(&serde_json::to_value(&raw).unwrap());
    let exact = OwnedInputLimits {
        max_entries: count,
        max_collection_entries: max_collection,
        ..limits()
    };
    let first = BuildProject::new(raw.clone(), exact).unwrap();
    assert!(compose(&first, &selection(), None, exact).is_ok());
    let tighter = OwnedInputLimits {
        max_entries: count - 1,
        ..exact
    };
    assert!(matches!(
        structure_error(BuildProject::new(raw.clone(), tighter).unwrap_err()),
        StructuralErrorKind::LimitExceeded
    ));
    assert!(compose(&first, &selection(), None, tighter).is_err());
    let mut value = serde_json::to_value(raw).unwrap();
    reverse_arrays(&mut value);
    let second = BuildProject::new(serde_json::from_value(value).unwrap(), exact).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        compose(&first, &selection(), None, exact).unwrap(),
        compose(&second, &selection(), None, exact).unwrap()
    );
}

#[test]
fn choice_rewards_are_additive_without_overrides_or_cloning_shared_occurrences() {
    let raw = input();
    let baseline = compose(
        &BuildProject::new(raw.clone(), limits()).unwrap(),
        &selection(),
        None,
        limits(),
    )
    .unwrap();
    let mut overlap = raw.clone();
    overlap
        .choice_presets
        .iter_mut()
        .find(|p| p.id == id(140))
        .unwrap()
        .rewards = vec![id(30)];
    let overlap = BuildProject::new(overlap, limits()).unwrap();
    assert_eq!(
        compose(&overlap, &selection(), None, limits()).unwrap(),
        baseline
    );
    let mut two = raw;
    two.choice_presets
        .iter_mut()
        .find(|p| p.id == id(140))
        .unwrap()
        .rewards = vec![id(31), id(30)];
    let project = BuildProject::new(two, limits()).unwrap();
    let build = compose(&project, &selection(), None, limits()).unwrap();
    assert_eq!(
        build
            .input()
            .character
            .rewards
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![id(30), id(31)]
    );
    assert_eq!(
        build.input().character.rewards[0].definition,
        build.input().character.rewards[1].definition
    );
    let other = compose(
        &project,
        &VariantSelection {
            choices: id(149),
            ..selection()
        },
        None,
        limits(),
    )
    .unwrap();
    assert_eq!(
        other.input().character.rewards,
        baseline.input().character.rewards
    );
    let doc = OwnedDocument::Project(Box::new(project));
    assert_eq!(
        decode_owned(&encode_owned(&doc, limits()).unwrap(), limits()).unwrap(),
        doc
    );
}

#[test]
fn choice_reward_lists_validate_and_charge_original_references_before_cross_contributor_union() {
    for refs in [
        vec![id::<RewardSelectionId>(30), id(30)],
        vec![id(40)],
        vec![id(199)],
    ] {
        let mut raw = input();
        raw.choice_presets[0].rewards = refs;
        assert!(BuildProject::new(raw, limits()).is_err());
    }
    let mut raw = input();
    raw.choice_presets
        .iter_mut()
        .find(|p| p.id == id(140))
        .unwrap()
        .rewards = vec![id(31), id(30)];
    let (count, collection) = array_stats(&serde_json::to_value(&raw).unwrap());
    let exact = OwnedInputLimits {
        max_entries: count,
        max_collection_entries: collection,
        ..limits()
    };
    let project = BuildProject::new(raw.clone(), exact).unwrap();
    assert_eq!(
        compose(&project, &selection(), None, exact)
            .unwrap()
            .input()
            .character
            .rewards
            .len(),
        2
    );
    let tight = OwnedInputLimits {
        max_entries: count - 1,
        ..exact
    };
    assert!(BuildProject::new(raw, tight).is_err());
    assert!(compose(&project, &selection(), None, tight).is_err());
}
