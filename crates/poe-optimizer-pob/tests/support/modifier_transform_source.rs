//! Original modifier-magnitude producer discovery, eligibility and order.
//! Source observations only: no canonical admission, native parity or closure authority.
use super::{FormatterOracle, Function, Json, Table, Value, json};
use mlua::LuaSerdeExt;
use sha2::{Digest, Sha256};
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

fn assert_source_metadata(oracle: &FormatterOracle, metadata: &Json) {
    assert_eq!(
        metadata["source_manifest_sha256"].as_str().unwrap(),
        poe_optimizer_pob::source::manifest_sha256()
    );
    assert_eq!(
        metadata["source_revision"].as_str().unwrap(),
        poe_optimizer_pob::source::UPSTREAM_REVISION
    );
    for pin in metadata["source_files"].as_array().unwrap() {
        let name = pin["path"].as_str().unwrap();
        assert_eq!(
            pin["sha256"].as_str().unwrap(),
            poe_optimizer_pob::source::expected_file_sha256(name).unwrap()
        );
    }
    let data: Table = oracle.source.lua.globals().get("data").unwrap();
    let scalability: Table = data.get("modScalability").unwrap();
    let rows = metadata["scalability"].as_array().unwrap();
    assert_eq!(rows.len(), 47);
    for row in rows {
        let name = row["template"].as_str().unwrap();
        let actual: Table = scalability.get(name).unwrap();
        let actual = actual
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        let expected = row["components"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len(), "{name}");
        for (actual, expected) in actual.iter().zip(expected) {
            assert_eq!(
                Some(actual.get::<bool>("isScalable").unwrap()),
                expected["is_scalable"].as_bool(),
                "{name}"
            );
            let formats: Option<Table> = actual.get("formats").unwrap();
            let formats = formats.map(|v| {
                v.sequence_values::<String>()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
            });
            assert_eq!(json!(formats), expected["formats"], "{name}");
        }
    }
    let item_mods: Table = data.get("itemMods").unwrap();
    let rows = metadata["catalog_modifiers"].as_array().unwrap();
    assert_eq!(rows.len(), 15);
    for row in rows {
        let table: Table = item_mods.get(row["table"].as_str().unwrap()).unwrap();
        let actual: Table = table.get(row["id"].as_str().unwrap()).unwrap();
        let lines = actual
            .clone()
            .sequence_values::<String>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(json!(lines), row["lines"], "{} lines", row["id"]);
        for key in ["type", "group"] {
            assert_eq!(
                json!(actual.get::<Option<String>>(key).unwrap()),
                row[key],
                "{} {key}",
                row["id"]
            );
        }
        assert_eq!(
            actual.get::<Option<f64>>("level").unwrap(),
            row["level"].as_f64()
        );
        let tags: Table = actual.get("modTags").unwrap();
        assert_eq!(
            json!(
                tags.sequence_values::<String>()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
            ),
            row["mod_tags"]
        );
        let order: Table = actual.get("statOrder").unwrap();
        let order = order
            .sequence_values::<i64>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(json!(order), row["stat_order"]);
    }
}

fn check(case: &Json, report: &Json) {
    assert_eq!(report["original_functions_preserved"], true);
    let state = &report["constructed"];
    for key in ["rebuilt", "rebuilt_twice", "direct_parse"] {
        assert_eq!(state, &report[key], "{} {key}", case["id"]);
    }
    let expected = &case["expected"];
    if let Some(capacity) = expected.get("capacity") {
        assert_eq!(&state["capacity"], capacity, "{}", case["id"]);
    } else {
        assert_eq!(case["scope"], "source-spelling-diagnostic");
    }
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
        ("producer_tags", "tags"),
        ("producer_any_tags", "any_tags"),
        ("producer_mod_types", "mod_type"),
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
    for (field, property) in [
        ("producer_value_scalars", "value_scalar"),
        ("producer_corrupted_ranges", "corrupted_range"),
    ] {
        if let Some(expected) = expected.get(field) {
            assert_eq!(
                json!(
                    producers
                        .iter()
                        .map(|p| p["source_line"][property].clone())
                        .collect::<Vec<_>>()
                ),
                *expected,
                "{} {field}",
                case["id"]
            );
        }
    }
    if let Some(text) = expected.get("unknown_line_contains") {
        assert!(
            state["lines"]
                .as_object()
                .unwrap()
                .values()
                .flat_map(|v| rows(v).iter())
                .any(|r| r["line"]
                    .as_str()
                    .is_some_and(|s| s.contains(text.as_str().unwrap()))
                    && r["extra"].as_str().is_some_and(|s| !s.is_empty())),
            "{} unknown source line",
            case["id"]
        );
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
    assert_eq!(cases["cases"].as_array().unwrap().len(), 56);
    let metadata_text =
        include_str!("modifier_transform_source_metadata.json").replace("\r\n", "\n");
    let metadata: Json = serde_json::from_str(&metadata_text).unwrap();
    let metadata_digest = format!("{:x}", Sha256::digest(metadata_text.as_bytes()));
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
            assert_source_metadata(&oracle, &metadata);
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
            let report = json!({"jit":jit,"replay":replay,"source_manifest_sha256":poe_optimizer_pob::source::manifest_sha256(),"catalog_metadata_sha256":metadata_digest,"cases":observed});
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
        "status":"source_controls_passed","fresh_replays":4,"cases_per_replay":56,
        "catalog_metadata_sha256":metadata_digest,"scalability_rows":47,"upstream_modifier_ids":15,
        "source_manifest_sha256":poe_optimizer_pob::source::manifest_sha256(),
        "source_admission_authority":false,"closure_authority":false,"native_parity":false,
        "full_build_evaluation":false,"original_functions_preserved":true,
        "scope":"Original magnitude producer discovery, eligibility, arithmetic kind, encoding and order; diagnostics are not adopted native semantics"
    })).unwrap()).unwrap();
    println!("Modifier transform source controls: {}", output.display());
}
