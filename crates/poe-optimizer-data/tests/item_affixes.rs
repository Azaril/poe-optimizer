use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::*};
use std::collections::BTreeMap;

fn row(label: &str) -> ItemMetadataValue {
    ItemMetadataValue::Table(ItemMetadataTable {
        fields: BTreeMap::from([(
            "authoredLabel".into(),
            ItemMetadataValue::Text(label.into()),
        )]),
        indexed: BTreeMap::new(),
    })
}
fn data(table: ItemMetadataTable) -> ItemLoadingData {
    let mut data = bundled_snapshot().unwrap().item_loading().data().clone();
    data.policy.affix_loading.legacy_label_field = "authoredLabel".into();
    data.modifier_tables = BTreeMap::from([("Authored Family".into(), table)]);
    data
}
fn table(rows: impl IntoIterator<Item = (&'static str, ItemMetadataValue)>) -> ItemMetadataTable {
    ItemMetadataTable {
        fields: rows.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        indexed: BTreeMap::new(),
    }
}
#[test]
fn injected_exact_and_unique_legacy_lookup_are_borrowed_and_immutable() {
    let mut input = data(table([("Authored ID", row("Authored Legacy"))]));
    let catalog = ItemLoadingCatalog::new(input.clone()).unwrap();
    input.modifier_tables.clear();
    assert!(matches!(
        catalog.affix_lookup(Some("Authored Family"), "Authored ID"),
        ItemAffixLookup::Exact {
            mod_id: "Authored ID",
            ..
        }
    ));
    let ItemAffixLookup::Legacy { mod_id, value } =
        catalog.affix_lookup(Some("Authored Family"), "Authored Legacy")
    else {
        panic!("unique source label must resolve");
    };
    let (stored_id, stored_value) = catalog
        .modifier_table("Authored Family")
        .unwrap()
        .fields
        .get_key_value("Authored ID")
        .unwrap();
    assert_eq!(mod_id.as_ptr(), stored_id.as_ptr());
    assert!(std::ptr::eq(value, stored_value));
    assert_eq!(
        catalog.affix_lookup(Some("Authored Family"), "Definitely missing"),
        ItemAffixLookup::Missing
    );
    assert_eq!(
        catalog.affix_lookup(Some("Unknown family"), "Authored ID"),
        ItemAffixLookup::Unavailable
    );
    assert_eq!(
        catalog.affix_lookup(None, "Authored ID"),
        ItemAffixLookup::Unavailable
    );
}
#[test]
fn duplicate_legacy_labels_never_select_sorted_winners_and_exact_still_wins() {
    let catalog = ItemLoadingCatalog::new(data(table([
        ("A", row("Shared")),
        ("Z", row("Shared")),
        ("Shared", row("Other")),
        ("B", row("Ambiguous")),
        ("Y", row("Ambiguous")),
    ])))
    .unwrap();
    assert_eq!(
        catalog.affix_lookup(Some("Authored Family"), "Ambiguous"),
        ItemAffixLookup::Ambiguous
    );
    assert!(matches!(
        catalog.affix_lookup(Some("Authored Family"), "Shared"),
        ItemAffixLookup::Exact {
            mod_id: "Shared",
            ..
        }
    ));
}
#[test]
fn scalar_truthiness_and_malformed_fallback_follow_original_lookup_order() {
    for exact_value in [
        ItemMetadataValue::Boolean(true),
        ItemMetadataValue::Number(0.0),
        ItemMetadataValue::Number(-0.0),
        ItemMetadataValue::Text(String::new()),
    ] {
        let catalog = ItemLoadingCatalog::new(data(table([
            ("Exact", exact_value),
            ("Malformed", ItemMetadataValue::Boolean(false)),
        ])))
        .unwrap();
        assert!(matches!(
            catalog.affix_lookup(Some("Authored Family"), "Exact"),
            ItemAffixLookup::Exact {
                mod_id: "Exact",
                ..
            }
        ));
        assert_eq!(
            catalog.affix_lookup(Some("Authored Family"), "Missing"),
            ItemAffixLookup::Unavailable
        );
        assert_eq!(
            catalog.affix_lookup(Some("Authored Family"), "Malformed"),
            ItemAffixLookup::Unavailable
        );
    }
    let catalog = ItemLoadingCatalog::new(data(table([
        ("Text", ItemMetadataValue::Text("source string".into())),
        (
            "Array",
            ItemMetadataValue::Array(vec![ItemMetadataValue::Boolean(false)]),
        ),
        ("Only", row("Legacy")),
    ])))
    .unwrap();
    assert_eq!(
        catalog.affix_lookup(Some("Authored Family"), "Missing"),
        ItemAffixLookup::Missing
    );
    assert!(matches!(
        catalog.affix_lookup(Some("Authored Family"), "Legacy"),
        ItemAffixLookup::Legacy { mod_id: "Only", .. }
    ));
}
#[test]
fn numeric_legacy_identity_is_explicitly_unavailable_only_when_it_matches() {
    let mut values = table([("Named", row("Named Legacy"))]);
    values.indexed.insert(3, row("Numeric Legacy"));
    let catalog = ItemLoadingCatalog::new(data(values.clone())).unwrap();
    assert_eq!(
        catalog.affix_lookup(Some("Authored Family"), "Numeric Legacy"),
        ItemAffixLookup::Unavailable
    );
    assert_eq!(
        catalog.affix_lookup(Some("Authored Family"), "Missing"),
        ItemAffixLookup::Missing
    );
    assert!(matches!(
        catalog.affix_lookup(Some("Authored Family"), "Named Legacy"),
        ItemAffixLookup::Legacy {
            mod_id: "Named",
            ..
        }
    ));
    values.fields.insert("Named".into(), row("Numeric Legacy"));
    let catalog = ItemLoadingCatalog::new(data(values)).unwrap();
    assert_eq!(
        catalog.affix_lookup(Some("Authored Family"), "Numeric Legacy"),
        ItemAffixLookup::Unavailable
    );
}
#[test]
fn affix_policy_rejects_missing_precedence_conflicting_headers_and_unbounded_numbers() {
    let original = data(table([]));
    let mut changed = original.clone();
    changed
        .policy
        .affix_loading
        .preceding_line_effects
        .push("Unknown exact effect".into());
    assert!(
        changed
            .validate()
            .unwrap_err()
            .to_string()
            .contains("preceding")
    );
    let mut changed = original.clone();
    let first = changed.policy.affix_loading.preceding_line_effects[0].clone();
    changed
        .policy
        .affix_loading
        .preceding_line_effects
        .push(first);
    assert!(
        changed
            .validate()
            .unwrap_err()
            .to_string()
            .contains("duplicated")
    );
    let mut changed = original.clone();
    let header = changed
        .policy
        .defence_header_keys
        .keys()
        .next()
        .unwrap()
        .clone();
    changed
        .policy
        .affix_loading
        .headers
        .insert(header, ItemAffixSide::Prefix);
    assert!(
        changed
            .validate()
            .unwrap_err()
            .to_string()
            .contains("conflicts")
    );
    let mut changed = original.clone();
    changed.policy.affix_loading.reconcile.side_divisor = 0.0;
    assert!(
        changed
            .validate()
            .unwrap_err()
            .to_string()
            .contains("divisor")
    );
    let mut changed = original.clone();
    changed.policy.affix_loading.reconcile.magic_limit = f64::INFINITY;
    assert!(
        changed
            .validate()
            .unwrap_err()
            .to_string()
            .contains("numeric")
    );
    let mut changed = original.clone();
    changed.policy.affix_loading.reconcile.magic_rarity = "Unknown Rarity".into();
    assert!(
        changed
            .validate()
            .unwrap_err()
            .to_string()
            .contains("rarity")
    );
    let mut changed = original;
    changed.policy.affix_loading.range_pattern = "x".repeat(4097);
    assert!(changed.validate().is_err());
}

#[test]
fn empty_source_patterns_and_present_empty_predecessor_are_valid_operations() {
    let mut input = data(table([]));
    input.policy.affix_loading.fractured_pattern.clear();
    input.policy.affix_loading.fractured_remove_pattern.clear();
    input.policy.affix_loading.range_pattern.clear();
    input.policy.affix_loading.range_separator.clear();
    input.policy.affix_loading.range_value_pattern.clear();
    input.policy.affix_loading.other_header_patterns = vec![String::new()];
    for rule in &mut input.policy.affix_loading.limit_rules {
        rule.match_pattern.clear();
        rule.positive_pattern.clear();
        rule.negative_pattern.clear();
    }
    let effects = input
        .policy
        .compatibility
        .get_mut("postparse_line_effects")
        .unwrap();
    let ItemMetadataValue::Table(effects) = effects else {
        panic!("effects must be a table");
    };
    effects.fields.insert(
        String::new(),
        ItemMetadataValue::Table(ItemMetadataTable::default()),
    );
    input.policy.affix_loading.preceding_line_effects = vec![String::new()];
    input.validate().unwrap();
    // Pattern applicability/collisions belong to the native pattern consumer;
    // the data seam must not confuse valid empty grammar with absent identity.
    let mut absent_identity = input.clone();
    absent_identity.policy.affix_loading.none_mod_id.clear();
    assert!(
        absent_identity
            .validate()
            .unwrap_err()
            .to_string()
            .contains("empty")
    );
    let mut absent_predecessor = input;
    absent_predecessor.policy.compatibility.insert(
        "postparse_line_effects".into(),
        ItemMetadataValue::Table(ItemMetadataTable::default()),
    );
    assert!(
        absent_predecessor
            .validate()
            .unwrap_err()
            .to_string()
            .contains("preceding")
    );
}
