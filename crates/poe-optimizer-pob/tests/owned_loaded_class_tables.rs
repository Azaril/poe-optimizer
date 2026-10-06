//! Loaded-tree class identity and original class-base initializer evidence.
//! This closes an acquisition-proof gap, not Class or shared Actor coverage.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVE: &str = include_str!("support/loaded_class_tables_source.lua");
const TEST: &str = "actual_loaded_class_tables_match_catalogue_and_original_initializer";
const CHILD: &str = "POE_LOADED_CLASS_TABLES_SOURCE_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_LOADED_CLASS_TABLES_SOURCE_OUT";
const TREE_JSON: &str = "src/TreeData/0_5/tree.json";
const TREE_JSON_SHA256: &str = "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95";
const POLICY: &str = "data/owned/poe2/3887ae68/class-bases/policy.json";
const POLICY_SHA256: &str = "67d071f9966fd418e6cd4d14d68393b7573c92c04917ed58f4e8884b7f249afe";
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
fn normalized(path: &Path, expected: &str) -> String {
    let text = fs::read_to_string(path).unwrap().replace("\r\n", "\n");
    assert_eq!(
        hash(text.as_bytes()),
        expected,
        "source pin {}",
        path.display()
    );
    text
}
fn catalogue(root: &Path) -> Vec<Json> {
    let json: Json = serde_json::from_str(&normalized(
        &root.join("vendor/path-of-building-poe2").join(TREE_JSON),
        TREE_JSON_SHA256,
    ))
    .unwrap();
    let policy: Json =
        serde_json::from_str(&normalized(&root.join(POLICY), POLICY_SHA256)).unwrap();
    assert_eq!(policy["source_path"], TREE_JSON);
    let expected_fields: BTreeSet<_> = rows(&policy["expected_class_fields"])
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    let mut classes = rows(&json["classes"]).to_vec();
    assert_eq!(classes.len(), 8);
    classes.sort_by_key(|x| x["integerId"].as_u64().unwrap());
    let ids: BTreeSet<_> = classes
        .iter()
        .map(|x| x["integerId"].as_u64().unwrap())
        .collect();
    assert_eq!(ids.len(), 8);
    for class in &classes {
        assert_eq!(
            class
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            expected_fields
        );
    }
    classes
}
fn observed(root: &Path, xml: &str, enabled: bool, hooked: bool) -> Json {
    let module = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.load(if enabled {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()?;
        let observer: Table = lua
            .load(OBSERVE)
            .set_name("@loaded_class_tables_source.lua")
            .eval()?;
        observer
            .raw_get::<Function>("start")?
            .call::<()>((hooked, enabled))?;
        *module.borrow_mut() = Some(observer);
        Ok(())
    };
    let before_build = |_: &Lua| -> Result<Function, RuntimeError> {
        Ok(module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("before_build")?
            .call(())?)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let result: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("observe")?
            .call(())?;
        Ok(lua.from_value(Value::Table(result))?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before_source),
        Some(&before_build),
        Some(&after),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    json!({"source_hash":report["source_hash"], "selected":report["selected"], "state":report["additional_observation"]})
}
fn check_classes(actual: &Json, catalogue: &[Json]) {
    assert_eq!(rows(actual).len(), catalogue.len());
    for (actual, expected) in rows(actual).iter().zip(catalogue) {
        assert_eq!(actual["id"], expected["integerId"]);
        assert_eq!(actual["classes_alias_ascendancies"], true);
        let mut fields = actual["fields"].clone();
        assert!(
            fields
                .as_object_mut()
                .unwrap()
                .remove("startNodeId")
                .unwrap()
                .is_number()
        );
        let mut expected_fields = expected.clone();
        expected_fields
            .as_object_mut()
            .unwrap()
            .remove("ascendancies");
        assert_eq!(
            fields, expected_fields,
            "constructed class preserves all raw scalar metadata"
        );
        let inventory: BTreeSet<_> = rows(&actual["field_inventory"])
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        let mut expected_inventory: BTreeSet<_> = expected
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        expected_inventory.extend(["classes", "startNodeId"]);
        assert_eq!(inventory, expected_inventory);
        let asc = rows(&actual["ascendancies"]);
        assert_eq!(asc.len(), rows(&expected["ascendancies"]).len() + 1);
        assert_eq!(asc[0], json!({"index":0,"fields":{"name":"None"}}));
        for (i, (actual_asc, expected_asc)) in asc[1..]
            .iter()
            .zip(rows(&expected["ascendancies"]))
            .enumerate()
        {
            assert_eq!(actual_asc["index"], i + 1);
            let mut fields = actual_asc["fields"].clone();
            if let Some(start) = fields.as_object_mut().unwrap().remove("startNodeId") {
                assert!(start.is_number());
            }
            assert_eq!(
                &fields, expected_asc,
                "all original ascendancy fields survive construction"
            );
        }
    }
}
fn check_records(records: &Json, class: &Json) {
    for (stat, field) in [
        ("Str", "base_str"),
        ("Dex", "base_dex"),
        ("Int", "base_int"),
    ] {
        let record = &records[stat];
        assert_eq!(record["name"], stat);
        assert_eq!(record["type"], "BASE");
        assert_eq!(record["source"], "Base");
        assert_eq!(record["flags"], 0);
        assert_eq!(record["keyword_flags"], 0);
        assert_eq!(record["value"], class[field]);
        assert!(rows(&record["tags"]).is_empty());
    }
}
fn check(host: &Json, catalogue: &[Json], hooked: bool) {
    let state = &host["state"];
    assert_eq!(state["hooked"], hooked);
    assert_eq!(state["tree_version"], "0_5");
    assert_eq!(
        state["character_data"],
        json!({"present":false,"kind":"nil"})
    );
    check_classes(&state["classes"], catalogue);
    assert_eq!(state["methods"]["constructor"]["first"], 59);
    assert_eq!(state["methods"]["constructor_wrapper"]["first"], 167);
    assert_eq!(state["methods"]["initializer"]["first"], 717);
    assert_eq!(!rows(&state["constructors"]).is_empty(), hooked);
    assert_eq!(!rows(&state["initializers"]).is_empty(), hooked);
    let selected_trees = rows(&state["constructors"])
        .iter()
        .filter(|x| x["exact_selected_tree"] == true)
        .count();
    assert_eq!(selected_trees, usize::from(hooked));
    for constructor in rows(&state["constructors"]) {
        assert_eq!(constructor["tree_version"], "0_5");
        assert_eq!(rows(&constructor["raw_classes"]), catalogue);
        assert_eq!(
            constructor["character_data_at_load"],
            state["character_data"]
        );
        assert_eq!(
            constructor["character_data_at_return"],
            state["character_data"]
        );
        check_classes(&constructor["constructed_classes"], catalogue);
    }
    for initializer in rows(&state["initializers"]) {
        let class = catalogue
            .iter()
            .find(|x| x["integerId"] == initializer["class_id"])
            .unwrap();
        assert_eq!(initializer["character_data"], state["character_data"]);
        assert_eq!(initializer["selected_is_classes_row"], true);
        assert_eq!(initializer["selected_is_character_data_row"], false);
        for field in ["base_str", "base_dex", "base_int"] {
            assert_eq!(initializer["selected_bases"][field], class[field]);
        }
        check_records(&initializer["records"], class);
    }
    for mode in ["MAIN", "CALCS"] {
        let actual = &state["modes"][mode];
        let class = catalogue
            .iter()
            .find(|x| x["integerId"] == actual["class_id"])
            .unwrap();
        assert_eq!(actual["class_name"], class["name"]);
        check_records(&actual["records"], class);
        if hooked {
            for stat in ["Str", "Dex", "Int"] {
                let proof = &actual["provenance"][stat];
                assert_eq!(proof["exact_initialized_record"], true);
                assert_eq!(proof["selected_is_classes_row"], true);
                assert_eq!(proof["selected_is_character_data_row"], false);
                assert!(proof["initializer_store_is_owner"].is_boolean());
                for field in ["base_str", "base_dex", "base_int"] {
                    assert_eq!(proof["selected_bases"][field], class[field]);
                }
            }
        } else {
            assert!(actual.get("provenance").is_none());
        }
    }
    for field in [
        "original_constructor",
        "original_initializer",
        "original_methods_preserved",
        "scalar_outputs_preserved",
        "saved_items_preserved",
        "saved_selection_preserved",
    ] {
        assert_eq!(state["evidence"][field], true);
    }
    for field in ["exact_selected_tree", "exact_selected_class_row"] {
        assert_eq!(state["evidence"][field], hooked);
    }
    for field in [
        "business_method_wrappers",
        "copied_initializer",
        "full_output_graph",
        "class_coverage_closed",
        "shared_actor_state_closed",
        "complete_build_claim",
    ] {
        assert_eq!(state["evidence"][field], false);
    }
}
fn without_capture(mut value: Json) -> Json {
    let state = value["state"].as_object_mut().unwrap();
    for key in ["hooked", "constructors", "initializers"] {
        state.remove(key);
    }
    for mode in ["MAIN", "CALCS"] {
        state.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    for key in ["exact_selected_tree", "exact_selected_class_row"] {
        state
            .get_mut("evidence")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    value
}
fn child(root: &Path, out: &Path, enabled: bool) {
    let catalogue = catalogue(root);
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json =
        serde_json::from_slice(&fs::read(fixtures.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|n| fs::read_to_string(fixtures.join(format!("build-{n:02}.xml"))).unwrap())
        .collect();
    for (i, xml) in originals.iter().enumerate() {
        let name = format!("build-{:02}.xml", i + 1);
        let pin = rows(&index["builds"])
            .iter()
            .find(|x| x["xml"] == name)
            .unwrap();
        assert_eq!(pin["xml_sha256"], hash(xml.as_bytes()));
    }
    let mode = if enabled { "on" } else { "off" };
    let mut cases = Vec::new();
    for (i, original_index) in [0, 1, 2, 3, 4, 4].into_iter().enumerate() {
        let name = if i == 5 {
            "repeat-original-05".into()
        } else {
            format!("original-{:02}", i + 1)
        };
        let xml = &originals[original_index];
        eprintln!("Loaded class tables {}/6 {name} JIT {mode}", i + 1);
        let actual = observed(root, xml, enabled, true);
        let unhooked = observed(root, xml, enabled, false);
        let capture = json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"original":actual,"unhooked":unhooked});
        fs::write(
            out.join(format!("source-jit-{mode}-case-{:02}.raw.json", i + 1)),
            serde_json::to_vec_pretty(&capture).unwrap(),
        )
        .unwrap();
        check(&actual, &catalogue, true);
        check(&unhooked, &catalogue, false);
        assert!(
            without_capture(actual) == without_capture(unhooked),
            "hook-independent catalogue, scalar outputs and selected inputs; see retained case {name}"
        );
        cases.push(capture);
    }
    assert!(
        cases[4]["original"] == cases[5]["original"],
        "independent fresh initializer/tree replay agrees exactly"
    );
    assert!(
        cases[4]["unhooked"] == cases[5]["unhooked"],
        "independent fresh hook-free replay agrees exactly"
    );
    for (i, xml) in originals.iter().enumerate() {
        assert!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", i + 1))).unwrap() == *xml
        );
    }
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":hash(OBSERVE.as_bytes()),
        "bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":(["src/Classes/PassiveTree.lua","src/Modules/CalcSetup.lua","src/Modules/Common.lua",
            "src/Classes/ModStore.lua","src/Classes/ModDB.lua","src/TreeData/0_5/tree.lua","src/HeadlessWrapper.lua"]
            .map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "external_files":[{"path":format!("vendor/path-of-building-poe2/{TREE_JSON}"),"sha256":TREE_JSON_SHA256,"normalization":"crlf-to-lf"},
            {"path":POLICY,"sha256":POLICY_SHA256,"normalization":"crlf-to-lf"}],
        "original_sources":originals.iter().enumerate().map(|(i,xml)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":hash(xml.as_bytes())})).collect::<Vec<_>>(),
        "catalogue":catalogue,"case_count":6,"complete_loads_per_jit":12,
        "scope":{"original_tree_constructor":true,"original_class_initializer":true,"all_eight_class_rows":true,
            "all_raw_class_and_ascendancy_fields":true,"actual_character_data_presence":true,
            "exact_current_record_identity":true,"independent_fresh_repeat":true,"fresh_unhooked_comparison":true,
            "scalar_output_comparison":true,"full_output_graph":false,"warm_rebuild_claim":false,
            "all_eight_classes_selected_in_calculation":false,"class_closure":false,"complete_build":false},"cases":cases});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires complete pinned PoB source; bounded original loaded-tree/class initializer witness"]
fn actual_loaded_class_tables_match_catalogue_and_original_initializer() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-loaded-class-tables-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "use a fresh immutable source output");
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
        "both JIT modes preserve byte-identical original evidence"
    );
}
