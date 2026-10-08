//! Computed preparation inputs from the ordinary rule DAG. No caller supplies scalars.
use super::*;
use crate::owned_supports::{
    EffectiveSupportValues, SupportPreparationGap, SupportPreparationLimits,
    SupportPreparationOutcome, prepare_build_supports_with_budget,
};
use poe_optimizer_data::{
    owned_stages::OwnedEvaluationStages, owned_support_inputs::OwnedSupportInputBindings,
    owned_supports::OwnedSupportPreparation,
};

mod inputs;
pub(super) use inputs::ComputedSupportInputs;

pub struct SupportPreparationPlanInputs<I> {
    pub request: Arc<OwnedEvaluationRequest>,
    pub definitions: Arc<I>,
    pub rules: Arc<CompiledRulePackage>,
    pub routing: Arc<OwnedActionRouting>,
    pub stages: Arc<OwnedEvaluationStages>,
    pub preparation: Arc<OwnedSupportPreparation>,
    pub inputs: Arc<OwnedSupportInputBindings>,
    pub target: SkillTarget,
}

/// Preparation is not numerical support delivery or whole-build metric coverage.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ComputedSupportOutcome {
    Prepared {
        result: SupportPreparationOutcome,
    },
    Unavailable {
        cause: EffectValue,
        input: Option<Box<PlanValueKey>>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ComputedSupportReport {
    pub identity: OwnedContentDigest,
    pub gaps: Vec<PlanGap>,
    pub outcome: ComputedSupportOutcome,
}

/// Immutable, source-free preparation plan with worker-owned execution scratch.
/// Its internal graph cannot be promoted to a public metric/effect plan.
pub struct OwnedSupportPreparationPlan<I> {
    plan: OwnedEffectPlan<I>,
    inputs: Arc<OwnedSupportInputBindings>,
    preparation: Arc<OwnedSupportPreparation>,
    target: SkillTarget,
    identity: OwnedContentDigest,
    schedule: Vec<usize>,
    classified: bool,
    limits: SupportPreparationLimits,
}

fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}
fn before_or_equal(
    stages: &OwnedEvaluationStages,
    before: &OwnedDefinitionKey,
    after: &OwnedDefinitionKey,
) -> bool {
    before == after || stages.precedes(before, after)
}

impl<I: DefinitionSchemaIndex> OwnedSupportPreparationPlan<I> {
    pub fn compile(
        args: SupportPreparationPlanInputs<I>,
        limits: PlanLimits,
        support_limits: SupportPreparationLimits,
    ) -> Result<Self> {
        let SupportPreparationPlanInputs {
            request,
            definitions,
            rules,
            routing,
            stages,
            preparation,
            inputs,
            target,
        } = args;
        support_limits
            .validate()
            .map_err(|e| PlanError::Invalid(e.to_string()))?;
        // All packages are validated immutable objects. Recheck their exact join to
        // the compiled rule identity; their constructors used the storage package.
        let stage = stages.input();
        let support = preparation.input();
        let input = inputs.input();
        let source_rules = rules.source_identity().ok_or_else(|| {
            invalid(
                "computed support inputs require rules compiled from a validated stored package",
            )
        })?;
        if stage.definitions != *definitions.identity()
            || support.definitions != *definitions.identity()
            || input.definitions != *definitions.identity()
            || stage.namespace != *definitions.namespace()
            || support.namespace != *definitions.namespace()
            || input.namespace != *definitions.namespace()
            || stage.rules != source_rules
            || support.rules != source_rules
            || input.rules != source_rules
            || stage.routing != *routing.identity()
            || input.stages != *stages.identity()
            || input.preparation != *preparation.identity()
        {
            return Err(invalid("computed support packages have different bindings"));
        }
        let plan =
            compile::compile_with_stages(request, definitions, rules, routing, limits, &stages)?;
        if !plan.preparation_gates.contains_key(&target) {
            return Err(invalid(
                "support preparation target is not a bound skill occurrence",
            ));
        }
        let mut work = limits.max_work;
        let (schedule, classified) =
            preparation_schedule(&plan, &stages, &inputs, std::iter::once(&target), &mut work)?;
        let identity = digest_owned(
            "owned-computed-support-plan-v1",
            &(
                plan.identity,
                *stages.identity(),
                *preparation.identity(),
                *inputs.identity(),
                &target,
            ),
            limits.max_wire_bytes,
        )?;
        Ok(Self {
            plan,
            inputs,
            preparation,
            target,
            identity,
            schedule,
            classified,
            limits: support_limits,
        })
    }
    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    pub fn gaps(&self) -> &[PlanGap] {
        &self.plan.gaps
    }
    pub fn new_scratch(&self) -> OwnedPlanScratch {
        self.plan.new_scratch()
    }
    pub fn evaluate(&self, scratch: &mut OwnedPlanScratch) -> Result<ComputedSupportReport> {
        let mut work = self.plan.limits.max_work;
        self.evaluate_with_budget(scratch, &mut work)
    }
    /// One decreasing allowance for ordinary effects, exports and preparation.
    pub fn evaluate_with_budget(
        &self,
        scratch: &mut OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<ComputedSupportReport> {
        let result = self.evaluate_attempt(scratch, work);
        // Reports are diagnostics. Neither success nor failure leaves an attempt
        // available for a different plan/receiver to consume as preparation proof.
        graph::clear_attempt(scratch);
        result.map(|outcome| ComputedSupportReport {
            identity: self.identity,
            gaps: self.plan.gaps.clone(),
            outcome,
        })
    }
    fn evaluate_attempt(
        &self,
        scratch: &mut OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<ComputedSupportOutcome> {
        graph::begin_attempt(&self.plan, scratch, work)?;
        charge(work, 1)?;
        // This is deliberately independent of which scalars the selection policy
        // demands. Even one support cannot bypass a Partial unrelated owner.
        if !self.plan.complete || !self.classified {
            return Ok(ComputedSupportOutcome::Unavailable {
                cause: EffectValue::unresolved(PlanGapReason::IncompleteContributors),
                input: None,
            });
        }
        graph::execute_indices(&self.plan, scratch, &self.schedule, work)?;
        if let Some(cause) = graph::gate_result(
            &self.plan.preparation_gates[&self.target],
            &scratch.values,
            &self.inputs.input().preparation_stage,
            work,
        )? {
            return Ok(match cause {
                EffectValue::Inactive => ComputedSupportOutcome::Prepared {
                    result: SupportPreparationOutcome::Inactive {
                        target: self.target.clone(),
                    },
                },
                cause => ComputedSupportOutcome::Unavailable { cause, input: None },
            });
        }
        let input_context = ComputedSupportInputs::new(&self.plan, &self.inputs, self.limits);
        let target = match input_context.target_inputs(&self.target, scratch, work)? {
            Ok(target) => target,
            Err(failure) => {
                return Ok(ComputedSupportOutcome::Unavailable {
                    cause: failure.cause,
                    input: Some(failure.key),
                });
            }
        };
        let mut values = Vec::new();
        let mut failures = BTreeMap::new();
        let rows = &self.plan.request.build().input().supports;
        let depth = match &self.target {
            SkillTarget::Authored(_) => 0,
            SkillTarget::Generated(s) => s.provider.grant_path.len(),
        };
        if depth > self.limits.max_target_depth {
            return Err(PlanError::Limit("target depth"));
        }
        charge(work, rows.len().saturating_mul(depth + 1))?;
        for assignment in rows.iter().filter(|s| s.target == self.target) {
            if values.len() >= self.limits.max_origins {
                return Err(PlanError::Limit("origins"));
            }
            charge(work, 1)?;
            let entity = ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(assignment.id));
            let level = input_context.stat(
                scratch,
                work,
                entity.clone(),
                &self.inputs.input().effective_level,
            )?;
            let quality = input_context.stat(
                scratch,
                work,
                entity,
                &self.inputs.input().effective_quality,
            )?;
            values.push(EffectiveSupportValues {
                assignment: assignment.id,
                effective_level: match &level {
                    EffectValue::Known {
                        value: ParameterValue::Integer(v),
                    } => Some(*v),
                    _ => None,
                },
                effective_quality: match &quality {
                    EffectValue::Known {
                        value: ParameterValue::Quantity(v),
                    } => Some(v.clone()),
                    _ => None,
                },
            });
            failures.insert(assignment.id, (level, quality));
        }
        let outcome = prepare_build_supports_with_budget(
            &self.preparation,
            self.plan.request.build(),
            &target,
            &values,
            self.limits,
            work,
        )
        .map_err(|error| match error {
            crate::owned_supports::SupportPreparationError::Limit(name) => PlanError::Limit(name),
            _ => PlanError::Invalid(error.to_string()),
        })?;
        // Preserve upstream failure provenance when the policy actually demands
        // effective level/quality. Unneeded scalar failures cannot pick a winner.
        if let SupportPreparationOutcome::Unresolved {
            reason,
            origin_index: Some(index),
        } = &outcome
            && matches!(
                reason,
                SupportPreparationGap::EffectiveLevel | SupportPreparationGap::EffectiveQuality
            )
            && let Some(sequence) = self
                .plan
                .request
                .build()
                .input()
                .authored_support_order
                .as_ref()
                .and_then(|rows| rows.iter().find(|s| s.target == self.target))
            && let Some(id) = sequence.assignments.get(*index)
            && let Some((level, quality)) = failures.get(id)
        {
            let cause = if *reason == SupportPreparationGap::EffectiveLevel {
                level
            } else {
                quality
            };
            let stat = if *reason == SupportPreparationGap::EffectiveLevel {
                &self.inputs.input().effective_level
            } else {
                &self.inputs.input().effective_quality
            };
            return Ok(ComputedSupportOutcome::Unavailable {
                cause: cause.clone(),
                input: Some(Box::new(PlanValueKey::Stat {
                    entity: ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(*id)),
                    stat: stat.clone(),
                })),
            });
        }
        Ok(ComputedSupportOutcome::Prepared { result: outcome })
    }
}

/// One private prefix for any finite set of exact targets. Stage membership
/// never repairs incomplete owners or makes this an ordinary metric plan.
pub(super) fn preparation_schedule<'a, I>(
    plan: &OwnedEffectPlan<I>,
    stages: &OwnedEvaluationStages,
    inputs: &OwnedSupportInputBindings,
    targets: impl IntoIterator<Item = &'a SkillTarget>,
    work: &mut usize,
) -> Result<(Vec<usize>, bool)> {
    let stage = stages.input();
    let input = inputs.input();
    charge(work, plan.effects.len() + stage.stages.len())?;
    let effect_stages: Vec<_> = plan
        .effects
        .iter()
        .map(|effect| compile::effect_stage(effect, stages, work))
        .collect::<Result<Vec<_>>>()?;
    let classified = stages.is_complete()
        && effect_stages.iter().enumerate().all(|(index, stage)| {
            stage.is_some()
                || matches!(
                    plan.effects[index].operation,
                    EffectOperation::GeneratedInput { .. }
                )
        });
    // Check concrete actor/grant/required-input edges as well as static rules.
    for (index, node) in plan.effects.iter().enumerate() {
        charge(work, node.dependencies.len() + 1)?;
        for dep in &node.dependencies {
            let dependencies = input_dependencies(|index| plan.effects.get(index), dep, work)?;
            for &dep in dependencies.as_ref() {
                if let (Some(before), Some(after)) = (effect_stages[dep], effect_stages[index])
                    && !before_or_equal(stages, before, after)
                {
                    return Err(invalid(
                        "effect dependency crosses a stage backwards or without declared precedence",
                    ));
                }
            }
        }
    }
    let mut gate_dependencies = BTreeSet::new();
    for target in targets {
        charge(work, 1)?;
        let gates = plan
            .preparation_gates
            .get(target)
            .ok_or_else(|| invalid("support preparation target is not a bound skill occurrence"))?;
        for gate in gates {
            compile::read_dependencies(gate, &mut gate_dependencies, work)?;
        }
    }
    for dep in gate_dependencies {
        for &dep in input_dependencies(|index| plan.effects.get(index), &dep, work)?.as_ref() {
            if let Some(before) = effect_stages[dep]
                && !before_or_equal(stages, before, &input.preparation_stage)
            {
                return Err(invalid(
                    "support target activation is scheduled after preparation",
                ));
            }
        }
    }
    // Stable stage order followed by original dependency order within a stage.
    // Contribution-folding order remains sealed in the original read bindings.
    let mut remaining: BTreeSet<_> = stage.stages.iter().map(|s| s.id.clone()).collect();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        charge(work, remaining.len().saturating_mul(remaining.len()))?;
        let next = remaining
            .iter()
            .find(|candidate| {
                !remaining
                    .iter()
                    .any(|other| stages.precedes(other, candidate))
            })
            .cloned()
            .ok_or_else(|| invalid("cyclic stage schedule"))?;
        remaining.remove(&next);
        if before_or_equal(stages, &next, &input.preparation_stage) {
            ordered.push(next);
        }
    }
    let mut prefix = Vec::new();
    for stage_id in ordered {
        charge(work, plan.order.len())?;
        prefix.extend(
            plan.order
                .iter()
                .copied()
                .filter(|i| effect_stages[*i] == Some(&stage_id)),
        );
    }
    if plan
        .effects
        .iter()
        .any(|effect| matches!(effect.operation, EffectOperation::GeneratedInput { .. }))
    {
        // Raw request producers have no invented authored stage. Include them
        // with the dependency-closed prefix and use the existing graph order.
        // Their ordinary parent dependencies still need explicit early stages.
        charge(work, plan.effects.len() + prefix.len())?;
        let mut included: BTreeSet<_> = prefix.into_iter().collect();
        let mut pending: Vec<_> = plan
            .effects
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                matches!(node.operation, EffectOperation::GeneratedInput { .. }).then_some(i)
            })
            .collect();
        while let Some(index) = pending.pop() {
            charge(work, 1)?;
            if !included.insert(index) {
                continue;
            }
            let node = &plan.effects[index];
            if !matches!(node.operation, EffectOperation::GeneratedInput { .. }) {
                match effect_stages[index] {
                    Some(stage) if before_or_equal(stages, stage, &input.preparation_stage) => {}
                    _ => {
                        return Err(invalid(
                            "generated input parent dependency is not scheduled before preparation",
                        ));
                    }
                }
            }
            charge(work, node.dependencies.len())?;
            pending.extend(node.dependencies.iter().copied());
        }
        charge(work, plan.order.len())?;
        prefix = plan
            .order
            .iter()
            .copied()
            .filter(|index| included.contains(index))
            .collect();
    }
    Ok((prefix, classified))
}

/// Request-input nodes are transparent to authored stage ordering. Walk only
/// these nodes; ordinary rule dependencies keep the historical checks above.
pub(super) fn input_dependencies<'a, 'g>(
    effect: impl Fn(usize) -> Option<&'g EffectNode>,
    index: &'a usize,
    work: &mut usize,
) -> Result<std::borrow::Cow<'a, [usize]>> {
    let node =
        effect(*index).ok_or_else(|| invalid("generated input dependency is out of bounds"))?;
    if !matches!(node.operation, EffectOperation::GeneratedInput { .. }) {
        return Ok(std::borrow::Cow::Borrowed(std::slice::from_ref(index)));
    }
    let mut seen = BTreeSet::new();
    let mut pending = vec![*index];
    let mut result = Vec::new();
    while let Some(index) = pending.pop() {
        charge(work, 1)?;
        if !seen.insert(index) {
            continue;
        }
        let node =
            effect(index).ok_or_else(|| invalid("generated input dependency is out of bounds"))?;
        if matches!(node.operation, EffectOperation::GeneratedInput { .. }) {
            charge(work, node.dependencies.len())?;
            pending.extend(node.dependencies.iter().copied());
        } else {
            result.push(index);
        }
    }
    Ok(std::borrow::Cow::Owned(result))
}
