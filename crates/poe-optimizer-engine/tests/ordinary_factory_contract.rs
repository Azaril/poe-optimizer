//! Injected-data, call ordering and resource contracts for ordinary factories.
use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::lua_pattern::{MatchBudget, MatchLimits, PatternError};
use poe_optimizer_engine::modifier_parser::{
    CompiledModifierParser, ModifierTable, ModifierValue as V, ParseOutcome, ParserError,
};
use poe_optimizer_engine::modifier_scan::ScanError;
use std::sync::{Arc, OnceLock};
type E = ParserFactoryExpr;
type F = ParserFactoryField;
type L = ParserFactoryLiteral;
type D = ParserDictionary;
fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn program(data: &mut ModifierParserData, id: ParserCallbackId) -> &mut ParserPureFactory {
    let ParserFactoryDisposition::Pure(factory) = data.factories.get_mut(&id).unwrap() else {
        panic!("pure factory")
    };
    factory
}
fn dictionary(data: &mut ModifierParserData, kind: D) -> &mut ParserTable {
    let id = data.dictionaries[&kind];
    &mut data.tables[id.0 as usize - 1]
}
fn named(key: &str, value: E) -> F {
    F::Named {
        key: key.into(),
        value,
    }
}
fn text(value: &str) -> E {
    E::Literal(L::Text(value.into()))
}
fn metadata() -> E {
    E::Table(vec![named(
        "tag",
        E::Table(vec![
            named("type", text("CallerTag")),
            named("value", E::Argument(0)),
            named("raw", E::Argument(1)),
            named("tail", E::Argument(2)),
            named("missing", E::Argument(5)),
        ]),
    )])
}
fn fixture() -> (ModifierParserData, ParserCallbackId, ParserCallbackId) {
    let mut data = snapshot().modifier_parser().data().clone();
    // Authored legacy fixture changes do not retain original program admissions.
    data.programs = Default::default();
    let select = |kind| {
        data.tables[data.dictionaries[&kind].0 as usize - 1]
            .fields
            .values()
            .find_map(|v| match v {
                ParserValue::Callback(id)
                    if matches!(
                        data.factories.get(id),
                        Some(ParserFactoryDisposition::Pure(_))
                    ) =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .unwrap()
    };
    let prefix = select(D::PreFlag);
    let tag = select(D::ModTag);
    for id in data.dictionaries.values() {
        data.tables[id.0 as usize - 1] = ParserTable::default();
    }
    dictionary(&mut data, D::Form)
        .fields
        .insert("^(%d+) to ".into(), ParserValue::Text("BASE".into()));
    dictionary(&mut data, D::ModName)
        .fields
        .insert("value".into(), ParserValue::Text("CallerStat".into()));
    for id in [prefix, tag] {
        let factory = program(&mut data, id);
        factory.provenance.constructor = None;
        factory.parameter_count = 6;
        factory.body = metadata();
    }
    (data, prefix, tag)
}
fn compile(data: ModifierParserData) -> CompiledModifierParser {
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn parse(parser: &CompiledModifierParser, input: &[u8]) -> Result<ParseOutcome, ParserError> {
    parser.parse(input, &mut MatchBudget::default())
}
fn first_modifier(out: &ParseOutcome) -> &ModifierTable {
    assert!(out.extra.is_none(), "unexpected remainder: {:?}", out.extra);
    out.modifiers
        .as_ref()
        .unwrap()
        .indexed_value(1)
        .as_table()
        .unwrap()
}
#[test]
fn prefix_raw_and_both_tag_argument_vectors_preserve_positions() {
    let (mut data, prefix, tag) = fixture();
    dictionary(&mut data, D::PreFlag)
        .fields
        .insert("^pre ([^ ]+) ".into(), ParserValue::Callback(prefix));
    for name in ["first", "second"] {
        dictionary(&mut data, D::ModTag).fields.insert(
            format!("{name} ([^ ]+) ([^ ]+)"),
            ParserValue::Callback(tag),
        );
    }
    let parser = compile(data);
    let out = parse(&parser, b"pre 007 7 to value first x1 A second 12 B").unwrap();
    let row = first_modifier(&out);
    let prefix = row.indexed_value(1).as_table().unwrap();
    assert_eq!(prefix.field("value"), &V::Bytes(b"007".to_vec()));
    assert_eq!(prefix.field("raw"), &V::Nil);
    let first = row.indexed_value(2).as_table().unwrap();
    assert_eq!(first.field("value"), &V::Nil);
    assert_eq!(first.field("raw"), &V::Bytes(b"x1".to_vec()));
    assert_eq!(first.field("tail"), &V::Bytes(b"a".to_vec()));
    let second = row.indexed_value(3).as_table().unwrap();
    assert_eq!(second.field("value"), &V::Number(12.0));
    assert_eq!(second.field("raw"), &V::Bytes(b"12".to_vec()));
    assert_eq!(second.field("tail"), &V::Bytes(b"b".to_vec()));
    for tag in [prefix, first, second] {
        assert_eq!(tag.field("missing"), &V::Nil);
    }
}
#[test]
fn injected_pattern_is_lazy_and_uses_match_semantics_before_dispatch() {
    let (mut data, prefix, tag) = fixture();
    dictionary(&mut data, D::PreFlag)
        .fields
        .insert("^pre ([^ ]+) ".into(), ParserValue::Callback(prefix));
    data.policy.tag_capture_numeric_pattern = "%".into();
    let no_tag = compile(data.clone());
    assert!(parse(&no_tag, b"pre 007 7 to value").is_ok());
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("tag ([^ ]+)".into(), ParserValue::Boolean(true));
    assert!(parse(&compile(data.clone()), b"7 to value tag abc").is_ok());
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("tag ([^ ]+)".into(), ParserValue::Callback(tag));
    data.factories.insert(
        tag,
        ParserFactoryDisposition::Unsupported {
            reason: "Caller pending helper".into(),
        },
    );
    assert!(matches!(
        parse(&compile(data.clone()), b"7 to value tag abc"),
        Err(ParserError::Scan(ScanError::Pattern(PatternError::Source(
            _
        ))))
    ));
    data.policy.tag_capture_numeric_pattern = "^%d+$".into();
    assert!(
        matches!(parse(&compile(data), b"7 to value tag abc"), Err(ParserError::Deferred { stage: "modifier tag callback", callback: Some(actual) }) if actual == tag)
    );
}
#[test]
fn missing_and_position_capture_errors_precede_unsupported_body_and_pattern() {
    for pattern in ["tag", "tag()"] {
        let (mut data, _, tag) = fixture();
        data.policy.tag_capture_numeric_pattern = "%".into();
        data.factories.insert(
            tag,
            ParserFactoryDisposition::Unsupported {
                reason: "Caller pending".into(),
            },
        );
        dictionary(&mut data, D::ModTag)
            .fields
            .insert(pattern.into(), ParserValue::Callback(tag));
        let parser = compile(data);
        assert!(matches!(
            parse(&parser, b"7 to value tag"),
            Err(ParserError::SourceError(_))
        ));
        // A missing form stops before either tag method lookup.
        assert!(parse(&parser, b"unknown tag").is_ok());
    }
}
#[test]
fn first_tag_nil_skips_second_callback_precheck_but_empty_table_reaches_it() {
    let (mut data, prefix, tag) = fixture();
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("first ([^ ]+)".into(), ParserValue::Callback(tag));
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("second".into(), ParserValue::Callback(prefix));
    program(&mut data, tag).body = E::Literal(L::Nil);
    let skipped = parse(&compile(data.clone()), b"7 to value first x second").unwrap();
    assert!(skipped.extra.is_some());
    program(&mut data, tag).body = E::Table(vec![]);
    assert!(matches!(
        parse(&compile(data), b"7 to value first x second"),
        Err(ParserError::SourceError(_))
    ));
}
#[test]
fn custom_precheck_patterns_and_shared_catalogs_do_not_leak_between_requests() {
    let (mut data, _, tag) = fixture();
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("tag ([^ ]+)".into(), ParserValue::Callback(tag));
    data.policy.tag_capture_numeric_pattern = "^%d+$".into();
    let text_parser = Arc::new(compile(data.clone()));
    data.policy.tag_capture_numeric_pattern = String::new();
    let numeric_parser = Arc::new(compile(data));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let text_parser = text_parser.clone();
            let numeric_parser = numeric_parser.clone();
            scope.spawn(move || {
                for _ in 0..16 {
                    let raw = parse(&text_parser, b"7 to value tag x1").unwrap();
                    let numeric = parse(&numeric_parser, b"7 to value tag x1").unwrap();
                    assert_eq!(
                        first_modifier(&raw)
                            .indexed_value(1)
                            .as_table()
                            .unwrap()
                            .field("value"),
                        &V::Bytes(b"x1".to_vec())
                    );
                    assert_eq!(
                        first_modifier(&numeric)
                            .indexed_value(1)
                            .as_table()
                            .unwrap()
                            .field("value"),
                        &V::Nil
                    );
                }
            });
        }
    });
}
#[test]
fn mandatory_tag_precheck_is_charged_even_when_the_factory_ignores_captures() {
    let (mut data, _, tag) = fixture();
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("tag (.*)".into(), ParserValue::Boolean(true));
    let static_parser = compile(data.clone());
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("tag (.*)".into(), ParserValue::Callback(tag));
    program(&mut data, tag).body = E::Table(vec![]);
    let parser = compile(data);
    let line = format!("7 to value tag {}", "a".repeat(8192));
    let mut static_budget = MatchBudget::default();
    static_parser
        .parse(line.as_bytes(), &mut static_budget)
        .unwrap();
    let mut callback_budget = MatchBudget::default();
    parser.parse(line.as_bytes(), &mut callback_budget).unwrap();
    assert!(callback_budget.steps_used() > static_budget.steps_used() + 8192);
    let mut bounded = MatchBudget::new(MatchLimits {
        max_steps: static_budget.steps_used(),
        ..MatchLimits::default()
    });
    assert!(matches!(
        parser.parse(line.as_bytes(), &mut bounded),
        Err(ParserError::Scan(ScanError::Pattern(
            PatternError::Resource(_)
        )))
    ));
}
#[test]
fn repeated_raw_capture_output_is_bounded_before_a_later_body_error() {
    let (mut data, _, tag) = fixture();
    dictionary(&mut data, D::ModTag)
        .fields
        .insert("tag (.*)".into(), ParserValue::Callback(tag));
    let mut fields = vec![F::List(E::Argument(1)); 1200];
    fields.push(F::List(E::Negate(Box::new(E::Literal(L::Boolean(false))))));
    program(&mut data, tag).body = E::Table(fields);
    let parser = compile(data);
    let line = format!("7 to value tag {}", "a".repeat(8192));
    assert!(matches!(
        parse(&parser, line.as_bytes()),
        Err(ParserError::ResourceBound(_))
    ));
}
