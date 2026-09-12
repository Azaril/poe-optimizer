//! Complete observed source functions; result packs and failure prefixes against LuaJIT.
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::*, lower_from_sources};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const PATH: &str = "tests/support/source_program_strings.lua";
const TEXT: &str = include_str!("support/source_program_strings.lua");
struct Fixture {
    lua: Lua,
    functions: BTreeMap<String, Function>,
    compiled: CompiledSourcePrograms,
    callbacks: BTreeMap<String, SourceCallbackId>,
}
impl Fixture {
    fn new() -> Self {
        let lua = Lua::new();
        lua.load("jit.off(); jit.flush(); assert(not jit.status())")
            .exec()
            .unwrap();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let functions: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let functions = functions
            .pairs::<String, Function>()
            .map(Result::unwrap)
            .collect();
        let sources = BTreeMap::from([(PATH.into(), TEXT.into())]);
        let source = ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: [(
                PATH.into(),
                format!("{:x}", Sha256::digest(TEXT.as_bytes())),
            )]
            .into(),
            construction_spans: Default::default(),
            module_order: vec![PATH.into()],
        };
        let observed = observer
            .observe_with_context(
                &lua,
                &sources,
                source,
                &functions,
                SourceCaptureContext::default(),
            )
            .unwrap();
        let lowered = lower_from_sources(&sources, observed.owner()).unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{:?}",
            lowered.unsupported()
        );
        Self {
            lua,
            functions,
            compiled: CompiledSourcePrograms::new(lowered.catalog()).unwrap(),
            callbacks: observed.callbacks().clone(),
        }
    }
    fn bytes(&self, value: &[u8]) -> Value {
        Value::String(self.lua.create_string(value).unwrap())
    }
    fn source(&self, name: &str, args: &[Value]) -> mlua::Result<MultiValue> {
        self.functions[name].call(MultiValue::from_vec(args.to_vec()))
    }
    fn compare(&self, name: &str, args: &[Value]) -> Option<ProgramValueGraph> {
        let actual = self.source(name, args);
        let native = self.compiled.execute(
            self.callbacks[name],
            &observation::capture(args),
            ProgramLimits::default(),
        );
        match (actual, native) {
            (Ok(actual), Ok(native)) => {
                assert_eq!(
                    observation::canonical(native.graph()),
                    observation::canonical(&observation::capture(&actual.into_vec())),
                    "{name} {args:?}"
                );
                Some(native.graph().clone())
            }
            (Err(actual), Err(native)) => {
                assert_eq!(
                    native.kind,
                    ProgramRuntimeErrorKind::Source,
                    "source error must not be an unsupported substitute: {name} {args:?}: {actual}; {native}"
                );
                None
            }
            (actual, native) => panic!("{name} {args:?}: source={actual:?}, native={native:?}"),
        }
    }
}
#[test]
fn original_lower_binary_coercion_and_full_packs() {
    let f = Fixture::new();
    let all_bytes: Vec<u8> = (0..=255).collect();
    for value in [
        f.bytes(b""),
        f.bytes(b"AbC XYZ 123"),
        f.bytes(b"A\0Z\xff"),
        f.bytes(b"\xc3\x89COLE \xce\xa3"),
        f.bytes(&all_bytes),
        Value::Integer(123),
        Value::Number(-0.0),
        Value::Number(12.5),
        Value::Number(1e20),
    ] {
        for name in ["captured_lower", "global_lower"] {
            let result = f
                .compare(name, &[value.clone(), Value::Boolean(false)])
                .unwrap();
            assert_eq!(result.values.len(), 1);
        }
        if matches!(value, Value::String(_)) {
            f.compare("method_lower", &[value]);
        }
    }
    for args in [
        vec![],
        vec![Value::Nil],
        vec![Value::Boolean(false)],
        vec![Value::Table(f.lua.create_table().unwrap())],
    ] {
        assert!(f.compare("captured_lower", &args).is_none());
    }
}
#[test]
fn original_find_patterns_plain_bytes_positions_and_failure_order() {
    let f = Fixture::new();
    let cases: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"abc", b""),
        (b"abcabc", b"bc"),
        (b"abc", b"z"),
        (b"a\0b\xff", b"\0b"),
        (b"a\0b\xff", b"b\xff"),
        (b"a\0b", b"\0."),
        (b"a12 b345", b"(%a+)(%d+)"),
        (b"a12", b"()(%a+)()"),
        (b"(a(b)c)tail", b"%b()"),
        (b"cat cats", b"%f[%a]cat%f[%A]"),
        (b"abc", b"^a"),
        (b"abc", b"c$"),
        (b"abc", b"$"),
        (b"aaa", b"(a*)"),
        (b"abc", b"["),
        (b"abc", b"(."),
        (b"abc", b"%"),
        (b"abc", b"%1"),
        (b"abab", b"(ab)%1"),
        (b"a\0b", b".\0["),
    ];
    let starts = [
        Value::Nil,
        Value::Integer(0),
        Value::Integer(1),
        Value::Integer(2),
        Value::Integer(4),
        Value::Integer(100),
        Value::Integer(-1),
        Value::Integer(-100),
        Value::Number(1.9),
        f.bytes(b" 0x2 "),
    ];
    for &(subject, pattern) in cases {
        for start in &starts {
            f.compare(
                "captured_find",
                &[f.bytes(subject), f.bytes(pattern), start.clone()],
            );
        }
        f.compare("method_find", &[f.bytes(subject), f.bytes(pattern)]);
        f.compare(
            "global_find",
            &[
                f.bytes(subject),
                f.bytes(pattern),
                Value::Nil,
                Value::Boolean(false),
            ],
        );
    }
    for plain in [
        Value::Boolean(true),
        Value::Integer(0),
        f.bytes(b""),
        Value::Table(f.lua.create_table().unwrap()),
    ] {
        for (subject, needle) in [
            (b"a[b".as_slice(), b"[".as_slice()),
            (b"a\0[", b"\0["),
            (b"a(b", b"("),
            (b"abc", b""),
            (b"abc", b"z"),
        ] {
            f.compare(
                "captured_find",
                &[
                    f.bytes(subject),
                    f.bytes(needle),
                    Value::Integer(1),
                    plain.clone(),
                ],
            );
        }
    }
    for args in [
        vec![],
        vec![Value::Nil, f.bytes(b".")],
        vec![f.bytes(b"a"), Value::Boolean(false)],
        vec![f.bytes(b"a"), f.bytes(b"a"), Value::Boolean(false)],
        vec![Value::Integer(12321), Value::Integer(23)],
        vec![Value::Number(-0.0), f.bytes(b"0")],
        vec![f.bytes(b"abc"), f.bytes(b"["), Value::Integer(100)],
        vec![f.bytes(b"abc"), f.bytes(b"a"), f.bytes(b"no")],
    ] {
        f.compare("captured_find", &args);
    }
    let captures = f
        .compare(
            "captured_find",
            &[f.bytes(b"a12"), f.bytes(b"()(%a+)(%d+)()")],
        )
        .unwrap();
    assert_eq!(
        captures.values.len(),
        6,
        "start/end and all four captures survive"
    );
    let missing = f
        .compare("captured_find", &[f.bytes(b"abc"), f.bytes(b"z")])
        .unwrap();
    assert_eq!(
        missing.values,
        vec![ProgramValue::Nil],
        "find no-match is one nil"
    );
}
#[test]
fn original_sub_clipping_defaults_binary_and_conversion_order() {
    let f = Fixture::new();
    let bounds = [
        Value::Nil,
        Value::Integer(0),
        Value::Integer(1),
        Value::Integer(2),
        Value::Integer(5),
        Value::Integer(100),
        Value::Integer(-1),
        Value::Integer(-2),
        Value::Integer(-100),
        Value::Number(-1.9),
        f.bytes(b"2e0"),
        f.bytes(b"no"),
        Value::Boolean(false),
    ];
    for start in &bounds {
        for finish in &bounds {
            f.compare(
                "captured_sub",
                &[f.bytes(b"A\0b\xff"), start.clone(), finish.clone()],
            );
        }
        f.compare("global_sub", &[f.bytes(b"abcd"), start.clone()]);
        f.compare("method_sub", &[f.bytes(b"abcd"), start.clone()]);
    }
    for args in [
        vec![],
        vec![f.bytes(b"a")],
        vec![Value::Nil, Value::Integer(1)],
        vec![Value::Boolean(false), Value::Integer(1)],
        vec![Value::Integer(1234), Value::Integer(2), Value::Integer(3)],
        vec![f.bytes(b""), Value::Integer(1)],
        vec![
            f.bytes(b"abcd"),
            Value::Integer(i64::from(i32::MIN)),
            Value::Integer(i64::from(i32::MAX)),
        ],
    ] {
        f.compare("captured_sub", &args);
    }
}
#[test]
fn source_argument_effects_saved_method_targets_and_error_prefixes() {
    let f = Fixture::new();
    let ordered = f.compare("method_order", &[]).unwrap();
    assert_eq!(ordered.values[3], ProgramValue::Number(3.0));
    assert_eq!(ordered.values[4], ProgramValue::Boolean(true));
    for (name, rest, expected_count) in [
        ("lower_effect", vec![f.bytes(b"ABC")], 1),
        ("lower_effect", vec![Value::Nil], 1),
        ("find_effect", vec![f.bytes(b"abc"), f.bytes(b"[")], 1),
        (
            "find_effect",
            vec![Value::Boolean(false), Value::Boolean(false)],
            1,
        ),
        (
            "sub_effect",
            vec![f.bytes(b"abc"), Value::Boolean(false)],
            1,
        ),
        ("method_effect", vec![Value::Integer(123)], 0),
        ("method_effect", vec![f.bytes(b"ABC")], 1),
        ("failed_target", vec![Value::Boolean(false)], 1),
        ("failed_target", vec![Value::Nil], 1),
    ] {
        let side = f.lua.create_table().unwrap();
        side.raw_set("count", 0).unwrap();
        let mut args = vec![Value::Table(side.clone())];
        args.extend(rest);
        let (mut session, native_args) = f
            .compiled
            .session(&observation::capture(&args), ProgramLimits::default())
            .unwrap();
        let original = f.source(name, &args);
        let actual = session.invoke(f.callbacks[name], &native_args);
        match (original, actual) {
            (Ok(source), Ok(native)) => assert_eq!(
                observation::canonical(session.snapshot(&native).unwrap().graph()),
                observation::canonical(&observation::capture(&source.into_vec())),
                "{name}"
            ),
            (Err(_), Err(error)) => assert_eq!(
                error.kind,
                ProgramRuntimeErrorKind::Source,
                "{name}: {error}"
            ),
            (source, native) => panic!("{name}: {source:?} {native:?}"),
        }
        assert_eq!(side.raw_get::<i64>("count").unwrap(), expected_count);
        assert_eq!(
            observation::canonical(session.snapshot(&native_args[..1]).unwrap().graph()),
            observation::canonical(&observation::capture(&args[..1])),
            "prefix {name}"
        );
    }
}
#[test]
fn nonportable_index_conversions_remain_explicit_frontiers_after_prior_checks() {
    let f = Fixture::new();
    let mut observations = vec![];
    for bound in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483648.0,
        -2147483649.0,
    ] {
        for (name, args) in [
            ("captured_sub", vec![f.bytes(b"abc"), Value::Number(bound)]),
            (
                "captured_sub",
                vec![f.bytes(b"abc"), Value::Integer(1), Value::Number(bound)],
            ),
            (
                "captured_find",
                vec![f.bytes(b"abc"), f.bytes(b"a"), Value::Number(bound)],
            ),
        ] {
            let source = match f.source(name, &args) {
                Ok(values) => observation::canonical(&observation::capture(&values.into_vec())),
                Err(error) => serde_json::json!({"source_error":error.to_string()}),
            };
            let error = f
                .compiled
                .execute(
                    f.callbacks[name],
                    &observation::capture(&args),
                    ProgramLimits::default(),
                )
                .unwrap_err();
            assert_eq!(
                error.kind,
                ProgramRuntimeErrorKind::UnsupportedCapability,
                "{name} {bound}: {error}"
            );
            observations.push(serde_json::json!({"function":name,"bound_bits":format!("{:016x}",bound.to_bits()),"source":source,"native":"unsupported"}));
        }
        assert!(
            f.compare(
                "captured_sub",
                &[Value::Boolean(false), Value::Number(bound)]
            )
            .is_none()
        );
        assert!(
            f.compare(
                "captured_find",
                &[f.bytes(b"abc"), Value::Boolean(false), Value::Number(bound)]
            )
            .is_none()
        );
    }
    println!(
        "index_conversion_frontiers={}",
        serde_json::to_string(&observations).unwrap()
    );
}
#[test]
fn exact_original_functions_match_warmed_valid_full_packs() {
    let f = Fixture::new();
    let driver = warm::SourceWarmDriver::new(&f.lua).unwrap();
    let cases = [
        ("warm_lower", vec![f.bytes(b"AbC XYZ")]),
        ("warm_lower", vec![f.bytes(b"A\0Z\xff")]),
        ("warm_lower", vec![f.bytes(b"")]),
        ("warm_lower", vec![Value::Number(123.5)]),
        (
            "warm_sub",
            vec![f.bytes(b"A\0bc\xff"), Value::Integer(2), Value::Integer(-1)],
        ),
        ("warm_sub", vec![f.bytes(b"abc"), Value::Integer(9)]),
        ("warm_sub", vec![f.bytes(b"abc"), Value::Number(-1.9)]),
        (
            "warm_sub",
            vec![Value::Integer(1234), Value::Integer(2), Value::Integer(3)],
        ),
        ("warm_find", vec![f.bytes(b"abcabc"), f.bytes(b"bc")]),
        ("warm_find", vec![f.bytes(b"abc"), f.bytes(b"z")]),
        (
            "warm_find",
            vec![
                f.bytes(b"a\0[b"),
                f.bytes(b"\0["),
                Value::Integer(1),
                Value::Boolean(true),
            ],
        ),
        (
            "warm_find",
            vec![f.bytes(b"abc"), f.bytes(b""), Value::Integer(100)],
        ),
        (
            "warm_find",
            vec![f.bytes(b"abcabc"), f.bytes(b"bc"), Value::Integer(-3)],
        ),
        ("warm_find", vec![Value::Integer(1234), Value::Integer(23)]),
    ];
    for (name, args) in &cases {
        let cold = f.compare(name, args).unwrap();
        let warmed = driver
            .run(&f.lua, &f.functions[*name], args, None)
            .unwrap_or_else(|error| panic!("{name} {args:?}: {error}"));
        assert!(warmed.success, "{name} {args:?}: {:?}", warmed.value);
        assert_eq!(warmed.calls, 128);
        assert_eq!(warmed.seed_calls, 0);
        assert!(
            warmed.target_live_traces > 0,
            "exact original {name} must occur in completed live trace: {args:?}"
        );
        assert!(warmed.live_traces >= warmed.target_live_traces);
        assert_eq!(
            observation::canonical(&cold),
            observation::canonical(&observation::capture(&[warmed.value])),
            "warm {name} {args:?}"
        );
    }
    println!(
        "exact_function_warm_vectors={}; calls_per_vector=128; pattern_find_is_interpreted_only",
        cases.len()
    );
}
