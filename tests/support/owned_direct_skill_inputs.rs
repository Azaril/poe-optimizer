//! Authored raw Direct inputs, with source evidence separate from calculation coverage.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SchemaState, SkillInputAuthority, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_normalize::DirectSkillInputPolicy,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/direct-skill-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn check_authored() {
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let inputs: DirectSkillInputPolicy = read("inputs.json");
    let authoring: Value = read("authoring.json");
    assert_eq!(migration.schema_version, 3);
    assert_eq!(migration.contract.schema_version, 5);
    assert_eq!(
        migration.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert_eq!(migration.schema.len(), 6);
    assert!(migration.tables.is_empty() && migration.owners.is_empty());
    assert!(migration.receivers.is_empty() && migration.query_targets.is_empty());
    assert!(migration.evaluation.is_none());
    let DirectSkillInputPolicy::PobManualDirectSkillV1 {
        skills,
        source,
        catalog,
        ..
    } = &inputs
    else {
        panic!("fixture requires Direct V1")
    };
    assert_eq!(skills.len(), 2);
    for row in skills {
        assert_eq!(row.parameters.len(), 2);
        let schema = migration
            .schema
            .iter()
            .find_map(|entry| match entry {
                SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(entry))
                    if entry.id == row.skill =>
                {
                    Some(&entry.schema)
                }
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(schema) = schema else {
            panic!("explicit input schema")
        };
        assert!(schema.directly_selectable);
        for complete in [
            schema.declarations.parameters.is_complete(),
            schema.declarations.choices.is_complete(),
            schema.declarations.grants.is_complete(),
            schema.declarations.actors.is_complete(),
            schema.declarations.skill_grants.is_complete(),
            schema.declarations.outputs.is_complete(),
            schema.declarations.sockets.is_complete(),
        ] {
            assert!(!complete, "raw inputs do not prove another inventory");
        }
        for input in &row.parameters {
            assert!(schema.declarations.parameters.members.contains(&input.slot));
            let slot = migration
                .schema
                .iter()
                .find_map(|entry| match entry {
                    SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(entry))
                        if entry.id == input.slot =>
                    {
                        Some(&entry.schema)
                    }
                    _ => None,
                })
                .unwrap();
            let SchemaState::Known(slot) = slot else {
                panic!("explicit parameter schema")
            };
            assert_eq!(
                slot.skill_input,
                Some(SkillInputAuthority::AuthoredOrProjected)
            );
        }
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        authoring["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], source.revision);
    assert_eq!(json!(catalog), authoring["catalog"]);
    for name in ["migration", "inputs", "bindings"] {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(data().join(format!("{name}.json"))).unwrap())
            ),
            authoring["artifact_sha256"][name]
        );
    }
    let catalog: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap();
    for row in skills {
        let matched: Vec<_> = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|gem| gem["game_id"] == row.game_id && gem["variant_id"] == row.variant_id)
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0]["primary_effect_id"], row.skill_id);
        assert_eq!(
            matched[0]["additional_effects"].as_array().unwrap().len(),
            1
        );
        let skill = catalog["skills"]
            .as_array()
            .unwrap()
            .iter()
            .find(|skill| skill["id"] == row.skill_id)
            .unwrap();
        assert_eq!(skill["from_tree"], true);
        assert_ne!(skill["support"], true);
    }
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root().join(proof["evidence_off"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on"].as_str().unwrap())).unwrap();
    assert!(off == on, "complete source reports agree across JIT modes");
    assert_eq!(off.len() as u64, proof["bytes"].as_u64().unwrap());
    assert_eq!(format!("{:x}", Sha256::digest(&off)), proof["sha256"]);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    source_proof(&authoring);
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let migrated = compile_owned_release_migration(prior, migration, Default::default()).unwrap();
    let mut inputs: DirectSkillInputPolicy = read("inputs.json");
    let DirectSkillInputPolicy::PobManualDirectSkillV1 {
        definitions,
        roles,
        source,
        catalog,
        ..
    } = &mut inputs
    else {
        panic!("fixture requires Direct V1")
    };
    assert_eq!(*definitions, *prior.assembled().schema().identity());
    assert_eq!(*roles, *prior.roles().identity());
    assert_eq!(*source, prior.roles().input().compilation.source);
    assert_eq!(*catalog, prior.roles().input().compilation.catalog_digest);
    *definitions = migrated.assembled().schema().identity().clone();
    *roles = *migrated.roles().identity();
    let b = migrated.input();
    let mut normalization = b.normalization.clone();
    assert!(normalization.direct_skill_inputs.is_none());
    normalization.direct_skill_inputs = Some(inputs);
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: b.recipe.clone(),
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        b.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    assert!(full.query_sets == prior.input().query_sets);
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("manual-direct-raw-input-source-policy"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-direct-skill-input-authoring-v1",
            &(authoring, read::<Value>("inputs.json")),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
