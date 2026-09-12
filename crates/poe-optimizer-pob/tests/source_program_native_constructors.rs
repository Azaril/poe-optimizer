//! Complete original fixture constructors, through source-owned layout and sessions.
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{
    capture::*, lower_observed_closures_and_constructors_from_sources,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const PATH: &str = "tests/support/source_program_native_constructors.lua";
const TEXT: &str = include_str!("support/source_program_native_constructors.lua");
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
        let lua = unsafe { Lua::unsafe_new() };
        lua.load("jit.off();jit.flush();assert(not jit.status())")
            .exec()
            .unwrap();
        let observer = SourceClosureObserver::capture_before_source_with_closures(
            &lua,
            SourceTableRuntimeProfile::luajit21_x64_single(),
        )
        .unwrap();
        let api: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let functions: BTreeMap<String, Function> = api.pairs().map(Result::unwrap).collect();
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
            .observe_session(
                &lua,
                &sources,
                source,
                SourceSessionCaptureRequest {
                    callbacks: functions.clone(),
                    definitions: SourceCaptureContext {
                        capture_iteration: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        let lowered = lower_observed_closures_and_constructors_from_sources(
            &sources,
            observed.owner(),
            observed.closure_observations().unwrap(),
            observed.constructor_observations().unwrap(),
        )
        .unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{:?}",
            lowered.unsupported()
        );
        assert!(
            lowered.constructor_unsupported().is_empty(),
            "all fixture list/empty sites must be mapped: {:?}",
            lowered.constructor_unsupported()
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
                    max_values: 500_000,
                    max_bytes: 32 * 1024 * 1024,
                    ..ProgramLimits::default()
                },
            )
            .unwrap();
        Pair {
            fixture: self,
            session,
            roots,
        }
    }
    fn input(&self, n: usize, pattern: usize) -> Table {
        let table = self.lua.create_table().unwrap();
        table.raw_set("n", n).unwrap();
        for i in 1..=n {
            let value = match pattern {
                0 => Value::Integer(i as i64),
                1 if i == n => Value::Boolean(false),
                2 if i == 1 => Value::Integer(7),
                3 if i % 2 == 0 => Value::Integer(i as i64),
                _ => Value::Nil,
            };
            table.raw_set(i, value).unwrap();
        }
        table
    }
}
impl Pair<'_> {
    fn import(&mut self, values: &[Value]) -> Vec<SessionValue> {
        self.session
            .import_with_coverage(&observation::capture(values), &ProgramTableCoverage::new())
            .unwrap()
    }
    fn root(&self, name: &str) -> SessionValue {
        self.roots[self.fixture.observed.root_index(name).unwrap()].clone()
    }
    fn native(
        &mut self,
        name: &str,
        args: &[SessionValue],
    ) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
        self.session.invoke_callable(&self.root(name), args)
    }
    fn invoke(&mut self, name: &str, args: &[Value]) -> (Vec<Value>, Vec<SessionValue>) {
        let input = self.import(args);
        let source = self.fixture.functions[name]
            .call::<MultiValue>(MultiValue::from_vec(args.to_vec()))
            .unwrap()
            .into_vec();
        let native = self
            .native(name, &input)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        self.compare(&source, &native);
        (source, native)
    }
    fn compare(&mut self, source: &[Value], native: &[SessionValue]) {
        assert_eq!(source.len(), native.len(), "full result pack");
        assert_eq!(
            observation::canonical(&observation::capture(source)),
            observation::canonical(self.session.snapshot(native).unwrap().graph())
        );
    }
    fn walk(&mut self, source: &Value, native: &SessionValue, admitted: bool) {
        let out = self.fixture.lua.create_table().unwrap();
        let native_out = self.import(&[Value::Table(out.clone())]).remove(0);
        let source_count: Value = self.fixture.functions["walk"]
            .call((source.clone(), out.clone()))
            .unwrap();
        let result = self.native("walk", &[native.clone(), native_out.clone()]);
        if admitted {
            let values = result.unwrap();
            self.compare(&[source_count], &values);
            self.compare(&[Value::Table(out)], &[native_out]);
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                ProgramRuntimeErrorKind::UnsupportedCapability
            );
        }
    }
}
#[test]
fn original_list_constructors_preserve_all_packs_and_reserved_traversal() {
    let fixture = Fixture::new();
    let mut pair = fixture.pair();
    let mut evidence = vec![];
    for n in [0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256] {
        for pattern in 0..5 {
            for name in ["tail", "prefix", "prefix_two", "single_tail", "separator"] {
                let mut args = vec![];
                if name != "tail" {
                    args.push(Value::Integer(91));
                }
                if name == "prefix_two" {
                    args.push(Value::Nil);
                }
                args.push(Value::Table(fixture.input(n, pattern)));
                let (source, native) = pair.invoke(name, &args);
                let fits = if name == "single_tail" {
                    true
                } else if name == "tail" {
                    n <= 2
                } else {
                    n <= 1
                };
                pair.walk(&source[0], &native[0], fits);
                evidence.push(serde_json::json!({"constructor":name,"tail_results":n,"pattern":pattern,"raw_values_compared":true,"traversal_compared":fits,"growing_layout_frontier":!fits}));
            }
        }
    }
    assert_eq!(evidence.len(), 400);
    assert_eq!(
        evidence
            .iter()
            .filter(|row| row["traversal_compared"] == true)
            .count(),
        125
    );
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../runs/r2q-native-constructor-lists.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}
#[test]
fn original_multiple_nested_and_branch_sites_keep_private_array_histories() {
    let fixture = Fixture::new();
    let mut pair = fixture.pair();
    for values in [
        [Value::Nil, Value::Boolean(false)],
        [Value::Integer(1), Value::Nil],
        [Value::Nil, Value::Nil],
    ] {
        for name in ["pair", "nested"] {
            let (source, native) = pair.invoke(name, &values);
            pair.walk(&source[0], &native[0], true);
        }
        for flag in [false, true] {
            let (source, native) = pair.invoke(
                "branch",
                &[Value::Boolean(flag), values[0].clone(), values[1].clone()],
            );
            pair.walk(&source[0], &native[0], true);
        }
    }
    let (source, native) = pair.invoke("pair", &[Value::Nil, Value::Boolean(false)]);
    let values = pair.native("default_unpack", &[native[0].clone()]).unwrap();
    let actual = fixture.functions["default_unpack"]
        .call::<MultiValue>(source[0].clone())
        .unwrap()
        .into_vec();
    pair.compare(&actual, &values);
    for control in [0, 1, 2, 3] {
        let args = pair.import(&[Value::Integer(control)]);
        let actual =
            fixture.functions["next_value"].call::<MultiValue>((source[0].clone(), control));
        let result = pair.native("next_value", &[native[0].clone(), args[0].clone()]);
        match (actual, result) {
            (Ok(a), Ok(b)) => pair.compare(&a.into_vec(), &b),
            (Err(_), Err(e)) => assert_eq!(e.kind, ProgramRuntimeErrorKind::Source),
            (a, b) => panic!("next({control}): {a:?} {b:?}"),
        }
    }
}
#[test]
fn original_constructor_effects_and_error_prefixes_remain_visible() {
    let fixture = Fixture::new();
    let mut pair = fixture.pair();
    for name in ["effect", "effect_single", "failed_tail"] {
        let state = fixture.lua.create_table().unwrap();
        for (k, v) in [
            ("initial", 2),
            ("changed", 10),
            ("right", 7),
            ("first", 9),
            ("effects", 0),
        ] {
            state.raw_set(k, v).unwrap();
        }
        let native_state = pair.import(&[Value::Table(state.clone())]).remove(0);
        let actual = fixture.functions[name].call::<MultiValue>(state.clone());
        let native = pair.native(name, std::slice::from_ref(&native_state));
        match (actual, native) {
            (Ok(a), Ok(b)) => pair.compare(&a.into_vec(), &b),
            (Err(_), Err(e)) => assert_eq!(e.kind, ProgramRuntimeErrorKind::Source),
            (a, b) => panic!("{name}: {a:?} {b:?}"),
        }
        pair.compare(&[Value::Table(state)], &[native_state]);
    }
}

#[test]
fn original_reserved_list_layout_matches_exact_warmed_constructors() {
    let fixture = Fixture::new();
    let mut pair = fixture.pair();
    let driver = warm::SourceWarmDriver::new(&fixture.lua).unwrap();
    let mut cases = vec![];
    let values = [Value::Nil, Value::Boolean(false), Value::Integer(7)];
    for a in &values {
        for b in &values {
            cases.push(("pair", vec![a.clone(), b.clone()]));
        }
    }
    for name in ["tail", "prefix", "prefix_two", "single_tail", "separator"] {
        let maximum = if name == "tail" { 2 } else { 1 };
        for n in 0..=maximum {
            for pattern in 0..5 {
                let mut args = vec![];
                if name != "tail" {
                    args.push(Value::Integer(91));
                }
                if name == "prefix_two" {
                    args.push(Value::Nil);
                }
                args.push(Value::Table(fixture.input(n, pattern)));
                cases.push((name, args));
            }
        }
    }
    let mut evidence = vec![];
    for (name, args) in cases {
        fixture.lua.load("jit.off();jit.flush()").exec().unwrap();
        let (cold, native) = pair.invoke(name, &args);
        let warmed = driver
            .run(&fixture.lua, &fixture.functions[name], &args, None)
            .unwrap();
        assert!(warmed.success);
        assert_eq!(warmed.calls, 128);
        assert_eq!(warmed.seed_calls, 0);
        assert!(warmed.target_live_traces > 0);
        let source = warmed.value;
        pair.compare(std::slice::from_ref(&source), &native);
        pair.walk(&source, &native[0], true);
        let actual = fixture.functions["default_unpack"]
            .call::<MultiValue>(source.clone())
            .unwrap();
        let unpack = match pair.native("default_unpack", &native) {
            Ok(result) => {
                pair.compare(&actual.into_vec(), &result);
                serde_json::json!({"compared":true})
            }
            Err(error) => {
                assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
                assert_eq!(
                    error.message,
                    "source JIT length hints have multiple possible array boundaries"
                );
                serde_json::json!({"compared":false,"frontier":error.message})
            }
        };
        for control in 0..=5 {
            let input = pair.import(&[Value::Integer(control)]);
            let actual =
                fixture.functions["next_value"].call::<MultiValue>((source.clone(), control));
            let result = pair.native("next_value", &[native[0].clone(), input[0].clone()]);
            match (actual, result) {
                (Ok(a), Ok(b)) => pair.compare(&a.into_vec(), &b),
                (Err(_), Err(e)) => assert_eq!(e.kind, ProgramRuntimeErrorKind::Source),
                (a, b) => panic!("warm {name}/next({control}): {a:?} {b:?}"),
            }
        }
        evidence.push(serde_json::json!({"constructor":name,"cold":observation::canonical(&observation::capture(&cold)),"warm":observation::canonical(&observation::capture(&[source])),"calls":warmed.calls,"target_traces":warmed.target_live_traces,"unpack":unpack,"six_next_controls_compared":true}));
    }
    assert_eq!(evidence.len(), 64);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../runs/r2q-native-constructor-warm.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}
