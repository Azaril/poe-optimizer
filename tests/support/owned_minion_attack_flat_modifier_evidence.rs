//! Original-query evidence for two declared affixes, not obtainable item authority.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub const PACKET: &str =
    "data/owned/poe2/3887ae68/minion-attack-flat-modifier/source-evidence.json";
pub const PRIOR: &str = "data/owned/poe2/3887ae68/self-flat-attack-damage/source-evidence.json";
pub const PRIOR_SHA: &str = "2a07ddefddd45cdd01a8593c506412428997eff3c618ba2fc365b6e0d93c5ba8";
pub const FAMILIES: [(&str, &str, [u32; 4]); 2] = [
    (
        "UniqueAddedPhysicalToMinionAttacks1",
        "Minions deal (5-8) to (10-12) additional Attack Physical Damage",
        [5, 8, 10, 12],
    ),
    (
        "AddedPhysicalToMinionAttacksUnique__1",
        "Minions deal (5-8) to (12-16) additional Attack Physical Damage",
        [5, 8, 12, 16],
    ),
];
pub fn root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if p.join("data/owned").is_dir() {
        p
    } else {
        p.join("../..")
    }
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn prior() -> Value {
    let bytes = std::fs::read(root().join(PRIOR)).unwrap();
    assert_eq!(digest(&bytes), PRIOR_SHA);
    serde_json::from_slice(&bytes).unwrap()
}
pub fn scope() -> Value {
    json!({"declared_affix_semantics_only":true,"game_obtainability":false,
        "complete_selected_input_coverage":false,"native_total_flat_coverage":false,
        "full_build_execution":false,"original_helper_and_queries":true,
        "query_context":"reviewed-source03-Basic-scalar-projection",
        "jit":"off","zero_and_condition_controls_are_outside_admitted_family":true})
}
pub fn cases() -> Vec<Value> {
    let mut rows = Vec::new();
    for (id, _, bounds) in FAMILIES {
        for (suffix, min, max) in [
            ("lower", bounds[0], bounds[2]),
            ("cross-a", bounds[0], bounds[3]),
            ("cross-b", bounds[1], bounds[2]),
            ("upper", bounds[1], bounds[3]),
            ("interior", 7, if bounds[2] == 10 { 11 } else { 14 }),
        ] {
            rows.push(json!({"id":format!("{id}-{suffix}"),"family":id,"lines":[format!("Minions deal {min} to {max} additional Attack Physical Damage")],"minimum":min,"maximum":max,"full_life":false,"raw_count":2,"admitted_roll":true}));
        }
    }
    for (id, lines, min, max, full_life, raw_count) in [
        ("empty", vec![], 0, 0, false, 0),
        (
            "explicit-zero",
            vec!["Minions deal 0 to 0 additional Attack Physical Damage"],
            0,
            0,
            false,
            2,
        ),
        (
            "conditional-disabled",
            vec!["Minions deal 7 to 11 additional Attack Physical Damage while on Full Life"],
            0,
            0,
            false,
            2,
        ),
        (
            "conditional-enabled",
            vec!["Minions deal 7 to 11 additional Attack Physical Damage while on Full Life"],
            7,
            11,
            true,
            2,
        ),
        (
            "duplicate",
            vec![
                "Minions deal 7 to 11 additional Attack Physical Damage",
                "Minions deal 7 to 11 additional Attack Physical Damage",
            ],
            14,
            22,
            false,
            4,
        ),
    ] {
        rows.push(json!({"id":id,"family":null,"lines":lines,"minimum":min,"maximum":max,"full_life":full_life,"raw_count":raw_count,"admitted_roll":false}));
    }
    rows
}
pub fn check(v: &Value) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["kind"], "original-minion-attack-flat-affix-queries");
    assert_eq!(
        v["upstream_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(v["scope"], scope());
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .unwrap(),
    )
    .unwrap();
    let files = v["files"].as_object().unwrap();
    for (path, sha) in files {
        let pinned = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["path"] == *path)
            .unwrap();
        assert_eq!(sha, &pinned["sha256"]);
    }
    let order = v["module_order"].as_array().unwrap();
    for path in order {
        assert!(files.contains_key(path.as_str().unwrap()));
    }
    assert_eq!(
        files.len(),
        order
            .iter()
            .map(|p| p.as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            + 1
    );
    for path in [
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModDB.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModItemExclusive.lua",
    ] {
        assert!(order.contains(&json!(path)));
    }
    assert_eq!(v["delivery_helper"]["path"], "src/Modules/CalcPerform.lua");
    assert_eq!(v["delivery_helper"]["first_line"], 1161);
    assert_eq!(
        v["delivery_helper"]["sha256"],
        "50257f2b58a59f976fc5d60106c8d76abf481b2da22f5a820b58c481c6c8a97c"
    );
    assert_eq!(
        digest(v["delivery_helper"]["body"].as_str().unwrap().as_bytes()),
        v["delivery_helper"]["sha256"]
    );
    assert_eq!(v["dependency"], json!({"path":PRIOR,"sha256":PRIOR_SHA}));
    let p = prior();
    assert_eq!(v["source_context"], p["observations"][0]);
    assert_eq!(v["source_context"]["case"], "original-05");
    assert_eq!(v["source_context"]["observed"], true);
    assert_eq!(v["source_context"]["cfg"]["fields"]["keywordFlags"], 330752);
    let projection = &v["source_context"]["cfg"];
    let mut context = projection["fields"].clone();
    for (target, source) in [
        ("skillCond", "skill_conditions"),
        ("skillStats", "skill_stats"),
        ("skillGem", "skill_gem"),
        ("skillGemTags", "skill_gem_tags"),
        ("grantedEffect", "granted_effect"),
    ] {
        context[target] = projection[source].clone();
    }
    context["skillTypes"] = json!({});
    for row in projection["skill_types"].as_array().unwrap() {
        context["skillTypes"][format!("[{}]", row["id"])] = row["value"].clone();
    }
    let declarations = v["declarations"].as_array().unwrap();
    assert_eq!(declarations.len(), 2);
    for (row, (id, line, bounds)) in declarations.iter().zip(FAMILIES) {
        assert_eq!(row["id"], id);
        assert_eq!(row["bounds"], json!(bounds));
        assert_eq!(
            row["body"],
            json!({"[1]":line,"affix":"","statOrder":{"[1]":3442},"level":1,"group":"AddedPhysicalToMinionAttacks","weightKey":{},"weightVal":{},"modTags":{"[1]":"minion_damage","[2]":"physical_damage","[3]":"damage","[4]":"physical","[5]":"minion"},"tradeHashes":{"[797833282]":{"[1]":line}}})
        );
    }
    let rows = v["cases"].as_array().unwrap();
    let expected = cases();
    assert_eq!(rows.len(), expected.len());
    for (row, case) in rows.iter().zip(expected) {
        assert_eq!(row["input"], case);
        let result = &row["result"];
        assert_eq!(result["context_before"], context);
        assert_eq!(result["context_before"], result["context_after"]);
        assert_eq!(result["records_before"], result["records_after"]);
        assert_eq!(result["original_payload_identity"], true);
        assert_eq!(result["query_repeat_equal"], true);
        assert_eq!(result["parent_identity"], true);
        let raw = result["raw"].as_array().unwrap();
        assert_eq!(raw.len(), case["raw_count"].as_u64().unwrap() as usize);
        for (i, record) in raw.iter().enumerate() {
            let line = case["lines"][i / 2].as_str().unwrap();
            let number = line
                .split_whitespace()
                .nth(if i % 2 == 0 { 2 } else { 4 })
                .unwrap()
                .parse::<f64>()
                .unwrap();
            assert_eq!(record["value"].as_f64(), Some(number));
            assert_eq!(
                record["name"],
                if i % 2 == 0 {
                    "PhysicalMin"
                } else {
                    "PhysicalMax"
                }
            );
            assert_eq!(record["type"], "BASE");
            assert_eq!(record["flags"], 0);
            assert_eq!(record["keywordFlags"], 65536);
            if case["id"].as_str().unwrap().starts_with("conditional-") {
                assert_eq!(record["[1]"], json!({"type":"Condition","var":"FullLife"}));
            } else {
                assert!(record.get("[1]").is_none());
            }
            assert_eq!(
                record.as_object().unwrap().len(),
                if line.contains("while") { 6 } else { 5 }
            );
            let outer = &result["records_before"][format!("[{}]", i + 1)];
            assert_eq!(
                outer,
                &json!({"name":"MinionModifier","type":"LIST","flags":0,"keywordFlags":0,"value":{"mod":record}})
            );
        }
        for lane in ["direct", "inherited"] {
            let q = &result[lane];
            assert_eq!(q["minimum"].as_f64(), case["minimum"].as_f64());
            assert_eq!(q["maximum"].as_f64(), case["maximum"].as_f64());
            let n = if case["minimum"].as_u64() == Some(0) {
                0
            } else {
                raw.len() / 2
            };
            assert_eq!(q["minimum_count"], n);
            assert_eq!(q["maximum_count"], n);
            assert_eq!(q["original_record_identity"], true);
            for (name, offset) in [("minimum_records", 0), ("maximum_records", 1)] {
                let expected: serde_json::Map<String, Value> = (0..n)
                    .map(|i| {
                        let r = &raw[i * 2 + offset];
                        (format!("[{}]", i + 1), json!({"value":r["value"],"mod":r}))
                    })
                    .collect();
                assert_eq!(q[name], Value::Object(expected));
            }
        }
        for lane in ["non_attack_direct", "non_attack_inherited"] {
            let q = &result[lane];
            assert_eq!(q["minimum"].as_f64(), Some(0.));
            assert_eq!(q["maximum"].as_f64(), Some(0.));
            assert_eq!(q["minimum_count"], 0);
            assert_eq!(q["maximum_count"], 0);
            assert_eq!(q["minimum_records"], json!({}));
            assert_eq!(q["maximum_records"], json!({}));
        }
        assert_eq!(result["context_before"]["flags"], 42949813253u64);
        assert_eq!(result["context_before"]["keywordFlags"], 330752);
        assert_eq!(result["non_attack_context"]["keywordFlags"], 265216);
        let mut inverse = result["non_attack_context"].clone();
        inverse["keywordFlags"] = json!(330752);
        assert_eq!(inverse, result["context_before"]);
    }
}

#[test]
fn retained_affix_witness_rejects_changed_inputs_qualification_and_scope() {
    let bytes = std::fs::read(root().join(PACKET)).unwrap();
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    check(&original);
    for edit in 0..8 {
        let mut v = original.clone();
        match edit {
            0 => v["scope"]["game_obtainability"] = json!(true),
            1 => v["declarations"][0]["body"]["[1]"] = json!("different family"),
            2 => v["cases"][0]["result"]["non_attack_direct"]["minimum"] = json!(5),
            3 => v["cases"][0]["result"]["raw"][0]["keywordFlags"] = json!(0),
            4 => v["cases"][11]["result"]["raw"] = json!([]),
            5 => v["cases"][0]["result"]["original_payload_identity"] = json!(false),
            6 => v["source_context"]["observed"] = json!(false),
            7 => v["cases"][0]["input"]["minimum"] = json!(0),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| check(&v)).is_err());
    }
}
