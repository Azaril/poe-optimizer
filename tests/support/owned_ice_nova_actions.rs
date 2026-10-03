//! Offline checked correspondence publication; no numerical coverage is inferred.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_source_actions::SourceActionCorrespondenceInput,
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/ice-nova-actions")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn check_authored() {
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let c: Value = read("correspondence.json");
    let a: Value = read("authoring.json");
    assert_eq!(m.schema_version, 3);
    assert_eq!(m.contract.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(c["stat_sets"].as_array().unwrap().len(), 2);
    assert_eq!(c["mappings"].as_array().unwrap().len(), 5);
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(manifest_bytes))
    );
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    for (name, hash) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*hash, json!(format!("{:x}", Sha256::digest(bytes))));
    }
    assert_eq!(c["before"], a["before"]);
    assert_eq!(c["catalog"], a["catalog"]);
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let c: Value = read("correspondence.json");
    let proof = &a["source_validation"];
    assert_eq!(proof["status"], "passed");
    let mut reports = vec![];
    for (path, size, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let bytes = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, proof[size].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), proof[hash]);
        reports.push(bytes);
    }
    assert!(reports[0] == reports[1]);
    let r: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(r["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(r["catalog_digest"], a["catalog"]);
    assert_eq!(r["native_action_identity"], false);
    assert_eq!(r["native_parity"], false);
    assert_eq!(r["baseline"], r["repeat"]);
    assert_eq!(r["cases"].as_array().unwrap().len(), 13);
    for pin in a["source_files"].as_array().unwrap() {
        assert_eq!(
            r["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let source = &c["source_identity"];
    let rows: Vec<_> = r["baseline"]["constructed"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["effect"] == source["primary_effect_id"])
        .collect();
    assert_eq!(rows.len(), 1);
    let row = rows[0];
    assert_eq!(row["physical_id"], source["game_id"]);
    assert_eq!(row["primary_same_object"], true);
    assert_eq!(row["has_parts"], false);
    assert_eq!(row["has_global_effect"], false);
    assert_eq!(
        row["stat_sets"].as_array().unwrap().len(),
        c["stat_sets"].as_array().unwrap().len()
    );
    for set in c["stat_sets"].as_array().unwrap() {
        let observed: Vec<_> = row["stat_sets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["index"] == set["source_index"])
            .collect();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0]["label"], set["label"]);
        assert_eq!(
            observed[0]["stat_description_scope"],
            set["stat_description_scope"]
        );
    }
    assert!(
        row["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["standalone_skill"] == false && v["in_effect_list"] == false)
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    let c: Value = read("correspondence.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "normalization", "roles"] {
        assert_eq!(receipt[field], a[field]);
    }
    let dependencies: Value =
        serde_json::from_slice(
            &fs::read(root().join(
                "crates/poe-optimizer-engine/tests/support/ice_nova_physical_dependencies.json",
            ))
            .unwrap(),
        )
        .unwrap();
    assert_eq!(dependencies["source"]["input"], receipt["input"]);
    assert_eq!(
        dependencies["source"]["definitions"],
        receipt["definitions"]
    );
    let schema = &prior.input().recipe.schema;
    let definitions: Vec<poe_optimizer_core::owned_schema::DefinitionDescriptor> =
        serde_json::from_value(dependencies["definitions"].clone()).unwrap();
    for row in definitions {
        assert_eq!(
            schema
                .definitions
                .iter()
                .filter(|candidate| **candidate == row)
                .count(),
            1,
            "exact typed predecessor definition"
        );
    }
    let slots: Vec<poe_optimizer_core::owned_schema::SlotDescriptor> =
        serde_json::from_value(dependencies["slots"].clone()).unwrap();
    for row in slots {
        assert_eq!(
            schema
                .slots
                .iter()
                .filter(|candidate| **candidate == row)
                .count(),
            1,
            "exact typed predecessor slot"
        );
    }
    let migrated =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let b = migrated.input();
    let carried = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: b.recipe.clone(),
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        CatalogAppend {
            mappings: serde_json::from_value(c["mappings"].clone()).unwrap(),
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = carried.tree().map(|t| t.input().clone());
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("ice-nova-action-correspondence"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-ice-nova-action-correspondence-v1",
            &(a, c),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.input().query_sets, prior.input().query_sets);
    assert!(next.evaluation().is_none());
    assert_eq!(
        next.input().recipe.rules.effect_applications,
        prior.input().recipe.rules.effect_applications
    );
    assert_eq!(next.input().recipe.registry.last_issued.get(), 0x3284);
    assert_eq!(
        next.input().recipe.registry.entries.len(),
        prior.input().recipe.registry.entries.len() + 7
    );
    assert_eq!(
        next.input().mapping.entries.len(),
        prior.input().mapping.entries.len() + 5
    );
    next
}

pub fn adapter(release: &StagedOwnedRelease) -> SourceActionCorrespondenceInput {
    let c: Value = read("correspondence.json");
    let source = &c["source_identity"];
    serde_json::from_value(json!({
        "kind":"pob_physical_primary_stat_sets_v1",
        "definitions":release.receipt().definitions,"source":release.roles().input().compilation.source,
        "roles":release.receipt().roles,"catalog":c["catalog"],
        "gem":c["physical_gem"],"game_id":source["game_id"],"variant_id":source["variant_id"],"skill_id":source["primary_effect_id"],"name_spec":source["name_spec"],
        "primary":c["primary_skill"],"primary_supply":c["primary_supply"],"entering_grant":c["entering_grant"],"output":c["output"],"part":c["part"],"mode":c["mode"],
        "stat_sets":c["stat_sets"].as_array().unwrap().iter().map(|v|json!({"source_index":v["source_index"],"stat_set":v["stat_set"]})).collect::<Vec<_>>(),
        "absent_stat_set":c["source_decoding"]["absent_stat_set"],"index":c["source_decoding"]["index"]
    })).unwrap()
}
