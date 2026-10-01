//! A finite fresh-configuration branch, not a callback or configuration evaluator.
use super::source_shape::{fresh_config_sets, value};
use super::*;
use crate::owned_value::DecimalSyntax;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnemyLevelPolicy {
    /// Reviewed source evidence establishes the final level under these absent
    /// input keys. A matching saved placeholder confirms this finite domain;
    /// it is not a general fallback for missing values or source callbacks.
    PobFreshDefaultConfigLevelV1 {
        mapping_source: OwnedContentDigest,
        absent_input_names: Vec<String>,
        placeholder: ValueRecipeInput,
        expected_level: u16,
    },
}

pub(super) struct CompiledEnemyLevel<'p> {
    absent: BTreeSet<&'p str>,
    recipe: ValueRecipe,
    expected: u16,
    work: usize,
}

pub(super) fn compile<'p>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledEnemyLevel<'p>>> {
    let Some(EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 {
        mapping_source,
        absent_input_names,
        placeholder,
        expected_level,
    }) = &policy.enemy_level
    else {
        return Ok(None);
    };
    if mapping_source != mappings.source_identity()
        || placeholder.codec.namespace != policy.namespace
    {
        return Err(NormalizationError::Binding);
    }
    if absent_input_names.is_empty()
        || absent_input_names.len() > 16
        || absent_input_names.len() > limits.value.max_selectors
        || *expected_level == 0
        || placeholder.tiers.len() != 1
        || placeholder.tiers[0].selectors.len() != 1
        || placeholder.tiers[0].duplicates != DuplicatePolicy::Reject
        || !matches!(placeholder.missing, MissingValuePolicy::Pending)
        || !placeholder.numeric_aliases.is_empty()
        || !matches!(
            placeholder.codec.codec,
            ValueCodecKind::Integer {
                syntax: DecimalSyntax::Integer
            }
        )
    {
        return Err(NormalizationError::Policy("enemy level profile"));
    }
    let selector = &placeholder.tiers[0].selectors[0];
    if selector.lane != ValueLane::PlaceholderNumber {
        return Err(NormalizationError::Policy("enemy level source lane"));
    }
    let mut work = absent_input_names.len();
    for name in absent_input_names {
        if name.is_empty()
            || name.len() > 128
            || name.len() > limits.value.max_selector_bytes
            || name.trim_ascii() != name
            || name.chars().any(char::is_control)
        {
            return Err(NormalizationError::Policy("enemy level absent keys"));
        }
        work = work
            .checked_add(name.len())
            .ok_or(NormalizationError::Limit("enemy level policy work"))?;
        if work > limits.max_work || work > limits.value.max_total_selector_bytes {
            return Err(NormalizationError::Limit("enemy level policy work"));
        }
    }
    let absent: BTreeSet<_> = absent_input_names.iter().map(String::as_str).collect();
    if absent.len() != absent_input_names.len() || !absent.contains(selector.name.as_str()) {
        return Err(NormalizationError::Policy("enemy level absent keys"));
    }
    let recipe = ValueRecipe::new(placeholder.clone(), limits.value)?;
    Ok(Some(CompiledEnemyLevel {
        absent,
        recipe,
        expected: *expected_level,
        work,
    }))
}

/// Private authority retains the exact source scope and chosen source attribute.
pub(super) struct ProvenEnemyLevel {
    scope: SourceOccurrenceId,
    origin: SourceAttributeRef,
    level: u16,
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    policy: Option<&CompiledEnemyLevel<'_>>,
) -> Result<BTreeMap<SourceOccurrenceId, ProvenEnemyLevel>> {
    let mut result = BTreeMap::new();
    let Some(policy) = policy else {
        return Ok(result);
    };
    b.charge(policy.work)?;
    let Some(sets) = fresh_config_sets(b)? else {
        return Ok(result);
    };
    let evidence = b.evidence;
    let selector = &policy.recipe.input().tiers[0].selectors[0];
    for scope in sets {
        let row = &evidence.rows()[scope.ordinal() as usize];
        let mut blocked = false;
        let mut candidate = None;
        for id in row.children() {
            b.charge(1)?;
            let child = &evidence.rows()[id.ordinal() as usize];
            let Some(name) = value(child, "name") else {
                continue;
            };
            if !policy.absent.contains(name) {
                continue;
            }
            if child.occurrence().name() != "Placeholder"
                || name != selector.name
                || value(child, "number").is_none()
            {
                blocked = true;
                break;
            }
            let (index, attribute) = b.attributes[id.ordinal() as usize]["number"];
            candidate = Some(ValueCandidate {
                selector,
                origin: SourceAttributeRef {
                    occurrence: *id,
                    index,
                },
                value: match attribute.decoded() {
                    Ok(text) => CandidateValue::Decoded(text),
                    Err(error) => CandidateValue::Unavailable(error),
                },
            });
        }
        if blocked {
            continue;
        }
        if let ValueOutcome::Selected {
            origin,
            value: ParameterValue::Integer(level),
        } = policy.recipe.decide(candidate.as_slice())?.outcome
            && level.get() == i64::from(policy.expected)
        {
            b.charge(1)?;
            result.insert(
                scope,
                ProvenEnemyLevel {
                    scope,
                    origin,
                    level: policy.expected,
                },
            );
        }
    }
    Ok(result)
}

pub(super) fn level(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: ScenarioPresetId,
    proof: Option<&ProvenEnemyLevel>,
) -> Result<DraftField<u16>> {
    let Some(proof) = proof else {
        return b.pending(scope, "enemy-level-not-converted");
    };
    if proof.scope != scope {
        return Err(NormalizationError::Policy("enemy level source scope"));
    }
    b.link(
        proof.origin.occurrence,
        OwnedOriginTarget::ScenarioPreset(scenario),
    )?;
    Ok(proof.level.into())
}
