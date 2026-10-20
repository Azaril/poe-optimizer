//! Injected requirements assessed from the same private attempt as measurements.
use super::graph::ExecutionGraphView;
use super::*;

pub(super) type RequirementOwnerKey = (Option<DefinitionAddress>, Option<SlotAddress>);
pub(super) type RequirementOwnerCensus = BTreeMap<RequirementOwnerKey, SchemaSubject>;

pub(super) fn owner_key(subject: &SchemaSubject) -> RequirementOwnerKey {
    match subject {
        SchemaSubject::Definition(id) => (Some(id.clone()), None),
        SchemaSubject::Slot(id) => (None, Some(id.clone())),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RequirementCoverage {
    Complete,
    Incomplete {
        /// Missing owner/program closure and requirement-specific census gaps.
        gaps: Vec<PlanGap>,
        /// An attempt-wide failure, independent of any known individual violation.
        cause: Option<EffectValue>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RequirementOutcome {
    Satisfied,
    Violated,
    Inapplicable,
    Unavailable { cause: EffectValue },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnedRequirementResult {
    /// Exact source, recipient, owner, program and effect occurrence.
    pub key: EffectOccurrenceKey,
    pub code: OwnedDefinitionKey,
    pub outcome: RequirementOutcome,
}

/// This assesses injected requirements, not complete gameplay legality or parity.
/// Known violations and incomplete coverage are deliberately independent.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnedRequirementAssessment {
    pub coverage: RequirementCoverage,
    pub results: Vec<OwnedRequirementResult>,
}
impl OwnedRequirementAssessment {
    pub fn has_violations(&self) -> bool {
        self.results
            .iter()
            .any(|row| matches!(row.outcome, RequirementOutcome::Violated))
    }

    /// All injected requirements are accounted for and none is violated.
    /// Other build coverage and admission gates remain the caller's responsibility.
    pub fn is_satisfied(&self) -> bool {
        matches!(self.coverage, RequirementCoverage::Complete)
            && self.results.iter().all(|row| {
                matches!(
                    row.outcome,
                    RequirementOutcome::Satisfied | RequirementOutcome::Inapplicable
                )
            })
    }
}

/// Cold inventory evidence is independent of requested metric rows. Until Action
/// context discovery is mechanical, even a queried instance cannot close its family.
pub(super) struct RequirementProjection {
    context_gaps: Vec<PlanGap>,
}
impl RequirementProjection {
    pub(super) fn compile(
        rules: &CompiledRulePackage,
        selected_owners: &RequirementOwnerCensus,
        selected_output_gaps: &[PlanGap],
        work: &mut usize,
    ) -> Result<Self> {
        charge(
            work,
            rules.input().owners.len() + selected_output_gaps.len(),
        )?;
        let mut context_gaps = selected_output_gaps.to_vec();
        charge(work, selected_owners.len())?;
        let mut missing_owners = selected_owners.clone();
        for owner in &rules.input().owners {
            charge(
                work,
                selected_owners.len().checked_ilog2().unwrap_or(0) as usize + 1,
            )?;
            let owner_key = owner_key(&owner.owner);
            if !selected_owners.contains_key(&owner_key) {
                continue;
            }
            missing_owners.remove(&owner_key);
            if !owner.programs.is_complete() {
                context_gaps.push(PlanGap {
                    provider: None,
                    subject: Some(owner.owner.clone()),
                    reason: PlanGapReason::PartialPrograms,
                });
            }
            charge(work, owner.programs.members.len())?;
            let mut missing_census = false;
            for program in &owner.programs.members {
                if program.context != RuleEntityKind::Action {
                    continue;
                }
                charge(work, program.effects.len())?;
                missing_census |= program
                    .effects
                    .iter()
                    .any(|effect| matches!(effect.effect, RuleEffectKind::Requirement { .. }));
            }
            if missing_census {
                // The structural owner census is deliberately conservative: no
                // query or observed invocation closes a selected Action family.
                context_gaps.push(PlanGap {
                    provider: None,
                    subject: Some(owner.owner.clone()),
                    reason: PlanGapReason::IncompleteRequirementContexts,
                });
            }
        }
        for owner in missing_owners.into_values() {
            context_gaps.push(PlanGap {
                provider: None,
                subject: Some(owner),
                reason: PlanGapReason::MissingPrograms,
            });
        }
        Ok(Self { context_gaps })
    }

    pub(super) fn collect<G: ExecutionGraphView + ?Sized>(
        &self,
        graph: &G,
        scratch: &OwnedPlanScratch,
        gaps: &[PlanGap],
        attempt_failure: Option<&EffectValue>,
        work: &mut usize,
    ) -> Result<OwnedRequirementAssessment> {
        charge(
            work,
            graph.effect_count() + gaps.len() + self.context_gaps.len(),
        )?;
        let mut incomplete =
            !gaps.is_empty() || !self.context_gaps.is_empty() || attempt_failure.is_some();
        let mut results = Vec::new();
        for index in 0..graph.effect_count() {
            let effect = graph.effect(index).ok_or_else(|| {
                PlanError::Invalid("requirement graph effect index is out of bounds".into())
            })?;
            let BoundEffectTarget::Requirement { code } = &effect.target else {
                continue;
            };
            let value = scratch.values.get(index).and_then(Option::as_ref);
            let outcome = match value {
                Some(EffectValue::Known {
                    value: ParameterValue::Boolean(true),
                }) => RequirementOutcome::Satisfied,
                Some(EffectValue::Known {
                    value: ParameterValue::Boolean(false),
                }) => RequirementOutcome::Violated,
                Some(EffectValue::Known { .. }) => {
                    return Err(PlanError::Invalid(
                        "requirement value violates its Boolean type".into(),
                    ));
                }
                Some(EffectValue::Inactive) => RequirementOutcome::Inapplicable,
                Some(cause) => RequirementOutcome::Unavailable {
                    cause: cause.clone(),
                },
                None => RequirementOutcome::Unavailable {
                    cause: attempt_failure.cloned().ok_or_else(|| {
                        PlanError::Invalid("requirement effect schedule is incomplete".into())
                    })?,
                },
            };
            incomplete |= matches!(outcome, RequirementOutcome::Unavailable { .. });
            results.push(OwnedRequirementResult {
                key: effect.key.clone(),
                code: code.clone(),
                outcome,
            });
        }
        Ok(OwnedRequirementAssessment {
            coverage: if incomplete {
                let mut gaps = gaps.to_vec();
                gaps.extend(self.context_gaps.iter().cloned());
                RequirementCoverage::Incomplete {
                    gaps,
                    cause: attempt_failure.cloned(),
                }
            } else {
                RequirementCoverage::Complete
            },
            results,
        })
    }
}
