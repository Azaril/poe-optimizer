//! Independent original LuaJIT pattern and ModParser scan parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/lua_pattern_source.rs"]
mod pattern_source;
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[path = "support/mod_parser_scan_source.rs"]
mod scan_source;
use mlua::{Table, Value};
use pattern_source::{Capture, Find, PatternSource};
use scan_source::ScanSource;

#[test]
fn original_byte_matcher_preserves_empty_position_error_and_warm_observations() {
    let source = PatternSource::new();
    assert_eq!(
        source.find(b"abc", b"()", 1, false),
        Find::Match {
            start: 0,
            end: 0,
            captures: vec![Capture::Position(1)]
        }
    );
    assert_eq!(
        source.find(b"abc", b"z*", 1, false),
        Find::Match {
            start: 0,
            end: 0,
            captures: vec![]
        }
    );
    assert_eq!(source.find(b"abc", b"Z", 1, false), Find::Absent);
    assert!(matches!(
        source.find(b"abc", b"[", 1, false),
        Find::Error(_)
    ));
    let warm = source.warm();
    assert!(warm.get::<i64>("count").unwrap() > 0);
    assert!(
        warm.get::<Table>("live").unwrap().raw_len() > 0,
        "no completed live string.find host traces"
    );
    assert!(warm.get::<String>("version").unwrap().starts_with("LuaJIT"));
}
#[test]
fn complete_original_tables_and_scan_are_available_without_shadow_construction() {
    let source = ScanSource::new();
    let keys = source.keys();
    assert_eq!(source.tables.clone().pairs::<Value, Value>().count(), 20);
    assert!(
        keys.len() > 2500,
        "unexpectedly partial original dictionaries: {}",
        keys.len()
    );
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for (name, _, _) in &keys {
        *counts.entry(name.clone()).or_default() += 1;
    }
    eprintln!("Original complete parser table key counts: {counts:?}");
    // Exercise the separately loaded unchanged public parser and item host too.
    let parsed = source
        .source
        .parse("Rarity: Normal\nRusted Greathelm\nQuality: 0");
    assert_eq!(
        parsed.get::<String>("baseName").unwrap(),
        "Rusted Greathelm"
    );
    assert!(
        source
            .source
            .load("<Items/>", false)
            .get::<bool>("ok")
            .unwrap()
    );
    let live = source.warm();
    assert!(
        live.clone()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .any(|row| row.get::<usize>("line").unwrap() == 6592),
        "original scan absent from completed live source traces: {live:?}"
    );
    eprintln!(
        "Original scan6592 completed live source trace observations: {}",
        live.raw_len()
    );
}
fn original_scan(
    source: &ScanSource,
    line: &[u8],
    table: Table,
    plain: bool,
) -> Result<(Value, Vec<u8>, Option<Table>), mlua::Error> {
    source
        .scan
        .call((source.source.lua.create_string(line).unwrap(), table, plain))
        .map(
            |(value, remainder, caps): (Value, mlua::LuaString, Option<Table>)| {
                (value, remainder.as_bytes().to_vec(), caps)
            },
        )
}
#[test]
fn original_scan_winner_truthiness_and_first_five_captures_are_source_contracts() {
    let source = ScanSource::new();
    let table = source.source.lua.create_table().unwrap();
    table.set("alpha", false).unwrap();
    table.set("omega", true).unwrap();
    let (value, remainder, caps) = original_scan(&source, b"ALPHA omega", table, false).unwrap();
    assert!(matches!(value, Value::Nil));
    assert_eq!(remainder, b"ALPHA omega");
    assert!(caps.is_none());
    let table = source.source.lua.create_table().unwrap();
    table.set("()(%a)(%a)(%a)(%a)(%a)(%a)", 7).unwrap();
    let (value, remainder, caps) = original_scan(&source, b"AbcDeF Z", table, false).unwrap();
    assert_eq!(value.as_integer(), Some(7));
    assert_eq!(remainder, b" Z");
    let caps = caps.unwrap();
    assert_eq!(caps.raw_len(), 5);
    assert_eq!(caps.get::<usize>(1).unwrap(), 1);
    assert_eq!(caps.get::<String>(5).unwrap(), "d");
}

fn paired_find(
    source: &PatternSource,
    subject: &[u8],
    pattern: &[u8],
    init: i32,
    plain: bool,
) -> bool {
    use poe_optimizer_engine::lua_pattern::{
        Capture as NativeCapture, LuaPattern, MatchBudget, PatternError,
    };
    let expected = source.find(subject, pattern, init, plain);
    let compiled = LuaPattern::compile(pattern).unwrap();
    let actual = compiled.find(subject, init, plain, &mut MatchBudget::default());
    match (&expected, actual) {
        (Find::Error(message), Err(PatternError::Source(error))) => {
            assert!(
                message.contains(error.message()),
                "wrong source error: {message}; native={error:?}; pattern={pattern:?},subject={subject:?},init={init},plain={plain}"
            );
        }
        (Find::Absent, Ok(None)) => {}
        (
            Find::Match {
                start,
                end,
                captures,
            },
            Ok(Some(found)),
        ) => {
            assert_eq!(
                found.range(),
                *start..*end,
                "pattern={pattern:?},subject={subject:?},init={init},plain={plain}"
            );
            let actual = found
                .captures()
                .iter()
                .map(|c| match *c {
                    NativeCapture::Bytes { start, end } => {
                        Capture::Bytes(subject[start..end].to_vec())
                    }
                    NativeCapture::Position(pos) => Capture::Position(pos),
                })
                .collect::<Vec<_>>();
            assert_eq!(
                actual, *captures,
                "captures pattern={pattern:?},subject={subject:?},init={init},plain={plain}"
            );
        }
        (expected, actual) => panic!(
            "pattern={pattern:?},subject={subject:?},init={init},plain={plain}: source={expected:?}; native={actual:?}"
        ),
    }
    matches!(expected, Find::Match { .. })
}
#[test]
fn original_find_matches_native_adversarial_patterns_bytes_captures_and_lazy_errors() {
    let patterns: &[&[u8]] = &[
        b"",
        b"a",
        b"^a",
        b"a$",
        b"^$",
        b"$",
        b".",
        b".*",
        b".-",
        b"a*",
        b"a+",
        b"a-",
        b"a?",
        b"a?a",
        b"a*a",
        b"a-a",
        b".*b",
        b".-b",
        b"(a*)",
        b"()",
        b"()a()",
        b"((a)(b))",
        b"(%a+)%s+(%d+)",
        b"(%a+)%s+%1",
        b"()a%1",
        b"%0",
        b"%1",
        b"(%a)%2",
        b"(a%1)",
        b"%b()",
        b"%b[]",
        b"%bxx",
        b"%f[%a]abc",
        b"%f[%z]",
        b"%f[^%z]",
        b"%f[]]",
        b"[abc]",
        b"[^abc]",
        b"[a-z]",
        b"[z-a]",
        b"[]a]",
        b"[^]a]",
        b"[-a]",
        b"[a-]",
        b"[%a%d_]",
        b"[%]]",
        b"[%]",
        b"[%",
        b"[",
        b"[^",
        b"[]",
        b"%",
        b"%f",
        b"%fX",
        b"%b",
        b"%b(",
        b"(",
        b")",
        b"a)",
        b"(a",
        b"a(",
        b"a[",
        b"z[",
        b"^z[",
        b"a**",
        b"a++",
        b"a??",
        b"*a",
        b"-",
        b"%q",
        b"%Q",
        b"%g",
        b"%G",
        b"%z",
        b"%Z",
        b"\0",
        b"a\0(",
        b"a\0%",
        b"\0%a",
        b"[\0]",
        b"+#",
        b"%%",
        b"%.",
        b"%+",
        b"[A-Z]",
        b"(%d+)([^%.])",
        b"(%d+%.?%d*)",
    ];
    let subjects: &[&[u8]] = &[
        b"",
        b"a",
        b"b",
        b"abc",
        b"ABC",
        b"aaaa",
        b"aaab",
        b"abc abc",
        b"a a",
        b"a9",
        b"12.5",
        b"123.",
        b"12",
        b"a(",
        b"a)",
        b"(",
        b")",
        b"(x(y)z)",
        b"[a[b]c]",
        b"xx",
        b"+#",
        b"%",
        b"a\0(",
        b"a\0%",
        b"\0a",
        b"\0",
        b"\xff\x80Aa\0z",
        b"abc 123",
        b" abc ",
    ];
    let mut count = 0;
    for warm in [false, true] {
        let source = PatternSource::new();
        if warm {
            assert!(source.warm().get::<Table>("live").unwrap().raw_len() > 0);
        }
        for pattern in patterns {
            for subject in subjects {
                for init in [i32::MIN, -100, -3, -1, 0, 1, 2, 4, 100, i32::MAX] {
                    for plain in [false, true] {
                        paired_find(&source, subject, pattern, init, plain);
                        count += 1;
                    }
                }
            }
        }
    }
    eprintln!("Original byte-pattern adversarial find comparisons: {count}");
}
#[test]
fn every_byte_obeys_original_lua_classes_including_complements() {
    let mut count = 0;
    for warm in [false, true] {
        let source = PatternSource::new();
        if warm {
            source.warm();
        }
        for class in b"acdg lpsuwxzACDG LPSUWXZ"
            .iter()
            .copied()
            .filter(|b| *b != b' ')
        {
            let pattern = [b'%', class];
            for byte in 0..=255 {
                paired_find(&source, &[byte], &pattern, 1, false);
                count += 1;
            }
        }
    }
    eprintln!("Original all-byte class comparisons: {count}");
}

#[test]
fn every_constructed_dictionary_key_matches_original_find_on_independent_witnesses() {
    let mut count = 0;
    let mut positive = 0;
    let mut key_count = 0;
    for warm in [false, true] {
        let tables = ScanSource::new();
        let source = PatternSource::new();
        if warm {
            tables.warm();
            source.warm();
        }
        let generator = source
            .lua
            .load(include_str!("support/lua_pattern_witness.lua"))
            .set_name("@test-only-pattern-input-generator")
            .eval::<mlua::Function>()
            .unwrap();
        let keys = tables.keys();
        key_count = keys.len();
        for (dictionary, pattern, plain) in keys {
            let witness = if plain {
                pattern.clone()
            } else {
                generator
                    .call::<mlua::LuaString>(source.lua.create_string(&pattern).unwrap())
                    .unwrap()
                    .as_bytes()
                    .to_vec()
            };
            let padded = [b"prefix ".as_slice(), &witness, b" suffix"].concat();
            for subject in [
                &[][..],
                witness.as_slice(),
                padded.as_slice(),
                pattern.as_slice(),
            ] {
                positive += usize::from(paired_find(&source, subject, &pattern, 1, plain));
                count += 1;
            }
            assert!(!dictionary.is_empty());
        }
    }
    assert!(
        positive > key_count,
        "test generator did not exercise enough original positive matches"
    );
    eprintln!(
        "All20 original dictionaries: {key_count} keys, {count} paired finds, {positive} source-positive observations"
    );
}
fn paired_scan(source: &ScanSource, line: &[u8], patterns: &[&[u8]], plain: bool) -> bool {
    use poe_optimizer_engine::{
        lua_pattern::{MatchBudget, PatternError},
        modifier_scan::{ScanCapture, ScanError, ScanTable},
    };
    let table = source.source.lua.create_table().unwrap();
    for (index, pattern) in patterns.iter().enumerate() {
        table
            .raw_set(source.source.lua.create_string(pattern).unwrap(), index + 1)
            .unwrap();
    }
    let ordered = table
        .clone()
        .pairs::<mlua::LuaString, usize>()
        .map(|r| {
            let (k, v) = r.unwrap();
            (k.as_bytes().to_vec(), v)
        })
        .collect::<Vec<_>>();
    let native = ScanTable::compile(ordered.iter().map(|r| &r.0)).unwrap();
    let expected = original_scan(source, line, table, plain);
    let actual = native.scan(line, plain, &mut MatchBudget::default());
    match (expected, actual) {
        (Err(expected), Err(ScanError::Pattern(PatternError::Source(error)))) => {
            assert!(
                expected.to_string().contains(error.message()),
                "source={expected},native={error:?}"
            );
            false
        }
        (Ok((Value::Nil, remainder, None)), Ok(None)) => {
            assert_eq!(remainder, line);
            false
        }
        (Ok((value, remainder, caps)), Ok(Some(found))) => {
            let id = value.as_integer().expect("source row token") as usize;
            assert_eq!(
                ordered[found.row_index].1, id,
                "source-selected row; line={line:?},patterns={patterns:?}"
            );
            assert_eq!(
                found.remainder(),
                remainder,
                "source original-case remainder"
            );
            let caps = caps
                .unwrap()
                .sequence_values::<Value>()
                .map(|v| match v.unwrap() {
                    Value::String(s) => ScanCapture::Bytes(s.as_bytes().to_vec()),
                    Value::Integer(n) => ScanCapture::Position(n as usize),
                    other => panic!("original scan capture {other:?}"),
                })
                .collect::<Vec<_>>();
            assert_eq!(found.captures, caps);
            let lower = source
                .source
                .lua
                .globals()
                .get::<Table>("string")
                .unwrap()
                .get::<mlua::Function>("lower")
                .unwrap()
                .call::<mlua::LuaString>(source.source.lua.create_string(line).unwrap())
                .unwrap();
            let original_find = source
                .source
                .lua
                .globals()
                .get::<Table>("string")
                .unwrap()
                .get::<mlua::Function>("find")
                .unwrap();
            let mut ties = vec![];
            for (index, (pattern, _)) in ordered.iter().enumerate() {
                let values = original_find
                    .call::<mlua::MultiValue>((
                        lower.clone(),
                        source.source.lua.create_string(pattern).unwrap(),
                        1,
                        plain,
                    ))
                    .unwrap();
                if let (Some(a), Some(b)) = (
                    values.front().and_then(Value::as_integer),
                    values.get(1).and_then(Value::as_integer),
                ) && a as usize - 1 == found.range.start
                    && b as usize == found.range.end
                    && pattern.len() == ordered[found.row_index].0.len()
                    && index != found.row_index
                {
                    ties.push(index);
                }
            }
            assert_eq!(found.tied_rows, ties, "original exact-ranked rows");
            true
        }
        (expected, actual) => panic!(
            "scan line={line:?},patterns={patterns:?},plain={plain}: source={expected:?};native={actual:?}"
        ),
    }
}
#[test]
fn original_scan_matches_selection_ties_case_captures_empty_and_late_errors() {
    let mut count = 0;
    let sets: &[&[&[u8]]] = &[
        &[b"alpha", b"omega"],
        &[b"a", b"aa", b"a."],
        &[b"a.", b".a"],
        &[b"[a]a", b"a."],
        &[b"[ab]", b"[ac]", b"a"],
        &[b"", b"()"],
        &[b"$", b"^"],
        &[b"()(%a)(%a)(%a)(%a)(%a)(%a)", b"abcdef"],
        &[b"(%a+)", b"(%d+)"],
        &[b"z[", b"a"],
        &[b"a", b"["],
        &[b"%", b""],
        &[b"%b()", b".-"],
        &[b"\xffa", b"\x80a", b"a"],
        &[b"a\0(", b"a"],
        &[b"%f[%a](%a+)", b"(%d+)"],
    ];
    for warm in [false, true] {
        let source = ScanSource::new();
        if warm {
            source.warm();
        }
        for set in sets {
            for line in [
                b"".as_slice(),
                b"AA",
                b"ALPHA omega",
                b"AbcDeF Z",
                b"prefix123",
                b"(A(B)c)",
                b"\xffA\x80a",
                b"a\0(",
            ] {
                for plain in [false, true] {
                    paired_scan(&source, line, set, plain);
                    count += 1;
                }
            }
        }
    }
    eprintln!("Original full scan selection/capture/tie/error comparisons: {count}");
}

#[test]
fn original_match_capture_api_preserves_whole_match_and_interpreted_syntax() {
    use poe_optimizer_engine::lua_pattern::{
        Capture as NativeCapture, LuaPattern, MatchBudget, PatternError,
    };
    let patterns: &[&[u8]] = &[
        b"",
        b"a",
        b".",
        b".*",
        b"a+",
        b"a-",
        b"()",
        b"(a*)",
        b"()(%a+)()",
        b"((a)(b))",
        b"(%a+)%s+%1",
        b"(%d+)",
        b"%b()",
        b"%f[%a](%a+)",
        b"(",
        b")",
        b"a)",
        b"%",
        b"a[",
        b"z[",
        b"%1",
        b"()%1",
        b"a\0(",
        b"a\0",
        b"(%z)",
        b"[^a]*",
        b"[A-Z]",
        b"^$",
        b"$",
        b"^a",
    ];
    let subjects: &[&[u8]] = &[
        b"", b"a", b"abc", b"AAA", b"aa", b"ab", b"a a", b"123", b"(a(b)c)", b" abc ", b"a)",
        b"a\0(", b"a\0", b"\0", b"\xffa",
    ];
    let mut count = 0;
    for warm in [false, true] {
        let source = PatternSource::new();
        if warm {
            source.warm();
        }
        for pattern in patterns {
            let compiled = LuaPattern::compile(pattern).unwrap();
            for subject in subjects {
                for init in [-100, -3, -1, 0, 1, 2, 4, 100] {
                    let expected = source.match_captures(subject, pattern, init);
                    let actual =
                        compiled.match_captures(subject, init, &mut MatchBudget::default());
                    match (expected, actual) {
                        (Ok(None), Ok(None)) => {}
                        (Ok(Some(expected)), Ok(Some(found))) => {
                            let actual = found
                                .captures()
                                .iter()
                                .map(|c| match *c {
                                    NativeCapture::Bytes { start, end } => {
                                        Capture::Bytes(subject[start..end].to_vec())
                                    }
                                    NativeCapture::Position(position) => {
                                        Capture::Position(position)
                                    }
                                })
                                .collect::<Vec<_>>();
                            assert_eq!(
                                actual, expected,
                                "match pattern={pattern:?},subject={subject:?},init={init}"
                            );
                        }
                        (Err(message), Err(PatternError::Source(error))) => assert!(
                            message.contains(error.message()),
                            "source={message},native={error:?}"
                        ),
                        (expected, actual) => panic!(
                            "match pattern={pattern:?},subject={subject:?},init={init}: source={expected:?},native={actual:?}"
                        ),
                    }
                    count += 1;
                }
            }
        }
    }
    eprintln!("Original string.match typed-capture comparisons: {count}");
}
#[test]
fn original_capture_depth_errors_are_distinct_from_native_resource_limits() {
    use poe_optimizer_engine::lua_pattern::{
        CompileLimits, LuaPattern, MatchBudget, MatchLimits, PatternError, ResourceKind,
        SourcePatternError,
    };
    let source = PatternSource::new();
    for count in [31, 32, 33] {
        paired_find(&source, b"", &b"()".repeat(count), 1, false);
        let pattern = [b"(".repeat(count), b"a".to_vec(), b")".repeat(count)].concat();
        paired_find(&source, b"a", &pattern, 1, false);
    }
    for count in [199, 200, 201] {
        let pattern = b"a?".repeat(count);
        let subject = b"a".repeat(count);
        paired_find(&source, &subject, &pattern, 1, false);
    }
    let too_many = LuaPattern::compile(&b"()".repeat(33)).unwrap();
    assert_eq!(
        too_many.find(b"", 1, false, &mut MatchBudget::default()),
        Err(PatternError::Source(SourcePatternError::TooManyCaptures))
    );
    let deep = LuaPattern::compile(&b"a?".repeat(201)).unwrap();
    assert_eq!(
        deep.find(&b"a".repeat(201), 1, false, &mut MatchBudget::default()),
        Err(PatternError::Source(SourcePatternError::PatternTooComplex))
    );
    assert!(matches!(
        source.find(b"abc", b"abc", 1, false),
        Find::Match { .. }
    ));
    assert!(matches!(
        LuaPattern::compile_with_limits(
            b"abc",
            CompileLimits {
                max_pattern_bytes: 2,
                ..Default::default()
            }
        ),
        Err(PatternError::Resource(ResourceKind::PatternBytes))
    ));
    let literal = LuaPattern::compile(b"abc").unwrap();
    assert_eq!(
        literal.find(
            b"abc",
            1,
            false,
            &mut MatchBudget::new(MatchLimits {
                max_subject_bytes: 2,
                ..Default::default()
            })
        ),
        Err(PatternError::Resource(ResourceKind::SubjectBytes))
    );
    assert_eq!(
        literal.find(
            b"abc",
            1,
            false,
            &mut MatchBudget::new(MatchLimits {
                max_steps: 0,
                ..Default::default()
            })
        ),
        Err(PatternError::Resource(ResourceKind::MatchSteps))
    );
    let branch = LuaPattern::compile(b"a?a").unwrap();
    assert!(matches!(
        source.find(b"aa", b"a?a", 1, false),
        Find::Match { .. }
    ));
    assert_eq!(
        branch.find(
            b"aa",
            1,
            false,
            &mut MatchBudget::new(MatchLimits {
                max_backtrack_frames: 0,
                ..Default::default()
            })
        ),
        Err(PatternError::Resource(ResourceKind::BacktrackFrames))
    );
    let mut shared = MatchBudget::new(MatchLimits {
        max_steps: 30,
        ..Default::default()
    });
    let mut previous = 0;
    let mut rejected = false;
    for _ in 0..32 {
        let result = literal.find(b"abc", 1, false, &mut shared);
        assert!(shared.steps_used() >= previous);
        previous = shared.steps_used();
        if result == Err(PatternError::Resource(ResourceKind::MatchSteps)) {
            rejected = true;
            break;
        }
        assert!(result.unwrap().is_some());
    }
    assert!(rejected, "candidate calls did not share a work budget");
}
