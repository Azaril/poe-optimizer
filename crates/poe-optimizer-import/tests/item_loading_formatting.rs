use poe_optimizer_data::game_data::{
    GameDataLoader, GameDataSnapshot, LoadLimits, TrustPolicy, bundled_snapshot,
};
use poe_optimizer_data::item_scalability::{ItemFormatAssignments, ItemScalabilityValue};
use poe_optimizer_import::item_loading::*;
use std::sync::OnceLock;
fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
struct Recognized;
impl ItemLoadProvider for Recognized {
    fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
        DependencyResult::Available(ParseOutcome {
            modifiers: Some(vec![]),
            extra: None,
        })
    }
}
#[test]
fn builtin_known_formatting_and_parsing_feed_owned_native_assembly() {
    let data = snapshot();
    let mut provider = BuiltinItemLoadProvider::new(data);
    let mut machine = ItemLoadMachine::new(data.item_loading());
    machine
        .apply_text(
            "Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\n+17.5 to Strength",
            &mut provider,
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Complete);
    assert!(machine.pending().is_none());
    assert!(machine.assembly_progress().unwrap().is_complete());
    assert!(
        machine.assembled().is_none(),
        "registration still needs final load"
    );
    assert_eq!(machine.state().parser_calls[0].text, "+18 to Strength");
    assert!(machine.state().format_parser_calls.is_empty());
    assert_eq!(
        machine.state().explicit_mod_lines[0].modifiers[0].fields["value"].as_f64(),
        Some(18.0)
    );
}
#[test]
fn caller_scalability_changes_only_its_own_exact_key_and_data_identity() {
    let data = snapshot();
    let mut package = data.package().clone();
    package.item_scalability.entries.insert(
        "Caller roll #".into(),
        vec![ItemScalabilityValue {
            is_scalable: true,
            formats: Some(vec!["caller_fixed".into()]),
        }],
    );
    package.item_scalability.format_assignments.insert(
        "caller_fixed".into(),
        ItemFormatAssignments {
            precision: Some(1000.0),
            display_precision: Some(1),
            if_required: Some(false),
        },
    );
    package.refresh_section_digests().unwrap();
    let custom = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    for (data, expected) in [(data, "Caller roll 10.049"), (&custom, "Caller roll 10.0")] {
        let mut provider =
            NativeItemLoadProvider::with_dependencies(data, UnavailableItemLoadProvider);
        let mut machine = ItemLoadMachine::new(data.item_loading());
        machine
            .apply_text(
                "Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\nCaller roll 10.049",
                &mut provider,
            )
            .unwrap();
        assert_eq!(
            machine.pending().unwrap().kind,
            DependencyKind::ModifierParser
        );
        assert_eq!(machine.state().parser_calls.len(), 1);
        assert_eq!(machine.state().parser_calls[0].text, expected);
        assert!(machine.state().format_parser_calls.is_empty());
    }
    assert_ne!(data.identity(), custom.identity());
}
#[test]
fn selected_range_is_retained_while_initial_parse_uses_maximum_range() {
    let data = snapshot();
    let mut provider = NativeItemLoadProvider::with_dependencies(data, Recognized);
    let mut machine = ItemLoadMachine::new(data.item_loading());
    machine
        .apply_text(
            "Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\n{range:0.25}+(1-5) to Strength",
            &mut provider,
        )
        .unwrap();
    assert_eq!(machine.pending().unwrap().kind, DependencyKind::Assembly);
    assert_eq!(
        machine.state().format_calls[0].range,
        ItemNumber::Finite(1.0)
    );
    assert_eq!(machine.state().parser_calls[0].text, "+5 to Strength");
    assert_eq!(
        machine.state().explicit_mod_lines[0].line,
        "+(1-5) to Strength"
    );
    assert_eq!(
        machine.state().explicit_mod_lines[0].range,
        ItemNumber::Finite(0.25)
    );
}
#[test]
fn native_fallback_requests_and_direct_modifier_parse_have_distinct_ordered_evidence() {
    let data = snapshot();
    let raw = "Rarity: NORMAL\nRusted Greathelm\nCatalyst: Flesh\nCatalystQuality: 20\nImplicits: 0\n{tags:life}Caller 10.123 units";
    let mut provider = NativeItemLoadProvider::with_dependencies(data, UnavailableItemLoadProvider);
    let mut machine = ItemLoadMachine::new(data.item_loading());
    machine.apply_text(raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().unwrap().kind,
        DependencyKind::ModifierParser
    );
    assert!(machine.state().parser_calls.is_empty());
    assert_eq!(machine.state().format_calls[0].sequence, 0);
    assert_eq!(machine.state().format_parser_calls[0].request.sequence, 1);
    assert_eq!(
        machine.state().format_parser_calls[0].request.text,
        "Caller 10.123 units"
    );
    assert!(matches!(
        machine.state().format_parser_calls[0].result,
        DependencyResult::Unavailable(_)
    ));
    let mut provider = NativeItemLoadProvider::with_dependencies(data, Recognized);
    let mut machine = ItemLoadMachine::new(data.item_loading());
    machine.apply_text(raw, &mut provider).unwrap();
    assert_eq!(machine.pending().unwrap().kind, DependencyKind::Assembly);
    assert_eq!(machine.state().parser_calls[0].sequence, 2);
    assert_eq!(machine.state().parser_calls[0].text, "Caller 12.1 units");
    assert_eq!(
        machine.state().explicit_mod_lines[0].value_scalar,
        ItemNumber::Finite(1.2)
    );
}
#[test]
fn formatter_errors_are_not_unavailable_and_trace_messages_remain_bounded() {
    struct Source;
    impl ItemLoadProvider for Source {
        fn format_line(&mut self, _: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::SourceError("source arithmetic failed".into())
        }
    }
    let data = snapshot();
    let mut machine = ItemLoadMachine::new(data.item_loading());
    assert!(
        machine
            .apply_text(
                "Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\nLine",
                &mut Source
            )
            .is_err()
    );
    assert_eq!(machine.status(), ItemLoadStatus::SourceError);
    assert!(machine.pending().is_none());
    struct InvalidTrace;
    impl ItemLoadProvider for InvalidTrace {
        fn format_with_trace(&mut self, r: &FormatRequest) -> FormatOutcome {
            FormatOutcome {
                result: DependencyResult::Unavailable(
                    "x".repeat(MAX_ITEM_LOADING_DEPENDENCY_MESSAGE + 1),
                ),
                precision_parser_calls: vec![FormatParserCall {
                    request: ParseRequest {
                        sequence: r.sequence + 1,
                        line_index: r.line_index,
                        origin: None,
                        text: "Line".into(),
                        combined: false,
                    },
                    result: DependencyResult::Unavailable(
                        "x".repeat(MAX_ITEM_LOADING_DEPENDENCY_MESSAGE + 1),
                    ),
                }],
            }
        }
    }
    let mut machine = ItemLoadMachine::new(data.item_loading());
    assert!(
        machine
            .apply_text(
                "Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\nLine",
                &mut InvalidTrace
            )
            .unwrap_err()
            .to_string()
            .contains("message bound")
    );
    assert!(machine.state().format_parser_calls.is_empty());
}

#[test]
fn unscalable_suffixes_are_single_pass_source_order_and_unicode_exact() {
    let data = snapshot();
    for (authored, expected) in [
        (
            "Line - Unscalable Value - Unscalable Value",
            "Line - Unscalable Value",
        ),
        (
            "Line \u{2014} Unscalable Value \u{2014} Unscalable Value",
            "Line \u{2014} Unscalable Value",
        ),
        ("Line \u{2014} Unscalable Value - Unscalable Value", "Line"),
        (
            "Line - Unscalable Value \u{2014} Unscalable Value",
            "Line - Unscalable Value",
        ),
        (
            "Line \u{fffd} Unscalable Value",
            "Line \u{fffd} Unscalable Value",
        ),
    ] {
        let mut provider =
            NativeItemLoadProvider::with_dependencies(data, UnavailableItemLoadProvider);
        let mut machine = ItemLoadMachine::new(data.item_loading());
        machine
            .apply_text(
                &format!("Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\n{authored}"),
                &mut provider,
            )
            .unwrap();
        assert_eq!(
            machine.pending().unwrap().kind,
            DependencyKind::ModifierParser
        );
        assert_eq!(machine.state().parser_calls[0].text, expected, "{authored}");
    }
}
#[test]
fn balanced_nested_enum_preprocessing_remains_explicitly_pending() {
    let data = snapshot();
    for line in [
        "Line (foo(bar)-baz)",
        "Line (foo-(bar)baz)",
        "Line (a(b(c-d)))",
        "12(10)",
    ] {
        let mut provider =
            NativeItemLoadProvider::with_dependencies(data, UnavailableItemLoadProvider);
        let mut machine = ItemLoadMachine::new(data.item_loading());
        machine
            .apply_text(
                &format!("Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\n{line}"),
                &mut provider,
            )
            .unwrap();
        assert_eq!(
            machine.pending().unwrap().kind,
            DependencyKind::RangeFormatting,
            "{line}"
        );
        assert!(machine.state().format_calls.is_empty());
        assert!(machine.state().parser_calls.is_empty());
    }
}

#[test]
fn native_precision_provider_diagnostic_is_bounded_before_trace_duplication() {
    struct Oversized;
    impl ItemLoadProvider for Oversized {
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Unavailable("x".repeat(MAX_ITEM_LOADING_DEPENDENCY_MESSAGE + 1))
        }
    }
    let data = snapshot();
    let mut provider = NativeItemLoadProvider::with_dependencies(data, Oversized);
    let result = provider.format_with_trace(&FormatRequest {
        sequence: 0,
        line_index: Some(1),
        text: "Caller 10.123 units".into(),
        range: ItemNumber::Finite(1.0),
        scalar: ItemNumber::Finite(1.2),
        corrupted_range: ItemNumber::Nil,
    });
    assert!(matches!(result.result, DependencyResult::ResourceError(_)));
    assert!(result.precision_parser_calls.is_empty());
}

#[test]
fn custom_formatter_cannot_resume_after_failed_precision_dependency() {
    struct Contradictory {
        kind: u8,
        later_call: bool,
    }
    impl ItemLoadProvider for Contradictory {
        fn format_with_trace(&mut self, r: &FormatRequest) -> FormatOutcome {
            let failure = match self.kind {
                0 => DependencyResult::Unavailable("pending".into()),
                1 => DependencyResult::SourceError("source".into()),
                _ => DependencyResult::ResourceError("bounded".into()),
            };
            let mut calls = vec![FormatParserCall {
                request: ParseRequest {
                    sequence: r.sequence + 1,
                    line_index: r.line_index,
                    origin: None,
                    text: r.text.clone(),
                    combined: false,
                },
                result: failure,
            }];
            if self.later_call {
                calls.push(FormatParserCall {
                    request: ParseRequest {
                        sequence: r.sequence + 2,
                        line_index: r.line_index,
                        origin: None,
                        text: r.text.clone(),
                        combined: false,
                    },
                    result: DependencyResult::Available(ParseOutcome {
                        modifiers: Some(vec![]),
                        extra: None,
                    }),
                });
            }
            FormatOutcome {
                result: DependencyResult::Available("Invented success".into()),
                precision_parser_calls: calls,
            }
        }
    }
    let data = snapshot();
    for kind in 0..3 {
        for later_call in [false, true] {
            let mut machine = ItemLoadMachine::new(data.item_loading());
            let error = machine
                .apply_text(
                    "Rarity: NORMAL\nRusted Greathelm\nImplicits: 0\nLine",
                    &mut Contradictory { kind, later_call },
                )
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("inconsistent format precision parser result")
            );
            assert!(machine.state().format_parser_calls.is_empty());
            assert!(machine.state().parser_calls.is_empty());
            assert!(machine.state().explicit_mod_lines.is_empty());
        }
    }
}
