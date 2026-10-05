//! Shared authoring-only support migration proof. This is not a runtime importer.
use super::super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
    owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_import::{
    owned_mapping::{MappingEntry, OwnedIdRegistry},
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::Path};
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceivingFragment {
    roles: Vec<SupportReceivingRole>,
    targets: Vec<SupportTargetReceivingRoles>,
    supports: Vec<SupportReceivingEntry>,
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn read<T: DeserializeOwned>(directory: &Path, name: &str) -> T {
    serde_json::from_slice(&fs::read(directory.join(name)).unwrap()).unwrap()
}
pub fn authoring_digest(directory: &Path, domain: &'static str) -> OwnedContentDigest {
    // Preserve typed tuple and field order: existing historical receipts depend on it.
    digest_owned(
        domain,
        &(
            read::<Value>(directory, "authoring.json"),
            read::<Value>(directory, "bindings.json"),
            read::<Value>(directory, "dependencies.json"),
            read::<Value>(directory, "source-vectors.json"),
            read::<OwnedReleaseMigrationInput>(directory, "migration.json"),
            read::<ReceivingFragment>(directory, "receiving.json"),
            read::<SupportPreparationInput>(directory, "preparation.json"),
        ),
        8 * 1024 * 1024,
    )
    .unwrap()
}
pub fn stage(
    prior: &StagedOwnedRelease,
    directory: &Path,
    kind: &str,
    digest: OwnedContentDigest,
) -> StagedOwnedRelease {
    let a: Value = read(directory, "authoring.json");
    let d: Value = read(directory, "dependencies.json");
    let m: OwnedReleaseMigrationInput = read(directory, "migration.json");
    let receipt = json!(prior.receipt());
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert_eq!(receipt["input"], a["before"]);
    assert!(prior.evaluation().is_none());
    for row in decode::<Vec<DefinitionDescriptor>>(&d["supporting_definitions"]) {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    for row in decode::<Vec<SlotDescriptor>>(&d["slots"]) {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    for row in decode::<Vec<MappingEntry>>(&d["mapping_rows"]) {
        assert_eq!(
            prior
                .input()
                .mapping
                .entries
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    let old: Vec<DefinitionRules> = decode(&d["owners"]);
    for owner in &old {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|r| *r == owner)
                .count(),
            1
        );
    }
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(kind),
        prior_input: prior.receipt().input,
        authoring_input: digest,
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let mut added = vec![];
    for entry in &m.schema {
        let SchemaExtensionEntry::Definition(DefinitionDescriptor::Stat(definition)) = entry else {
            panic!("support delivery may allocate only explicit Stat definitions")
        };
        let allocated = registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address();
        assert_eq!(allocated, definition.id.address());
        added.push(allocated);
    }
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + added.len()
    );
    assert_eq!(
        restored.rules.owners.len(),
        prior.input().recipe.rules.owners.len()
    );
    restored
        .schema
        .definitions
        .retain(|d| !added.contains(&d.address()));
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    for owner in old {
        let row = restored
            .rules
            .owners
            .iter_mut()
            .find(|r| r.owner == owner.owner)
            .unwrap();
        *row = owner;
    }
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert!(
        restored == prior.input().recipe,
        "recipe changed beyond declared Stat allocations and replaced owners"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
