use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::*};
use std::collections::BTreeMap;

fn raw_table(
    fields: impl IntoIterator<Item = (&'static str, ItemMetadataValue)>,
    indexed: impl IntoIterator<Item = (i64, ItemMetadataValue)>,
) -> ItemMetadataValue {
    ItemMetadataValue::Table(ItemMetadataTable {
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        indexed: indexed.into_iter().collect(),
    })
}
#[test]
fn rune_catalog_borrows_complete_raw_shapes_without_eager_optional_validation() {
    let mut data = bundled_snapshot().unwrap().item_loading().data().clone();
    data.policy.rune_loading.rune_table = "Caller Rune Family".into();
    let sparse = raw_table(
        [
            ("bonded", ItemMetadataValue::Number(7.0)),
            ("statOrder", ItemMetadataValue::Boolean(false)),
        ],
        [
            (1, ItemMetadataValue::Boolean(false)),
            (3, ItemMetadataValue::Text("after hole".into())),
        ],
    );
    data.modifier_tables.insert(
        "Caller Rune Family".into(),
        ItemMetadataTable {
            fields: BTreeMap::from([
                ("Caller ID".into(), raw_table([("caller slot", sparse)], [])),
                ("False ID".into(), ItemMetadataValue::Boolean(false)),
                ("Wrong ID".into(), ItemMetadataValue::Number(9.0)),
            ]),
            indexed: BTreeMap::from([(7, ItemMetadataValue::Boolean(true))]),
        },
    );
    let catalog = ItemLoadingCatalog::new(data.clone()).unwrap();
    data.modifier_tables.clear();
    let runes = catalog.runes().unwrap();
    assert_eq!(runes.identities().count(), 3);
    assert_eq!(runes.table().indexed[&7], ItemMetadataValue::Boolean(true));
    assert!(matches!(
        runes.lookup("False ID"),
        Some(ItemMetadataValue::Boolean(false))
    ));
    assert!(ItemRuneRecord::new(runes.lookup("Wrong ID").unwrap()).is_none());
    assert!(runes.lookup("Missing ID").is_none());
    let value = runes.lookup("Caller ID").unwrap();
    assert!(std::ptr::eq(
        value,
        &catalog.modifier_table("Caller Rune Family").unwrap().fields["Caller ID"]
    ));
    let slot = ItemRuneRecord::new(
        ItemRuneRecord::new(value)
            .unwrap()
            .field("caller slot")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(slot.dense_prefix().count(), 1);
    assert_eq!(slot.indexed(3).unwrap().as_str(), Some("after hole"));
    assert_eq!(slot.field("bonded").unwrap().as_f64(), Some(7.0));
    assert_eq!(slot.field("statOrder").unwrap().as_bool(), Some(false));
}
#[test]
fn array_dense_prefix_and_named_fields_preserve_lua_table_semantics() {
    let value = ItemMetadataValue::Array(vec![
        ItemMetadataValue::Boolean(false),
        ItemMetadataValue::Text("line".into()),
    ]);
    let record = ItemRuneRecord::new(&value).unwrap();
    assert_eq!(record.dense_prefix().count(), 2);
    for i in [i64::MIN, -1, 0, 3, i64::MAX] {
        assert!(record.indexed(i).is_none());
    }
    assert_eq!(record.indexed(1).unwrap().as_bool(), Some(false));
    assert!(record.field("bonded").is_none());
    let empty = raw_table([], []);
    assert_eq!(
        ItemRuneRecord::new(&empty).unwrap().dense_prefix().count(),
        0
    );
}
#[test]
fn missing_rune_family_differs_from_present_empty_family() {
    let mut data = bundled_snapshot().unwrap().item_loading().data().clone();
    data.policy.rune_loading.rune_table = "Caller Family".into();
    assert!(
        ItemLoadingCatalog::new(data.clone())
            .unwrap()
            .runes()
            .is_none()
    );
    data.modifier_tables
        .insert("Caller Family".into(), ItemMetadataTable::default());
    let catalog = ItemLoadingCatalog::new(data).unwrap();
    assert_eq!(catalog.runes().unwrap().identities().count(), 0);
}
#[test]
fn grammar_remains_lazy_while_resource_and_dispatch_collisions_are_rejected() {
    let original = bundled_snapshot().unwrap().item_loading().data().clone();
    let mut data = original.clone();
    data.policy.rune_loading.numeric_pattern = "[".into();
    data.policy.rune_loading.item_socket_pattern = String::new();
    data.policy.rune_loading.stripped_marker = String::new();
    data.policy.rune_loading.order_default = -3.5;
    data.policy.rune_loading.vector_tolerance = -1.0;
    data.policy.rune_loading.effect_divisor = 0.0;
    ItemLoadingCatalog::new(data).unwrap();
    let mut data = original.clone();
    data.policy.rune_loading.rune_header = data.policy.rune_loading.socket_header.clone();
    assert!(ItemLoadingCatalog::new(data).is_err());
    let mut data = original.clone();
    data.policy.rune_loading.rune_header = data
        .policy
        .affix_loading
        .headers
        .keys()
        .next()
        .unwrap()
        .clone();
    assert!(ItemLoadingCatalog::new(data).is_err());
    let mut data = original.clone();
    data.policy.rune_loading.socket_character_pattern = "x".repeat(4097);
    assert!(ItemLoadingCatalog::new(data).is_err());
    let mut data = original.clone();
    data.policy.rune_loading.other_header_patterns = (0..40)
        .map(|i| format!("{i:04}{}", "x".repeat(4092)))
        .collect();
    assert!(ItemLoadingCatalog::new(data).is_err());
    let mut data = original;
    data.policy.rune_loading.no_number_value = f64::NAN;
    assert!(ItemLoadingCatalog::new(data).is_err());
}
#[test]
fn bundled_rune_inventory_is_complete_without_granting_evaluation_capability() {
    let snapshot = bundled_snapshot().unwrap();
    assert_eq!(
        snapshot.item_loading().data().capability,
        ItemLoadingCapability::DefinitionsOnly
    );
    let runes = snapshot.item_loading().runes().unwrap();
    assert_eq!(runes.identities().count(), 287);
    let mut slots = 0;
    for (_, value) in runes.identities() {
        let ItemRuneRecord::Table(definition) = ItemRuneRecord::new(value).unwrap() else {
            panic!("original named slot table");
        };
        slots += definition.fields.len();
    }
    assert_eq!(slots, 594);
}
