use poe_optimizer_engine::lua_pattern::{
    GsubLimits, GsubResult, LuaPattern, MatchBudget, MatchLimits, PatternError, ResourceKind,
    SourcePatternError,
};
fn replace(
    subject: &[u8],
    pattern: &[u8],
    replacement: &[u8],
    count: Option<i32>,
) -> Result<GsubResult, PatternError> {
    LuaPattern::compile(pattern).unwrap().gsub(
        subject,
        replacement,
        count,
        &mut MatchBudget::default(),
        GsubLimits::default(),
    )
}
#[test]
fn captures_positions_and_binary_replacements_keep_lua_51_meaning() {
    assert_eq!(
        replace(b"abc 123", b"()(%a+)", b"%2@%1%%:%0", None).unwrap(),
        GsubResult {
            bytes: b"abc@1%:abc 123".to_vec(),
            substitutions: 1
        }
    );
    assert_eq!(
        replace(b"aa", b"a", b"%1%q%", None).unwrap().bytes,
        b"aq\0aq\0"
    );
    assert_eq!(
        replace(b"\xff\0a", b"(.)", b"%1\0", None).unwrap().bytes,
        b"\xff\0\0\0a\0"
    );
    assert_eq!(replace(b"aa", b"a", b"%10", None).unwrap().bytes, b"a0a0");
}
#[test]
fn empty_matches_advance_once_and_anchors_only_try_the_start() {
    assert_eq!(
        replace(b"a", b"a*", b"X", None).unwrap(),
        GsubResult {
            bytes: b"XX".to_vec(),
            substitutions: 2
        }
    );
    assert_eq!(replace(b"ab", b"", b"|", None).unwrap().bytes, b"|a|b|");
    assert_eq!(replace(b"ab", b"^", b"|", None).unwrap().bytes, b"|ab");
    assert_eq!(replace(b"ab", b"$", b"|", None).unwrap().bytes, b"ab|");
    assert_eq!(replace(b"ba", b"^a", b"X", None).unwrap().substitutions, 0);
    assert_eq!(replace(b"ab", b"", b"|", Some(2)).unwrap().bytes, b"|a|b");
}
#[test]
fn unused_capture_errors_are_lazy_without_changing_find_contract() {
    for maximum in [Some(i32::MIN), Some(-1), Some(0)] {
        assert_eq!(
            replace(b"a", b"[", b"%9", maximum).unwrap(),
            GsubResult {
                bytes: b"a".to_vec(),
                substitutions: 0
            }
        );
    }
    assert_eq!(replace(b"a", b"z[", b"%9", None).unwrap().substitutions, 0);
    assert_eq!(replace(b"a", b"(", b"X", None).unwrap().bytes, b"XaX");
    assert_eq!(replace(b"a", b"(a()", b"%2", None).unwrap().bytes, b"2");
    assert_eq!(
        replace(b"a", b"(a()", b"%1", None),
        Err(PatternError::Source(SourcePatternError::UnfinishedCapture))
    );
    assert_eq!(
        replace(b"a", b"a", b"%2", None),
        Err(PatternError::Source(
            SourcePatternError::InvalidCaptureIndex
        ))
    );
    assert_eq!(
        LuaPattern::compile(b"(")
            .unwrap()
            .find(b"a", 1, false, &mut MatchBudget::default()),
        Err(PatternError::Source(SourcePatternError::UnfinishedCapture))
    );
}
#[test]
fn subject_replacement_output_and_shared_work_limits_are_separate() {
    let pattern = LuaPattern::compile(b"a").unwrap();
    let run = |subject: &[u8], replacement: &[u8], budget: &mut MatchBudget, limits: GsubLimits| {
        pattern.gsub(subject, replacement, None, budget, limits)
    };
    assert_eq!(
        run(
            b"aa",
            b"X",
            &mut MatchBudget::new(MatchLimits {
                max_subject_bytes: 1,
                ..Default::default()
            }),
            GsubLimits::default()
        ),
        Err(PatternError::Resource(ResourceKind::SubjectBytes))
    );
    assert_eq!(
        run(
            b"a",
            b"XY",
            &mut MatchBudget::default(),
            GsubLimits {
                max_replacement_bytes: 1,
                ..Default::default()
            }
        ),
        Err(PatternError::Resource(ResourceKind::ReplacementBytes))
    );
    assert_eq!(
        run(
            b"aa",
            b"XY",
            &mut MatchBudget::default(),
            GsubLimits {
                max_output_bytes: 3,
                ..Default::default()
            }
        ),
        Err(PatternError::Resource(ResourceKind::OutputBytes))
    );
    let exact = run(
        b"aa",
        b"XY",
        &mut MatchBudget::default(),
        GsubLimits {
            max_output_bytes: 4,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exact.bytes, b"XYXY");
    assert_eq!(exact.bytes.capacity(), 4);
    assert_eq!(
        run(
            b"",
            b"",
            &mut MatchBudget::default(),
            GsubLimits {
                max_output_bytes: 0,
                ..Default::default()
            }
        )
        .unwrap()
        .bytes,
        b""
    );
    let mut budget = MatchBudget::new(MatchLimits {
        max_steps: 40,
        ..Default::default()
    });
    let mut calls = 0;
    loop {
        match run(b"aa", b"XY", &mut budget, GsubLimits::default()) {
            Ok(_) => calls += 1,
            Err(error) => {
                assert_eq!(error, PatternError::Resource(ResourceKind::MatchSteps));
                break;
            }
        }
        assert!(calls < 40);
    }
    assert!(calls > 0);
}
#[test]
fn replacement_deletion_does_not_preallocate_subject_sized_output() {
    let subject = vec![b'a'; 10000];
    let result = LuaPattern::compile(b"a+")
        .unwrap()
        .gsub(
            &subject,
            b"",
            None,
            &mut MatchBudget::default(),
            GsubLimits {
                max_output_bytes: 0,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(result.bytes.is_empty());
    assert_eq!(result.bytes.capacity(), 0);
    assert_eq!(result.substitutions, 1);
}
#[test]
fn compiled_patterns_are_shareable_with_independent_worker_state() {
    let pattern = LuaPattern::compile(b"()(%a+)").unwrap();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|id| {
                let pattern = &pattern;
                scope.spawn(move || {
                    let replacement = format!("%2/%1/{id}");
                    for _ in 0..64 {
                        let result = pattern
                            .gsub(
                                b"hello world",
                                replacement.as_bytes(),
                                None,
                                &mut MatchBudget::default(),
                                GsubLimits::default(),
                            )
                            .unwrap();
                        assert_eq!(
                            result.bytes,
                            format!("hello/1/{id} world/7/{id}").as_bytes()
                        );
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
