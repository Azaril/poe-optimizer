//! Original callable factories evaluated through the injected native source catalog.
#[path = "support/source_program_observation.rs"]
mod observation;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::*, lower_observed_closures_from_sources};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const PATH: &str = "tests/support/source_program_native_factories.lua";
const TEXT: &str = include_str!("support/source_program_native_factories.lua");
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
        let lua = Lua::new();
        lua.load("jit.off();jit.flush();assert(not jit.status())")
            .exec()
            .unwrap();
        let observer = SourceClosureObserver::capture_before_source_with_closures(
            &lua,
            SourceTableRuntimeProfile::luajit21_x64_single(),
        )
        .unwrap();
        let exports: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let functions: BTreeMap<String, Function> = exports.pairs().map(Result::unwrap).collect();
        assert_eq!(functions.len(), 17, "all complete fixture roots retained");
        let sources = BTreeMap::from([(PATH.into(), TEXT.into())]);
        let source = ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: BTreeMap::from([(
                PATH.into(),
                format!("{:x}", Sha256::digest(TEXT.as_bytes())),
            )]),
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
                    ..Default::default()
                },
            )
            .unwrap();
        let lowered = lower_observed_closures_from_sources(
            &sources,
            observed.owner(),
            observed.closure_observations().unwrap(),
        )
        .unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "complete original factory bodies: {:?}",
            lowered.unsupported()
        );
        assert_eq!(
            lowered.catalog().data().programs.len(),
            42,
            "all original root and child bodies"
        );
        assert_eq!(
            lowered.catalog().closure_creations().unwrap().sites.len(),
            25,
            "all original creation sites"
        );
        assert_eq!(
            observed.input().closures.len(),
            17,
            "children were not pre-instantiated"
        );
        assert!(
            observed.input().cells.is_empty(),
            "all initial captures are created by execution"
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
            .session_from_input(self.observed.input(), ProgramLimits::default())
            .unwrap();
        Pair {
            fixture: self,
            session,
            roots,
        }
    }
    fn table(&self) -> Table {
        self.lua.create_table().unwrap()
    }
    fn string(&self, s: &str) -> Value {
        Value::String(self.lua.create_string(s).unwrap())
    }
}
impl Pair<'_> {
    fn import(&mut self, values: &[Value]) -> Vec<SessionValue> {
        self.session
            .import_with_coverage(&observation::capture(values), &ProgramTableCoverage::new())
            .unwrap()
    }
    fn invoke(
        &mut self,
        name: &str,
        source_args: &[Value],
        native_args: &[SessionValue],
    ) -> (Vec<Value>, Vec<SessionValue>) {
        let function = self.fixture.functions[name].clone();
        let root = self.roots[self.fixture.observed.root_index(name).unwrap()].clone();
        self.call(&function, &root, source_args, native_args)
    }
    fn call(
        &mut self,
        source: &Function,
        native: &SessionValue,
        source_args: &[Value],
        native_args: &[SessionValue],
    ) -> (Vec<Value>, Vec<SessionValue>) {
        let source = source
            .call::<MultiValue>(MultiValue::from_vec(source_args.to_vec()))
            .unwrap()
            .into_vec();
        let native = self.session.invoke_callable(native, native_args).unwrap();
        assert_eq!(source.len(), native.len(), "complete result pack");
        (source, native)
    }
    fn compare(&mut self, source: &[Value], native: &[SessionValue]) {
        assert_eq!(
            observation::canonical(&observation::capture(source)),
            observation::canonical(self.session.snapshot(native).unwrap().graph())
        );
    }
    fn get(&mut self, source: &Table, native: &SessionValue, key: Value) -> (Value, SessionValue) {
        let native_key = self.import(std::slice::from_ref(&key));
        let (source, native) = self.invoke(
            "read",
            &[Value::Table(source.clone()), key],
            &[native.clone(), native_key[0].clone()],
        );
        assert_eq!(source.len(), 1);
        assert_eq!(native.len(), 1);
        (source[0].clone(), native[0].clone())
    }
}
#[test]
fn complete_original_factories_share_cells_and_preserve_fresh_function_identity() {
    let fixture = Fixture::new();
    let mut pair = fixture.pair();
    let args = [Value::Integer(1)];
    let native_args = pair.import(&args);
    let (a, na) = pair.invoke("pair", &args, &native_args);
    let (b, nb) = pair.invoke("pair", &args, &native_args);
    let (value, nvalue) = pair.invoke(
        "equal",
        &[a[0].clone(), b[0].clone()],
        &[na[0].clone(), nb[0].clone()],
    );
    assert_eq!(value, [Value::Boolean(false)]);
    pair.compare(&value, &nvalue);
    let args = [Value::Integer(7)];
    let native_args = pair.import(&args);
    let (output, noutput) = pair.call(a[1].as_function().unwrap(), &na[1], &args, &native_args);
    pair.compare(&output, &noutput);
    for (get, nget, expected) in [(&a[0], &na[0], 7), (&b[0], &nb[0], 1)] {
        let (output, noutput) = pair.call(get.as_function().unwrap(), nget, &[], &[]);
        assert_eq!(
            fixture.lua.coerce_integer(output[0].clone()).unwrap(),
            Some(expected)
        );
        pair.compare(&output, &noutput);
    }
    let args = [Value::Integer(2)];
    let native_args = pair.import(&args);
    let (forward, nforward) = pair.invoke("forward", &args, &native_args);
    let (nested, nnested) = pair.call(forward[0].as_function().unwrap(), &nforward[0], &[], &[]);
    let args = [Value::Integer(12)];
    let native_args = pair.import(&args);
    pair.call(
        nested[0].as_function().unwrap(),
        &nnested[0],
        &args,
        &native_args,
    );
    for (get, nget) in [(&forward[1], &nforward[1]), (&nested[1], &nnested[1])] {
        let (output, noutput) = pair.call(get.as_function().unwrap(), nget, &[], &[]);
        assert_eq!(
            fixture.lua.coerce_integer(output[0].clone()).unwrap(),
            Some(12)
        );
        pair.compare(&output, &noutput);
    }
    let (recursive, nrecursive) = pair.invoke("recursive", &[], &[]);
    let args = [Value::Integer(5)];
    let native_args = pair.import(&args);
    let (output, noutput) = pair.call(
        recursive[0].as_function().unwrap(),
        &nrecursive[0],
        &args,
        &native_args,
    );
    assert_eq!(
        fixture.lua.coerce_integer(output[0].clone()).unwrap(),
        Some(120)
    );
    pair.compare(&output, &noutput);
    let args = [Value::Integer(0)];
    let native_args = pair.import(&args);
    let (identity, nidentity) = pair.call(
        recursive[0].as_function().unwrap(),
        &nrecursive[0],
        &args,
        &native_args,
    );
    let (output, noutput) = pair.invoke(
        "equal",
        &[recursive[0].clone(), identity[0].clone()],
        &[nrecursive[0].clone(), nidentity[0].clone()],
    );
    assert_eq!(output, [Value::Boolean(true)]);
    pair.compare(&output, &noutput);
}
#[test]
fn complete_original_loop_factories_keep_visible_and_outer_binding_lifetimes() {
    let fixture = Fixture::new();
    for name in ["numeric_loop", "generic_loop", "shared_outer", "break_loop"] {
        let mut pair = fixture.pair();
        let out = fixture.table();
        let input = fixture.table();
        for i in 1..=3 {
            input.raw_set(i, i + 3).unwrap();
        }
        let args = [Value::Table(out.clone()), Value::Table(input)];
        let native_args = pair.import(&args);
        let (output, noutput) = pair.invoke(name, &args, &native_args);
        pair.compare(&output, &noutput);
        let keys: Vec<Value> = if name == "break_loop" {
            vec![fixture.string("get")]
        } else {
            (1..=3).map(Value::Integer).collect()
        };
        let mut functions = Vec::new();
        for key in keys {
            let (get, nget) = pair.get(&out, &native_args[0], key);
            let (output, noutput) = pair.call(get.as_function().unwrap(), &nget, &[], &[]);
            pair.compare(&output, &noutput);
            functions.push((get, nget));
        }
        if name == "numeric_loop" || name == "generic_loop" {
            let args = [Value::Integer(40)];
            let native_values = pair.import(&args);
            let (output, noutput) = pair.call(
                functions[1].0.as_function().unwrap(),
                &functions[1].1,
                &args,
                &native_values,
            );
            pair.compare(&output, &noutput);
            for (get, nget) in &functions {
                let (output, noutput) = pair.call(get.as_function().unwrap(), nget, &[], &[]);
                pair.compare(&output, &noutput);
            }
        }
        if name == "shared_outer" {
            let args = [Value::Integer(5)];
            let native_values = pair.import(&args);
            pair.call(
                functions[0].0.as_function().unwrap(),
                &functions[0].1,
                &args,
                &native_values,
            );
            let (output, noutput) = pair.call(
                functions[2].0.as_function().unwrap(),
                &functions[2].1,
                &[],
                &[],
            );
            assert_eq!(
                fixture.lua.coerce_integer(output[0].clone()).unwrap(),
                Some(15)
            );
            pair.compare(&output, &noutput);
        }
    }
    let mut pair = fixture.pair();
    let (get, nget) = pair.invoke("return_loop", &[], &[]);
    let (output, noutput) = pair.call(get[0].as_function().unwrap(), &nget[0], &[], &[]);
    pair.compare(&output, &noutput);
    for flag in [true, false] {
        let args = [Value::Boolean(flag)];
        let native_args = pair.import(&args);
        let (get, nget) = pair.invoke("branch", &args, &native_args);
        let (output, noutput) = pair.call(get[0].as_function().unwrap(), &nget[0], &[], &[]);
        pair.compare(&output, &noutput);
    }
}
#[test]
fn complete_original_factory_errors_keep_prior_cells_and_operand_effects() {
    let fixture = Fixture::new();
    let mut pair = fixture.pair();
    let state = fixture.table();
    let args = [Value::Table(state.clone())];
    let native_args = pair.import(&args);
    let root = pair.roots[fixture.observed.root_index("fail_escape").unwrap()].clone();
    assert!(
        fixture.functions["fail_escape"]
            .call::<MultiValue>(state.clone())
            .is_err()
    );
    assert_eq!(
        pair.session
            .invoke_callable(&root, &native_args)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let (get, nget) = pair.get(&state, &native_args[0], fixture.string("get"));
    let (set, nset) = pair.get(&state, &native_args[0], fixture.string("set"));
    let (output, noutput) = pair.call(get.as_function().unwrap(), &nget, &[], &[]);
    assert_eq!(
        fixture.lua.coerce_integer(output[0].clone()).unwrap(),
        Some(9)
    );
    pair.compare(&output, &noutput);
    let args = [Value::Integer(17)];
    let native_values = pair.import(&args);
    pair.call(set.as_function().unwrap(), &nset, &args, &native_values);
    let (output, noutput) = pair.call(get.as_function().unwrap(), &nget, &[], &[]);
    assert_eq!(
        fixture.lua.coerce_integer(output[0].clone()).unwrap(),
        Some(17)
    );
    pair.compare(&output, &noutput);
    for (name, initial, changed, success, effects) in [
        ("binary", fixture.string("bad"), Value::Integer(10), true, 1),
        ("binary", Value::Integer(2), fixture.string("bad"), false, 1),
        (
            "binary_computed",
            fixture.string("bad"),
            Value::Integer(10),
            false,
            0,
        ),
        (
            "binary_computed",
            Value::Integer(2),
            fixture.string("bad"),
            true,
            1,
        ),
    ] {
        let state = fixture.table();
        state.raw_set("initial", initial).unwrap();
        state.raw_set("changed", changed).unwrap();
        state.raw_set("right", 2).unwrap();
        state.raw_set("effects", 0).unwrap();
        let args = [Value::Table(state.clone())];
        let native_args = pair.import(&args);
        let root = pair.roots[fixture.observed.root_index(name).unwrap()].clone();
        let source = fixture.functions[name].call::<MultiValue>(state.clone());
        let native = pair.session.invoke_callable(&root, &native_args);
        match (source, native, success) {
            (Ok(a), Ok(b), true) => pair.compare(&a.into_vec(), &b),
            (Err(_), Err(e), false) => assert_eq!(e.kind, ProgramRuntimeErrorKind::Source),
            (a, b, _) => panic!("{name}:source={a:?},native={b:?}"),
        }
        assert_eq!(state.raw_get::<i32>("effects").unwrap(), effects);
        let (value, nvalue) = pair.get(&state, &native_args[0], fixture.string("effects"));
        pair.compare(&[value], &[nvalue]);
        let (get, nget) = pair.get(&state, &native_args[0], fixture.string("get"));
        let (output, noutput) = pair.call(get.as_function().unwrap(), &nget, &[], &[]);
        pair.compare(&output, &noutput);
    }
    let (output, noutput) = pair.invoke("capture_binary", &[], &[]);
    pair.compare(&output, &noutput);
    for name in ["addresses", "address_hazard"] {
        let args = [Value::Table(fixture.table()), Value::Table(fixture.table())];
        let native_args = pair.import(&args);
        let (output, noutput) = pair.invoke(name, &args, &native_args);
        pair.compare(&output, &noutput);
        pair.compare(&args, &native_args);
    }
}
