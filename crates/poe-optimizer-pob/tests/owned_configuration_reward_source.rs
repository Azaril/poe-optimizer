//! Complete original configuration loading plus the finite real quest callbacks.
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

const TEST: &str = "fresh_configuration_preserves_exhaustive_reward_controls_and_provenance";
const CHILD: &str = "POE_CONFIGURATION_REWARD_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/configuration_reward_source.lua");
const CHECK: &str = "questAct 1ClearfellBeira";
const LIST: &str = "questAct 2Valley of the TitansMedallion";
const UNKNOWN: &str = "+123 to maximum Life";

#[test]
fn fresh_configuration_preserves_exhaustive_reward_controls_and_provenance() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-configuration-reward-source-01");
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
    let facts = read(&root.join("data/owned/poe2/3887ae68/import/reward-source-facts.json"));
    let policy = read(&root.join("data/owned/poe2/3887ae68/import/reward-policy-seed.json"));
    assert_eq!(rows(&policy["rules"]).len(), 17);
    assert_eq!(
        rows(&policy["rules"])
            .iter()
            .flat_map(|r| rows(&r["outcomes"]))
            .filter(|r| r["outcome"]["kind"] == "reward")
            .count(),
        31
    );
    let mut originals = vec![];
    let mut joins = vec![];
    for (index, ordinal) in [390, 427, 195, 373, 423].into_iter().enumerate() {
        let file = format!("build-{:02}.xml", index + 1);
        let xml = fs::read_to_string(fixtures.join(&file)).unwrap();
        let entry = rows(&manifest["builds"])
            .iter()
            .find(|r| r["xml"] == file)
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([62; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let found: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "ConfigSet"
                    && r.attribute("id").and_then(|a| a.decoded().ok()) == Some("1")
            })
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].occurrence().id().ordinal(), ordinal);
        joins.push(
            json!({"original":index+1,"config_source":ordinal,"xml_sha256":digest(xml.as_bytes())}),
        );
        originals.push((format!("original-{:02}", index + 1), xml));
    }
    let mut inputs = originals.clone();
    inputs.extend(controls(&originals[4].1, &facts));
    let attempts = inputs.len();
    let mut cases = vec![];
    let mut source_hash = None;
    for (name, text) in inputs {
        let before = |lua: &Lua| {
            lua.globals().set("rewardXml", text.as_str())?;
            lua.globals().set("rewardMatrix", name == "original-05")?;
            lua.globals()
                .set("rewardSwitch", name == "switch-history")?;
            lua.globals().set("rewardJit", enabled)?;
            lua.load("if rewardJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let diagnostics = Rc::new(RefCell::new(None::<Json>));
        let before_build = |lua: &Lua| {
            lua.globals().set("rewardPhase", "before")?;
            let cleanup: Function = lua
                .load(OBSERVE)
                .set_name("@configuration-reward-source-observer")
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
            !name.starts_with("original-"),
            Some(&before),
            Some(&before_build),
            Some(&observe),
        );
        assert!(
            diagnostics.borrow().is_some(),
            "method identity cleanup missing for {name}: {}",
            result
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default()
        );
        let load = diagnostics.borrow_mut().take().unwrap();
        let row = match result {
            Ok(result) => {
                assert_eq!(result["configuration_method_wrappers"], false);
                assert_eq!(result["original_build_output_available"], true);
                if let Some(hash) = &source_hash {
                    assert_eq!(&result["source_hash"], hash)
                } else {
                    source_hash = Some(result["source_hash"].clone())
                }
                json!({"name":name,"xml_sha256":digest(text.as_bytes()),"available":true,"load":load,"state":result["additional_observation"]})
            }
            Err(error) => {
                assert!(matches!(error, RuntimeError::Lua(_)), "{name}: {error}");
                json!({"name":name,"xml_sha256":digest(text.as_bytes()),"available":false,"load":load,"source_error":error.to_string()})
            }
        };
        cases.push(row);
    }
    for (index, (_, xml)) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read_to_string(fixtures.join(format!("build-{:02}.xml", index + 1))).unwrap(),
            xml
        )
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":source_hash,"evidence":{"manifest_sha256":pinned::manifest_sha256(),"originals":joins,"complete_load_attempts":attempts,"full_controls":attempts-5,"finite_callback_cases":39,"source_quest_records":29,"configuration_controls":17,"excluded_weapon_point_records":12,"owned_non_none_metadata_outcomes":31,"native_effect_coverage":false,"business_method_wrappers":false,"files":(["src/Launch.lua","src/Modules/Common.lua","src/Classes/ConfigTab.lua","src/Classes/DropDownControl.lua","src/Classes/CalcsTab.lua","src/Classes/ModList.lua","src/Classes/ModDB.lua","src/Modules/ConfigOptions.lua","src/Modules/ModParser.lua","src/Modules/ModTools.lua","src/Modules/Build.lua","src/Modules/CalcSetup.lua","src/Data/QuestRewards.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))} ,"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result, &facts, &policy);
}

fn controls(xml: &str, facts: &Json) -> Vec<(String, String)> {
    let absent = without_rewards(xml);
    let all_none = rows(&facts["rows"])
        .iter()
        .map(|r| {
            input(
                r["config_key"].as_str().unwrap(),
                if r["choice"]["kind"] == "boolean" {
                    "boolean"
                } else {
                    "string"
                },
                if r["choice"]["kind"] == "boolean" {
                    "false"
                } else {
                    "None"
                },
            )
        })
        .collect::<String>();
    let mut cases = vec![
        ("all-absent".into(), absent.clone()),
        ("all-none".into(), add(&absent, &all_none)),
    ];
    for (name, key, lane, value) in [
        ("check-false", CHECK, "boolean", "false"),
        ("check-true", CHECK, "boolean", "true"),
        ("check-string-false", CHECK, "string", "false"),
        ("check-malformed-boolean", CHECK, "boolean", "maybe"),
        ("list-unknown-parsable", LIST, "string", UNKNOWN),
        (
            "list-unknown-unparsed",
            LIST,
            "string",
            "unreviewed reward text",
        ),
    ] {
        cases.push((name.into(), add(xml, &input(key, lane, value))))
    }
    cases.push((
        "placeholder-string".into(),
        add(
            xml,
            &input(LIST, "string", UNKNOWN).replace("<Input ", "<Placeholder "),
        ),
    ));
    cases.push((
        "placeholder-number".into(),
        add(
            xml,
            &input(LIST, "number", "0").replace("<Input ", "<Placeholder "),
        ),
    ));
    cases.push((
        "duplicate-last-none".into(),
        add(
            xml,
            &format!(
                "{}{}",
                input(LIST, "string", UNKNOWN),
                input(LIST, "string", "None")
            ),
        ),
    ));
    cases.push((
        "duplicate-last-value".into(),
        add(
            xml,
            &format!(
                "{}{}",
                input(LIST, "string", "None"),
                input(LIST, "string", UNKNOWN)
            ),
        ),
    ));
    cases.push((
        "mixed-typed-list".into(),
        add(
            xml,
            &format!(
                "<Input name=\"{}\" number=\"0\" string=\"{}\"/>",
                escape(LIST),
                escape(UNKNOWN)
            ),
        ),
    ));
    cases.push((
        "missing-typed-list".into(),
        add(xml, &format!("<Input name=\"{}\"/>", escape(LIST))),
    ));
    cases.push((
        "namespaced-input".into(),
        add(
            xml,
            &format!(
                "<q:Input xmlns:q=\"urn:owned-test\" name=\"{}\" string=\"None\"/>",
                escape(LIST)
            ),
        ),
    ));
    cases.push((
        "nested-input".into(),
        add(
            xml,
            &format!("<Unreviewed>{}</Unreviewed>", input(LIST, "string", "None")),
        ),
    ));
    cases.push((
        "unknown-key".into(),
        add(xml, &input("questUnreviewed", "string", UNKNOWN)),
    ));
    cases.push((
        "default-namespace".into(),
        replace(
            xml,
            "<Config activeConfigSet=\"1\">",
            "<Config xmlns=\"urn:owned-test\" activeConfigSet=\"1\">",
        ),
    ));
    for (name, enabled) in [("custom-enabled", "true"), ("custom-disabled", "false")] {
        cases.push((name.into(),add(xml,&format!("<CustomModifierBlock title=\"Reward contrast\" enabled=\"{enabled}\">{UNKNOWN}</CustomModifierBlock>"))))
    }
    cases.push((
        "legacy-custom".into(),
        add(xml, &input("customMods", "string", UNKNOWN)),
    ));
    let second = format!(
        "<ConfigSet id=\"2\" title=\"Reward archive\">{}{}</ConfigSet></Config>",
        input(CHECK, "boolean", "false"),
        input(LIST, "string", UNKNOWN)
    );
    let archive = replace(xml, "</Config>", &second);
    cases.push(("archived-set".into(), archive.clone()));
    cases.push((
        "selected-set".into(),
        replace(
            &archive,
            "<Config activeConfigSet=\"1\">",
            "<Config activeConfigSet=\"2\">",
        ),
    ));
    cases.push(("switch-history".into(), archive));
    cases.push((
        "duplicate-set".into(),
        replace(
            xml,
            "</Config>",
            &format!(
                "<ConfigSet id=\"1\" title=\"Replacement\">{}</ConfigSet></Config>",
                input(CHECK, "boolean", "false")
            ),
        ),
    ));
    assert_eq!(cases.len(), 25);
    cases
}
fn without_rewards(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut ranges: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.has_tag_name("Input") && n.attribute("name").is_some_and(|s| s.starts_with("quest"))
        })
        .map(|n| n.range())
        .collect();
    ranges.sort_by_key(|r| r.start);
    let mut out = xml.to_owned();
    for r in ranges.into_iter().rev() {
        out.replace_range(r, "")
    }
    out
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn input(key: &str, lane: &str, value: &str) -> String {
    format!(
        "<Input name=\"{}\" {lane}=\"{}\"/>",
        escape(key),
        escape(value)
    )
}
fn add(xml: &str, row: &str) -> String {
    replace(xml, "</ConfigSet>", &format!("{row}</ConfigSet>"))
}
fn replace(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "{old}");
    text.replace(old, new)
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("rewardPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@configuration-reward-source-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Json) -> &[Json] {
    if let Some(rows) = v.as_array() {
        rows
    } else {
        assert!(
            v.as_object().is_some_and(|v| v.is_empty()),
            "expected source list: {v}"
        );
        &[]
    }
}
fn named<'a>(list: &'a Json, key: &str, value: &str) -> &'a Json {
    let found: Vec<_> = rows(list).iter().filter(|r| r[key] == value).collect();
    assert_eq!(found.len(), 1, "{key}={value}");
    found[0]
}
fn state<'a>(result: &'a Json, name: &str) -> &'a Json {
    let c = named(&result["cases"], "name", name);
    assert_eq!(c["available"], true, "{name}: {}", c["source_error"]);
    &c["state"]
}
fn control<'a>(snapshot: &'a Json, key: &str) -> &'a Json {
    named(&snapshot["controls"], "key", key)
}
fn sorted(records: &Json) -> Vec<String> {
    let mut out: Vec<_> = rows(records)
        .iter()
        .map(|v| serde_json::to_string(v).unwrap())
        .collect();
    out.sort();
    out
}
fn source_records(snapshot: &Json, source: &str) -> Vec<Json> {
    rows(&snapshot["quest"])
        .iter()
        .filter(|r| r["source"] == source)
        .cloned()
        .collect()
}

fn exact_metadata(actual: &Json, typed: &Json) {
    match typed["kind"].as_str().unwrap() {
        "number" => {
            assert!(actual.is_number() && typed["value"].is_number());
            assert_eq!(actual.as_f64(), typed["value"].as_f64());
        }
        "text" => {
            assert!(actual.is_string() && typed["value"].is_string());
            assert_eq!(actual, &typed["value"]);
        }
        "boolean" => {
            assert!(actual.is_boolean() && typed["value"].is_boolean());
            assert_eq!(actual, &typed["value"]);
        }
        "array" => {
            let actual = actual.as_array().unwrap();
            let expected = typed["value"].as_array().unwrap();
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                exact_metadata(actual, expected);
            }
        }
        kind => panic!("unreviewed source metadata type {kind}"),
    }
}

fn check(result: &Json, facts: &Json, policy: &Json) {
    let baseline = state(result, "original-05");
    let matrix = rows(&baseline["matrix"]);
    assert_eq!(matrix.len(), 39);
    let census = rows(&baseline["dynamic_census"]);
    assert_eq!(census.len(), 17);
    assert_eq!(rows(&baseline["excluded_source_rows"]).len(), 12);
    let mut reward_values = 0;
    let mut none_values = 0;
    for fact in rows(&facts["rows"]) {
        let key = fact["config_key"].as_str().unwrap();
        let actual = named(&baseline["dynamic_census"], "key", key);
        assert_eq!(actual["source_index"], fact["source_index"]);
        let raw = fact["source_record"].as_object().unwrap();
        assert_eq!(actual["quest"].as_object().unwrap().len(), raw.len());
        for (name, value) in raw {
            exact_metadata(&actual["quest"][name], value);
        }
        if fact["choice"]["kind"] == "boolean" {
            assert_eq!(actual["widget"], "check");
            assert_eq!(actual["default"], true)
        } else {
            assert_eq!(actual["widget"], "list");
            assert_eq!(actual["default_index"], 1);
            assert_eq!(actual["options"], fact["choice"]["options"])
        }
        for sample in matrix.iter().filter(|r| r["key"] == key) {
            assert!(rows(&sample["enemy_records"]).is_empty());
            if sample["value"] == "None" {
                none_values += 1;
                assert!(rows(&sample["records"]).is_empty())
            } else {
                reward_values += 1;
                assert!(
                    !rows(&sample["records"]).is_empty(),
                    "no parsed source reward {key}: {}",
                    sample["value"]
                )
            }
            for record in rows(&sample["records"]) {
                assert_eq!(record["source"], actual["source"])
            }
        }
    }
    assert_eq!((reward_values, none_values), (31, 8));
    // Join every existing finite owned outcome to the actual generated callback
    // value, rather than treating the total number of definitions as authority.
    for rule in rows(&policy["rules"]) {
        let selectors = rows(&rule["recipe"]["tiers"][0]["selectors"]);
        assert_eq!(selectors.len(), 1);
        let key = selectors[0]["name"].as_str().unwrap();
        let fact = named(&facts["rows"], "config_key", key);
        assert_eq!(rule["recipe"]["missing"]["kind"], "explicit");
        let tokens = rows(&rule["recipe"]["codec"]["codec"]["value"]["tokens"]);
        if fact["choice"]["kind"] == "boolean" {
            assert_eq!(selectors[0]["lane"], "input_boolean");
            assert_eq!(rule["recipe"]["missing"]["value"]["value"], true);
            assert_eq!(
                tokens
                    .iter()
                    .map(|t| t["token"].clone())
                    .collect::<Vec<_>>(),
                vec![json!("false"), json!("true")]
            );
        } else {
            assert_eq!(selectors[0]["lane"], "input_string");
            assert_eq!(
                json!(
                    tokens
                        .iter()
                        .map(|t| t["token"].clone())
                        .collect::<Vec<_>>()
                ),
                fact["choice"]["options"]
            );
            assert_eq!(
                rule["recipe"]["missing"]["value"]["value"],
                tokens[0]["value"]
            );
        }
        for outcome in rows(&rule["outcomes"]) {
            let token = tokens
                .iter()
                .find(|t| t["value"] == outcome["when"]["value"])
                .unwrap();
            if outcome["outcome"]["kind"] == "none" {
                assert!(token["token"] == "false" || token["token"] == "None");
            } else {
                let selector = &outcome["outcome"]["selector"]["value"];
                assert_eq!(selector["key"]["value"], key);
                assert_eq!(selector["value"]["value"], token["token"]);
                let value = if token["token"] == "true" {
                    json!(true)
                } else {
                    token["token"].clone()
                };
                assert_eq!(
                    matrix
                        .iter()
                        .filter(|m| m["key"] == key && m["value"] == value)
                        .count(),
                    1
                );
            }
        }
    }
    for (index, expected) in [16, 17, 15, 16, 17].into_iter().enumerate() {
        let name = format!("original-{:02}", index + 1);
        let s = state(result, &name);
        let saved = &s["saved"];
        assert_eq!(s["dynamic_census"], baseline["dynamic_census"]);
        assert_eq!(s["original_methods_preserved"], true);
        assert_eq!(s["business_method_wrappers"], false);
        assert_eq!(s["selected_state_and_outputs_preserved"], true);
        assert_eq!(saved["selected"]["config"], 1);
        assert!(rows(&saved["enemy_quest"]).is_empty());
        let mut selected = vec![];
        let mut count = 0;
        for row in rows(&saved["controls"]) {
            let value = &row["input"];
            if *value == false || *value == "None" {
                continue;
            }
            let matching: Vec<_> = matrix
                .iter()
                .filter(|m| m["key"] == row["key"] && m["value"] == *value)
                .collect();
            assert_eq!(matching.len(), 1, "{name}: {}", row["key"]);
            selected.extend_from_slice(rows(&matching[0]["records"]));
            count += 1;
        }
        assert_eq!(count, expected);
        assert_eq!(sorted(&json!(selected)), sorted(&saved["quest"]));
        for mode in ["MAIN", "CALCS"] {
            assert_eq!(saved["modes"][mode]["available"], true);
            assert!(
                !saved["modes"][mode]["player"]
                    .as_object()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                sorted(&saved["modes"][mode]["quest"]),
                sorted(&saved["quest"])
            );
        }
    }
    let saved = &baseline["saved"];
    for name in [
        "check-true",
        "check-string-false",
        "placeholder-number",
        "namespaced-input",
        "nested-input",
        "unknown-key",
        "default-namespace",
        "archived-set",
        "switch-history",
    ] {
        assert_eq!(
            state(result, name)["saved"]["quest"],
            saved["quest"],
            "{name}"
        );
    }
    let absent = &state(result, "all-absent")["saved"];
    for row in census {
        assert_eq!(
            control(absent, row["key"].as_str().unwrap())["input"],
            if row["widget"] == "check" {
                json!(true)
            } else {
                json!("None")
            }
        )
    }
    assert!(rows(&state(result, "all-none")["saved"]["quest"]).is_empty());
    let check_source = named(&baseline["dynamic_census"], "key", CHECK)["source"]
        .as_str()
        .unwrap();
    for name in ["check-false", "check-malformed-boolean"] {
        assert_eq!(
            control(&state(result, name)["saved"], CHECK)["input"],
            false
        );
        assert!(source_records(&state(result, name)["saved"], check_source).is_empty());
    }
    let list_source = named(&baseline["dynamic_census"], "key", LIST)["source"]
        .as_str()
        .unwrap();
    for name in [
        "list-unknown-parsable",
        "placeholder-string",
        "duplicate-last-value",
    ] {
        let s = &state(result, name)["saved"];
        assert_eq!(control(s, LIST)["input"], UNKNOWN);
        let records = source_records(s, list_source);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["name"], "Life");
        assert_eq!(records[0]["value"], 123);
        assert_eq!(records[0]["type"], "BASE");
        assert_ne!(control(s, LIST)["ui_value"], UNKNOWN);
    }
    for name in [
        "list-unknown-unparsed",
        "duplicate-last-none",
        "mixed-typed-list",
    ] {
        assert!(
            source_records(&state(result, name)["saved"], list_source).is_empty(),
            "{name}"
        )
    }
    assert_eq!(
        control(&state(result, "mixed-typed-list")["saved"], LIST)["input"],
        0
    );
    let malformed = named(&result["cases"], "name", "missing-typed-list");
    let prompt = malformed["load"]["prompt"].as_str().unwrap();
    assert!(
        prompt.contains("Input") && prompt.contains("missing number, string or boolean"),
        "{prompt}"
    );
    for case in rows(&result["cases"]) {
        if case["name"] != "missing-typed-list" {
            assert!(
                case["load"]["prompt"].is_null(),
                "{}: {}",
                case["name"],
                case["load"]["prompt"]
            );
            assert_eq!(case["available"], true, "{}", case["name"]);
        }
    }
    for name in ["custom-enabled", "legacy-custom"] {
        let s = &state(result, name)["saved"];
        assert_eq!(s["quest"], saved["quest"]);
        let records: Vec<_> = rows(&s["custom"])
            .iter()
            .filter(|r| r["name"] == "Life" && r["value"] == 123)
            .collect();
        assert_eq!(records.len(), 1);
        assert!(
            records[0]["source"]
                .as_str()
                .unwrap()
                .starts_with("Custom:")
        );
        for mode in ["MAIN", "CALCS"] {
            assert!(
                rows(&s["modes"][mode]["custom"])
                    .iter()
                    .any(|r| r["name"] == "Life" && r["value"] == 123)
            );
        }
    }
    assert_eq!(
        state(result, "custom-disabled")["saved"]["custom"],
        saved["custom"]
    );
    let history = rows(&state(result, "switch-history")["history"]);
    assert_eq!(history.len(), 4);
    for index in [0, 2] {
        let fresh = &state(result, "selected-set")["saved"];
        // SelByValue leaves the old UI choice on an unknown string. Only the
        // actual input/records/output are independent of that presentation history.
        for key in [
            "input",
            "placeholder",
            "quest",
            "custom",
            "enemy_quest",
            "custom_blocks",
            "modes",
            "selected",
        ] {
            assert_eq!(history[index][key], fresh[key], "switched/fresh {key}");
        }
        assert_eq!(control(&history[index], LIST)["input"], UNKNOWN);
        assert_eq!(
            control(&history[index], LIST)["ui_value"],
            control(saved, LIST)["ui_value"]
        );
        assert_eq!(control(fresh, LIST)["ui_value"], "None");
    }
    for index in [1, 3] {
        assert_eq!(history[index], state(result, "archived-set")["saved"])
    }
    assert_eq!(
        state(result, "selected-set")["saved"]["selected"]["config"],
        2
    );
    for key in ["skills", "items", "spec"] {
        assert_eq!(
            state(result, "selected-set")["saved"]["selected"][key],
            saved["selected"][key]
        );
    }
    let duplicate = &state(result, "duplicate-set")["saved"];
    assert_eq!(duplicate["selected"]["config"], 1);
    assert_eq!(control(duplicate, CHECK)["input"], false);
    assert_eq!(control(duplicate, LIST)["input"], "None");
}
