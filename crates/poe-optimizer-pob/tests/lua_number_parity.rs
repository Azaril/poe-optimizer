//! Independent bundled LuaJIT tonumber parity: source results are exact IEEE bits.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Value};
use poe_optimizer_engine::lua_number::parse_number;
use std::collections::BTreeSet;
fn original(lua: &Lua, parser: &Function, input: &[u8]) -> Option<u64> {
    match parser
        .call::<Value>(lua.create_string(input).unwrap())
        .unwrap()
    {
        Value::Nil => None,
        Value::Number(value) => Some(value.to_bits()),
        Value::Integer(value) => Some((value as f64).to_bits()),
        value => panic!("tonumber returned {}", value.type_name()),
    }
}
fn compare(inputs: BTreeSet<Vec<u8>>) {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    let parser: Function = lua.globals().get("tonumber").unwrap();
    let mut failures = vec![];
    for input in &inputs {
        let expected = original(&lua, &parser, input);
        let actual = parse_number(input).map(f64::to_bits);
        if actual != expected {
            if failures.len() < 40 {
                eprintln!(
                    "tonumber mismatch {input:?}: original={expected:x?}, native={actual:x?}"
                );
            }
            failures.push(input);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} numeric conversion observations differ",
        failures.len(),
        inputs.len()
    );
    eprintln!("Exact original tonumber observations: {}", inputs.len());
}
#[test]
fn lua_tonumber_byte_syntax_and_authored_exponent_limits() {
    let mut inputs: BTreeSet<Vec<u8>> = [
        "",
        " ",
        "+",
        "-",
        ".",
        "+.",
        "0",
        "-0",
        "+0",
        "0000",
        "-000",
        "1.",
        ".1",
        "-.0",
        "1e",
        "1e+",
        "1e-",
        "1e2",
        "1.2.3",
        " 1 2 ",
        "0x",
        "0x.",
        "0x.p1",
        "0x1",
        "-0x0",
        "0x.1",
        "0x1.",
        "0x1p-1075",
        "0x1.00000000000001p-1075",
        "0x1fffffffffffff",
        "0x20000000000001",
        "0x1.fffffffffffff8p1023",
        "0x1e2",
        "0x1e-2",
        "0x1p",
        "0b",
        "0b0",
        "-0b0",
        "0b.1",
        "0b1p2",
        "0b2",
        "nan",
        "NaN",
        "-nan",
        "+nan",
        "nan()",
        "nan(1)",
        "Inf",
        "infinity",
        "-INFINITY",
        "+Inf",
        "infinite",
        "1f",
        "2u",
        "3LL",
        "0x10ULL",
    ]
    .into_iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    for byte in 0..=255 {
        for base in [b"1".as_slice(), b"nan", b"0x1", b"0b1"] {
            for at in 0..=base.len() {
                let mut text = base.to_vec();
                text.insert(at, byte);
                inputs.insert(text);
            }
        }
    }
    for prefix in ["", "-", "+"] {
        for body in ["1e", "0e", "1.0e", "0x1p", "0x0p", "0x1.0p"] {
            for exponent in [
                "1048575",
                "1048576",
                "1048577",
                "-1048575",
                "-1048576",
                "+1048575",
                "+1048576",
                "000001048575",
                "000001048576",
                "999999999999999999999999999999999999",
            ] {
                inputs.insert(format!("{prefix}{body}{exponent}").into_bytes());
            }
        }
    }
    for significant in [0, 1, 31, 32, 53, 63, 64, 65, 66, 128] {
        for prefix in ["", "+", "-"] {
            for zeros in [0, 1, 80] {
                inputs.insert(
                    format!("{prefix}0b{}{}", "0".repeat(zeros), "1".repeat(significant))
                        .into_bytes(),
                );
            }
        }
    }
    compare(inputs);
}
#[test]
fn lua_tonumber_decimal_and_hex_rounding_are_bit_exact() {
    let mut inputs = BTreeSet::new();
    let mut seed = 0x9e3779b97f4a7c15u64;
    for index in 0..12000 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let value = f64::from_bits(seed);
        if value.is_finite() {
            inputs.insert(value.to_string().into_bytes());
            inputs.insert(format!("{value:.17e}").into_bytes());
            inputs.insert(format!("{value:.40e}").into_bytes());
        }
        let exponent = (seed.rotate_left(23) % 2400) as i32 - 1200;
        inputs.insert(
            format!(
                "{}0x{:x}.{:016x}p{exponent}",
                if index % 2 == 0 { "-" } else { "" },
                seed >> 60,
                seed.rotate_left(13)
            )
            .into_bytes(),
        );
        inputs.insert(format!("{}e{exponent}", seed).into_bytes());
    }
    for exponent in [
        -1076, -1075, -1074, -1073, -1024, -1023, -1022, -1021, -54, -53, -52, -1, 0, 1, 51, 52,
        53, 1022, 1023, 1024,
    ] {
        for mantissa in [
            "0",
            "1",
            "1.0",
            "1.00000000000007",
            "1.00000000000008",
            "1.00000000000009",
            "1.fffffffffffff7",
            "1.fffffffffffff8",
            "1.fffffffffffff9",
            "f.ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        ] {
            for sign in ["", "-"] {
                inputs.insert(format!("{sign}0x{mantissa}p{exponent}").into_bytes());
            }
        }
    }
    for value in [
        "2.4703282292062327208828439643411068618252990130716238221279284125033775363510437593264991818081799618989828234772285886546332835517796989819938739800539093906315035659515570226392290858392449105184435931802849936536152500319370457678249219365623669863658480757001585769269903706311928279558551332927834338409351978015531246597263579574622766465272827220056374006485499977096599470454020828166226237857393450736339007967761930577506740176324673600968951340535537458516661342223766678604162159680461914467291840300530057530849048765391711386591646239524912623653881879636239373280423891018672348497668235089863388587925628302755995657524455507255189313690836254779186948667994968324049705821028513185451396213837722826145437693412532098591327667236328125e-324",
        "9007199254740993",
        "9007199254740995",
        "1.7976931348623157e308",
        "1.7976931348623158e308",
        "1.7976931348623159e308",
        "2.2250738585072011e-308",
        "2.2250738585072012e-308",
        "2.2250738585072013e-308",
        "4.9406564584124654e-324",
    ] {
        inputs.insert(value.as_bytes().to_vec());
    }
    compare(inputs);
}

#[test]
fn lua_tonumber_fraction_length_limits_follow_original_source() {
    let mut inputs = BTreeSet::new();
    for count in [1_048_574, 1_048_575, 1_048_576] {
        for (prefix, last) in [
            ("0.", "1"),
            ("-0.", "1"),
            ("0x0.", "1"),
            ("0.", "0"),
            ("0.1", ""),
        ] {
            inputs.insert(format!("{prefix}{}{last}", "0".repeat(count)).into_bytes());
        }
    }
    compare(inputs);
}
