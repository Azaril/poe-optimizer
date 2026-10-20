//! Publish direct Gem declarations while preserving supplied Skill obligations.
#[allow(dead_code)]
#[path = "support/owned_gem_direct_inventories.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
#[path = "support/owned_structural_inventory_publication.rs"]
mod structural;

use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey,
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease},
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

const BEFORE: &str = "420b4a071361ba61b95c6cb92a84ecd82ca36c2ba733c2bab8086647caeec3be";
const KIND: &str = "reviewed-gem-direct-inventories";
const CLOSED: [&str; 3] = ["grants", "actors", "skill_grants"];
const OWNERS: [&str; 4] = [
    "def.00000000000006c5",
    "def.0000000000000797",
    "def.00000000000007dd",
    "def.000000000000086b",
];

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/gem-direct-inventories")
}

fn check_descriptor(old: &DefinitionDescriptor, next: &DefinitionDescriptor) {
    assert!(matches!(old, DefinitionDescriptor::Gem(_)));
    assert_eq!(old.address(), next.address());
    let before = json!(old);
    let mut restored = json!(next);
    for field in CLOSED {
        let path = format!("/value/schema/value/declarations/{field}");
        let prior = before.pointer(&path).expect("known Gem declaration");
        assert_eq!(prior["closure"]["kind"], "partial");
        let replacement = restored.pointer_mut(&path).expect("preserved declaration");
        assert_eq!(
            *replacement,
            json!({"members":prior["members"],"closure":{"kind":"complete"}}),
            "every existing direct grant and supply must survive exactly"
        );
        replacement["closure"] = prior["closure"].clone();
    }
    assert_eq!(
        restored, before,
        "only the three reviewed direct closure fields may change"
    );
}

fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let migration = family::checked_migration();
    assert_eq!(migration.before.to_string(), BEFORE);
    let originals = family::prior_descriptors();
    let mut keys: Vec<_> = originals
        .iter()
        .map(|value| value.address().key().as_str().to_owned())
        .collect();
    keys.sort();
    assert_eq!(keys, OWNERS);
    let authoring: Value =
        serde_json::from_slice(&fs::read(data().join("authoring.json")).unwrap()).unwrap();
    let authoring_input = digest_owned(KIND, &(&authoring, &migration), 2 * 1024 * 1024).unwrap();
    structural::stage(
        prior,
        migration,
        originals,
        check_descriptor,
        OwnedReleaseProvenance {
            kind: OwnedDefinitionKey::new(KIND).unwrap(),
            prior_input: prior.receipt().input,
            authoring_input,
        },
    )
}

#[test]
fn four_gems_preserve_primary_chains_and_empty_intrinsic_support_declarations() {
    let migration = family::checked_migration();
    let originals = family::prior_descriptors();
    assert_eq!(originals.len(), OWNERS.len());
    assert_eq!(migration.schema.len(), originals.len());
    let mut inventory_counts = Vec::new();
    for old in &originals {
        let next = migration
            .schema
            .iter()
            .find_map(|row| match row {
                SchemaExtensionEntry::Definition(value) if value.address() == old.address() => {
                    Some(value)
                }
                _ => None,
            })
            .unwrap();
        check_descriptor(old, next);
        let old = json!(old);
        inventory_counts.push(CLOSED.map(|field| {
            old["value"]["schema"]["value"]["declarations"][field]["members"]
                .as_array()
                .unwrap()
                .len()
        }));
    }
    inventory_counts.sort();
    assert_eq!(
        inventory_counts,
        [[0, 0, 0], [1, 0, 1], [1, 0, 1], [1, 0, 1]],
        "preserve three primary chains without importing descendant or receiving actors"
    );
}

#[test]
fn publication_inverse_refuses_erasing_a_primary_grant_or_potential_skill() {
    let migration = family::checked_migration();
    let originals = family::prior_descriptors();
    let old = originals
        .iter()
        .find(|value| value.address().key().as_str() == OWNERS[1])
        .unwrap();
    let next = migration
        .schema
        .iter()
        .find_map(|row| match row {
            SchemaExtensionEntry::Definition(value) if value.address() == old.address() => {
                Some(value)
            }
            _ => None,
        })
        .unwrap();
    for path in [
        "/value/schema/value/declarations/grants/members",
        "/value/schema/value/skills/members",
    ] {
        let mut erased = json!(next);
        let members = erased.pointer_mut(path).expect("existing positive members");
        assert!(!members.as_array().unwrap().is_empty());
        *members = json!([]);
        let erased: DefinitionDescriptor = serde_json::from_value(erased).unwrap();
        assert!(std::panic::catch_unwind(|| check_descriptor(old, &erased)).is_err());
    }
}

#[test]
#[ignore = "explicit immutable predecessor and output; executes all five unchanged imports"]
fn publish_gem_direct_inventories_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        std::env::var_os("POE_OPTIMIZER_TEST_GEM_DIRECT_INVENTORIES_PRIOR")
            .expect("checked current predecessor")
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_GEM_DIRECT_INVENTORIES_OUTPUT")
            .expect("fresh output")
            .into(),
        &data(),
        &[],
        &["authoring.json", "evidence.json"],
        stage,
        json!({
            "new_definitions":0,
            "new_programs":0,
            "structural_declaration_refinement":true,
            "closed_intrinsic_declarations":12,
            "gem_definitions":4,
            "preserved_primary_chains":3,
            "primary_grant_chains_preserved":true,
            "receiving_ground_obligations_preserved":true,
            "potential_skill_inventory_unchanged":true,
            "descendant_topology_unchanged":true,
            "numerical_coverage_unchanged":true,
            "scenario_intent_inventory_unchanged":true
        }),
        [107, 118, 110, 124, 5],
    );
}
