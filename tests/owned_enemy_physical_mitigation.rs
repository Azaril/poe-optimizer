//! Portable reference contract for the next owned mitigation consumer.
//! These observations authorize no source membership or native default.
use flate2::read::GzDecoder;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf};

const FAMILY: &str = "data/owned/poe2/3887ae68/enemy-physical-mitigation";
const REPORT_HASH: &str = "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935";
const BODIES: [&str; 5] = [
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/Data.lua",
    "src/Data/Misc.lua",
    "src/Modules/ConfigOptions.lua",
];
const SPANS: [(usize, u64, u64, &str); 9] = [
    (1, 56, 69, "signed-armour-response"),
    (0, 466, 475, "penetration-order"),
    (0, 3430, 3433, "physical-resistance-applicability"),
    (
        0,
        4197,
        4320,
        "separate-normal-critical-and-physical-reduction",
    ),
    (0, 4361, 4406, "damage-taken-and-final-mitigation"),
    (2, 240, 265, "game-coefficients-and-monster-cap"),
    (3, 245, 260, "monster-constants"),
    (4, 2073, 2097, "pinnacle-rating-default"),
    (4, 2183, 2188, "configured-armour-contribution"),
];
const FIELDS: [&str; 23] = [
    "PhysicalStoredHitMin",
    "PhysicalStoredHitMax",
    "PhysicalStoredHitAvg",
    "PhysicalStoredCritMin",
    "PhysicalStoredCritMax",
    "PhysicalStoredCritAvg",
    "PhysicalHitAverage",
    "PhysicalCritAverage",
    "CritChance",
    "CritMultiplier",
    "HitChance",
    "AverageDamage",
    "Speed",
    "DpsMultiplier",
    "TotalDPS",
    "LightningHitAverage",
    "LightningCritAverage",
    "ColdHitAverage",
    "ColdCritAverage",
    "FireHitAverage",
    "FireCritAverage",
    "ChaosHitAverage",
    "ChaosCritAverage",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(FAMILY).join(name)).unwrap()).unwrap()
}
fn authenticated(pin: &Value, limit: u64) -> Vec<u8> {
    let path = root().join(pin["path"].as_str().unwrap());
    assert!(fs::metadata(&path).unwrap().len() <= limit);
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len(), pin["bytes"]);
    assert_eq!(hash(&bytes), pin["sha256"]);
    bytes
}
fn expanded(bytes: &[u8], limit: u64) -> Vec<u8> {
    let mut raw = vec![];
    GzDecoder::new(bytes)
        .take(limit + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= limit);
    raw
}
fn report() -> Value {
    let evidence = read("source-evidence.json");
    let pin = &evidence["report"];
    let raw = expanded(
        &authenticated(&pin["archive"], 2 * 1024 * 1024),
        24 * 1024 * 1024,
    );
    assert_eq!(raw.len(), pin["bytes"]);
    assert_eq!(hash(&raw), REPORT_HASH);
    assert_eq!(pin["sha256"], REPORT_HASH);
    serde_json::from_slice(&raw).unwrap()
}

#[test]
fn mitigation_authority_keeps_complete_original_bodies_and_distinct_monster_constants() {
    let evidence = read("source-evidence.json");
    assert_eq!(evidence["schema_version"], 1);
    assert_eq!(
        evidence["scope"],
        json!({
            "published": false, "native_consumer_implemented": false,
            "source_supplier_complete": false, "new_source_execution": false,
            "source_values_are_runtime_inputs": false
        })
    );
    let manifest: Value =
        serde_json::from_slice(&authenticated(&evidence["manifest"], 256 * 1024)).unwrap();
    assert_eq!(
        evidence["manifest"]["sha256"],
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    assert_eq!(evidence["source_revision"], manifest["upstream_revision"]);
    assert_eq!(
        evidence["license"]["sha256"],
        "22d2d075c1d361971764fbbd1e12e1485bdf35f0769ffac4eca8a79afc60dda8"
    );
    authenticated(&evidence["license"], 128 * 1024);
    let mut bodies = BTreeMap::new();
    assert_eq!(evidence["bodies"].as_array().unwrap().len(), BODIES.len());
    for (body, expected) in evidence["bodies"].as_array().unwrap().iter().zip(BODIES) {
        assert_eq!(body["source_path"], expected);
        let stored = authenticated(&body["archive"], 512 * 1024);
        let raw = match body["encoding"].as_str().unwrap() {
            "gzip" => expanded(&stored, 512 * 1024),
            "plain" => stored,
            _ => panic!("unknown source encoding"),
        };
        assert_eq!(raw.len(), body["bytes"]);
        assert_eq!(hash(&raw), body["sha256"]);
        let source = body["source_path"].as_str().unwrap();
        let entry = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["path"] == source)
            .unwrap();
        assert_eq!(body["sha256"], entry["sha256"]);
        assert!(
            bodies
                .insert(source.to_owned(), String::from_utf8(raw).unwrap())
                .is_none()
        );
    }
    assert_eq!(evidence["spans"].as_array().unwrap().len(), SPANS.len());
    for (span, (source, expected_first, expected_last, role)) in
        evidence["spans"].as_array().unwrap().iter().zip(SPANS)
    {
        assert_eq!(span["source_path"], BODIES[source]);
        assert_eq!(span["first"], expected_first);
        assert_eq!(span["last"], expected_last);
        assert_eq!(span["role"], role);
        let text = &bodies[span["source_path"].as_str().unwrap()];
        let first = span["first"].as_u64().unwrap() as usize;
        let last = span["last"].as_u64().unwrap() as usize;
        assert!(first > 0 && last >= first && last <= text.lines().count());
        let excerpt = text
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(hash(excerpt.as_bytes()), span["sha256"]);
    }
    let misc = &bodies["src/Data/Misc.lua"];
    let (_, monster) = misc.split_once("data.monsterConstants = {").unwrap();
    assert!(monster.contains("[\"maximum_physical_damage_reduction_%\"] = 75,"));
    let data = &bodies["src/Modules/Data.lua"];
    assert!(data.contains("EnemyPhysicalDamageReductionCap = data.monsterConstants[\"maximum_physical_damage_reduction_%\"]"));
    assert!(data.contains("ArmourRatio = 10,"));
    assert!(data.contains("NegArmourDmgBonusCap = 100,"));
    let inputs: Value = serde_json::from_slice(&authenticated(
        &evidence["existing_armour_producer"][1],
        64 * 1024,
    ))
    .unwrap();
    authenticated(&evidence["existing_armour_producer"][0], 512 * 1024);
    assert_eq!(
        inputs["inputs"][0]["contribution_stat"]["key"],
        "def.00000000000031e0"
    );
    assert_eq!(
        inputs["inputs"][0]["program"],
        "configured-enemy-armour-base"
    );
}

#[test]
fn every_observed_exact_sniper_pass_is_preserved_without_inventing_missing_outputs() {
    let source = report();
    let mut rows = vec![];
    for case in source["cases"].as_array().unwrap() {
        for mode in ["main", "calcs"] {
            let env = &case["state"][mode];
            let Some(actors) = env["actors"].as_array() else {
                continue;
            };
            for actor in actors
                .iter()
                .filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
            {
                for child in actor["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| c["effect_id"] == "MinionMeleeBow")
                {
                    let Some(passes) = child["passes"].as_array() else {
                        continue;
                    };
                    for pass in passes {
                        let output: Map<String, Value> = FIELDS
                            .iter()
                            .filter_map(|field| {
                                pass["output"]
                                    .get(field)
                                    .map(|v| ((*field).into(), v.clone()))
                            })
                            .collect();
                        rows.push(json!({"case":case["name"],"mode":mode,"effective":env["effective"],"actor_profile":actor["actor_profile"],"source_occurrence":actor["source_occurrence"],"label":pass["label"],"output":output}));
                    }
                }
            }
        }
    }
    assert_eq!(rows.len(), 39);
    let evidence = read("source-evidence.json");
    let vectors: Value =
        serde_json::from_slice(&authenticated(&evidence["vectors"], 128 * 1024)).unwrap();
    assert_eq!(
        vectors,
        json!({"schema_version":1,"kind":"observed-enemy-physical-mitigation","source_report_sha256":REPORT_HASH,"rows":rows})
    );
    let suppressed = rows
        .iter()
        .find(|r| r["case"] == "no-physical-damage")
        .unwrap();
    assert!(suppressed["output"].get("PhysicalStoredHitAvg").is_none());
    assert_eq!(suppressed["output"]["PhysicalHitAverage"], 0);
}

#[test]
fn retained_results_require_separate_critical_mitigation_and_preserve_other_damage_types() {
    let vectors = read("source-vectors.json");
    let rows = vectors["rows"].as_array().unwrap();
    let original = rows.iter().find(|r| r["case"] == "original-05").unwrap();
    let o = &original["output"];
    assert_eq!(o["PhysicalHitAverage"], 414.2081976279727);
    assert_eq!(o["PhysicalCritAverage"], 1101.2392272188865);
    assert_ne!(
        o["PhysicalHitAverage"].as_f64().unwrap() * o["CritMultiplier"].as_f64().unwrap(),
        o["PhysicalCritAverage"].as_f64().unwrap()
    );
    for row in rows {
        let output = &row["output"];
        let number = |name: &str| output[name].as_f64().unwrap();
        let mut normal = 0.;
        let mut critical = 0.;
        for damage in ["Physical", "Lightning", "Cold", "Fire", "Chaos"] {
            normal += number(&format!("{damage}HitAverage"));
            critical += number(&format!("{damage}CritAverage"));
        }
        let average =
            normal * (1. - number("CritChance") / 100.) + critical * number("CritChance") / 100.;
        assert_eq!(
            average * number("HitChance") / 100.,
            number("AverageDamage"),
            "{}/{}",
            row["case"],
            row["mode"]
        );
        if row["effective"] == false {
            assert_eq!(output["PhysicalStoredHitAvg"], output["PhysicalHitAverage"]);
            assert_eq!(
                output["PhysicalStoredCritAvg"],
                output["PhysicalCritAverage"]
            );
        }
    }
    let converted = rows
        .iter()
        .find(|r| r["case"] == "conversion-over-100")
        .unwrap();
    assert_eq!(converted["output"]["PhysicalHitAverage"], 0);
    assert!(converted["output"]["TotalDPS"].as_f64().unwrap() > 0.);
}
