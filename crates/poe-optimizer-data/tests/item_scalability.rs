use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot};
use poe_optimizer_data::item_scalability::*;

#[test]
fn complete_catalog_distinguishes_empty_keys_missing_keys_and_partial_formats() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.item_scalability();
    assert_eq!(snapshot.identity().schema_version, 30);
    assert_eq!(catalog.data().entries.len(), 15_090);
    let empty = catalog
        .data()
        .entries
        .iter()
        .find(|(_, v)| v.is_empty())
        .unwrap()
        .0;
    assert_eq!(catalog.lookup_exact(empty), Some([].as_slice()));
    assert_eq!(catalog.lookup_exact("caller key not present"), None);
    assert_eq!(catalog.format_assignments("negate"), None);
    assert_eq!(
        catalog
            .format_assignments("divide_by_three")
            .unwrap()
            .display_precision,
        None
    );
    assert_eq!(snapshot.package().actor.high_precision_mods.len(), 40);
    assert_eq!(snapshot.item_loading().policy().catalysts.len(), 13);
}
#[test]
fn custom_exact_keys_no_op_labels_partial_updates_and_independent_defaults_roundtrip() {
    let snapshot = bundled_snapshot().unwrap();
    let mut package = snapshot.package().clone();
    package.item_scalability.entries.insert(
        "Caller #\n\u{2022} Text".into(),
        vec![ItemScalabilityValue {
            is_scalable: false,
            formats: Some(vec!["caller_precision".into(), "unknown_no_op".into()]),
        }],
    );
    package
        .item_scalability
        .entries
        .insert("Empty caller definition".into(), vec![]);
    package.item_scalability.format_assignments.insert(
        "caller_precision".into(),
        ItemFormatAssignments {
            precision: Some(12.0),
            display_precision: None,
            if_required: Some(false),
        },
    );
    package.item_scalability.default_high_precision = 4;
    package.item_scalability.missing_range_value = 0.25;
    package.item_scalability.catalyst_scaling.default_quality = 7.0;
    package.item_scalability.catalyst_scaling.percent_divisor = 50.0;
    package.item_scalability.catalyst_scaling.extra_tag_flags = vec!["customFlag".into()];
    package
        .item_scalability
        .antonyms
        .insert("caller".into(), "opposite".into());
    package.refresh_section_digests().unwrap();
    let loaded = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    assert_eq!(loaded.item_scalability().data(), &package.item_scalability);
    assert_ne!(loaded.identity(), snapshot.identity());
    assert_eq!(loaded.package().actor, snapshot.package().actor);
    assert_eq!(loaded.item_loading().data(), snapshot.item_loading().data());
    assert_eq!(
        loaded.package().item_formatting,
        snapshot.package().item_formatting
    );
}
#[test]
fn validation_rejects_bad_shapes_numbers_provenance_and_aggregate_bounds() {
    let baseline = bundled_snapshot()
        .unwrap()
        .item_scalability()
        .data()
        .clone();
    let cases: [fn(&mut ItemScalabilityData); 14] = [
        |d| d.schema_version = 0,
        |d| {
            d.entries.insert("Missing capture #".into(), vec![]);
        },
        |d| {
            d.entries.insert("Nul\0".into(), vec![]);
        },
        |d| {
            d.format_assignments.insert(
                "bad".into(),
                ItemFormatAssignments {
                    precision: Some(f64::INFINITY),
                    ..Default::default()
                },
            );
        },
        |d| {
            d.format_assignments.insert(
                "bad".into(),
                ItemFormatAssignments {
                    precision: Some(0.0),
                    ..Default::default()
                },
            );
        },
        |d| {
            d.format_assignments.insert(
                "bad".into(),
                ItemFormatAssignments {
                    display_precision: Some(3),
                    ..Default::default()
                },
            );
        },
        |d| d.default_high_precision = 13,
        |d| d.missing_range_value = f64::NAN,
        |d| d.catalyst_scaling.percent_divisor = 0.0,
        |d| d.catalyst_scaling.extra_tag_flags = vec!["duplicate".into(), "duplicate".into()],
        |d| {
            d.source
                .files
                .insert("src/../outside.lua".into(), "0".repeat(64));
        },
        |d| {
            d.source
                .construction_spans
                .values_mut()
                .next()
                .unwrap()
                .path = "src/unknown.lua".into()
        },
        |d| {
            d.entries.insert(
                "#".into(),
                vec![ItemScalabilityValue {
                    is_scalable: true,
                    formats: Some(vec!["label".into(); 33]),
                }],
            );
        },
        |d| {
            d.entries = (0..50_001).map(|i| (format!("key{i}"), vec![])).collect();
        },
    ];
    for (case, mut data) in cases.into_iter().zip(std::iter::repeat(baseline.clone())) {
        case(&mut data);
        assert!(ItemScalabilityCatalog::new(data).is_err());
    }
}
#[test]
fn unknown_fields_and_duplicate_serialized_keys_do_not_pass_package_validation() {
    let mut package = bundled_snapshot().unwrap().package().clone();
    package.item_scalability.entries.insert(
        "caller #".into(),
        vec![ItemScalabilityValue {
            is_scalable: true,
            formats: None,
        }],
    );
    package.refresh_section_digests().unwrap();
    let value = serde_json::to_value(&package).unwrap();
    let mut changed = value.clone();
    changed["item_scalability"]["entries"]["caller #"][0]["unknown"] = true.into();
    assert!(
        GameDataLoader::from_bytes(
            &serde_json::to_vec(&changed).unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .is_err()
    );
    let bytes = serde_json::to_string(&value).unwrap();
    let duplicate = bytes.replacen("\"caller #\":[", "\"caller #\":[],\"caller #\":[", 1);
    assert_ne!(bytes, duplicate);
    assert!(
        GameDataLoader::from_bytes(
            duplicate.as_bytes(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .is_err()
    );
}
