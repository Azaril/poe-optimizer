//! Original bundled LuaJIT5.1 byte-string replacement, not Lua5.4 semantics.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, HookTriggers, Lua, LuaString, Table, VmState};
use poe_optimizer_engine::lua_pattern::{
    GsubLimits, LuaPattern, MatchBudget, MatchLimits, PatternError,
};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
struct Case {
    subject: Vec<u8>,
    pattern: Vec<u8>,
    replacement: Vec<u8>,
    maximum: Option<i32>,
}
impl Case {
    fn new(subject: &[u8], pattern: &[u8], replacement: &[u8], maximum: Option<i32>) -> Self {
        Self {
            subject: subject.to_vec(),
            pattern: pattern.to_vec(),
            replacement: replacement.to_vec(),
            maximum,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Replaced {
    bytes: Vec<u8>,
    substitutions: usize,
}
struct Original {
    lua: Lua,
    gsub: Function,
}
impl Original {
    fn new() -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(64 * 1024 * 1024).unwrap();
        let started = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(10_000),
            move |_, _| {
                if started.elapsed() > Duration::from_secs(60) {
                    Err(mlua::Error::RuntimeError("gsub oracle deadline".into()))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        lua.load("jit.off(); jit.flush()").exec().unwrap();
        let gsub: Function = lua
            .globals()
            .get::<Table>("string")
            .unwrap()
            .get("gsub")
            .unwrap();
        assert_eq!(gsub.info().what, "C");
        Self { lua, gsub }
    }
    fn apply(&self, case: &Case) -> Result<Replaced, String> {
        let subject = self.lua.create_string(&case.subject).unwrap();
        let pattern = self.lua.create_string(&case.pattern).unwrap();
        let replacement = self.lua.create_string(&case.replacement).unwrap();
        let result: mlua::Result<(LuaString, usize)> = match case.maximum {
            Some(maximum) => self.gsub.call((subject, pattern, replacement, maximum)),
            None => self.gsub.call((subject, pattern, replacement)),
        };
        result
            .map(|(bytes, substitutions)| Replaced {
                bytes: bytes.as_bytes().to_vec(),
                substitutions,
            })
            .map_err(|error| error.to_string())
    }
}
fn compare(source: &Original, case: &Case) -> Result<Replaced, String> {
    let expected = source.apply(case);
    let pattern = LuaPattern::compile(&case.pattern).unwrap();
    let actual = pattern.gsub(
        &case.subject,
        &case.replacement,
        case.maximum,
        &mut MatchBudget::new(MatchLimits {
            max_subject_bytes: 128 * 1024,
            max_steps: 16_000_000,
            max_backtrack_frames: 512,
        }),
        GsubLimits {
            max_replacement_bytes: 64 * 1024,
            max_output_bytes: 4 * 1024 * 1024,
        },
    );
    match (&expected, actual) {
        (Ok(expected), Ok(actual)) => {
            assert_eq!(actual.bytes, expected.bytes, "{case:?}");
            assert_eq!(actual.substitutions, expected.substitutions, "{case:?}");
        }
        (Err(message), Err(PatternError::Source(error))) => {
            assert!(
                message.contains(error.message()),
                "wrong original error: {message}; native={error:?}; {case:?}"
            );
        }
        (expected, actual) => panic!("original={expected:?}; native={actual:?}; {case:?}"),
    }
    expected
}

#[test]
fn original_replacement_quirks_are_byte_exact_and_selected_lazily() {
    let source = Original::new();
    for (subject, pattern, replacement, bytes, substitutions) in [
        (&b"ab"[..], &b"."[..], &b"%1"[..], &b"ab"[..], 2),
        (b"ab", b".", b"%0", b"ab", 2),
        (b"ab", b".", b"%", b"\0\0", 2),
        (b"ab", b".", b"%q", b"qq", 2),
        (b"ab", b".", b"%\0", b"\0\0", 2),
        (b"abc", b"()", b"%1", b"1a2b3c4", 4),
        (b"abc", b"(", b"x", b"xaxbxcx", 4),
        (b"aba", b"(a", b"X", b"XbX", 2),
        (b"aba", b"(a", b"%0", b"aba", 2),
        (b"a\0b", b"%z", b"X", b"aXb", 1),
        (b"ab", b"a\0[", b"%0%", b"a\0b", 1),
        (b"abc", b"^", b"X", b"Xabc", 1),
        (b"abc", b"$", b"X", b"abcX", 1),
        (b"abc", b"a*", b"X", b"XXbXcX", 4),
    ] {
        assert_eq!(
            compare(&source, &Case::new(subject, pattern, replacement, None)).unwrap(),
            Replaced {
                bytes: bytes.to_vec(),
                substitutions
            }
        );
    }
    for (pattern, replacement) in [
        (&b"("[..], &b"%1"[..]),
        (b"(a", b"%1"),
        (b".", b"%2"),
        (b"()", b"%2"),
    ] {
        assert!(compare(&source, &Case::new(b"abc", pattern, replacement, None)).is_err());
    }
    for maximum in [i32::MIN, -1, 0] {
        assert_eq!(
            compare(&source, &Case::new(b"abc", b"[", b"%9", Some(maximum))).unwrap(),
            Replaced {
                bytes: b"abc".to_vec(),
                substitutions: 0
            }
        );
    }
    assert_eq!(
        compare(&source, &Case::new(b"abc", b"z", b"%9", None)).unwrap(),
        Replaced {
            bytes: b"abc".to_vec(),
            substitutions: 0
        }
    );
}

#[test]
fn original_patterns_replacements_and_signed_counts_form_a_broad_matrix() {
    let source = Original::new();
    let subjects: &[&[u8]] = &[
        b"",
        b"a",
        b"ab",
        b"aaa",
        b"a1 b22",
        b"()((x))",
        b"\0",
        b"a\0b",
        b"a\n\r b",
        b"\x80\xffa",
        b"abc123abc",
        b"%1%%",
    ];
    let patterns: &[&[u8]] = &[
        b"",
        b"^",
        b"$",
        b"^$",
        b".",
        b"a",
        b"a*",
        b"a-",
        b"a?",
        b"a+",
        b"a*$",
        b"^a*",
        b"(%a+)",
        b"(%d+)",
        b"()",
        b"()(.)()",
        b"(a)(b)",
        b"((a))",
        b"(a*)%1",
        b"(%z)",
        b"%f[%a]",
        b"%f[%z]",
        b"%b()",
        b"[^a]",
        b"[%z\x80-\xff]",
        b"a\0[",
        b"\0%",
        b"(",
        b"(a",
        b"(",
        b"(()",
        b")",
        b"[",
        b"%",
        b"%f",
        b"%b(",
        b"a[",
    ];
    let replacements: &[&[u8]] = &[
        b"", b"X", b"%0", b"%1", b"%2", b"%9", b"%%", b"%", b"%x", b"%01", b"%10", b"%1%0%1",
        b"%2%1", b"a\0b", b"%\0%0", b"\xff%1",
    ];
    let caps = [
        None,
        Some(i32::MIN),
        Some(-1),
        Some(0),
        Some(1),
        Some(2),
        Some(9),
        Some(i32::MAX),
    ];
    let mut successes = 0usize;
    let mut errors = 0usize;
    for subject in subjects {
        for pattern in patterns {
            for replacement in replacements {
                for maximum in caps {
                    match compare(&source, &Case::new(subject, pattern, replacement, maximum)) {
                        Ok(_) => successes += 1,
                        Err(_) => errors += 1,
                    }
                }
            }
        }
    }
    assert!(successes > 10_000 && errors > 1_000);
    eprintln!(
        "Original gsub syntax/cap observations: {successes} successes, {errors} source errors"
    );
}

#[test]
fn every_byte_and_percent_escape_matches_original_replacement_behavior() {
    let source = Original::new();
    let mut observations = 0usize;
    for byte in 0..=255u8 {
        for subject in [vec![byte], vec![b'a', byte, b'b'], vec![byte, 0, byte]] {
            for pattern in [&b"."[..], b"(.)", b"()", b"%z", b"[^%z]", b"%f[%z]"] {
                for replacement in [vec![byte], vec![b'%', byte], vec![byte, b'%', b'0', 0]] {
                    for maximum in [None, Some(1)] {
                        let _ = compare(
                            &source,
                            &Case::new(&subject, pattern, &replacement, maximum),
                        );
                        observations += 1;
                    }
                }
            }
        }
    }
    assert_eq!(observations, 27_648);
    eprintln!("Original gsub all-byte observations: {observations}");
}

#[test]
fn original_empty_progress_nested_captures_and_output_growth_remain_exact() {
    let source = Original::new();
    for length in [0, 1, 2, 9, 31, 255, 1024, 4096] {
        let subject = vec![b'a'; length];
        for (pattern, replacement) in [
            (&b""[..], &b"%1%0X"[..]),
            (b"()", b"<%1>"),
            (b"a*", b"[%0]"),
            (b"(.)", b"%1%1%1%1"),
        ] {
            for maximum in [None, Some(0), Some(1), Some(17), Some(i32::MAX)] {
                let _ = compare(&source, &Case::new(&subject, pattern, replacement, maximum));
            }
        }
    }
    for count in [1, 2, 8, 9, 31, 32, 33] {
        let mut pattern = vec![b'('; count];
        pattern.push(b'a');
        pattern.extend(std::iter::repeat_n(b')', count));
        for replacement in [&b"X"[..], b"%0", b"%1", b"%9", b"%9%1"] {
            let _ = compare(&source, &Case::new(b"aba", &pattern, replacement, None));
        }
    }
}

#[test]
fn warmed_lua_hosts_repeat_original_c_replacements_without_claiming_c_traces() {
    let source = Original::new();
    let cases = source.lua.create_table().unwrap();
    let pair = |a: &[u8], b: &[u8], pattern: &[u8], replacement: &[u8], maximum| {
        [
            Case::new(a, pattern, replacement, maximum),
            Case::new(b, pattern, replacement, maximum),
        ]
    };
    let cases_spec = [
        pair(b"a1 b22", b"x333", b"(%d+)", b"[%1]", None),
        pair(b"abc", b"xy", b"()", b"%1", None),
        pair(b"abc", b"xy", b"(", b"X", None),
        pair(b"a\0b", b"\0\xff", b".", b"%", None),
        pair(b"aaa", b"aaba", b"a*", b"%0X", Some(2)),
        pair(b"abc", b"\0", b"[", b"%9", Some(0)),
        pair(b"(x)(y)", b"((z))", b"%b()", b"<%0>", None),
        pair(b"a b", b"xy z", b"%f[%a]", b"X", Some(i32::MAX)),
    ];
    for (index, [a, b]) in cases_spec.iter().enumerate() {
        let case = source.lua.create_table().unwrap();
        case.set(
            "subjects",
            source
                .lua
                .create_sequence_from([
                    source.lua.create_string(&a.subject).unwrap(),
                    source.lua.create_string(&b.subject).unwrap(),
                ])
                .unwrap(),
        )
        .unwrap();
        case.set("pattern", source.lua.create_string(&a.pattern).unwrap())
            .unwrap();
        case.set(
            "replacement",
            source.lua.create_string(&a.replacement).unwrap(),
        )
        .unwrap();
        case.set("maximum", a.maximum).unwrap();
        cases.set(index + 1, case).unwrap();
    }
    let evidence: Table = source
        .lua
        .load(include_str!("support/lua_gsub_warm.lua"))
        .set_name("@test-only-gsub-warm")
        .call((source.gsub.clone(), cases))
        .unwrap();
    assert_eq!(
        evidence.get::<usize>("executions").unwrap(),
        cases_spec.len() * 256
    );
    assert!(evidence.get::<bool>("original_binding_unchanged").unwrap());
    assert_eq!(source.gsub.info().what, "C");
    assert!(evidence.get::<u32>("primitive_ffid").unwrap() > 0);
    assert!(!evidence.get::<bool>("claims_c_internals_traced").unwrap());
    let outputs: Table = evidence.get("outputs").unwrap();
    for (index, pair) in cases_spec.iter().enumerate() {
        let output: Table = outputs.get(index + 1).unwrap();
        for (position, case) in pair.iter().enumerate() {
            let expected = compare(&source, case).unwrap();
            let row: Table = output.get(position + 1).unwrap();
            assert_eq!(
                row.get::<LuaString>("bytes").unwrap().as_bytes().as_ref(),
                expected.bytes
            );
            assert_eq!(
                row.get::<usize>("substitutions").unwrap(),
                expected.substitutions
            );
        }
    }
    let live: Table = evidence.get("live").unwrap();
    let mut observed_cases = std::collections::BTreeMap::<usize, usize>::new();
    let mut observations = Vec::new();
    for row in live.sequence_values::<Table>() {
        let row = row.unwrap();
        assert_eq!(row.get::<String>("source").unwrap(), "@test-only-gsub-warm");
        let id = row.get::<usize>("id").unwrap();
        let case = row.get::<usize>("case").unwrap();
        let line = row.get::<usize>("line").unwrap();
        assert!(id > 0 && (1..=cases_spec.len()).contains(&case));
        *observed_cases.entry(case).or_default() += 1;
        observations.push((case, id, line));
    }
    assert_eq!(
        observed_cases.len(),
        cases_spec.len(),
        "cases missing completed live host traces: {observed_cases:?}"
    );
    eprintln!(
        "Original gsub warm executions={}, cases={}, completed live host observations={observations:?}; C internals not claimed traced",
        evidence.get::<usize>("executions").unwrap(),
        cases_spec.len()
    );
}
