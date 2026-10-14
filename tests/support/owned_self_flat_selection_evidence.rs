//! Offline selected-input evidence, not native source-completion authority.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
pub const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
pub const ARTIFACT: &str = "data/owned/poe2/3887ae68/self-flat-attack-damage/selected-domain.json";
const CENSUS: &str = "data/owned/poe2/3887ae68/self-flat-attack-damage/source-evidence.json";
const REVIEWED: &str = "abc764c4728687f465ffea2ea8b16e597e2a91121ee7e2e37a5bbe7d2c90e213";
fn root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if p.join("data/owned").is_dir() {
        p
    } else {
        p.join("../..")
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn dependencies() -> Value {
    json!([{"path":CENSUS,"sha256":"2a07ddefddd45cdd01a8593c506412428997eff3c618ba2fc365b6e0d93c5ba8"}])
}
fn census() -> Value {
    let bytes = std::fs::read(root().join(CENSUS)).unwrap();
    assert_eq!(hash(&bytes), dependencies()[0]["sha256"]);
    serde_json::from_slice(&bytes).unwrap()
}
pub fn scope() -> Value {
    json!({"selected_recipe_certificate":true,"universal_absence":false,"native_coverage_authorized":false,
    "input_obligations_retired":false,"configuration_authority_added":false,"positive_item_obtainability_proof":false,
    "requires_exact_selected_source_inventory":true,"unobserved_dynamic_flags_inferred_false":false,
    "callbacks_executed":false,"configuration_settings_applied":false,"supplied_actor_buffs_applied":false})
}
pub fn observed() -> Value {
    let census = census();
    let reference = &census["selected_pass_corroboration"];
    let bytes = std::fs::read(root().join(reference["path"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&bytes), reference["sha256"]);
    assert_eq!(bytes.len(), reference["bytes"]);
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    let case = &report["cases"][0];
    let xml = std::fs::read(root().join(XML)).unwrap();
    assert_eq!(case["xml_sha256"], hash(&xml));
    let main = &case["state"]["main"];
    let actors: Vec<_> = main["actors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["summon_effect_id"] == "SummonSkeletalSnipersPlayer")
        .collect();
    assert_eq!(actors.len(), 1);
    let actor = actors[0];
    let children: Vec<_> = actor["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["effect_id"] == "MinionMeleeBow")
        .collect();
    assert_eq!(children.len(), 1);
    let child = children[0];
    assert_eq!(child["passes"].as_array().unwrap().len(), 1);
    let inputs = &child["passes"][0]["inputs"];
    let skills:Vec<_>=main["skills"].as_array().unwrap().iter().map(|s|json!({"ordinal":s["ordinal"],"effect_id":s["effect_id"],"source_occurrence":s["source_occurrence"],"flags":s["flags"],"buffs":s["buffs"]})).collect();
    let supplied_actors:Vec<_>=main["actors"].as_array().unwrap().iter().map(|a|json!({"ordinal":a["ordinal"],"profile":a["actor_profile"],"summon_effect_id":a["summon_effect_id"],"source_occurrence":a["source_occurrence"],"children":a["children"].as_array().unwrap().iter().map(|c|json!({"effect_id":c["effect_id"],"source_occurrence":c["source_occurrence"]})).collect::<Vec<_>>()})).collect();
    assert_eq!(skills.len(), 16);
    assert!(skills.iter().all(|s| s["effect_id"] != "RallyingCryPlayer"));
    assert_eq!(
        inputs["raw_player_minion_modifiers"]
            .as_array()
            .unwrap()
            .len(),
        30
    );
    for field in [
        "original_functions_preserved",
        "loaded_state_preserved",
        "saved_specs_preserved",
        "query_state_preserved",
        "cached_outputs_preserved",
    ] {
        assert_eq!(case["state"][field], true);
    }
    json!({"report":reference,"xml_sha256":case["xml_sha256"],"selected":case["state"]["selected"],"active_effects":skills,"supplied_actors":supplied_actors,
        "minion_source_occurrence":actor["source_occurrence"],"raw_player_minion_modifiers":inputs["raw_player_minion_modifiers"],
        "cfg":child["base_calls"][0]["preconversion"]["cfg"],"actual_minimum":inputs["bases"]["Physical"]["minimum"],"actual_maximum":inputs["bases"]["Physical"]["maximum"],
        "custom_blocks":case["state"]["config"]["custom_blocks"]})
}
fn named_records<'a>(v: &'a Value, name: &str, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(m) => {
            if m.get("name").and_then(Value::as_str) == Some(name) && m.contains_key("type") {
                out.push(v)
            }
            for child in m.values() {
                named_records(child, name, out)
            }
        }
        Value::Array(a) => {
            for child in a {
                named_records(child, name, out)
            }
        }
        _ => {}
    }
}
pub fn has_record(v: &Value, name: &str) -> bool {
    let mut found = Vec::new();
    named_records(v, name, &mut found);
    !found.is_empty()
}
pub fn fingerprint(v: &Value) -> String {
    hash(&serde_json::to_vec(v).unwrap())
}
pub fn check_semantics(v: &Value) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["kind"], "bounded-selected-self-flat-source-recipes");
    assert_eq!(v["scope"], scope());
    assert_eq!(v["dependencies"], dependencies());
    assert_eq!(v["observed"], observed());
    for name in ["GainMainHandDmgFromParent", "UseHollowPalmDamage"] {
        let mut found = Vec::new();
        for field in [
            "items",
            "tree",
            "configuration",
            "skills",
            "minion_profiles",
        ] {
            named_records(&v[field], name, &mut found);
        }
        assert!(
            found.is_empty(),
            "selected recipe supplies unreviewed dynamic gate {name}"
        );
    }
    let basic: Vec<_> = v["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["id"] == "MinionMeleeBow")
        .collect();
    assert_eq!(basic.len(), 1);
    assert!(basic[0]["metadata"]["weaponTypes"].get("Staff").is_none());
    let party = &v["inputs"]["party"];
    assert_eq!(party["name"], "Party");
    assert_eq!(party["children"], json!([]));
    assert_eq!(party["text"], "");
    assert_eq!(
        fingerprint(v),
        REVIEWED,
        "selected recipe/input scope changed; no implicit source admission"
    );
}
pub fn check(v: &Value) {
    check_semantics(v);
    let prior = census();
    assert_eq!(v["source_files_sha256"], prior["source_files_sha256"]);
    assert_eq!(v["source_revision"], prior["upstream_revision"]);
    assert_eq!(
        v["xml_sha256"],
        hash(&std::fs::read(root().join(XML)).unwrap())
    );
}

#[test]
fn selected_certificate_does_not_complete_owners_or_erase_inputs() {
    let original: Value =
        serde_json::from_slice(&std::fs::read(root().join(ARTIFACT)).unwrap()).unwrap();
    check(&original);
    for edit in 0..6 {
        let mut v = original.clone();
        match edit {
            0 => v["scope"]["native_coverage_authorized"] = json!(true),
            1 => {
                v["inputs"]["party"]["children"] =
                    json!([{"name":"ImportedBuffs","text":"0|Source|PhysicalMin|BASE|-|-|"}])
            }
            2 => v["items"][0]["uses"].as_array_mut().unwrap().clear(),
            3 => v["observed"]["raw_player_minion_modifiers"]
                .as_array_mut()
                .unwrap()
                .clear(),
            4 => v["configuration"].as_array_mut().unwrap().clear(),
            5 => v["original_skill_module_order"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| check_semantics(&v)).is_err());
    }
}
