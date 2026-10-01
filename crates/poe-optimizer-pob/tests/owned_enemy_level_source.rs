//! Fresh complete configuration lifecycle and final source enemy actors.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
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
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

const TEST: &str = "fresh_config_callbacks_preserve_enemy_level_and_selected_scope";
const CHILD: &str = "POE_ENEMY_LEVEL_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/enemy_level_source.lua");
const PLACEHOLDER: &str = "<Placeholder number=\"82\" name=\"enemyLevel\"/>";

#[test]
fn fresh_config_callbacks_preserve_enemy_level_and_selected_scope() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-enemy-level-source-01");
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
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let manifest = read(&fixtures.join("index.json"));
    let mut originals = vec![];
    let mut source_joins = vec![];
    for (index, ordinal) in [411, 458, 211, 390, 438].into_iter().enumerate() {
        let file = format!("build-{:02}.xml", index + 1);
        let xml = fs::read_to_string(fixtures.join(&file)).unwrap();
        let entry = manifest["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["xml"] == file)
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([61; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let found: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Placeholder"
                    && r.attribute("name").and_then(|a| a.decoded().ok()) == Some("enemyLevel")
            })
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].occurrence().id().ordinal(), ordinal);
        assert_eq!(
            found[0].attribute("number").and_then(|a| a.decoded().ok()),
            Some("82")
        );
        source_joins.push(json!({"build":index+1,"placeholder_source":ordinal,"xml_sha256":digest(xml.as_bytes())}));
        originals.push((format!("original-{:02}", index + 1), xml));
    }
    let mut inputs = originals.clone();
    inputs.extend(controls(&originals[4].1));
    assert_eq!(inputs.len(), 29);
    let mut cases = vec![];
    let mut source_hash = None;
    for (name, text) in inputs {
        let before = |lua: &Lua| {
            lua.globals().set("enemyLevelXml", text.as_str())?;
            lua.globals().set("enemyLevelJit", enabled)?;
            lua.load("if enemyLevelJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let diagnostics = Rc::new(RefCell::new(None::<Json>));
        let before_build = |lua: &Lua| {
            lua.globals().set("enemyLevelPhase", "before")?;
            let cleanup: Function = lua
                .load(OBSERVE)
                .set_name("@enemy-level-source-observer")
                .eval()?;
            let diagnostics = Rc::clone(&diagnostics);
            // Wrap only our observer cleanup, never a source business method.
            Ok(lua.create_function(move |lua, ()| {
                let captured: Value = cleanup.call(())?;
                *diagnostics.borrow_mut() = Some(lua.from_value(captured)?);
                Ok(())
            })?)
        };
        let temp = tempfile::tempdir().unwrap();
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &text,
            None,
            !name.starts_with("original-"),
            Some(&before),
            Some(&before_build),
            Some(&observe),
        );
        assert!(
            diagnostics.borrow().is_some(),
            "original method identity cleanup not completed for {name}: {}",
            result
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_else(|| "load unexpectedly succeeded without cleanup".into())
        );
        let source_load = diagnostics.borrow_mut().take().unwrap();
        let case = match result {
            Ok(result) => {
                assert_eq!(result["configuration_method_wrappers"], false);
                assert_eq!(result["original_build_output_available"], true);
                if let Some(hash) = &source_hash {
                    assert_eq!(&result["source_hash"], hash);
                } else {
                    source_hash = Some(result["source_hash"].clone());
                }
                assert!(
                    source_load["prompt"].is_null(),
                    "source prompt for {name}: {source_load}"
                );
                json!({"name":name,"xml_sha256":digest(text.as_bytes()),"available":true,"source_load":source_load,"state":result["additional_observation"]})
            }
            Err(error) => {
                assert!(
                    matches!(error, RuntimeError::Lua(_)),
                    "unexpected error class for {name}: {error}"
                );
                json!({"name":name,"xml_sha256":digest(text.as_bytes()),"available":false,"source_load":source_load,"source_error":error.to_string()})
            }
        };
        cases.push(case);
    }
    for (index, (_, xml)) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read_to_string(fixtures.join(format!("build-{:02}.xml", index + 1))).unwrap(),
            xml
        );
    }
    let result = json!({
        "source_hash":source_hash,
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "evidence":{
            "manifest_sha256":pinned::manifest_sha256(),
            "originals":source_joins,
            "complete_load_attempts":29,
            "controls":24,
            "configuration_method_wrappers":false,
            "native_parity":false,
            "files":(["src/Launch.lua","src/Modules/Common.lua","src/Classes/ConfigTab.lua","src/Classes/EditControl.lua","src/Modules/ConfigOptions.lua","src/Modules/Build.lua","src/Modules/CalcSetup.lua","src/Modules/Data.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))
        },
        "cases":cases
    });
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

fn controls(xml: &str) -> Vec<(String, String)> {
    let mut cases = vec![
        ("placeholder-missing".into(), replace(xml, PLACEHOLDER, "")),
        (
            "placeholder-seven".into(),
            replace(
                xml,
                PLACEHOLDER,
                "<Placeholder number=\"7\" name=\"enemyLevel\"/>",
            ),
        ),
    ];
    for (name, value) in [
        ("boss-none", "None"),
        ("boss-boss", "Boss"),
        ("boss-uber", "Uber"),
        ("boss-legacy-shaper", "shaper"),
    ] {
        cases.push((
            name.into(),
            add_row(
                xml,
                &format!("<Input name=\"enemyIsBoss\" string=\"{value}\"/>"),
            ),
        ));
    }
    for (name, value) in [
        ("explicit-twenty", "20"),
        ("explicit-hundred", "100"),
        ("explicit-zero", "0"),
        ("explicit-negative", "-1"),
        ("explicit-malformed", "not-a-number"),
        ("explicit-fraction", "82.5"),
    ] {
        cases.push((
            name.into(),
            add_row(
                xml,
                &format!("<Input name=\"enemyLevel\" number=\"{value}\"/>"),
            ),
        ));
    }
    for (name, body) in [
        (
            "duplicate-twenty-forty",
            "<Input name=\"enemyLevel\" number=\"20\"/><Input name=\"enemyLevel\" number=\"40\"/>",
        ),
        (
            "duplicate-forty-twenty",
            "<Input name=\"enemyLevel\" number=\"40\"/><Input name=\"enemyLevel\" number=\"20\"/>",
        ),
        (
            "numeric-and-string",
            "<Input name=\"enemyLevel\" number=\"20\" string=\"40\"/>",
        ),
        ("input-string", "<Input name=\"enemyLevel\" string=\"40\"/>"),
        (
            "input-boolean",
            "<Input name=\"enemyLevel\" boolean=\"true\"/>",
        ),
        (
            "namespaced-input",
            "<e:Input xmlns:e=\"urn:owned-test\" name=\"enemyLevel\" number=\"20\"/>",
        ),
    ] {
        cases.push((name.into(), add_row(xml, body)));
    }
    cases.push((
        "placeholder-string".into(),
        replace(
            xml,
            PLACEHOLDER,
            "<Placeholder name=\"enemyLevel\" string=\"40\"/>",
        ),
    ));
    cases.push((
        "default-namespace-config".into(),
        replace(
            xml,
            "<Config activeConfigSet=\"1\">",
            "<Config xmlns=\"urn:owned-test\" activeConfigSet=\"1\">",
        ),
    ));
    let with_two = replace(
        xml,
        "</Config>",
        "<ConfigSet id=\"2\" title=\"Archive control\"><Input name=\"enemyLevel\" number=\"20\"/></ConfigSet></Config>",
    );
    cases.push(("archived-twenty".into(), with_two.clone()));
    cases.push((
        "selected-two".into(),
        replace(
            &with_two,
            "<Config activeConfigSet=\"1\">",
            "<Config activeConfigSet=\"2\">",
        ),
    ));
    cases.push((
        "unknown-selected".into(),
        replace(
            &with_two,
            "<Config activeConfigSet=\"1\">",
            "<Config activeConfigSet=\"99\">",
        ),
    ));
    cases.push(("duplicate-config-id".into(), replace(xml, "</Config>", "<ConfigSet id=\"1\" title=\"Replacement control\"><Input name=\"enemyLevel\" number=\"20\"/></ConfigSet></Config>")));
    assert_eq!(cases.len(), 24);
    cases
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("enemyLevelPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@enemy-level-source-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn add_row(xml: &str, row: &str) -> String {
    replace(xml, "</ConfigSet>", &format!("{row}</ConfigSet>"))
}
fn replace(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "{old}");
    text.replace(old, new)
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn case<'a>(result: &'a Json, name: &str) -> &'a Json {
    let found: Vec<_> = result["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["name"] == name)
        .collect();
    assert_eq!(found.len(), 1, "{name}");
    found[0]
}
fn state<'a>(result: &'a Json, name: &str) -> &'a Json {
    let c = case(result, name);
    assert_eq!(c["available"], true, "{name}: {}", c["source_error"]);
    &c["state"]
}
fn level(result: &Json, name: &str, expected: u64) {
    let state = state(result, name);
    assert_eq!(state["config_level"], expected, "{name}");
    assert_eq!(state["config_level_type"], "number");
    assert_eq!(state["original_methods_preserved"], true);
    assert_eq!(state["saved_sets_preserved"], true);
    assert_eq!(state["method_wrappers"], false);
    for mode in ["MAIN", "CALCS"] {
        let actor = &state["modes"][mode];
        assert_eq!(actor["available"], true);
        assert_eq!(actor["enemy_level"], expected, "{name}/{mode}");
        assert_eq!(actor["actor_level"], expected, "{name}/{mode}");
        assert_eq!(actor["actor_level_type"], "number");
        for key in [
            "input_alias",
            "placeholder_alias",
            "enemy_actor_alias",
            "outputs_available",
        ] {
            assert_eq!(actor[key], true);
        }
    }
}
fn check(result: &Json) {
    assert_eq!(result["cases"].as_array().unwrap().len(), 29);
    for index in 1..=5 {
        let name = format!("original-{index:02}");
        level(result, &name, 82);
        let s = state(result, &name);
        assert_eq!(s["selected"]["config"], 1);
        assert_eq!(s["input"]["enemyIsBoss"], "Pinnacle");
        assert!(s["input"]["enemyLevel"].is_null());
        assert_eq!(s["placeholder"]["enemyLevel"], 82);
        assert_eq!(s["definitions"]["boss_default_index"], 3);
        assert_eq!(s["definitions"]["boss_default_value"], "Pinnacle");
        assert_eq!(s["source_data"]["maximum_enemy_level"], 85);
        let configs = s["raw_config"].as_array().unwrap();
        assert_eq!(configs.len(), 1);
        let sets = configs[0]["sets"].as_array().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0]["attributes"]["id"], "1");
        let entries = sets[0]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["element"], "Placeholder");
        assert_eq!(entries[0]["attributes"]["number"], "82");
    }
    for name in [
        "placeholder-missing",
        "placeholder-seven",
        "boss-uber",
        "boss-legacy-shaper",
        "explicit-zero",
        "explicit-negative",
        "explicit-malformed",
        "namespaced-input",
        "default-namespace-config",
        "archived-twenty",
        "unknown-selected",
    ] {
        level(result, name, 82);
    }
    for name in ["boss-none", "boss-boss", "explicit-hundred"] {
        level(result, name, 85);
    }
    for name in [
        "explicit-twenty",
        "duplicate-forty-twenty",
        "numeric-and-string",
        "selected-two",
        "duplicate-config-id",
    ] {
        level(result, name, 20);
    }
    level(result, "duplicate-twenty-forty", 40);
    for name in ["input-string", "input-boolean", "placeholder-string"] {
        let c = case(result, name);
        assert_eq!(c["available"], false, "{name}");
        assert_eq!(c["source_load"]["original_methods_preserved"], true);
        let error = c["source_load"]["prompt"].as_str().unwrap();
        assert!(
            error.contains("In 'OnFrame'")
                && error.contains("ConfigTab.lua:1160")
                && error.contains("compare"),
            "{name}: {error}"
        );
    }
    let fractional = case(result, "explicit-fraction");
    assert_eq!(fractional["available"], false);
    assert_eq!(
        fractional["source_load"]["original_methods_preserved"],
        true
    );
    let error = fractional["source_load"]["prompt"].as_str().unwrap();
    assert!(
        error.contains("In 'OnFrame'")
            && error.contains("ConfigOptions.lua")
            && error.contains("arithmetic"),
        "{error}"
    );
    // The callback's Pinnacle damage floor is not an actor-level floor.
    let baseline = state(result, "original-05");
    let lower_actor = state(result, "explicit-twenty");
    assert_eq!(lower_actor["input"]["enemyLevel"], 20);
    for key in ["enemyPhysicalDamage", "enemyArmour", "enemyEvasion"] {
        assert!(baseline["placeholder"][key].is_number());
        assert_eq!(
            baseline["placeholder"][key],
            lower_actor["placeholder"][key]
        );
    }
    for name in ["placeholder-missing", "placeholder-seven"] {
        assert_eq!(state(result, name)["placeholder"]["enemyLevel"], 82);
    }
    for name in ["boss-none", "boss-boss"] {
        let s = state(result, name);
        assert_eq!(s["placeholder"]["enemyLevel"], s["character_level"]);
        assert!(s["character_level"].as_u64().unwrap() > 85);
    }
    assert_eq!(
        state(result, "boss-legacy-shaper")["input"]["enemyIsBoss"],
        "Pinnacle"
    );
    for name in ["archived-twenty", "selected-two", "unknown-selected"] {
        let s = state(result, name);
        let sets = s["sets"].as_array().unwrap();
        assert_eq!(sets.len(), 2);
        assert!(sets[0]["input"]["enemyLevel"].is_null());
        assert_eq!(sets[1]["input"]["enemyLevel"], 20);
        assert_eq!(
            s["selected"]["config"],
            if name == "selected-two" { 2 } else { 1 }
        );
        for key in ["items", "skills", "spec"] {
            assert_eq!(s["selected"][key], baseline["selected"][key]);
        }
    }
}
