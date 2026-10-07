//! Publish a real Player input and its three native contributions together.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationDefaultInput, ConfigurationInputsPolicy},
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
use std::{fs, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/configuration-resistance-penalty")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let closure: Value = read("closure.json");
    let dependencies: Value = read("dependencies.json");
    let input: ConfigurationDefaultInput = read("input.json");
    let b = prior.input();
    assert_eq!(
        prior.receipt().input.to_string(),
        "b559c6c55aedfe5cf6e947ca210103c14444647b42f5f0483a32d5b7fa8c3d88"
    );
    assert_eq!(b.recipe.registry.last_issued.get(), 0x334c);
    assert!(b.evaluation.is_none());
    for row in dependencies["definitions"].as_array().unwrap() {
        assert!(b.recipe.schema.definitions.iter().any(|d| json!(d) == *row));
    }
    assert!(
        b.recipe
            .rules
            .owners
            .iter()
            .any(|d| json!(d) == dependencies["actor_owner"])
    );
    let mut owners: Vec<poe_optimizer_core::owned_rules::DefinitionRules> =
        serde_json::from_value(closure["owners"].clone()).unwrap();
    let old_owner: poe_optimizer_core::owned_rules::DefinitionRules =
        serde_json::from_value(dependencies["actor_owner"].clone()).unwrap();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].owner, old_owner.owner);
    assert_eq!(owners[0].programs.closure, old_owner.programs.closure);
    assert_eq!(
        &owners[0].programs.members[..old_owner.programs.members.len()],
        &old_owner.programs.members
    );
    owners[0]
        .programs
        .members
        .drain(..old_owner.programs.members.len());
    let extension = OwnedRecipeExtension {
        schema_version: 1,
        version: key("pob-3887ae68-player-resistance-penalty-v1"),
        schema: serde_json::from_value::<Vec<_>>(closure["definitions"].clone())
            .unwrap()
            .into_iter()
            .map(SchemaExtensionEntry::Definition)
            .collect(),
        operations_version: None,
        tables: vec![],
        owners,
        receivers: vec![],
    };
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 1);
    assert_eq!(extended.receipt.refined_subjects, 1);
    assert_eq!(extended.receipt.appended_programs, 1);
    assert_eq!(extended.receipt.appended_tables, 0);
    assert_eq!(extended.receipt.appended_receivers, 0);
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
        extended.refinement.unwrap(),
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
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut full.normalization.configuration_inputs
    else {
        panic!("current configuration policy")
    };
    assert!(default_inputs.is_empty());
    default_inputs.push(input);
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
        kind: key("configured-player-resistance-penalty"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-resistance-penalty-authoring-v1",
            &(
                closure,
                dependencies,
                read::<Value>("input.json"),
                read::<Value>("source-vectors.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    // Reverse the exact schema and rule additions; all other numerical records stay exact.
    let schema = &next.input().recipe.schema;
    assert_eq!(
        schema.definitions.len(),
        b.recipe.schema.definitions.len() + 1
    );
    // The source-independent refinement API already checks membership. Compare
    // every old descriptor, allowing only this declared encounter refinement.
    for old in &b.recipe.schema.definitions {
        let actual = schema
            .definitions
            .iter()
            .find(|d| d.address() == old.address())
            .unwrap();
        if let Some(SchemaExtensionEntry::Definition(changed)) = extension.schema.iter().find(|d| {
            d.subject()
                == poe_optimizer_core::owned_schema::SchemaSubject::Definition(old.address())
        }) {
            assert_eq!(actual, changed);
        } else {
            assert_eq!(actual, old);
        }
    }
    let added = &extension.owners[0];
    for old in &b.recipe.rules.owners {
        let actual = next
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == old.owner)
            .unwrap();
        let mut expected = old.clone();
        if old.owner == added.owner {
            expected
                .programs
                .members
                .extend(added.programs.members.clone());
        }
        assert_eq!(actual, &expected);
    }
    assert_eq!(next.input().recipe.rules.tables, b.recipe.rules.tables);
    assert_eq!(
        next.input().recipe.rules.effect_applications,
        b.recipe.rules.effect_applications
    );
    assert_eq!(next.input().query_sets, b.query_sets);
    next
}
