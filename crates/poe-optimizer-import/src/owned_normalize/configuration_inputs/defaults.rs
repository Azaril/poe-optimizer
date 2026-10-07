//! Typed constructor defaults, admitted only by a proven fresh Config frame.
use super::*;
use std::borrow::Cow;

/// Stable scenario targets require no project-local occurrence identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationInputTarget {
    Enemy,
    Environment,
    Player,
}
impl ConfigurationInputTarget {
    fn role(self) -> AssumptionTargetKind {
        match self {
            Self::Enemy => AssumptionTargetKind::Enemy,
            Self::Environment => AssumptionTargetKind::Environment,
            Self::Player => AssumptionTargetKind::Actor,
        }
    }
    pub(super) fn draft(self) -> DraftAssumptionTarget {
        match self {
            Self::Enemy => DraftAssumptionTarget::Enemy,
            Self::Environment => DraftAssumptionTarget::Environment,
            Self::Player => DraftAssumptionTarget::Actor(DraftActorKey::Player),
        }
    }
}

/// A typed authored Input, or its injected constructor value on proven absence.
/// A matching malformed record never authorizes the default. Numeric
/// Placeholders may be ignored only with explicit source-reader evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDefaultInput {
    pub source_name: String,
    pub value_input: ExternalInputDefId,
    pub target: ConfigurationInputTarget,
    pub recipe: ValueRecipeInput,
    pub constructor_default: ParameterValue,
    /// Admit a valid numeric Placeholder as unused source data. It supplies no
    /// value or provenance and does not suppress the constructor default when
    /// the authored Input is absent. Only numeric recipes permit this opt-in.
    #[serde(default, skip_serializing_if = "is_false")]
    pub ignore_numeric_placeholder: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
}
/// Borrow policy bytes until their size, identity and schema checks have passed.
pub(super) struct DefaultInputRef<'a> {
    pub(super) source_name: &'a str,
    pub(super) value_input: &'a ExternalInputDefId,
    pub(super) target: ConfigurationInputTarget,
    pub(super) recipe: &'a ValueRecipeInput,
    constructor_default: Cow<'a, ParameterValue>,
    ignore_numeric_placeholder: bool,
}
impl<'a> From<&'a ConfigurationOptionInput> for DefaultInputRef<'a> {
    fn from(input: &'a ConfigurationOptionInput) -> Self {
        Self {
            source_name: &input.source_name,
            value_input: &input.value_input,
            target: ConfigurationInputTarget::Enemy,
            recipe: &input.recipe,
            constructor_default: Cow::Owned(ParameterValue::Option(
                input.constructor_default.clone(),
            )),
            ignore_numeric_placeholder: false,
        }
    }
}
impl<'a> From<&'a ConfigurationDefaultInput> for DefaultInputRef<'a> {
    fn from(input: &'a ConfigurationDefaultInput) -> Self {
        Self {
            source_name: &input.source_name,
            value_input: &input.value_input,
            target: input.target,
            recipe: &input.recipe,
            constructor_default: Cow::Borrowed(&input.constructor_default),
            ignore_numeric_placeholder: input.ignore_numeric_placeholder,
        }
    }
}

pub(super) struct CompiledDefault {
    source_name: String,
    value_input: ExternalInputDefId,
    target: ConfigurationInputTarget,
    recipe: ValueRecipe,
    schema: ValueSchema,
    constructor_default: ParameterValue,
    attribute: &'static str,
    ignored_placeholder_codec: Option<OwnedValueCodec>,
}
pub(super) struct ProvenDefault {
    pub(super) value_input: ExternalInputDefId,
    pub(super) target: ConfigurationInputTarget,
    pub(super) origin: SourceOccurrenceId,
    pub(super) value: ParameterValue,
}

#[derive(Default)]
pub(super) struct TokenBudget {
    count: usize,
    bytes: usize,
}
impl TokenBudget {
    fn count(&mut self, count: usize, limits: NormalizationLimits) -> Result<()> {
        self.count = self
            .count
            .checked_add(count)
            .filter(|n| *n <= limits.value.value.max_tokens)
            .ok_or(NormalizationError::Limit("configuration input tokens"))?;
        Ok(())
    }
    fn token(&mut self, text: &str, work: &mut usize, limits: NormalizationLimits) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(text.len())
            .filter(|n| *n <= limits.value.value.max_total_token_bytes)
            .ok_or(NormalizationError::Limit("configuration input token bytes"))?;
        charge(work, text.len().saturating_add(1), limits)
    }
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    input: &DefaultInputRef<'_>,
    definitions: &I,
    declared: &BTreeSet<&ExternalInputDefId>,
    limits: NormalizationLimits,
    work: &mut usize,
    tokens: &mut TokenBudget,
) -> Result<CompiledDefault> {
    if input.value_input.namespace() != definitions.namespace()
        || &input.recipe.codec.namespace != definitions.namespace()
    {
        return Err(NormalizationError::Binding);
    }
    if !declared.contains(input.value_input) {
        return Err(NormalizationError::Policy("configuration input membership"));
    }
    let SchemaLookup::Known(raw) = definitions.definition(input.value_input) else {
        return Err(NormalizationError::Policy("configuration input schema"));
    };
    charge(work, raw.targets.len().saturating_add(1), limits)?;
    if !raw.targets.contains(&input.target.role()) {
        return Err(NormalizationError::Policy(
            "configuration input target or type",
        ));
    }
    let (lane, attribute) = match (&raw.value, &input.recipe.codec.codec) {
        (ValueSchema::Boolean, ValueCodecKind::Boolean { tokens: values }) => {
            if values.is_empty() {
                return Err(NormalizationError::Policy(
                    "configuration input boolean tokens",
                ));
            }
            tokens.count(values.len(), limits)?;
            for value in values {
                tokens.token(&value.token, work, limits)?;
            }
            (ValueLane::InputBoolean, "boolean")
        }
        (ValueSchema::Integer(_), ValueCodecKind::Integer { .. }) => {
            (ValueLane::InputNumber, "number")
        }
        (ValueSchema::Quantity(range), ValueCodecKind::Quantity { unit, .. }) => {
            if unit != range.minimum.unit()
                || unit != range.maximum.unit()
                || !matches!(definitions.definition(unit), SchemaLookup::Known(_))
            {
                return Err(NormalizationError::Policy(
                    "configuration input quantity unit",
                ));
            }
            (ValueLane::InputNumber, "number")
        }
        (ValueSchema::Option { allowed }, ValueCodecKind::Option { tokens: values }) => {
            if values.is_empty() {
                return Err(NormalizationError::Policy(
                    "configuration input option tokens",
                ));
            }
            charge(work, allowed.members.len(), limits)?;
            let allowed: BTreeSet<_> = allowed.members.iter().collect();
            tokens.count(values.len(), limits)?;
            for value in values {
                tokens.token(&value.token, work, limits)?;
                charge(
                    work,
                    value.value.key().as_str().len().saturating_add(1),
                    limits,
                )?;
                if value.value.namespace() != definitions.namespace()
                    || !allowed.contains(&value.value)
                    || !matches!(definitions.definition(&value.value), SchemaLookup::Known(_))
                {
                    return Err(NormalizationError::Policy(
                        "configuration input option domain",
                    ));
                }
            }
            (ValueLane::InputString, "string")
        }
        _ => {
            return Err(NormalizationError::Policy(
                "configuration input typed codec",
            ));
        }
    };
    if input.ignore_numeric_placeholder && lane != ValueLane::InputNumber {
        return Err(NormalizationError::Policy(
            "configuration input numeric placeholder codec",
        ));
    }
    if let ValueSchema::Option { allowed } = &raw.value {
        charge(work, allowed.members.len(), limits)?;
    }
    if !gem_inputs::value_valid(&input.constructor_default, &raw.value) {
        return Err(NormalizationError::Policy(
            "configuration input constructor default",
        ));
    }
    if let ParameterValue::Option(value) = input.constructor_default.as_ref() {
        charge(work, value.key().as_str().len().saturating_add(1), limits)?;
        if value.namespace() != definitions.namespace()
            || !matches!(definitions.definition(value), SchemaLookup::Known(_))
        {
            return Err(NormalizationError::Policy(
                "configuration input constructor option",
            ));
        }
    }
    let recipe = input.recipe;
    if recipe.codec.whitespace != WhitespacePolicy::Exact
        || recipe.tiers.len() != 1
        || recipe.tiers[0].selectors.len() != 1
        || recipe.tiers[0].duplicates != DuplicatePolicy::Reject
        || recipe.tiers[0].selectors[0].lane != lane
        || recipe.tiers[0].selectors[0].name != input.source_name
        || !matches!(recipe.missing, MissingValuePolicy::Pending)
        || !recipe.numeric_aliases.is_empty()
    {
        return Err(NormalizationError::Policy(
            "configuration input default recipe",
        ));
    }
    charge(work, input.source_name.len(), limits)?;
    let ignored_placeholder_codec = input
        .ignore_numeric_placeholder
        .then(|| OwnedValueCodec::new(recipe.codec.clone(), limits.value.value))
        .transpose()
        .map_err(ValuePolicyError::from)?;
    Ok(CompiledDefault {
        source_name: input.source_name.to_owned(),
        value_input: input.value_input.clone(),
        target: input.target,
        recipe: ValueRecipe::new(recipe.clone(), limits.value)?,
        schema: raw.value.clone(),
        constructor_default: input.constructor_default.as_ref().clone(),
        attribute,
        ignored_placeholder_codec,
    })
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    inputs: &[CompiledDefault],
) -> Result<Vec<ProvenDefault>> {
    let evidence = b.evidence;
    let row = &evidence.rows()[scope.ordinal() as usize];
    let mut output = Vec::new();
    for input in inputs {
        let mut selected = None;
        let mut blocked = false;
        for id in row.children() {
            let child = &evidence.rows()[id.ordinal() as usize];
            if value(child, "name") != Some(input.source_name.as_str()) {
                continue;
            }
            // A present record with the wrong lane is never evidence that the
            // constructor's input was absent, even under the numeric opt-in.
            let Some(text) = value(child, input.attribute) else {
                blocked = true;
                break;
            };
            if text.len() > b.limits.value.value.max_source_bytes {
                return Err(NormalizationError::Limit(
                    "configuration input source bytes",
                ));
            }
            b.charge(text.len())?;
            if child.occurrence().name() == "Placeholder"
                && let Some(codec) = &input.ignored_placeholder_codec
            {
                // The declared source reader never consumes this value. Check
                // its finite numeric spelling through the same typed codec,
                // without imposing the destination value's allowed range.
                if codec.decode(text).is_err() {
                    blocked = true;
                    break;
                }
                continue;
            }
            if child.occurrence().name() != "Input" || selected.is_some() {
                blocked = true;
                break;
            }
            let (index, attribute) = b.attributes[id.ordinal() as usize][input.attribute];
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
            if let ValueSchema::Option { allowed } = &input.schema {
                b.charge(allowed.members.len())?;
            }
            match input.recipe.decide(&[candidate])?.outcome {
                ValueOutcome::Selected { value, .. }
                    if gem_inputs::value_valid(&value, &input.schema) =>
                {
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
            output.push(ProvenDefault {
                value_input: input.value_input.clone(),
                target: input.target,
                origin,
                value,
            });
        }
    }
    Ok(output)
}
