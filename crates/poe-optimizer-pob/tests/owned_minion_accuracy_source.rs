//! Complete, fresh source loads for actual minion accuracy and enemy block.
//! Distance controls are reference evidence, not native player coverage.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
#[path = "../../../tests/support/minion_accuracy_source_vectors.rs"]
mod vectors;

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

const TEST: &str = "fresh_minion_accuracy_and_block_follow_original_source";
const CHILD: &str = "POE_MINION_ACCURACY_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_minion_accuracy_source.lua");
const SNIPER: &str = "SummonSkeletalSnipersPlayer";
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
    "src/Classes/ConfigTab.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/ModParser.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModStore.lua",
    "src/Data/Gems.lua",
    "src/Data/Minions.lua",
    "src/Data/Misc.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/act_dex.lua",
    "src/Data/Skills/minion.lua",
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
fn fresh_minion_accuracy_and_block_follow_original_source() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-minion-accuracy-source-01");
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
            // Fresh full loads plus read-only store/state snapshots are much
            // slower than evaluation alone; allow bounded headroom for CI.
            if start.elapsed() > Duration::from_secs(1200) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json")),
        "exact scoped JIT evidence"
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
                .find(|v| v["xml"] == filename)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            (filename, xml)
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
        })
        .collect();
    let sniper = &originals[4].1;
    for (name, input, placeholder) in [
        ("block-placeholder-37", None, Some("37")),
        ("block-input-zero-placeholder-37", Some("0"), Some("37")),
        ("block-input-25-placeholder-37", Some("25"), Some("37")),
        ("block-input-negative", Some("-10"), None),
        ("block-input-fractional", Some("12.5"), None),
        ("block-input-100", Some("100"), None),
        ("block-input-125", Some("125"), None),
    ] {
        push(
            &mut cases,
            name,
            config_number(sniper, "enemyBlockChance", input, placeholder),
        );
    }
    for level in [1, 40] {
        push(
            &mut cases,
            &format!("sniper-physical-{level}"),
            sniper_level(sniper, level),
        );
    }
    cases.push(Case {
        name: "repeat-original-05".into(),
        xml: sniper.clone(),
        warm: None,
        original: true,
    });
    cases.push(Case {
        name: "warm-block-high-to-original".into(),
        xml: sniper.clone(),
        warm: Some(config_number(
            &sniper_level(sniper, 40),
            "enemyBlockChance",
            Some("125"),
            None,
        )),
        original: true,
    });
    push(
        &mut cases,
        "block-cannot-block-custom",
        custom(
            &config_number(sniper, "enemyBlockChance", Some("37"), None),
            "Monsters cannot block your attacks",
        ),
    );
    for mode in ["EFFECTIVE", "COMBAT"] {
        let xml = calcs_input(
            &config_number(sniper, "enemyBlockChance", None, Some("37")),
            "skill_number",
            "number",
            "3",
        );
        push(
            &mut cases,
            &format!("sniper-calcs-{}", mode.to_lowercase()),
            calcs_input(&xml, "misc_buffMode", "string", mode),
        );
    }
    // Original02 supplies the actual Twister build; no replacement actor or skill.
    let twister = &originals[1].1;
    for (name, value) in [
        ("negative", "-10"),
        ("start", "20"),
        ("fractional", "55.5"),
        ("end", "90"),
        ("above", "120"),
    ] {
        push(
            &mut cases,
            &format!("distance-{name}"),
            config_number(twister, "enemyDistance", Some(value), None),
        );
    }
    for (name, input, placeholder) in [
        ("distance-absent", None, None),
        ("distance-placeholder-80", None, Some("80")),
        ("distance-input-zero-placeholder-80", Some("0"), Some("80")),
        ("distance-input-50-placeholder-80", Some("50"), Some("80")),
        ("distance-placeholder-zero", None, Some("0")),
    ] {
        push(
            &mut cases,
            name,
            config_number(twister, "enemyDistance", input, placeholder),
        );
    }
    for mode in ["EFFECTIVE", "COMBAT"] {
        let xml = calcs_input(
            &config_number(twister, "enemyDistance", None, Some("80")),
            "skill_number",
            "number",
            "8",
        );
        push(
            &mut cases,
            &format!("distance-calcs-{}", mode.to_lowercase()),
            calcs_input(&xml, "misc_buffMode", "string", mode),
        );
    }
    assert_eq!(cases.len(), 31);
    let mut observed_cases = Vec::new();
    for case in &cases {
        eprintln!("complete minion accuracy case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("minionAccuracyJit", enabled)?;
            lua.load("if minionAccuracyJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("minionAccuracyPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@minion-accuracy-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("minionAccuracyPhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@minion-accuracy-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let observed = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        let row = match observed {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|v|digest(v.as_bytes())),"xml_config":xml_config(&case.xml),"available":true,"state":value["additional_observation"]})
            }
            Err(error) => {
                json!({"name":case.name,"available":false,"source_error":error.to_string()})
            }
        };
        observed_cases.push(row);
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&observed_cases).unwrap(),
        )
        .unwrap();
    }
    for (filename, xml) in &originals {
        assert_eq!(fs::read_to_string(fixtures.join(filename)).unwrap(), *xml);
    }
    let evidence = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),
        "evidence":{"case_count":cases.len(),"complete_load_attempts_per_jit":cases.len()+1,"observer_sha256":digest(OBSERVE.as_bytes()),
            "business_method_wrappers":false,"actor_level_mutation":false,"native_coverage":false,"whole_build_parity":false,
            "files":FILES.iter().map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})).collect::<Vec<_>>(),
            "originals":originals.iter().map(|(n,x)|json!({"name":n,"sha256":digest(x.as_bytes())})).collect::<Vec<_>>()},"cases":observed_cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    check(&evidence);
    assert_eq!(
        vectors::project(&evidence),
        read(&root.join("tests/fixtures/calibration/minion-accuracy-3887ae68.json")),
        "fresh source differs from committed native parity vectors"
    );
}

#[test]
#[ignore = "manual candidate export from matching, validated complete-source evidence"]
fn export_minion_accuracy_calibration_candidate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-minion-accuracy-source-01");
    let off = fs::read(out.join("source-jit-off.json")).unwrap();
    let on = fs::read(out.join("source-jit-on.json")).unwrap();
    assert_eq!(off, on, "candidate requires exact JIT agreement");
    let source: Json = serde_json::from_slice(&off).unwrap();
    assert_eq!(
        source["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(source["source_hash"], pinned::manifest_sha256());
    assert_eq!(
        source["evidence"]["observer_sha256"],
        digest(OBSERVE.as_bytes())
    );
    assert_eq!(
        source["evidence"]["files"],
        json!(
            FILES
                .iter()
                .map(|p| json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()}))
                .collect::<Vec<_>>()
        )
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    assert_eq!(
        source["evidence"]["originals"],
        json!(
            (1..=5)
                .map(|n| {
                    let name = format!("build-{n:02}.xml");
                    json!({"name":name,"sha256":digest(&fs::read(fixtures.join(name)).unwrap())})
                })
                .collect::<Vec<_>>()
        )
    );
    check(&source);
    // Explicit candidate only. The normal test never changes either this file
    // or the committed fixture; accepting a candidate is a separate action.
    fs::write(
        out.join("calibration-candidate.json"),
        serde_json::to_vec_pretty(&vectors::project(&source)).unwrap(),
    )
    .unwrap();
}

fn check(evidence: &Json) {
    let cases = rows(&evidence["cases"]);
    assert_eq!(cases.len(), 31);
    for case in cases {
        assert_eq!(
            case["available"], true,
            "{}: {}",
            case["name"], case["source_error"]
        );
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
            "saved_specs_preserved",
        ] {
            assert_eq!(case["state"][field], true);
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        assert_eq!(case["state"]["business_method_wrappers"], false);
        assert_eq!(case["state"]["constants"]["falloff_start"], 20);
        assert_eq!(case["state"]["constants"]["falloff_end"], 90);
        check_config(case, "enemyBlockChance", "enemy_block", None, true);
        check_config(case, "enemyDistance", "enemy_distance", Some(20.0), false);
        for mode in ["main", "calcs"] {
            for actor in rows(&case["state"][mode]["actors"]) {
                assert_eq!(actor["fresh_actor"], true);
                if actor["summon_effect_id"] == SNIPER {
                    for child in rows(&actor["children"])
                        .iter()
                        .filter(|c| c["effect_id"] == "MinionMeleeBow")
                    {
                        assert_eq!(child["effect_name"], "Basic Attack");
                        for pass in rows_or_missing(&child["passes"]) {
                            assert_eq!(pass["flags"]["attack"], true);
                            assert_eq!(
                                pass["flags"]["player_minion_accuracy_equals_accuracy"],
                                false
                            );
                            assert_eq!(pass["flags"]["skill_cannot_be_evaded"], true);
                            assert_eq!(pass["skill_inherits_actor_modifiers"], true);
                            assert!(rows(&pass["actor_flag_records"]).iter().any(
                                |r| r["mod"]["source"] == "Minion Attacks always hit"
                                    && r["mod"]["name"] == "CannotBeEvaded"
                                    && r["mod"]["type"] == "FLAG"
                                    && r["mod"]["value"] == 1
                            ));
                            assert_eq!(pass["output"]["AccuracyHitChance"], 100);
                            check_block(pass);
                        }
                    }
                }
            }
        }
    }
    let original = named(cases, "original-05");
    let baseline = sniper_pass(original, "main");
    assert_eq!(baseline["output"]["Accuracy"], 0);
    assert_eq!(baseline["output"]["HitChance"], 100);
    for name in ["repeat-original-05", "warm-block-high-to-original"] {
        assert_eq!(
            named(cases, name)["state"],
            original["state"],
            "fresh/reused {name}"
        );
    }
    for (name, raw, clamped) in [
        ("block-placeholder-37", 37.0, 37.0),
        ("block-input-zero-placeholder-37", 0.0, 0.0),
        ("block-input-25-placeholder-37", 25.0, 25.0),
        ("block-input-negative", -10.0, 0.0),
        ("block-input-fractional", 12.5, 12.5),
        ("block-input-100", 100.0, 100.0),
        ("block-input-125", 125.0, 100.0),
    ] {
        let pass = sniper_pass(named(cases, name), "main");
        assert_eq!(number(&pass["block"]["base"]), raw, "{name}");
        assert_eq!(number(&pass["block"]["reduction"]), 0.0);
        assert_eq!(number(&pass["output"]["enemyBlockChance"]), clamped);
        close(number(&pass["output"]["HitChance"]), 100.0 - clamped);
    }
    for mode in ["effective", "combat"] {
        let case = named(cases, &format!("sniper-calcs-{mode}"));
        let pass = sniper_pass(case, "calcs");
        assert_eq!(pass["effective"], mode == "effective");
        assert_eq!(number(&pass["block"]["base"]), 37.0);
        check_block(pass);
    }
    for case in cases
        .iter()
        .filter(|c| c["name"].as_str().unwrap().starts_with("distance-"))
    {
        assert_eq!(
            case["state"]["main"]["player"]["effect_id"],
            "TwisterPlayer"
        );
        for pass in rows(&case["state"]["main"]["player"]["passes"]) {
            check_generic_accuracy(pass, &case["state"]["constants"]);
        }
    }
    let effective = named(cases, "distance-calcs-effective");
    let combat = named(cases, "distance-calcs-combat");
    for (case, enabled, distance) in [(effective, true, 80.0), (combat, false, 0.0)] {
        assert_eq!(
            case["state"]["calcs"]["player"]["effect_id"],
            "TwisterPlayer"
        );
        for pass in rows(&case["state"]["calcs"]["player"]["passes"]) {
            assert_eq!(pass["effective"], enabled);
            assert_eq!(number(&pass["accuracy"]["distance_sum"]), distance);
            check_generic_accuracy(pass, &case["state"]["constants"]);
        }
    }
    // Authored custom text must survive parsing; recipient behavior is observed.
    let custom = named(cases, "block-cannot-block-custom");
    assert!(
        rows(&custom["state"]["config"]["custom_blocks"])
            .iter()
            .any(|b| b["text"] == "Monsters cannot block your attacks")
    );
    let custom_pass = sniper_pass(custom, "main");
    assert_eq!(custom_pass["flags"]["enemy_cannot_block_attacks"], true);
    assert_eq!(custom_pass["block"]["base"], 37);
    assert_eq!(custom_pass["output"]["enemyBlockChance"], 0);
    assert_eq!(custom_pass["output"]["HitChance"], 100);
    assert!(
        rows(&custom_pass["block"]["cannot_block_records"])
            .iter()
            .any(|entry| {
                let modifier = &entry["mod"];
                modifier["name"] == "CannotBlockAttacks"
                    && modifier["type"] == "FLAG"
                    && modifier["source"] == "Custom:Accuracy source control"
                    && modifier["value"] == true
            })
    );
    check_block(custom_pass);
}
fn check_block(pass: &Json) {
    let block = if pass["flags"]["attack"] == true
        && pass["flags"]["enemy_cannot_block_attacks"] == true
    {
        0.0
    } else {
        (number(&pass["block"]["base"]).min(100.0) - number(&pass["block"]["reduction"])).max(0.0)
    };
    close(number(&pass["output"]["enemyBlockChance"]), block);
    close(
        number(&pass["output"]["HitChance"]),
        number(&pass["output"]["AccuracyHitChance"]) * (1.0 - block / 100.0),
    );
    assert_eq!(pass["query_state_preserved"], true);
}
fn check_generic_accuracy(pass: &Json, constants: &Json) {
    let a = &pass["accuracy"];
    let f = &pass["flags"];
    assert_eq!(f["attack"], true);
    assert_eq!(f["offhand_accuracy_is_main"], false);
    assert_eq!(f["hit_chance_can_exceed_100"], false);
    let accuracy =
        (number(&a["base"]) * (1.0 + number(&a["increased"]) / 100.0) * number(&a["more"]))
            .floor()
            .max(0.0);
    close(number(&pass["output"]["Accuracy"]), accuracy);
    let mut versus = (number(&a["base_vs_enemy"])
        * (1.0 + number(&a["increased_vs_enemy"]) / 100.0)
        * number(&a["more_vs_enemy"]))
    .floor()
    .max(0.0);
    if f["no_accuracy_distance_penalty"] != true {
        let start = number(&constants["falloff_start"]);
        let end = number(&constants["falloff_end"]);
        let distance = number(&a["distance_sum"]).clamp(start, end);
        let penalty = 1.0
            - (distance - start) / (end - start)
                * (number(&constants["max_penalty"]) * number(&a["penalty_multiplier"])).floor()
                / 100.0;
        versus = (versus * penalty).floor();
    }
    let cannot = f["skill_cannot_be_evaded"] == true
        || f["skill_data_cannot_be_evaded"] == true
        || (pass["effective"] == true && f["enemy_cannot_evade"] == true);
    let hit = if cannot {
        100.0
    } else {
        ((versus * 1.25) / (versus + number(&a["enemy_evasion"]) * 0.3) * 100.0)
            .round()
            .clamp(5.0, 100.0)
            * number(&a["hit_chance_multiplier"])
    };
    close(number(&pass["output"]["AccuracyHitChance"]), hit);
    check_block(pass);
}
fn check_config(case: &Json, key: &str, field: &str, default: Option<f64>, zero_wins: bool) {
    let intent = &case["xml_config"][key];
    let raw = &case["state"]["config"][field];
    let input = intent["input"].as_str().map(|s| s.parse::<f64>().unwrap());
    let placeholder = intent["placeholder"]
        .as_str()
        .map(|s| s.parse::<f64>().unwrap())
        .or(default);
    assert_eq!(raw["input_present"], input.is_some());
    assert_eq!(raw["placeholder_present"], placeholder.is_some());
    if let Some(v) = input {
        close(number(&raw["input"]), v)
    }
    if let Some(v) = placeholder {
        close(number(&raw["placeholder"]), v)
    }
    if zero_wins {
        let records = rows(&case["state"]["config"]["block_records"]);
        if let Some(value) = input.or(placeholder) {
            assert_eq!(records.len(), 1);
            close(number(&records[0]["value"]), value);
            assert_eq!(records[0]["mod"]["source"], "Config");
        } else {
            assert!(records.is_empty());
        }
    } else {
        let expected = input
            .filter(|v| *v != 0.0)
            .or(placeholder.filter(|v| *v != 0.0))
            .unwrap_or(0.0);
        for pass in rows_or_missing(&case["state"]["main"]["player"]["passes"]) {
            close(number(&pass["accuracy"]["distance_sum"]), expected);
        }
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9, "{a} != {b}");
}
fn number(v: &Json) -> f64 {
    v.as_f64().unwrap()
}
fn sniper_pass<'a>(case: &'a Json, mode: &str) -> &'a Json {
    let actors: Vec<_> = rows(&case["state"][mode]["actors"])
        .iter()
        .filter(|a| a["summon_effect_id"] == SNIPER)
        .collect();
    assert_eq!(actors.len(), 1);
    let children: Vec<_> = rows(&actors[0]["children"])
        .iter()
        .filter(|c| c["effect_id"] == "MinionMeleeBow")
        .collect();
    assert_eq!(children.len(), 1);
    let passes = rows(&children[0]["passes"]);
    assert_eq!(passes.len(), 1);
    &passes[0]
}
fn named<'a>(values: &'a [Json], name: &str) -> &'a Json {
    let matches: Vec<_> = values.iter().filter(|v| v["name"] == name).collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
fn rows(v: &Json) -> &[Json] {
    if let Some(rows) = v.as_array() {
        rows
    } else {
        assert!(
            v.as_object().is_some_and(|o| o.is_empty()),
            "not source list: {v}"
        );
        &[]
    }
}
fn rows_or_missing(v: &Json) -> &[Json] {
    if v.is_null() { &[] } else { rows(v) }
}
fn read(p: &Path) -> Json {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn push(cases: &mut Vec<Case>, name: &str, xml: String) {
    cases.push(Case {
        name: name.into(),
        xml,
        warm: None,
        original: false,
    });
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn selected_config<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> roxmltree::Node<'a, 'i> {
    let config = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let id = config.attribute("activeConfigSet").unwrap();
    config
        .children()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(id))
        .unwrap()
}
fn replace_children(
    xml: &str,
    node: roxmltree::Node<'_, '_>,
    remove: impl Fn(roxmltree::Node<'_, '_>) -> bool,
    extra: &str,
) -> String {
    let mut edits: Vec<_> = node
        .children()
        .filter(|n| remove(*n))
        .map(|n| (n.range(), String::new()))
        .collect();
    let end = node.range().end - format!("</{}>", node.tag_name().name()).len();
    edits.push((end..end, extra.to_owned()));
    edits.sort_by_key(|(range, _)| range.start);
    let mut out = xml.to_owned();
    for (range, value) in edits.into_iter().rev() {
        out.replace_range(range, &value)
    }
    out
}
fn config_number(xml: &str, key: &str, input: Option<&str>, placeholder: Option<&str>) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_config(&doc);
    let extra = [("Input", input), ("Placeholder", placeholder)]
        .into_iter()
        .filter_map(|(kind, value)| {
            value.map(|v| format!("<{kind} name=\"{key}\" number=\"{v}\"/>"))
        })
        .collect::<String>();
    replace_children(
        xml,
        set,
        |n| {
            (n.has_tag_name("Input") || n.has_tag_name("Placeholder"))
                && n.attribute("name") == Some(key)
        },
        &extra,
    )
}
fn calcs_input(xml: &str, key: &str, kind: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let calcs = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Calcs"))
        .unwrap();
    replace_children(
        xml,
        calcs,
        |n| n.has_tag_name("Input") && n.attribute("name") == Some(key),
        &format!("<Input name=\"{key}\" {kind}=\"{value}\"/>"),
    )
}
fn custom(xml: &str, text: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_config(&doc);
    replace_children(
        xml,
        set,
        |_| false,
        &format!(
            "<CustomModifierBlock title=\"Accuracy source control\" enabled=\"true\">{}</CustomModifierBlock>",
            escape(text)
        ),
    )
}
fn xml_config(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_config(&doc);
    let mut out = serde_json::Map::new();
    for name in ["enemyBlockChance", "enemyDistance"] {
        let mut row = serde_json::Map::new();
        for (tag, key) in [("Input", "input"), ("Placeholder", "placeholder")] {
            let matches: Vec<_> = set
                .children()
                .filter(|n| n.has_tag_name(tag) && n.attribute("name") == Some(name))
                .collect();
            assert!(matches.len() <= 1);
            if let Some(node) = matches.first() {
                row.insert(key.into(), json!(node.attribute("number").unwrap()));
            }
        }
        out.insert(name.into(), Json::Object(row));
    }
    Json::Object(out)
}
fn sniper_level(xml: &str, level: u32) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let id = skills.attribute("activeSkillSet").unwrap();
    let set = skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(id))
        .unwrap();
    let gems: Vec<_> = set
        .descendants()
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(SNIPER))
        .collect();
    assert_eq!(gems.len(), 1);
    let gem = gems[0];
    let attrs = gem
        .attributes()
        .map(|a| {
            format!(
                "{}=\"{}\"",
                a.name(),
                escape(&if a.name() == "level" {
                    level.to_string()
                } else {
                    a.value().into()
                })
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = xml.to_owned();
    out.replace_range(gem.range(), &format!("<Gem {attrs}/>"));
    out
}
fn tail(path: &Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    file.seek(SeekFrom::Start(length.saturating_sub(16 * 1024)))
        .unwrap();
    let mut bytes = Vec::new();
    file.take(16 * 1024).read_to_end(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}
