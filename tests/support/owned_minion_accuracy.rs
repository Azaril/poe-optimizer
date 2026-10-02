//! Checked joint publication of raw block configuration and native action consumers.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey,
    owned_routing::ActionOutputRoutes, owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_normalize::ConfigurationInputsPolicy,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(family: &str, file: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68")
                .join(family)
                .join(file),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn policy() -> ConfigurationInputsPolicy {
    read("configuration-block-inputs", "policy.json")
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn check_authored() {
    let ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
        inputs,
        placeholder_fallback_inputs,
        ..
    } = policy()
    else {
        panic!("explicit fallback adapter");
    };
    assert_eq!(inputs.len(), 6);
    assert_eq!(placeholder_fallback_inputs.len(), 1);
    assert_eq!(
        placeholder_fallback_inputs[0].source_name,
        "enemyBlockChance"
    );
    let native: OwnedRecipeExtension = read("minion-accuracy", "extension.json");
    assert!(!native.schema.is_empty());
    assert!(!native.owners.is_empty());
    assert!(
        native
            .owners
            .iter()
            .all(|owner| !owner.programs.is_complete())
    );
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(
        off == on,
        "exact complete-source evidence in both JIT modes"
    );
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(evidence["source_revision"], authoring["source_revision"]);
    assert_eq!(evidence["source_hash"], authoring["source_manifest_sha256"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(
            evidence["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|actual| actual["path"] == pin["path"] && actual["sha256"] == pin["sha256"])
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let config_authoring: Value = read("configuration-block-inputs", "authoring.json");
    let native_authoring: Value = read("minion-accuracy", "authoring.json");
    for authoring in [&config_authoring, &native_authoring] {
        assert_eq!(authoring["before"], json!(prior.receipt().input));
        source_proof(authoring);
    }
    let config: OwnedRecipeExtension = read("configuration-block-inputs", "extension.json");
    let native: OwnedRecipeExtension = read("minion-accuracy", "extension.json");
    for family in ["configuration-block-inputs", "minion-accuracy"] {
        let dependencies: Vec<DefinitionDescriptor> = read(family, "dependencies.json");
        for definition in dependencies {
            let in_prior = prior
                .input()
                .recipe
                .schema
                .definitions
                .contains(&definition);
            let in_config = family == "minion-accuracy" && config.schema.iter().any(|entry| {
                matches!(entry, SchemaExtensionEntry::Definition(actual) if actual == &definition)
            });
            assert!(in_prior || in_config, "exact typed dependency in {family}");
        }
    }
    let mut combined = config.clone();
    combined.version = key("native-minion-hit-chance-with-block-configuration-v1");
    assert_eq!(combined.operations_version, native.operations_version);
    combined.schema.extend(native.schema.clone());
    combined.tables.extend(native.tables.clone());
    combined.receivers.extend(native.receivers.clone());
    for owner in &native.owners {
        if let Some(existing) = combined
            .owners
            .iter_mut()
            .find(|row| row.owner == owner.owner)
        {
            assert_eq!(existing.programs.closure, owner.programs.closure);
            existing
                .programs
                .members
                .extend(owner.programs.members.clone());
        } else {
            combined.owners.push(owner.clone());
        }
    }
    let mut extended =
        extend_owned_recipe(prior.assembled(), &combined, Default::default()).unwrap();
    let routes: Vec<ActionOutputRoutes> = read("minion-accuracy", "routes.json");
    for added in &routes {
        let target = extended
            .successor
            .routing
            .outputs
            .iter_mut()
            .find(|o| o.output == added.output)
            .unwrap();
        assert_eq!(target.routes.closure, added.routes.closure);
        assert_eq!(target.source_selectors, added.source_selectors);
        assert!(!target.routes.is_complete());
        target.routes.members.extend(added.routes.members.clone());
    }
    let b = prior.input();
    let carried = transition_owned_catalog_with_membership_refinement_compact(
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
        extended
            .refinement
            .expect("explicit encounter input membership growth"),
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
    full.normalization.configuration_inputs = Some(policy());
    full.normalization.version = key("selected-configuration-block-inputs-v2");
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
        kind: key("native-minion-hit-chance-with-block-configuration"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-minion-hit-chance-publication-v1",
            &(
                config_authoring,
                native_authoring,
                combined,
                routes,
                policy(),
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert_eq!(next.input().query_sets, prior.input().query_sets);
    assert_eq!(
        next.mapping().input().entries,
        prior.mapping().input().entries
    );
    next
}
