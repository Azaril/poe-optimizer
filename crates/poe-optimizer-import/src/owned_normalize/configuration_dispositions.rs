//! Checked configuration responsibilities, independent of native game rules.
//!
//! Adapter receipts and injected source observations share one accounting owner.
//! A leaf receipt never completes a configuration or scenario inventory.
use super::*;
mod accounting;

pub(super) use accounting::{AdapterReceipts, account};

/// Produced only after an adapter checks its actual typed output and scope.
pub(super) struct CheckedConfigurationReceipt {
    pub(super) scope: SourceOccurrenceId,
    pub(super) origin: Option<SourceOccurrenceId>,
    pub(super) selector: Option<ValueSelector>,
    pub(super) control: ConfigurationAdapterControl,
    pub(super) source_state: ConfigurationSourceState,
    pub(super) target: OwnedOriginTarget,
    pub(super) source_only: Option<OwnedDefinitionKey>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ConfigurationSourceState {
    Authored,
    AuthoredNone,
    Placeholder,
    ConstructorDefault,
    ConstructorDefaultNone,
    Absent,
    /// Existing raw-only adapter authority, after checking both presence and value.
    OverwrittenRawPlaceholder,
    IgnoredDefaultPlaceholder,
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
    pub rewards: OwnedContentDigest,
}

/// An import control's existing semantic adapter, not a new runtime destination.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigurationAdapterControl {
    Encounter,
    EnemyLevel,
    Reward { recipe: OwnedDefinitionKey },
    ScalarInput { input: ExternalInputDefId },
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
/// Rows are a partial census and do not carry a complete-inventory permission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDispositionPolicy {
    pub mapping_source: OwnedContentDigest,
    pub catalogue_evidence: OwnedContentDigest,
    pub constructor_evidence: OwnedContentDigest,
    pub dependencies: ConfigurationDispositionDependencies,
    pub rules: Vec<ConfigurationDispositionRule>,
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
    Ok(ConfigurationDispositionDependencies {
        encounter: component(&policy.encounter, limits)?,
        enemy_level: component(&policy.enemy_level, limits)?,
        scalar_inputs: component(&policy.configuration_inputs, limits)?,
        reward_inventory: component(&policy.configuration_reward_inventory, limits)?,
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
        rules.push(CompiledRule {
            selector: row.selector.clone(),
            destinations,
        });
    }
    Ok(Some(CompiledConfigurationDispositions { rules, work }))
}
