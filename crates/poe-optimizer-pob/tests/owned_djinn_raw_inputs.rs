//! Raw saved Djinn values and complete-source preparation remain separate.
//! Optional source evidence only; this grants no native numeric parity.
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
    cell::{Cell, RefCell},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

const TEST: &str = "complete_djinn_raw_inputs_preserve_source_boundaries";
const CHILD: &str = "POE_DJINN_RAW_INPUTS_CHILD";
const OBSERVE: &str = include_str!("support/djinn_provider_source.lua");
const RAW: &str = include_str!("support/djinn_raw_inputs_source.lua");
const SAND: &str = "SummonSandDjinnPlayer";
const WATER: &str = "SummonWaterDjinnPlayer";
const XML_SHA: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";

#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for raw Djinn inputs"]
fn complete_djinn_raw_inputs_preserve_source_boundaries() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-djinn-raw-inputs-01");
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
            .args(["--exact", TEST, "--ignored", "--nocapture"])
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
                    "source child failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off = fs::read(out.join("source-jit-off.json")).unwrap();
    let on = fs::read(out.join("source-jit-on.json")).unwrap();
    assert_eq!(
        off, on,
        "complete deterministic source evidence differs between JIT modes"
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let paths: Vec<_> = (1..=5)
        .map(|i| {
            root.join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{i:02}.xml"
            ))
        })
        .collect();
    let originals: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
    assert_eq!(digest(&originals[4]), XML_SHA);
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([73; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let joins: Vec<_> = [(216, SAND), (229, WATER)]
        .into_iter()
        .map(|(ordinal, effect)| {
            let node = evidence
                .rows()
                .iter()
                .find(|n| n.occurrence().id().ordinal() == ordinal)
                .unwrap();
            assert_eq!(node.occurrence().name(), "Gem");
            assert_eq!(
                node.attribute("skillId").and_then(|a| a.decoded().ok()),
                Some(effect)
            );
            json!({"source":ordinal,"effect":effect})
        })
        .collect();
    let mut cases = vec![];
    for (name, text) in controls(xml) {
        eprintln!(
            "source case {name}, JIT {}",
            if enabled { "on" } else { "off" }
        );
        let state = run(root, &text, enabled);
        cases.push(json!({"name":name,"xml_sha256":digest(text.as_bytes()),"state":state}));
    }
    let result = json!({
        "manifest_sha256":pinned::manifest_sha256(),"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "original_xml_sha256":XML_SHA,"source_instance_joins":joins,
        "business_wrappers":false,"native_parity":false,"quality_formula_inferred":false,
        "raw_observation":"non-mutating call hook reads original LoadSkill argument before original ProcessSocketGroup; removed after saved Djinn inventory or failure",
        "failure_trace_location":"source-jit-off.log and source-jit-on.log contain full unmodified errors; report records stable source error site",
        "files":(["src/Classes/SkillsTab.lua","src/Modules/CalcTools.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Data/Gems.lua","src/Data/Skills/other.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "cases":cases,
    });
    let bytes = serde_json::to_vec_pretty(&result).unwrap();
    assert!(bytes.len() <= 32 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    for (path, bytes) in paths.iter().zip(&originals) {
        assert_eq!(&fs::read(path).unwrap(), bytes);
    }
    check(&result);
}

fn run(root: &Path, xml: &str, enabled: bool) -> Json {
    let reached_observer = Cell::new(false);
    let raw = Rc::new(RefCell::new(Json::Null));
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("djinnXml", xml)?;
        lua.globals().set("djinnJit", enabled)?;
        lua.load("if djinnJit then jit.on() else jit.off(); jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let finish: Function = lua
            .load(OBSERVE)
            .set_name("@djinn-source-authentication")
            .eval()?;
        let capture: Function = lua.load(RAW).set_name("@djinn-raw-source-capture").eval()?;
        let captured = Rc::clone(&raw);
        Ok(lua.create_function(move |lua, ()| {
            capture.call::<()>(())?;
            *captured.borrow_mut() =
                lua.from_value(lua.globals().get::<Value>("djinnRawResult")?)?;
            finish.call::<()>(())
        })?)
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        reached_observer.set(true);
        lua.globals().set("djinnPhase", "observe")?;
        let value: Value = lua
            .load(OBSERVE)
            .set_name("@djinn-source-observer")
            .eval()?;
        Ok(lua.from_value::<Json>(value)?)
    };
    let temp = tempfile::tempdir().unwrap();
    match source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        temp.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&install),
        Some(&observe),
    ) {
        Ok(value) => {
            assert_eq!(value["configuration_method_wrappers"], false);
            assert_eq!(value["original_build_output_available"], true);
            json!({"kind":"complete","raw":raw.borrow().clone(),"source_hash":value["source_hash"],"prepared":value["additional_observation"]})
        }
        Err(error) => {
            let error = error.to_string();
            eprintln!("complete source failure:\n{error}");
            let site = error
                .lines()
                .find(|line| line.contains(".lua:") && !line.trim_start().starts_with("["))
                .or_else(|| error.lines().find(|line| line.contains(".lua:")))
                .unwrap_or(&error)
                .trim();
            json!({"kind":if reached_observer.get(){"failed_observation_after_lifecycle"}else{"failed_complete_lifecycle"},"raw":raw.borrow().clone(),"source_error_site":site})
        }
    }
}

fn controls(xml: &str) -> Vec<(String, String)> {
    let mut cases = vec![("original".into(), xml.to_owned())];
    for (name, effect, field, value) in [
        ("sand-level-one", SAND, "level", Some("1")),
        ("water-level-seven", WATER, "level", Some("7")),
        ("sand-quality-fractional", SAND, "quality", Some("12.5")),
        ("water-quality-fractional", WATER, "quality", Some("17.25")),
        ("sand-level-zero", SAND, "level", Some("0")),
        ("sand-level-above-table", SAND, "level", Some("41")),
        ("sand-level-fractional", SAND, "level", Some("1.5")),
        ("water-level-negative", WATER, "level", Some("-1")),
        ("water-level-table-max", WATER, "level", Some("40")),
        ("water-level-fractional", WATER, "level", Some("20.5")),
        ("sand-level-missing", SAND, "level", None),
        ("water-level-invalid", WATER, "level", Some("bad")),
        ("sand-level-nil", SAND, "level", Some("nil")),
        ("sand-quality-missing", SAND, "quality", None),
        ("water-quality-invalid", WATER, "quality", Some("bad")),
        ("sand-quality-nil", SAND, "quality", Some("nil")),
        ("water-quality-negative", WATER, "quality", Some("-1")),
        ("sand-quality-above-hundred", SAND, "quality", Some("101.5")),
    ] {
        cases.push((name.into(), mutate(xml, Some(4), effect, &[(field, value)])));
    }
    let archived = mutate(
        xml,
        None,
        SAND,
        &[("level", Some("7")), ("quality", Some("2.5"))],
    );
    let archived = mutate(
        &archived,
        None,
        WATER,
        &[("level", Some("9")), ("quality", Some("3.5"))],
    );
    cases.push(("archived-independent-values".into(), archived));
    cases.push(("repeat-original".into(), xml.to_owned()));
    assert_eq!(cases.len(), 21);
    cases
}

fn mutate(
    xml: &str,
    selected: Option<u32>,
    effect: &str,
    updates: &[(&str, Option<&str>)],
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gems: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.has_tag_name("Gem")
                && n.attribute("skillId") == Some(effect)
                && n.parent().is_some_and(|p| p.attribute("source").is_none())
                && n.ancestors().any(|a| {
                    a.has_tag_name("SkillSet")
                        && if let Some(sid) = selected {
                            a.attribute("id") == Some(sid.to_string().as_str())
                        } else {
                            a.attribute("id") != Some("4")
                        }
                })
        })
        .collect();
    if selected.is_some() {
        assert_eq!(gems.len(), 1)
    } else {
        assert!(gems.len() >= 2)
    }
    let mut output = xml.to_owned();
    for gem in gems.into_iter().rev() {
        assert!(gem.children().all(|n| !n.is_element()));
        let mut replacement = String::from("<Gem");
        for attr in gem.attributes() {
            if !updates.iter().any(|(key, _)| *key == attr.name()) {
                replacement.push_str(&format!(
                    " {}=\"{}\"",
                    attr.name(),
                    attr.value()
                        .replace('&', "&amp;")
                        .replace('"', "&quot;")
                        .replace('<', "&lt;")
                ));
            }
        }
        for (key, value) in updates {
            if let Some(value) = value {
                replacement.push_str(&format!(" {key}=\"{value}\""));
            }
        }
        replacement.push_str("/>");
        output.replace_range(gem.range(), &replacement);
    }
    assert_ne!(output, xml);
    output
}

fn rows(v: &Json) -> &[Json] {
    if let Some(v) = v.as_array() {
        v
    } else {
        assert!(
            v.as_object().is_some_and(|v| v.is_empty()),
            "expected rows: {v}"
        );
        &[]
    }
}
fn state<'a>(r: &'a Json, name: &str) -> &'a Json {
    &rows(&r["cases"])
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()["state"]
}
fn source_row<'a>(s: &'a Json, effect: &str) -> &'a Json {
    let key = format!("4/{effect}/manual");
    rows(&s["raw"]["records"])
        .iter()
        .find(|r| r["key"] == key)
        .unwrap()
}
fn input_projection(s: &Json) -> Json {
    json!(rows(&s["prepared"]["runtime_groups"]).iter().filter(|g|g["selected"]==true).map(|g|{
        let actions:Vec<_>=["MAIN","CALCS"].into_iter().map(|mode|json!({"mode":mode,"actions":rows(&g["actions"][mode]).iter().map(|a|json!({"effect":a["effect"],"source_instance":a["source_instance"],"level":a["level"],"quality":a["quality"],"actor_level":a["minion"]["level"]})).collect::<Vec<_>>()})).collect();
        json!({"key":g["key"],"source":g["source"],"actions":actions})
    }).collect::<Vec<_>>())
}
fn check(result: &Json) {
    let base = state(result, "original");
    assert_eq!(base["kind"], "complete");
    assert_eq!(base, state(result, "repeat-original"));
    for case in rows(&result["cases"]) {
        let s = &case["state"];
        assert_ne!(
            s["kind"], "failed_observation_after_lifecycle",
            "{}: {s}",
            case["name"]
        );
        assert_eq!(s["raw"]["hook_removed"], true, "{}", case["name"]);
        assert_eq!(s["raw"]["business_wrappers"], false);
        for def in rows(&s["raw"]["definitions"]) {
            assert_eq!(def["levels"], json!((1..=40).collect::<Vec<_>>()));
            assert_eq!(def["natural_max_level"], 20);
        }
        for row in rows(&s["raw"]["records"]) {
            assert_eq!(row["caller_exact"], true);
            assert_eq!(row["process_exact"], true);
        }
        if s["kind"] != "complete" {
            continue;
        }
        assert_eq!(s["raw"]["captured_count"], s["raw"]["expected_count"]);
        let p = &s["prepared"];
        for flag in [
            "original_functions_preserved",
            "saved_instances_preserved",
            "selected_state_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(p[flag], true, "{} {flag}", case["name"]);
        }
        for (key, value) in [
            ("skills", 4),
            ("spec", 3),
            ("items", 2),
            ("config", 1),
            ("main_group", 3),
            ("calcs_group", 1),
        ] {
            assert_eq!(p["selection"][key], value);
        }
        for g in rows(&p["runtime_groups"]) {
            for mode in ["MAIN", "CALCS"] {
                let actions = rows(&g["actions"][mode]);
                if g["selected"] == false {
                    assert!(actions.is_empty());
                    continue;
                }
                assert_eq!(actions.len(), 2, "{} {}", case["name"], g["key"]);
                assert_eq!(actions[0]["source_instance"], actions[1]["source_instance"]);
                assert_eq!(actions[0]["level"], actions[1]["level"]);
                assert_eq!(actions[0]["quality"], actions[1]["quality"]);
                for a in actions {
                    assert_eq!(a["group_exact"], true);
                    assert_eq!(a["source_instance"]["key"], g["key"]);
                    if a["minion_available"] == true {
                        for child in rows(&a["minion"]["children"]) {
                            assert_eq!(child["actor_is_parent_minion"], true);
                            assert_eq!(child["summon_skill_exact"], true);
                            assert_eq!(child["support_list_same_parent"], true);
                        }
                    }
                }
                if g["source"].as_str().is_some_and(|v| v.starts_with("Tree:")) {
                    for a in actions {
                        assert_eq!(a["level"], 3);
                        assert_eq!(a["quality"], 0);
                    }
                }
            }
        }
        for effect in [SAND, WATER] {
            assert_eq!(source_row(s, effect)["same_saved_instance"], true);
        }
    }
    for (name, effect, loaded, processed) in [
        ("sand-level-one", SAND, 1.0, 1),
        ("water-level-seven", WATER, 7.0, 7),
        ("sand-level-zero", SAND, 0.0, 1),
        ("sand-level-above-table", SAND, 41.0, 40),
        ("sand-level-fractional", SAND, 1.5, 20),
        ("water-level-negative", WATER, -1.0, 1),
        ("water-level-table-max", WATER, 40.0, 40),
        ("water-level-fractional", WATER, 20.5, 20),
    ] {
        let s = state(result, name);
        assert_eq!(s["kind"], "complete", "{name}: {s}");
        let row = source_row(s, effect);
        assert_eq!(row["loaded"]["level"].as_f64(), Some(loaded));
        assert_eq!(row["source_processed"]["level"], processed);
        assert_eq!(row["loaded"]["quality"], 0);
        assert_eq!(row["source_processed"]["quality"], 0);
        let other = if effect == SAND { WATER } else { SAND };
        assert_eq!(source_row(s, other), source_row(base, other));
    }
    for (name, effect, value) in [
        ("sand-quality-fractional", SAND, 12.5),
        ("water-quality-fractional", WATER, 17.25),
        ("water-quality-negative", WATER, -1.0),
        ("sand-quality-above-hundred", SAND, 101.5),
    ] {
        let s = state(result, name);
        assert_eq!(s["kind"], "complete", "{name}: {s}");
        let row = source_row(s, effect);
        assert_eq!(row["loaded"]["quality"].as_f64(), Some(value));
        assert_eq!(row["source_processed"]["quality"].as_f64(), Some(value));
        assert_eq!(row["loaded"]["level"], 20);
        assert_eq!(row["source_processed"]["level"], 20);
    }
    for (name, effect, field) in [
        ("sand-level-missing", SAND, "level"),
        ("water-level-invalid", WATER, "level"),
        ("sand-level-nil", SAND, "level"),
        ("sand-quality-missing", SAND, "quality"),
        ("water-quality-invalid", WATER, "quality"),
        ("sand-quality-nil", SAND, "quality"),
    ] {
        let s = state(result, name);
        assert_eq!(s["kind"], "failed_complete_lifecycle", "{name}: {s}");
        let row = source_row(s, effect);
        assert!(row["loaded"][field].is_null());
        assert_eq!(row["loaded"][format!("{field}_type")], "nil");
    }
    let archived = state(result, "archived-independent-values");
    assert_eq!(archived["kind"], "complete");
    assert_eq!(input_projection(base), input_projection(archived));
    assert_eq!(base["prepared"]["outputs"], archived["prepared"]["outputs"]);
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_else(|e| e.to_string());
    let mut lines: Vec<_> = text.lines().rev().take(40).collect();
    lines.reverse();
    lines.join("\n")
}
