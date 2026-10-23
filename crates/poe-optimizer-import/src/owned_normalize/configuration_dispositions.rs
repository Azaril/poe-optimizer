//! Checked configuration responsibilities, independent of native game rules.
//!
//! Adapter receipts and injected source observations share one accounting owner.
//! A leaf receipt never completes a configuration or scenario inventory.
use super::*;
mod accounting;
mod inventory;
mod receipts;
pub use inventory::{
    ConfigurationCensusClosure, ConfigurationInventoryControl, ConfigurationInventoryDisposition,
    ConfigurationInventoryPolicy, ConfigurationSourceCensus,
    configuration_inventory_census_identity,
};

pub(super) use accounting::account;
pub(super) use receipts::AdapterReceipts;

/// Produced only after an adapter checks its actual typed output and scope.
pub(super) struct CheckedConfigurationReceipt {
    pub(super) scope: SourceOccurrenceId,
    pub(super) origin: Option<SourceOccurrenceId>,
    pub(super) selector: Option<ValueSelector>,
    pub(super) control: ConfigurationAdapterControl,
    pub(super) source_state: ConfigurationSourceState,
    pub(super) target: OwnedOriginTarget,
    /// Exact additional provenance established by the same checked output.
    /// Other adapters keep this empty; a selected Reward also retains its own ID.
    pub(super) additional_targets: Vec<OwnedOriginTarget>,
    pub(super) source_only: Option<OwnedDefinitionKey>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationSourceState {
    Authored,
    AuthoredNone,
    Placeholder,
    ConstructorDefault,
    ConstructorDefaultNone,
    Absent,
    /// Existing raw-only adapter authority, after checking both presence and value.
    OverwrittenRawPlaceholder,
    IgnoredDefaultPlaceholder,
    UnselectedProjection,
}

/// Commitments to the exact adapters whose output is being accounted for.
/// Deliberately excludes this policy, so its identity has no self dependency.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDispositionDependencies {
    pub encounter: Option<OwnedContentDigest>,
    pub enemy_level: Option<OwnedContentDigest>,
    pub scalar_inputs: Option<OwnedContentDigest>,
    pub reward_inventory: Option<OwnedContentDigest>,
    pub usage_inputs: OwnedContentDigest,
    pub rewards: OwnedContentDigest,
}

/// An import control's existing semantic adapter, not a new runtime destination.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigurationAdapterControl {
    Encounter,
    EnemyLevel,
    Reward {
        recipe: OwnedDefinitionKey,
    },
    ScalarInput {
        input: ExternalInputDefId,
    },
    UsageIntent {
        policy: UsagePolicyDefId,
        parameter: DeclaredSlot<ParameterSlotDefId>,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationResponsibility {
    BuildChoices,
    ScenarioAssumptions,
    ScenarioUsage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigurationDestination {
    /// Requires exact scoped source and emitted-output correspondence.
    Adapter {
        control: ConfigurationAdapterControl,
    },
    /// A reviewed branch consumes no independent owned input from this leaf.
    /// Every guard must have been proved by the bound Encounter adapter.
    InapplicableObservation {
        recipe: Box<ValueRecipeInput>,
        absent_input_names: Vec<String>,
        reason: OwnedDefinitionKey,
    },
    /// Records a reviewed remaining responsibility. It grants no leaf retirement
    /// or inventory completion while that responsibility remains unresolved.
    Deferred {
        responsibility: ConfigurationResponsibility,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDispositionRule {
    pub id: OwnedDefinitionKey,
    pub selector: ValueSelector,
    /// The reviewed list must include every destination of the source control.
    pub destinations: Vec<ConfigurationDestination>,
}

/// Reviewed import data. Evidence digests are authenticated by offline publication;
/// normalization verifies their source/adapter bindings and actual correspondence.
/// Leaf rows are a partial census. Only the separate source-bound inventory
/// policy can request its strictly checked, bounded completion domain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDispositionPolicy {
    pub mapping_source: OwnedContentDigest,
    pub catalogue_evidence: OwnedContentDigest,
    pub constructor_evidence: OwnedContentDigest,
    pub dependencies: ConfigurationDispositionDependencies,
    pub rules: Vec<ConfigurationDispositionRule>,
    /// Absent means no inventory authority, not an obsolete wire format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<ConfigurationInventoryPolicy>,
}

/// Authored responsibilities and historical evidence, without current adapter
/// commitments. The optional census and its evidence identity remain unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDispositionContent {
    pub mapping_source: OwnedContentDigest,
    pub catalogue_evidence: OwnedContentDigest,
    pub constructor_evidence: OwnedContentDigest,
    pub rules: Vec<ConfigurationDispositionRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<ConfigurationInventoryPolicy>,
}

impl ConfigurationDispositionContent {
    /// Only the checked aggregate owner may discard dependency commitments.
    pub(super) fn from_checked(input: ConfigurationDispositionPolicy) -> Self {
        let ConfigurationDispositionPolicy {
            mapping_source,
            catalogue_evidence,
            constructor_evidence,
            dependencies: _,
            rules,
            inventory,
        } = input;
        Self {
            mapping_source,
            catalogue_evidence,
            constructor_evidence,
            rules,
            inventory,
        }
    }

    /// Constructs raw input for the existing shared semantic validators.
    pub(super) fn into_input(
        self,
        prerequisites: ConfigurationDispositionPrerequisites<'_>,
        rewards: &OwnedRewardPolicy,
        limits: NormalizationLimits,
    ) -> Result<ConfigurationDispositionPolicy> {
        Ok(ConfigurationDispositionPolicy {
            mapping_source: self.mapping_source,
            catalogue_evidence: self.catalogue_evidence,
            constructor_evidence: self.constructor_evidence,
            dependencies: dependencies_from_prerequisites(prerequisites, rewards, limits)?,
            rules: self.rules,
            inventory: self.inventory,
        })
    }
}

/// Borrow the actual constructed optional adapters before the aggregate policy
/// exists. Absence is committed exactly as by the existing public helper.
pub(super) struct ConfigurationDispositionPrerequisites<'a> {
    pub(super) encounter: &'a Option<EncounterPolicy>,
    pub(super) enemy_level: &'a Option<EnemyLevelPolicy>,
    pub(super) scalar_inputs: &'a Option<ConfigurationInputsPolicy>,
    pub(super) reward_inventory: &'a Option<ConfigurationRewardInventoryPolicy>,
    pub(super) usage_inputs: &'a Option<UsageInputPolicy>,
}

fn component<T: Serialize>(
    value: &Option<T>,
    limits: NormalizationLimits,
) -> Result<Option<OwnedContentDigest>> {
    value
        .as_ref()
        .map(|v| {
            digest_owned("owned-configuration-adapter-v1", v, limits.max_policy_bytes)
                .map_err(Into::into)
        })
        .transpose()
}

/// Authoring helper; construction alone grants no source or output authority.
pub fn configuration_disposition_dependencies(
    policy: &NormalizationPolicy,
    rewards: &OwnedRewardPolicy,
    limits: NormalizationLimits,
) -> Result<ConfigurationDispositionDependencies> {
    dependencies_from_prerequisites(
        ConfigurationDispositionPrerequisites {
            encounter: &policy.encounter,
            enemy_level: &policy.enemy_level,
            scalar_inputs: &policy.configuration_inputs,
            reward_inventory: &policy.configuration_reward_inventory,
            usage_inputs: &policy.usage_inputs,
        },
        rewards,
        limits,
    )
}

pub(super) fn dependencies_from_prerequisites(
    prerequisites: ConfigurationDispositionPrerequisites<'_>,
    rewards: &OwnedRewardPolicy,
    limits: NormalizationLimits,
) -> Result<ConfigurationDispositionDependencies> {
    Ok(ConfigurationDispositionDependencies {
        encounter: component(prerequisites.encounter, limits)?,
        enemy_level: component(prerequisites.enemy_level, limits)?,
        scalar_inputs: component(prerequisites.scalar_inputs, limits)?,
        reward_inventory: component(prerequisites.reward_inventory, limits)?,
        usage_inputs: super::usage_inputs::usage_policy_identity(
            prerequisites.usage_inputs,
            limits,
        )?,
        rewards: *rewards.identity(),
    })
}

pub(super) fn validate_rewards(
    policy: &NormalizationPolicy,
    rewards: &OwnedRewardPolicy,
) -> Result<()> {
    if policy
        .configuration_dispositions
        .as_ref()
        .is_some_and(|p| p.dependencies.rewards != *rewards.identity())
    {
        return Err(NormalizationError::Binding);
    }
    Ok(())
}

/// Only inherited releases whose prior commitments were already checked may call
/// this helper. Replacement policies must be validated without repairing them.
pub(crate) fn rebind(
    policy: &mut NormalizationPolicy,
    rewards: &OwnedRewardPolicy,
    limits: NormalizationLimits,
) -> Result<()> {
    if policy.configuration_dispositions.is_none() {
        return Ok(());
    }
    let dependencies = configuration_disposition_dependencies(policy, rewards, limits)?;
    if let Some(policy) = &mut policy.configuration_dispositions {
        policy.dependencies = dependencies;
    }
    Ok(())
}

pub(super) enum CompiledDestination {
    Adapter(ConfigurationAdapterControl),
    Inapplicable {
        recipe: Box<ValueRecipe>,
        absent: BTreeSet<String>,
        reason: OwnedDefinitionKey,
    },
    Deferred(ConfigurationResponsibility),
}
pub(super) struct CompiledRule {
    selector: ValueSelector,
    destinations: Vec<CompiledDestination>,
}
pub(super) struct CompiledConfigurationDispositions {
    rules: Vec<CompiledRule>,
    inventory: Option<inventory::CompiledInventory>,
    work: usize,
}

fn name_valid(name: &str, limits: NormalizationLimits) -> bool {
    !name.is_empty()
        && name.len() <= limits.value.max_selector_bytes
        && name.trim_ascii() == name
        && !name.chars().any(char::is_control)
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledConfigurationDispositions>> {
    let Some(p) = &policy.configuration_dispositions else {
        return Ok(None);
    };
    if p.mapping_source != *mappings.source_identity()
        || p.dependencies.encounter != component(&policy.encounter, limits)?
        || p.dependencies.enemy_level != component(&policy.enemy_level, limits)?
        || p.dependencies.scalar_inputs != component(&policy.configuration_inputs, limits)?
        || p.dependencies.reward_inventory
            != component(&policy.configuration_reward_inventory, limits)?
        || p.dependencies.usage_inputs != usage_inputs_identity(policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    if p.rules.len() > limits.value.max_selectors
        || p.rules.len() > limits.draft.input.max_collection_entries
    {
        return Err(NormalizationError::Limit("configuration disposition rows"));
    }
    let mut work = 0usize;
    let mut ids = BTreeSet::new();
    let mut selectors = BTreeSet::new();
    let mut rules = Vec::new();
    for row in &p.rules {
        work = work
            .saturating_add(row.id.as_str().len())
            .saturating_add(row.selector.name.len())
            .saturating_add(row.destinations.len());
        if work > limits.max_work || work > limits.value.max_total_selector_bytes {
            return Err(NormalizationError::Limit(
                "configuration disposition policy work",
            ));
        }
        if !ids.insert(&row.id)
            || !selectors.insert(&row.selector)
            || !name_valid(&row.selector.name, limits)
            || !matches!(
                row.selector.lane,
                ValueLane::InputNumber
                    | ValueLane::InputBoolean
                    | ValueLane::InputString
                    | ValueLane::PlaceholderNumber
                    | ValueLane::PlaceholderString
            )
            || row.destinations.is_empty()
            || row.destinations.len() > limits.value.max_selectors
        {
            return Err(NormalizationError::Policy("configuration disposition rule"));
        }
        let mut destinations = Vec::new();
        let mut adapters = BTreeSet::new();
        let mut deferred = BTreeSet::new();
        for destination in &row.destinations {
            destinations.push(match destination {
                ConfigurationDestination::Adapter { control } => {
                    if !adapters.insert(control) {
                        return Err(NormalizationError::Policy("duplicate configuration destination"));
                    }
                    let adapter_work = match control {
                        ConfigurationAdapterControl::Reward { recipe } => {
                            recipe.as_str().len().saturating_add(policy.configuration_reward_inventory.as_ref().map_or(0, |p| match p {
                                ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 { controls, .. } => controls.iter().fold(0usize, |n, c| n.saturating_add(c.recipe.as_str().len()).saturating_add(c.selector.name.len()).saturating_add(1)),
                            }))
                        },
                        ConfigurationAdapterControl::ScalarInput { input } => input.key().as_str().len(),
                        ConfigurationAdapterControl::UsageIntent { policy, parameter } => policy.key().as_str().len().saturating_add(parameter.slot.key().as_str().len()),
                        ConfigurationAdapterControl::Encounter | ConfigurationAdapterControl::EnemyLevel => 1,
                    };
                    work = work.saturating_add(adapter_work);
                    if work > limits.max_work { return Err(NormalizationError::Limit("configuration disposition policy work")); }
                    match control {
                        ConfigurationAdapterControl::Encounter if policy.encounter.is_some() => {},
                        ConfigurationAdapterControl::EnemyLevel if policy.enemy_level.is_some() => {},
                        ConfigurationAdapterControl::Reward { recipe } if policy.configuration_reward_inventory.as_ref().is_some_and(|p| match p {
                            ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 { controls, .. } => controls.iter().any(|c| &c.recipe == recipe && c.selector == row.selector),
                        }) => {},
                        ConfigurationAdapterControl::ScalarInput { input } if policy.configuration_inputs.is_some() && input.namespace() == &policy.namespace && matches!(definitions.definition(input), SchemaLookup::Known(_)) => {},
                        ConfigurationAdapterControl::UsageIntent { .. } if usage_inputs::configuration_control_matches(policy, &row.selector, control, &mut work, limits)? => {},
                        _ => return Err(NormalizationError::Policy("configuration disposition adapter")),
                    }
                    CompiledDestination::Adapter(control.clone())
                },
                ConfigurationDestination::Deferred { responsibility } => {
                    if !deferred.insert(*responsibility) { return Err(NormalizationError::Policy("duplicate configuration responsibility")); }
                    CompiledDestination::Deferred(*responsibility)
                },
                ConfigurationDestination::InapplicableObservation { recipe, absent_input_names, reason } => {
                    let Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 { absent_input_names: proven, .. }) = &policy.encounter else {
                        return Err(NormalizationError::Policy("configuration observation requires encounter"));
                    };
                    work = work.saturating_add(reason.as_str().len()).saturating_add(recipe.id.as_str().len()).saturating_add(absent_input_names.len().saturating_mul(proven.iter().map(|name| name.len().saturating_add(1)).sum::<usize>()));
                    if work > limits.max_work { return Err(NormalizationError::Limit("configuration disposition policy work")); }
                    if row.destinations.len() != 1 || row.selector.lane != ValueLane::PlaceholderNumber
                        || recipe.codec.namespace != policy.namespace || recipe.tiers.len() != 1
                        || recipe.tiers[0].selectors.as_slice() != [row.selector.clone()]
                        || recipe.tiers[0].duplicates != DuplicatePolicy::Reject
                        || !matches!(recipe.missing, MissingValuePolicy::Pending) || !recipe.numeric_aliases.is_empty()
                        || !matches!(recipe.codec.codec, ValueCodecKind::Integer { .. } | ValueCodecKind::Quantity { .. })
                        || absent_input_names.is_empty() || absent_input_names.len() > limits.value.max_selectors {
                        return Err(NormalizationError::Policy("configuration observation grammar"));
                    }
                    let mut absent = BTreeSet::new();
                    for guard in absent_input_names {
                        work = work.saturating_add(guard.len()).saturating_add(1);
                        if work > limits.max_work || work > limits.value.max_total_selector_bytes { return Err(NormalizationError::Limit("configuration disposition policy work")); }
                        if !name_valid(guard, limits) || !proven.contains(guard) || !absent.insert(guard.clone()) { return Err(NormalizationError::Policy("configuration observation absent guard")); }
                    }
                    CompiledDestination::Inapplicable { recipe: Box::new(ValueRecipe::new(recipe.as_ref().clone(), limits.value)?), absent, reason: reason.clone() }
                },
            });
        }
        configuration_inputs::check_projection_destinations(
            policy,
            &row.selector,
            &adapters,
            &mut work,
            limits,
        )?;
        usage_inputs::check_configuration_projection_destinations(
            policy,
            &row.selector,
            &adapters,
            &mut work,
            limits,
        )?;
        rules.push(CompiledRule {
            selector: row.selector.clone(),
            destinations,
        });
    }
    let inventory = inventory::compile(p, policy, definitions, limits, &mut work)?;
    Ok(Some(CompiledConfigurationDispositions {
        rules,
        inventory,
        work,
    }))
}
