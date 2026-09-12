//! Full source modulo/bit semantics through genuine retained primitives.
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::item_loading::ItemLoadingSource;
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::*, lower_from_sources};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};
const PATH: &str = "tests/support/source_program_bit_arithmetic.lua";
const TEXT: &str = include_str!("support/source_program_bit_arithmetic.lua");
struct Fixture {
    lua: Lua,
    functions: BTreeMap<String, Function>,
    observed: ObservedSourceSession,
    compiled: CompiledSourcePrograms,
}
struct Pair<'a> {
    fixture: &'a Fixture,
    session: ProgramSession,
    roots: Vec<SessionValue>,
}
impl Fixture {
    fn new() -> Self {
        Self::with_rebinding(false)
    }
    fn with_rebinding(rebind: bool) -> Self {
        let lua = Lua::new();
        lua.load("jit.off();jit.flush()").exec().unwrap();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let api: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let functions: BTreeMap<String, Function> = api.pairs().map(Result::unwrap).collect();
        if rebind {
            lua.globals()
                .raw_get::<Table>("bit")
                .unwrap()
                .raw_set("bor", functions["replacement"].clone())
                .unwrap();
        }
        let globals = lua.globals();
        let bit: Table = globals.raw_get("bit").unwrap();
        let texts = BTreeMap::from([(PATH.into(), TEXT.into())]);
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
            .observe_session(
                &lua,
                &texts,
                source,
                SourceSessionCaptureRequest {
                    callbacks: functions
                        .iter()
                        .filter(|(name, _)| name.as_str() != "replacement")
                        .map(|(name, f)| (name.clone(), f.clone()))
                        .collect(),
                    definitions: SourceCaptureContext {
                        environment: Some(SourceEnvironmentSelection {
                            table: globals.clone(),
                            root_name: "environment".into(),
                        }),
                        projections: vec![
                            SourceTableSelection {
                                table: globals,
                                fields: ["bit".into()].into(),
                                indexed: Default::default(),
                                allow_index_fallback: false,
                                allow_call_fallback: false,
                            },
                            SourceTableSelection {
                                table: bit,
                                fields: ["band", "bor", "bxor", "bnot"].map(str::to_owned).into(),
                                indexed: Default::default(),
                                allow_index_fallback: false,
                                allow_call_fallback: false,
                            },
                        ],
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        let lowered = lower_from_sources(&texts, observed.owner()).unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{:?}",
            lowered.unsupported()
        );
        let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
        Self {
            lua,
            functions,
            observed,
            compiled,
        }
    }
    fn pair(&self) -> Pair<'_> {
        let (session, roots) = self
            .compiled
            .session_from_input(
                self.observed.input(),
                ProgramLimits {
                    max_steps: 20_000_000,
                    max_values: 1_000_000,
                    max_bytes: 64 * 1024 * 1024,
                    ..Default::default()
                },
            )
            .unwrap();
        Pair {
            fixture: self,
            session,
            roots,
        }
    }
    fn text(&self, s: &str) -> Value {
        Value::String(self.lua.create_string(s).unwrap())
    }
}
impl Pair<'_> {
    fn root(&self, n: &str) -> SessionValue {
        self.roots[self.fixture.observed.root_index(n).unwrap()].clone()
    }
    fn import(&mut self, args: &[Value]) -> Vec<SessionValue> {
        self.session
            .import_with_coverage(&observation::capture(args), &ProgramTableCoverage::new())
            .unwrap()
    }
    fn call(
        &mut self,
        name: &str,
        args: &[SessionValue],
    ) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
        self.session.invoke_callable(&self.root(name), args)
    }
    fn compare(&mut self, source: &[Value], native: &[SessionValue], label: &str) {
        assert_eq!(source.len(), native.len(), "full result pack {label}");
        let actual = self.session.snapshot(native).unwrap();
        let mut expected = observation::capture(source);
        let mut actual = actual.graph().clone();
        // The established numeric contract treats NaN payload/sign as unspecified;
        // every non-NaN bit, including signed zero, remains exact.
        for graph in [&mut expected, &mut actual] {
            for value in &mut graph.values {
                if let ProgramValue::Number(v) = value
                    && v.is_nan()
                {
                    *v = f64::NAN;
                }
            }
        }
        assert_eq!(
            observation::canonical(&expected),
            observation::canonical(&actual),
            "{label}"
        );
    }
    fn paired(&mut self, name: &str, args: &[Value]) -> (bool, Vec<SessionValue>) {
        self.fixture
            .lua
            .load("jit.off();jit.flush()")
            .exec()
            .unwrap();
        let input = self.import(args);
        let source =
            self.fixture.functions[name].call::<MultiValue>(MultiValue::from_vec(args.to_vec()));
        let actual = self.call(name, &input);
        match (source, actual) {
            (Ok(source), Ok(actual)) => {
                self.compare(&source.into_vec(), &actual, name);
                (true, actual)
            }
            (Err(_), Err(error)) => {
                assert_eq!(
                    error.kind,
                    ProgramRuntimeErrorKind::Source,
                    "{name}: {error}"
                );
                (false, Vec::new())
            }
            (source, actual) => panic!("{name}/{args:?}: source={source:?}; native={actual:?}"),
        }
    }
}
fn save(name: &str, value: Json) {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../runs")
        .join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
fn numbers() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        0.1,
        -0.1,
        2147483647.0,
        2147483648.0,
        4294967295.0,
        4294967296.0,
        4503599627370495.5,
        4503599627370496.0,
        9007199254740991.0,
        9007199254740992.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ]
}
#[test]
fn original_modulo_matches_full_source_numeric_and_operand_semantics() {
    let f = Fixture::new();
    let mut p = f.pair();
    let mut cases = vec![];
    for a in numbers() {
        for b in numbers() {
            let args = [Value::Number(a), Value::Number(b)];
            let (success, out) = p.paired("modulo", &args);
            assert!(success);
            cases.push(json!({"a":format!("{:016x}",a.to_bits()),"b":format!("{:016x}",b.to_bits()),"result":observation::canonical(p.session.snapshot(&out).unwrap().graph())}));
        }
    }
    for args in [
        vec![],
        vec![Value::Number(2.0)],
        vec![Value::Nil, Value::Number(2.0)],
        vec![Value::Boolean(false), Value::Number(2.0)],
        vec![f.text("bad"), f.text("bad")],
        vec![f.text(" 0x1.8p2 "), f.text("0b10")],
        vec![f.text("-0"), f.text("3")],
        vec![f.text("nan"), f.text("inf")],
        vec![f.text("2\0"), f.text("1")],
    ] {
        p.paired("modulo", &args);
    }
    for args in [
        [13.0, 5.0, 3.0],
        [-13.0, 5.0, 3.0],
        [13.0, -5.0, 3.0],
        [1.5, 2.0, 0.5],
    ] {
        p.paired("precedence", &args.map(Value::Number));
    }
    save(
        "r2s-modulo-cold.json",
        json!({"numeric_cases":cases,"extra_conversion_cases":9,"precedence_cases":4,"nan_policy":"class only; finite/signed zero bits exact"}),
    );
}
#[test]
fn original_bit_primitives_preserve_all_arguments_conversion_and_retained_identity() {
    let f = Fixture::new();
    let mut p = f.pair();
    let mut count = 0;
    let mut values = numbers().into_iter().map(Value::Number).collect::<Vec<_>>();
    values.extend([
        f.text(" 0x1.8p2 "),
        f.text("0b11"),
        f.text("-0"),
        f.text("nan"),
        f.text("inf"),
        f.text("1.5"),
    ]);
    for name in ["band", "bor", "bxor", "bnot"] {
        for (index, value) in values.iter().enumerate() {
            for args in [
                vec![value.clone()],
                vec![value.clone(), values[(index + 7) % values.len()].clone()],
                vec![
                    Value::Number(-1.0),
                    value.clone(),
                    Value::Number(2147483648.0),
                    Value::Number(7.0),
                ],
            ] {
                p.paired(name, &args);
                count += 1;
            }
        }
        for args in [
            vec![],
            vec![Value::Nil],
            vec![Value::Boolean(false)],
            vec![f.text("bad")],
            vec![f.text("2\0")],
            vec![Value::Number(1.0), Value::Boolean(false)],
            vec![Value::Number(1.0), Value::Number(2.0), f.text("bad")],
        ] {
            p.paired(name, &args);
            count += 1;
        }
    }
    let original = f.functions["originals"].call::<MultiValue>(()).unwrap();
    let native = p.call("originals", &[]).unwrap();
    assert_eq!(native.len(), 4);
    let bit: Table = f.lua.globals().raw_get("bit").unwrap();
    assert_eq!(original[1], bit.raw_get::<Value>("bor").unwrap());
    let rebound_fixture = Fixture::with_rebinding(true);
    let mut rebound = rebound_fixture.pair();
    rebound.paired("rebound", &[Value::Number(5.0), Value::Number(8.0)]);
    rebound.paired("bor", &[Value::Number(5.0), Value::Number(8.0)]);
    let captured = rebound_fixture.functions["originals"]
        .call::<MultiValue>(())
        .unwrap();
    assert_ne!(
        captured[1],
        rebound_fixture
            .lua
            .globals()
            .raw_get::<Table>("bit")
            .unwrap()
            .raw_get::<Value>("bor")
            .unwrap()
    );
    let after = p.call("originals", &[]).unwrap();
    for i in 0..4 {
        let same = p
            .call("same", &[native[i].clone(), after[i].clone()])
            .unwrap();
        p.compare(
            &[Value::Boolean(true)],
            &same,
            "retained original bit identity",
        );
    }
    save(
        "r2s-bit-cold.json",
        json!({"cases":count,"primitives":["band","bor","bxor","bnot"],"source_rebinding_compared":true,"retained_identities_compared":4}),
    );
}
#[test]
fn original_modulo_and_bits_keep_effects_before_failures_and_old_indexed_values() {
    let f = Fixture::new();
    let mut p = f.pair();
    for (name, args_tail) in [
        ("modulo_effect", vec![f.text("bad"), Value::Number(3.0)]),
        ("bit_effect", vec![f.text("bad"), Value::Number(3.0)]),
        (
            "ignored_bnot_effect",
            vec![Value::Number(1.0), Value::Boolean(false)],
        ),
        ("modulo_indexed", vec![Value::Number(4.0)]),
    ] {
        let state = f.lua.create_table().unwrap();
        state.raw_set("effects", 0).unwrap();
        state.raw_set("left", 13).unwrap();
        state.raw_set("replacement", 3).unwrap();
        let mut args = vec![Value::Table(state.clone())];
        args.extend(args_tail);
        let input = p.import(&args);
        let source = f.functions[name].call::<MultiValue>(MultiValue::from_vec(args));
        let native = p.call(name, &input);
        match (source, native) {
            (Ok(a), Ok(b)) => p.compare(&a.into_vec(), &b, name),
            (Err(_), Err(error)) => assert_eq!(error.kind, ProgramRuntimeErrorKind::Source),
            other => panic!("{name}:{other:?}"),
        }
        assert_eq!(state.raw_get::<i32>("effects").unwrap(), 1);
        p.compare(&[Value::Table(state)], &input[..1], name);
    }
}
#[test]
fn original_modulo_and_bit_wrappers_compare_exact_warmed_targets() {
    let f = Fixture::new();
    let mut p = f.pair();
    let driver = warm::SourceWarmDriver::new(&f.lua).unwrap();
    let mut cases = vec![];
    let pairs = [
        [13.0, 5.0],
        [-13.0, 5.0],
        [13.0, -5.0],
        [-13.0, -5.0],
        [0.0, 3.0],
        [-0.0, 3.0],
        [1.5, 2.0],
        [2.5, 2.0],
        [4294967295.0, 4294967296.0],
        [9007199254740991.0, 4294967296.0],
        [4503599627370496.0, 3.0],
        [f64::MAX, 3.0],
        [f64::from_bits(1), 2.0],
        [1.0, 0.0],
        [f64::INFINITY, 3.0],
        [1.0, f64::INFINITY],
        [f64::NAN, 3.0],
    ];
    for name in ["modulo", "band", "bor", "bxor", "bnot"] {
        for args in pairs {
            let args = args.map(Value::Number);
            let (success, out) = p.paired(name, &args);
            assert!(success);
            let result = driver
                .run(&f.lua, &f.functions[name], &args, None)
                .unwrap_or_else(|e| panic!("{name}/{args:?}: {e}"));
            assert!(result.success);
            assert_eq!(result.calls, 128);
            assert_eq!(result.seed_calls, 0);
            assert!(result.target_live_traces > 0);
            p.compare(&[result.value], &out, name);
            cases.push(json!({"function":name,"arguments":observation::canonical(&observation::capture(&args)),"calls":result.calls,"target_traces":result.target_live_traces,"seed_calls":result.seed_calls}));
        }
    }
    save(
        "r2s-bit-modulo-warm.json",
        json!({"cases":cases,"nan_policy":"class only; all other scalar bits exact"}),
    );
}
