//! Exact original placement calls for five selected equipment bases.
//! No native algorithm, item mechanics closure, or augment admission is supplied.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/item_assembly_graph.rs"]
mod graph;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
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
const BINDING: &str = include_str!("support/item_slot_validity_source.lua");
const OBSERVE: &str = include_str!("support/selected_equipment_placement_source.lua");
const TEST: &str = "actual_selected_equipment_placement_is_stable_across_slots_sets_and_flags";
const CHILD: &str = "POE_SELECTED_EQUIPMENT_PLACEMENT_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_SELECTED_EQUIPMENT_PLACEMENT_SOURCE_OUT";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn rows(value: &Json) -> &[Json] {
    if let Some(a) = value.as_array() {
        a
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn profiles() -> [Json; 5] {
    [
        json!({"id":19,"base_name":"Tattered Robe","item_type":"Body Armour","source_slots":["Body Armour"],"selected_source_slots":["Body Armour"],"boundary_slot":"Body Armour 1"}),
        json!({"id":20,"base_name":"Rope Cuffs","item_type":"Gloves","source_slots":["Gloves"],"selected_source_slots":["Gloves"],"boundary_slot":"Gloves 1"}),
        json!({"id":26,"base_name":"Sapphire Ring","item_type":"Ring","source_slots":["Ring 1","Ring 2","Ring 3"],"selected_source_slots":["Ring 1","Ring 2"],"boundary_slot":"Ring 4"}),
        json!({"id":27,"base_name":"Fine Belt","item_type":"Belt","source_slots":["Belt"],"selected_source_slots":["Belt"],"boundary_slot":"Belt 1"}),
        json!({"id":28,"base_name":"Ashen Staff","item_type":"Staff","source_slots":["Weapon 1","Weapon 1 Swap"],"selected_source_slots":["Weapon 1"],"boundary_slot":"Weapon"}),
    ]
}
fn observed(root: &Path, xml: &str, enabled: bool, execute: bool) -> Json {
    let module = Rc::new(RefCell::new(None::<Table>));
    let before = |lua: &Lua| {
        lua.load(if enabled {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()?;
        *module.borrow_mut() = Some(
            lua.load(BINDING)
                .set_name("@item_slot_validity_source.lua")
                .eval()?,
        );
        Ok(())
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let bound: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("bind")?
            .call(())?;
        let observer: Function = lua
            .load(OBSERVE)
            .set_name("@selected_equipment_placement_source.lua")
            .eval()?;
        let mut results = Vec::new();
        for profile in profiles() {
            let profile = lua.to_value(&profile)?;
            let result: Table = observer.call((bound.clone(), execute, enabled, profile))?;
            let before: Table = result.raw_get("context_before")?;
            let after: Table = result.raw_get("context_after")?;
            let before =
                graph::canonical(&graph::capture(&[Value::Table(before)]).unwrap()).unwrap();
            let after = graph::canonical(&graph::capture(&[Value::Table(after)]).unwrap()).unwrap();
            assert_eq!(
                before, after,
                "original declared read-set retains aliases and values"
            );
            result.raw_set("context_before", Value::Nil)?;
            result.raw_set("context_after", Value::Nil)?;
            let mut out: Json = lua.from_value(Value::Table(result))?;
            out["read_set"] = before;
            out["evidence"]["read_set_unchanged"] = json!(true);
            results.push(out);
        }
        Ok(json!(results))
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
fn without_calls(mut host: Json) -> Json {
    for state in host["state"].as_array_mut().unwrap() {
        for key in ["calls", "executed", "boundary"] {
            state.as_object_mut().unwrap().remove(key);
        }
    }
    host
}
fn check(host: &Json) {
    let states = rows(&host["state"]);
    assert_eq!(states.len(), 5);
    for (s, profile) in states.iter().zip(profiles()) {
        assert_eq!(s["executed"], true);
        assert_eq!(
            s["method"],
            json!({"path":"Classes/ItemsTab.lua","first":2603,"last":2687})
        );
        assert_eq!(s["item"]["id"], profile["id"]);
        assert_eq!(s["item"]["base_name"], profile["base_name"]);
        assert_eq!(s["item"]["item_type"], profile["item_type"]);
        assert_eq!(s["item"]["base"]["type"], profile["item_type"]);
        let slots = rows(&s["registered_slots"]);
        assert_eq!(slots.len(), 113);
        assert!(slots.windows(2).all(|w| w[0].as_str() < w[1].as_str()));
        assert_eq!(rows(&s["base_slots"]).len(), 20);
        assert_eq!(s["set_ids"], json!([1, 2, 3, 4, 5, 6]));
        assert_eq!(rows(&s["flag_cases"]).len(), 9);
        assert_eq!(
            s["selected"],
            json!({"items":2,"spec":3,"skills":4,"config":1,"group":3})
        );
        for slot in rows(&profile["selected_source_slots"]) {
            assert!(
                rows(&s["selected_uses"])
                    .iter()
                    .any(|u| u == &json!({"set":2,"slot":slot}))
            );
        }
        for tag in ["onehand", "one_hand_weapon", "axe", "mace", "sword"] {
            assert!(s["item"]["base"]["tags"].get(tag).is_none());
        }
        if profile["item_type"] == "Staff" {
            assert_eq!(s["item"]["base"]["tags"]["twohand"], true);
        } else {
            assert!(s["item"]["base"]["tags"].get("twohand").is_none());
        }
        let calls = rows(&s["calls"]);
        assert_eq!(calls.len(), 7119);
        let mut keys = BTreeSet::new();
        for r in calls {
            let context = r["set"].as_str().unwrap();
            assert!(
                context == "active-default" || (1..=6).any(|i| context == format!("saved-{i}"))
            );
            assert!(slots.contains(&r["slot"]));
            assert!(rows(&s["flag_cases"]).iter().any(|f| f["id"] == r["flags"]));
            assert!(keys.insert((
                context,
                r["slot"].as_str().unwrap(),
                r["flags"].as_str().unwrap()
            )));
            let o = &r["outcome"];
            match o["kind"].as_str().unwrap() {
                "none" => assert_eq!(*o, json!({"return_count":0,"kind":"none"})),
                "nil" => assert_eq!(*o, json!({"return_count":1,"kind":"nil"})),
                "boolean" => {
                    assert_eq!(o["return_count"], 1);
                    assert!(o["value"].is_boolean());
                }
                _ => panic!("unknown original return"),
            }
            assert_eq!(
                o["value"] == true,
                rows(&profile["source_slots"]).contains(&r["slot"])
            );
        }
        assert_eq!(
            s["boundary"],
            json!({"slot":profile["boundary_slot"],"registered":false,"outcome":{"return_count":1,"kind":"boolean","value":true},"native_admission":false})
        );
        for flag in [
            "exact_catalogue_base",
            "exact_registered_item",
            "exact_selected_main_and_calcs_item",
            "original_functions_preserved",
            "saved_items_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "repeated_calls_equal",
            "read_set_unchanged",
        ] {
            assert_eq!(s["evidence"][flag], true);
        }
        for flag in [
            "call_hook",
            "method_wrappers",
            "native_owner_coverage",
            "numerical_parity",
            "socket_configuration_admission",
        ] {
            assert_eq!(s["evidence"][flag], false);
        }
    }
}
#[test]
fn actual_selected_equipment_placement_is_stable_across_slots_sets_and_flags() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let temporary = std::env::var_os(OUTPUT)
        .is_none()
        .then(|| tempfile::tempdir().unwrap());
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| temporary.as_ref().unwrap().path().join("evidence"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
        let index = read(&fixtures.join("index.json"));
        let entry = rows(&index["builds"])
            .iter()
            .find(|b| b["xml"] == "build-05.xml")
            .unwrap();
        assert_eq!(entry["xml_sha256"], hash(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([53; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let source_items:Vec<_>=["19","20","26","27","28"].into_iter().map(|id|{
            let item:Vec<_>=evidence.rows().iter().filter(|r|r.occurrence().name()=="Item"&&r.attribute("id").and_then(|a|a.decoded().ok())==Some(id)).collect();
            assert_eq!(item.len(),1);
            json!({"id":id.parse::<u32>().unwrap(),"ordinal":item[0].occurrence().id().ordinal(),"content_entry":0})
        }).collect();
        let control = observed(&root, &xml, mode == "on", false);
        let first = observed(&root, &xml, mode == "on", true);
        let repeat = observed(&root, &xml, mode == "on", true);
        check(&first);
        check(&repeat);
        assert_eq!(
            first, repeat,
            "independent fresh original runtimes agree exactly"
        );
        assert_eq!(
            without_calls(control.clone()),
            without_calls(first.clone()),
            "no-call fresh control retains exact context, selected item and outputs"
        );
        assert_eq!(
            fs::read_to_string(fixtures.join("build-05.xml")).unwrap(),
            xml
        );
        let report = json!({"schema_version":1,"source_hash":first["source_hash"],
            "source_xml_sha256":hash(xml.as_bytes()),"source_items":source_items,
            "evidence":{"manifest_sha256":pinned::manifest_sha256(),
                "binding_sha256":hash(BINDING.as_bytes()),"observer_sha256":hash(OBSERVE.as_bytes()),
                "fresh_runtimes":3,"unhooked":true,"independent_replay_equal":true,"no_call_control_equal":true,
                "scope":"registered-source-slot-catalogue-only",
                "files":(["src/Classes/ItemsTab.lua","src/Classes/Item.lua","src/Classes/ModStore.lua",
                    "src/Classes/ModDB.lua","src/Modules/Build.lua", "src/Modules/Common.lua","src/Data/Bases/body.lua","src/Data/Bases/gloves.lua","src/Data/Bases/belt.lua","src/Data/Bases/ring.lua","src/Data/Bases/staff.lua"]
                    .map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))},
            "control":control,"original":first,"repeat":repeat});
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if mode == "on" { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        return;
    }
    assert!(
        !out.exists(),
        "use a fresh immutable source evidence output"
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("source-jit-off.json")).unwrap(),
        fs::read(out.join("source-jit-on.json")).unwrap(),
        "JIT modes produce byte-identical source evidence"
    );
}
