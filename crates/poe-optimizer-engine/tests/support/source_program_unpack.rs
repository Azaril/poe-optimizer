//! Standalone unpack raw reads, bounded result reservation and portable frontiers.
use super::*;
fn library() -> CompiledSourcePrograms {
    compile(
        vec![(
            1,
            0,
            true,
            vec![ParserProgramBinding::Intrinsic {
                operation: ParserProgramIntrinsic::Unpack,
                source: ParserProgramIntrinsicSource::OriginalGlobal,
            }],
            vec![tail(call(
                0,
                None,
                ParserProgramValueList {
                    values: vec![],
                    tail: Some(Box::new(ParserProgramPack::Varargs)),
                },
            ))],
        )],
        vec![vec![]],
    )
}
fn input(start: Option<ProgramValue>, end: Option<ProgramValue>) -> ProgramValueGraph {
    let mut values = vec![ProgramValue::Table(ProgramTableId(1))];
    if let Some(start) = start {
        values.push(start);
    }
    if let Some(end) = end {
        values.push(end);
    }
    ProgramValueGraph {
        values,
        tables: vec![ProgramTable {
            entries: vec![
                (ProgramValue::Number(1.0), ProgramValue::Boolean(false)),
                (ProgramValue::Number(3.0), text("third")),
            ],
        }],
    }
}
#[test]
fn explicit_range_preserves_nil_false_identity_and_does_not_require_length() {
    let lib = library();
    let result = lib
        .execute(
            ParserCallbackId(1),
            &input(
                Some(ProgramValue::Number(0.0)),
                Some(ProgramValue::Number(3.0)),
            ),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_values(
        &result.graph().values,
        &[
            ProgramValue::Nil,
            ProgramValue::Boolean(false),
            ProgramValue::Nil,
            text("third"),
        ],
    );
    assert_eq!(
        lib.execute(
            ParserCallbackId(1),
            &input(None, None),
            ProgramLimits::default()
        )
        .unwrap_err()
        .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let graph = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![
            ProgramTable {
                entries: vec![
                    (
                        ProgramValue::Number(1.0),
                        ProgramValue::Table(ProgramTableId(2)),
                    ),
                    (
                        ProgramValue::Number(2.0),
                        ProgramValue::Table(ProgramTableId(2)),
                    ),
                    (
                        ProgramValue::Number(3.0),
                        ProgramValue::Callback(ParserCallbackId(1)),
                    ),
                ],
            },
            ProgramTable::default(),
        ],
    };
    let result = lib
        .execute(ParserCallbackId(1), &graph, ProgramLimits::default())
        .unwrap();
    assert_eq!(result.graph().values.len(), 3);
    assert_eq!(result.graph().values[0], result.graph().values[1]);
    assert_eq!(
        result.graph().values[2],
        ProgramValue::Callback(ParserCallbackId(1))
    );
}
#[test]
fn coverage_frontiers_and_argument_errors_have_source_order() {
    let lib = library();
    let graph = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable {
            entries: vec![(ProgramValue::Number(1.0), ProgramValue::Boolean(false))],
        }],
    };
    let coverage = ProgramTableCoverage::from([(
        ProgramTableId(1),
        SourceTableCoverage {
            inventory: SourceTableInventory::Complete,
            known_absent: Default::default(),
            unavailable: [SourceTableKey::Integer(2)].into(),
            index_fallback: SourceTableIndexFallback::Unavailable,
            call_fallback: SourceTableCallFallback::NonCallable,
        },
    )]);
    let (mut session, roots) = lib
        .session_with_coverage(&graph, &coverage, ProgramLimits::default())
        .unwrap();
    for (start, end, expected) in [
        (0.0, 0.0, Ok(vec![ProgramValue::Nil])),
        (1.0, 1.0, Ok(vec![ProgramValue::Boolean(false)])),
        (
            2.0,
            2.0,
            Err(ProgramRuntimeErrorKind::UnsupportedCapability),
        ),
        (3.0, 2.0, Ok(vec![])),
    ] {
        let mut args = roots.clone();
        args.extend(
            session
                .borrow(&super::graph(vec![
                    ProgramValue::Number(start),
                    ProgramValue::Number(end),
                ]))
                .unwrap(),
        );
        match expected {
            Ok(expected) => {
                let values = session.invoke(ParserCallbackId(1), &args).unwrap();
                assert_values(
                    &session.snapshot(&values).unwrap().graph().values,
                    &expected,
                );
            }
            Err(expected) => assert_eq!(
                session.invoke(ParserCallbackId(1), &args).unwrap_err().kind,
                expected
            ),
        }
    }
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &roots)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let invalid = lib
        .execute(
            ParserCallbackId(1),
            &super::graph(vec![
                ProgramValue::Boolean(false),
                text("bad"),
                text("also bad"),
            ]),
            ProgramLimits::default(),
        )
        .unwrap_err();
    assert_eq!(invalid.kind, ProgramRuntimeErrorKind::Source);
    assert_eq!(invalid.message, "table argument expected");
    let invalid = lib
        .execute(
            ParserCallbackId(1),
            &input(Some(text("bad")), None),
            ProgramLimits::default(),
        )
        .unwrap_err();
    assert_eq!(invalid.kind, ProgramRuntimeErrorKind::Source);
    assert_eq!(invalid.message, "number argument expected");
}
#[test]
fn result_and_work_reservations_bound_allocation_and_accumulate() {
    let lib = library();
    for (start, end, expected) in [
        (1.0, 5.0, ProgramRuntimeErrorKind::ResourceBound),
        (1.0, 8000.0, ProgramRuntimeErrorKind::Source),
        (
            i32::MIN as f64,
            i32::MAX as f64,
            ProgramRuntimeErrorKind::Source,
        ),
    ] {
        let limits = ProgramLimits {
            max_results: 4,
            ..ProgramLimits::default()
        };
        let (mut session, args) = lib
            .session(
                &input(
                    Some(ProgramValue::Number(start)),
                    Some(ProgramValue::Number(end)),
                ),
                limits,
            )
            .unwrap();
        let before = session.allocations();
        assert_eq!(
            session.invoke(ParserCallbackId(1), &args).unwrap_err().kind,
            expected
        );
        assert_eq!(session.allocations().tables, before.tables);
        assert!(session.allocations().values <= limits.max_values);
    }
    let limits = ProgramLimits {
        pattern: MatchLimits {
            max_steps: 5,
            ..MatchLimits::default()
        },
        ..ProgramLimits::default()
    };
    let (mut session, args) = lib
        .session(
            &input(
                Some(ProgramValue::Number(1.0)),
                Some(ProgramValue::Number(3.0)),
            ),
            limits,
        )
        .unwrap();
    assert_eq!(session.invoke(ParserCallbackId(1), &args).unwrap().len(), 3);
    assert_eq!(
        session.invoke(ParserCallbackId(1), &args).unwrap_err().kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let (mut independent, args) = lib
        .session(
            &input(
                Some(ProgramValue::Number(1.0)),
                Some(ProgramValue::Number(3.0)),
            ),
            limits,
        )
        .unwrap();
    assert_eq!(
        independent
            .invoke(ParserCallbackId(1), &args)
            .unwrap()
            .len(),
        3
    );
}
#[test]
fn int32_boundary_keeps_nonfinite_and_overflow_as_portable_frontiers() {
    let lib = library();
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483648.0,
        -2147483649.0,
    ] {
        assert_eq!(
            lib.execute(
                ParserCallbackId(1),
                &input(
                    Some(ProgramValue::Number(value)),
                    Some(ProgramValue::Number(value))
                ),
                ProgramLimits::default()
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
    }
    for value in [i32::MIN as f64, i32::MAX as f64] {
        let result = lib
            .execute(
                ParserCallbackId(1),
                &input(
                    Some(ProgramValue::Number(value)),
                    Some(ProgramValue::Number(value)),
                ),
                ProgramLimits::default(),
            )
            .unwrap();
        assert_values(&result.graph().values, &[ProgramValue::Nil]);
    }
}
