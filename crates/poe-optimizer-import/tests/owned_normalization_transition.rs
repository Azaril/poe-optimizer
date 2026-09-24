//! Publication records the real prior normalization identity before replacement.
#[path = "support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::SourceComponent,
    owned_normalize::{GemInputPolicy, SkillScopePolicy},
    owned_successor::*,
};
use std::path::PathBuf;

fn prior() -> StagedSuccessorBundle {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baseline = transition_owned_bundle(fixture::input(&root), Default::default()).unwrap();
    let input = fixture::next(&baseline);
    transition_owned_catalog_with_tree_compact(
        input.clone(),
        fixture::append(&input),
        fixture::tree(&input),
        Default::default(),
    )
    .unwrap()
}

#[test]
fn explicit_policy_install_preserves_true_prior_and_rebinds_only_tree_policy_identity() {
    let prior = prior();
    let input = fixture::next(&prior);
    let mut replacement = input.normalization.clone();
    replacement.skill_scopes = Some(SkillScopePolicy {
        slot_attribute: "slot".into(),
        shared_slots: vec![SourceComponent::Missing],
    });
    replacement.gem_inputs = Some(GemInputPolicy {
        definitions: prior.assembled().schema().identity().clone(),
        gems: vec![],
    });
    let result = transition_owned_normalization_with_tree_compact(
        input,
        prior.tree().unwrap().input().clone(),
        replacement.clone(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(result.transition().before, prior.transition().after);
    assert_ne!(
        result.transition().before.normalization,
        result.transition().after.normalization
    );
    let mut restored = result.transition().after.clone();
    restored.normalization = prior.transition().after.normalization;
    assert_eq!(restored, prior.transition().after);
    assert_eq!(result.normalization(), &replacement);
    assert_eq!(result.recipe(), prior.recipe());
    assert_eq!(result.query_sets(), prior.query_sets());
    assert_eq!(result.items().input(), prior.items().input());
    assert_eq!(result.item_source().input(), prior.item_source().input());
    let mut restored_tree = result.tree().unwrap().input().clone();
    restored_tree.normalization = prior.tree().unwrap().input().normalization;
    assert_eq!(&restored_tree, prior.tree().unwrap().input());
    assert!(result.transition().schema_refinement.is_none());
    assert_eq!(result.transition().calculation, "not_run");
    assert_eq!(result.transition().whole_build_parity, "not_established");

    // Existing validated gem policy follows an explicit later schema successor.
    // An authored replacement with a stale binding is tested separately below.
    let mut next = fixture::next(&result);
    next.successor.schema.release = OwnedDefinitionKey::new("next-test-release").unwrap();
    let schema =
        OwnedDefinitionSchemaPackage::new(next.successor.schema.clone(), Default::default())
            .unwrap();
    next.successor.rules.definitions = schema.identity().clone();
    next.successor.routing.definitions = schema.identity().clone();
    let next = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(result.tree().unwrap().input().clone()),
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        next.normalization()
            .gem_inputs
            .as_ref()
            .unwrap()
            .definitions,
        *next.assembled().schema().identity()
    );
    assert_ne!(
        next.normalization()
            .gem_inputs
            .as_ref()
            .unwrap()
            .definitions,
        replacement.gem_inputs.unwrap().definitions
    );
}

#[test]
fn stale_authored_policy_prior_policy_tree_and_recipe_mutations_are_rejected() {
    let prior = prior();
    let input = fixture::next(&prior);
    let tree = prior.tree().unwrap().input().clone();
    let mut replacement = input.normalization.clone();
    let mut stale = prior.assembled().schema().identity().clone();
    stale.content_sha256 = "0".repeat(64);
    replacement.gem_inputs = Some(GemInputPolicy {
        definitions: stale,
        gems: vec![],
    });
    assert!(
        transition_owned_normalization_with_tree_compact(
            input.clone(),
            tree.clone(),
            replacement,
            Default::default()
        )
        .is_err()
    );
    let mut stale_prior = input.clone();
    stale_prior.normalization.version = OwnedDefinitionKey::new("different-prior-policy").unwrap();
    assert!(
        transition_owned_normalization_with_tree_compact(
            stale_prior,
            tree.clone(),
            input.normalization.clone(),
            Default::default()
        )
        .is_err()
    );
    let mut stale_tree = tree.clone();
    stale_tree.normalization = digest_owned("wrong-policy", &0, 100).unwrap();
    assert!(
        transition_owned_normalization_with_tree_compact(
            input.clone(),
            stale_tree,
            input.normalization.clone(),
            Default::default()
        )
        .is_err()
    );
    let mut changed = input.clone();
    changed.successor.rules.release = OwnedDefinitionKey::new("other-rules").unwrap();
    assert!(
        transition_owned_normalization_with_tree_compact(
            changed,
            tree.clone(),
            input.normalization.clone(),
            Default::default()
        )
        .is_err()
    );
    let mut malformed = input.normalization.clone();
    malformed.skill_scopes = Some(SkillScopePolicy {
        slot_attribute: String::new(),
        shared_slots: vec![],
    });
    assert!(
        transition_owned_normalization_with_tree_compact(
            input,
            tree,
            malformed,
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn replacement_content_is_committed_and_reapplication_is_an_explicit_noop() {
    let prior = prior();
    let input = fixture::next(&prior);
    let tree = prior.tree().unwrap().input().clone();
    let mut replacement = input.normalization.clone();
    replacement.version = OwnedDefinitionKey::new("replacement-a").unwrap();
    let first = transition_owned_normalization_with_tree_compact(
        input.clone(),
        tree.clone(),
        replacement.clone(),
        Default::default(),
    )
    .unwrap();
    replacement.version = OwnedDefinitionKey::new("replacement-b").unwrap();
    let second = transition_owned_normalization_with_tree_compact(
        input,
        tree,
        replacement,
        Default::default(),
    )
    .unwrap();
    assert_eq!(first.transition().before, second.transition().before);
    assert_ne!(first.transition().input, second.transition().input);
    assert_ne!(
        first.transition().after.normalization,
        second.transition().after.normalization
    );
    let replay = transition_owned_normalization_with_tree_compact(
        fixture::next(&first),
        first.tree().unwrap().input().clone(),
        first.normalization().clone(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(replay.transition().before, replay.transition().after);
    assert_eq!(replay.normalization(), first.normalization());
    assert_eq!(
        replay.tree().unwrap().input(),
        first.tree().unwrap().input()
    );
}
