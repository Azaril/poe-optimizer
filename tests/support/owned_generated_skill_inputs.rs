//! Source-authenticated generated raw inputs; no numerical coverage is inferred.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::GeneratedSkillInputPolicy,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/generated-preset-inputs-v1")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn check_authored() {
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let policy: GeneratedSkillInputPolicy = read("generated-inputs.json");
    let authoring: Value = read("authoring.json");
    assert_eq!(migration.schema_version, 4);
    assert_eq!(migration.contract.schema_version, 6);
    assert_eq!(
        migration.contract.operations_version.as_str(),
        "owned-domain-operations-v19"
    );
    assert_eq!(migration.schema.len(), 5);
    assert!(migration.tables.is_empty() && migration.owners.is_empty());
    assert!(migration.receivers.is_empty() && migration.query_targets.is_empty());
    assert!(migration.evaluation.is_none());
    let GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { rows, source, .. } = policy;
    assert_eq!(
        rows.len(),
        4,
        "two tree supplies and two actual item-name frames"
    );
    assert!(rows.iter().all(|row| row.parameters.len() == 1));
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&manifest_bytes)),
        authoring["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], source.revision);
    for section in ["source_files", "source_fact_files"] {
        for file in authoring[section].as_array().unwrap() {
            let pinned = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["path"] == file["path"])
                .unwrap();
            assert_eq!(pinned["sha256"], file["sha256"]);
            if let Some(bytes) = file.get("bytes") {
                assert_eq!(&pinned["bytes"], bytes);
            }
        }
    }
    for (name, expected) in authoring["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), *expected, "{name}");
    }
}

fn source_proof() {
    let authoring: Value = read("authoring.json");
    let vectors: Value = read("source-vectors.json");
    let reports = vectors["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    let mut previous = None;
    for report in reports {
        let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(digest, report["sha256"]);
        assert_eq!(bytes.len() as u64, report["bytes"].as_u64().unwrap());
        if let Some(previous) = &previous {
            assert_eq!(previous, &digest);
        }
        previous = Some(digest);
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(document["files"], authoring["source_files"]);
        assert_eq!(document["source_revision"], authoring["source_revision"]);
        assert_eq!(
            document["manifest_sha256"],
            authoring["source_manifest_sha256"]
        );
        for observation in report["observations"].as_array().unwrap() {
            assert_eq!(
                document
                    .pointer(observation["pointer"].as_str().unwrap())
                    .unwrap(),
                &observation["value"]
            );
        }
    }
    let facts: Value = read("source-facts.json");
    assert_eq!(facts["source_revision"], authoring["source_revision"]);
    assert_eq!(
        facts["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(facts["originals"].as_array().unwrap().len(), 5);
    for original in facts["originals"].as_array().unwrap() {
        let bytes = fs::read(root().join(original["path"].as_str().unwrap())).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), original["sha256"]);
    }
    for excerpt in facts["excerpts"].as_array().unwrap() {
        let file = &excerpt["file"];
        assert!(
            authoring["source_fact_files"]
                .as_array()
                .unwrap()
                .contains(file)
        );
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(file["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(text.len() as u64, file["bytes"].as_u64().unwrap());
        assert_eq!(
            format!("{:x}", Sha256::digest(text.as_bytes())),
            file["sha256"]
        );
        let first = excerpt["first_line"].as_u64().unwrap() as usize;
        let last = excerpt["last_line"].as_u64().unwrap() as usize;
        assert_eq!(
            json!(
                text.lines()
                    .skip(first - 1)
                    .take(last - first + 1)
                    .collect::<Vec<_>>()
            ),
            excerpt["lines"]
        );
    }
}

fn minimal_schema_delta(prior: &StagedOwnedRelease, migrated: &StagedOwnedRelease) {
    let before = json!(prior.input().recipe.schema);
    let mut after = json!(migrated.input().recipe.schema);
    let mut grants = 0;
    let mut added = 0;
    after["slots"].as_array_mut().unwrap().retain_mut(|slot| {
        let id = &slot["value"]["id"];
        let old = before["slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|old| old["value"]["id"] == *id);
        if let Some(old) = old {
            if slot != old {
                assert_eq!(slot["kind"], "skill_grant");
                let permission = slot["value"]["schema"]["value"]
                    .as_object_mut()
                    .unwrap()
                    .remove("preset_inputs")
                    .unwrap();
                assert_eq!(permission["schema_version"], 1);
                assert_eq!(permission["parameters"]["closure"]["kind"], "complete");
                assert_eq!(
                    permission["parameters"]["members"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
                assert_eq!(slot, old);
                grants += 1;
            }
            true
        } else {
            assert_eq!(slot["kind"], "parameter");
            assert_eq!(id["slot"]["key"], "def.00000000000032ef");
            assert_eq!(slot["value"]["schema"]["value"]["skill_input"], "projected");
            assert_eq!(
                slot["value"]["schema"]["value"]["presence"],
                "required_once"
            );
            added += 1;
            false
        }
    });
    assert_eq!((grants, added), (3, 1));
    let mut skills = 0;
    for skill in after["definitions"].as_array_mut().unwrap() {
        let old = before["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|old| old["value"]["id"] == skill["value"]["id"])
            .unwrap();
        if skill != old {
            assert_eq!(skill["value"]["id"]["key"], "def.0000000000000134");
            let members =
                skill["value"]["schema"]["value"]["declarations"]["parameters"]["members"]
                    .as_array_mut()
                    .unwrap();
            assert_eq!(members.len(), 2);
            assert_eq!(
                members.pop().unwrap()["slot"]["key"],
                "def.00000000000032ef"
            );
            assert_eq!(skill, old, "every existing Partial facet survives");
            skills += 1;
        }
    }
    assert_eq!(skills, 1);
    assert!(
        after["slots"] == before["slots"],
        "existing slot inventory is unchanged"
    );
    assert!(
        after["definitions"] == before["definitions"],
        "existing definition inventory is unchanged"
    );
    assert_eq!(
        prior.input().recipe.rules.owners,
        migrated.input().recipe.rules.owners
    );
    assert_eq!(
        prior.input().recipe.rules.tables,
        migrated.input().recipe.rules.tables
    );
    assert_eq!(
        prior.input().recipe.rules.receivers,
        migrated.input().recipe.rules.receivers
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let authoring: Value = read("authoring.json");
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    let dependencies: Value = read("dependencies.json");
    let schema = json!(prior.input().recipe.schema);
    for field in ["definitions", "slots"] {
        for row in dependencies[field].as_array().unwrap() {
            assert!(
                schema[field].as_array().unwrap().contains(row),
                "exact prior {field}"
            );
        }
    }
    let rules = json!(prior.input().recipe.rules);
    for row in dependencies["owners"].as_array().unwrap() {
        assert!(
            rules["owners"].as_array().unwrap().contains(row),
            "exact provider body"
        );
    }
    let migrated =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    minimal_schema_delta(prior, &migrated);
    super::migration_preservation::assert_import_rebindings_only(prior, &migrated);
    let mut policy: GeneratedSkillInputPolicy = read("generated-inputs.json");
    let GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 {
        definitions,
        roles,
        source,
        catalog,
        ..
    } = &mut policy;
    assert_eq!(*definitions, prior.receipt().definitions);
    assert_eq!(*roles, *prior.roles().identity());
    assert_eq!(*source, prior.roles().input().compilation.source);
    assert_eq!(*catalog, prior.roles().input().compilation.catalog_digest);
    *definitions = migrated.receipt().definitions.clone();
    *roles = *migrated.roles().identity();
    let base = migrated.input();
    let mut normalization = base.normalization.clone();
    assert!(normalization.generated_skill_inputs.is_none());
    normalization.generated_skill_inputs = Some(policy);
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: base.recipe.clone(),
            successor: base.recipe.clone(),
            mapping: base.mapping.clone(),
            roles: base.roles.clone(),
            normalization: base.normalization.clone(),
            rewards: base.rewards.clone(),
            query_sets: base.query_sets.clone(),
            items: base.items.clone(),
            item_source: base.item_source.clone(),
        },
        base.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = base.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        base.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("generated-preset-raw-input-source-policy"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-generated-preset-input-authoring-v1",
            &(authoring, read::<Value>("generated-inputs.json")),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let mut restored = full.clone();
    restored.normalization = base.normalization.clone();
    restored.tree = base.tree.clone();
    restored.provenance.pop();
    assert!(restored == *base);
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.input().query_sets == prior.input().query_sets);
    assert!(next.evaluation().is_none());
    next
}
