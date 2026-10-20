//! Publish reviewed intrinsic topology without completing numerical gameplay coverage.
#[allow(dead_code)]
#[path = "support/owned_item_structural_inventories.rs"]
mod items;
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
#[allow(dead_code)]
#[path = "support/owned_support_structural_inventories.rs"]
mod supports;

use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey,
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease},
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf};

const BEFORE: &str = "091556fee21e868c21ac3c2a61390c3e90c5900ab4576552c90a46a9fcc63ffd";
const KIND: &str = "reviewed-intrinsic-structural-inventories";
const RELEASE: &str = "pob-3887ae68-structural-inventories-v1";
const CLOSED: [&str; 3] = ["grants", "actors", "skill_grants"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn merge(
    mut supports: OwnedReleaseMigrationInput,
    items: OwnedReleaseMigrationInput,
) -> Result<OwnedReleaseMigrationInput, &'static str> {
    if supports.before.to_string() != BEFORE
        || supports.before != items.before
        || supports.schema_version != 5
        || supports.schema_version != items.schema_version
        || supports.contract != items.contract
    {
        return Err("same current authenticated predecessor and contract required");
    }
    for packet in [&supports, &items] {
        if !packet.tables.is_empty()
            || !packet.owners.is_empty()
            || !packet.receivers.is_empty()
            || !packet.query_targets.is_empty()
            || packet.evaluation.is_some()
        {
            return Err("intrinsic structure cannot change executable or query content");
        }
    }
    if supports.schema.len() != 11 || items.schema.len() != 4 {
        return Err("complete reviewed packet scopes required");
    }
    if supports.schema.iter().any(|row| {
        !matches!(
            row,
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Gem(_))
        )
    }) || items.schema.iter().any(|row| {
        !matches!(
            row,
            SchemaExtensionEntry::Definition(DefinitionDescriptor::ItemTemplate(_))
        )
    }) {
        return Err("exact support-Gem and intrinsic item-template owners required");
    }
    supports.schema.extend(items.schema);
    supports.schema.sort_by_key(|row| match row {
        SchemaExtensionEntry::Definition(value) => value.address().key().clone(),
        _ => unreachable!("checked definitions only"),
    });
    let mut seen = BTreeSet::new();
    for row in &supports.schema {
        let SchemaExtensionEntry::Definition(value) = row else {
            unreachable!("checked definitions only");
        };
        if !seen.insert(value.address().key().clone()) {
            return Err("duplicate structural owner");
        }
    }
    supports.release = OwnedDefinitionKey::new(RELEASE).unwrap();
    supports.reason = OwnedDefinitionKey::new(KIND).unwrap();
    Ok(supports)
}

fn checked_migration() -> OwnedReleaseMigrationInput {
    merge(supports::checked_migration(), items::checked_migration()).unwrap()
}

fn check_descriptor(old: &DefinitionDescriptor, next: &DefinitionDescriptor) {
    assert_eq!(old.address(), next.address());
    let before = json!(old);
    let mut restored = json!(next);
    for field in CLOSED {
        let path = format!("/value/schema/value/declarations/{field}");
        let prior = before.pointer(&path).expect("known intrinsic declaration");
        let replacement = restored
            .pointer_mut(&path)
            .expect("preserved intrinsic declaration");
        assert_eq!(prior["members"], json!([]));
        assert_eq!(prior["closure"]["kind"], "partial");
        assert_eq!(
            *replacement,
            json!({"members":[],"closure":{"kind":"complete"}})
        );
        replacement["closure"] = prior["closure"].clone();
    }
    if matches!(old, DefinitionDescriptor::Gem(_)) {
        let path = "/value/schema/value/skills";
        let prior = before
            .pointer(path)
            .expect("known potential Skill inventory");
        let replacement = restored
            .pointer_mut(path)
            .expect("preserved potential Skill inventory");
        assert_eq!(prior["members"], json!([]));
        assert_eq!(prior["closure"]["kind"], "partial");
        assert_eq!(
            *replacement,
            json!({"members":[],"closure":{"kind":"complete"}})
        );
        replacement["closure"] = prior["closure"].clone();
    }
    assert_eq!(
        restored, before,
        "only reviewed structural closures may change"
    );
}

fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let migration = checked_migration();
    let mut originals = supports::prior_descriptors();
    originals.extend(items::prior_descriptors());
    let packets: Vec<Value> = [
        "support-structural-inventories",
        "item-structural-inventories",
    ]
    .map(|name| {
        serde_json::from_slice(
            &fs::read(root().join(format!("data/owned/poe2/3887ae68/{name}/authoring.json")))
                .unwrap(),
        )
        .unwrap()
    })
    .into();
    let authoring_input = digest_owned(KIND, &(&packets, &migration), 2 * 1024 * 1024).unwrap();
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
fn combined_packet_has_fifteen_disjoint_reviewed_owners() {
    let migration = checked_migration();
    assert_eq!(migration.schema.len(), 15);
    let mut originals = supports::prior_descriptors();
    originals.extend(items::prior_descriptors());
    for old in originals {
        let replacement = migration.schema.iter().find_map(|row| match row {
            SchemaExtensionEntry::Definition(value) if value.address() == old.address() => {
                Some(value)
            }
            _ => None,
        });
        check_descriptor(&old, replacement.unwrap());
    }
}

#[test]
fn combined_packet_refuses_duplicate_owners_and_unreviewed_contract_changes() {
    let supports = supports::checked_migration();
    let items = items::checked_migration();
    let mut duplicate = supports.clone();
    duplicate.schema[1] = duplicate.schema[0].clone();
    assert!(merge(duplicate, items.clone()).is_err());
    let mut wrong_scope = supports.clone();
    wrong_scope.schema[0] = items.schema[0].clone();
    assert!(merge(wrong_scope, items.clone()).is_err());
    let mut wrong_contract = items.clone();
    wrong_contract.contract.schema_version += 1;
    assert!(merge(supports.clone(), wrong_contract).is_err());
    let mut missing = items.clone();
    missing.schema.pop();
    assert!(merge(supports, missing).is_err());
}

#[test]
#[ignore = "explicit immutable predecessor and output; executes all five unchanged imports"]
fn publish_intrinsic_structural_inventories_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        std::env::var_os("POE_OPTIMIZER_TEST_STRUCTURAL_INVENTORIES_PRIOR")
            .expect("checked current predecessor")
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_STRUCTURAL_INVENTORIES_OUTPUT")
            .expect("fresh output")
            .into(),
        &root().join("data/owned/poe2/3887ae68/support-structural-inventories"),
        &[],
        &[
            "authoring.json",
            "evidence.json",
            "../item-structural-inventories/authoring.json",
            "../item-structural-inventories/source-vectors.json",
        ],
        stage,
        json!({
            "new_definitions":0,
            "new_programs":0,
            "structural_declaration_refinement":true,
            "closed_intrinsic_declarations":45,
            "closed_potential_skill_inventories":11,
            "closed_structural_inventories":56,
            "support_gem_definitions":11,
            "item_template_definitions":4,
            "descendant_topology_unchanged":true,
            "modifier_owned_firebolt_preserved":true,
            "numerical_coverage_unchanged":true,
            "scenario_intent_inventory_unchanged":true
        }),
        [107, 118, 110, 124, 5],
    );
}
