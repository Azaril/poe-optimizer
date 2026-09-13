use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_data::item_loading::ItemMetadataValue;
use poe_optimizer_engine::modifier_parser::{CompiledModifierParser, ParserError};
use poe_optimizer_import::item_loading::*;
use std::sync::{Arc, OnceLock};
fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn request(text: &str) -> ParseRequest {
    ParseRequest {
        sequence: 0,
        line_index: Some(0),
        origin: None,
        text: text.into(),
        combined: false,
    }
}
#[test]
fn native_structural_parser_preserves_nil_empty_extra_and_local_tag_shape() {
    let mut parser = NativeModifierParserProvider::new(data().modifier_parser());
    assert!(parser.compilation_error().is_none());
    let DependencyResult::Available(unknown) =
        parser.parse_modifier(&request("unrecognized wording"))
    else {
        panic!("source nil result must be available")
    };
    assert!(unknown.modifiers.is_none());
    assert_eq!(unknown.extra.as_deref(), Some("unrecognized wording "));
    let DependencyResult::Available(empty) =
        parser.parse_modifier(&request("20% increased nonexistent"))
    else {
        panic!("source empty result must be available")
    };
    assert!(empty.modifiers.unwrap().is_empty());
    assert!(empty.extra.is_some());
    let DependencyResult::Available(local) =
        parser.parse_modifier(&request("Grants 3 Life per Enemy Hit"))
    else {
        panic!("local form should parse")
    };
    let mods = local.modifiers.unwrap();
    assert!(local.extra.is_none());
    let tag = mods[0].indexed[&1].as_table().unwrap();
    assert_eq!(tag.fields["type"].as_str(), Some("Condition"));
    assert_eq!(tag.fields["var"].as_str(), Some("{Hand}Attack"));
    assert!(!tag.fields.contains_key("tag"));
}
#[test]
fn formatting_and_native_parsing_advance_to_assembly_with_exact_rounded_value() {
    let data = data();
    let parser = NativeModifierParserProvider::new(data.modifier_parser());
    let mut provider = NativeItemLoadProvider::with_dependencies(data, parser);
    let mut machine = ItemLoadMachine::new(data.item_loading());
    machine
        .apply_text(
            "Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\n+17.5 to Strength",
            &mut provider,
        )
        .unwrap();
    assert_eq!(machine.pending().unwrap().kind, DependencyKind::Assembly);
    assert_eq!(machine.state().parser_calls[0].text, "+18 to Strength");
    let modifier = &machine.state().explicit_mod_lines[0].modifiers[0];
    assert_eq!(modifier.fields["name"].as_str(), Some("Str"));
    assert_eq!(modifier.fields["value"].as_f64(), Some(18.0));
}
#[test]
fn pure_tag_metadata_advances_while_nonfinite_and_stateful_outputs_defer() {
    let mut parser = NativeModifierParserProvider::new(data().modifier_parser());
    let DependencyResult::Available(out) =
        parser.parse_modifier(&request("1% increased Damage per 10 maximum Life"))
    else {
        panic!("the pure PerStat tag now reaches finite metadata")
    };
    assert!(out.extra.is_none());
    let rows = out.modifiers.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fields.len(), 5);
    assert_eq!(rows[0].fields["name"].as_str(), Some("Damage"));
    assert_eq!(rows[0].fields["type"].as_str(), Some("INC"));
    assert_eq!(rows[0].fields["value"].as_f64(), Some(1.0));
    assert_eq!(rows[0].fields["flags"].as_f64(), Some(0.0));
    assert_eq!(rows[0].fields["keywordFlags"].as_f64(), Some(0.0));
    assert_eq!(rows[0].indexed.len(), 1);
    let tag = rows[0].indexed[&1].as_table().unwrap();
    assert_eq!(tag.fields.len(), 3);
    assert!(tag.indexed.is_empty());
    assert_eq!(tag.fields["type"].as_str(), Some("PerStat"));
    assert_eq!(tag.fields["stat"].as_str(), Some("Life"));
    assert_eq!(tag.fields["div"].as_f64(), Some(10.0));
    for (text, reason) in [
        (
            "Any number of Poisons from this Weapon can affect a target at the same time",
            "non-finite",
        ),
        (
            "Strength and Dexterity is doubled",
            "shared dictionary mutation",
        ),
    ] {
        let result = parser.parse_modifier(&request(text));
        assert!(
            matches!(&result, DependencyResult::Unavailable(message) if message.contains(reason)),
            "{text}: {result:?}"
        );
    }
}
#[test]
fn native_parser_input_bound_is_a_resource_error_not_missing_support() {
    let mut parser = NativeModifierParserProvider::new(data().modifier_parser());
    assert!(matches!(
        parser.parse_modifier(&request(&"x".repeat(1024 * 1024))),
        DependencyResult::ResourceError(_)
    ));
}

#[test]
fn defence_headers_preserve_loaded_values_before_assembly() {
    for header in [
        "Armour",
        "Evasion Rating",
        "Evasion",
        "Energy Shield",
        "Ward",
        "Runic Ward",
    ] {
        let snapshot = data();
        // Inspect authored headers before local assembly replaces display values.
        let parser = NativeModifierParserProvider::new(snapshot.modifier_parser());
        let mut provider = NativeItemLoadProvider::with_dependencies(snapshot, parser);
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine
            .apply_text(
                &format!(
                    "Rarity: NORMAL\nRusted Greathelm\n{header}: 24\nImplicits: 0\n+17 to Strength"
                ),
                &mut provider,
            )
            .unwrap();
        assert_eq!(
            machine.pending().unwrap().kind,
            DependencyKind::Assembly,
            "{header}"
        );
        assert!(
            !machine.state().retained_fields.contains_key("hidden_specs"),
            "{header}"
        );
        let key = snapshot.item_loading().defence_header_key(header).unwrap();
        assert_eq!(
            machine.state().armour_data.as_ref().unwrap().get(key),
            Some(&ItemNumber::new(24.0))
        );
        assert_eq!(machine.state().parser_calls.len(), 1, "{header}");
        assert_eq!(machine.state().assembly_calls, 2, "{header}");
    }
}

#[test]
fn providers_share_compilation_without_sharing_owned_request_outputs() {
    let compiled = Arc::new(CompiledModifierParser::new(data().modifier_parser()).unwrap());
    let mut left = NativeModifierParserProvider::from_compilation_result(Ok(Arc::clone(&compiled)));
    let mut right = NativeModifierParserProvider::from_compiled(Arc::clone(&compiled));
    let mut fresh = NativeModifierParserProvider::new(data().modifier_parser());
    drop(compiled);
    let input = request("+18 to Strength");
    let DependencyResult::Available(mut first) = left.parse_modifier(&input) else {
        panic!("shared parser must remain usable after its caller's Arc is dropped")
    };
    let expected = serde_json::to_value(fresh.parse_modifier(&input)).unwrap();
    first.modifiers.as_mut().unwrap()[0].fields.insert(
        "name".into(),
        ItemMetadataValue::Text("caller mutation".into()),
    );
    // A failed request and a caller mutation affect neither another provider nor
    // the next request's conversion/output budgets and tables.
    assert!(matches!(
        left.parse_modifier(&request(&"x".repeat(1024 * 1024))),
        DependencyResult::ResourceError(_)
    ));
    assert_eq!(
        serde_json::to_value(right.parse_modifier(&input)).unwrap(),
        expected
    );
    assert_eq!(
        serde_json::to_value(left.parse_modifier(&input)).unwrap(),
        expected
    );
    assert_eq!(
        first.modifiers.unwrap()[0].fields["name"].as_str(),
        Some("caller mutation")
    );
}

#[test]
fn retained_compilation_error_is_reported_only_by_reached_parser_requests() {
    // Constructor transport/classification contract. The engine data-injection
    // test separately constructs a real catalog that produces this compile error.
    let failure = ParserError::ResourceBound("compiled dictionary rows");
    let parser = NativeModifierParserProvider::from_compilation_result(Err(failure.clone()));
    assert_eq!(parser.compilation_error(), Some(&failure));
    let mut provider = NativeItemLoadProvider::with_dependencies(data(), parser);
    let formatted = provider.format_with_trace(&FormatRequest {
        sequence: 0,
        line_index: None,
        text: "plain unchanged line".into(),
        range: ItemNumber::Nil,
        scalar: ItemNumber::Nil,
        corrupted_range: ItemNumber::Nil,
    });
    assert!(
        matches!(formatted.result, DependencyResult::Available(ref text) if text == "plain unchanged line")
    );
    assert!(formatted.precision_parser_calls.is_empty());
    for _ in 0..2 {
        assert!(matches!(
            provider.parse_modifier(&request("+18 to Strength")),
            DependencyResult::ResourceError(ref message) if message.contains("compiled dictionary rows")
        ));
    }
    assert_eq!(provider.dependencies().compilation_error(), Some(&failure));
}
