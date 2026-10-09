//! Retained original-source evidence; no source formulas or observations are
//! admitted as native runtime inputs by this authoring module.
use super::{read, root};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    value.as_array().unwrap()
}
fn pin(pin: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
    assert_eq!(hash(&bytes), pin["sha256"]);
    bytes
}

fn projection(report: &Value, names: &Value) -> Value {
    let mut result = vec![];
    for name in rows(names) {
        let cases: Vec<_> = rows(&report["cases"])
            .iter()
            .filter(|c| &c["name"] == name)
            .collect();
        assert_eq!(cases.len(), 1);
        let case = cases[0];
        assert_eq!(case["available"], true);
        for mode in ["main", "calcs"] {
            for actor in rows(&case["state"][mode]["actors"]).iter().filter(|a| {
                a["actor_profile"] == "RaisedSkeletonSniper"
                    && a["summon_effect_id"] == "SummonSkeletalSnipersPlayer"
            }) {
                let children: Vec<_> = rows(&actor["children"])
                    .iter()
                    .filter(|c| c["effect_id"] == "MinionMeleeBow")
                    .collect();
                assert_eq!(children.len(), 1);
                let child = children[0];
                let observed = child.get("passes").is_some();
                let mut row = json!({"case":name,"mode":mode,"xml_sha256":case["xml_sha256"],"warm_xml_sha256":case["warm_xml_sha256"],"actor_ordinal":actor["ordinal"],"actor_profile":actor["actor_profile"],"summon_effect_id":actor["summon_effect_id"],"actor_level":actor["actor_level"],"source_occurrence":actor["source_occurrence"],"profile":actor["profile"],"hostile":actor["hostile"],"child_effect":child["effect_id"],"summoner_owns_actor":child["summoner_owns_actor"],"summoner_source":child["summoner_source"],"observed":observed});
                if observed {
                    assert_eq!(rows(&child["passes"]).len(), 1);
                    let inputs = &child["passes"][0]["inputs"];
                    row["cfg"] = inputs["cfg"].clone();
                    row["added_more"] = inputs["bases"]["Physical"]["added_more"].clone();
                    row["query_state_preserved"] = inputs["query_state_preserved"].clone();
                    let calls: Vec<_> = rows(&child["base_calls"])
                        .iter()
                        .filter(|c| c["damage_type"] == "Physical")
                        .collect();
                    assert_eq!(calls.len(), 1);
                    row["base_call"] = calls[0].clone();
                }
                result.push(row);
            }
        }
    }
    json!(result)
}

pub fn check(v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let a: Value = read("authoring.json");
    assert_eq!(a["source_revision"], v["source_revision"]);
    assert_eq!(a["source_manifest_sha256"], v["source_manifest_sha256"]);
    let profile_packet: Value = serde_json::from_slice(&pin(&v["profile_packet"])).unwrap();
    assert_eq!(profile_packet["status"], "passed");
    assert_eq!(profile_packet["definitions"]["profile"], v["full_profile"]);
    assert_eq!(profile_packet["reports"], v["profile_reports"]);
    assert_eq!(
        profile_packet["report_metadata"],
        v["profile_report_metadata"]
    );
    assert_eq!(
        v["profile_identity"],
        json!({"profile_id":"RaisedSkeletonSniper","global_profile_identity":true,"profile_hostile":{"present":false}})
    );
    for (key, value) in v["profile_identity"].as_object().unwrap() {
        assert_eq!(&profile_packet["definitions"][key], value);
    }
    let p = &v["full_profile"];
    assert!(
        p.get("damageFixup").is_none(),
        "absence is distinct from a present numeric zero or Boolean false"
    );
    assert_eq!(p["damage"], 1.15);
    assert_eq!(p["name"], "Skeletal Sniper");
    assert_eq!(
        p["skillList"],
        json!(["MinionMeleeBow", "GasShotSkeletonSniperMinion"])
    );
    let vectors = rows(&v["vectors"]);
    assert_eq!(vectors.len(), 24);
    let observed: Vec<_> = vectors.iter().filter(|r| r["observed"] == true).collect();
    assert_eq!(observed.len(), 13);
    for row in vectors {
        assert_eq!(row["actor_profile"], "RaisedSkeletonSniper");
        assert_eq!(row["summon_effect_id"], "SummonSkeletalSnipersPlayer");
        assert_eq!(row["child_effect"], "MinionMeleeBow");
        assert_eq!(row["hostile"], false);
        assert_eq!(row["summoner_owns_actor"], true);
        assert_eq!(row["source_occurrence"]["source_present"], true);
        assert_eq!(rows(&row["source_occurrence"]["matches"]).len(), 1);
        assert_eq!(
            row["source_occurrence"]["matches"][0]["gem_id"],
            "Metadata/Items/Gems/SkillGemSkeletalSniper"
        );
        // This source observer retains scalar profile fields only. The complete
        // profile above, authenticated by its original-source witness, supplies
        // the absence proof; scalar projection alone cannot prove it.
        for (key, value) in row["profile"].as_object().unwrap() {
            assert_eq!(&p[key], value);
        }
        if row["observed"] != true {
            assert!(row.get("added_more").is_none());
            continue;
        }
        assert_eq!(row["query_state_preserved"], true);
        assert_eq!(row["cfg"]["skillName"], "Basic Attack");
        assert_eq!(row["cfg"]["summonSkillName"], "Skeletal Sniper Minion");
        assert_ne!(row["cfg"]["flags"].as_u64().unwrap() & 1, 0);
        let channel = &row["added_more"];
        assert_eq!(
            channel["names"],
            json!(["AddedPhysicalDamage", "AddedDamage"])
        );
        assert_eq!(rows(&channel["records"]).len(), 1);
        let record = &channel["records"][0];
        let amount = 14.999_999_999_999_991_f64;
        assert_eq!(
            record["value"].as_f64().unwrap().to_bits(),
            amount.to_bits()
        );
        assert_eq!(
            record["mod"],
            json!({"flags":1,"keyword_flags":0,"name":"AddedDamage","source":"Skeletal Sniper Damage Multiplier","tags":[{"neg":true,"partialMatch":true,"skillNameList":["Spectre","Companion"],"summonSkill":true,"type":"SkillName"}],"type":"MORE","value":amount})
        );
        assert_eq!(channel["value"], 1.15);
        assert_eq!(row["base_call"]["added_more"], *channel);
        assert_eq!(row["base_call"]["added_multiplier"], 1.15);
        assert_eq!(row["base_call"]["observed_at"], 4137);
        assert_eq!(row["base_call"]["query_state_preserved"], true);
    }
    for name in ["flat-physical", "flat-physical-quality-20"] {
        let row = observed
            .iter()
            .find(|r| r["case"] == name && r["mode"] == "main")
            .unwrap();
        assert_eq!(row["base_call"]["added_min"].as_f64(), Some(3.0));
        assert_eq!(row["base_call"]["added_max"].as_f64(), Some(7.0));
    }
    let report_pins = rows(&v["reports"]);
    assert_eq!(report_pins.len(), 2);
    for r in report_pins {
        assert_eq!(r["bytes"], 19_612_949);
        assert_eq!(
            r["sha256"],
            "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935"
        );
    }
    assert_eq!(
        v["observer"]["sha256"],
        "73ff0ee76552730818460ab93f5b3940bc612165c54e6bac6ab02faa7c54dcd8"
    );
    if !full {
        return;
    }
    let report = super::super::plain_damage_source::source_proof(&a);
    assert_eq!(
        report["evidence"]["observer_sha256"],
        v["observer"]["sha256"]
    );
    assert_eq!(report["evidence"]["case_count"], 37);
    assert_eq!(report["evidence"]["complete_load_attempts_per_jit"], 38);
    assert_eq!(projection(&report, &v["case_names"]), v["vectors"]);
    for report_pin in report_pins {
        assert_eq!(
            serde_json::from_slice::<Value>(&pin(report_pin)).unwrap(),
            report
        );
    }
    for life_pin in rows(&v["profile_reports"]) {
        let actual: Value = serde_json::from_slice(&pin(life_pin)).unwrap();
        assert_eq!(actual["definitions"]["profile"], v["full_profile"]);
        assert_eq!(actual["definitions"]["global_profile_identity"], true);
        assert_eq!(actual["definitions"]["profile_id"], "RaisedSkeletonSniper");
        let mut metadata = actual.clone();
        metadata.as_object_mut().unwrap().remove("cases");
        metadata.as_object_mut().unwrap().remove("definitions");
        assert_eq!(metadata, v["profile_report_metadata"]);
        for case in rows(&actual["cases"]) {
            assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
            assert_eq!(case["state"]["original_functions_preserved"], true);
            assert_eq!(case["state"]["intrinsic_life_methods_preserved"], true);
        }
    }
    for file in rows(&a["source_files"]) {
        let t = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(file["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(hash(t.as_bytes()), file["sha256"]);
    }
    for excerpt in rows(&v["source_excerpts"]) {
        let t = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(excerpt["path"].as_str().unwrap()),
        )
        .unwrap();
        let first = excerpt["first_line"].as_u64().unwrap() as usize;
        let last = excerpt["last_line"].as_u64().unwrap() as usize;
        assert_eq!(
            t.lines()
                .skip(first - 1)
                .take(last - first + 1)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
            excerpt["text"]
        );
    }
}
