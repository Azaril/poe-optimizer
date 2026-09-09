use poe_optimizer_engine::lua_number::parse_number;

#[test]
fn decimal_syntax_preserves_signed_zero_and_rounds_at_binary_boundaries() {
    for (text, value) in [
        ("0", 0.0),
        ("-0", -0.0),
        ("+00.000e-0", 0.0),
        ("  \t\r\n\x0b\x0c-12.5E+1\t", -125.0),
        (".5", 0.5),
        ("1.", 1.0),
        ("01", 1.0),
        ("9007199254740993", 9007199254740992.0),
        ("9007199254740995", 9007199254740996.0),
        ("1.7976931348623157e308", f64::MAX),
        ("1e309", f64::INFINITY),
        ("-1e-400", -0.0),
        ("4.9406564584124654e-324", f64::from_bits(1)),
    ] {
        assert_eq!(
            parse_number(text.as_bytes()).map(f64::to_bits),
            Some(value.to_bits()),
            "{text:?}"
        );
    }
}

#[test]
fn hexadecimal_fraction_exponents_round_once_including_subnormals() {
    for (text, bits) in [
        ("0x0", 0),
        ("-0X00.p1023", 1 << 63),
        ("0x.8", 0.5f64.to_bits()),
        ("0x10", 16.0f64.to_bits()),
        ("+0X1.Ap+3", 13.0f64.to_bits()),
        ("0x1.00000000000008", 1.0f64.to_bits()),
        ("0x1.000000000000080001", 1.0f64.to_bits() + 1),
        ("0x1.00000000000018", 1.0f64.to_bits() + 2),
        ("0x1p-1074", 1),
        ("0x1p-1075", 0),
        ("0x1.0001p-1075", 1),
        ("0x3p-1075", 2),
        ("0x5p-1075", 2),
        ("0x1.fffffffffffffp-1023", 0x0010_0000_0000_0000),
        ("0x1.fffffffffffffp1023", f64::MAX.to_bits()),
        ("0x1.fffffffffffff8p1023", f64::INFINITY.to_bits()),
        ("-0x1p-999999", 1 << 63),
    ] {
        assert_eq!(
            parse_number(text.as_bytes()).map(f64::to_bits),
            Some(bits),
            "{text}"
        );
    }
}

#[test]
fn binary_literals_use_source_significant_bit_limit() {
    assert_eq!(parse_number(b"0B10101"), Some(21.0));
    assert_eq!(
        parse_number(b"-0b000").unwrap().to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(
        parse_number(format!("0b{}", "1".repeat(64)).as_bytes()),
        Some(u64::MAX as f64)
    );
    assert_eq!(
        parse_number(format!("0b{}", "1".repeat(65)).as_bytes()),
        None
    );
    assert_eq!(
        parse_number(format!("0b{}1", "0".repeat(1000)).as_bytes()),
        Some(1.0)
    );
    for text in ["0b", "0b2", "0b1.0", "0b0.0", "0b1p0", "0b1e0"] {
        assert_eq!(parse_number(text.as_bytes()), None, "{text}");
    }
}

#[test]
fn nonfinite_spellings_and_invalid_bytes_follow_luajit() {
    for text in ["nan", "+NaN", "-NAN"] {
        assert_eq!(
            parse_number(text.as_bytes()).unwrap().to_bits(),
            0xfff8_0000_0000_0000
        );
    }
    assert_eq!(parse_number(b"\t+INFINITY "), Some(f64::INFINITY));
    assert_eq!(parse_number(b"-iNF"), Some(f64::NEG_INFINITY));
    for text in [
        "",
        " ",
        "+",
        "-",
        ".",
        "+.",
        "1..0",
        "1e",
        "1e+",
        "1e-",
        "1e+-1",
        "1p0",
        "++1",
        "1 0",
        "nan(1)",
        "nanx",
        "infinit",
        "infinityx",
        "1f",
        "1LL",
        "0x",
        "0x.",
        "0x1p",
        "0x1p+",
        "0x1.2.3",
        "0x1p1.0",
        "0x1g",
        "0x1i",
        "0o12",
        "١",
        "１２",
        "\u{a0}1",
        "1\u{a0}",
        "1,5",
        "1\0",
        "1\0 ",
        "\0",
        "\0nan",
    ] {
        assert!(parse_number(text.as_bytes()).is_none(), "{text:?}");
    }
    for text in [b"1\xff".as_slice(), b"\xff1", b"1\x801"] {
        assert!(parse_number(text).is_none(), "{text:?}");
    }
}

#[test]
fn authored_exponents_are_bounded_before_numeric_overflow() {
    for text in ["1e1048575", "0x1p1048575"] {
        assert_eq!(parse_number(text.as_bytes()), Some(f64::INFINITY));
    }
    for text in [
        "1e1048576",
        "-1e-1048576",
        "0e1048576",
        "0x0p1048576",
        "0x1p-1048576",
        "1e999999999999999999999999999",
    ] {
        assert!(parse_number(text.as_bytes()).is_none(), "{text}");
    }
    assert_eq!(
        parse_number(b"1e000000000000000000000000000001"),
        Some(10.0)
    );
}

#[test]
fn fractional_bound_discounts_only_source_trailing_zeros() {
    let zeros = "0".repeat(1_048_575);
    assert_eq!(parse_number(format!("0.{zeros}1").as_bytes()), None);
    assert_eq!(parse_number(format!("0x0.{zeros}1").as_bytes()), None);
    assert_eq!(parse_number(format!("0.{zeros}").as_bytes()), Some(0.0));
    assert_eq!(parse_number(format!("1.{zeros}").as_bytes()), Some(1.0));
    assert_eq!(parse_number(format!("0.1{zeros}").as_bytes()), Some(0.1));
    assert_eq!(parse_number(format!("0x1.{zeros}").as_bytes()), Some(1.0));
}
