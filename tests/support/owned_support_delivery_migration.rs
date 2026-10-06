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
#[allow(dead_code)] // Included by families that opt in to the separate topology stage below.
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

/// Explicit authoring-only extension for a support family that first needs a
/// real Skill Action output. The historical Stat-only `stage` stays unchanged.
/// Replacements are exact old descriptors in `dependencies.replaced_schema`;
/// new Gem rule owners are explicitly listed in `dependencies.new_owner_ids`.
#[allow(dead_code)] // Historical families retain the strict Stat-only stage above.
pub fn stage_with_action_topology(
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
        assert_eq!(receipt[field], d["source"][field]);
    }
    assert_eq!(receipt["input"], a["before"]);
    assert_eq!(receipt["input"], d["source"]["input"]);
    assert_eq!(m.before, prior.receipt().input);
    assert!(prior.evaluation().is_none());
    assert!(m.evaluation.is_none());
    assert!(m.tables.is_empty());
    assert!(m.receivers.is_empty());
    assert!(m.query_targets.is_empty());
    let old_recipe = &prior.input().recipe;
    assert_eq!(m.contract.schema_version, old_recipe.schema.schema_version);
    assert_eq!(
        m.contract.schema_semantics_version,
        old_recipe.schema.semantics_version
    );
    assert_eq!(
        m.contract.operations_version,
        old_recipe.rules.operations_version
    );
    assert_eq!(
        m.contract.rule_semantics_version,
        old_recipe.rules.semantics_version
    );
    for row in decode::<Vec<DefinitionDescriptor>>(&d["supporting_definitions"]) {
        assert_eq!(
            old_recipe
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
            old_recipe
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
    let old_owners: Vec<DefinitionRules> = decode(&d["owners"]);
    for owner in &old_owners {
        assert_eq!(
            old_recipe
                .rules
                .owners
                .iter()
                .filter(|r| *r == owner)
                .count(),
            1
        );
        assert_eq!(
            old_owners.iter().filter(|r| r.owner == owner.owner).count(),
            1
        );
    }
    let replacements: Vec<SchemaExtensionEntry> = decode(&d["replaced_schema"]);
    assert!(!replacements.is_empty());
    for old in &replacements {
        assert_eq!(
            replacements
                .iter()
                .filter(|r| r.subject() == old.subject())
                .count(),
            1
        );
        let count = match old {
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(row)) => {
                assert!(matches!(row.schema, SchemaState::Known(_)));
                old_recipe
                    .schema
                    .definitions
                    .iter()
                    .filter(|r| **r == DefinitionDescriptor::Skill(row.clone()))
                    .count()
            }
            SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(row)) => {
                assert!(matches!(row.id.declaration, SlotOwnerDefId::Gem(_)));
                assert!(matches!(row.schema, SchemaState::Known(_)));
                old_recipe
                    .schema
                    .slots
                    .iter()
                    .filter(|r| **r == SlotDescriptor::SkillGrant(row.clone()))
                    .count()
            }
            _ => panic!(
                "Action topology may replace only exact Skill and Gem SkillGrant descriptors"
            ),
        };
        assert_eq!(count, 1);
        assert_eq!(
            m.schema
                .iter()
                .filter(|r| r.subject() == old.subject())
                .count(),
            1
        );
    }
    let new_owners: Vec<SchemaSubject> = decode(&d["new_owner_ids"]);
    assert!(!new_owners.is_empty());
    for owner in &new_owners {
        let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) = owner else {
            panic!("Action topology may add only explicitly declared Gem rule owners")
        };
        assert_eq!(new_owners.iter().filter(|r| *r == owner).count(), 1);
        assert!(!old_recipe.rules.owners.iter().any(|r| &r.owner == owner));
        assert_eq!(old_recipe.schema.definitions.iter().filter(|r| matches!(r,
            DefinitionDescriptor::Gem(row) if &row.id == gem && matches!(row.schema, SchemaState::Known(_))
        )).count(), 1);
    }
    assert_eq!(m.owners.len(), old_owners.len() + new_owners.len());
    for owner in &m.owners {
        assert_eq!(
            m.owners.iter().filter(|r| r.owner == owner.owner).count(),
            1
        );
        assert_eq!(
            usize::from(old_owners.iter().any(|r| r.owner == owner.owner))
                + usize::from(new_owners.contains(&owner.owner)),
            1
        );
    }

    let mut registry =
        OwnedIdRegistry::new(old_recipe.registry.clone(), Default::default()).unwrap();
    let mut added = Vec::new();
    let mut outputs = Vec::new();
    for entry in &m.schema {
        if replacements
            .iter()
            .any(|old| old.subject() == entry.subject())
        {
            continue;
        }
        let allocated = match entry {
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Stat(row)) => {
                assert!(matches!(row.schema, SchemaState::Known(_)));
                SchemaSubject::Definition(
                    registry
                        .allocate_definition::<StatDefinition>()
                        .unwrap()
                        .address(),
                )
            }
            SchemaExtensionEntry::Definition(DefinitionDescriptor::ActionPart(row)) => {
                assert!(matches!(row.schema, SchemaState::Known(_)));
                SchemaSubject::Definition(
                    registry
                        .allocate_definition::<ActionPartDefinition>()
                        .unwrap()
                        .address(),
                )
            }
            SchemaExtensionEntry::Definition(DefinitionDescriptor::ActionMode(row)) => {
                assert!(matches!(row.schema, SchemaState::Known(_)));
                SchemaSubject::Definition(
                    registry
                        .allocate_definition::<ActionModeDefinition>()
                        .unwrap()
                        .address(),
                )
            }
            SchemaExtensionEntry::Definition(DefinitionDescriptor::ActionStatSet(row)) => {
                assert!(matches!(row.schema, SchemaState::Known(_)));
                SchemaSubject::Definition(
                    registry
                        .allocate_definition::<ActionStatSetDefinition>()
                        .unwrap()
                        .address(),
                )
            }
            SchemaExtensionEntry::Slot(SlotDescriptor::ActionOutput(row)) => {
                let SlotOwnerDefId::Skill(skill) = &row.id.declaration else {
                    panic!("new Action output must belong to the replaced Skill")
                };
                assert!(matches!(row.schema, SchemaState::Known(_)));
                assert_eq!(replacements.iter().filter(|old| matches!(old,
                    SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(old)) if &old.id == skill
                )).count(), 1);
                outputs.push(row.id.clone());
                let slot = registry
                    .allocate_slot::<ActionOutputDefinition>(row.id.declaration.clone())
                    .unwrap();
                SchemaSubject::Slot(ActionOutputDefId::address(&slot))
            }
            _ => panic!(
                "Action topology may allocate only Stat/ActionPart/ActionMode/ActionStatSet definitions and Skill ActionOutput slots"
            ),
        };
        assert_eq!(allocated, entry.subject());
        assert!(!added.contains(&allocated));
        added.push(allocated);
    }
    assert!(!outputs.is_empty());
    for old in &replacements {
        let next = m
            .schema
            .iter()
            .find(|r| r.subject() == old.subject())
            .unwrap();
        assert_action_output_append(old, next, &outputs);
    }

    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(kind),
        prior_input: prior.receipt().input,
        authoring_input: digest,
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_eq!(next.input().recipe.registry, *registry.input());
    // First prove the published delta itself is exact, then invert only it.
    for entry in &m.schema {
        let count = match entry {
            SchemaExtensionEntry::Definition(row) => next
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|r| *r == row)
                .count(),
            SchemaExtensionEntry::Slot(row) => next
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|r| *r == row)
                .count(),
        };
        assert_eq!(count, 1);
    }
    for owner in &m.owners {
        assert_eq!(
            next.input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|r| *r == owner)
                .count(),
            1
        );
    }
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        old_recipe.schema.definitions.len()
            + added
                .iter()
                .filter(|r| matches!(r, SchemaSubject::Definition(_)))
                .count()
    );
    assert_eq!(
        restored.schema.slots.len(),
        old_recipe.schema.slots.len() + outputs.len()
    );
    assert_eq!(
        restored.rules.owners.len(),
        old_recipe.rules.owners.len() + new_owners.len()
    );
    restored
        .schema
        .definitions
        .retain(|r| !added.contains(&SchemaSubject::Definition(r.address())));
    restored
        .schema
        .slots
        .retain(|r| !added.contains(&SchemaSubject::Slot(r.address())));
    for old in replacements {
        match old {
            SchemaExtensionEntry::Definition(row) => {
                let target = restored
                    .schema
                    .definitions
                    .iter_mut()
                    .find(|r| r.address() == row.address())
                    .unwrap();
                *target = row;
            }
            SchemaExtensionEntry::Slot(row) => {
                let target = restored
                    .schema
                    .slots
                    .iter_mut()
                    .find(|r| r.address() == row.address())
                    .unwrap();
                *target = row;
            }
        }
    }
    restored
        .rules
        .owners
        .retain(|r| !new_owners.contains(&r.owner));
    for old in old_owners {
        let target = restored
            .rules
            .owners
            .iter_mut()
            .find(|r| r.owner == old.owner)
            .unwrap();
        *target = old;
    }
    restored.registry = old_recipe.registry.clone();
    restored.schema.release = old_recipe.schema.release.clone();
    restored.rules.definitions = old_recipe.rules.definitions.clone();
    restored.routing.definitions = old_recipe.routing.definitions.clone();
    assert!(
        restored == *old_recipe,
        "recipe changed beyond declared Action topology and support owners"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

#[allow(dead_code)] // Used only by the explicit topology authoring stage.
fn assert_action_output_append(
    old: &SchemaExtensionEntry,
    next: &SchemaExtensionEntry,
    outputs: &[poe_optimizer_core::owned_build::DeclaredSlot<ActionOutputDefId>],
) {
    match (old, next) {
        (
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(old)),
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(next)),
        ) => {
            let SchemaState::Known(mut expected) = old.schema.clone() else {
                panic!("known old Skill")
            };
            let new: Vec<_> = outputs
                .iter()
                .filter(|out| out.declaration == SlotOwnerDefId::Skill(old.id.clone()))
                .cloned()
                .collect();
            assert!(!new.is_empty());
            expected.declarations.outputs.members.extend(new);
            assert_eq!(old.id, next.id);
            assert_eq!(
                next.schema,
                SchemaState::Known(expected),
                "only output membership may change; every prior field/closure survives"
            );
        }
        (
            SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(old)),
            SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(next)),
        ) => {
            let SchemaState::Known(mut expected) = old.schema.clone() else {
                panic!("known old Gem SkillGrant")
            };
            let new: Vec<_> = outputs
                .iter()
                .filter(|out| out.declaration == SlotOwnerDefId::Skill(expected.skill.clone()))
                .cloned()
                .collect();
            assert!(!new.is_empty());
            expected.outputs.members.extend(new);
            assert_eq!(old.id, next.id);
            assert_eq!(
                next.schema,
                SchemaState::Known(expected),
                "only output membership may change; the supplied Skill and input authority survive"
            );
        }
        _ => panic!("replacement kind changed outside the Action topology contract"),
    }
}
