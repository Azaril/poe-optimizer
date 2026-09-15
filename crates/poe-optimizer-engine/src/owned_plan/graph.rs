//! Worker execution of a prebound effect DAG. No schema/source lookup or serialization.
use super::*;

fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}
fn known(value: ParameterValue) -> EffectValue {
    EffectValue::Known { value }
}
fn value_at(values: &[Option<EffectValue>], index: usize) -> Result<&EffectValue> {
    values
        .get(index)
        .and_then(Option::as_ref)
        .ok_or_else(|| invalid("effect dependency is absent or not scheduled before its consumer"))
}

/// Stable left fold in the compiler's contribution order. Integer results remain
/// in the exact owned integer domain; quantities retain their prebound unit.
fn combine(
    left: ParameterValue,
    right: &ParameterValue,
    reduction: ContributionReduction,
    node: &OwnedDefinitionKey,
) -> Result<EffectValue> {
    match (left, right) {
        (ParameterValue::Integer(a), ParameterValue::Integer(b)) => {
            let result = match reduction {
                ContributionReduction::Sum => a.get().checked_add(b.get()),
                ContributionReduction::Product => a.get().checked_mul(b.get()),
            }
            .and_then(|v| BoundedInteger::new(v).ok());
            Ok(match result {
                Some(value) => known(ParameterValue::Integer(value)),
                None => EffectValue::NumericalError {
                    node: node.clone(),
                    reason: NumericalFailure::IntegerOverflow,
                },
            })
        }
        (ParameterValue::Quantity(a), ParameterValue::Quantity(b)) if a.unit() == b.unit() => {
            let value = match reduction {
                ContributionReduction::Sum => a.value() + b.value(),
                ContributionReduction::Product => a.value() * b.value(),
            };
            Ok(match FiniteQuantity::new(value, a.unit().clone()) {
                Ok(value) => known(ParameterValue::Quantity(value)),
                Err(_) => EffectValue::NumericalError {
                    node: node.clone(),
                    reason: NumericalFailure::NonFinite,
                },
            })
        }
        _ => Err(invalid("prebound contribution kinds or units differ")),
    }
}

pub(super) fn read(
    binding: &ReadBinding,
    values: &[Option<EffectValue>],
    node: &OwnedDefinitionKey,
    work: &mut usize,
) -> Result<EffectValue> {
    charge(work, 1)?;
    match binding {
        ReadBinding::Constant(Some(value)) => Ok(known(value.clone())),
        ReadBinding::Constant(None) => Ok(EffectValue::unresolved(PlanGapReason::MissingInput)),
        ReadBinding::Missing(reason) => Ok(EffectValue::unresolved(*reason)),
        ReadBinding::Present { source } => Ok(match read(source, values, node, work)? {
            EffectValue::Known { .. } => known(ParameterValue::Boolean(true)),
            EffectValue::Inactive => EffectValue::unresolved(PlanGapReason::MissingInput),
            EffectValue::UnsupportedValue { .. } => {
                EffectValue::unresolved(PlanGapReason::UpstreamUnavailable)
            }
            failure => failure,
        }),
        ReadBinding::Final {
            complete: false, ..
        }
        | ReadBinding::Reduction {
            complete: false, ..
        } => Ok(EffectValue::unresolved(
            PlanGapReason::IncompleteContributors,
        )),
        ReadBinding::Final {
            effect: None,
            complete: true,
        } => Ok(EffectValue::unresolved(PlanGapReason::MissingProducer)),
        ReadBinding::Final {
            effect: Some(index),
            complete: true,
        } => Ok(value_at(values, *index)?.clone()),
        ReadBinding::Reduction {
            effects,
            reduction,
            empty,
            complete: true,
        } => {
            let mut accumulated: Option<ParameterValue> = None;
            for index in effects {
                charge(work, 1)?;
                match value_at(values, *index)? {
                    EffectValue::Inactive => {}
                    EffectValue::Known { value } => {
                        // The explicit identity supplies the expected kind/unit even
                        // when the first actual member would otherwise need no fold.
                        let matches = match (empty, value) {
                            (ParameterValue::Integer(_), ParameterValue::Integer(_)) => true,
                            (ParameterValue::Quantity(a), ParameterValue::Quantity(b)) => {
                                a.unit() == b.unit()
                            }
                            _ => false,
                        };
                        if !matches {
                            return Err(invalid(
                                "prebound contribution does not match its reduction type",
                            ));
                        }
                        match accumulated.take() {
                            None => accumulated = Some(value.clone()),
                            Some(previous) => match combine(previous, value, *reduction, node)? {
                                EffectValue::Known { value } => accumulated = Some(value),
                                failure => return Ok(failure),
                            },
                        }
                    }
                    failure => return Ok(failure.clone()),
                }
            }
            Ok(known(accumulated.unwrap_or_else(|| empty.clone())))
        }
    }
}

/// Gates are a closed conjunction. A false/Inactive gate dominates unknown or
/// numerical gates regardless of their authored order; candidate facts stay lazy.
pub(super) fn gate_result(
    gates: &[ReadBinding],
    values: &[Option<EffectValue>],
    node: &OwnedDefinitionKey,
    work: &mut usize,
) -> Result<Option<EffectValue>> {
    let mut blocked = None;
    for gate in gates {
        match read(gate, values, node, work)? {
            EffectValue::Known {
                value: ParameterValue::Boolean(true),
            } => {}
            EffectValue::Known {
                value: ParameterValue::Boolean(false),
            }
            | EffectValue::Inactive => return Ok(Some(EffectValue::Inactive)),
            EffectValue::Known { .. } => {
                return Err(invalid("prebound activation gate is not boolean"));
            }
            failure => {
                if blocked.is_none() {
                    blocked = Some(failure);
                }
            }
        }
    }
    Ok(blocked)
}

/// An absent upstream final is not a numeric identity. Preserve upstream failure
/// provenance only when the shared lazy expression evaluator actually demands it.
fn demanded(value: EffectValue, input: OwnedDefinitionKey) -> Result<EffectValue> {
    Ok(match value {
        EffectValue::Known { .. } => {
            return Err(invalid(
                "prepared evaluator reported a supplied read as missing",
            ));
        }
        EffectValue::Inactive => EffectValue::Unresolved {
            reason: PlanGapReason::UpstreamUnavailable,
            read: Some(input),
        },
        EffectValue::Unresolved { reason, read } => EffectValue::Unresolved {
            reason,
            read: read.or(Some(input)),
        },
        failure => failure,
    })
}
fn program(
    invocation: &Invocation,
    effect: usize,
    values: &[Option<EffectValue>],
    facts: &mut Vec<Option<ParameterValue>>,
    scratch: &mut RuleScratch,
    work: &mut usize,
) -> Result<EffectValue> {
    if invocation.reads.len() != invocation.read_ids.len() {
        return Err(invalid("prebound read identity table differs"));
    }
    charge(work, invocation.reads.len())?;
    facts.clear();
    facts.resize_with(invocation.reads.len(), || None);
    for index in invocation.program.effect_read_indices(effect)? {
        charge(work, 1)?;
        let binding = invocation
            .reads
            .get(*index)
            .ok_or_else(|| invalid("prebound effect read index is out of bounds"))?;
        let value = read(binding, values, &invocation.read_ids[*index], work)?;
        facts[*index] = value.value().cloned();
    }
    if *work == 0 {
        return Err(PlanError::Limit("work"));
    }
    let evaluated = invocation
        .program
        .evaluate_effect_indexed(effect, facts, scratch, *work);
    // Also charge failed attempts. Prepared execution caps this allowance by its
    // package limit and preserves attempted work telemetry after clearing errors.
    charge(work, scratch.work_used())?;
    Ok(match evaluated? {
        EffectDisposition::Applied { value } => known(value),
        EffectDisposition::Inactive => EffectValue::Inactive,
        EffectDisposition::UnsupportedValue { value } => EffectValue::UnsupportedValue { value },
        EffectDisposition::UnsupportedDomain {
            node,
            table,
            key,
            minimum,
            maximum,
        } => EffectValue::UnsupportedDomain {
            node,
            table,
            key,
            minimum,
            maximum,
        },
        EffectDisposition::NumericalError { node, reason } => {
            EffectValue::NumericalError { node, reason }
        }
        EffectDisposition::Unresolved { input } => {
            // Canonical prepared read IDs are sorted once during compilation.
            charge(work, invocation.read_ids.len())?;
            let index = invocation
                .read_ids
                .binary_search(&input)
                .map_err(|_| invalid("prepared evaluator reported an unknown read"))?;
            demanded(read(&invocation.reads[index], values, &input, work)?, input)?
        }
    })
}

// Individual effect rows remain useful evidence under a partial plan. Public
// final values require the compiler's complete incoming-membership proof.
fn final_value(value: &EffectValue, complete: bool) -> EffectValue {
    if complete {
        value.clone()
    } else {
        EffectValue::unresolved(PlanGapReason::IncompleteContributors)
    }
}

pub(super) fn execute<I: DefinitionSchemaIndex>(
    plan: &OwnedEffectPlan<I>,
    scratch: &mut OwnedPlanScratch,
) -> Result<usize> {
    execute_limited(plan, scratch, plan.limits.max_work)
}

// A composite native evaluation shares one decreasing budget across loadouts.
// Existing single-plan callers retain their exact previous limit.
pub(super) fn execute_limited<I: DefinitionSchemaIndex>(
    plan: &OwnedEffectPlan<I>,
    scratch: &mut OwnedPlanScratch,
    maximum_work: usize,
) -> Result<usize> {
    // No value from an earlier attempt can satisfy a dependency in this one.
    scratch.values.clear();
    scratch.facts.clear();
    scratch.values.resize_with(plan.effects.len(), || None);
    let mut work = maximum_work.min(plan.limits.max_work);
    let result = (|| {
        charge(&mut work, plan.effects.len())?;
        if plan.order.len() != plan.effects.len() {
            return Err(invalid("prebound effect order is incomplete"));
        }
        for index in &plan.order {
            charge(&mut work, 1)?;
            let effect = plan
                .effects
                .get(*index)
                .ok_or_else(|| invalid("prebound effect order is out of bounds"))?;
            if scratch.values[*index].is_some() {
                return Err(invalid("prebound effect order repeats a node"));
            }
            for dependency in &effect.dependencies {
                charge(&mut work, 1)?;
                value_at(&scratch.values, *dependency)?;
            }
            let value = if let Some(blocked) = gate_result(
                &effect.gates,
                &scratch.values,
                &effect.key.effect,
                &mut work,
            )? {
                blocked
            } else {
                match &effect.operation {
                    EffectOperation::Route { source } => {
                        read(source, &scratch.values, &effect.key.effect, &mut work)?
                    }
                    EffectOperation::Program { invocation, effect } => {
                        let invocation = plan
                            .invocations
                            .get(*invocation)
                            .ok_or_else(|| invalid("prebound invocation index is out of bounds"))?;
                        program(
                            invocation,
                            *effect,
                            &scratch.values,
                            &mut scratch.facts,
                            &mut scratch.rule,
                            &mut work,
                        )?
                    }
                }
            };
            scratch.values[*index] = Some(value);
        }
        Ok(work)
    })();
    if result.is_err() {
        scratch.values.clear();
        scratch.facts.clear();
    }
    result
}

pub(super) fn evaluate<I: DefinitionSchemaIndex>(
    plan: &OwnedEffectPlan<I>,
    scratch: &mut OwnedPlanScratch,
) -> Result<OwnedEffectsReport> {
    let mut work = execute(plan, scratch)?;
    let result = (|| {
        charge(
            &mut work,
            plan.effects.len() + plan.values.len() + plan.gaps.len(),
        )?;
        let effects = plan
            .effects
            .iter()
            .enumerate()
            .map(|(i, effect)| {
                Ok(BoundEffectResult {
                    key: effect.key.clone(),
                    target: effect.target.clone(),
                    value: value_at(&scratch.values, i)?.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let values = plan
            .values
            .iter()
            .map(|(key, index)| {
                Ok(ResolvedPlanValue {
                    key: key.clone(),
                    value: final_value(value_at(&scratch.values, *index)?, plan.complete),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(OwnedEffectsReport {
            identity: plan.identity,
            gaps: plan.gaps.clone(),
            effects,
            values,
        })
    })();
    if result.is_err() {
        scratch.values.clear();
        scratch.facts.clear();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(v: &str) -> OwnedDefinitionKey {
        OwnedDefinitionKey::new(v).unwrap()
    }
    fn integer(v: i64) -> ParameterValue {
        ParameterValue::Integer(BoundedInteger::new(v).unwrap())
    }
    fn quantity(v: f64) -> ParameterValue {
        ParameterValue::Quantity(
            FiniteQuantity::new(
                v,
                UnitDefId::parse(GameVersionNamespace::new("graph", "v1").unwrap(), "unit")
                    .unwrap(),
            )
            .unwrap(),
        )
    }
    fn reduction(
        effects: Vec<usize>,
        empty: ParameterValue,
        operation: ContributionReduction,
        complete: bool,
    ) -> ReadBinding {
        ReadBinding::Reduction {
            effects,
            reduction: operation,
            empty,
            complete,
        }
    }
    #[test]
    fn complete_reductions_skip_inactive_but_never_default_missing_or_partial_members() {
        let values = vec![
            Some(EffectValue::Inactive),
            Some(known(integer(3))),
            Some(known(integer(5))),
        ];
        let mut work = 100;
        assert_eq!(
            read(
                &reduction(vec![0, 1, 2], integer(0), ContributionReduction::Sum, true),
                &values,
                &key("sum"),
                &mut work
            )
            .unwrap(),
            known(integer(8))
        );
        assert_eq!(
            read(
                &reduction(vec![0], integer(1), ContributionReduction::Product, true),
                &values,
                &key("product"),
                &mut work
            )
            .unwrap(),
            known(integer(1))
        );
        assert_eq!(
            read(
                &reduction(vec![], integer(0), ContributionReduction::Sum, false),
                &values,
                &key("partial"),
                &mut work
            )
            .unwrap(),
            EffectValue::unresolved(PlanGapReason::IncompleteContributors)
        );
        assert_eq!(
            read(
                &ReadBinding::Final {
                    effect: None,
                    complete: true
                },
                &values,
                &key("final"),
                &mut work
            )
            .unwrap(),
            EffectValue::unresolved(PlanGapReason::MissingProducer)
        );
        assert_eq!(
            read(
                &ReadBinding::Final {
                    effect: Some(0),
                    complete: true
                },
                &values,
                &key("final"),
                &mut work
            )
            .unwrap(),
            EffectValue::Inactive
        );
        assert_eq!(
            demanded(EffectValue::Inactive, key("needed")).unwrap(),
            EffectValue::Unresolved {
                reason: PlanGapReason::UpstreamUnavailable,
                read: Some(key("needed"))
            }
        );
    }
    #[test]
    fn reduction_order_is_stable_and_overflow_preserves_typed_failure() {
        let mut work = 100;
        let order = vec![
            Some(known(quantity(1e16))),
            Some(known(quantity(-1e16))),
            Some(known(quantity(1.0))),
        ];
        assert_eq!(
            read(
                &reduction(
                    vec![0, 1, 2],
                    quantity(0.0),
                    ContributionReduction::Sum,
                    true
                ),
                &order,
                &key("sum"),
                &mut work
            )
            .unwrap(),
            known(quantity(1.0))
        );
        let integers = vec![
            Some(known(integer(BoundedInteger::MAX))),
            Some(known(integer(1))),
        ];
        assert_eq!(
            read(
                &reduction(vec![0, 1], integer(0), ContributionReduction::Sum, true),
                &integers,
                &key("sum"),
                &mut work
            )
            .unwrap(),
            EffectValue::NumericalError {
                node: key("sum"),
                reason: NumericalFailure::IntegerOverflow
            }
        );
        let quantities = vec![Some(known(quantity(f64::MAX))), Some(known(quantity(2.0)))];
        assert_eq!(
            read(
                &reduction(
                    vec![0, 1],
                    quantity(1.0),
                    ContributionReduction::Product,
                    true
                ),
                &quantities,
                &key("product"),
                &mut work
            )
            .unwrap(),
            EffectValue::NumericalError {
                node: key("product"),
                reason: NumericalFailure::NonFinite
            }
        );
    }
    #[test]
    fn false_gates_dominate_unknowns_and_failures_in_either_order() {
        let unknown = ReadBinding::Missing(PlanGapReason::MissingProducer);
        let no = ReadBinding::Constant(Some(ParameterValue::Boolean(false)));
        let values = vec![
            Some(EffectValue::NumericalError {
                node: key("upstream"),
                reason: NumericalFailure::DivisionByZero,
            }),
            Some(EffectValue::Inactive),
        ];
        let failure = ReadBinding::Final {
            effect: Some(0),
            complete: true,
        };
        for gates in [
            vec![unknown.clone(), failure.clone(), no.clone()],
            vec![no, unknown, failure],
        ] {
            assert_eq!(
                gate_result(&gates, &values, &key("gate"), &mut 100).unwrap(),
                Some(EffectValue::Inactive)
            );
        }
        assert_eq!(
            gate_result(
                &[ReadBinding::Final {
                    effect: Some(1),
                    complete: true
                }],
                &values,
                &key("gate"),
                &mut 100
            )
            .unwrap(),
            Some(EffectValue::Inactive)
        );
        let upstream = values[0].clone().unwrap();
        assert_eq!(demanded(upstream.clone(), key("read")).unwrap(), upstream);
    }
    #[test]
    fn incomplete_plan_never_promotes_component_evidence_to_final_values() {
        let rows = [
            known(integer(17)),
            EffectValue::Inactive,
            EffectValue::NumericalError {
                node: key("component"),
                reason: NumericalFailure::DivisionByZero,
            },
        ];
        for evidence in rows {
            let preserved = evidence.clone();
            assert_eq!(
                final_value(&evidence, false),
                EffectValue::unresolved(PlanGapReason::IncompleteContributors)
            );
            assert_eq!(final_value(&evidence, true), preserved);
            assert_eq!(evidence, preserved);
        }
    }
    #[test]
    fn required_presence_is_typed_value_availability_not_truthiness_or_activation() {
        let values = vec![
            Some(known(integer(0))),
            Some(known(ParameterValue::Boolean(false))),
            Some(EffectValue::Inactive),
            Some(EffectValue::UnsupportedValue {
                value: integer(101),
            }),
        ];
        let present = |index| ReadBinding::Present {
            source: Box::new(ReadBinding::Final {
                effect: Some(index),
                complete: true,
            }),
        };
        for index in [0, 1] {
            assert_eq!(
                read(&present(index), &values, &key("required"), &mut 100).unwrap(),
                known(ParameterValue::Boolean(true))
            );
        }
        assert_eq!(
            read(&present(2), &values, &key("required"), &mut 100).unwrap(),
            EffectValue::unresolved(PlanGapReason::MissingInput)
        );
        assert_eq!(
            read(&present(3), &values, &key("required"), &mut 100).unwrap(),
            EffectValue::unresolved(PlanGapReason::UpstreamUnavailable)
        );
        assert_eq!(
            gate_result(
                &[
                    present(2),
                    ReadBinding::Constant(Some(ParameterValue::Boolean(false)))
                ],
                &values,
                &key("gates"),
                &mut 100
            )
            .unwrap(),
            Some(EffectValue::Inactive)
        );
    }
    #[test]
    fn unsupported_lookup_domain_survives_required_reads_reductions_and_demand() {
        let failure = EffectValue::UnsupportedDomain {
            node: key("lookup"),
            table: key("levels"),
            key: BoundedInteger::new(41).unwrap(),
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(40).unwrap(),
        };
        let values = vec![Some(failure.clone())];
        let final_read = ReadBinding::Final {
            effect: Some(0),
            complete: true,
        };
        let required = ReadBinding::Present {
            source: Box::new(final_read.clone()),
        };
        let reduction = ReadBinding::Reduction {
            effects: vec![0],
            reduction: ContributionReduction::Sum,
            empty: integer(0),
            complete: true,
        };
        for binding in [final_read, required.clone(), reduction] {
            let result = read(&binding, &values, &key("consumer"), &mut 100).unwrap();
            assert_eq!(result, failure);
            assert_eq!(demanded(result, key("demanded")).unwrap(), failure);
        }
        let inactive = ReadBinding::Constant(Some(ParameterValue::Boolean(false)));
        for gates in [[required.clone(), inactive.clone()], [inactive, required]] {
            assert_eq!(
                gate_result(&gates, &values, &key("consumer"), &mut 100).unwrap(),
                Some(EffectValue::Inactive),
            );
        }
        assert_eq!(final_value(&failure, true), failure);
        assert_eq!(
            final_value(&failure, false),
            EffectValue::unresolved(PlanGapReason::IncompleteContributors),
        );
        assert_eq!(values[0].as_ref(), Some(&failure));
    }
}
