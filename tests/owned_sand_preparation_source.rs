//! Ordinary CI authenticates retained acquisition evidence without loading PoB.
#[allow(dead_code)]
#[path = "support/owned_sand_preparation_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::owned_build::ParameterValue;
use poe_optimizer_engine::owned_plan::EffectValue;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs};

fn evidence() -> Value {
    let proof: Value = read("source-vectors.json");
    assert_eq!(proof["status"], "retained-source-witness-passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        proof["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    for path in [
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcTools.lua",
        "src/Data/Misc.lua",
        "src/Data/Gems.lua",
        "src/Data/Skills/other.lua",
    ] {
        assert_eq!(
            proof["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == path)
                .count(),
            1
        );
    }
    for pin in proof["source_files"].as_array().unwrap() {
        let rows: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["path"] == pin["path"])
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["sha256"], pin["sha256"]);
    }
    for pin in proof["local_pins"].as_array().unwrap() {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    }
    assert_eq!(proof["local_pins"].as_array().unwrap().len(), 5);
    for value in proof["authority"].as_object().unwrap().values() {
        assert_eq!(value, false);
    }
    assert_eq!(proof["case_count"], 18);
    assert_eq!(proof["vectors"].as_array().unwrap().len(), 140);
    for row in proof["vectors"].as_array().unwrap() {
        assert_eq!(
            row["observed_stages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["fresh", "rebuilt_once", "rebuilt_twice"])
        );
    }
    proof
}

#[test]
fn retained_source_binds_tags_order_population_and_excludes_unproved_authority() {
    let p = evidence();
    let dependencies: Value = read("dependencies.json");
    let rows = dependencies["tables"][0]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 40);
    for (native, source) in rows.iter().zip(p["minion_level_table"].as_array().unwrap()) {
        assert_eq!(native["value"], source["output"]);
    }
    let original: Vec<_> = p["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["case"] == "original-05")
        .collect();
    assert_eq!(original.len(), 8);
    for row in original {
        for tag in ["minion", "persistent", "command", "physical"] {
            assert_eq!(row["catalog"]["tags"][tag], true);
        }
        assert_eq!(row["catalog"]["tags"]["spell"], Value::Null);
        let ordinary = row["ordinary"].as_array().unwrap();
        assert_eq!(ordinary.len(), 1);
        let matched = ordinary[0]["matched"].as_array().unwrap();
        assert_eq!(matched.len(), 3);
        assert_eq!(
            matched
                .iter()
                .map(|r| r["value"]["value"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            vec![1., 1., 0.]
        );
        assert_eq!(matched[0]["sourceSlot"], "Helmet");
        assert_eq!(matched[1]["sourceSlot"], "Amulet");
        assert_eq!(matched[2]["sourceSlot"], "Amulet");
        let supported = row["supported"].as_array().unwrap();
        assert_eq!(supported.len(), 1);
        assert!(supported[0]["properties"].as_array().unwrap().is_empty());
        assert_eq!(supported[0]["query_parent_is_actor"], true);
        assert_eq!(supported[0]["cache_identity_preserved"], true);
        let tree = row["group"]["group_source"] == "Tree:13289";
        assert_eq!(supported[0]["count"], if tree { 0 } else { 3 });
        assert_eq!(row["group"]["no_supports"], tree);
        if row["effect"] == "SummonSandDjinnPlayer" {
            let population = &row["population"];
            assert_eq!(population["present"], true);
            assert_eq!(population["level"], population["table_output"]);
            let overrides = population["overrides"].as_object().unwrap();
            assert_eq!(overrides.len(), 4);
            assert!(overrides.values().all(|r| r["present"] == false));
            let children = population["children"].as_array().unwrap();
            assert_eq!(children.len(), 3);
            for c in children {
                assert_eq!(c["input"]["level"], 1);
                assert_eq!(c["input"]["quality"], 0);
                assert_eq!(c["actor_level"], population["level"]);
                assert_eq!(c["source_instance_present"], false);
                assert_eq!(c["catalog_present"], false);
                assert_eq!(c["exact_parent"], true);
                assert_eq!(c["exact_actor"], true);
            }
        }
    }
}

#[test]
fn actual_arithmetic_matches_retained_manual_and_tree_source_vectors() {
    let p = evidence();
    let rows = p["vectors"].as_array().unwrap();
    let mut cases = BTreeSet::new();
    for row in rows.iter().filter(|r| {
        (r["mode"] == "MAIN" || r["mode"] == "CALCS")
            && r["effect"] == "SummonSandDjinnPlayer"
            && r["group"]["group_source"] != "Tree:13289"
            && !r["case"].as_str().unwrap().contains("diagnostic")
            && r["ordinary"].as_array().unwrap().len() == 1
    }) {
        let name = row["case"].as_str().unwrap();
        let mode = row["mode"].as_str().unwrap();
        let Some(tree_row) = rows.iter().find(|r| {
            r["case"] == name
                && r["mode"] == mode
                && r["effect"] == "SummonSandDjinnPlayer"
                && r["group"]["group_source"] == "Tree:13289"
        }) else {
            continue;
        };
        let ordinary = row["ordinary"][0]["matched"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["value"]["value"].as_f64().unwrap())
            .sum();
        let w = World::new(
            row["raw"]["level"].as_f64().unwrap(),
            row["raw"]["quality"].as_f64().unwrap(),
            tree_row["raw"]["quality"].as_f64().unwrap(),
            ordinary,
        );
        let plan = w.compile().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let effects = evaluated(&report);
        for (target, source) in [(manual(), row), (tree(), tree_row)] {
            assert_eq!(
                value(effects, &stat_key(target.clone(), 0x334e)),
                Some(&EffectValue::Known {
                    value: ParameterValue::Integer(integer(
                        source["final"]["level"].as_i64().unwrap()
                    ))
                }),
                "{name}/{mode}"
            );
            assert_eq!(
                value(effects, &stat_key(target.clone(), 0x334f)),
                Some(&EffectValue::Known {
                    value: quantity(source["final"]["quality"].as_f64().unwrap(), 2)
                }),
                "{name}/{mode}"
            );
            assert_eq!(
                value(effects, &actor_level(&target)),
                Some(&EffectValue::Known {
                    value: ParameterValue::Integer(integer(
                        source["population"]["level"].as_i64().unwrap()
                    ))
                }),
                "{name}/{mode}"
            );
            // Command is a distinct effect of this exact source occurrence.
            // This proves projected inputs, not its unconverted numerical rules.
            let commands: Vec<_> = rows
                .iter()
                .filter(|r| {
                    r["case"] == name
                        && r["mode"] == mode
                        && r["effect"] == "CommandSandDjinnKnifeThrowPlayer"
                        && r["source"] == source["source"]
                })
                .collect();
            assert_eq!(commands.len(), 1, "{name}/{mode}/{target:?}");
            let command = commands[0];
            assert_eq!(command["population"]["present"], false);
            assert_eq!(
                value(effects, &command_parameter(&target, 0x3350)),
                Some(&EffectValue::Known {
                    value: ParameterValue::Integer(integer(
                        command["final"]["level"].as_i64().unwrap()
                    ))
                }),
                "{name}/{mode}/{target:?} Command level"
            );
            assert_eq!(
                value(effects, &command_parameter(&target, 0x3351)),
                Some(&EffectValue::Known {
                    value: quantity(command["final"]["quality"].as_f64().unwrap(), 2)
                }),
                "{name}/{mode}/{target:?} Command quality"
            );
        }
        cases.insert((name, mode));
    }
    for mode in ["MAIN", "CALCS"] {
        for name in [
            "original-01",
            "original-05",
            "manual-level-domain-max",
            "manual-quality-negative",
            "manual-quality-above-hundred",
        ] {
            assert!(cases.contains(&(name, mode)), "{name}/{mode}");
        }
    }
}
