//! Physical input inventory publication, separate from unresolved usage semantics.
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_import::{
    owned_normalize::{GemInventoryPolicy, PrimarySkillGemInventory, usage_inputs_identity},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/active-gem-inventory")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn inventory() -> Vec<PrimarySkillGemInventory> {
    read("inventory.json")
}
pub fn check_authored() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let authoring: Value = read("authoring.json");
    let manifest =
        fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&manifest)),
        authoring["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(manifest["upstream_revision"], authoring["source_revision"]);
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(fs::read(data().join("inventory.json")).unwrap())
        ),
        authoring["inventory_sha256"]
    );
    let catalog: Value = serde_json::from_slice(
        &fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap();
    let checked_catalog = poe_optimizer_data::skill_identities::SkillIdentityCatalog::new(
        serde_json::from_value(catalog.clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        json!(
            digest_owned(
                "owned-skill-source-catalog-v1",
                checked_catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        ),
        authoring["catalog"]
    );
    for row in inventory() {
        let row = row.physical;
        let matching: Vec<_> = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|gem| gem["game_id"] == row.game_id && gem["variant_id"] == row.variant_id)
            .collect();
        assert_eq!(matching.len(), 1);
        let gem = matching[0];
        assert_eq!(gem["primary_effect_id"], row.skill_id);
        assert_eq!(gem["effect_list"], json!([row.skill_id]));
        for field in [
            "declared_additional_effects",
            "declared_additional_stat_sets",
            "constructed_additional_effects",
            "additional_effects",
        ] {
            assert_eq!(gem[field], json!([]), "finite single-primary source domain");
        }
        let skill = catalog["skills"]
            .as_array()
            .unwrap()
            .iter()
            .find(|skill| skill["id"] == row.skill_id)
            .unwrap();
        assert_ne!(skill["support"], true);
        assert_ne!(skill["from_tree"], true);
    }
    for field in ["new_definitions", "new_programs", "new_scalar_values"] {
        assert_eq!(authoring[field], 0);
    }
    assert_eq!(authoring["usage_inventory"], "pending");
}

fn source_proof(authoring: &Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root.join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root.join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(off == on, "complete observations agree in both JIT modes");
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(
        evidence["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(evidence["native_inventory_authority"], false);
    assert_eq!(evidence["native_build_parity"], false);
    let xml = fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    assert_eq!(
        evidence["original_xml_sha256"],
        format!("{:x}", Sha256::digest(xml))
    );
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap(),
    )
    .unwrap();
    for pin in evidence["files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    assert_eq!(
        evidence["controls"].as_array().unwrap().len() as u64,
        proof["controls"].as_u64().unwrap()
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    source_proof(&authoring);
    let receipt = json!(prior.receipt());
    assert_eq!(authoring["before"], receipt["input"]);
    for field in ["definitions", "registry", "normalization", "roles"] {
        assert_eq!(
            authoring[field], receipt[field],
            "exact predecessor {field}"
        );
    }
    let b = prior.input();
    let Some(GemInventoryPolicy::PobFreshSingleSupportV1 {
        definitions,
        roles,
        catalog,
        scalar_inputs,
        gems,
    }) = &b.normalization.gem_inventory
    else {
        panic!("exact prior support inventory");
    };
    assert_eq!(json!(catalog), authoring["catalog"]);
    let mut normalization = b.normalization.clone();
    normalization.gem_inventory = Some(GemInventoryPolicy::PobFreshPhysicalV2 {
        definitions: definitions.clone(),
        roles: *roles,
        catalog: *catalog,
        scalar_inputs: *scalar_inputs,
        usage_inputs: usage_inputs_identity(&normalization, Default::default()).unwrap(),
        supports: gems.clone(),
        primary_skills: inventory(),
    });
    let transition = transition_owned_normalization_with_tree_compact(
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
        b.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("active-physical-inventory-with-pending-usage").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-active-physical-inventory-v1",
            &(authoring, inventory()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization = b.normalization.clone();
    restored.tree = b.tree.clone();
    restored.provenance.pop();
    assert!(
        restored == *b,
        "only physical inventory proof, tree commitment and provenance change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    next
}
