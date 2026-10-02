//! Source-bound nonphysical rows affect only physical support inventory.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::SupportOriginOrderPolicy,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/nonphysical-skill-inventory")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn check_authored() {
    let authoring: Value = read("authoring.json");
    let policy: SupportOriginOrderPolicy = read("policy.json");
    assert_eq!(authoring["schema_version"], 1);
    assert_eq!(authoring["allocated_definitions"], 0);
    assert_eq!(json!(policy)["mapping_source"], authoring["mapping_source"]);
    assert_eq!(json!(policy)["roles"], authoring["roles"]);
    assert_eq!(
        json!(policy)["kind"],
        "saved_manual_group_order_with_nonphysical_skill_inventory_v3"
    );
    assert_eq!(
        json!(policy)["nonphysical_skill_ids"],
        authoring["nonphysical_skill_ids"]
    );
    assert_eq!(authoring["nonphysical_skill_ids"], json!(["EnemyExplode"]));
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(
        off == on,
        "exact complete-source evidence in both JIT modes"
    );
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let source: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(source["source_revision"], authoring["source_revision"]);
    assert_eq!(source["source_hash"], authoring["source_manifest_sha256"]);
    for flag in [
        "business_method_wrappers",
        "synthetic_role_injection",
        "native_coverage",
        "whole_build_parity",
    ] {
        assert_eq!(source["evidence"][flag], false, "source scope {flag}");
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
    for (i, original) in originals.iter().enumerate() {
        assert_eq!(original["name"], format!("build-{:02}.xml", i + 1));
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
    assert_eq!(cases.len(), 23);
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
    let old = prior.normalization().support_origin_order.as_ref().unwrap();
    assert!(matches!(
        old,
        SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 { .. }
    ));
    let policy: SupportOriginOrderPolicy = read("policy.json");
    for field in ["mapping_source", "roles"] {
        assert_eq!(json!(old)[field], json!(policy)[field]);
    }
    let mut full = prior.input().clone();
    full.normalization.support_origin_order = Some(policy.clone());
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
        kind: OwnedDefinitionKey::new("nonphysical-skill-support-inventory").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-nonphysical-skill-inventory-v1",
            &(authoring, policy),
            1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization.support_origin_order =
        prior.normalization().support_origin_order.clone();
    restored.tree = prior.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(&restored, prior.input(), "no unrelated release changes");
    next
}
