//! Observe actual minion physical calcDamage calls, without replacing source methods.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/command_damage_source.rs"]
mod command_damage;
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[path = "support/minion_life_adjustments.rs"]
mod life_adjustments;
#[path = "support/minion_life_transformations.rs"]
mod life_transformations;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "fresh_minion_physical_damage_observes_original_calls";
const CHILD: &str = "POE_MINION_PHYSICAL_DAMAGE_SOURCE_CHILD";
const COMMAND_TEST: &str = "command_cooldown_receiving_observes_original_contexts";
const COMMAND_CHILD: &str = "POE_MINION_COMMAND_RECEIVING_SOURCE_CHILD";
const OFFERING_TEST: &str = "offering_source_scopes_and_original_more_rounding";
const OFFERING_CHILD: &str = "POE_OFFERING_SCALING_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_minion_physical_damage_source.lua");
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
    "src/Classes/PassiveSpec.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/ModTools.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModStore.lua",
    "src/Data/Gems.lua",
    "src/Data/Minions.lua",
    "src/Data/Misc.lua",
    "src/Data/SkillStatMap.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/minion.lua",
    "src/TreeData/0_1/tree.lua",
    "src/TreeData/0_4/tree.lua",
    "src/TreeData/0_5/tree.lua",
    "runtime/lua/xml.lua",
];

struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    original: bool,
}

#[test]
fn fresh_minion_physical_damage_observes_original_calls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    // The optional receiving projection changes the observer's authenticated
    // bytes even when disabled. Never overwrite the pinned source-02 receipts;
    // legacy state compatibility and full-report byte identity are distinct.
    let out = root.join("runs/owned-minion-physical-damage-source-02-compatibility");
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
            // Complete unchanged builds and modifier-preservation snapshots need
            // bounded headroom on slower CI hosts; kill and reap on expiry.
            if start.elapsed() > Duration::from_secs(1200) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off = fs::read(out.join("source-jit-off.json")).unwrap();
    let on = fs::read(out.join("source-jit-on.json")).unwrap();
    assert!(
        off == on,
        "JIT evidence differs: off {} bytes/{}; on {} bytes/{}; inspect saved JSON",
        off.len(),
        digest(&off),
        on.len(),
        digest(&on)
    );
}

#[test]
#[ignore = "requires complete pinned PoB runtime; original Command receiving evidence"]
fn command_cooldown_receiving_observes_original_contexts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    // source-03 is retained as the failed, complete seven-contributor census.
    let out = root.join("runs/owned-minion-physical-damage-source-04");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(COMMAND_CHILD) {
        assert!(mode == "off" || mode == "on");
        run_command_child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", COMMAND_TEST, "--ignored", "--nocapture"])
            .env(COMMAND_CHILD, mode)
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
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Command receiving JIT evidence",
    );
}

fn command_control(xml: &str) -> String {
    let gas = gem_attribute(
        &gem_attribute(xml, SNIPER, "skillMinionSkill", "2"),
        SNIPER,
        "skillMinionSkillCalcs",
        "2",
    );
    calcs_input(
        &calcs_input(&gas, "skill_number", "number", "3"),
        "misc_buffMode",
        "string",
        "EFFECTIVE",
    )
}

#[test]
fn command_control_changes_only_existing_sniper_and_calcs_selections() {
    let original = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let changed = command_control(original);
    let before = roxmltree::Document::parse(original).unwrap();
    let after = roxmltree::Document::parse(&changed).unwrap();
    let original_gem = selected_gem(&before, SNIPER);
    let edited_gem = selected_gem(&after, SNIPER);
    assert_eq!(original_gem.children().count(), 0);
    assert_eq!(edited_gem.children().count(), 0);
    for attribute in original_gem.attributes() {
        let expected = if matches!(
            attribute.name(),
            "skillMinionSkill" | "skillMinionSkillCalcs"
        ) {
            "2"
        } else {
            attribute.value()
        };
        assert_eq!(edited_gem.attribute(attribute.name()), Some(expected));
    }
    assert_eq!(
        original_gem.attributes().len(),
        edited_gem.attributes().len()
    );
    for tag in ["Tree", "Items", "Config", "Build"] {
        let prior = before
            .descendants()
            .find(|node| node.has_tag_name(tag))
            .unwrap();
        let next = after
            .descendants()
            .find(|node| node.has_tag_name(tag))
            .unwrap();
        assert_eq!(
            &original[prior.range()],
            &changed[next.range()],
            "changed unrelated {tag}"
        );
    }
    assert_eq!(
        before
            .descendants()
            .filter(|node| node.has_tag_name("Gem"))
            .count(),
        after
            .descendants()
            .filter(|node| node.has_tag_name("Gem"))
            .count()
    );
}

fn run_command_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixture = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    let original_hash = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
    assert_eq!(digest(original.as_bytes()), original_hash);
    let index = read(&fixture.parent().unwrap().join("index.json"));
    assert_eq!(index["builds"][4]["xml_sha256"], original_hash);
    let gas = command_control(&original);
    let cases = [
        Case {
            name: "original-05".into(),
            xml: original.clone(),
            warm: None,
            original: true,
        },
        Case {
            name: "sniper-command-gas-both-contexts".into(),
            xml: gas.clone(),
            warm: None,
            original: false,
        },
        Case {
            name: "warm-command-to-original".into(),
            xml: original.clone(),
            warm: Some(gas),
            original: true,
        },
    ];
    let mut observations = Vec::new();
    for case in cases {
        eprintln!("Command receiving source case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("physicalDamageJit", enabled)?;
            lua.globals().set("physicalDamageCommandEvidence", true)?;
            lua.load("if physicalDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("physicalDamagePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@minion-command-receiving-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("physicalDamagePhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@minion-command-receiving-observation")
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
        )
        .unwrap_or_else(|error| panic!("{}: {error}", case.name));
        assert_eq!(observed["configuration_method_wrappers"], false);
        assert_eq!(observed["original_build_output_available"], true);
        observations.push(json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),
            "warm_xml_sha256":case.warm.as_ref().map(|value|digest(value.as_bytes())),"state":observed["additional_observation"]}));
    }
    let mut report = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "source_hash":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVE.as_bytes()),
        "files":FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_xml_sha256":original_hash,"case_count":3,"complete_load_attempts_per_jit":4,
        "business_method_wrappers":false,"native_coverage":false,"whole_build_parity":false,
        "receiving_query_authority":"Original source queries on actual received stores; distinct from captured original calcSkillCooldown calls",
        "presentation_canonicalization":"Only the existing complete buff-modifier multisets and uniquely keyed buff inventories are sorted; raw evidence is retained separately",
        "cases":observations});
    let mode = if enabled { "on" } else { "off" };
    let raw = serde_json::to_vec(&report).unwrap();
    assert!(
        raw.len() <= 16 * 1024 * 1024,
        "receiving report is {} bytes",
        raw.len()
    );
    fs::write(out.join(format!("source-jit-{mode}-raw.json")), raw).unwrap();
    for case in report["cases"].as_array_mut().unwrap() {
        case["state"] = stable_state(&case["state"]);
    }
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    check_command_receiving(&report);
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
}

fn check_command_receiving(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0]["xml_sha256"], cases[2]["xml_sha256"]);
    assert_ne!(cases[0]["xml_sha256"], cases[1]["xml_sha256"]);
    assert_eq!(
        json_evidence::first_difference(&cases[0]["state"], &cases[2]["state"], "warm-restoration"),
        None
    );
    let reviewed_sources: BTreeMap<_, _> = ["Tree:14598", "Tree:4345", "Tree:43979", "Tree:50837"]
        .into_iter()
        .map(|source| (source, 8.0))
        .collect();
    let mut expected_sources = reviewed_sources.clone();
    expected_sources.extend([
        ("Tree:6077", 20.0),
        ("Tree:14945", 20.0),
        ("Tree:35645", 20.0),
    ]);
    for (index, case) in cases.iter().enumerate() {
        let state = &case["state"];
        for flag in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "saved_specs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
            "command_receiving_methods_preserved",
        ] {
            assert_eq!(state[flag], true, "{} {flag}", case["name"]);
        }
        let family = rows(&state["command_cooldown_family"]);
        assert_eq!(family.len(), 7);
        for (id, amount, conditional) in [
            (14598, 8.0, true),
            (4345, 8.0, true),
            (43979, 8.0, true),
            (50837, 8.0, true),
            (6077, 20.0, true),
            (35645, 20.0, true),
            (14945, 20.0, false),
        ] {
            let node = family.iter().find(|row| row["id"] == id).unwrap();
            assert_eq!(node["allocated"], true);
            assert_eq!(node["effective_name"], node["name"]);
            if amount == 8.0 {
                assert_eq!(rows(&node["modifiers"]).len(), 2);
                assert_eq!(
                    node["stats"],
                    json!([
                        "Minions deal 6% increased Damage",
                        "Minions have 8% increased Cooldown Recovery Rate for Command Skills"
                    ])
                );
            } else if conditional {
                assert_eq!(node["name"], "Command Skill Cooldown");
                assert_eq!(rows(&node["modifiers"]).len(), 1);
                assert_eq!(
                    node["stats"],
                    json!(["Minions have 20% increased Cooldown Recovery Rate for Command Skills"])
                );
            } else {
                assert_eq!(node["name"], "Growing Swarm");
                assert_eq!(
                    node["stats"],
                    json!([
                        "Minions have 20% increased Area of Effect",
                        "Minions have 20% increased Cooldown Recovery Rate"
                    ])
                );
                assert_eq!(rows(&node["modifiers"]).len(), 2);
                let area: Vec<_> = rows(&node["modifiers"])
                    .iter()
                    .filter(|row| row["value"]["mod"]["name"] == "AreaOfEffect")
                    .collect();
                assert_eq!(area.len(), 1);
                assert_eq!(area[0]["value"]["mod"]["type"], "INC");
                assert_eq!(number(&area[0]["value"]["mod"]["value"]), 20.0);
            }
            let nested: Vec<_> = rows(&node["modifiers"])
                .iter()
                .filter(|row| {
                    row["name"] == "MinionModifier"
                        && row["value"]["mod"]["name"] == "CooldownRecovery"
                })
                .collect();
            assert_eq!(nested.len(), 1);
            let outer = nested[0];
            assert_eq!(outer["type"], "LIST");
            assert_eq!(outer["flags"], 0);
            assert_eq!(outer["keyword_flags"], 0);
            assert!(rows(&outer["tags"]).is_empty());
            let inner = &outer["value"]["mod"];
            assert_eq!(inner["type"], "INC");
            assert_eq!(number(&inner["value"]), amount);
            assert_eq!(inner["source"], format!("Tree:{id}"));
            assert_eq!(inner["flags"], 0);
            assert_eq!(inner["keywordFlags"], 0);
            if conditional {
                assert_eq!(
                    inner["_positions"],
                    json!([{"index":1,"value":{"type":"Condition","var":"CommandableSkill"}}])
                );
            } else {
                assert!(inner["_positions"].is_null());
            }
        }
        for mode in if index == 1 {
            &["main", "calcs"][..]
        } else {
            &["main"][..]
        } {
            let actor = rows(&state[mode]["actors"])
                .iter()
                .find(|actor| actor["summon_effect_id"] == SNIPER)
                .unwrap();
            assert_eq!(actor["actor_profile"], "RaisedSkeletonSniper");
            let selected: Vec<_> = rows(&actor["children"])
                .iter()
                .filter(|child| child["selected"] == true)
                .collect();
            assert_eq!(selected.len(), 1);
            let positive = index == 1;
            assert_eq!(
                selected[0]["effect_id"],
                if positive {
                    "GasShotSkeletonSniperMinion"
                } else {
                    "MinionMeleeBow"
                }
            );
            assert_eq!(selected[0]["summoner_owns_actor"], true);
            let receiving = &selected[0]["command_receiving"];
            assert_eq!(receiving["commandable"], positive);
            for flag in [
                "actor_is_actual_minion",
                "actor_parent_is_player",
                "source_query_state_preserved",
                "selected",
            ] {
                assert_eq!(receiving[flag], true);
            }
            let candidates: BTreeMap<_, _> = rows(&receiving["raw_cooldown_modifiers"])
                .iter()
                .filter(|row| row["mod"]["type"] == "INC")
                .map(|row| {
                    (
                        row["mod"]["source"].as_str().unwrap(),
                        number(&row["mod"]["value"]),
                    )
                })
                .collect();
            assert_eq!(candidates, expected_sources);
            let raw = rows(&receiving["raw_cooldown_modifiers"]);
            assert_eq!(raw.len(), 7);
            for row in raw {
                if row["mod"]["source"] == "Tree:14945" {
                    assert!(rows(&row["mod"]["tags"]).is_empty());
                } else {
                    assert_eq!(
                        row["mod"]["tags"],
                        json!([{"type":"Condition","var":"CommandableSkill"}])
                    );
                }
            }
            let joins = rows(&receiving["producer_joins"]);
            assert_eq!(joins.len(), 7);
            for join in joins {
                let matches = rows(&join["player_minion_modifiers"]);
                assert_eq!(matches.len(), 1);
                assert_eq!(matches[0]["exact_inner_object"], true);
            }
            let received = rows(&receiving["received"]["records"]);
            assert_eq!(received.len(), if positive { 7 } else { 1 });
            assert_eq!(
                number(&receiving["received"]["value"]),
                if positive { 92.0 } else { 20.0 }
            );
            let accepted: BTreeMap<_, _> = received
                .iter()
                .map(|row| {
                    (
                        row["mod"]["source"].as_str().unwrap(),
                        number(&row["value"]),
                    )
                })
                .collect();
            let reviewed_received: BTreeMap<_, _> = accepted
                .iter()
                .filter(|(source, _)| reviewed_sources.contains_key(**source))
                .map(|(source, value)| (*source, *value))
                .collect();
            assert_eq!(
                reviewed_received.values().sum::<f64>(),
                if positive { 32.0 } else { 0.0 }
            );
            if positive {
                assert_eq!(accepted, expected_sources);
                assert_eq!(reviewed_received, reviewed_sources);
                assert!(!rows(&receiving["condition_records"]).is_empty());
                let calls = rows(&receiving["original_cooldown_calls"]);
                assert!(!calls.is_empty());
                for call in calls {
                    assert_eq!(call["exact_skill_store"], true);
                    assert_eq!(call["exact_cfg"], true);
                    assert!(number(&call["cooldown"]) > 0.0);
                }
            } else {
                assert_eq!(accepted, BTreeMap::from([("Tree:14945", 20.0)]));
                assert!(reviewed_received.is_empty());
                assert!(rows(&receiving["condition_records"]).is_empty());
                assert!(rows(&receiving["original_cooldown_calls"]).is_empty());
            }
        }
    }
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
            let name = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&name)).unwrap();
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
        })
        .collect();
    let sniper = &originals[4].1;
    let calcs = calcs_input(
        &calcs_input(sniper, "skill_number", "number", "3"),
        "misc_buffMode",
        "string",
        "EFFECTIVE",
    );
    cases.push(Case {
        name: "sniper-calcs-effective".into(),
        xml: calcs.clone(),
        warm: None,
        original: false,
    });
    cases.push(Case {
        name: "repeat-original-05".into(),
        xml: sniper.clone(),
        warm: None,
        original: true,
    });
    cases.push(Case {
        name: "warm-calcs-to-original".into(),
        xml: sniper.clone(),
        warm: Some(calcs),
        original: true,
    });
    for quality in [1, 20] {
        push(
            &mut cases,
            &format!("sniper-quality-{quality}"),
            gem_attribute(sniper, SNIPER, "quality", &quality.to_string()),
        );
    }
    for (name, node) in [
        ("without-plain-node", "95"),
        ("without-command-node", "25927"),
        ("without-gigantic", "46365"),
    ] {
        push(&mut cases, name, remove_node(sniper, node));
    }
    push(
        &mut cases,
        "quality-20-without-gigantic",
        gem_attribute(&remove_node(sniper, "46365"), SNIPER, "quality", "20"),
    );
    for mode in ["COMBAT", "BUFFED", "UNBUFFED"] {
        push(
            &mut cases,
            &format!("sniper-calcs-{}", mode.to_lowercase()),
            calcs_input(
                &calcs_input(sniper, "skill_number", "number", "3"),
                "misc_buffMode",
                "string",
                mode,
            ),
        );
    }
    push(
        &mut cases,
        "offering-disabled",
        gem_attribute(sniper, "PainOfferingPlayer", "enabled", "false"),
    );
    push(
        &mut cases,
        "offering-level-1",
        gem_attribute(sniper, "PainOfferingPlayer", "level", "1"),
    );
    push(
        &mut cases,
        "offering-duplicate-equal",
        duplicate_offering(sniper, "20", false),
    );
    push(
        &mut cases,
        "offering-higher-first",
        duplicate_offering(sniper, "30", true),
    );
    push(
        &mut cases,
        "offering-higher-last",
        duplicate_offering(sniper, "30", false),
    );
    for (name, text) in [
        (
            "flat-physical",
            "Minions deal 3 to 7 additional Physical Damage",
        ),
        (
            "conversion-fire-25",
            "Minions convert 25% of Physical Damage to Fire Damage",
        ),
        (
            "conversion-over-100",
            "Minions convert 150% of Physical Damage to Fire Damage\nMinions convert 150% of Physical Damage to Cold Damage",
        ),
        (
            "physical-gain-fire",
            "Minions gain 20% of Physical Damage as Extra Fire Damage",
        ),
        ("offering-source-buff-effect", "50% increased Buff Effect"),
        (
            "offering-recipient-buff-effect",
            "Minions have 25% increased Effect of Buffs on you",
        ),
        (
            "offering-both-buff-effects",
            "50% increased Buff Effect\nMinions have 25% increased Effect of Buffs on you",
        ),
        ("no-physical-damage", "Minions deal no non-Fire Damage"),
        (
            "offering-positive-half-tie",
            "25% less Buff Effect\nMinions have 57% less Effect of Buffs on you",
        ),
        (
            "offering-negative-half-tie",
            "175% reduced Buff Effect\nMinions have 57% less Effect of Buffs on you",
        ),
        (
            "offering-combined-more",
            "25% more Buff Effect\nMinions have 50% more Effect of Buffs on you",
        ),
        (
            "offering-magnitude",
            "25% increased Magnitudes\n20% more Magnitudes",
        ),
    ] {
        push(&mut cases, name, custom(sniper, text));
    }
    push(
        &mut cases,
        "flat-physical-quality-20",
        custom(
            &gem_attribute(sniper, SNIPER, "quality", "20"),
            "Minions deal 3 to 7 additional Physical Damage",
        ),
    );
    for (name, clone_in_main) in [
        ("two-recipients-original-main-clone-calcs", false),
        ("two-recipients-clone-main-original-calcs", true),
    ] {
        push(
            &mut cases,
            name,
            duplicate_recipients(sniper, clone_in_main),
        );
    }
    assert_eq!(cases.len(), 37);
    fs::create_dir_all(out.join("inputs")).unwrap();
    let mut observed_cases = Vec::new();
    for case in &cases {
        eprintln!("complete minion physical damage case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            &case.xml,
        )
        .unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("physicalDamageJit", enabled)?;
            lua.load("if physicalDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("physicalDamagePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@minion-physical-damage-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("physicalDamagePhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@minion-physical-damage-observation")
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
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|v|digest(v.as_bytes())),"available":true,"state":value["additional_observation"]})
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
    for (name, xml) in &originals {
        assert_eq!(fs::read_to_string(fixtures.join(name)).unwrap(), *xml);
    }
    let mut evidence = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),
        "evidence":{"case_count":cases.len(),"complete_load_attempts_per_jit":cases.len()+1,"observer_sha256":digest(OBSERVE.as_bytes()),
            "presentation_canonicalization":"Skill buff.modifiers and offering merge source_modifiers/merged_modifiers are sorted as complete multisets. Completed per-skill buff inventories are sorted only when every row has a unique explicit (type,name); ambiguous inventories retain their order. Untouched source observation is saved separately as source-jit-{mode}-raw.json.",
            "business_method_wrappers":false,"actor_level_mutation":false,"native_coverage":false,"whole_build_parity":false,
            "files":FILES.iter().map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})).collect::<Vec<_>>(),
            "originals":originals.iter().map(|(n,x)|json!({"name":n,"sha256":digest(x.as_bytes())})).collect::<Vec<_>>()},"cases":observed_cases});
    fs::write(
        out.join(format!(
            "source-jit-{}-raw.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    for case in evidence["cases"].as_array_mut().unwrap() {
        if case["available"] == true {
            case["state"] = stable_state(&case["state"]);
        }
    }
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
    assert_eq!(cases.len(), 37);
    for case in cases {
        assert_eq!(
            case["available"], true,
            "{} {}",
            case["name"], case["source_error"]
        );
        for key in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "saved_specs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
        ] {
            assert_eq!(case["state"][key], true, "{} {key}", case["name"]);
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        assert_eq!(case["state"]["business_method_wrappers"], false);
        for mode in ["main", "calcs"] {
            for actor in rows(&case["state"][mode]["actors"]) {
                for child in rows(&actor["children"]) {
                    if child["damage_calls"].is_null() {
                        continue;
                    }
                    assert_eq!(
                        rows(&child["base_calls"]).len(),
                        rows(&child["passes"]).len() * 5,
                        "{} {mode} live base calls",
                        case["name"]
                    );
                    for base in rows(&child["base_calls"]) {
                        assert_eq!(base["observed_at"], 4137);
                        assert_eq!(base["query_state_preserved"], true);
                        let kind = base["damage_type"].as_str().unwrap();
                        let added_min = number(&base["minimum"]["value"])
                            + number(&base["enemy_minimum"]["value"]);
                        let added_max = number(&base["maximum"]["value"])
                            + number(&base["enemy_maximum"]["value"]);
                        close(number(&base["added_min"]), added_min);
                        close(number(&base["added_max"]), added_max);
                        let mult = (1.0 + number(&base["added_increased"]["value"]) / 100.0)
                            * number(&base["added_more"]["value"]);
                        close(number(&base["added_multiplier"]), mult);
                        for (end, added) in [("Min", added_min), ("Max", added_max)] {
                            let source = base["source"][format!("{kind}{end}")]
                                .as_f64()
                                .unwrap_or(0.0);
                            let bonus = base["source"][format!("{kind}Bonus{end}")]
                                .as_f64()
                                .unwrap_or(0.0);
                            close(
                                number(&base[format!("base_{}", end.to_lowercase())]),
                                (source + bonus + added * mult) * number(&base["base_multiplier"]),
                            );
                        }
                    }
                    for call in rows(&child["damage_calls"]) {
                        assert_eq!(call["return_observed_at"], 4216);
                        assert_eq!(call["all_mult_observed_at"], 4257);
                        assert_eq!(call["query_state_preserved"], true);
                        let min = number(&call["summed_min"]);
                        let max = number(&call["summed_max"]);
                        if min == 0.0 && max == 0.0 {
                            close(number(&call["returned_min"]), 0.0);
                            close(number(&call["returned_max"]), 0.0);
                        } else {
                            close(
                                number(&call["increased_factor"]),
                                1.0 + rows(&call["increased_records"])
                                    .iter()
                                    .map(|r| number(&r["value"]))
                                    .sum::<f64>()
                                    / 100.0,
                            );
                            check_more_boundary(
                                call,
                                &child["passes"][0]["inputs"]["raw_skill_modifiers"],
                            );
                            let common =
                                number(&call["increased_factor"]) * number(&call["more_factor"]);
                            close(
                                number(&call["returned_min"]),
                                (min * common * number(&call["minimum_more"])
                                    + number(&call["add_min"])
                                    + 0.5)
                                    .floor(),
                            );
                            close(
                                number(&call["returned_max"]),
                                (max * common * number(&call["maximum_more"])
                                    + number(&call["add_max"])
                                    + 0.5)
                                    .floor(),
                            );
                        }
                    }
                }
            }
        }
    }
    let original = named(cases, "original-05");
    let baseline = sniper_child(original, "main");
    let physical: Vec<_> = rows(&baseline["damage_calls"])
        .iter()
        .filter(|call| call["damage_type"] == "Physical" && call["critical"] == false)
        .collect();
    assert_eq!(physical.len(), 1);
    close(number(&physical[0]["returned_min"]), 574.0);
    close(number(&physical[0]["returned_max"]), 1068.0);
    close(number(&physical[0]["later_all_mult"]), 1.0);
    let expected_sources: BTreeMap<_, _> = [
        ("Tree:19006", 6.0),
        ("Tree:14598", 6.0),
        ("Tree:54453", 6.0),
        ("Tree:95", 10.0),
        ("Tree:39461", 6.0),
        ("Tree:8737", 10.0),
        ("Tree:229", 6.0),
        ("Tree:50837", 6.0),
        ("Tree:4345", 6.0),
        ("Tree:43979", 6.0),
        ("Skill:PainOfferingPlayer", 62.0),
    ]
    .into_iter()
    .collect();
    let actual_sources: BTreeMap<_, _> = rows(&physical[0]["increased_records"])
        .iter()
        .map(|r| (r["mod"]["source"].as_str().unwrap(), number(&r["value"])))
        .collect();
    assert_eq!(
        rows(&physical[0]["increased_records"]).len(),
        expected_sources.len()
    );
    assert_eq!(actual_sources, expected_sources);
    let input = &baseline["passes"][0]["inputs"];
    assert_eq!(input["base_coefficient"]["level_present"], false);
    assert_eq!(input["base_coefficient"]["skill_data_present"], false);
    close(
        number(&input["bases"]["Physical"]["added_more"]["value"]),
        1.15,
    );
    let hidden: Vec<_> = rows(&input["raw_skill_modifiers"])
        .iter()
        .filter(|r| r["mod"]["source"] == "Hidden Level Scaling" && r["mod"]["name"] == "Damage")
        .collect();
    assert_eq!(hidden.len(), 1);
    assert_eq!(hidden[0]["ancestor_depth"], 1);
    close(number(&hidden[0]["mod"]["value"]), 0.0);
    let command: Vec<_> = rows(&input["raw_skill_modifiers"])
        .iter()
        .filter(|r| r["mod"]["source"] == "Tree:25927" && r["mod"]["name"] == "Damage")
        .collect();
    assert_eq!(command.len(), 1);
    assert_eq!(command[0]["mod"]["tags"][0]["var"], "CommandableSkill");
    assert_eq!(
        sniper_child(named(cases, "sniper-calcs-effective"), "calcs")["selected"],
        true
    );
    for name in ["repeat-original-05", "warm-calcs-to-original"] {
        let state = stable_state(&named(cases, name)["state"]);
        let prior = stable_state(&original["state"]);
        for key in prior.as_object().unwrap().keys() {
            assert!(state[key] == prior[key], "{name} changed {key}");
        }
    }
    let family = rows(&original["state"]["plain_minion_damage_family"]);
    assert_eq!(family.len(), 41);
    for node in family {
        let damage: Vec<_> = rows(&node["modifiers"])
            .iter()
            .filter(|m| {
                m["name"] == "MinionModifier"
                    && m["value"]["mod"]["name"] == "Damage"
                    && m["value"]["mod"]["type"] == "INC"
            })
            .collect();
        assert_eq!(damage.len(), 1, "source node {}", node["id"]);
        assert_eq!(damage[0]["flags"], 0);
        assert_eq!(damage[0]["keyword_flags"], 0);
        assert!(rows(&damage[0]["tags"]).is_empty());
        let inner = &damage[0]["value"]["mod"];
        assert_eq!(inner["flags"], 0);
        assert_eq!(inner["keywordFlags"], 0);
        assert!(number(&inner["value"]) > 0.0);
        assert!(inner.as_object().unwrap().keys().all(|key| matches!(
            key.as_str(),
            "name" | "type" | "value" | "flags" | "keywordFlags" | "source"
        )));
    }
    check_controls(cases);
    check_offering_definition(cases);
    check_offering_scaling(cases);
    check_distinct_recipients(cases);
}

fn check_more_boundary(call: &Json, raw: &Json) {
    // This observed domain has no high-precision override. Source rounds the
    // product within each stat-name/store bucket before multiplying ancestors.
    let mut groups: BTreeMap<(u64, String), f64> = BTreeMap::new();
    for row in rows(&call["more_records"]) {
        let matched: Vec<_> = rows(raw)
            .iter()
            .filter(|r| r["mod"] == row["mod"])
            .collect();
        assert_eq!(matched.len(), 1, "ambiguous source store lineage");
        let key = (
            matched[0]["ancestor_depth"].as_u64().unwrap(),
            row["mod"]["name"].as_str().unwrap().to_owned(),
        );
        *groups.entry(key).or_insert(1.0) *= 1.0 + number(&row["value"]) / 100.0;
    }
    let factor = groups
        .values()
        .map(|v| (v * 100.0 + 0.5).floor() / 100.0)
        .product();
    close(number(&call["more_factor"]), factor);
}
fn physical_call<'a>(case: &'a Json, mode: &str) -> &'a Json {
    let calls: Vec<_> = rows(&sniper_child(case, mode)["damage_calls"])
        .iter()
        .filter(|r| r["damage_type"] == "Physical" && r["critical"] == false)
        .collect();
    assert_eq!(calls.len(), 1, "{} {mode}", case["name"]);
    calls[0]
}
fn input<'a>(case: &'a Json, mode: &str) -> &'a Json {
    &sniper_child(case, mode)["passes"][0]["inputs"]
}
fn check_controls(cases: &[Json]) {
    for (name, mode, min, max, inc, more, offering) in [
        ("original-05", "main", 574.0, 1068.0, 2.3, 1.2, Some(62.0)),
        (
            "sniper-quality-1",
            "main",
            579.0,
            1077.0,
            2.3,
            1.21,
            Some(62.0),
        ),
        (
            "sniper-quality-20",
            "main",
            689.0,
            1282.0,
            2.3,
            1.44,
            Some(62.0),
        ),
        (
            "without-plain-node",
            "main",
            524.0,
            975.0,
            2.1,
            1.2,
            Some(62.0),
        ),
        (
            "without-command-node",
            "main",
            574.0,
            1068.0,
            2.3,
            1.2,
            Some(62.0),
        ),
        (
            "without-gigantic",
            "main",
            478.0,
            890.0,
            2.3,
            1.0,
            Some(62.0),
        ),
        (
            "quality-20-without-gigantic",
            "main",
            574.0,
            1068.0,
            2.3,
            1.2,
            Some(62.0),
        ),
        (
            "sniper-calcs-effective",
            "calcs",
            574.0,
            1068.0,
            2.3,
            1.2,
            Some(62.0),
        ),
        (
            "sniper-calcs-combat",
            "calcs",
            574.0,
            1068.0,
            2.3,
            1.2,
            Some(62.0),
        ),
        (
            "sniper-calcs-buffed",
            "calcs",
            478.0,
            890.0,
            2.3,
            1.0,
            Some(62.0),
        ),
        (
            "sniper-calcs-unbuffed",
            "calcs",
            349.0,
            650.0,
            1.68,
            1.0,
            None,
        ),
        ("offering-disabled", "main", 419.0, 780.0, 1.68, 1.2, None),
        (
            "offering-level-1",
            "main",
            479.0,
            892.0,
            1.92,
            1.2,
            Some(24.0),
        ),
        (
            "offering-duplicate-equal",
            "main",
            574.0,
            1068.0,
            2.3,
            1.2,
            Some(62.0),
        ),
        (
            "offering-higher-first",
            "main",
            619.0,
            1152.0,
            2.48,
            1.2,
            Some(80.0),
        ),
        (
            "offering-higher-last",
            "main",
            619.0,
            1152.0,
            2.48,
            1.2,
            Some(80.0),
        ),
        ("flat-physical", "main", 584.0, 1090.0, 2.3, 1.2, Some(62.0)),
        (
            "flat-physical-quality-20",
            "main",
            700.0,
            1308.0,
            2.3,
            1.44,
            Some(62.0),
        ),
        (
            "conversion-fire-25",
            "main",
            431.0,
            801.0,
            2.3,
            1.2,
            Some(62.0),
        ),
        (
            "physical-gain-fire",
            "main",
            574.0,
            1068.0,
            2.3,
            1.2,
            Some(62.0),
        ),
        (
            "offering-source-buff-effect",
            "main",
            651.0,
            1212.0,
            2.61,
            1.2,
            Some(93.0),
        ),
        (
            "offering-recipient-buff-effect",
            "main",
            612.0,
            1138.0,
            2.45,
            1.2,
            Some(77.0),
        ),
        (
            "offering-both-buff-effects",
            "main",
            689.0,
            1282.0,
            2.76,
            1.2,
            Some(108.0),
        ),
    ] {
        let case = named(cases, name);
        let call = physical_call(case, mode);
        close(number(&call["returned_min"]), min);
        close(number(&call["returned_max"]), max);
        close(number(&call["increased_factor"]), inc);
        close(number(&call["more_factor"]), more);
        let received: Vec<_> = rows(&call["increased_records"])
            .iter()
            .filter(|r| r["mod"]["source"] == "Skill:PainOfferingPlayer")
            .collect();
        assert_eq!(received.len(), usize::from(offering.is_some()), "{name}");
        if let Some(value) = offering {
            close(number(&received[0]["value"]), value);
        }
        close(number(&call["later_all_mult"]), 1.0);
    }
    let original = named(cases, "original-05");
    for row in rows(&original["state"]["modifier_precision"]["overrides"]) {
        assert_eq!(row["present"], false);
    }
    let q1 = named(cases, "sniper-quality-1");
    let more = rows(&physical_call(q1, "main")["more_records"]);
    assert_eq!(more.len(), 2);
    close(
        more.iter()
            .map(|r| 1.0 + number(&r["value"]) / 100.0)
            .product(),
        1.212,
    );
    let pruned = named(cases, "without-plain-node");
    // Removing the connecting node95 causes source allocation pruning of8737.
    // Assert the actual remaining membership; this is not an isolated -10 probe.
    for id in [95, 8737] {
        let row = rows(&pruned["state"]["plain_minion_damage_family"])
            .iter()
            .find(|r| r["id"] == id)
            .unwrap();
        assert_eq!(row["allocated"], false);
    }
    for (name, mode, levels, damage) in [
        ("original-05", "main", vec![22], vec![62.0]),
        ("offering-disabled", "main", vec![], vec![]),
        ("sniper-calcs-unbuffed", "calcs", vec![], vec![]),
        ("offering-level-1", "main", vec![3], vec![24.0]),
        (
            "offering-duplicate-equal",
            "main",
            vec![22, 22],
            vec![62.0, 62.0],
        ),
        (
            "offering-higher-first",
            "main",
            vec![32, 22],
            vec![80.0, 62.0],
        ),
        (
            "offering-higher-last",
            "main",
            vec![22, 32],
            vec![62.0, 80.0],
        ),
        ("offering-source-buff-effect", "main", vec![22], vec![93.0]),
        (
            "offering-recipient-buff-effect",
            "main",
            vec![22],
            vec![77.0],
        ),
        ("offering-both-buff-effects", "main", vec![22], vec![108.0]),
    ] {
        let events = rows(&named(cases, name)["state"][mode]["offering_merge_events"]);
        assert_eq!(events.len(), levels.len(), "{name}");
        let mut highest = f64::NEG_INFINITY;
        for ((event, level), value) in events.iter().zip(levels).zip(damage) {
            assert_eq!(event["source_level"], level);
            assert_eq!(event["source_effect"], "PainOfferingPlayer");
            assert_eq!(event["recipient_profile"], "RaisedSkeletonSniper");
            assert_eq!(event["minion_destination"], true);
            let source = rows(&event["source_modifiers"])
                .iter()
                .find(|r| r["name"] == "Damage")
                .unwrap();
            close(number(&source["value"]), value);
            highest = highest.max(value);
            let merged = rows(&event["merged_modifiers"])
                .iter()
                .find(|r| r["name"] == "Damage")
                .unwrap();
            close(number(&merged["value"]), highest);
        }
    }
    for (name, source_inc, recipient_inc) in [
        ("original-05", 0.0, 0.0),
        ("offering-source-buff-effect", 50.0, 0.0),
        ("offering-recipient-buff-effect", 0.0, 25.0),
        ("offering-both-buff-effects", 50.0, 25.0),
    ] {
        let skill = rows(&named(cases, name)["state"]["main"]["skills"])
            .iter()
            .find(|s| s["effect_id"] == "PainOfferingPlayer")
            .unwrap();
        assert_eq!(skill["effective_level"], 22);
        assert_eq!(skill["physical_level"], 20);
        let buff = &skill["buffs"][0];
        for flag in ["activeSkillBuff", "applyMinions", "applyNotPlayer"] {
            assert_eq!(buff["fields"][flag], true);
        }
        let s = &buff["scaling"];
        assert_eq!(s["source_store_is_skill"], true);
        close(number(&s["source_buff_increased"]["value"]), source_inc);
        close(number(&s["recipient_increased"]["value"]), recipient_inc);
        for key in [
            "source_buff_more",
            "source_magnitude_more",
            "recipient_more",
        ] {
            close(number(&s[key]["value"]), 1.0);
        }
        close(number(&s["source_magnitude_increased"]["value"]), 0.0);
    }
    for name in ["flat-physical", "flat-physical-quality-20"] {
        let call = rows(&sniper_child(named(cases, name), "main")["base_calls"])
            .iter()
            .find(|r| r["damage_type"] == "Physical")
            .unwrap();
        close(number(&call["added_min"]), 3.0);
        close(number(&call["added_max"]), 7.0);
        close(number(&call["added_multiplier"]), 1.15);
        close(number(&call["base_min"]), 211.45);
        close(number(&call["base_max"]), 395.05);
        for key in ["minimum", "maximum"] {
            assert_eq!(
                call[key]["records"][0]["mod"]["source"],
                "Custom:Physical damage source control"
            );
        }
    }
    let converted = input(named(cases, "conversion-fire-25"), "main");
    close(
        number(&converted["conversion"]["Physical"]["Fire"]["global"]["value"]),
        25.0,
    );
    close(
        number(&converted["conversion_table"]["Physical"]["Fire"]),
        0.25,
    );
    close(
        number(&converted["conversion_table"]["Physical"]["mult"]),
        0.75,
    );
    let over = named(cases, "conversion-over-100");
    let conversion = input(over, "main");
    for kind in ["Fire", "Cold"] {
        close(
            number(&conversion["conversion"]["Physical"][kind]["global"]["value"]),
            150.0,
        );
        close(
            number(&conversion["conversion_table"]["Physical"][kind]),
            0.5,
        );
    }
    assert!(physical_call(over, "main")["increased_factor"].is_null());
    close(number(&physical_call(over, "main")["returned_min"]), 0.0);
    let gain = input(named(cases, "physical-gain-fire"), "main");
    close(
        number(&gain["gain"]["Physical"]["Fire"]["global"]["value"]),
        20.0,
    );
    close(number(&gain["gain_table"]["Physical"]["Fire"]), 0.2);
    let disabled = sniper_child(named(cases, "no-physical-damage"), "main");
    assert!(
        !rows(&disabled["damage_calls"])
            .iter()
            .any(|r| r["damage_type"] == "Physical")
    );
    assert_eq!(
        disabled["passes"][0]["inputs"]["bases"]["Physical"]["can_deal"],
        false
    );
}

fn check_offering_definition(cases: &[Json]) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let authored = read(&root.join("data/owned/poe2/3887ae68/pain-offering/extension.json"));
    let tables: Vec<_> = rows(&authored["tables"])
        .iter()
        .filter(|table| table["id"] == "pain-offering.damage-increase")
        .collect();
    assert_eq!(tables.len(), 1);
    let table = tables[0];
    assert_eq!(table["minimum"], 1);
    assert_eq!(table["maximum"], 40);
    assert_eq!(rows(&table["rows"]).len(), 40);
    let unit = &table["value_type"]["value"]["unit"];
    assert_eq!(unit["key"], "def.0000000000000002");
    for case in cases {
        let definition = &case["state"]["offering_definition"];
        assert_eq!(definition["source_tables_preserved"], true);
        assert_eq!(definition["physical_effect_identity"], true);
        assert_eq!(definition["effect_id"], "PainOfferingPlayer");
        assert_eq!(
            definition["physical_gem"]["gameId"],
            "Metadata/Items/Gems/SkillGemPainOffering"
        );
        assert_eq!(definition["physical_gem"]["naturalMaxLevel"], 20);
        assert_eq!(definition["physical_tags"]["minion"], true);
        let sets = rows(&definition["stat_sets"]);
        assert_eq!(sets.len(), 1);
        let set = &sets[0];
        let damage_column = rows(&set["stats"])
            .iter()
            .position(|stat| stat == "pain_offering_damage_+%")
            .unwrap();
        let levels = rows(&set["levels"]);
        assert_eq!(levels.len(), 40);
        for (index, (level, owned)) in levels.iter().zip(rows(&table["rows"])).enumerate() {
            assert_eq!(level["level"], index + 1);
            assert_eq!(owned["kind"], "quantity");
            assert_eq!(&owned["value"]["unit"], unit);
            close(
                number(&level["values"][damage_column]),
                number(&owned["value"]["value"]),
            );
        }
        let mapping = rows(&set["stat_map"])
            .iter()
            .find(|row| row["stat"] == "pain_offering_damage_+%")
            .unwrap();
        assert_eq!(rows(&mapping["modifiers"]).len(), 1);
        let modifier = &mapping["modifiers"][0]["modifier"];
        assert_eq!(modifier["name"], "Damage");
        assert_eq!(modifier["type"], "INC");
        assert_eq!(modifier["flags"], 0);
        assert_eq!(modifier["keyword_flags"], 0);
        assert_eq!(
            modifier["tags"],
            json!([{"type":"GlobalEffect","effectType":"Buff"}])
        );
        for field in ["buffMinions", "buffNotPlayer"] {
            let flags: Vec<_> = rows(&set["base_modifiers"])
                .iter()
                .filter(|m| m["name"] == "SkillData" && m["value"]["key"] == field)
                .collect();
            assert_eq!(flags.len(), 1);
            assert_eq!(flags[0]["value"]["value"], true);
        }
        assert_eq!(
            definition["quality_stats"][0][0],
            "active_skill_base_area_of_effect_radius"
        );
        assert_eq!(
            definition["alternate_quality_stats"][0][0],
            "pain_offering_attack_and_cast_speed_+%"
        );
    }
}

fn check_offering_scaling(cases: &[Json]) {
    for (name, source_inc, source_more, recipient_more, magnitude_inc, magnitude_more) in [
        ("offering-positive-half-tie", 0.0, 0.75, 0.43, 0.0, 1.0),
        ("offering-negative-half-tie", -175.0, 1.0, 0.43, 0.0, 1.0),
        ("offering-combined-more", 0.0, 1.25, 1.5, 0.0, 1.0),
        ("offering-magnitude", 0.0, 1.0, 1.0, 25.0, 1.2),
    ] {
        let case = named(cases, name);
        let skill = rows(&case["state"]["main"]["skills"])
            .iter()
            .find(|s| s["effect_id"] == "PainOfferingPlayer")
            .unwrap();
        assert_eq!(skill["effective_level"], 22);
        let buffs = rows(&skill["buffs"]);
        assert_eq!(buffs.len(), 1);
        let scaling = &buffs[0]["scaling"];
        assert_eq!(scaling["source_store_is_skill"], true);
        assert_eq!(scaling["recipient_hostile"], false);
        let cfg = &scaling["source_cfg"];
        assert_eq!(cfg["present"], true);
        assert_eq!(cfg["skill_gem_is_source"], true);
        assert_eq!(cfg["granted_effect_is_source"], true);
        assert_eq!(cfg["granted_effect"]["id"], "PainOfferingPlayer");
        assert_eq!(cfg["skill_gem_tags"]["minion"], true);
        for (channel, stat, kind, value) in [
            ("source_buff_increased", "BuffEffect", "INC", source_inc),
            ("source_buff_more", "BuffEffect", "MORE", source_more),
            ("recipient_increased", "BuffEffectOnSelf", "INC", 0.0),
            ("recipient_more", "BuffEffectOnSelf", "MORE", recipient_more),
            (
                "source_magnitude_increased",
                "Magnitude",
                "INC",
                magnitude_inc,
            ),
            ("source_magnitude_more", "Magnitude", "MORE", magnitude_more),
        ] {
            close(number(&scaling[channel]["value"]), value);
            let records = rows(&scaling[channel]["records"]);
            let neutral = if kind == "MORE" { 1.0 } else { 0.0 };
            assert_eq!(
                records.len(),
                usize::from(value != neutral),
                "{name} {channel}"
            );
            if value != neutral {
                let record = &records[0];
                let modifier = &record["mod"];
                assert_eq!(modifier["name"], stat);
                assert_eq!(modifier["type"], kind);
                assert_eq!(modifier["source"], "Custom:Physical damage source control");
                assert_eq!(modifier["flags"], 0);
                assert_eq!(modifier["keyword_flags"], 0);
                assert!(rows(&modifier["tags"]).is_empty());
                close(
                    number(&record["value"]),
                    if kind == "MORE" {
                        (value - 1.0) * 100.0
                    } else {
                        value
                    },
                );
            }
        }
        let raw = rows(&buffs[0]["modifiers"])
            .iter()
            .find(|m| m["name"] == "Damage")
            .unwrap();
        close(number(&raw["value"]), 62.0);
        // Preserve CalcPerform's multiplication order and ModStore's two stages:
        // Common.round(value * scale, 2), then math.modf's integral component.
        let magnitude = (1.0 + magnitude_inc / 100.0) * magnitude_more;
        let more = source_more * recipient_more * magnitude;
        let scale = (1.0 + source_inc / 100.0) * more;
        let product = number(&raw["value"]) * scale;
        let scaled = ((product * 100.0 + 0.5).floor() / 100.0).trunc();
        let events = rows(&case["state"]["main"]["offering_merge_events"]);
        assert_eq!(events.len(), 1, "{name}");
        assert_eq!(events[0]["minion_destination"], true);
        close(number(&events[0]["scaling_increased"]), source_inc);
        close(number(&events[0]["scaling_more"]), more);
        for key in ["source_modifiers", "merged_modifiers"] {
            let damage: Vec<_> = rows(&events[0][key])
                .iter()
                .filter(|m| m["name"] == "Damage")
                .collect();
            assert_eq!(damage.len(), 1);
            close(number(&damage[0]["value"]), scaled);
        }
        let call = physical_call(case, "main");
        let received: Vec<_> = rows(&call["increased_records"])
            .iter()
            .filter(|r| r["mod"]["source"] == "Skill:PainOfferingPlayer")
            .collect();
        assert_eq!(received.len(), 1);
        close(number(&received[0]["value"]), scaled);
        close(
            number(&call["increased_factor"]),
            1.0 + (68.0 + scaled) / 100.0,
        );
    }
}

fn check_distinct_recipients(cases: &[Json]) {
    for (name, clone_in_main) in [
        ("two-recipients-original-main-clone-calcs", false),
        ("two-recipients-clone-main-original-calcs", true),
    ] {
        let case = named(cases, name);
        for mode in ["main", "calcs"] {
            let env = &case["state"][mode];
            let actors: Vec<_> = rows(&env["actors"])
                .iter()
                .filter(|actor| actor["summon_effect_id"] == SNIPER)
                .collect();
            assert_eq!(actors.len(), 2, "{name} {mode}");
            let selected: Vec<_> = actors
                .iter()
                .filter(|actor| actor["is_environment_minion"] == true)
                .collect();
            assert_eq!(selected.len(), 1);
            let selected = *selected[0];
            let clone_selected = (mode == "main") == clone_in_main;
            let quality = if clone_selected { 20 } else { 0 };
            assert_eq!(selected["quality"], quality);
            let mut groups = std::collections::BTreeSet::new();
            let mut qualities = std::collections::BTreeSet::new();
            for actor in &actors {
                assert_eq!(actor["physical_level"], 20);
                assert_eq!(actor["effective_level"], 22);
                assert_eq!(actor["source_occurrence"]["source_present"], true);
                let occurrences = rows(&actor["source_occurrence"]["matches"]);
                assert_eq!(occurrences.len(), 1);
                let occurrence = &occurrences[0];
                assert_eq!(occurrence["group_is_active_socket_group"], true);
                assert_eq!(occurrence["source_enabled"], true);
                assert_eq!(occurrence["raw_quality"], actor["quality"]);
                assert_eq!(
                    occurrence["physical_gem_id"],
                    "Metadata/Items/Gems/SkillGemSkeletalSniper"
                );
                assert!(groups.insert(occurrence["group_index"].as_u64().unwrap()));
                assert!(qualities.insert(occurrence["raw_quality"].as_u64().unwrap()));
                if actor["is_environment_minion"] == true {
                    assert_eq!(occurrence["group_index"], env["main_group"]);
                }
            }
            assert_eq!(qualities, [0, 20].into_iter().collect());
            let recipient = rows(&env["selected_minion"]["matches"]);
            assert_eq!(recipient.len(), 1);
            assert_eq!(recipient[0]["ordinal"], selected["ordinal"]);
            assert_eq!(recipient[0]["source"], selected["source_occurrence"]);
            let child = rows(&selected["children"])
                .iter()
                .find(|child| child["effect_id"] == "MinionMeleeBow")
                .unwrap();
            assert_eq!(child["selected"], true);
            assert_eq!(child["summoner_owns_actor"], true);
            assert_eq!(child["summoner_source"], selected["source_occurrence"]);
            let calls: Vec<_> = rows(&child["damage_calls"])
                .iter()
                .filter(|call| call["damage_type"] == "Physical" && call["critical"] == false)
                .collect();
            assert_eq!(calls.len(), 1);
            let call = calls[0];
            let reference = physical_call(
                named(
                    cases,
                    if clone_selected {
                        "sniper-quality-20"
                    } else {
                        "original-05"
                    },
                ),
                "main",
            );
            for field in [
                "summed_min",
                "summed_max",
                "increased_factor",
                "more_factor",
                "returned_min",
                "returned_max",
                "later_all_mult",
            ] {
                close(number(&call[field]), number(&reference[field]));
            }
            let offering: Vec<_> = rows(&call["increased_records"])
                .iter()
                .filter(|r| r["mod"]["source"] == "Skill:PainOfferingPlayer")
                .collect();
            assert_eq!(offering.len(), 1);
            close(number(&offering[0]["value"]), 62.0);
            let quality_mods: Vec<_> = rows(&call["more_records"])
                .iter()
                .filter(|r| r["mod"]["source"] == "Skill:SummonSkeletalSnipersPlayer")
                .collect();
            assert_eq!(quality_mods.len(), usize::from(clone_selected));
            if clone_selected {
                close(number(&quality_mods[0]["value"]), 20.0);
            }
            let events = rows(&env["offering_merge_events"]);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0]["recipient_occurrence"], env["selected_minion"]);
            assert_eq!(events[0]["minion_destination"], true);
            let offering_skill = rows(&env["skills"])
                .iter()
                .find(|skill| skill["effect_id"] == "PainOfferingPlayer")
                .unwrap();
            assert_eq!(
                events[0]["source_occurrence"],
                offering_skill["source_occurrence"]
            );
            assert_eq!(
                offering_skill["buffs"][0]["scaling"]["recipient_occurrence"],
                env["selected_minion"]
            );
        }
    }
}

fn stable_state(state: &Json) -> Json {
    let mut state = state.clone();
    for mode in ["main", "calcs"] {
        for skill in state[mode]["skills"].as_array_mut().unwrap() {
            if let Some(buffs) = skill["buffs"].as_array_mut() {
                for buff in buffs.iter_mut() {
                    sort_modifiers(&mut buff["modifiers"]);
                }
                sort_unique_buff_inventory(buffs);
            }
        }
        if let Some(events) = state[mode]["offering_merge_events"].as_array_mut() {
            for event in events {
                sort_modifiers(&mut event["source_modifiers"]);
                sort_modifiers(&mut event["merged_modifiers"]);
            }
        }
    }
    state
}
fn sort_unique_buff_inventory(buffs: &mut [Json]) {
    // CalcActiveSkill's global-effect extraction (1031+) groups the completed
    // inventory by (type,name), but inserts groups in incoming modifier order.
    // Freezing Mark's independent Buff and Curse groups reverse under JIT.
    // Preserve every full row; do not normalize call histories or ambiguous
    // same-identity groups, whose order could carry a different meaning.
    let mut identities = std::collections::BTreeSet::new();
    for buff in buffs.iter() {
        let (Some(kind), Some(name)) = (
            buff["fields"]["type"].as_str(),
            buff["fields"]["name"].as_str(),
        ) else {
            return;
        };
        if kind.is_empty() || name.is_empty() || !identities.insert((kind, name)) {
            return;
        }
    }
    buffs.sort_by_cached_key(|row| serde_json::to_string(row).unwrap());
}
fn sort_modifiers(value: &mut Json) {
    // Only buff inventories are unordered: warm Lua table traversal can reverse
    // Damage and Speed. Keep raw evidence and compare the entire multiset.
    if let Some(rows) = value.as_array_mut() {
        rows.sort_by_cached_key(|r| serde_json::to_string(r).unwrap());
    }
}

#[test]
fn buff_inventory_canonicalization_preserves_multiplicity_and_other_order() {
    let damage =
        json!({"name":"Damage","value":62,"tags":[{"type":"GlobalEffect","effectType":"Buff"}]});
    let distinct =
        json!({"name":"Damage","value":63,"tags":[{"type":"GlobalEffect","effectType":"Buff"}]});
    let speed = json!({"name":"Speed","value":30,"tags":[]});
    let original = json!({"main":{"skills":[{"effect_id":"first","buffs":[{"modifiers":[speed.clone(),damage.clone(),damage.clone(),distinct.clone()]}]},{"effect_id":"second","buffs":{}}],"offering_merge_events":[{"source_level":32,"source_modifiers":[speed.clone(),distinct.clone()],"merged_modifiers":[speed.clone(),distinct.clone()]},{"source_level":22,"source_modifiers":[speed.clone(),damage.clone()],"merged_modifiers":[speed.clone(),distinct.clone()]}],"damage_calls":[{"returned_min":2},{"returned_min":1}]},"calcs":{"skills":[],"offering_merge_events":{}}});
    let canonical = stable_state(&original);
    let mods = rows(&canonical["main"]["skills"][0]["buffs"][0]["modifiers"]);
    assert_eq!(mods.len(), 4);
    assert_eq!(mods.iter().filter(|m| **m == damage).count(), 2);
    assert_eq!(mods.iter().filter(|m| **m == distinct).count(), 1);
    assert_eq!(canonical["main"]["skills"][0]["effect_id"], "first");
    assert_eq!(canonical["main"]["skills"][1]["effect_id"], "second");
    assert_eq!(
        canonical["main"]["offering_merge_events"][0]["source_level"],
        32
    );
    assert_eq!(
        canonical["main"]["offering_merge_events"][1]["source_level"],
        22
    );
    assert_eq!(
        canonical["main"]["damage_calls"],
        original["main"]["damage_calls"]
    );
    assert_eq!(
        original["main"]["skills"][0]["buffs"][0]["modifiers"][0],
        speed
    );
    assert_eq!(stable_state(&canonical), canonical);
}

#[test]
fn buff_group_canonicalization_requires_unique_explicit_identity() {
    let buff = json!({"fields":{"type":"Buff","name":"same-name"},"modifiers":[{"name":"DamageGainAsCold","value":30}]});
    let curse = json!({"fields":{"type":"Curse","name":"same-name"},"modifiers":[{"name":"FreezeBuildup","value":38}]});
    let mut first = vec![buff.clone(), curse.clone()];
    let mut reversed = vec![curse.clone(), buff.clone()];
    sort_unique_buff_inventory(&mut first);
    sort_unique_buff_inventory(&mut reversed);
    assert_eq!(first, reversed);
    assert_eq!(first.len(), 2);
    assert!(first.contains(&buff) && first.contains(&curse));

    let mut other_buff = buff.clone();
    other_buff["modifiers"][0]["value"] = json!(31);
    for mut ambiguous in [
        vec![other_buff.clone(), buff.clone()],
        vec![curse.clone(), buff.clone(), buff.clone()],
        vec![curse.clone(), json!({"fields":{"name":"missing-type"}})],
        vec![curse, json!({"fields":{"type":"Buff","name":""}})],
    ] {
        let before = ambiguous.clone();
        sort_unique_buff_inventory(&mut ambiguous);
        assert_eq!(ambiguous, before);
    }
}

fn sniper_child<'a>(case: &'a Json, mode: &str) -> &'a Json {
    let actors: Vec<_> = rows(&case["state"][mode]["actors"])
        .iter()
        .filter(|actor| actor["summon_effect_id"] == SNIPER)
        .collect();
    assert_eq!(actors.len(), 1);
    let children: Vec<_> = rows(&actors[0]["children"])
        .iter()
        .filter(|child| child["effect_id"] == "MinionMeleeBow")
        .collect();
    assert_eq!(children.len(), 1);
    children[0]
}
fn number(v: &Json) -> f64 {
    v.as_f64().unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9, "{a} != {b}");
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
fn named<'a>(values: &'a [Json], name: &str) -> &'a Json {
    let matches: Vec<_> = values.iter().filter(|v| v["name"] == name).collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
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
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn selected_skill_set<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> roxmltree::Node<'a, 'i> {
    let skills = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let id = skills.attribute("activeSkillSet").unwrap();
    skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(id))
        .unwrap()
}
fn selected_gem<'a, 'i>(doc: &'a roxmltree::Document<'i>, effect: &str) -> roxmltree::Node<'a, 'i> {
    let gems: Vec<_> = selected_skill_set(doc)
        .descendants()
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(effect))
        .collect();
    assert_eq!(gems.len(), 1);
    gems[0]
}
fn gem_attribute(xml: &str, effect: &str, key: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = selected_gem(&doc, effect);
    assert!(gem.attribute(key).is_some());
    let attrs = gem
        .attributes()
        .map(|a| {
            format!(
                "{}=\"{}\"",
                a.name(),
                escape(if a.name() == key { value } else { a.value() })
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = xml.to_owned();
    out.replace_range(gem.range(), &format!("<Gem {attrs}/>"));
    out
}
fn duplicate_offering(xml: &str, level: &str, before: bool) -> String {
    let changed = gem_attribute(xml, "PainOfferingPlayer", "level", level);
    let doc = roxmltree::Document::parse(&changed).unwrap();
    let group = selected_gem(&doc, "PainOfferingPlayer").parent().unwrap();
    let addition = &changed[group.range()];
    let original = roxmltree::Document::parse(xml).unwrap();
    let group = selected_gem(&original, "PainOfferingPlayer")
        .parent()
        .unwrap();
    let index = if before {
        group.range().start
    } else {
        group.range().end
    };
    let mut out = xml.to_owned();
    out.insert_str(index, addition);
    out
}
fn duplicate_recipients(xml: &str, clone_in_main: bool) -> String {
    let changed = gem_attribute(xml, SNIPER, "quality", "20");
    let changed_doc = roxmltree::Document::parse(&changed).unwrap();
    let cloned = selected_gem(&changed_doc, SNIPER).parent().unwrap();
    let original = roxmltree::Document::parse(xml).unwrap();
    let set = selected_skill_set(&original);
    let group = selected_gem(&original, SNIPER).parent().unwrap();
    let groups: Vec<_> = set.children().filter(|n| n.has_tag_name("Skill")).collect();
    let original_index = groups.iter().position(|n| *n == group).unwrap() + 1;
    let clone_index = groups.len() + 1;
    let build = original
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    assert_eq!(
        build
            .attribute("mainSocketGroup")
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        original_index
    );
    let main_index = if clone_in_main {
        clone_index
    } else {
        original_index
    };
    let calcs_index = if clone_in_main {
        original_index
    } else {
        clone_index
    };
    let mut out = xml.to_owned();
    out.insert_str(
        set.range().end - "</SkillSet>".len(),
        &changed[cloned.range()],
    );
    let start = build.range().start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    let header = xml[start..end].replacen(
        &format!("mainSocketGroup=\"{original_index}\""),
        &format!("mainSocketGroup=\"{main_index}\""),
        1,
    );
    out.replace_range(start..end, &header);
    calcs_input(
        &calcs_input(&out, "skill_number", "number", &calcs_index.to_string()),
        "misc_buffMode",
        "string",
        "EFFECTIVE",
    )
}
fn remove_node(xml: &str, remove: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let tree = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Tree"))
        .unwrap();
    let active: usize = tree.attribute("activeSpec").unwrap().parse().unwrap();
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(active - 1)
        .unwrap();
    let nodes = spec.attribute("nodes").unwrap();
    assert_eq!(nodes.split(',').filter(|n| *n == remove).count(), 1);
    let replaced = nodes
        .split(',')
        .filter(|n| *n != remove)
        .collect::<Vec<_>>()
        .join(",");
    let original = &xml[spec.range()];
    let edited = original.replacen(
        &format!("nodes=\"{nodes}\""),
        &format!("nodes=\"{replaced}\""),
        1,
    );
    assert_ne!(original, edited);
    let mut out = xml.to_owned();
    out.replace_range(spec.range(), &edited);
    out
}
fn custom(xml: &str, text: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let id = config.attribute("activeConfigSet").unwrap();
    let set = config
        .children()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(id))
        .unwrap();
    let end = set.range().end - "</ConfigSet>".len();
    let mut out = xml.to_owned();
    out.insert_str(end,&format!("<CustomModifierBlock title=\"Physical damage source control\" enabled=\"true\">{}</CustomModifierBlock>",escape(text)));
    out
}
fn calcs_input(xml: &str, key: &str, kind: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let calcs = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Calcs"))
        .unwrap();
    let mut edits: Vec<_> = calcs
        .children()
        .filter(|n| n.has_tag_name("Input") && n.attribute("name") == Some(key))
        .map(|n| (n.range(), String::new()))
        .collect();
    let end = calcs.range().end - "</Calcs>".len();
    edits.push((
        end..end,
        format!("<Input name=\"{key}\" {kind}=\"{value}\"/>"),
    ));
    edits.sort_by_key(|(range, _)| range.start);
    let mut out = xml.to_owned();
    for (range, text) in edits.into_iter().rev() {
        out.replace_range(range, &text);
    }
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

const BENEFIT_TEST: &str = "gigantic_benefits_observe_original_life_and_damage_consumers";
const BENEFIT_CHILD: &str = "POE_MINION_GIGANTIC_BENEFITS_SOURCE_CHILD";

#[test]
#[ignore = "requires complete pinned PoB runtime; actual Gigantic Life consumer evidence"]
fn gigantic_benefits_observe_original_life_and_damage_consumers() {
    run_life_source_modes(
        BENEFIT_TEST,
        BENEFIT_CHILD,
        "POE_MINION_GIGANTIC_BENEFITS_SOURCE_OUT",
        "runs/owned-gigantic-benefits-source-01",
        run_benefit_child,
    );
}

#[test]
#[ignore = "requires pinned PoB runtime; focused Offering arithmetic and support-origin evidence"]
fn offering_source_scopes_and_original_more_rounding() {
    run_life_source_modes(
        OFFERING_TEST,
        OFFERING_CHILD,
        "POE_OFFERING_SCALING_SOURCE_OUT",
        "runs/owned-offering-scaling-source-01",
        run_offering_child,
    );
}

fn offering_custom_pair(xml: &str, values: [i32; 2], reverse: bool) -> String {
    let mut out = xml.to_owned();
    for index in if reverse { [1, 0] } else { [0, 1] } {
        out = custom(
            &out,
            &format!(
                "{}% {} Buff Effect",
                values[index].unsigned_abs(),
                if values[index] < 0 { "less" } else { "more" }
            ),
        );
        let title = "Physical damage source control";
        assert_eq!(out.matches(title).count(), 1);
        out = out.replacen(title, &format!("Offering grouping {}", index + 1), 1);
    }
    out
}

fn offering_support_pair(
    xml: &str,
    supported: Option<usize>,
    enabled: bool,
    reverse: bool,
) -> String {
    let recipients = offering_selected_recipient_groups(xml);
    let doc = roxmltree::Document::parse(xml).unwrap();
    let group = selected_gem(&doc, "PainOfferingPlayer").parent().unwrap();
    assert_eq!(group.attribute("label"), Some(""));
    let groups: Vec<_> = (0..2)
        .map(|index| {
            let mut copy = xml[group.range()].replacen(
                "label=\"\"",
                &format!("label=\"Offering {}\"", index + 1),
                1,
            );
            if supported == Some(index) {
                let gem = format!(
                    "<Gem corruptLevel=\"0\" corrupted=\"false\" level=\"1\" count=\"1\" gemId=\"Metadata/Items/Gem/SupportGemDanseMacabre\" enabled=\"{enabled}\" variantId=\"DanseMacabreSupport\" quality=\"0\" enableGlobal2=\"true\" enableGlobal1=\"true\" skillId=\"SupportDanseMacabrePlayer\" nameSpec=\"Danse Macabre\"/>"
                );
                copy.insert_str(copy.len() - "</Skill>".len(), &gem);
            }
            copy
        })
        .collect();
    let mut out = xml.to_owned();
    out.replace_range(
        group.range(),
        &if reverse {
            format!("{}{}", groups[1], groups[0])
        } else {
            groups.concat()
        },
    );
    // Inserting a sibling Offering moves later source occurrences. Resolve the
    // saved selection against the unchanged exact recipient groups in the final
    // input, rather than assuming either recipient's old numeric position.
    let final_doc = roxmltree::Document::parse(&out).unwrap();
    let final_groups: Vec<_> = selected_skill_set(&final_doc)
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .collect();
    let positions = recipients.map(|recipient| {
        let matches: Vec<_> = final_groups
            .iter()
            .enumerate()
            .filter(|(_, group)| &out[group.range()] == recipient)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "recipient group must resolve exactly once"
        );
        matches[0].0 + 1
    });
    let build = final_doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    let old_main = build.attribute("mainSocketGroup").unwrap();
    let range = build.range();
    let updated = out[range.clone()].replacen(
        &format!("mainSocketGroup=\"{old_main}\""),
        &format!("mainSocketGroup=\"{}\"", positions[0]),
        1,
    );
    out.replace_range(range, &updated);
    calcs_input(&out, "skill_number", "number", &positions[1].to_string())
}

fn offering_selected_recipient_groups(xml: &str) -> [&str; 2] {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let groups: Vec<_> = selected_skill_set(&doc)
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .collect();
    let main = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap()
        .attribute("mainSocketGroup")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let calcs = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Calcs"))
        .unwrap()
        .children()
        .filter(|n| n.attribute("name") == Some("skill_number"))
        .collect::<Vec<_>>();
    assert_eq!(calcs.len(), 1);
    let calcs = calcs[0]
        .attribute("number")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    [main, calcs].map(|index| &xml[groups[index - 1].range()])
}

fn offering_cases(original: &str) -> Vec<Case> {
    let recipients = duplicate_recipients(original, false);
    let supported = offering_support_pair(&recipients, Some(0), true, false);
    let mut cases = Vec::new();
    for name in ["original", "original-repeat"] {
        cases.push(Case {
            name: name.into(),
            xml: original.into(),
            warm: None,
            original: true,
        });
    }
    cases.push(Case {
        name: "warm-support-pair-to-original".into(),
        xml: original.into(),
        warm: Some(supported.clone()),
        original: true,
    });
    for (name, values, reverse) in [
        ("more-one-one", [1, 1], false),
        ("more-one-one-reversed", [1, 1], true),
        ("more-zero-zero", [0, 0], false),
        ("more-one-negative", [1, -1], false),
        ("more-one-negative-reversed", [1, -1], true),
    ] {
        push(
            &mut cases,
            name,
            offering_custom_pair(original, values, reverse),
        );
    }
    for (name, slot, enabled, reverse) in [
        ("pair-no-danse", None, true, false),
        ("pair-danse-first", Some(0), true, false),
        ("pair-danse-second", Some(1), true, false),
        ("pair-danse-disabled", Some(0), false, false),
        ("pair-danse-reordered", Some(0), true, true),
    ] {
        push(
            &mut cases,
            name,
            offering_support_pair(&recipients, slot, enabled, reverse),
        );
    }
    assert_eq!(cases.len(), 13);
    for (left, right) in [
        ("more-one-one", "more-one-one-reversed"),
        ("more-one-negative", "more-one-negative-reversed"),
        ("pair-danse-first", "pair-danse-second"),
        ("pair-danse-first", "pair-danse-disabled"),
        ("pair-danse-first", "pair-danse-reordered"),
    ] {
        let input = |name| &cases.iter().find(|case| case.name == name).unwrap().xml;
        assert_ne!(input(left), input(right), "distinct control {left}/{right}");
        assert_ne!(
            digest(input(left).as_bytes()),
            digest(input(right).as_bytes()),
            "distinct control identity {left}/{right}"
        );
    }
    cases
}

#[test]
fn offering_controls_preserve_unrelated_inputs_and_exact_support_assignment() {
    let original = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let before = roxmltree::Document::parse(original).unwrap();
    let recipient_input = duplicate_recipients(original, false);
    let expected_recipients = offering_selected_recipient_groups(&recipient_input);
    assert_ne!(expected_recipients[0], expected_recipients[1]);
    for recipient in expected_recipients {
        let doc = roxmltree::Document::parse(recipient).unwrap();
        let gem = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name("Gem"))
            .unwrap();
        assert_eq!(gem.attribute("skillId"), Some(SNIPER));
    }
    for case in offering_cases(original) {
        let after = roxmltree::Document::parse(&case.xml).unwrap();
        for tag in ["Tree", "Items", "Build"] {
            let a = before
                .root_element()
                .children()
                .find(|n| n.has_tag_name(tag))
                .unwrap();
            let b = after
                .root_element()
                .children()
                .find(|n| n.has_tag_name(tag))
                .unwrap();
            assert_eq!(
                &original[a.range()],
                &case.xml[b.range()],
                "{} {tag}",
                case.name
            );
        }
        let selected = selected_skill_set(&after);
        let offerings: Vec<_> = selected
            .descendants()
            .filter(|n| n.attribute("skillId") == Some("PainOfferingPlayer"))
            .collect();
        assert_eq!(
            offerings.len(),
            if case.name.starts_with("pair-") { 2 } else { 1 }
        );
        if case.name.starts_with("pair-") {
            assert_eq!(
                offering_selected_recipient_groups(&case.xml),
                expected_recipients
            );
        }
        if let Some(warm) = &case.warm {
            assert_eq!(
                offering_selected_recipient_groups(warm),
                expected_recipients
            );
        }
        let supports: Vec<_> = selected
            .descendants()
            .filter(|n| n.attribute("skillId") == Some("SupportDanseMacabrePlayer"))
            .collect();
        let has_support = case.name.starts_with("pair-danse-");
        assert_eq!(supports.len(), usize::from(has_support));
        if has_support {
            assert_eq!(supports[0].attribute("level"), Some("1"));
            assert_eq!(
                supports[0].attribute("enabled"),
                Some(if case.name == "pair-danse-disabled" {
                    "false"
                } else {
                    "true"
                })
            );
            assert_eq!(
                supports[0].parent().unwrap().attribute("label"),
                Some(if case.name == "pair-danse-second" {
                    "Offering 2"
                } else {
                    "Offering 1"
                })
            );
        }
    }
}

fn offering_projection(state: &Json) -> Json {
    let state = stable_state(state);
    let mut result = json!({
        "support_definition":state["offering_support_definition"],
        "saved_support_inventory":state["offering_support_inventory"],
        "config":state["config"],
        "output_snapshot":state["offering_output_snapshot"]
    });
    for name in [
        "original_functions_preserved",
        "loaded_state_preserved",
        "cached_outputs_preserved",
        "saved_specs_preserved",
        "fresh_actor_construction",
        "query_state_preserved",
        "source_actor_level_mutated",
        "business_method_wrappers",
    ] {
        result[name] = state[name].clone();
    }
    for mode in ["main", "calcs"] {
        result[mode] = json!({
            "mode":state[mode]["mode"],"selected_minion":state[mode]["selected_minion"],
            "buffs_enabled":state[mode]["buffs_enabled"],
            "skills":rows(&state[mode]["skills"]).iter().filter(|s|s["effect_id"]=="PainOfferingPlayer").collect::<Vec<_>>(),
            "offering_merge_events":state[mode]["offering_merge_events"],
            "offering_more_calls":state[mode]["offering_more_calls"],
            "player_output":state[mode]["output"],
            "recipients":rows(&state[mode]["actors"]).iter().map(|a|json!({"source_occurrence":a["source_occurrence"],"actor_profile":a["actor_profile"],"is_environment_minion":a["is_environment_minion"],"children":rows(&a["children"]).iter().map(|c|json!({"effect_id":c["effect_id"],"output":c["output"]})).collect::<Vec<_>>()})).collect::<Vec<_>>()
        });
    }
    result
}

fn run_offering_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let path = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&path).unwrap();
    assert_eq!(
        digest(original.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    fs::create_dir_all(out.join("inputs")).unwrap();
    let mut observed_cases = Vec::new();
    for case in offering_cases(&original) {
        eprintln!("focused Offering source case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            &case.xml,
        )
        .unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("physicalDamageJit", enabled)?;
            lua.globals().set("physicalDamageOfferingEvidence", true)?;
            lua.load("if physicalDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("physicalDamagePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@offering-scaling-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("physicalDamagePhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@offering-scaling-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let snapshot = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals()
                .set("physicalDamagePhase", "offering_snapshot")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@offering-uninstrumented-output-snapshot")
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
        let scratch = tempfile::tempdir().unwrap();
        let plain = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            None,
            Some(&snapshot),
        );
        let mut row = match observed {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|v|digest(v.as_bytes())),"available":true,"state":offering_projection(&value["additional_observation"])})
            }
            Err(error) => {
                json!({"name":case.name,"available":false,"source_error":error.to_string()})
            }
        };
        match plain {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                row["unhooked_available"] = json!(true);
                row["unhooked"] = value["additional_observation"].clone();
            }
            Err(error) => {
                row["unhooked_available"] = json!(false);
                row["unhooked_source_error"] = json!(error.to_string());
            }
        }
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
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    let report = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),
        "evidence":{"case_count":13,"complete_load_attempts_per_jit":28,"instrumented_load_attempts_per_jit":14,"uninstrumented_load_attempts_per_jit":14,"observer_sha256":digest(OBSERVE.as_bytes()),
            "test_sha256":digest(include_bytes!("owned_minion_physical_damage_source.rs")),
            "files":FILES.iter().copied().chain(["src/Data/Skills/sup_int.lua"]).map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})).collect::<Vec<_>>(),
            "custom_controls":"Arithmetic probes via ordinary Config custom blocks; no matching legal game producer is inferred.",
            "support_controls":"Actual Danse Macabre gem assignments establish pinned PoB source/delivery only. Its description requires an additional consumed skeleton; the pinned statMap's Offering tag does not prove gameplay activation/availability.",
            "scope":"Original local MORE product before rounding, rounded local result before parent multiplication, and original returned result. Source-store depth is diagnostic PoB provenance only, not an owned grouping model.",
            "instrumentation_check":"Each case is loaded again in an independent uninstrumented VM with the identical XML and warm input. Exact MAIN/CALCS cached, Player and minion child scalar output availability/values and selected source identities must match.",
            "deterministic_projection":{"id":"offering-rejected-modlist-position-v1",
                "raw":"Full pre-projection report retained as source-jit-{mode}-raw.json; source-jit-{mode}-receipt.json commits raw/semantic bytes and every omitted metadata path.",
                "excluded":"Only positive absolute mixed-ModList positions on local_candidates whose type is not MORE. ModList::MoreInternal line170 rejects them before tag evaluation or multiplication. Candidate order, full records, all accepted MORE positions, every original execution step, arithmetic result, identity and output remain exact. ModDB positions remain exact.",
                "source01_observed":"The complete off/on comparison had exactly two differences: pair-danse-reordered MAIN/CALCS offering_more_calls[8].local_candidates[0].position was5 versus4 for the same rejected Danse BuffEffect INC30. Everything else agreed.",
                "source01_inference":"CalcActiveSkill mergeStatSet iterates pairs(stats) before appending different-channel support modifiers. The exact neighboring modifier swapped with Danse INC was not retained and is not claimed as observed."},
            "native_coverage":false,"whole_build_parity":false,"business_method_wrappers":false},"cases":observed_cases});
    let report = write_offering_reports(out, enabled, &report);
    check_offering_scope_report(&report);
}

fn offering_semantic_report(raw: &Json) -> (Json, Vec<String>) {
    let mut semantic = raw.clone();
    let mut omitted = Vec::new();
    for (case_index, case) in semantic["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        if case["available"] != true {
            continue;
        }
        for mode in ["main", "calcs"] {
            let calls = &mut case["state"][mode]["offering_more_calls"];
            let Some(calls) = calls.as_array_mut() else {
                assert!(rows(calls).is_empty());
                continue;
            };
            for (call_index, call) in calls.iter_mut().enumerate() {
                if call["store_kind"] != "ModList" {
                    // ModDB indices refer to its named bucket. This narrow
                    // projection makes no equivalence claim about those indices.
                    continue;
                }
                assert_eq!(call["original_function_line"], 164);
                assert_eq!(call["caller_line"], 2147);
                let name = call["name"].as_str().unwrap().to_owned();
                let steps = rows(&call["original_steps"]);
                assert!(
                    steps
                        .iter()
                        .all(|s| s["mod"]["name"] == name && s["mod"]["type"] == "MORE")
                );
                let candidates = &mut call["local_candidates"];
                let Some(candidates) = candidates.as_array_mut() else {
                    assert!(rows(candidates).is_empty());
                    continue;
                };
                let mut previous = 0;
                for (candidate_index, candidate) in candidates.iter_mut().enumerate() {
                    let position = candidate["position"]
                        .as_u64()
                        .expect("raw position must be an integer");
                    assert!(
                        position > previous,
                        "raw mixed-store positions must be positive and strictly increasing"
                    );
                    previous = position;
                    assert_eq!(candidate["mod"]["name"], name);
                    let kind = candidate["mod"]["type"].as_str().unwrap();
                    assert!(!kind.is_empty());
                    if kind != "MORE" {
                        // Original MoreInternal's first conjunction rejects
                        // this record before reading tags or evaluating values.
                        // Preserve its identity and original relative position
                        // within this candidate array; omit only the index in
                        // the larger mixed-channel store, never sort records.
                        candidate
                            .as_object_mut()
                            .unwrap()
                            .remove("position")
                            .unwrap();
                        omitted.push(format!("cases[{case_index}].state.{mode}.offering_more_calls[{call_index}].local_candidates[{candidate_index}].position"));
                    }
                }
            }
        }
    }
    (semantic, omitted)
}

fn write_offering_reports(out: &Path, enabled: bool, raw: &Json) -> Json {
    let mode = if enabled { "on" } else { "off" };
    let raw_bytes = serde_json::to_vec_pretty(raw).unwrap();
    let raw_name = format!("source-jit-{mode}-raw.json");
    fs::write(out.join(&raw_name), &raw_bytes).unwrap();
    let (semantic, omitted) = offering_semantic_report(raw);
    let semantic_bytes = serde_json::to_vec_pretty(&semantic).unwrap();
    let semantic_name = format!("source-jit-{mode}.json");
    fs::write(out.join(&semantic_name), &semantic_bytes).unwrap();
    let receipt = json!({"schema_version":1,"projection":"offering-rejected-modlist-position-v1",
        "raw":{"path":raw_name,"bytes":raw_bytes.len(),"sha256":digest(&raw_bytes)},
        "semantic":{"path":semantic_name,"bytes":semantic_bytes.len(),"sha256":digest(&semantic_bytes)},
        "omitted_metadata_paths":omitted});
    fs::write(
        out.join(format!("source-jit-{mode}-receipt.json")),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    semantic
}

fn offering_projection_control() -> Json {
    let inc =
        json!({"name":"BuffEffect","type":"INC","value":30,"source":"support","tags":[],"flags":0});
    let first =
        json!({"name":"BuffEffect","type":"MORE","value":1,"source":"first","tags":[],"flags":0});
    let second =
        json!({"name":"BuffEffect","type":"MORE","value":2,"source":"second","tags":[],"flags":0});
    json!({"cases":[{"available":true,"state":{
        "main":{"offering_more_calls":[{"store_kind":"ModList","original_function_line":164,"caller_line":2147,"name":"BuffEffect",
            "source_occurrence":{"group":7},"recipient_occurrence":{"group":3},
            "local_candidates":[{"position":2,"mod":inc},{"position":4,"mod":first.clone()},{"position":6,"mod":second.clone()}],
            "original_steps":[{"mod":first,"product_after":1.01},{"mod":second,"product_after":1.0302}],
            "local_product_before_rounding":1.0302,"local_result_before_parent":1.03,"original_return_result":1.03}]},
        "calcs":{"offering_more_calls":[]},"output_snapshot":{"available":true,"damage":80}}}]})
}

#[test]
fn offering_projection_omits_only_rejected_absolute_positions_and_retains_raw() {
    let raw = offering_projection_control();
    let before = raw.clone();
    let (semantic, omitted) = offering_semantic_report(&raw);
    assert_eq!(raw, before);
    assert_eq!(
        omitted,
        vec!["cases[0].state.main.offering_more_calls[0].local_candidates[0].position"]
    );
    let mut moved = raw.clone();
    moved["cases"][0]["state"]["main"]["offering_more_calls"][0]["local_candidates"][0]["position"] =
        json!(3);
    assert_eq!(offering_semantic_report(&moved).0, semantic);
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(write_offering_reports(dir.path(), false, &raw), semantic);
    assert_eq!(read(&dir.path().join("source-jit-off-raw.json")), raw);
    assert_eq!(read(&dir.path().join("source-jit-off.json")), semantic);
    let receipt = read(&dir.path().join("source-jit-off-receipt.json"));
    assert_eq!(receipt["omitted_metadata_paths"], json!(omitted));
    for (kind, name) in [
        ("raw", "source-jit-off-raw.json"),
        ("semantic", "source-jit-off.json"),
    ] {
        let bytes = fs::read(dir.path().join(name)).unwrap();
        assert_eq!(receipt[kind]["bytes"], json!(bytes.len()));
        assert_eq!(receipt[kind]["sha256"], digest(&bytes));
    }
}

#[test]
fn offering_projection_keeps_records_order_duplicates_execution_and_outputs_exact() {
    let raw = offering_projection_control();
    let expected = offering_semantic_report(&raw).0;
    for mutation in 0..10 {
        let mut changed = raw.clone();
        let call = &mut changed["cases"][0]["state"]["main"]["offering_more_calls"][0];
        match mutation {
            0 => {
                call["local_candidates"][0]["mod"]["value"] = json!(31);
            }
            1 => {
                call["local_candidates"][0]["mod"]["source"] = json!("different support");
            }
            2 => {
                call["local_candidates"][1]["position"] = json!(5);
            }
            3 => {
                let candidates = call["local_candidates"].as_array_mut().unwrap();
                candidates.swap(1, 2);
                candidates[1]["position"] = json!(4);
                candidates[2]["position"] = json!(6);
            }
            4 => {
                let mut duplicate = call["local_candidates"][0].clone();
                duplicate["position"] = json!(3);
                call["local_candidates"]
                    .as_array_mut()
                    .unwrap()
                    .insert(1, duplicate);
            }
            5 => {
                call["original_steps"].as_array_mut().unwrap().swap(0, 1);
            }
            6 => {
                call["original_steps"][0]["product_after"] = json!(1.0101);
            }
            7 => {
                call["original_return_result"] = json!(1.0302);
            }
            8 => {
                call["recipient_occurrence"]["group"] = json!(4);
            }
            9 => {
                changed["cases"][0]["state"]["output_snapshot"]["available"] = json!(false);
            }
            _ => unreachable!(),
        }
        assert_ne!(
            offering_semantic_report(&changed).0,
            expected,
            "meaningful mutation {mutation}"
        );
    }
    let mut bucket = raw.clone();
    let call = &mut bucket["cases"][0]["state"]["main"]["offering_more_calls"][0];
    call["store_kind"] = json!("ModDB");
    call["original_function_line"] = json!(214);
    let (unchanged, omitted) = offering_semantic_report(&bucket);
    assert_eq!(unchanged, bucket);
    assert!(omitted.is_empty());
}

#[test]
fn offering_projection_refuses_invalid_raw_position_or_execution_claims() {
    for mutation in 0..5 {
        let mut raw = offering_projection_control();
        let call = &mut raw["cases"][0]["state"]["main"]["offering_more_calls"][0];
        match mutation {
            0 => call["local_candidates"][0]["position"] = json!(0),
            1 => call["local_candidates"][0]["position"] = json!(2.5),
            2 => call["local_candidates"][1]["position"] = json!(2),
            3 => call["local_candidates"][0]["mod"]["name"] = json!("unrelated"),
            4 => call["original_steps"][0]["mod"]["type"] = json!("INC"),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| offering_semantic_report(&raw)).is_err(),
            "invalid claim {mutation}"
        );
    }
}

fn offering_damage(modifiers: &Json) -> f64 {
    let found: Vec<_> = rows(modifiers)
        .iter()
        .filter(|m| m["name"] == "Damage" && m["type"] == "INC")
        .collect();
    assert_eq!(found.len(), 1);
    number(&found[0]["value"])
}

fn check_offering_scope_report(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 13);
    for case in cases {
        assert_eq!(
            case["available"], true,
            "{} {}",
            case["name"], case["source_error"]
        );
        assert_eq!(
            case["unhooked_available"], true,
            "{} unhooked {}",
            case["name"], case["unhooked_source_error"]
        );
        let state = &case["state"];
        assert_eq!(
            json_evidence::first_difference(
                &state["output_snapshot"],
                &case["unhooked"],
                "uninstrumented"
            ),
            None,
            "{}: instrumentation must preserve output values, availability and selected identities",
            case["name"]
        );
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "saved_specs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
        ] {
            assert_eq!(state[field], true, "{} {field}", case["name"]);
        }
        assert_eq!(state["business_method_wrappers"], false);
        assert_eq!(state["source_actor_level_mutated"], false);
        assert_eq!(
            state["support_definition"]["effect_id"],
            "SupportDanseMacabrePlayer"
        );
        assert!(
            state["support_definition"]["description"]
                .as_str()
                .unwrap()
                .contains("additional skeletal Minion")
        );
        for mode in ["main", "calcs"] {
            let env = &state[mode];
            assert_eq!(env["buffs_enabled"], true);
            let calls = rows(&env["offering_more_calls"]);
            assert!(
                !calls.is_empty(),
                "{} {mode}: original Offering consumer must execute",
                case["name"]
            );
            for call in calls {
                assert_eq!(call["caller_line"], 2147);
                assert_eq!(call["return_observed"], true);
                assert_eq!(call["recipient_occurrence"], env["selected_minion"]);
                assert_eq!(rows(&call["source_occurrence"]["matches"]).len(), 1);
                assert!(call["local_product_before_rounding"].is_number());
                assert!(call["local_result_before_parent"].is_number());
                assert!(call["original_return_result"].is_number());
                if call["domain"] == "source_skill" {
                    assert_eq!(call["context_is_source_skill"], true);
                } else {
                    assert_eq!(call["domain"], "recipient_actor");
                    assert_eq!(call["context_is_recipient_actor"], true);
                }
            }
            for event in rows(&env["offering_merge_events"]) {
                assert_eq!(event["minion_destination"], true);
                assert_eq!(event["recipient_occurrence"], env["selected_minion"]);
                let root: Vec<_> = calls
                    .iter()
                    .filter(|c| {
                        c["name"] == "BuffEffect"
                            && c["domain"] == "source_skill"
                            && c["source_store_depth"] == 0
                            && c["source_occurrence"] == event["source_occurrence"]
                    })
                    .collect();
                assert_eq!(root.len(), 1);
                close(
                    number(&event["scaling_more"]),
                    number(&root[0]["original_return_result"]),
                );
            }
        }
    }
    for name in ["original-repeat", "warm-support-pair-to-original"] {
        assert_eq!(
            named(cases, "original")["state"],
            named(cases, name)["state"],
            "{name}"
        );
    }
    for (name, values, before, rounded, order) in [
        ("more-one-one", [1.0, 1.0], 1.0201, 1.02, [1, 2]),
        ("more-one-one-reversed", [1.0, 1.0], 1.0201, 1.02, [2, 1]),
        ("more-zero-zero", [0.0, 0.0], 1.0, 1.0, [1, 2]),
        ("more-one-negative", [1.0, -1.0], 0.9999, 1.0, [1, 2]),
        (
            "more-one-negative-reversed",
            [-1.0, 1.0],
            0.9999,
            1.0,
            [2, 1],
        ),
    ] {
        for mode in ["main", "calcs"] {
            let env = &named(cases, name)["state"][mode];
            let grouped: Vec<_> = rows(&env["offering_more_calls"])
                .iter()
                .filter(|c| c["name"] == "BuffEffect" && rows(&c["original_steps"]).len() == 2)
                .collect();
            assert_eq!(grouped.len(), 1, "{name} {mode}");
            let call = grouped[0];
            assert_eq!(call["domain"], "source_skill");
            assert_eq!(call["precision_present"], false);
            close(number(&call["local_product_before_rounding"]), before);
            close(number(&call["local_result_before_parent"]), rounded);
            for (index, step) in rows(&call["original_steps"]).iter().enumerate() {
                close(number(&step["mod"]["value"]), values[index]);
                assert_eq!(
                    step["mod"]["source"],
                    format!("Custom:Offering grouping {}", order[index])
                );
            }
            close(
                number(&rows(&env["offering_merge_events"])[0]["scaling_more"]),
                rounded,
            );
        }
    }
    for (name, supported) in [
        ("pair-no-danse", None),
        ("pair-danse-first", Some("Offering 1")),
        ("pair-danse-second", Some("Offering 2")),
        ("pair-danse-disabled", None),
        ("pair-danse-reordered", Some("Offering 1")),
    ] {
        let state = &named(cases, name)["state"];
        assert_ne!(
            state["main"]["selected_minion"],
            state["calcs"]["selected_minion"]
        );
        for mode in ["main", "calcs"] {
            let env = &state[mode];
            assert_eq!(rows(&env["selected_minion"]["matches"]).len(), 1);
            assert_eq!(env["selected_minion"]["matches"][0]["effect_id"], SNIPER);
            assert_eq!(rows(&env["skills"]).len(), 2);
            for skill in rows(&env["skills"]) {
                let occurrence = &skill["source_occurrence"]["matches"][0];
                let label = occurrence["group_label"].as_str().unwrap();
                let active = supported == Some(label);
                let danse: Vec<_> = rows(&skill["offering_supports"])
                    .iter()
                    .filter(|s| s["effect_id"] == "SupportDanseMacabrePlayer")
                    .collect();
                let admitted: Vec<_> = danse
                    .iter()
                    .filter(|s| s["admitted_by_original_effect_list"] == true)
                    .collect();
                assert_eq!(admitted.len(), usize::from(active), "{name} {mode} {label}");
                let scaling = &rows(&skill["buffs"])[0]["scaling"];
                close(
                    number(&scaling["source_buff_increased"]["value"]),
                    if active { 30.0 } else { 0.0 },
                );
                close(number(&scaling["source_buff_more"]["value"]), 1.0);
                close(number(&scaling["recipient_increased"]["value"]), 0.0);
                close(number(&scaling["recipient_more"]["value"]), 1.0);
                let received = rows(&scaling["source_buff_increased"]["records"]);
                assert_eq!(received.len(), usize::from(active));
                if active {
                    let support = admitted[0];
                    assert_eq!(support["supports_exact_source"], true);
                    assert_eq!(
                        support["source_occurrence"]["matches"][0]["group_index"],
                        occurrence["group_index"]
                    );
                    assert_eq!(
                        support["source_occurrence"]["matches"][0]["source_enabled"],
                        true
                    );
                    assert_eq!(received[0]["mod"]["source"], support["modifier_source"]);
                    assert_eq!(received[0]["mod"]["type"], "INC");
                    close(number(&received[0]["value"]), 30.0);
                }
                let events: Vec<_> = rows(&env["offering_merge_events"])
                    .iter()
                    .filter(|e| e["source_occurrence"] == skill["source_occurrence"])
                    .collect();
                assert_eq!(events.len(), 1);
                close(
                    number(&events[0]["scaling_increased"]),
                    if active { 30.0 } else { 0.0 },
                );
                close(
                    offering_damage(&events[0]["source_modifiers"]),
                    if active { 80.0 } else { 62.0 },
                );
            }
            let events = rows(&env["offering_merge_events"]);
            assert_eq!(events.len(), 2);
            close(
                offering_damage(&events.last().unwrap()["merged_modifiers"]),
                if supported.is_some() { 80.0 } else { 62.0 },
            );
        }
    }
}

fn run_life_source_modes(
    test: &str,
    child_env: &str,
    output_env: &str,
    default_output: &str,
    child_run: fn(&Path, &Path, bool),
) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(output_env)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(default_output));
    // The child runs from PoB/src; bind relative overrides before dispatch so
    // both processes write to the same repository-relative evidence directory.
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    if let Some(mode) = std::env::var_os(child_env) {
        assert!(mode == "on" || mode == "off");
        child_run(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "use a fresh evidence directory");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--ignored", "--nocapture"])
            .env(child_env, mode)
            .env(output_env, &out)
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
                    "Resource child failed {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(900) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "Resource source deadline {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Resource source JIT evidence",
    );
}

fn run_benefit_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixture = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    assert_eq!(
        digest(original.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let removed = remove_node(&original, "46365");
    let mut cases = vec![
        Case {
            name: "original-05".into(),
            xml: original.clone(),
            warm: None,
            original: true,
        },
        Case {
            name: "repeat-original-05".into(),
            xml: original.clone(),
            warm: None,
            original: true,
        },
        Case {
            name: "without-gigantic".into(),
            xml: removed.clone(),
            warm: None,
            original: false,
        },
        Case {
            name: "warm-removal-to-original".into(),
            xml: original.clone(),
            warm: Some(removed),
            original: true,
        },
        Case {
            name: "duplicate-gigantic-grant".into(),
            xml: custom(&original, "Your Minions are Gigantic"),
            warm: None,
            original: false,
        },
    ];
    for mode in ["EFFECTIVE", "COMBAT", "BUFFED", "UNBUFFED"] {
        cases.push(Case {
            name: format!("sniper-calcs-{}", mode.to_lowercase()),
            xml: calcs_input(
                &calcs_input(&original, "skill_number", "number", "3"),
                "misc_buffMode",
                "string",
                mode,
            ),
            warm: None,
            original: false,
        });
    }
    let mut observations = vec![];
    for case in &cases {
        eprintln!("Gigantic original Life consumer case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("physicalDamageJit", enabled)?;
            lua.globals().set("physicalDamageBenefitEvidence", true)?;
            lua.load("if physicalDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("physicalDamagePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@gigantic-benefits-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("physicalDamagePhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@gigantic-benefits-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let snapshot = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals()
                .set("physicalDamagePhase", "benefit_snapshot")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@gigantic-benefits-unhooked-snapshot")
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
        )
        .unwrap_or_else(|e| panic!("{}: {e}", case.name));
        let scratch = tempfile::tempdir().unwrap();
        let plain = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            None,
            Some(&snapshot),
        )
        .unwrap_or_else(|e| panic!("{} unhooked: {e}", case.name));
        for result in [&observed, &plain] {
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            assert_eq!(result["source_hash"], pinned::manifest_sha256());
        }
        assert_eq!(
            json_evidence::first_difference(
                &observed["additional_observation"]["benefit_snapshot"],
                &plain["additional_observation"],
                "unhooked"
            ),
            None,
            "{}",
            case.name
        );
        observations.push(
            json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),
            "warm_xml_sha256":case.warm.as_ref().map(|x|digest(x.as_bytes())),
            "state":observed["additional_observation"],"unhooked":plain["additional_observation"],
            "synthetic_second_source":case.name=="duplicate-gigantic-grant"}),
        );
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec(&observations).unwrap(),
        )
        .unwrap();
    }
    let mut files = FILES.to_vec();
    files.push("src/Modules/CalcDefence.lua");
    files.sort_unstable();
    files.dedup();
    let mut report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVE.as_bytes()),
        "files":files.iter().map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})).collect::<Vec<_>>(),
        "case_count":cases.len(),"complete_load_attempts_per_jit":2*(cases.len()+1),
        "original_xml_sha256":digest(original.as_bytes()),"business_method_wrappers":false,
        "scope":{"whole_build_parity":false,"native_coverage":false,"full_life_formula_parity":false,
            "composed_more_factor_parity":false,"obtainable_second_grant_claimed":false,
            "receiver_profiles":["RaisedSkeletonSniper"]},
        "capture":{"consumer":"original calcs.doActorLifeManaSpirit","after_assignment_line":97,
            "original_more_local":true,"original_actor_output_after_return":true,
            "copied_formula_as_evidence":false,"unhooked_controls":true},"cases":observations});
    let mode = if enabled { "on" } else { "off" };
    fs::write(
        out.join(format!("source-jit-{mode}-raw.json")),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    for case in report["cases"].as_array_mut().unwrap() {
        case["state"] = stable_state(&case["state"]);
    }
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= 16 * 1024 * 1024);
    fs::write(out.join(format!("source-jit-{mode}.json")), bytes).unwrap();
    check_benefits(&report);
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
}

fn check_benefits(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 9);
    for repeat in [1, 3] {
        assert_eq!(cases[0]["xml_sha256"], cases[repeat]["xml_sha256"]);
        assert_eq!(
            json_evidence::first_difference(
                &cases[0]["state"],
                &cases[repeat]["state"],
                "fresh-replay"
            ),
            None
        );
    }
    for case in cases {
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "saved_specs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
        ] {
            assert_eq!(case["state"][field], true, "{} {field}", case["name"]);
        }
        assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
        let removed = case["name"] == "without-gigantic";
        let duplicate = case["name"] == "duplicate-gigantic-grant";
        for mode in ["main", "calcs"] {
            let frame = &case["state"][mode];
            let actors: Vec<_> = rows(&frame["actors"])
                .iter()
                .filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .collect();
            assert_eq!(actors.len(), 1);
            let actor = actors[0];
            let benefits = &actor["gigantic_benefits"];
            assert_eq!(benefits["exact_parent"], true);
            assert_eq!(benefits["exact_summoner"], true);
            let calls = rows(&benefits["original_life_calls"]);
            if actor["is_environment_minion"] != true {
                continue;
            }
            assert!(
                !calls.is_empty(),
                "{} {mode} must execute real Life consumer",
                case["name"]
            );
            let applies = !removed && frame["combat"] == true;
            for call in calls {
                for field in [
                    "exact_actor_store",
                    "exact_actor_output",
                    "summoner_owns_actor",
                    "summoner_actor_is_parent",
                ] {
                    assert_eq!(call[field], true);
                }
                assert_eq!(call["source"], actor["source_occurrence"]);
                assert_eq!(call["return_life"], call["post_return_life"]);
                assert!(
                    call["post_return_observed_at"].as_u64().unwrap()
                        > call["caller_line"].as_u64().unwrap()
                );
                let c = &call["computation"];
                assert_eq!(c["observed_at"], 97);
                assert_eq!(c["override_present"], false);
                assert_eq!(c["life_after_assignment"], call["return_life"]);
                assert_eq!(c["life_precision"]["present"], false);
                assert_eq!(c["gigantic"], !removed);
                assert_eq!(
                    rows(&c["gigantic_records"]).len(),
                    if removed {
                        0
                    } else if duplicate {
                        2
                    } else {
                        1
                    }
                );
                let grant_sources: std::collections::BTreeSet<_> = rows(&c["gigantic_records"])
                    .iter()
                    .map(|r| {
                        assert_eq!(r["value"], true);
                        assert_eq!(r["mod"]["name"], "Gigantic");
                        assert_eq!(r["mod"]["type"], "FLAG");
                        r["mod"]["source"].as_str().unwrap()
                    })
                    .collect();
                assert_eq!(
                    grant_sources.len(),
                    if removed {
                        0
                    } else if duplicate {
                        2
                    } else {
                        1
                    }
                );
                assert_eq!(grant_sources.contains("Tree:46365"), !removed);
                let more = rows(&c["eligible_life_more"]);
                assert_eq!(more.len(), usize::from(applies));
                for row in more {
                    assert_eq!(row["value"], 20);
                    assert_eq!(
                        row["mod"],
                        json!({"name":"Life","type":"MORE","value":20,
                        "source":"Gigantic","flags":0,"keyword_flags":0,"tags":{}})
                    );
                }
                assert_eq!(c["more"], if applies { json!(1.2) } else { json!(1) });
                for stat in ["Life", "Damage"] {
                    let generated: Vec<_> = rows(&c["raw_modifiers"])
                        .iter()
                        .filter(|r| r["mod"]["source"] == "Gigantic" && r["mod"]["name"] == stat)
                        .collect();
                    assert_eq!(
                        generated.len(),
                        usize::from(applies),
                        "one generated benefit per actor, not per grant"
                    );
                    for row in generated {
                        assert_eq!(row["ancestor_depth"], 0);
                        assert_eq!(row["mod"]["type"], "MORE");
                        assert_eq!(row["mod"]["value"], 20);
                    }
                }
            }
            assert_eq!(
                calls.last().unwrap()["post_return_life"],
                benefits["actor_output_life"]
            );
            let call = physical_call(case, mode);
            assert_eq!(
                call["more_factor"],
                if applies { json!(1.2) } else { json!(1) }
            );
            let generated: Vec<_> = rows(&call["more_records"])
                .iter()
                .filter(|r| r["mod"]["source"] == "Gigantic")
                .collect();
            assert_eq!(generated.len(), usize::from(applies));
        }
    }
}

const INTRINSIC_LIFE_TEST: &str = "intrinsic_minion_life_observes_original_table_and_base";
const INTRINSIC_LIFE_CHILD: &str = "POE_MINION_INTRINSIC_LIFE_SOURCE_CHILD";

#[test]
#[ignore = "requires complete pinned PoB runtime; original allied Life table and initializer"]
fn intrinsic_minion_life_observes_original_table_and_base() {
    run_life_source_modes(
        INTRINSIC_LIFE_TEST,
        INTRINSIC_LIFE_CHILD,
        "POE_MINION_INTRINSIC_LIFE_SOURCE_OUT",
        "runs/owned-minion-intrinsic-life-source-01",
        run_intrinsic_life_child,
    );
}

fn without_selected_level_items(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let active = items.attribute("activeItemSet").unwrap();
    let set = items
        .children()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some(active))
        .unwrap();
    let mut changes = vec![];
    for (slot, id) in [("Amulet", "23"), ("Helmet", "21")] {
        let row = set
            .children()
            .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some(slot))
            .unwrap();
        assert_eq!(row.attribute("itemId"), Some(id));
        let attrs = row
            .attributes()
            .map(|a| {
                format!(
                    "{}=\"{}\"",
                    a.name(),
                    escape(if a.name() == "itemId" { "0" } else { a.value() })
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        changes.push((row.range(), format!("<Slot {attrs}/>")));
    }
    changes.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut out = xml.to_owned();
    for (range, replacement) in changes {
        out.replace_range(range, &replacement);
    }
    out
}

fn run_intrinsic_life_child(root: &Path, out: &Path, enabled: bool) {
    run_actor_life_child(root, out, enabled, ActorLifeEvidence::Intrinsic);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActorLifeEvidence {
    Intrinsic,
    Delivery,
    Adjustments,
    Transformations,
}

fn run_actor_life_child(root: &Path, out: &Path, enabled: bool, evidence: ActorLifeEvidence) {
    let delivery = evidence != ActorLifeEvidence::Intrinsic;
    let transformations = evidence == ActorLifeEvidence::Transformations;
    let adjustments = evidence == ActorLifeEvidence::Adjustments || transformations;
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixture = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    assert_eq!(
        digest(original.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let selected = calcs_input(&original, "skill_number", "number", "3");
    let ordinary = without_selected_level_items(&selected);
    let low = gem_attribute(&ordinary, SNIPER, "level", "1");
    let mut cases = vec![
        Case {
            name: "original-05".into(),
            xml: original.clone(),
            warm: None,
            original: true,
        },
        Case {
            name: "repeat-original-05".into(),
            xml: original.clone(),
            warm: None,
            original: true,
        },
        Case {
            name: "warm-level-one-to-original".into(),
            xml: original.clone(),
            warm: Some(low),
            original: true,
        },
        Case {
            name: "sniper-calcs-selected".into(),
            xml: selected,
            warm: None,
            original: false,
        },
    ];
    for level in [1, 2, 19, 20, 40] {
        push(
            &mut cases,
            &format!("saved-level-{level}"),
            gem_attribute(&ordinary, SNIPER, "level", &level.to_string()),
        );
    }
    if delivery {
        let empty = ["19006", "229", "39461", "54453", "1218", "40894"]
            .into_iter()
            .fold(original.clone(), |xml, id| remove_node(&xml, id));
        cases = vec![
            Case {
                name: "original-05".into(),
                xml: original.clone(),
                warm: None,
                original: true,
            },
            Case {
                name: "repeat-original-05".into(),
                xml: original.clone(),
                warm: None,
                original: true,
            },
            Case {
                name: "warm-empty-to-original".into(),
                xml: original.clone(),
                warm: Some(empty.clone()),
                original: true,
            },
            Case {
                name: "without-life-229".into(),
                xml: remove_node(&original, "229"),
                warm: None,
                original: false,
            },
            Case {
                name: "without-life-1218".into(),
                xml: remove_node(&original, "1218"),
                warm: None,
                original: false,
            },
            Case {
                name: "without-six-life-nodes".into(),
                xml: empty,
                warm: None,
                original: false,
            },
            Case {
                name: "sniper-calcs-selected".into(),
                xml: calcs_input(&original, "skill_number", "number", "3"),
                warm: None,
                original: false,
            },
        ];
    }
    let mut observations = vec![];
    let mut definitions = None;
    for case in &cases {
        eprintln!("Actor Life source case {}", case.name);
        let setup = |lua: &Lua, jit_enabled: bool| {
            lua.globals().set("physicalDamageJit", jit_enabled)?;
            lua.globals().set("physicalDamageBenefitEvidence", true)?;
            lua.globals()
                .set("physicalDamageLifeDeliveryEvidence", delivery)?;
            lua.globals()
                .set("physicalDamageLifeAdjustmentEvidence", adjustments)?;
            lua.globals()
                .set("physicalDamageLifeTransformationEvidence", transformations)?;
            lua.globals()
                .set("physicalDamageIntrinsicLifeEvidence", true)?;
            lua.load("if physicalDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        // Debug call/line observations are interpreter evidence. Independent
        // unhooked loads below exercise the requested off/on reference mode.
        let before = |lua: &Lua| setup(lua, enabled && !adjustments);
        let before_plain = |lua: &Lua| setup(lua, enabled);
        let install = |lua: &Lua| {
            lua.globals().set("physicalDamagePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@intrinsic-life-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            if adjustments {
                assert!(!lua.load("return jit.status()").eval::<bool>()?);
            }
            lua.globals().set("physicalDamagePhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@intrinsic-life-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let snapshot = |lua: &Lua| -> Result<Json, RuntimeError> {
            if adjustments {
                assert_eq!(lua.load("return jit.status()").eval::<bool>()?, enabled);
            }
            lua.globals()
                .set("physicalDamagePhase", "benefit_snapshot")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@intrinsic-life-unhooked")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let mut observed = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            Some(&install),
            Some(&observe),
        )
        .unwrap_or_else(|e| panic!("{}: {e}", case.name));
        let scratch = tempfile::tempdir().unwrap();
        let plain = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before_plain),
            None,
            Some(&snapshot),
        )
        .unwrap_or_else(|e| panic!("{} unhooked: {e}", case.name));
        for result in [&observed, &plain] {
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            assert_eq!(result["source_hash"], pinned::manifest_sha256());
        }
        assert_eq!(
            json_evidence::first_difference(
                &observed["additional_observation"]["benefit_snapshot"],
                &plain["additional_observation"],
                "unhooked"
            ),
            None,
            "{}",
            case.name
        );
        let body = observed["additional_observation"].as_object_mut().unwrap();
        let current = body.remove("intrinsic_life_definitions").unwrap();
        if let Some(previous) = &definitions {
            assert_eq!(previous, &current);
        } else {
            definitions = Some(current);
        }
        observations.push(
            json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),
            "warm_xml_sha256":case.warm.as_ref().map(|x|digest(x.as_bytes())),
            "state":observed["additional_observation"],"unhooked":plain["additional_observation"],
            "authored_domain_boundary_only":case.name=="saved-level-40"}),
        );
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec(&observations).unwrap(),
        )
        .unwrap();
    }
    let mut files = FILES.to_vec();
    files.push("src/Modules/CalcDefence.lua");
    files.sort_unstable();
    files.dedup();
    let mut report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVE.as_bytes()),
        "files":files.iter().map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})).collect::<Vec<_>>(),
        "case_count":cases.len(),"complete_load_attempts_per_jit":2*(cases.len()+1),
        "original_xml_sha256":digest(original.as_bytes()),"business_method_wrappers":false,
        "scope":{"receiver_profiles":["RaisedSkeletonSniper"],"whole_build_parity":false,"native_coverage":false,
            "final_life_formula_parity":false,"hostile_profile_admitted":false,
            "physical_level_40_obtainable_claimed":false,"actor_level_mutation":false},
        "capture":{"original_table_selection":true,"original_base_initializer":true,
            "original_life_consumer":true,"copied_formula_as_evidence":false,"unhooked_controls":true},
        "definitions":definitions.unwrap(),"cases":observations});
    if delivery {
        report["capture"]["original_minion_modifier_list"] = json!(true);
        report["capture"]["original_modifier_insertion"] = json!(true);
        report["scope"]["passive_life_delivery_only"] = json!(true);
        report["scope"]["removal_may_prune_other_nodes"] = json!(true);
    }
    if adjustments {
        report["scope"]["passive_life_delivery_only"] = json!(false);
        report["capture"]["minion_life_adjustment_census"] = json!(true);
        report["capture"]["raw_zero_valued_sources_retained"] = json!(true);
        report["capture"]["observer_jit_enabled"] = json!(false);
        report["capture"]["unhooked_jit_mode_from_report_filename"] = json!(true);
    }
    if transformations {
        report["capture"]["original_minion_resource_transformation"] = json!(true);
        report["capture"]["incoming_life_channels"] = json!(10);
        report["capture"]["original_generated_record_identity"] = json!(true);
        report["scope"]["positive_transformation_admission"] = json!(false);
        report["driver_sha256"] = json!(digest(include_bytes!(
            "owned_minion_physical_damage_source.rs"
        )));
        report["source_helper_sha256"] = json!(digest(include_bytes!(
            "support/minion_life_transformations.rs"
        )));
        report["bootstrap_sha256"] = json!(digest(include_bytes!(
            "support/configuration_preparation_source.rs"
        )));
    }
    let mode = if enabled { "on" } else { "off" };
    fs::write(
        out.join(format!("source-jit-{mode}-raw.json")),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    for case in report["cases"].as_array_mut().unwrap() {
        case["state"] = stable_state(&case["state"]);
    }
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= 16 * 1024 * 1024);
    fs::write(out.join(format!("source-jit-{mode}.json")), bytes).unwrap();
    if delivery {
        check_life_delivery(&report);
    } else {
        check_intrinsic_life(&report);
    }
    if adjustments {
        life_adjustments::check(&report);
    }
    if transformations {
        life_transformations::check(&report);
    }
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
}

fn check_intrinsic_life(report: &Json) {
    let definitions = &report["definitions"];
    let curve = rows(&definitions["allied_life"]);
    assert_eq!(curve.len(), 100);
    assert_eq!(curve[0], 51);
    assert_eq!(curve[99], 17980);
    assert_eq!(definitions["profile_id"], "RaisedSkeletonSniper");
    assert_eq!(definitions["profile"]["life"], 0.55);
    assert_eq!(definitions["profile_hostile"], json!({"present":false}));
    assert_eq!(definitions["global_table_identity"], true);
    assert_eq!(definitions["global_profile_identity"], true);
    assert_eq!(rows(&definitions["minion_levels"]).len(), 40);
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 9);
    for index in [1, 2] {
        assert_eq!(cases[0]["xml_sha256"], cases[index]["xml_sha256"]);
        assert_eq!(
            json_evidence::first_difference(&cases[0]["state"], &cases[index]["state"], "replay"),
            None
        );
    }
    for (index, case) in cases.iter().enumerate() {
        let (raw, effective, actor_level) = match index {
            0..=3 => (20, 22, 44),
            4 => (1, 1, 2),
            5 => (2, 2, 4),
            6 => (19, 19, 38),
            7 => (20, 20, 40),
            8 => (40, 40, 80),
            _ => unreachable!(),
        };
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "saved_specs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
            "intrinsic_life_methods_preserved",
        ] {
            assert_eq!(case["state"][field], true, "{} {field}", case["name"]);
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
        for mode in ["main", "calcs"] {
            let actors: Vec<_> = rows(&case["state"][mode]["actors"])
                .iter()
                .filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .collect();
            assert_eq!(actors.len(), 1);
            let actor = actors[0];
            let selected = mode == "main" || index >= 3;
            assert_eq!(
                actor["is_environment_minion"], selected,
                "{} {mode}",
                case["name"]
            );
            assert_eq!(actor["physical_level"], raw);
            assert_eq!(actor["effective_level"], effective);
            assert_eq!(actor["actor_level"], actor_level);
            assert_eq!(actor["hostile"], false);
            let intrinsic = &actor["intrinsic_life"];
            let selections = rows(&intrinsic["table_selections"]);
            assert_eq!(selections.len(), 1);
            let selection = &selections[0];
            assert_eq!(selection["observed_at"], 963);
            assert_eq!(selection["ally_branch_executed"], true);
            for facts in [selection, &intrinsic["facts"]] {
                for field in [
                    "profile_is_loaded",
                    "life_table_is_allied",
                    "exact_source_actor",
                    "exact_parent",
                ] {
                    assert_eq!(facts[field], true);
                }
                assert_eq!(facts["hostile"], false);
                assert_eq!(facts["profile_hostile"], json!({"present":false}));
                assert_eq!(facts["life_table_is_hostile"], false);
                assert_eq!(facts["profile_life"], definitions["profile"]["life"]);
                assert_eq!(facts["actor_level"], actor_level);
                assert_eq!(facts["effective_level"], effective);
                assert_eq!(facts["table_value"], curve[actor_level as usize - 1]);
                assert_eq!(facts["source"], actor["source_occurrence"]);
            }
            assert_eq!(selection["level_table_value"], actor_level);
            let calls = rows(&actor["gigantic_benefits"]["original_life_calls"]);
            let initializers = rows(&intrinsic["initializers"]);
            assert_eq!(initializers.len(), usize::from(selected));
            if !selected {
                assert!(calls.is_empty());
                assert!(actor["gigantic_benefits"]["actor_output_life"].is_null());
                continue;
            }
            assert!(!calls.is_empty());
            let initialized = &initializers[0];
            assert_eq!(initialized["input"], intrinsic["facts"]);
            assert_eq!(initialized["unrounded_observed_at"], 1061);
            assert_eq!(initialized["stored_observed_at"], 1065);
            assert_eq!(initialized["selected"], true);
            assert_eq!(initialized["exact_actor_store"], true);
            assert_eq!(initialized["original_record_preserved"], true);
            assert_eq!(initialized["source"], actor["source_occurrence"]);
            assert_eq!(
                initialized["unrounded_base_life"],
                initialized["base_life_at_store"]
            );
            let stored = &initialized["stored_base"];
            assert_eq!(stored["name"], "Life");
            assert_eq!(stored["type"], "BASE");
            assert_eq!(stored["source"], "Base");
            assert_eq!(stored["flags"], 0);
            assert_eq!(stored["keyword_flags"], 0);
            assert!(rows(&stored["tags"]).is_empty());
            assert!(stored["value"].as_f64().unwrap() > 0.0);
            for call in calls {
                assert_eq!(call["exact_actor_store"], true);
                assert_eq!(call["exact_actor_output"], true);
                assert_eq!(call["source"], actor["source_occurrence"]);
                let c = &call["computation"];
                assert_eq!(c["intrinsic_base"]["record"], *stored);
                assert_eq!(c["intrinsic_base"]["original_record_is_eligible"], true);
                assert_eq!(c["base"], stored["value"]);
                assert_eq!(c["life_after_assignment"], call["return_life"]);
                assert_eq!(call["return_life"], call["post_return_life"]);
            }
            assert_eq!(
                calls.last().unwrap()["post_return_life"],
                actor["gigantic_benefits"]["actor_output_life"]
            );
        }
    }
}

const LIFE_DELIVERY_TEST: &str = "minion_life_increase_observes_original_delivery";
const LIFE_DELIVERY_CHILD: &str = "POE_MINION_LIFE_DELIVERY_SOURCE_CHILD";

#[test]
#[ignore = "requires complete pinned PoB runtime; actual passive Life delivery"]
fn minion_life_increase_observes_original_delivery() {
    run_life_source_modes(
        LIFE_DELIVERY_TEST,
        LIFE_DELIVERY_CHILD,
        "POE_MINION_LIFE_DELIVERY_SOURCE_OUT",
        "runs/owned-minion-life-delivery-source-01",
        run_life_delivery_child,
    );
}
fn run_life_delivery_child(root: &Path, out: &Path, enabled: bool) {
    run_actor_life_child(root, out, enabled, ActorLifeEvidence::Delivery);
}
fn check_life_delivery(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 7);
    for index in [1, 2] {
        assert_eq!(cases[0]["xml_sha256"], cases[index]["xml_sha256"]);
        assert_eq!(
            json_evidence::first_difference(&cases[0]["state"], &cases[index]["state"], "replay"),
            None
        );
    }
    let all = BTreeMap::from([
        ("Tree:19006", 6),
        ("Tree:229", 6),
        ("Tree:39461", 6),
        ("Tree:54453", 6),
        ("Tree:1218", 10),
        ("Tree:40894", 10),
    ]);
    for (index, case) in cases.iter().enumerate() {
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "saved_specs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
            "intrinsic_life_methods_preserved",
            "life_delivery_methods_preserved",
        ] {
            assert_eq!(case["state"][field], true, "{} {field}", case["name"]);
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        assert_eq!(case["state"]["business_method_wrappers"], false);
        assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
        for mode in ["main", "calcs"] {
            let actors: Vec<_> = rows(&case["state"][mode]["actors"])
                .iter()
                .filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .collect();
            assert_eq!(actors.len(), 1);
            let actor = actors[0];
            let selected = mode == "main" || index == 6;
            assert_eq!(actor["is_environment_minion"], selected);
            assert_eq!(actor["actor_level"], 44);
            assert_eq!(actor["effective_level"], 22);
            let delivery = &actor["life_delivery"];
            let transfers = rows(&delivery["transfers"]);
            let calls = rows(&actor["gigantic_benefits"]["original_life_calls"]);
            if !selected {
                assert!(transfers.is_empty() && calls.is_empty());
                continue;
            }
            assert!(!transfers.is_empty() && !calls.is_empty());
            let mut expected = all.clone();
            if index == 3 {
                expected.remove("Tree:229");
            }
            if index == 4 {
                expected.remove("Tree:1218");
            }
            if index == 5 {
                expected.clear();
            }
            let expected_total: i64 = expected.values().sum();
            let mut delivered = BTreeMap::new();
            let mut parent_calls = 0;
            for transfer in transfers {
                assert_eq!(transfer["selected"], true);
                assert_eq!(transfer["exact_parent"], true);
                assert_eq!(transfer["exact_summoner"], true);
                assert_eq!(transfer["exact_parent_cfg"], true);
                assert_eq!(transfer["source"], actor["source_occurrence"]);
                assert_eq!(transfer["list_return_observed"], true);
                assert_eq!(transfer["list_caller_line"], 1162);
                let listed = rows(&transfer["listed_life"]);
                let inserted = rows(&transfer["inserted_life"]);
                if transfer["parent_skill_store"] == true {
                    parent_calls += 1;
                    assert_eq!(transfer["caller_line"], 1854);
                    assert_eq!(listed.len(), expected.len());
                    assert_eq!(inserted.len(), expected.len());
                } else {
                    assert!(listed.is_empty() && inserted.is_empty());
                }
                for row in listed {
                    assert_eq!(row["recipient_type"], json!({"present":false}));
                    let record = &row["record"];
                    let source = record["source"].as_str().unwrap();
                    assert_eq!(record["name"], "Life");
                    assert_eq!(record["type"], "INC");
                    assert_eq!(record["value"], expected[source]);
                    assert_eq!(record["flags"], 0);
                    assert_eq!(record["keyword_flags"], 0);
                    assert!(rows(&record["tags"]).is_empty());
                    let provider = &row["provider"];
                    assert_eq!(provider["tree_source"], true);
                    assert_eq!(provider["allocated"], true);
                    let id: u64 = source.strip_prefix("Tree:").unwrap().parse().unwrap();
                    assert_eq!(provider["node_id"], id);
                    assert!(rows(&delivery["allocated_node_ids"]).contains(&json!(id)));
                    let matches = rows(&provider["matches"]);
                    assert_eq!(matches.len(), 1);
                    assert_eq!(matches[0]["record"], *record);
                    let inserted: Vec<_> = inserted
                        .iter()
                        .filter(|i| i["payload_index"] == row["payload_index"])
                        .collect();
                    assert_eq!(inserted.len(), 1);
                    let inserted = inserted[0];
                    assert_eq!(inserted["record"], *record);
                    assert_eq!(inserted["provider"], *provider);
                    assert_eq!(inserted["addmod_caller_line"], 1164);
                    assert_eq!(inserted["exact_list_payload"], true);
                    assert_eq!(inserted["exact_actor_store"], true);
                    assert_eq!(inserted["stored_identity_count"], 1);
                    assert!(delivered.insert(source, record).is_none());
                }
            }
            assert_eq!(parent_calls, 1);
            assert_eq!(delivered.len(), expected.len());
            for call in calls {
                assert_eq!(call["source"], actor["source_occurrence"]);
                assert_eq!(call["exact_actor_store"], true);
                assert_eq!(call["exact_actor_output"], true);
                let c = &call["computation"];
                assert_eq!(c["increased"], expected_total);
                assert_eq!(c["base"], 1615);
                assert_eq!(c["life_after_assignment"], call["return_life"]);
                assert_eq!(call["return_life"], call["post_return_life"]);
                let eligible = rows(&c["life_increase_delivery"]["eligible"]);
                assert_eq!(eligible.len(), expected.len());
                let mut seen = std::collections::BTreeSet::new();
                for row in eligible {
                    let source = row["record"]["source"].as_str().unwrap();
                    assert!(seen.insert(source));
                    assert_eq!(row["actual_transfer_count"], 1);
                    assert_eq!(row["record"], *delivered[source]);
                    assert_eq!(row["value"], expected[source]);
                }
            }
            for removed in match index {
                3 => vec![229],
                4 => vec![1218],
                5 => vec![19006, 229, 39461, 54453, 1218, 40894],
                _ => vec![],
            } {
                assert!(!rows(&delivery["allocated_node_ids"]).contains(&json!(removed)));
            }
        }
    }
}
