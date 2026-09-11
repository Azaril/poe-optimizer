//! Complete source lowering/execution against cold and traced original LuaJIT functions.
#[path = "support/source_program_warm.rs"]
mod source_program_warm;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::SourceClosureObserver, lower_from_sources};
use sha2::{Digest, Sha256};
use source_program_warm::SourceWarmDriver;
use std::collections::BTreeMap;
const PATH: &str = "tests/support/source_program_arithmetic.lua";
const TEXT: &str = include_str!("support/source_program_arithmetic.lua");

fn text(value: &str) -> ProgramValue {
    ProgramValue::Bytes(value.as_bytes().to_vec())
}
fn number(value: f64) -> ProgramValue {
    ProgramValue::Number(value)
}
fn lua_value(lua: &Lua, value: &ProgramValue) -> Value {
    match value {
        ProgramValue::Nil => Value::Nil,
        ProgramValue::Boolean(value) => Value::Boolean(*value),
        ProgramValue::Number(value) => Value::Number(*value),
        ProgramValue::Bytes(value) => Value::String(lua.create_string(value).unwrap()),
        _ => panic!("scalar fixture value"),
    }
}
fn cases() -> BTreeMap<&'static str, Vec<Vec<ProgramValue>>> {
    let mut power = Vec::new();
    for a in [
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        10.0,
        0.1,
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1.0f64.to_bits() - 1),
        f64::from_bits(1.0f64.to_bits() + 1),
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        for b in [
            0.0,
            -0.0,
            1.0,
            -1.0,
            2.0,
            -2.0,
            3.0,
            -3.0,
            0.5,
            -0.5,
            308.0,
            309.0,
            -323.0,
            -324.0,
            9007199254740992.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            power.push(vec![number(a), number(b)]);
        }
    }
    power.extend([
        vec![],
        vec![number(2.0)],
        vec![text(" 0x2 "), text("0b11")],
        vec![text("-0"), text("-3")],
        vec![text("nan"), text("0")],
        vec![ProgramValue::Boolean(false), number(2.0)],
        vec![number(2.0), ProgramValue::Boolean(false)],
        vec![text("bad"), text("bad")],
        vec![text("2\0"), text("3")],
    ]);
    assert_eq!(power.len(), 297);
    let mut floor = vec![
        vec![],
        vec![ProgramValue::Nil],
        vec![ProgramValue::Boolean(false)],
        vec![text("bad")],
        vec![text("1\0")],
        vec![text(" 0x1.8p1 ")],
        vec![text("0b101")],
        vec![text("-0")],
        vec![text("inf")],
        vec![text("-infinity")],
        vec![text("nan")],
        vec![number(2.75), text("ignored"), ProgramValue::Nil],
    ];
    for value in [
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        0.5,
        -0.5,
        1.0,
        -1.0,
        f64::from_bits(1.0f64.to_bits() - 1),
        f64::from_bits(1.0f64.to_bits() + 1),
        4503599627370495.5,
        4503599627370496.0,
        -4503599627370495.5,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0xfff8_0000_0000_0000),
    ] {
        floor.push(vec![number(value)]);
    }
    let precedence = [
        [2.0, 2.0, 2.0],
        [2.0, 3.0, 2.0],
        [-2.0, 3.0, 2.0],
        [2.0, -2.0, 2.0],
        [-2.0, -3.0, 1.0],
        [4.0, 0.5, 2.0],
        [0.0, 2.0, 3.0],
        [-0.0, 3.0, 1.0],
        [10.0, 2.0, 0.5],
    ]
    .into_iter()
    .map(|row| row.into_iter().map(number).collect())
    .collect::<Vec<_>>();
    [
        ("power", power),
        ("floor", floor),
        ("negative_base", precedence.clone()),
        ("negative_exponent", precedence.clone()),
        ("right_associative", precedence),
    ]
    .into()
}
fn equal_value(expected: &Value, actual: &ProgramValue, label: &str) {
    let expected = match expected {
        Value::Number(value) => *value,
        Value::Integer(value) => *value as f64,
        other => panic!("{label}: unexpected source value {other:?}"),
    };
    let ProgramValue::Number(actual) = actual else {
        panic!("{label}: unexpected native value {actual:?}")
    };
    if expected.is_nan() && actual.is_nan() {
        return;
    }
    assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "{label}: expected {expected:?}, actual {actual:?}"
    );
}
#[test]
fn complete_arithmetic_source_matches_interpreted_and_traced_luajit() {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let functions: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let roots = functions
        .pairs::<String, Function>()
        .map(Result::unwrap)
        .collect::<BTreeMap<_, _>>();
    assert_eq!(roots.len(), 5);
    let sources = [(PATH.into(), TEXT.into())].into();
    let provenance = ItemLoadingSource {
        upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
        files: [(
            PATH.into(),
            format!("{:x}", Sha256::digest(TEXT.as_bytes())),
        )]
        .into(),
        construction_spans: BTreeMap::new(),
        module_order: vec![PATH.into()],
    };
    let observed = observer
        .observe(&lua, &sources, provenance, &roots)
        .unwrap();
    let (definitions, callbacks) = observed.into_parts();
    let owner = SourceProgramOwner::new(definitions).unwrap();
    let lowered = lower_from_sources(&sources, &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 5);
    let native = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    let warm = SourceWarmDriver::new(&lua).unwrap();
    let mut compared = 0usize;
    let mut errors = 0usize;
    let mut target_traces = 0usize;
    for (name, cases) in cases() {
        let function = &roots[name];
        for (index, arguments) in cases.into_iter().enumerate() {
            lua.load("jit.off(); jit.flush(); assert(not jit.status())")
                .exec()
                .unwrap();
            let source_arguments = arguments
                .iter()
                .map(|value| lua_value(&lua, value))
                .collect::<Vec<_>>();
            let source =
                function.call::<MultiValue>(MultiValue::from_vec(source_arguments.clone()));
            let expected_error = source.is_err();
            let actual = native.execute(
                callbacks[name],
                &ProgramValueGraph {
                    values: arguments.clone(),
                    tables: vec![],
                },
                ProgramLimits::default(),
            );
            let label = format!("{name}[{index}] {arguments:?}");
            match (&source, &actual) {
                (Ok(source), Ok(actual)) => {
                    assert_eq!(source.len(), 1, "{label}");
                    assert_eq!(actual.graph().values.len(), 1, "{label}");
                    equal_value(
                        &source[0],
                        &actual.graph().values[0],
                        &format!("cold {label}"),
                    );
                }
                (Err(_), Err(actual)) => {
                    assert_eq!(actual.kind, ProgramRuntimeErrorKind::Source, "{label}");
                    errors += 1;
                }
                _ => panic!("cold {label}: source={source:?}, native={actual:?}"),
            }
            let seed = [Value::Number(2.75), Value::Number(3.0), Value::Number(2.0)];
            let traced = warm
                .run(
                    &lua,
                    function,
                    &source_arguments,
                    expected_error.then_some(seed.as_slice()),
                )
                .unwrap();
            assert_eq!(traced.calls, 128);
            assert_eq!(traced.seed_calls, if expected_error { 128 } else { 0 });
            assert!(traced.target_live_traces > 0);
            assert!(traced.live_traces >= traced.target_live_traces);
            target_traces += traced.target_live_traces;
            assert_eq!(
                traced.success, !expected_error,
                "warm error category {label}"
            );
            if let Ok(actual) = actual {
                equal_value(
                    &traced.value,
                    &actual.graph().values[0],
                    &format!("warm {label}"),
                );
            }
            compared += 1;
        }
    }
    assert_eq!(compared, 354);
    eprintln!(
        "Arithmetic source parity: {compared} vectors in both cold and warmed LuaJIT; {errors} source-error vectors; {target_traces} completed live target traces; 128 measured calls per warm vector."
    );
}
