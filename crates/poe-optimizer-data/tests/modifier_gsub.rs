use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
fn fixture(
    pattern: &str,
    replacement: ParserFactoryReplacement,
) -> (ModifierParserData, ParserCallbackId) {
    let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
    data.programs = Default::default();
    let owner = *data
        .factories
        .iter()
        .find(|(_, f)| matches!(f, ParserFactoryDisposition::Pure(_)))
        .unwrap()
        .0;
    let ParserFactoryDisposition::Pure(factory) = data.factories.get_mut(&owner).unwrap() else {
        panic!()
    };
    factory.provenance.constructor = None;
    factory.body = ParserFactoryExpr::Table(vec![ParserFactoryField::Named {
        key: "result".into(),
        value: ParserFactoryExpr::Gsub {
            value: Box::new(ParserFactoryExpr::Literal(ParserFactoryLiteral::Text(
                "caller".into(),
            ))),
            pattern: pattern.into(),
            replacement,
        },
    }]);
    (data, owner)
}
#[test]
fn gsub_patterns_remain_lazy_but_all_injected_text_is_bounded() {
    for pattern in ["", "[", "()(.)", "^z[", "\0"] {
        let (data, _) = fixture(pattern, ParserFactoryReplacement::Text("%2".into()));
        ModifierParserCatalog::new(data).unwrap();
    }
    for (pattern, replacement) in [
        ("x".repeat(4097), String::new()),
        (String::new(), "x".repeat(4097)),
    ] {
        let (data, _) = fixture(&pattern, ParserFactoryReplacement::Text(replacement));
        assert!(ModifierParserCatalog::new(data).is_err());
    }
}
#[test]
fn gsub_upper_rejects_shadowed_string_and_unknown_wire_replacements() {
    let (mut data, owner) = fixture(".", ParserFactoryReplacement::StringUpper);
    data.callbacks[owner.0 as usize - 1]
        .upvalues
        .push(ParserUpvalue {
            name: "string".into(),
            value: ParserValue::Nil,
        });
    assert!(ModifierParserCatalog::new(data).is_err());
    for value in [
        r#"{"kind":"callback","value":1}"#,
        r#"{"kind":"string_upper","extra":1}"#,
    ] {
        assert!(serde_json::from_str::<ParserFactoryReplacement>(value).is_err());
    }
    let (data, _) = fixture(".", ParserFactoryReplacement::StringUpper);
    ModifierParserCatalog::new(data).unwrap();
}
