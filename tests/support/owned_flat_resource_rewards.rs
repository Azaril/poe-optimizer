//! Four exact reward producers; schema, selection policy and all resource totals stay unchanged.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules, owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/flat-resource-rewards")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn observation<'a>(report: &'a Value, pointer: &str) -> &'a Value {
    let matching: Vec<_> = report["observations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["pointer"] == pointer)
        .collect();
    assert_eq!(matching.len(), 1, "exact evidence pointer {pointer}");
    &matching[0]["value"]
}
fn source_record(binding: &Value) -> Value {
    json!({
        "flags":0, "keyword_flags":0, "name":binding["source_stat"],
        "source":binding["source_origin"], "tags":{}, "type":"BASE", "value":binding["amount"]
    })
}
fn expected_owner(binding: &Value) -> Value {
    json!({
        "owner":{"kind":"definition","value":{"kind":"reward","value":binding["reward"]}},
        "programs":{"closure":{"kind":"complete"},"members":[{
            "id":binding["program"],"context":"actor","reads":[],
            "nodes":[{"id":"amount","expression":{"kind":"literal","value":{
                "kind":"quantity","value":{"value":binding["amount"].as_f64().unwrap(),"unit":binding["unit"]}
            }}}],
            "effects":[{"id":"grant","when":null,"effect":{
                "kind":"contribute","entity":"player","stat":binding["stat"],"contribution":"add","value":"amount"
            }}]
        }]}
    })
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    assert_eq!(a["allocated_definitions"], 0);
    assert_eq!(a["registry_last_issued_before"], 0x32e5);
    assert_eq!(a["registry_last_issued_after"], 0x32e5);
    assert_eq!(a["new_complete_program_owners"], 4);
    assert_eq!(a["appended_numerical_programs"], 4);
    assert_eq!(a["changed_schema_descriptors"], 0);
    for field in [
        "before",
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
    ] {
        assert_eq!(b[field], a[field]);
        assert_eq!(d["source"][field], a[field]);
    }
    assert_eq!(b["reward_policy"], a["rewards"]);
    assert_eq!(d["source"]["rewards"], a["rewards"]);
    assert_eq!(c["definitions"], json!([]));
    assert_eq!(d["owners"], json!([]));
    assert_eq!(a["scope"]["selection_policy_changed"], false);
    assert_eq!(a["scope"]["whole_resource_results"], false);
    assert_eq!(a["scope"]["selected_build_complete"], false);
    assert_eq!(a["scope"]["published_scalar_reducers"], 0);
    assert_eq!(a["scope"]["published_receivers"], 0);
    let bindings = b["rewards"].as_array().unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    assert_eq!(bindings.len(), 4);
    assert_eq!(owners.len(), 4);
    assert_eq!(definitions.len(), 8);
    assert_eq!(d["absent_owners"].as_array().unwrap().len(), 4);
    let expected = [
        ("0028", "Spirit", 30, "3166", "0004", 3),
        ("0029", "Life", 20, "311a", "3119", 5),
        ("0030", "Spirit", 30, "3166", "0004", 11),
        ("0063", "Spirit", 40, "3166", "0004", 27),
    ];
    for (index, (binding, (reward, name, amount, stat, unit, source_index))) in
        bindings.iter().zip(expected).enumerate()
    {
        assert_eq!(
            binding["reward"]["key"],
            format!("def.000000000000{reward}")
        );
        assert_eq!(binding["source_stat"], name);
        assert_eq!(binding["amount"], amount);
        assert_eq!(binding["stat"]["key"], format!("def.000000000000{stat}"));
        assert_eq!(binding["unit"]["key"], format!("def.000000000000{unit}"));
        assert_eq!(binding["source_index"], source_index);
        assert_eq!(binding["source_value"], true);
        assert_eq!(binding["program"], "flat-resource-contribution");
        assert_eq!(json!(owners[index]), expected_owner(binding));
        assert_eq!(d["absent_owners"][index], json!(owners[index].owner));
        let reward_schema = &d["definitions"][index];
        assert_eq!(reward_schema["kind"], "reward");
        assert_eq!(reward_schema["value"]["id"], binding["reward"]);
        assert_eq!(reward_schema["value"]["schema"]["kind"], "known");
        let declarations = reward_schema["value"]["schema"]["value"]["declarations"]
            .as_object()
            .unwrap();
        assert_eq!(declarations.len(), 7);
        for declaration in declarations.values() {
            assert_eq!(
                *declaration,
                json!({"members":[],"closure":{"kind":"complete"}})
            );
        }
        let stat_schema = d["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["value"]["id"] == binding["stat"])
            .unwrap();
        assert_eq!(
            stat_schema["value"]["schema"],
            json!({"kind":"known","value":{
                "value":{"kind":"quantity","value":{"unit":binding["unit"]}},"targets":["actor"]
            }})
        );
    }
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "closure", "dependencies", "source-vectors"]
    );
    for (name, digest) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*digest, hash(&bytes));
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut pins = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(pins.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|candidate| *candidate == pin)
                .count(),
            1
        );
    }
    assert!(pins.contains("src/Data/QuestRewards.lua"));
    assert!(pins.contains("src/Modules/ConfigOptions.lua"));
    let source_rows = v["reward_rows"].as_array().unwrap();
    assert_eq!(source_rows.len(), 4);
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    for (binding, source) in bindings.iter().zip(source_rows) {
        assert_eq!(binding["reward"], source["reward"]);
        let census = observation(&reports[0], source["census_pointer"].as_str().unwrap());
        let matrix = observation(&reports[0], source["matrix_pointer"].as_str().unwrap());
        assert_eq!(census["key"], binding["source_option"]);
        assert_eq!(census["source_index"], binding["source_index"]);
        assert_eq!(census["source"], binding["source_origin"]);
        assert_eq!(census["widget"], "check");
        assert_eq!(census["default"], true);
        assert_eq!(census["quest"]["useConfig"], true);
        assert!(census["quest"].get("questPoints").is_none());
        assert!(census["quest"].get("Options").is_none());
        assert_eq!(
            matrix,
            &json!({"key":binding["source_option"],"value":true,"records":[source_record(binding)],"enemy_records":{}})
        );
        let policy = &source["policy"];
        assert_eq!(
            policy["recipe"]["tiers"],
            json!([{"selectors":[{"lane":"input_boolean","name":binding["source_option"]}],"duplicates":"last_in_source_order"}])
        );
        assert_eq!(
            policy["recipe"]["missing"],
            json!({"kind":"explicit","value":{"kind":"boolean","value":true}})
        );
        assert_eq!(
            policy["outcomes"][0],
            json!({"when":{"kind":"boolean","value":false},"outcome":{"kind":"none"}})
        );
        assert_eq!(policy["outcomes"].as_array().unwrap().len(), 2);
        assert_eq!(
            policy["outcomes"][1]["when"],
            json!({"kind":"boolean","value":true})
        );
        assert_eq!(policy["outcomes"][1]["outcome"]["parameters"], json!([]));
        let mappings = source["mappings"].as_array().unwrap();
        assert_eq!(mappings.len(), 2);
        for (mapping, lane) in mappings.iter().zip(["input", "default"]) {
            assert_eq!(
                mapping["source"],
                json!({"kind":"configuration","value":{
                    "key":{"kind":"text","value":binding["source_option"]},"source":lane,"role":"reward","value":{"kind":"text","value":"true"}
                }})
            );
            assert_eq!(mapping["outcome"]["kind"], "mapped");
            assert_eq!(
                mapping["outcome"]["value"]["target"],
                expected_owner(binding)["owner"]
            );
        }
        assert_eq!(
            policy["outcomes"][1]["outcome"]["selector"],
            mappings[0]["source"]
        );
    }
    for (i, report) in reports.iter().enumerate() {
        let (path, bytes, sha) = if i == 0 {
            ("evidence_json", "evidence_bytes", "evidence_sha256")
        } else {
            (
                "evidence_on_json",
                "evidence_on_bytes",
                "evidence_on_sha256",
            )
        };
        assert_eq!(report["path"], a["source_validation"][path]);
        assert_eq!(report["bytes"], a["source_validation"][bytes]);
        assert_eq!(report["sha256"], a["source_validation"][sha]);
        for original in 0..5 {
            assert_eq!(
                observation(report, &format!("/cases/{original}/name")),
                &json!(format!("original-{:02}", original + 1))
            );
            assert_eq!(
                observation(report, &format!("/cases/{original}/available")),
                &json!(true)
            );
        }
        for case in [4, 5, 6] {
            let enabled = case != 6;
            for binding in bindings {
                let controls = rows(observation(
                    report,
                    &format!("/cases/{case}/state/saved/controls"),
                ));
                let matches: Vec<_> = controls
                    .iter()
                    .filter(|row| row["key"] == binding["source_option"])
                    .collect();
                assert_eq!(matches.len(), 1);
                assert_eq!(matches[0]["input"], enabled);
                assert_eq!(matches[0]["input_type"], "boolean");
                for suffix in ["quest", "modes/MAIN/quest", "modes/CALCS/quest"] {
                    let quest = rows(observation(
                        report,
                        &format!("/cases/{case}/state/saved/{suffix}"),
                    ));
                    let actual: Vec<_> = quest
                        .iter()
                        .filter(|row| row["source"] == binding["source_origin"])
                        .collect();
                    if enabled {
                        assert_eq!(actual, vec![&source_record(binding)]);
                    } else {
                        assert!(actual.is_empty());
                    }
                }
            }
        }
    }
    assert_eq!(a["source_validation"]["status"], "passed");
    assert_eq!(a["source_validation"]["whole_build_parity"], false);
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    for pin in a["source_files"].as_array().unwrap() {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap();
        let normalized = text.replace("\r\n", "\n");
        assert_eq!(normalized.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(normalized.as_bytes()), pin["sha256"]);
    }
    for evidence in v["reports"].as_array().unwrap() {
        let bytes = fs::read(root().join(evidence["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, evidence["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), evidence["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report["source_hash"], a["source_manifest_sha256"]);
        assert_eq!(report["source_revision"], a["source_revision"]);
        assert_eq!(report["cases"].as_array().unwrap().len(), 30);
        assert_eq!(report["evidence"]["complete_load_attempts"], 30);
        assert_eq!(report["evidence"]["finite_callback_cases"], 39);
        assert_eq!(report["evidence"]["business_method_wrappers"], false);
        let mut pointers = BTreeSet::new();
        for row in evidence["observations"].as_array().unwrap() {
            let pointer = row["pointer"].as_str().unwrap();
            assert!(pointers.insert(pointer));
            assert_eq!(report.pointer(pointer), Some(&row["value"]));
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    assert!(prior.evaluation().is_none());
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
        "rewards",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert_eq!(receipt["rules"], d["source"]["rules"]);
    let dependencies: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    for definition in dependencies {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|row| **row == definition)
                .count(),
            1
        );
    }
    let mapping = json!(prior.input().mapping);
    let policy = json!(prior.input().rewards);
    for source in v["reward_rows"].as_array().unwrap() {
        assert_eq!(
            policy["rules"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| **row == source["policy"])
                .count(),
            1
        );
        for row in source["mappings"].as_array().unwrap() {
            assert_eq!(
                mapping["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|entry| *entry == row)
                    .count(),
                1
            );
        }
    }
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    let mut input = prior.input().clone();
    for owner in &owners {
        assert!(
            !input
                .recipe
                .rules
                .owners
                .iter()
                .any(|row| row.owner == owner.owner),
            "reviewed missing owner only"
        );
        input.recipe.rules.owners.push(owner.clone());
    }
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("source-bound-flat-resource-rewards"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-flat-resource-rewards-v1",
            &(a, b, d, c, v),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().clone();
    assert_eq!(
        restored.recipe.rules.owners.len(),
        prior.input().recipe.rules.owners.len() + owners.len()
    );
    for owner in owners {
        assert_eq!(
            restored
                .recipe
                .rules
                .owners
                .iter()
                .filter(|row| **row == owner)
                .count(),
            1
        );
        restored
            .recipe
            .rules
            .owners
            .retain(|row| row.owner != owner.owner);
    }
    assert_eq!(
        restored.provenance.len(),
        prior.input().provenance.len() + 1
    );
    restored.provenance.pop();
    assert_eq!(
        restored,
        *prior.input(),
        "no other rule, schema, registry, policy, route, query or input changed"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
