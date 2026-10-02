//! Authentic saved-row roles before and after provider-driven reconstruction.
//! No group-wide exclusion, physical gem definition, or evaluator coverage is inferred.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "fresh_nonphysical_skill_rows_preserve_physical_support_census";
const CHILD: &str = "POE_NONPHYSICAL_SKILL_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_nonphysical_skill_source.lua");
const EFFECT: &str = "EnemyExplode";
const PHYSICAL: &str = "Enemies you kill have a 31% chance to explode, dealing a tenth of their maximum Life as Physical damage";
const CHAOS: &str = "Enemies you kill have a 16% chance to explode, dealing a quarter of their maximum Life as Chaos damage";
const FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Modules/Data.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/Item.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/ModTools.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModStore.lua",
    "src/Data/Gems.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/sup_int.lua",
    "src/TreeData/0_5/tree.lua",
    "runtime/lua/xml.lua",
];
#[derive(Clone)]
struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    original: bool,
}

#[test]
fn fresh_nonphysical_skill_rows_preserve_physical_support_census() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-nonphysical-skill-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
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
            // Full fresh loads and preservation checks need slower-CI headroom.
            if start.elapsed() > Duration::from_secs(1200) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("source-jit-off.json")).unwrap(),
        fs::read(out.join("source-jit-on.json")).unwrap(),
        "exact source evidence across JIT modes"
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
            let name = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&name)).unwrap();
            let entry = rows(&index["builds"])
                .iter()
                .find(|v| v["xml"] == name)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            (name, xml)
        })
        .collect();
    let original = &originals[2].1;
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, (_, xml))| Case {
            name: format!("original-{:02}", i + 1),
            xml: xml.clone(),
            warm: None,
            original: true,
        })
        .collect();
    push(&mut cases, "remove-saved-group", remove_group(original));
    push(
        &mut cases,
        "remove-physical-provider-line",
        remove_line(original, PHYSICAL),
    );
    push(
        &mut cases,
        "remove-chaos-provider-line",
        remove_line(original, CHAOS),
    );
    let absent = remove_line(&remove_line(original, PHYSICAL), CHAOS);
    push(&mut cases, "remove-both-provider-lines", absent.clone());
    push(&mut cases, "unequip-provider", unequip_provider(original));
    cases.push(Case {
        name: "repeat-original-03".into(),
        xml: original.clone(),
        warm: None,
        original: true,
    });
    cases.push(Case {
        name: "warm-absent-to-original".into(),
        xml: original.clone(),
        warm: Some(absent.clone()),
        original: true,
    });
    cases.push(Case {
        name: "warm-original-to-absent".into(),
        xml: absent,
        warm: Some(original.clone()),
        original: false,
    });
    let support = selected_support(original);
    let support_name = support["nameSpec"].as_str().unwrap();
    push(
        &mut cases,
        "known-effect-misleading-name",
        change_gem(original, &[("nameSpec", support_name)]),
    );
    push(
        &mut cases,
        "unknown-effect-blank-name",
        change_gem(original, &[("skillId", "__missing_source_effect__")]),
    );
    push(
        &mut cases,
        "unknown-effect-support-name",
        change_gem(
            original,
            &[
                ("skillId", "__missing_source_effect__"),
                ("nameSpec", support_name),
            ],
        ),
    );
    push(
        &mut cases,
        "physical-gem-id-wins",
        change_gem(
            original,
            &[
                ("gemId", support["gemId"].as_str().unwrap()),
                ("variantId", support["variantId"].as_str().unwrap()),
            ],
        ),
    );
    push(
        &mut cases,
        "invalid-gem-id-blocks-effect",
        change_gem(original, &[("gemId", "__missing_game_gem__")]),
    );
    push(
        &mut cases,
        "empty-gem-id-blocks-effect",
        change_gem(original, &[("gemId", "")]),
    );
    push(
        &mut cases,
        "invalid-gem-id-support-name",
        change_gem(
            original,
            &[
                ("gemId", "__missing_game_gem__"),
                ("nameSpec", support_name),
            ],
        ),
    );
    push(
        &mut cases,
        "append-physical-support",
        append_support(original, false),
    );
    push(
        &mut cases,
        "duplicate-saved-group",
        duplicate_group(original),
    );
    push(
        &mut cases,
        "unexpected-child-support",
        append_support(original, true),
    );
    assert_eq!(cases.len(), 23);
    let mut observed = Vec::new();
    fs::create_dir_all(out.join("inputs")).unwrap();
    for case in &cases {
        eprintln!("complete nonphysical saved skill case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            case.xml.as_bytes(),
        )
        .unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("nonphysicalSkillJit", enabled)?;
            lua.load("if nonphysicalSkillJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("nonphysicalSkillPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@nonphysical-skill-authentication")
                .eval::<Function>()?)
        };
        let after = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("nonphysicalSkillPhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@nonphysical-skill-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let value = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            Some(&install),
            Some(&after),
        );
        observed.push(match value {
            Ok(value)=>{assert_eq!(value["configuration_method_wrappers"],false);assert_eq!(value["original_build_output_available"],true);assert_eq!(value["source_hash"],pinned::manifest_sha256());
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|v|digest(v.as_bytes())),"xml_selected":selected_inventory(&case.xml),"available":true,"state":value["additional_observation"]})},
            Err(error)=>json!({"name":case.name,"available":false,"source_error":error.to_string()}),
        });
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&observed).unwrap(),
        )
        .unwrap();
    }
    for (name, xml) in &originals {
        assert_eq!(fs::read_to_string(fixtures.join(name)).unwrap(), *xml);
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),
        "evidence":{"case_count":cases.len(),"complete_load_attempts_per_jit":cases.len()+2,"observer_sha256":digest(OBSERVE.as_bytes()),
            "business_method_wrappers":false,"synthetic_role_injection":false,"native_coverage":false,"whole_build_parity":false,
            "files":FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
            "originals":originals.iter().map(|(name,xml)|json!({"name":name,"sha256":digest(xml.as_bytes())})).collect::<Vec<_>>()},"cases":observed});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result, &support);
}
fn check(source: &Json, support: &Json) {
    let cases = rows(&source["cases"]);
    assert_eq!(cases.len(), 23);
    for case in cases {
        assert_eq!(
            case["available"], true,
            "{}: {}",
            case["name"], case["source_error"]
        );
        for key in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "saved_specs_preserved",
            "cached_outputs_preserved",
        ] {
            assert_eq!(case["state"][key], true);
        }
        assert_eq!(case["state"]["business_method_wrappers"], false);
        assert_eq!(case["state"]["synthetic_role_injection"], false);
        assert_eq!(
            case["state"]["selected"]["skills"],
            case["xml_selected"]["skill_set"]
        );
        assert_eq!(
            case["state"]["selected_loaded_gems"],
            case["xml_selected"]["children"]
        );
        for call in rows(&case["state"]["support_process_calls"]) {
            assert_eq!(call["target_lists_unchanged"], true);
            nonphysical(&call["gem"]);
        }
    }
    let original = named(cases, "original-03");
    let row = only_loaded(original);
    assert_eq!(original["state"]["selected_loaded_gems"], 62);
    assert_eq!(row["raw_children"][0]["attributes"]["skillId"], EFFECT);
    assert!(row["raw_children"][0]["attributes"].get("gemId").is_none());
    assert_eq!(row["raw_children"][0]["attributes"]["nameSpec"], "");
    nonphysical(&rows(&row["gems"])[0]);
    assert_eq!(row["no_supports_present"], false);
    check_reused(original, 1);
    check_provider(original, &["Physical", "Chaos"]);
    assert!(!rows(&original["state"]["support_process_calls"]).is_empty());
    for name in ["repeat-original-03", "warm-absent-to-original"] {
        same_stable_state(named(cases, name), original);
    }
    let base_count = original["state"]["selected_loaded_physical_supports"]
        .as_u64()
        .unwrap();
    let removed = named(cases, "remove-saved-group");
    assert!(loaded(removed).is_empty());
    let groups = final_groups(removed);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["loaded_group"], false);
    assert_eq!(groups[0]["no_supports_present"], true);
    assert_eq!(groups[0]["no_supports"], true);
    generated_effects(removed);
    for (name, kind) in [
        ("remove-physical-provider-line", "Chaos"),
        ("remove-chaos-provider-line", "Physical"),
    ] {
        let case = named(cases, name);
        check_reused(case, 1);
        check_provider(case, &[kind]);
    }
    for name in [
        "remove-both-provider-lines",
        "unequip-provider",
        "warm-original-to-absent",
    ] {
        let case = named(cases, name);
        assert!(final_groups(case).is_empty());
        assert!(rows(&case["state"]["main"]["providers"]).is_empty());
        for mode in ["main", "calcs"] {
            assert!(rows(&case["state"][mode]["explosion_effects"]).is_empty());
        }
        nonphysical(&rows(&only_loaded(case)["gems"])[0]);
        assert_eq!(
            case["state"]["selected_loaded_physical_supports"],
            base_count
        );
    }
    same_stable_state(
        named(cases, "warm-original-to-absent"),
        named(cases, "remove-both-provider-lines"),
    );
    let misleading = named(cases, "known-effect-misleading-name");
    nonphysical(&rows(&only_loaded(misleading)["gems"])[0]);
    for name in [
        "unknown-effect-blank-name",
        "invalid-gem-id-blocks-effect",
        "empty-gem-id-blocks-effect",
    ] {
        let case = named(cases, name);
        let gem = &rows(&only_loaded(case)["gems"])[0];
        assert_eq!(gem["gem_data_present"], false);
        assert_eq!(gem["direct_effect_present"], false);
        assert!(gem["resolved_effect"].is_null());
        check_reused(case, 1);
    }
    for name in [
        "unknown-effect-support-name",
        "physical-gem-id-wins",
        "invalid-gem-id-support-name",
    ] {
        let case = named(cases, name);
        let gem = &rows(&only_loaded(case)["gems"])[0];
        physical_support(gem, support);
        assert_eq!(
            case["state"]["selected_loaded_physical_supports"],
            base_count + 1
        );
        check_reused(case, 1);
    }
    for name in ["append-physical-support", "unexpected-child-support"] {
        let case = named(cases, name);
        let gems = rows(&only_loaded(case)["gems"]);
        assert_eq!(gems.len(), 2);
        nonphysical(&gems[0]);
        physical_support(&gems[1], support);
        assert_eq!(
            case["state"]["selected_loaded_physical_supports"],
            base_count + 1
        );
        check_reused(case, 1);
    }
    assert_eq!(
        only_loaded(named(cases, "unexpected-child-support"))["raw_children"][1]["element"],
        "UnexpectedSupport"
    );
    let duplicate = named(cases, "duplicate-saved-group");
    assert_eq!(loaded(duplicate).len(), 2);
    check_reused(duplicate, 1);
    assert_eq!(
        duplicate["state"]["selected_loaded_physical_supports"],
        base_count
    );
}
fn nonphysical(gem: &Json) {
    assert_eq!(gem["gem_data_present"], false);
    assert_eq!(gem["direct_effect_present"], true);
    assert_eq!(gem["resolved_effect"], EFFECT);
    assert_eq!(gem["effect_is_source_definition"], true);
    assert_eq!(gem["support_present"], false);
    assert_eq!(gem["support"], false);
    assert_eq!(gem["mapped_gem_by_effect_present"], false);
    assert_eq!(gem["mapped_gem_by_id_present"], false);
}
fn physical_support(gem: &Json, support: &Json) {
    assert_eq!(gem["gem_data_present"], true);
    assert_eq!(gem["support"], true);
    assert_eq!(gem["effect_is_source_definition"], true);
    assert_eq!(gem["resolved_effect"], support["skillId"]);
}
fn check_reused(case: &Json, count: usize) {
    let groups = final_groups(case);
    assert_eq!(groups.len(), count, "{}", case["name"]);
    for group in groups {
        assert_eq!(group["loaded_group"], true);
        assert_eq!(group["no_supports_present"], false);
        assert!(rows(&group["retained_loaded_gems"]).is_empty());
        for gem in rows(&group["gems"]) {
            nonphysical(gem);
            assert_eq!(gem["explode_source_present"], true);
        }
    }
    generated_effects(case);
}
fn generated_effects(case: &Json) {
    for mode in ["main", "calcs"] {
        let effects = rows(&case["state"][mode]["explosion_effects"]);
        assert_eq!(effects.len(), 1, "{} {mode}", case["name"]);
        assert_eq!(effects[0]["effect"], EFFECT);
        nonphysical(&effects[0]["gem"]);
        assert_eq!(effects[0]["gem"]["triggered"], true);
        assert_eq!(effects[0]["gem"]["explode_source"]["id"], 12);
        assert!(rows(&effects[0]["supports"]).is_empty());
    }
}
fn same_stable_state(actual: &Json, expected: &Json) {
    let a = actual["state"].as_object().unwrap();
    let b = expected["state"].as_object().unwrap();
    assert!(
        a.keys().eq(b.keys()),
        "state fields differ for {}",
        actual["name"]
    );
    for (key, value) in a {
        // Warm loads perform fewer cache-preparation passes (measured22→7
        // support calls and18→3 group process events). Keep both raw histories
        // in evidence and validate every observed no-op support call above;
        // compare all loaded/final facts and metrics exactly, not call counts.
        if key == "process_events" || key == "support_process_calls" {
            continue;
        }
        assert!(
            *value == b[key],
            "stable state differs: {}.{key}",
            actual["name"]
        );
    }
}
fn check_provider(case: &Json, kinds: &[&str]) {
    let providers = rows(&case["state"]["main"]["providers"]);
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0]["item_id"], 12);
    let items: Vec<_> = rows(&case["state"]["provider_items"])
        .iter()
        .filter(|item| item["item_id"] == 12)
        .collect();
    assert_eq!(items.len(), 1);
    let mut actual: Vec<_> = rows(&items[0]["mods"])
        .iter()
        .filter(|m| m["name"] == "ExplodeMod")
        .map(|m| m["value"]["type"].as_str().unwrap())
        .collect();
    actual.sort();
    let mut expected = kinds.to_vec();
    expected.sort();
    assert_eq!(actual, expected);
    let flags: Vec<_> = rows(&items[0]["mods"])
        .iter()
        .filter(|m| m["name"] == "CanExplode")
        .collect();
    assert_eq!(flags.len(), kinds.len());
    for modifier in rows(&items[0]["mods"]) {
        assert_eq!(modifier["source"], providers[0]["mod_source"]);
        assert_eq!(modifier["flags"], 0);
        assert_eq!(modifier["keyword_flags"], 0);
        if modifier["name"] == "CanExplode" {
            assert_eq!(modifier["type"], "FLAG");
            assert_eq!(modifier["value"], true);
        } else {
            assert_eq!(modifier["type"], "LIST");
            let (chance, amount) = match modifier["value"]["type"].as_str().unwrap() {
                "Physical" => (31, 10),
                "Chaos" => (16, 25),
                other => panic!("unexpected provider type {other}"),
            };
            assert_eq!(modifier["value"]["value"], chance);
            assert_eq!(modifier["value"]["amount"], amount);
        }
    }
}
fn loaded(case: &Json) -> Vec<&Json> {
    rows(&case["state"]["loaded_groups"])
        .iter()
        .filter(|g| {
            g["skill_set"] == case["state"]["selected"]["skills"] && g["source"] == "Explode"
        })
        .collect()
}
fn only_loaded(case: &Json) -> &Json {
    let groups = loaded(case);
    assert_eq!(groups.len(), 1, "{}", case["name"]);
    groups[0]
}
fn final_groups(case: &Json) -> Vec<&Json> {
    rows(&case["state"]["final_groups"])
        .iter()
        .filter(|g| g["source"] == "Explode")
        .collect()
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|m| m.is_empty()),
            "not source list: {value}"
        );
        &[]
    }
}
fn named<'a>(cases: &'a [Json], name: &str) -> &'a Json {
    let found: Vec<_> = cases.iter().filter(|c| c["name"] == name).collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn push(cases: &mut Vec<Case>, name: &str, xml: String) {
    cases.push(Case {
        name: name.into(),
        xml,
        warm: None,
        original: false,
    });
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn selected_set<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> roxmltree::Node<'a, 'i> {
    let skills = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let id = skills.attribute("activeSkillSet").unwrap();
    skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(id))
        .unwrap()
}
fn explode_group<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> roxmltree::Node<'a, 'i> {
    let groups: Vec<_> = selected_set(doc)
        .children()
        .filter(|n| n.has_tag_name("Skill") && n.attribute("source") == Some("Explode"))
        .collect();
    assert_eq!(groups.len(), 1);
    groups[0]
}
fn selected_support(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = selected_set(&doc)
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem") && n.attribute("skillId") == Some("SupportMagnifiedAreaPlayerTwo")
        })
        .unwrap();
    Json::Object(
        gem.attributes()
            .map(|a| (a.name().into(), json!(a.value())))
            .collect(),
    )
}
fn change_gem(xml: &str, changes: &[(&str, &str)]) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = explode_group(&doc)
        .children()
        .find(|n| n.has_tag_name("Gem"))
        .unwrap();
    let mut attrs: std::collections::BTreeMap<_, _> =
        gem.attributes().map(|a| (a.name(), a.value())).collect();
    for (key, value) in changes {
        attrs.insert(key, value);
    }
    let text = attrs
        .into_iter()
        .map(|(key, value)| format!("{key}=\"{}\"", escape(value)))
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = xml.to_owned();
    out.replace_range(gem.range(), &format!("<Gem {text}/>"));
    out
}
fn remove_group(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut out = xml.to_owned();
    out.replace_range(explode_group(&doc).range(), "");
    out
}
fn duplicate_group(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let group = explode_group(&doc);
    let end = selected_set(&doc).range().end - "</SkillSet>".len();
    let mut out = xml.to_owned();
    out.insert_str(end, &xml[group.range()]);
    out
}
fn append_support(xml: &str, unexpected: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let support = selected_set(&doc)
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem") && n.attribute("skillId") == Some("SupportMagnifiedAreaPlayerTwo")
        })
        .unwrap();
    let raw = &xml[support.range()];
    let child = if unexpected {
        raw.replacen("<Gem ", "<UnexpectedSupport ", 1)
    } else {
        raw.to_owned()
    };
    let end = explode_group(&doc).range().end - "</Skill>".len();
    let mut out = xml.to_owned();
    out.insert_str(end, &child);
    out
}
fn remove_line(xml: &str, line: &str) -> String {
    assert_eq!(xml.matches(line).count(), 1);
    xml.replacen(line, "", 1)
}
fn unequip_provider(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let id = items.attribute("activeItemSet").unwrap();
    let set = items
        .children()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some(id))
        .unwrap();
    let slot = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Gloves"))
        .unwrap();
    assert_eq!(slot.attribute("itemId"), Some("12"));
    let attrs = slot
        .attributes()
        .map(|a| {
            format!(
                "{}=\"{}\"",
                a.name(),
                escape(if a.name() == "itemId" { "0" } else { a.value() })
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = xml.to_owned();
    out.replace_range(slot.range(), &format!("<Slot {attrs}/>"));
    out
}
fn selected_inventory(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let groups: Vec<_> = set.children().filter(|n| n.is_element()).collect();
    let children = groups
        .iter()
        .map(|g| g.children().filter(|n| n.is_element()).count())
        .sum::<usize>();
    json!({"skill_set":set.attribute("id").unwrap().parse::<u32>().unwrap(),"groups":groups.len(),"children":children})
}
fn tail(path: &Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let len = file.metadata().unwrap().len();
    file.seek(SeekFrom::Start(len.saturating_sub(16 * 1024)))
        .unwrap();
    let mut bytes = Vec::new();
    file.take(16 * 1024).read_to_end(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}
