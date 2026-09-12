//! Independently probe source operand descriptors using explicit host instrumentation.
//! This verifies lowering classification, not execution of source-created closures.
use mlua::{Function, Lua, Table};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_pob::source_programs::{capture::*, lower_from_sources};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};
const PATH: &str = "tests/generated_assignment_classifier.lua";
fn expressions() -> BTreeSet<String> {
    let atoms = [
        "k",
        "true",
        "false",
        "nil",
        "0",
        "1",
        "''",
        "(1+2)",
        "(-0)",
        "(0/0)",
        "(1/0)",
        "(-1/0)",
        "not k",
        "(k and false)",
        "(k or true)",
    ];
    let mut values: BTreeSet<_> = atoms.into_iter().map(str::to_owned).collect();
    for left in atoms {
        for right in atoms {
            for operator in ["and", "or"] {
                values.insert(format!("({left}) {operator} ({right})"));
                values.insert(format!("(({left}) {operator} ({right})) and k"));
                values.insert(format!("(({left}) {operator} ({right})) or k"));
            }
        }
    }
    for value in [
        "k",
        "true and k",
        "false or k",
        "1 and k",
        "not not k",
        "(1==1) and k",
        "(1<2) and k",
        "(k and false) or k",
        "(k or true) and k",
        "(0/0) and k",
        "(-0) and k",
        "(0-0) and k",
        "(0/-1) and k",
        r"('\255') and k",
        r"('\000\255') and k",
        r"('\255') or k",
        r"false or (('\255') and k)",
        r"not ('\255') or k",
    ] {
        values.insert(format!("({value})"));
    }
    assert!(values.len() < 1500);
    values
}
#[test]
fn source_register_classification_matches_instrumented_original_bytecode() {
    // SAFETY: Only fixed test code and expressions from the bounded grammar above
    // run here. Debug access intentionally modifies one named caller parameter;
    // it never enters native execution or serves as production source behavior.
    let lua = unsafe { Lua::unsafe_new() };
    lua.load("jit.off();jit.flush();assert(not jit.status())")
        .exec()
        .unwrap();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let state: Table = lua
        .load(
            r#"local state = {calls=0}
state.rhs = function()
        local name, value = debug.getlocal(2, 1)
        assert(name == 'k' and value == 'old')
        assert(debug.setlocal(2, 1, 'new') == 'k')
        state.calls = state.calls + 1
        return 11
    end
return state"#,
        )
        .set_name("@tests/assignment_classifier_host_instrumentation.lua")
        .eval()
        .unwrap();
    let rhs: Function = state.raw_get("rhs").unwrap();
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    for expression in expressions() {
        let text = format!(
            "return function(k, rhs)\nlocal t = {{}}\nt[{expression}] = rhs()\nreturn t\nend\n"
        );
        let function: Function = lua.load(&text).set_name(format!("@{PATH}")).eval().unwrap();
        let source = ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: BTreeMap::from([(
                PATH.into(),
                format!("{:x}", Sha256::digest(text.as_bytes())),
            )]),
            construction_spans: BTreeMap::new(),
            module_order: vec![PATH.into()],
        };
        let sources = BTreeMap::from([(PATH.into(), text)]);
        let observed = observer
            .observe_with_context(
                &lua,
                &sources,
                source,
                &BTreeMap::from([("store".into(), function.clone())]),
                SourceCaptureContext::default(),
            )
            .unwrap();
        let lowered = lower_from_sources(&sources, observed.owner()).unwrap();
        let callback = observed.callbacks()["store"];
        let program = lowered.catalog().for_callback(callback).unwrap_or_else(|| {
            panic!(
                "whole function failed for {expression}: {:?}",
                lowered.unsupported()
            )
        });
        let SourceProgramStatementKind::MixedAssign { targets, .. } = &program.body[1].operation
        else {
            panic!("expected source mixed assignment: {expression}");
        };
        let SourceProgramAssignmentTargetKind::Indexed { key, .. } = &targets[0].operation else {
            panic!("expected indexed source target");
        };
        let lowered_live = matches!(
            key,
            SourceProgramAssignmentOperand::LocalRegister { local: 0 }
        );
        state.raw_set("calls", 0).unwrap();
        let result = function.call::<Table>(("old", rhs.clone()));
        assert_eq!(
            state.raw_get::<usize>("calls").unwrap(),
            1,
            "RHS injection: {expression}"
        );
        let (source_live, source_error) = match result {
            Ok(table) => (
                table.raw_get::<Option<i32>>("new").unwrap() == Some(11),
                false,
            ),
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("table index is nil")
                        || message.contains("table index is NaN"),
                    "unexpected source failure for {expression}: {message}"
                );
                (false, true)
            }
        };
        if source_live != lowered_live {
            failures.push(expression.clone());
        }
        rows.push(json!({"expression":expression,"source_live":source_live,
            "lowered_live":lowered_live,"source_error":source_error}));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::create_dir_all(root.join("runs")).unwrap();
    fs::write(root.join("runs/r2o-assignment-classifier.json"), serde_json::to_vec_pretty(&json!({
        "scope":"lowerer classification against actual original bytecode with explicit debug.setlocal host injection; no native factory admission",
        "cases":rows,"mismatches":failures})).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "source register descriptor mismatches: {failures:?}"
    );
}

#[test]
fn indexed_read_base_classification_matches_key_effects_in_original_source() {
    // SAFETY: Fixed test code and bounded expressions only; instrumentation is
    // explicit and cannot authenticate native source-created closure execution.
    let lua = unsafe { Lua::unsafe_new() };
    lua.load("jit.off();jit.flush();assert(not jit.status())")
        .exec()
        .unwrap();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let state: Table = lua
        .load(
            r#"local state = {calls=0}
state.key = function()
    local name, value = debug.getlocal(2, 1)
    assert(name == 't' and value == state.old)
    assert(debug.setlocal(2, 1, state.new) == 't')
    state.calls = state.calls + 1
    return 'value'
end
return state"#,
        )
        .set_name("@tests/indexed_read_host_instrumentation.lua")
        .eval()
        .unwrap();
    let key: Function = state.raw_get("key").unwrap();
    let old = lua.create_table().unwrap();
    old.raw_set("value", 11).unwrap();
    let new = lua.create_table().unwrap();
    new.raw_set("value", 22).unwrap();
    state.raw_set("old", old.clone()).unwrap();
    state.raw_set("new", new).unwrap();
    let mut rows = Vec::new();
    for expression in [
        "t",
        "(t)",
        "true and t",
        "false or t",
        "nil or t",
        "1 and t",
        "0 and t",
        "'' and t",
        "(1+2) and t",
        "(-0) and t",
        "(0/0) and t",
        "(1/0) and t",
        "(-1/0) and t",
        "(0/-1) and t",
        "t or false",
        "t and t",
        "(t and false) or t",
        "(t or true) and t",
        "(1==1) and t",
        "(1<2) and t",
        "not not t and t",
        "false and t",
        r"('\255') and t",
        r"('\000\255') and t",
    ] {
        let text = format!("return function(t, key)\nreturn ({expression})[key()]\nend\n");
        let function: Function = lua.load(&text).set_name(format!("@{PATH}")).eval().unwrap();
        let source = ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: BTreeMap::from([(
                PATH.into(),
                format!("{:x}", Sha256::digest(text.as_bytes())),
            )]),
            construction_spans: BTreeMap::new(),
            module_order: vec![PATH.into()],
        };
        let sources = BTreeMap::from([(PATH.into(), text)]);
        let observed = observer
            .observe_with_context(
                &lua,
                &sources,
                source,
                &BTreeMap::from([("read".into(), function.clone())]),
                SourceCaptureContext::default(),
            )
            .unwrap();
        let lowered = lower_from_sources(&sources, observed.owner()).unwrap();
        let program = lowered
            .catalog()
            .for_callback(observed.callbacks()["read"])
            .unwrap_or_else(|| {
                panic!(
                    "whole indexed-read function failed for {expression}: {:?}",
                    lowered.unsupported()
                )
            });
        let SourceProgramStatementKind::Return { values } = &program.body[0].operation else {
            panic!("return")
        };
        let SourceProgramExprKind::IndexedRead { table, .. } = &values.values[0].operation else {
            panic!("indexed read")
        };
        let lowered_live = matches!(
            &**table,
            SourceProgramAssignmentOperand::LocalRegister { local: 0 }
        );
        state.raw_set("calls", 0).unwrap();
        let result = function.call::<mlua::Value>((old.clone(), key.clone()));
        assert_eq!(
            state.raw_get::<usize>("calls").unwrap(),
            1,
            "key injection: {expression}"
        );
        let (source_live, source_error) = match result {
            Ok(value) => (lua.coerce_number(value).unwrap() == Some(22.0), false),
            Err(error) => {
                assert!(
                    error.to_string().contains("attempt to index"),
                    "unexpected source error: {error}"
                );
                (false, true)
            }
        };
        rows.push(json!({"expression":expression,"lowered_live":lowered_live,"source_live":source_live,"source_error":source_error}));
        assert_eq!(lowered_live, source_live, "indexed read base: {expression}");
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::create_dir_all(root.join("runs")).unwrap();
    fs::write(root.join("runs/r2o-indexed-read-classifier.json"),serde_json::to_vec_pretty(&json!({
        "scope":"actual source indexed-read timing under explicit host key injection; classification evidence, no native factory admission",
        "cases":rows})).unwrap()).unwrap();
}
