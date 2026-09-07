//! Explicit `GetMultiplier` inputs and a numeric, current-store `EvalMod` slice.
//!
//! Multiplier-producing modifiers may use the existing condition tags, but not
//! multiplier/scaling tags themselves. This excludes recursive dependencies until
//! they have a separate validated representation. Actor-targeted multipliers,
//! table-valued modifiers and unknown fields reject rather than being omitted.

use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    conditions::{ConditionEnvironment, ModifierTag},
    modifiers::{ModifierDatabase, ModifierError, QueryContext, SumKind},
};

pub type MultiplierValues = BTreeMap<String, f64>;

/// Complete local/parent explicit values and numeric modifier layers for one store.
/// The supplied DB must retain all unsupported entries during its own validation.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiplierEnvironment {
    modifiers: ModifierDatabase,
    values: Vec<MultiplierValues>,
    conditions: ConditionEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiplierError {
    Modifier(ModifierError),
    LayerCount {
        modifiers: usize,
        values: usize,
        conditions: usize,
    },
    UnsupportedContext(String),
    UnsupportedTag {
        index: usize,
        feature: String,
    },
}

impl fmt::Display for MultiplierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported native multiplier input: {self:?}")
    }
}

impl Error for MultiplierError {}

impl From<ModifierError> for MultiplierError {
    fn from(error: ModifierError) -> Self {
        Self::Modifier(error)
    }
}

impl MultiplierEnvironment {
    pub fn try_new(
        modifiers: ModifierDatabase,
        values: Vec<MultiplierValues>,
        conditions: ConditionEnvironment,
        unsupported_features: Vec<String>,
    ) -> Result<Self, MultiplierError> {
        if let Some(feature) = unsupported_features.into_iter().next() {
            return Err(MultiplierError::UnsupportedContext(feature));
        }
        let modifier_layers = modifiers.layer_count();
        let condition_layers = conditions.input().store_conditions.len();
        if modifier_layers == 0
            || values.len() != modifier_layers
            || condition_layers != modifier_layers
        {
            return Err(MultiplierError::LayerCount {
                modifiers: modifier_layers,
                values: values.len(),
                conditions: condition_layers,
            });
        }
        Ok(Self {
            modifiers,
            values,
            conditions,
        })
    }

    /// Numeric zero is a present OVERRIDE. Parent explicit values are combined
    /// recursively, then BASE modifiers are summed against the queried store.
    /// Parent OVERRIDE/BASE queries are not repeated during explicit-value lookup.
    pub fn get_multiplier(
        &self,
        variable: &str,
        query: &QueryContext,
    ) -> Result<f64, MultiplierError> {
        let name = format!("Multiplier:{variable}");
        if let Some(value) =
            self.modifiers
                .override_with_conditions(query, &[&name], &self.conditions)?
        {
            return Ok(value);
        }
        let mut parent = 0.0;
        for layer in self.values.iter().skip(1).rev() {
            // Preserve the final +0 from noMod=true, including signed zeros.
            parent = (layer.get(variable).copied().unwrap_or(0.0) + parent) + 0.0;
        }
        let explicit = self.values[0].get(variable).copied().unwrap_or(0.0) + parent;
        Ok(explicit
            + self.modifiers.sum_with_conditions(
                SumKind::Base,
                query,
                &[&name],
                &self.conditions,
            )?)
    }

    fn scalar(&self, source: &ScalarSource, query: &QueryContext) -> Result<f64, MultiplierError> {
        match source {
            ScalarSource::Constant(value) => Ok(*value),
            ScalarSource::Multiplier(variable) => self.get_multiplier(variable, query),
        }
    }

    fn variable_sum(
        &self,
        variables: &MultiplierVariables,
        query: &QueryContext,
    ) -> Result<f64, MultiplierError> {
        match variables {
            MultiplierVariables::One(variable) => self.get_multiplier(variable, query),
            MultiplierVariables::Sum(variables) => {
                let mut sum = 0.0;
                for variable in variables {
                    sum += self.get_multiplier(variable, query)?;
                }
                Ok(sum)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiplierVariables {
    One(String),
    /// Dense, ordered Lua array only; mixed-key varList tables are unsupported.
    Sum(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarSource {
    Constant(f64),
    Multiplier(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplierLimitMode {
    FactorMaximum,
    TotalMaximum,
    TotalMinimum,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiplierLimit {
    pub value: ScalarSource,
    pub mode: MultiplierLimitMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiplierScale {
    pub variables: MultiplierVariables,
    /// Use Constant(1) for an absent div; divVar takes precedence over div.
    pub divisor: ScalarSource,
    /// Upstream tag.base, defaulting to zero, added after multiplying the value.
    pub base: f64,
    pub invert: bool,
    pub limit: Option<MultiplierLimit>,
    /// Present actor fields are retained but rejected by this current-store slice.
    pub actor: Option<String>,
    pub limit_actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiplierThreshold {
    pub variables: MultiplierVariables,
    pub threshold: ScalarSource,
    pub upper: bool,
    pub equals: bool,
    pub actor: Option<String>,
    pub threshold_actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalingTag {
    Multiplier(MultiplierScale),
    Threshold(MultiplierThreshold),
    Limit {
        value: ScalarSource,
        negative: bool,
    },
    Condition(ModifierTag),
    /// Also use this for unrepresented fields on an otherwise known tag.
    Unsupported(String),
}

/// A complete, ordered sequence of numeric tags. Validation checks every entry,
/// including tags following a condition that would disable this modifier.
#[derive(Debug, Clone, PartialEq)]
pub struct ScalingProgram {
    tags: Vec<ScalingTag>,
}

impl ScalingProgram {
    pub fn try_new(tags: Vec<ScalingTag>) -> Result<Self, MultiplierError> {
        for (index, tag) in tags.iter().enumerate() {
            let unsupported = match tag {
                ScalingTag::Multiplier(tag) if tag.actor.is_some() || tag.limit_actor.is_some() => {
                    Some("Actor-targeted Multiplier")
                }
                ScalingTag::Threshold(tag)
                    if tag.actor.is_some() || tag.threshold_actor.is_some() =>
                {
                    Some("Actor-targeted MultiplierThreshold")
                }
                ScalingTag::Unsupported(feature)
                | ScalingTag::Condition(ModifierTag::Unsupported(feature)) => {
                    Some(feature.as_str())
                }
                _ => None,
            };
            if let Some(feature) = unsupported {
                return Err(MultiplierError::UnsupportedTag {
                    index,
                    feature: feature.to_owned(),
                });
            }
        }
        Ok(Self { tags })
    }

    pub fn tags(&self) -> &[ScalingTag] {
        &self.tags
    }

    /// Execute only the represented numeric EvalMod tags. None means disabled;
    /// Some(0) remains a present value. This is not a complete ModDB aggregation
    /// or an extractor: query selection/source checks occur in the caller.
    pub fn evaluate(
        &self,
        mut value: f64,
        environment: &MultiplierEnvironment,
        query: &QueryContext,
    ) -> Result<Option<f64>, MultiplierError> {
        for tag in &self.tags {
            match tag {
                ScalingTag::Multiplier(tag) => {
                    let base = environment.variable_sum(&tag.variables, query)?;
                    let divisor = environment.scalar(&tag.divisor, query)?;
                    let mut multiplier = (base / divisor + 0.0001).floor();
                    let limit = tag
                        .limit
                        .as_ref()
                        .map(|limit| environment.scalar(&limit.value, query))
                        .transpose()?;
                    if let (Some(limit), Some(spec)) = (limit, &tag.limit)
                        && spec.mode == MultiplierLimitMode::FactorMaximum
                    {
                        multiplier = lua_min(multiplier, limit);
                    }
                    if tag.invert && multiplier != 0.0 {
                        multiplier = 1.0 / multiplier;
                    }
                    value = value * multiplier + tag.base;
                    if let (Some(limit), Some(spec)) = (limit, &tag.limit) {
                        match spec.mode {
                            MultiplierLimitMode::FactorMaximum => {}
                            MultiplierLimitMode::TotalMaximum => value = lua_min(value, limit),
                            MultiplierLimitMode::TotalMinimum => value = lua_max(value, limit),
                        }
                    }
                }
                ScalingTag::Threshold(tag) => {
                    let multiplier = environment.variable_sum(&tag.variables, query)?;
                    let threshold = environment.scalar(&tag.threshold, query)?;
                    if (tag.upper && multiplier > threshold)
                        || (tag.equals && multiplier != threshold)
                        || (!tag.upper && multiplier < threshold)
                    {
                        return Ok(None);
                    }
                }
                ScalingTag::Limit {
                    value: limit,
                    negative,
                } => {
                    let limit = environment.scalar(limit, query)?;
                    value = if *negative {
                        lua_max(value, -limit)
                    } else {
                        lua_min(value, limit)
                    };
                }
                ScalingTag::Condition(tag) => {
                    if !environment.conditions.matches(std::slice::from_ref(tag)) {
                        return Ok(None);
                    }
                }
                ScalingTag::Unsupported(_) => {
                    unreachable!("Scaling program validated at construction")
                }
            }
        }
        Ok(Some(value))
    }
}

// Match the selected x64 LuaJIT oracle's second operand on equal/unordered inputs.
fn lua_min(left: f64, right: f64) -> f64 {
    if left < right { left } else { right }
}
fn lua_max(left: f64, right: f64) -> f64 {
    if left > right { left } else { right }
}
