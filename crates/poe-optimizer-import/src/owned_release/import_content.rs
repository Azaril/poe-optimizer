//! Offline content construction; runtime readers still require exact bindings.
use super::*;
use crate::{
    owned_item_lines::ItemLinePolicyContent,
    owned_item_source::ItemSourceLayoutContent,
    owned_mapping::{MappingContent, OwnedIdRegistrySnapshot},
    owned_normalize::{NormalizationContent, NormalizationContentDependencies},
    owned_reward_policy::RewardPolicyContent,
    owned_skill_roles::SkillRoleContent,
    owned_tree_policy::TreeNormalizationContent,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;

/// Authored Import declarations. Historical evidence is retained; current
/// dependency commitments are constructed from the supplied checked packages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSourceContent {
    pub mapping: MappingContent,
    pub roles: SkillRoleContent,
    pub normalization: NormalizationContent,
    pub rewards: RewardPolicyContent,
    pub items: ItemLinePolicyContent,
    pub item_source: ItemSourceLayoutContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<TreeNormalizationContent>,
}

/// Constructed raw inputs, not a checked release or normalization certificate.
/// Ordinary complete-release assembly must validate them against the final
/// recipe and evaluation artifacts before they can be published or evaluated.
pub struct ImportSourceInput {
    pub mapping: MappingPackageInput,
    pub roles: OwnedSkillRolePackageInput,
    pub normalization: NormalizationPolicy,
    pub rewards: RewardPolicyInput,
    pub items: ItemLinePolicyInput,
    pub item_source: ItemSourceLayoutPolicyInput,
    pub tree: Option<TreeNormalizationPackageInput>,
}

impl ImportSourceContent {
    pub(super) fn from_checked(input: constituents::CheckedImportConstituents) -> Result<Self> {
        let constituents::CheckedImportConstituents {
            mapping,
            roles,
            normalization,
            rewards,
            items,
            item_source,
            tree,
        } = input;
        Ok(Self {
            mapping: mapping.into_authored_content(),
            roles: roles.into_authored_content(),
            normalization: normalization.into_authored_content(),
            rewards: rewards.into_authored_content(),
            items: items.into_authored_content()?,
            item_source: item_source.into_authored_content()?,
            tree: tree.map(OwnedTreeNormalizationPolicy::into_authored_content),
        })
    }

    /// Construct the connected Import dependency graph in one order. This does
    /// not accept sealed runtime inputs, rewrite historical evidence, or touch
    /// evaluation data. Callers still owe ordinary final release validation.
    pub fn into_input(
        self,
        registry: &OwnedIdRegistrySnapshot,
        schema: &OwnedDefinitionSchemaPackage,
        limits: OwnedReleaseLimits,
    ) -> Result<ImportSourceInput> {
        limits.validate()?;
        limits.normalization.validate()?;
        digest_owned(
            "owned-import-source-content-v1",
            &self,
            limits.max_input_bytes,
        )?;
        let Self {
            mapping,
            roles,
            normalization,
            rewards,
            items,
            item_source,
            tree,
        } = self;
        let mapping =
            OwnedMappingIndex::bind_new(mapping, registry, schema, limits.catalog.mapping)?;
        let roles = OwnedSkillRoleIndex::bind_new(roles, &mapping, schema, limits.catalog)?;
        let rewards = OwnedRewardPolicy::bind_new(rewards, &mapping, schema, limits.rewards)?;
        let items = OwnedItemLinePolicy::bind_new(items, schema, limits.items)?;
        let item_source =
            ItemSourceLayoutPolicy::bind_new(item_source, &items, schema, limits.item_source)?;
        let normalization = normalization.into_input(
            NormalizationContentDependencies {
                definitions: schema,
                mappings: &mapping,
                roles: &roles,
                rewards: &rewards,
                items: &items,
                item_source: &item_source,
                tree: tree.as_ref(),
                tree_limits: limits.tree,
            },
            limits.normalization,
        )?;
        let tree = tree
            .map(|content| {
                OwnedTreeNormalizationPolicy::bind_new(
                    content,
                    registry,
                    schema,
                    &mapping,
                    &normalization,
                    limits.tree,
                )
                .map(|v| v.input().clone())
            })
            .transpose()?;
        Ok(ImportSourceInput {
            mapping: mapping.input().clone(),
            roles: roles.input().clone(),
            normalization,
            rewards: rewards.input().clone(),
            items: items.input().clone(),
            item_source: item_source.input().clone(),
            tree,
        })
    }
}
