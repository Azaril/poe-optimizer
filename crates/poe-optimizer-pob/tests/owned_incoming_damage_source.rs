//! Actual Config callback defaults and incoming-damage queries in complete original builds.
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
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_original_incoming_damage_inputs_defaults_and_queries";
const CHILD: &str = "POE_INCOMING_DAMAGE_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_incoming_damage_source.lua");
const TYPES: [&str; 5] = ["Physical", "Lightning", "Cold", "Fire", "Chaos"];
const FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Modules/Data.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/EditControl.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModList.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/ModTools.lua",
    "src/Data/Global.lua",
    "src/Data/Misc.lua",
    "runtime/lua/xml.lua",
];
struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    original: bool,
    expected_failure: bool,
}

#[test]
fn complete_original_incoming_damage_inputs_defaults_and_queries() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-incoming-damage-source-01");
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
                assert!(status.success(), "{}: {}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(1800) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}: {}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "incoming damage JIT source evidence",
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|ordinal| {
            let name = format!("build-{ordinal:02}.xml");
            let xml = fs::read_to_string(dir.join(&name)).unwrap();
            let entry = rows(&index["builds"])
                .iter()
                .find(|row| row["xml"] == name)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            (name, xml)
        })
        .collect();
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, (_, xml))| Case {
            name: format!("original-{:02}", i + 1),
            xml: xml.clone(),
            warm: None,
            original: true,
            expected_failure: false,
        })
        .collect();
    let baseline = &originals[4].1;
    let fields: Vec<_> = TYPES
        .iter()
        .map(|kind| format!("enemy{kind}Damage"))
        .chain(["enemyLightningPen", "enemyColdPen", "enemyFirePen"].map(String::from))
        .collect();
    let zero = config_field(
        baseline,
        "Input",
        "enemyPhysicalDamage",
        Some(("number", "0")),
    );
    push(&mut cases, "physical-explicit-zero", zero);
    let mut changed = baseline.clone();
    for (key, value) in fields
        .iter()
        .zip(["0", "10.25", "-3.5", "123.75", "2.5", "0", "-2.25", "17.5"])
    {
        changed = config_field(&changed, "Input", key, Some(("number", value)));
    }
    push(&mut cases, "signed-fractional-inputs", changed.clone());
    push(
        &mut cases,
        "changed-physical-input",
        config_field(
            &changed,
            "Input",
            "enemyPhysicalDamage",
            Some(("number", "125")),
        ),
    );
    push(
        &mut cases,
        "removed-physical-input",
        config_field(&changed, "Input", "enemyPhysicalDamage", None),
    );
    for (name, value) in [
        ("changed-saved-placeholders", Some("12345.5")),
        ("zero-saved-placeholders", Some("0")),
        ("missing-saved-placeholders", None),
    ] {
        let mut xml = baseline.clone();
        for key in &fields {
            xml = config_field(&xml, "Placeholder", key, value.map(|v| ("number", v)));
        }
        push(&mut cases, name, xml);
    }
    for level in [1, 81, 82, 83, 85, 1000, 0, -1] {
        push(
            &mut cases,
            &format!("enemy-level-{level}"),
            config_field(
                baseline,
                "Input",
                "enemyLevel",
                Some(("number", &level.to_string())),
            ),
        );
    }
    for category in [
        "Average",
        "Untyped",
        "DamageOverTime",
        "Melee",
        "Projectile",
        "Spell",
        "SpellProjectile",
    ] {
        push(
            &mut cases,
            &format!("category-{category}"),
            config_field(
                baseline,
                "Input",
                "enemyDamageType",
                Some(("string", category)),
            ),
        );
    }
    push(
        &mut cases,
        "category-absent",
        config_field(baseline, "Input", "enemyDamageType", None),
    );
    for (name, attribute, value) in [
        (
            "category-unknown",
            "string",
            "__unknown_incoming_category__",
        ),
        ("category-number", "number", "7"),
    ] {
        // The complete original source, including its controls, decides the result.
        cases.push(Case {
            name: name.into(),
            xml: config_field(
                baseline,
                "Input",
                "enemyDamageType",
                Some((attribute, value)),
            ),
            warm: None,
            original: false,
            expected_failure: true,
        });
    }
    let custom = add_to_config(
        baseline,
        "<CustomModifierBlock title=\"Incoming witness\" enabled=\"true\">Nearby enemies deal 10 to 14 Physical Damage\nNearby enemies deal 2 to 6 Fire Damage</CustomModifierBlock>",
    );
    push(&mut cases, "source-parsed-enemy-min-max", custom);
    cases.push(Case {
        name: "repeat-original-05".into(),
        xml: baseline.clone(),
        warm: None,
        original: true,
        expected_failure: false,
    });
    cases.push(Case {
        name: "warm-changed-to-original-05".into(),
        xml: baseline.clone(),
        warm: Some(changed),
        original: true,
        expected_failure: false,
    });
    fs::create_dir_all(out.join("inputs")).unwrap();
    let mut observed = Vec::new();
    for case in &cases {
        eprintln!("complete incoming damage source case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            &case.xml,
        )
        .unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("incomingDamageJit", enabled)?;
            lua.load("if incomingDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("incomingDamagePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@incoming-damage-authentication")
                .eval::<Function>()?)
        };
        let after = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("incomingDamagePhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@incoming-damage-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            Some(&install),
            Some(&after),
        );
        let mut row = json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|xml|digest(xml.as_bytes())),"expected_failure":case.expected_failure,"raw_config":raw_config(&case.xml)});
        match result {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                row["available"] = json!(true);
                row["state"] = value["additional_observation"].clone();
            }
            Err(error) => {
                row["available"] = json!(false);
                row["source_error"] = json!(error.to_string());
            }
        }
        observed.push(row);
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
        assert_eq!(fs::read_to_string(dir.join(name)).unwrap(), *xml);
    }
    let evidence = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),"evidence":{"case_count":cases.len(),"complete_load_attempts_per_jit":cases.len()+1,"observer_sha256":digest(OBSERVE.as_bytes()),"business_method_wrappers":false,"native_coverage":false,"whole_build_parity":false,"files":FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),"originals":originals.iter().map(|(name,xml)|json!({"name":name,"sha256":digest(xml.as_bytes())})).collect::<Vec<_>>()},"cases":observed});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    check(&evidence);
}

fn check(evidence: &Json) {
    let cases = rows(&evidence["cases"]);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        if case["expected_failure"] == true {
            assert_eq!(
                case["available"], false,
                "{name}: malformed category unexpectedly completed"
            );
            assert!(
                case["source_error"]
                    .as_str()
                    .is_some_and(|v| v.contains("CalcDefence.lua")),
                "{name}: expected original calculation error, got {}",
                bounded(&case["source_error"].to_string())
            );
            continue;
        }
        assert_eq!(
            case["available"],
            true,
            "{name}: {}",
            bounded(&case["source_error"].to_string())
        );
        let state = &case["state"];
        for key in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "saved_specs_preserved",
            "cached_outputs_preserved",
            "hook_restored",
        ] {
            assert_eq!(state[key], true, "{name}: {key}");
        }
        assert_eq!(state["business_method_wrappers"], false);
        assert!(
            !rows(&state["constructors"]).is_empty(),
            "{name}: fresh constructor missing"
        );
        assert!(
            !rows(&state["load_entries"]).is_empty(),
            "{name}: load entry missing"
        );
        for constructor in rows(&state["constructors"]) {
            assert_eq!(
                constructor["default_state"]["enemyDamageType"], "Average",
                "{name}: fresh constructor default"
            );
            assert!(constructor["input"]["enemyDamageType"].is_null());
        }
        let table = rows(&state["monster_damage_table"]);
        assert_eq!(
            table.len(),
            100,
            "{name}: complete live source damage table"
        );
        assert!(
            table
                .iter()
                .all(|value| value.as_f64().is_some_and(f64::is_finite))
        );
        for (index, expected) in [
            (0, 9.1599998474121),
            (80, 343.60000610352),
            (81, 353.67001342773),
            (82, 364.0),
            (84, 385.42001342773),
            (99, 584.04998779297),
        ] {
            assert_eq!(table[index].as_f64().unwrap(), expected);
        }
        assert_eq!(state["max_enemy_level"], 85);
        assert_eq!(state["pinnacle_multiplier"].as_f64().unwrap(), 8.0 / 4.40);
        assert_eq!(state["pinnacle_pen"].as_f64().unwrap(), 15.0 / 5.0);
        for mode in ["main", "calcs"] {
            let view = &state[mode];
            assert_eq!(view["mode"], if mode == "main" { "MAIN" } else { "CALCS" });
            assert_eq!(view["input"]["enemyIsBoss"], "Pinnacle");
            let default_level = view["default_level"].as_u64().unwrap() as usize;
            assert_eq!(view["monster_damage"], table[default_level - 1]);
            if view["input"]["enemyDamageType"].is_null() {
                assert_eq!(
                    view["category"], "Average",
                    "{name}: original calculation fallback"
                );
            } else {
                assert_eq!(view["category"], view["input"]["enemyDamageType"]);
            }
            if view["category"] == "DamageOverTime" {
                assert!(rows(&view["rows"]).is_empty());
                assert_eq!(view["total_in"], 0);
                for kind in TYPES {
                    assert_eq!(view["player"][format!("{kind}EnemyPen")], 0);
                }
            } else {
                assert_eq!(rows(&view["rows"]).len(), 5);
                for (row, kind) in rows(&view["rows"]).iter().zip(TYPES) {
                    assert_eq!(row["damage_type"], kind);
                    for query in ["minimum", "maximum"] {
                        assert_eq!(row[query]["operation"], "BASE");
                        assert_eq!(
                            row[query]["cfg"]["keywordFlags"],
                            state["enemy_keyword_flags"]
                        );
                    }
                }
            }
        }
        if name.starts_with("original-") {
            for mode in ["main", "calcs"] {
                assert_eq!(state[mode]["total_in"], 4246, "{name}: {mode}");
            }
        }
    }
    let baseline = &named(cases, "original-05")["state"];
    for case in cases.iter().filter(|case| case["available"] == true) {
        for field in [
            "monster_damage_table",
            "max_enemy_level",
            "pinnacle_multiplier",
            "pinnacle_pen",
        ] {
            same(
                &baseline[field],
                &case["state"][field],
                &format!("{}/{field}", case["name"]),
            );
        }
    }
    for case in cases.iter().filter(|case| {
        case["available"] == true && !case["name"].as_str().unwrap().starts_with("original-")
    }) {
        for mode in ["main", "calcs"] {
            for metric in ["Life", "Mana", "EnergyShield", "Str", "Dex", "Int"] {
                let expected = &baseline[mode]["player"][metric];
                assert!(
                    !expected.is_null(),
                    "baseline missing unaffected metric {metric}"
                );
                assert_eq!(
                    &case["state"][mode]["player"][metric], expected,
                    "{} {mode} unaffected {metric}",
                    case["name"]
                );
            }
        }
    }
    for name in [
        "changed-saved-placeholders",
        "zero-saved-placeholders",
        "missing-saved-placeholders",
        "category-absent",
        "repeat-original-05",
        "warm-changed-to-original-05",
    ] {
        let other = &named(cases, name)["state"];
        for field in ["main", "calcs", "input", "placeholder", "selected"] {
            same(&baseline[field], &other[field], &format!("{name}/{field}"));
        }
    }
    for mode in ["main", "calcs"] {
        let explicit_average = &named(cases, "category-Average")["state"][mode];
        assert!(baseline[mode]["input"]["enemyDamageType"].is_null());
        assert_eq!(explicit_average["input"]["enemyDamageType"], "Average");
        for field in ["category", "rows", "total_in", "output", "player", "enemy"] {
            same(
                &baseline[mode][field],
                &explicit_average[field],
                &format!("explicit-average/{mode}/{field}"),
            );
        }
        assert_eq!(
            named(cases, "physical-explicit-zero")["state"][mode]["total_in"],
            3281
        );
        for (row, expected) in
            rows(&named(cases, "signed-fractional-inputs")["state"][mode]["rows"])
                .iter()
                .zip([0.0, 10.25, -3.5, 123.75, 2.5])
        {
            assert_eq!(row["raw_input"].as_f64().unwrap(), expected);
            assert_eq!(row["preconversion_amount"].as_f64().unwrap(), expected);
        }
        let modified = &named(cases, "source-parsed-enemy-min-max")["state"][mode];
        assert_eq!(modified["total_in"], 4262);
        for (index, low, high) in [(0, 10.0, 14.0), (3, 2.0, 6.0)] {
            let row = &rows(&modified["rows"])[index];
            assert_eq!(row["minimum"]["result"].as_f64().unwrap(), low);
            assert_eq!(row["maximum"]["result"].as_f64().unwrap(), high);
            assert!(!rows(&row["minimum"]["bucket_membership"]).is_empty());
            assert!(!rows(&row["maximum"]["bucket_membership"]).is_empty());
        }
    }
    for level in [1, 81, 82, 83, 85, 1000, 0, -1] {
        let state = &named(cases, &format!("enemy-level-{level}"))["state"];
        let effective = if level > 0 {
            f64::from(level).min(state["max_enemy_level"].as_f64().unwrap())
        } else {
            82.0
        };
        assert_eq!(state["main"]["enemy_level"].as_f64().unwrap(), effective);
        assert_eq!(
            state["main"]["default_level"].as_f64().unwrap(),
            effective.max(82.0)
        );
    }
}
fn config_field(xml: &str, element: &str, key: &str, value: Option<(&str, &str)>) -> String {
    let document = roxmltree::Document::parse(xml).unwrap();
    let mut ranges: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name(element) && node.attribute("name") == Some(key))
        .map(|node| node.range())
        .collect();
    ranges.sort_by_key(|range| range.start);
    let mut result = xml.to_owned();
    for range in ranges.into_iter().rev() {
        result.replace_range(range, "");
    }
    if let Some((attribute, value)) = value {
        assert!(!value.contains(['<', '&', '"']));
        result = add_to_config(
            &result,
            &format!("<{element} name=\"{key}\" {attribute}=\"{value}\"/>"),
        );
    }
    result
}
fn add_to_config(xml: &str, body: &str) -> String {
    assert_eq!(xml.matches("</ConfigSet>").count(), 1);
    xml.replacen("</ConfigSet>", &format!("{body}</ConfigSet>"), 1)
}
fn raw_config(xml: &str) -> Json {
    let document = roxmltree::Document::parse(xml).unwrap();
    json!(
        document
            .descendants()
            .filter(
                |node| (node.has_tag_name("Input") || node.has_tag_name("Placeholder"))
                    && node
                        .attribute("name")
                        .is_some_and(|name| name.starts_with("enemy"))
            )
            .map(|node| {
                let attributes: std::collections::BTreeMap<_, _> = node
                    .attributes()
                    .map(|attribute| (attribute.name(), attribute.value()))
                    .collect();
                json!({"element":node.tag_name().name(),"attributes":attributes})
            })
            .collect::<Vec<_>>()
    )
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|map| map.is_empty()));
        &[]
    }
}
fn named<'a>(cases: &'a [Json], name: &str) -> &'a Json {
    cases.iter().find(|case| case["name"] == name).unwrap()
}
fn push(cases: &mut Vec<Case>, name: &str, xml: String) {
    cases.push(Case {
        name: name.into(),
        xml,
        warm: None,
        original: false,
        expected_failure: false,
    });
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bounded(value: &str) -> String {
    value.chars().take(1200).collect()
}
fn same(left: &Json, right: &Json, label: &str) {
    if let Some(difference) = json_evidence::first_difference(left, right, "$") {
        panic!("{label}: {difference}; complete JSON retained in source evidence files");
    }
}
fn tail(path: &Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = fs::File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    file.seek(SeekFrom::Start(length.saturating_sub(12000)))
        .unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes).into()
}
