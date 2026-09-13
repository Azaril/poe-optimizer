//! Original-source observations only: no native correctness or full-build claim.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[path = "support/explosion_helper_observer.rs"]
mod observer;
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use factory_source::{FactorySource, upvalue};
use mlua::{Function, MultiValue, Table, Value};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const LINE: &[u8] = b"Warcries Explode Corpses dealing 10% of their Life as Physical Damage";
const PATTERN: &str = "^warcries explode corpses dealing (%d+)%% of their life as (.+) damage$";
struct Original {
    source: FactorySource,
    callback: Function,
    helper: Function,
    upper: Function,
    flag: Function,
}
fn declaration(f: &Function, first: usize, last: usize) {
    let info = f.info();
    assert_eq!(info.source.as_deref(), Some("@src/Modules/ModParser.lua"));
    assert_eq!(info.line_defined, Some(first));
    assert_eq!(info.last_line_defined, Some(last));
}
impl Original {
    fn new() -> Self {
        let source = FactorySource::new();
        let callback: Function = source.special.raw_get(PATTERN).unwrap();
        let helper = upvalue(&source.public.source.lua, &callback, "explodeFunc")
            .as_function()
            .unwrap()
            .clone();
        let upper = upvalue(&source.public.source.lua, &helper, "firstToUpper")
            .as_function()
            .unwrap()
            .clone();
        let flag = upvalue(&source.public.source.lua, &helper, "flag")
            .as_function()
            .unwrap()
            .clone();
        declaration(&callback, 2336, 2338);
        declaration(&helper, 2255, 2266);
        declaration(&upper, 13, 15);
        declaration(&flag, 2177, 2179);
        assert_eq!(
            upvalue(&source.public.source.lua, &helper, "mod").as_function(),
            Some(&source.create_mod)
        );
        assert_eq!(
            upvalue(&source.public.source.lua, &flag, "mod").as_function(),
            Some(&source.create_mod)
        );
        Self {
            source,
            callback,
            helper,
            upper,
            flag,
        }
    }
    fn verify(&self) {
        let lua = &self.source.public.source.lua;
        assert_eq!(
            self.source.special.raw_get::<Function>(PATTERN).unwrap(),
            self.callback
        );
        for (f, n, wanted) in [
            (&self.callback, "explodeFunc", &self.helper),
            (&self.helper, "firstToUpper", &self.upper),
            (&self.helper, "flag", &self.flag),
            (&self.helper, "mod", &self.source.create_mod),
            (&self.flag, "mod", &self.source.create_mod),
        ] {
            assert_eq!(upvalue(lua, f, n).as_function(), Some(wanted));
        }
        declaration(&self.callback, 2336, 2338);
        declaration(&self.helper, 2255, 2266);
    }
    fn text(&self, v: impl AsRef<[u8]>) -> Value {
        self.source.text(v)
    }
    fn run(
        &self,
        entry: &Function,
        args: Vec<Value>,
    ) -> (serde_json::Value, mlua::Result<MultiValue>) {
        self.verify();
        let args = MultiValue::from_vec(args);
        let supplied = observer::graph(&self.source, args.clone());
        let info = entry.info();
        let input = json!({
            "entry_source": info.source,
            "entry_first_line": info.line_defined,
            "entry_last_line": info.last_line_defined,
            "supplied_inputs": supplied,
            "internal_call_frames_observed": false
        });
        let output = entry.call::<MultiValue>(args);
        self.verify();
        (input, output)
    }
    fn public(&self, line: &[u8]) -> (serde_json::Value, mlua::Result<MultiValue>) {
        self.run(
            &self.source.public.parse,
            vec![self.text(line), Value::Boolean(false)],
        )
    }
    fn direct(&self, args: Vec<Value>) -> (serde_json::Value, mlua::Result<MultiValue>) {
        self.run(&self.helper, args)
    }
}
fn write(name: &str, body: serde_json::Value) {
    let directory = std::env::var_os("POE_EXPLOSION_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime::repository().join("runs/r2aj-explosion-helper-01/source-output")
        });
    std::fs::create_dir_all(&directory).unwrap();
    let source = runtime::verified("src/Modules/ModParser.lua").unwrap();
    let report = json!({"scope":"original-source observation only; no native correctness, runtime admission or complete-build claim","source_module_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"source_helpers_unchanged":true,"internal_call_frames_observed":false,"inputs_are_supplied_direct_call_packs":true,"output":body});
    std::fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
fn rows(values: &MultiValue) -> Table {
    assert_eq!(
        values.len(),
        1,
        "complete original helper/public pack has one table"
    );
    values.front().unwrap().as_table().unwrap().clone()
}
fn payload(values: &MultiValue) -> Table {
    rows(values)
        .raw_get::<Table>(1)
        .unwrap()
        .raw_get("value")
        .unwrap()
}
fn number(v: Value) -> f64 {
    match v {
        Value::Integer(v) => v as f64,
        Value::Number(v) => v,
        _ => panic!("expected number"),
    }
}
fn assert_success_pack(values: &MultiValue, chance: f64, amount: f64, kind: &[u8]) {
    let list = rows(values);
    assert_eq!(list.raw_len(), 2);
    let first: Table = list.raw_get(1).unwrap();
    let second: Table = list.raw_get(2).unwrap();
    assert_eq!(first.raw_get::<String>("name").unwrap(), "ExplodeMod");
    assert_eq!(first.raw_get::<String>("type").unwrap(), "LIST");
    let p = payload(values);
    assert_eq!(
        p.raw_get::<mlua::LuaString>("type")
            .unwrap()
            .as_bytes()
            .as_ref(),
        kind
    );
    assert_eq!(
        number(p.raw_get("value").unwrap()).to_bits(),
        chance.to_bits()
    );
    let actual = number(p.raw_get("amount").unwrap());
    if amount.is_nan() {
        assert!(actual.is_nan());
    } else {
        assert_eq!(actual.to_bits(), amount.to_bits());
    }
    assert_eq!(p.raw_get::<String>("keyOfScaledMod").unwrap(), "value");
    assert_eq!(second.raw_get::<String>("name").unwrap(), "CanExplode");
    assert_eq!(second.raw_get::<String>("type").unwrap(), "FLAG");
    assert!(second.raw_get::<bool>("value").unwrap());
    for row in [first, second] {
        assert_eq!(number(row.raw_get("flags").unwrap()), 0.0);
        assert_eq!(number(row.raw_get("keywordFlags").unwrap()), 0.0);
    }
}

#[test]
fn actual_deidbell_public_full_output_and_cache_copy_history() {
    let original = Original::new();
    let control = Original::new();
    let cache_key = original.text(LINE);
    assert!(
        original
            .source
            .public
            .cache
            .raw_get::<Value>(cache_key.clone())
            .unwrap()
            .is_nil()
    );
    let (first_input, first) = original.public(LINE);
    let first = first.unwrap();
    assert_success_pack(&first, 100.0, 10.0, b"Physical");
    let initial_graph = observer::graph(&original.source, first.clone());
    let cache_record: Table = original
        .source
        .public
        .cache
        .raw_get(cache_key.clone())
        .unwrap();
    let cache_graph = observer::graph(
        &original.source,
        MultiValue::from_vec(vec![Value::Table(cache_record.clone())]),
    );
    let control_graph = observer::graph(&control.source, control.source.public.raw(LINE).unwrap());
    assert_eq!(
        initial_graph, control_graph,
        "separate original host control over the complete returned graph"
    );
    let (hit_input, hit) = original.public(LINE);
    let hit = hit.unwrap();
    assert_eq!(
        original
            .source
            .public
            .cache
            .raw_get::<Table>(cache_key.clone())
            .unwrap(),
        cache_record
    );
    assert_ne!(rows(&first).to_pointer(), rows(&hit).to_pointer());
    assert_ne!(payload(&first).to_pointer(), payload(&hit).to_pointer());
    assert_eq!(initial_graph, observer::graph(&original.source, hit));
    payload(&first).raw_set("amount", 999).unwrap();
    rows(&first)
        .raw_get::<Table>(2)
        .unwrap()
        .raw_set("name", "caller mutation")
        .unwrap();
    let (after_mutation, output) = original.public(LINE);
    let output = output.unwrap();
    assert_eq!(initial_graph, observer::graph(&original.source, output));
    assert_eq!(
        cache_graph,
        observer::graph(
            &original.source,
            MultiValue::from_vec(vec![Value::Table(cache_record.clone())])
        )
    );
    assert_eq!(
        original
            .source
            .public
            .cache
            .raw_get::<Table>(cache_key.clone())
            .unwrap(),
        cache_record
    );
    original
        .source
        .public
        .cache
        .raw_set(cache_key.clone(), Value::Nil)
        .unwrap();
    assert!(
        original
            .source
            .public
            .cache
            .raw_get::<Value>(cache_key.clone())
            .unwrap()
            .is_nil()
    );
    let (reparse, output) = original.public(LINE);
    let output = output.unwrap();
    assert_ne!(
        original
            .source
            .public
            .cache
            .raw_get::<Table>(cache_key)
            .unwrap(),
        cache_record
    );
    assert_eq!(initial_graph, observer::graph(&original.source, output));
    original.verify();
    control.verify();
    write(
        "actual-deidbell",
        json!({"line":LINE,"first":first_input,"cache_hit":hit_input,"after_return_mutation":after_mutation,"eviction_reparse":reparse,"complete_result_graph":initial_graph,"retained_cache_record_graph":cache_graph,"independent_control_graph":control_graph,"internal_call_frames_observed":false,"scope":"supplied public calls, complete results and live returned-copy isolation; cache eviction is an explicit derived history"}),
    );
}

#[test]
fn same_original_public_pattern_variants_keep_complete_result_or_remainder() {
    let original = Original::new();
    let control = Original::new();
    let mut results = vec![];
    for line in [
        "Warcries Explode Corpses dealing 0% of their Life as Physical Damage",
        "Warcries Explode Corpses dealing 007% of their Life as FIRE Damage",
        "Warcries Explode Corpses dealing 25% of their Life as cold damage",
        "Warcries Explode Corpses dealing 10% of their Life as 123 damage",
        "Warcries Explode Corpses dealing 10% of their Life as damage",
        "Warcries Explode Corpses dealing 10% of their Life as Physical Damage unmatched",
    ] {
        let (w, actual) = original.public(line.as_bytes());
        let actual = actual.unwrap();
        let a = observer::graph(&original.source, actual);
        let c = observer::graph(
            &control.source,
            control.source.public.raw(line.as_bytes()).unwrap(),
        );
        assert_eq!(a, c);
        results.push(json!({"line":line,"supplied_call":w,"complete_graph":a}));
    }
    original.verify();
    control.verify();
    write(
        "public-variants",
        json!({"cases":results,"scope":"same unchanged original pattern/module; matched and unmatched source returns retained without inventing a modifier"}),
    );
}

#[test]
fn direct_original_amount_fallback_zero_truthiness_and_zero_return() {
    let original = Original::new();
    let mut results = vec![];
    let successful = vec![
        (Value::Number(0.0), 0.0),
        (Value::Number(-0.0), -0.0),
        (Value::Number(17.25), 17.25),
        (original.text(" 7e0 "), 7.0),
        (original.text("0x8"), 8.0),
        (original.text("tenth"), 10.0),
        (original.text("quarter"), 25.0),
        (Value::Number(f64::INFINITY), f64::INFINITY),
        (Value::Number(f64::NEG_INFINITY), f64::NEG_INFINITY),
        (Value::Number(f64::NAN), f64::NAN),
    ];
    for (amount, expected) in successful {
        let input = observer::graph(&original.source, MultiValue::from_vec(vec![amount.clone()]));
        let (w, result) =
            original.direct(vec![Value::Number(37.0), amount, original.text("physical")]);
        let values = result.unwrap();
        assert_success_pack(&values, 37.0, expected, b"Physical");
        results.push(json!({"input_amount":input,"supplied_call":w,"result":observer::graph(&original.source,values)}));
    }
    for amount in [
        Value::Nil,
        Value::Boolean(false),
        original.text("Tenth"),
        original.text("nonsense"),
    ] {
        let (w, result) = original.direct(vec![Value::Number(100.0), amount, Value::Nil]);
        let values = result.unwrap();
        assert!(values.is_empty(), "zero results, not one nil or empty list");
        results.push(json!({"early_return":true,"supplied_call":w,"result":observer::graph(&original.source,values)}));
    }
    write(
        "direct-amounts",
        json!({"scope":"labelled direct calls to retained unchanged original helper; broader than Deidbell numeric wrapper","cases":results}),
    );
}

#[test]
fn dead_local_key_write_and_upper_failures_report_exact_original_lines() {
    let original = Original::new();
    let mut results = vec![];
    for (label, kind, fragment, source_line) in [
        ("nil key", Value::Nil, "table index is nil", 2261),
        (
            "NaN key",
            Value::Number(f64::NAN),
            "table index is NaN",
            2261,
        ),
        (
            "boolean reaches original upper",
            Value::Boolean(true),
            "attempt to index",
            14,
        ),
        (
            "plain table reaches missing gsub",
            Value::Table(original.source.public.source.lua.create_table().unwrap()),
            "gsub",
            14,
        ),
    ] {
        let (w, result) = original.direct(vec![Value::Number(100.0), Value::Number(10.0), kind]);
        let error = result.unwrap_err();
        assert!(
            observer::source_error_at(&error, fragment, source_line),
            "unexpected host/observer/source error: {error}"
        );
        results.push(json!({"case":label,"supplied_call":w,"source_error":error.to_string(),"expected_original_error_line":source_line,"output_graph_available":false}));
    }
    write(
        "direct-errors",
        json!({"cases":results,"scope":"actual original failure at the asserted source line; no observed internal call prefix, output graph or native claim"}),
    );
}

#[test]
fn original_casing_and_forwarded_vararg_holes_keep_tag_identity() {
    let original = Original::new();
    let mut cases = vec![];
    for (input, expected) in [
        ("physical", "Physical"),
        ("PHYSICAL", "PHYSICAL"),
        ("cold damage", "Cold damage"),
        (" leading", " leading"),
        ("7physical", "7physical"),
        ("\u{03bb}damage", "\u{03bb}damage"),
        ("a\0z", "A\0z"),
    ] {
        let (w, result) = original.direct(vec![
            Value::Number(100.0),
            Value::Number(10.0),
            original.text(input),
        ]);
        let values = result.unwrap();
        assert_success_pack(&values, 100.0, 10.0, expected.as_bytes());
        cases.push(
            json!({"input":input,"supplied_call":w,"result":observer::graph(&original.source,values)}),
        );
    }
    let tag = original.source.public.source.lua.create_table().unwrap();
    tag.raw_set("type", "Condition").unwrap();
    tag.raw_set("var", "CallerTag").unwrap();
    let args = vec![
        Value::Number(100.0),
        Value::Number(10.0),
        original.text("physical"),
        original.text("caller source"),
        Value::Number(7.0),
        Value::Number(9.0),
        Value::Table(tag.clone()),
        Value::Nil,
        Value::Table(tag.clone()),
    ];
    let expected_tail = observer::graph(&original.source, MultiValue::from_vec(args[3..].to_vec()));
    let (w, result) = original.direct(args);
    let values = result.unwrap();
    let first: Table = rows(&values).raw_get(1).unwrap();
    let second: Table = rows(&values).raw_get(2).unwrap();
    assert_eq!(first.raw_get::<String>("source").unwrap(), "caller source");
    assert_eq!(number(first.raw_get("flags").unwrap()), 7.0);
    assert_eq!(number(first.raw_get("keywordFlags").unwrap()), 9.0);
    assert_eq!(
        first.raw_get::<Table>(1).unwrap().to_pointer(),
        tag.to_pointer()
    );
    assert!(first.raw_get::<Value>(2).unwrap().is_nil());
    assert_eq!(
        first.raw_get::<Table>(3).unwrap().to_pointer(),
        tag.to_pointer()
    );
    assert!(second.raw_get::<Value>("source").unwrap().is_nil());
    assert!(second.raw_get::<Value>(1).unwrap().is_nil());
    assert_eq!(number(second.raw_get("flags").unwrap()), 0.0);
    write(
        "direct-casing-tags",
        json!({"casing":cases,"forwarded":{"supplied_call":w,"supplied_tail":expected_tail,"full_result":observer::graph(&original.source,values),"same_live_tag_at_indices_1_and_3":true},"scope":"original helper direct inputs; tag holes and identities are observed, internal calls unobserved; no native or physical-layout equivalence claim"}),
    );
}

#[test]
fn observer_rejects_host_errors_and_unbounded_or_nonplain_graphs() {
    assert!(!observer::source_error(
        &mlua::Error::MemoryError("out of memory".into()),
        "memory"
    ));
    assert!(!observer::source_error(
        &mlua::Error::RuntimeError("item loading oracle deadline at ModParser.lua:2256".into()),
        "deadline"
    ));
    assert!(!observer::source_error(
        &mlua::Error::RuntimeError(
            "explosion observer: ModParser.lua:2261 table index is nil".into()
        ),
        "table index is nil"
    ));
    assert!(observer::source_error(
        &mlua::Error::RuntimeError("src/Modules/ModParser.lua:2261: table index is nil".into()),
        "table index is nil"
    ));
    let original_error =
        mlua::Error::RuntimeError("src/Modules/ModParser.lua:2261: table index is nil".into());
    assert!(observer::source_error_at(
        &original_error,
        "table index is nil",
        2261
    ));
    assert!(!observer::source_error_at(
        &original_error,
        "table index is nil",
        14
    ));
    let lua = mlua::Lua::new();
    let t = lua.create_table().unwrap();
    t.set_metatable(Some(lua.create_table().unwrap())).unwrap();
    assert!(observer::preflight(&[Value::Table(t)]).is_err());
    let text = lua.create_string(vec![b'x'; 65_537]).unwrap();
    assert!(observer::preflight(&[Value::String(text)]).is_err());
    assert!(observer::preflight(&vec![Value::Nil; 4097]).is_err());
}
