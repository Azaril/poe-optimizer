//! Complete source evidence for saved and effective Config inputs and original downstream records.
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
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

const TEST: &str = "complete_configuration_inputs_preserve_source_lifecycle_and_delivery";
const CHILD: &str = "POE_CONFIGURATION_INPUTS_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/configuration_inputs_source.lua");
const RESISTANCE_FIELDS: [&str; 4] = [
    "enemyFireResist",
    "enemyColdResist",
    "enemyLightningResist",
    "enemyChaosResist",
];
const RESISTANCE_PLACEHOLDERS: [f64; 4] = [12345.5, -123.25, 99.5, 777.0];
const RESISTANCE_INPUTS: [f64; 4] = [91.5, -12.25, 37.5, 123.75];
type ResistanceControl = (&'static str, Option<[f64; 4]>, Option<[f64; 4]>);

#[test]
fn complete_configuration_inputs_preserve_source_lifecycle_and_delivery() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let output = std::env::var_os("POE_CONFIGURATION_INPUTS_SOURCE_OUT");
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let out = root.join(output.expect("parent must select the evidence directory"));
        assert!(out.is_dir(), "parent must create the evidence directory");
        run_child(&root, &out, mode == "on");
        return;
    }
    let out = if let Some(path) = output {
        let out = root.join(path);
        assert!(
            !out.exists(),
            "refusing to overwrite source evidence at {}; set POE_CONFIGURATION_INPUTS_SOURCE_OUT to a fresh directory",
            out.display()
        );
        fs::create_dir_all(&out).unwrap();
        out
    } else {
        let runs = root.join("runs");
        fs::create_dir_all(&runs).unwrap();
        tempfile::Builder::new()
            .prefix("owned-configuration-inputs-source-")
            .tempdir_in(runs)
            .unwrap()
            .keep()
    };
    eprintln!("configuration source evidence: {}", out.display());
    for mode in ["off", "on"] {
        let log_path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env("POE_CONFIGURATION_INPUTS_SOURCE_OUT", &out)
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
    same(
        &read(&out.join("source-jit-off.json")),
        &read(&out.join("source-jit-on.json")),
        "JIT source evidence differs; full JSON retained",
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
    assert_eq!(inputs.len(), 29);
    let mut cases = Vec::new();
    for (name, text) in inputs {
        eprintln!("complete configuration source case {name}");
        let before = |lua: &Lua| {
            lua.globals().set("configurationInputsXml", text.as_str())?;
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
                .set_name("@configuration-inputs-source-observer")
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
            // Originals retain the evaluator preflight. Controlled structural
            // contrasts use the existing decoded/bounded original-loader path,
            // so namespace tolerance is observed rather than guessed from the
            // deliberately stricter compatibility adapter.
            !name.starts_with("original-"),
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
        "complete_load_attempts_per_jit":29,"originals":originals.iter().map(|(name,text)|json!({"name":name,"sha256":digest(text.as_bytes()),"source_census":xml_census(text)})).collect::<Vec<_>>(),
        "full_controls":24,"native_effect_coverage":false,"business_method_wrappers":false,"callback_invocation_trace":false,
        "observer_sha256":digest(OBSERVE.as_bytes()),
        "reward_metadata_sha256":digest(&fs::read(root.join("data/owned/poe2/3887ae68/import/reward-source-facts.json")).unwrap()),
        "files":(["src/Launch.lua","src/GameVersions.lua","src/Modules/Common.lua","src/Modules/Main.lua","src/Modules/Build.lua","src/Classes/ConfigTab.lua","src/Classes/CalcsTab.lua","src/Classes/PopupDialog.lua","src/Modules/CalcSetup.lua","src/Modules/CalcDefence.lua","src/Modules/CalcOffence.lua","src/Modules/Data.lua","src/Classes/ModList.lua","src/Classes/ModDB.lua","src/Classes/ModStore.lua","src/Classes/EditControl.lua","src/Modules/ModParser.lua","src/Data/BossSkills.lua","src/Data/Bosses.lua","src/Data/Misc.lua","src/Modules/ConfigOptions.lua","src/Data/QuestRewards.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))},"cases":cases});
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
    lua.globals().set("configurationInputsPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@configuration-inputs-source-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}

fn controls(originals: &[(String, String)], _facts: &Json) -> Vec<(String, String)> {
    let xml = &originals[4].1;
    let keys = [
        "multiplierLinkedTargets",
        "enemyBlockChance",
        "linkedSourceRate",
        "enemyPhysicalReduction",
        "enemyColdResist",
    ];
    let clean = remove_keys(xml, &keys);
    let placeholders = "<Placeholder name=\"multiplierLinkedTargets\" number=\"7\"/><Placeholder name=\"enemyBlockChance\" number=\"13\"/><Placeholder name=\"linkedSourceRate\" number=\"1.25\"/><Placeholder name=\"enemyPhysicalReduction\" number=\"-13\"/><Placeholder name=\"enemyColdResist\" number=\"7\"/>";
    let numeric = add(&clean, placeholders);
    let mut cases = vec![
        ("numeric-placeholders".into(), numeric.clone()),
        (
            "numeric-input-priority".into(),
            add(
                &numeric,
                "<Input name=\"multiplierLinkedTargets\" number=\"3\"/><Input name=\"enemyBlockChance\" number=\"11\"/><Input name=\"linkedSourceRate\" number=\"2.5\"/><Input name=\"enemyPhysicalReduction\" number=\"-11\"/><Input name=\"enemyColdResist\" number=\"50\"/>",
            ),
        ),
        (
            "numeric-zero".into(),
            add(
                &numeric,
                &keys
                    .iter()
                    .map(|key| format!("<Input name=\"{key}\" number=\"0\"/>"))
                    .collect::<String>(),
            ),
        ),
        ("numeric-missing".into(), clean.clone()),
        (
            "boss-none".into(),
            add(xml, "<Input name=\"enemyIsBoss\" string=\"None\"/>"),
        ),
        (
            "boss-preset-size".into(),
            add(
                xml,
                "<Input name=\"enemyIsBoss\" string=\"Uber\"/><Input name=\"presetBossSkills\" string=\"Shaper Ball\"/><Input name=\"enemySizePreset\" string=\"Large\"/>",
            ),
        ),
        (
            "placeholder-string-and-alias".into(),
            add(
                xml,
                "<Placeholder name=\"enemyIsBoss\" string=\"Shaper\"/><Input name=\"presetBossSkills\" string=\"Uber Shaper Ball\"/>",
            ),
        ),
        (
            "duplicate-unknown-namespace".into(),
            add(
                &numeric,
                "<Input name=\"multiplierLinkedTargets\" number=\"2\"/><Input name=\"multiplierLinkedTargets\" number=\"9\" string=\"17\"/><Input name=\"unreviewedConfiguration\" number=\"73\"/><Placeholder name=\"unreviewedPlaceholder\" number=\"19\"/><e:Input xmlns:e=\"urn:owned-test\" name=\"multiplierLinkedTargets\" number=\"99\"/>",
            ),
        ),
        (
            "numeric-malformed-last".into(),
            add(
                &numeric,
                "<Input name=\"multiplierLinkedTargets\" number=\"2\"/><Input name=\"multiplierLinkedTargets\" number=\"bad\"/><Input name=\"enemyColdResist\" number=\"90\"/>",
            ),
        ),
        (
            "custom-enabled".into(),
            add(
                xml,
                "<CustomModifierBlock title=\"Witness\" enabled=\"true\">+17 to maximum Life</CustomModifierBlock>",
            ),
        ),
        (
            "custom-disabled".into(),
            add(
                xml,
                "<CustomModifierBlock title=\"Witness\" enabled=\"false\">+17 to maximum Life</CustomModifierBlock>",
            ),
        ),
        (
            "custom-legacy".into(),
            add(
                xml,
                "<Input name=\"customMods\" string=\"+17 to maximum Life\"/>",
            ),
        ),
        (
            "custom-block-over-legacy".into(),
            add(
                xml,
                "<Input name=\"customMods\" string=\"+91 to maximum Life\"/><CustomModifierBlock title=\"Witness\" enabled=\"true\">+17 to maximum Life</CustomModifierBlock>",
            ),
        ),
    ];
    let archive = xml.replacen("</Config>", "<ConfigSet id=\"2\" title=\"Witness archive\"><Input name=\"multiplierLinkedTargets\" number=\"9\"/></ConfigSet></Config>", 1);
    cases.push(("archived-set".into(), archive.clone()));
    cases.push((
        "selected-set".into(),
        archive.replace("activeConfigSet=\"1\"", "activeConfigSet=\"2\""),
    ));
    cases.push((
        "unknown-selected-set".into(),
        archive.replace("activeConfigSet=\"1\"", "activeConfigSet=\"99\""),
    ));
    cases.push((
        "direct-damage-zero".into(),
        add(xml, "<Input name=\"enemyPhysicalDamage\" number=\"0\"/>"),
    ));
    cases.push((
        "direct-damage-explicit".into(),
        add(xml, "<Input name=\"enemyPhysicalDamage\" number=\"125\"/>"),
    ));
    let resistance_clean = remove_keys(xml, &RESISTANCE_FIELDS);
    for (name, placeholders, inputs) in resistance_controls() {
        let mut body = String::new();
        for (index, key) in RESISTANCE_FIELDS.iter().enumerate() {
            if let Some(values) = placeholders {
                body.push_str(&format!(
                    "<Placeholder name=\"{key}\" number=\"{}\"/>",
                    values[index]
                ));
            }
            if let Some(values) = inputs {
                body.push_str(&format!(
                    "<Input name=\"{key}\" number=\"{}\"/>",
                    values[index]
                ));
            }
        }
        cases.push((name.into(), add(&resistance_clean, &body)));
    }
    assert_eq!(cases.len(), 24);
    cases
}
fn resistance_controls() -> [ResistanceControl; 6] {
    [
        (
            "resistance-changed-placeholders",
            Some(RESISTANCE_PLACEHOLDERS),
            None,
        ),
        ("resistance-missing-placeholders", None, None),
        (
            "resistance-zero-inputs-with-placeholders",
            Some(RESISTANCE_PLACEHOLDERS),
            Some([0.; 4]),
        ),
        (
            "resistance-signed-inputs-with-placeholders",
            Some(RESISTANCE_PLACEHOLDERS),
            Some(RESISTANCE_INPUTS),
        ),
        (
            "resistance-zero-inputs-without-placeholders",
            None,
            Some([0.; 4]),
        ),
        (
            "resistance-signed-inputs-without-placeholders",
            None,
            Some(RESISTANCE_INPUTS),
        ),
    ]
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
    let mut output = xml.to_owned();
    for range in ranges.into_iter().rev() {
        output.replace_range(range, "");
    }
    output
}
fn add(xml: &str, body: &str) -> String {
    assert_eq!(xml.matches("</ConfigSet>").count(), 1);
    xml.replacen("</ConfigSet>", &format!("{body}</ConfigSet>"), 1)
}
fn xml_census(xml: &str) -> Json {
    fn attributes(node: roxmltree::Node<'_, '_>) -> BTreeMap<String, String> {
        node.attributes()
            .map(|attribute| {
                // PoB preserves literal attribute whitespace. roxmltree's decoded
                // value normalizes LF/tab, including significant quest list text.
                let value = poe_optimizer_import::source_xml::attribute(node, attribute.name())
                    .unwrap()
                    .unwrap();
                (attribute.name().to_owned(), value.decoded().to_owned())
            })
            .collect()
    }
    fn parsed_entry(node: roxmltree::Node<'_, '_>) -> Json {
        use poe_optimizer_import::source_xml::{PobContentEntry, ordered_content};

        let mut fragments_left = node.range().len();
        let mut text_bytes_left = node.range().len();
        let content = ordered_content(node, &mut fragments_left, &mut text_bytes_left).unwrap();
        let mut expected = serde_json::Map::from_iter([
            ("elem".to_owned(), json!(node.tag_name().name())),
            ("attrib".to_owned(), json!(attributes(node))),
        ]);
        for (index, entry) in content.consumed().iter().enumerate() {
            let PobContentEntry::Text { text, .. } = entry else {
                panic!("original Config entry unexpectedly contains an element");
            };
            expected.insert((index + 1).to_string(), json!(text));
        }
        if content.consumed().is_empty() {
            expected.insert("empty".to_owned(), json!(true));
        }
        Json::Object(expected)
    }
    let doc = roxmltree::Document::parse(xml).unwrap();
    let all: Vec<_> = doc.descendants().filter(|n| n.is_element()).collect();
    let configs:Vec<_>=all.iter().enumerate().filter(|(_,n)|n.has_tag_name("Config")).map(|(index,n)|{
        let sets:Vec<_>=n.children().filter(|n|n.is_element()).map(|set|{
            let entries:Vec<_>=set.children().filter(|n|n.is_element()).map(|entry|json!({"source":all.iter().position(|n|*n==entry).unwrap(),"element":entry.tag_name().name(),"attributes":attributes(entry),"text":entry.text(),"parsed_xml":parsed_entry(entry)})).collect();
            json!({"source":all.iter().position(|n|n==&set).unwrap(),"element":set.tag_name().name(),"attributes":attributes(set),"entries":entries})
        }).collect();
        json!({"source":index,"attributes":attributes(*n),"sets":sets})
    }).collect();
    json!(configs)
}
fn rows(value: &Json) -> &[Json] {
    if let Some(r) = value.as_array() {
        r
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
        .take(35)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.chars().take(700).collect::<String>())
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
fn records<'a>(saved: &'a Json, target: &str, name: &str, source: &str) -> Vec<&'a Json> {
    rows(&saved[target])
        .iter()
        .filter(|m| m["name"] == name && m["source"] == source)
        .collect()
}
fn value(saved: &Json, target: &str, name: &str, source: &str) -> Json {
    let found = records(saved, target, name, source);
    assert_eq!(found.len(), 1, "{target} {name} {source}: occurrence count");
    found[0]["value"].clone()
}
fn same(a: &Json, b: &Json, context: &str) {
    assert!(a == b, "{context}: {}", difference_summary(a, b));
}
fn difference_summary(a: &Json, b: &Json) -> String {
    fn brief(value: Option<&Json>) -> String {
        match value {
            None => "missing".to_owned(),
            Some(Json::Null) => "null".to_owned(),
            Some(Json::Bool(v)) => format!("boolean {v}"),
            Some(Json::Number(v)) => format!("number {v}"),
            Some(Json::String(v)) => {
                format!("string {:?}", v.chars().take(120).collect::<String>())
            }
            Some(Json::Array(v)) => format!("array length {}", v.len()),
            Some(Json::Object(v)) => format!("object keys {}", v.len()),
        }
    }
    fn visit(a: &Json, b: &Json, path: &str, differences: &mut Vec<String>) {
        if a == b || differences.len() >= 8 {
            return;
        }
        match (a, b) {
            (Json::Object(a), Json::Object(b)) => {
                let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
                for key in keys {
                    if differences.len() >= 8 {
                        break;
                    }
                    let path = format!("{path}.{key}");
                    match (a.get(key), b.get(key)) {
                        (Some(a), Some(b)) => visit(a, b, &path, differences),
                        (a, b) => differences.push(format!("{path}: {} != {}", brief(a), brief(b))),
                    }
                }
            }
            (Json::Array(a), Json::Array(b)) => {
                if a.len() != b.len() {
                    differences.push(format!("{path}: array lengths {} != {}", a.len(), b.len()));
                }
                for (index, (a, b)) in a.iter().zip(b).enumerate() {
                    if differences.len() >= 8 {
                        break;
                    }
                    visit(a, b, &format!("{path}[{index}]"), differences);
                }
            }
            _ => differences.push(format!("{path}: {} != {}", brief(Some(a)), brief(Some(b)))),
        }
    }
    let mut differences = Vec::new();
    visit(a, b, "$", &mut differences);
    differences.join("; ")
}
fn metadata(actual: &Json, typed: &Json) {
    match typed["kind"].as_str().unwrap() {
        "number" => assert_eq!(actual.as_f64(), typed["value"].as_f64()),
        "text" | "boolean" => same(actual, &typed["value"], "reward metadata"),
        "array" => {
            assert_eq!(rows(actual).len(), rows(&typed["value"]).len());
            for (a, b) in rows(actual).iter().zip(rows(&typed["value"])) {
                metadata(a, b)
            }
        }
        kind => panic!("unreviewed typed metadata {kind}"),
    }
}
fn check_resistance_controls(result: &Json) {
    let baseline = state(result, "original-05");
    let base = &baseline["saved"];
    let defaults = [50., 50., 50., 0.];
    for ((key, first), default) in RESISTANCE_FIELDS
        .iter()
        .zip([2171, 2168, 2165, 2174])
        .zip(defaults)
    {
        let definitions: Vec<_> = rows(&baseline["catalogue"])
            .iter()
            .filter(|row| row["definition"]["var"] == *key)
            .collect();
        assert_eq!(definitions.len(), 1, "unique resistance definition {key}");
        let definition = &definitions[0]["definition"];
        assert_eq!(definition["type"], "countAllowZero");
        assert!(definition["defaultState"].is_null());
        assert!(definition["defaultPlaceholderState"].is_null());
        assert_eq!(definition["apply"]["function_source"]["first"], first);
        assert!(base["input"].as_object().unwrap().get(*key).is_none());
        assert_eq!(base["placeholder"][key].as_f64(), Some(default));
    }
    for (name, placeholders, inputs) in resistance_controls() {
        let observed = state(result, name);
        let saved = &observed["saved"];
        let configs = rows(&observed["raw"]);
        assert_eq!(configs.len(), 1, "{name} Config census");
        let sets = rows(&configs[0]["sets"]);
        assert_eq!(sets.len(), 1, "{name} ConfigSet census");
        assert_eq!(sets[0]["attributes"]["id"], "1");
        assert_eq!(saved["input"]["enemyIsBoss"], "Pinnacle");
        let mut expected_input = base["input"].clone();
        for (index, key) in RESISTANCE_FIELDS.iter().enumerate() {
            // Authenticate both saved lanes independently. Missing numeric Input
            // stays absent; an overwritten Placeholder never supplies that lane.
            for (element, values) in [("Placeholder", placeholders), ("Input", inputs)] {
                let entries: Vec<_> = rows(&sets[0]["entries"])
                    .iter()
                    .filter(|entry| {
                        entry["xml"]["elem"] == element && entry["xml"]["attrib"]["name"] == *key
                    })
                    .collect();
                assert_eq!(
                    entries.len(),
                    usize::from(values.is_some()),
                    "{name} {key} {element}"
                );
                if let Some(values) = values {
                    let attributes = &entries[0]["xml"]["attrib"];
                    assert_eq!(attributes.as_object().unwrap().len(), 2);
                    assert_eq!(
                        attributes["number"]
                            .as_str()
                            .unwrap()
                            .parse::<f64>()
                            .unwrap(),
                        values[index],
                        "{name} {key} actual saved {element}"
                    );
                }
            }
            let expected_raw = inputs.map(|values| values[index]);
            if let Some(raw) = expected_raw {
                assert_eq!(
                    saved["input"][key].as_f64(),
                    Some(raw),
                    "{name} {key} raw Input"
                );
                expected_input[*key] = saved["input"][key].clone();
            } else {
                assert!(
                    saved["input"].as_object().unwrap().get(*key).is_none(),
                    "{name} {key} must remain absent"
                );
            }
            assert_eq!(
                saved["placeholder"][key].as_f64(),
                Some(defaults[index]),
                "{name} {key} callback overwrites the saved Placeholder"
            );
            let stat = key.strip_prefix("enemy").unwrap();
            let expected = expected_raw.unwrap_or(defaults[index]);
            let actual = records(saved, "enemy_mods", stat, "EnemyConfig");
            let original = records(base, "enemy_mods", stat, "EnemyConfig");
            assert_eq!(actual.len(), 1, "{name} {stat} contribution count");
            assert_eq!(original.len(), 1, "baseline {stat} contribution count");
            assert_eq!(actual[0]["type"], "BASE");
            assert_eq!(
                actual[0]["value"].as_f64(),
                Some(expected),
                "{name} {stat} contribution value"
            );
            let mut expected_record = original[0].clone();
            expected_record["value"] = actual[0]["value"].clone();
            expected_record["all_fields"]["value"] = actual[0]["value"].clone();
            same(
                actual[0],
                &expected_record,
                &format!("{name} {stat} exact contribution"),
            );
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(saved["modes"][mode]["input_alias"], true);
                assert_eq!(saved["modes"][mode]["placeholder_alias"], true);
                assert_eq!(
                    saved["modes"][mode]["enemy_base"][stat]["enemy_config"].as_f64(),
                    Some(expected),
                    "{name} {mode} {stat} source-owned BASE"
                );
            }
        }
        same(
            &saved["input"],
            &expected_input,
            &format!("{name} exact effective Input map"),
        );
        for field in [
            "placeholder",
            "selected",
            "control_defaults",
            "enemy_level",
            "player_mods",
        ] {
            same(
                &saved[field],
                &base[field],
                &format!("{name} stable Config {field}"),
            );
        }
        for field in ["input", "placeholder"] {
            same(
                &case(result, name)["load"][field],
                &saved[field],
                &format!("{name} load/observation {field}"),
            );
        }
    }
    // Paired runs vary only saved Placeholder presence. Compare the configuration
    // contract, not unrelated whole-build output or final resistance semantics.
    for (with, without) in [
        (
            "resistance-changed-placeholders",
            "resistance-missing-placeholders",
        ),
        (
            "resistance-zero-inputs-with-placeholders",
            "resistance-zero-inputs-without-placeholders",
        ),
        (
            "resistance-signed-inputs-with-placeholders",
            "resistance-signed-inputs-without-placeholders",
        ),
    ] {
        for field in [
            "input",
            "placeholder",
            "selected",
            "sets",
            "player_mods",
            "enemy_mods",
        ] {
            same(
                &state(result, with)["saved"][field],
                &state(result, without)["saved"][field],
                &format!("{with}/{without} {field}"),
            );
        }
    }
}
fn check(result: &Json, facts: &Json) {
    assert_eq!(rows(&result["cases"]).len(), 29);
    let catalogue = &state(result, "original-05")["catalogue"];
    assert!(
        rows(catalogue).len() > 300,
        "complete Config catalogue unexpectedly small"
    );
    for case in rows(&result["cases"]) {
        let name = case["name"].as_str().unwrap();
        let state = state(result, name);
        let saved = &state["saved"];
        for flag in [
            "methods_preserved",
            "objects_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(state[flag], true, "{name} {flag}");
        }
        assert_eq!(state["business_wrappers"], false, "{name}");
        assert_eq!(state["callback_invocation_trace"], false, "{name}");
        assert_eq!(
            state["delivery_scope"], "injectively_matched_actual_config_records_in_config_order",
            "{name}"
        );
        assert_eq!(
            state["unrelated_global_database_order_excluded"], true,
            "{name}"
        );
        same(&state["catalogue"], catalogue, &format!("{name} catalogue"));
        for mode in ["MAIN", "CALCS"] {
            for (target, field) in [
                ("player_mods", "player_delivery"),
                ("enemy_mods", "enemy_delivery"),
            ] {
                let delivered = &saved["modes"][mode][field];
                assert_eq!(
                    rows(&delivered["joins"]).len(),
                    rows(&saved[target]).len(),
                    "{name} {mode} {field}"
                );
                assert_eq!(
                    rows(&delivered["records"]).len(),
                    rows(&saved[target]).len(),
                    "{name} {mode} {field} exact matched count"
                );
                let mut seen = std::collections::BTreeSet::new();
                for (index, join) in rows(&delivered["joins"]).iter().enumerate() {
                    assert_eq!(join["config_index"], index + 1, "{name} {mode}");
                    let n = join["matched_index"].as_u64().unwrap() as usize;
                    assert!(seen.insert(n), "{name} {mode} duplicate delivery");
                    assert!(
                        delivered["records"][n - 1]["depth"]
                            .as_u64()
                            .is_some_and(|depth| (1..=16).contains(&depth)),
                        "{name} {mode} exact database depth"
                    );
                    same(
                        &saved[target][index],
                        &delivered["records"][n - 1]["record"],
                        &format!("{name} {mode} exact record {index}"),
                    );
                }
            }
        }
    }
    for n in 1..=5 {
        let name = format!("original-{n:02}");
        let actual = &state(result, &name)["raw"];
        let expected = &result["evidence"]["originals"][n - 1]["source_census"];
        assert_eq!(rows(actual).len(), rows(expected).len(), "{name}");
        for (a, b) in rows(actual).iter().zip(rows(expected)) {
            same(&a["source"], &b["source"], &name);
            same(&a["attributes"], &b["attributes"], &name);
            assert_eq!(rows(&a["sets"]).len(), rows(&b["sets"]).len(), "{name}");
            for (a, b) in rows(&a["sets"]).iter().zip(rows(&b["sets"])) {
                for field in ["source", "element", "attributes"] {
                    same(&a[field], &b[field], &format!("{name} {field}"));
                }
                assert_eq!(
                    rows(&a["entries"]).len(),
                    rows(&b["entries"]).len(),
                    "{name}"
                );
                for (a, b) in rows(&a["entries"]).iter().zip(rows(&b["entries"])) {
                    same(&a["source"], &b["source"], &name);
                    same(&a["xml"]["elem"], &b["element"], &name);
                    same(&a["xml"]["attrib"], &b["attributes"], &name);
                    same(
                        &a["xml"],
                        &b["parsed_xml"],
                        &format!("{name} source {} exact parsed entry", b["source"]),
                    );
                }
            }
        }
    }
    for fact in rows(&facts["rows"]) {
        let index = fact["source_index"].as_u64().unwrap() as usize;
        let actual = &state(result, "original-05")["quest_catalogue"][index - 1];
        let expected = fact["source_record"].as_object().unwrap();
        assert_eq!(actual.as_object().unwrap().len(), expected.len());
        for (key, typed) in expected {
            metadata(&actual[key], typed)
        }
        let key = fact["config_key"].as_str().unwrap();
        let defs: Vec<_> = rows(catalogue)
            .iter()
            .filter(|r| r["definition"]["var"] == key)
            .collect();
        assert_eq!(defs.len(), 1, "quest {key}");
        assert!(
            defs[0]["definition"]["apply"]["function_source"]["path"]
                .as_str()
                .unwrap()
                .ends_with("Modules/ConfigOptions.lua")
        );
    }
    let base = &state(result, "original-05")["saved"];
    let source_data = &state(result, "original-05")["source_data"];
    assert_eq!(rows(&source_data["levels"]).len(), 85);
    for (index, row) in rows(&source_data["levels"]).iter().enumerate() {
        assert_eq!(row["level"], index + 1);
        assert!(row["armour"].as_f64().is_some_and(|n| n > 0.));
        assert!(row["evasion"].as_f64().is_some_and(|n| n > 0.));
    }
    assert!(
        source_data["pinnacle_armour_mean"]
            .as_f64()
            .is_some_and(|n| n > 0.)
    );
    assert!(
        source_data["pinnacle_evasion_mean"]
            .as_f64()
            .is_some_and(|n| n > 0.)
    );
    for (name, raw, expected) in [
        ("numeric-placeholders", None, 50),
        ("numeric-input-priority", Some(50), 50),
        ("numeric-zero", Some(0), 0),
        ("numeric-malformed-last", Some(90), 90),
    ] {
        let saved = &state(result, name)["saved"];
        same(
            &saved["input"]["enemyColdResist"],
            &json!(raw),
            &format!("{name} raw cold presence"),
        );
        assert_eq!(
            saved["placeholder"]["enemyColdResist"], 50,
            "{name} Pinnacle callback overwrite"
        );
        assert_eq!(
            value(saved, "enemy_mods", "ColdResist", "EnemyConfig"),
            expected,
            "{name}"
        );
        for mode in ["MAIN", "CALCS"] {
            assert_eq!(
                saved["modes"][mode]["enemy_base"]["ColdResist"]["enemy_config"], expected,
                "{name} {mode} source-owned BASE"
            );
        }
    }
    // Equal BASE is not a statement of general raw-lane equivalence: the full
    // outputs and original consumer breakdowns retain the separate Input state.
    assert!(base["input"]["enemyColdResist"].is_null());
    check_resistance_controls(result);
    for (name, expected) in [
        ("numeric-placeholders", [7., 13., 1.25, -13.]),
        ("numeric-input-priority", [3., 11., 2.5, -11.]),
        ("numeric-zero", [7., 0., 0., 0.]),
    ] {
        let saved = &state(result, name)["saved"];
        for ((target, key, source), number) in [
            ("player_mods", "Multiplier:LinkedTargets", "Config"),
            ("enemy_mods", "BlockChance", "Config"),
            ("player_mods", "IntuitiveLinkSourceRate", "Config"),
            ("enemy_mods", "PhysicalDamageReduction", "EnemyConfig"),
        ]
        .into_iter()
        .zip(expected)
        {
            assert_eq!(
                value(saved, target, key, source).as_f64(),
                Some(number),
                "{name} {key}"
            );
        }
    }
    let missing = &state(result, "numeric-missing")["saved"];
    for (target, key, source) in [
        ("player_mods", "Multiplier:LinkedTargets", "Config"),
        ("enemy_mods", "BlockChance", "Config"),
        ("player_mods", "IntuitiveLinkSourceRate", "Config"),
        ("enemy_mods", "PhysicalDamageReduction", "EnemyConfig"),
    ] {
        assert!(
            records(missing, target, key, source).is_empty(),
            "missing {key}"
        );
    }
    let duplicate = &state(result, "duplicate-unknown-namespace")["saved"];
    assert_eq!(duplicate["input"]["multiplierLinkedTargets"], 9);
    assert_eq!(duplicate["input"]["unreviewedConfiguration"], 73);
    assert_eq!(duplicate["placeholder"]["unreviewedPlaceholder"], 19);
    assert_eq!(
        value(
            duplicate,
            "player_mods",
            "Multiplier:LinkedTargets",
            "Config"
        ),
        9
    );
    let malformed = &state(result, "numeric-malformed-last")["saved"];
    assert!(malformed["input"]["multiplierLinkedTargets"].is_null());
    assert_eq!(
        value(
            malformed,
            "player_mods",
            "Multiplier:LinkedTargets",
            "Config"
        ),
        7
    );
    let alias = &state(result, "placeholder-string-and-alias")["saved"];
    assert_eq!(
        alias["input"]["enemyIsBoss"], "Shaper",
        "Placeholder strings bypass Input alias normalization"
    );
    assert_eq!(alias["input"]["presetBossSkills"], "Shaper Ball");
    assert_eq!(
        state(result, "boss-none")["saved"]["input"]["enemyIsBoss"],
        "None"
    );
    let boss = &state(result, "boss-preset-size")["saved"];
    assert_eq!(boss["input"]["enemyIsBoss"], "Uber");
    assert_eq!(boss["input"]["presetBossSkills"], "Shaper Ball");
    assert_eq!(boss["input"]["enemySizePreset"], "Large");
    assert_ne!(boss["enemy_mods"], base["enemy_mods"]);
    for (name, source) in [
        ("custom-enabled", "Custom:Witness"),
        ("custom-block-over-legacy", "Custom:Witness"),
        ("custom-legacy", "Custom:Default"),
    ] {
        let saved = &state(result, name)["saved"];
        assert_eq!(value(saved, "player_mods", "Life", source), 17, "{name}");
        assert!(saved["input"]["customMods"].is_null(), "{name} migration");
        assert!(
            !rows(&saved["player_mods"]).iter().any(|m| m["source"]
                .as_str()
                .is_some_and(|s| s.starts_with("Custom"))
                && m["value"] == 91),
            "{name}"
        );
    }
    let disabled = &state(result, "custom-disabled")["saved"];
    same(
        &disabled["player_mods"],
        &base["player_mods"],
        "disabled custom records",
    );
    same(
        &disabled["modes"],
        &base["modes"],
        "disabled custom outputs",
    );
    for name in ["archived-set", "unknown-selected-set"] {
        let saved = &state(result, name)["saved"];
        assert_eq!(rows(&saved["sets"]).len(), 2);
        for field in [
            "selected",
            "input",
            "placeholder",
            "player_mods",
            "enemy_mods",
            "modes",
        ] {
            same(&saved[field], &base[field], &format!("{name} {field}"));
        }
    }
    let selected = &state(result, "selected-set")["saved"];
    assert_eq!(selected["selected"]["config"], 2);
    assert_eq!(
        value(
            selected,
            "player_mods",
            "Multiplier:LinkedTargets",
            "Config"
        ),
        9
    );
    for (key, default) in [
        ("enemyIsBoss", json!("Pinnacle")),
        ("multiplierCurrentManaPercentage", json!(100)),
        ("inDemonForm", json!(true)),
        ("resistancePenalty", json!(-60)),
    ] {
        same(&base["input"][key], &default, &format!("default {key}"));
    }
    for key in [
        "enemyPhysicalDamage",
        "enemyLightningDamage",
        "enemyColdDamage",
        "enemyFireDamage",
        "enemyChaosDamage",
        "enemySpeed",
        "enemyCritChance",
        "DisableEHPGainOnBlock",
    ] {
        let defs: Vec<_> = rows(catalogue)
            .iter()
            .filter(|r| r["definition"]["var"] == key)
            .collect();
        assert!(!defs.is_empty(), "missing direct consumer {key}");
        assert!(
            defs.iter().all(|r| r["definition"]["apply"].is_null()),
            "{key} unexpected callback"
        );
    }
    let zero = &state(result, "direct-damage-zero")["saved"];
    let explicit = &state(result, "direct-damage-explicit")["saved"];
    assert_eq!(zero["input"]["enemyPhysicalDamage"], 0);
    assert_eq!(explicit["input"]["enemyPhysicalDamage"], 125);
    for saved in [zero, explicit] {
        same(
            &saved["player_mods"],
            &base["player_mods"],
            "direct damage Player callbacks",
        );
        same(
            &saved["enemy_mods"],
            &base["enemy_mods"],
            "direct damage Enemy callbacks",
        );
    }
    assert_eq!(
        zero["modes"]["MAIN"]["outputs"]["player"]["PhysicalEnemyDamage"],
        0
    );
    assert!(
        explicit["modes"]["MAIN"]["outputs"]["player"]["PhysicalEnemyDamage"]
            .as_f64()
            .is_some_and(|n| n > 0.),
        "direct damage consumer absent"
    );
}
