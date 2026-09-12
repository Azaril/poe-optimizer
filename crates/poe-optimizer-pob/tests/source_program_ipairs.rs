//! Original ipairs factory/auxiliary semantics through source-owned native sessions.
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::*, lower_from_sources};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};
const PATH: &str = "tests/support/source_program_ipairs.lua";
const TEXT: &str = include_str!("support/source_program_ipairs.lua");
struct Fixture {
    lua: Lua,
    functions: BTreeMap<String, Function>,
    original_factory: Function,
    original_aux: Function,
    observed: ObservedSourceSession,
    compiled: CompiledSourcePrograms,
}
struct Pair<'a> {
    fixture: &'a Fixture,
    session: ProgramSession,
    roots: Vec<SessionValue>,
}
impl Fixture {
    fn new(rebind: bool) -> Self {
        let lua = unsafe { Lua::unsafe_new() };
        lua.load("jit.off(); jit.flush(); assert(not jit.status())")
            .exec()
            .unwrap();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let api: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let functions: BTreeMap<String, Function> = api.pairs().map(Result::unwrap).collect();
        let originals: MultiValue = functions["return_originals"].call(()).unwrap();
        assert_eq!(originals.len(), 2);
        let original_factory = originals[0].as_function().unwrap().clone();
        let original_aux = originals[1].as_function().unwrap().clone();
        assert_eq!(original_factory.info().what, "C");
        assert_eq!(original_aux.info().what, "C");
        if rebind {
            lua.globals()
                .raw_set("ipairs", functions["replacement"].clone())
                .unwrap();
        }
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
                        .map(|(n, f)| (n.clone(), f.clone()))
                        .collect(),
                    state_roots: [
                        (
                            "original.factory".into(),
                            Value::Function(original_factory.clone()),
                        ),
                        ("original.aux".into(), Value::Function(original_aux.clone())),
                    ]
                    .into(),
                    definitions: SourceCaptureContext {
                        capture_iteration: true,
                        environment: Some(SourceEnvironmentSelection {
                            table: lua.globals(),
                            root_name: "environment".into(),
                        }),
                        projections: vec![SourceTableSelection {
                            table: lua.globals(),
                            fields: ["ipairs".into(), "select".into()].into(),
                            indexed: Default::default(),
                            allow_index_fallback: false,
                            allow_call_fallback: false,
                        }],
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
            original_factory,
            original_aux,
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
    fn bytes(&self, text: &str) -> Value {
        Value::String(self.lua.create_string(text).unwrap())
    }
    fn table(&self) -> Table {
        let t = self.lua.create_table().unwrap();
        for (key, value) in [
            (-2147483648, self.bytes("wrapped")),
            (-1, self.bytes("negative")),
            (0, self.bytes("zero")),
            (1, Value::Boolean(false)),
            (2, self.bytes("second")),
            (4, self.bytes("after hole")),
        ] {
            t.raw_set(key, value).unwrap();
        }
        t
    }
    fn output(&self) -> Table {
        let t = self.lua.create_table().unwrap();
        t.raw_set("calls", 0).unwrap();
        for key in ["counts", "firsts", "seconds", "controls"] {
            t.raw_set(key, self.lua.create_table().unwrap()).unwrap();
        }
        t
    }
}
impl Pair<'_> {
    fn root(&self, name: &str) -> SessionValue {
        self.roots[self.fixture.observed.root_index(name).unwrap()].clone()
    }
    fn import(&mut self, values: &[Value]) -> Vec<SessionValue> {
        self.session
            .import_with_coverage(&observation::capture(values), &ProgramTableCoverage::new())
            .unwrap()
    }
    fn call(
        &mut self,
        name: &str,
        args: &[SessionValue],
    ) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
        self.session.invoke_callable(&self.root(name), args)
    }
    fn compare(&mut self, source: &[Value], native: &[SessionValue]) {
        assert_eq!(source.len(), native.len(), "full result pack");
        assert_eq!(
            observation::canonical(&observation::capture(source)),
            observation::canonical(self.session.snapshot(native).unwrap().graph())
        );
    }
    fn same(&mut self, a: &SessionValue, b: &SessionValue) -> bool {
        let out = self.call("same", &[a.clone(), b.clone()]).unwrap();
        let graph = self.session.snapshot(&out).unwrap();
        let ProgramValue::Boolean(value) = graph.graph().values[0] else {
            panic!("boolean")
        };
        value
    }
    fn plain_call(
        &mut self,
        name: &str,
        args: &[Value],
        input: &[SessionValue],
    ) -> Vec<SessionValue> {
        let actual = self.fixture.functions[name]
            .call::<MultiValue>(MultiValue::from_vec(args.to_vec()))
            .unwrap();
        let native = self
            .call(name, input)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        self.compare(&actual.into_vec(), &native);
        native
    }
    fn factory(&mut self, table: &Table, native_table: &SessionValue) -> Vec<SessionValue> {
        let actual = self.fixture.functions["factory"]
            .call::<MultiValue>(table.clone())
            .unwrap();
        assert_eq!(actual.len(), 3);
        assert_eq!(
            actual[0],
            Value::Function(self.fixture.original_aux.clone())
        );
        assert_eq!(actual[1], Value::Table(table.clone()));
        assert_eq!(actual[2], Value::Integer(0));
        let native = self
            .call("factory", std::slice::from_ref(native_table))
            .unwrap();
        assert_eq!(native.len(), 3);
        assert!(self.same(&native[0], &self.root("original.aux")));
        assert!(self.same(&native[1], native_table));
        self.compare(&[actual[2].clone()], &native[2..]);
        native
    }
}
fn save(name: &str, value: Json) {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../runs")
        .join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
#[test]
fn original_ipairs_factory_keeps_retained_identity_rebinding_and_private_state() {
    for rebind in [false, true] {
        let f = Fixture::new(rebind);
        let mut p = f.pair();
        let mut other = f.pair();
        let table = f.table();
        let input = p.import(&[Value::Table(table.clone())]).remove(0);
        let first = p.factory(&table, &input);
        let second = p.factory(&table, &input);
        assert!(p.same(&first[0], &second[0]));
        let kind = p.call("kind", &[first[0].clone()]).unwrap();
        p.compare(&[f.bytes("function")], &kind);
        let original = f
            .original_factory
            .call::<MultiValue>(table.clone())
            .unwrap();
        assert_eq!(original[0], Value::Function(f.original_aux.clone()));
        let originals = p.call("return_originals", &[]).unwrap();
        assert_eq!(originals.len(), 2);
        assert!(p.same(&originals[0], &p.root("original.factory")));
        assert!(p.same(&originals[1], &first[0]));
        if rebind {
            let out = p.call("rebound", std::slice::from_ref(&input)).unwrap();
            let actual = f.functions["rebound"]
                .call::<MultiValue>(table.clone())
                .unwrap();
            p.compare(&actual.into_vec(), &out);
        } else {
            let actual = p.call("rebound", std::slice::from_ref(&input)).unwrap();
            assert!(p.same(&actual[0], &first[0]));
        }
        let independent_table = f.table();
        let independent = other
            .import(&[Value::Table(independent_table.clone())])
            .remove(0);
        other.factory(&independent_table, &independent);
        let key = p.import(&[Value::Integer(1)]).remove(0);
        p.call("write", &[input.clone(), key, first[0].clone()])
            .unwrap();
        table.raw_set(1, f.original_aux.clone()).unwrap();
        let control = p.import(&[Value::Integer(0)]).remove(0);
        let result = p.call("direct", &[input.clone(), control]).unwrap();
        assert_eq!(result.len(), 2);
        assert!(p.same(&result[1], &first[0]));
        let control = other.import(&[Value::Integer(0)]).remove(0);
        other.plain_call(
            "direct",
            &[Value::Table(independent_table), Value::Integer(0)],
            &[independent, control],
        );
    }
}
#[test]
fn original_ipairs_auxiliary_preserves_numeric_controls_zero_packs_and_mutation() {
    let f = Fixture::new(false);
    let mut p = f.pair();
    let table = f.table();
    let input = p.import(&[Value::Table(table.clone())]).remove(0);
    let controls = vec![
        Value::Integer(-2147483648),
        Value::Integer(-2),
        Value::Integer(-1),
        Value::Number(-0.0),
        Value::Integer(0),
        Value::Integer(1),
        Value::Integer(2),
        Value::Integer(3),
        Value::Integer(4),
        Value::Integer(2147483647),
        Value::Number(-1.9),
        Value::Number(1.9),
        f.bytes("0"),
        f.bytes(" 1.9 "),
        f.bytes("0x1"),
        f.bytes("-2147483648"),
        f.bytes("2147483647"),
    ];
    let mut evidence = vec![];
    for control in controls {
        let arg = p.import(std::slice::from_ref(&control)).remove(0);
        let actual = f.functions["direct"]
            .call::<MultiValue>((table.clone(), control.clone()))
            .unwrap();
        let native = p
            .call("direct", &[input.clone(), arg])
            .unwrap_or_else(|e| panic!("control {control:?}: {e}"));
        p.compare(&actual.clone().into_vec(), &native);
        evidence.push(json!({"control":observation::canonical(&observation::capture(&[control])),"result":observation::canonical(&observation::capture(&actual.into_vec()))}));
    }
    for value in [Value::Nil, Value::Boolean(false), f.bytes("refilled")] {
        table.raw_set(1, value.clone()).unwrap();
        let args = p.import(&[Value::Integer(1), value]);
        p.call("write", &[input.clone(), args[0].clone(), args[1].clone()])
            .unwrap();
        let zero = p.import(&[Value::Integer(0)]).remove(0);
        p.plain_call(
            "direct",
            &[Value::Table(table.clone()), Value::Integer(0)],
            &[input.clone(), zero],
        );
    }
    save(
        "r2r-ipairs-controls.json",
        json!({"mode":"original interpreter/native full packs","cases":evidence,"mutation_steps":3}),
    );
}
#[test]
fn original_ipairs_generic_loops_and_stored_protocol_read_live_tables() {
    let f = Fixture::new(false);
    let mut p = f.pair();
    for name in ["walk", "walk_mutating"] {
        let t = f.table();
        t.raw_set(3, "third").unwrap();
        let out = f.lua.create_table().unwrap();
        let native = p.import(&[Value::Table(t.clone()), Value::Table(out.clone())]);
        p.plain_call(
            name,
            &[Value::Table(t.clone()), Value::Table(out.clone())],
            &native,
        );
        p.compare(&[Value::Table(t)], &native[..1]);
    }
    let t = f.table();
    let state = f.lua.create_table().unwrap();
    let args = p.import(&[Value::Table(t.clone()), Value::Table(state.clone())]);
    f.functions["stored"]
        .call::<Value>((t.clone(), state.clone()))
        .unwrap();
    let result = p.call("stored", &args).unwrap();
    assert!(p.same(&result[0], &args[1]));
    for (field, expected) in [
        ("step", p.root("original.aux")),
        ("subject", args[0].clone()),
    ] {
        let key = p.import(&[f.bytes(field)]).remove(0);
        let value = p.call("read", &[args[1].clone(), key]).unwrap();
        assert!(p.same(&value[0], &expected));
    }
    for _ in 0..3 {
        p.plain_call("resume", &[Value::Table(state.clone())], &[args[1].clone()]);
    }
    // Nil terminates the generic protocol; calling the auxiliary again with
    // that nil control must report a source type error rather than rewind.
    assert!(f.functions["resume"].call::<MultiValue>(state).is_err());
    assert_eq!(
        p.call("resume", &[args[1].clone()]).unwrap_err().kind,
        ProgramRuntimeErrorKind::Source
    );
}
#[test]
fn original_ipairs_errors_keep_argument_effects_and_raw_lookup_ignores_fallback() {
    let f = Fixture::new(false);
    let mut p = f.pair();
    let t = f.table();
    let cases = vec![
        ("factory", vec![]),
        ("factory", vec![Value::Nil]),
        ("factory", vec![Value::Boolean(false)]),
        ("direct", vec![]),
        ("direct", vec![Value::Table(t.clone())]),
        ("direct", vec![Value::Table(t.clone()), Value::Nil]),
        (
            "direct",
            vec![Value::Table(t.clone()), Value::Boolean(false)],
        ),
        ("direct", vec![Value::Table(t.clone()), f.bytes("bad")]),
    ];
    for (name, args) in cases {
        let input = p.import(&args);
        assert!(
            f.functions[name]
                .call::<MultiValue>(MultiValue::from_vec(args))
                .is_err()
        );
        assert_eq!(
            p.call(name, &input).unwrap_err().kind,
            ProgramRuntimeErrorKind::Source
        );
    }
    for name in ["factory_effect", "aux_effect"] {
        let state = f.lua.create_table().unwrap();
        state.raw_set("effects", 0).unwrap();
        state.raw_set("value", 7).unwrap();
        let args = if name == "factory_effect" {
            vec![Value::Table(state.clone()), Value::Nil]
        } else {
            vec![
                Value::Table(state.clone()),
                Value::Table(t.clone()),
                f.bytes("bad"),
            ]
        };
        let native = p.import(&args);
        assert!(
            f.functions[name]
                .call::<MultiValue>(MultiValue::from_vec(args))
                .is_err()
        );
        assert_eq!(
            p.call(name, &native).unwrap_err().kind,
            ProgramRuntimeErrorKind::Source
        );
        assert_eq!(state.raw_get::<i32>("effects").unwrap(), 1);
        p.compare(&[Value::Table(state)], &native[..1]);
    }
    let raw = f.lua.create_table().unwrap();
    raw.raw_set(1, false).unwrap();
    let meta = f.lua.create_table().unwrap();
    let effects = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = effects.clone();
    meta.raw_set(
        "__index",
        f.lua
            .create_function(move |_, _: MultiValue| {
                counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(99)
            })
            .unwrap(),
    )
    .unwrap();
    raw.set_metatable(Some(meta)).unwrap();
    let graph = observation::capture(&[Value::Table(raw.clone())]);
    let coverage = ProgramTableCoverage::from([(
        ProgramTableId(1),
        SourceTableCoverage {
            inventory: SourceTableInventory::Complete,
            known_absent: Default::default(),
            unavailable: Default::default(),
            index_fallback: SourceTableIndexFallback::Unavailable,
            call_fallback: SourceTableCallFallback::NonCallable,
        },
    )]);
    let input = p
        .session
        .import_with_coverage(&graph, &coverage)
        .unwrap()
        .remove(0);
    for control in [0, 1] {
        let arg = p.import(&[Value::Integer(control)]).remove(0);
        p.plain_call(
            "direct",
            &[Value::Table(raw.clone()), Value::Integer(control)],
            &[input.clone(), arg],
        );
    }
    assert_eq!(effects.load(std::sync::atomic::Ordering::Relaxed), 0);
    let key = p.import(&[Value::Integer(2)]).remove(0);
    assert_eq!(
        p.call("read", &[input, key]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn original_ipairs_warmed_wrappers_retain_full_factory_and_auxiliary_protocols() {
    let f = Fixture::new(false);
    let mut p = f.pair();
    let warm = warm::SourceWarmDriver::new(&f.lua).unwrap();
    let mut cases = vec![];
    for (name, controls) in [
        ("record_factory", vec![None]),
        (
            "record_direct",
            vec![Some(-2), Some(-1), Some(0), Some(1), Some(2), Some(3)],
        ),
    ] {
        for control in controls {
            f.lua.load("jit.off();jit.flush()").exec().unwrap();
            // LuaJIT cannot trace nonconstant negative integer lookup into a table
            // with a nonempty array part (NYITMIX). The mixed negative case is covered
            // by the interpreter oracle above; this exact negative-control trace
            // uses a pure-hash subject, without a substitute seed or target.
            let (t, shape) = if control == Some(-2) {
                let t = f.lua.create_table().unwrap();
                t.raw_set(-2147483648, "wrapped").unwrap();
                t.raw_set(-1, "negative").unwrap();
                (t, "pure_hash")
            } else {
                (f.table(), "mixed_sparse_dense")
            };
            let out = f.output();
            let mut args = vec![Value::Table(t)];
            if let Some(c) = control {
                args.push(Value::Integer(c));
            }
            args.push(Value::Table(out.clone()));
            let input = p.import(&args);
            let result = warm
                .run(&f.lua, &f.functions[name], &args, None)
                .unwrap_or_else(|e| panic!("{name}/{control:?}: {e}"));
            assert!(result.success);
            assert_eq!(result.calls, 128);
            assert_eq!(result.seed_calls, 0);
            assert!(result.target_live_traces > 0);
            assert!(result.live_traces > 0);
            assert_eq!(result.value, Value::Table(out.clone()));
            for _ in 0..128 {
                p.call(name, &input).unwrap();
            }
            p.compare(&[Value::Table(out)], &input[input.len() - 1..]);
            cases.push(json!({"wrapper":name,"control":control,"table_shape":shape,"calls":result.calls,"seed_calls":result.seed_calls,"builtin_calls_per_wrapper":2,"target_traces":result.target_live_traces,"scope":"exact original Lua wrapper traces and complete captured result rows; builtin identities checked separately"}));
        }
    }
    save(
        "r2r-ipairs-warm.json",
        json!({"cases":cases,"native_sessions_reused":true,"source_trace_limit":"mixed sparse/dense table with negative hash lookup is interpreter-only (LuaJIT NYITMIX); pure-hash negative case traces the exact target without a seed"}),
    );
}
