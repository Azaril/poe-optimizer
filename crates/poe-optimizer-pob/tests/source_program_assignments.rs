//! Pinned source-only timing probes and separately captured, complete native-callable functions.
//! Nested factories/metamethods in the timing probes are evidence, not native admission.
#[path = "support/source_program_observation.rs"]
mod observation;
use mlua::{Function, Lua, Table, Value};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};
const PATH: &str = "tests/support/source_program_assignments.lua";
const TEXT: &str = include_str!("support/source_program_assignments.lua");
#[test]
fn pinned_source_assignment_timing_and_store_order() {
    let lua = Lua::new();
    lua.load("jit.off();jit.flush();assert(not jit.status())")
        .exec()
        .unwrap();
    let probes: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let mut rows = BTreeMap::<String, Json>::new();
    for entry in probes.pairs::<String, Value>() {
        let (name, function) = entry.unwrap();
        if name.starts_with("native") {
            continue;
        }
        let Value::Function(function) = function else {
            panic!("source probe function")
        };
        let result: Table = function.call(()).unwrap();
        assert_source_semantics(&name, &result);
        rows.insert(
            name,
            observation::canonical(&observation::capture(&[mlua::Value::Table(result)])),
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let evidence = json!({"mode":"pinned LuaJIT interpreter; source-only; no native admission", "source":PATH,
        "source_sha256":format!("{:x}",Sha256::digest(TEXT.as_bytes())),"cases":rows});
    fs::create_dir_all(root.join("runs")).unwrap();
    fs::write(
        root.join("runs/r2o-assignment-source-semantics.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&evidence).unwrap());
    assert_eq!(rows.len(), 26);
}

fn plain(value: Value) -> Json {
    match value {
        Value::Nil => Json::Null,
        Value::Boolean(value) => json!(value),
        Value::Integer(value) => json!(value),
        Value::Number(value) => {
            if value.fract() == 0.0 {
                json!(value as i64)
            } else {
                json!(value)
            }
        }
        Value::String(value) => json!(value.to_str().unwrap().as_ref()),
        Value::Table(value) => Json::Object(
            value
                .pairs::<Value, Value>()
                .map(|entry| {
                    let (key, value) = entry.unwrap();
                    let key = match key {
                        Value::String(value) => value.to_str().unwrap().to_owned(),
                        Value::Integer(value) => value.to_string(),
                        Value::Number(value) => value.to_string(),
                        _ => panic!("plain key"),
                    };
                    (key, plain(value))
                })
                .collect(),
        ),
        _ => panic!("plain source result"),
    }
}
fn assert_source_semantics(name: &str, result: &Table) {
    let mut actual = plain(Value::Table(result.clone()));
    let snapshot = |a: Json, b: Json, k: Json, x: Json, events: Json| json!({"a":a,"b":b,"t_is_a":false,"t_is_b":true,"k":k,"x":x,"events":events});
    let expected = match name {
        "local_registers_changed_by_rhs" => snapshot(
            json!({}),
            json!({"new":11}),
            json!("new"),
            json!(22),
            json!({"1":"rhs"}),
        ),
        "explicit_address_temporaries" => snapshot(
            json!({"old":11}),
            json!({}),
            json!("new"),
            json!(22),
            json!({"1":"table","2":"key","3":"rhs"}),
        ),
        "local_table_temporary_key" | "dynamic_logical_key" => snapshot(
            json!({}),
            json!({"old":11}),
            json!("new"),
            json!(22),
            json!({}),
        ),
        "temporary_table_local_key" => snapshot(
            json!({"new":11}),
            json!({}),
            json!("new"),
            json!(22),
            json!({}),
        ),
        "upvalues_changed_by_rhs" => snapshot(
            json!({"old":11}),
            json!({}),
            json!("new"),
            json!(22),
            json!({}),
        ),
        "parenthesized_locals"
        | "constant_true_logical_key"
        | "constant_false_logical_key"
        | "constant_arithmetic_logical_key"
        | "constant_true_logical_table" => snapshot(
            json!({}),
            json!({"new":11}),
            json!("new"),
            json!(22),
            json!({}),
        ),
        "later_table_local_conflict" => {
            json!({"a":{"new":11},"b":{},"c":{},"t_is_c":true,"k":"new"})
        }
        "later_key_local_conflict" => json!({"a":{},"b":{"old":11},"t_is_b":true,"k":"assigned"}),
        "both_later_local_conflicts" => {
            json!({"a":{"old":11},"b":{},"c":{},"t_is_c":true,"k":"assigned"})
        }
        "local_before_index" | "index_before_local" => json!({"i":2,"t":{"1":10}}),
        "repeated_local_and_table_aliases" => json!({"i":1,"t":{"1":3},"alias_is_t":true}),
        "lhs_address_effects_precede_rhs" => snapshot(
            json!({}),
            json!({"address":11}),
            json!("new"),
            json!(22),
            json!({"1":"key","2":"rhs"}),
        ),
        "later_lhs_effects_precede_hazard_copy" => snapshot(
            json!({"second":22}),
            json!({"new":11}),
            json!(33),
            json!("initial"),
            json!({"1":"later-key","2":"rhs"}),
        ),
        "mixed_upvalue_local_index" => json!({"captured":1,"x":2,"t":{"1":3}}),
        "store_metamethod_rebinds_local_address" => {
            json!({"a":{},"b":{"new":11},"t_is_b":true,"k":"new","events":{"1":"right-store"}})
        }
        "result_packs_and_extra_effects" => {
            json!({"first":{"a":1,"c":3},"a":1,"b":9,"c":3,"events":{"1":"values","2":"values","3":"extra","4":"extra"}})
        }
        "rhs_error_keeps_address_effects_and_no_stores" => {
            json!({"ok":false,"t":{},"x":"initial","events":{"1":"key","2":"rhs"}})
        }
        "right_store_error_prevents_left_store" => {
            json!({"ok":false,"t":{},"events":{"1":"left","2":"right","3":"rhs"}})
        }
        "left_store_error_preserves_right_store" => {
            json!({"ok":false,"t":{"slot":22},"events":{"1":"left","2":"right","3":"rhs"}})
        }
        "address_error_prevents_later_addresses_and_rhs" => json!({"ok":false,"t":{},"events":{}}),
        _ => panic!("unasserted source case {name}"),
    };
    if let Some(error) = actual.as_object_mut().unwrap().remove("error") {
        let marker = if name.starts_with("rhs_error") {
            "rhs sentinel"
        } else {
            "attempt to index"
        };
        assert!(error.as_str().unwrap().contains(marker), "{name}: {error}");
    }
    assert_eq!(actual, expected, "pinned source semantics {name}");
}

#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::MultiValue;
use poe_optimizer_data::item_loading::ItemLoadingSource;
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::*, lower_from_sources};
struct NativeFixture {
    lua: Lua,
    functions: BTreeMap<String, Function>,
    observed: ObservedSourceSession,
    compiled: CompiledSourcePrograms,
}
impl NativeFixture {
    fn new() -> Self {
        let lua = Lua::new();
        lua.load("jit.off();jit.flush();assert(not jit.status())")
            .exec()
            .unwrap();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let exports: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let mut functions: BTreeMap<String, Function> = exports
            .raw_get::<Table>("native")
            .unwrap()
            .pairs()
            .map(Result::unwrap)
            .collect();
        for entry in exports
            .raw_get::<Table>("native_live")
            .unwrap()
            .pairs::<String, Function>()
        {
            let (name, function) = entry.unwrap();
            functions.insert(format!("live.{name}"), function);
        }
        assert_eq!(functions.len(), 15, "all original callable roots");
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
                    ..Default::default()
                },
            )
            .unwrap();
        let lowered = lower_from_sources(&sources, observed.owner()).unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{:?}",
            lowered.unsupported()
        );
        assert_eq!(
            lowered.catalog().data().programs.len(),
            19,
            "complete original roots and transitive helpers"
        );
        let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
        Self {
            lua,
            functions,
            observed,
            compiled,
        }
    }
    fn pair(&self) -> NativePair<'_> {
        let (session, roots) = self
            .compiled
            .session_from_input(self.observed.input(), ProgramLimits::default())
            .unwrap();
        NativePair {
            fixture: self,
            session,
            roots,
            last_output: vec![],
        }
    }
    fn table(&self) -> Table {
        self.lua.create_table().unwrap()
    }
    fn side(&self) -> Table {
        let table = self.table();
        table.raw_set("count", 0).unwrap();
        table
    }
    fn bytes(&self, value: &str) -> Value {
        Value::String(self.lua.create_string(value).unwrap())
    }
}
struct NativePair<'a> {
    fixture: &'a NativeFixture,
    session: ProgramSession,
    roots: Vec<SessionValue>,
    last_output: Vec<SessionValue>,
}
impl NativePair<'_> {
    fn compare(&mut self, name: &str, args: &[Value], failure: bool) -> Vec<Value> {
        let native_args = self
            .session
            .import_with_coverage(&observation::capture(args), &ProgramTableCoverage::new())
            .unwrap();
        let actual =
            self.fixture.functions[name].call::<MultiValue>(MultiValue::from_vec(args.to_vec()));
        let root = self.roots[self.fixture.observed.root_index(name).unwrap()].clone();
        let native = self.session.invoke_callable(&root, &native_args);
        let output = match (actual, native, failure) {
            (Ok(actual), Ok(native), false) => {
                self.last_output = native.clone();
                assert_eq!(
                    observation::canonical(self.session.snapshot(&native).unwrap().graph()),
                    observation::canonical(&observation::capture(&actual.clone().into_vec())),
                    "result {name}"
                );
                actual.into_vec()
            }
            (Err(_), Err(error), true) => {
                assert_eq!(
                    error.kind,
                    ProgramRuntimeErrorKind::Source,
                    "no unsupported stand-in: {name}: {error}"
                );
                vec![]
            }
            (actual, native, _) => {
                panic!("{name}: source={actual:?}; native={native:?}; expected_failure={failure}")
            }
        };
        assert_eq!(
            observation::canonical(self.session.snapshot(&native_args).unwrap().graph()),
            observation::canonical(&observation::capture(args)),
            "state prefix {name}"
        );
        output
    }
}
#[test]
fn original_native_assignments_preserve_aliases_packs_and_error_prefixes() {
    let f = NativeFixture::new();
    for name in ["index_before_local", "local_before_index"] {
        let table = f.table();
        let result = f.pair().compare(
            name,
            &[
                Value::Integer(1),
                Value::Table(table.clone()),
                Value::Integer(10),
                Value::Integer(2),
            ],
            false,
        );
        assert_eq!(f.lua.coerce_number(result[0].clone()).unwrap(), Some(2.0));
        assert_eq!(table.raw_get::<i32>(1).unwrap(), 10);
        assert_eq!(table.raw_get::<Value>(2).unwrap(), Value::Nil);
    }
    let table = f.table();
    let result = f.pair().compare(
        "duplicate",
        &[Value::Integer(0), Value::Table(table.clone())],
        false,
    );
    assert_eq!(f.lua.coerce_number(result[0].clone()).unwrap(), Some(1.0));
    assert_eq!(table.raw_get::<i32>(1).unwrap(), 3);
    let table = f.table();
    let side = f.side();
    f.pair().compare(
        "extra",
        &[Value::Table(table.clone()), Value::Table(side.clone())],
        false,
    );
    assert_eq!(table.raw_get::<i32>(1).unwrap(), 2);
    assert_eq!(side.raw_get::<i32>("count").unwrap(), 3);
    let table = f.table();
    f.pair()
        .compare("packs", &[Value::Table(table.clone())], false);
    assert_eq!(table.raw_get::<Value>(1).unwrap(), Value::Nil);
    assert_eq!(table.raw_get::<i32>(2).unwrap(), 3);
    assert_eq!(table.raw_get::<i32>(3).unwrap(), 4);
    for (name, count, stored) in [
        ("rhs_error", 2, false),
        ("store_right_error", 2, false),
        ("store_left_error", 2, true),
        ("address_error", 0, false),
    ] {
        let table = f.table();
        let side = f.side();
        f.pair().compare(
            name,
            &[
                Value::Table(table.clone()),
                Value::Table(side.clone()),
                Value::Nil,
            ],
            true,
        );
        assert_eq!(side.raw_get::<i32>("count").unwrap(), count, "{name}");
        assert_eq!(
            table.raw_get::<Value>("right").unwrap() != Value::Nil,
            stored,
            "{name}"
        );
    }
    for name in ["single", "read", "nested"] {
        let table = f.table();
        table.raw_set("slot", 9).unwrap();
        let child = f.table();
        child.raw_set("slot", 8).unwrap();
        table.raw_set("child", child).unwrap();
        f.pair()
            .compare(name, &[Value::Table(table), Value::Table(f.side())], false);
        let side = f.side();
        f.pair()
            .compare(name, &[Value::Nil, Value::Table(side.clone())], true);
        assert_eq!(
            side.raw_get::<i32>("count").unwrap(),
            if name == "single" { 2 } else { 1 }
        );
    }
}
#[test]
fn original_live_capture_assignments_continue_with_exact_cells_and_frozen_addresses() {
    let f = NativeFixture::new();
    let mut pair = f.pair();
    let initial = pair.compare("live.read", &[], false);
    let old = initial[1].as_table().unwrap().clone();
    let native_old = pair.last_output[1].clone();
    let replacement = f.table();
    pair.compare(
        "live.capture_addresses",
        &[Value::Table(replacement.clone()), f.bytes("new")],
        false,
    );
    assert_eq!(old.raw_get::<i32>("old").unwrap(), 11);
    assert_eq!(
        observation::canonical(pair.session.snapshot(&[native_old]).unwrap().graph()),
        observation::canonical(&observation::capture(&[Value::Table(old.clone())]))
    );
    assert_eq!(replacement.raw_get::<Value>("old").unwrap(), Value::Nil);
    let current = pair.compare("live.read", &[], false);
    let native_replacement = pair.last_output[1].clone();
    assert_eq!(
        current[1].as_table().unwrap().to_pointer(),
        replacement.to_pointer()
    );
    let final_table = f.table();
    pair.compare(
        "live.capture_addresses",
        &[Value::Table(final_table.clone()), f.bytes("final")],
        false,
    );
    assert_eq!(replacement.raw_get::<i32>("new").unwrap(), 11);
    assert_eq!(
        observation::canonical(
            pair.session
                .snapshot(&[native_replacement])
                .unwrap()
                .graph()
        ),
        observation::canonical(&observation::capture(&[Value::Table(replacement.clone())]))
    );
    assert_eq!(final_table.raw_get::<Value>("new").unwrap(), Value::Nil);
    let table = f.table();
    let mixed = pair.compare("live.mixed", &[Value::Table(table.clone())], false);
    assert_eq!(f.lua.coerce_number(mixed[0].clone()).unwrap(), Some(1.0));
    assert_eq!(f.lua.coerce_number(mixed[1].clone()).unwrap(), Some(2.0));
    assert_eq!(table.raw_get::<i32>(1).unwrap(), 3);
    pair.compare("live.read", &[], false);
    // A fresh session uses the same shared owner and original input artifact;
    // the source comparison independently constructs the original closures again.
    let (mut isolated, roots) = f
        .compiled
        .session_from_input(f.observed.input(), ProgramLimits::default())
        .unwrap();
    let actual = isolated
        .invoke_callable(&roots[f.observed.root_index("live.read").unwrap()], &[])
        .unwrap();
    let fresh: Table = f
        .lua
        .load(TEXT)
        .set_name(format!("@{PATH}"))
        .eval()
        .unwrap();
    let source: MultiValue = fresh
        .raw_get::<Table>("native_live")
        .unwrap()
        .raw_get::<Function>("read")
        .unwrap()
        .call(())
        .unwrap();
    assert_eq!(
        observation::canonical(isolated.snapshot(&actual).unwrap().graph()),
        observation::canonical(&observation::capture(&source.into_vec()))
    );
}
#[test]
fn exact_original_assignment_functions_match_warmed_alias_results() {
    let f = NativeFixture::new();
    let driver = warm::SourceWarmDriver::new(&f.lua).unwrap();
    for name in ["index_before_local", "local_before_index", "duplicate"] {
        let table = f.table();
        let args = if name == "duplicate" {
            vec![Value::Integer(0), Value::Table(table.clone())]
        } else {
            vec![
                Value::Integer(1),
                Value::Table(table.clone()),
                Value::Integer(10),
                Value::Integer(2),
            ]
        };
        let expected = f.pair().compare(name, &args, false);
        let before = observation::canonical(&observation::capture(&args));
        let result = driver.run(&f.lua, &f.functions[name], &args, None).unwrap();
        assert!(result.success);
        assert_eq!(result.calls, 128);
        assert_eq!(result.seed_calls, 0);
        assert!(result.target_live_traces > 0 && result.live_traces >= result.target_live_traces);
        assert_eq!(result.value, expected[0]);
        assert_eq!(observation::canonical(&observation::capture(&args)), before);
    }
}
