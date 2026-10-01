//! Complete source evidence for character-only selection inventory, separate from Config rewards.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

const TEST: &str = "fresh_character_loaders_keep_configuration_rewards_and_quest_budgets_separate";
const CHILD: &str = "POE_CHARACTER_REWARD_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/character_reward_source.lua");

#[test]
fn fresh_character_loaders_keep_configuration_rewards_and_quest_budgets_separate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-character-reward-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let log_path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
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
                    "source child failed; log {}; evidence {}\n{}",
                    log_path.display(),
                    out.join(format!("source-jit-{mode}.json")).display(),
                    tail(&log_path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "source deadline {}; retained evidence {}\n{}",
                    log_path.display(),
                    out.display(),
                    tail(&log_path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        read(&out.join("source-jit-off.json")) == read(&out.join("source-jit-on.json")),
        "JIT source evidence differs; inspect retained JSON files"
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index = read(&fixtures.join("index.json"));
    let facts = read(&root.join("data/owned/poe2/3887ae68/import/reward-source-facts.json"));
    let mut originals = Vec::new();
    for n in 1..=5 {
        let name = format!("build-{n:02}.xml");
        let text = fs::read_to_string(fixtures.join(&name)).unwrap();
        let entry = rows(&index["builds"])
            .iter()
            .find(|r| r["xml"] == name)
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(text.as_bytes()));
        originals.push((format!("original-{n:02}"), text));
    }
    let mut inputs = originals.clone();
    inputs.extend(controls(&originals, &facts));
    assert_eq!(inputs.len(), 16);
    let mut cases = Vec::new();
    for (name, text) in inputs {
        eprintln!("complete character source case {name}");
        let before = |lua: &Lua| {
            lua.globals().set("characterRewardXml", text.as_str())?;
            lua.globals().set("characterRewardJit", enabled)?;
            lua.load("if characterRewardJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let diagnostics = Rc::new(RefCell::new(None::<Json>));
        let install = |lua: &Lua| {
            lua.globals().set("characterRewardPhase", "before")?;
            let cleanup: Function = lua
                .load(OBSERVE)
                .set_name("@character-reward-source-observer")
                .eval()?;
            let diagnostics = Rc::clone(&diagnostics);
            Ok(lua.create_function(move |lua, ()| {
                let value: Value = cleanup.call(())?;
                *diagnostics.borrow_mut() = Some(lua.from_value(value)?);
                Ok(())
            })?)
        };
        let temp = tempfile::tempdir().unwrap();
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &text,
            None,
            name.starts_with("excluded-"),
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        assert!(
            diagnostics.borrow().is_some(),
            "{name} missing method cleanup: {}",
            result
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default()
        );
        let load = diagnostics.borrow_mut().take().unwrap();
        let row = match result {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false, "{name}");
                assert_eq!(value["original_build_output_available"], true, "{name}");
                assert_eq!(value["source_hash"], pinned::manifest_sha256(), "{name}");
                json!({"name":name,"xml_sha256":digest(text.as_bytes()),"available":true,"load":load,"state":value["additional_observation"]})
            }
            Err(error) => {
                assert!(matches!(error, RuntimeError::Lua(_)), "{name}: {error}");
                json!({"name":name,"xml_sha256":digest(text.as_bytes()),"available":false,"load":load,"source_error":error.to_string()})
            }
        };
        cases.push(row);
        // Preserve the completed prefix if a later complete load fails.
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
    for (n, (_, original)) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read_to_string(fixtures.join(format!("build-{:02}.xml", n + 1))).unwrap(),
            original
        );
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4", "source_hash":pinned::manifest_sha256(), "evidence":{
        "complete_load_attempts_per_jit":16,"originals":originals.iter().map(|(name,text)|json!({"name":name,"sha256":digest(text.as_bytes()),"source_census":xml_census(text)})).collect::<Vec<_>>(),
        "full_controls":11,"native_effect_coverage":false,"business_method_wrappers":false,"synthetic_reward_collection":false,
        "reward_metadata_sha256":digest(&fs::read(root.join("data/owned/poe2/3887ae68/import/reward-source-facts.json")).unwrap()),
        "files":(["src/Launch.lua","src/GameVersions.lua","src/Modules/Common.lua","src/Modules/Main.lua","src/Modules/Build.lua","src/Classes/TreeTab.lua","src/Classes/PassiveSpec.lua","src/Classes/ConfigTab.lua","src/Classes/ImportTab.lua","src/Classes/CalcsTab.lua","src/Classes/PopupDialog.lua","src/Modules/CalcSetup.lua","src/Modules/ConfigOptions.lua","src/Data/QuestRewards.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))},"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result, &facts);
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("characterRewardPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@character-reward-source-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn controls(originals: &[(String, String)], facts: &Json) -> Vec<(String, String)> {
    let xml = &originals[4].1;
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .descendants()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some("1"))
        .unwrap();
    let cuts = config
        .children()
        .filter(|n| {
            n.has_tag_name("Input") && n.attribute("name").is_some_and(|s| s.starts_with("quest"))
        })
        .map(|n| (n.range(), String::new()))
        .collect();
    let without = patch(xml, cuts);
    let supplied = rows(&facts["rows"])
        .iter()
        .map(|r| {
            let key = escape(r["config_key"].as_str().unwrap());
            if r["choice"]["kind"] == "boolean" {
                format!("<Input name=\"{key}\" boolean=\"false\"/>")
            } else {
                format!("<Input name=\"{key}\" string=\"None\"/>")
            }
        })
        .collect::<String>();
    let none = without.replacen("</ConfigSet>", &format!("{supplied}</ConfigSet>"), 1);
    let mut cached = Vec::new();
    for row in doc
        .descendants()
        .filter(|n| n.has_tag_name("PlayerStat") || n.has_tag_name("MinionStat"))
    {
        cached.push((
            row.range(),
            format!(
                "<{} stat=\"{}\" value=\"123456\"/>",
                row.tag_name().name(),
                escape(row.attribute("stat").unwrap())
            ),
        ));
    }
    let buffs = doc.descendants().find(|n| n.has_tag_name("Buffs")).unwrap();
    cached.push((buffs.range(),"<Buffs buffList=\"untrusted cached buff\" combatList=\"cached combat\" curseList=\"cached curse\"/>".into()));
    let timeless = doc
        .descendants()
        .find(|n| n.has_tag_name("TimelessData"))
        .unwrap();
    let timeless=patch(xml,vec![(timeless.range(),"<TimelessData devotionVariant1=\"2\" devotionVariant2=\"3\" searchList=\"Cold Resistance\" searchListFallback=\"Armour\" socketFilterDistance=\"7\"/>".into())]);
    let beast_doc = roxmltree::Document::parse(&originals[1].1).unwrap();
    let beast = beast_doc
        .descendants()
        .find(|n| n.has_tag_name("BeastCompanion"))
        .unwrap();
    let tree = doc.descendants().find(|n| n.has_tag_name("Tree")).unwrap();
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(2)
        .unwrap();
    let legacy = patch(xml, vec![(tree.range(), xml[spec.range()].to_owned())]);
    let unknown = set_attr(
        &set_attr(xml, "Build", 0, "bandit", "Alira"),
        "Spec",
        2,
        "reward",
        "unknown-character-choice",
    );
    vec![
        ("config-rewards-none".into(), none),
        ("cached-output-buffs".into(), patch(xml, cached)),
        ("timeless-nonreward-state".into(), timeless),
        (
            "beast-companion-removed".into(),
            patch(&originals[1].1, vec![(beast.range(), String::new())]),
        ),
        (
            "selected-spec-one".into(),
            set_attr(xml, "Tree", 0, "activeSpec", "1"),
        ),
        ("excluded-reward-looking-fields".into(), unknown),
        (
            "excluded-auto-level".into(),
            set_attr(xml, "Build", 0, "characterLevelAutoMode", "true"),
        ),
        (
            "excluded-url-only".into(),
            remove_attr(xml, "Spec", 2, "nodes"),
        ),
        ("excluded-legacy-root-spec".into(), legacy),
        (
            "excluded-target-missing".into(),
            remove_attr(xml, "Build", 0, "targetVersion"),
        ),
        (
            "excluded-target-changed".into(),
            set_attr(xml, "Build", 0, "targetVersion", "0_0"),
        ),
    ]
}
fn set_attr(xml: &str, tag: &str, index: usize, name: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let node = doc
        .descendants()
        .filter(|n| n.has_tag_name(tag))
        .nth(index)
        .unwrap();
    let start = node.range().start;
    let end = start + xml[start..].find('>').unwrap();
    let opening = &xml[start..end];
    let next = if let Some(old) = node.attribute(name) {
        let literal = format!("{name}=\"{}\"", escape(old));
        assert_eq!(opening.matches(&literal).count(), 1);
        opening.replacen(&literal, &format!("{name}=\"{}\"", escape(value)), 1)
    } else {
        format!("{opening} {name}=\"{}\"", escape(value))
    };
    patch(xml, vec![(start..end, next)])
}
fn remove_attr(xml: &str, tag: &str, index: usize, name: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let node = doc
        .descendants()
        .filter(|n| n.has_tag_name(tag))
        .nth(index)
        .unwrap();
    let start = node.range().start;
    let end = start + xml[start..].find('>').unwrap();
    let old = format!(" {name}=\"{}\"", escape(node.attribute(name).unwrap()));
    let opening = &xml[start..end];
    assert_eq!(opening.matches(&old).count(), 1);
    patch(xml, vec![(start..end, opening.replacen(&old, "", 1))])
}
fn patch(xml: &str, mut changes: Vec<(Range<usize>, String)>) -> String {
    changes.sort_by_key(|(range, _)| range.start);
    for pair in changes.windows(2) {
        assert!(pair[0].0.end <= pair[1].0.start)
    }
    let mut out = xml.to_owned();
    for (range, text) in changes.into_iter().rev() {
        out.replace_range(range, &text)
    }
    out
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn xml_census(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut build_children = BTreeMap::<String, usize>::new();
    let mut specs = Vec::new();
    let mut build = None;
    for (ordinal, node) in doc.descendants().filter(|n| n.is_element()).enumerate() {
        if node.has_tag_name("Build") {
            build = Some(
                json!({"source":ordinal,"attributes":node.attributes().map(|a|(a.name(),a.value())).collect::<BTreeMap<_,_>>()}),
            );
            for child in node.children().filter(|n| n.is_element()) {
                *build_children
                    .entry(child.tag_name().name().into())
                    .or_default() += 1
            }
        }
        if node.has_tag_name("Spec") {
            specs.push(json!({"source":ordinal,"attributes":node.attributes().map(|a|(a.name(),a.value())).collect::<BTreeMap<_,_>>()}))
        }
    }
    json!({"build":build,"build_children":build_children,"specs":specs})
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else if value.as_object().is_some_and(|r| r.is_empty()) {
        &[]
    } else {
        panic!("expected array: {value}")
    }
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(50)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}
fn case<'a>(result: &'a Json, name: &str) -> &'a Json {
    rows(&result["cases"])
        .iter()
        .find(|r| r["name"] == name)
        .unwrap()
}
fn state<'a>(result: &'a Json, name: &str) -> &'a Json {
    let row = case(result, name);
    assert_eq!(row["available"], true, "{name}: {}", row["source_error"]);
    &row["state"]
}
fn metadata(actual: &Json, typed: &Json) {
    match typed["kind"].as_str().unwrap() {
        "number" => assert_eq!(actual.as_f64(), typed["value"].as_f64()),
        "text" | "boolean" => assert_eq!(actual, &typed["value"]),
        "array" => {
            assert_eq!(rows(actual).len(), rows(&typed["value"]).len());
            for (a, b) in rows(actual).iter().zip(rows(&typed["value"])) {
                metadata(a, b)
            }
        }
        kind => panic!("unreviewed metadata {kind}"),
    }
}
fn check(result: &Json, facts: &Json) {
    assert_eq!(rows(&result["cases"]).len(), 16);
    let mut spec_count = 0;
    let mut children = BTreeMap::<String, u64>::new();
    for (index, (level, specs, selected)) in
        [(96, 1, 1), (88, 6, 6), (93, 1, 1), (96, 1, 1), (92, 7, 3)]
            .into_iter()
            .enumerate()
    {
        let name = format!("original-{:02}", index + 1);
        let state = state(result, &name);
        let saved = &state["saved"];
        let census = &result["evidence"]["originals"][index]["source_census"];
        assert_eq!(saved["character"]["level"], level, "{name}");
        assert_eq!(saved["character"]["auto"], false, "{name}");
        assert_eq!(saved["selected"]["spec"], selected, "{name}");
        assert_eq!(rows(&saved["specs"]).len(), specs, "{name}");
        spec_count += specs;
        assert_eq!(
            state["raw"]["builds"][0]["source"], census["build"]["source"],
            "{name}"
        );
        assert_eq!(
            state["raw"]["builds"][0]["xml"]["attrib"], census["build"]["attributes"],
            "{name}"
        );
        let raw_specs = rows(&state["raw"]["trees"][0]["specs"]);
        assert_eq!(raw_specs.len(), specs, "{name}");
        for (actual, expected) in raw_specs.iter().zip(rows(&census["specs"])) {
            assert_eq!(actual["source"], expected["source"], "{name}");
            assert_eq!(actual["xml"]["attrib"], expected["attributes"], "{name}")
        }
        for (key, value) in census["build_children"].as_object().unwrap() {
            *children.entry(key.clone()).or_default() += value.as_u64().unwrap()
        }
        assert!(!rows(&saved["config_quest"]).is_empty(), "{name}");
        for mode in ["MAIN", "CALCS"] {
            assert!(
                !rows(&saved["modes"][mode]["quest"]).is_empty(),
                "{name} {mode}"
            )
        }
    }
    assert_eq!(spec_count, 16);
    assert_eq!(
        children,
        BTreeMap::from([
            ("PlayerStat".into(), 488),
            ("MinionStat".into(), 37),
            ("Buffs".into(), 5),
            ("BeastCompanion".into(), 1),
            ("TimelessData".into(), 5)
        ])
    );
    for row in rows(&result["cases"]) {
        let name = row["name"].as_str().unwrap();
        assert_eq!(row["load"]["methods_preserved"], true, "{name}");
        assert_eq!(row["load"]["live_target_version"], "0_1", "{name}");
        if name.starts_with("excluded-target-") {
            assert_eq!(row["available"], false, "{name} incorrectly completed");
            assert_eq!(
                row["load"]["popup_titles"],
                json!(["Game Version"]),
                "{name}"
            );
            assert_eq!(row["load"]["loaded_target_version_type"], "nil", "{name}");
            assert!(
                row["source_error"]
                    .as_str()
                    .is_some_and(|s| s.contains("assertion failed")),
                "{name}: {}",
                row["source_error"]
            );
            continue;
        }
        let state = state(result, name);
        assert_eq!(row["load"]["loaded_target_version"], "0_1", "{name}");
        for flag in [
            "methods_preserved",
            "objects_preserved",
            "outputs_and_selection_preserved",
        ] {
            assert_eq!(state[flag], true, "{name} {flag}")
        }
        assert_eq!(state["business_method_wrappers"], false, "{name}");
        assert_eq!(state["synthetic_reward_collection"], false, "{name}");
        let catalog = rows(&state["quest_catalogue"]);
        assert_eq!(catalog.len(), 29, "{name}");
        let budget: Vec<_> = catalog
            .iter()
            .filter(|q| q["quest"]["useConfig"] == false)
            .collect();
        assert_eq!(budget.len(), 12, "{name}");
        assert!(
            budget
                .iter()
                .all(|q| q["config_control_present"] == false && q["quest"]["questPoints"] == 2),
            "{name}"
        );
        assert_eq!(state["saved"]["budget"]["max_weapon_sets"], 24, "{name}");
        for fact in rows(&facts["rows"]) {
            let actual = catalog
                .iter()
                .find(|q| q["key"] == fact["config_key"])
                .unwrap();
            assert_eq!(actual["source_index"], fact["source_index"], "{name}");
            assert_eq!(actual["config_control_present"], true, "{name}");
            let expected = fact["source_record"].as_object().unwrap();
            assert_eq!(
                actual["quest"].as_object().unwrap().len(),
                expected.len(),
                "{name}"
            );
            for (key, value) in expected {
                metadata(&actual["quest"][key], value)
            }
        }
    }
    let base = &state(result, "original-05")["saved"];
    let none = &state(result, "config-rewards-none")["saved"];
    for field in ["character", "specs", "budget", "selected"] {
        assert_eq!(
            none[field], base[field],
            "Config-only mutation changed {field}"
        )
    }
    assert!(rows(&none["config_quest"]).is_empty());
    assert!(!rows(&base["config_quest"]).is_empty());
    for mode in ["MAIN", "CALCS"] {
        assert!(rows(&none["modes"][mode]["quest"]).is_empty(), "{mode}")
    }
    assert_eq!(
        state(result, "cached-output-buffs")["saved"],
        *base,
        "cached display data changed live source state"
    );
    let timeless = &state(result, "timeless-nonreward-state")["saved"];
    assert_eq!(timeless["timeless"]["devotionVariant1"], 2);
    assert_eq!(timeless["timeless"]["devotionVariant2"], 3);
    assert_eq!(timeless["timeless"]["searchList"], "Cold Resistance");
    assert_eq!(timeless["timeless"]["socketFilterDistance"], 7);
    for field in ["character", "budget", "config_quest"] {
        assert_eq!(timeless[field], base[field], "TimelessData changed {field}")
    }
    let beast_base = &state(result, "original-02")["saved"];
    let beast = &state(result, "beast-companion-removed")["saved"];
    assert_eq!(rows(&beast_base["beasts"]).len(), 1);
    assert!(rows(&beast["beasts"]).is_empty());
    for field in ["character", "budget", "config_quest"] {
        assert_eq!(
            beast[field], beast_base[field],
            "BeastCompanion changed {field}"
        )
    }
    let switched = &state(result, "selected-spec-one")["saved"];
    assert_eq!(switched["selected"]["spec"], 1);
    assert_eq!(switched["character"]["level"], 92);
    assert_eq!(switched["character"]["auto"], false);
    for field in ["budget", "config_quest", "config_inputs"] {
        assert_eq!(
            switched[field], base[field],
            "Spec selection changed {field}"
        )
    }
    let unknown = &state(result, "excluded-reward-looking-fields")["saved"];
    assert!(
        unknown == base,
        "unknown fields changed source state; excluded profile requires review"
    );
    assert_eq!(
        state(result, "excluded-auto-level")["saved"]["character"]["auto"],
        true
    );
    assert!(
        state(result, "excluded-url-only")["raw"]["trees"][0]["specs"][2]["xml"]["attrib"]["nodes"]
            .is_null()
    );
    let legacy = state(result, "excluded-legacy-root-spec");
    assert_eq!(rows(&legacy["raw"]["legacy_specs"]).len(), 1);
    assert_eq!(rows(&legacy["saved"]["specs"]).len(), 1);
}
