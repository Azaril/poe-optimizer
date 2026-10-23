//! Import dependencies checked against one exact registry and definition space.
//! No rule compilation, evaluation authority or dependency rewriting occurs here.
use super::*;
use crate::owned_mapping::OwnedIdRegistrySnapshot;
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;

/// Own the exact policy only after every constituent-dependent check succeeds.
/// Construction stays private to this module's validation pipeline; retaining
/// the policy and its digest grants no rule or complete-build coverage.
pub(super) struct CheckedNormalizationPolicy {
    input: NormalizationPolicy,
    identity: OwnedContentDigest,
}

impl CheckedNormalizationPolicy {
    pub(super) fn into_authored_content(self) -> crate::owned_normalize::NormalizationContent {
        crate::owned_normalize::NormalizationContent::from_checked(self.input)
    }

    pub(super) fn input(&self) -> &NormalizationPolicy {
        &self.input
    }

    pub(super) fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
}

pub(super) struct CheckedImportConstituents {
    pub(super) mapping: OwnedMappingIndex,
    pub(super) roles: OwnedSkillRoleIndex,
    pub(super) normalization: CheckedNormalizationPolicy,
    pub(super) rewards: OwnedRewardPolicy,
    pub(super) items: OwnedItemLinePolicy,
    pub(super) item_source: ItemSourceLayoutPolicy,
    pub(super) tree: Option<OwnedTreeNormalizationPolicy>,
}

impl CheckedImportConstituents {
    /// Callers bound the complete release before this constructor clones any
    /// constituent. Each existing loader retains its independent limits.
    pub(super) fn check(
        input: &ReleaseView<'_>,
        registry: &OwnedIdRegistrySnapshot,
        schema: &OwnedDefinitionSchemaPackage,
        limits: OwnedReleaseLimits,
    ) -> Result<Self> {
        let mapping = OwnedMappingIndex::new(
            input.mapping.clone(),
            registry,
            schema,
            limits.catalog.mapping,
        )?;
        let roles =
            OwnedSkillRoleIndex::new(input.roles.clone(), &mapping, schema, limits.catalog)?;
        validate_normalization_inputs(
            input.normalization,
            &mapping,
            schema,
            &[],
            limits.normalization,
        )?;
        validate_role_bound_normalization(
            input.normalization,
            schema,
            &roles,
            &mapping,
            limits.normalization,
        )?;
        let normalization = digest_owned(
            "owned-normalization-policy-v3",
            input.normalization,
            limits.normalization.max_policy_bytes,
        )?;
        let rewards =
            OwnedRewardPolicy::new(input.rewards.clone(), &mapping, schema, limits.rewards)?;
        validate_configuration_reward_inventory(
            input.normalization,
            &mapping,
            &rewards,
            limits.normalization,
        )?;
        let items = OwnedItemLinePolicy::new(input.items.clone(), schema, limits.items)?;
        let item_source = ItemSourceLayoutPolicy::new(
            input.item_source.clone(),
            &items,
            schema,
            limits.item_source,
        )?;
        validate_item_modifier_membership(
            input.normalization,
            schema,
            &items,
            &item_source,
            limits.normalization,
        )?;
        validate_item_parameter_inputs(
            input.normalization,
            schema,
            &items,
            &item_source,
            limits.normalization,
        )?;
        let tree = input
            .tree
            .map(|value| {
                OwnedTreeNormalizationPolicy::new(
                    value.clone(),
                    registry,
                    schema,
                    &mapping,
                    input.normalization,
                    limits.tree,
                )
            })
            .transpose()?;
        validate_passive_socket_placement(
            input.normalization,
            schema,
            &mapping,
            &items,
            &item_source,
            tree.as_ref(),
            limits.normalization,
        )?;
        Ok(Self {
            mapping,
            roles,
            normalization: CheckedNormalizationPolicy {
                input: input.normalization.clone(),
                identity: normalization,
            },
            rewards,
            items,
            item_source,
            tree,
        })
    }
}
