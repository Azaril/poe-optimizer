//! Actual persisted catalog/recipe integration; no source checkout or VM.
use poe_optimizer_core::owned_build::ParameterValue;
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use poe_optimizer_import::{
    owned_catalog_recipe::*, owned_mapping::*, owned_recipe::*, owned_skill_catalog::*,
};
use serde::de::DeserializeOwned;
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
