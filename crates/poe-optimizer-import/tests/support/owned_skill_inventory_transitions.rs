//! Checked transitions use the same real Direct root/recipes as normalization.
//! Empty rule and route inventories here do not assert executable build coverage.
use super::{Fixture, add_generated, configure};
use crate::{actions::key, inventory::items};
use poe_optimizer_core::{
    owned_content::OwnedContentDigest,
    owned_definitions::FiniteQuantity,
    owned_routing::{ActionRoutingInput, OWNED_ACTION_ROUTING_VERSION},
    owned_rules::{OWNED_RULE_OPERATIONS_V17, OWNED_RULE_PACKAGE_VERSION, RulePackageInput},
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_normalize::*, owned_recipe::OwnedRecipeInput, owned_release::*,
    owned_release_migration::*, owned_release_revision::*,
    owned_source_actions::SourceActionCorrespondenceInput, owned_successor::*,
    owned_tree_policy::*,
};

fn prior(with_direct: bool) -> StagedOwnedRelease {
    let mut f = Fixture::new();
    configure(&mut f);
    add_generated(&mut f);
    if !with_direct {
        f.base.policy.direct_skill_inputs = None;
        let SkillInventoryPolicy::PobFreshAuthoredRootsV1 { direct_inputs, .. } =
            f.base.policy.skill_inventory.as_mut().unwrap();
        *direct_inputs = None;
    } else {
        f.base.policy.direct_support_targets =
            Some(DirectSupportTargetPolicy::PobManualSingleDirectRootV1 {
                direct_inputs: direct_skill_inputs_identity(
                    f.base.policy.direct_skill_inputs.as_ref().unwrap(),
                    Default::default(),
                )
                .unwrap(),
            });
    }
    let base = &f.base.base;
    let schema = &base.schema;
    let empty_items = items::empty_items(schema);
    let item_source = items::empty_source_for_items(schema, &empty_items);
    let tree = OwnedTreeNormalizationPolicy::bind_new(
        TreeNormalizationContent {
            version: key("finite-empty-tree"),
            source: base.mapping.input().source.clone(),
            catalog: base.roles.input().compilation.catalog_digest,
            policy: *base.mapping.source_identity(),
            tree_version: "fixture-v1".into(),
            classes: vec![],
            ascendancies: vec![],
            tokens: vec![],
            attributes: vec![],
            syntax: TreeNormalizationSyntax {
                tree_version_attribute: "treeVersion".into(),
                class_attribute: "classId".into(),
                ascendancy_attribute: "ascendClassId".into(),
                class_consistency_attribute: None,
                ascendancy_consistency_attribute: None,
                overrides_element: "Overrides".into(),
                attribute_override_element: "AttributeOverride".into(),
                weapon_overlays: vec![],
                ignored_spec_children: vec![],
            },
            access: None,
        },
        &base.registry,
        schema,
        &base.mapping,
        &f.base.policy,
        Default::default(),
    )
    .unwrap();
    assemble_owned_release(
        OwnedReleaseInput {
            schema_version: 1,
            recipe: OwnedRecipeInput {
                schema_version: 1,
                registry: base.registry.input().clone(),
                schema: schema.input().clone(),
                rules: RulePackageInput {
                    existing_actor_rules: None,
                    ordered_contributions: None,
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: schema.namespace().clone(),
                    release: key("finite-import-only-rules"),
                    semantics_version: key("finite-import-only-rules-v1"),
                    operations_version: key(OWNED_RULE_OPERATIONS_V17),
                    definitions: schema.identity().clone(),
                    tables: vec![],
                    owners: vec![],
                    receivers: DeclaredSet::complete(vec![]),
                    effect_applications: Some(DeclaredSet::complete(vec![])),
                },
                routing: ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: schema.namespace().clone(),
                    release: key("finite-import-only-routes"),
                    definitions: schema.identity().clone(),
                    outputs: vec![],
                },
            },
            mapping: base.mapping.input().clone(),
            roles: base.roles.input().clone(),
            normalization: f.base.policy.clone(),
            rewards: f.base.rewards.input().clone(),
            items: empty_items.input().clone(),
            item_source: item_source.input().clone(),
            tree: Some(tree.input().clone()),
            evaluation: None,
            query_sets: vec![],
            provenance: vec![],
        },
        Default::default(),
    )
    .unwrap()
}

fn commitment(policy: &NormalizationPolicy) -> Option<OwnedContentDigest> {
    let SkillInventoryPolicy::PobFreshAuthoredRootsV1 { direct_inputs, .. } =
        policy.skill_inventory.as_ref().unwrap();
    *direct_inputs
}

/// Assert the complete policy after changing only independently known bindings.
/// This includes nested Direct disposition correspondence and every source row.
fn assert_rebound(
    prior: &StagedOwnedRelease,
    actual: &NormalizationPolicy,
    roles: &poe_optimizer_import::owned_skill_catalog::OwnedSkillRoleIndex,
) {
    assert_ne!(roles.identity(), prior.roles().identity());
    let mut expected = prior.input().normalization.clone();
    let GemQualityPolicy::Attributes(quality) = &mut expected.gem_quality else {
        unreachable!()
    };
    quality.definitions = roles.input().definitions.clone();
    if let Some(DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions,
        roles: binding,
        dispositions,
        ..
    }) = &mut expected.direct_skill_inputs
    {
        *definitions = roles.input().definitions.clone();
        *binding = *roles.identity();
        for disposition in dispositions {
            let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
                definitions,
                roles: binding,
                ..
            } = &mut disposition.reference_action
            else {
                unreachable!()
            };
            *definitions = roles.input().definitions.clone();
            *binding = *roles.identity();
        }
    }
    let digest = expected
        .direct_skill_inputs
        .as_ref()
        .map(|policy| direct_skill_inputs_identity(policy, Default::default()).unwrap());
    let SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
        roles: binding,
        direct_inputs,
        ..
    } = expected.skill_inventory.as_mut().unwrap();
    *binding = *roles.identity();
    *direct_inputs = digest;
    if let Some(DirectSupportTargetPolicy::PobManualSingleDirectRootV1 { direct_inputs }) =
        &mut expected.direct_support_targets
    {
        *direct_inputs = digest.expect("Direct target policy requires source policy");
    }
    assert_eq!(actual, &expected);
    assert_eq!(commitment(actual), digest);
    if digest.is_some() {
        assert_ne!(digest, commitment(&prior.input().normalization));
    }
}

fn correction(prior: &StagedOwnedRelease) -> OwnedReleaseRevisionInput {
    let mut parameter = prior
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| {
            matches!(row, SlotDescriptor::Parameter(entry)
            if matches!(&entry.schema, SchemaState::Known(schema)
                if schema.skill_input.is_some()))
        })
        .unwrap()
        .clone();
    let SlotDescriptor::Parameter(entry) = &mut parameter else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let ValueSchema::Quantity(range) = &mut schema.value else {
        unreachable!()
    };
    range.minimum = FiniteQuantity::new(-999.0, range.minimum.unit().clone()).unwrap();
    OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: key("reviewed-skill-input-bounds"),
        reason: key("reviewed-finite-raw-domain"),
        definitions: vec![],
        slots: vec![parameter],
    }
}

fn successor(prior: &StagedOwnedRelease, changed: bool) -> SuccessorBundleInput {
    let mut recipe = prior.input().recipe.clone();
    if changed {
        recipe.schema.release = key("authored-root-successor");
    }
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), Default::default()).unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
    let empty_items = items::empty_items(&schema);
    let item_source = items::empty_source_for_items(&schema, &empty_items);
    SuccessorBundleInput {
        schema_version: 1,
        prior: prior.input().recipe.clone(),
        successor: recipe,
        mapping: prior.input().mapping.clone(),
        roles: prior.input().roles.clone(),
        normalization: prior.input().normalization.clone(),
        rewards: prior.input().rewards.clone(),
        query_sets: prior.input().query_sets.clone(),
        items: empty_items.input().clone(),
        item_source: item_source.input().clone(),
    }
}

#[test]
fn schema_revision_refreshes_exact_roles_and_rebound_direct_policy_commitment() {
    // The no-Direct case separately exercises skill_inventory as the only
    // role-dependent optional policy; Direct cannot mask a missing condition.
    for with_direct in [false, true] {
        let prior = prior(with_direct);
        let before = serde_json::to_vec(prior.input()).unwrap();
        let next =
            compile_owned_release_revision(&prior, correction(&prior), Default::default()).unwrap();
        assert_rebound(&prior, &next.input().normalization, next.roles());
        assert_eq!(prior.input().recipe.registry, next.input().recipe.registry);
        assert_eq!(before, serde_json::to_vec(prior.input()).unwrap());
    }
}

#[test]
fn inherited_successor_refreshes_exact_roles_and_direct_policy_commitment() {
    let prior = prior(true);
    let next = transition_owned_bundle(successor(&prior, true), Default::default()).unwrap();
    assert_rebound(&prior, next.normalization(), next.roles());
    assert_eq!(prior.input().recipe.registry, next.recipe().registry);
}

#[test]
fn checked_contract_migration_rebinds_actual_direct_recipes_before_inventory_digest() {
    let prior = prior(true);
    let revision = correction(&prior);
    let migration = OwnedReleaseMigrationInput {
        schema_version: 3,
        before: prior.receipt().input,
        release: key("authored-root-contract-migration"),
        reason: key("reviewed-finite-raw-domain"),
        contract: OwnedReleaseContractMigration {
            schema_version: 5,
            schema_semantics_version: prior.input().recipe.schema.semantics_version.clone(),
            operations_version: key(OWNED_RULE_OPERATIONS_V17),
            rule_semantics_version: prior.input().recipe.rules.semantics_version.clone(),
        },
        schema: revision
            .slots
            .into_iter()
            .map(poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Slot)
            .collect(),
        tables: vec![],
        owners: vec![],
        receivers: vec![],
        query_targets: vec![],
        evaluation: None,
    };
    let next = compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
    assert_rebound(&prior, &next.input().normalization, next.roles());
    assert_eq!(prior.input().recipe.registry, next.input().recipe.registry);
}

#[test]
fn explicit_replacement_rejects_stale_roles_and_stale_direct_digest_without_repair() {
    let prior = prior(true);
    let next =
        compile_owned_release_revision(&prior, correction(&prior), Default::default()).unwrap();
    let old = prior
        .input()
        .normalization
        .skill_inventory
        .as_ref()
        .unwrap();
    let SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
        roles: old_roles,
        direct_inputs: old_direct,
        ..
    } = old;
    for stale in [None, Some(false), Some(true)] {
        let mut replacement = next.input().normalization.clone();
        if let Some(stale_digest) = stale {
            let SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
                roles,
                direct_inputs,
                ..
            } = replacement.skill_inventory.as_mut().unwrap();
            if stale_digest {
                *direct_inputs = *old_direct;
            } else {
                *roles = *old_roles;
            }
        }
        let result = transition_owned_normalization_with_tree_compact(
            successor(&next, false),
            next.input().tree.clone().unwrap(),
            replacement,
            Default::default(),
        );
        if stale.is_some() {
            assert!(matches!(
                result,
                Err(SuccessorBundleError::Normalization(
                    NormalizationError::Binding
                ))
            ));
        } else {
            assert_eq!(result.unwrap().normalization(), &next.input().normalization);
        }
    }
}

#[test]
fn direct_target_successors_reject_stale_commitments_before_rebinding() {
    let prior = prior(true);
    let next =
        compile_owned_release_revision(&prior, correction(&prior), Default::default()).unwrap();
    let mut replacement = next.input().normalization.clone();
    replacement.direct_support_targets = prior.input().normalization.direct_support_targets.clone();
    assert!(matches!(
        transition_owned_normalization_with_tree_compact(
            successor(&next, false),
            next.input().tree.clone().unwrap(),
            replacement,
            Default::default(),
        ),
        Err(SuccessorBundleError::Normalization(
            NormalizationError::Binding
        ))
    ));
    let mut inherited = successor(&prior, true);
    let DirectSupportTargetPolicy::PobManualSingleDirectRootV1 { direct_inputs } = inherited
        .normalization
        .direct_support_targets
        .as_mut()
        .unwrap();
    *direct_inputs = "0".repeat(64).parse().unwrap();
    assert!(matches!(
        transition_owned_bundle(inherited, Default::default()),
        Err(SuccessorBundleError::Normalization(
            NormalizationError::Binding
        ))
    ));
}
