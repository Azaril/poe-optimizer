//! Bounded precedence/default selection over caller-prepared lexical facts.
//!
//! This utility does not inspect source trees, establish context membership, write
//! Core inputs or attest that caller facts came from an import. The normalizer owns
//! those checks. Presence, duplicate selection and lexical conversion are separate.
use crate::{owned_source::SourceAttributeRef, owned_value::*, source_xml::SourceXmlError};
use poe_optimizer_core::{owned_build::ParameterValue, owned_definitions::OwnedDefinitionKey};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueLane {
    Attribute,
    ParentAttribute,
    InputNumber,
    InputBoolean,
    InputString,
    PlaceholderNumber,
    PlaceholderBoolean,
    PlaceholderString,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueSelector {
    pub lane: ValueLane,
    pub name: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicatePolicy {
    Reject,
    LastInSourceOrder,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueTier {
    pub selectors: Vec<ValueSelector>,
    pub duplicates: DuplicatePolicy,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MissingValuePolicy {
    Pending,
    Explicit { value: ParameterValue },
    Absent,
}
/// One explicitly reviewed nonnumeric token decoded through the same numeric
/// codec as ordinary source text. Replacements never invoke another alias.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericTokenAlias {
    pub token: String,
    pub replacement: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueRecipeInput {
    pub id: OwnedDefinitionKey,
    pub codec: ValueCodecInput,
    pub tiers: Vec<ValueTier>,
    pub missing: MissingValuePolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numeric_aliases: Vec<NumericTokenAlias>,
}

/// Policy bounds are tighten-only. Lexical codec limits retain their own ceilings.
#[derive(Clone, Copy, Debug)]
pub struct ValuePolicyLimits {
    pub value: OwnedValueLimits,
    pub max_tiers: usize,
    pub max_selectors: usize,
    pub max_selector_bytes: usize,
    pub max_total_selector_bytes: usize,
    pub max_candidates: usize,
    pub max_total_candidate_bytes: usize,
    pub max_trace_entries: usize,
}
impl Default for ValuePolicyLimits {
    fn default() -> Self {
        Self {
            value: OwnedValueLimits::default(),
            max_tiers: 32,
            max_selectors: 256,
            max_selector_bytes: 1024,
            max_total_selector_bytes: 64 * 1024,
            max_candidates: 16_384,
            max_total_candidate_bytes: 4 * 1024 * 1024,
            max_trace_entries: 16_384,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValuePolicyResource {
    Tiers,
    Selectors,
    SelectorBytes,
    TotalSelectorBytes,
    Candidates,
    CandidateBytes,
    TotalCandidateBytes,
    TraceEntries,
    NumericAliases,
    AliasTokenBytes,
    AliasReplacementBytes,
    TotalAliasBytes,
}
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ValuePolicyError {
    #[error("invalid {resource:?} limit: {actual}; maximum is {maximum}")]
    InvalidLimit {
        resource: ValuePolicyResource,
        actual: usize,
        maximum: usize,
    },
    #[error("{resource:?} exceeds the maximum {maximum}")]
    ResourceLimit {
        resource: ValuePolicyResource,
        maximum: usize,
    },
    #[error("tier {tier} has no selectors")]
    EmptyTier { tier: usize },
    #[error("tier {tier} contains an empty selector name")]
    EmptySelector { tier: usize },
    #[error("selector at tier {tier} repeats one in tier {first_tier}")]
    DuplicateSelector { tier: usize, first_tier: usize },
    #[error("candidate selector name is empty")]
    EmptyCandidateSelector,
    #[error("candidate context repeats a source attribute")]
    DuplicateOrigin { origin: SourceAttributeRef },
    #[error("candidate context mixes different source snapshots")]
    MixedSourceSnapshots,
    #[error("tier {tier} rejects {count} present source values")]
    MultipleValues { tier: usize, count: usize },
    #[error("explicit default does not match codec output kind")]
    DefaultKindMismatch,
    #[error("explicit default belongs to another owned namespace")]
    ForeignDefaultNamespace,
    #[error("explicit quantity default does not use the codec's unit")]
    DefaultUnitMismatch,
    #[error("numeric aliases require an Integer or Quantity codec")]
    UnsupportedNumericAliasCodec,
    #[error("numeric alias rows {first} and {second} collide under the codec whitespace policy")]
    DuplicateNumericAlias { first: usize, second: usize },
    #[error("numeric alias {index} would override a decimal spelling")]
    NumericAliasOverridesNumber { index: usize },
    #[error("numeric alias {index} has an invalid replacement: {error}")]
    NumericAliasReplacement {
        index: usize,
        error: ValueDecodeError,
    },
    #[error(transparent)]
    Codec(#[from] ValueCodecError),
}
fn bounded(
    actual: usize,
    maximum: usize,
    resource: ValuePolicyResource,
) -> Result<(), ValuePolicyError> {
    if actual > maximum {
        Err(ValuePolicyError::ResourceLimit { resource, maximum })
    } else {
        Ok(())
    }
}
fn charge(
    remaining: &mut usize,
    amount: usize,
    maximum: usize,
    resource: ValuePolicyResource,
) -> Result<(), ValuePolicyError> {
    if amount > *remaining {
        return Err(ValuePolicyError::ResourceLimit { resource, maximum });
    }
    *remaining -= amount;
    Ok(())
}
impl ValuePolicyLimits {
    pub fn validate(self) -> Result<(), ValuePolicyError> {
        self.value.validate()?;
        let hard = Self::default();
        for (resource, actual, maximum) in [
            (ValuePolicyResource::Tiers, self.max_tiers, hard.max_tiers),
            (
                ValuePolicyResource::Selectors,
                self.max_selectors,
                hard.max_selectors,
            ),
            (
                ValuePolicyResource::SelectorBytes,
                self.max_selector_bytes,
                hard.max_selector_bytes,
            ),
            (
                ValuePolicyResource::TotalSelectorBytes,
                self.max_total_selector_bytes,
                hard.max_total_selector_bytes,
            ),
            (
                ValuePolicyResource::Candidates,
                self.max_candidates,
                hard.max_candidates,
            ),
            (
                ValuePolicyResource::TotalCandidateBytes,
                self.max_total_candidate_bytes,
                hard.max_total_candidate_bytes,
            ),
            (
                ValuePolicyResource::TraceEntries,
                self.max_trace_entries,
                hard.max_trace_entries,
            ),
        ] {
            if actual == 0 || actual > maximum {
                return Err(ValuePolicyError::InvalidLimit {
                    resource,
                    actual,
                    maximum,
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CandidateValue<'a> {
    Decoded(&'a str),
    Unavailable(&'a SourceXmlError),
}
/// Context is supplied by the caller; origin is evidence, not a membership proof.
#[derive(Clone, Copy, Debug)]
pub struct ValueCandidate<'a> {
    pub selector: &'a ValueSelector,
    pub origin: SourceAttributeRef,
    pub value: CandidateValue<'a>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "diagnostic", rename_all = "snake_case")]
pub enum ValuePendingReason<'a> {
    Missing,
    Unavailable(&'a SourceXmlError),
    Decode(#[serde(serialize_with = "serialize_decode_error")] ValueDecodeError),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValueOutcome<'a> {
    Selected {
        origin: SourceAttributeRef,
        value: ParameterValue,
    },
    Defaulted {
        value: ParameterValue,
    },
    Absent,
    Pending {
        reason: ValuePendingReason<'a>,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueMatchDisposition {
    Selected,
    ShadowedSameTier,
    ShadowedLowerTier,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ValueMatch {
    pub origin: SourceAttributeRef,
    pub tier: usize,
    pub disposition: ValueMatchDisposition,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ValueDecision<'a> {
    pub recipe: OwnedDefinitionKey,
    pub chosen_tier: Option<usize>,
    pub outcome: ValueOutcome<'a>,
    /// Every matching fact, in source order; no source value is copied into this trace.
    pub matched: Vec<ValueMatch>,
}

#[derive(Clone, Debug)]
pub struct ValueRecipe {
    input: ValueRecipeInput,
    codec: OwnedValueCodec,
    selectors: BTreeMap<ValueSelector, usize>,
    numeric_aliases: BTreeMap<String, (usize, ParameterValue)>,
    limits: ValuePolicyLimits,
}
impl ValueRecipe {
    pub fn new(
        input: ValueRecipeInput,
        limits: ValuePolicyLimits,
    ) -> Result<Self, ValuePolicyError> {
        limits.validate()?;
        bounded(
            input.tiers.len(),
            limits.max_tiers,
            ValuePolicyResource::Tiers,
        )?;
        let mut rows_left = limits.max_selectors;
        let mut bytes_left = limits.max_total_selector_bytes;
        for (tier, value) in input.tiers.iter().enumerate() {
            if value.selectors.is_empty() {
                return Err(ValuePolicyError::EmptyTier { tier });
            }
            charge(
                &mut rows_left,
                value.selectors.len(),
                limits.max_selectors,
                ValuePolicyResource::Selectors,
            )?;
            for selector in &value.selectors {
                if selector.name.is_empty() {
                    return Err(ValuePolicyError::EmptySelector { tier });
                }
                bounded(
                    selector.name.len(),
                    limits.max_selector_bytes,
                    ValuePolicyResource::SelectorBytes,
                )?;
                charge(
                    &mut bytes_left,
                    selector.name.len(),
                    limits.max_total_selector_bytes,
                    ValuePolicyResource::TotalSelectorBytes,
                )?;
            }
        }
        // Validate the owned codec before cloning its bounded token tables for input().
        let ValueRecipeInput {
            id,
            codec,
            tiers,
            missing,
            numeric_aliases,
        } = input;
        let codec = OwnedValueCodec::new(codec, limits.value)?;
        validate_default(&missing, codec.input())?;
        let aliases = compile_numeric_aliases(&numeric_aliases, &codec, limits.value)?;
        let mut selectors = BTreeMap::new();
        for (tier, value) in tiers.iter().enumerate() {
            for selector in &value.selectors {
                if let Some(first_tier) = selectors.insert(selector.clone(), tier) {
                    return Err(ValuePolicyError::DuplicateSelector { tier, first_tier });
                }
            }
        }
        let input = ValueRecipeInput {
            id,
            codec: codec.input().clone(),
            tiers,
            missing,
            numeric_aliases,
        };
        Ok(Self {
            input,
            codec,
            selectors,
            numeric_aliases: aliases,
            limits,
        })
    }
    pub fn input(&self) -> &ValueRecipeInput {
        &self.input
    }
    /// Typed replacement values, for target-specific schema validation. These
    /// are already decoded/scaled and never expose source text to native inputs.
    pub(crate) fn alias_values(&self) -> impl Iterator<Item = &ParameterValue> {
        self.numeric_aliases.values().map(|(_, value)| value)
    }
    fn decode_selected(&self, text: &str) -> Result<ParameterValue, ValueDecodeError> {
        // decide() bounds every original candidate before trimming or lookup.
        let token = self.codec.input().whitespace.apply(text);
        if let Some((_, value)) = self.numeric_aliases.get(token) {
            return Ok(value.clone());
        }
        self.codec.decode(text)
    }
    pub fn decide<'a>(
        &self,
        candidates: &[ValueCandidate<'a>],
    ) -> Result<ValueDecision<'a>, ValuePolicyError> {
        bounded(
            candidates.len(),
            self.limits.max_candidates,
            ValuePolicyResource::Candidates,
        )?;
        let mut bytes_left = self.limits.max_total_candidate_bytes;
        let mut origins = BTreeSet::new();
        let mut source = None;
        let mut matches = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.selector.name.is_empty() {
                return Err(ValuePolicyError::EmptyCandidateSelector);
            }
            bounded(
                candidate.selector.name.len(),
                self.limits.max_selector_bytes,
                ValuePolicyResource::SelectorBytes,
            )?;
            charge(
                &mut bytes_left,
                candidate.selector.name.len(),
                self.limits.max_total_candidate_bytes,
                ValuePolicyResource::TotalCandidateBytes,
            )?;
            let text_bytes = match candidate.value {
                CandidateValue::Decoded(text) => text.len(),
                CandidateValue::Unavailable(error) => error.reason.len(),
            };
            bounded(
                text_bytes,
                self.limits.value.max_source_bytes,
                ValuePolicyResource::CandidateBytes,
            )?;
            charge(
                &mut bytes_left,
                text_bytes,
                self.limits.max_total_candidate_bytes,
                ValuePolicyResource::TotalCandidateBytes,
            )?;
            let snapshot = candidate.origin.occurrence.source_sha256();
            if source.as_ref().is_some_and(|source| source != &snapshot) {
                return Err(ValuePolicyError::MixedSourceSnapshots);
            }
            source = Some(snapshot);
            if !origins.insert((candidate.origin.occurrence, candidate.origin.index)) {
                return Err(ValuePolicyError::DuplicateOrigin {
                    origin: candidate.origin,
                });
            }
            if let Some(tier) = self.selectors.get(candidate.selector) {
                bounded(
                    matches.len() + 1,
                    self.limits.max_trace_entries,
                    ValuePolicyResource::TraceEntries,
                )?;
                matches.push((index, *tier));
            }
        }
        matches.sort_by_key(|(index, _)| {
            let origin = candidates[*index].origin;
            (origin.occurrence.ordinal(), origin.index)
        });
        let chosen_tier = matches.iter().map(|(_, tier)| *tier).min();
        let (selected, outcome) = if let Some(tier) = chosen_tier {
            let mut selected = None;
            let mut count = 0;
            for (index, matched_tier) in &matches {
                if *matched_tier == tier {
                    selected = Some(*index);
                    count += 1;
                }
            }
            if count > 1 && self.input.tiers[tier].duplicates == DuplicatePolicy::Reject {
                return Err(ValuePolicyError::MultipleValues { tier, count });
            }
            let index = selected.expect("present tier has a match");
            let candidate = &candidates[index];
            let outcome = match candidate.value {
                CandidateValue::Unavailable(error) => ValueOutcome::Pending {
                    reason: ValuePendingReason::Unavailable(error),
                },
                CandidateValue::Decoded(text) => match self.decode_selected(text) {
                    Ok(value) => ValueOutcome::Selected {
                        origin: candidate.origin,
                        value,
                    },
                    Err(error) => ValueOutcome::Pending {
                        reason: ValuePendingReason::Decode(error),
                    },
                },
            };
            (Some(index), outcome)
        } else {
            (
                None,
                match &self.input.missing {
                    MissingValuePolicy::Pending => ValueOutcome::Pending {
                        reason: ValuePendingReason::Missing,
                    },
                    MissingValuePolicy::Explicit { value } => ValueOutcome::Defaulted {
                        value: value.clone(),
                    },
                    MissingValuePolicy::Absent => ValueOutcome::Absent,
                },
            )
        };
        let matched = matches
            .into_iter()
            .map(|(index, tier)| ValueMatch {
                origin: candidates[index].origin,
                tier,
                disposition: if selected == Some(index) {
                    ValueMatchDisposition::Selected
                } else if chosen_tier == Some(tier) {
                    ValueMatchDisposition::ShadowedSameTier
                } else {
                    ValueMatchDisposition::ShadowedLowerTier
                },
            })
            .collect();
        Ok(ValueDecision {
            recipe: self.input.id.clone(),
            chosen_tier,
            outcome,
            matched,
        })
    }
}
fn compile_numeric_aliases(
    aliases: &[NumericTokenAlias],
    codec: &OwnedValueCodec,
    limits: OwnedValueLimits,
) -> Result<BTreeMap<String, (usize, ParameterValue)>, ValuePolicyError> {
    bounded(
        aliases.len(),
        limits.max_tokens,
        ValuePolicyResource::NumericAliases,
    )?;
    if !aliases.is_empty()
        && !matches!(
            codec.input().codec,
            ValueCodecKind::Integer { .. } | ValueCodecKind::Quantity { .. }
        )
    {
        return Err(ValuePolicyError::UnsupportedNumericAliasCodec);
    }
    // Charge every original string before allocating any lookup key or running
    // lexical conversion. Tokens and replacements share one aggregate budget.
    let mut remaining = limits.max_total_token_bytes;
    for alias in aliases {
        bounded(
            alias.token.len(),
            limits.max_token_bytes,
            ValuePolicyResource::AliasTokenBytes,
        )?;
        bounded(
            alias.replacement.len(),
            limits.max_token_bytes.min(limits.max_source_bytes),
            ValuePolicyResource::AliasReplacementBytes,
        )?;
        for bytes in [alias.token.len(), alias.replacement.len()] {
            charge(
                &mut remaining,
                bytes,
                limits.max_total_token_bytes,
                ValuePolicyResource::TotalAliasBytes,
            )?;
        }
    }
    let mut output = BTreeMap::new();
    for (index, alias) in aliases.iter().enumerate() {
        let token = codec.input().whitespace.apply(&alias.token);
        // Use the broadest grammar, independent of this codec's admitted syntax.
        // An alias cannot mask a non-integral number or numeric overflow either.
        if numeric_prefix_length(token, DecimalSyntax::Scientific) == Some(token.len()) {
            return Err(ValuePolicyError::NumericAliasOverridesNumber { index });
        }
        if let Some((first, _)) = output.get(token) {
            return Err(ValuePolicyError::DuplicateNumericAlias {
                first: *first,
                second: index,
            });
        }
        let value = codec
            .decode(&alias.replacement)
            .map_err(|error| ValuePolicyError::NumericAliasReplacement { index, error })?;
        output.insert(token.to_owned(), (index, value));
    }
    Ok(output)
}
fn validate_default(
    policy: &MissingValuePolicy,
    codec: &ValueCodecInput,
) -> Result<(), ValuePolicyError> {
    let MissingValuePolicy::Explicit { value } = policy else {
        return Ok(());
    };
    match (&codec.codec, value) {
        (ValueCodecKind::Boolean { .. }, ParameterValue::Boolean(_))
        | (ValueCodecKind::Integer { .. }, ParameterValue::Integer(_)) => Ok(()),
        (ValueCodecKind::Quantity { unit, .. }, ParameterValue::Quantity(value)) => {
            if value.unit().namespace() != &codec.namespace {
                return Err(ValuePolicyError::ForeignDefaultNamespace);
            }
            if value.unit() != unit {
                return Err(ValuePolicyError::DefaultUnitMismatch);
            }
            Ok(())
        }
        (ValueCodecKind::Option { .. }, ParameterValue::Option(value)) => {
            if value.namespace() != &codec.namespace {
                return Err(ValuePolicyError::ForeignDefaultNamespace);
            }
            Ok(())
        }
        _ => Err(ValuePolicyError::DefaultKindMismatch),
    }
}

fn serialize_decode_error<S: Serializer>(
    error: &ValueDecodeError,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Diagnostic {
        code: &'static str,
        actual: Option<usize>,
        maximum: Option<usize>,
    }
    let (code, actual, maximum) = match error {
        ValueDecodeError::SourceTooLarge { actual, maximum } => {
            ("source_too_large", Some(*actual), Some(*maximum))
        }
        ValueDecodeError::UnknownToken => ("unknown_token", None, None),
        ValueDecodeError::MalformedDecimal => ("malformed_decimal", None, None),
        ValueDecodeError::NonIntegralInteger => ("non_integral_integer", None, None),
        ValueDecodeError::IntegerOutOfRange => ("integer_out_of_range", None, None),
        ValueDecodeError::NonFiniteInput => ("non_finite_input", None, None),
        ValueDecodeError::NonFiniteResult => ("non_finite_result", None, None),
    };
    Diagnostic {
        code,
        actual,
        maximum,
    }
    .serialize(serializer)
}
