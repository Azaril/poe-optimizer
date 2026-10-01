//! Input completeness authority stays bound across offline publication paths.
#[path = "support/owned_compact_fixture.rs"]
mod fixture;

use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::OwnedDefinitionKey, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_normalize::{EquipmentMembershipPolicy, ItemModifierMembershipPolicy},
    owned_release::*,
    owned_release_revision::*,
    owned_successor::*,
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use std::{path::PathBuf, sync::OnceLock};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn prior() -> StagedOwnedRelease {
    static INPUT: OnceLock<OwnedReleaseInput> = OnceLock::new();
    let input = INPUT.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let first =
            transition_owned_bundle_compact(fixture::input(&root), Default::default()).unwrap();
        let next = fixture::next(&first);
        let bundle = transition_owned_catalog_with_tree_compact(
            next.clone(),
            fixture::append(&next),
            fixture::tree(&next),
            Default::default(),
        )
        .unwrap();
        let mut normalization = bundle.normalization().clone();
        normalization.equipment_membership =
            Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
                definitions: bundle.assembled().schema().identity().clone(),
                templates: vec![],
                source_base_names: vec![],
                loader_jewel_fallback_titles: vec![],
            });
        // An empty domain grants no item authority, but its package commitments
        // must be checked just as strictly as a nonempty author's policy.
        normalization.item_modifier_membership =
            Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
                definitions: bundle.assembled().schema().identity().clone(),
                item_lines: *bundle.items().identity(),
                item_source: *bundle.item_source().identity(),
                templates: vec![],
                modifier_rules: vec![],
            });
        let tree = OwnedTreeNormalizationPolicy::bind_new(
            bundle.tree().unwrap().input().content.clone(),
            bundle.assembled().registry(),
            bundle.assembled().schema(),
            bundle.mapping(),
            &normalization,
            Default::default(),
        )
        .unwrap();
        OwnedReleaseInput {
            schema_version: OWNED_RELEASE_VERSION,
            recipe: bundle.recipe().clone(),
            mapping: bundle.mapping().input().clone(),
            roles: bundle.roles().input().clone(),
            normalization,
            rewards: bundle.rewards().input().clone(),
            items: bundle.items().input().clone(),
            item_source: bundle.item_source().input().clone(),
            tree: Some(tree.input().clone()),
            evaluation: None,
            query_sets: bundle.query_sets().to_vec(),
            provenance: vec![],
        }
    });
    assemble_owned_release(input.clone(), Default::default()).unwrap()
}

fn successor(prior: &StagedOwnedRelease) -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: prior.input().recipe.clone(),
        successor: prior.input().recipe.clone(),
        mapping: prior.input().mapping.clone(),
        roles: prior.input().roles.clone(),
        normalization: prior.input().normalization.clone(),
        rewards: prior.input().rewards.clone(),
        items: prior.input().items.clone(),
        item_source: prior.input().item_source.clone(),
        query_sets: prior.input().query_sets.clone(),
    }
}

#[test]
fn standalone_release_rejects_each_stale_completeness_commitment() {
    let prior = prior();
    for field in ["definitions", "item_lines", "item_source"] {
        let mut input = prior.input().clone();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions,
            item_lines,
            item_source,
            ..
        }) = &mut input.normalization.item_modifier_membership
        else {
            unreachable!()
        };
        let wrong = digest_owned("unrelated-membership-commitment", &17, 100).unwrap();
        match field {
            "definitions" => definitions.content_sha256 = "0".repeat(64),
            "item_lines" => *item_lines = wrong,
            "item_source" => *item_source = wrong,
            _ => unreachable!(),
        }
        assert!(
            matches!(
                assemble_owned_release(input, Default::default()),
                Err(OwnedReleaseError::Normalization(
                    poe_optimizer_import::owned_normalize::NormalizationError::Binding
                ))
            ),
            "{field} binding must fail before tree construction/publication"
        );
    }
}

#[test]
fn successor_rebinds_only_checked_unchanged_prior_item_policies() {
    let prior = prior();
    let before = serde_json::to_vec(prior.input()).unwrap();
    let mut next = successor(&prior);
    next.successor.schema.release = key("membership-schema-successor");
    let schema =
        OwnedDefinitionSchemaPackage::new(next.successor.schema.clone(), Default::default())
            .unwrap();
    next.successor.rules.definitions = schema.identity().clone();
    next.successor.routing.definitions = schema.identity().clone();
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(prior.tree().unwrap().input().clone()),
    };
    let after = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        tree.clone(),
        Default::default(),
    )
    .unwrap();
    let mut expected = prior
        .normalization()
        .item_modifier_membership
        .clone()
        .unwrap();
    let ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        item_lines,
        item_source,
        ..
    } = &mut expected;
    *definitions = after.assembled().schema().identity().clone();
    *item_lines = *after.items().identity();
    *item_source = *after.item_source().identity();
    assert_eq!(
        after.normalization().item_modifier_membership,
        Some(expected)
    );
    assert_ne!(after.items().identity(), prior.items().identity());
    assert_ne!(
        after.item_source().identity(),
        prior.item_source().identity()
    );
    assert_eq!(after.query_sets(), prior.query_sets());
    assert_eq!(serde_json::to_vec(prior.input()).unwrap(), before);

    // A stale prior is rejected before the rebind could hide it.
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 { item_source, .. }) =
        &mut next.normalization.item_modifier_membership
    else {
        unreachable!()
    };
    *item_source = digest_owned("wrong-prior-source", &1, 100).unwrap();
    assert!(
        transition_owned_catalog_with_tree_compact(
            next.clone(),
            fixture::append(&next),
            tree,
            Default::default(),
        )
        .is_err()
    );

    // Supplied source policies carry their own commitments; they are not a
    // request to bless new source semantics using the prior proof.
    let mut supplied = successor(&prior);
    supplied.item_source.version = key("changed-source-policy");
    assert!(transition_owned_bundle_compact(supplied, Default::default()).is_err());

    let mut replacement = prior.normalization().clone();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 { item_lines, .. }) =
        &mut replacement.item_modifier_membership
    else {
        unreachable!()
    };
    *item_lines = digest_owned("wrong-replacement-lines", &1, 100).unwrap();
    assert!(
        transition_owned_normalization_with_tree_compact(
            successor(&prior),
            prior.tree().unwrap().input().clone(),
            replacement,
            Default::default(),
        )
        .is_err()
    );
}

#[test]
fn explicit_schema_revision_preserves_proof_domain_and_rebinds_its_dependencies() {
    let prior = prior();
    let mut definition = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|row| {
            matches!(row, DefinitionDescriptor::Gem(gem)
                if matches!(&gem.schema, SchemaState::Known(schema)
                    if schema.declarations.parameters.is_complete()))
        })
        .unwrap()
        .clone();
    let subject = SchemaSubject::Definition(definition.address());
    let DefinitionDescriptor::Gem(gem) = &mut definition else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut gem.schema else {
        unreachable!()
    };
    schema.declarations.parameters.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::InputSchema,
            code: key("input-review-incomplete"),
        }],
    };
    let revised = compile_owned_release_revision(
        &prior,
        OwnedReleaseRevisionInput {
            schema_version: 1,
            before: prior.receipt().input,
            release: key("membership-revised-release"),
            reason: key("correct-input-closure"),
            definitions: vec![definition],
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    let mut expected = prior
        .normalization()
        .item_modifier_membership
        .clone()
        .unwrap();
    let ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        item_lines,
        item_source,
        ..
    } = &mut expected;
    *definitions = revised.receipt().definitions.clone();
    *item_lines = *revised.items().identity();
    *item_source = *revised.item_source().identity();
    assert_eq!(
        revised.normalization().item_modifier_membership,
        Some(expected)
    );
    assert_ne!(revised.receipt().definitions, prior.receipt().definitions);
    assert_eq!(revised.query_sets(), prior.query_sets());
}
