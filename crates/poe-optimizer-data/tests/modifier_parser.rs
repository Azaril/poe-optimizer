use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
fn data() -> ModifierParserData {
    bundled_snapshot().unwrap().modifier_parser().data().clone()
}
#[test]
fn complete_catalog_preserves_uninterpreted_graph_and_source_evidence() {
    let data = data();
    data.validate().unwrap();
    assert_eq!(data.dictionaries.len(), 28);
    assert!(data.tables.len() > 10_000);
    assert!(data.callbacks.len() > 1_600);
    assert_eq!(data.capability, ParserCapability::DefinitionsOnly);
    assert!(
        data.callbacks
            .iter()
            .any(|f| matches!(f.kind, ParserCallbackKind::Builtin { .. }))
    );
    assert!(
        data.callbacks
            .iter()
            .flat_map(|f| &f.upvalues)
            .any(|u| matches!(u.value, ParserValue::Nil))
    );
    assert!(
        data.tables
            .iter()
            .flat_map(|t| t.fields.values().chain(t.indexed.values()))
            .any(|v| matches!(v, ParserValue::NonFinite(_)))
    );
    assert!(
        data.declarations
            .iter()
            .any(|d| d.phase == "literal_declaration" && d.payload.is_none())
    );
    assert!(
        data.declarations
            .iter()
            .any(|d| d.phase == "constructed_final" && d.payload.is_some())
    );
    let encoded = serde_json::to_vec(&data).unwrap();
    let decoded: ModifierParserData = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(data, decoded);
}
#[test]
fn rejects_dangling_references_nils_nonfinite_numbers_and_missing_dictionaries() {
    let original = data();
    for value in [
        ParserValue::Table(ParserTableId(0)),
        ParserValue::Table(ParserTableId(u32::MAX)),
        ParserValue::Callback(ParserCallbackId(u32::MAX)),
        ParserValue::Nil,
        ParserValue::Number(f64::NAN),
    ] {
        let mut data = original.clone();
        data.tables[0].fields.insert("injected".into(), value);
        assert!(data.validate().is_err());
    }
    let mut data = original.clone();
    data.dictionaries.remove(&ParserDictionary::Form);
    assert!(data.validate().is_err());
    let mut data = original.clone();
    data.tables[0]
        .indexed
        .insert(i64::MAX, ParserValue::Boolean(true));
    assert!(data.validate().is_err());
    let mut data = original;
    data.callbacks[0].upvalues.push(ParserUpvalue {
        name: "bad".into(),
        value: ParserValue::Table(ParserTableId(0)),
    });
    assert!(data.validate().is_err());
}
#[test]
fn graph_cycles_are_preserved_without_recursive_validation() {
    let mut data = data();
    data.tables[0]
        .fields
        .insert("cycle".into(), ParserValue::Table(ParserTableId(1)));
    let catalog = ModifierParserCatalog::new(data).unwrap();
    assert_eq!(
        catalog.table(ParserTableId(1)).unwrap().fields["cycle"],
        ParserValue::Table(ParserTableId(1))
    );
}
#[test]
fn rejects_duplicate_and_noncanonical_lua_table_keys() {
    for json in [
        r#"{"fields":{"x":{"kind":"nil"},"x":{"kind":"nil"}},"indexed":{}}"#,
        r#"{"fields":{},"indexed":{"01":{"kind":"nil"}}}"#,
        r#"{"fields":{},"indexed":{"1":{"kind":"nil"},"1":{"kind":"nil"}}}"#,
    ] {
        assert!(serde_json::from_str::<ParserTable>(json).is_err());
    }
}
#[test]
fn injected_catalog_data_remains_independent_of_bundled_definitions() {
    let mut custom = data();
    let id = custom.dictionaries[&ParserDictionary::ModName];
    custom.tables[id.0 as usize - 1].fields.insert(
        "user-defined attribute".into(),
        ParserValue::Text("CustomAttribute".into()),
    );
    let custom = ModifierParserCatalog::new(custom).unwrap();
    assert_eq!(
        custom.exact(ParserDictionary::ModName, "user-defined attribute"),
        Some(&ParserValue::Text("CustomAttribute".into()))
    );
    assert!(
        ModifierParserCatalog::new(data())
            .unwrap()
            .exact(ParserDictionary::ModName, "user-defined attribute")
            .is_none()
    );
}

#[test]
fn ambiguous_name_partition_cannot_drop_or_invent_a_winner() {
    let original = data();
    let key = original
        .dynamic_dependencies
        .gem_for_base_name_ambiguities
        .keys()
        .next()
        .unwrap()
        .clone();
    for mutation in 0..5 {
        let mut data = original.clone();
        let d = &mut data.dynamic_dependencies;
        match mutation {
            0 => {
                d.gem_for_base_name_ambiguities.remove(&key);
            }
            1 => {
                d.gem_for_base_name_ambiguities.get_mut(&key).unwrap().pop();
            }
            2 => {
                let v = d.gem_for_base_name_ambiguities.get_mut(&key).unwrap();
                v.push(v[0].clone());
            }
            3 => {
                d.gem_for_base_name_ambiguities.get_mut(&key).unwrap()[0] = "UnknownGem".into();
            }
            _ => {
                data.tables[d.gem_for_base_name.0 as usize - 1]
                    .fields
                    .insert(
                        key.clone(),
                        ParserValue::Text(d.gem_for_base_name_ambiguities[&key][0].clone()),
                    );
            }
        }
        assert!(data.validate().is_err(), "mutation {mutation}");
    }
}

#[test]
fn aggregate_key_bytes_cannot_bypass_graph_budget() {
    let mut data = data();
    let table = data.tables.first_mut().unwrap();
    for i in 0..5000 {
        table.fields.insert(
            format!("{i:08}{}", "x".repeat(4000)),
            ParserValue::Boolean(true),
        );
    }
    let error = data.validate().unwrap_err().to_string();
    assert!(error.contains("aggregate text"), "{error}");
}
