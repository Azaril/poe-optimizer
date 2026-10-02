//! Complete original-source proof for explicit empty character RuneSlot entries.
#![cfg(not(target_arch = "wasm32"))]
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
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const TEST: &str = "explicit_empty_character_runes_preserve_complete_source_state";
const CHILD: &str = "POE_EMPTY_CHARACTER_RUNES_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_empty_character_runes_source.lua");
const SLOTS: &[&str] = &[
    "Helmet Rune #1",
    "Body Armour Rune #1",
    "Body Armour Rune #2",
    "Gloves Rune #1",
    "Boots Rune #1",
];
const PINNED_FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Classes/Item.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/ItemSlotControl.lua",
    "src/Classes/DropDownControl.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Classes/TreeTab.lua",
    "src/Modules/ItemTools.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/CalcSetup.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModStore.lua",
    "src/Data/ModRunes.lua",
    "src/TreeData/0_5/tree.lua",
    "runtime/lua/xml.lua",
];
#[derive(Clone)]
struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    switch: bool,
}

#[test]
fn explicit_empty_character_runes_preserve_complete_source_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-empty-character-runes-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source child failed {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json")),
        "exact scoped JIT evidence"
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index = read(&fixtures.join("index.json"));
    let originals: Vec<_> = (1..=5)
        .map(|n| {
            let filename = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&filename)).unwrap();
            let row = rows(&index["builds"])
                .iter()
                .find(|r| r["xml"] == filename)
                .unwrap();
            assert_eq!(row["xml_sha256"], digest(xml.as_bytes()));
            Case {
                name: format!("original-{n:02}"),
                xml,
                warm: None,
                switch: false,
            }
        })
        .collect();
    let original = &originals[3].xml;
    let target = "<RuneSlot runeName=\"None\" slotName=\"Helmet Rune #1\"/>";
    let occupied = "<RuneSlot runeName=\"Storm Rune\" slotName=\"Helmet Rune #1\"/>";
    assert_eq!(original.matches(target).count(), 1);
    let mut inputs = originals.clone();
    for (name, replacement) in [
        ("absent-entry", ""),
        (
            "missing-rune-name",
            "<RuneSlot slotName=\"Helmet Rune #1\"/>",
        ),
        (
            "empty-rune-name",
            "<RuneSlot runeName=\"\" slotName=\"Helmet Rune #1\"/>",
        ),
        (
            "lowercase-none",
            "<RuneSlot runeName=\"none\" slotName=\"Helmet Rune #1\"/>",
        ),
        (
            "unknown-rune-name",
            "<RuneSlot runeName=\"Owned Unknown Rune\" slotName=\"Helmet Rune #1\"/>",
        ),
        ("occupied-rune", occupied),
        (
            "unknown-slot",
            "<RuneSlot runeName=\"None\" slotName=\"Owned Unknown Slot\"/>",
        ),
        ("missing-slot", "<RuneSlot runeName=\"None\"/>"),
        (
            "ordinary-equipment-slot",
            "<RuneSlot runeName=\"None\" slotName=\"Helmet\"/>",
        ),
        (
            "extra-item-attribute",
            "<RuneSlot runeName=\"None\" slotName=\"Helmet Rune #1\" itemId=\"1\"/>",
        ),
        (
            "extra-rune-attribute",
            "<RuneSlot runeName=\"None\" slotName=\"Helmet Rune #1\" rune=\"Storm Rune\"/>",
        ),
        (
            "child-content",
            "<RuneSlot runeName=\"None\" slotName=\"Helmet Rune #1\"><Unknown/></RuneSlot>",
        ),
        (
            "text-content",
            "<RuneSlot runeName=\"None\" slotName=\"Helmet Rune #1\">unknown</RuneSlot>",
        ),
        (
            "namespaced-element",
            "<p:RuneSlot xmlns:p=\"urn:owned-probe\" runeName=\"None\" slotName=\"Helmet Rune #1\"/>",
        ),
        (
            "namespaced-attribute",
            "<RuneSlot xmlns:p=\"urn:owned-probe\" p:runeName=\"None\" slotName=\"Helmet Rune #1\"/>",
        ),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: original.replace(target, replacement),
            warm: None,
            switch: false,
        });
    }
    for (name, replacement) in [
        ("duplicate-none", format!("{target}{target}")),
        ("duplicate-none-occupied", format!("{target}{occupied}")),
        ("duplicate-occupied-none", format!("{occupied}{target}")),
        (
            "ordinary-slot-collision-before",
            format!("<Slot name=\"Helmet Rune #1\" itemId=\"1\"/>{target}"),
        ),
        (
            "ordinary-slot-collision-after",
            format!("{target}<Slot name=\"Helmet Rune #1\" itemId=\"1\"/>"),
        ),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: original.replace(target, &replacement),
            warm: None,
            switch: false,
        });
    }
    let occupied_xml = original.replace(target, occupied);
    for (name, xml) in [
        ("gate-none", original.clone()),
        ("gate-occupied", occupied_xml.clone()),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: add_gate(&xml),
            warm: None,
            switch: false,
        });
    }
    for (name, replacement) in [
        ("warm-occupied-to-absent", ""),
        (
            "warm-occupied-to-unknown",
            "<RuneSlot runeName=\"Owned Unknown Rune\" slotName=\"Helmet Rune #1\"/>",
        ),
        (
            "warm-occupied-to-empty",
            "<RuneSlot runeName=\"\" slotName=\"Helmet Rune #1\"/>",
        ),
        ("warm-occupied-to-none", target),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: original.replace(target, replacement),
            warm: Some(occupied_xml.clone()),
            switch: false,
        });
    }
    for selected in [1, 2] {
        let xml = with_two_sets(&add_gate(original), target, occupied, selected);
        inputs.push(Case {
            name: format!("two-sets-selected-{selected}"),
            xml,
            warm: None,
            switch: true,
        });
    }
    for order in ["before", "after"] {
        for (name, metadata) in [
            ("id", "id"),
            ("title", "title"),
            ("weapon", "useSecondWeaponSet"),
        ] {
            let bad = format!("<Slot name=\"{metadata}\" itemId=\"1\"/>");
            let replacement = if order == "before" {
                format!("{bad}{target}")
            } else {
                format!("{target}{bad}")
            };
            let mut xml = original.replace(target, &replacement);
            if name == "weapon" {
                xml = item_set_weapon_flag(&xml);
            }
            inputs.push(Case {
                name: format!("loader-error-same-slot-{name}-{order}"),
                xml,
                warm: None,
                switch: false,
            });
        }
        let bad = "<RuneSlot slotName=\"id\" runeName=\"None\"/>";
        let replacement = if order == "before" {
            format!("{bad}{target}")
        } else {
            format!("{target}{bad}")
        };
        inputs.push(Case {
            name: format!("loader-error-same-rune-id-{order}"),
            xml: original.replace(target, &replacement),
            warm: None,
            switch: false,
        });
        for (name, body) in [
            ("slot-id", "<Slot name=\"id\" itemId=\"1\"/>"),
            (
                "url",
                "<SocketIdURL nodeId=\"not-a-number\" itemPbURL=\"\"/>",
            ),
        ] {
            inputs.push(Case {
                name: format!("loader-error-sibling-{name}-{order}"),
                xml: with_sibling_set(original, body, order),
                warm: None,
                switch: false,
            });
        }
    }
    let mut cases = vec![];
    for case in &inputs {
        eprintln!("complete empty character rune source case {}", case.name);
        let captured_cleanup = Arc::new(Mutex::new(None::<Json>));
        let before = |lua: &Lua| {
            let capture = Arc::clone(&captured_cleanup);
            lua.globals().set(
                "_emptyCharacterRunesRecordCleanup",
                lua.create_function(move |lua, value: Value| {
                    *capture.lock().unwrap() = Some(lua.from_value(value)?);
                    Ok(())
                })?,
            )?;
            lua.globals()
                .set("emptyCharacterRunesXml", case.xml.as_str())?;
            lua.globals()
                .set("emptyCharacterRunesControls", case.name == "original-04")?;
            lua.globals()
                .set("emptyCharacterRunesSwitch", case.switch)?;
            lua.globals()
                .set("emptyCharacterRunesReuse", case.warm.is_some())?;
            lua.globals().set("emptyCharacterRunesJit", enabled)?;
            lua.load("if emptyCharacterRunesJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("emptyCharacterRunesPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@empty-character-runes-authentication")
                .eval::<Function>()?)
        };
        let temp = tempfile::tempdir().unwrap();
        let observed = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.name.starts_with("original-"),
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        let cleanup = captured_cleanup
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(Json::Null);
        let row = match observed {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                assert_eq!(cleanup["methods_preserved"], true);
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"expected_selections":selections(&case.xml),"available":true,"load_cleanup":cleanup,"state":value["additional_observation"]})
            }
            Err(error) => {
                let error = error.to_string();
                let expected = expected_loader_failure(&case.name).map(|(line, message)| {
                    // Launch.OnFrame catches the original source error before
                    // the shared harness refuses incomplete build state. Read
                    // its existing recorder through the cleanup hook, without
                    // replacing that handler or accepting a harness assertion.
                    let prompt = cleanup["source_prompt"].as_str().unwrap_or_default();
                    json!({"recorded_by":"Launch.OnFrame / launch.promptMsg","source_location":format!("Classes/ItemsTab.lua:{line}"),"message_fragment":message,"observed":cleanup["methods_preserved"] == true && cleanup["abort_save"] == true && prompt.replace('\\', "/").contains(&format!("Classes/ItemsTab.lua:{line}")) && prompt.contains(message)})
                });
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"available":false,"source_error":error,"load_cleanup":cleanup,"expected_loader_failure":expected})
            }
        };
        cases.push(row);
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
    for (index, case) in originals.iter().enumerate() {
        assert_eq!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", index + 1))).unwrap(),
            case.xml
        );
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),"evidence":{
        "case_count":inputs.len(),"full_controls":inputs.len()-5,"warm_loads":inputs.iter().filter(|c|c.warm.is_some()).count(),"complete_load_attempts_per_jit":inputs.len()+inputs.iter().filter(|c|c.warm.is_some()).count(),"isolated_dropdown_controls":20,"item_set_switch_controls":2,"reused_items_tab_load_controls":4,"expected_loader_failure_controls":12,
        "saved_explicit_empty_occurrences":5,"character_slot_count":5,"native_effect_coverage":false,"whole_build_parity":false,"business_method_wrappers":false,
        "observer_sha256":digest(OBSERVE.as_bytes()),"originals":originals.iter().map(|c|json!({"name":c.name,"sha256":digest(c.xml.as_bytes())})).collect::<Vec<_>>(),
        "files":PINNED_FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>()},"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result);
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("emptyCharacterRunesPhase", "after")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@empty-character-runes-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    let failures: Vec<_> = cases
        .iter()
        .filter(|case| expected_loader_failure(case["name"].as_str().unwrap()).is_some())
        .collect();
    assert_eq!(failures.len(), 12);
    for failure in failures {
        assert_eq!(failure["available"], false, "{}", failure["name"]);
        assert_eq!(
            failure["expected_loader_failure"]["observed"], true,
            "{} {}",
            failure["name"], failure["source_error"]
        );
    }
    let find = |name: &str| cases.iter().find(|c| c["name"] == name).unwrap();
    let selected = |case: &Json| {
        rows(&case["state"]["slots"])
            .iter()
            .find(|r| r["name"] == "Helmet Rune #1")
            .unwrap()
            .clone()
    };
    let mut saved = 0;
    for (index, case) in cases[..5].iter().enumerate() {
        preserved(case);
        let state = &case["state"];
        let names: BTreeSet<_> = rows(&state["slots"])
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, SLOTS.iter().copied().collect());
        for slot in rows(&state["slots"]) {
            assert_eq!(slot["selected"]["name"], "None");
            assert_eq!(slot["stored_rune_name"], "None");
            assert!(rows(&slot["selected"]["mods"]).is_empty());
            assert!(rows(&slot["none"]["mods"]).is_empty());
            assert_eq!(slot["is_item_slot"], false);
            assert_eq!(slot["is_saved_item_reference"], false);
        }
        for set in rows(&state["item_sets"]) {
            saved += rows(&set["saved_rune_rows"]).len();
        }
        assert_eq!(
            rows(&state["selected_saved_rune_rows"]).len(),
            if index == 3 { 5 } else { 0 }
        );
        for mode in ["main", "calcs"] {
            assert!(rows(&state["consumer"][mode]["character_rune_modifiers"]).is_empty());
        }
    }
    assert_eq!(saved, 5);
    let first_set = &rows(&cases[0]["state"]["item_sets"])[0]["loaded_state"];
    assert_eq!(first_set["source_table_kind"], "mixed_or_sparse");
    assert!(
        rows(&first_set["entries"])
            .iter()
            .any(|entry| { entry["key"]["kind"] == "number" && entry["key"]["value"] == 21984 })
    );
    assert!(
        rows(&first_set["entries"])
            .iter()
            .any(|entry| { entry["key"]["kind"] == "string" && entry["key"]["value"] == "id" })
    );
    for name in [
        "occupied-rune",
        "gate-none",
        "gate-occupied",
        "duplicate-none",
        "duplicate-none-occupied",
        "duplicate-occupied-none",
        "missing-rune-name",
        "absent-entry",
        "warm-occupied-to-none",
        "warm-occupied-to-absent",
        "two-sets-selected-1",
        "two-sets-selected-2",
    ] {
        preserved(find(name));
    }
    for name in [
        "missing-rune-name",
        "absent-entry",
        "duplicate-none",
        "duplicate-occupied-none",
        "warm-occupied-to-none",
        "warm-occupied-to-absent",
        "gate-none",
        "two-sets-selected-1",
    ] {
        assert_eq!(selected(find(name))["selected"]["name"], "None", "{name}");
    }
    for name in [
        "occupied-rune",
        "gate-occupied",
        "duplicate-none-occupied",
        "two-sets-selected-2",
    ] {
        assert_eq!(
            selected(find(name))["selected"]["name"],
            "Storm Rune",
            "{name}"
        );
    }
    for name in [
        "gate-none",
        "gate-occupied",
        "two-sets-selected-1",
        "two-sets-selected-2",
    ] {
        for mode in ["main", "calcs"] {
            assert_eq!(
                find(name)["state"]["consumer"][mode]["final_gate"],
                true,
                "{name} {mode}"
            );
        }
    }
    for mode in ["main", "calcs"] {
        assert!(
            rows(&find("gate-none")["state"]["consumer"][mode]["character_rune_modifiers"])
                .is_empty()
        );
        assert!(
            rows(&find("occupied-rune")["state"]["consumer"][mode]["character_rune_modifiers"])
                .is_empty()
        );
        let occupied = &find("gate-occupied")["state"]["consumer"][mode];
        let delivered = rows(&occupied["character_rune_modifiers"]);
        assert!(!delivered.is_empty());
        assert!(delivered.iter().all(|m| m["source"] == "Rune:Storm Rune"));
        assert_eq!(occupied["selected_modifier_identity_matches"], true);
    }
    let controls = rows(&cases[3]["state"]["dropdown_controls"]);
    assert_eq!(controls.len(), 20);
    for row in controls {
        assert_eq!(row["seed"]["name"], "Storm Rune");
        assert_eq!(
            row["after"]["name"],
            if row["input"] == "None" {
                "None"
            } else {
                "Storm Rune"
            }
        );
        assert_eq!(row["restored"], true);
    }
    for name in [
        "warm-occupied-to-absent",
        "warm-occupied-to-none",
        "warm-occupied-to-unknown",
        "warm-occupied-to-empty",
    ] {
        preserved(find(name));
        let probe = &find(name)["state"]["reused_items_tab_load"];
        assert_eq!(probe["same_items_tab"], true);
        let helmet = |field: &str| {
            rows(&probe[field])
                .iter()
                .find(|r| r["slot"] == "Helmet Rune #1")
                .unwrap()
                .clone()
        };
        assert_eq!(helmet("before")["selected"], "Storm Rune");
        assert_eq!(
            helmet("after")["selected"],
            if name.ends_with("unknown") || name.ends_with("empty") {
                "Storm Rune"
            } else {
                "None"
            }
        );
    }
    for name in ["two-sets-selected-1", "two-sets-selected-2"] {
        let switch = &find(name)["state"]["item_set_switch"];
        assert_eq!(switch["restored_selections"], true);
        assert_eq!(rows(&switch["transitions"]).len(), 2);
    }
}
fn preserved(case: &Json) {
    assert_eq!(
        case["available"], true,
        "{} {}",
        case["name"], case["source_error"]
    );
    for flag in [
        "saved_items_preserved",
        "saved_specs_preserved",
        "saved_item_sets_preserved",
        "saved_selections_preserved",
        "main_output_preserved",
        "calcs_output_preserved",
        "original_functions_preserved",
        "fresh_loaded_objects",
    ] {
        assert_eq!(case["state"][flag], true, "{} {flag}", case["name"]);
    }
    assert_eq!(
        case["state"]["selected"], case["expected_selections"],
        "{}",
        case["name"]
    );
}
fn add_gate(xml: &str) -> String {
    assert_eq!(xml.matches("<ConfigSet id=\"1\">").count(), 1);
    xml.replacen("<ConfigSet id=\"1\">","<ConfigSet id=\"1\"><CustomModifierBlock title=\"Owned source gate control\" enabled=\"true\">Can tattoo Runes onto your body, gaining</CustomModifierBlock>",1)
}
fn item_set_weapon_flag(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let sets: Vec<_> = items
        .children()
        .filter(|n| n.has_tag_name("ItemSet"))
        .collect();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].attribute("useSecondWeaponSet"), Some("nil"));
    let range = sets[0].range();
    let saved = &xml[range.clone()];
    assert_eq!(saved.matches("useSecondWeaponSet=\"nil\"").count(), 1);
    let replacement = saved.replacen(
        "useSecondWeaponSet=\"nil\"",
        "useSecondWeaponSet=\"true\"",
        1,
    );
    let mut result = xml.to_owned();
    result.replace_range(range, &replacement);
    result
}
fn expected_loader_failure(name: &str) -> Option<(usize, &'static str)> {
    if !name.starts_with("loader-error-") {
        return None;
    }
    Some(if name.contains("-url-") {
        (1287, "table index is nil")
    } else if name.contains("-rune-") {
        (1283, "attempt to index")
    } else {
        (1273, "attempt to index")
    })
}
fn with_sibling_set(xml: &str, body: &str, order: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let sets: Vec<_> = items
        .children()
        .filter(|n| n.has_tag_name("ItemSet"))
        .collect();
    assert_eq!(sets.len(), 1);
    let range = sets[0].range();
    let valid = &xml[range.clone()];
    let sibling = format!("<ItemSet id=\"2\">{body}</ItemSet>");
    let replacement = if order == "before" {
        format!("{sibling}\n{valid}")
    } else {
        format!("{valid}\n{sibling}")
    };
    let mut result = xml.to_owned();
    result.replace_range(range, &replacement);
    result
}
fn with_two_sets(xml: &str, target: &str, occupied: &str, selected: usize) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let sets: Vec<_> = items
        .children()
        .filter(|n| n.has_tag_name("ItemSet"))
        .collect();
    assert_eq!(sets.len(), 1);
    let range = sets[0].range();
    let set = &xml[range.clone()];
    let second = set
        .replacen("id=\"1\"", "id=\"2\"", 1)
        .replace(target, occupied);
    let mut result = xml.to_owned();
    result.replace_range(range, &format!("{set}\n{second}"));
    result.replacen(
        "activeItemSet=\"1\"",
        &format!("activeItemSet=\"{selected}\""),
        1,
    )
}
fn selections(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut result = serde_json::Map::new();
    for (key, element, attribute) in [
        ("config", "Config", "activeConfigSet"),
        ("items", "Items", "activeItemSet"),
        ("skills", "Skills", "activeSkillSet"),
        ("spec", "Tree", "activeSpec"),
        ("group", "Build", "mainSocketGroup"),
    ] {
        let node = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name(element))
            .unwrap();
        result.insert(
            key.into(),
            json!(node.attribute(attribute).unwrap().parse::<usize>().unwrap()),
        );
    }
    Json::Object(result)
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "not source list: {value}"
        );
        &[]
    }
}
fn tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}
