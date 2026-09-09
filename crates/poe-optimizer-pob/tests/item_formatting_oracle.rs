//! Exact string and parser-dependency parity against authenticated original ItemTools.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/item_formatter_native.rs"]
mod native;
#[path = "support/item_formatter_runtime.rs"]
mod oracle;
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use mlua::{Table, Value};
use oracle::FormatterOracle;
use sha2::{Digest, Sha256};

#[test]
fn original_formatter_harness_authenticates_source_and_records_live_formatter_traces() {
    for (path, hash) in [
        (
            "src/Modules/ItemTools.lua",
            "24e114bc64d8e213d4ed970c34fa013088e7b298050c65055c179a0b5f302973",
        ),
        (
            "src/Data/ModScalability.lua",
            "89a26737f5c62c51b5f87cb1cfffbf27fad4b01af71594edafa48333911f291c",
        ),
        (
            "src/Modules/Common.lua",
            "bae6d0704a92fb04ed56a6033f9229eb2683c53785c14b9c5571b4c0591b0fe8",
        ),
    ] {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(runtime::verified(path).unwrap().as_bytes())
            ),
            hash
        );
    }
    let oracle = FormatterOracle::new();
    let observed = oracle.observe(
        "range",
        &[
            oracle.text("+(10-20) to maximum Life"),
            Value::Number(0.5),
            Value::Number(1.5),
        ],
    );
    assert!(observed.get::<bool>("ok").unwrap());
    assert_eq!(
        observed.get::<String>("value").unwrap(),
        "+22 to maximum Life"
    );
    let trace_names = oracle.warm();
    // applyValueScalar callback paths may remain interpreted; do not claim their traces.
    for line in [45, 130] {
        assert!(
            trace_names.contains(&format!("@src/Modules/ItemTools.lua:{line}")),
            "missing original function on a completed live trace: {line}: {trace_names:?}"
        );
    }
    eprintln!("Completed live original formatter traces: {trace_names:?}");
    // Exercise both complete-source helper entry points; neither constructs native expectations.
    assert!(
        oracle
            .source
            .parse("Rarity: Normal\nRusted Greathelm\nQuality: 0")
            .contains_key("baseName")
            .unwrap()
    );
    assert!(
        oracle
            .source
            .load("<Items/>", false)
            .get::<bool>("ok")
            .unwrap()
    );
    let assignment = oracle.assignment(&["divide_by_two_0dp".into(), "divide_by_three".into()]);
    assert_eq!(
        assignment.get::<f64>("precision").unwrap().to_bits(),
        3f64.to_bits()
    );
    assert_eq!(assignment.get::<u32>("display_precision").unwrap(), 0);
    assert!(assignment.get::<bool>("if_required").unwrap());
    assert!(observed.get::<Table>("calls").unwrap().is_empty());
}

fn observed_assignment(row: Table) -> (f64, Option<u8>, Option<bool>) {
    (
        row.get("precision").unwrap(),
        row.get("display_precision").unwrap(),
        row.get("if_required").unwrap(),
    )
}
fn source_format_labels() -> std::collections::BTreeSet<String> {
    let source = runtime::verified("src/Modules/ItemTools.lua").unwrap();
    let begin = source.find("if scalability.formats then").unwrap();
    let end = source[begin..].find("if scalability.isScalable").unwrap() + begin;
    source[begin..end]
        .split("format == \"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap().to_owned())
        .collect()
}
#[test]
fn complete_catalog_and_partial_assignments_match_original_tables_and_dispatch() {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let catalog = snapshot.item_scalability();
    let data = catalog.data();
    let mut pairs = 0usize;
    for warm in [false, true] {
        let oracle = FormatterOracle::new();
        if warm {
            assert!(!oracle.warm().is_empty());
        }
        let source_data = oracle.source.lua.globals().get::<Table>("data").unwrap();
        let entries = source_data.get::<Table>("modScalability").unwrap();
        let mut originals = std::collections::BTreeMap::new();
        let mut all_labels = std::collections::BTreeSet::new();
        let mut captures = 0usize;
        for row in entries.pairs::<String, Table>() {
            let (key, values) = row.unwrap();
            let values = values
                .sequence_values::<Table>()
                .map(|v| {
                    let value = v.unwrap();
                    let fields = value
                        .clone()
                        .pairs::<String, Value>()
                        .map(|r| r.unwrap().0)
                        .collect::<std::collections::BTreeSet<_>>();
                    assert!(
                        fields
                            .iter()
                            .all(|k| matches!(k.as_str(), "isScalable" | "formats"))
                    );
                    let formats = value.get::<Option<Table>>("formats").unwrap().map(|list| {
                        list.sequence_values::<String>()
                            .map(Result::unwrap)
                            .collect::<Vec<_>>()
                    });
                    if let Some(labels) = &formats {
                        all_labels.extend(labels.iter().cloned());
                    }
                    captures += 1;
                    poe_optimizer_data::item_scalability::ItemScalabilityValue {
                        is_scalable: value.get("isScalable").unwrap(),
                        formats,
                    }
                })
                .collect::<Vec<_>>();
            originals.insert(key, values);
        }
        assert_eq!(
            originals, data.entries,
            "all original constructed keys/ordered descriptors, warm={warm}"
        );
        assert_eq!(originals.len(), 15090);
        assert_eq!(captures, 12040);
        pairs += originals.len();
        let handled = source_format_labels();
        assert_eq!(handled.len(), 33);
        assert_eq!(
            data.format_assignments
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            handled
        );
        let noops = all_labels.difference(&handled).cloned().collect::<Vec<_>>();
        assert_eq!(noops.len(), 14);
        // Ordered prior settings distinguish partial assignments from reset-to-default.
        let prefixes = std::iter::once(Vec::<String>::new())
            .chain(handled.iter().map(|label| vec![label.clone()]))
            .collect::<Vec<_>>();
        for prefix in &prefixes {
            let before = observed_assignment(oracle.assignment(prefix));
            for label in handled.iter().chain(&noops) {
                let mut chain = prefix.clone();
                chain.push(label.clone());
                let actual = observed_assignment(oracle.assignment(&chain));
                let expected = if let Some(patch) = data.format_assignments.get(label) {
                    (
                        patch.precision.unwrap_or(before.0),
                        patch.display_precision.or(before.1),
                        patch.if_required.or(before.2),
                    )
                } else {
                    before
                };
                assert_eq!(actual, expected, "ordered formats {chain:?}, warm={warm}");
                pairs += 1;
            }
        }
        assert_eq!(
            source_data.get::<u8>("defaultHighPrecision").unwrap(),
            data.default_high_precision
        );
        let source = runtime::verified("src/Modules/ItemTools.lua").unwrap();
        let begin = source.find("local antonyms =").unwrap();
        let end = source[begin..].find("local function antonymFunc").unwrap() + begin;
        let antonyms = oracle
            .source
            .lua
            .load(format!("{}\nreturn antonyms", &source[begin..end]))
            .set_name("@unchanged-source-antonyms-table")
            .eval::<Table>()
            .unwrap()
            .pairs::<String, String>()
            .map(Result::unwrap)
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(antonyms, data.antonyms);
        for (path, hash) in &data.source.files {
            assert_eq!(
                format!(
                    "{:x}",
                    Sha256::digest(runtime::verified(path).unwrap().as_bytes())
                ),
                *hash
            );
        }
        for span in data.source.construction_spans.values() {
            let source = runtime::verified(&span.path).unwrap();
            let bytes = source
                .split_inclusive('\n')
                .skip(span.line as usize - 1)
                .take((span.end_line - span.line + 1) as usize)
                .collect::<String>();
            assert_eq!(
                format!("{:x}", Sha256::digest(bytes.as_bytes())),
                span.sha256
            );
        }
    }
    eprintln!(
        "Independent complete catalog: {pairs} cold/warmed table and ordered-dispatch comparisons"
    );
}

fn optional(value: Option<f64>) -> Value {
    value.map_or(Value::Nil, Value::Number)
}
fn exact_text(
    actual: Result<String, poe_optimizer_engine::item_tools::FormatError>,
    source: &Table,
    context: &str,
) {
    assert!(
        source.get::<bool>("ok").unwrap(),
        "unexpected original source error for {context}: {:?}",
        source.get::<Option<String>>("error").unwrap()
    );
    let expected = source.get::<String>("value").unwrap();
    assert_eq!(
        actual.unwrap_or_else(|e| panic!("native failed {context}: {e}; source={expected:?}")),
        expected,
        "{context}"
    );
}
#[test]
fn raw_numeric_formatting_matches_exact_source_strings_cold_and_warmed() {
    use poe_optimizer_engine::item_tools::{format_value, lua_number_text};
    let mut comparisons = 0;
    for warm in [false, true] {
        let oracle = FormatterOracle::new();
        if warm {
            assert!(!oracle.warm().is_empty());
        }
        let tostring = oracle
            .source
            .lua
            .globals()
            .get::<mlua::Function>("tostring")
            .unwrap();
        for value in [
            -f64::MAX,
            -1e14,
            -0.00001,
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.00001,
            0.0001,
            9.99999999999996,
            1e14,
            f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            assert_eq!(
                lua_number_text(value),
                tostring.call::<String>(value).unwrap(),
                "Lua numeric spelling {value:?}, warm={warm}"
            );
            comparisons += 1;
        }
        for value in [
            -1e20,
            -17.5,
            -0.505,
            -0.5,
            -f64::from_bits(1),
            -0.0,
            0.0,
            0.005,
            0.5,
            17.5,
            1e14,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            for (base, scalar) in [
                (None, None),
                (Some(1.), Some(1.)),
                (Some(1.3), Some(1.2)),
                (Some(0.), Some(0.)),
                (Some(0.8), Some(1.3)),
                (Some(-1.), Some(-1.)),
            ] {
                for precision in [1., 2., 3., 10., 60., 1000.] {
                    for display in [None, Some(0u8), Some(1), Some(2)] {
                        for required in [false, true] {
                            let source = oracle.observe(
                                "value",
                                &[
                                    Value::Number(value),
                                    optional(base),
                                    optional(scalar),
                                    Value::Number(precision),
                                    display.map_or(Value::Nil, |v| Value::Integer(v.into())),
                                    Value::Boolean(required),
                                ],
                            );
                            exact_text(
                                format_value(value, base, scalar, precision, display, required),
                                &source,
                                &format!(
                                    "formatValue({value:?},{base:?},{scalar:?},{precision},{display:?},{required}) warm={warm}"
                                ),
                            );
                            comparisons += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("Exact original formatValue/tostring comparisons: {comparisons}");
}
#[test]
fn raw_legacy_scalar_patterns_match_original_number_limits_and_rounding() {
    use poe_optimizer_engine::item_tools::apply_value_scalar;
    let mut comparisons = 0;
    for warm in [false, true] {
        let oracle = FormatterOracle::new();
        if warm {
            assert!(!oracle.warm().is_empty());
        }
        for line in [
            "+12 to Life",
            "-12 to Life",
            "-12.5 to Life",
            "12",
            "12.",
            "A 12 B 34 C",
            "Ω12.05 then 0023 values",
            "-0% reduced Life",
            "1e-07 values",
            "(10-20) to 30 values",
        ] {
            for (scalar, base) in [
                (None, None),
                (Some(1.), Some(1.)),
                (Some(1.3), Some(0.8)),
                (Some(0.), Some(-1.)),
                (Some(-1.3), None),
            ] {
                for numbers in [None, Some(0usize), Some(1), Some(2), Some(64)] {
                    for precision in [None, Some(0.), Some(1.), Some(2.), Some(-1.), Some(0.5)] {
                        let source = oracle.observe(
                            "scalar",
                            &[
                                oracle.text(line),
                                optional(scalar),
                                optional(base),
                                numbers.map_or(Value::Nil, |v| Value::Integer(v as i64)),
                                optional(precision),
                            ],
                        );
                        exact_text(
                            apply_value_scalar(line, scalar, base, numbers, precision),
                            &source,
                            &format!(
                                "applyValueScalar({line:?},{scalar:?},{base:?},{numbers:?},{precision:?}) warm={warm}"
                            ),
                        );
                        comparisons += 1;
                    }
                }
            }
        }
    }
    eprintln!("Exact original applyValueScalar comparisons: {comparisons}");
}

#[test]
fn every_original_scalability_key_formats_exactly_with_scalable_and_fixed_capture_records() {
    use poe_optimizer_engine::item_tools::{FormatInput, ItemFormatter, RangeInput};
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let formatter = ItemFormatter::new(snapshot.item_scalability(), &snapshot.package().actor);
    let mut count = 0;
    for warm in [false, true] {
        let oracle = FormatterOracle::new();
        if warm {
            assert!(!oracle.warm().is_empty());
        }
        for key in snapshot.item_scalability().data().entries.keys() {
            let line = key.replace('#', "17.5");
            let input = FormatInput {
                line: &line,
                range: RangeInput::Scalar(0.25),
                value_scalar: Some(1.3),
                base_value_scalar: Some(1.1),
            };
            let source = native::source(&oracle, input);
            assert_eq!(
                native::compare(&formatter, input, &source),
                0,
                "known-key dispatcher must not need parser: {key}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 30180);
    eprintln!("Complete original scalability-key string comparisons: {count}");
}
#[test]
fn ranges_signs_missing_selectors_and_actual_precision_parser_match_original() {
    use poe_optimizer_engine::item_tools::{FormatInput, ItemFormatter, RangeInput};
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let formatter = ItemFormatter::new(snapshot.item_scalability(), &snapshot.package().actor);
    let sparse = [None, Some(0.0)];
    let explicit = [Some(0.25), Some(1.0)];
    let ranges = [
        RangeInput::Missing,
        RangeInput::Scalar(0.),
        RangeInput::Scalar(0.5),
        RangeInput::Scalar(1.),
        RangeInput::Scalar(-1.),
        RangeInput::Scalar(2.),
        RangeInput::Scalar(-0.0),
        RangeInput::Scalar(f64::NAN),
        RangeInput::Scalar(f64::INFINITY),
        RangeInput::Scalar(f64::NEG_INFINITY),
        RangeInput::Values(&[]),
        RangeInput::Values(&sparse),
        RangeInput::Values(&explicit),
    ];
    let mut counts = (0, 0);
    for warm in [false, true] {
        let oracle = FormatterOracle::new();
        if warm {
            assert!(!oracle.warm().is_empty());
        }
        for line in [
            "+(10-20) to maximum Life",
            "+(10-20) to Maximum Life",
            "Adds (10-20) to (30-40) Physical Damage",
            "(10--10)% increased Charges per use",
            "+(-25-50)% to Fire Resistance",
            "-(0.1-0.3)% increased Movement Speed",
            "-(0-0) to maximum Life",
            "+(0-0.0000002) to maximum Life",
            "-(0-0.0000002) to maximum Life",
            "Regenerate (66.7-75) Life per second",
            "Minions Regenerate (1.23-3.45)% of Life per second",
            "Minions have (1.23-3.45)% to Critical Hit Chance",
            "+17.5 to Evasion Rating",
            "+17.5 to evasion rating",
            "Oracle (1.25-2.5) values and 13 other values",
            "Oracle 0017.50 values",
            "Oracle -0% increased Damage",
            "Oracle -17.5% more Damage",
            "Oracle 1e-07 values",
            "Oracle .5 values",
            "Oracle 12 final3",
        ] {
            for range in ranges {
                for (scalar, base) in [
                    (None, None),
                    (Some(1.3), Some(1.1)),
                    (Some(0.), Some(0.)),
                    (Some(-1.), None),
                ] {
                    let input = FormatInput {
                        line,
                        range,
                        value_scalar: scalar,
                        base_value_scalar: base,
                    };
                    let source = native::source(&oracle, input);
                    counts.1 += native::compare(&formatter, input, &source);
                    counts.0 += 1;
                }
            }
        }
    }
    eprintln!(
        "Original range/fallback comparisons: {}, exact parser requests: {}",
        counts.0, counts.1
    );
}
#[test]
fn injected_exact_keys_preserve_literal_priority_empty_entries_and_source_case() {
    use poe_optimizer_data::item_scalability::{ItemScalabilityCatalog, ItemScalabilityValue};
    use poe_optimizer_engine::item_tools::{FormatInput, ItemFormatter, RangeInput};
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let original = snapshot.item_scalability().data();
    for full_literal in [false, true] {
        let mut custom = original.clone();
        custom.entries = std::collections::BTreeMap::from([
            (
                "Oracle # #".into(),
                vec![
                    ItemScalabilityValue {
                        is_scalable: true,
                        formats: None
                    };
                    2
                ],
            ),
            (
                "Oracle 1 #".into(),
                vec![ItemScalabilityValue {
                    is_scalable: false,
                    formats: None,
                }],
            ),
            (
                "Oracle # 2".into(),
                vec![ItemScalabilityValue {
                    is_scalable: true,
                    formats: Some(vec!["divide_by_ten_1dp".into()]),
                }],
            ),
            (
                "Oracle # alpha 4 #".into(),
                vec![
                    ItemScalabilityValue {
                        is_scalable: true,
                        formats: None
                    };
                    2
                ],
            ),
        ]);
        if full_literal {
            custom.entries.insert("Oracle 1 2".into(), vec![]);
        }
        let catalog = ItemScalabilityCatalog::new(custom).unwrap();
        let formatter = ItemFormatter::new(&catalog, &snapshot.package().actor);
        let oracle = FormatterOracle::new();
        native::install_entries(&oracle, &catalog);
        for line in [
            "Oracle 1 2",
            "Oracle 3 2",
            "Oracle 1 3",
            "Oracle 03 2",
            "oracle 1 2",
            "Oracle 1 alpha 4 2",
        ] {
            let input = FormatInput {
                line,
                range: RangeInput::Scalar(1.),
                value_scalar: Some(1.7),
                base_value_scalar: Some(1.1),
            };
            let source = native::source(&oracle, input);
            native::compare(&formatter, input, &source);
        }
    }
}

#[test]
fn catalyst_tags_defaults_zero_quality_unscalable_and_injected_prefix_tags_match_source() {
    use poe_optimizer_engine::item_tools::catalyst_scalar;
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let policy = &snapshot.item_scalability().data().catalyst_scaling;
    let mut count = 0usize;
    for injected in [false, true] {
        let oracle = FormatterOracle::new();
        let mut catalysts = snapshot.item_loading().policy().catalysts.clone();
        if injected {
            catalysts[0].tags = vec!["prefix".into(), "suffix".into()];
            oracle
                .source
                .lua
                .globals()
                .get::<Table>("itemPolicy")
                .unwrap()
                .get::<Table>("tags")
                .unwrap()
                .raw_set(
                    1,
                    oracle
                        .source
                        .lua
                        .create_sequence_from(catalysts[0].tags.clone())
                        .unwrap(),
                )
                .unwrap();
        }
        let choices = [
            None,
            Some(vec![]),
            Some(vec!["life".to_owned()]),
            Some(vec!["Life".to_owned()]),
            Some(vec!["other".to_owned()]),
            Some(vec!["mana".into(), "fire".into()]),
        ];
        let ids = std::iter::once(None)
            .chain((0..=14).map(|n| Some(f64::from(n))))
            .chain([Some(1.5), Some(f64::NAN), Some(f64::INFINITY)])
            .collect::<Vec<_>>();
        for id in ids {
            for tags in &choices {
                for unscalable in [false, true] {
                    for flags in [
                        std::collections::BTreeSet::<String>::new(),
                        std::collections::BTreeSet::from(["prefix".into()]),
                        std::collections::BTreeSet::from(["suffix".into()]),
                    ] {
                        for quality in [
                            None,
                            Some(0.),
                            Some(-0.),
                            Some(20.),
                            Some(30.),
                            Some(-100.),
                            Some(f64::NAN),
                            Some(f64::INFINITY),
                        ] {
                            let row = oracle.source.lua.create_table().unwrap();
                            if let Some(tags) = tags {
                                row.set(
                                    "modTags",
                                    oracle
                                        .source
                                        .lua
                                        .create_sequence_from(tags.clone())
                                        .unwrap(),
                                )
                                .unwrap();
                            }
                            for flag in &flags {
                                row.set(flag.as_str(), true).unwrap();
                            }
                            row.set("unscalable", unscalable).unwrap();
                            let source = oracle.observe(
                                "catalyst",
                                &[optional(id), Value::Table(row), optional(quality)],
                            );
                            assert!(source.get::<bool>("ok").unwrap());
                            let expected = source.get::<f64>("value").unwrap();
                            let actual = catalyst_scalar(
                                policy,
                                &catalysts,
                                id,
                                tags.as_deref(),
                                &flags,
                                unscalable,
                                quality,
                            )
                            .unwrap();
                            assert!(
                                if expected.is_nan() {
                                    actual.is_nan()
                                } else {
                                    actual.to_bits() == expected.to_bits()
                                },
                                "catalyst id={id:?},tags={tags:?},flags={flags:?},unscalable={unscalable},quality={quality:?}: {actual:?} != {expected:?}"
                            );
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("Original catalyst scalar comparisons: {count}");
}
#[test]
fn all_caller_corpus_formatting_preparation_matches_original_with_exact_parser_callbacks() {
    use poe_optimizer_engine::item_tools::{FormatInput, ItemFormatter, RangeInput};
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let formatter = ItemFormatter::new(snapshot.item_scalability(), &snapshot.package().actor);
    let oracle = FormatterOracle::new();
    let directory = runtime::repository().join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    let mut counts = (0usize, 0usize, 0usize);
    for row in index["builds"].as_array().unwrap() {
        let path = directory.join(row["xml"].as_str().unwrap());
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            row["xml_sha256"].as_str().unwrap()
        );
        let xml = std::str::from_utf8(&bytes).unwrap();
        let original = oracle.source.load(xml, false);
        let captured = oracle
            .source
            .lua
            .globals()
            .get::<mlua::Function>("formatter_capture_load")
            .unwrap()
            .call::<Table>(xml)
            .unwrap();
        let loaded = captured.get::<Table>("loaded").unwrap();
        assert!(original.get::<bool>("ok").unwrap());
        assert!(
            loaded.get::<bool>("ok").unwrap(),
            "{:?}",
            loaded.get::<Option<String>>("error").unwrap()
        );
        assert_eq!(
            original.get::<Table>("items").unwrap().raw_len(),
            loaded.get::<Table>("items").unwrap().raw_len()
        );
        counts.0 += loaded.get::<Table>("items").unwrap().raw_len();
        for event in captured
            .get::<Table>("formats")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
        {
            let args = event.get::<Table>("arguments").unwrap();
            let line = args.get::<String>(1).unwrap();
            let raw_range = args.get::<Value>(2).unwrap();
            let storage = match &raw_range {
                Value::Table(table) => (1..=table
                    .clone()
                    .pairs::<i64, Value>()
                    .map(|r| r.unwrap().0)
                    .max()
                    .unwrap_or(0))
                    .map(|i| table.get::<Option<f64>>(i).unwrap())
                    .collect::<Vec<_>>(),
                _ => vec![],
            };
            let range = match raw_range {
                Value::Nil => RangeInput::Missing,
                Value::Integer(v) => RangeInput::Scalar(v as f64),
                Value::Number(v) => RangeInput::Scalar(v),
                Value::Table(_) => RangeInput::Values(&storage),
                other => panic!("unrepresented original range {other:?}"),
            };
            let input = FormatInput {
                line: &line,
                range,
                value_scalar: args.get(3).unwrap(),
                base_value_scalar: args.get(4).unwrap(),
            };
            counts.2 += native::compare(&formatter, input, &event.get::<Table>("result").unwrap());
            counts.1 += 1;
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    assert_eq!(counts.0, 116);
    assert!(counts.1 > 116);
    eprintln!(
        "Frozen original corpus preparation: {} items, {} exact formatter calls, {} exact precision-parser calls. No native item-assembly claim.",
        counts.0, counts.1, counts.2
    );
}

fn source_events(source: &Table, kind: &str) -> Vec<Table> {
    source
        .get::<Table>("events")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|row| row.get::<String>("kind").unwrap() == kind)
        .collect()
}
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
#[test]
fn native_provider_preserves_initial_range_one_catalysts_and_preassembly_source_state() {
    use poe_optimizer_import::item_loading::*;
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let oracle = FormatterOracle::new();
    for newline in ["\n", "\r\n"] {
        for (header, line) in [
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
                "{tags:life}{corruptedRange:1.2}+17.5 to maximum Life",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life - Unscalable Value",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life \u{2014} Unscalable Value",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life \u{fffd} Unscalable Value",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life - Unscalable Value - Unscalable Value",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life \u{2014} Unscalable Value \u{2014} Unscalable Value",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life \u{2014} Unscalable Value - Unscalable Value",
            ),
            (
                "Catalyst: Flesh",
                "{tags:life}+17.5 to maximum Life - Unscalable Value \u{2014} Unscalable Value",
            ),
            ("", "Minions Regenerate (1.25-2.75)% of Life per second"),
        ] {
            let raw=format!("Rarity: Rare\nCaller amulet\nAmber Amulet\nItem Level: 60\n{header}\nImplicits: 0\n{line}").replace('\n',newline);
            let source = oracle.source.load(
                &format!("<Items><Item id=\"3\">{}</Item></Items>", escape_xml(&raw)),
                false,
            );
            assert!(
                source.get::<bool>("ok").unwrap(),
                "{:?}",
                source.get::<Option<String>>("error").unwrap()
            );
            let parse = source_events(&source, "parse_raw")
                .into_iter()
                .find(|r| !r.get::<String>("raw").unwrap().is_empty())
                .unwrap();
            let consumed = parse.get::<String>("raw").unwrap();
            let before = source_events(&source, "build_mod_list")
                .into_iter()
                .map(|r| r.get::<Table>("before").unwrap())
                .find(|r| r.get::<String>("raw").unwrap() == consumed)
                .unwrap();
            let mut provider = NativeItemLoadProvider::with_dependencies(
                &snapshot,
                native::OriginalParser {
                    oracle: &oracle,
                    calls: vec![],
                },
            );
            let mut machine = ItemLoadMachine::new(snapshot.item_loading());
            machine.set_xml_attributes(&std::collections::BTreeMap::from([(
                "id".into(),
                "3".into(),
            )]));
            machine.apply_text(&consumed, &mut provider).unwrap();
            assert_eq!(machine.status(), ItemLoadStatus::Pending, "{raw}");
            assert_eq!(
                machine.pending().unwrap().kind,
                DependencyKind::Assembly,
                "{raw}"
            );
            native::compare_state(machine.state(), &before);
            let expected = parse
                .get::<Table>("calls")
                .unwrap()
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .filter(|r| r.get::<String>("phase").unwrap() == "parse_raw")
                .map(|r| {
                    (
                        r.get::<String>("text").unwrap(),
                        r.get::<bool>("combined").unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(
                provider
                    .dependencies()
                    .calls
                    .iter()
                    .map(|r| (r.text.clone(), r.combined))
                    .collect::<Vec<_>>(),
                expected,
                "complete precision+direct source parser order"
            );
            let formats = source_events(&source, "format");
            assert_eq!(machine.state().format_calls.len(), formats.len());
            for (actual, expected) in machine.state().format_calls.iter().zip(formats) {
                assert_eq!(actual.text, expected.get::<String>("text").unwrap());
                native::assert_number(
                    actual.range,
                    native::number(expected.get("range").unwrap()),
                    "initial source range",
                );
                native::assert_number(
                    actual.scalar,
                    native::number(expected.get("scalar").unwrap()),
                    "source catalyst",
                );
                native::assert_number(
                    actual.corrupted_range,
                    native::number(expected.get("corrupted").unwrap()),
                    "source corrupted scalar",
                );
                native::assert_number(
                    actual.range,
                    ItemNumber::Finite(1.),
                    "parse uses range1, not selected quarter",
                );
            }
            let mut order = machine
                .state()
                .format_calls
                .iter()
                .map(|r| r.sequence)
                .chain(machine.state().parser_calls.iter().map(|r| r.sequence))
                .chain(
                    machine
                        .state()
                        .format_parser_calls
                        .iter()
                        .map(|r| r.request.sequence),
                )
                .collect::<Vec<_>>();
            order.sort();
            assert_eq!(
                order,
                (0..order.len()).collect::<Vec<_>>(),
                "shared source operation sequence"
            );
        }
    }
}
#[test]
fn forced_fallback_uses_actual_nested_parser_records_and_ordered_precision_providers() {
    use poe_optimizer_data::{
        game_data::ActorNumericOperation, item_scalability::ItemScalabilityCatalog,
    };
    use poe_optimizer_engine::item_tools::{FormatInput, ItemFormatter, RangeInput};
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let mut custom = snapshot.item_scalability().data().clone();
    custom.entries = std::collections::BTreeMap::from([("Unrelated literal".into(), vec![])]);
    let catalog = ItemScalabilityCatalog::new(custom).unwrap();
    let oracle = FormatterOracle::new();
    native::install_entries(&oracle, &catalog);
    let mut actor = snapshot.package().actor.clone();
    let high = oracle
        .source
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get::<Table>("highPrecisionMods")
        .unwrap();
    // The complete original parser creates both records; only their precision policy is injected.
    for (name, precision) in [("LifeRegen", 1u8), ("ManaRegen", 2u8)] {
        actor.high_precision_mods.insert(
            name.into(),
            std::collections::BTreeMap::from([(ActorNumericOperation::Increased, precision)]),
        );
        let table = oracle.source.lua.create_table().unwrap();
        table.set("INC", precision).unwrap();
        high.set(name, table).unwrap();
    }
    let formatter = ItemFormatter::new(&catalog, &actor);
    let mut nested = 0;
    let mut multiple = 0;
    for line in [
        "Minions Regenerate (1.23-3.45)% of Life per second",
        "(1.23-3.45)% increased Life and Mana Regeneration Rate",
    ] {
        let input = FormatInput {
            line,
            range: RangeInput::Scalar(0.5),
            value_scalar: Some(1.3),
            base_value_scalar: None,
        };
        let source = native::source(&oracle, input);
        assert_eq!(native::compare(&formatter, input, &source), 1);
        let calls = source.get::<Table>("calls").unwrap();
        let mods = calls
            .get::<Table>(1)
            .unwrap()
            .get::<Table>("modifiers")
            .unwrap();
        let rows = mods
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        multiple += usize::from(rows.len() > 1);
        nested += rows
            .iter()
            .filter(|r| {
                r.get::<Option<Table>>("value")
                    .ok()
                    .flatten()
                    .is_some_and(|t| t.contains_key("mod").unwrap())
            })
            .count();
    }
    assert!(
        nested > 0,
        "actual source MinionModifier precision path was not exercised"
    );
    assert!(
        multiple > 0,
        "actual source multiple numeric records were not exercised"
    );
}

#[test]
fn explicit_raw_feedback_preserves_nil_empty_extra_and_nested_scalar_semantics() {
    use poe_optimizer_data::item_scalability::ItemScalabilityCatalog;
    use poe_optimizer_engine::item_tools::{
        FormatError, FormatInput, FormatResult, ItemFormatter, ParserFeedback, RangeInput,
    };
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let mut custom = snapshot.item_scalability().data().clone();
    custom.entries = std::collections::BTreeMap::from([("Unrelated literal".into(), vec![])]);
    let catalog = ItemScalabilityCatalog::new(custom).unwrap();
    let formatter = ItemFormatter::new(&catalog, &snapshot.package().actor);
    let oracle = FormatterOracle::new();
    native::install_entries(&oracle, &catalog);
    let input = FormatInput {
        line: "Oracle (1.23-3.45) values",
        range: RangeInput::Scalar(0.5),
        value_scalar: Some(1.3),
        base_value_scalar: None,
    };
    let cases = [
        ("nil", "nil", None, false),
        ("empty", "{}", None, false),
        ("false nested", "{{value={mod=false}}}", None, false),
        ("string nested", "{{value={mod='text'}}}", None, false),
        ("array nested", "{{value={mod={'text'}}}}", None, false),
        ("numeric nested", "{{value={mod=0}}}", None, true),
        ("boolean nested", "{{value={mod=true}}}", None, true),
        (
            "empty extra bypasses nested error",
            "{{value={mod=true}}}",
            Some(""),
            false,
        ),
        (
            "text extra bypasses nested error",
            "{{value={mod=0}}}",
            Some("remaining"),
            false,
        ),
    ];
    for (label, expression, extra, source_error) in cases {
        let mods = oracle
            .source
            .lua
            .load(format!("return {expression}"))
            .eval::<Value>()
            .unwrap();
        let args = oracle.args(&[
            oracle.text(input.line),
            native::range_value(&oracle, input.range),
            optional(input.value_scalar),
            optional(input.base_value_scalar),
        ]);
        let source = oracle
            .source
            .lua
            .globals()
            .get::<mlua::Function>("formatter_feedback")
            .unwrap()
            .call::<Table>((args, mods, extra))
            .unwrap();
        if source_error {
            assert!(!source.get::<bool>("ok").unwrap(), "{label}");
            let error = source.get::<String>("error").unwrap();
            assert!(
                error.contains("Modules/ItemTools.lua:314:")
                    && error.contains("attempt to index")
                    && error.contains(if label == "numeric nested" {
                        "number value"
                    } else {
                        "boolean value"
                    }),
                "{label}: {error}"
            );
            let calls = source.get::<Table>("calls").unwrap();
            assert_eq!(calls.raw_len(), 1);
            let call = calls.get::<Table>(1).unwrap();
            let rows = call
                .get::<Table>("modifiers")
                .unwrap()
                .sequence_values::<Table>()
                .map(|r| native::metadata(r.unwrap()))
                .collect::<Vec<_>>();
            let FormatResult::NeedsParser(fallback) = formatter.apply_range(input).unwrap() else {
                panic!("{label} missing parser request")
            };
            assert_eq!(fallback.parser_text(), call.get::<String>("text").unwrap());
            assert_eq!(
                formatter.resume(
                    fallback,
                    ParserFeedback {
                        modifiers: Some(&rows),
                        extra: None
                    }
                ),
                Err(FormatError::SourceError(
                    "nested value.mod is not an indexable modifier"
                )),
                "{label}"
            );
        } else {
            assert!(
                source.get::<bool>("ok").unwrap(),
                "{label}: {:?}",
                source.get::<Option<String>>("error").unwrap()
            );
            assert_eq!(native::compare(&formatter, input, &source), 1, "{label}");
        }
    }
}
#[test]
fn original_balanced_enum_preprocessing_stays_an_explicit_native_boundary() {
    use poe_optimizer_import::item_loading::*;
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let oracle = FormatterOracle::new();
    for line in [
        "+17.5 to maximum Life (foo(bar)-baz)",
        "+17.5 to maximum Life (foo-(bar))",
        "+17.5 to maximum Life (foo-bar)",
    ] {
        let raw = format!(
            "Rarity: Rare\nCaller amulet\nAmber Amulet\nItem Level: 60\nImplicits: 0\n{line}"
        );
        let source = oracle.source.load(
            &format!("<Items><Item id=\"3\">{}</Item></Items>", escape_xml(&raw)),
            false,
        );
        assert!(source.get::<bool>("ok").unwrap());
        let parse = source_events(&source, "parse_raw")
            .into_iter()
            .find(|r| !r.get::<String>("raw").unwrap().is_empty())
            .unwrap();
        let consumed = parse.get::<String>("raw").unwrap();
        let formats = source_events(&source, "format");
        assert!(!formats.is_empty());
        assert_eq!(
            formats[0].get::<String>("text").unwrap(),
            "+17.5 to maximum Life",
            "actual balanced source preprocessing"
        );
        let mut provider = NativeItemLoadProvider::with_dependencies(
            &snapshot,
            native::OriginalParser {
                oracle: &oracle,
                calls: vec![],
            },
        );
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.apply_text(&consumed, &mut provider).unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Pending);
        assert_eq!(
            machine.pending().unwrap().kind,
            DependencyKind::RangeFormatting
        );
        assert!(machine.state().format_calls.is_empty());
        assert!(machine.state().parser_calls.is_empty());
        assert!(provider.dependencies().calls.is_empty());
    }
}
