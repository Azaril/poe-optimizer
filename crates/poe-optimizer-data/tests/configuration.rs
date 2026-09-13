use poe_optimizer_core::options::Scalar;
use poe_optimizer_data::configuration::*;
use std::collections::BTreeMap;
type Mutation = Box<dyn Fn(&mut ConfigurationData)>;

fn span() -> ConfigSourceSpan {
    ConfigSourceSpan {
        path: "src/Config.lua".into(),
        line: 1,
        end_line: 9,
        sha256: "a".repeat(64),
    }
}
fn row(index: u32, key: &str, widget: ConfigWidgetKind) -> ConfigDefinition {
    ConfigDefinition {
        id: format!("occurrence-{index}"),
        key: key.into(),
        source_table_index: index * 2,
        source_variable_order: index,
        widget,
        scalar_kinds: vec![match widget {
            ConfigWidgetKind::Check => ConfigScalarKind::Boolean,
            ConfigWidgetKind::Text => ConfigScalarKind::Text,
            _ => ConfigScalarKind::Number,
        }],
        label: Some("Source label".into()),
        options: vec![],
        defaults: ConfigDeclaredDefaults::default(),
        source: ConfigDefinitionSource {
            location: ConfigSourceLocation {
                path: span().path,
                line: 2,
            },
            quest: None,
            generator: None,
        },
        metadata: BTreeMap::new(),
    }
}
fn data(definitions: Vec<ConfigDefinition>) -> ConfigurationData {
    ConfigurationData {
        schema_version: CONFIGURATION_SCHEMA_VERSION,
        source: ConfigSourceIdentity {
            upstream_revision: "1".repeat(40),
            files: BTreeMap::from([("src/Config.lua".into(), "b".repeat(64))]),
            create_config_set: span(),
            get_default_state: span(),
        },
        authored_load: ConfigAuthoredLoadPolicy {
            source: span(),
            set_active_source: span(),
            default_set_title: "Default".into(),
            default_custom_block_title: "Default".into(),
            legacy_custom_mods_key: "customMods".into(),
            input_string_rewrites: vec![],
        },
        source_table_rows: definitions.len() as u32 * 2 + 1,
        definitions,
        capability: ConfigCapability::MetadataOnly,
    }
}
fn option(index: u32, value: Scalar) -> ConfigOption {
    ConfigOption {
        index,
        value,
        label: None,
        metadata: BTreeMap::new(),
    }
}
fn list(index: u32, key: &str) -> ConfigDefinition {
    let mut value = row(index, key, ConfigWidgetKind::List);
    value.scalar_kinds = vec![
        ConfigScalarKind::Text,
        ConfigScalarKind::Number,
        ConfigScalarKind::Boolean,
    ];
    value.options = vec![
        option(1, Scalar::Text("one\n\ttwo".into())),
        option(2, Scalar::Number(-0.0)),
        option(3, Scalar::Boolean(false)),
    ];
    value
}
#[test]
fn ordered_duplicate_occurrences_are_retained_and_initial_nil_assignments_remove_values() {
    let mut first = row(1, "duplicate", ConfigWidgetKind::Count);
    first.defaults.input = Some(Scalar::Number(12.0));
    first.defaults.placeholder = Some(Scalar::Number(8.0));
    let last = row(2, "duplicate", ConfigWidgetKind::Count);
    let catalog = ConfigDefinitionCatalog::new(data(vec![first, last])).unwrap();
    assert_eq!(catalog.unique_key_count(), 1);
    assert_eq!(
        catalog
            .definitions_for_key("duplicate")
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["occurrence-1", "occurrence-2"]
    );
    assert_eq!(
        catalog
            .definition("occurrence-2")
            .unwrap()
            .source_table_index,
        4
    );
    assert!(catalog.definitions_for_key("unknown").next().is_none());
    assert!(catalog.definition("unknown").is_none());
    assert_eq!(catalog.initial_state(), ConfigInitialState::default());
}
#[test]
fn source_option_index_overwrites_input_without_overwriting_placeholder_even_for_false() {
    let mut value = list(1, "choice");
    value.defaults.input = Some(Scalar::Text("not an option".into()));
    value.defaults.placeholder = Some(Scalar::Number(42.0));
    value.defaults.option_index = Some(3);
    let catalog = ConfigDefinitionCatalog::new(data(vec![value])).unwrap();
    let state = catalog.initial_state();
    assert_eq!(state.inputs["choice"], Scalar::Boolean(false));
    assert_eq!(state.placeholders["choice"], Scalar::Number(42.0));
}
#[test]
fn initial_state_does_not_add_widget_fallbacks_or_run_callback_metadata() {
    let mut rows = vec![
        row(1, "check", ConfigWidgetKind::Check),
        row(2, "number", ConfigWidgetKind::Count),
        row(3, "text", ConfigWidgetKind::Text),
        list(4, "list"),
    ];
    rows[0]
        .metadata
        .insert("apply".into(), ConfigMetadataValue::Callback(span()));
    let catalog = ConfigDefinitionCatalog::new(data(rows)).unwrap();
    assert!(catalog.initial_state().inputs.is_empty());
    assert!(catalog.initial_state().placeholders.is_empty());
    assert_eq!(catalog.data().capability, ConfigCapability::MetadataOnly);
}
#[test]
fn explicit_false_zero_empty_signed_zero_and_subnormal_defaults_survive() {
    let values = [
        Scalar::Boolean(false),
        Scalar::Number(0.0),
        Scalar::Text(String::new()),
        Scalar::Number(-0.0),
        Scalar::Number(f64::from_bits(1)),
    ];
    let rows = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let mut value = row(
                i as u32 + 1,
                &format!("key{i}"),
                match v {
                    Scalar::Boolean(_) => ConfigWidgetKind::Check,
                    Scalar::Text(_) => ConfigWidgetKind::Text,
                    _ => ConfigWidgetKind::Float,
                },
            );
            value.defaults.input = Some(v.clone());
            value
        })
        .collect();
    let raw = data(rows);
    let encoded = serde_json::to_vec(&raw).unwrap();
    let decoded: ConfigurationData = serde_json::from_slice(&encoded).unwrap();
    let state = ConfigDefinitionCatalog::new(decoded)
        .unwrap()
        .initial_state();
    assert_eq!(state.inputs.len(), values.len());
    for (i, v) in values.iter().enumerate() {
        assert_eq!(&state.inputs[&format!("key{i}")], v);
    }
    let Scalar::Number(zero) = state.inputs["key3"] else {
        panic!()
    };
    assert_eq!(zero.to_bits(), (-0.0f64).to_bits());
    let Scalar::Number(tiny) = state.inputs["key4"] else {
        panic!()
    };
    assert_eq!(tiny.to_bits(), 1);
}
#[test]
fn exact_typed_options_preserve_bytes_order_and_source_numeric_equality() {
    let catalog = ConfigDefinitionCatalog::new(data(vec![list(1, "choice")])).unwrap();
    let options = &catalog.definitions()[0].options;
    assert!(options[0].matches(&Scalar::Text("one\n\ttwo".into())));
    assert!(!options[0].matches(&Scalar::Text("one two".into())));
    assert!(options[1].matches(&Scalar::Number(0.0)));
    assert!(!options[1].matches(&Scalar::Text("0".into())));
    assert!(!options[1].matches(&Scalar::Boolean(false)));
    assert_eq!(
        catalog.definitions()[0].scalar_kinds[0],
        ConfigScalarKind::Text
    );
}
#[test]
fn source_reordering_and_injected_keys_change_defaults_without_fixture_dispatch() {
    let mut a = row(1, "caller-key", ConfigWidgetKind::Count);
    a.defaults.input = Some(Scalar::Number(2.0));
    let mut b = row(2, "caller-key", ConfigWidgetKind::Count);
    b.defaults.input = Some(Scalar::Number(7.0));
    let mut raw = data(vec![a, b]);
    assert_eq!(
        ConfigDefinitionCatalog::new(raw.clone())
            .unwrap()
            .initial_state()
            .inputs["caller-key"],
        Scalar::Number(7.0)
    );
    raw.definitions.reverse();
    for (i, row) in raw.definitions.iter_mut().enumerate() {
        row.source_variable_order = i as u32 + 1;
        row.source_table_index = i as u32 * 2 + 2;
    }
    assert_eq!(
        ConfigDefinitionCatalog::new(raw)
            .unwrap()
            .initial_state()
            .inputs["caller-key"],
        Scalar::Number(2.0)
    );
}
#[test]
fn catalog_clone_shares_validated_immutable_data_and_indexes() {
    let catalog = ConfigDefinitionCatalog::new(data(vec![list(1, "choice")])).unwrap();
    let clone = catalog.clone();
    assert!(std::ptr::eq(catalog.data(), clone.data()));
    assert!(std::ptr::eq(
        catalog.definition("occurrence-1").unwrap(),
        clone.definition("occurrence-1").unwrap()
    ));
}
#[test]
fn invalid_scalar_types_option_indexes_and_occurrence_order_are_rejected() {
    let seed = data(vec![list(1, "choice")]);
    let cases: Vec<Mutation> = vec![
        Box::new(|d| d.definitions[0].defaults.option_index = Some(0)),
        Box::new(|d| d.definitions[0].defaults.option_index = Some(4)),
        Box::new(|d| d.definitions[0].options[1].index = 1),
        Box::new(|d| d.definitions[0].source_variable_order = 2),
        Box::new(|d| d.definitions[0].source_table_index = 0),
        Box::new(|d| d.definitions[0].scalar_kinds.push(ConfigScalarKind::Number)),
        Box::new(|d| d.definitions[0].scalar_kinds.pop().map(|_| ()).unwrap()),
        Box::new(|d| d.definitions[0].widget = ConfigWidgetKind::Count),
    ];
    for change in cases {
        let mut raw = seed.clone();
        change(&mut raw);
        assert!(ConfigDefinitionCatalog::new(raw).is_err());
    }
}
#[test]
fn source_defaults_are_copied_even_outside_the_widgets_authoring_kind() {
    let mut raw = data(vec![row(1, "number", ConfigWidgetKind::Count)]);
    raw.definitions[0].defaults.input = Some(Scalar::Boolean(false));
    raw.definitions[0].defaults.placeholder = Some(Scalar::Text("exact source default".into()));
    let catalog = ConfigDefinitionCatalog::new(raw).unwrap();
    assert_eq!(
        catalog.initial_state().inputs["number"],
        Scalar::Boolean(false)
    );
    assert_eq!(
        catalog.initial_state().placeholders["number"],
        Scalar::Text("exact source default".into())
    );
}
#[test]
fn duplicate_ids_and_unordered_positions_reject_but_duplicate_values_are_preserved() {
    let mut raw = data(vec![
        row(1, "x", ConfigWidgetKind::Check),
        row(2, "x", ConfigWidgetKind::Check),
    ]);
    raw.definitions[1].id = raw.definitions[0].id.clone();
    assert!(raw.validate().is_err());
    raw.definitions[1].id = "other".into();
    raw.definitions[1].source_table_index = 2;
    assert!(raw.validate().is_err());
    let mut raw = data(vec![list(1, "x")]);
    raw.definitions[0]
        .options
        .push(option(4, Scalar::Number(0.0)));
    let c = ConfigDefinitionCatalog::new(raw).unwrap();
    assert_eq!(
        c.definitions()[0]
            .options
            .iter()
            .filter(|o| o.matches(&Scalar::Number(0.0)))
            .map(|o| o.index)
            .collect::<Vec<_>>(),
        [2, 4]
    );
}
#[test]
fn nonfinite_values_and_resource_excess_reject_before_catalog_creation() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut raw = data(vec![row(1, "n", ConfigWidgetKind::Count)]);
        raw.definitions[0].defaults.input = Some(Scalar::Number(bad));
        assert!(raw.validate().is_err());
        raw.definitions[0].defaults.input = None;
        raw.definitions[0]
            .metadata
            .insert("observation".into(), ConfigMetadataValue::Number(bad));
        assert!(raw.validate().is_err());
        let mut raw = data(vec![list(1, "n")]);
        raw.definitions[0].options[1].value = Scalar::Number(bad);
        assert!(raw.validate().is_err());
    }
    let mut raw = data(vec![row(1, "x", ConfigWidgetKind::Text)]);
    raw.definitions[0].defaults.input = Some(Scalar::Text("x".repeat(4097)));
    assert!(raw.validate().is_err());
    raw.definitions[0].defaults.input = None;
    let mut nested = ConfigMetadataValue::Boolean(true);
    for _ in 0..25 {
        nested = ConfigMetadataValue::Array(vec![nested]);
    }
    raw.definitions[0].metadata.insert("deep".into(), nested);
    assert!(raw.validate().is_err());
    let mut raw = data(vec![row(1, "x", ConfigWidgetKind::Text)]);
    raw.source_table_rows = 100_001;
    assert!(raw.validate().is_err());
}
#[test]
fn source_evidence_requires_valid_relative_files_spans_and_paired_quest_provenance() {
    let seed = data(vec![row(1, "x", ConfigWidgetKind::Check)]);
    let cases: Vec<Mutation> = vec![
        Box::new(|d| d.source.upstream_revision = "unknown".into()),
        Box::new(|d| d.source.create_config_set.end_line = 0),
        Box::new(|d| d.definitions[0].source.location.path = "../outside.lua".into()),
        Box::new(|d| d.definitions[0].source.location.path = "src/undeclared.lua".into()),
        Box::new(|d| d.definitions[0].source.generator = Some(span())),
        Box::new(|d| {
            d.definitions[0]
                .metadata
                .insert(
                    "apply".into(),
                    ConfigMetadataValue::Callback(ConfigSourceSpan {
                        sha256: "bad".into(),
                        ..span()
                    }),
                )
                .map(|_| ())
                .unwrap_or(())
        }),
    ];
    for change in cases {
        let mut raw = seed.clone();
        change(&mut raw);
        assert!(raw.validate().is_err());
    }
    let mut raw = seed;
    raw.definitions[0].source.generator = Some(span());
    raw.definitions[0].source.quest = Some(ConfigQuestSource {
        source_index: 17,
        record: BTreeMap::from([(
            "exact".into(),
            ConfigMetadataValue::Text("choice\n\ttext".into()),
        )]),
    });
    assert!(raw.validate().is_ok());
}
#[test]
fn strict_serialization_rejects_unknown_fields_kinds_and_missing_declared_defaults() {
    let seed = serde_json::to_value(data(vec![list(1, "x")])).unwrap();
    for (pointer, value) in [
        ("/unexpected", serde_json::json!(1)),
        ("/capability", serde_json::json!("native")),
        ("/definitions/0/widget", serde_json::json!("slider")),
        ("/definitions/0/options/0/extra", serde_json::json!(true)),
        (
            "/definitions/0/metadata/apply",
            serde_json::json!({"kind":"executable","value":"run"}),
        ),
    ] {
        let mut raw = seed.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let map = raw.pointer_mut(parent).unwrap().as_object_mut().unwrap();
        map.insert(key.into(), value);
        assert!(
            serde_json::from_value::<ConfigurationData>(raw).is_err(),
            "{pointer}"
        );
    }
    let mut raw = seed;
    raw["definitions"][0]["defaults"]
        .as_object_mut()
        .unwrap()
        .remove("placeholder");
    assert!(serde_json::from_value::<ConfigurationData>(raw).is_err());
}
#[test]
fn callback_descriptors_and_nested_metadata_round_trip_without_semantic_promotion() {
    let mut raw = data(vec![row(1, "x", ConfigWidgetKind::Check)]);
    raw.definitions[0]
        .metadata
        .insert("apply".into(), ConfigMetadataValue::Callback(span()));
    raw.definitions[0].metadata.insert(
        "ifCond".into(),
        ConfigMetadataValue::Array(vec![ConfigMetadataValue::Text("UnknownCondition".into())]),
    );
    let decoded: ConfigurationData =
        serde_json::from_slice(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(decoded, raw);
    assert_eq!(
        ConfigDefinitionCatalog::new(decoded)
            .unwrap()
            .initial_state(),
        ConfigInitialState::default()
    );
}

#[test]
fn bundled_snapshot_exposes_complete_ordered_catalog_independently_of_an_evaluator() {
    use poe_optimizer_data::game_data::*;
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.configuration();
    assert_eq!(snapshot.identity().schema_version, 34);
    assert_eq!(catalog.data(), &snapshot.package().configuration);
    assert_eq!(catalog.definitions().len(), 564);
    assert_eq!(catalog.unique_key_count(), 563);
    assert_eq!(catalog.data().source_table_rows, 663);
    assert_eq!(
        catalog
            .definitions_for_key("conditionEnemyExitedPresenceRecently")
            .map(|r| r.source_table_index)
            .collect::<Vec<_>>(),
        [523, 524]
    );
    assert_eq!(catalog.data().capability, ConfigCapability::MetadataOnly);
    let detached = ConfigDefinitionCatalog::new(snapshot.package().configuration.clone()).unwrap();
    assert_eq!(detached.initial_state(), catalog.initial_state());
    assert_eq!(
        serde_json::to_vec(catalog.data()).unwrap(),
        serde_json::to_vec(detached.data()).unwrap()
    );
}
#[test]
fn catalog_content_changes_are_section_and_host_bound_without_granting_native_effects() {
    use poe_optimizer_data::game_data::*;
    let original = bundled_snapshot().unwrap();
    let original_key = original.configuration().definitions()[0].key.clone();
    let mut package = original.package().clone();
    package.configuration.definitions[0].key = "caller-injected-config-key".into();
    let bytes = package.canonical_bytes().unwrap();
    assert!(
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap_err()
            .to_string()
            .contains("section SHA-256")
    );
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let custom =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    assert_ne!(original.identity(), custom.identity());
    assert_ne!(
        original.package().manifest.section_sha256["configuration"],
        custom.package().manifest.section_sha256["configuration"]
    );
    for (section, digest) in &original.package().manifest.section_sha256 {
        if section != "configuration" {
            assert_eq!(digest, &custom.package().manifest.section_sha256[section]);
        }
    }
    assert_eq!(custom.trust(), &DataTrust::CustomUnreviewed);
    assert_eq!(
        custom.configuration().data().capability,
        ConfigCapability::MetadataOnly
    );
    assert!(
        custom
            .configuration()
            .definitions_for_key("caller-injected-config-key")
            .next()
            .is_some()
    );
    assert!(
        original
            .configuration()
            .definitions_for_key(&original_key)
            .next()
            .is_some()
    );
    assert!(
        GameDataLoader::from_bytes(
            &bytes,
            &TrustPolicy::Reviewed {
                expected_sha256: bundled_package_sha256().into()
            },
            &LoadLimits::default()
        )
        .is_err()
    );
}
#[test]
fn package_requires_configuration_and_validates_it_even_with_fresh_self_hashes() {
    use poe_optimizer_data::game_data::*;
    let snapshot = bundled_snapshot().unwrap();
    let mut value = serde_json::to_value(snapshot.package()).unwrap();
    value.as_object_mut().unwrap().remove("configuration");
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&value).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
    let mut package = snapshot.package().clone();
    let index = package
        .configuration
        .definitions
        .iter()
        .position(|r| r.widget == ConfigWidgetKind::List)
        .unwrap();
    package.configuration.definitions[index]
        .defaults
        .option_index = Some(0);
    package.refresh_section_digests().unwrap();
    assert!(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("defaultIndex")
    );
}

#[test]
fn authored_load_policy_is_injected_bounded_and_strictly_serialized() {
    let mut seed = data(vec![row(1, "x", ConfigWidgetKind::Check)]);
    seed.authored_load.default_set_title = "caller title".into();
    seed.authored_load.default_custom_block_title = String::new();
    seed.authored_load.legacy_custom_mods_key = "caller_legacy".into();
    seed.authored_load
        .input_string_rewrites
        .push(ConfigInputStringRewrite {
            key: "caller_input".into(),
            source: span(),
            operations: vec![
                ConfigStringRewrite::AsciiLower,
                ConfigStringRewrite::AsciiTitleWords,
                ConfigStringRewrite::LuaGsub {
                    pattern: "^old ".into(),
                    replacement: "new ".into(),
                },
            ],
        });
    let catalog = ConfigDefinitionCatalog::new(seed.clone()).unwrap();
    assert_eq!(catalog.data().authored_load, seed.authored_load);
    assert_eq!(catalog.data().capability, ConfigCapability::MetadataOnly);
    let encoded = serde_json::to_value(&seed).unwrap();
    assert_eq!(
        serde_json::from_value::<ConfigurationData>(encoded.clone()).unwrap(),
        seed
    );
    let mut missing = encoded.clone();
    missing.as_object_mut().unwrap().remove("authored_load");
    assert!(serde_json::from_value::<ConfigurationData>(missing).is_err());
    let mut unknown = encoded;
    unknown["authored_load"]["input_string_rewrites"][0]["operations"][0]["kind"] =
        "arbitrary_code".into();
    assert!(serde_json::from_value::<ConfigurationData>(unknown).is_err());
    let cases: Vec<Mutation> = vec![
        Box::new(|d| d.authored_load.source.sha256 = "unverified".into()),
        Box::new(|d| d.authored_load.set_active_source.path = "../outside.lua".into()),
        Box::new(|d| d.authored_load.input_string_rewrites[0].source.end_line = 10),
        Box::new(|d| d.authored_load.input_string_rewrites[0].operations.clear()),
        Box::new(|d| {
            d.authored_load.input_string_rewrites[0].operations =
                vec![ConfigStringRewrite::AsciiLower; 65]
        }),
        Box::new(|d| {
            d.authored_load
                .input_string_rewrites
                .push(d.authored_load.input_string_rewrites[0].clone())
        }),
        Box::new(|d| d.authored_load.legacy_custom_mods_key.clear()),
        Box::new(|d| d.authored_load.default_set_title = "x".repeat(4097)),
        Box::new(|d| {
            d.authored_load.input_string_rewrites[0].operations =
                vec![ConfigStringRewrite::LuaGsub {
                    pattern: "x".repeat(4097),
                    replacement: String::new(),
                }]
        }),
    ];
    for mutate in cases {
        let mut changed = seed.clone();
        mutate(&mut changed);
        assert!(changed.validate().is_err());
    }
}
