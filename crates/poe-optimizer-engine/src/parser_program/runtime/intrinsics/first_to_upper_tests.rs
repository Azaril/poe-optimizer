//! Authored runtime contracts; complete original-source differential is in PoB.
use super::*;
use crate::lua_pattern::MatchLimits;
use crate::parser_program::{
    CompiledParserPrograms, ProgramRuntimeErrorKind, ProgramValue, ProgramValueGraph,
};
use poe_optimizer_data::game_data::bundled_snapshot;
use poe_optimizer_data::modifier_parser::*;
use std::collections::BTreeMap;

fn owner(pattern: &str) -> ModifierParserCatalog {
    let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
    data.programs = ParserProgramPayload::default();
    data.policy.first_to_upper_pattern = pattern.into();
    ModifierParserCatalog::new(data).unwrap()
}
fn bytes(value: &[u8]) -> V {
    V::Bytes(Arc::from(value))
}
fn fixture(pattern: &str) -> (Heap<'static>, MatchBudget, ProgramLimits, ParserCallbackId) {
    let owner = owner(pattern);
    let callback = owner.data().helpers["firstToUpper"];
    let limits = ProgramLimits::default();
    let (heap, _) = Heap::new(&owner, &ProgramValueGraph::default(), &limits).unwrap();
    (heap, MatchBudget::new(limits.pattern), limits, callback)
}
fn invoke(
    callback: ParserCallbackId,
    arguments: &[V],
    heap: &mut Heap,
    work: &mut MatchBudget,
    limits: &ProgramLimits,
) -> RuntimeResult<Vec<V>> {
    call_bound(
        ParserProgramIntrinsic::FirstToUpper,
        Some(callback),
        &BTreeMap::new(),
        arguments,
        heap,
        work,
        limits,
    )
}
fn one_bytes(result: Vec<V>, expected: &[u8]) {
    assert_eq!(
        result.len(),
        1,
        "parenthesized helper result discards gsub's count"
    );
    assert_eq!(result[0].as_bytes(), Some(expected));
}

#[test]
fn first_to_upper_keeps_byte_capture_empty_and_anchor_semantics() {
    let all: Vec<u8> = (0..=255).collect();
    let upper: Vec<u8> = all.iter().map(u8::to_ascii_uppercase).collect();
    let cases: Vec<(&str, &[u8], &[u8])> = vec![
        ("(.)", &all, &upper),
        ("()(.)", b"ab", b"12"),
        ("(.)()", b"ab", b"AB"),
        ("()", b"ab", b"1a2b3"),
        ("", b"ab", b"ab"),
        ("^%l", b"a\0z", b"A\0z"),
        ("^%l", "éclair".as_bytes(), "éclair".as_bytes()),
        ("(.)", "aéz".as_bytes(), "AéZ".as_bytes()),
        ("a*", b"ab", b"Ab"),
        ("^z[", b"ab", b"ab"),
    ];
    for (pattern, input, expected) in cases {
        let (mut heap, mut work, limits, callback) = fixture(pattern);
        one_bytes(
            invoke(callback, &[bytes(input)], &mut heap, &mut work, &limits).unwrap(),
            expected,
        );
    }
}

#[test]
fn first_to_upper_ignores_surplus_arguments_and_needs_exact_owner_binding() {
    let (mut heap, mut work, limits, callback) = fixture("^%l");
    let extra = heap.new_table().unwrap();
    one_bytes(
        invoke(
            callback,
            &[
                bytes(b"cold"),
                V::Nil,
                V::Boolean(false),
                extra,
                V::Callback(callback),
            ],
            &mut heap,
            &mut work,
            &limits,
        )
        .unwrap(),
        b"Cold",
    );
    for called in [None, Some(ParserCallbackId(u32::MAX))] {
        let error = call_bound(
            ParserProgramIntrinsic::FirstToUpper,
            called,
            &BTreeMap::new(),
            &[bytes(b"cold")],
            &mut heap,
            &mut work,
            &limits,
        )
        .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    }
    assert_eq!(
        call(
            ParserProgramIntrinsic::FirstToUpper,
            &[bytes(b"cold")],
            &mut heap,
            &mut work,
            &limits
        )
        .unwrap_err()
        .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}

#[test]
fn first_to_upper_resolves_receiver_before_lazy_pattern_and_resource_work() {
    let (mut heap, mut work, limits, callback) = fixture("[");
    for args in [
        vec![],
        vec![V::Nil],
        vec![V::Number(3.0)],
        vec![V::Boolean(false)],
        vec![V::Callback(callback)],
    ] {
        let previous_work = work.steps_used();
        let error = invoke(callback, &args, &mut heap, &mut work, &limits).unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
        assert!(error.message.contains("index"));
        assert_eq!(work.steps_used(), previous_work);
    }
    let error = invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &limits).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert!(!error.message.contains("receiver"));
    heap.charge_bytes(heap.remaining_bytes()).unwrap();
    // Even a completely exhausted output heap cannot hide this earlier source error.
    let error = invoke(callback, &[V::Number(3.0)], &mut heap, &mut work, &limits).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
}

#[test]
fn first_to_upper_table_methods_remain_explicit_and_do_not_mutate_receiver() {
    let (mut heap, mut work, limits, callback) = fixture("[");
    let receiver = heap.new_table().unwrap();
    let table_method = heap.new_table().unwrap();
    for method in [
        V::Nil,
        V::Boolean(false),
        V::Number(0.0),
        bytes(b"text"),
        table_method,
    ] {
        heap.set(&receiver, bytes(b"gsub"), method, &mut work)
            .unwrap();
        assert_eq!(
            invoke(
                callback,
                std::slice::from_ref(&receiver),
                &mut heap,
                &mut work,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
    heap.set(&receiver, bytes(b"gsub"), V::Callback(callback), &mut work)
        .unwrap();
    let error = invoke(
        callback,
        std::slice::from_ref(&receiver),
        &mut heap,
        &mut work,
        &limits,
    )
    .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    assert!(error.message.contains(&format!("{callback:?}")));
    assert!(
        matches!(heap.get(&receiver, &bytes(b"gsub")).unwrap(), V::Callback(id) if id == callback)
    );
}

#[test]
fn first_to_upper_charges_each_reached_compile_and_all_cumulative_output() {
    let (mut heap, mut work, limits, callback) = fixture("(.)");
    let before = heap.stats();
    let work_before = work.steps_used();
    one_bytes(
        invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &limits).unwrap(),
        b"AB",
    );
    let one = heap.stats();
    let one_work = work.steps_used();
    one_bytes(
        invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &limits).unwrap(),
        b"AB",
    );
    let two = heap.stats();
    assert_eq!(two.bytes - one.bytes, one.bytes - before.bytes);
    assert!(one.bytes - before.bytes >= LuaPattern::compile(b"(.)").unwrap().compiled_bytes() + 4);
    assert_eq!(work.steps_used() - one_work, one_work - work_before);
    heap.charge_bytes(heap.remaining_bytes()).unwrap();
    assert_eq!(
        invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &limits)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}

#[test]
fn first_to_upper_bounds_growth_work_subject_and_result_pack() {
    let (mut heap, _, limits, callback) = fixture("()");
    let mut work = MatchBudget::new(MatchLimits {
        max_steps: 0,
        ..MatchLimits::default()
    });
    assert_eq!(
        invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &limits)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let mut work = MatchBudget::new(MatchLimits {
        max_subject_bytes: 1,
        ..MatchLimits::default()
    });
    assert_eq!(
        invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &limits)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let mut work = MatchBudget::default();
    let no_results = ProgramLimits {
        max_results: 0,
        ..limits
    };
    assert_eq!(
        invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &no_results)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let (mut heap, mut work, limits, callback) = fixture("(.)");
    let compiled = LuaPattern::compile(b"(.)").unwrap().compiled_bytes();
    heap.charge_bytes(heap.remaining_bytes() - compiled - 1)
        .unwrap();
    let error = invoke(callback, &[bytes(b"ab")], &mut heap, &mut work, &limits).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
    assert!(heap.stats().bytes <= limits.max_bytes);
}

// Authored forwarding IR bound to a real captured slot solely tests the existing
// executor boundary. It is not evidence that this authored body came from source.
fn program(pattern: &str) -> (CompiledParserPrograms, ParserCallbackId, ParserCallbackId) {
    let mut data = owner(pattern).data().clone();
    let helper = data.helpers["firstToUpper"];
    let (caller, slot, p) = data
        .factories
        .iter()
        .find_map(|(&id, disposition)| {
            let ParserFactoryDisposition::Pure(recipe) = disposition else {
                return None;
            };
            let slot = data.callbacks[id.0 as usize - 1]
                .upvalues
                .iter()
                .position(|upvalue| upvalue.value == ParserValue::Callback(helper))?;
            Some((id, u16::try_from(slot).unwrap(), recipe.provenance.clone()))
        })
        .expect("packaged captured helper supplies a descriptor for authored mechanics");
    data.factories.insert(
        caller,
        ParserFactoryDisposition::Unsupported {
            reason: "authored FirstToUpper runtime contract".into(),
        },
    );
    let owner = ModifierParserCatalog::new(data).unwrap();
    let location = ParserProgramLocation { start: 0, end: 1 };
    let program = ParserProgram {
        callback: caller,
        parameter_count: 0,
        variadic: true,
        local_count: 0,
        bindings: vec![ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::FirstToUpper,
            source: ParserProgramIntrinsicSource::Captured {
                upvalue: slot,
                callback: helper,
            },
        }],
        body: vec![ParserProgramStatement {
            location,
            operation: ParserProgramStatementKind::Return {
                values: ParserProgramValueList {
                    values: vec![],
                    tail: Some(Box::new(ParserProgramPack::Call {
                        call: ParserProgramCall {
                            binding: 0,
                            receiver: None,
                            arguments: ParserProgramValueList {
                                values: vec![],
                                tail: Some(Box::new(ParserProgramPack::Varargs)),
                            },
                        },
                    })),
                },
            },
        }],
        provenance: ParserProgramProvenance {
            source: p.source,
            function_start: p.function_start,
            function_end: p.function_end,
            function_sha256: p.function_sha256,
        },
    };
    let catalog = ParserProgramCatalog::new(
        ParserProgramData {
            schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
            programs: vec![program],
            callbacks: BTreeMap::from([(caller, ParserProgramId(1))]),
        },
        owner,
    )
    .unwrap();
    (
        CompiledParserPrograms::new(&catalog).unwrap(),
        caller,
        helper,
    )
}

#[test]
fn first_to_upper_program_keeps_one_result_and_reports_actual_caller_callsite() {
    let (plan, caller, helper) = program("(.)");
    let input = ProgramValueGraph {
        values: vec![
            ProgramValue::Bytes(b"ab".to_vec()),
            ProgramValue::Nil,
            ProgramValue::Boolean(false),
        ],
        tables: vec![],
    };
    let output = plan
        .execute(caller, &input, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        output.graph().values,
        vec![ProgramValue::Bytes(b"AB".to_vec())]
    );
    for values in [vec![], vec![ProgramValue::Number(3.0)]] {
        let error = plan
            .execute(
                caller,
                &ProgramValueGraph {
                    values,
                    tables: vec![],
                },
                ProgramLimits::default(),
            )
            .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
        assert_eq!(error.callback, Some(caller));
        assert_ne!(caller, helper);
        assert_eq!(
            error.location,
            Some(ParserProgramLocation { start: 0, end: 1 })
        );
    }
    assert_eq!(input.values[0], ProgramValue::Bytes(b"ab".to_vec()));
}

#[test]
fn first_to_upper_reused_programs_keep_their_own_injected_pattern() {
    let (letters, caller, _) = program("(.)");
    let (positions, other, _) = program("()(.)");
    let input = ProgramValueGraph {
        values: vec![ProgramValue::Bytes(b"ab".to_vec())],
        tables: vec![],
    };
    for _ in 0..3 {
        for (plan, id, expected) in [(&letters, caller, b"AB"), (&positions, other, b"12")] {
            let result = plan.execute(id, &input, ProgramLimits::default()).unwrap();
            assert_eq!(
                result.graph().values,
                vec![ProgramValue::Bytes(expected.to_vec())]
            );
        }
    }
}
