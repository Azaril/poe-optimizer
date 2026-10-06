//! Explicit offline publication of a source-bound primary-effect preference.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{UsageInputPolicy, gem_inventory_scalar_inputs_identity},
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/skill-usage-inputs")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn check_authored() {
    let extension: OwnedRecipeExtension = read("extension.json");
    assert_eq!(extension.schema.len(), 2);
    assert_eq!(extension.owners.len(), 1);
    assert_eq!(extension.owners[0].programs.members.len(), 1);
    assert!(extension.owners[0].programs.is_complete());
    assert!(extension.tables.is_empty());
    assert!(extension.receivers.is_empty());
    let policy: UsageInputPolicy = read("policy.json");
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical: gems, .. } = policy;
    assert_eq!(gems.len(), 1);
    assert_eq!(gems[0].policies.len(), 1);
    assert_eq!(gems[0].policies[0].parameters.len(), 1);
    let native: Value = read("native-inputs.json");
    assert_eq!(native["policy"], json!(gems[0].policies[0].policy));
    let authoring: Value = read("authoring.json");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    assert_eq!(
        authoring["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
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
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(
        off == on,
        "exact full-source observations in both JIT modes"
    );
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(
        evidence["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(evidence["native_build_parity"], false);
    assert_eq!(evidence["native_inventory_authority"], false);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(
            evidence["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
        );
    }
    for case in 1..=5 {
        let bytes = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        assert_eq!(
            evidence["original_xml_sha256"][case - 1],
            format!("{:x}", Sha256::digest(&bytes))
        );
    }
    let vectors =
        fs::read(root().join("data/owned/poe2/3887ae68/skill-usage-inputs/reference-vectors.json"))
            .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(vectors)),
        proof["reference_vectors_sha256"]
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    source_proof(&authoring);
    let receipt = json!(prior.receipt());
    assert_eq!(authoring["before"], receipt["input"]);
    for field in ["definitions", "registry"] {
        assert_eq!(authoring[field], receipt[field]);
    }
    let b = prior.input();
    assert!(b.evaluation.is_none());
    assert!(b.normalization.usage_inputs.is_none());
    let extension: OwnedRecipeExtension = read("extension.json");
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert!(extended.refinement.is_none());
    assert_eq!(extended.receipt.allocated_entries, 2);
    assert_eq!(extended.receipt.appended_programs, 1);
    assert_eq!(extended.receipt.appended_tables, 0);
    let carried = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: extended.successor,
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    let mut policy: UsageInputPolicy = read("policy.json");
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions,
        source,
        roles,
        catalog,
        scalar_inputs,
        ..
    } = &mut policy;
    // This is explicit authoring against one checked predecessor, not runtime
    // repair of stale policies. The published policy carries its final bindings.
    assert_eq!(definitions, prior.assembled().schema().identity());
    assert_eq!(*source, prior.roles().input().compilation.source);
    assert_eq!(roles, prior.roles().identity());
    assert_eq!(*catalog, b.roles.compilation.catalog_digest);
    assert_eq!(
        *scalar_inputs,
        gem_inventory_scalar_inputs_identity(&b.normalization, Default::default()).unwrap()
    );
    *definitions = carried.assembled().schema().identity().clone();
    *roles = *carried.roles().identity();
    *scalar_inputs =
        gem_inventory_scalar_inputs_identity(&full.normalization, Default::default()).unwrap();
    full.normalization.usage_inputs = Some(policy);
    full.normalization.version = key("physical-primary-skill-usage-v1");
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            b.tree.as_ref().unwrap().content.clone(),
            carried.assembled().registry(),
            carried.assembled().schema(),
            carried.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("physical-primary-skill-usage"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-skill-usage-inputs-authoring-v1",
            &(
                authoring,
                &extension,
                read::<UsageInputPolicy>("policy.json"),
                read::<Value>("native-inputs.json"),
                read::<Value>("reference-vectors.json"),
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.input().query_sets, b.query_sets);
    assert_eq!(
        next.input().recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries
    );
    assert_eq!(
        next.input().recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 2
    );
    // Checked packages sort descriptor kinds canonically; new members need not
    // sit at the end. Remove exactly the authenticated additions before comparing.
    let mut restored_schema = next.input().recipe.schema.clone();
    for addition in &extension.schema {
        match addition {
            SchemaExtensionEntry::Definition(definition) => {
                let at = restored_schema
                    .definitions
                    .iter()
                    .position(|row| row.address() == definition.address())
                    .unwrap();
                assert_eq!(restored_schema.definitions.remove(at), *definition);
            }
            SchemaExtensionEntry::Slot(slot) => {
                let at = restored_schema
                    .slots
                    .iter()
                    .position(|row| row.address() == slot.address())
                    .unwrap();
                assert_eq!(restored_schema.slots.remove(at), *slot);
            }
        }
    }
    assert!(
        restored_schema == b.recipe.schema,
        "all prior schema entries and order remain intact"
    );
    let mut restored_owners = next.input().recipe.rules.owners.clone();
    let addition = &extension.owners[0];
    let at = restored_owners
        .iter()
        .position(|row| row.owner == addition.owner)
        .unwrap();
    assert_eq!(restored_owners.remove(at), *addition);
    assert!(
        restored_owners == b.recipe.rules.owners,
        "all prior rule owners remain intact"
    );
    assert_eq!(next.input().recipe.rules.tables, b.recipe.rules.tables);
    assert_eq!(
        next.input().recipe.rules.effect_applications,
        b.recipe.rules.effect_applications
    );
    assert_eq!(
        next.input().recipe.rules.operations_version,
        b.recipe.rules.operations_version
    );
    let mut restored = next.input().normalization.clone();
    restored.usage_inputs = None;
    restored.version = b.normalization.version.clone();
    assert_eq!(
        &restored,
        carried.normalization(),
        "only explicit usage policy added to checked carry-forward"
    );
    next
}
