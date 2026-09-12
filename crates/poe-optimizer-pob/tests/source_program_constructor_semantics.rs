//! Original LuaJIT constructor observations, separate from native admission.
#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::{Function, Lua, MultiValue, Table, Value};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};
const PATH: &str = "tests/support/source_program_constructor_semantics.lua";
const TEXT: &str = include_str!("support/source_program_constructor_semantics.lua");
fn scalar(value: Value) -> Json {
    match value {
        Value::Nil => Json::Null,
        Value::Boolean(v) => json!(v),
        Value::Integer(v) => json!(v),
        Value::Number(v) => json!(v),
        Value::String(v) => json!(v.to_str().unwrap().as_ref()),
        Value::Table(v) => state(&v),
        other => panic!("unsupported probe value {other:?}"),
    }
}
fn state(table: &Table) -> Json {
    let entries: Vec<_> = table
        .clone()
        .pairs::<Value, Value>()
        .map(|pair| {
            let (k, v) = pair.unwrap();
            json!([scalar(k), scalar(v)])
        })
        .collect();
    json!({"raw_len":table.raw_len(),"entries":entries})
}
fn host() -> (Lua, Table) {
    let lua = unsafe { Lua::unsafe_new() };
    lua.load("jit.off();jit.flush();assert(not jit.status())")
        .exec()
        .unwrap();
    let api: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    (lua, api)
}
fn input(lua: &Lua, n: usize, pattern: usize) -> Table {
    let table = lua.create_table().unwrap();
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
fn save(name: &str, data: Json) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../runs")
        .join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
}
#[test]
fn original_list_allocations_tail_packs_and_numeric_traversal() {
    let (lua, api) = host();
    let utility: Table = lua.load("return require('jit.util')").eval().unwrap();
    let bc: Function = utility.raw_get("funcbc").unwrap();
    let info: Function = utility.raw_get("funcinfo").unwrap();
    let mut words = BTreeMap::new();
    for (name, function) in api.clone().pairs::<String, Function>().map(Result::unwrap) {
        let metadata: Table = info.call(function.clone()).unwrap();
        let count: u32 = metadata.raw_get("bytecodes").unwrap();
        let mut selected = vec![];
        for pc in 1..count {
            let values: MultiValue = bc.call((function.clone(), pc)).unwrap();
            let word = match values[0] {
                Value::Integer(v) => v as u32,
                Value::Number(v) => v as i32 as u32,
                ref v => panic!("word{v:?}"),
            };
            if matches!(word & 255, 52 | 53 | 63) {
                selected.push(json!({"pc":pc,"word":word,"opcode":word&255,"d":word>>16}));
            }
        }
        words.insert(name, selected);
    }
    let mut cases = vec![];
    for n in [0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256] {
        for pattern in 0..5 {
            let source = input(&lua, n, pattern);
            for name in ["tail", "prefix", "prefix_two", "single_tail", "separator"] {
                let mut args = vec![];
                if name != "tail" {
                    args.push(Value::Integer(91));
                }
                if name == "prefix_two" {
                    args.push(Value::Nil);
                }
                args.push(Value::Table(source.clone()));
                let result: Table = api
                    .raw_get::<Function>(name)
                    .unwrap()
                    .call(MultiValue::from_vec(args))
                    .unwrap();
                let expected_n = match name {
                    "tail" => n,
                    "prefix_two" => n + 2,
                    "single_tail" => 2,
                    _ => n + 1,
                };
                let prefix = usize::from(name != "tail") + usize::from(name == "prefix_two");
                for i in 1..=expected_n {
                    let expected = if i == 1 && prefix > 0 {
                        Value::Integer(91)
                    } else if i == 2 && prefix == 2 {
                        Value::Nil
                    } else {
                        source.raw_get(i - prefix).unwrap()
                    };
                    assert_eq!(
                        result.raw_get::<Value>(i).unwrap(),
                        expected,
                        "{name}/{n}/{pattern}/{i}"
                    );
                }
                assert_eq!(result.raw_get::<Value>(expected_n + 1).unwrap(), Value::Nil);
                let keys: Vec<usize> = result
                    .clone()
                    .pairs::<usize, Value>()
                    .map(|r| r.unwrap().0)
                    .collect();
                assert!(
                    keys.windows(2).all(|pair| pair[0] < pair[1]),
                    "physical array traversal {name}/{n}/{pattern}: {keys:?}"
                );
                cases.push(json!({"constructor":name,"tail_results":n,"pattern":pattern,"state":state(&result)}));
            }
        }
    }
    assert_eq!(cases.len(), 400);
    save(
        "r2q-source-constructor-lists.json",
        json!({"mode":"pinned LuaJIT interpreter; source-only, no native admission","source":PATH,"source_sha256":format!("{:x}",Sha256::digest(TEXT.as_bytes())),"bytecode":words,"cases":cases}),
    );
}
#[test]
fn original_list_operand_effects_and_failure_prefixes() {
    let (lua, api) = host();
    let mut cases = vec![];
    for name in ["effect", "effect_single", "failed_tail"] {
        let source = lua.create_table().unwrap();
        source.raw_set("initial", 2).unwrap();
        source.raw_set("changed", 10).unwrap();
        source.raw_set("right", 7).unwrap();
        source.raw_set("first", 9).unwrap();
        source.raw_set("effects", 0).unwrap();
        let result = api
            .raw_get::<Function>(name)
            .unwrap()
            .call::<Table>(source.clone());
        assert_eq!(source.raw_get::<usize>("effects").unwrap(), 1);
        let outcome = if name == "failed_tail" {
            assert!(result.is_err());
            json!({"source_error":true})
        } else {
            let table = result.unwrap();
            assert_eq!(table.raw_get::<i32>(1).unwrap(), 2);
            assert_eq!(table.raw_get::<i32>(2).unwrap(), 7);
            assert_eq!(table.raw_get::<Value>(3).unwrap(), Value::Nil);
            assert_eq!(
                table.raw_get::<Value>(4).unwrap(),
                if name == "effect" {
                    Value::Boolean(false)
                } else {
                    Value::Nil
                }
            );
            state(&table)
        };
        cases.push(json!({"name":name,"effects":1,"outcome":outcome}));
    }
    save(
        "r2q-source-constructor-effects.json",
        json!({"mode":"pinned LuaJIT interpreter; source-only, no native admission","cases":cases}),
    );
}
#[test]
fn original_constructor_target_traces_record_tail_allocation_differences() {
    let (lua, api) = host();
    let driver = warm::SourceWarmDriver::new(&lua).unwrap();
    let mut cases = vec![];
    for name in ["tail", "prefix", "prefix_two", "single_tail", "separator"] {
        for (n, pattern) in [(0, 0), (2, 1), (7, 3), (32, 2), (128, 0)] {
            let mut args = vec![];
            if name != "tail" {
                args.push(Value::Integer(91));
            }
            if name == "prefix_two" {
                args.push(Value::Nil);
            }
            args.push(Value::Table(input(&lua, n, pattern)));
            lua.load("jit.off();jit.flush()").exec().unwrap();
            let function: Function = api.raw_get(name).unwrap();
            let cold: Table = function.call(MultiValue::from_vec(args.clone())).unwrap();
            let result = driver.run(&lua, &function, &args, None).unwrap();
            assert!(result.success);
            assert_eq!(result.calls, 128);
            assert_eq!(result.seed_calls, 0);
            let Value::Table(table) = result.value else {
                panic!("returned table")
            };
            let cold_state = state(&cold);
            let warm_state = state(&table);
            assert_eq!(
                cold_state["entries"], warm_state["entries"],
                "same values/order {name}/{n}/{pattern}"
            );
            let next: Function = lua.globals().raw_get("next").unwrap();
            let mut controls = vec![];
            for key in 0..=(n + 12) {
                let read = |value: &Table| match next.call::<MultiValue>((value.clone(), key)) {
                    Ok(values) => {
                        json!({"pack":values.into_vec().into_iter().map(scalar).collect::<Vec<_>>()})
                    }
                    Err(error) => {
                        assert!(
                            error.to_string().contains("invalid key to 'next'"),
                            "{error}"
                        );
                        json!({"source_error":true})
                    }
                };
                controls.push(json!({"control":key,"cold":read(&cold),"warm":read(&table)}));
            }
            let unpack: Function = lua.globals().raw_get("unpack").unwrap();
            let cold_pack: MultiValue = unpack.call(cold.clone()).unwrap();
            let warm_pack: MultiValue = unpack.call(table.clone()).unwrap();
            if name == "prefix_two" && n == 2 && pattern == 1 {
                assert_eq!(cold_state["raw_len"], 1);
                assert_eq!(warm_state["raw_len"], 4);
                assert_eq!(cold_pack.len(), 1);
                assert_eq!(warm_pack.len(), 4);
            }
            cases.push(json!({"constructor":name,"tail_results":n,"pattern":pattern,"cold":cold_state,"warm":warm_state,"next_controls":controls,"cold_unpack":cold_pack.into_vec().into_iter().map(scalar).collect::<Vec<_>>(),"warm_unpack":warm_pack.into_vec().into_iter().map(scalar).collect::<Vec<_>>(),"target_traces":result.target_live_traces,"calls":result.calls}));
        }
    }
    assert_eq!(cases.len(), 25);
    save(
        "r2q-source-constructor-warm.json",
        json!({"mode":"pinned LuaJIT exact original constructor; source-only, no native admission","cases":cases}),
    );
}

#[test]
fn original_fixed_list_mutations_record_warmed_layout_uncertainty() {
    let (lua, api) = host();
    let driver = warm::SourceWarmDriver::new(&lua).unwrap();
    let function: Function = api.raw_get("mutate_pair").unwrap();
    let next: Function = lua.globals().raw_get("next").unwrap();
    let mut cases = vec![];
    for key in [0, 1, 2, 3, 4, 16] {
        for value in [Value::Nil, Value::Boolean(false), Value::Integer(7)] {
            let args = [
                Value::Nil,
                Value::Boolean(false),
                Value::Integer(key),
                value.clone(),
            ];
            lua.load("jit.off();jit.flush()").exec().unwrap();
            let cold: Table = function.call(MultiValue::from_vec(args.to_vec())).unwrap();
            let result = driver.run(&lua, &function, &args, None).unwrap();
            assert!(result.success);
            assert_eq!(result.calls, 128);
            assert_eq!(result.seed_calls, 0);
            let Value::Table(table) = result.value else {
                panic!("returned table")
            };
            // Values are compared by key, independently of physical traversal order.
            for index in 0..=20 {
                assert_eq!(
                    cold.raw_get::<Value>(index).unwrap(),
                    table.raw_get::<Value>(index).unwrap()
                );
            }
            let mut controls = vec![];
            for control in 0..=20 {
                let read = |value: &Table| match next.call::<MultiValue>((value.clone(), control)) {
                    Ok(values) => {
                        json!({"pack":values.into_vec().into_iter().map(scalar).collect::<Vec<_>>()})
                    }
                    Err(error) => {
                        assert!(
                            error.to_string().contains("invalid key to 'next'"),
                            "{error}"
                        );
                        json!({"source_error":true})
                    }
                };
                controls.push(json!({"control":control,"cold":read(&cold),"warm":read(&table)}));
            }
            cases.push(json!({"key":key,"value":scalar(value),"cold":state(&cold),"warm":state(&table),"next_controls":controls,"target_traces":result.target_live_traces,"calls":result.calls}));
        }
    }
    assert_eq!(cases.len(), 18);
    save(
        "r2q-source-constructor-mutations.json",
        json!({"mode":"pinned LuaJIT original fixed-list constructor and subsequent store; source-only, no native admission","cases":cases}),
    );
}
