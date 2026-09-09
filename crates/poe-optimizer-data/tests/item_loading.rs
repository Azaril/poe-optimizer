use poe_optimizer_data::item_loading::*;
use std::collections::{BTreeMap, BTreeSet};
fn table(fields: impl IntoIterator<Item = (&'static str, ItemMetadataValue)>) -> ItemMetadataTable {
    ItemMetadataTable {
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        indexed: BTreeMap::new(),
    }
}
fn compatibility() -> BTreeMap<String, ItemMetadataValue> {
    let mut p = BTreeMap::new();
    for key in [
        "base_aliases",
        "hidden_specs",
        "selection_headers",
        "fallback_jewel_socket_counts",
        "header_assignments",
        "literal_state_flags",
        "postparse_line_effects",
    ] {
        p.insert(
            key.into(),
            ItemMetadataValue::Table(ItemMetadataTable::default()),
        );
    }
    for key in ["noncorruptible_types", "mod_magnitude_patterns"] {
        p.insert(key.into(), ItemMetadataValue::Array(vec![]));
    }
    p.insert(
        "superior_prefix".into(),
        ItemMetadataValue::Text("Superior ".into()),
    );
    p.insert(
        "fallback_modifier_table".into(),
        ItemMetadataValue::Text("Caller Mods".into()),
    );
    p.insert(
        "rarity_roles".into(),
        ItemMetadataValue::Table(table(
            ["default", "normal", "magic", "unique", "relic"]
                .map(|k| (k, ItemMetadataValue::Text("CALLER".into()))),
        )),
    );
    p.insert(
        "base_aliases".into(),
        ItemMetadataValue::Table(table([(
            "armour_header_rewrites",
            ItemMetadataValue::Table(ItemMetadataTable::default()),
        )])),
    );
    p
}
fn catalog() -> ItemLoadingData {
    let path = "src/Data/Bases/caller.lua".to_owned();
    ItemLoadingData {
        schema_version: ITEM_LOADING_SCHEMA_VERSION,
        capability: ItemLoadingCapability::DefinitionsOnly,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: BTreeMap::from([(path.clone(), "b".repeat(64))]),
            construction_spans: BTreeMap::from([(
                "bases".into(),
                ItemSourceSpan {
                    path: path.clone(),
                    line: 1,
                    end_line: 3,
                    sha256: "c".repeat(64),
                },
            )]),
            module_order: vec![path.clone()],
        },
        policy: ItemLoadingPolicy {
            affix_loading: ItemAffixLoadingPolicy {
                headers: BTreeMap::new(),
                other_headers: BTreeSet::new(),
                other_header_patterns: vec![],
                fractured_pattern: "^caller fractured".into(),
                fractured_remove_pattern: "^caller fractured".into(),
                range_pattern: "{caller range:([^}]+)}(.+)".into(),
                range_separator: ",".into(),
                range_value_pattern: "[^,]+".into(),
                none_mod_id: "Caller Empty".into(),
                legacy_label_field: "callerAffix".into(),
                limit_rules: vec![],
                preceding_line_effects: vec![],
                limit_default: 0.0,
                reconcile: ItemAffixReconciliationPolicy {
                    magic_rarity: "CALLER".into(),
                    rare_rarity: "CALLER".into(),
                    jewel_type: "Caller Jewel".into(),
                    corrupted_jewel_subtype: "Caller Abyss".into(),
                    initial_limit: 0.0,
                    minimum_limit: 0.0,
                    magic_limit: 2.0,
                    magic_side_base: 1.0,
                    magic_side_max: 2.0,
                    rare_limit: 6.0,
                    rare_jewel_limit: 4.0,
                    side_divisor: 2.0,
                },
            },
            default_affix_quality: 0.25,
            default_item_quality: 17.0,
            catalysts: vec![],
            line_flags: BTreeSet::from(["caller_flag".into()]),
            rarities: BTreeSet::from(["CALLER".into()]),
            header_names: BTreeSet::new(),
            defence_header_keys: BTreeMap::new(),
            compatibility: compatibility(),
        },
        bases: vec![ItemBaseDefinition {
            name: "Caller Base".into(),
            item_type: "Caller Type".into(),
            source_module: path,
            fields: table([
                ("type", ItemMetadataValue::Text("Caller Type".into())),
                ("hidden", ItemMetadataValue::Boolean(true)),
                ("quality", ItemMetadataValue::Number(17.0)),
                (
                    "req",
                    ItemMetadataValue::Table(table([("level", ItemMetadataValue::Number(7.0))])),
                ),
            ]),
        }],
        modifier_tables: BTreeMap::from([("Caller Mods".into(), ItemMetadataTable::default())]),
        unique_groups: BTreeMap::new(),
        jewel_radii: ItemMetadataTable::default(),
    }
}
#[test]
fn caller_defined_catalog_is_injected_and_immutable_without_pob_or_named_bases() {
    let data = catalog();
    let compiled = ItemLoadingCatalog::new(data.clone()).unwrap();
    let mut changed = data;
    changed.bases[0].name = "Changed Base".into();
    assert!(compiled.base("Changed Base").is_none());
    assert_eq!(compiled.base("Caller Base").unwrap().hidden(), Some(true));
    assert_eq!(compiled.base("Caller Base").unwrap().quality(), Some(17.0));
    assert_eq!(compiled.policy().default_affix_quality, 0.25);
    assert!(compiled.base("Rusted Greathelm").is_none());
    assert_eq!(
        compiled.data().capability,
        ItemLoadingCapability::DefinitionsOnly
    );
}
#[test]
fn mixed_numeric_and_string_keys_roundtrip_without_aliasing() {
    let raw = r#"{"fields":{"1":"text key","opaque":{"callback":{"path":"src/callback.lua","line":7,"end_line":9,"sha256":"abc"}}},"indexed":{"1":"numeric key","4294967295":1.25,"-2":false}}"#;
    let value: ItemMetadataValue = serde_json::from_str(raw).unwrap();
    let t = value.as_table().unwrap();
    assert_eq!(t.fields["1"].as_str(), Some("text key"));
    assert_eq!(t.indexed[&1].as_str(), Some("numeric key"));
    assert_eq!(
        serde_json::from_slice::<ItemMetadataValue>(&serde_json::to_vec(&value).unwrap()).unwrap(),
        value
    );
    assert!(matches!(t.fields["opaque"], ItemMetadataValue::Callback(_)));
}
#[test]
fn numeric_key_aliases_and_duplicates_are_rejected() {
    for entries in [
        r#""1":1,"01":2"#,
        r#""1":1,"+1":2"#,
        r#""0":1,"-0":2"#,
        r#""1":1,"1":2"#,
        r#""1.0":1"#,
    ] {
        let raw = format!("{{\"fields\":{{}},\"indexed\":{{{entries}}}}}");
        assert!(
            serde_json::from_str::<ItemMetadataValue>(&raw).is_err(),
            "{raw}"
        );
    }
}
#[test]
fn source_identity_and_typed_core_cannot_disagree() {
    let mut data = catalog();
    data.bases[0].item_type = "Other".into();
    assert!(data.validate().is_err());
    let mut data = catalog();
    data.bases.push(data.bases[0].clone());
    assert!(data.validate().is_err());
    let mut data = catalog();
    data.bases[0].source_module = "src/missing.lua".into();
    assert!(data.validate().is_err());
    let mut data = catalog();
    data.source
        .files
        .insert("src/../escape.lua".into(), "a".repeat(64));
    assert!(data.validate().is_err());
}
#[test]
fn unknown_fields_are_retained_but_nonfinite_callbacks_and_depth_are_bounded() {
    let mut data = catalog();
    data.bases[0].fields.fields.insert(
        "future".into(),
        ItemMetadataValue::Array(vec![ItemMetadataValue::Boolean(false)]),
    );
    data.validate().unwrap();
    data.bases[0]
        .fields
        .fields
        .insert("bad".into(), ItemMetadataValue::Number(f64::NAN));
    assert!(data.validate().is_err());
    data.bases[0].fields.fields.remove("bad");
    data.bases[0].fields.fields.insert(
        "callback".into(),
        ItemMetadataValue::Callback(ItemOpaqueFunction {
            callback: ItemSourceSpan {
                path: "src/missing.lua".into(),
                line: 1,
                end_line: 1,
                sha256: "a".repeat(64),
            },
        }),
    );
    assert!(data.validate().is_err());
    data.bases[0].fields.fields.remove("callback");
    let mut v = ItemMetadataValue::Boolean(true);
    for _ in 0..26 {
        v = ItemMetadataValue::Array(vec![v]);
    }
    data.bases[0].fields.fields.insert("deep".into(), v);
    assert!(data.validate().is_err());
}
#[test]
fn large_raw_unique_prototypes_have_a_separate_bounded_contract() {
    let mut data = catalog();
    data.unique_groups
        .insert("caller".into(), vec!["a".repeat(65_536)]);
    data.validate().unwrap();
    data.unique_groups.get_mut("caller").unwrap()[0].push('a');
    assert!(data.validate().is_err());
    data.unique_groups.clear();
    data.bases[0]
        .fields
        .fields
        .insert("large".into(), ItemMetadataValue::Text("a".repeat(4097)));
    assert!(data.validate().is_err());
}

#[test]
fn package_unique_string_exception_is_confined_to_exact_prototype_path() {
    use poe_optimizer_data::game_data::{
        GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot,
    };
    let snapshot = bundled_snapshot().unwrap();
    let mut package = snapshot.package().clone();
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "test changed raw prototype construction inputs",
        );
    let largest = package
        .item_loading
        .unique_groups
        .values()
        .flatten()
        .max_by_key(|s| s.len())
        .unwrap();
    assert!(largest.len() > 4096 && largest.len() <= 65_536);
    package
        .item_loading
        .unique_groups
        .insert("caller".into(), vec!["x".repeat(65_536)]);
    package.refresh_section_digests().unwrap();
    GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    package
        .item_loading
        .unique_groups
        .get_mut("caller")
        .unwrap()[0]
        .push('x');
    package.refresh_section_digests().unwrap();
    assert!(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("string limit")
    );
    package.item_loading.unique_groups.remove("caller");
    package.manifest.release = "x".repeat(4097);
    package.refresh_section_digests().unwrap();
    assert!(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("string limit")
    );
    let mut v = serde_json::to_value(snapshot.package()).unwrap();
    v["item_loading"]["unique_groups"]["forged"] = serde_json::json!({"nested":["x".repeat(4097)]});
    assert!(
        GameDataLoader::from_bytes(
            &serde_json::to_vec(&v).unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("string limit")
    );
}
#[test]
fn appended_base_and_altered_numeric_fields_use_selected_package_identity() {
    use poe_optimizer_data::game_data::{
        GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot,
    };
    let initial = bundled_snapshot().unwrap();
    let mut p = initial.package().clone();
    p.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "test changed item base construction inputs",
        );
    let mut base = p
        .item_loading
        .bases
        .iter()
        .find(|b| b.requirements().is_some())
        .unwrap()
        .clone();
    base.name = "Arbitrary caller base".into();
    base.fields
        .fields
        .insert("quality".into(), ItemMetadataValue::Number(13.5));
    p.item_loading.bases.push(base);
    p.refresh_section_digests().unwrap();
    let changed = GameDataLoader::from_bytes(
        &p.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    assert_ne!(initial.identity(), changed.identity());
    assert!(
        initial
            .item_loading()
            .base("Arbitrary caller base")
            .is_none()
    );
    assert_eq!(
        changed
            .item_loading()
            .base("Arbitrary caller base")
            .unwrap()
            .quality(),
        Some(13.5)
    );
    assert!(matches!(
        changed.trust(),
        poe_optimizer_data::game_data::DataTrust::CustomUnreviewed
    ));
}

#[test]
fn required_policy_shapes_fail_closed_but_empty_exclusion_arrays_are_valid() {
    let data = catalog();
    data.validate().unwrap();
    for key in data.policy.compatibility.keys() {
        let mut changed = data.clone();
        changed.policy.compatibility.remove(key);
        assert!(changed.validate().is_err(), "{key}");
    }
    let mut data = data;
    data.policy.compatibility.insert(
        "noncorruptible_types".into(),
        ItemMetadataValue::Table(ItemMetadataTable::default()),
    );
    assert!(data.validate().is_err());
}

fn defence_catalog() -> ItemLoadingData {
    let mut data = catalog();
    data.policy
        .header_names
        .extend(["Caller Guard".into(), "Caller Rating".into()]);
    data.policy.defence_header_keys = BTreeMap::from([
        ("Caller Guard".into(), "GuardTotal".into()),
        ("Caller Rating".into(), "GuardTotal".into()),
    ]);
    data.policy.compatibility.insert(
        "base_aliases".into(),
        ItemMetadataValue::Table(table([(
            "armour_header_rewrites",
            ItemMetadataValue::Table(table([(
                "Caller Guard",
                ItemMetadataValue::Table(table([
                    (
                        "from",
                        ItemMetadataValue::Text("Absent starting base".into()),
                    ),
                    (
                        "to",
                        ItemMetadataValue::Text("Absent replacement base".into()),
                    ),
                ])),
            )])),
        )])),
    );
    data
}
fn rewrites(data: &mut ItemLoadingData) -> &mut ItemMetadataTable {
    let ItemMetadataValue::Table(aliases) =
        data.policy.compatibility.get_mut("base_aliases").unwrap()
    else {
        panic!()
    };
    let ItemMetadataValue::Table(rewrites) =
        aliases.fields.get_mut("armour_header_rewrites").unwrap()
    else {
        panic!()
    };
    rewrites
}
#[test]
fn defence_header_aliases_and_absent_base_references_are_injected_without_capability() {
    let data = defence_catalog();
    let compiled = ItemLoadingCatalog::new(data.clone()).unwrap();
    assert_eq!(
        compiled.defence_header_key("Caller Guard"),
        Some("GuardTotal")
    );
    assert_eq!(
        compiled.defence_header_key("Caller Rating"),
        Some("GuardTotal")
    );
    assert_eq!(compiled.defence_header_key("Armour"), None);
    assert_eq!(
        compiled.armour_header_rewrite("Caller Guard"),
        Some(ItemBaseRewrite {
            from: "Absent starting base",
            to: "Absent replacement base",
        })
    );
    assert_eq!(compiled.armour_header_rewrite("Caller Rating"), None);
    assert!(compiled.base("Absent replacement base").is_none());
    assert_eq!(
        compiled.data().capability,
        ItemLoadingCapability::DefinitionsOnly
    );
    let mut changed = data;
    changed
        .policy
        .defence_header_keys
        .insert("Caller Guard".into(), "Different".into());
    assert_eq!(
        compiled.defence_header_key("Caller Guard"),
        Some("GuardTotal")
    );
}
#[test]
fn defence_header_schema_rejects_missing_duplicate_unknown_and_oversized_entries() {
    let data = defence_catalog();
    let json = serde_json::to_string(&data).unwrap();
    let duplicate = json.replace(
        "\"Caller Guard\":\"GuardTotal\"",
        "\"Caller Guard\":\"GuardTotal\",\"Caller Guard\":\"Other\"",
    );
    assert!(serde_json::from_str::<ItemLoadingData>(&duplicate).is_err());
    let duplicate_from = json.replace(
        "\"from\":\"Absent starting base\"",
        "\"from\":\"Absent starting base\",\"from\":\"Other\"",
    );
    assert!(serde_json::from_str::<ItemLoadingData>(&duplicate_from).is_err());
    let mut value = serde_json::to_value(&data).unwrap();
    value["policy"]
        .as_object_mut()
        .unwrap()
        .remove("defence_header_keys");
    assert!(serde_json::from_value::<ItemLoadingData>(value).is_err());
    let mut changed = data.clone();
    changed.schema_version = 1;
    assert!(changed.validate().is_err());
    for (header, key) in [
        ("Unknown", "Value"),
        ("Caller Guard", ""),
        ("Caller Guard", "bad\0key"),
    ] {
        let mut changed = data.clone();
        changed
            .policy
            .defence_header_keys
            .insert(header.into(), key.into());
        assert!(changed.validate().is_err(), "{header} {key}");
    }
    let mut changed = data.clone();
    changed
        .policy
        .defence_header_keys
        .insert("Caller Guard".into(), "x".repeat(257));
    assert!(changed.validate().is_err());
    let mut changed = data;
    changed.policy.defence_header_keys = (0..257)
        .map(|i| (format!("Header {i}"), "Value".into()))
        .collect();
    changed.policy.header_names = changed.policy.defence_header_keys.keys().cloned().collect();
    assert!(changed.validate().is_err());
}
#[test]
fn defence_rewrites_validate_shape_and_header_membership_without_requiring_base_membership() {
    let data = defence_catalog();
    for rule in [
        ItemMetadataValue::Text("wrong".into()),
        ItemMetadataValue::Table(table([("from", ItemMetadataValue::Text("A".into()))])),
        ItemMetadataValue::Table(table([
            ("from", ItemMetadataValue::Text("A".into())),
            ("to", ItemMetadataValue::Number(2.0)),
        ])),
        ItemMetadataValue::Table(table([
            ("from", ItemMetadataValue::Text("".into())),
            ("to", ItemMetadataValue::Text("B".into())),
        ])),
        ItemMetadataValue::Table(table([
            ("from", ItemMetadataValue::Text("A".into())),
            ("to", ItemMetadataValue::Text("B".into())),
            ("extra", ItemMetadataValue::Boolean(true)),
        ])),
    ] {
        let mut changed = data.clone();
        rewrites(&mut changed)
            .fields
            .insert("Caller Guard".into(), rule);
        assert!(changed.validate().is_err());
    }
    let mut changed = data.clone();
    let rule = rewrites(&mut changed)
        .fields
        .remove("Caller Guard")
        .unwrap();
    rewrites(&mut changed)
        .fields
        .insert("Other Header".into(), rule);
    assert!(changed.validate().is_err());
    let mut changed = data.clone();
    rewrites(&mut changed)
        .indexed
        .insert(1, ItemMetadataValue::Boolean(true));
    assert!(changed.validate().is_err());
    let mut changed = data;
    if let ItemMetadataValue::Table(aliases) = changed
        .policy
        .compatibility
        .get_mut("base_aliases")
        .unwrap()
    {
        aliases.fields.remove("armour_header_rewrites");
    }
    assert!(changed.validate().is_err());
}
#[test]
fn defence_headers_allow_later_hidden_overlap_but_reject_other_declared_operation_collisions() {
    let mut data = defence_catalog();
    data.policy.compatibility.insert(
        "hidden_specs".into(),
        ItemMetadataValue::Table(table([("Caller Guard", ItemMetadataValue::Boolean(true))])),
    );
    data.validate().unwrap();
    let mut changed = data.clone();
    changed.policy.compatibility.insert(
        "selection_headers".into(),
        ItemMetadataValue::Table(table([("Caller Guard", ItemMetadataValue::Boolean(true))])),
    );
    assert!(changed.validate().is_err());
    data.policy.compatibility.insert(
        "header_assignments".into(),
        ItemMetadataValue::Table(table([(
            "Caller Guard",
            ItemMetadataValue::Table(table([
                ("field", ItemMetadataValue::Text("other".into())),
                ("kind", ItemMetadataValue::Text("number".into())),
            ])),
        )])),
    );
    assert!(data.validate().is_err());
}
