//! A finite fresh-configuration branch, not a callback or configuration evaluator.
use super::source_shape::{charge_row, container_text, plain_row, value};
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

/// Canonical positive Lua-exact numeric configuration identities. Reject aliases
/// before using source keys: the source loader indexes them through tonumber.
fn numeric_key(text: &str) -> Option<u64> {
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value = text.parse::<u64>().ok()?;
    (value < (1_u64 << 53)).then_some(value)
}

/// Private authority retains the exact source scope and chosen source attribute.
pub(super) struct ProvenEnemyLevel {
    scope: SourceOccurrenceId,
    origin: SourceAttributeRef,
    level: u16,
}

/// Validate the complete container shape before admitting any independent set.
/// A malformed sibling cannot silently change which configuration source loaded.
fn config_sets(b: &mut Builder<'_, '_>) -> Result<Option<Vec<SourceOccurrenceId>>> {
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    if root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(None);
    }
    let mut config = None;
    for id in root.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        if row.occurrence().name() == "Config" {
            if config.replace(row).is_some() {
                return Ok(None);
            }
        } else if row.occurrence().name() == "ConfigSet" {
            return Ok(None);
        }
    }
    let Some(config) = config else {
        return Ok(None);
    };
    charge_row(b, config)?;
    if !plain_row(config, &["activeConfigSet"], false) || !container_text(config) {
        return Ok(None);
    }
    let Some(selected) = value(config, "activeConfigSet").and_then(numeric_key) else {
        return Ok(None);
    };
    if config.children().len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("enemy level configuration sets"));
    }
    let mut ids = BTreeSet::new();
    let mut sets = Vec::new();
    for id in config.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        charge_row(b, row)?;
        if row.occurrence().name() != "ConfigSet"
            || row.occurrence().parent() != Some(config.occurrence().id())
            || !plain_row(row, &["id", "title"], false)
            || !container_text(row)
            || !matches!(
                row.authored_instance(),
                Some(AuthoredInstanceId::ConfigSet(_))
            )
        {
            return Ok(None);
        }
        let Some(key) = value(row, "id").and_then(numeric_key) else {
            return Ok(None);
        };
        if !ids.insert(key) {
            return Ok(None);
        }
        let mut names = BTreeSet::new();
        for child in row.children() {
            let child = &evidence.rows()[child.ordinal() as usize];
            charge_row(b, child)?;
            match child.occurrence().name() {
                "Input" | "Placeholder" => {
                    if !plain_row(child, &["name", "number", "string", "boolean"], true)
                        || child.attributes().len() != 2
                    {
                        return Ok(None);
                    }
                    let Some(name) = value(child, "name").filter(|name| !name.is_empty()) else {
                        return Ok(None);
                    };
                    let numeric = value(child, "number").is_some();
                    let string = value(child, "string").is_some();
                    let boolean = value(child, "boolean");
                    if usize::from(numeric) + usize::from(string) + usize::from(boolean.is_some())
                        != 1
                        || boolean.is_some_and(|v| !matches!(v, "true" | "false"))
                        || (child.occurrence().name() == "Placeholder" && boolean.is_some())
                        || !names.insert((child.occurrence().name(), name))
                    {
                        return Ok(None);
                    }
                }
                "CustomModifierBlock" => {
                    if !plain_row(child, &["title", "enabled"], false)
                        || !child.children().is_empty()
                        || value(child, "enabled").is_some_and(|v| !matches!(v, "true" | "false"))
                    {
                        return Ok(None);
                    }
                }
                _ => return Ok(None),
            }
        }
        sets.push(*id);
    }
    Ok(ids.contains(&selected).then_some(sets))
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
    let Some(sets) = config_sets(b)? else {
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
