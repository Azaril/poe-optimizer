//! Observe actual minion physical calcDamage calls, without replacing source methods.
#![cfg(not(target_arch = "wasm32"))]
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
    let out = root.join("runs/owned-minion-physical-damage-source-01");
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
    assert_eq!(cases.len(), 31);
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
    assert_eq!(cases.len(), 31);
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
