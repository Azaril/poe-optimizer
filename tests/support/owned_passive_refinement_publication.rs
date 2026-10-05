//! Shared checked migration, finite passive refinement and exact recipe inverse.
//! Family helpers independently authenticate their complete source inventories.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{ActionStatSetDefinition, OwnedDefinitionKey, StatDefinition},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SlotDescriptor},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, PassiveDeclarationRefinement, SuccessorBundleInput,
        TreePolicyTransitionInput, transition_owned_catalog_with_tree_refinement_compact,
    },
};
use serde_json::{Value, json};
use std::{fs, path::Path};
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).unwrap()
}
pub fn stage(
    prior: &StagedOwnedRelease,
    directory: &Path,
    provenance: &str,
    domain: &'static str,
) -> StagedOwnedRelease {
    let read = |name: &str| -> Value {
        serde_json::from_slice(&fs::read(directory.join(name)).unwrap()).unwrap()
    };
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let b: Value = read("bindings.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert!(prior.evaluation().is_none());
    let old_definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    let supporting: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["supporting_definitions"].clone()).unwrap();
    let old_owners: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let action_owners: Vec<DefinitionRules> =
        serde_json::from_value(d["action_owners"].clone()).unwrap();
    let slots: Vec<SlotDescriptor> = serde_json::from_value(d["slots"].clone()).unwrap();
    for row in old_definitions.iter().chain(&supporting) {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    for row in old_owners.iter().chain(&action_owners) {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    for row in &slots {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    for row in d["mapping_rows"].as_array().unwrap() {
        let expected: poe_optimizer_import::owned_mapping::MappingEntry =
            serde_json::from_value(row.clone()).unwrap();
        assert_eq!(
            prior
                .input()
                .mapping
                .entries
                .iter()
                .filter(|x| **x == expected)
                .count(),
            1
        );
    }
    let migration: OwnedReleaseMigrationInput =
        serde_json::from_value(read("migration.json")).unwrap();
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    let mut recipe = migrated.input().recipe.clone();
    let next_definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let next_owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    assert_eq!(old_definitions.len(), next_definitions.len());
    assert_eq!(old_owners.len(), next_owners.len());
    let mut nodes = vec![];
    for (old, new) in old_definitions.iter().zip(&next_definitions) {
        let row = recipe
            .schema
            .definitions
            .iter_mut()
            .find(|x| x.address() == old.address())
            .unwrap();
        assert_eq!(row, old);
        let DefinitionDescriptor::PassiveNode(passive) = new else {
            panic!("passive refinement")
        };
        nodes.push(passive.id.clone());
        *row = new.clone();
    }
    for (old, new) in old_owners.iter().zip(&next_owners) {
        let row = recipe
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == old.owner)
            .unwrap();
        assert_eq!(row, old);
        *row = new.clone();
    }
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), Default::default()).unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
    let refined = transition_owned_catalog_with_tree_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: migrated.input().recipe.clone(),
            successor: recipe,
            mapping: migrated.input().mapping.clone(),
            roles: migrated.input().roles.clone(),
            normalization: migrated.input().normalization.clone(),
            rewards: migrated.input().rewards.clone(),
            query_sets: migrated.input().query_sets.clone(),
            items: migrated.input().items.clone(),
            item_source: migrated.input().item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: migrated.input().mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(migrated.input().tree.clone().unwrap()),
        },
        PassiveDeclarationRefinement {
            schema_version: 1,
            before: migrated.receipt().definitions.clone(),
            after: schema.identity().clone(),
            nodes,
        },
        Default::default(),
    )
    .unwrap();
    let mut input = migrated.input().clone();
    input.recipe = refined.recipe().clone();
    input.mapping = refined.mapping().input().clone();
    input.roles = refined.roles().input().clone();
    input.normalization = refined.normalization().clone();
    input.rewards = refined.rewards().input().clone();
    input.items = refined.items().input().clone();
    input.item_source = refined.item_source().input().clone();
    input.tree = refined.tree().map(|tree| tree.input().clone());
    input.query_sets = refined.query_sets().to_vec();
    // The checked migration is an intermediate construction step. Publish one
    // packet receipt from the actual predecessor; authoring commits the exact
    // migration bytes as well as the subsequent passive refinement.
    assert_eq!(
        migrated.input().provenance.len(),
        prior.input().provenance.len() + 1
    );
    assert_eq!(
        migrated.input().provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    input.provenance = prior.input().provenance.clone();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key(provenance),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            domain,
            &(a, b, d, c, read("source-vectors.json")),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut expected = prior.input().recipe.clone();
    let mut registry = OwnedIdRegistry::new(expected.registry.clone(), Default::default()).unwrap();
    for entry in &migration.schema {
        if let SchemaExtensionEntry::Definition(row) = entry {
            let allocated = match row {
                DefinitionDescriptor::Stat(_) => registry
                    .allocate_definition::<StatDefinition>()
                    .unwrap()
                    .address(),
                DefinitionDescriptor::ActionStatSet(_) => registry
                    .allocate_definition::<ActionStatSetDefinition>()
                    .unwrap()
                    .address(),
                _ => panic!(
                    "this shared passive publisher admits only exact new Stat or ActionStatSet descriptors"
                ),
            };
            assert_eq!(allocated, row.address());
        }
    }
    expected.registry = registry.input().clone();
    expected.schema.release = migration.release.clone();
    for entry in &migration.schema {
        match entry {
            SchemaExtensionEntry::Definition(row) => {
                assert!(
                    !expected
                        .schema
                        .definitions
                        .iter()
                        .any(|d| d.address() == row.address())
                );
                expected.schema.definitions.push(row.clone());
            }
            SchemaExtensionEntry::Slot(row) => {
                *expected
                    .schema
                    .slots
                    .iter_mut()
                    .find(|s| s.address() == row.address())
                    .unwrap() = row.clone();
            }
        }
    }
    for row in next_definitions {
        let destination = expected
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == row.address())
            .unwrap();
        *destination = row;
    }
    expected
        .schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    expected
        .schema
        .slots
        .sort_by_cached_key(SlotDescriptor::address);
    for row in &migration.owners {
        if let Some(owner) = expected
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.owner)
        {
            *owner = row.clone();
        } else {
            expected.rules.owners.push(row.clone());
        }
    }
    for row in next_owners {
        let destination = expected
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.owner)
            .unwrap();
        *destination = row;
    }
    expected.rules.receivers.members.extend(migration.receivers);
    expected
        .rules
        .receivers
        .members
        .sort_by(|a, b| a.id.cmp(&b.id));
    expected.rules.definitions = next.receipt().definitions.clone();
    expected.routing.definitions = next.receipt().definitions.clone();
    assert_eq!(
        expected,
        next.input().recipe,
        "exact authored migration and passive refinements; every other recipe field is unchanged"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.input().query_sets, prior.input().query_sets);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
