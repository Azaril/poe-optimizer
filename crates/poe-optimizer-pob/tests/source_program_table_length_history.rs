//! Independent original LuaJIT history probe; no native table implementation used.
#[path = "support/source_program_warm.rs"]
mod source_program_warm;
use mlua::{Function, Lua, Table, Value};
use source_program_warm::SourceWarmDriver;
use std::{fs, path::Path};

fn output(lua: &Lua) -> Table {
    let out = lua.create_table().unwrap();
    out.raw_set("n", 0).unwrap();
    out
}
fn rows(table: &Table, kind: &str) -> serde_json::Value {
    let count: usize = table.raw_get("n").unwrap();
    let rows: Vec<_> = (1..=count)
        .map(|i| {
            if kind == "length" {
                serde_json::json!(table.raw_get::<usize>(i).unwrap())
            } else {
                let pack: Table = table.raw_get(i).unwrap();
                let n: usize = pack.raw_get("n").unwrap();
                let values: Vec<_> = (1..=n)
                    .map(|i| match pack.raw_get::<Value>(i).unwrap() {
                        Value::Nil => serde_json::Value::Null,
                        Value::String(v) => serde_json::json!(v.to_str().unwrap().as_ref()),
                        Value::Boolean(v) => serde_json::json!(v),
                        other => panic!("unexpected result {other:?}"),
                    })
                    .collect();
                serde_json::json!({"n":n,"values":values})
            }
        })
        .collect();
    serde_json::json!(rows)
}
#[test]
fn original_sparse_length_and_unpack_are_checked_after_exact_function_warm_histories() {
    let lua = unsafe { Lua::unsafe_new() };
    let api: Table = lua
        .load(include_str!(
            "support/source_program_table_length_history.lua"
        ))
        .set_name("@tests/support/source_program_table_length_history.lua")
        .eval()
        .unwrap();
    let make: Function = api.raw_get("make").unwrap();
    let warm = SourceWarmDriver::new(&lua).unwrap();
    let mut evidence = vec![];
    for kind in ["length", "unpack"] {
        let function: Function = api.raw_get(kind).unwrap();
        for (seed, target) in [
            ("zero_only", "zero_and_two"),
            ("reserved_two_empty", "only_two"),
            ("three_dense", "three_hole_two"),
            ("three_hole_two", "three_dense"),
        ] {
            lua.load("require('jit').off(); require('jit').flush()")
                .exec()
                .unwrap();
            let target_table: Table = make.call(target).unwrap();
            let cold = output(&lua);
            function
                .call::<Table>((target_table.clone(), cold.clone()))
                .unwrap();
            let seed_table: Table = make.call(seed).unwrap();
            let seed_output = output(&lua);
            let warm_output = output(&lua);
            let result = warm
                .run(
                    &lua,
                    &function,
                    &[
                        Value::Table(target_table),
                        Value::Table(warm_output.clone()),
                    ],
                    Some(&[Value::Table(seed_table), Value::Table(seed_output.clone())]),
                )
                .unwrap();
            assert!(result.success);
            assert_eq!(result.calls, 128);
            assert_eq!(result.seed_calls, 128);
            assert!(
                matches!(&result.value,Value::Table(value) if value.to_pointer()==warm_output.to_pointer())
            );
            assert_eq!(warm_output.raw_get::<usize>("n").unwrap(), 128);
            assert_eq!(seed_output.raw_get::<usize>("n").unwrap(), 128);
            // This is evidence for these observed histories, not an admission of
            // every ALEN hint. Native ambiguous-boundary rejection stays separate.
            let cold_rows = rows(&cold, kind);
            assert!(
                rows(&warm_output, kind)
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|value| value == &cold_rows[0])
            );
            evidence.push(serde_json::json!({"operation":kind,"seed":seed,"target":target,"cold":rows(&cold,kind),"seed_rows":rows(&seed_output,kind),"warm_rows":rows(&warm_output,kind),"target_live_traces":result.target_live_traces,"live_traces":result.live_traces}));
        }
    }
    let function: Function = api.raw_get("nil_transition").unwrap();
    lua.load("require('jit').off(); require('jit').flush()")
        .exec()
        .unwrap();
    let cold = output(&lua);
    function.call::<Table>((Value::Nil, cold.clone())).unwrap();
    let warmed = output(&lua);
    let result = warm
        .run(
            &lua,
            &function,
            &[Value::Nil, Value::Table(warmed.clone())],
            None,
        )
        .unwrap();
    assert!(result.success);
    assert_eq!(warmed.raw_get::<usize>("n").unwrap(), 128);
    assert!(
        rows(&warmed, "unpack")
            .as_array()
            .unwrap()
            .iter()
            .all(|value| value == &rows(&cold, "unpack")[0])
    );
    evidence.push(serde_json::json!({"operation":"nil_transition","seed":null,"target":"empty_nil_at_two_then_next_one","cold":rows(&cold,"unpack"),"seed_rows":[],"warm_rows":rows(&warmed,"unpack"),"target_live_traces":result.target_live_traces,"live_traces":result.live_traces}));
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runs/r2m-length-history.json");
    fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}
