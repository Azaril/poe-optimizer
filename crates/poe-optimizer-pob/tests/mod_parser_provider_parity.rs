//! Tests at the public finite item-metadata provider seam, not only engine DTOs.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
// This established source transport also provides full item-state comparison;
// individual provider tests use only a subset of its unrelated helper methods.
#[allow(dead_code)]
#[path = "support/mod_parser_scan_source.rs"]
mod dictionaries;
#[allow(dead_code)]
#[path = "support/item_formatter_runtime.rs"]
mod oracle;
#[allow(dead_code)]
#[path = "support/item_loading_native.rs"]
mod reference;
use mlua::{Table, Value};
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    item_loading::{ItemMetadataTable, ItemMetadataValue},
};
use poe_optimizer_import::item_loading::*;
use std::collections::{BTreeMap, BTreeSet};
fn request(text: &str) -> ParseRequest {
    ParseRequest {
        sequence: 0,
        line_index: Some(0),
        origin: None,
        text: text.into(),
        combined: false,
    }
}
fn canonical_value(value: &ItemMetadataValue) -> serde_json::Value {
    match value {
        ItemMetadataValue::Number(n) => {
            serde_json::json!(["number", format!("{:016x}", n.to_bits())])
        }
        ItemMetadataValue::Boolean(v) => serde_json::json!(["boolean", v]),
        ItemMetadataValue::Text(v) => serde_json::json!(["text", v]),
        ItemMetadataValue::Array(rows) => serde_json::json!([
            "array",
            rows.iter().map(canonical_value).collect::<Vec<_>>()
        ]),
        ItemMetadataValue::Table(table) => canonical_table(table),
        ItemMetadataValue::Callback(callback) => serde_json::json!(["callback", callback]),
    }
}
fn canonical_table(table: &ItemMetadataTable) -> serde_json::Value {
    serde_json::json!([
        "table",
        table
            .fields
            .iter()
            .map(|(k, v)| (k, canonical_value(v)))
            .collect::<BTreeMap<_, _>>(),
        table
            .indexed
            .iter()
            .map(|(k, v)| (k, canonical_value(v)))
            .collect::<BTreeMap<_, _>>()
    ])
}
fn canonical_outcome(outcome: &ParseOutcome) -> serde_json::Value {
    serde_json::json!({"modifiers":outcome.modifiers.as_ref().map(|rows|rows.iter().map(canonical_table).collect::<Vec<_>>()),"extra":outcome.extra})
}
fn finite_source(value: &Value) -> bool {
    match value {
        Value::Nil | Value::Boolean(_) | Value::Integer(_) => true,
        Value::Number(n) => n.is_finite(),
        Value::String(s) => s.to_str().is_ok(),
        Value::Table(t) => t.clone().pairs::<Value, Value>().all(|row| {
            let (k, v) = row.unwrap();
            finite_source(&k) && finite_source(&v)
        }),
        _ => false,
    }
}
fn original_parse(
    source: &runtime::Oracle,
    request: &ParseRequest,
) -> Result<(Option<Table>, Option<String>), mlua::Error> {
    source
        .lua
        .globals()
        .get::<mlua::Function>("item_loading_parse_dependency")
        .unwrap()
        .call((request.text.as_str(), request.combined))
}
fn compare_available(
    actual: &ParseOutcome,
    modifiers: Option<Table>,
    extra: Option<String>,
    label: &str,
) {
    let expected = ParseOutcome {
        modifiers: modifiers.map(|table| {
            table
                .sequence_values::<Table>()
                .map(|row| reference::metadata(row.unwrap()))
                .collect()
        }),
        extra,
    };
    assert_eq!(
        canonical_outcome(actual),
        canonical_outcome(&expected),
        "{label}"
    );
}
#[test]
fn native_parser_metadata_preserves_dense_array_and_tag_shape() {
    let snapshot = bundled_snapshot().unwrap();
    let source = runtime::Oracle::new();
    assert_eq!(
        source
            .parse("Rarity: Normal\nRusted Greathelm\nQuality: 0")
            .get::<String>("baseName")
            .unwrap(),
        "Rusted Greathelm"
    );
    let mut provider = NativeModifierParserProvider::new(snapshot.modifier_parser());
    let req = request("+7 to strength and dexterity while dual wielding or holding a shield");
    let DependencyResult::Available(actual) = provider.parse_modifier(&req) else {
        panic!("ordinary varList parse unexpectedly deferred")
    };
    let (mods, extra) = original_parse(&source, &req).unwrap();
    compare_available(&actual, mods, extra, &req.text);
    for modifier in actual.modifiers.unwrap() {
        let ItemMetadataValue::Table(tag) = &modifier.indexed[&1] else {
            panic!("tag must be a mixed/string table")
        };
        assert!(matches!(&tag.fields["varList"],ItemMetadataValue::Array(rows)if rows.len()==2));
    }
}
#[test]
fn broad_native_parser_provider_results_match_original_metadata_or_explicitly_defer() {
    let snapshot = bundled_snapshot().unwrap();
    let source = dictionaries::ScanSource::new();
    let mut cases = BTreeSet::new();
    for row in source
        .tables
        .get::<Table>("modNameList")
        .unwrap()
        .pairs::<String, Value>()
    {
        let name = row.unwrap().0;
        for (prefix, suffix) in [
            ("+7 to ", ""),
            ("13% increased ", " while dual wielding or holding a shield"),
        ] {
            cases.insert(format!("{prefix}{name}{suffix}"));
        }
    }
    for text in [
        "",
        "no modifier",
        "13% increased no actual stat",
        "Grants 3 Life per Enemy Hit",
        "-0 to maximum Life",
        "+1..5 to maximum Life",
        "Minions Regenerate 2.5% of Life per second",
        "Immunity to Freeze and Shock",
        "Added Small Passive Skills also grant: unchanged raw text",
        "Strength and Dexterity is doubled",
    ] {
        cases.insert(text.into());
    }
    let mut provider = NativeModifierParserProvider::new(snapshot.modifier_parser());
    let mut reference_provider = reference::OriginalDependencies::new(&source.source);
    let mut paired = 0;
    let mut deferred = 0;
    let mut failures = vec![];
    for text in &cases {
        let req = request(text);
        match provider.parse_modifier(&req) {
            DependencyResult::Available(actual) => {
                let (mods, extra) = original_parse(&source.source, &req).unwrap();
                assert!(
                    mods.as_ref()
                        .is_none_or(|m| finite_source(&Value::Table(m.clone()))),
                    "available provider discarded an unrepresentable source value: {text}"
                );
                let DependencyResult::Available(expected) = reference_provider.parse_modifier(&req)
                else {
                    unreachable!()
                };
                assert_eq!(expected.extra, extra);
                if canonical_outcome(&actual) != canonical_outcome(&expected) {
                    if failures.len() < 8 {
                        eprintln!(
                            "Provider metadata mismatch {text}: expected={} actual={}",
                            canonical_outcome(&expected),
                            canonical_outcome(&actual)
                        );
                    }
                    failures.push(text);
                }
                paired += 1;
            }
            DependencyResult::Unavailable(_) => deferred += 1,
            DependencyResult::SourceError(_) => assert!(
                original_parse(&source.source, &req).is_err(),
                "unexpected native source error for {text}"
            ),
            DependencyResult::ResourceError(error) => {
                panic!("unexpected provider resource error for {text}: {error}")
            }
        }
    }
    eprintln!(
        "Metadata provider {} cases, {paired} paired, {deferred} deferred, {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(paired >= 1500);
    assert!(failures.is_empty());
}

#[test]
fn native_provider_preserves_nil_empty_and_remainder_without_defaulting() {
    let snapshot = bundled_snapshot().unwrap();
    let source = runtime::Oracle::new();
    let mut provider = NativeModifierParserProvider::new(snapshot.modifier_parser());
    for (text, count, extra) in [
        ("", None, Some(" ")),
        (
            "not an actual modifier",
            None,
            Some("not an actual modifier "),
        ),
        ("20% increased not a stat", Some(0), Some(" not a stat ")),
        ("+2 to maximum Life", Some(1), None),
    ] {
        for combined in [false, true] {
            let mut req = request(text);
            req.combined = combined;
            let DependencyResult::Available(actual) = provider.parse_modifier(&req) else {
                panic!("representable parser result unexpectedly deferred: {text}")
            };
            let (mods, remainder) = original_parse(&source, &req).unwrap();
            compare_available(&actual, mods, remainder, text);
            assert_eq!(actual.modifiers.as_ref().map(Vec::len), count);
            assert_eq!(actual.extra.as_deref(), extra);
        }
    }
}
fn contains_value(value: Value, predicate: fn(&Value) -> bool) -> bool {
    if predicate(&value) {
        return true;
    }
    match value {
        Value::Table(table) => table.pairs::<Value, Value>().any(|row| {
            let (key, value) = row.unwrap();
            contains_value(key, predicate) || contains_value(value, predicate)
        }),
        _ => false,
    }
}
#[test]
fn nonfinite_and_captured_function_outputs_explicitly_defer_at_finite_metadata_boundary() {
    let snapshot = bundled_snapshot().unwrap();
    let source = runtime::Oracle::new();
    let mut provider = NativeModifierParserProvider::new(snapshot.modifier_parser());
    for (text, fragment, kind) in [
        (
            "Any number of poisons from this weapon can affect a target at the same time",
            "non-finite",
            "infinity",
        ),
        (
            "Dexterity from Passives in Radius is Transformed to Intelligence",
            "captured-state",
            "function",
        ),
    ] {
        let req = request(text);
        let (mods, extra) = original_parse(&source, &req).unwrap();
        assert!(extra.is_none());
        let original = Value::Table(mods.unwrap());
        let observed = match kind {
            "infinity" => contains_value(
                original,
                |v| matches!(v, Value::Number(n) if n.is_infinite()),
            ),
            "function" => contains_value(original, |v| matches!(v, Value::Function(_))),
            _ => unreachable!(),
        };
        assert!(observed, "original {kind} witness missing: {text}");
        let DependencyResult::Unavailable(reason) = provider.parse_modifier(&req) else {
            panic!("provider accepted unrepresentable original {kind}: {text}")
        };
        assert!(reason.contains(fragment), "{reason}");
    }
}

// Same source-value observation contract required by the reusable item-state comparator.
fn canonical(value: Value) -> serde_json::Value {
    canonical_inner(value, 0)
}
fn canonical_inner(value: Value, depth: usize) -> serde_json::Value {
    assert!(
        depth < 32,
        "unexpected observed nesting depth {depth}, {value:?}"
    );
    match value {
        Value::Nil => serde_json::json!(["nil"]),
        Value::Boolean(v) => serde_json::json!(["boolean", v]),
        Value::Integer(v) => serde_json::json!(["number", (v as f64).to_bits().to_string()]),
        Value::Number(v) => serde_json::json!(["number", v.to_bits().to_string()]),
        Value::String(v) => serde_json::json!(["string", v.to_str().unwrap().to_owned()]),
        Value::Table(t) => {
            let mut rows = t
                .pairs::<Value, Value>()
                .map(|r| {
                    let (k, v) = r.unwrap();
                    (canonical_inner(k, depth + 1), canonical_inner(v, depth + 1))
                })
                .collect::<Vec<_>>();
            rows.sort_by_key(|(k, _)| k.to_string());
            serde_json::json!(["table", rows])
        }
        Value::Function(f) => {
            let info = f.info();
            serde_json::json!([
                "function_descriptor_only",
                info.source,
                info.line_defined,
                info.last_line_defined
            ])
        }
        other => panic!("unsupported observed value {other:?}"),
    }
}

struct TracedNative<'a> {
    inner: BuiltinItemLoadProvider<'a>,
    parser: Vec<(ParseRequest, DependencyResult<ParseOutcome>)>,
    formats: Vec<(FormatRequest, FormatOutcome)>,
    assembly: Vec<AssemblyRequest>,
    uniques: Vec<(UniqueRequest, DependencyResult<Option<UniqueOutcome>>)>,
}
impl<'a> TracedNative<'a> {
    fn new(snapshot: &'a poe_optimizer_data::game_data::GameDataSnapshot) -> Self {
        Self {
            inner: BuiltinItemLoadProvider::new(snapshot),
            parser: vec![],
            formats: vec![],
            assembly: vec![],
            uniques: vec![],
        }
    }
}
impl ItemLoadProvider for TracedNative<'_> {
    fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        let result = self.inner.parse_modifier(request);
        self.parser.push((request.clone(), result.clone()));
        result
    }
    fn format_with_trace(&mut self, request: &FormatRequest) -> FormatOutcome {
        let result = self.inner.format_with_trace(request);
        self.formats.push((request.clone(), result.clone()));
        result
    }
    fn lookup_unique(
        &mut self,
        request: &UniqueRequest,
    ) -> DependencyResult<Option<UniqueOutcome>> {
        let result = self.inner.lookup_unique(request);
        self.uniques.push((request.clone(), result.clone()));
        result
    }
    fn assemble(&mut self, request: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        self.assembly.push(request.clone());
        self.inner.assemble(request)
    }
    fn catalyst_scaling(
        &self,
    ) -> Option<&poe_optimizer_data::item_scalability::CatalystScalingData> {
        self.inner.catalyst_scaling()
    }
}
fn events(source: &Table, kind: &str) -> Vec<Table> {
    source
        .get::<Table>("events")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|event| event.get::<String>("kind").unwrap() == kind)
        .collect()
}
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn verify_trace(
    provider: &TracedNative<'_>,
    parse: &Table,
    loaded: &Table,
    oracle: &oracle::FormatterOracle,
) -> (usize, usize, usize) {
    let consumed = parse.get::<String>("raw").unwrap();
    let mut original = reference::OriginalDependencies::new(&oracle.source);
    for (request, actual) in &provider.uniques {
        let expected = original.lookup_unique(request);
        match (actual, expected) {
            (DependencyResult::Available(None), DependencyResult::Available(None)) => {}
            (
                DependencyResult::Available(Some(actual)),
                DependencyResult::Available(Some(expected)),
            ) => {
                for (name, a, b) in [
                    (
                        "natural unique requirement",
                        actual.natural_level,
                        expected.natural_level,
                    ),
                    ("equipped unique requirement", actual.level, expected.level),
                ] {
                    assert_eq!(a.is_some(), b.is_some(), "{name}: {request:?}");
                    if let (Some(a), Some(b)) = (a, b) {
                        reference::assert_number(a, b, name);
                    }
                }
            }
            (actual, expected) => {
                panic!("unique lookup mismatch for {request:?}: {actual:?} != {expected:?}")
            }
        }
    }

    let expected = parse
        .get::<Table>("calls")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|call| call.get::<String>("phase").unwrap() == "parse_raw")
        .collect::<Vec<_>>();
    let mut actual = provider
        .parser
        .iter()
        .map(|(r, v)| (r, v))
        .chain(provider.formats.iter().flat_map(|(_, f)| {
            f.precision_parser_calls
                .iter()
                .map(|c| (&c.request, &c.result))
        }))
        .collect::<Vec<_>>();
    actual.sort_by_key(|(request, _)| request.sequence);
    assert!(
        actual.len() <= expected.len(),
        "native parser sequence extends past source for {consumed}"
    );
    for ((request, result), expected) in actual.iter().zip(&expected) {
        assert_eq!(
            request.text,
            expected.get::<String>("text").unwrap(),
            "parser prefix for {consumed}"
        );
        assert_eq!(request.combined, expected.get::<bool>("combined").unwrap());
        if let DependencyResult::Available(actual) = result {
            let (mods, extra) = original_parse(&oracle.source, request).unwrap();
            compare_available(actual, mods, extra, &request.text);
        }
    }
    let formats = events(loaded, "format")
        .into_iter()
        .filter(|event| {
            event
                .get::<Table>("before")
                .unwrap()
                .get::<String>("raw")
                .unwrap()
                == consumed
        })
        .collect::<Vec<_>>();
    assert!(
        provider.formats.len() <= formats.len(),
        "native format prefix extends past source"
    );
    let mut sequence = provider
        .formats
        .iter()
        .map(|(r, _)| r.sequence)
        .chain(actual.iter().map(|(r, _)| r.sequence))
        .collect::<Vec<_>>();
    sequence.sort();
    assert_eq!(sequence, (0..sequence.len()).collect::<Vec<_>>());
    let mut precision = 0;
    for ((request, result), expected) in provider.formats.iter().zip(formats) {
        assert_eq!(request.text, expected.get::<String>("text").unwrap());
        for (key, actual) in [
            ("range", request.range),
            ("scalar", request.scalar),
            ("corrupted", request.corrupted_range),
        ] {
            reference::assert_number(actual, reference::number(expected.get(key).unwrap()), key);
        }
        let value = |number: ItemNumber| number.value().map_or(Value::Nil, Value::Number);
        let source = oracle.observe(
            "range",
            &[
                oracle.text(&request.text),
                value(request.range),
                value(request.scalar),
                value(request.corrupted_range),
            ],
        );
        let calls = source
            .get::<Table>("calls")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(
            result.precision_parser_calls.len(),
            calls.len(),
            "precision callback count"
        );
        for (actual, expected) in result.precision_parser_calls.iter().zip(calls) {
            assert_eq!(actual.request.text, expected.get::<String>("text").unwrap());
            assert_eq!(
                actual.request.combined,
                expected.get::<bool>("combined").unwrap()
            );
            precision += 1;
        }
        match &result.result {
            DependencyResult::Available(text) => {
                assert!(source.get::<bool>("ok").unwrap());
                assert_eq!(*text, source.get::<String>("value").unwrap());
            }
            DependencyResult::Unavailable(_) => assert!(
                result
                    .precision_parser_calls
                    .iter()
                    .any(|call| matches!(call.result, DependencyResult::Unavailable(_))),
                "unexplained formatting deferral"
            ),
            DependencyResult::SourceError(_) => assert!(!source.get::<bool>("ok").unwrap()),
            DependencyResult::ResourceError(error) => panic!("unexpected formatter bound: {error}"),
        }
    }
    (actual.len(), provider.formats.len(), precision)
}
#[test]
fn combined_native_parser_formatter_preserve_complete_preassembly_state_and_callback_order() {
    let snapshot = bundled_snapshot().unwrap();
    let oracle = oracle::FormatterOracle::new();
    let mut counts = (0, 0, 0, 0);
    for newline in ["\n", "\r\n"] {
        for (header, body) in [
            (
                "",
                "+7 to strength and dexterity while dual wielding or holding a shield",
            ),
            ("", "10% increased\nArmour"),
            ("", "unknown unrelated text\n+10 to maximum Life"),
            ("", "Grants 3 Life per Enemy Hit"),
            (
                "",
                "{enchant}+10 to Strength\n{implicit}+11 to Dexterity\n+12 to Intelligence",
            ),
            ("", "{range:0.25}+(10-20) to maximum Life"),
            (
                "Catalyst: Flesh",
                "{tags:life}{range:0.25}+(10-20) to maximum Life",
            ),
            (
                "Quality (Life Modifiers): 30%",
                "{tags:life}+17.5 to maximum Life",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life - Unscalable Value",
            ),
            ("", "Minions Regenerate (1.25-2.75)% of Life per second"),
            ("", "(1.23-3.45)% increased Life and Mana Regeneration Rate"),
            (
                "",
                "+10 to maximum Life\n13% increased damage while wielding a mace",
            ),
        ] {
            let raw=format!("Rarity: Rare\nProvider test amulet\nAmber Amulet\nItem Level: 60\n{header}\nImplicits: 0\n{body}").replace('\n',newline);
            let loaded = oracle.source.load(
                &format!("<Items><Item id=\"7\">{}</Item></Items>", escaped(&raw)),
                false,
            );
            assert!(loaded.get::<bool>("ok").unwrap(), "source item load {raw}");
            let parse = events(&loaded, "parse_raw")
                .into_iter()
                .find(|event| !event.get::<String>("raw").unwrap().is_empty())
                .unwrap();
            let consumed = parse.get::<String>("raw").unwrap();
            let before = events(&loaded, "build_mod_list")
                .into_iter()
                .map(|event| event.get::<Table>("before").unwrap())
                .find(|before| before.get::<String>("raw").unwrap() == consumed)
                .unwrap();
            let mut provider = TracedNative::new(&snapshot);
            let mut machine = ItemLoadMachine::new(snapshot.item_loading());
            machine.set_xml_attributes(&source_attributes(&parse));
            machine.apply_text(&consumed, &mut provider).unwrap();
            assert_eq!(
                machine.pending().map(|p| p.kind),
                Some(DependencyKind::Assembly),
                "{raw}: {:?}",
                machine.pending()
            );
            reference::compare_state(machine.state(), &before);
            let (parses, formats, precision) = verify_trace(&provider, &parse, &loaded, &oracle);
            counts.0 += 1;
            counts.1 += parses;
            counts.2 += formats;
            counts.3 += precision;
        }
    }
    assert_eq!(counts.0, 24);
    eprintln!(
        "Combined provider preassembly {} items, {} parser calls, {} formats, {} precision parses",
        counts.0, counts.1, counts.2, counts.3
    );
}
#[test]
fn all_116_corpus_items_preserve_native_formatter_parser_progress_prefixes() {
    use sha2::{Digest, Sha256};
    let snapshot = bundled_snapshot().unwrap();
    let oracle = oracle::FormatterOracle::new();
    let directory = runtime::repository().join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    let mut counts = (0, 0, 0, 0, 0, 0);
    let mut pending = BTreeMap::<String, usize>::new();
    let mut state_failures = vec![];
    let mut affix_evidence = vec![];
    for row in index["builds"].as_array().unwrap() {
        let path = directory.join(row["xml"].as_str().unwrap());
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            row["xml_sha256"].as_str().unwrap()
        );
        let loaded = oracle
            .source
            .load(std::str::from_utf8(&bytes).unwrap(), false);
        assert!(loaded.get::<bool>("ok").unwrap());
        for parse in events(&loaded, "parse_raw")
            .into_iter()
            .filter(|event| !event.get::<String>("raw").unwrap().is_empty())
        {
            let raw = parse.get::<String>("raw").unwrap();
            let mut provider = TracedNative::new(&snapshot);
            let mut machine = ItemLoadMachine::new(snapshot.item_loading());
            machine.set_xml_attributes(&source_attributes(&parse));
            machine.apply_text(&raw, &mut provider).unwrap();
            assert_eq!(
                machine.status(),
                ItemLoadStatus::Pending,
                "corpus item must explicitly stop before unavailable assembly"
            );
            let kind = machine.pending().unwrap().kind;
            *pending.entry(format!("{kind:?}")).or_default() += 1;
            let (parses, formats, precision) = verify_trace(&provider, &parse, &loaded, &oracle);
            counts.0 += 1;
            counts.1 += parses;
            counts.2 += formats;
            counts.3 += precision;
            let mut affix_boundary: Option<(&str, Table)> = None;
            let mut full_state_boundary = None;
            let mut full_state_paired = false;
            if kind == DependencyKind::Assembly && !provider.assembly.is_empty() {
                assert_eq!(provider.assembly.len(), 1);
                let before = events(&loaded, "build_mod_list")
                    .into_iter()
                    .map(|event| event.get::<Table>("before").unwrap())
                    .find(|before| before.get::<String>("raw").unwrap() == raw)
                    .unwrap();
                affix_boundary = Some(("complete_preassembly", before.clone()));
                full_state_boundary = Some("complete_preassembly");
                let compared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    reference::compare_state(machine.state(), &before);
                }));
                if let Err(error) = compared {
                    let detail = error
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| error.downcast_ref::<&str>().copied())
                        .unwrap_or("unknown panic");
                    eprintln!(
                        "Corpus preassembly mismatch {} id {:?}: {detail}\n{raw}",
                        path.display(),
                        source_attributes(&parse).get("id")
                    );
                    state_failures.push(format!("{}: {detail}", path.display()));
                } else {
                    counts.4 += 1;
                    full_state_paired = true;
                }
            }
            if kind == DependencyKind::ModifierParser {
                let call = parse
                    .get::<Table>("calls")
                    .unwrap()
                    .sequence_values::<Table>()
                    .map(Result::unwrap)
                    .filter(|call| call.get::<String>("phase").unwrap() == "parse_raw")
                    .nth(
                        parses
                            .checked_sub(1)
                            .expect("unavailable parser was attempted"),
                    )
                    .expect("matching original parser frontier");
                let before = call.get::<Table>("before").unwrap();
                affix_boundary = Some(("before_unavailable_parser", before.clone()));
                full_state_boundary = Some("before_unavailable_parser");
                let compared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    reference::compare_state(machine.state(), &before);
                }));
                if let Err(error) = compared {
                    let detail = error
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| error.downcast_ref::<&str>().copied())
                        .unwrap_or("unknown panic");
                    eprintln!(
                        "Corpus parser frontier mismatch {} id {:?}: {detail}\n{raw}",
                        path.display(),
                        source_attributes(&parse).get("id")
                    );
                    state_failures.push(format!("{}: {detail}", path.display()));
                } else {
                    counts.5 += 1;
                    full_state_paired = true;
                }
            }
            if affix_boundary.is_none()
                && kind == DependencyKind::RuneReconstruction
                && machine.pending().unwrap().line_index.is_none()
            {
                affix_boundary = Some((
                    "after_raw_headers_before_affix_reconcile",
                    parse.get::<Table>("affix_before_reconcile").unwrap(),
                ));
            }
            if affix_boundary.is_none() {
                let formats = events(&loaded, "format")
                    .into_iter()
                    .filter(|event| {
                        event
                            .get::<Table>("before")
                            .unwrap()
                            .get::<String>("raw")
                            .unwrap()
                            == raw
                    })
                    .collect::<Vec<_>>();
                let chosen = if kind == DependencyKind::ModifierParser {
                    provider
                        .formats
                        .len()
                        .checked_sub(1)
                        .map(|i| ("before_unavailable_parser", i))
                } else if kind == DependencyKind::RuneReconstruction
                    && machine.pending().unwrap().message
                        == "rune display/reconstruction requires complete slot and modifier dependencies"
                {
                    Some(("before_rune_display_format", provider.formats.len()))
                } else {
                    None
                };
                if let Some((boundary, index)) = chosen {
                    affix_boundary = Some((
                        boundary,
                        formats
                            .get(index)
                            .expect("matching source format boundary")
                            .get::<Table>("before")
                            .unwrap(),
                    ));
                }
            }
            let source_affixes = affix_boundary.as_ref().map(|(boundary, before)| {
                reference::compare_affixes(&machine.state().prefixes, before.get("prefixes").unwrap(), "corpus prefixes at original frontier");
                reference::compare_affixes(&machine.state().suffixes, before.get("suffixes").unwrap(), "corpus suffixes at original frontier");
                serde_json::json!({"boundary":boundary,"prefixes":reference::source_affixes(before.get("prefixes").unwrap()),"suffixes":reference::source_affixes(before.get("suffixes").unwrap())})
            });
            affix_evidence.push(serde_json::json!({
                "fixture":row["xml"],"fixture_sha256":row["xml_sha256"],"id":source_attributes(&parse).get("id"),
                "raw_sha256":format!("{:x}",Sha256::digest(raw.as_bytes())),"pending":machine.pending(),
                "source_affixes":source_affixes,
                "native_affixes":{"prefixes":&machine.state().prefixes,"suffixes":&machine.state().suffixes},
                "native_state":machine.state(),
                "parser_prefix_calls":parses,"format_prefix_calls":formats,"precision_parser_calls":precision,
                "source_full_state_boundary":full_state_boundary,"source_full_state_paired":full_state_paired
            }));
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    if let Some(path) = std::env::var_os("POE_OPTIMIZER_AFFIX_ORACLE_LEDGER") {
        let evidence = serde_json::json!({"schema_version":2,"upstream_revision":poe_optimizer_pob::source::UPSTREAM_REVISION,
            "data":snapshot.identity(),"item_loading_implementation_sha256":implementation_fingerprint(),
            "definition_implementation_sha256":poe_optimizer_data::implementation_fingerprint(),
            "items":affix_evidence,"full_state_failures":state_failures});
        std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
    eprintln!(
        "Original affix frontier comparisons: {} of {} corpus occurrences",
        affix_evidence
            .iter()
            .filter(|row| !row["source_affixes"].is_null())
            .count(),
        affix_evidence.len()
    );
    assert_eq!(counts.0, 116);
    assert!(counts.1 > 0 && counts.2 > 0 && counts.4 > 0);
    eprintln!(
        "Combined corpus provider {} items, {} parser prefix calls, {} format prefixes, {} precision calls, {} complete preassembly states, {} complete parser frontier states, pending {pending:?}",
        counts.0, counts.1, counts.2, counts.3, counts.4, counts.5
    );
    assert!(
        state_failures.is_empty(),
        "{} full-state frontier mismatches",
        state_failures.len()
    );
}

#[test]
fn original_defence_header_precedence_preserves_armour_data_before_pending_assembly() {
    let snapshot = bundled_snapshot().unwrap();
    let source = runtime::Oracle::new();
    for (header, key) in [
        ("Armour", "Armour"),
        ("Evasion Rating", "Evasion"),
        ("Evasion", "Evasion"),
        ("Energy Shield", "EnergyShield"),
        ("Ward", "Ward"),
        ("Runic Ward", "Ward"),
    ] {
        let raw = format!("Rarity: NORMAL\nRusted Greathelm\n{header}: 17");
        let loaded = source.load(
            &format!("<Items><Item id=\"9\">{}</Item></Items>", escaped(&raw)),
            false,
        );
        assert!(loaded.get::<bool>("ok").unwrap(), "{header}");
        let parse = events(&loaded, "parse_raw")
            .into_iter()
            .find(|event| !event.get::<String>("raw").unwrap().is_empty())
            .unwrap();
        let raw = parse.get::<String>("raw").unwrap();
        let before = events(&loaded, "build_mod_list")
            .into_iter()
            .map(|event| event.get::<Table>("before").unwrap())
            .find(|before| before.get::<String>("raw").unwrap() == raw)
            .unwrap();
        let armour = before.get::<Table>("armourData").unwrap();
        assert_eq!(
            armour.get::<f64>(key).unwrap().to_bits(),
            17.0f64.to_bits(),
            "{header}"
        );
        assert!(
            matches!(before.get::<Value>("hidden_specs").unwrap(), Value::Nil),
            "earlier original branch preempts hidden_specs for {header}"
        );
        let mut provider = TracedNative::new(&snapshot);
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.set_xml_attributes(&source_attributes(&parse));
        machine.apply_text(&raw, &mut provider).unwrap();
        assert_eq!(
            machine.pending().map(|pending| pending.kind),
            Some(DependencyKind::Assembly)
        );
        assert!(!machine.state().retained_fields.contains_key("hidden_specs"));
        assert!(provider.parser.is_empty() && provider.formats.is_empty());
        assert_eq!(provider.assembly.len(), 1);
        reference::compare_state(machine.state(), &before);
    }
}

fn source_attributes(parse: &Table) -> BTreeMap<String, String> {
    let before = parse.get::<Table>("before").unwrap();
    [
        "id",
        "variant",
        "variantAlt",
        "variantAlt2",
        "variantAlt3",
        "variantAlt4",
        "variantAlt5",
    ]
    .into_iter()
    .filter_map(|key| {
        let value = before.get::<Value>(key).unwrap();
        let text = match value {
            Value::Integer(n) => n.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Nil => return None,
            value => panic!("unexpected XML-bound original state {}", value.type_name()),
        };
        Some((key.to_owned(), text))
    })
    .collect()
}
