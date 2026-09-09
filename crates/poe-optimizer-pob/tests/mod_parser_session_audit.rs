//! Bounded source session observations for the next mutable parser-overlay phase.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use mlua::{Function, HookTriggers, Lua, Table, Value, VmState};
use public_source::PublicSource;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
fn upvalues(lua: &Lua, function: &Function) -> Vec<(String, Value)> {
    (1..=i32::from(function.info().num_upvalues))
        .map(|index| {
            // SAFETY: inspect one validated and rooted Function, and return exactly
            // its existing upvalue name and value. No upvalue or environment changes.
            unsafe {
                lua.exec_raw(function.clone(), |state| {
                    let name = mlua::ffi::lua_getupvalue(state, 1, index);
                    mlua::ffi::lua_pushstring(state, name);
                    mlua::ffi::lua_insert(state, -2);
                    mlua::ffi::lua_remove(state, 1);
                })
            }
            .unwrap()
        })
        .collect()
}
fn upvalue(source: &PublicSource, function: &Function, name: &str) -> Value {
    upvalues(&source.source.lua, function)
        .into_iter()
        .find(|(key, _)| key == name)
        .unwrap()
        .1
}
fn internal(source: &PublicSource) -> Function {
    upvalue(source, &source.parse, "parseMod")
        .as_function()
        .unwrap()
        .clone()
}
fn dictionary(source: &PublicSource, name: &str) -> Table {
    upvalue(source, &internal(source), name)
        .as_table()
        .unwrap()
        .clone()
}
fn names(source: &PublicSource, text: &str) -> Vec<String> {
    source
        .raw(text.as_bytes())
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .clone()
        .sequence_values::<Table>()
        .map(|row| row.unwrap().get("name").unwrap())
        .collect()
}
#[test]
fn original_cache_keys_retries_and_eviction_are_session_observable() {
    let source = PublicSource::new();
    let lines = Arc::new(Mutex::new(Vec::new()));
    let observed = lines.clone();
    source
        .source
        .lua
        .set_hook(HookTriggers::EVERY_LINE, move |_, debug| {
            if debug.source().source.as_deref() == Some("@src/Modules/ModParser.lua")
                && debug.current_line() == Some(6621)
            {
                observed.lock().unwrap().push(debug.current_line().unwrap());
            }
            Ok(VmState::Continue)
        })
        .unwrap();
    for (text, expected) in [
        ("not an actual modifier", vec![6621]),
        ("20% increased not a stat", vec![6621, 6621]),
        ("+7 to Strength and Dexterity", vec![6621]),
        ("+7 to strength and dexterity", vec![6621]),
    ] {
        lines.lock().unwrap().clear();
        source
            .parse
            .call::<mlua::MultiValue>((text, false))
            .unwrap();
        assert_eq!(*lines.lock().unwrap(), expected, "{text}");
        lines.lock().unwrap().clear();
        source.parse.call::<mlua::MultiValue>((text, true)).unwrap();
        assert!(
            lines.lock().unwrap().is_empty(),
            "combined changes cache key"
        );
    }
    source.source.lua.remove_hook();
    assert_eq!(source.cache.clone().pairs::<Value, Value>().count(), 4);
    assert_eq!(
        names(&source, "+7 to Strength and Dexterity"),
        ["Str", "Dex", "StrDex"]
    );
    names(&source, "Strength and Dexterity is doubled");
    assert_eq!(
        names(&source, "+7 to Strength and Dexterity"),
        ["Str", "Dex", "StrDex"]
    );
    assert_eq!(
        names(&source, "+8 to Strength and Dexterity"),
        ["Str", "Multiplier:StrDoubled", "StrDex"]
    );
    source
        .cache
        .raw_set("+7 to Strength and Dexterity", Value::Nil)
        .unwrap();
    assert_eq!(
        names(&source, "+7 to Strength and Dexterity"),
        ["Str", "Multiplier:StrDoubled", "StrDex"]
    );
    eprintln!(
        "Original cache: exact case-sensitive key, combined excluded, nil failure cached, truthy empty list retries; eviction changes old outputs after DOUBLED"
    );
}
#[test]
fn mutation_survives_injected_host_error_before_cache_assignment() {
    let source = PublicSource::new();
    let line = "Strength and Dexterity is doubled";
    source
        .source
        .lua
        .set_hook(HookTriggers::EVERY_LINE, |_, debug| {
            if debug.source().source.as_deref() == Some("@src/Modules/ModParser.lua")
                && debug.current_line() == Some(6923)
            {
                return Err(mlua::Error::RuntimeError(
                    "test-only host abort after original mutation".into(),
                ));
            }
            Ok(VmState::Continue)
        })
        .unwrap();
    let failure = source.raw(line.as_bytes()).unwrap_err();
    source.source.lua.remove_hook();
    assert!(failure.to_string().contains("test-only host abort"));
    assert!(matches!(
        source.cache.raw_get::<Value>(line).unwrap(),
        Value::Nil
    ));
    assert_eq!(
        names(&source, "+8 to Strength and Dexterity"),
        ["Str", "Multiplier:StrDoubled", "StrDex"]
    );
    let result = names(&source, line);
    assert_eq!(result, ["Str", "Multiplier:StrDoubled", "StrDex"]);
    assert!(matches!(
        source.cache.raw_get::<Value>(line).unwrap(),
        Value::Table(_)
    ));
    eprintln!(
        "Original unmodified instructions retain DOUBLED mutation after explicitly injected host hook error; failed line has no cache entry and retries"
    );
}
fn reaches_table(lua: &Lua, value: Value, target: usize, seen: &mut BTreeSet<(u8, usize)>) -> bool {
    assert!(seen.len() < 100_000, "source graph observation bound");
    match value {
        Value::Table(table) => {
            let pointer = table.to_pointer() as usize;
            if pointer == target {
                return true;
            }
            if !seen.insert((0, pointer)) {
                return false;
            }
            table.pairs::<Value, Value>().any(|row| {
                let (key, value) = row.unwrap();
                reaches_table(lua, key, target, seen) || reaches_table(lua, value, target, seen)
            })
        }
        Value::Function(function) => {
            if !seen.insert((1, function.to_pointer() as usize)) {
                return false;
            }
            upvalues(lua, &function)
                .into_iter()
                .any(|(_, v)| reaches_table(lua, v, target, seen))
        }
        _ => false,
    }
}
#[test]
fn original_static_tables_are_isolated_but_jewel_closures_capture_live_parser_state() {
    let source = PublicSource::new();
    let dictionary = dictionary(&source, "modNameList");
    let target = dictionary
        .get::<Table>("strength and dexterity")
        .unwrap()
        .to_pointer() as usize;
    let mut aliases = vec![];
    for row in dictionary.clone().pairs::<String, Value>() {
        let (name, value) = row.unwrap();
        if value
            .as_table()
            .is_some_and(|table| table.to_pointer() as usize == target)
        {
            aliases.push(name);
        }
    }
    aliases.sort();
    assert_eq!(aliases, ["strength and dexterity"]);
    {
        let name = "specialModList";
        let mut seen = BTreeSet::new();
        assert!(
            !reaches_table(
                &source.source.lua,
                Value::Table(dictionary_for(&source, name)),
                target,
                &mut seen
            ),
            "{name} aliases shared name table"
        );
        eprintln!(
            "Original {name} graph: {} distinct tables/functions, no composite name table alias",
            seen.len()
        );
    }
    for text in [
        "+7 to Strength and Dexterity",
        "Strength and Dexterity is doubled",
        "+8 to Strength and Dexterity",
    ] {
        let output = source.raw(text.as_bytes()).unwrap();
        assert!(!output.into_iter().any(|value| reaches_table(
            &source.source.lua,
            value,
            target,
            &mut BTreeSet::new()
        )));
    }
    assert!(!reaches_table(
        &source.source.lua,
        Value::Table(source.cache.clone()),
        target,
        &mut BTreeSet::new()
    ));
    let jewel = source
        .raw(b"Notable Passive Skills in Radius also grant +7 to Strength and Dexterity")
        .unwrap();
    let root = jewel.front().unwrap().as_table().unwrap();
    let modifier = root.get::<Table>(1).unwrap();
    let payload = modifier.get::<Table>("value").unwrap();
    let function = payload.get::<Function>("func").unwrap();
    assert_eq!(function.info().line_defined, Some(7383));
    let inner = upvalue(&source, &function, "innerFuncOrNil")
        .as_function()
        .unwrap()
        .clone();
    assert_eq!(inner.info().line_defined, Some(7140));
    let captured_parser = upvalue(&source, &inner, "parseMod")
        .as_function()
        .unwrap()
        .clone();
    assert_eq!(captured_parser.to_pointer(), internal(&source).to_pointer());
    let captured_names = upvalue(&source, &captured_parser, "modNameList")
        .as_table()
        .unwrap()
        .clone();
    assert_eq!(
        captured_names
            .get::<Table>("strength and dexterity")
            .unwrap()
            .to_pointer() as usize,
        target
    );
    assert!(reaches_table(
        &source.source.lua,
        Value::Table(root.clone()),
        target,
        &mut BTreeSet::new()
    ));
    assert!(reaches_table(
        &source.source.lua,
        Value::Table(source.cache.clone()),
        target,
        &mut BTreeSet::new()
    ));
    eprintln!(
        "Verified original alias: cached/public JewelFunc.value.func(7383) -> innerFuncOrNil(7140) -> parseMod(6619) -> modNameList['strength and dexterity']; copyTable preserves function identity"
    );
}
fn dictionary_for(source: &PublicSource, name: &str) -> Table {
    dictionary(source, name)
}
