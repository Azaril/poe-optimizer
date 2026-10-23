//! Authored interpretation, separate from current dependency commitments.
use super::*;
use crate::owned_tree_policy::{TreeNormalizationContent, TreePolicyLimits, tree_content_identity};

fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

/// Semantic import declarations and historical evidence. This is not a checked
/// normalization policy, nor permission to repair a supplied runtime artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizationContent {
    pub version: OwnedDefinitionKey,
    pub character_level: ValueRecipeInput,
    pub gem_level: ValueRecipeInput,
    pub gem_enabled: ValueRecipeInput,
    pub group_enabled: ValueRecipeInput,
    pub manual_skill_sources: Vec<SourceComponent>,
    pub empty_item_keys: Vec<SourceComponent>,
    pub generated_support_prefixes: Vec<String>,
    pub allocation_attribute: String,
    pub single_active_support_target: bool,
    pub equipment_loadouts: Vec<EquipmentLoadoutRule>,
    pub gem_quality: GemQualityContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_scopes: Option<SkillScopePolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gem_inputs: Option<GemInputContent>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub direct_skill_inputs: Option<DirectSkillInputContent>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub direct_support_targets: Option<DirectSupportTargetContent>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub generated_skill_inputs: Option<GeneratedSkillInputContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gem_inventory: Option<GemInventoryContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_inputs: Option<UsageInputContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_origin_order: Option<SupportOriginOrderContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_inventory: Option<PayloadInventoryContent>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub skill_inventory: Option<SkillInventoryContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment_membership: Option<EquipmentMembershipContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passive_socket_membership: Option<PassiveSocketMembershipContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_modifier_membership: Option<ItemModifierMembershipContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_parameter_inputs: Option<ItemParameterInputsContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_level: Option<EnemyLevelPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_reward_inventory: Option<ConfigurationRewardInventoryContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encounter: Option<EncounterPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_reward_inventory: Option<CharacterRewardInventoryPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_inputs: Option<ConfigurationInputsPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_dispositions: Option<ConfigurationDispositionContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_presentation: Option<SourcePresentationPolicy>,
}

pub(crate) struct NormalizationContentDependencies<'a, I> {
    pub definitions: &'a I,
    pub mappings: &'a OwnedMappingIndex,
    pub roles: &'a OwnedSkillRoleIndex,
    pub rewards: &'a OwnedRewardPolicy,
    pub items: &'a OwnedItemLinePolicy,
    pub item_source: &'a ItemSourceLayoutPolicy,
    pub tree: Option<&'a TreeNormalizationContent>,
    pub tree_limits: TreePolicyLimits,
}

impl NormalizationContent {
    /// Called only when consuming the enclosing checked normalization owner.
    pub(crate) fn from_checked(input: NormalizationPolicy) -> Self {
        let NormalizationPolicy {
            version,
            namespace: _,
            character_level,
            gem_level,
            gem_enabled,
            group_enabled,
            manual_skill_sources,
            empty_item_keys,
            generated_support_prefixes,
            allocation_attribute,
            single_active_support_target,
            equipment_loadouts,
            gem_quality,
            skill_scopes,
            gem_inputs,
            direct_skill_inputs,
            direct_support_targets,
            generated_skill_inputs,
            gem_inventory,
            usage_inputs,
            support_origin_order,
            payload_inventory,
            skill_inventory,
            equipment_membership,
            passive_socket_membership,
            item_modifier_membership,
            item_parameter_inputs,
            enemy_level,
            configuration_reward_inventory,
            encounter,
            character_reward_inventory,
            configuration_inputs,
            configuration_dispositions,
            source_presentation,
        } = input;
        Self {
            version,
            character_level,
            gem_level,
            gem_enabled,
            group_enabled,
            manual_skill_sources,
            empty_item_keys,
            generated_support_prefixes,
            allocation_attribute,
            single_active_support_target,
            equipment_loadouts,
            gem_quality: GemQualityContent::from_checked(gem_quality),
            skill_scopes,
            gem_inputs: gem_inputs.map(GemInputContent::from_checked),
            direct_skill_inputs: direct_skill_inputs.map(DirectSkillInputContent::from_checked),
            direct_support_targets: direct_support_targets
                .map(DirectSupportTargetContent::from_checked),
            generated_skill_inputs: generated_skill_inputs
                .map(GeneratedSkillInputContent::from_checked),
            gem_inventory: gem_inventory.map(GemInventoryContent::from_checked),
            usage_inputs: usage_inputs.map(UsageInputContent::from_checked),
            support_origin_order: support_origin_order.map(SupportOriginOrderContent::from_checked),
            payload_inventory: payload_inventory.map(PayloadInventoryContent::from_checked),
            skill_inventory: skill_inventory.map(SkillInventoryContent::from_checked),
            equipment_membership: equipment_membership
                .map(EquipmentMembershipContent::from_checked),
            passive_socket_membership: passive_socket_membership
                .map(PassiveSocketMembershipContent::from_checked),
            item_modifier_membership: item_modifier_membership
                .map(ItemModifierMembershipContent::from_checked),
            item_parameter_inputs: item_parameter_inputs
                .map(ItemParameterInputsContent::from_checked),
            enemy_level,
            configuration_reward_inventory: configuration_reward_inventory
                .map(ConfigurationRewardInventoryContent::from_checked),
            encounter,
            character_reward_inventory,
            configuration_inputs,
            configuration_dispositions: configuration_dispositions
                .map(ConfigurationDispositionContent::from_checked),
            source_presentation,
        }
    }

    /// Construct one raw policy in dependency order. Complete release checking
    /// still owns all membership, semantic, coverage and work-limit validation.
    pub(crate) fn into_input<I: DefinitionSchemaIndex>(
        self,
        dependencies: NormalizationContentDependencies<'_, I>,
        limits: NormalizationLimits,
    ) -> Result<NormalizationPolicy> {
        limits.validate()?;
        // Bound authored data before conversion allocates replacement row vectors.
        digest_owned(
            "owned-normalization-content-v1",
            &self,
            limits.max_policy_bytes,
        )?;
        let NormalizationContentDependencies {
            definitions,
            mappings,
            roles,
            rewards,
            items,
            item_source,
            tree,
            tree_limits,
        } = dependencies;
        let Self {
            version,
            character_level,
            gem_level,
            gem_enabled,
            group_enabled,
            manual_skill_sources,
            empty_item_keys,
            generated_support_prefixes,
            allocation_attribute,
            single_active_support_target,
            equipment_loadouts,
            gem_quality,
            skill_scopes,
            gem_inputs,
            direct_skill_inputs,
            direct_support_targets,
            generated_skill_inputs,
            gem_inventory,
            usage_inputs,
            support_origin_order,
            payload_inventory,
            skill_inventory,
            equipment_membership,
            passive_socket_membership,
            item_modifier_membership,
            item_parameter_inputs,
            enemy_level,
            configuration_reward_inventory,
            encounter,
            character_reward_inventory,
            configuration_inputs,
            configuration_dispositions,
            source_presentation,
        } = self;
        let gem_quality = gem_quality.into_input(definitions);
        let gem_inputs = gem_inputs.map(|v| v.into_input(definitions));
        let direct_skill_inputs = direct_skill_inputs.map(|v| v.into_input(definitions, roles));
        let direct_support_targets = direct_support_targets
            .map(|v| v.into_input(direct_skill_inputs.as_ref(), limits))
            .transpose()?;
        let generated_skill_inputs =
            generated_skill_inputs.map(|v| v.into_input(definitions, roles));
        let skill_inventory = skill_inventory
            .map(|v| v.into_input(roles, direct_skill_inputs.as_ref(), limits))
            .transpose()?;
        let scalar_inputs = if usage_inputs.is_some() || gem_inventory.is_some() {
            Some(gem_inventory::scalar_inputs_identity(
                gem_inventory::ScalarInputs {
                    gem_inputs: &gem_inputs,
                    gem_level: &gem_level,
                    gem_quality: &gem_quality,
                    gem_enabled: &gem_enabled,
                    group_enabled: &group_enabled,
                    manual_skill_sources: &manual_skill_sources,
                    generated_support_prefixes: &generated_support_prefixes,
                },
                limits,
            )?)
        } else {
            None
        };
        let usage_inputs = usage_inputs.map(|v| {
            v.into_input(
                definitions,
                roles,
                scalar_inputs.expect("usage prerequisite computed"),
            )
        });
        let gem_inventory = gem_inventory
            .map(|v| {
                v.into_input(
                    definitions,
                    roles,
                    scalar_inputs.expect("inventory prerequisite computed"),
                    &usage_inputs,
                    limits,
                )
            })
            .transpose()?;
        let support_origin_order = support_origin_order.map(|v| v.into_input(roles));
        let payload_inventory = payload_inventory.map(|v| v.into_input(roles));
        let configuration_reward_inventory =
            configuration_reward_inventory.map(|v| v.into_input(rewards));
        let configuration_dispositions = configuration_dispositions
            .map(|v| {
                v.into_input(
                    configuration_dispositions::ConfigurationDispositionPrerequisites {
                        encounter: &encounter,
                        enemy_level: &enemy_level,
                        scalar_inputs: &configuration_inputs,
                        reward_inventory: &configuration_reward_inventory,
                        usage_inputs: &usage_inputs,
                    },
                    rewards,
                    limits,
                )
            })
            .transpose()?;
        let equipment_membership =
            equipment_membership.map(|v| v.into_input(definitions, items, item_source));
        let item_modifier_membership =
            item_modifier_membership.map(|v| v.into_input(definitions, items, item_source));
        let item_parameter_inputs =
            item_parameter_inputs.map(|v| v.into_input(definitions, items, item_source));
        let passive_socket_membership = passive_socket_membership
            .map(|v| {
                let (equipment, tree_content) = if matches!(
                    v,
                    PassiveSocketMembershipContent::PobOrdinarySharedSpecSocketsV2 { .. }
                ) {
                    (
                        equipment_membership
                            .as_ref()
                            .map(|v| equipment_membership_identity(v, limits))
                            .transpose()?,
                        tree.map(|v| tree_content_identity(v, tree_limits))
                            .transpose()?,
                    )
                } else {
                    (None, None)
                };
                v.into_input(
                    definitions,
                    mappings,
                    items,
                    item_source,
                    equipment,
                    tree_content,
                )
            })
            .transpose()?;
        Ok(NormalizationPolicy {
            version,
            namespace: definitions.namespace().clone(),
            character_level,
            gem_level,
            gem_enabled,
            group_enabled,
            manual_skill_sources,
            empty_item_keys,
            generated_support_prefixes,
            allocation_attribute,
            single_active_support_target,
            equipment_loadouts,
            gem_quality,
            skill_scopes,
            gem_inputs,
            direct_skill_inputs,
            direct_support_targets,
            generated_skill_inputs,
            gem_inventory,
            usage_inputs,
            support_origin_order,
            payload_inventory,
            skill_inventory,
            equipment_membership,
            passive_socket_membership,
            item_modifier_membership,
            item_parameter_inputs,
            enemy_level,
            configuration_reward_inventory,
            encounter,
            character_reward_inventory,
            configuration_inputs,
            configuration_dispositions,
            source_presentation,
        })
    }
}
