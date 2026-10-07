//! Incoming critical inputs observed during complete, unmodified pinned PoB loads.
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

const TEST: &str = "complete_original_incoming_critical_inputs_and_queries";
const CHILD: &str = "POE_INCOMING_CRITICAL_SOURCE_CHILD";
const OUT: &str = "POE_INCOMING_CRITICAL_SOURCE_OUT";
const OBSERVE: &str = include_str!("support/owned_incoming_critical_source.lua");
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
}

#[test]
#[ignore = "complete pinned PoB loads; writes independent JIT source evidence"]
fn complete_original_incoming_critical_inputs_and_queries() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = match std::env::var_os(OUT) {
        Some(path) => root.join(path),
        None => {
            fs::create_dir_all(root.join("runs")).unwrap();
            tempfile::Builder::new()
                .prefix("owned-incoming-critical-source-")
                .tempdir_in(root.join("runs"))
                .unwrap()
                .keep()
        }
    };
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    eprintln!("incoming critical evidence: {}", out.display());
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUT, &out)
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
        "incoming critical JIT source evidence",
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let baseline = fs::read_to_string(dir.join("build-05.xml")).unwrap();
    let entry = rows(&index["builds"])
        .iter()
        .find(|row| row["xml"] == "build-05.xml")
        .unwrap();
    assert_eq!(entry["xml_sha256"], digest(baseline.as_bytes()));
    let mut cases = vec![Case {
        name: "original-05".into(),
        xml: baseline.clone(),
        warm: None,
        original: true,
    }];
    for (field, values) in [
        ("enemyCritChance", &["0", "-3.5", "12.5", "150"][..]),
        ("enemyCritDamage", &["0", "-17.25", "37.5"][..]),
    ] {
        for value in values {
            push(
                &mut cases,
                &format!("{field}-{value}"),
                config_field(&baseline, "Input", field, Some(("number", value))),
            );
        }
    }
    for (name, value) in [
        ("changed-saved-placeholders", Some("12345.5")),
        ("zero-saved-placeholders", Some("0")),
        ("missing-saved-placeholders", None),
    ] {
        let mut xml = baseline.clone();
        for key in ["enemyCritChance", "enemyCritDamage"] {
            xml = config_field(&xml, "Placeholder", key, value.map(|v| ("number", v)));
        }
        push(&mut cases, name, xml);
    }
    let mut changed = baseline.clone();
    for (field, value) in [("enemyCritChance", "12.5"), ("enemyCritDamage", "37.5")] {
        changed = config_field(&changed, "Input", field, Some(("number", value)));
        changed = config_field(&changed, "Placeholder", field, Some(("number", "9999")));
    }
    push(
        &mut cases,
        "authored-inputs-with-conflicting-placeholders",
        changed.clone(),
    );
    for (name, text) in [
        ("never", "Nearby Enemies cannot deal Critical Hits"),
        ("always", "Hits against you are always Critical Hits"),
        (
            "never-and-always",
            "Nearby Enemies cannot deal Critical Hits\nHits against you are always Critical Hits",
        ),
        (
            "unlucky",
            "Enemy Critical Hit Chance against you is Unlucky",
        ),
        (
            "always-and-unlucky",
            "Hits against you are always Critical Hits\nEnemy Critical Hit Chance against you is Unlucky",
        ),
        (
            "actor-and-enemy-increase",
            "Hits have 20% increased Critical Hit Chance against you\nNearby Enemies have 30% increased Critical Hit Chance",
        ),
        (
            "enemy-bonus-base-and-increase",
            "Nearby Enemies have +10% to Critical Damage Bonus\nNearby Enemies have 25% increased Critical Damage Bonus",
        ),
        (
            "reduction",
            "You take 25% reduced extra damage from Critical Hits",
        ),
        (
            "reduction-upper-clamp",
            "You take 150% reduced extra damage from Critical Hits",
        ),
        (
            "negative-reduction",
            "You take 25% increased extra damage from Critical Hits",
        ),
        ("configured-evade", "+1000 to Evasion Rating"),
    ] {
        assert!(!text.contains(['<', '&']));
        push(
            &mut cases,
            name,
            add_to_config(
                &baseline,
                &format!(
                    "<CustomModifierBlock title=\"Incoming critical witness\" enabled=\"true\">{text}</CustomModifierBlock>"
                ),
            ),
        );
    }
    push(
        &mut cases,
        "damage-over-time",
        config_field(
            &baseline,
            "Input",
            "enemyDamageType",
            Some(("string", "DamageOverTime")),
        ),
    );
    cases.push(Case {
        name: "repeat-original-05".into(),
        xml: baseline.clone(),
        warm: None,
        original: true,
    });
    cases.push(Case {
        name: "warm-changed-to-original-05".into(),
        xml: baseline.clone(),
        warm: Some(changed),
        original: true,
    });
    fs::create_dir_all(out.join("inputs")).unwrap();
    let mut observed = Vec::new();
    for case in &cases {
        eprintln!("complete incoming critical source case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            &case.xml,
        )
        .unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("incomingCriticalJit", enabled)?;
            lua.load("if incomingCriticalJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("incomingCriticalPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@incoming-critical-authentication")
                .eval::<Function>()?)
        };
        let after = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("incomingCriticalPhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@incoming-critical-observation")
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
        let mut row = json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),
            "warm_xml_sha256":case.warm.as_ref().map(|xml|digest(xml.as_bytes())),"raw_config":raw_config(&case.xml)});
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
    assert_eq!(
        fs::read_to_string(dir.join("build-05.xml")).unwrap(),
        baseline
    );
    let evidence = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "source_hash":pinned::manifest_sha256(),"evidence":{"case_count":cases.len(),
        "complete_load_attempts_per_jit":cases.len()+1,"observer_sha256":digest(OBSERVE.as_bytes()),
        "business_method_wrappers":false,"native_coverage":false,"whole_build_parity":false,
        "scope":"complete original Player incoming-critical branch; no direct modDB injection",
        "lowercase_override":"bounded absence in these ordinary XML/custom-modifier loads; no override-producing mechanic admitted",
        "files":FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original":{"name":"build-05.xml","sha256":digest(baseline.as_bytes())}},"cases":observed});
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
        assert_eq!(case["available"], true, "{name}: {}", case["source_error"]);
        let state = &case["state"];
        for key in ["original_functions_preserved", "hook_restored"] {
            assert_eq!(state[key], true, "{name}/{key}");
        }
        assert_eq!(state["business_method_wrappers"], false);
        assert!(!rows(&state["constructors"]).is_empty());
        assert!(!rows(&state["load_entries"]).is_empty());
        assert!(!rows(&state["callbacks"]).is_empty());
        assert_eq!(number(&state["default_critical_bonus"]), 30.);
        for mode in ["main", "calcs"] {
            let view = &state[mode];
            assert_eq!(view["mode"], if mode == "main" { "MAIN" } else { "CALCS" });
            let queries = &view["queries"];
            assert!(
                rows(&queries["override_chance"]["bucket_membership"]).is_empty(),
                "{name}: dormant lowercase override acquired a producer"
            );
            assert_eq!(queries["override_chance"]["result_available"], false);
            assert_eq!(number(&view["placeholder"]["enemyCritChance"]), 5.);
            assert_eq!(number(&view["placeholder"]["enemyCritDamage"]), 30.);
            if view["category"] == "DamageOverTime" {
                assert_eq!(view["hit_branch"], false);
                assert!(view["chance"].is_null() && view["critical_effect"].is_null());
                for query in queries.as_object().unwrap().values() {
                    assert_eq!(query["queried"], false);
                }
                continue;
            }
            assert_eq!(view["hit_branch"], true);
            let never = flag(&queries["never_crit"]);
            let mut chance = if never {
                assert_eq!(queries["always_crit"]["queried"], false);
                assert_eq!(queries["override_chance"]["queried"], false);
                0.
            } else if flag(&queries["always_crit"]) {
                assert_eq!(queries["override_chance"]["queried"], false);
                100.
            } else {
                assert_eq!(queries["override_chance"]["queried"], true);
                assert_eq!(queries["override_chance"]["returned"], true);
                let raw = configured(view, "enemyCritChance");
                (raw * (1.
                    + query_number(&queries["actor_chance_increase"]) / 100.
                    + query_number(&queries["enemy_chance_increase"]) / 100.)
                    * (1. - number(&view["configured_evade"]) / 100.))
                    .clamp(0., 100.)
            };
            if flag(&queries["unlucky_crit"]) {
                chance = chance / 100. * chance;
            }
            let bonus = ((configured(view, "enemyCritDamage")
                + query_number(&queries["enemy_bonus_base"]))
                * (1. + query_number(&queries["enemy_bonus_increase"]) / 100.))
                .max(0.);
            assert_eq!(queries["enemy_bonus_increase"]["flags"], state["hit_flag"]);
            let reduction = query_number(&view["reduction_query"]).min(100.);
            near(number(&view["extra_damage_reduction"]), reduction, name);
            near(number(&view["chance"]), chance, name);
            near(number(&view["damage_bonus"]), bonus, name);
            // Keep the original association, including the final reduction multiplication.
            near(
                number(&view["critical_effect"]),
                1. + chance / 100. * (bonus / 100.) * (1. - reduction / 100.),
                name,
            );
        }
        for key in [
            "category",
            "hit_branch",
            "input",
            "placeholder",
            "queries",
            "configured_evade",
            "extra_damage_reduction",
            "chance",
            "damage_bonus",
            "critical_effect",
            "reduction_query",
        ] {
            same(
                &state["main"][key],
                &state["calcs"][key],
                &format!("{name}/MAIN-CALCS/{key}"),
            );
        }
    }
    let baseline = &named(cases, "original-05")["state"];
    for name in [
        "changed-saved-placeholders",
        "zero-saved-placeholders",
        "missing-saved-placeholders",
        "repeat-original-05",
        "warm-changed-to-original-05",
    ] {
        for key in ["main", "calcs", "selected"] {
            same(
                &baseline[key],
                &named(cases, name)["state"][key],
                &format!("{name}/{key}"),
            );
        }
    }
    for mode in ["main", "calcs"] {
        let view = |name| &named(cases, name)["state"][mode];
        assert!(baseline[mode]["input"]["enemyCritChance"].is_null());
        assert!(baseline[mode]["input"]["enemyCritDamage"].is_null());
        near(number(&baseline[mode]["chance"]), 5., "baseline chance");
        near(
            number(&baseline[mode]["critical_effect"]),
            1.015,
            "baseline effect",
        );
        for (name, chance) in [
            ("enemyCritChance-0", 0.),
            ("enemyCritChance--3.5", 0.),
            ("enemyCritChance-12.5", 12.5),
            ("enemyCritChance-150", 100.),
            ("never", 0.),
            ("always", 100.),
            ("never-and-always", 0.),
            ("unlucky", 0.25),
            ("always-and-unlucky", 100.),
            ("actor-and-enemy-increase", 7.5),
        ] {
            near(number(&view(name)["chance"]), chance, name);
        }
        for (name, bonus) in [
            ("enemyCritDamage-0", 0.),
            ("enemyCritDamage--17.25", 0.),
            ("enemyCritDamage-37.5", 37.5),
            ("enemy-bonus-base-and-increase", 50.),
        ] {
            near(number(&view(name)["damage_bonus"]), bonus, name);
        }
        for (name, reduction) in [
            ("reduction", 25.),
            ("reduction-upper-clamp", 100.),
            ("negative-reduction", -25.),
        ] {
            near(
                number(&view(name)["extra_damage_reduction"]),
                reduction,
                name,
            );
            assert!(!rows(&view(name)["reduction_query"]["bucket_membership"]).is_empty());
        }
        for (name, query) in [
            ("never", "never_crit"),
            ("always", "always_crit"),
            ("unlucky", "unlucky_crit"),
        ] {
            let observed = &view(name)["queries"][query];
            assert!(flag(observed));
            assert_eq!(observed["winner"]["type"], "FLAG");
            assert_eq!(
                observed["winner"]["source"],
                "Custom:Incoming critical witness"
            );
            assert!(!rows(&observed["bucket_membership"]).is_empty());
        }
        let both = &view("never-and-always")["queries"];
        assert!(flag(&both["never_crit"]));
        assert_eq!(both["always_crit"]["queried"], false);
        assert!(!rows(&both["always_crit"]["bucket_membership"]).is_empty());
        for (name, query, expected) in [
            ("actor-and-enemy-increase", "actor_chance_increase", 20.),
            ("actor-and-enemy-increase", "enemy_chance_increase", 30.),
            ("enemy-bonus-base-and-increase", "enemy_bonus_base", 10.),
            ("enemy-bonus-base-and-increase", "enemy_bonus_increase", 25.),
        ] {
            let observed = &view(name)["queries"][query];
            near(query_number(observed), expected, name);
            assert!(!rows(&observed["bucket_membership"]).is_empty());
            assert!(
                rows(&observed["bucket_membership"])
                    .iter()
                    .any(|row| row["record"]["source"] == "Custom:Incoming critical witness")
            );
        }
        assert!(number(&view("configured-evade")["configured_evade"]) > 0.);
        assert!(number(&view("configured-evade")["chance"]) < 5.);
    }
}

fn flag(query: &Json) -> bool {
    assert_eq!(query["queried"], true);
    assert_eq!(query["result_available"], true);
    query["result"].as_bool().unwrap()
}
fn query_number(query: &Json) -> f64 {
    assert_eq!(query["queried"], true);
    assert_eq!(query["result_available"], true);
    number(&query["result"])
}
fn configured(view: &Json, key: &str) -> f64 {
    // An authored zero remains authoritative. Source placeholders were observed after callbacks.
    view["input"][key]
        .as_f64()
        .or_else(|| view["placeholder"][key].as_f64())
        .unwrap()
}
fn number(value: &Json) -> f64 {
    let value = value.as_f64().unwrap();
    assert!(value.is_finite());
    value
}
fn near(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(1.),
        "{label}: {actual} != {expected}"
    );
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
    });
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn same(left: &Json, right: &Json, label: &str) {
    if let Some(difference) = json_evidence::first_difference(left, right, "$") {
        panic!("{label}: {difference}; complete JSON retained");
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
