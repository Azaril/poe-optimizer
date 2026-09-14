//! Independent lexical conversion contracts; no source fixture, UI or evaluator.
use poe_optimizer_core::{owned_build::ParameterValue, owned_definitions::*};
use poe_optimizer_import::owned_value::*;
use serde_json::{Value, json};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("value-game", "v1").unwrap()
}
fn integer(value: i64) -> BoundedInteger {
    BoundedInteger::new(value).unwrap()
}
fn unit() -> UnitDefId {
    UnitDefId::parse(ns(), "owned-unit").unwrap()
}
fn option(name: &str) -> OptionDefId {
    OptionDefId::parse(ns(), name).unwrap()
}
fn input(codec: ValueCodecKind, whitespace: WhitespacePolicy) -> ValueCodecInput {
    ValueCodecInput {
        namespace: ns(),
        whitespace,
        codec,
    }
}
fn codec(kind: ValueCodecKind) -> OwnedValueCodec {
    OwnedValueCodec::new(
        input(kind, WhitespacePolicy::Exact),
        OwnedValueLimits::default(),
    )
    .unwrap()
}
fn integers(syntax: DecimalSyntax) -> OwnedValueCodec {
    codec(ValueCodecKind::Integer { syntax })
}
fn quantities(numerator: i64, denominator: i64) -> OwnedValueCodec {
    codec(ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Scientific,
        unit: unit(),
        scale: RationalScale {
            numerator: integer(numerator),
            denominator: integer(denominator),
        },
    })
}
fn bool_rows(rows: &[(&str, bool)]) -> ValueCodecKind {
    ValueCodecKind::Boolean {
        tokens: rows
            .iter()
            .map(|(token, value)| BooleanToken {
                token: (*token).into(),
                value: *value,
            })
            .collect(),
    }
}
fn assert_integer(codec: &OwnedValueCodec, source: &str, expected: i64) {
    assert_eq!(
        codec.decode(source).unwrap(),
        ParameterValue::Integer(integer(expected)),
        "{source}"
    );
}
fn quantity_value(value: ParameterValue) -> FiniteQuantity {
    match value {
        ParameterValue::Quantity(value) => value,
        other => panic!("expected quantity, got {other:?}"),
    }
}

#[test]
fn integer_bounds_are_exact_before_any_floating_point_rounding() {
    let codec = integers(DecimalSyntax::Scientific);
    for (source, value) in [
        ("9007199254740991", BoundedInteger::MAX),
        ("-9007199254740991", BoundedInteger::MIN),
        ("+09007199254740991.000", BoundedInteger::MAX),
        ("9.007199254740991e15", BoundedInteger::MAX),
        ("-90071992547409910e-1", BoundedInteger::MIN),
        ("9007199254740990", BoundedInteger::MAX - 1),
    ] {
        assert_integer(&codec, source, value);
    }
    for source in [
        "9007199254740992",
        "-9007199254740992",
        "9.007199254740992e15",
        "90071992547409910",
        "1e999999999999999999999",
    ] {
        assert_eq!(
            codec.decode(source),
            Err(ValueDecodeError::IntegerOutOfRange),
            "{source}"
        );
    }
    // These spellings would lose their fractional evidence if parsed through f64.
    for source in [
        "9007199254740991.1",
        "9007199254740990.9",
        "-9007199254740991.0000000001",
    ] {
        assert_eq!(
            codec.decode(source),
            Err(ValueDecodeError::NonIntegralInteger),
            "{source}"
        );
    }
}

#[test]
fn decimal_and_exponent_integrality_uses_digits_including_zero_extremes() {
    let codec = integers(DecimalSyntax::Scientific);
    for (source, expected) in [
        ("-0", 0),
        ("+0e999999999999999", 0),
        ("-0e-99999999999999", 0),
        ("000123.000e0", 123),
        ("12300e-2", 123),
        (".125e3", 125),
        ("1.e1", 10),
        ("10e-1", 1),
        ("-00001000e-3", -1),
        ("000.0001e4", 1),
        ("1.20e+1", 12),
        ("1E00000000000000000001", 10),
    ] {
        assert_integer(&codec, source, expected);
    }
    for source in [
        "1e-999999999999999",
        "1.1",
        "10e-2",
        ".0001e3",
        "1001e-3",
        "1.00000000000000000001",
    ] {
        assert_eq!(
            codec.decode(source),
            Err(ValueDecodeError::NonIntegralInteger),
            "{source}"
        );
    }
    let long = format!("{}1{}e-1000", "0".repeat(1000), "0".repeat(1000));
    assert_integer(&codec, &long, 1);
}

#[test]
fn admitted_numeric_grammar_is_explicit_and_never_an_expression_language() {
    let whole = integers(DecimalSyntax::Integer);
    assert_integer(&whole, "+123", 123);
    for source in ["1.0", "1e0", ".1"] {
        assert_eq!(
            whole.decode(source),
            Err(ValueDecodeError::MalformedDecimal)
        );
    }
    let decimal = integers(DecimalSyntax::Decimal);
    assert_integer(&decimal, "1.0", 1);
    assert_eq!(
        decimal.decode("1e0"),
        Err(ValueDecodeError::MalformedDecimal)
    );
    let scientific = integers(DecimalSyntax::Scientific);
    for source in [
        "",
        "+",
        "-",
        ".",
        "1e",
        "1e+",
        "1e-",
        "--1",
        "NaN",
        "inf",
        "-Infinity",
        "0x10",
        "1_000",
        "1/2",
        "1 + 2",
        "1.2.3",
        "1e2e3",
        "12\0",
        "１２",
        "1 2",
    ] {
        assert_eq!(
            scientific.decode(source),
            Err(ValueDecodeError::MalformedDecimal),
            "{source:?}"
        );
    }
}

#[test]
fn exact_boolean_tables_keep_false_empty_and_unknown_distinct() {
    let codec = codec(bool_rows(&[
        ("enabled", true),
        ("disabled", false),
        ("", false),
    ]));
    assert_eq!(
        codec.decode("enabled").unwrap(),
        ParameterValue::Boolean(true)
    );
    assert_eq!(
        codec.decode("disabled").unwrap(),
        ParameterValue::Boolean(false)
    );
    assert_eq!(codec.decode("").unwrap(), ParameterValue::Boolean(false));
    for source in ["true", "false", "nil", "0", "Enabled", " enabled "] {
        assert_eq!(codec.decode(source), Err(ValueDecodeError::UnknownToken));
    }
    let empty = codec_empty_table();
    assert_eq!(empty.decode(""), Err(ValueDecodeError::UnknownToken));
}
fn codec_empty_table() -> OwnedValueCodec {
    codec(bool_rows(&[]))
}

#[test]
fn whitespace_policy_is_explicit_ascii_only_and_detects_colliding_keys() {
    let raw = input(
        bool_rows(&[("yes", true), (" yes", false)]),
        WhitespacePolicy::Exact,
    );
    let exact = OwnedValueCodec::new(raw.clone(), OwnedValueLimits::default()).unwrap();
    assert_eq!(exact.decode("yes").unwrap(), ParameterValue::Boolean(true));
    assert_eq!(
        exact.decode(" yes").unwrap(),
        ParameterValue::Boolean(false)
    );
    let trim = ValueCodecInput {
        whitespace: WhitespacePolicy::TrimAscii,
        ..raw
    };
    assert!(matches!(
        OwnedValueCodec::new(trim, OwnedValueLimits::default()),
        Err(ValueCodecError::DuplicateToken {
            first: 0,
            second: 1
        })
    ));
    let trimmed = OwnedValueCodec::new(
        input(
            bool_rows(&[("\tyes\r\n", true)]),
            WhitespacePolicy::TrimAscii,
        ),
        OwnedValueLimits::default(),
    )
    .unwrap();
    assert_eq!(
        trimmed.decode(" \tyes\n").unwrap(),
        ParameterValue::Boolean(true)
    );
    assert_eq!(
        trimmed.decode("\u{a0}yes\u{a0}"),
        Err(ValueDecodeError::UnknownToken)
    );
    let numeric = OwnedValueCodec::new(
        input(
            ValueCodecKind::Integer {
                syntax: DecimalSyntax::Decimal,
            },
            WhitespacePolicy::TrimAscii,
        ),
        OwnedValueLimits::default(),
    )
    .unwrap();
    assert_integer(&numeric, "\r\n 12.0\t", 12);
    assert_eq!(
        numeric.decode("\u{a0}12\u{a0}"),
        Err(ValueDecodeError::MalformedDecimal)
    );
    let duplicate = input(
        bool_rows(&[("yes", true), ("yes", true)]),
        WhitespacePolicy::Exact,
    );
    assert!(matches!(
        OwnedValueCodec::new(duplicate, OwnedValueLimits::default()),
        Err(ValueCodecError::DuplicateToken { .. })
    ));
}

#[test]
fn option_tokens_preserve_exact_multiline_text_and_typed_namespace() {
    let kind = ValueCodecKind::Option {
        tokens: vec![
            OptionToken {
                token: "first\nsecond\n".into(),
                value: option("first-option"),
            },
            OptionToken {
                token: "".into(),
                value: option("empty-option"),
            },
        ],
    };
    let codec = codec(kind.clone());
    assert_eq!(
        codec.decode("first\nsecond\n").unwrap(),
        ParameterValue::Option(option("first-option"))
    );
    assert_eq!(
        codec.decode("").unwrap(),
        ParameterValue::Option(option("empty-option"))
    );
    for source in [
        "first second",
        "first\nsecond",
        "First\nsecond\n",
        "unknown",
    ] {
        assert_eq!(codec.decode(source), Err(ValueDecodeError::UnknownToken));
    }
    let ValueCodecKind::Option { mut tokens } = kind else {
        unreachable!()
    };
    tokens[0].value = OptionDefId::parse(
        GameVersionNamespace::new("value-game", "v2").unwrap(),
        "first-option",
    )
    .unwrap();
    assert!(matches!(
        OwnedValueCodec::new(
            input(ValueCodecKind::Option { tokens }, WhitespacePolicy::Exact),
            OwnedValueLimits::default()
        ),
        Err(ValueCodecError::ForeignOptionNamespace { index: 0 })
    ));
    let duplicate = ValueCodecKind::Option {
        tokens: vec![
            OptionToken {
                token: "same".into(),
                value: option("a"),
            },
            OptionToken {
                token: "same".into(),
                value: option("b"),
            },
        ],
    };
    assert!(matches!(
        OwnedValueCodec::new(
            input(duplicate, WhitespacePolicy::Exact),
            OwnedValueLimits::default()
        ),
        Err(ValueCodecError::DuplicateToken { .. })
    ));
}

#[test]
fn rational_quantities_preserve_units_negative_values_and_normalized_zero() {
    let negative_scale = quantities(-3, 2);
    let result = quantity_value(negative_scale.decode("2").unwrap());
    assert_eq!(result.value(), -3.0);
    assert_eq!(result.unit(), &unit());
    assert_eq!(
        quantity_value(negative_scale.decode("-2").unwrap()).value(),
        3.0
    );
    let result = quantity_value(negative_scale.decode("0").unwrap());
    assert_eq!(result.value().to_bits(), 0.0_f64.to_bits());
    assert_eq!(
        quantity_value(quantities(1, 1).decode("-0").unwrap())
            .value()
            .to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(
        quantity_value(quantities(0, BoundedInteger::MAX).decode("1e300").unwrap()).value(),
        0.0
    );
    assert_eq!(
        quantity_value(quantities(1, 100).decode("25").unwrap()).value(),
        0.25
    );
    assert_eq!(
        quantity_value(
            quantities(BoundedInteger::MAX, BoundedInteger::MAX)
                .decode("1e308")
                .unwrap()
        )
        .value(),
        1e308
    );
}

#[test]
fn quantity_failures_do_not_become_zero_or_an_alternate_value() {
    for source in ["NaN", "Infinity", "0/0", "", "1 + 2"] {
        assert_eq!(
            quantities(0, 1).decode(source),
            Err(ValueDecodeError::MalformedDecimal)
        );
    }
    assert_eq!(
        quantities(0, 1).decode("1e309"),
        Err(ValueDecodeError::NonFiniteInput)
    );
    assert_eq!(
        quantities(2, 1).decode("1e308"),
        Err(ValueDecodeError::NonFiniteResult)
    );
    for denominator in [0, -1] {
        let raw = input(
            ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Scientific,
                unit: unit(),
                scale: RationalScale {
                    numerator: integer(1),
                    denominator: integer(denominator),
                },
            },
            WhitespacePolicy::Exact,
        );
        assert!(matches!(
            OwnedValueCodec::new(raw, OwnedValueLimits::default()),
            Err(ValueCodecError::InvalidScaleDenominator)
        ));
    }
    let foreign = UnitDefId::parse(
        GameVersionNamespace::new("another-game", "v1").unwrap(),
        "owned-unit",
    )
    .unwrap();
    let raw = input(
        ValueCodecKind::Quantity {
            syntax: DecimalSyntax::Integer,
            unit: foreign,
            scale: RationalScale {
                numerator: integer(1),
                denominator: integer(1),
            },
        },
        WhitespacePolicy::Exact,
    );
    assert!(matches!(
        OwnedValueCodec::new(raw, OwnedValueLimits::default()),
        Err(ValueCodecError::ForeignUnitNamespace)
    ));
}

#[test]
fn every_configurable_resource_ceiling_is_checked_inclusively() {
    let raw = input(
        ValueCodecKind::Integer {
            syntax: DecimalSyntax::Integer,
        },
        WhitespacePolicy::Exact,
    );
    for (resource, maximum) in [
        (ValueResource::SourceBytes, HARD_VALUE_SOURCE_BYTES),
        (ValueResource::TokenBytes, HARD_VALUE_TOKEN_BYTES),
        (ValueResource::Tokens, HARD_VALUE_TOKENS),
        (ValueResource::TotalTokenBytes, HARD_VALUE_TOTAL_TOKEN_BYTES),
    ] {
        for amount in [0, maximum + 1, maximum] {
            let mut limits = OwnedValueLimits::default();
            match resource {
                ValueResource::SourceBytes => limits.max_source_bytes = amount,
                ValueResource::TokenBytes => limits.max_token_bytes = amount,
                ValueResource::Tokens => limits.max_tokens = amount,
                ValueResource::TotalTokenBytes => limits.max_total_token_bytes = amount,
            }
            let result = OwnedValueCodec::new(raw.clone(), limits);
            if amount == maximum {
                result.unwrap();
            } else {
                assert!(
                    matches!(result, Err(ValueCodecError::InvalidLimit { resource: found, .. }) if found == resource)
                );
            }
        }
    }
}

#[test]
fn actual_source_and_token_budgets_apply_before_whitespace_transformation() {
    let small = OwnedValueLimits {
        max_source_bytes: 2,
        max_token_bytes: 2,
        max_tokens: 2,
        max_total_token_bytes: 3,
    };
    let codec = OwnedValueCodec::new(
        input(
            bool_rows(&[("a", true), ("é", false)]),
            WhitespacePolicy::TrimAscii,
        ),
        small,
    )
    .unwrap();
    assert_eq!(codec.decode("é").unwrap(), ParameterValue::Boolean(false));
    assert_eq!(codec.decode(" a").unwrap(), ParameterValue::Boolean(true));
    assert_eq!(
        codec.decode(" a "),
        Err(ValueDecodeError::SourceTooLarge {
            actual: 3,
            maximum: 2
        })
    );
    let cases = [
        (bool_rows(&[("€", true)]), ValueResource::TokenBytes),
        (
            bool_rows(&[("a", true), ("b", false), ("c", true)]),
            ValueResource::Tokens,
        ),
        (
            bool_rows(&[("aa", true), ("bb", false)]),
            ValueResource::TotalTokenBytes,
        ),
        (bool_rows(&[(" a ", true)]), ValueResource::TokenBytes),
    ];
    for (kind, resource) in cases {
        assert!(
            matches!(OwnedValueCodec::new(input(kind, WhitespacePolicy::TrimAscii), small), Err(ValueCodecError::ResourceLimit { resource: found, .. }) if found == resource)
        );
    }
    let empty = OwnedValueCodec::new(
        input(bool_rows(&[("", true)]), WhitespacePolicy::TrimAscii),
        small,
    )
    .unwrap();
    assert_eq!(empty.decode("  ").unwrap(), ParameterValue::Boolean(true));
    assert!(matches!(
        empty.decode("   "),
        Err(ValueDecodeError::SourceTooLarge { .. })
    ));
}

#[test]
fn raw_codec_serde_is_strict_and_has_no_implicit_fields_or_type_coercions() {
    let raw = input(bool_rows(&[("enabled", true)]), WhitespacePolicy::Exact);
    let value = serde_json::to_value(&raw).unwrap();
    for field in ["namespace", "whitespace", "codec"] {
        let mut bad = value.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ValueCodecInput>(bad).is_err());
    }
    for pointer in ["", "/codec", "/codec/value/tokens/0"] {
        let mut bad = value.clone();
        bad.pointer_mut(pointer).unwrap()["expression"] = json!("not supported");
        assert!(serde_json::from_value::<ValueCodecInput>(bad).is_err());
    }
    let mut bad = value;
    bad["codec"]["value"]["tokens"][0]["value"] = json!("true");
    assert!(serde_json::from_value::<ValueCodecInput>(bad).is_err());
    let quantity = quantities(1, 1);
    let mut bad = serde_json::to_value(quantity.input()).unwrap();
    bad["codec"]["value"]["scale"]["numerator"] = json!(9_007_199_254_740_992_u64);
    assert!(serde_json::from_value::<ValueCodecInput>(bad).is_err());
    let mut bad = serde_json::to_value(quantity.input()).unwrap();
    bad["codec"]["value"]["unit"]["kind"] = json!("option");
    assert!(serde_json::from_value::<ValueCodecInput>(bad).is_err());
}

#[test]
fn codecs_roundtrip_their_injected_policy_without_defaulting_or_mutation() {
    let kinds = [
        bool_rows(&[("second", false), ("first", true)]),
        ValueCodecKind::Integer {
            syntax: DecimalSyntax::Scientific,
        },
        quantities(-3, 2).input().codec.clone(),
        ValueCodecKind::Option {
            tokens: vec![OptionToken {
                token: "selected".into(),
                value: option("selected-option"),
            }],
        },
    ];
    for kind in kinds {
        let raw = input(kind, WhitespacePolicy::TrimAscii);
        let encoded = serde_json::to_vec(&raw).unwrap();
        let restored: ValueCodecInput = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, raw);
        let codec = OwnedValueCodec::new(restored, OwnedValueLimits::default()).unwrap();
        let before: Value = serde_json::to_value(codec.input()).unwrap();
        let _ = codec.decode("present-but-unknown");
        assert_eq!(serde_json::to_value(codec.input()).unwrap(), before);
    }
}
