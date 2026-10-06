//! Checked source transport only: no population or Full DPS completeness claim.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, OccurrenceUsageRule, UsageInputPolicy, usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/occurrence-counts-v1")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn digest(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let rules: Vec<OccurrenceUsageRule> = read("usage.json");
    assert_eq!(rules.len(), 6);
    let rules = json!(rules);
    assert_eq!(
        rules
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["target"]["kind"] == "authored_direct")
            .count(),
        2
    );
    for (key, filename) in [
        ("usage", "usage"),
        ("companion_evidence", "companion-evidence"),
        ("source_facts", "source-facts"),
        ("source_vectors", "source-vectors"),
    ] {
        assert_eq!(
            a["artifact_sha256"][key],
            digest(data().join(format!("{filename}.json")))
        );
    }
    for field in ["new_definitions", "new_slots", "new_programs", "new_tables"] {
        assert_eq!(a[field], 0);
    }
    assert_eq!(a["usage_inventory"], "pending");
    assert_eq!(a["full_dps_aggregation"], "unimplemented");
    let manifest_path = root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    assert_eq!(a["source_manifest_sha256"], digest(&manifest_path));
    let manifest: Value = serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let facts: Value = read("source-facts.json");
    assert_eq!(facts["source_revision"], a["source_revision"]);
    for e in facts["excerpts"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(&e["file"]));
        assert_eq!(
            e["lines"].as_array().unwrap().len() as u64,
            e["last_line"].as_u64().unwrap() - e["first_line"].as_u64().unwrap() + 1
        );
    }
    let catalog: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap();
    let negative: Value = read("companion-evidence.json");
    for row in negative.as_array().unwrap() {
        assert!(
            catalog["gems"]
                .as_array()
                .unwrap()
                .contains(&row["catalog"])
        );
        for field in [
            "effect_list",
            "additional_effects",
            "declared_additional_effects",
            "constructed_additional_effects",
        ] {
            if let Some(effects) = row["catalog"][field].as_array() {
                assert!(effects.iter().all(|effect| effect != &row["target"]));
            }
        }
    }
    for row in rules.as_array().unwrap() {
        assert_eq!(row["policies"].as_array().unwrap().len(), 1);
        let policy = &row["policies"][0];
        assert_eq!(policy["policy"]["key"], "def.000000000000326a");
        assert_eq!(policy["parameters"].as_array().unwrap().len(), 1);
        assert_eq!(
            policy["parameters"][0]["slot"]["slot"]["key"],
            "def.000000000000326b"
        );
        let source = &policy["parameters"][0]["source"];
        assert_eq!(source["kind"], "containing_group_override");
        assert_eq!(source["group"]["missing"]["kind"], "pending");
        assert_eq!(source["occurrence"]["missing"]["kind"], "pending");
        assert_eq!(
            source["fallback_admission"]["kind"],
            "unique_reviewed_primary"
        );
        if row["target"]["kind"] == "generated" {
            assert_eq!(
                source["occurrence"]["numeric_aliases"],
                json!([{"token":"nil","replacement":"1"}])
            );
            assert!(
                source["fallback_admission"]["companions"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        } else {
            assert!(source["occurrence"].get("numeric_aliases").is_none());
            for c in source["fallback_admission"]["companions"]
                .as_array()
                .unwrap()
            {
                assert!(
                    negative
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|n| n["target"] == row["target"]["skill_id"]
                            && n["catalog"]["game_id"] == c["game_id"]
                            && n["catalog"]["variant_id"] == c["variant_id"])
                );
            }
        }
    }
}
fn source_proof() {
    let a: Value = read("authoring.json");
    let vectors: Value = read("source-vectors.json");
    let mut previous = None;
    for report in vectors["reports"].as_array().unwrap() {
        let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, report["bytes"]);
        let hash = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(hash, report["sha256"]);
        if let Some(prior) = &previous {
            assert_eq!(prior, &hash);
        }
        previous = Some(hash);
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(document["source_revision"], a["source_revision"]);
        assert_eq!(document["manifest_sha256"], a["source_manifest_sha256"]);
        assert_eq!(document["files"], a["source_files"]);
        for row in vectors["observations"].as_array().unwrap() {
            assert_eq!(
                document.pointer(row["pointer"].as_str().unwrap()).unwrap(),
                &row["value"]
            );
        }
    }
    let facts: Value = read("source-facts.json");
    for e in facts["excerpts"].as_array().unwrap() {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(e["file"]["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(
            format!("{:x}", Sha256::digest(text.as_bytes())),
            e["file"]["sha256"]
        );
        let first = e["first_line"].as_u64().unwrap() as usize;
        let last = e["last_line"].as_u64().unwrap() as usize;
        assert_eq!(
            json!(
                text.lines()
                    .skip(first - 1)
                    .take(last - first + 1)
                    .collect::<Vec<_>>()
            ),
            e["lines"]
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    assert_eq!(a["before"], json!(prior.receipt().input));
    let b = prior.input();
    let mut normalization = b.normalization.clone();
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        occurrences,
        source,
        ..
    }) = &mut normalization.usage_inputs
    else {
        panic!("current occurrence usage predecessor")
    };
    assert!(occurrences.is_empty());
    assert_eq!(*source, prior.roles().input().compilation.source);
    *occurrences = read("usage.json");
    let identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("V3 inventory")
    };
    *usage_inputs = identity;
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
        kind: OwnedDefinitionKey::new("occurrence-requested-counts").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-occurrence-requested-counts-v1",
            &(a, read::<Value>("usage.json")),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization = b.normalization.clone();
    restored.tree = b.tree.clone();
    restored.provenance.pop();
    assert!(
        restored == *b,
        "only import policy, its tree commitment and provenance change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
