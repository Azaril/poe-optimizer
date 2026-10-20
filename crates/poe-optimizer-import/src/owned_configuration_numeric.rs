//! Checked numeric selection over caller-collected configuration facts.
//!
//! Selection is independent of the destination owner. The caller proves source
//! scope and target authority; this module neither discovers settings nor writes
//! Rewards, Scenario intents, or any other owned occurrence.
use crate::{owned_source::SourceAttributeRef, owned_value::ValueCodecKind, owned_value_policy::*};
use poe_optimizer_core::{owned_build::ParameterValue, owned_schema::ValueSchema};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Import reader semantics; neither variant becomes a native condition.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationNumericSelection {
    /// Ordinary checked recipe precedence, including explicitly authored zero.
    RecipePrecedence,
    /// Neither zero lane is consumed. No selected value remains unresolved;
    /// absence does not establish a zero-valued fact.
    NonzeroInputThenNonzeroPlaceholder,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigurationNumericDecision {
    pub selected: Option<SourceAttributeRef>,
    pub sources: Vec<ConfigurationNumericSource>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigurationNumericSource {
    pub origin: SourceAttributeRef,
    pub disposition: ConfigurationNumericDisposition,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationNumericDisposition {
    Selected,
    ZeroSkipped,
    Unselected,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationNumericReason {
    ValuePending,
    ValueAbsent,
    ProjectedValueOutOfRange,
    InvalidNumericCandidate,
    NoNumericSelection,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigurationNumericOutcome {
    Selected { value: ParameterValue },
    Unavailable { reason: ConfigurationNumericReason },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConfigurationNumericResult<'a> {
    /// The ordinary recipe decision can select zero while the numeric selection
    /// chooses a lower nonzero lane. `numeric.selected` is the final authority.
    pub value: ValueDecision<'a>,
    pub outcome: ConfigurationNumericOutcome,
    pub numeric: ConfigurationNumericDecision,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationNumericViolation {
    UnsupportedRecipe,
    NumericDefault,
    UnsupportedLane,
    UnsupportedSelection,
}
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConfigurationNumericError {
    #[error("invalid numeric configuration recipe: {0:?}")]
    Invalid(ConfigurationNumericViolation),
    #[error(transparent)]
    Value(#[from] ValuePolicyError),
}

#[derive(Clone, Debug)]
pub struct ConfigurationNumericRecipe {
    recipe: ValueRecipe,
    selection: ConfigurationNumericSelection,
}
impl ConfigurationNumericRecipe {
    /// Check the numeric-only contract independently of destination validation.
    /// `new` also performs all bounded ordinary recipe/codec validation.
    pub fn validate_recipe(
        recipe: &ValueRecipeInput,
        selection: ConfigurationNumericSelection,
    ) -> Result<(), ConfigurationNumericError> {
        use ConfigurationNumericError::Invalid;
        use ConfigurationNumericViolation::*;
        if !matches!(
            recipe.codec.codec,
            ValueCodecKind::Integer { .. } | ValueCodecKind::Quantity { .. }
        ) {
            return Err(Invalid(UnsupportedRecipe));
        }
        if matches!(recipe.missing, MissingValuePolicy::Explicit { .. }) {
            return Err(Invalid(NumericDefault));
        }
        if recipe.tiers.iter().flat_map(|t| &t.selectors).any(|s| {
            !matches!(
                s.lane,
                ValueLane::InputNumber | ValueLane::PlaceholderNumber
            )
        }) {
            return Err(Invalid(UnsupportedLane));
        }
        if recipe
            .tiers
            .iter()
            .any(|t| t.duplicates != DuplicatePolicy::Reject)
        {
            return Err(Invalid(UnsupportedSelection));
        }
        if selection == ConfigurationNumericSelection::NonzeroInputThenNonzeroPlaceholder {
            let valid = recipe.tiers.len() == 2
                && recipe.tiers.iter().all(|t| t.selectors.len() == 1)
                && recipe.tiers[0].selectors[0].lane == ValueLane::InputNumber
                && recipe.tiers[1].selectors[0].lane == ValueLane::PlaceholderNumber
                && recipe.tiers[0].selectors[0].name == recipe.tiers[1].selectors[0].name;
            if !valid {
                return Err(Invalid(UnsupportedSelection));
            }
        }
        Ok(())
    }

    pub fn new(
        input: ValueRecipeInput,
        selection: ConfigurationNumericSelection,
        limits: ValuePolicyLimits,
    ) -> Result<Self, ConfigurationNumericError> {
        Self::validate_recipe(&input, selection)?;
        Ok(Self {
            recipe: ValueRecipe::new(input, limits)?,
            selection,
        })
    }

    pub fn input(&self) -> &ValueRecipeInput {
        self.recipe.input()
    }

    /// All matching lanes must decode and fit the optional target domain,
    /// including losing lanes. The caller must establish this entire source
    /// scope before absence can be used; no default is synthesized.
    pub fn decide<'a>(
        &self,
        candidates: &[ValueCandidate<'a>],
        domain: Option<&ValueSchema>,
    ) -> Result<ConfigurationNumericResult<'a>, ConfigurationNumericError> {
        let value = self.recipe.decide(candidates)?;
        let mut tiers = BTreeMap::new();
        for matched in &value.matched {
            *tiers.entry(matched.tier).or_insert(0usize) += 1;
        }
        for (tier, count) in tiers {
            if count > 1 {
                return Err(ValuePolicyError::MultipleValues { tier, count }.into());
            }
        }
        let origins: BTreeSet<_> = value.matched.iter().map(|m| m.origin).collect();
        let mut decoded = BTreeMap::new();
        let mut unresolved = None;
        // The full recipe call checks budgets and source identities. Recheck
        // every matched value so a malformed loser cannot gain a valid receipt.
        for candidate in candidates.iter().filter(|c| origins.contains(&c.origin)) {
            match self.recipe.decide(std::slice::from_ref(candidate))?.outcome {
                ValueOutcome::Selected { value, .. } => {
                    if domain.is_some_and(|domain| !in_domain(&value, domain)) {
                        unresolved
                            .get_or_insert(ConfigurationNumericReason::ProjectedValueOutOfRange);
                    }
                    decoded.insert(candidate.origin, value);
                }
                _ => {
                    unresolved.get_or_insert(ConfigurationNumericReason::InvalidNumericCandidate);
                }
            }
        }
        let selected = if unresolved.is_some() {
            None
        } else {
            match self.selection {
                ConfigurationNumericSelection::RecipePrecedence => match &value.outcome {
                    ValueOutcome::Selected { origin, .. } => Some(*origin),
                    _ => None,
                },
                ConfigurationNumericSelection::NonzeroInputThenNonzeroPlaceholder => value
                    .matched
                    .iter()
                    .filter(|m| decoded.get(&m.origin).is_some_and(|v| !numeric_zero(v)))
                    .min_by_key(|m| m.tier)
                    .map(|m| m.origin),
            }
        };
        let sources = value
            .matched
            .iter()
            .map(|m| ConfigurationNumericSource {
                origin: m.origin,
                disposition: if selected == Some(m.origin) {
                    ConfigurationNumericDisposition::Selected
                } else if self.selection
                    == ConfigurationNumericSelection::NonzeroInputThenNonzeroPlaceholder
                    && decoded.get(&m.origin).is_some_and(numeric_zero)
                {
                    ConfigurationNumericDisposition::ZeroSkipped
                } else {
                    ConfigurationNumericDisposition::Unselected
                },
            })
            .collect();
        let outcome = if let Some(reason) = unresolved {
            ConfigurationNumericOutcome::Unavailable { reason }
        } else if let Some(origin) = selected {
            ConfigurationNumericOutcome::Selected {
                value: decoded[&origin].clone(),
            }
        } else {
            ConfigurationNumericOutcome::Unavailable {
                reason: match value.outcome {
                    ValueOutcome::Absent => ConfigurationNumericReason::ValueAbsent,
                    ValueOutcome::Pending { .. } => ConfigurationNumericReason::ValuePending,
                    _ => ConfigurationNumericReason::NoNumericSelection,
                },
            }
        };
        Ok(ConfigurationNumericResult {
            value,
            outcome,
            numeric: ConfigurationNumericDecision { selected, sources },
        })
    }
}

fn in_domain(value: &ParameterValue, domain: &ValueSchema) -> bool {
    match (value, domain) {
        (ParameterValue::Integer(value), ValueSchema::Integer(range)) => {
            value >= &range.minimum && value <= &range.maximum
        }
        (ParameterValue::Quantity(value), ValueSchema::Quantity(range)) => {
            value.unit() == range.minimum.unit()
                && value.value() >= range.minimum.value()
                && value.value() <= range.maximum.value()
        }
        _ => false,
    }
}
fn numeric_zero(value: &ParameterValue) -> bool {
    match value {
        ParameterValue::Integer(value) => value.get() == 0,
        ParameterValue::Quantity(value) => value.value() == 0.,
        _ => false,
    }
}
