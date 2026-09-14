use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_inventory::*,
};
use serde_json::json;

fn limits() -> OwnedInputLimits {
    OwnedInputLimits::default()
}
fn lineage() -> BuildLineage {
    BuildLineage::from_bytes([0x33; 16])
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("stock-test", "v1").unwrap()
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(lineage(), local).unwrap())
}
fn definition<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn parameter(owner: SlotOwnerDefId, key: &str, value: i64) -> ParameterAssignment {
    ParameterAssignment {
        slot: DeclaredSlot {
            declaration: owner,
            slot: definition(key),
        },
        value: ParameterValue::Integer(BoundedInteger::new(value).unwrap()),
    }
}
fn item(record: u64, first_modifier: u64) -> ItemRecord {
    let owner = SlotOwnerDefId::ItemTemplate(definition("same-template"));
    let modifier_owner = SlotOwnerDefId::Modifier(definition("same-modifier"));
    ItemRecord {
        id: id(record),
        template: definition("same-template"),
        item_level: 60,
        quality: None,
        parameters: vec![
            parameter(owner.clone(), "z-property", 2),
            parameter(owner, "a-property", 1),
        ],
        modifiers: vec![
            RolledModifier {
                id: id(first_modifier + 1),
                definition: definition("same-modifier"),
                rolls: vec![
                    parameter(modifier_owner.clone(), "z-roll", 5),
                    parameter(modifier_owner.clone(), "a-roll", 4),
                ],
            },
            RolledModifier {
                id: id(first_modifier),
                definition: definition("same-modifier"),
                rolls: vec![parameter(modifier_owner, "magnitude", 3)],
            },
        ],
    }
}
fn build_input() -> BuildInput {
    BuildInput {
        allocator: InstanceAllocatorState::from_parts(lineage(), 100),
        revision: BuildRevision::from_u64(1),
        game_version: namespace(),
        character: CharacterSpec {
            class: definition("class"),
            ascendancy: None,
            level: 60,
            rewards: vec![],
        },
        weapon_loadouts: vec![id(2), id(1)],
        active_weapon_loadout: id(1),
        items: vec![item(3, 4)],
        gems: vec![GemInstance {
            id: id(8),
            definition: definition("gem"),
            parameters: vec![],
            level: 1,
            quality: None,
        }],
        equipment: vec![
            EquipmentUse {
                id: id(7),
                item: id(3),
                destination: EquipmentDestination::CharacterSlot(definition("slot-b")),
                scope: LoadoutScope::Shared,
            },
            EquipmentUse {
                id: id(6),
                item: id(3),
                destination: EquipmentDestination::CharacterSlot(definition("slot-a")),
                scope: LoadoutScope::Shared,
            },
        ],
        allocations: vec![],
        skills: vec![SkillUse {
            id: id(9),
            source: AuthoredSkillSource::Gem(id(8)),
            enabled: true,
            scope: LoadoutScope::Shared,
        }],
        supports: vec![],
        payload_links: vec![],
        choices: vec![],
    }
}
fn build() -> BuildSpec {
    BuildSpec::new(build_input(), limits()).unwrap()
}
fn inventory_input() -> InventoryInput {
    InventoryInput {
        allocator: InstanceAllocatorState::from_parts(lineage(), 100),
        revision: BuildRevision::from_u64(2),
        game_version: namespace(),
        items: vec![item(30, 31), item(3, 4)],
        copies: vec![
            InventoryItem {
                id: id(22),
                item: id(3),
            },
            InventoryItem {
                id: id(21),
                item: id(30),
            },
            InventoryItem {
                id: id(20),
                item: id(3),
            },
        ],
        completeness: InventoryCompleteness::Complete,
    }
}
fn inventory() -> InventorySnapshot {
    InventorySnapshot::new(inventory_input(), limits()).unwrap()
}
fn assignments(
    build: &BuildSpec,
    stock: &InventorySnapshot,
    claims: &[(u64, AvailabilityClaim)],
) -> AvailabilityAssignments {
    AvailabilityAssignments {
        build: build_content_binding(build, limits()).unwrap(),
        inventory: inventory_content_binding(stock, limits()).unwrap(),
        uses: claims
            .iter()
            .map(|(usage, claim)| UseAvailability {
                equipment_use: id(*usage),
                claim: *claim,
            })
            .collect(),
    }
}
fn known(build: &BuildSpec, stock: &InventorySnapshot) -> AvailabilityAssignments {
    assignments(
        build,
        stock,
        &[
            (7, AvailabilityClaim::KnownCopy(id(22))),
            (6, AvailabilityClaim::KnownCopy(id(20))),
        ],
    )
}
fn structural(error: InventoryError, expected: &str) {
    let InventoryError::Structure(error) = error else {
        panic!("expected structural error, received {error}")
    };
    assert!(
        error.to_string().to_lowercase().contains(expected),
        "{error}"
    );
}

#[test]
fn inventory_is_self_contained_and_canonical_without_a_build_or_game_package() {
    let original = inventory_input();
    let mut reordered = original.clone();
    reordered.items.reverse();
    reordered.copies.reverse();
    for item in &mut reordered.items {
        item.parameters.reverse();
        item.modifiers.reverse();
        for modifier in &mut item.modifiers {
            modifier.rolls.reverse();
        }
    }
    let first = InventorySnapshot::new(original, limits()).unwrap();
    let second = InventorySnapshot::new(reordered, limits()).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    assert_eq!(
        first
            .input()
            .copies
            .iter()
            .map(|copy| copy.id)
            .collect::<Vec<_>>(),
        vec![id::<InventoryItemId>(20), id(21), id(22)]
    );
    assert_eq!(
        first.copy(id(20)).unwrap().item,
        first.copy(id(22)).unwrap().item
    );
    assert_ne!(
        first.copy(id(20)).unwrap().id,
        first.copy(id(22)).unwrap().id
    );
    let decoded: InventoryInput =
        serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
    assert_eq!(InventorySnapshot::new(decoded, limits()).unwrap(), first);
    assert_eq!(first.input().allocator.last_issued(), 100);
}

#[test]
fn stock_occurrences_reject_duplicates_cross_domain_aliases_and_missing_records() {
    let mut duplicate = inventory_input();
    duplicate.copies.push(duplicate.copies[0].clone());
    structural(
        InventorySnapshot::new(duplicate, limits()).unwrap_err(),
        "duplicate",
    );
    let mut duplicate = inventory_input();
    duplicate.items.push(duplicate.items[0].clone());
    structural(
        InventorySnapshot::new(duplicate, limits()).unwrap_err(),
        "duplicate",
    );
    let mut collision = inventory_input();
    collision.copies[0].id = id(3);
    structural(
        InventorySnapshot::new(collision, limits()).unwrap_err(),
        "duplicate",
    );
    let mut missing = inventory_input();
    missing.copies[0].item = id(29);
    structural(
        InventorySnapshot::new(missing, limits()).unwrap_err(),
        "missingreference",
    );
    let mut watermark = inventory_input();
    watermark.allocator = InstanceAllocatorState::from_parts(lineage(), 21);
    structural(
        InventorySnapshot::new(watermark, limits()).unwrap_err(),
        "watermark",
    );
    let mut foreign = inventory_input();
    foreign.copies[0].id = InventoryItemId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x77; 16]), 22).unwrap(),
    );
    structural(
        InventorySnapshot::new(foreign, limits()).unwrap_err(),
        "lineage",
    );
    let mut foreign_reference = inventory_input();
    foreign_reference.copies[0].item = ItemRecordId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x77; 16]), 3).unwrap(),
    );
    structural(
        InventorySnapshot::new(foreign_reference, limits()).unwrap_err(),
        "lineage",
    );
}

#[test]
fn inventory_reuses_intrinsic_modifier_and_unit_validation() {
    let mut wrong = inventory_input();
    wrong.items[0].parameters[0].slot.declaration =
        SlotOwnerDefId::ItemTemplate(definition("wrong-template"));
    structural(
        InventorySnapshot::new(wrong, limits()).unwrap_err(),
        "wrongdeclaration",
    );
    let mut wrong = inventory_input();
    wrong.items[0].modifiers[0].rolls[0].slot.declaration =
        SlotOwnerDefId::Modifier(definition("wrong-modifier"));
    structural(
        InventorySnapshot::new(wrong, limits()).unwrap_err(),
        "wrongdeclaration",
    );
    let mut wrong = inventory_input();
    wrong.items[0].parameters[0].value = ParameterValue::Quantity(
        FiniteQuantity::new(
            1.0,
            UnitDefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "unit").unwrap(),
        )
        .unwrap(),
    );
    structural(
        InventorySnapshot::new(wrong, limits()).unwrap_err(),
        "namespace",
    );
}

#[test]
fn union_joins_identical_records_without_merging_distinct_ids_or_revisions() {
    let build = build();
    let stock = inventory();
    assert_ne!(build.input().revision, stock.input().revision);
    let union = union_build_inventory(&build, &stock, limits()).unwrap();
    assert_eq!(union.items().len(), 2);
    assert_eq!(union.item(id(3)).unwrap(), &build.input().items[0]);
    assert_eq!(
        union.item(id(3)).unwrap().template,
        union.item(id(30)).unwrap().template
    );
    assert_ne!(
        union.item(id(3)).unwrap().id,
        union.item(id(30)).unwrap().id
    );
    assert_eq!(union.game_version(), &namespace());
    assert_eq!(union.allocator().last_issued(), 100);
    assert_eq!(build.input().items.len(), 1);
}

#[test]
fn conflicting_shared_content_is_rejected_before_any_selection() {
    let build = build();
    for mutation in 0..4 {
        let mut input = inventory_input();
        let shared = input
            .items
            .iter_mut()
            .find(|item| item.id == id::<ItemRecordId>(3))
            .unwrap();
        match mutation {
            0 => shared.item_level += 1,
            1 => {
                shared.parameters[0].value =
                    ParameterValue::Integer(BoundedInteger::new(99).unwrap())
            }
            2 => {
                shared.modifiers[0].rolls[0].value =
                    ParameterValue::Integer(BoundedInteger::new(99).unwrap())
            }
            3 => shared.modifiers[0].id = id(40),
            _ => unreachable!(),
        }
        let stock = InventorySnapshot::new(input, limits()).unwrap();
        assert!(
            matches!(union_build_inventory(&build, &stock, limits()), Err(InventoryError::ConflictingItemRecord(record)) if record == id::<ItemRecordId>(3))
        );
    }
    let mut input = build_input();
    input.items.push(item(30, 31));
    let with_unused = BuildSpec::new(input, limits()).unwrap();
    let mut stock = inventory_input();
    stock.items[0].item_level += 1;
    assert!(
        matches!(union_build_inventory(&with_unused, &InventorySnapshot::new(stock, limits()).unwrap(), limits()), Err(InventoryError::ConflictingItemRecord(record)) if record == id::<ItemRecordId>(30))
    );
}

#[test]
fn union_checks_global_domains_and_does_not_silently_rebase_inventory() {
    let build = build();
    let mut collision = inventory_input();
    collision.copies[0].id = id(8);
    structural(
        union_build_inventory(
            &build,
            &InventorySnapshot::new(collision, limits()).unwrap(),
            limits(),
        )
        .unwrap_err(),
        "duplicate",
    );
    let mut collision = inventory_input();
    collision.items = vec![item(30, 4)];
    collision.copies = vec![InventoryItem {
        id: id(20),
        item: id(30),
    }];
    structural(
        union_build_inventory(
            &build,
            &InventorySnapshot::new(collision, limits()).unwrap(),
            limits(),
        )
        .unwrap_err(),
        "duplicate",
    );
    let empty = InventoryInput {
        allocator: InstanceAllocatorState::from_parts(lineage(), 100),
        revision: BuildRevision::INITIAL,
        game_version: namespace(),
        items: vec![],
        copies: vec![],
        completeness: InventoryCompleteness::Partial,
    };
    let mut foreign = empty.clone();
    foreign.game_version = GameVersionNamespace::new("other", "v1").unwrap();
    assert!(matches!(
        union_build_inventory(
            &build,
            &InventorySnapshot::new(foreign, limits()).unwrap(),
            limits()
        ),
        Err(InventoryError::ForeignNamespace)
    ));
    let mut foreign = empty;
    foreign.allocator =
        InstanceAllocatorState::from_parts(BuildLineage::from_bytes([0x77; 16]), 100);
    assert!(matches!(
        union_build_inventory(
            &build,
            &InventorySnapshot::new(foreign, limits()).unwrap(),
            limits()
        ),
        Err(InventoryError::ForeignLineage)
    ));
}

#[test]
fn an_unequipped_inventory_record_can_supply_a_new_self_contained_build_use() {
    let seed = build();
    let stock = inventory();
    let union = union_build_inventory(&seed, &stock, limits()).unwrap();
    assert!(
        seed.input()
            .items
            .iter()
            .all(|item| item.id != id::<ItemRecordId>(30))
    );
    let mut replacement = seed.clone().into_input();
    replacement.revision = replacement.revision.checked_next().unwrap();
    replacement.items = vec![union.item(id(30)).unwrap().clone()];
    replacement.equipment = vec![EquipmentUse {
        id: id(23),
        item: id(30),
        destination: EquipmentDestination::CharacterSlot(definition("slot-a")),
        scope: LoadoutScope::Shared,
    }];
    let replacement = BuildSpec::new(replacement, limits()).unwrap();
    let claims = assignments(
        &replacement,
        &stock,
        &[(23, AvailabilityClaim::KnownCopy(id(21)))],
    );
    bind_availability(&replacement, &stock, claims, limits()).unwrap();
    assert_eq!(replacement.input().items[0], *stock.item(id(30)).unwrap());
    assert_eq!(seed.input().equipment.len(), 2);
}

#[test]
fn snapshot_bindings_change_with_content_even_when_revision_is_unchanged() {
    let build = build();
    let stock = inventory();
    let claims = known(&build, &stock);
    let mut changed = build.clone().into_input();
    changed.character.level += 1;
    let changed = BuildSpec::new(changed, limits()).unwrap();
    assert_eq!(changed.input().revision, build.input().revision);
    assert_ne!(
        build_content_binding(&changed, limits()).unwrap(),
        claims.build
    );
    assert!(matches!(
        bind_availability(&changed, &stock, claims.clone(), limits()),
        Err(InventoryError::BuildBindingMismatch)
    ));
    let mut changed = stock.clone().into_input();
    changed.completeness = InventoryCompleteness::Partial;
    let changed = InventorySnapshot::new(changed, limits()).unwrap();
    assert_eq!(changed.input().revision, stock.input().revision);
    assert!(matches!(
        bind_availability(&build, &changed, claims.clone(), limits()),
        Err(InventoryError::InventoryBindingMismatch)
    ));
    let mut changed = stock.clone().into_input();
    changed.copies.push(InventoryItem {
        id: id(25),
        item: id(30),
    });
    let changed = InventorySnapshot::new(changed, limits()).unwrap();
    assert_ne!(
        inventory_content_binding(&changed, limits()).unwrap(),
        claims.inventory
    );
    assert_eq!(
        build_content_binding(&build, limits()).unwrap(),
        claims.build
    );
}

#[test]
fn binding_includes_watermarks_revisions_and_recomputes_deserialized_digest_claims() {
    let build = build();
    let stock = inventory();
    let claims = known(&build, &stock);
    let mut input = build.clone().into_input();
    input.allocator = InstanceAllocatorState::from_parts(lineage(), 101);
    assert_ne!(
        build_content_binding(&BuildSpec::new(input, limits()).unwrap(), limits()).unwrap(),
        claims.build
    );
    let mut input = stock.clone().into_input();
    input.revision = input.revision.checked_next().unwrap();
    assert_ne!(
        inventory_content_binding(&InventorySnapshot::new(input, limits()).unwrap(), limits())
            .unwrap(),
        claims.inventory
    );
    let mut input = stock.clone().into_input();
    input.allocator = InstanceAllocatorState::from_parts(lineage(), 101);
    assert_ne!(
        inventory_content_binding(&InventorySnapshot::new(input, limits()).unwrap(), limits())
            .unwrap(),
        claims.inventory
    );
    assert_eq!(claims.build.lineage(), lineage());
    assert_eq!(claims.build.revision(), build.input().revision);
    assert_eq!(claims.inventory.lineage(), lineage());
    assert_eq!(claims.inventory.revision(), stock.input().revision);
    let mut wire = serde_json::to_value(&claims).unwrap();
    wire["build"]["digest"] = json!("0".repeat(64));
    let forged: AvailabilityAssignments = serde_json::from_value(wire).unwrap();
    assert!(matches!(
        bind_availability(&build, &stock, forged, limits()),
        Err(InventoryError::BuildBindingMismatch)
    ));
}

#[test]
fn claims_cover_every_exact_use_once_and_known_copies_supply_the_exact_record() {
    let build = build();
    let stock = inventory();
    let claims = known(&build, &stock);
    let bound = bind_availability(&build, &stock, claims.clone(), limits()).unwrap();
    assert_eq!(
        bound
            .assignments()
            .uses
            .iter()
            .map(|usage| usage.equipment_use)
            .collect::<Vec<_>>(),
        vec![id::<ItemSlotUseId>(6), id(7)]
    );
    let mut duplicate = claims.clone();
    duplicate.uses.push(duplicate.uses[0].clone());
    assert!(matches!(
        bind_availability(&build, &stock, duplicate, limits()),
        Err(InventoryError::DuplicateEquipmentUse(_))
    ));
    let mut missing = claims.clone();
    missing.uses.pop();
    assert!(matches!(
        bind_availability(&build, &stock, missing, limits()),
        Err(InventoryError::MissingEquipmentUse(_))
    ));
    let mut wrong = claims.clone();
    wrong.uses[0].equipment_use = id(29);
    assert!(matches!(
        bind_availability(&build, &stock, wrong, limits()),
        Err(InventoryError::UnknownEquipmentUse(_))
    ));
    let mut wrong = claims.clone();
    wrong.uses[0].claim = AvailabilityClaim::KnownCopy(id(29));
    assert!(matches!(
        bind_availability(&build, &stock, wrong, limits()),
        Err(InventoryError::UnknownCopy(_))
    ));
    let mut wrong = claims.clone();
    wrong.uses[0].claim = AvailabilityClaim::KnownCopy(id(3));
    assert!(matches!(
        bind_availability(&build, &stock, wrong, limits()),
        Err(InventoryError::UnknownCopy(_))
    ));
    let mut wrong = claims;
    wrong.uses[0].claim = AvailabilityClaim::KnownCopy(id(21));
    assert!(matches!(
        bind_availability(&build, &stock, wrong, limits()),
        Err(InventoryError::CopyItemMismatch { .. })
    ));
}

#[test]
fn replacement_and_hypothetical_roll_edits_do_not_inherit_stale_stock_claims() {
    let build = build();
    let stock = inventory();
    let claims = known(&build, &stock);
    let mut replacement = build.clone().into_input();
    replacement.equipment[0].id = id(24);
    let replacement = BuildSpec::new(replacement, limits()).unwrap();
    let mut stale_uses = claims.clone();
    stale_uses.build = build_content_binding(&replacement, limits()).unwrap();
    assert!(matches!(
        bind_availability(&replacement, &stock, stale_uses, limits()),
        Err(InventoryError::UnknownEquipmentUse(_))
    ));
    let mut edited = build.clone().into_input();
    edited.items[0].item_level += 1;
    let edited = BuildSpec::new(edited, limits()).unwrap();
    assert!(matches!(
        union_build_inventory(&edited, &stock, limits()),
        Err(InventoryError::ConflictingItemRecord(_))
    ));
    let mut coherent = stock.clone().into_input();
    coherent
        .items
        .iter_mut()
        .find(|item| item.id == id::<ItemRecordId>(3))
        .unwrap()
        .item_level += 1;
    let coherent = InventorySnapshot::new(coherent, limits()).unwrap();
    assert!(matches!(
        bind_availability(&edited, &coherent, claims, limits()),
        Err(InventoryError::BuildBindingMismatch)
    ));
    bind_availability(&edited, &coherent, known(&edited, &coherent), limits()).unwrap();
}

#[test]
fn unspecified_availability_preserves_unknown_stock_without_inventing_copies() {
    let build = build();
    for completeness in [
        InventoryCompleteness::Complete,
        InventoryCompleteness::Partial,
    ] {
        let empty = InventorySnapshot::new(
            InventoryInput {
                allocator: InstanceAllocatorState::from_parts(lineage(), 100),
                revision: BuildRevision::INITIAL,
                game_version: namespace(),
                items: vec![],
                copies: vec![],
                completeness,
            },
            limits(),
        )
        .unwrap();
        let unspecified = assignments(
            &build,
            &empty,
            &[
                (6, AvailabilityClaim::Unspecified),
                (7, AvailabilityClaim::Unspecified),
            ],
        );
        let bound = bind_availability(&build, &empty, unspecified, limits()).unwrap();
        assert!(
            bound
                .authored_copy_overlaps(&build, limits())
                .unwrap()
                .is_empty()
        );
        assert!(empty.input().copies.is_empty());
        let known = assignments(
            &build,
            &empty,
            &[
                (6, AvailabilityClaim::KnownCopy(id(20))),
                (7, AvailabilityClaim::Unspecified),
            ],
        );
        assert!(matches!(
            bind_availability(&build, &empty, known, limits()),
            Err(InventoryError::UnknownCopy(_))
        ));
    }
}

#[test]
fn shared_physical_copy_conflicts_are_separate_from_structural_binding() {
    let build = build();
    let stock = inventory();
    let reused = assignments(
        &build,
        &stock,
        &[
            (6, AvailabilityClaim::KnownCopy(id(20))),
            (7, AvailabilityClaim::KnownCopy(id(20))),
        ],
    );
    let bound = bind_availability(&build, &stock, reused, limits()).unwrap();
    let overlaps = bound.authored_copy_overlaps(&build, limits()).unwrap();
    assert_eq!(
        overlaps,
        vec![KnownCopyOverlap {
            copy: id(20),
            equipment_uses: [id(6), id(7)],
            loadouts: vec![id(1), id(2)]
        }]
    );
    let distinct = bind_availability(&build, &stock, known(&build, &stock), limits()).unwrap();
    assert!(
        distinct
            .authored_copy_overlaps(&build, limits())
            .unwrap()
            .is_empty()
    );
    let mut separate = build.clone().into_input();
    for (index, usage) in separate.equipment.iter_mut().enumerate() {
        usage.scope = LoadoutScope::Selected {
            loadouts: vec![id(index as u64 + 1)],
        };
    }
    let separate = BuildSpec::new(separate, limits()).unwrap();
    let claims = assignments(
        &separate,
        &stock,
        &[
            (6, AvailabilityClaim::KnownCopy(id(20))),
            (7, AvailabilityClaim::KnownCopy(id(20))),
        ],
    );
    assert!(
        bind_availability(&separate, &stock, claims, limits())
            .unwrap()
            .authored_copy_overlaps(&separate, limits())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        bound.authored_copy_overlaps(&separate, limits()),
        Err(InventoryError::BuildBindingMismatch)
    ));
}

#[test]
fn limits_reject_oversized_tables_and_bound_overlap_work_independently() {
    let stock = inventory();
    let mut tighter = limits();
    tighter.max_collection_entries = 2;
    structural(stock.validate_limits(tighter).unwrap_err(), "limit");
    let mut tighter = limits();
    tighter.max_wire_bytes = 1;
    assert!(matches!(
        inventory_content_binding(&stock, tighter),
        Err(InventoryError::Digest(_))
    ));
    let mut input = build_input();
    input.equipment = (60..80)
        .map(|local| EquipmentUse {
            id: id(local),
            item: id(3),
            destination: EquipmentDestination::CharacterSlot(definition(&format!("slot-{local}"))),
            scope: LoadoutScope::Shared,
        })
        .collect();
    let crowded = BuildSpec::new(input, limits()).unwrap();
    let uses: Vec<_> = (60..80)
        .map(|local| (local, AvailabilityClaim::KnownCopy(id(20))))
        .collect();
    let bound = bind_availability(
        &crowded,
        &stock,
        assignments(&crowded, &stock, &uses),
        limits(),
    )
    .unwrap();
    let mut bounded = limits();
    bounded.max_entries = 60;
    assert!(matches!(
        bound.authored_copy_overlaps(&crowded, bounded),
        Err(InventoryError::OverlapAnalysisLimit)
    ));
    assert_eq!(bound.assignments().uses.len(), 20);
}

#[test]
fn raw_inventory_and_claim_wires_reject_unknown_fields_without_conveying_authority() {
    let mut wire = serde_json::to_value(inventory_input()).unwrap();
    wire["unknown_stock_state"] = json!(true);
    assert!(serde_json::from_value::<InventoryInput>(wire).is_err());
    let mut wire = serde_json::to_value(inventory_input()).unwrap();
    wire["copies"][0]["available"] = json!(true);
    assert!(serde_json::from_value::<InventoryInput>(wire).is_err());
    let mut wire = serde_json::to_value(known(&build(), &inventory())).unwrap();
    wire["uses"][0]["claim"]["derived"] = json!(true);
    assert!(serde_json::from_value::<AvailabilityAssignments>(wire).is_err());
    let stock = inventory();
    let mut raw: InventoryInput =
        serde_json::from_slice(&serde_json::to_vec(&stock).unwrap()).unwrap();
    raw.copies[0].item = id(29);
    assert!(InventorySnapshot::new(raw, limits()).is_err());
}
