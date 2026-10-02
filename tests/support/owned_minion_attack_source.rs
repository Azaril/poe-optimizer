//! Offline assembly of injected intrinsic minion weapon programs and action routes.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey,
    owned_routing::ActionOutputRoutes, owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/minion-attack-source")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn check_authored() {
    let e: OwnedRecipeExtension = read("extension.json");
    let routes: Vec<ActionOutputRoutes> = read("routes.json");
    assert_eq!(e.schema.len(), 7);
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].routes.members.len(), 4);
    assert!(
        !routes[0].routes.is_complete(),
        "intrinsic source is not complete action coverage"
    );
}
fn source_proof(a: &Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = &a["source_validation"];
    assert_eq!(p["status"], "passed");
    let off = fs::read(root.join(p["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root.join(p["evidence_on_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        off, on,
        "both JIT modes have exact fresh-actor source evidence"
    );
    assert_eq!(off.len() as u64, p["evidence_bytes"].as_u64().unwrap());
    assert_eq!(format!("{:x}", Sha256::digest(&off)), p["evidence_sha256"]);
    let evidence: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(evidence["source_revision"], a["source_revision"]);
    assert_eq!(evidence["source_hash"], a["source_manifest_sha256"]);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    source_proof(&a);
    assert_eq!(json!(prior.receipt().input), a["before"]);
    let dependencies: Vec<DefinitionDescriptor> = read("dependencies.json");
    for definition in dependencies {
        assert!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .contains(&definition),
            "exact injected input definition"
        );
    }
    // The finite test world may complete its own inventories, but its starting
    // numerical programs, tables and definitions must be the published ones.
    let snapshot: Value =
        serde_json::from_slice(
            &fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "crates/poe-optimizer-engine/tests/support/minion_attack_source_snapshot.json",
            ))
            .unwrap(),
        )
        .unwrap();
    let recipe = json!(prior.input().recipe);
    for (field, actual) in [
        ("schema_definitions", &recipe["schema"]["definitions"]),
        ("schema_slots", &recipe["schema"]["slots"]),
        ("owners", &recipe["rules"]["owners"]),
        ("tables", &recipe["rules"]["tables"]),
    ] {
        for row in snapshot[field].as_array().unwrap() {
            assert!(
                actual.as_array().unwrap().contains(row),
                "finite fixture retains exact published {field}"
            );
        }
    }
    let e: OwnedRecipeExtension = read("extension.json");
    let mut extended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert!(extended.refinement.is_none());
    assert_eq!(extended.receipt.allocated_entries, 7);
    let routes: Vec<ActionOutputRoutes> = read("routes.json");
    for added in &routes {
        let target = extended
            .successor
            .routing
            .outputs
            .iter_mut()
            .find(|o| o.output == added.output)
            .unwrap();
        assert_eq!(target.routes.closure, added.routes.closure);
        assert_eq!(target.source_selectors, added.source_selectors);
        assert!(
            target.routes.members.is_empty(),
            "exact unrouted predecessor output"
        );
        target.routes.members.extend(added.routes.members.clone());
    }
    let b = prior.input();
    let carried = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: extended.successor,
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
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
        kind: OwnedDefinitionKey::new("intrinsic-minion-attack-source").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-intrinsic-minion-attack-source-v1",
            &(a, e, routes),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    assemble_owned_release(full, Default::default()).unwrap()
}
