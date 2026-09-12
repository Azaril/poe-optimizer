//! Pinned original-source contracts for lexical cells and active-local operand timing.
//! These source-only probes do not themselves grant native factory admission.
use mlua::{Function, Lua, MultiValue, Table, Value};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};
const PATH: &str = "tests/support/source_program_factory_semantics.lua";
const TEXT: &str = include_str!("support/source_program_factory_semantics.lua");
fn pack(values: MultiValue) -> Json {
    Json::Array(
        values
            .into_iter()
            .map(|v| match v {
                Value::Nil => Json::Null,
                Value::Boolean(v) => json!(v),
                Value::Integer(v) => json!(v),
                Value::Number(v) if v.is_finite() && v.fract() == 0.0 => json!(v as i64),
                Value::Number(v) if v.is_finite() => json!(v),
                Value::String(v) => json!(v.to_str().unwrap().as_ref()),
                other => panic!("unexpected source contract result: {other:?}"),
            })
            .collect(),
    )
}
fn source() -> Lua {
    let lua = Lua::new();
    lua.load("jit.off();jit.flush();assert(not jit.status())")
        .exec()
        .unwrap();
    lua
}
fn save(name: &str, evidence: &Json) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::create_dir_all(root.join("runs")).unwrap();
    fs::write(
        root.join("runs").join(name),
        serde_json::to_vec_pretty(evidence).unwrap(),
    )
    .unwrap();
}
#[test]
fn original_lexical_cells_survive_scope_loop_return_and_failure() {
    let lua = source();
    let probes: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let expected = BTreeMap::from([
        ("siblings", json!([1, 7])),
        ("independent_factories", json!([false, true, 7, 9])),
        ("forwarded_capture", json!([12, 12])),
        ("shadowing_and_initializer", json!([9, 9])),
        ("recursive_local", json!([120])),
        ("recursive_self_identity", json!([true, true])),
        ("loop_numeric_visible", json!([11, 12, 13, 3, 3, 3])),
        ("loop_generic_visible", json!([1, 2, 3, 14, 15, 16])),
        ("loop_body_declarations", json!([1, 20, 3])),
        ("loop_while_declarations", json!([1, 2, 3, 3])),
        ("loop_repeat_declarations", json!([1, 2, 3])),
        ("break_closes_visible_binding", json!([2, 10])),
        ("return_closes_visible_binding", json!([2, 4])),
        ("error_keeps_escaped_cells", json!([false, true, 9, 11])),
        ("branch_scopes", json!([3, 7])),
        ("function_identity_keys", json!([false, 7, 11, 1, 1])),
        ("local_capture_reassignment", json!([12, 10])),
        ("outer_capture_is_eager", json!([4, 10])),
        (
            "captured_nil_false_packs",
            json!([null, false, null, false, false, null]),
        ),
    ]);
    let mut actual = BTreeMap::new();
    for row in probes.pairs::<String, Function>() {
        let (name, function) = row.unwrap();
        let values = pack(function.call(()).unwrap());
        assert_eq!(
            Some(&values),
            expected.get(name.as_str()),
            "source cell contract {name}"
        );
        actual.insert(name, values);
    }
    assert_eq!(actual.len(), expected.len());
    save(
        "r2p-source-factory-cells.json",
        &json!({"mode":"pinned LuaJIT interpreter; source-only, no native admission","source":PATH,"source_sha256":format!("{:x}",Sha256::digest(TEXT.as_bytes())),"cases":actual}),
    );
}
#[test]
fn original_active_local_reads_follow_operator_and_operand_position() {
    let lua = source();
    // Each source body creates an actual nested mutator of its own current-frame
    // local. No debug hooks or host value substitution supplies the mutation.
    let cases = [
        ("x+change()", json!([12, 10])),
        ("(x+0)+change()", json!([4, 10])),
        ("x-change()", json!([8, 10])),
        ("x*change()", json!([20, 10])),
        ("x/change()", json!([5, 10])),
        ("x%change()", json!([0, 10])),
        ("x^change()", json!([100, 10])),
        ("x<change()", json!([false, 10])),
        ("x<=change()", json!([false, 10])),
        ("x>change()", json!([true, 10])),
        ("x>=change()", json!([true, 10])),
        ("x==change()", json!([false, 10])),
        ("x~=change()", json!([true, 10])),
        ("x..change()", json!(["22", 10])),
        ("x+(change()+1)", json!([13, 10])),
        ("(true and x)+change()", json!([12, 10])),
        ("(x and x)+change()", json!([4, 10])),
        ("x,change()", json!([2, 2, 10])),
        ("id(x,change())", json!([2, 10])),
        ("({x,change()})[1]", json!([2, 10])),
        ("({[x]=change()})[2]", json!([null, 10])),
        ("({[x]=change()})[10]", json!([2, 10])),
        ("({value=x,other=change()}).value", json!([2, 10])),
        ("x+(-change())", json!([8, 10])),
        ("x..(change()..x)", json!(["2210", 10])),
    ];
    let mut actual = BTreeMap::new();
    for (expression, expected) in cases {
        let text = format!(
            "return function()\nlocal x=2\nlocal function change() x=10;return 2 end\nlocal function id(a,b)return a,b end\nreturn {expression},x\nend\n"
        );
        let function: Function = lua
            .load(&text)
            .set_name("@tests/generated_factory_operands.lua")
            .eval()
            .unwrap();
        let values = pack(function.call(()).unwrap());
        assert_eq!(values, expected, "source operand contract {expression}");
        actual.insert(
            expression,
            json!({"source_sha256":format!("{:x}",Sha256::digest(text.as_bytes())),"pack":values}),
        );
    }
    let call_cases = [
        (
            "function target",
            "local f=function()return 1 end;local function change()f=function()return 2 end end;return f(change()),f()",
        ),
        (
            "method receiver",
            "local t={value=1};function t:get()return self.value end;local function change()t={value=2}end;return t:get(change()),t.value",
        ),
    ];
    for (name, text) in call_cases {
        let values = pack(lua.load(text).eval().unwrap());
        assert_eq!(values, json!([1, 2]), "source call contract {name}");
        actual.insert(
            name,
            json!({"source_sha256":format!("{:x}",Sha256::digest(text.as_bytes())),"pack":values}),
        );
    }
    assert_eq!(actual.len(), 27);
    save(
        "r2p-source-factory-operands.json",
        &json!({"mode":"pinned LuaJIT interpreter; source-only, no native admission","compiler_contract":"lj_parse.c bcemit_binop_left/bcemit_arith/bcemit_comp/expr_table/expr_list/parse_args","cases":actual}),
    );
}

#[test]
fn original_binary_failure_order_retains_actual_cell_writes() {
    let lua = source();
    let cases = [
        (
            "local becomes numeric",
            "'bad'",
            "10",
            "x+change()",
            Some(json!([12])),
            1,
            json!([10]),
            None,
        ),
        (
            "local becomes invalid",
            "2",
            "'bad'",
            "x+change()",
            None,
            1,
            json!(["bad"]),
            Some("arithmetic"),
        ),
        (
            "computed left fails first",
            "'bad'",
            "10",
            "(x+0)+change()",
            None,
            0,
            json!(["bad"]),
            Some("arithmetic"),
        ),
        (
            "computed left retained",
            "2",
            "'bad'",
            "(x+0)+change()",
            Some(json!([4])),
            1,
            json!(["bad"]),
            None,
        ),
        (
            "concat retains left",
            "'bad'",
            "10",
            "x..change()",
            Some(json!(["bad2"])),
            1,
            json!([10]),
            None,
        ),
        (
            "comparison sees invalid new value",
            "2",
            "'bad'",
            "x<change()",
            None,
            1,
            json!(["bad"]),
            Some("compare"),
        ),
    ];
    let mut rows = BTreeMap::new();
    for (name, initial, changed, expression, expected, effects, latest, error_marker) in cases {
        let text = format!(
            "return function(state)\nlocal x={initial}\nstate.get=function()return x end\nlocal function change()x={changed};state.effects=state.effects+1;return 2 end\nreturn {expression}\nend\n"
        );
        let function: Function = lua
            .load(&text)
            .set_name("@tests/generated_factory_errors.lua")
            .eval()
            .unwrap();
        let state = lua.create_table().unwrap();
        state.raw_set("effects", 0).unwrap();
        let result = function.call::<MultiValue>(state.clone());
        let recorded = match (result, expected, error_marker) {
            (Ok(result), Some(expected), None) => {
                let result = pack(result);
                assert_eq!(result, expected, "source error contract {name}");
                json!({"pack":result})
            }
            (Err(error), None, Some(marker)) => {
                assert!(
                    error.to_string().contains(marker),
                    "source error contract {name}: {error}"
                );
                json!({"error_family":marker})
            }
            (actual, expected, _) => {
                panic!("source error contract {name}: {actual:?}; expected {expected:?}")
            }
        };
        assert_eq!(state.raw_get::<i32>("effects").unwrap(), effects, "{name}");
        let getter: Function = state.raw_get("get").unwrap();
        let current = pack(getter.call(()).unwrap());
        assert_eq!(current, latest, "escaped source cell {name}");
        rows.insert(name,json!({"source_sha256":format!("{:x}",Sha256::digest(text.as_bytes())),"result":recorded,"effects":effects,"escaped_cell":current}));
    }
    save(
        "r2p-source-factory-errors.json",
        &json!({"mode":"pinned LuaJIT interpreter; source-only, no native admission","cases":rows}),
    );
}
