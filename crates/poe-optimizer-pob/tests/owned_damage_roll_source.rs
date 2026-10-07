//! Complete original calculations witness the scope of the boss damage-roll input.
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

const TEST: &str = "damage_roll_range_is_inactive_without_selected_boss_skill";
const CHILD: &str = "POE_DAMAGE_ROLL_SOURCE_CHILD";
const OUT: &str = "POE_DAMAGE_ROLL_SOURCE_OUT";
const OBSERVE: &str = include_str!("support/owned_damage_roll_source.lua");
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
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcOffence.lua",
    "src/Data/BossSkills.lua",
    "runtime/lua/xml.lua",
];
struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    original: bool,
}

#[test]
#[ignore = "complete pinned PoB loads with full scalar output comparisons and JIT evidence"]
fn damage_roll_range_is_inactive_without_selected_boss_skill() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = match std::env::var_os(OUT) {
        Some(path) => root.join(path),
        None => {
            fs::create_dir_all(root.join("runs")).unwrap();
            tempfile::Builder::new()
                .prefix("owned-damage-roll-source-")
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
    eprintln!("damage roll source evidence: {}", out.display());
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
        "damage roll JIT source evidence",
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let original = fs::read_to_string(dir.join("build-05.xml")).unwrap();
    let entry = rows(&index["builds"])
        .iter()
        .find(|row| row["xml"] == "build-05.xml")
        .unwrap();
    assert_eq!(entry["xml_sha256"], digest(original.as_bytes()));
    let mut cases = vec![Case {
        name: "original-05".into(),
        xml: original.clone(),
        warm: None,
        original: true,
    }];
    for value in ["0", "100", "37.5"] {
        push(
            &mut cases,
            &format!("input-{value}"),
            config_field(
                &original,
                "Input",
                "enemyDamageRollRange",
                Some(("number", value)),
            ),
        );
    }
    for (name, value) in [
        ("placeholder-0", Some("0")),
        ("placeholder-100", Some("100")),
        ("placeholder-missing", None),
    ] {
        push(
            &mut cases,
            name,
            config_field(
                &original,
                "Placeholder",
                "enemyDamageRollRange",
                value.map(|v| ("number", v)),
            ),
        );
    }
    let changed = config_field(
        &original,
        "Input",
        "enemyDamageRollRange",
        Some(("number", "100")),
    );
    push(
        &mut cases,
        "explicit-none-input-100",
        config_field(
            &changed,
            "Input",
            "presetBossSkills",
            Some(("string", "None")),
        ),
    );
    cases.push(Case {
        name: "repeat-original-05".into(),
        xml: original.clone(),
        warm: None,
        original: true,
    });
    cases.push(Case {
        name: "warm-changed-to-original-05".into(),
        xml: original.clone(),
        warm: Some(changed),
        original: true,
    });
    for value in ["0", "100"] {
        let xml = config_field(
            &original,
            "Input",
            "presetBossSkills",
            Some(("string", "Shaper Ball")),
        );
        push(
            &mut cases,
            &format!("boss-shaper-ball-{value}"),
            config_field(
                &xml,
                "Input",
                "enemyDamageRollRange",
                Some(("number", value)),
            ),
        );
    }
    fs::create_dir_all(out.join("inputs")).unwrap();
    let mode = if enabled { "on" } else { "off" };
    let mut observed = Vec::new();
    for case in &cases {
        eprintln!("damage roll source case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            &case.xml,
        )
        .unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("damageRollJit", enabled)?;
            lua.load("if damageRollJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("damageRollPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@damage-roll-authentication")
                .eval::<Function>()?)
        };
        let after = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("damageRollPhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@damage-roll-observation")
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
            out.join(format!("source-jit-{mode}-progress.json")),
            serde_json::to_vec_pretty(&compact(&observed)).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(
        fs::read_to_string(dir.join("build-05.xml")).unwrap(),
        original
    );
    let evidence = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),
        "evidence":{"case_count":cases.len(),"complete_load_attempts_per_jit":cases.len()+1,
        "observer_sha256":digest(OBSERVE.as_bytes()),"business_method_wrappers":false,"native_coverage":false,"whole_build_parity":false,
        "comparison_scope":"all top-level scalar outputs and complete top-level output availability/types for Player and selected minion, per mode (saved MAIN Sniper; saved CALCS Arsonist)",
        "boss_contrast_scope":"pinned PoB callback contrast only; this does not admit native boss-skill presets",
        "files":FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original":{"name":"build-05.xml","sha256":digest(original.as_bytes())}},"cases":compact(&observed)});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    check(&observed);
}

fn check(cases: &[Json]) {
    let baseline = &named(cases, "original-05")["state"];
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(case["available"], true, "{name}: {}", case["source_error"]);
        let state = &case["state"];
        let boss = name.starts_with("boss-");
        assert_eq!(state["original_functions_preserved"], true);
        assert_eq!(state["business_method_wrappers"], false);
        assert_eq!(state["hook_restored"], true);
        assert_eq!(
            state["roll_definition"],
            json!({"has_apply":false,"kind":"countAllowZero","default_placeholder":70})
        );
        let callbacks = rows(&state["callbacks"]);
        assert!(!callbacks.is_empty());
        assert_eq!(
            callbacks.last().unwrap()["value"],
            if boss { "Shaper Ball" } else { "None" }
        );
        for callback in callbacks {
            assert_eq!(
                callback["guard_observed"], true,
                "{name}: callback guard missing"
            );
            // A fresh construction may call the default before the XML's preset.
            let selected_boss = callback["value"] == "Shaper Ball";
            assert!(callback["value"] == "None" || (boss && selected_boss));
            assert_eq!(callback["boss_branch"], selected_boss);
            assert_eq!(callback["none_branch"], !selected_boss);
            assert_eq!(callback["roll_read"], selected_boss);
            assert_eq!(callback["roll_available"], selected_boss);
            if !selected_boss {
                assert!(callback["roll"].is_null());
                for field in ["damage_input", "damage_placeholder"] {
                    same(
                        &callback["before"][field],
                        &callback["after"][field],
                        &format!("{name}/callback/{field}"),
                    );
                }
            }
        }
        same(
            &state["selected"],
            &baseline["selected"],
            &format!("{name}/selection"),
        );
        for mode in ["main", "calcs"] {
            let view = &state[mode];
            assert_eq!(view["mode"], if mode == "main" { "MAIN" } else { "CALCS" });
            // The saved build deliberately has different selections: Build mainSocketGroup=3,
            // while Calcs skill_number=1. Preserve both original selections.
            assert_eq!(
                view["identity"]["summon"],
                if mode == "main" {
                    "SummonSkeletalSnipersPlayer"
                } else {
                    "SummonSkeletalArsonistsPlayer"
                }
            );
            assert_eq!(
                view["identity"]["main_group"],
                if mode == "main" { 3 } else { 1 }
            );
            assert!(!view["identity"]["selected"].as_str().unwrap().is_empty());
            same(
                &view["identity"],
                &baseline[mode]["identity"],
                &format!("{name}/{mode}/identity"),
            );
            for actor in ["player", "selected_minion"] {
                assert!(
                    view[actor]["scalars"].as_object().unwrap().len() > 100,
                    "{name}: incomplete {actor} scalar observation"
                );
                assert!(view[actor]["availability"]["Life"] == "number");
                assert!(
                    view[actor]["scalars"]["Life"]
                        .as_f64()
                        .is_some_and(|value| value > 0.)
                );
                if !boss {
                    same(
                        &view[actor],
                        &baseline[mode][actor],
                        &format!("{name}/{mode}/{actor} complete scalar outputs/availability"),
                    );
                }
            }
            if !boss {
                assert_eq!(view["boss"], "None");
                for field in ["input_damage", "placeholder_damage", "damage_type"] {
                    same(
                        &view[field],
                        &baseline[mode][field],
                        &format!("{name}/{mode}/{field}"),
                    );
                }
            }
        }
    }
    for mode in ["main", "calcs"] {
        let low = &named(cases, "boss-shaper-ball-0")["state"][mode];
        let high = &named(cases, "boss-shaper-ball-100")["state"][mode];
        assert_eq!(low["boss"], "Shaper Ball");
        assert_eq!(high["boss"], "Shaper Ball");
        assert_eq!(low["damage_type"], "SpellProjectile");
        assert_eq!(high["damage_type"], "SpellProjectile");
        assert!(
            high["placeholder_damage"]["Cold"].as_f64().unwrap()
                > low["placeholder_damage"]["Cold"].as_f64().unwrap()
        );
        assert!(
            high["player"]["scalars"]["totalEnemyDamageIn"]
                .as_f64()
                .unwrap()
                > low["player"]["scalars"]["totalEnemyDamageIn"]
                    .as_f64()
                    .unwrap()
        );
    }
    for (name, expected) in [("boss-shaper-ball-0", 0.), ("boss-shaper-ball-100", 100.)] {
        for callback in rows(&named(cases, name)["state"]["callbacks"])
            .iter()
            .filter(|callback| callback["value"] == "Shaper Ball")
        {
            assert_eq!(callback["roll"].as_f64(), Some(expected));
        }
    }
}

// Compare full maps in memory above. Retain one complete baseline plus every
// case's hashes/counts and bounded sentinels instead of repeating thousands of stats.
fn compact(cases: &[Json]) -> Vec<Json> {
    cases
        .iter()
        .map(|case| {
            let mut row = case.clone();
            if row["available"] == true && row["name"] != "original-05" {
                for mode in ["main", "calcs"] {
                    for actor in ["player", "selected_minion"] {
                        row["state"][mode][actor] = output_commitment(&case["state"][mode][actor]);
                    }
                }
            }
            row
        })
        .collect()
}
fn output_commitment(output: &Json) -> Json {
    let scalar = &output["scalars"];
    let sentinels: serde_json::Map<_, _> = [
        "Life",
        "Mana",
        "EnergyShield",
        "CombinedDPS",
        "TotalDPS",
        "totalEnemyDamageIn",
        "totalEnemyDamage",
        "EnemyCritChance",
        "EnemyCritEffect",
    ]
    .into_iter()
    .filter_map(|key| scalar.get(key).map(|value| (key.into(), value.clone())))
    .collect();
    json!({"scalar_count":scalar.as_object().unwrap().len(),"scalar_sha256":digest(&serde_json::to_vec(scalar).unwrap()),
        "availability_count":output["availability"].as_object().unwrap().len(),"availability_sha256":digest(&serde_json::to_vec(&output["availability"]).unwrap()),"sentinels":sentinels})
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
        assert_eq!(result.matches("</ConfigSet>").count(), 1);
        result = result.replacen(
            "</ConfigSet>",
            &format!("<{element} name=\"{key}\" {attribute}=\"{value}\"/></ConfigSet>"),
            1,
        );
    }
    result
}
fn raw_config(xml: &str) -> Json {
    let document = roxmltree::Document::parse(xml).unwrap();
    json!(
        document
            .descendants()
            .filter(
                |node| (node.has_tag_name("Input") || node.has_tag_name("Placeholder"))
                    && matches!(
                        node.attribute("name"),
                        Some("presetBossSkills" | "enemyDamageRollRange")
                    )
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
        panic!("{label}: {difference}; complete baseline and output commitments retained");
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
