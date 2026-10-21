//! Independent modifier orders are owned input, never record/ID sorting policy.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*, owned_inventory::*,
};
use serde_json::json;

fn limits() -> OwnedInputLimits {
    OwnedInputLimits::default()
}
fn lineage() -> BuildLineage {
    BuildLineage::from_bytes([0x74; 16])
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(lineage(), local).unwrap())
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("modifier-order", "v1").unwrap()
}
fn def<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(namespace(), name).unwrap()
}
fn item(record: u64, modifiers: &[u64]) -> ItemRecord {
    ItemRecord {
        id: id(record),
        template: def("template"),
        parameters: vec![],
        item_level: None,
        quality: None,
        modifiers: modifiers
            .iter()
            .map(|local| RolledModifier {
                id: id(*local),
                definition: def("modifier"),
                rolls: vec![],
            })
            .collect(),
        modifier_orders: ItemModifierOrders {
            contributions: modifiers.iter().map(|local| id(*local)).collect(),
            numeric_transforms: modifiers.iter().rev().map(|local| id(*local)).collect(),
        },
    }
}
fn input() -> BuildInput {
    BuildInput {
        usage_preferences: vec![],
        generated_inputs: None,
        authored_support_order: None,
        allocator: InstanceAllocatorState::from_parts(lineage(), 100),
        revision: BuildRevision::INITIAL,
        game_version: namespace(),
        character: CharacterSpec {
            class: def("class"),
            ascendancy: None,
            level: 1,
            rewards: vec![],
        },
        weapon_loadouts: vec![id(1)],
        active_weapon_loadout: id(1),
        items: vec![item(2, &[4, 3]), item(5, &[6])],
        gems: vec![],
        equipment: vec![],
        allocations: vec![],
        skills: vec![],
        supports: vec![],
        payload_links: vec![],
        choices: vec![],
    }
}
fn stock(items: Vec<ItemRecord>) -> InventorySnapshot {
    InventorySnapshot::new(
        InventoryInput {
            allocator: input().allocator,
            revision: BuildRevision::INITIAL,
            game_version: namespace(),
            items,
            copies: vec![],
            completeness: InventoryCompleteness::Complete,
        },
        limits(),
    )
    .unwrap()
}
fn empty<T>() -> DraftList<T> {
    DraftList {
        members: vec![],
        completion: DraftListCompletion::Complete,
    }
}
fn draft() -> DraftSessionInput {
    DraftSessionInput {
        allocator: input().allocator,
        revision: BuildRevision::INITIAL,
        game_version: namespace(),
        weapon_loadouts: vec![id::<WeaponLoadoutId>(1)].into(),
        items: input().items.into(),
        gems: empty(),
        rewards: empty(),
        equipment: empty(),
        allocations: empty(),
        skills: empty(),
        supports: empty(),
        payload_links: empty(),
        character_presets: empty(),
        equipment_presets: empty(),
        allocation_presets: empty(),
        skill_presets: empty(),
        choice_presets: empty(),
        scenario_presets: empty(),
        query_presets: empty(),
        saved_variants: empty(),
    }
}
fn pending(candidates: Vec<Vec<ModifierInstanceId>>) -> DraftField<Vec<ModifierInstanceId>> {
    DraftField::Pending(PendingValue {
        id: id(90),
        code: OwnedDefinitionKey::new("order-not-established").unwrap(),
        candidates,
    })
}
fn order_mut(value: &mut ItemModifierOrders, numeric: bool) -> &mut Vec<ModifierInstanceId> {
    if numeric {
        &mut value.numeric_transforms
    } else {
        &mut value.contributions
    }
}
fn draft_order_mut(
    value: &mut ItemModifierOrdersDraft,
    numeric: bool,
) -> &mut DraftField<Vec<ModifierInstanceId>> {
    if numeric {
        &mut value.numeric_transforms
    } else {
        &mut value.contributions
    }
}
fn domain(numeric: bool) -> &'static str {
    if numeric {
        "numeric_transforms"
    } else {
        "contributions"
    }
}

#[test]
fn both_semantic_orders_survive_canonicalization_and_independently_change_identity() {
    let a = BuildSpec::new(input(), limits()).unwrap();
    assert_eq!(
        a.input().items[0]
            .modifiers
            .iter()
            .map(|m| m.id)
            .collect::<Vec<_>>(),
        vec![id(3), id(4)]
    );
    assert_eq!(
        a.input().items[0].modifier_orders.contributions,
        vec![id(4), id(3)]
    );
    assert_eq!(
        a.input().items[0].modifier_orders.numeric_transforms,
        vec![id(3), id(4)]
    );
    let mut reordered_records = input();
    reordered_records.items.reverse();
    for item in &mut reordered_records.items {
        item.modifiers.reverse();
    }
    let same = BuildSpec::new(reordered_records, limits()).unwrap();
    assert_eq!(a, same);
    assert_eq!(
        build_content_binding(&a, limits()).unwrap(),
        build_content_binding(&same, limits()).unwrap()
    );
    let stock_a = stock(a.input().items.clone());
    for numeric in [false, true] {
        let mut changed = input();
        order_mut(&mut changed.items[0].modifier_orders, numeric).reverse();
        let b = BuildSpec::new(changed, limits()).unwrap();
        assert_ne!(
            build_content_binding(&a, limits()).unwrap(),
            build_content_binding(&b, limits()).unwrap()
        );
        let stock_b = stock(b.input().items.clone());
        assert_ne!(
            inventory_content_binding(&stock_a, limits()).unwrap(),
            inventory_content_binding(&stock_b, limits()).unwrap()
        );
        assert!(
            matches!(union_build_inventory(&a, &stock_b, limits()), Err(InventoryError::ConflictingItemRecord(value)) if value == id::<ItemRecordId>(2))
        );
    }
    assert!(union_build_inventory(&a, &stock_a, limits()).is_ok());
}

#[test]
fn complete_orders_reject_missing_duplicate_foreign_and_cross_item_members() {
    let foreign = ModifierInstanceId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x75; 16]), 3).unwrap(),
    );
    for (order, expected) in [
        (vec![], StructuralErrorKind::InvalidModifierOrder),
        (vec![id(4)], StructuralErrorKind::InvalidModifierOrder),
        (vec![id(4), id(4)], StructuralErrorKind::DuplicateAssignment),
        (vec![id(4), id(6)], StructuralErrorKind::WrongProviderOwner),
        (vec![id(4), foreign], StructuralErrorKind::ForeignLineage),
        (vec![id(4), id(101)], StructuralErrorKind::BeyondWatermark),
        (
            vec![id(4), id(9)],
            StructuralErrorKind::MissingReference {
                expected: OccurrenceKind::Modifier,
                id: id::<ModifierInstanceId>(9).instance_id(),
            },
        ),
        (
            vec![id(4), id(5)],
            StructuralErrorKind::MissingReference {
                expected: OccurrenceKind::Modifier,
                id: id::<ModifierInstanceId>(5).instance_id(),
            },
        ),
    ] {
        for numeric in [false, true] {
            let mut raw = input();
            *order_mut(&mut raw.items[0].modifier_orders, numeric) = order.clone();
            let error = BuildSpec::new(raw, limits()).unwrap_err();
            assert_eq!(error.kind, expected);
            assert!(
                error
                    .path
                    .contains(&format!("modifier_orders.{}", domain(numeric)))
            );
        }
    }
}

#[test]
fn build_inventory_and_raw_item_wire_require_order_and_reject_previous_envelope() {
    let raw = input();
    for document in [
        OwnedDocument::Build(Box::new(BuildSpec::new(raw.clone(), limits()).unwrap())),
        OwnedDocument::Inventory(Box::new(stock(raw.items.clone()))),
    ] {
        let encoded = encode_owned(&document, limits()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(value["schema_version"], 5);
        assert_eq!(decode_owned(&encoded, limits()).unwrap(), document);
        let mut old = value.clone();
        old["schema_version"] = json!(3);
        assert!(decode_owned(&serde_json::to_vec(&old).unwrap(), limits()).is_err());
        let mut missing = value.clone();
        missing["document"]["value"]["items"][0]
            .as_object_mut()
            .unwrap()
            .remove("modifier_orders");
        assert!(decode_owned(&serde_json::to_vec(&missing).unwrap(), limits()).is_err());
        for field in ["contributions", "numeric_transforms"] {
            let mut missing = value.clone();
            missing["document"]["value"]["items"][0]["modifier_orders"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(decode_owned(&serde_json::to_vec(&missing).unwrap(), limits()).is_err());
        }
    }
    let item = serde_json::to_value(&raw.items[0]).unwrap();
    for key in ["contributions", "numeric_transforms"] {
        let field = format!("\"{key}\":{}", item["modifier_orders"][key]);
        let duplicate = item
            .to_string()
            .replacen(&field, &format!("{field},{field}"), 1);
        assert!(serde_json::from_str::<ItemRecord>(&duplicate).is_err());
        let mut null = item.clone();
        null["modifier_orders"][key] = serde_json::Value::Null;
        assert!(serde_json::from_value::<ItemRecord>(null).is_err());
    }
    let mut old = item;
    let previous = old
        .as_object_mut()
        .unwrap()
        .remove("modifier_orders")
        .unwrap();
    old["modifier_order"] = previous["contributions"].clone();
    assert!(serde_json::from_value::<ItemRecord>(old).is_err());
}

#[test]
fn pending_order_candidates_are_checked_locally_even_with_open_modifier_membership() {
    for numeric in [false, true] {
        let mut raw = draft();
        raw.items.members[0].modifiers.completion = DraftListCompletion::Pending {
            id: id(91),
            code: OwnedDefinitionKey::new("members-not-established").unwrap(),
        };
        *draft_order_mut(&mut raw.items.members[0].modifier_orders, numeric) =
            pending(vec![vec![id(4), id(3)], vec![id(3), id(4)]]);
        let session = DraftSession::new(raw.clone(), DraftLimits::default()).unwrap();
        assert_eq!(
            session
                .validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .len(),
            2
        );
        assert!(session.input().items.members[0].to_resolved().is_none());
        for order in [
            vec![id(4)],
            vec![id(4), id(4)],
            vec![id(4), id(6)],
            vec![id(4), id(9)],
        ] {
            let mut candidate = raw.clone();
            *draft_order_mut(&mut candidate.items.members[0].modifier_orders, numeric) =
                pending(vec![order.clone()]);
            assert!(DraftSession::new(candidate, DraftLimits::default()).is_err());
            let mut known = raw.clone();
            *draft_order_mut(&mut known.items.members[0].modifier_orders, numeric) =
                DraftField::Known { value: order };
            assert!(DraftSession::new(known, DraftLimits::default()).is_err());
        }
    }
}

#[test]
fn resolving_one_order_does_not_resolve_the_other() {
    let mut raw = draft();
    raw.items.members[0].modifier_orders.contributions = pending(vec![vec![id(4), id(3)]]);
    raw.items.members[0].modifier_orders.numeric_transforms = DraftField::Pending(PendingValue {
        id: id(92),
        code: OwnedDefinitionKey::new("transform-order-not-established").unwrap(),
        candidates: vec![vec![id(3), id(4)]],
    });
    let session = DraftSession::new(raw.clone(), DraftLimits::default()).unwrap();
    assert_eq!(
        session
            .validate_limits(DraftLimits::default())
            .unwrap()
            .issues
            .len(),
        2
    );
    for numeric in [false, true] {
        let mut partial = raw.clone();
        *draft_order_mut(&mut partial.items.members[0].modifier_orders, numeric) =
            DraftField::Known {
                value: if numeric {
                    vec![id(3), id(4)]
                } else {
                    vec![id(4), id(3)]
                },
            };
        let session = DraftSession::new(partial, DraftLimits::default()).unwrap();
        let issues = session
            .validate_limits(DraftLimits::default())
            .unwrap()
            .issues;
        assert_eq!(issues.len(), 1);
        assert!(
            issues[0]
                .path
                .ends_with(&format!(".modifier_orders.{}", domain(!numeric)))
        );
        assert!(session.input().items.members[0].to_resolved().is_none());
    }
    raw.items.members[0].modifier_orders = input().items[0].modifier_orders.clone().into();
    let resolved = DraftSession::new(raw, DraftLimits::default()).unwrap();
    assert_eq!(
        resolved.input().items.members[0]
            .to_resolved()
            .unwrap()
            .modifier_orders,
        input().items[0].modifier_orders
    );
}

#[test]
fn draft_wire_order_presence_identity_and_nested_sequence_budgets_are_explicit() {
    let a = DraftSession::new(draft(), DraftLimits::default()).unwrap();
    for numeric in [false, true] {
        let mut raw = draft();
        let DraftField::Known { value } =
            draft_order_mut(&mut raw.items.members[0].modifier_orders, numeric)
        else {
            panic!()
        };
        value.reverse();
        let b = DraftSession::new(raw, DraftLimits::default()).unwrap();
        assert_ne!(
            a.digest(limits().max_wire_bytes).unwrap(),
            b.digest(limits().max_wire_bytes).unwrap()
        );
    }
    let bytes = encode_draft(&a, DraftLimits::default()).unwrap();
    assert_eq!(decode_draft(&bytes, DraftLimits::default()).unwrap(), a);
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["schema_version"], 5);
    value["schema_version"] = json!(3);
    assert!(decode_draft(&serde_json::to_vec(&value).unwrap(), DraftLimits::default()).is_err());
    value["schema_version"] = json!(5);
    let mut missing = value.clone();
    missing["draft"]["items"]["members"][0]
        .as_object_mut()
        .unwrap()
        .remove("modifier_orders");
    assert!(
        decode_draft(
            &serde_json::to_vec(&missing).unwrap(),
            DraftLimits::default()
        )
        .is_err()
    );
    for key in ["contributions", "numeric_transforms"] {
        let mut missing = value.clone();
        missing["draft"]["items"]["members"][0]["modifier_orders"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(
            decode_draft(
                &serde_json::to_vec(&missing).unwrap(),
                DraftLimits::default()
            )
            .is_err()
        );
    }
    for numeric in [false, true] {
        let mut raw = draft();
        *draft_order_mut(&mut raw.items.members[0].modifier_orders, numeric) =
            pending(vec![vec![id(4), id(3)]; 3]);
        let tight_candidates = DraftLimits {
            max_candidates_per_field: 2,
            ..DraftLimits::default()
        };
        assert_eq!(
            DraftSession::new(raw, tight_candidates).unwrap_err().kind,
            StructuralErrorKind::LimitExceeded
        );
        let mut raw = draft();
        *draft_order_mut(&mut raw.items.members[0].modifier_orders, numeric) =
            pending(vec![vec![id(4), id(3), id(4)]]);
        let tight_sequence = DraftLimits {
            input: OwnedInputLimits {
                max_collection_entries: 2,
                ..limits()
            },
            ..DraftLimits::default()
        };
        assert_eq!(
            DraftSession::new(raw, tight_sequence).unwrap_err().kind,
            StructuralErrorKind::LimitExceeded
        );
    }
}
