//! Raw numeric Config overrides, distinct from callback-derived defaults.
use super::source_shape::{fresh_config_sets, value};
use super::*;
use crate::owned_value::{OwnedValueCodec, WhitespacePolicy};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigurationInputsPolicy {
    PobFreshNumericConfigOverridesV1 {
        mapping_source: OwnedContentDigest,
        encounter: EncounterDefId,
        inputs: Vec<ConfigurationNumericInput>,
    },
    /// Retains the V1 raw overrides and separately admits saved numeric values
    /// with Input-first, Placeholder-second precedence. Zero is a value in both
    /// lanes. This does not implement count-style zero fallback or defaults.
    PobFreshNumericConfigFallbacksV2 {
        mapping_source: OwnedContentDigest,
        encounter: EncounterDefId,
        inputs: Vec<ConfigurationNumericInput>,
        placeholder_fallback_inputs: Vec<ConfigurationNumericInput>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationNumericInput {
    pub source_name: String,
    pub presence_input: ExternalInputDefId,
    pub value_input: ExternalInputDefId,
    pub recipe: ValueRecipeInput,
}

pub(super) struct CompiledConfigurationInputs {
    encounter: EncounterDefId,
    inputs: Vec<CompiledInput>,
    work: usize,
}
struct CompiledInput {
    source_name: String,
    presence_input: ExternalInputDefId,
    value_input: ExternalInputDefId,
    range: QuantityRange,
    recipe: ValueRecipe,
    placeholder_codec: OwnedValueCodec,
    placeholder_fallback: bool,
}

fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|n| *n <= limits.max_work)
        .ok_or(NormalizationError::Limit("configuration input policy work"))?;
    Ok(())
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledConfigurationInputs>> {
    let (mapping_source, encounter, inputs, fallbacks) = match &policy.configuration_inputs {
        None => return Ok(None),
        Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
            mapping_source,
            encounter,
            inputs,
        }) => (mapping_source, encounter, inputs.as_slice(), &[][..]),
        Some(ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
            mapping_source,
            encounter,
            inputs,
            placeholder_fallback_inputs,
        }) => (
            mapping_source,
            encounter,
            inputs.as_slice(),
            placeholder_fallback_inputs.as_slice(),
        ),
    };
    if mapping_source != mappings.source_identity() || encounter.namespace() != &policy.namespace {
        return Err(NormalizationError::Binding);
    }
    let rows = inputs.len().saturating_add(fallbacks.len());
    if rows == 0
        || rows > 64
        || inputs
            .len()
            .saturating_add(fallbacks.len().saturating_mul(2))
            > limits.value.max_selectors
        || rows.saturating_mul(2) > limits.draft.input.max_collection_entries
    {
        return Err(NormalizationError::Limit("configuration input rows"));
    }
    let SchemaLookup::Known(owner) = definitions.definition(encounter) else {
        return Err(NormalizationError::Policy(
            "configuration input encounter schema",
        ));
    };
    let mut work = 0;
    charge(&mut work, owner.external_inputs.members.len(), limits)?;
    let declared: BTreeSet<_> = owner.external_inputs.members.iter().collect();
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut recipes = BTreeSet::new();
    let mut compiled = Vec::new();
    let mut selector_bytes = 0usize;
    for (input, placeholder_fallback) in inputs
        .iter()
        .map(|input| (input, false))
        .chain(fallbacks.iter().map(|input| (input, true)))
    {
        selector_bytes = selector_bytes
            .checked_add(
                input
                    .source_name
                    .len()
                    .saturating_mul(1 + usize::from(placeholder_fallback)),
            )
            .filter(|n| *n <= limits.value.max_total_selector_bytes)
            .ok_or(NormalizationError::Limit(
                "configuration input selector bytes",
            ))?;
        charge(&mut work, input.source_name.len().saturating_add(1), limits)?;
        if input.source_name.is_empty()
            || input.source_name.len() > 128
            || input.source_name.len() > limits.value.max_selector_bytes
            || input.source_name.trim_ascii() != input.source_name
            || input.source_name.chars().any(char::is_control)
            || !names.insert(&input.source_name)
            || !ids.insert(&input.presence_input)
            || !ids.insert(&input.value_input)
            || !recipes.insert(&input.recipe.id)
        {
            return Err(NormalizationError::Policy("configuration input identities"));
        }
        if input.presence_input.namespace() != &policy.namespace
            || input.value_input.namespace() != &policy.namespace
            || input.recipe.codec.namespace != policy.namespace
        {
            return Err(NormalizationError::Binding);
        }
        if !declared.contains(&input.presence_input) || !declared.contains(&input.value_input) {
            return Err(NormalizationError::Policy("configuration input membership"));
        }
        let (SchemaLookup::Known(presence), SchemaLookup::Known(raw)) = (
            definitions.definition(&input.presence_input),
            definitions.definition(&input.value_input),
        ) else {
            return Err(NormalizationError::Policy("configuration input schema"));
        };
        charge(
            &mut work,
            presence.targets.len().saturating_add(raw.targets.len()),
            limits,
        )?;
        if presence.value != ValueSchema::Boolean
            || !presence.targets.contains(&AssumptionTargetKind::Enemy)
            || !raw.targets.contains(&AssumptionTargetKind::Enemy)
        {
            return Err(NormalizationError::Policy(
                "configuration input target or type",
            ));
        }
        let (ValueSchema::Quantity(range), ValueCodecKind::Quantity { unit, scale, .. }) =
            (&raw.value, &input.recipe.codec.codec)
        else {
            return Err(NormalizationError::Policy(
                "configuration input quantity codec",
            ));
        };
        if unit != range.minimum.unit()
            || unit != range.maximum.unit()
            || !matches!(definitions.definition(unit), SchemaLookup::Known(_))
            || scale.numerator.get() != 1
            || scale.denominator.get() != 1
            || input.recipe.codec.whitespace != WhitespacePolicy::Exact
        {
            return Err(NormalizationError::Policy("configuration input raw unit"));
        }
        let recipe = &input.recipe;
        let lanes = if placeholder_fallback {
            &[ValueLane::InputNumber, ValueLane::PlaceholderNumber][..]
        } else {
            &[ValueLane::InputNumber][..]
        };
        if recipe.tiers.len() != lanes.len()
            || recipe.tiers.iter().zip(lanes).any(|(tier, lane)| {
                tier.selectors.len() != 1
                    || tier.duplicates != DuplicatePolicy::Reject
                    || tier.selectors[0].lane != *lane
                    || tier.selectors[0].name != input.source_name
            })
            || !matches!(recipe.missing, MissingValuePolicy::Pending)
            || !recipe.numeric_aliases.is_empty()
        {
            return Err(NormalizationError::Policy("configuration input recipe"));
        }
        charge(
            &mut work,
            input.source_name.len().saturating_mul(lanes.len()),
            limits,
        )?;
        let placeholder_codec = OwnedValueCodec::new(recipe.codec.clone(), limits.value.value)
            .map_err(ValuePolicyError::from)?;
        compiled.push(CompiledInput {
            source_name: input.source_name.clone(),
            presence_input: input.presence_input.clone(),
            value_input: input.value_input.clone(),
            range: range.clone(),
            recipe: ValueRecipe::new(recipe.clone(), limits.value)?,
            placeholder_codec,
            placeholder_fallback,
        });
    }
    Ok(Some(CompiledConfigurationInputs {
        encounter: encounter.clone(),
        inputs: compiled,
        work,
    }))
}

struct ProvenInput {
    presence_input: ExternalInputDefId,
    value_input: ExternalInputDefId,
    raw: Option<(SourceOccurrenceId, ParameterValue)>,
}
pub(super) struct ProvenConfigurationInputs {
    scope: SourceOccurrenceId,
    encounter: EncounterDefId,
    inputs: Vec<ProvenInput>,
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    policy: Option<&CompiledConfigurationInputs>,
) -> Result<BTreeMap<SourceOccurrenceId, ProvenConfigurationInputs>> {
    let mut result = BTreeMap::new();
    let Some(policy) = policy else {
        return Ok(result);
    };
    b.charge(policy.work)?;
    let Some(sets) = fresh_config_sets(b)? else {
        return Ok(result);
    };
    let evidence = b.evidence;
    for scope in sets {
        let row = &evidence.rows()[scope.ordinal() as usize];
        b.charge(row.children().len().saturating_mul(policy.inputs.len()))?;
        let mut inputs = Vec::new();
        for input in &policy.inputs {
            let mut authored = None;
            let mut saved_placeholder = None;
            let mut blocked = false;
            for id in row.children() {
                let child = &evidence.rows()[id.ordinal() as usize];
                if value(child, "name") != Some(input.source_name.as_str()) {
                    continue;
                }
                let Some(text) = value(child, "number") else {
                    blocked = true;
                    break;
                };
                if text.len() > b.limits.value.value.max_source_bytes {
                    return Err(NormalizationError::Limit(
                        "configuration input source bytes",
                    ));
                }
                if child.occurrence().name() == "Placeholder" && !input.placeholder_fallback {
                    // It cannot establish a raw override. Validate its numeric
                    // source spelling, without promoting a callback default.
                    if input.placeholder_codec.decode(text).is_err() {
                        blocked = true;
                        break;
                    }
                } else if child.occurrence().name() == "Input"
                    || (input.placeholder_fallback && child.occurrence().name() == "Placeholder")
                {
                    let is_placeholder = child.occurrence().name() == "Placeholder";
                    if input.placeholder_fallback {
                        // This lane additionally decodes and checks every saved
                        // fallback, even when an Input later takes precedence.
                        b.charge(text.len())?;
                    }
                    let (index, attribute) = b.attributes[id.ordinal() as usize]["number"];
                    let candidate = ValueCandidate {
                        selector: &input.recipe.input().tiers[usize::from(is_placeholder)]
                            .selectors[0],
                        origin: SourceAttributeRef {
                            occurrence: *id,
                            index,
                        },
                        value: match attribute.decoded() {
                            Ok(text) => CandidateValue::Decoded(text),
                            Err(error) => CandidateValue::Unavailable(error),
                        },
                    };
                    match input.recipe.decide(&[candidate])?.outcome {
                        ValueOutcome::Selected {
                            value: ParameterValue::Quantity(value),
                            ..
                        } if value.unit() == input.range.minimum.unit()
                            && value.value() >= input.range.minimum.value()
                            && value.value() <= input.range.maximum.value() =>
                        {
                            let selected = Some((*id, ParameterValue::Quantity(value)));
                            if is_placeholder {
                                saved_placeholder = selected;
                            } else {
                                authored = selected;
                            }
                        }
                        _ => {
                            blocked = true;
                            break;
                        }
                    }
                } else {
                    blocked = true;
                    break;
                }
            }
            if !blocked {
                b.charge(1)?;
                inputs.push(ProvenInput {
                    presence_input: input.presence_input.clone(),
                    value_input: input.value_input.clone(),
                    raw: authored.or(saved_placeholder),
                });
            }
        }
        if !inputs.is_empty() {
            b.charge(1)?;
            result.insert(
                scope,
                ProvenConfigurationInputs {
                    scope,
                    encounter: policy.encounter.clone(),
                    inputs,
                },
            );
        }
    }
    Ok(result)
}

/// Called after existing scoped adapters and the final fallback-link pass: new
/// authority never removes an old unresolved-role link or changes the assumptions
/// inventory obligation.
pub(super) fn materialize(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: &mut ScenarioPresetDraft,
    proof: Option<&ProvenConfigurationInputs>,
) -> Result<()> {
    let Some(proof) = proof else { return Ok(()) };
    if proof.scope != scope {
        return Err(NormalizationError::Policy(
            "configuration input source scope",
        ));
    }
    if scenario.scenario.enemy.encounter.to_resolved().as_ref() != Some(&proof.encounter) {
        return Ok(());
    }
    let output = &mut scenario.scenario.assumptions.members;
    let additional = proof
        .inputs
        .iter()
        .map(|input| 1 + usize::from(input.raw.is_some()))
        .sum::<usize>();
    b.charge(additional.saturating_add(output.len().saturating_mul(proof.inputs.len())))?;
    if output.len().saturating_add(additional) > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("configuration input output"));
    }
    for input in &proof.inputs {
        if output.iter().any(|a| {
            a.input
                .to_resolved()
                .is_some_and(|id| id == input.presence_input || id == input.value_input)
        }) {
            return Err(NormalizationError::Policy(
                "configuration input duplicate output",
            ));
        }
        output.push(ExternalAssumptionDraft {
            input: input.presence_input.clone().into(),
            target: DraftAssumptionTarget::Enemy,
            value: ParameterValue::Boolean(input.raw.is_some()).into(),
        });
        if let Some((origin, value)) = &input.raw {
            output.push(ExternalAssumptionDraft {
                input: input.value_input.clone().into(),
                target: DraftAssumptionTarget::Enemy,
                value: value.clone().into(),
            });
            b.link(*origin, OwnedOriginTarget::ScenarioPreset(scenario.id))?;
        }
    }
    Ok(())
}
