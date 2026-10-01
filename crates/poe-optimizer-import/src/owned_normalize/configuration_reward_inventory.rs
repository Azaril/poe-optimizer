//! Source-bound finite reward-list authority, separate from partial reward rules.
use super::source_shape::{fresh_config_sets, value};
use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationRewardControl {
    pub recipe: OwnedDefinitionKey,
    pub selector: ValueSelector,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigurationRewardInventoryPolicy {
    /// The injected census exhausts the generated reward controls in this exact
    /// source revision. It supplies no authority over other configuration roles.
    PobFreshGeneratedControlsV1 {
        mapping_source: OwnedContentDigest,
        reward_policy: OwnedContentDigest,
        controls: Vec<ConfigurationRewardControl>,
    },
}

pub(super) struct CompiledConfigurationRewardInventory<'p> {
    controls: BTreeMap<&'p str, &'p ValueSelector>,
    work: usize,
}

/// None deliberately performs no extra validation or work. Publication callers
/// validate the old commitment before rebinding a retained reward dependency.
pub(crate) fn validate_configuration_reward_inventory(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    rewards: &OwnedRewardPolicy,
    limits: NormalizationLimits,
) -> Result<()> {
    compile(policy, mappings, rewards, limits).map(|_| ())
}

pub(super) fn compile<'p>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    rewards: &OwnedRewardPolicy,
    limits: NormalizationLimits,
) -> Result<Option<CompiledConfigurationRewardInventory<'p>>> {
    let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
        mapping_source,
        reward_policy,
        controls,
    }) = &policy.configuration_reward_inventory
    else {
        return Ok(None);
    };
    if mapping_source != mappings.source_identity() || reward_policy != rewards.identity() {
        return Err(NormalizationError::Binding);
    }
    if controls.is_empty()
        || controls.len() > 256
        || controls.len() > limits.value.max_selectors
        || controls.len() > limits.draft.input.max_collection_entries
    {
        return Err(NormalizationError::Policy(
            "configuration reward control census",
        ));
    }
    if controls.len() != rewards.rules().len() {
        return Err(NormalizationError::Policy(
            "configuration reward rule census",
        ));
    }
    let mut work = controls.len();
    for control in controls {
        let name = &control.selector.name;
        if name.is_empty()
            || name.len() > limits.value.max_selector_bytes
            || name.trim_ascii() != name
            || name.chars().any(char::is_control)
            || !matches!(
                control.selector.lane,
                ValueLane::InputBoolean | ValueLane::InputString
            )
        {
            return Err(NormalizationError::Policy("configuration reward selector"));
        }
        work = work
            .checked_add(name.len())
            .and_then(|n| n.checked_add(control.recipe.as_str().len()))
            .ok_or(NormalizationError::Limit(
                "configuration reward policy work",
            ))?;
        if work > limits.max_work || work > limits.value.max_total_selector_bytes {
            return Err(NormalizationError::Limit(
                "configuration reward policy work",
            ));
        }
    }
    // Bound all policy rows/bytes before allocating indexes. One source name
    // cannot acquire competing lanes or two independent default authorities.
    let mut by_recipe = BTreeMap::new();
    let mut by_name = BTreeMap::new();
    for control in controls {
        if by_recipe
            .insert(&control.recipe, &control.selector)
            .is_some()
            || by_name
                .insert(control.selector.name.as_str(), &control.selector)
                .is_some()
        {
            return Err(NormalizationError::Policy(
                "configuration reward duplicate control",
            ));
        }
    }
    for rule in rewards.rules() {
        if rule.recipe.tiers.len() != 1
            || rule.recipe.tiers[0].selectors.len() != 1
            || by_recipe.get(&rule.recipe.id).copied() != rule.recipe.tiers[0].selectors.first()
        {
            return Err(NormalizationError::Policy(
                "configuration reward rule selector",
            ));
        }
    }
    Ok(Some(CompiledConfigurationRewardInventory {
        controls: by_name,
        work,
    }))
}

/// Private proof retains source scope, every reviewed explicit control origin,
/// and the exact ordered outputs. A known count alone is never authority.
pub(super) struct ProvenConfigurationRewards {
    scope: SourceOccurrenceId,
    reviewed: BTreeSet<SourceOccurrenceId>,
    rewards: Vec<(RewardDefId, Vec<ParameterAssignment>)>,
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    policy: Option<&CompiledConfigurationRewardInventory<'_>>,
) -> Result<BTreeMap<SourceOccurrenceId, ProvenConfigurationRewards>> {
    let mut proofs = BTreeMap::new();
    let Some(policy) = policy else {
        return Ok(proofs);
    };
    b.charge(policy.work)?;
    let Some(sets) = fresh_config_sets(b)? else {
        return Ok(proofs);
    };
    let evidence = b.evidence;
    let rewards = b.rewards;
    for scope in sets {
        let row = &evidence.rows()[scope.ordinal() as usize];
        let mut inputs = BTreeMap::new();
        let mut reviewed = BTreeSet::new();
        let mut uncertain = false;
        for id in row.children() {
            b.charge(1)?;
            let child = &evidence.rows()[id.ordinal() as usize];
            let Some(name) = value(child, "name") else {
                continue;
            };
            let Some(selector) = policy.controls.get(name) else {
                continue;
            };
            let attribute = match selector.lane {
                ValueLane::InputBoolean => "boolean",
                ValueLane::InputString => "string",
                _ => {
                    return Err(NormalizationError::Policy(
                        "configuration reward source lane",
                    ));
                }
            };
            if child.occurrence().name() != "Input"
                || value(child, attribute).is_none()
                || inputs.insert(name, (*id, attribute)).is_some()
            {
                uncertain = true;
                break;
            }
            reviewed.insert(*id);
        }
        if uncertain {
            continue;
        }
        let mut expected = Vec::new();
        for rule in rewards.rules() {
            b.charge(1)?;
            let selector = &rule.recipe.tiers[0].selectors[0];
            let candidate = inputs.get(selector.name.as_str()).map(|(id, attribute)| {
                let (index, value) = b.attributes[id.ordinal() as usize][attribute];
                ValueCandidate {
                    selector,
                    origin: SourceAttributeRef {
                        occurrence: *id,
                        index,
                    },
                    value: match value.decoded() {
                        Ok(text) => CandidateValue::Decoded(text),
                        Err(error) => CandidateValue::Unavailable(error),
                    },
                }
            });
            let decision = rewards.decide(&rule.recipe.id, candidate.as_slice())?;
            match decision.outcome {
                RewardOutcome::None => {}
                RewardOutcome::Reward {
                    definition,
                    parameters,
                } => {
                    b.charge(parameters.len().saturating_add(1))?;
                    expected.push((definition, parameters));
                }
                RewardOutcome::Unmapped { .. } => {
                    uncertain = true;
                    break;
                }
            }
        }
        if !uncertain {
            b.charge(1)?;
            proofs.insert(
                scope,
                ProvenConfigurationRewards {
                    scope,
                    reviewed,
                    rewards: expected,
                },
            );
        }
    }
    Ok(proofs)
}

pub(super) fn finish(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    choice: ChoicePresetId,
    roles: &DraftListCompletion,
    output: &mut DraftList<RewardSelectionId>,
    emitted: &[RewardDraft],
    proof: Option<&ProvenConfigurationRewards>,
) -> Result<()> {
    let Some(proof) = proof else {
        return Ok(());
    };
    let DraftListCompletion::Pending { id: retired, code } = &output.completion else {
        return Err(NormalizationError::Policy(
            "configuration reward obligation",
        ));
    };
    let DraftListCompletion::Pending {
        id: roles_issue,
        code: roles_code,
    } = roles
    else {
        return Err(NormalizationError::Policy("configuration reward roles"));
    };
    if proof.scope != scope
        || code.as_str() != "configuration-rewards-not-converted"
        || roles_code.as_str() != "configuration-roles-not-converted"
        || output.members.len() != proof.rewards.len()
        || emitted.len() != proof.rewards.len()
    {
        return Err(NormalizationError::Policy(
            "configuration reward correspondence",
        ));
    }
    b.charge(emitted.len())?;
    for ((id, actual), (definition, parameters)) in
        output.members.iter().zip(emitted).zip(&proof.rewards)
    {
        b.charge(parameters.len())?;
        if *id != actual.id
            || !matches!(&actual.definition, DraftField::Known { value } if value == definition)
            || actual.parameters.completion != DraftListCompletion::Complete
            || actual.parameters.members.len() != parameters.len()
            || !actual
                .parameters
                .members
                .iter()
                .zip(parameters)
                .all(|(a, e)| a.to_resolved().as_ref() == Some(e))
        {
            return Err(NormalizationError::Policy(
                "configuration reward correspondence",
            ));
        }
    }
    let retired = *retired;
    let roles_issue = *roles_issue;
    let evidence = b.evidence;
    let row = &evidence.rows()[scope.ordinal() as usize];
    for id in std::iter::once(&scope).chain(row.children()) {
        b.charge(
            b.origins[id.ordinal() as usize]
                .links
                .len()
                .saturating_add(1),
        )?;
        b.origins[id.ordinal() as usize]
            .links
            .retain(|link| !matches!(link, OwnedOriginTarget::Issue(issue) if *issue == retired));
        if proof.reviewed.contains(id) {
            b.link(*id, OwnedOriginTarget::ChoicePreset(choice))?;
        } else if b.origins[id.ordinal() as usize].links.is_empty() {
            // Keep unrelated settings unresolved in their own configuration;
            // never let the later fallback attach issues from other scopes.
            b.link(*id, OwnedOriginTarget::Issue(roles_issue))?;
        }
    }
    output.completion = DraftListCompletion::Complete;
    Ok(())
}
