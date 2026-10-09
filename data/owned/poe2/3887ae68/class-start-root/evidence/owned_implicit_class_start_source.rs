//! Original constructor/default-inventory evidence for implicit root 54447.
//! This does not close Class, universal Player, or externally transformed behavior.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const OBSERVE: &str = include_str!("support/implicit_class_start_source.lua");
const TEST: &str = "actual_implicit_class_start_preserves_default_inventory_and_shared_root";
const CHILD: &str = "POE_IMPLICIT_CLASS_START_SOURCE_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_IMPLICIT_CLASS_START_SOURCE_OUT";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(array) = value.as_array() {
        array
    } else {
        assert!(value.as_object().is_some_and(|x| x.is_empty()));
        &[]
    }
}
fn observed(root: &Path, xml: &str, enabled: bool, execute: bool) -> Json {
    let before = |lua: &Lua| {
        lua.load(if enabled {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()?;
        Ok(())
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let observer: Function = lua
            .load(OBSERVE)
            .set_name("@implicit_class_start_source.lua")
            .eval()?;
        let result: Table = observer.call((execute, enabled))?;
        Ok(lua.from_value(Value::Table(result))?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        None,
        Some(&after),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    json!({"source_hash":report["source_hash"],"selected":report["selected"],
        "state":report["additional_observation"]})
}
fn without_probe(mut host: Json) -> Json {
    let state = host["state"].as_object_mut().unwrap();
    state.remove("executed");
    state.remove("constructor_probe");
    host
}
fn check(host: &Json, executed: bool) {
    let s = &host["state"];
    assert_eq!(s["executed"], executed);
    assert_eq!(s["raw"]["skill"], 54447);
    assert_eq!(s["raw"]["classesStart"], json!(["Witch", "Sorceress"]));
    assert!(rows(&s["raw"]["stats"]).is_empty());
    assert_eq!(
        s["methods"]["constructor"]["path"],
        "Classes/PassiveTree.lua"
    );
    assert_eq!(s["methods"]["constructor"]["first"], 59);
    assert_eq!(s["methods"]["constructor_wrapper"]["first"], 167);
    assert_eq!(s["methods"]["process_stats"]["first"], 448);
    let node = &s["constructed"];
    assert_eq!(node["id"], 54447);
    assert_eq!(node["type"], "ClassStart");
    assert_eq!(node["default_mod_count"], 0);
    assert_eq!(node["fields"]["modKey"], "");
    for field in ["mods", "stats", "sd"] {
        assert!(rows(&node["fields"][field]).is_empty());
    }
    let mods = &node["default_modifiers"];
    assert!(rows(&mods["records"]).is_empty());
    assert_eq!(mods["fields"]["parent"], false);
    for field in ["actor", "multipliers", "conditions"] {
        assert!(mods["fields"][field].as_object().unwrap().is_empty());
    }
    assert_eq!(mods["metatable"], "ModList");
    assert_eq!(mods["parent_constructor_verified"], true);
    let declarations = rows(&node["declaration_fields"]);
    assert_eq!(declarations.len(), 26);
    for declaration in declarations {
        assert_eq!(
            declaration["present"], false,
            "unexpected intrinsic field {}",
            declaration["name"]
        );
        assert!(declaration.get("value").is_none());
    }
    let inventory = rows(&node["field_inventory"]);
    assert!(
        inventory
            .windows(2)
            .all(|v| v[0]["key"].as_str() < v[1]["key"].as_str())
    );
    let classes = rows(&s["classes"]);
    assert_eq!(classes.len(), 2);
    for (row, (id, name)) in classes.iter().zip([(1, "Witch"), (7, "Sorceress")]) {
        assert_eq!(row["class_id"], id);
        assert_eq!(row["name"], name);
        assert_eq!(row["start_node_id"], 54447);
        assert_eq!(row["same_root"], true);
    }
    assert_eq!(
        s["selected_root"],
        json!({"source_id":54447,"class_id":7,
        "tree_prototype_identity":true,"allocated_object_identity":true,
        "default_modifier_object_identity":true})
    );
    assert_eq!(
        s["selected"],
        json!({"items":2,"spec":3,"skills":4,"config":1,"group":3})
    );
    let neighbors = rows(&s["neighbor_connection_flags"]);
    assert!(!neighbors.is_empty());
    assert!(
        neighbors
            .windows(2)
            .all(|v| v[0]["container_node_id"].as_u64() < v[1]["container_node_id"].as_u64())
    );
    for neighbor in neighbors {
        let id = neighbor["container_node_id"].as_u64().unwrap();
        assert_ne!(id, 54447);
        assert_eq!(neighbor["distinct_from_root"], true);
        for flag in rows(&neighbor["connection_flags"]) {
            assert_eq!(flag["type"], "FLAG");
            assert_eq!(flag["value"], true);
            assert_eq!(flag["source"], format!("Tree:{id}"));
        }
    }
    if executed {
        assert!(
            s["constructor_probe"] == *node,
            "fresh original constructor matches loaded default root"
        );
    } else {
        assert!(s.get("constructor_probe").is_none());
    }
    for key in [
        "original_constructor",
        "original_process_stats",
        "original_methods_preserved",
        "exact_raw_descriptor",
        "complete_intrinsic_modifier_fields",
        "selected_default_root_unchanged",
        "cached_scalar_outputs_preserved",
        "both_class_roots_identical",
        "saved_selection_preserved",
    ] {
        assert_eq!(s["evidence"][key], true);
    }
    for key in [
        "class_owner_closed",
        "universal_player_initialization_closed",
        "external_transformations_closed",
        "all_same_source_labels_owned_by_root",
        "complete_build_claim",
    ] {
        assert_eq!(s["evidence"][key], false);
    }
}
fn child(root: &Path, out: &Path, enabled: bool) {
    let source_root = root.join("vendor/path-of-building-poe2");
    let files = [
        "src/Classes/PassiveTree.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/Common.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModStore.lua",
        "src/TreeData/0_5/tree.lua",
    ]
    .map(|path| {
        let expected = pinned::expected_file_sha256(path).unwrap();
        let text = pinned::read_verified_text(&source_root, path).unwrap();
        assert_eq!(hash(text.as_bytes()), expected, "source pin {path}");
        json!({"path":path,"sha256":expected})
    });
    let source_path = "tests/fixtures/builds/breadth-20260908/build-05.xml";
    let xml = fs::read_to_string(root.join(source_path)).unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    let pin = rows(&index["builds"])
        .iter()
        .find(|v| v["xml"] == "build-05.xml")
        .unwrap();
    assert_eq!(pin["xml_sha256"], hash(xml.as_bytes()));
    let mode = if enabled { "on" } else { "off" };
    let mut captured = vec![];
    for (name, execute) in [("original", true), ("repeat", true), ("control", false)] {
        eprintln!("Implicit class root {name} JIT {mode}");
        let host = observed(root, &xml, enabled, execute);
        fs::write(
            out.join(format!("source-jit-{mode}-{name}.raw.json")),
            serde_json::to_vec_pretty(&host).unwrap(),
        )
        .unwrap();
        check(&host, execute);
        captured.push(host);
    }
    assert!(
        captured[0] == captured[1],
        "independent fresh root replay differs; inspect raw evidence"
    );
    assert!(
        without_probe(captured[0].clone()) == without_probe(captured[2].clone()),
        "constructor probe changed actual selected state; inspect raw evidence"
    );
    assert_eq!(fs::read_to_string(root.join(source_path)).unwrap(), xml);
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":hash(OBSERVE.as_bytes()),
        "bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":files,"original_source":{"path":source_path,"sha256":hash(xml.as_bytes())},
        "original":captured[0],"repeat":captured[1],"control":captured[2],
        "complete_loads_per_jit":3,"explicit_constructor_probes_per_jit":2,
        "scope":{"original_loaded_tree_default":true,"original_full_constructor_probe":true,
            "whole_raw_descriptor":true,"complete_intrinsic_modifier_inventory":true,
            "exact_constructed_field_inventory":true,"shared_witch_sorceress_root":true,
            "fresh_repeat":true,"fresh_no_probe_control":true,"scalar_output_comparison":true,
            "full_runtime_object_graph":false,"external_transformations_closed":false,
            "class_owner_closed":false,"universal_player_initialization_closed":false,
            "complete_build_claim":false}});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires complete pinned PoB source; original implicit class-start constructor witness"]
fn actual_implicit_class_start_preserves_default_inventory_and_shared_root() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-implicit-class-start-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "use fresh immutable source output");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        fs::read(out.join("source-jit-off.json")).unwrap()
            == fs::read(out.join("source-jit-on.json")).unwrap(),
        "fresh JIT lanes must preserve byte-identical root evidence"
    );
}
