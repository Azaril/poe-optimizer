use poe_optimizer_engine::lua_pattern::*;
fn find(pattern: &[u8], subject: &[u8]) -> Result<Option<PatternMatch>, PatternError> {
    LuaPattern::compile(pattern)
        .unwrap()
        .find(subject, 1, false, &mut MatchBudget::default())
}
#[test]
fn byte_spans_empty_and_position_captures_are_distinct() {
    let result = find(b"()(%a+)()", b"12alpha!").unwrap().unwrap();
    assert_eq!(result.range(), 2..7);
    assert_eq!(result.lua_indices(), (3, 7));
    assert_eq!(
        result.captures(),
        &[
            Capture::Position(3),
            Capture::Bytes { start: 2, end: 7 },
            Capture::Position(8)
        ]
    );
    let result = find(b"a*", b"x").unwrap().unwrap();
    assert_eq!(result.range(), 0..0);
    assert_eq!(result.lua_indices(), (1, 0));
    assert!(result.captures().is_empty());
    let result = LuaPattern::compile(b"a*")
        .unwrap()
        .match_captures(b"x", 1, &mut MatchBudget::default())
        .unwrap()
        .unwrap();
    assert_eq!(result.captures(), &[Capture::Bytes { start: 0, end: 0 }]);
}
#[test]
fn greedy_minimal_and_optional_backtracking_restore_nested_captures() {
    let result = find(b"(a*)%1", b"aaa").unwrap().unwrap();
    assert_eq!(result.range(), 0..2);
    assert_eq!(result.captures(), &[Capture::Bytes { start: 0, end: 1 }]);
    let result = find(b"((a-)%a)b", b"aaab").unwrap().unwrap();
    assert_eq!(result.range(), 0..4);
    assert_eq!(
        result.captures(),
        &[
            Capture::Bytes { start: 0, end: 3 },
            Capture::Bytes { start: 0, end: 2 }
        ]
    );
    let result = find(b"(a?)a", b"a").unwrap().unwrap();
    assert_eq!(result.captures(), &[Capture::Bytes { start: 0, end: 0 }]);
    assert!(find(b"a+b", b"b").unwrap().is_none());
    assert_eq!(find(b"a-b", b"aaab").unwrap().unwrap().range(), 0..4);
}
#[test]
fn classes_are_ascii_bytes_and_bracket_ranges_follow_source_rules() {
    for value in 0..=255u8 {
        for (class, expected) in [
            (b'a', value.is_ascii_alphabetic()),
            (
                b's',
                matches!(value, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12),
            ),
            (b'w', value.is_ascii_alphanumeric()),
            (b'z', value == 0),
        ] {
            assert_eq!(
                find(&[b'^', b'%', class, b'$'], &[value])
                    .unwrap()
                    .is_some(),
                expected
            );
            assert_eq!(
                find(&[b'^', b'%', class.to_ascii_uppercase(), b'$'], &[value])
                    .unwrap()
                    .is_some(),
                !expected
            );
        }
    }
    assert!(find(b"^[]]$", b"]").unwrap().is_some());
    assert!(find(b"^[^]]$", &[255]).unwrap().is_some());
    assert!(find(b"^[a-%d]$", b"d").unwrap().is_some());
    assert!(find(b"^[a-%d]$", b"5").unwrap().is_none());
    assert!(find(b"^[%q]$", b"q").unwrap().is_some());
    assert!(find(b"^.$", &[0]).unwrap().is_some());
}
#[test]
fn balances_frontiers_and_backreferences_use_subject_boundary_sentinels() {
    assert_eq!(find(b"%b()", b"z(a(b)c)d").unwrap().unwrap().range(), 1..8);
    assert!(find(b"%b()", b"(ab").unwrap().is_none());
    assert_eq!(find(b"%bxx", b"xxx").unwrap().unwrap().range(), 0..2);
    assert_eq!(
        find(b"%f[%a]%a+%f[%A]", b"1abc2").unwrap().unwrap().range(),
        1..4
    );
    assert_eq!(find(b"%f[%z]", b"a").unwrap().unwrap().range(), 1..1);
    assert!(find(b"()%1", b"a").unwrap().is_none());
    assert_eq!(
        find(b"%0", b""),
        Err(PatternError::Source(
            SourcePatternError::InvalidCaptureIndex
        ))
    );
}
#[test]
fn lazy_errors_are_raised_only_when_reached_or_returned() {
    for (suffix, error) in [
        (b"%".as_slice(), SourcePatternError::MalformedEscape),
        (b"[".as_slice(), SourcePatternError::MissingBracket),
        (b"%f".as_slice(), SourcePatternError::MissingFrontierClass),
        (b"%b(".as_slice(), SourcePatternError::UnbalancedPattern),
        (b")".as_slice(), SourcePatternError::InvalidCaptureClose),
    ] {
        let mut pattern = b"x".to_vec();
        pattern.extend_from_slice(suffix);
        assert!(find(&pattern, b"y").unwrap().is_none());
        // ')' alone would select the source fixed-string path; force VM with ^.
        pattern.insert(0, b'^');
        assert_eq!(find(&pattern, b"x"), Err(PatternError::Source(error)));
    }
    assert!(find(b"(x", b"y").unwrap().is_none());
    assert_eq!(
        find(b"(x", b"x"),
        Err(PatternError::Source(SourcePatternError::UnfinishedCapture))
    );
    assert_eq!(
        find(b"(%1)", b"x"),
        Err(PatternError::Source(
            SourcePatternError::InvalidCaptureIndex
        ))
    );
}
#[test]
fn plain_fast_path_nul_and_lua51_relative_initial_index_remain_separate() {
    let p = LuaPattern::compile(b")").unwrap();
    assert_eq!(
        p.find(b"a)b", 1, false, &mut MatchBudget::default())
            .unwrap()
            .unwrap()
            .range(),
        1..2
    );
    assert_eq!(
        p.match_captures(b"a)b", 1, &mut MatchBudget::default()),
        Err(PatternError::Source(
            SourcePatternError::InvalidCaptureClose
        ))
    );
    let p = LuaPattern::compile(b"a\0b").unwrap();
    assert_eq!(
        p.find(b"za\0b", 1, false, &mut MatchBudget::default())
            .unwrap()
            .unwrap()
            .range(),
        1..4
    );
    assert_eq!(
        p.match_captures(b"za\0b", 1, &mut MatchBudget::default())
            .unwrap()
            .unwrap()
            .range(),
        1..2
    );
    let p = LuaPattern::compile(b"a\0b(").unwrap();
    assert_eq!(
        p.find(b"za\0b(", 1, false, &mut MatchBudget::default())
            .unwrap()
            .unwrap()
            .range(),
        1..2
    );
    assert_eq!(
        p.find(b"za\0b(", 1, true, &mut MatchBudget::default())
            .unwrap()
            .unwrap()
            .range(),
        1..5
    );
    let p = LuaPattern::compile(b"^").unwrap();
    for (init, start) in [(0, 0), (-1, 2), (-99, 0), (99, 3), (i32::MIN, 0)] {
        assert_eq!(
            p.find(b"abc", init, false, &mut MatchBudget::default())
                .unwrap()
                .unwrap()
                .range(),
            start..start
        );
    }
}
#[test]
fn source_capture_depth_and_implementation_work_bounds_are_distinct() {
    assert_eq!(
        find(&b"()".repeat(33), b""),
        Err(PatternError::Source(SourcePatternError::TooManyCaptures))
    );
    assert_eq!(
        find(&b"a?".repeat(200), &[b'a'; 200]),
        Err(PatternError::Source(SourcePatternError::PatternTooComplex))
    );
    let p = LuaPattern::compile(b"a-").unwrap();
    assert_eq!(
        p.find(
            b"a",
            1,
            false,
            &mut MatchBudget::new(MatchLimits {
                max_backtrack_frames: 0,
                ..MatchLimits::default()
            })
        ),
        Err(PatternError::Resource(ResourceKind::BacktrackFrames))
    );
    let p = LuaPattern::compile(b"a*a*a*a*a*b").unwrap();
    assert_eq!(
        p.find(
            &[b'a'; 100],
            1,
            false,
            &mut MatchBudget::new(MatchLimits {
                max_steps: 100,
                ..MatchLimits::default()
            })
        ),
        Err(PatternError::Resource(ResourceKind::MatchSteps))
    );
    assert!(matches!(
        LuaPattern::compile_with_limits(
            b"abc",
            CompileLimits {
                max_pattern_bytes: 2,
                ..CompileLimits::default()
            }
        ),
        Err(PatternError::Resource(ResourceKind::PatternBytes))
    ));
    let p = LuaPattern::compile(b"").unwrap();
    let mut shared = MatchBudget::new(MatchLimits {
        max_steps: 1,
        ..MatchLimits::default()
    });
    assert!(p.find(b"", 1, false, &mut shared).unwrap().is_some());
    assert_eq!(
        p.find(b"", 1, false, &mut shared),
        Err(PatternError::Resource(ResourceKind::MatchSteps))
    );
}
#[test]
fn compiled_size_is_bounded_and_matching_never_mutates_shared_pattern() {
    let p = LuaPattern::compile(b"(%a+)%s+%1").unwrap();
    let first = p
        .find(b"abc abc", 1, false, &mut MatchBudget::default())
        .unwrap();
    assert!(
        p.find(b"abc def", 1, false, &mut MatchBudget::default())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        p.find(b"abc abc", 1, false, &mut MatchBudget::default())
            .unwrap(),
        first
    );
    let limits = CompileLimits {
        max_pattern_bytes: 100,
        max_compiled_bytes: 1024,
    };
    let p = LuaPattern::compile_with_limits(b"[%a%d]+", limits).unwrap();
    assert!(p.compiled_bytes() <= limits.max_compiled_bytes);
    assert!(matches!(
        LuaPattern::compile_with_limits(
            b"abc",
            CompileLimits {
                max_compiled_bytes: 1,
                ..limits
            }
        ),
        Err(PatternError::Resource(ResourceKind::CompiledBytes))
    ));
}

#[test]
fn compiled_storage_accepts_exact_tiny_limit_without_capacity_growth() {
    for pattern in [b"".as_slice(), b"a", b"%a", b"()", b"[ab]", b"\0ignored"] {
        let bytes = LuaPattern::compile(pattern).unwrap().compiled_bytes();
        let limits = CompileLimits {
            max_compiled_bytes: bytes,
            ..CompileLimits::default()
        };
        let compiled = LuaPattern::compile_with_limits(pattern, limits).unwrap();
        assert_eq!(compiled.compiled_bytes(), bytes);
        assert!(matches!(
            LuaPattern::compile_with_limits(
                pattern,
                CompileLimits {
                    max_compiled_bytes: bytes - 1,
                    ..limits
                }
            ),
            Err(PatternError::Resource(ResourceKind::CompiledBytes))
        ));
    }
}
