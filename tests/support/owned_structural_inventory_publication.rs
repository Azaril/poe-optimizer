//! Exact whole-release inversion shared by structural-only authoring packets.
use super::migration_preservation;
use poe_optimizer_core::owned_schema::DefinitionDescriptor;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};

/// The family-specific checker proves every permitted descriptor change before
/// the common publisher restores those exact rows and compares the whole recipe.
pub fn stage(
    prior: &StagedOwnedRelease,
    migration: OwnedReleaseMigrationInput,
    originals: Vec<DefinitionDescriptor>,
    check_descriptor: fn(&DefinitionDescriptor, &DefinitionDescriptor),
    provenance: OwnedReleaseProvenance,
) -> StagedOwnedRelease {
    assert_eq!(prior.receipt().input, migration.before);
    assert_eq!(provenance.prior_input, prior.receipt().input);
    assert_eq!(originals.len(), migration.schema.len());
    assert!(
        migration.tables.is_empty()
            && migration.owners.is_empty()
            && migration.receivers.is_empty()
            && migration.query_targets.is_empty()
            && migration.evaluation.is_none(),
        "structural refinements cannot replace executable or query content"
    );
    for old in &originals {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|value| *value == old)
                .count(),
            1,
            "exact original descriptor must exist"
        );
        let replacements: Vec<_> = migration
            .schema
            .iter()
            .filter_map(|row| match row {
                SchemaExtensionEntry::Definition(value) if value.address() == old.address() => {
                    Some(value)
                }
                _ => None,
            })
            .collect();
        assert_eq!(replacements.len(), 1);
        check_descriptor(old, replacements[0]);
    }
    let migrated = compile_owned_release_migration(prior, migration, Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    *input.provenance.last_mut().unwrap() = provenance;
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().recipe.clone();
    for old in originals {
        let current = restored
            .schema
            .definitions
            .iter_mut()
            .find(|value| value.address() == old.address())
            .unwrap();
        check_descriptor(&old, current);
        *current = old;
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "all other schema, rules, identities and routing remain exact"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
