//! Publish numeric usage without converting it into an inventory-completeness proof.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, PrimarySkillUsageInput, UsageInputPolicy, usage_inputs_identity,
    },
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
    root().join("data/owned/poe2/3887ae68/sniper-reservation")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

pub fn check_authored() {
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let usage: PrimarySkillUsageInput = read("usage.json");
    let authoring: Value = read("authoring.json");
    assert_eq!(migration.schema_version, 3);
    assert_eq!(migration.contract.schema_version, 5);
    assert_eq!(
        migration.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert!(
        migration.tables.is_empty(),
        "reuse the existing source level table"
    );
    assert!(migration.receivers.is_empty());
    assert!(migration.query_targets.is_empty());
    assert!(migration.evaluation.is_none());
    assert_eq!(usage.policies.len(), 1);
    assert_eq!(usage.policies[0].parameters.len(), 1);
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(
        authoring["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    for (name, expected) in authoring["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'), "owned authoring bytes use LF");
        assert_eq!(
            expected,
            &json!(format!("{:x}", Sha256::digest(bytes))),
            "{name}"
        );
    }
    assert_eq!(authoring["usage_inventory"], "pending");
    assert_eq!(authoring["native_mechanics_coverage"], "partial");
    // This is offline acquisition evidence. Runtime normalization consumes the
    // explicit reviewed identities, not PoB tables or additional-effect logic.
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_slice(
            &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json"))
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        authoring["catalog"],
        json!(
            digest_owned(
                "owned-skill-source-catalog-v1",
                catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        )
    );
    let catalog = json!(catalog.data());
    let value = json!(usage);
    let fallback = &value["policies"][0]["parameters"][0]["source"]["fallback_admission"];
    assert_eq!(fallback["kind"], "unique_reviewed_primary");
    for companion in fallback["companions"].as_array().unwrap() {
        let rows: Vec<_> = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["game_id"] == companion["game_id"]
                    && row["variant_id"] == companion["variant_id"]
            })
            .collect();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row["primary_effect_id"], companion["skill_id"]);
        let source_name = row["name_spec"]
            .as_str()
            .unwrap_or_else(|| row["name"].as_str().unwrap());
        assert_eq!(companion["name_spec"], source_name);
        assert_ne!(row["primary_effect_id"], json!(usage.skill_id));
        assert_ne!(row["primary_effect_id"], Value::Null);
        assert!(
            row["effect_list"]
                .as_array()
                .unwrap()
                .iter()
                .all(|effect| effect != &json!(usage.skill_id))
        );
        assert!(
            row["additional_effects"]
                .as_array()
                .unwrap()
                .iter()
                .all(|effect| effect != &json!(usage.skill_id))
        );
        for field in [
            "declared_additional_effects",
            "declared_additional_stat_sets",
            "constructed_additional_effects",
        ] {
            assert!(
                row[field]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|effect| effect["id"] != json!(usage.skill_id)),
                "review every declared or unresolved companion effect: {field}"
            );
        }
    }
}

fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let mut reports = vec![];
    for (path, size, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let bytes = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, proof[size].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), proof[hash]);
        reports.push(bytes);
    }
    assert!(
        reports[0] == reports[1],
        "JIT modes preserve the complete source report"
    );
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(
        report["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(report["source_revision"], authoring["source_revision"]);
    assert_eq!(report["catalog_digest"], authoring["catalog"]);
    assert_eq!(report["native_inventory_authority"], false);
    assert_eq!(report["native_build_parity"], false);
    assert_eq!(report["canonical_lifecycle_selected"], false);
    assert_eq!(report["business_wrappers"], false);
    assert_eq!(report["copied_arithmetic_as_evidence"], false);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert_eq!(
            report["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let cases = report["cases"].as_array().unwrap();
    let expected = [
        "original",
        "count-zero",
        "count-two",
        "count-three",
        "count-four",
        "group-zero",
        "group-four",
        "raw-level-one",
        "raw-level-nineteen",
        "duplicates-one-three",
        "duplicates-three-one",
        "reduced-reservation-seven",
        "efficiency-seven",
        "less-reservation-seven",
        "combined-modifiers",
        "added-hulking-support",
        "warrior-free-count-1",
        "warrior-free-count-2",
        "warrior-free-count-3",
        "original-01",
        "repeat-original",
    ];
    assert_eq!(cases.len(), expected.len());
    for (case, name) in cases.iter().zip(expected) {
        assert_eq!(case["name"], name);
        assert_eq!(case["states"].as_object().unwrap().len(), 3);
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            assert!(case["states"][stage].is_object());
            for flag in [
                "source_methods_preserved",
                "hook_removed",
                "physical_objects_preserved",
            ] {
                assert_eq!(case["states"][stage][flag], true);
            }
        }
    }
    for (index, original) in [(0, 5), (19, 1), (20, 5)] {
        let bytes = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{original:02}.xml"
        )))
        .unwrap();
        assert_eq!(
            cases[index]["xml_sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    assert_eq!(cases[0]["states"], cases[20]["states"]);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    source_proof(&authoring);
    let receipt = json!(prior.receipt());
    for field in ["definitions", "registry", "normalization", "roles"] {
        assert_eq!(
            authoring[field], receipt[field],
            "exact predecessor {field}"
        );
    }
    assert_eq!(authoring["before"], receipt["input"]);
    assert_eq!(
        authoring["catalog"],
        json!(prior.roles().input().compilation.catalog_digest)
    );
    let migrated =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let b = migrated.input();
    let mut normalization = b.normalization.clone();
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. }) =
        &mut normalization.usage_inputs
    else {
        panic!("exact predecessor has the current occurrence usage contract")
    };
    assert_eq!(physical.len(), 2);
    physical.push(read("usage.json"));
    let usage_identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV2 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("retain the existing physical inventory rules")
    };
    *usage_inputs = usage_identity;
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
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("sniper-count-and-flat-spirit-reservation"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-sniper-reservation-authoring-v1",
            &(authoring, read::<Value>("usage.json")),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.input().query_sets == prior.input().query_sets);
    assert!(next.evaluation().is_none());
    assert_eq!(
        next.input().recipe.rules.effect_applications,
        prior.input().recipe.rules.effect_applications
    );
    let mut restored = next.input().clone();
    restored.normalization = b.normalization.clone();
    restored.tree = b.tree.clone();
    restored.provenance.pop();
    assert!(
        restored == *b,
        "only numeric usage and its commitments change after migration"
    );
    next
}
