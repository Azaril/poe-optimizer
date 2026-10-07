//! Checked offline publication of Sand preparation; no receiving coverage promotion.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{DefinitionRules, IntegerRuleTable, OWNED_RULE_OPERATIONS_V22},
    owned_schema::{DefinitionDescriptor, SchemaState, SchemaSubject, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{
        OwnedReleaseContractMigration, OwnedReleaseMigrationInput, compile_owned_release_migration,
    },
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[path = "owned_sand_preparation_evidence.rs"]
mod evidence;

const KIND: &str = "sand-preparation-and-generated-provider-roles";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    schema_version: u32,
    definitions: Vec<DefinitionDescriptor>,
    slots: Vec<SlotDescriptor>,
    owners: Vec<DefinitionRules>,
    tables: Vec<IntegerRuleTable>,
    upstream_input_file_sha256: String,
    upstream_semantic_input: String,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/sand-preparation")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn subject_key(subject: SchemaSubject) -> OwnedDefinitionKey {
    match subject {
        SchemaSubject::Definition(id) => id.key().clone(),
        SchemaSubject::Slot(id) => id.key().clone(),
    }
}

fn source(required_reports: bool) {
    let proof: Value = read("source-vectors.json");
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(proof["schema_version"], 1);
    assert_eq!(proof["status"], "retained-source-witness-passed");
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    assert_eq!(proof["source_manifest_sha256"], hash(&manifest_bytes));
    assert_eq!(proof["case_count"], 18);
    assert!(!proof["vectors"].as_array().unwrap().is_empty());
    assert!(
        proof["authority"]
            .as_object()
            .unwrap()
            .values()
            .all(|value| value == false),
        "source witness grants no native coverage or fallback authority"
    );
    for pin in proof["source_files"].as_array().unwrap() {
        let matching: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
            .collect();
        assert_eq!(matching.len(), 1);
        let path = root()
            .join("vendor/path-of-building-poe2")
            .join(pin["path"].as_str().unwrap());
        if path.exists() {
            let content = fs::read_to_string(path).unwrap().replace("\r\n", "\n");
            assert_eq!(pin["sha256"], hash(content.as_bytes()));
        }
    }
    for pin in proof["local_pins"].as_array().unwrap() {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    let reports = proof["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["sha256"], reports[1]["sha256"]);
    assert_eq!(reports[0]["bytes"], reports[1]["bytes"]);
    if required_reports {
        evidence::authenticate_reports(&root(), &proof);
    }
}

pub fn check_authored() {
    source(false);
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(dependencies.schema_version, 1);
    assert_eq!(dependencies.upstream_input_file_sha256.len(), 64);
    let extension: OwnedRecipeExtension = read("extension.json");
    assert_eq!(extension.schema_version, 1);
    assert_eq!(extension.schema.len(), 6);
    assert_eq!(
        extension.operations_version,
        Some(key(OWNED_RULE_OPERATIONS_V22))
    );
    assert!(extension.tables.is_empty() && extension.receivers.is_empty());
    assert_eq!(extension.owners.len(), 1);
    assert_eq!(
        extension
            .owners
            .iter()
            .map(|owner| owner.programs.members.len())
            .sum::<usize>(),
        4
    );
    for owner in &extension.owners {
        let old = dependencies
            .owners
            .iter()
            .find(|row| row.owner == owner.owner)
            .unwrap();
        assert_eq!(owner.programs.closure, old.programs.closure);
        assert!(!owner.programs.is_complete());
        for program in &owner.programs.members {
            assert!(!old.programs.members.iter().any(|row| row.id == program.id));
        }
    }
    let corrections: Vec<SlotDescriptor> = read("corrections.json");
    assert_eq!(corrections.len(), 2);
    for correction in &corrections {
        let old = dependencies
            .slots
            .iter()
            .find(|row| row.address() == correction.address())
            .unwrap();
        let mut restored = json!(correction);
        assert_eq!(restored["kind"], "grant");
        assert_eq!(
            restored["value"]["schema"]["value"]["provider_roles"],
            json!(["skill_use", "allocation"])
        );
        restored["value"]["schema"]["value"]["provider_roles"] = json!(["skill_use"]);
        assert_eq!(
            restored,
            json!(old),
            "only the actual allocation role is added"
        );
    }
    let additions: Vec<_> = extension
        .schema
        .iter()
        .filter_map(|row| match row {
            SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(row)) => Some(row.id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(additions.len(), 2);
    let refinements: Vec<_> = extension
        .schema
        .iter()
        .filter_map(|row| match row {
            SchemaExtensionEntry::Definition(row) => dependencies
                .definitions
                .iter()
                .find(|old| old.address() == row.address())
                .map(|old| (old, row)),
            _ => None,
        })
        .collect();
    let [(DefinitionDescriptor::Skill(old), DefinitionDescriptor::Skill(next))] =
        refinements.as_slice()
    else {
        panic!("only the Command parameter membership changes")
    };
    let (SchemaState::Known(old_schema), SchemaState::Known(next_schema)) =
        (&old.schema, &next.schema)
    else {
        panic!("known Command schema")
    };
    let mut expected = old_schema.clone();
    expected.declarations.parameters.members.extend(additions);
    assert_eq!(&expected, next_schema);
    assert!(!next_schema.declarations.parameters.is_complete());
    let relations: Value = read("source-properties.json");
    assert_eq!(relations["relations"]["closure"]["kind"], "partial");
    assert_eq!(
        relations["relations"]["members"].as_array().unwrap().len(),
        2
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source(true);
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(
        prior.receipt().input.to_string(),
        dependencies.upstream_semantic_input
    );
    let before = prior.input();
    assert!(before.evaluation.is_none());
    assert_eq!(before.recipe.registry.last_issued.get(), 0x334d);
    for row in &dependencies.definitions {
        assert!(before.recipe.schema.definitions.contains(row));
    }
    for row in &dependencies.slots {
        assert!(before.recipe.schema.slots.contains(row));
    }
    for row in &dependencies.owners {
        assert!(before.recipe.rules.owners.contains(row));
    }
    for row in &dependencies.tables {
        assert!(before.recipe.rules.tables.contains(row));
    }
    let extension: OwnedRecipeExtension = read("extension.json");
    let corrections: Vec<SlotDescriptor> = read("corrections.json");
    let mut schema = extension.schema.clone();
    schema.extend(corrections.into_iter().map(SchemaExtensionEntry::Slot));
    schema.sort_by_key(|row| subject_key(row.subject()));
    let migration = OwnedReleaseMigrationInput {
        schema_version: 5,
        before: prior.receipt().input,
        release: key("pob-3887ae68-sand-preparation-v1"),
        reason: key(KIND),
        contract: OwnedReleaseContractMigration {
            schema_version: before.recipe.schema.schema_version,
            schema_semantics_version: before.recipe.schema.semantics_version.clone(),
            operations_version: extension.operations_version.clone().unwrap(),
            rule_semantics_version: before.recipe.rules.semantics_version.clone(),
        },
        schema,
        tables: extension.tables.clone(),
        owners: extension.owners.clone(),
        receivers: extension.receivers.clone(),
        query_targets: vec![],
        evaluation: None,
    };
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let proof = input.provenance.last_mut().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(proof.prior_input, prior.receipt().input);
    assert_eq!(
        proof.authoring_input,
        digest_owned(
            "owned-release-contract-migration-v5",
            &migration,
            16 * 1024 * 1024
        )
        .unwrap()
    );
    proof.authoring_input = digest_owned(
        "owned-sand-preparation-publication-v1",
        &(
            &migration,
            read::<Value>("extension.json"),
            read::<Value>("corrections.json"),
            read::<Value>("dependencies.json"),
            read::<Value>("source-properties.json"),
            read::<Value>("readiness.json"),
            read::<Value>("source-vectors.json"),
        ),
        4 * 1024 * 1024,
    )
    .unwrap();
    let next = assemble_owned_release(input, Default::default()).unwrap();
    inverse(prior, &next, &migration);
    next
}

fn inverse(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    migration: &OwnedReleaseMigrationInput,
) {
    let before = &prior.input().recipe;
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.registry.entries.len(),
        before.registry.entries.len() + 5
    );
    assert_eq!(restored.registry.last_issued.get(), 0x3352);
    assert_eq!(
        restored.registry.entries[..before.registry.entries.len()],
        before.registry.entries
    );
    restored.registry = before.registry.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        before.schema.definitions.len() + 3
    );
    assert_eq!(restored.schema.slots.len(), before.schema.slots.len() + 2);
    for row in &migration.schema {
        match row {
            SchemaExtensionEntry::Definition(expected) => {
                let at = restored
                    .schema
                    .definitions
                    .iter()
                    .position(|row| row.address() == expected.address())
                    .unwrap();
                assert_eq!(&restored.schema.definitions[at], expected);
                if let Some(old) = before
                    .schema
                    .definitions
                    .iter()
                    .find(|row| row.address() == expected.address())
                {
                    restored.schema.definitions[at] = old.clone();
                } else {
                    restored.schema.definitions.remove(at);
                }
            }
            SchemaExtensionEntry::Slot(expected) => {
                let at = restored
                    .schema
                    .slots
                    .iter()
                    .position(|row| row.address() == expected.address())
                    .unwrap();
                assert_eq!(&restored.schema.slots[at], expected);
                if let Some(old) = before
                    .schema
                    .slots
                    .iter()
                    .find(|row| row.address() == expected.address())
                {
                    restored.schema.slots[at] = old.clone();
                } else {
                    restored.schema.slots.remove(at);
                }
            }
        }
    }
    for addition in &migration.owners {
        let owner = restored
            .rules
            .owners
            .iter_mut()
            .find(|row| row.owner == addition.owner)
            .unwrap();
        let old = before
            .rules
            .owners
            .iter()
            .find(|row| row.owner == addition.owner)
            .unwrap();
        let mut expected = old.clone();
        expected
            .programs
            .members
            .extend(addition.programs.members.clone());
        assert_eq!(*owner, expected);
        *owner = old.clone();
    }
    restored.schema.release = before.schema.release.clone();
    restored.rules.definitions = before.rules.definitions.clone();
    restored.routing.definitions = before.routing.definitions.clone();
    assert!(
        restored == *before,
        "exact predecessor recipe after only the reviewed delta"
    );
    migration_preservation::assert_import_rebindings_only(prior, next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
}
