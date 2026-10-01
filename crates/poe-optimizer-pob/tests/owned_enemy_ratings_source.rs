//! Full unchanged-source evidence for configured enemy Armour/Evasion BASE.
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
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

const TEST: &str = "complete_enemy_ratings_follow_source_level_and_raw_input_precedence";
const CHILD: &str = "POE_ENEMY_RATINGS_SOURCE_CHILD";
// The committed observer retains original method identity and full local state checks.
const OBSERVE: &str = include_str!("support/configuration_inputs_source.lua");
const PINNED_FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/PopupDialog.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/Data.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/EditControl.lua",
    "src/Modules/ModParser.lua",
    "src/Data/BossSkills.lua",
    "src/Data/Bosses.lua",
    "src/Data/Misc.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Data/QuestRewards.lua",
    "runtime/lua/xml.lua",
];

#[derive(Clone)]
struct Case {
    name: String,
    xml: String,
    level: u16,
    raw: Option<[f64; 2]>,
    pinnacle: bool,
}

#[test]
fn complete_enemy_ratings_follow_source_level_and_raw_input_precedence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-enemy-ratings-source-01");
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
                    "source child failed; log {}; evidence {}\n{}",
                    path.display(),
                    out.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "source deadline; log {}; evidence {}\n{}",
                    path.display(),
                    out.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    same(
        &read(&out.join("source-jit-off.json")),
        &read(&out.join("source-jit-on.json")),
        "exact scoped JIT evidence",
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
            let entry = rows(&index["builds"])
                .iter()
                .find(|r| r["xml"] == filename)
                .unwrap();
            assert_eq!(
                entry["xml_sha256"],
                digest(xml.as_bytes()),
                "fixture {filename}"
            );
            Case {
                name: format!("original-{n:02}"),
                xml,
                level: 82,
                raw: None,
                pinnacle: true,
            }
        })
        .collect();
    let mut inputs = originals.clone();
    inputs.extend(controls(&originals[4].xml));
    assert_eq!(inputs.len(), 17);
    let mut cases = Vec::new();
    for case in &inputs {
        eprintln!("complete enemy ratings source case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals()
                .set("configurationInputsXml", case.xml.as_str())?;
            lua.globals().set("configurationInputsJit", enabled)?;
            lua.load("if configurationInputsJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let diagnostics = Rc::new(RefCell::new(None::<Json>));
        let install = |lua: &Lua| {
            lua.globals().set("configurationInputsPhase", "before")?;
            let cleanup: Function = lua
                .load(OBSERVE)
                .set_name("@enemy-ratings-shared-config-observer")
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
            &case.xml,
            None,
            false,
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        let load = diagnostics.borrow_mut().take();
        let row = match result {
            Ok(value) => {
                assert!(load.is_some(), "{}: cleanup not completed", case.name);
                assert_eq!(
                    value["configuration_method_wrappers"], false,
                    "{}",
                    case.name
                );
                assert_eq!(
                    value["original_build_output_available"], true,
                    "{}",
                    case.name
                );
                assert_eq!(
                    value["source_hash"],
                    pinned::manifest_sha256(),
                    "{}",
                    case.name
                );
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"raw_rating_fields":raw_fields(&case.xml),"expected_selections":selections(&case.xml),"available":true,"load":load,"state":value["additional_observation"]})
            }
            Err(error) => {
                json!({"name":case.name,"available":false,"load":load,"source_error":error.to_string()})
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
    for (n, original) in originals.iter().enumerate() {
        assert_eq!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", n + 1))).unwrap(),
            original.xml,
            "original fixture bytes"
        );
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),"evidence":{
        "complete_load_attempts_per_jit":17,"full_controls":12,"pinnacle_parity_cases":16,"pinnacle_base_comparisons":32,
        "native_effect_coverage":false,"whole_configuration_coverage":false,"business_method_wrappers":false,
        "non_pinnacle_case":"non-pinnacle-placeholders","non_pinnacle_native_parity":false,
        "shared_observer":"tests/support/configuration_inputs_source.lua","shared_observer_sha256":digest(OBSERVE.as_bytes()),
        "originals":originals.iter().map(|c|json!({"name":c.name,"sha256":digest(c.xml.as_bytes())})).collect::<Vec<_>>(),
        "files":PINNED_FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>()},"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result, &inputs);
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("configurationInputsPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@enemy-ratings-shared-config-observer")
        .eval()?;
    let mut result: Json = lua.from_value(value)?;
    // Full source catalogue authentication has already run. Keep the three
    // actual descriptors for this component in its scoped cross-run artifact.
    result["catalogue"].as_array_mut().unwrap().retain(|r| {
        matches!(
            r["definition"]["var"].as_str(),
            Some("enemyIsBoss" | "enemyArmour" | "enemyEvasion")
        )
    });
    Ok(result)
}

fn controls(original: &str) -> Vec<Case> {
    let clean = remove_keys(original, &["enemyLevel", "enemyArmour", "enemyEvasion"]);
    let mut cases: Vec<_> = [20, 82, 83, 84, 85]
        .into_iter()
        .map(|level| Case {
            name: format!("level-{level}"),
            xml: add(
                &clean,
                &format!("<Input name=\"enemyLevel\" number=\"{level}\"/>"),
            ),
            level,
            raw: None,
            pinnacle: true,
        })
        .collect();
    let level83 = add(&clean, "<Input name=\"enemyLevel\" number=\"83\"/>");
    for (name, values) in [
        ("raw-zero", [0.0, 0.0]),
        ("raw-negative", [-123.0, -456.0]),
        ("raw-high", [1_000_000.0, 999_999.0]),
        ("raw-fractional", [12.5, 34.25]),
    ] {
        cases.push(Case { name:name.into(), xml:add(&level83, &format!("<Input name=\"enemyArmour\" number=\"{}\"/><Input name=\"enemyEvasion\" number=\"{}\"/>", values[0], values[1])), level:83, raw:Some(values), pinnacle:true });
    }
    let placeholders = add(
        &level83,
        "<Placeholder name=\"enemyArmour\" number=\"7\"/><Placeholder name=\"enemyEvasion\" number=\"11\"/>",
    );
    cases.push(Case {
        name: "pinnacle-placeholders".into(),
        xml: placeholders.clone(),
        level: 83,
        raw: None,
        pinnacle: true,
    });
    cases.push(Case { name:"input-over-placeholder".into(), xml:add(&placeholders,"<Input name=\"enemyArmour\" number=\"17\"/><Input name=\"enemyEvasion\" number=\"29\"/>"), level:83, raw:Some([17.0,29.0]), pinnacle:true });
    cases.push(Case {
        name: "non-pinnacle-placeholders".into(),
        xml: add(
            &remove_keys(&placeholders, &["enemyIsBoss"]),
            "<Input name=\"enemyIsBoss\" string=\"None\"/>",
        ),
        level: 83,
        raw: None,
        pinnacle: false,
    });
    assert_eq!(cases.len(), 12);
    cases
}
fn remove_keys(xml: &str, keys: &[&str]) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut ranges: Vec<_> = doc
        .descendants()
        .filter(|n| {
            (n.has_tag_name("Input") || n.has_tag_name("Placeholder"))
                && n.attribute("name").is_some_and(|key| keys.contains(&key))
        })
        .map(|n| n.range())
        .collect();
    ranges.sort_by_key(|r| r.start);
    let mut result = xml.to_owned();
    for range in ranges.into_iter().rev() {
        result.replace_range(range, "");
    }
    result
}
fn add(xml: &str, body: &str) -> String {
    assert_eq!(xml.matches("</ConfigSet>").count(), 1);
    xml.replacen("</ConfigSet>", &format!("{body}</ConfigSet>"), 1)
}
fn selections(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut output = serde_json::Map::new();
    for (key, element, attribute) in [
        ("config", "Config", "activeConfigSet"),
        ("items", "Items", "activeItemSet"),
        ("skills", "Skills", "activeSkillSet"),
        ("spec", "Tree", "activeSpec"),
    ] {
        let node = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name(element))
            .unwrap();
        output.insert(
            key.into(),
            json!(node.attribute(attribute).unwrap().parse::<usize>().unwrap()),
        );
    }
    Json::Object(output)
}
fn raw_fields(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let entries: Vec<_> = doc
        .descendants()
        .filter(|n| {
            (n.has_tag_name("Input") || n.has_tag_name("Placeholder"))
                && matches!(
                    n.attribute("name"),
                    Some("enemyLevel" | "enemyArmour" | "enemyEvasion" | "enemyIsBoss")
                )
        })
        .map(|n| {
            let attributes: std::collections::BTreeMap<_, _> = n
                .attributes()
                .map(|a| {
                    (
                        a.name(),
                        poe_optimizer_import::source_xml::attribute(n, a.name())
                            .unwrap()
                            .unwrap()
                            .decoded()
                            .to_owned(),
                    )
                })
                .collect();
            json!({"element":n.tag_name().name(),"attributes":attributes})
        })
        .collect();
    json!(entries)
}

fn check(result: &Json, cases: &[Case]) {
    let evidence = &result["cases"];
    assert_eq!(rows(evidence).len(), 17);
    let original_data = &evidence[0]["state"]["source_data"];
    let tables = rows(&original_data["levels"]);
    assert_eq!(tables.len(), 85);
    for (index, row) in tables.iter().enumerate() {
        assert_eq!(row["level"], index + 1);
        for key in ["armour", "evasion"] {
            assert!(
                row[key].as_f64().is_some_and(|n| n > 0. && n.is_finite()),
                "raw table {key} {}",
                index + 1
            );
        }
    }
    for (case, row) in cases.iter().zip(rows(evidence)) {
        let name = &case.name;
        assert_eq!(row["name"], *name);
        assert_eq!(row["available"], true, "{name}: {}", row["source_error"]);
        let state = &row["state"];
        let saved = &state["saved"];
        for flag in [
            "methods_preserved",
            "objects_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(state[flag], true, "{name} {flag}");
        }
        assert_eq!(state["business_wrappers"], false, "{name}");
        same(
            &state["source_data"],
            original_data,
            &format!("{name} raw tables/means"),
        );
        same(
            &state["catalogue"],
            &evidence[0]["state"]["catalogue"],
            &format!("{name} source descriptors"),
        );
        same(
            &saved["selected"],
            &row["expected_selections"],
            &format!("{name} untouched selections"),
        );
        assert_eq!(saved["enemy_level"], case.level, "{name} Config level");
        assert_eq!(
            saved["input"]["enemyIsBoss"],
            if case.pinnacle { "Pinnacle" } else { "None" },
            "{name}"
        );
        let descriptors = rows(&state["catalogue"]);
        assert_eq!(descriptors.len(), 3, "{name} scoped descriptor census");
        for key in ["enemyArmour", "enemyEvasion"] {
            let descriptor = &descriptors
                .iter()
                .find(|d| d["definition"]["var"] == key)
                .unwrap()["definition"];
            assert_eq!(descriptor["type"], "countAllowZero", "{name} {key}");
            assert!(
                descriptor["apply"]["function_source"]["path"]
                    .as_str()
                    .unwrap()
                    .ends_with("Modules/ConfigOptions.lua"),
                "{name} {key} original callback"
            );
        }
        for (index, (key, stat, table, mean)) in [
            ("enemyArmour", "Armour", "armour", "pinnacle_armour_mean"),
            (
                "enemyEvasion",
                "Evasion",
                "evasion",
                "pinnacle_evasion_mean",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let factor = original_data[mean].as_f64().unwrap() / 100.0;
            assert!(factor.is_finite() && factor > 0.0, "{name} {mean}");
            let default = (tables[usize::from(case.level.max(82)) - 1][table]
                .as_f64()
                .unwrap()
                * factor
                + 0.5)
                .floor();
            let placeholder = if case.pinnacle {
                default
            } else {
                // ConfigOptions' None branch also resets saved placeholders,
                // using the raw table at the actual level without a boss mean.
                tables[usize::from(case.level) - 1][table].as_f64().unwrap()
            };
            assert_eq!(
                saved["placeholder"][key].as_f64(),
                Some(placeholder),
                "{name} {key} source placeholder"
            );
            match case.raw {
                Some(raw) => assert_eq!(
                    saved["input"][key].as_f64(),
                    Some(raw[index]),
                    "{name} exact raw {key}"
                ),
                None => assert!(
                    saved["input"].get(key).is_none(),
                    "{name} {key} absence is not zero"
                ),
            };
            let expected = case.raw.map_or(placeholder, |raw| raw[index]);
            let matches: Vec<_> = rows(&saved["enemy_mods"])
                .iter()
                .filter(|m| m["name"] == stat && m["source"] == "Config")
                .collect();
            assert_eq!(matches.len(), 1, "{name} Config {stat} exact occurrence");
            let actual = matches[0];
            assert_eq!(actual["type"], "BASE", "{name} {stat}");
            assert_eq!(
                actual["value"].as_f64(),
                Some(expected),
                "{name} Config {stat} formula/raw priority"
            );
            assert_eq!(actual["flags"], 0, "{name} {stat}");
            assert_eq!(actual["keyword_flags"], 0, "{name} {stat}");
            assert!(
                rows(&actual["tags"]).is_empty(),
                "{name} {stat} unconditional raw BASE"
            );
            for mode in ["MAIN", "CALCS"] {
                let observed = &saved["modes"][mode];
                assert_eq!(
                    observed["enemy_level"], case.level,
                    "{name} {mode} actor level"
                );
                assert_eq!(
                    observed["enemy_base"][stat]["config"].as_f64(),
                    Some(expected),
                    "{name} {mode} {stat} Config reduction"
                );
                assert_eq!(
                    observed["enemy_base"][stat]["enemy_config"].as_f64(),
                    Some(0.0),
                    "{name} {mode} {stat} distinct source"
                );
                let found: Vec<_> = rows(&observed["enemy_delivery"]["records"])
                    .iter()
                    .filter(|r| r["record"]["name"] == stat && r["record"]["source"] == "Config")
                    .collect();
                assert_eq!(found.len(), 1, "{name} {mode} {stat} actual delivery");
                same(
                    &found[0]["record"],
                    actual,
                    &format!("{name} {mode} {stat} unchanged record"),
                );
            }
        }
        for mode in ["MAIN", "CALCS"] {
            let delivered = &saved["modes"][mode]["enemy_delivery"];
            assert_eq!(
                rows(&delivered["records"]).len(),
                rows(&saved["enemy_mods"]).len(),
                "{name} {mode} complete Config delivery"
            );
            let mut seen = BTreeSet::new();
            for (index, join) in rows(&delivered["joins"]).iter().enumerate() {
                assert_eq!(join["config_index"], index + 1, "{name} {mode}");
                let position = join["matched_index"].as_u64().unwrap() as usize;
                assert!(seen.insert(position), "{name} {mode} injective join");
                same(
                    &delivered["records"][position - 1]["record"],
                    &saved["enemy_mods"][index],
                    &format!("{name} {mode} Config delivery {index}"),
                );
            }
        }
    }
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else if value.as_object().is_some_and(|r| r.is_empty()) {
        &[]
    } else {
        panic!("expected sequence; inspect retained evidence")
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
        .take(30)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.chars().take(600).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
fn same(a: &Json, b: &Json, context: &str) {
    fn first(a: &Json, b: &Json, path: String) -> String {
        if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
            for key in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
                if a.get(key) != b.get(key) {
                    return first(
                        a.get(key).unwrap_or(&Json::Null),
                        b.get(key).unwrap_or(&Json::Null),
                        format!("{path}.{key}"),
                    );
                }
            }
        } else if let (Some(a), Some(b)) = (a.as_array(), b.as_array()) {
            if a.len() != b.len() {
                return format!("{path}: array lengths {} != {}", a.len(), b.len());
            }
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                if a != b {
                    return first(a, b, format!("{path}[{index}]"));
                }
            }
        }
        format!(
            "{path}: {} != {}",
            a.to_string().chars().take(180).collect::<String>(),
            b.to_string().chars().take(180).collect::<String>()
        )
    }
    assert!(
        a == b,
        "{context}: {}; full evidence retained",
        first(a, b, "$".into())
    );
}
