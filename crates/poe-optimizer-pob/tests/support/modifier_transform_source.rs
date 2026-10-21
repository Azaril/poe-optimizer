//! Original modifier-magnitude producer discovery, eligibility and order.
//! Source observations only: no canonical admission, native parity or closure authority.
use super::{FormatterOracle, Function, Json, Table, Value, json};
use mlua::LuaSerdeExt;
use std::{fs, path::PathBuf};

fn source_handles(oracle: &FormatterOracle) -> Table {
    let lua = &oracle.source.lua;
    let methods: Table = lua.globals().get("item_loading_functions").unwrap();
    let handles = lua.create_table().unwrap();
    for (name, method, path, first) in [
        ("parse", "parse_raw", "@src/Classes/Item.lua", 468),
        ("build", "build_mod_list", "@src/Classes/Item.lua", 2694),
        ("constructor", "constructor", "@src/Modules/Common.lua", 167),
        ("check", "check_variant", "@src/Classes/Item.lua", 2289),
        ("count", "variant_count", "@src/Classes/Item.lua", 2320),
    ] {
        let function: Function = methods.get(method).unwrap();
        let info = function.info();
        assert_eq!(info.what, "Lua", "{name}");
        assert_eq!(info.source.as_deref(), Some(path), "{name}");
        assert_eq!(info.line_defined, Some(first), "{name}");
        handles.set(name, function).unwrap();
    }
    assert!(matches!(
        lua.globals().get::<Value>("debug").unwrap(),
        Value::Nil
    ));
    handles
}

fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}

fn check(case: &Json, report: &Json) {
    assert_eq!(report["original_functions_preserved"], true);
    let state = &report["constructed"];
    for key in ["rebuilt", "rebuilt_twice", "direct_parse"] {
        assert_eq!(state, &report[key], "{} {key}", case["id"]);
    }
    let expected = &case["expected"];
    assert_eq!(state["capacity"], expected["capacity"], "{}", case["id"]);
    let life: f64 = rows(&state["active_records"])
        .iter()
        .filter(|r| r["name"] == "Life" && r["type"] == "BASE")
        .map(|r| r["value"].as_f64().unwrap())
        .sum();
    assert_eq!(Some(life), expected["life"].as_f64(), "{}", case["id"]);
    let producers = rows(&state["producers"]);
    assert_eq!(
        producers.len() as u64,
        expected["producer_count"].as_u64().unwrap(),
        "{}",
        case["id"]
    );
    for (i, producer) in producers.iter().enumerate() {
        assert_eq!(producer["discovery_index"], (i + 1) as u64);
        assert!(
            producer["source_address"].is_object(),
            "{} exact source occurrence",
            case["id"]
        );
        let category = producer["source_address"]["category"].as_str().unwrap();
        let index = producer["source_address"]["index"].as_u64().unwrap() as usize;
        assert_eq!(producer["source_line"], state["lines"][category][index - 1]);
    }
    for (field, projection) in [
        ("producer_quality", "quality"),
        ("producer_operations", "operation"),
    ] {
        if let Some(expected) = expected.get(field) {
            assert_eq!(
                json!(
                    producers
                        .iter()
                        .map(|p| p[projection].clone())
                        .collect::<Vec<_>>()
                ),
                *expected,
                "{} {field}",
                case["id"]
            );
        }
    }
    for (field, property) in [
        ("source_storage_indices", "index"),
        ("source_storage_categories", "category"),
    ] {
        if let Some(expected) = expected.get(field) {
            assert_eq!(
                json!(
                    producers
                        .iter()
                        .map(|p| p["source_address"][property].clone())
                        .collect::<Vec<_>>()
                ),
                *expected,
                "{} {field}",
                case["id"]
            );
        }
    }
    if let Some(expected) = expected.get("producer_active") {
        assert_eq!(
            json!(
                producers
                    .iter()
                    .map(|p| p["source_line"]["active"].clone())
                    .collect::<Vec<_>>()
            ),
            *expected,
            "{} active",
            case["id"]
        );
    }
}

#[test]
fn original_modifier_transform_sources_preserve_discovery_order_and_recipient_eligibility() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::create_dir_all(root.join("runs")).unwrap();
    let output = tempfile::Builder::new()
        .prefix("modifier-transform-source-")
        .tempdir_in(root.join("runs"))
        .unwrap()
        .keep();
    let cases: Json =
        serde_json::from_str(include_str!("modifier_transform_source_cases.json")).unwrap();
    assert_eq!(cases["cases"].as_array().unwrap().len(), 31);
    let mut reports = Vec::new();
    for jit in [false, true] {
        for replay in 0..2 {
            let oracle = FormatterOracle::new();
            oracle
                .source
                .lua
                .load(if jit {
                    "jit.on()"
                } else {
                    "jit.off();jit.flush()"
                })
                .exec()
                .unwrap();
            let handles = source_handles(&oracle);
            let observe = oracle
                .source
                .lua
                .load(include_str!("modifier_transform_source_observer.lua"))
                .set_name("@test-only-original-transform-sources")
                .into_function()
                .unwrap();
            let mut observed = Vec::new();
            for case in cases["cases"].as_array().unwrap() {
                let result:Table=observe.call((case["raw"].as_str().unwrap(),handles.clone())).unwrap_or_else(|error| {
                    fs::write(output.join(format!("jit-{jit}-replay-{replay}-source-failure.json")),
                        serde_json::to_vec_pretty(&json!({"case":case,"error":error.to_string(),"completed":observed})).unwrap()).unwrap();
                    panic!("source control failed; evidence at {}: {error}",output.display());
                });
                let state: Json = oracle.source.lua.from_value(Value::Table(result)).unwrap();
                observed.push(json!({"case":case,"source":state}));
            }
            let report = json!({"jit":jit,"replay":replay,"source_manifest_sha256":poe_optimizer_pob::source::manifest_sha256(),"cases":observed});
            fs::write(
                output.join(format!("jit-{jit}-replay-{replay}.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            reports.push(report);
        }
    }
    for report in &reports[1..] {
        assert_eq!(reports[0]["cases"], report["cases"]);
    }
    for row in reports[0]["cases"].as_array().unwrap() {
        check(&row["case"], &row["source"]);
    }
    fs::write(output.join("summary.json"),serde_json::to_vec_pretty(&json!({
        "status":"source_controls_passed","fresh_replays":4,"cases_per_replay":31,
        "source_manifest_sha256":poe_optimizer_pob::source::manifest_sha256(),
        "source_admission_authority":false,"closure_authority":false,"native_parity":false,
        "full_build_evaluation":false,"original_functions_preserved":true,
        "scope":"Original magnitude producer discovery, eligibility, arithmetic kind, encoding and order; diagnostics are not adopted native semantics"
    })).unwrap()).unwrap();
    println!("Modifier transform source controls: {}", output.display());
}
