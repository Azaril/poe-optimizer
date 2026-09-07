//! Backend-neutral scalar objective assessment. This does not establish build legality.
use crate::metrics::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub const OBJECTIVE_SCHEMA_VERSION: u32 = 1;
pub const MAX_CONSTRAINTS: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveSpec {
    pub schema_version: u32,
    pub objective: ObjectivePolicy,
    #[serde(default)]
    pub constraints: Vec<MetricConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObjectivePolicy {
    Scalar {
        metric: MetricQuery,
        unit: MetricUnit,
        direction: Direction,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Maximize,
    Minimize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison {
    #[serde(rename = ">=")]
    AtLeast,
    #[serde(rename = ">")]
    GreaterThan,
    #[serde(rename = "<=")]
    AtMost,
    #[serde(rename = "<")]
    LessThan,
}
impl Comparison {
    fn passes(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::AtLeast => value >= threshold,
            Self::GreaterThan => value > threshold,
            Self::AtMost => value <= threshold,
            Self::LessThan => value < threshold,
        }
    }
    fn lower(self) -> bool {
        matches!(self, Self::AtLeast | Self::GreaterThan)
    }
    fn strict(self) -> bool {
        matches!(self, Self::GreaterThan | Self::LessThan)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricConstraint {
    pub id: String,
    pub metric: MetricQuery,
    pub unit: MetricUnit,
    pub operator: Comparison,
    pub threshold: f64,
    /// Explicit positive scale in the same units as this metric; no implicit weights.
    pub violation_scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    ConstraintsSatisfied,
    ConstraintsViolated,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintStatus {
    Satisfied,
    Violated,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintAssessment {
    pub constraint: MetricConstraint,
    pub status: ConstraintStatus,
    pub observed: MeasurementValue,
    /// Distance to the boundary in metric units. A failed strict equality has zero gap.
    /// Absent if the observation is unavailable or nonfinite; overflow stays classified.
    pub shortfall: Option<MeasurementValue>,
    pub normalized_violation: Option<MeasurementValue>,
    pub strict_boundary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveAssessment {
    pub schema_version: u32,
    pub specification: ObjectiveSpec,
    /// Combines primary availability and constraint results, never legality or coverage.
    pub status: AssessmentStatus,
    /// Required measurements with the exact metric schema versions used for assessment.
    pub measurements: Vec<MetricMeasurement>,
    pub objective_value: MeasurementValue,
    /// Finite objective oriented so larger is better; minimization negates the value.
    /// It never makes a nonfinite value eligible for selection.
    pub objective_score: MeasurementValue,
    pub constraints: Vec<ConstraintAssessment>,
    pub total_normalized_violation: MeasurementValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveError(pub String);
impl fmt::Display for ObjectiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ObjectiveError {}
fn invalid(message: impl Into<String>) -> ObjectiveError {
    ObjectiveError(message.into())
}

/// Search/application boundary for assessment; policies do not know game metric names.
/// A satisfied assessment is not proof that a candidate is legal or verified.
pub trait ScoringPolicy {
    fn required_metrics(&self) -> Vec<MetricQuery>;
    fn assess(
        &self,
        measurements: &[MetricMeasurement],
    ) -> Result<ObjectiveAssessment, ObjectiveError>;
}

pub struct CompiledObjective {
    specification: ObjectiveSpec,
    definitions: BTreeMap<MetricQuery, (MetricUnit, u32)>,
}
impl ObjectiveSpec {
    pub fn compile(
        &self,
        catalog: &[MetricDefinition],
    ) -> Result<CompiledObjective, ObjectiveError> {
        if self.schema_version != OBJECTIVE_SCHEMA_VERSION {
            return Err(invalid("Unsupported objective schema version"));
        }
        if self.constraints.len() > MAX_CONSTRAINTS {
            return Err(invalid("Too many objective constraints"));
        }
        let ObjectivePolicy::Scalar { metric, unit, .. } = &self.objective;
        let mut definitions = BTreeMap::new();
        let mut resolve = |query: &MetricQuery, unit: MetricUnit| -> Result<(), ObjectiveError> {
            let matching: Vec<_> = catalog
                .iter()
                .filter(|definition| {
                    definition.id == query.id && definition.actors.contains(&query.actor)
                })
                .collect();
            if matching.is_empty() {
                return Err(invalid(format!(
                    "Unknown objective metric {:?}.{}",
                    query.actor, query.id
                )));
            }
            if matching.len() != 1 || matching[0].schema_version == 0 {
                return Err(invalid("Ambiguous or invalid metric catalog definition"));
            }
            if matching[0].unit != unit {
                return Err(invalid(format!(
                    "Wrong unit for objective metric {:?}.{}",
                    query.actor, query.id
                )));
            }
            definitions.insert(query.clone(), (unit, matching[0].schema_version));
            Ok(())
        };
        resolve(metric, *unit)?;
        let mut ids = BTreeSet::new();
        for constraint in &self.constraints {
            if constraint.id.trim().is_empty()
                || constraint.id.len() > 128
                || !ids.insert(&constraint.id)
            {
                return Err(invalid(
                    "Constraint IDs must be nonempty, unique and at most 128 bytes",
                ));
            }
            if !constraint.threshold.is_finite()
                || !constraint.violation_scale.is_finite()
                || constraint.violation_scale <= 0.0
            {
                return Err(invalid(
                    "Constraint thresholds must be finite and violation scales finite and positive",
                ));
            }
            resolve(&constraint.metric, constraint.unit)?;
        }
        // Contradictions depend on metric identity, not constraint labels or display rounding.
        for lower in self.constraints.iter().filter(|c| c.operator.lower()) {
            for upper in self
                .constraints
                .iter()
                .filter(|c| !c.operator.lower() && c.metric == lower.metric)
            {
                if lower.threshold > upper.threshold
                    || (lower.threshold == upper.threshold
                        && (lower.operator.strict() || upper.operator.strict()))
                {
                    return Err(invalid(format!(
                        "Contradictory bounds: {} and {}",
                        lower.id, upper.id
                    )));
                }
            }
        }
        Ok(CompiledObjective {
            specification: self.clone(),
            definitions,
        })
    }
}

impl ScoringPolicy for CompiledObjective {
    fn required_metrics(&self) -> Vec<MetricQuery> {
        self.definitions.keys().cloned().collect()
    }
    fn assess(
        &self,
        measurements: &[MetricMeasurement],
    ) -> Result<ObjectiveAssessment, ObjectiveError> {
        let mut values = BTreeMap::new();
        for measurement in measurements {
            let Some((unit, version)) = self.definitions.get(&measurement.query) else {
                continue;
            };
            if measurement.unit != *unit || measurement.schema_version != *version {
                return Err(invalid(
                    "Measurement unit or schema disagrees with the compiled objective",
                ));
            }
            if matches!(measurement.value, MeasurementValue::Finite { value } if !value.is_finite())
            {
                return Err(invalid("A finite measurement contains a nonfinite value"));
            }
            if values
                .insert(&measurement.query, &measurement.value)
                .is_some()
            {
                return Err(invalid("Duplicate required measurement"));
            }
        }
        let observed = |query: &MetricQuery| {
            values
                .get(query)
                .map(|value| (*value).clone())
                .unwrap_or_else(|| MeasurementValue::Unavailable {
                    reason: "Required measurement is absent from this evaluation".into(),
                })
        };
        let ObjectivePolicy::Scalar {
            metric, direction, ..
        } = &self.specification.objective;
        let objective_value = observed(metric);
        let objective_score = match objective_value.finite() {
            Some(value) => MeasurementValue::Finite {
                value: match direction {
                    Direction::Maximize => value,
                    Direction::Minimize => -value,
                },
            },
            None => objective_value.clone(),
        };
        let mut unavailable = objective_value.finite().is_none();
        let mut violated = false;
        let mut total = 0.0;
        let mut constraints = Vec::new();
        for constraint in &self.specification.constraints {
            let value = observed(&constraint.metric);
            let (status, shortfall, normalized, strict_boundary) =
                if let Some(number) = value.finite() {
                    let passed = constraint.operator.passes(number, constraint.threshold);
                    violated |= !passed;
                    let gap = if passed {
                        0.0
                    } else if constraint.operator.lower() {
                        constraint.threshold - number
                    } else {
                        number - constraint.threshold
                    };
                    let normalized = gap / constraint.violation_scale;
                    total += normalized;
                    (
                        if passed {
                            ConstraintStatus::Satisfied
                        } else {
                            ConstraintStatus::Violated
                        },
                        Some(MeasurementValue::from_number(gap)),
                        Some(MeasurementValue::from_number(normalized)),
                        !passed && constraint.operator.strict() && number == constraint.threshold,
                    )
                } else {
                    unavailable = true;
                    (ConstraintStatus::Unavailable, None, None, false)
                };
            constraints.push(ConstraintAssessment {
                constraint: constraint.clone(),
                status,
                observed: value,
                shortfall,
                normalized_violation: normalized,
                strict_boundary,
            });
        }
        Ok(ObjectiveAssessment {
            schema_version: OBJECTIVE_SCHEMA_VERSION,
            specification: self.specification.clone(),
            status: if unavailable {
                AssessmentStatus::Unavailable
            } else if violated {
                AssessmentStatus::ConstraintsViolated
            } else {
                AssessmentStatus::ConstraintsSatisfied
            },
            measurements: self
                .definitions
                .iter()
                .map(|(query, (unit, schema_version))| MetricMeasurement {
                    query: query.clone(),
                    unit: *unit,
                    schema_version: *schema_version,
                    value: observed(query),
                })
                .collect(),
            objective_value,
            objective_score,
            constraints,
            total_normalized_violation: if unavailable {
                MeasurementValue::Unavailable {
                    reason: "An objective or constraint measurement is unavailable or nonfinite"
                        .into(),
                }
            } else {
                MeasurementValue::from_number(total)
            },
        })
    }
}
