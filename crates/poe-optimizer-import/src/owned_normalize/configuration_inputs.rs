//! Injected Config inputs, distinct from callback-derived defaults.
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
    /// Extends the numeric lanes without changing their precedence. String
    /// options use exact authored Inputs, or an injected constructor default
    /// only when the control is absent from a proven fresh configuration frame.
    PobFreshConfigInputsV3 {
        mapping_source: OwnedContentDigest,
        encounter: EncounterDefId,
        inputs: Vec<ConfigurationNumericInput>,
        placeholder_fallback_inputs: Vec<ConfigurationNumericInput>,
        option_inputs: Vec<ConfigurationOptionInput>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationOptionInput {
    pub source_name: String,
    pub value_input: ExternalInputDefId,
    pub recipe: ValueRecipeInput,
    pub constructor_default: OptionDefId,
}

pub(super) struct CompiledConfigurationInputs {
    encounter: EncounterDefId,
    inputs: Vec<CompiledInput>,
    options: Vec<CompiledOption>,
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
struct CompiledOption {
    source_name: String,
    value_input: ExternalInputDefId,
    recipe: ValueRecipe,
    constructor_default: OptionDefId,
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
    let (mapping_source, encounter, inputs, fallbacks, options) = match &policy.configuration_inputs
    {
        None => return Ok(None),
        Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
            mapping_source,
            encounter,
            inputs,
        }) => (
            mapping_source,
            encounter,
            inputs.as_slice(),
            &[][..],
            &[][..],
        ),
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
            &[][..],
        ),
        Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
            mapping_source,
            encounter,
            inputs,
            placeholder_fallback_inputs,
            option_inputs,
        }) => (
            mapping_source,
            encounter,
            inputs.as_slice(),
            placeholder_fallback_inputs.as_slice(),
            option_inputs.as_slice(),
        ),
    };
    if mapping_source != mappings.source_identity() || encounter.namespace() != &policy.namespace {
        return Err(NormalizationError::Binding);
    }
    let numeric_rows = inputs.len().saturating_add(fallbacks.len());
    let rows = numeric_rows.saturating_add(options.len());
    if rows == 0
        || rows > 64
        || inputs
            .len()
            .saturating_add(fallbacks.len().saturating_mul(2))
            .saturating_add(options.len())
            > limits.value.max_selectors
        || numeric_rows.saturating_mul(2).saturating_add(options.len())
            > limits.draft.input.max_collection_entries
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
    let mut compiled_options = Vec::new();
    let mut token_count = 0usize;
    let mut token_bytes = 0usize;
    for input in options {
        selector_bytes = selector_bytes
            .checked_add(input.source_name.len())
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
            || !ids.insert(&input.value_input)
            || !recipes.insert(&input.recipe.id)
        {
            return Err(NormalizationError::Policy("configuration input identities"));
        }
        if input.value_input.namespace() != &policy.namespace
            || input.constructor_default.namespace() != &policy.namespace
            || input.recipe.codec.namespace != policy.namespace
        {
            return Err(NormalizationError::Binding);
        }
        if !declared.contains(&input.value_input) {
            return Err(NormalizationError::Policy("configuration input membership"));
        }
        let SchemaLookup::Known(raw) = definitions.definition(&input.value_input) else {
            return Err(NormalizationError::Policy("configuration input schema"));
        };
        let (ValueSchema::Option { allowed }, ValueCodecKind::Option { tokens }) =
            (&raw.value, &input.recipe.codec.codec)
        else {
            return Err(NormalizationError::Policy(
                "configuration input option codec",
            ));
        };
        charge(
            &mut work,
            raw.targets.len().saturating_add(allowed.members.len()),
            limits,
        )?;
        let allowed: BTreeSet<_> = allowed.members.iter().collect();
        if !raw.targets.contains(&AssumptionTargetKind::Enemy)
            || !allowed.contains(&input.constructor_default)
            || !matches!(
                definitions.definition(&input.constructor_default),
                SchemaLookup::Known(_)
            )
            || tokens.is_empty()
            || input.recipe.codec.whitespace != WhitespacePolicy::Exact
        {
            return Err(NormalizationError::Policy(
                "configuration input option domain",
            ));
        }
        token_count = token_count
            .checked_add(tokens.len())
            .filter(|n| *n <= limits.value.value.max_tokens)
            .ok_or(NormalizationError::Limit(
                "configuration input option tokens",
            ))?;
        for token in tokens {
            token_bytes = token_bytes
                .checked_add(token.token.len())
                .filter(|n| *n <= limits.value.value.max_total_token_bytes)
                .ok_or(NormalizationError::Limit(
                    "configuration input option token bytes",
                ))?;
            charge(
                &mut work,
                token
                    .token
                    .len()
                    .saturating_add(token.value.key().as_str().len())
                    .saturating_add(1),
                limits,
            )?;
            if token.value.namespace() != &policy.namespace
                || !allowed.contains(&token.value)
                || !matches!(definitions.definition(&token.value), SchemaLookup::Known(_))
            {
                return Err(NormalizationError::Policy(
                    "configuration input option domain",
                ));
            }
        }
        let recipe = &input.recipe;
        if recipe.tiers.len() != 1
            || recipe.tiers[0].selectors.len() != 1
            || recipe.tiers[0].duplicates != DuplicatePolicy::Reject
            || recipe.tiers[0].selectors[0].lane != ValueLane::InputString
            || recipe.tiers[0].selectors[0].name != input.source_name
            || !matches!(recipe.missing, MissingValuePolicy::Pending)
            || !recipe.numeric_aliases.is_empty()
        {
            return Err(NormalizationError::Policy(
                "configuration input option recipe",
            ));
        }
        charge(&mut work, input.source_name.len(), limits)?;
        compiled_options.push(CompiledOption {
            source_name: input.source_name.clone(),
            value_input: input.value_input.clone(),
            recipe: ValueRecipe::new(recipe.clone(), limits.value)?,
            constructor_default: input.constructor_default.clone(),
        });
    }
    Ok(Some(CompiledConfigurationInputs {
        encounter: encounter.clone(),
        inputs: compiled,
        options: compiled_options,
        work,
    }))
}

struct ProvenInput {
    presence_input: ExternalInputDefId,
    value_input: ExternalInputDefId,
    raw: Option<(SourceOccurrenceId, ParameterValue)>,
}
struct ProvenOption {
    value_input: ExternalInputDefId,
    origin: SourceOccurrenceId,
    value: OptionDefId,
}
pub(super) struct ProvenConfigurationInputs {
    scope: SourceOccurrenceId,
    encounter: EncounterDefId,
    inputs: Vec<ProvenInput>,
    options: Vec<ProvenOption>,
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
        b.charge(
            row.children()
                .len()
                .saturating_mul(policy.inputs.len().saturating_add(policy.options.len())),
        )?;
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
        let mut options = Vec::new();
        for input in &policy.options {
            let mut selected = None;
            let mut blocked = false;
            for id in row.children() {
                let child = &evidence.rows()[id.ordinal() as usize];
                if value(child, "name") != Some(input.source_name.as_str()) {
                    continue;
                }
                // A matching saved Placeholder is neither an authored Input
                // nor proof of constructor absence. Do not borrow V2's numeric
                // fallback semantics for string controls.
                let Some(text) = value(child, "string") else {
                    blocked = true;
                    break;
                };
                if child.occurrence().name() != "Input" || selected.is_some() {
                    blocked = true;
                    break;
                }
                if text.len() > b.limits.value.value.max_source_bytes {
                    return Err(NormalizationError::Limit(
                        "configuration input source bytes",
                    ));
                }
                b.charge(text.len())?;
                let (index, attribute) = b.attributes[id.ordinal() as usize]["string"];
                let candidate = ValueCandidate {
                    selector: &input.recipe.input().tiers[0].selectors[0],
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
                        value: ParameterValue::Option(value),
                        ..
                    } => {
                        selected = Some((*id, value));
                    }
                    _ => {
                        blocked = true;
                        break;
                    }
                }
            }
            if !blocked {
                b.charge(1)?;
                let (origin, value) =
                    selected.unwrap_or_else(|| (scope, input.constructor_default.clone()));
                options.push(ProvenOption {
                    value_input: input.value_input.clone(),
                    origin,
                    value,
                });
            }
        }
        if !inputs.is_empty() || !options.is_empty() {
            b.charge(1)?;
            result.insert(
                scope,
                ProvenConfigurationInputs {
                    scope,
                    encounter: policy.encounter.clone(),
                    inputs,
                    options,
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
        .sum::<usize>()
        .saturating_add(proof.options.len());
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
    for input in &proof.options {
        b.charge(output.len())?;
        if output
            .iter()
            .any(|a| a.input.to_resolved().as_ref() == Some(&input.value_input))
        {
            return Err(NormalizationError::Policy(
                "configuration input duplicate output",
            ));
        }
        output.push(ExternalAssumptionDraft {
            input: input.value_input.clone().into(),
            target: DraftAssumptionTarget::Enemy,
            value: ParameterValue::Option(input.value.clone()).into(),
        });
        let target = OwnedOriginTarget::ScenarioPreset(scenario.id);
        // A constructor default is attributed to its already-linked ConfigSet.
        // Reuse that exact link so source selection still has one unambiguous
        // scenario target; authored Input values receive their own source link.
        b.charge(b.origins[input.origin.ordinal() as usize].links.len())?;
        let present = b.origins[input.origin.ordinal() as usize]
            .links
            .contains(&target);
        if !present {
            b.link(input.origin, target)?;
        }
    }
    Ok(())
}
