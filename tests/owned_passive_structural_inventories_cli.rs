//! Refine proven intrinsic passive inventories without changing their mechanics.
#[allow(dead_code)]
#[path = "support/owned_passive_structural_inventories.rs"]
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

const BEFORE: &str = "35d446c73be3aa977d55a49174aaf4656e27a38ebaeaab354e86cd7f9f36b5a5";
const KIND: &str = "reviewed-passive-structural-inventories";
const CLOSED: [&str; 3] = ["grants", "actors", "skill_grants"];
const OWNERS: [&str; 6] = [
    "def.0000000000000b34",
    "def.0000000000000d0f",
    "def.00000000000010d4",
    "def.00000000000011c2",
    "def.00000000000014d8",
    "def.00000000000019ec",
];

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/passive-structural-inventories")
}

fn check_descriptor(old: &DefinitionDescriptor, next: &DefinitionDescriptor) {
    assert!(matches!(old, DefinitionDescriptor::PassiveNode(_)));
    assert_eq!(old.address(), next.address());
    let before = json!(old);
    let mut restored = json!(next);
    for field in CLOSED {
        let path = format!("/value/schema/value/declarations/{field}");
        let prior = before.pointer(&path).expect("known passive declaration");
        assert_eq!(prior["closure"]["kind"], "partial");
        let replacement = restored.pointer_mut(&path).expect("preserved declaration");
        assert_eq!(
            *replacement,
            json!({"members":prior["members"],"closure":{"kind":"complete"}}),
            "existing grant and supply members must survive exactly"
        );
        replacement["closure"] = prior["closure"].clone();
    }
    assert_eq!(
        restored, before,
        "only the three reviewed structural closure fields may change"
    );
}

fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let migration = family::checked_migration();
    assert_eq!(migration.before.to_string(), BEFORE);
    let originals = family::prior_descriptors();
    assert_eq!(originals.len(), OWNERS.len());
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
fn passive_packet_preserves_all_existing_owner_members() {
    let migration = family::checked_migration();
    let originals = family::prior_descriptors();
    assert_eq!(originals.len(), OWNERS.len());
    assert_eq!(migration.schema.len(), originals.len());
    let mut nonempty = 0;
    for old in originals {
        let replacement = migration
            .schema
            .iter()
            .find_map(|row| match row {
                SchemaExtensionEntry::Definition(value) if value.address() == old.address() => {
                    Some(value)
                }
                _ => None,
            })
            .unwrap();
        check_descriptor(&old, replacement);
        let old = json!(old);
        nonempty += CLOSED
            .into_iter()
            .filter(|field| {
                !old["value"]["schema"]["value"]["declarations"][*field]["members"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            })
            .count();
    }
    assert_eq!(
        nonempty, 4,
        "both existing Barya grant/supply pairs survive"
    );
}

#[test]
fn publication_inverse_refuses_erasing_an_existing_grant() {
    let migration = family::checked_migration();
    let originals = family::prior_descriptors();
    let old = originals
        .iter()
        .find(|value| value.address().key().as_str() == OWNERS[0])
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
    let mut erased = json!(next);
    erased["value"]["schema"]["value"]["declarations"]["grants"]["members"] = json!([]);
    let erased: DefinitionDescriptor = serde_json::from_value(erased).unwrap();
    assert!(std::panic::catch_unwind(|| check_descriptor(old, &erased)).is_err());
}

#[test]
#[ignore = "explicit immutable predecessor and output; executes all five unchanged imports"]
fn publish_passive_structural_inventories_preserving_all_five_originals() {
    publication::run_with_item_parameter_completions_and_counts(
        std::env::var_os("POE_OPTIMIZER_TEST_PASSIVE_STRUCTURAL_PRIOR")
            .expect("checked current predecessor")
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_PASSIVE_STRUCTURAL_OUTPUT")
            .expect("fresh output")
            .into(),
        &data(),
        &[],
        &["authoring.json", "evidence.json"],
        stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: true,
            extra: json!({
                "new_definitions":0,
                "new_programs":0,
                "structural_declaration_refinement":true,
                "closed_intrinsic_declarations":18,
                "passive_definitions":6,
                "existing_barya_grants_preserved":true,
                "unresolved_gain_passive_owners":4,
                "unresolved_command_passive_owners":2,
                "descendant_topology_unchanged":true,
                "numerical_coverage_unchanged":true,
                "scenario_intent_inventory_unchanged":true
            }),
        },
        &[],
        [107, 118, 110, 124, 5],
    );
}
