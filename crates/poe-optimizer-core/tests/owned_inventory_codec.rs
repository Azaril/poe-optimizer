use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_inventory::*,
};
use serde_json::json;
fn input() -> InventoryInput {
    let lineage = BuildLineage::from_bytes([0x59; 16]);
    let namespace = GameVersionNamespace::new("codec-stock", "v1").unwrap();
    let raw = |n| InstanceId::from_parts(lineage, n).unwrap();
    InventoryInput {
        allocator: InstanceAllocatorState::from_parts(lineage, 10),
        revision: BuildRevision::from_u64(3),
        game_version: namespace.clone(),
        items: vec![ItemRecord {
            id: ItemRecordId::from_instance_id(raw(1)),
            template: ItemTemplateDefId::parse(namespace, "authored-template").unwrap(),
            parameters: vec![],
            item_level: 4,
            quality: None,
            modifiers: vec![],
        }],
        copies: vec![3, 2]
            .into_iter()
            .map(|n| InventoryItem {
                id: InventoryItemId::from_instance_id(raw(n)),
                item: ItemRecordId::from_instance_id(raw(1)),
            })
            .collect(),
        completeness: InventoryCompleteness::Partial,
    }
}
fn wire(input: InventoryInput) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":1,"document":{"kind":"inventory","value":input}}))
        .unwrap()
}
#[test]
fn inventory_wire_roundtrip_preserves_physical_copies_and_canonical_snapshot_binding() {
    let limits = OwnedInputLimits::default();
    let expected = InventorySnapshot::new(input(), limits).unwrap();
    let actual = decode_owned(&wire(input()), limits).unwrap();
    assert_eq!(actual, OwnedDocument::Inventory(Box::new(expected.clone())));
    let canonical = encode_owned(&actual, limits).unwrap();
    let OwnedDocument::Inventory(decoded) = decode_owned(&canonical, limits).unwrap() else {
        panic!("wrong kind")
    };
    assert_eq!(decoded.input().copies.len(), 2);
    assert_eq!(decoded.input().copies[0].id.instance_id().local(), 2);
    assert_eq!(decoded.input().completeness, InventoryCompleteness::Partial);
    assert_eq!(
        inventory_content_binding(&decoded, limits).unwrap(),
        inventory_content_binding(&expected, limits).unwrap()
    );
}
#[test]
fn bounded_inventory_codec_rechecks_input_and_output_limits() {
    let limits = OwnedInputLimits::default();
    let document = decode_owned(&wire(input()), limits).unwrap();
    let canonical = encode_owned(&document, limits).unwrap();
    let exact = OwnedInputLimits {
        max_wire_bytes: canonical.len(),
        ..limits
    };
    assert!(decode_owned(&canonical, exact).is_ok());
    assert!(encode_owned(&document, exact).is_ok());
    let short = OwnedInputLimits {
        max_wire_bytes: canonical.len() - 1,
        ..limits
    };
    assert!(matches!(
        decode_owned(&canonical, short),
        Err(CodecError::TooLarge { .. })
    ));
    assert!(matches!(
        encode_owned(&document, short),
        Err(CodecError::TooLarge { .. })
    ));
    let tight = OwnedInputLimits {
        max_collection_entries: 1,
        ..limits
    };
    assert!(matches!(
        decode_owned(&canonical, tight),
        Err(CodecError::Inventory(_))
    ));
    assert!(matches!(
        encode_owned(&document, tight),
        Err(CodecError::Inventory(_))
    ));
}
#[test]
fn malformed_stock_cannot_enter_the_codec_as_a_validated_document() {
    let mut duplicate = input();
    duplicate.copies.push(duplicate.copies[0].clone());
    assert!(matches!(
        decode_owned(&wire(duplicate), OwnedInputLimits::default()),
        Err(CodecError::Inventory(_))
    ));
    let mut value = json!({"schema_version":1,"document":{"kind":"inventory","value":input()}});
    value["document"]["value"]["pob_stock"] = json!({});
    assert!(matches!(
        decode_owned(
            &serde_json::to_vec(&value).unwrap(),
            OwnedInputLimits::default()
        ),
        Err(CodecError::Json(_))
    ));
}
