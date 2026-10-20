//! Publish potential Gem Skills while retaining direct and descendant coverage gaps.
#[allow(dead_code)]
#[path = "support/owned_gem_skill_inventories.rs"]
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

const BEFORE: &str = "520832751a5c6650954aa23c70f0b7a7b3ca5cc47c5cc39b8bd62277ee7f41e6";
const KIND: &str = "reviewed-gem-skill-inventories";
const OWNERS: [&str; 4] = [
    "def.00000000000006c5",
    "def.0000000000000797",
    "def.00000000000007dd",
    "def.000000000000086b",
];

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/gem-skill-inventories")
}

fn check_descriptor(old: &DefinitionDescriptor, next: &DefinitionDescriptor) {
    assert!(matches!(old, DefinitionDescriptor::Gem(_)));
    assert_eq!(old.address(), next.address());
    let before = json!(old);
    let mut restored = json!(next);
    let path = "/value/schema/value/skills";
    let prior = before.pointer(path).expect("known Gem Skill inventory");
    assert_eq!(prior["closure"]["kind"], "partial");
    let replacement = restored.pointer_mut(path).expect("preserved inventory");
    assert_eq!(
        *replacement,
        json!({"members":prior["members"],"closure":{"kind":"complete"}}),
        "every existing potential Skill must survive exactly"
    );
    replacement["closure"] = prior["closure"].clone();
    assert_eq!(
        restored, before,
        "direct declarations and every other descriptor field remain exact"
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
fn four_gem_inventories_preserve_all_potential_skill_members() {
    let migration = family::checked_migration();
    let originals = family::prior_descriptors();
    assert_eq!(originals.len(), OWNERS.len());
    assert_eq!(migration.schema.len(), originals.len());
    let mut member_counts = Vec::new();
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
        member_counts.push(
            json!(old)["value"]["schema"]["value"]["skills"]["members"]
                .as_array()
                .unwrap()
                .len(),
        );
    }
    member_counts.sort();
    assert_eq!(member_counts, [0, 1, 1, 1]);
}

#[test]
fn publication_inverse_rejects_unreviewed_direct_declaration_completion() {
    let migration = family::checked_migration();
    let originals = family::prior_descriptors();
    let old = &originals[0];
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
    let mut invalid = json!(next);
    assert_eq!(
        invalid["value"]["schema"]["value"]["declarations"]["actors"]["closure"]["kind"],
        "partial"
    );
    invalid["value"]["schema"]["value"]["declarations"]["actors"]["closure"] =
        json!({"kind":"complete"});
    let invalid: DefinitionDescriptor = serde_json::from_value(invalid).unwrap();
    assert!(std::panic::catch_unwind(|| check_descriptor(old, &invalid)).is_err());
}

#[test]
#[ignore = "explicit immutable predecessor and output; executes all five unchanged imports"]
fn publish_gem_skill_inventories_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        std::env::var_os("POE_OPTIMIZER_TEST_GEM_SKILL_INVENTORIES_PRIOR")
            .expect("checked current predecessor")
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_GEM_SKILL_INVENTORIES_OUTPUT")
            .expect("fresh output")
            .into(),
        &data(),
        &[],
        &["authoring.json", "evidence.json"],
        stage,
        json!({
            "new_definitions":0,
            "new_programs":0,
            "closed_potential_skill_inventories":4,
            "gem_definitions":4,
            "existing_skill_members_preserved":true,
            "direct_declaration_coverage_unchanged":true,
            "descendant_topology_unchanged":true,
            "numerical_coverage_unchanged":true,
            "scenario_intent_inventory_unchanged":true
        }),
        [107, 118, 110, 124, 5],
    );
}
