//! Retire the local early-Amulet consumer obligation, preserving every program.
#[path = "owned_amulet_copy_consumer_evidence.rs"]
mod evidence;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "source-bound-amulet-copy-consumer";
const REMOVED: &str = "amulet-bonus-copy-unconverted";
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/amulet-copy-consumer")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn retained_owner(d: &Value) -> Value {
    let mut parts = Vec::new();
    for (key, family, name, field) in [
        (
            "owner_artifact",
            "ordinary-item-routing",
            "routing-authoring",
            "owner",
        ),
        (
            "closure_artifact",
            "minion-level-scalability",
            "coverage",
            "after",
        ),
    ] {
        let pin = &d[key];
        let path = format!("data/owned/poe2/3887ae68/{family}/{name}.json");
        assert_eq!(pin["path"], path);
        assert_eq!(pin["field"], field);
        let bytes = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(&bytes)));
        let artifact: Value = serde_json::from_slice(&bytes).unwrap();
        parts.push(artifact[field].clone());
    }
    let mut owner = parts.remove(0);
    owner["programs"]["closure"] = parts.remove(0);
    owner
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = [
        "authoring",
        "bindings",
        "dependencies",
        "coverage",
        "source-vectors",
    ]
    .map(|name| read(&format!("{name}.json")))
    .into();
    digest_owned("owned-amulet-copy-consumer-v1", &values, 8 * 1024 * 1024).unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("coverage.json");
    let owner = retained_owner(&d);
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(
            a[field],
            d["source"][if field == "before" { "input" } else { field }]
        );
    }
    assert_eq!(b["modifier"]["key"], "def.00000000000030ca");
    assert_eq!(b["retired_code"], REMOVED);
    assert_eq!(b["registry_last_issued"], 0x3352);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(c["owner"], owner["owner"]);
    assert_eq!(c["before"], owner["programs"]["closure"]);
    assert_eq!(c["before"]["kind"], "partial");
    assert_eq!(c["after"]["kind"], "partial");
    let before = c["before"]["value"]["gaps"].as_array().unwrap();
    let after = c["after"]["value"]["gaps"].as_array().unwrap();
    assert_eq!((before.len(), after.len()), (6, 5));
    assert_eq!(before.iter().filter(|g| g["code"] == REMOVED).count(), 1);
    assert_eq!(
        before.iter().find(|g| g["code"] == REMOVED),
        Some(&c["retired"])
    );
    assert_eq!(
        after,
        &before
            .iter()
            .filter(|g| g["code"] != REMOVED)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        after
            .iter()
            .map(|g| g["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "source-encoding-and-corrupted-range-unproved",
            "remaining-ordered-magnitude-transforms-unconverted",
            "canonical-input-admission-unproved",
            "ordinary-item-routing-unconverted",
            "external-contributor-membership-unconverted",
        ]
    );
    assert_eq!(owner["programs"]["members"].as_array().unwrap().len(), 6);
    for field in [
        "new_definitions",
        "new_programs",
        "numerical_program_changes",
        "import_guard_changes",
        "closed_existing_rule_owners",
    ] {
        assert_eq!(b["scope"][field], 0);
    }
    for field in [
        "source_admission_widened",
        "whole_build_parity",
        "external_membership_completed",
    ] {
        assert_eq!(b["scope"][field], false);
    }
    let assets = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "coverage", "dependencies", "source-vectors"]
    );
    for (name, hash) in assets {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*hash, format!("{:x}", Sha256::digest(bytes)));
    }
    evidence::check(&read("source-vectors.json"), false);
}
fn expected_owner(after: bool) -> DefinitionRules {
    let d: Value = read("dependencies.json");
    let c: Value = read("coverage.json");
    let mut owner: DefinitionRules = serde_json::from_value(retained_owner(&d)).unwrap();
    if after {
        owner.programs.closure = serde_json::from_value(c["after"].clone()).unwrap();
    }
    owner
}
fn assert_owner(endpoint: &StagedOwnedRelease, expected: &DefinitionRules) {
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| *o == expected)
            .count(),
        1
    );
    evidence::assert_dependencies(endpoint);
}
/// Only the authenticated closure refinement may change the retained owner.
/// This is test/publication evidence, not a production compatibility reader.
#[allow(dead_code)]
pub fn retained_modifier_owner(
    endpoint: &StagedOwnedRelease,
    before: DefinitionRules,
) -> DefinitionRules {
    check_authored();
    assert_eq!(before, expected_owner(false));
    let a: Value = read("authoring.json");
    let proofs: Vec<_> = endpoint
        .receipt()
        .provenance
        .iter()
        .filter(|p| p.kind.as_str() == KIND)
        .collect();
    assert_eq!(proofs.len(), 1);
    assert_eq!(json!(proofs[0].prior_input), a["before"]);
    assert_eq!(proofs[0].authoring_input, digest());
    let after = expected_owner(true);
    assert_owner(endpoint, &after);
    after
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    evidence::check(&read("source-vectors.json"), true);
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    for field in [
        "input",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(
            receipt[field],
            a[if field == "input" { "before" } else { field }]
        );
    }
    let old = expected_owner(false);
    assert_owner(prior, &old);
    assert!(prior.evaluation().is_none());
    let new = expected_owner(true);
    assert_eq!(old.programs.members, new.programs.members);
    let mut input = prior.input().clone();
    *input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == old.owner)
        .unwrap() = new;
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    retained_modifier_owner(&next, old.clone());
    let mut inverse = next.input().clone();
    let restored = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == old.owner)
        .unwrap();
    *restored = old;
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "only one exact obligation and provenance change"
    );
    let after = json!(next.receipt());
    for field in [
        "definitions",
        "registry",
        "routing",
        "mapping",
        "roles",
        "normalization",
        "rewards",
        "items",
        "item_source",
        "tree",
        "query_sets",
        "query_rows",
        "query_policy_bytes",
    ] {
        assert_eq!(receipt[field], after[field], "unchanged {field}");
    }
    assert_ne!(prior.receipt().input, next.receipt().input);
    assert_ne!(prior.receipt().rules, next.receipt().rules);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
