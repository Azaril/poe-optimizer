use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
use poe_optimizer_data::item_scalability::{
    ItemFormatAssignments, ItemScalabilityCatalog, ItemScalabilityValue,
};
use poe_optimizer_engine::item_tools::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn row(scalable: bool, labels: &[&str]) -> ItemScalabilityValue {
    ItemScalabilityValue {
        is_scalable: scalable,
        formats: Some(labels.iter().map(|s| (*s).into()).collect()),
    }
}
fn catalog(entries: Vec<(&str, Vec<ItemScalabilityValue>)>) -> ItemScalabilityCatalog {
    let mut data = snapshot().item_scalability().data().clone();
    data.entries = entries.into_iter().map(|(k, v)| (k.into(), v)).collect();
    ItemScalabilityCatalog::new(data).unwrap()
}
fn complete(
    formatter: &ItemFormatter<'_>,
    line: &str,
    range: RangeInput<'_>,
    scalar: Option<f64>,
) -> String {
    match formatter
        .apply_range(FormatInput {
            line,
            range,
            value_scalar: scalar,
            base_value_scalar: None,
        })
        .unwrap()
    {
        FormatResult::Complete(v) => v,
        FormatResult::NeedsParser(_) => panic!("unexpected fallback"),
    }
}
#[test]
fn literal_specificity_empty_rows_and_source_combination_order_are_preserved() {
    let data = catalog(vec![
        ("Choice 1 and #", vec![row(false, &[])]),
        ("Choice # and 2", vec![row(true, &[])]),
        ("Choice # and #", vec![row(true, &[]); 2]),
        ("Literal 1.20", vec![]),
        ("Literal #", vec![row(true, &[])]),
    ]);
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    assert_eq!(
        complete(&f, "Choice 1 and 2", RangeInput::Missing, Some(9.0)),
        "Choice 1 and 2"
    );
    assert_eq!(
        complete(&f, "Literal 1.20", RangeInput::Missing, Some(9.0)),
        "Literal 1.20"
    );
    assert_eq!(
        complete(&f, "Literal 1.21", RangeInput::Missing, Some(9.0)),
        "Literal 9"
    );
}
#[test]
fn ordered_partial_assignments_and_unknown_labels_are_not_inferred() {
    let mut data = catalog(vec![(
        "Caller #",
        vec![row(true, &["first", "unknown_label", "second"])],
    )])
    .data()
    .clone();
    data.format_assignments.insert(
        "first".into(),
        ItemFormatAssignments {
            precision: Some(10.0),
            display_precision: Some(1),
            if_required: Some(false),
        },
    );
    data.format_assignments.insert(
        "second".into(),
        ItemFormatAssignments {
            precision: Some(1000.0),
            ..ItemFormatAssignments::default()
        },
    );
    let data = ItemScalabilityCatalog::new(data).unwrap();
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    assert_eq!(
        complete(&f, "Caller 10.049", RangeInput::Missing, None),
        "Caller 10.0"
    );
}
#[test]
fn nine_capture_generic_key_uses_full_algorithm_beyond_packaged_arity() {
    let key = format!("Caller {}", ["#"; 9].join(" and "));
    let data = catalog(vec![(&key, vec![row(true, &[]); 9])]);
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    let line = format!("Caller {}", ["1.2"; 9].join(" and "));
    assert_eq!(
        complete(&f, &line, RangeInput::Missing, Some(2.0)),
        format!("Caller {}", ["2"; 9].join(" and "))
    );
}
#[test]
fn table_ranges_use_injected_missing_value_and_negative_antonyms() {
    let data = catalog(vec![
        ("Pair # and #", vec![row(true, &[]); 2]),
        ("#% reduced speed", vec![row(true, &[])]),
    ]);
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    assert_eq!(
        complete(
            &f,
            "Pair +(1-3) and (2-4)",
            RangeInput::Values(&[Some(0.0), None]),
            None
        ),
        "Pair +1 and 3"
    );
    assert_eq!(
        complete(
            &f,
            "-(1-3)% increased speed",
            RangeInput::Scalar(1.0),
            Some(2.0)
        ),
        "6% reduced speed"
    );
    assert!(matches!(
        f.apply_range(FormatInput {
            line: "Unknown (1-3) rolls",
            range: RangeInput::Values(&[Some(0.5)]),
            value_scalar: Some(2.0),
            base_value_scalar: None
        }),
        Err(FormatError::SourceError(_))
    ));
}
fn modifier(name: &str) -> ItemMetadataTable {
    ItemMetadataTable {
        fields: [
            ("name".into(), ItemMetadataValue::Text(name.into())),
            ("type".into(), ItemMetadataValue::Text("BASE".into())),
            ("value".into(), ItemMetadataValue::Number(1.0)),
        ]
        .into_iter()
        .collect(),
        indexed: BTreeMap::new(),
    }
}
fn fallback<'a>(f: &ItemFormatter<'a>) -> FormatFallback<'a> {
    match f
        .apply_range(FormatInput {
            line: "Unknown 10.123 units",
            range: RangeInput::Scalar(0.5),
            value_scalar: Some(1.1),
            base_value_scalar: None,
        })
        .unwrap()
    {
        FormatResult::NeedsParser(v) => v,
        _ => panic!("missing parser dependency"),
    }
}
#[test]
fn fallback_uses_last_matching_nested_modifier_and_explicit_feedback_presence() {
    let data = catalog(vec![("Other #", vec![row(true, &[])])]);
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    let nested = ItemMetadataTable {
        fields: [(
            "value".into(),
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: [(
                    "mod".into(),
                    ItemMetadataValue::Table(modifier("CritChance")),
                )]
                .into_iter()
                .collect(),
                indexed: BTreeMap::new(),
            }),
        )]
        .into_iter()
        .collect(),
        indexed: BTreeMap::new(),
    };
    assert_eq!(fallback(&f).parser_text(), "Unknown 10.123 units");
    assert_eq!(
        f.resume(
            fallback(&f),
            ParserFeedback {
                modifiers: Some(&[nested.clone(), modifier("LifeRegen")]),
                extra: None
            }
        )
        .unwrap(),
        "Unknown 11.1 units"
    );
    assert_eq!(
        f.resume(
            fallback(&f),
            ParserFeedback {
                modifiers: Some(&[modifier("LifeRegen"), nested.clone()]),
                extra: None
            }
        )
        .unwrap(),
        "Unknown 11.13 units"
    );
    assert_eq!(
        f.resume(
            fallback(&f),
            ParserFeedback {
                modifiers: Some(&[nested]),
                extra: Some("")
            }
        )
        .unwrap(),
        "Unknown 11.1 units"
    );
    let other = data.clone();
    let other = ItemFormatter::new(&other, &snapshot().package().actor);
    assert_eq!(
        other.resume(
            fallback(&f),
            ParserFeedback {
                modifiers: None,
                extra: None
            }
        ),
        Err(FormatError::BindingMismatch)
    );
}
#[test]
fn neutral_unknown_lines_do_not_request_parser_and_errors_are_distinct() {
    let data = catalog(vec![("Other #", vec![row(true, &[])])]);
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    assert_eq!(
        complete(&f, "Caller roll 10.049", RangeInput::Missing, None),
        "Caller roll 10.049"
    );
    assert!(matches!(
        f.apply_range(FormatInput {
            line: "Unknown (1-3)",
            range: RangeInput::Missing,
            value_scalar: None,
            base_value_scalar: None
        }),
        Err(FormatError::SourceError(_))
    ));
    let long = "x".repeat(MAX_FORMAT_TEXT + 1);
    assert!(matches!(
        f.apply_range(FormatInput {
            line: &long,
            range: RangeInput::Missing,
            value_scalar: None,
            base_value_scalar: None
        }),
        Err(FormatError::ResourceBound(_))
    ));
    let many = "1 ".repeat(MAX_FORMAT_CAPTURES + 1);
    assert!(matches!(
        f.apply_range(FormatInput {
            line: &many,
            range: RangeInput::Missing,
            value_scalar: None,
            base_value_scalar: None
        }),
        Err(FormatError::ResourceBound(_))
    ));
}
#[test]
fn catalyst_flags_only_augment_existing_tags_and_zero_quality_is_present() {
    let policy = &snapshot().item_scalability().data().catalyst_scaling;
    let catalysts = vec![poe_optimizer_data::item_loading::ItemCatalystDefinition {
        name: "Caller".into(),
        descriptor: "Caller".into(),
        tags: vec!["prefix".into(), "caller".into()],
    }];
    let flags = BTreeSet::from(["prefix".into()]);
    assert_eq!(
        catalyst_scalar(
            policy,
            &catalysts,
            Some(1.0),
            Some(&[]),
            &flags,
            false,
            None
        )
        .unwrap(),
        1.0
    );
    let tags = vec!["other".into()];
    assert_eq!(
        catalyst_scalar(
            policy,
            &catalysts,
            Some(1.0),
            Some(&tags),
            &flags,
            false,
            None
        )
        .unwrap(),
        1.2
    );
    assert_eq!(
        catalyst_scalar(
            policy,
            &catalysts,
            Some(1.0),
            Some(&tags),
            &flags,
            false,
            Some(0.0)
        )
        .unwrap(),
        1.0
    );
    assert_eq!(
        catalyst_scalar(
            policy,
            &catalysts,
            Some(1.0),
            Some(&tags),
            &flags,
            true,
            Some(20.0)
        )
        .unwrap(),
        1.0
    );
}

#[test]
fn nested_feedback_obeys_lua_string_table_and_scalar_indexing() {
    let data = catalog(vec![("Other #", vec![row(true, &[])])]);
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    for value in [
        ItemMetadataValue::Text("text".into()),
        ItemMetadataValue::Text(String::new()),
        ItemMetadataValue::Array(vec![ItemMetadataValue::Text("text".into())]),
    ] {
        let nested = ItemMetadataTable {
            fields: [(
                "value".into(),
                ItemMetadataValue::Table(ItemMetadataTable {
                    fields: [("mod".into(), value)].into_iter().collect(),
                    indexed: BTreeMap::new(),
                }),
            )]
            .into_iter()
            .collect(),
            indexed: BTreeMap::new(),
        };
        assert_eq!(
            f.resume(
                fallback(&f),
                ParserFeedback {
                    modifiers: Some(&[nested]),
                    extra: None
                }
            )
            .unwrap(),
            "Unknown 11.1 units"
        );
    }
    for value in [
        ItemMetadataValue::Number(0.0),
        ItemMetadataValue::Boolean(true),
    ] {
        let nested = ItemMetadataTable {
            fields: [(
                "value".into(),
                ItemMetadataValue::Table(ItemMetadataTable {
                    fields: [("mod".into(), value)].into_iter().collect(),
                    indexed: BTreeMap::new(),
                }),
            )]
            .into_iter()
            .collect(),
            indexed: BTreeMap::new(),
        };
        assert!(matches!(
            f.resume(
                fallback(&f),
                ParserFeedback {
                    modifiers: Some(&[nested]),
                    extra: None
                }
            ),
            Err(FormatError::SourceError(_))
        ));
    }
}
#[test]
fn adversarial_specialization_is_bounded_and_long_digit_runs_finish() {
    let data = catalog(vec![("Other #", vec![row(true, &[])])]);
    let f = ItemFormatter::new(&data, &snapshot().package().actor);
    let line = ["1"; 18].join(" and ");
    assert!(matches!(
        f.apply_range(FormatInput {
            line: &line,
            range: RangeInput::Scalar(1.0),
            value_scalar: Some(1.2),
            base_value_scalar: None
        }),
        Err(FormatError::ResourceBound(_))
    ));
    let line = format!("{}x", "9".repeat(100_000));
    let FormatResult::NeedsParser(pending) = f
        .apply_range(FormatInput {
            line: &line,
            range: RangeInput::Scalar(1.0),
            value_scalar: Some(1.2),
            base_value_scalar: None,
        })
        .unwrap()
    else {
        panic!("expected explicit precision dependency")
    };
    assert_eq!(
        f.resume(
            pending,
            ParserFeedback {
                modifiers: None,
                extra: None
            }
        )
        .unwrap(),
        line
    );
}
