//! Actual persisted catalog/recipe integration; no source checkout or VM.
use poe_optimizer_core::owned_build::ParameterValue;
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use poe_optimizer_import::{
    owned_catalog_recipe::*,
    owned_item_lines::{ItemLinePolicyInput, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput},
    owned_mapping::*,
    owned_normalize::{GemQualityPolicy, NormalizationPolicy},
    owned_recipe::*,
    owned_reward_policy::{OwnedRewardPolicy, RewardPolicyInput},
    owned_skill_roles::*,
};
use serde::{Deserialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn dir() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/import")
}
fn load<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&std::fs::read(dir().join(name)).unwrap()).unwrap()
}
fn inputs() -> (
    OwnedRecipeInput,
    MappingPackageInput,
    SkillIdentityCatalog,
    SourcePin,
    SkillCatalogPolicy,
) {
    (
        load("recipe-seed.json"),
        load("mapping-seed.json"),
        SkillIdentityCatalog::new(load::<SkillIdentityData>("skill-identities.json")).unwrap(),
        load("source-pin.json"),
        load("skill-catalog-policy.json"),
    )
}
fn first() -> StagedCatalogRecipe {
    let (recipe, mapping, catalog, source, policy) = inputs();
    extend_owned_catalog_recipe(
        recipe,
        mapping,
        &catalog,
        &source,
        &policy,
        CatalogRecipeLimits::default(),
    )
    .unwrap()
}
#[test]
fn production_extension_reproduces_persisted_bundle_and_preserves_reviewed_mechanics() {
    let result = first();
    let original: OwnedRecipeInput = serde_json::from_slice(
        &std::fs::read(root().join("data/owned/poe2/3887ae68/recipe.json")).unwrap(),
    )
    .unwrap();
    let old = assemble_owned_recipe(original, OwnedRecipeLimits::default()).unwrap();
    old.registry()
        .validate_successor(result.assembled().registry())
        .unwrap();
    for entry in &old.schema().input().definitions {
        assert!(result.recipe().schema.definitions.contains(entry));
    }
    for entry in &old.schema().input().slots {
        assert!(result.recipe().schema.slots.contains(entry));
    }
    assert_eq!(old.rules().input().tables, result.recipe().rules.tables);
    assert_eq!(result.transition().counts.reused_gems, 2);
    assert_eq!(result.transition().counts.reused_skills, 4);
    assert_eq!(result.transition().counts.allocated_gems, 964);
    assert_eq!(result.transition().counts.allocated_skills, 1432);
    assert_eq!(result.roles().input().roles.len(), 966);
    assert_eq!(result.assembled().manifest().partial_rule_owners, 9);
    assert_eq!(result.assembled().manifest().partial_route_outputs, 4);
    for (name, bytes) in result.artifacts() {
        assert_eq!(
            bytes,
            std::fs::read(dir().join("compiled").join(name)).unwrap(),
            "{name}"
        );
    }
    assert_eq!(result.artifacts().count(), 9);
}

#[test]
fn archived_seed_to_bound_policies_preserve_content_and_authenticate_the_receipt() {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Artifact {
        path: String,
        bytes: usize,
        sha256: String,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Receipt {
        schema_version: u32,
        scope: String,
        before_definitions: poe_optimizer_core::data::DataIdentity,
        after_definitions: poe_optimizer_core::data::DataIdentity,
        before_mapping: poe_optimizer_core::owned_content::OwnedContentDigest,
        after_mapping: poe_optimizer_core::owned_content::OwnedContentDigest,
        compiled_transition_sha256: String,
        preserved_definition_count: usize,
        preserved_slot_count: usize,
        source_execution: bool,
        numeric_coverage: bool,
        artifacts: Vec<Artifact>,
    }

    let (recipe, mapping, _, _, _) = inputs();
    let seed = assemble_owned_recipe(recipe, Default::default()).unwrap();
    let seed_mapping =
        OwnedMappingIndex::new(mapping, seed.registry(), seed.schema(), Default::default())
            .unwrap();
    let compiled = first();
    let schema = compiled.assembled().schema();
    seed.registry()
        .validate_successor(compiled.assembled().registry())
        .unwrap();
    for row in &seed.schema().input().definitions {
        assert!(schema.input().definitions.contains(row));
    }
    for row in &seed.schema().input().slots {
        assert!(schema.input().slots.contains(row));
    }
    for row in &seed_mapping.input().entries {
        assert!(compiled.mapping().input().entries.contains(row));
    }
    let mut expected_rules = seed.rules().input().clone();
    expected_rules.definitions = schema.identity().clone();
    assert_eq!(compiled.assembled().rules().input(), &expected_rules);
    let mut expected_routing = seed.routing().input().clone();
    expected_routing.definitions = schema.identity().clone();
    assert_eq!(compiled.assembled().routing().input(), &expected_routing);

    // These retained policies have historical contract versions. Their ordinary
    // constructors still check them; current-only content extraction must not
    // silently upgrade them. Only the explicitly named dependency seals differ.
    let seed_rewards: RewardPolicyInput = load("reward-policy-seed.json");
    let seed_rewards = OwnedRewardPolicy::new(
        seed_rewards,
        &seed_mapping,
        seed.schema(),
        Default::default(),
    )
    .unwrap();
    let bound_rewards = OwnedRewardPolicy::new(
        load("policies/rewards.json"),
        compiled.mapping(),
        schema,
        Default::default(),
    )
    .unwrap();
    let mut expected_rewards = seed_rewards.input().clone();
    expected_rewards.definitions = schema.identity().clone();
    expected_rewards.mapping = *compiled.mapping().identity();
    assert_eq!(bound_rewards.input(), &expected_rewards);
    assert!(
        OwnedRewardPolicy::new(
            seed_rewards.input().clone(),
            compiled.mapping(),
            schema,
            Default::default()
        )
        .is_err()
    );

    let seed_items: ItemLinePolicyInput = load("item-policy-seed.json");
    let seed_items =
        OwnedItemLinePolicy::new(seed_items, seed.schema(), Default::default()).unwrap();
    let bound_items =
        OwnedItemLinePolicy::new(load("policies/items.json"), schema, Default::default()).unwrap();
    let mut expected_items = seed_items.input().clone();
    expected_items.definitions = schema.identity().clone();
    assert_eq!(bound_items.input(), &expected_items);
    assert!(
        OwnedItemLinePolicy::new(seed_items.input().clone(), schema, Default::default()).is_err()
    );

    let seed_source: ItemSourceLayoutPolicyInput = load("item-source-policy-seed.json");
    let seed_source =
        ItemSourceLayoutPolicy::new(seed_source, &seed_items, seed.schema(), Default::default())
            .unwrap();
    let bound_source = ItemSourceLayoutPolicy::new(
        load("policies/item-source.json"),
        &bound_items,
        schema,
        Default::default(),
    )
    .unwrap();
    let mut expected_source = seed_source.input().clone();
    expected_source.item_lines = *bound_items.identity();
    assert_eq!(bound_source.input(), &expected_source);
    assert!(
        ItemSourceLayoutPolicy::new(
            seed_source.input().clone(),
            &bound_items,
            schema,
            Default::default()
        )
        .is_err()
    );

    let mut expected_normalization: NormalizationPolicy = load("normalization-policy-seed.json");
    let bound_normalization: NormalizationPolicy = load("policies/normalization.json");
    let GemQualityPolicy::Attributes(quality) = &mut expected_normalization.gem_quality else {
        panic!("retained seed quality policy")
    };
    assert_eq!(&quality.definitions, seed.schema().identity());
    quality.definitions = schema.identity().clone();
    assert_eq!(bound_normalization, expected_normalization);

    let receipt: Receipt = load("policies/transition.json");
    assert_eq!(receipt.schema_version, 1);
    assert_eq!(receipt.scope, "explicit-checked-import-policy-successor");
    assert_eq!(&receipt.before_definitions, seed.schema().identity());
    assert_eq!(&receipt.after_definitions, schema.identity());
    assert_ne!(receipt.before_definitions, receipt.after_definitions);
    assert_eq!(&receipt.before_mapping, seed_mapping.identity());
    assert_eq!(&receipt.after_mapping, compiled.mapping().identity());
    assert_eq!(
        receipt.preserved_definition_count,
        seed.schema().input().definitions.len()
    );
    assert_eq!(
        receipt.preserved_slot_count,
        seed.schema().input().slots.len()
    );
    assert!(!receipt.source_execution);
    assert!(!receipt.numeric_coverage);
    let transition = compiled
        .artifacts()
        .find(|(name, _)| *name == "transition.json")
        .unwrap()
        .1;
    assert_eq!(
        receipt.compiled_transition_sha256,
        format!("{:x}", Sha256::digest(transition))
    );
    let names = [
        "policies/item-source.json",
        "policies/items.json",
        "policies/normalization.json",
        "policies/rewards.json",
    ];
    assert_eq!(receipt.artifacts.len(), names.len());
    for name in names {
        let rows = receipt
            .artifacts
            .iter()
            .filter(|row| row.path == name)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1, "exact receipt artifact {name}");
        let bytes = std::fs::read(dir().join(name)).unwrap();
        assert_eq!(rows[0].bytes, bytes.len());
        assert_eq!(rows[0].sha256, format!("{:x}", Sha256::digest(bytes)));
    }
}
#[test]
fn second_pass_allocates_zero_and_keeps_runtime_and_mapping_identities() {
    let result = first();
    let (_, _, catalog, source, policy) = inputs();
    let again = extend_owned_catalog_recipe(
        result.recipe().clone(),
        result.mapping().input().clone(),
        &catalog,
        &source,
        &policy,
        CatalogRecipeLimits::default(),
    )
    .unwrap();
    assert_eq!(again.transition().counts.allocated_gems, 0);
    assert_eq!(again.transition().counts.allocated_skills, 0);
    assert_eq!(again.transition().counts.reused_gems, 966);
    assert_eq!(again.transition().counts.reused_skills, 1436);
    assert_eq!(again.recipe(), result.recipe());
    assert_eq!(again.mapping().identity(), result.mapping().identity());
    assert_eq!(again.roles().input().roles, result.roles().input().roles);
    // Provenance is a new transition; unchanged runtime IDs are not receipt equality.
    assert_ne!(again.transition().input, result.transition().input);
}
#[test]
fn stale_prior_recipe_or_mapping_is_rejected_before_rebinding() {
    let (recipe, mapping, catalog, source, policy) = inputs();
    let mut bad = recipe.clone();
    bad.rules.definitions.content_sha256 = "ab".repeat(32);
    assert!(
        extend_owned_catalog_recipe(
            bad,
            mapping.clone(),
            &catalog,
            &source,
            &policy,
            CatalogRecipeLimits::default()
        )
        .is_err()
    );
    let mut bad_mapping = mapping;
    bad_mapping.definitions.content_sha256 = "ab".repeat(32);
    assert!(
        extend_owned_catalog_recipe(
            recipe,
            bad_mapping,
            &catalog,
            &source,
            &policy,
            CatalogRecipeLimits::default()
        )
        .is_err()
    );
}
#[test]
fn data_only_coefficients_preserve_ids_and_change_rule_identity() {
    let before = first();
    let (mut recipe, mapping, catalog, source, policy) = inputs();
    let cell = &mut recipe.rules.tables[0].rows[0];
    match cell {
        ParameterValue::Quantity(value) => {
            *value = poe_optimizer_core::owned_definitions::FiniteQuantity::new(
                value.value() + 0.125,
                value.unit().clone(),
            )
            .unwrap()
        }
        other => panic!("reviewed first table should contain quantities, got {other:?}"),
    }
    let changed = extend_owned_catalog_recipe(
        recipe,
        mapping,
        &catalog,
        &source,
        &policy,
        CatalogRecipeLimits::default(),
    )
    .unwrap();
    assert_eq!(changed.recipe().registry, before.recipe().registry);
    assert_eq!(changed.recipe().schema, before.recipe().schema);
    assert_eq!(changed.mapping().identity(), before.mapping().identity());
    assert_ne!(
        changed.assembled().rules().identity(),
        before.assembled().rules().identity()
    );
}
#[test]
fn aggregate_input_and_output_limits_cover_the_whole_bundle() {
    let result = first();
    let (recipe, mapping, catalog, source, policy) = inputs();
    let mut limits = CatalogRecipeLimits::default();
    limits.recipe.max_wire_bytes = 1024;
    assert!(
        extend_owned_catalog_recipe(
            recipe.clone(),
            mapping.clone(),
            &catalog,
            &source,
            &policy,
            limits
        )
        .is_err()
    );
    let mut limits = CatalogRecipeLimits::default();
    limits.recipe.max_output_bytes = result
        .assembled()
        .artifacts()
        .iter()
        .map(|a| a.bytes().len())
        .sum::<usize>()
        + 1;
    assert!(
        extend_owned_catalog_recipe(recipe, mapping, &catalog, &source, &policy, limits).is_err()
    );
}
