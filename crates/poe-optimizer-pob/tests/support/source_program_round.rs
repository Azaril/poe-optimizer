//! Differential scalar edges for the captured, complete original Common.round.
//! The warm driver calls that actual function; it contains no rounding formula.
use super::*;
use poe_optimizer_data::source_program::SourceCallbackId;
fn comparable(mut graph: ProgramValueGraph) -> Json {
    for value in &mut graph.values {
        if let ProgramValue::Number(value) = value
            && value.is_nan()
        {
            *value = f64::NAN;
        }
    }
    observation::canonical(&graph)
}
pub fn compare(
    lua: &Lua,
    compiled: &CompiledSourcePrograms,
    original: &Function,
    callback: SourceCallbackId,
) -> Json {
    assert_eq!(
        original.to_pointer(),
        lua.globals().get::<Function>("round").unwrap().to_pointer(),
        "same actual function used for native source capture"
    );
    let info = original.info();
    assert_eq!(info.source.as_deref(), Some("@Modules/Common.lua"));
    assert_eq!(
        (info.line_defined, info.last_line_defined),
        (Some(722), Some(728))
    );
    let bytes = |value: &[u8]| Value::String(lua.create_string(value).unwrap());
    let values = [
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        0.5,
        -0.5,
        2.5,
        -2.5,
        f64::from_bits(2.5f64.to_bits() - 1),
        f64::from_bits(2.5f64.to_bits() + 1),
        -f64::from_bits(2.5f64.to_bits() - 1),
        -f64::from_bits(2.5f64.to_bits() + 1),
        1.005,
        2.675,
        -1.005,
        -2.675,
        4503599627370495.5,
        4503599627370496.0,
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    let mut cases = Vec::new();
    for value in values {
        cases.push(vec![Value::Number(value)]);
        for decimal in [
            Value::Nil,
            Value::Boolean(false),
            Value::Integer(0),
            Value::Integer(2),
            Value::Integer(-2),
            Value::Number(0.5),
            Value::Integer(308),
            Value::Integer(309),
            Value::Integer(-323),
            Value::Integer(-324),
            Value::Number(f64::INFINITY),
            Value::Number(f64::NEG_INFINITY),
            Value::Number(f64::NAN),
            bytes(b"2"),
            bytes(b"0x2"),
        ] {
            cases.push(vec![Value::Number(value), decimal]);
        }
    }
    cases.extend([
        vec![],
        vec![Value::Nil],
        vec![Value::Boolean(false)],
        vec![bytes(b"bad")],
        vec![bytes(b"2.5\0")],
        vec![bytes(b"2.5")],
        vec![bytes(b"-2.5"), bytes(b"2")],
        vec![bytes(b"0x1.8p1"), bytes(b"0b10")],
        vec![Value::Number(2.5), Value::Boolean(true)],
        vec![Value::Number(2.5), bytes(b"bad")],
        vec![Value::Number(2.5), bytes(b"2\0")],
        vec![Value::Number(2.5), Value::Nil, Value::Boolean(false)],
        vec![Value::Table(lua.create_table().unwrap())],
        vec![
            Value::Number(2.5),
            Value::Table(lua.create_table().unwrap()),
        ],
    ]);
    let jit: Table = lua.globals().get("jit").unwrap();
    let status: MultiValue = jit.get::<Function>("status").unwrap().call(()).unwrap();
    let initially_enabled = status.front() == Some(&Value::Boolean(true));
    let driver = warm::SourceWarmDriver::new(lua).unwrap();
    let mut modes = Vec::new();
    for warm in [false, true] {
        jit.get::<Function>(if warm { "on" } else { "off" })
            .unwrap()
            .call::<()>(())
            .unwrap();
        jit.get::<Function>("flush")
            .unwrap()
            .call::<()>(())
            .unwrap();
        let mut warm_cases = Vec::new();
        let mut successes = 0;
        let mut failures = 0;
        for (index, args) in cases.iter().enumerate() {
            let cold = original.call::<MultiValue>(MultiValue::from_vec(args.clone()));
            let expected = if warm {
                let seed = [Value::Number(2.5), Value::Integer(2)];
                let result = driver
                    .run(lua, original, args, cold.is_err().then_some(&seed[..]))
                    .unwrap();
                assert_eq!(
                    result.success,
                    cold.is_ok(),
                    "success must agree across oracle modes"
                );
                assert_eq!(result.calls, 128);
                assert!(result.target_live_traces > 0);
                warm_cases.push(json!({"case":index,"calls":result.calls,"seed_calls":result.seed_calls,"live_traces":result.live_traces,"original_function_live_traces":result.target_live_traces,"trace_scope":if cold.is_err() {"valid_seed_before_invalid_vector"} else {"actual_vector"}}));
                result
                    .success
                    .then(|| MultiValue::from_vec(vec![result.value]))
            } else {
                cold.ok()
            };
            let actual = compiled.execute(
                callback,
                &observation::capture(args),
                ProgramLimits::default(),
            );
            match (expected, actual) {
                (Some(expected), Ok(actual)) => {
                    assert_eq!(expected.len(), 1, "round returns one value");
                    assert_eq!(
                        comparable(actual.graph().clone()),
                        comparable(observation::capture(&expected.into_vec())),
                        "original round case {index}, warm={warm}, args={args:?}"
                    );
                    successes += 1;
                }
                (None, Err(actual)) => {
                    assert_eq!(
                        actual.kind,
                        ProgramRuntimeErrorKind::Source,
                        "case {index}, warm={warm}: {actual}"
                    );
                    failures += 1;
                }
                (expected, actual) => panic!(
                    "round case {index}, warm={warm}, args={args:?}: source={expected:?}, native={actual:?}"
                ),
            }
        }
        modes.push(json!({"mode":if warm {"jit_enabled_128_calls"} else {"interpreter"},"successes":successes,"source_errors":failures,"warm_cases":warm_cases,"cases":cases.len()}));
    }
    jit.get::<Function>(if initially_enabled { "on" } else { "off" })
        .unwrap()
        .call::<()>(())
        .unwrap();
    json!({"source":"src/Modules/Common.lua","first_line":722,"last_line":728,"modes":modes,"numeric_comparison":"exact finite and signed-zero bits; NaN payloads normalized only","scope":"complete captured original function including decimal branch"})
}
