//! Optional, original-runtime evidence for the pre-Amulet contributor snapshot.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_original_pre_amulet_snapshot_preserves_contributor_census";
const CHILD: &str = "POE_AMULET_SNAPSHOT_SOURCE_CHILD";
const OBSERVER: &str = include_str!("support/amulet_snapshot_source.lua");
const LIFECYCLE: &str = include_str!("support/djinn_provider_source.lua");
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const FACTOR: &str = "EffectOfBonusesFromAmulet";
const BONUS: &str = "50% increased bonuses gained from equipped rings and amulets";
const ORIGINAL: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const FILES: [&str; 23] = [
    "src/HeadlessWrapper.lua",
    "src/Modules/Common.lua",
    "src/Modules/Build.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/CompareTab.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Classes/Item.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Modules/Data.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/ItemTools.lua",
    "src/Modules/ConfigOptions.lua",
    "src/TreeData/0_5/tree.lua",
];

#[test]
#[ignore = "requires complete pinned PoB runtime; contributor evidence only"]
fn complete_original_pre_amulet_snapshot_preserves_contributor_census() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-amulet-snapshot-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "pre-Amulet snapshot JIT parity",
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixture = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    assert_eq!(digest(original.as_bytes()), ORIGINAL);
    let index: Json =
        serde_json::from_slice(&fs::read(fixture.parent().unwrap().join("index.json")).unwrap())
            .unwrap();
    assert_eq!(index["builds"][4]["xml_sha256"], ORIGINAL);
    let changed = item_control(&original);
    let cases = [
        observe(root, "original-05", &original, enabled),
        observe(root, "amulet-jewellery-bonus-50", &changed, enabled),
        observe(root, "repeat-original-05", &original, enabled),
    ];
    let report = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "manifest_sha256":pinned::manifest_sha256(),
        "files":FILES.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":[{"path":"tests/fixtures/builds/breadth-20260908/build-05.xml","sha256":ORIGINAL}],
        "business_wrappers":false,"source_tables_mutated":false,"native_build_parity":false,
        "native_inventory_authority":false,"canonical_parity_lifecycle_selected":false,
        "lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= 8 * 1024 * 1024,
        "snapshot evidence is {} bytes",
        bytes.len()
    );
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    check(&report);
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
}

fn install_hook(lua: &Lua) -> Result<Function, RuntimeError> {
    lua.globals().set("amuletSnapshotPhase", "before")?;
    Ok(lua
        .load(OBSERVER)
        .set_name("@original-amulet-snapshot-install")
        .eval()?)
}
fn observe(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    eprintln!(
        "Amulet snapshot case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("amuletSnapshotXml", xml)?;
        lua.globals().set("amuletSnapshotJit", enabled)?;
        lua.load("if amuletSnapshotJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let auth: Function = lua
            .load(LIFECYCLE)
            .set_name("@amulet-original-lifecycle-authentication")
            .eval()?;
        let observer = install_hook(lua)?;
        let cleanup: Function = lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(cleanup.call((observer, auth))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_once = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_twice = stage(lua)?;
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}))
    };
    let scratch = tempfile::tempdir().unwrap();
    let observed = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&install),
        Some(&observer),
    )
    .unwrap_or_else(|error| panic!("{name}: complete source failed: {error}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"states":observed["additional_observation"]})
}
fn snapshot(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("amuletSnapshotPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVER)
        .set_name("@original-amulet-snapshot-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let before = snapshot(lua)?;
    let after = snapshot(lua)?;
    assert_eq!(
        json_evidence::first_difference(&before, &after, "observation-preservation"),
        None
    );
    Ok(before)
}
fn rebuild(lua: &Lua) -> Result<(), RuntimeError> {
    let cleanup = install_hook(lua)?;
    let result = lua
        .load(
            r#"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line);return f
end
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local frame=original(build.OnFrame,"Modules/Build.lua",1285)
local output=original(build.calcsTab.BuildOutput,"Classes/CalcsTab.lua",486)
assert(output==djinnOriginals.refs.calcs_tab_output and build.buildFlag==false)
local revision=build.outputRevision;local main,calcs=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
build.buildFlag=true;callback("OnFrame")
assert(build.buildFlag==false and build.outputRevision==revision+1)
assert(build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calcs)
assert(runCallback==callback and build.OnFrame==frame and build.calcsTab.BuildOutput==output)
"#,
        )
        .set_name("@original-amulet-frame-rebuild")
        .exec();
    let removed = cleanup.call::<()>(());
    result?;
    removed?;
    Ok(())
}

fn item_control(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc.descendants().find(|n| n.has_tag_name("Items")).unwrap();
    let selected = items.attribute("activeItemSet").unwrap();
    let set = items
        .children()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some(selected))
        .unwrap();
    let id = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Amulet"))
        .unwrap()
        .attribute("itemId")
        .unwrap();
    assert_eq!(id, "23");
    let item = items
        .children()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id))
        .unwrap();
    let raw = item.children().find(|n| n.is_text()).unwrap();
    let text = raw.text().unwrap();
    assert!(text.contains("Solar Amulet") && text.contains("+1 to Level of all Minion Skills"));
    assert!(!text.contains(BONUS));
    // ItemsTab parses each text node independently: insert into the original
    // complete raw item, before ModRange children, never after those children.
    let mut changed = xml.to_owned();
    changed.insert_str(raw.range().end, &format!("\n{BONUS}\n"));
    assert_eq!(changed.replacen(&format!("\n{BONUS}\n"), "", 1), xml);
    changed
}
#[test]
fn amulet_control_preserves_the_complete_raw_item_and_xml_metadata() {
    let original = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let changed = item_control(original);
    let old = roxmltree::Document::parse(original).unwrap();
    let new = roxmltree::Document::parse(&changed).unwrap();
    let old_nodes: Vec<_> = old.descendants().filter(|node| node.is_element()).collect();
    let new_nodes: Vec<_> = new.descendants().filter(|node| node.is_element()).collect();
    assert_eq!(old_nodes.len(), new_nodes.len());
    for (old, new) in old_nodes.iter().zip(&new_nodes) {
        assert_eq!(old.tag_name().name(), new.tag_name().name());
        assert_eq!(
            old.attributes().collect::<Vec<_>>(),
            new.attributes().collect::<Vec<_>>()
        );
    }
    let amulet = new
        .descendants()
        .find(|node| node.has_tag_name("Item") && node.attribute("id") == Some("23"))
        .unwrap();
    let text = amulet
        .children()
        .find(|node| node.is_text())
        .unwrap()
        .text()
        .unwrap();
    assert!(
        text.contains("Solar Amulet")
            && text.contains("+1 to Level of all Minion Skills")
            && text.contains(BONUS)
    );
}
fn rows(value: &Json) -> &[Json] {
    match value {
        Json::Array(rows) => rows,
        Json::Object(rows) if rows.is_empty() => &[],
        _ => panic!("expected source sequence, got {value}"),
    }
}
fn factor_rows(chain: &Json) -> Vec<&Json> {
    rows(chain)
        .iter()
        .flat_map(|level| rows(&level["rows"]))
        .filter(|row| row["mod"]["name"] == FACTOR)
        .collect()
}
fn check(report: &Json) {
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0]["xml_sha256"], ORIGINAL);
    assert_eq!(cases[2]["xml_sha256"], ORIGINAL);
    assert_ne!(cases[1]["xml_sha256"], ORIGINAL);
    assert_eq!(
        json_evidence::first_difference(&cases[0]["states"], &cases[2]["states"], "A-B-A"),
        None
    );
    for (case_index, case) in cases.iter().enumerate() {
        let positive = case_index == 1;
        for stage in STAGES {
            let state = &case["states"][stage];
            for flag in [
                "methods_preserved",
                "hook_removed",
                "requested_jit_mode_verified",
            ] {
                assert_eq!(state[flag], true, "{} {stage} {flag}", case["name"]);
            }
            assert_eq!(
                state["selected"],
                json!({"skills":4,"spec":3,"items":2,"config":1,"main_group":3})
            );
            assert_eq!(rows(&state["saved_node_ids"]).len(), 57);
            for mode in ["MAIN", "CALCS"] {
                let mode = &state["modes"][mode];
                assert_eq!(mode["exact_final_environment"], true);
                assert_eq!(mode["output_preserved"], true);
                let snapshot = &mode["snapshot"];
                assert_eq!(snapshot["line"], 1662);
                assert_eq!(snapshot["original_return_observed"], true);
                assert_eq!(
                    snapshot["result"].as_f64(),
                    Some(if positive { 50.0 } else { 0.0 })
                );
                let candidates = factor_rows(&snapshot["chain"]);
                assert_eq!(candidates.len(), usize::from(positive));
                if positive {
                    let value = &candidates[0]["mod"];
                    assert_eq!(value["type"], "INC");
                    assert_eq!(value["value"].as_f64(), Some(50.0));
                    assert!(value["source"].as_str().unwrap().starts_with("Item:23"));
                    assert!(rows(&value["tags"]).is_empty());
                }
                let frame = &snapshot["frame"];
                let joins = rows(&frame["candidate_joins"]);
                assert_eq!(joins.len(), usize::from(positive));
                if positive {
                    assert!(!rows(&joins[0]["item_database"]).is_empty());
                    assert!(!rows(&joins[0]["item_calls"]).is_empty());
                    for input in rows(&joins[0]["item_calls"]) {
                        assert_eq!(input["item_id"], 23);
                        assert_eq!(input["exact_object"], true);
                    }
                    assert!(rows(&joins[0]["configuration_positions"]).is_empty());
                }
                let items = rows(&frame["items"]);
                assert_eq!(items.len(), 9);
                let ids: BTreeSet<_> = items
                    .iter()
                    .map(|item| item["id"].as_u64().unwrap())
                    .collect();
                assert_eq!(ids, BTreeSet::from([19, 20, 21, 22, 23, 26, 27, 28]));
                for item in items {
                    assert_eq!(item["exact_loaded_item"], true);
                    assert_eq!(item["exact_saved_slot"], true);
                }
                let amulet = items.iter().find(|item| item["slot"] == "Amulet").unwrap();
                assert_eq!(amulet["id"], 23);
                assert_eq!(amulet["base_name"], "Solar Amulet");
                let physical_lines = rows(&amulet["source_lines"]);
                assert_eq!(physical_lines.len(), usize::from(positive));
                if positive {
                    assert_eq!(physical_lines[0]["line"], BONUS);
                    assert_eq!(rows(&physical_lines[0]["rows"]).len(), 4);
                }
                assert_eq!(rows(&frame["allocations"]).len(), 57);
                for node in rows(&frame["allocations"]) {
                    assert_eq!(node["exact_spec_node"], true);
                    assert!(rows(&node["rows"]).is_empty());
                }
                assert!(rows(&frame["granted_ids"]).is_empty());
                assert!(rows(&frame["config"]["rows"]).is_empty());
                assert!(!rows(&frame["node_calls"]).is_empty());
                for call in rows(&frame["node_calls"]) {
                    assert!(rows(&call["rows"]).is_empty());
                }
                assert_eq!(frame["radius_jewel_count"], 0);
                assert!(rows(&frame["extra_radius_node_ids"]).is_empty());
                assert_eq!(frame["talisman_list_present"], false);
                assert_eq!(frame["quiver_present"], false);
                let copies = rows(&mode["relevant_copies"]);
                let minion = copies
                    .iter()
                    .find(|copy| {
                        copy["input"]["name"] == "GemProperty"
                            && copy["input"]["value"]["keyword"] == "minion"
                    })
                    .unwrap();
                assert_eq!(minion["input"]["value"]["value"].as_f64(), Some(1.0));
                for copy in copies {
                    assert_eq!(copy["line"], 1667);
                    assert_eq!(
                        copy["scale"].as_f64(),
                        Some(if positive { 0.5 } else { 0.0 })
                    );
                    assert_eq!(
                        copy["scale"].as_f64(),
                        snapshot["result"].as_f64().map(|value| value / 100.0)
                    );
                    assert_eq!(copy["input_preserved"], true);
                    assert_eq!(copy["input_is_copy"], true);
                    assert_eq!(rows(&copy["insertions"]).len(), 1);
                    assert_eq!(copy["insertions"][0]["inserted_object_exact"], true);
                }
                if positive {
                    let copy = copies
                        .iter()
                        .find(|copy| copy["input"]["name"] == FACTOR)
                        .unwrap();
                    assert_eq!(copy["input"]["value"].as_f64(), Some(50.0));
                    assert_eq!(copy["insertions"][0]["mod"]["value"].as_f64(), Some(25.0));
                }
                let final_candidates = factor_rows(&mode["final_chain"]);
                assert_eq!(final_candidates.len(), if positive { 2 } else { 0 });
                if positive {
                    let mut values: Vec<_> = final_candidates
                        .iter()
                        .map(|row| row["mod"]["value"].as_f64().unwrap())
                        .collect();
                    values.sort_by(f64::total_cmp);
                    assert_eq!(values, [25.0, 50.0]);
                    assert_eq!(snapshot["result"].as_f64(), Some(50.0));
                }
            }
        }
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .chars()
        .rev()
        .take(6000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}
