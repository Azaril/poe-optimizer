//! A source-bound empty authored relationship inventory, independent of mechanics.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::PayloadInventoryPolicy,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/empty-payload-inventory")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn check_effect(effect: &Value) -> Vec<Value> {
    let mut exclusions = Vec::new();
    for name in ["Meta", "Triggers", "Triggered", "InbuiltTrigger"] {
        for (field, prefix) in [
            ("skill_types", "skill_type"),
            ("add_skill_types", "add_skill_type"),
        ] {
            if rows(&effect[field]).iter().any(|value| value == name) {
                exclusions.push(json!(format!("{prefix}:{name}")));
            }
        }
    }
    for field in ["is_trigger", "triggered"] {
        if effect[field].as_bool().unwrap() {
            exclusions.push(json!(field));
        }
    }
    assert_eq!(rows(&effect["exclusions"]), exclusions);
    assert_eq!(effect["non_container"], exclusions.is_empty());
    exclusions
}
pub fn check_authored() {
    let authoring: Value = read("authoring.json");
    let policy: PayloadInventoryPolicy = read("policy.json");
    let bindings: Vec<Value> = read("bindings.json");
    let records: Vec<Value> = read("source-records.json");
    let policy = json!(policy);
    assert_eq!(authoring["schema_version"], 1);
    assert_eq!(authoring["allocated_definitions"], 0);
    assert_eq!(
        policy["kind"],
        "saved_groups_without_authored_containers_v1"
    );
    for field in ["mapping_source", "roles"] {
        assert_eq!(policy[field], authoring[field]);
    }
    assert_eq!(records.len(), 966, "entire pinned Gem identity catalog");
    assert_eq!(bindings.len(), records.len());
    let catalog_bytes = fs::read(root().join(authoring["catalog_file"].as_str().unwrap())).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&catalog_bytes)),
        authoring["catalog_sha256"]
    );
    let catalog: Value = serde_json::from_slice(&catalog_bytes).unwrap();
    let catalog = catalog["gems"].as_array().unwrap();
    let mut source_ids = BTreeSet::new();
    let mut owned_ids = BTreeSet::new();
    let mut admitted = Vec::new();
    for (record, binding) in records.iter().zip(&bindings) {
        assert!(source_ids.insert(record["gem_id"].as_str().unwrap()));
        assert!(owned_ids.insert(binding["gem"]["key"].as_str().unwrap()));
        assert_eq!(record["gem_id"], binding["source_gem"]);
        assert_eq!(record["game_id"], binding["game_id"]);
        assert_eq!(record["variant_id"], binding["variant_id"]);
        assert_eq!(record["non_container"], binding["non_container"]);
        let identities: Vec<_> = catalog
            .iter()
            .filter(|g| g["key"] == record["gem_id"])
            .collect();
        assert_eq!(identities.len(), 1);
        let identity = identities[0];
        assert_eq!(identity["game_id"], record["game_id"]);
        assert_eq!(identity["variant_id"], record["variant_id"]);
        assert_eq!(identity["primary_effect_id"], record["primary_effect"]);
        let effects = rows(&record["effects"]);
        let mut exclusions = Vec::new();
        assert!(!effects.is_empty());
        assert_eq!(
            identity["effect_list"],
            json!(effects.iter().map(|e| &e["id"]).collect::<Vec<_>>())
        );
        let primary: Vec<_> = effects
            .iter()
            .filter(|effect| effect["id"] == record["primary_effect"])
            .collect();
        assert_eq!(primary.len(), 1);
        assert_eq!(primary[0]["support"], record["primary_support"]);
        for effect in effects {
            for reason in check_effect(effect) {
                exclusions.push(json!(format!(
                    "{}:{}",
                    effect["id"].as_str().unwrap(),
                    reason.as_str().unwrap()
                )));
            }
        }
        assert_eq!(rows(&record["exclusions"]), exclusions);
        assert_eq!(record["non_container"], exclusions.is_empty());
        if record["non_container"] == true {
            assert_eq!(record["selector_resolves_same"], true);
            admitted.push(binding["gem"].clone());
        }
    }
    admitted.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
    assert_eq!(policy["non_container_gems"], json!(admitted));
    assert_eq!(authoring["classified_gems"], records.len());
    assert_eq!(authoring["non_container_gems"], admitted.len());
    assert_eq!(
        policy["nonphysical_non_container_skill_ids"],
        authoring["nonphysical_non_container_skill_ids"]
    );
    let direct: Vec<Value> = read("nonphysical-source-records.json");
    let mut nonphysical = Vec::new();
    for effect in direct {
        assert!(check_effect(&effect).is_empty());
        assert_eq!(effect["support"], false);
        assert_eq!(effect["gem_mapping_by_object_present"], false);
        assert_eq!(effect["gem_mapping_by_id_present"], false);
        nonphysical.push(effect["id"].clone());
    }
    assert_eq!(
        policy["nonphysical_non_container_skill_ids"],
        json!(nonphysical)
    );
}

fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(off == on, "both complete original JIT modes agree");
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let source: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(source["source_revision"], authoring["source_revision"]);
    assert_eq!(source["source_hash"], authoring["source_manifest_sha256"]);
    let records: Value = read("source-records.json");
    assert_eq!(
        source["catalog"], records,
        "entire live catalog classification"
    );
    assert_eq!(
        source["nonphysical_catalog"],
        read::<Value>("nonphysical-source-records.json")
    );
    for field in [
        "business_method_wrappers",
        "native_coverage",
        "whole_build_parity",
    ] {
        assert_eq!(source["evidence"][field], false);
    }
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(
            source["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|actual| {
                    actual["path"] == pin["path"] && actual["sha256"] == pin["sha256"]
                })
        );
    }
    let originals = source["evidence"]["originals"].as_array().unwrap();
    assert_eq!(originals.len(), 5);
    for (index, original) in originals.iter().enumerate() {
        assert_eq!(original["name"], format!("build-{:02}.xml", index + 1));
        let bytes = fs::read(
            root()
                .join("tests/fixtures/builds/breadth-20260908")
                .join(original["name"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), original["sha256"]);
    }
    assert_eq!(source["evidence"]["case_count"], proof["cases_per_jit"]);
    assert_eq!(
        source["evidence"]["complete_load_attempts_per_jit"],
        proof["complete_load_attempts_per_jit"]
    );
    let cases = source["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, proof["cases_per_jit"].as_u64().unwrap());
    for case in cases {
        assert_eq!(case["available"], true, "{}", case["name"]);
        for flag in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "saved_specs_preserved",
            "cached_outputs_preserved",
        ] {
            assert_eq!(case["state"][flag], true, "{}: {flag}", case["name"]);
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    for field in [
        "input",
        "definitions",
        "registry",
        "mapping",
        "roles",
        "normalization",
        "tree",
    ] {
        assert_eq!(
            receipt[field], authoring[field],
            "exact predecessor {field}"
        );
    }
    assert_eq!(
        json!(prior.mapping().source_identity()),
        authoring["mapping_source"]
    );
    source_proof(&authoring);
    let bindings: Vec<Value> = read("bindings.json");
    let mapping = json!(prior.input().mapping);
    for binding in &bindings {
        let entries: Vec<_> = mapping["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| {
                let selector = &entry["source"]["value"];
                selector["kind"] == "gem"
                    && selector["value"]["game_id"]
                        == json!({"kind":"text","value":binding["game_id"]})
                    && selector["value"]["variant_id"]
                        == json!({"kind":"text","value":binding["variant_id"]})
            })
            .collect();
        assert_eq!(entries.len(), 1, "one exact owned Gem selector");
        assert_eq!(entries[0]["outcome"]["kind"], "mapped");
        assert_eq!(
            entries[0]["outcome"]["value"]["target"]["value"]["value"],
            binding["gem"]
        );
    }
    assert!(prior.normalization().payload_inventory.is_none());
    let policy: PayloadInventoryPolicy = read("policy.json");
    let mut full = prior.input().clone();
    full.normalization.payload_inventory = Some(policy.clone());
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            prior.assembled().registry(),
            prior.assembled().schema(),
            prior.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("empty-authored-payload-inventory").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-empty-payload-inventory-v1",
            &(
                authoring,
                policy,
                bindings,
                read::<Value>("source-records.json"),
                read::<Value>("nonphysical-source-records.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization.payload_inventory = None;
    restored.tree = prior.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(&restored, prior.input(), "normalization-only publication");
    next
}
