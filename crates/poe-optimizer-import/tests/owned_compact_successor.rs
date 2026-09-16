#[path = "support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_core::owned_content::{ContentDigestError, digest_owned};
use poe_optimizer_import::{owned_recipe::*, owned_successor::*};
use std::{collections::BTreeMap, path::PathBuf};
fn input() -> SuccessorBundleInput {
    fixture::input(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}
fn compact(input: SuccessorBundleInput) -> StagedSuccessorBundle {
    transition_owned_bundle_compact(input, Default::default()).unwrap()
}

#[test]
fn v1_preserves_supplied_order_and_original_digest_while_v2_reconstructs_canonically() {
    let mut supplied = input();
    supplied.successor.schema.definitions.reverse();
    supplied.successor.registry.entries.reverse();
    let expected = supplied.successor.clone();
    let original_digest =
        digest_owned("owned-successor-input-v1", &supplied, 64 * 1024 * 1024).unwrap();
    let legacy = transition_owned_bundle(supplied.clone(), Default::default()).unwrap();
    let current = compact(supplied);
    assert_eq!(legacy.transition().schema_version, 1);
    assert_eq!(legacy.transition().input, original_digest);
    assert_eq!(legacy.recipe(), &expected);
    assert_eq!(current.transition().schema_version, 2);
    assert_ne!(
        legacy.assembled().manifest().recipe,
        current.assembled().manifest().recipe
    );
    assert_eq!(legacy.transition().before, current.transition().before);
    assert_eq!(legacy.transition().after, current.transition().after);
    let files: BTreeMap<_, _> = current.artifacts().collect();
    assert!(!files.contains_key("recipe.json"));
    let reconstructed = OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: serde_json::from_slice(files["registry.json"]).unwrap(),
        schema: serde_json::from_slice(files["schema.json"]).unwrap(),
        rules: serde_json::from_slice(files["rules.json"]).unwrap(),
        routing: serde_json::from_slice(files["routing.json"]).unwrap(),
    };
    assert_eq!(current.recipe(), &reconstructed);
    let rebuilt = assemble_owned_recipe(reconstructed, Default::default()).unwrap();
    for artifact in rebuilt.artifacts() {
        assert_eq!(artifact.bytes(), files[artifact.name()]);
    }
    for artifact in legacy
        .assembled()
        .artifacts()
        .iter()
        .filter(|a| a.name() != "manifest.json")
    {
        assert_eq!(artifact.bytes(), files[artifact.name()]);
    }
    assert_eq!(current.items().input(), legacy.items().input());
    assert_eq!(current.item_source().input(), legacy.item_source().input());
    assert_eq!(current.query_sets(), legacy.query_sets());
}

#[test]
fn independently_bounded_input_streams_remove_only_the_duplicate_prior_recipe() {
    let supplied = input();
    let old_len = serde_json::to_vec(&supplied).unwrap().len();
    let prior_len = serde_json::to_vec(&supplied.prior).unwrap().len();
    let limits = SuccessorBundleLimits {
        max_input_bytes: old_len - prior_len / 2,
        ..Default::default()
    };
    assert!(matches!(
        transition_owned_bundle(supplied.clone(), limits),
        Err(SuccessorBundleError::Digest(
            ContentDigestError::TooLarge { .. }
        ))
    ));
    transition_owned_bundle_compact(supplied.clone(), limits).unwrap();
    let limits = SuccessorBundleLimits {
        max_input_bytes: prior_len + 128,
        ..Default::default()
    };
    // The prior fits; the independently bounded remaining stream does not.
    assert!(transition_owned_bundle_compact(supplied.clone(), limits).is_err());
    let limits = SuccessorBundleLimits {
        max_input_bytes: prior_len - 1,
        ..Default::default()
    };
    assert!(transition_owned_bundle_compact(supplied.clone(), limits).is_err());
    let limits = SuccessorBundleLimits {
        recipe: OwnedRecipeLimits {
            max_wire_bytes: prior_len - 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(transition_owned_bundle_compact(supplied, limits).is_err());
}

#[test]
fn compact_transition_commits_exact_prior_order_and_query_order() {
    let supplied = input();
    let first = compact(supplied.clone());
    let mut reordered = supplied.clone();
    reordered.prior.registry.entries.reverse();
    let second = compact(reordered);
    assert_ne!(first.transition().input, second.transition().input);
    assert_eq!(first.transition().before, second.transition().before);
    assert_eq!(first.transition().after, second.transition().after);
    let mut reordered = supplied;
    reordered.query_sets.reverse();
    let third = compact(reordered);
    assert_ne!(first.transition().input, third.transition().input);
    assert_ne!(first.query_sets(), third.query_sets());
}

#[test]
fn compact_tree_carry_is_a_fixed_point_with_exact_history_and_policy_content() {
    let old = transition_owned_bundle(input(), Default::default()).unwrap();
    let next = fixture::next(&old);
    let first = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        fixture::tree(&next),
        Default::default(),
    )
    .unwrap();
    let next = fixture::next(&first);
    let second = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(first.tree().unwrap().input().clone()),
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(second.recipe(), first.recipe());
    assert_eq!(second.transition().before, second.transition().after);
    assert_eq!(
        second.tree().unwrap().input(),
        first.tree().unwrap().input()
    );
    assert_eq!(second.recipe().registry, old.recipe().registry);
    assert_eq!(second.query_sets(), old.query_sets());
    assert_eq!(second.items().input(), old.items().input());
    assert_eq!(second.item_source().input(), old.item_source().input());
    assert_eq!(second.roles().input(), old.roles().input());
    assert_eq!(second.rewards().input(), old.rewards().input());
    assert_eq!(second.mapping().input(), old.mapping().input());
    let next = fixture::next(&second);
    let third = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(second.tree().unwrap().input().clone()),
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        second.artifacts().collect::<BTreeMap<_, _>>(),
        third.artifacts().collect::<BTreeMap<_, _>>()
    );
}

#[test]
fn compact_publication_still_enforces_output_history_and_binding_limits() {
    let supplied = input();
    let staged = compact(supplied.clone());
    let size = staged
        .artifacts()
        .map(|(_, bytes)| bytes.len())
        .sum::<usize>();
    assert!(
        transition_owned_bundle_compact(
            supplied.clone(),
            SuccessorBundleLimits {
                max_output_bytes: size - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut corrupt = supplied.clone();
    corrupt.successor.registry.entries.pop();
    assert!(transition_owned_bundle_compact(corrupt, Default::default()).is_err());
    let mut corrupt = supplied;
    corrupt.item_source.item_lines = digest_owned("wrong", &1, 100).unwrap();
    assert!(transition_owned_bundle_compact(corrupt, Default::default()).is_err());
}

#[test]
fn compact_publication_preserves_tombstones_and_rejects_resurrection() {
    use poe_optimizer_core::{
        owned_definitions::{BoundedInteger, OptionDefId, OwnedDefinitionKey},
        owned_schema::{SchemaDefinitionId, SchemaSubject},
    };
    use poe_optimizer_import::owned_mapping::{
        OwnedIdRegistry, OwnedMappingError, OwnedMappingErrorKind, RegistryState,
    };
    let old = compact(input());
    let mut supplied = fixture::next(&old);
    let mut registry =
        OwnedIdRegistry::new(supplied.successor.registry.clone(), Default::default()).unwrap();
    let retired: OptionDefId = registry.allocate_definition().unwrap();
    registry
        .retire(
            &SchemaSubject::Definition(retired.address()),
            OwnedDefinitionKey::new("test-retired").unwrap(),
        )
        .unwrap();
    supplied.successor.registry = registry.input().clone();
    let first = compact(supplied);
    let next = fixture::next(&first);
    let second = compact(next.clone());
    assert_eq!(second.recipe().registry, *registry.input());
    let mut corrupt = next;
    corrupt.successor.registry.entries.last_mut().unwrap().state = RegistryState::Active;
    corrupt.successor.registry.revision =
        BoundedInteger::new(corrupt.successor.registry.revision.get() - 1).unwrap();
    assert!(matches!(
        transition_owned_bundle_compact(corrupt, Default::default()),
        Err(SuccessorBundleError::Mapping(OwnedMappingError::Invalid {
            kind: OwnedMappingErrorKind::SuccessorConflict,
            ..
        }))
    ));
}
