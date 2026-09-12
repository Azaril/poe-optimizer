//! Original copyTable identity/alias witness tests, separate from native parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/source_program_copy_witness.rs"]
mod witness;
use mlua::{Function, HookTriggers, Lua, MultiValue, Table, Value, VmState};
use std::{cell::Cell, path::PathBuf, rc::Rc};
use witness::SourceCopyWitness;
fn host() -> (Lua, SourceCopyWitness, Function) {
    // Test host needs original debug inspection; no production debug API changes.
    let lua = unsafe { Lua::unsafe_new() };
    let witness = SourceCopyWitness::before_source(&lua).unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/path-of-building-poe2/src/Modules/Common.lua");
    let text = std::fs::read_to_string(path).unwrap();
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    assert_eq!(lines[494].trim_end(), "function copyTable(tbl, noRecurse)");
    lua.load(format!("{}{}", "\n".repeat(494), lines[494..505].concat()))
        .set_name("@src/Modules/Common.lua")
        .exec()
        .unwrap();
    let copy = lua.globals().raw_get("copyTable").unwrap();
    (lua, witness, copy)
}
fn no_hook(lua: &Lua) {
    let hook: Function = lua
        .globals()
        .get::<Table>("debug")
        .unwrap()
        .get("gethook")
        .unwrap();
    let values: MultiValue = hook.call(()).unwrap();
    assert!(matches!(values.front(), Some(Value::Nil) | None));
}
#[test]
fn exact_original_activation_identity_preserves_shared_and_equal_distinct_children() {
    let (lua, witness, copy) = host();
    let input: Table = lua
        .load("local shared={n=1}; return {left=shared,right=shared,equal={n=1}} ")
        .eval()
        .unwrap();
    let result = witness
        .call(
            &lua,
            &copy,
            &copy,
            MultiValue::from_vec(vec![Value::Table(input.clone())]),
        )
        .unwrap();
    let output: Table = result
        .result
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .clone();
    assert_eq!(result.activations.len(), 4);
    assert_eq!(result.activations[0].table, Value::Table(input.clone()));
    assert_eq!(result.activations[0].parent, None);
    assert_eq!(result.activations[0].depth, 1);
    for (i, activation) in result.activations.iter().enumerate() {
        assert_eq!(activation.ordinal, i);
        assert_eq!(activation.no_recurse, Value::Nil);
    }
    let child = |name: &str| {
        result
            .activations
            .iter()
            .find(|a| {
                a.parent_key
                    .as_ref()
                    .and_then(Value::as_string)
                    .is_some_and(|v| v.to_str().unwrap() == name)
            })
            .unwrap()
    };
    assert_eq!(child("left").table, child("right").table);
    assert_ne!(child("left").table, child("equal").table);
    assert_eq!(child("left").parent, Some(0));
    assert_eq!(child("left").depth, 2);
    assert_eq!(child("left").table, input.raw_get::<Value>("left").unwrap());
    assert_ne!(
        output.raw_get::<Table>("left").unwrap(),
        output.raw_get::<Table>("right").unwrap()
    );
    assert_ne!(
        Value::Table(output.raw_get::<Table>("left").unwrap()),
        child("left").table
    );
    let plain: Table = copy.call(input.clone()).unwrap();
    for key in ["left", "right", "equal"] {
        assert_eq!(
            output
                .raw_get::<Table>(key)
                .unwrap()
                .raw_get::<i64>("n")
                .unwrap(),
            plain
                .raw_get::<Table>(key)
                .unwrap()
                .raw_get::<i64>("n")
                .unwrap()
        );
    }
    assert!(result.loops.iter().any(|item| item.visible_key.is_some()));
    for item in &result.loops {
        assert_eq!(item.table, result.activations[item.activation].table);
        assert!([497, 498].contains(&item.source_line));
        assert_eq!(
            item.visible_control.is_none(),
            item.control_unavailable_reason.is_some()
        );
        assert!(!matches!(
            item.visible_control,
            Some(Value::LightUserData(_))
        ));
    }
    no_hook(&lua);
    // An equal source body in a different actual Function must produce no events.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/path-of-building-poe2/src/Modules/Common.lua");
    let text = std::fs::read_to_string(path).unwrap();
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let env = lua.create_table().unwrap();
    lua.load(format!("{}{}", "\n".repeat(494), lines[494..505].concat()))
        .set_name("@src/Modules/Common.lua")
        .set_environment(env.clone())
        .exec()
        .unwrap();
    let other: Function = env.raw_get("copyTable").unwrap();
    assert_eq!(other.info().source, copy.info().source);
    assert_eq!(other.info().line_defined, copy.info().line_defined);
    assert_ne!(other, copy);
    let wrong = witness
        .call(
            &lua,
            &copy,
            &other,
            MultiValue::from_vec(vec![Value::Table(input)]),
        )
        .unwrap();
    assert!(wrong.activations.is_empty() && wrong.loops.is_empty());
    assert!(wrong.result.is_ok());
    no_hook(&lua);
}
#[test]
fn source_errors_and_witness_bounds_remove_hooks_even_when_errors_are_caught() {
    let (lua, witness, copy) = host();
    let source = witness
        .call(
            &lua,
            &copy,
            &copy,
            MultiValue::from_vec(vec![Value::Number(7.0)]),
        )
        .unwrap();
    assert!(source.result.is_err());
    assert_eq!(source.activations.len(), 1);
    assert_eq!(source.activations[0].table, Value::Number(7.0));
    no_hook(&lua);
    let deep: Table = lua
        .load("local root={} local t=root for i=1,33 do t.child={} t=t.child end return root")
        .eval()
        .unwrap();
    let error = witness
        .call(
            &lua,
            &copy,
            &copy,
            MultiValue::from_vec(vec![Value::Table(deep)]),
        )
        .unwrap_err();
    assert!(error.to_string().contains("depth bound"), "{error}");
    no_hook(&lua);
    let many: Table = lua
        .load("local t={} for i=1,300 do t[i]=i end return t")
        .eval()
        .unwrap();
    let caught: Function = lua
        .load("return function(tbl) local ok,value=pcall(copyTable,tbl); return ok,value end")
        .eval()
        .unwrap();
    let error = witness
        .call(
            &lua,
            &caught,
            &copy,
            MultiValue::from_vec(vec![Value::Table(many)]),
        )
        .unwrap_err();
    assert!(error.to_string().contains("event bound"), "{error}");
    no_hook(&lua);
    let valid = lua.create_table().unwrap();
    assert!(
        witness
            .call(
                &lua,
                &copy,
                &copy,
                MultiValue::from_vec(vec![Value::Table(valid)])
            )
            .unwrap()
            .result
            .is_ok()
    );
    no_hook(&lua);
}
#[test]
fn existing_hooks_are_rejected_and_retained_inspection_ignores_global_rebinding() {
    let (lua, witness, copy) = host();
    let count = Rc::new(Cell::new(0));
    let events = count.clone();
    lua.set_hook(HookTriggers::EVERY_LINE, move |_, _| {
        events.set(events.get() + 1);
        Ok(VmState::Continue)
    })
    .unwrap();
    assert!(
        witness
            .call(&lua, &copy, &copy, MultiValue::new())
            .unwrap_err()
            .to_string()
            .contains("existing hook")
    );
    lua.load("local a=1\na=a+1\nreturn a")
        .eval::<i64>()
        .unwrap();
    assert!(count.get() > 0);
    lua.remove_hook();
    lua.load("debug.getinfo=function() error('rebound') end; debug.getlocal=debug.getinfo; debug.gethook=debug.getinfo").exec().unwrap();
    let table = lua.create_table().unwrap();
    let actual = witness
        .call(
            &lua,
            &copy,
            &copy,
            MultiValue::from_vec(vec![Value::Table(table)]),
        )
        .unwrap();
    assert!(actual.result.is_ok());
    assert_eq!(actual.activations.len(), 1);
}

#[test]
fn caught_source_errors_reset_activation_context_and_hook_tampering_is_explicit() {
    let (lua, witness, copy) = host();
    let parser:Function=lua.load("return function(tbl) local ok=pcall(copyTable,7); assert(not ok); return copyTable(tbl,true) end").eval().unwrap();
    let input: Table = lua.load("return {child={n=1}}").eval().unwrap();
    let actual = witness
        .call(
            &lua,
            &parser,
            &copy,
            MultiValue::from_vec(vec![Value::Table(input.clone())]),
        )
        .unwrap();
    let result = actual.result.unwrap();
    assert_eq!(actual.activations.len(), 2);
    assert_eq!(actual.activations[1].parent, None);
    assert_eq!(actual.activations[1].depth, 1);
    assert_eq!(actual.activations[1].no_recurse, Value::Boolean(true));
    assert_eq!(
        result[0]
            .as_table()
            .unwrap()
            .raw_get::<Value>("child")
            .unwrap(),
        input.raw_get::<Value>("child").unwrap()
    );
    no_hook(&lua);
    let tamper: Function = lua
        .load("return function(tbl) debug.sethook(); return copyTable(tbl) end")
        .eval()
        .unwrap();
    let error = witness
        .call(
            &lua,
            &tamper,
            &copy,
            MultiValue::from_vec(vec![Value::Table(input)]),
        )
        .unwrap_err();
    assert!(error.to_string().contains("hook was changed"), "{error}");
    no_hook(&lua);
    let (foreign, _, target) = host();
    let error = witness
        .call(&foreign, &target, &target, MultiValue::new())
        .unwrap_err();
    assert!(error.to_string().contains("different Lua host"), "{error}");
    no_hook(&foreign);
}
