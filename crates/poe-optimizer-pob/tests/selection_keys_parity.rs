//! Differential observations against the bundled, JIT-disabled LuaJIT runtime.
//! These cover fresh numeric-key tables, not arbitrary Lua table behavior.
#![cfg(not(target_arch = "wasm32"))]

use mlua::{Function, Lua, Table};
use poe_optimizer_engine::{
    lua_number::parse_number,
    selection_keys::{NumericSetKeyError, NumericSetKeys},
};

fn oracle() -> (Lua, Function) {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    let function = lua
        .load(
            r#"
        return function(input)
            local target = {}
            local result = {}
            for index = 1, #input do
                local key = tonumber(input[index])
                local exists = target[key] ~= nil
                local ok, message = pcall(function() target[key] = true end)
                result[index] = { #target, exists, ok, message }
            end
            return result
        end
    "#,
        )
        .eval()
        .unwrap();
    (lua, function)
}

fn compare(lua: &Lua, original: &Function, input: &[String]) -> usize {
    let source = lua
        .create_sequence_from(input.iter().map(String::as_str))
        .unwrap();
    let observations: Table = original.call(source).unwrap();
    let mut native = NumericSetKeys::new(input.len());
    for (index, input_key) in input.iter().enumerate() {
        let row: Table = observations.get(index + 1).unwrap();
        let expected_length: usize = row.get(1).unwrap();
        let expected_exists: bool = row.get(2).unwrap();
        let expected_ok: bool = row.get(3).unwrap();
        let actual_exists =
            parse_number(input_key.as_bytes()).is_some_and(|key| native.contains(key));
        let inserted = native.insert_text(input_key.as_bytes());
        assert_eq!(
            actual_exists,
            expected_exists,
            "prefix {:?}",
            &input[..=index]
        );
        assert_eq!(
            inserted.is_ok(),
            expected_ok,
            "prefix {:?}, native={inserted:?}",
            &input[..=index]
        );
        if let Ok(inserted) = inserted {
            assert_eq!(inserted, !expected_exists, "prefix {:?}", &input[..=index]);
        } else {
            let source_error: String = row.get(4).unwrap();
            match inserted.unwrap_err() {
                NumericSetKeyError::NotNumber => assert!(source_error.contains("index is nil")),
                NumericSetKeyError::NanKey => assert!(source_error.contains("index is NaN")),
                error => panic!("unexpected native resource error {error}"),
            }
        }
        assert_eq!(
            native.sequence_length(),
            expected_length,
            "prefix {:?}",
            &input[..=index]
        );
    }
    input.len()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn original_luajit_sparse_numeric_keys_and_duplicates() {
    let (lua, original) = oracle();
    let mut observations = 0;
    for input in [
        vec!["1", "3"],
        vec!["2", "8"],
        vec!["-2.5", "0"],
        vec!["1", "2", "4"],
        vec!["1", "3", "5", "2", "4"],
        vec!["nan", "invalid", "-nan", "1", "1.0", "0x1", "0b1", "1e0"],
        vec![
            "0", "-0", "-0.0", "0x0", "inf", "-inf", "1.5", "-1.5", "1e309",
        ],
        vec![
            "134217728",
            "134217729",
            "2147483647",
            "2147483648",
            "4294967296",
            "-2147483648",
            "-2147483649",
        ],
        vec![
            "9007199254740991",
            "9007199254740992",
            "9007199254740993",
            "4.9406564584124654e-324",
            "-4.9406564584124654e-324",
        ],
    ] {
        observations += compare(&lua, &original, &strings(&input));
    }
    // A large hash allocation followed by powers of two reaches the original
    // overflow fallback with a small key count and without a huge array.
    let mut widening: Vec<_> = (1..=96).map(|value| format!("-{value}")).collect();
    widening.extend((0..=30).map(|bit| (1u64 << bit).to_string()));
    observations += compare(&lua, &original, &widening);
    eprintln!("Original sparse/numeric key observations: {observations}");
}

#[test]
fn original_luajit_exhaustive_short_insertion_histories() {
    let (lua, original) = oracle();
    let alphabet = ["0", "1", "2", "3", "4", "8", "-1", "0.5", "inf"];
    let mut observations = 0;
    for encoded in 0..alphabet.len().pow(4) {
        let mut encoded = encoded;
        let mut input = Vec::new();
        for _ in 0..4 {
            input.push(alphabet[encoded % alphabet.len()].to_owned());
            encoded /= alphabet.len();
        }
        observations += compare(&lua, &original, &input);
    }
    eprintln!("Original exhaustive four-insertion observations: {observations}");
}

#[test]
fn original_luajit_resize_histories_and_automatic_ids() {
    let (lua, original) = oracle();
    let mut seed = 0xc831_7a52_b903_264du64;
    let mut observations = 0;
    for case in 0..1600 {
        let mut input = Vec::new();
        let mut expected = NumericSetKeys::new(192);
        for index in 0..192 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let key = if index % 11 == 0 {
                (expected.sequence_length() + 1).to_string()
            } else {
                match case % 8 {
                    0 => (seed % 513).to_string(),
                    1 => (seed % 65).to_string(),
                    2 => ((seed % 512) as i64 - 256).to_string(),
                    3 => ((seed % 2048) as f64 / 4.0).to_string(),
                    4 => (1u64 << (seed % 32)).to_string(),
                    5 => format!("-{}", seed % 1024),
                    6 => ["nan", "0", "-0", "inf", "-inf", "bad", "1.5", "134217728"]
                        [seed as usize % 8]
                        .to_owned(),
                    _ => {
                        let value = f64::from_bits(seed);
                        if value.is_finite() {
                            value.to_string()
                        } else {
                            "nan".to_owned()
                        }
                    }
                }
            };
            let _ = expected.insert_text(key.as_bytes());
            input.push(key);
        }
        observations += compare(&lua, &original, &input);
    }
    eprintln!("Original resize and automatic-ID observations: {observations}");
}
