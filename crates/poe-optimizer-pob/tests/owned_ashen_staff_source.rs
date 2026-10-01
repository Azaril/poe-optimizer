//! Complete pinned source lifecycle for the selected Ashen Staff provider.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
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
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "ashen_staff_preserves_full_item_and_generated_skill_lifecycle";
const CHILD: &str = "POE_ASHEN_STAFF_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/ashen_staff_source.lua");

#[test]
fn ashen_staff_preserves_full_item_and_generated_skill_lifecycle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-ashen-staff-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixtures.join("build-05.xml")).unwrap();
        let manifest = read(&fixtures.join("index.json"));
        let entry = rows(&manifest["builds"])
            .iter()
            .find(|r| r["xml"] == "build-05.xml")
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([54; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let matches: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Item"
                    && r.attribute("id").and_then(|a| a.decoded().ok()) == Some("28")
            })
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].occurrence().id().ordinal(), 594);
        let mut inputs = vec![("original", xml.clone())];
        for (label, fraction) in [
            ("xml-range-zero", "0"),
            ("xml-range-one", "1"),
            ("xml-range-quarter", "0.25"),
        ] {
            inputs.push((
                label,
                edit_element(&xml, "Item", 28, |body| {
                    body.replace(
                        "<ModRange range=\"0.5\"",
                        &format!("<ModRange range=\"{fraction}\""),
                    )
                }),
            ));
        }
        inputs.push((
            "inline-conflict",
            edit_element(&xml, "Item", 28, |b| {
                replace(b, "{range:0.5}Grants Skill", "{range:0}Grants Skill")
            }),
        ));
        inputs.push((
            "quality-zero",
            edit_element(&xml, "Item", 28, |b| {
                replace(b, "Quality: 20", "Quality: 0")
            }),
        ));
        inputs.push((
            "quality-absent",
            edit_element(&xml, "Item", 28, |b| replace(b, "Quality: 20\n", "")),
        ));
        inputs.push((
            "unequipped",
            edit_element(&xml, "ItemSet", 2, |b| {
                replace(b, "itemId=\"28\"", "itemId=\"0\"")
            }),
        ));
        inputs.push((
            "second-weapon-set",
            edit_element(&xml, "ItemSet", 2, |b| {
                replace(
                    b,
                    "useSecondWeaponSet=\"nil\"",
                    "useSecondWeaponSet=\"true\"",
                )
            }),
        ));
        inputs.push(("saved-group-absent", edit_provider(&xml, |_| String::new())));
        inputs.push((
            "saved-level-stale",
            edit_provider(&xml, |b| replace(b, "level=\"11\"", "level=\"7\"")),
        ));
        inputs.push((
            "saved-quality-seven",
            edit_provider(&xml, |b| replace(b, "quality=\"0\"", "quality=\"7\"")),
        ));
        inputs.push((
            "saved-group-disabled",
            edit_provider(&xml, |b| {
                b.replacen("enabled=\"true\"", "enabled=\"false\"", 1)
            }),
        ));
        inputs.push((
            "saved-global-false",
            edit_provider(&xml, |b| {
                replace(b, "enableGlobal1=\"true\"", "enableGlobal1=\"false\"")
            }),
        ));
        inputs.push((
            "saved-count-three",
            edit_provider(&xml, |b| replace(b, "count=\"nil\"", "count=\"3\"")),
        ));
        inputs.push((
            "item-removed",
            edit_element(&xml, "Item", 28, |_| String::new()),
        ));
        let mut cases = vec![];
        let mut source_hash = None;
        for (name, text) in inputs {
            let before = |lua: &Lua| {
                lua.globals().set("ashenXml", text.as_str())?;
                lua.globals().set("ashenControls", name == "original")?;
                lua.globals().set("ashenJit", enabled)?;
                lua.load("if ashenJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                &text,
                None,
                false,
                Some(&before),
                None,
                Some(&observe),
            )
            .unwrap();
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            if let Some(hash) = &source_hash {
                assert_eq!(&result["source_hash"], hash)
            } else {
                source_hash = Some(result["source_hash"].clone())
            }
            cases.push(json!({"name":name,"xml_sha256":digest(text.as_bytes()),"state":result["additional_observation"]}));
        }
        assert_eq!(
            fs::read_to_string(fixtures.join("build-05.xml")).unwrap(),
            xml
        );
        let result = json!({"source_hash":source_hash,"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","evidence":{"manifest_sha256":pinned::manifest_sha256(),"native_input_closure":false,"native_owner_coverage":false,"files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Classes/SkillsTab.lua","src/Modules/Build.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/ModScalability.lua","src/Data/Bases/staff.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))} ,"cases":cases});
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        check(&result);
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
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display())
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(v) = value.as_array() {
        v
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn replace(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "{old}");
    text.replace(old, new)
}
fn edit_element(xml: &str, element: &str, id: usize, edit: impl FnOnce(&str) -> String) -> String {
    let prefix = format!("<{element} ");
    let needle = format!("id=\"{id}\"");
    let starts: Vec<_> = xml
        .match_indices(&prefix)
        .filter_map(|(start, _)| {
            let end = start + xml[start..].find('>')?;
            xml[start..end]
                .split_whitespace()
                .any(|a| a == needle)
                .then_some(start)
        })
        .collect();
    assert_eq!(starts.len(), 1, "{element} {id}");
    let begin = starts[0];
    let close = format!("</{element}>");
    let end = begin + xml[begin..].find(&close).unwrap() + close.len();
    let mut out = xml.to_owned();
    out.replace_range(begin..end, &edit(&xml[begin..end]));
    out
}
fn edit_provider(xml: &str, edit: impl FnOnce(&str) -> String) -> String {
    edit_element(xml, "SkillSet", 4, |set| {
        let begin = set.find("<Skill source=\"Item:28:").unwrap();
        let end = begin + set[begin..].find("</Skill>").unwrap() + 8;
        let mut out = set.to_owned();
        out.replace_range(begin..end, &edit(&set[begin..end]));
        out
    })
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let v: Value = lua
        .load(OBSERVE)
        .set_name("@ashen-staff-source-observer")
        .eval()?;
    Ok(lua.from_value(v)?)
}
fn named<'a>(rows_: &'a Json, name: &str) -> &'a Json {
    let found: Vec<_> = rows(rows_).iter().filter(|r| r["name"] == name).collect();
    assert_eq!(found.len(), 1, "{name}");
    found[0]
}
fn active_values(snapshot: &Json, name: &str) -> Vec<Json> {
    rows(&snapshot["active"])
        .iter()
        .filter(|r| r["name"] == name)
        .map(|r| r["value"].clone())
        .collect()
}
fn grant_level(snapshot: &Json) -> Json {
    let list = rows(&snapshot["granted_skills"]);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["skillId"], "FireboltPlayer");
    list[0]["level"].clone()
}
fn selected_groups(state: &Json) -> Vec<&Json> {
    rows(&state["runtime_groups"])
        .iter()
        .filter(|g| g["selected"] == true)
        .collect()
}
fn check(result: &Json) {
    assert_eq!(
        result["evidence"]["manifest_sha256"],
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    assert_eq!(rows(&result["cases"]).len(), 16);
    for case in rows(&result["cases"]) {
        for flag in [
            "original_functions_preserved",
            "saved_instances_preserved",
            "selected_state_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(case["state"][flag], true, "{} {flag}", case["name"])
        }
        assert_eq!(case["state"]["selection"]["skills"], 4);
        assert_eq!(case["state"]["selection"]["items"], 2)
    }
    let original = &named(&result["cases"], "original")["state"];
    let item = &original["item"];
    let loaded = &item["loaded"];
    assert_eq!(loaded, &item["fresh"]);
    assert_eq!(rows(&item["probes"]).len(), 50);
    assert_eq!(loaded["base"], "Ashen Staff");
    assert_eq!(loaded["rarity"], "RARE");
    assert_eq!(loaded["crafted"], true);
    assert_eq!(loaded["quality"], 20);
    assert_eq!(loaded["itemSocketCount"], 4);
    assert_eq!(loaded["jewelSocketCount"], 0);
    assert_eq!(loaded["runes"], json!(["None", "None", "None", "None"]));
    assert_eq!(loaded["requirements"]["level"], 26);
    for field in ["itemLevel", "catalyst", "catalystQuality", "corrupted"] {
        assert_eq!(loaded["field_types"][field], "nil", "{field}");
    }
    assert!(item["raw"].as_str().unwrap().contains("LevelReq: 26"));
    assert_eq!(rows(&loaded["lists"]["implicit"]).len(), 1);
    assert_eq!(rows(&loaded["lists"]["explicit"]).len(), 1);
    for category in ["buff", "enchant", "rune", "classRequirement"] {
        assert!(rows(&loaded["lists"][category]).is_empty())
    }
    assert_eq!(grant_level(loaded), 11);
    assert_eq!(grant_level(&item["fresh"]), 11);
    assert_eq!(rows(&loaded["base_mods"]).len(), 2);
    assert_eq!(rows(&loaded["active"]).len(), 3);
    assert_eq!(
        active_values(loaded, "Multiplier:QualityOnWeapon 1"),
        vec![json!(20)]
    );
    assert_eq!(active_values(loaded, "Damage"), vec![json!(128)]);
    let spell = &rows(&loaded["lists"]["explicit"])[0];
    assert!(rows(&spell["modTags"]).is_empty());
    assert_eq!(spell["field_types"]["extra"], "nil");
    assert_eq!(spell["catalyst_factor"], 1.0);
    let records = rows(&spell["records"]);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["name"], "Damage");
    assert_eq!(records[0]["type"], "INC");
    assert_eq!(records[0]["flags"], original["spell_flag"]);
    assert_eq!(records[0]["keyword_flags"], 0);
    assert!(rows(&records[0]["tags"]).is_empty());
    let grant = &rows(&loaded["lists"]["implicit"])[0];
    assert_eq!(grant["valueScalar"], 1.0);
    assert_eq!(grant["catalyst_factor"], 1.0);
    assert_eq!(grant["field_types"]["extra"], "nil");
    assert!(rows(&grant["modTags"]).is_empty());
    assert_eq!(rows(&grant["records"]).len(), 1);
    assert_eq!(grant["records"][0]["name"], "ExtraSkill");
    assert_eq!(grant["records"][0]["type"], "LIST");
    assert_eq!(grant["records"][0]["value"]["level"], 11);
    for member in [spell, grant] {
        for field in [
            "corruptedRange",
            "custom",
            "desecrated",
            "fractured",
            "disabled",
            "prefix",
            "suffix",
            "unscalable",
            "variantList",
        ] {
            assert_eq!(member["field_types"][field], "nil", "{field}");
        }
    }
    assert_eq!(rows(&item["receiving"]).len(), 1);
    assert_eq!(item["selected_slots"], json!(["Weapon 1"]));
    assert_eq!(
        rows(&original["saved_groups"])
            .iter()
            .map(|g| g["set"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [2, 4]
    );
    let groups = selected_groups(original);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["source_item_id"], 28);
    assert_eq!(groups[0]["gems"][0]["fields"]["fromItem"], true);
    for mode in ["MAIN", "CALCS"] {
        let grants = rows(&original["granted"][mode]);
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0]["fields"]["level"], 11);
        let actions = rows(&original["actions"][mode]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["level"], 11);
        assert_eq!(actions[0]["quality"], 0);
        assert_eq!(actions[0]["actor_is_player"], true);
        assert_eq!(actions[0]["from_item"], true);
        assert_eq!(rows(&actions[0]["groups"]).len(), 1);
    }
    for (name, level) in [
        ("xml-range-zero", 1),
        ("xml-range-one", 20),
        ("xml-range-quarter", 6),
        ("inline-conflict", 11),
    ] {
        assert_eq!(
            grant_level(&named(&result["cases"], name)["state"]["item"]["loaded"]),
            level,
            "{name}"
        )
    }
    for (name, quality) in [("quality-zero", 0), ("quality-absent", 0)] {
        let state = &named(&result["cases"], name)["state"];
        assert_eq!(state["item"]["loaded"]["quality"], quality, "{name}");
        assert_eq!(grant_level(&state["item"]["loaded"]), 11);
        assert_eq!(
            active_values(&state["item"]["loaded"], "Damage"),
            vec![json!(128)]
        );
        assert_eq!(
            active_values(&state["item"]["loaded"], "Multiplier:QualityOnWeapon 1"),
            vec![json!(0)]
        );
    }
    for name in ["unequipped", "item-removed"] {
        let state = &named(&result["cases"], name)["state"];
        assert!(selected_groups(state).is_empty(), "{name}");
        assert!(rows(&state["granted"]["MAIN"]).is_empty());
        assert!(rows(&state["actions"]["MAIN"]).is_empty());
    }
    let swapped = &named(&result["cases"], "second-weapon-set")["state"];
    assert_eq!(selected_groups(swapped).len(), 1);
    assert_eq!(rows(&swapped["granted"]["MAIN"]).len(), 1);
    assert!(rows(&swapped["actions"]["MAIN"]).is_empty());
    for name in ["saved-group-absent", "saved-level-stale"] {
        let state = &named(&result["cases"], name)["state"];
        let groups = selected_groups(state);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["gems"][0]["fields"]["level"], 11);
        assert_eq!(groups[0]["gems"][0]["fields"]["quality"], 0);
    }
    let quality = &named(&result["cases"], "saved-quality-seven")["state"];
    assert_eq!(
        selected_groups(quality)[0]["gems"][0]["fields"]["quality"],
        7
    );
    assert_eq!(quality["actions"]["MAIN"][0]["quality"], 7);
    assert!(
        rows(&named(&result["cases"], "saved-group-disabled")["state"]["actions"]["MAIN"])
            .is_empty()
    );
    let global = &named(&result["cases"], "saved-global-false")["state"];
    assert_eq!(
        selected_groups(global)[0]["gems"][0]["fields"]["enableGlobal1"],
        true
    );
    assert_eq!(rows(&global["actions"]["MAIN"]).len(), 1);
    assert_eq!(
        selected_groups(&named(&result["cases"], "saved-count-three")["state"])[0]["gems"][0]["fields"]
            ["count"],
        3
    );
    for (name, level) in [
        ("grant-low", 1),
        ("grant-high", 20),
        ("grant-quarter", 6),
        ("grant-fixed", 11),
        ("grant-zero", 0),
        ("grant-one", 1),
        ("grant-twenty", 20),
        ("grant-hundred", 100),
        ("implicit-magnitude", 11),
        // Corrupted textual scaling precedes LIST construction. It is not
        // covered by the later ExtraSkill modifier-magnitude exemption.
        ("grant-corrupted-range", 17),
        ("grant-range-half-neighbour", 1),
        ("grant-positive-range-below-half", 1),
        ("grant-positive-range-half-neighbour", 2),
    ] {
        assert_eq!(
            grant_level(&named(&item["probes"], name)["state"]),
            level,
            "{name}"
        )
    }
    for (name, value) in [
        ("spell-zero", 0),
        ("spell-fraction", 129),
        ("spell-negative", -128),
        ("explicit-magnitude", 192),
        ("implicit-magnitude", 128),
        ("catalyst-untagged", 128),
        ("tagged-caster-catalyst", 153),
        ("tagged-caster-default", 153),
        ("tagged-caster-zero", 128),
        ("tagged-caster-wrong", 128),
        ("spell-corrupted-range", 192),
    ] {
        assert_eq!(
            active_values(&named(&item["probes"], name)["state"], "Damage"),
            vec![json!(value)],
            "{name}"
        )
    }
    let zero = &named(&item["probes"], "spell-zero")["state"]["lists"]["explicit"][0];
    assert_eq!(zero["field_types"]["extra"], "nil");
    assert_eq!(rows(&zero["records"]).len(), 1);
    assert_eq!(zero["records"][0]["flags"], original["spell_flag"]);
    for name in [
        "grant-negative",
        "grant-unknown",
        "grant-missing",
        // Unlike fixed0, getRangedModList filters the resolved zero line.
        "grant-range-zero",
        "grant-range-zero-endpoint",
        "grant-range-below-half",
    ] {
        assert!(
            rows(&named(&item["probes"], name)["state"]["granted_skills"]).is_empty(),
            "{name}"
        );
    }
    assert_eq!(
        named(&item["probes"], "grant-negative")["state"]["lists"]["implicit"][0]["extra"],
        "Grants Skill: Level -1 Firebolt"
    );
    assert_eq!(
        rows(&named(&item["probes"], "grant-duplicate")["state"]["granted_skills"]).len(),
        2
    );
    assert!(
        active_values(
            &named(&item["probes"], "spell-unknown-prefix")["state"],
            "Damage"
        )
        .is_empty()
    );
    for (name, flag) in [
        ("spell-desecrated", "desecrated"),
        ("spell-fractured", "fractured"),
    ] {
        assert_eq!(
            named(&item["probes"], name)["state"]["lists"]["explicit"][0][flag],
            true
        );
    }
    assert_eq!(
        named(&item["probes"], "tagged-caster")["state"]["lists"]["explicit"][0]["modTags"],
        json!(["caster"])
    );
    assert_eq!(
        named(&item["probes"], "unknown-rune")["state"]["runes"][0],
        "OwnedUnknownRune"
    );
    assert_eq!(
        named(&item["probes"], "sockets-absent")["state"]["itemSocketCount"],
        0
    );
    assert_eq!(
        named(&item["probes"], "item-level")["state"]["itemLevel"],
        77
    );
    assert_eq!(
        named(&item["probes"], "crafted-false")["state"]["crafted"],
        true
    );
    for name in ["requirement-zero", "requirement-absent"] {
        assert_eq!(
            named(&item["probes"], name)["state"]["requirements"]["level"],
            0
        );
    }
    let no_label = &named(&item["probes"], "affix-label-absent")["state"];
    assert_eq!(no_label["base_mods"], loaded["base_mods"]);
    assert_eq!(no_label["active"], loaded["active"]);
    let reparsed = &item["reparsed"];
    assert_eq!(reparsed["itemLevel"], 77);
    assert_eq!(reparsed["corrupted"], true);
    assert_eq!(
        item["fresh_after_reparse"]["field_types"]["itemLevel"],
        "nil"
    );
}
