//! Explicit offline corrections create a new data release, never a monotonic
//! successor. The prior release is checked before any dependency is rebound.
use crate::{
    owned_item_lines::OwnedItemLinePolicy,
    owned_item_source::ItemSourceLayoutPolicy,
    owned_mapping::OwnedMappingIndex,
    owned_normalize::{
        ConfigurationRewardInventoryPolicy, EquipmentMembershipPolicy, GemInventoryPolicy,
        GemQualityPolicy, ItemParameterInputsPolicy, PassiveSocketMembershipPolicy,
        SupportOriginOrderPolicy, equipment_membership_identity,
        gem_inventory_scalar_inputs_identity, rebind_payload_inventory_roles,
        rebind_support_inventory_roles, validate_configuration_reward_inventory,
    },
    owned_recipe::{OwnedRecipeError, assemble_owned_recipe},
    owned_release::{
        OwnedReleaseError, OwnedReleaseInput, OwnedReleaseLimits, OwnedReleaseProvenance,
        StagedOwnedRelease, assemble_owned_release, preflight,
    },
    owned_reward_policy::OwnedRewardPolicy,
    owned_skill_catalog::OwnedSkillRoleIndex,
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Full replacement descriptors for existing addresses, bound to the complete
/// prior authoring input. No identities are allocated, retired or reused.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseRevisionInput {
    pub schema_version: u32,
    pub before: OwnedContentDigest,
    pub release: OwnedDefinitionKey,
    pub reason: OwnedDefinitionKey,
    pub definitions: Vec<DefinitionDescriptor>,
    pub slots: Vec<SlotDescriptor>,
}

/// Correct explicitly named schemas in a new immutable release. All other
/// semantic content is preserved; dependent identities are rebuilt in order.
/// This function makes no assertion that builds authored for the old release
/// remain admissible, or that the supplied correction has numerical parity.
pub fn compile_owned_release_revision(
    prior: &StagedOwnedRelease,
    revision: OwnedReleaseRevisionInput,
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease, OwnedReleaseError> {
    limits.validate()?;
    if prior.evaluation().is_some() {
        return Err(OwnedReleaseError::Invalid(
            "schema revision needs an explicit evaluation-artifact migration",
        ));
    }
    let count = revision
        .definitions
        .len()
        .checked_add(revision.slots.len())
        .ok_or(OwnedReleaseError::Invalid(
            "release revision entry overflow",
        ))?;
    if count == 0 || count > limits.max_validation_entries {
        return Err(OwnedReleaseError::Invalid("release revision entry limit"));
    }
    let authoring = digest_owned(
        "owned-release-schema-revision-v1",
        &revision,
        limits.max_artifact_bytes,
    )?;
    if revision.schema_version != 1
        || revision.before != prior.receipt().input
        || revision.release == prior.assembled().schema().input().release
    {
        return Err(OwnedReleaseError::Invalid(
            "release revision version or endpoint",
        ));
    }
    if revision
        .definitions
        .windows(2)
        .any(|rows| rows[0].address() >= rows[1].address())
        || revision
            .slots
            .windows(2)
            .any(|rows| rows[0].address() >= rows[1].address())
    {
        return Err(OwnedReleaseError::Invalid(
            "release revision needs canonical unique targets",
        ));
    }

    // The prior constructor already bounded this state. Recheck its complete
    // commitment under the caller's potentially tighter budget before cloning.
    digest_owned(
        "owned-release-revision-prior-budget-v1",
        prior.input(),
        limits.max_input_bytes,
    )?;
    if prior.input().provenance.len() >= limits.max_provenance_entries {
        return Err(OwnedReleaseError::Limit("provenance entries"));
    }
    // The revision adds one authoring row; charge it before copying prior data.
    let mut prior_limits = limits;
    prior_limits.max_validation_entries = limits
        .max_validation_entries
        .checked_sub(1)
        .ok_or(OwnedReleaseError::Limit("validation entries"))?;
    preflight(prior.input(), prior_limits)?;
    let mut input = prior.input().clone();
    let definitions: BTreeMap<_, _> = input
        .recipe
        .schema
        .definitions
        .iter()
        .enumerate()
        .map(|(index, row)| (row.address(), index))
        .collect();
    let slots: BTreeMap<_, _> = input
        .recipe
        .schema
        .slots
        .iter()
        .enumerate()
        .map(|(index, row)| (row.address(), index))
        .collect();
    for row in revision.definitions {
        let position = definitions
            .get(&row.address())
            .ok_or(OwnedReleaseError::Invalid(
                "release revision cannot allocate definitions",
            ))?;
        if input.recipe.schema.definitions[*position] == row {
            return Err(OwnedReleaseError::Invalid(
                "release revision contains an unchanged definition",
            ));
        }
        input.recipe.schema.definitions[*position] = row;
    }
    for row in revision.slots {
        let position = slots.get(&row.address()).ok_or(OwnedReleaseError::Invalid(
            "release revision cannot allocate slots",
        ))?;
        if input.recipe.schema.slots[*position] == row {
            return Err(OwnedReleaseError::Invalid(
                "release revision contains an unchanged slot",
            ));
        }
        input.recipe.schema.slots[*position] = row;
    }
    input.recipe.schema.release = revision.release;
    rebind_release_dependencies(&mut input, limits)?;
    input.provenance.push(OwnedReleaseProvenance {
        kind: revision.reason,
        prior_input: prior.receipt().input,
        authoring_input: authoring,
    });
    assemble_owned_release(input, limits)
}

/// Rebuild dependent commitments only for a compiler that has already checked
/// its prior endpoint and bounded the complete authored migration.
pub(crate) fn rebind_release_dependencies(
    input: &mut OwnedReleaseInput,
    limits: OwnedReleaseLimits,
) -> Result<(), OwnedReleaseError> {
    if input.evaluation.is_some() {
        return Err(OwnedReleaseError::Invalid(
            "dependency rebinding needs an explicit evaluation-artifact migration",
        ));
    }
    let schema =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), limits.recipe.schema)
            .map_err(OwnedRecipeError::from)?;
    input.recipe.rules.definitions = schema.identity().clone();
    input.recipe.routing.definitions = schema.identity().clone();
    let runtime = assemble_owned_recipe(input.recipe.clone(), limits.recipe)?;

    input.mapping.definitions = runtime.schema().identity().clone();
    input.mapping.registry = runtime.registry().identity()?;
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        runtime.registry(),
        runtime.schema(),
        limits.catalog.mapping,
    )?;
    input.roles.mapping = *mapping.identity();
    input.roles.definitions = runtime.schema().identity().clone();
    if let GemQualityPolicy::Attributes(quality) = &mut input.normalization.gem_quality {
        quality.definitions = runtime.schema().identity().clone();
    }
    if let Some(gems) = &mut input.normalization.gem_inputs {
        gems.definitions = runtime.schema().identity().clone();
    }
    if input.normalization.gem_inventory.is_some()
        || input.normalization.payload_inventory.is_some()
        || matches!(
            input.normalization.support_origin_order,
            Some(
                SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 { .. }
                    | SupportOriginOrderPolicy::SavedManualGroupOrderWithNonphysicalSkillInventoryV3 { .. }
            )
        )
    {
        let roles = OwnedSkillRoleIndex::new(
            input.roles.clone(),
            &mapping,
            runtime.schema(),
            limits.catalog,
        )?;
        rebind_support_inventory_roles(&mut input.normalization, &roles);
        rebind_payload_inventory_roles(&mut input.normalization, &roles);
        let scalar_binding =
            gem_inventory_scalar_inputs_identity(&input.normalization, limits.normalization)?;
        if let Some(GemInventoryPolicy::PobFreshSingleSupportV1 {
            definitions,
            roles: role_binding,
            scalar_inputs,
            ..
        }) = &mut input.normalization.gem_inventory
        {
            *definitions = runtime.schema().identity().clone();
            *role_binding = *roles.identity();
            *scalar_inputs = scalar_binding;
        }
    }
    if let Some(policy) = &mut input.normalization.equipment_membership {
        *policy.definitions_mut() = runtime.schema().identity().clone();
    }
    input.rewards.mapping = *mapping.identity();
    input.rewards.definitions = runtime.schema().identity().clone();
    if input.normalization.configuration_reward_inventory.is_some() {
        let rewards = OwnedRewardPolicy::new(
            input.rewards.clone(),
            &mapping,
            runtime.schema(),
            limits.rewards,
        )?;
        let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
            reward_policy,
            ..
        }) = &mut input.normalization.configuration_reward_inventory
        else {
            unreachable!("checked optional reward inventory");
        };
        // This private helper is called only with an inherited, checked release.
        // Neither schema corrections nor contract migrations replace its reward
        // recipes or source census. Rebind only the resulting dependency digest.
        *reward_policy = *rewards.identity();
        validate_configuration_reward_inventory(
            &input.normalization,
            &mapping,
            &rewards,
            limits.normalization,
        )?;
    }
    input.items.definitions = runtime.schema().identity().clone();
    let items = OwnedItemLinePolicy::new(input.items.clone(), runtime.schema(), limits.items)?;
    input.item_source.item_lines = *items.identity();
    let scoped_source = if input.normalization.item_modifier_membership.is_some()
        || input.normalization.item_parameter_inputs.is_some()
        || matches!(
            input.normalization.equipment_membership,
            Some(
                EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 { .. }
                    | EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 { .. }
            )
        )
        || matches!(
            input.normalization.passive_socket_membership,
            Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 { .. })
        ) {
        Some(ItemSourceLayoutPolicy::new(
            input.item_source.clone(),
            &items,
            runtime.schema(),
            limits.item_source,
        )?)
    } else {
        None
    };
    if let Some(policy) = &mut input.normalization.equipment_membership
        && let Some((item_lines, item_source)) = policy.imported_bindings_mut()
    {
        *item_lines = *items.identity();
        *item_source = *scoped_source
            .as_ref()
            .expect("checked imported item source")
            .identity();
    }
    if let Some(policy) = &mut input.normalization.item_modifier_membership {
        let source = scoped_source.as_ref().expect("checked item source");
        // This compiler starts from a checked complete prior release. Only its
        // explicit schema correction changed these dependent commitments; full
        // successor construction validates the policy again before publication.
        let (definitions, item_lines, item_source) = policy.bindings_mut();
        *definitions = runtime.schema().identity().clone();
        *item_lines = *items.identity();
        *item_source = *source.identity();
    }
    if let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
        definitions,
        mapping: mapping_binding,
        item_lines,
        item_source,
        equipment,
        ..
    }) = &mut input.normalization.passive_socket_membership
    {
        *definitions = runtime.schema().identity().clone();
        *mapping_binding = *mapping.identity();
        *item_lines = *items.identity();
        *item_source = *scoped_source
            .as_ref()
            .expect("checked passive item source")
            .identity();
        *equipment = equipment_membership_identity(
            input
                .normalization
                .equipment_membership
                .as_ref()
                .expect("checked passive equipment inventory"),
            limits.normalization,
        )?;
    }
    if let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        definitions,
        item_lines,
        item_source,
        ..
    }) = &mut input.normalization.item_parameter_inputs
    {
        let source = scoped_source.as_ref().expect("checked item source");
        *definitions = runtime.schema().identity().clone();
        *item_lines = *items.identity();
        *item_source = *source.identity();
    }
    input.tree = input
        .tree
        .take()
        .map(|tree| {
            OwnedTreeNormalizationPolicy::bind_new(
                tree.content,
                runtime.registry(),
                runtime.schema(),
                &mapping,
                &input.normalization,
                limits.tree,
            )
            .map(|tree| tree.input().clone())
        })
        .transpose()?;
    Ok(())
}
