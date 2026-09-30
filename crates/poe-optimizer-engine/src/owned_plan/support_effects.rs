//! One native attempt: shared prefix, ordered selection, receiving admission, final closure.
use super::compile::{
    BoundSupportAdmission, BoundSupportReceiving, BoundSupportTemplate, RetainedApplication,
    SupportSuffix, SymbolicBindings,
};
use super::graph::ExecutionGraphView;
use super::support_outputs::{BoundSupportOutputs, PreparedTypeOutput};
use super::*;
use crate::owned_supports::*;
use poe_optimizer_data::{
    owned_stages::OwnedEvaluationStages, owned_support_inputs::OwnedSupportInputBindings,
    owned_support_outputs::OwnedSupportOutputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};

pub struct SupportEffectPlanInputs<I> {
    pub request: Arc<OwnedEvaluationRequest>,
    pub definitions: Arc<I>,
    pub rules: Arc<CompiledRulePackage>,
    pub routing: Arc<OwnedActionRouting>,
    pub stages: Arc<OwnedEvaluationStages>,
    pub preparation: Arc<OwnedSupportPreparation>,
    pub inputs: Arc<OwnedSupportInputBindings>,
    pub receiving: Arc<OwnedSupportReceiving>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SupportEffectsOutcome {
    Evaluated {
        effects: OwnedEffectsReport,
    },
    Unavailable {
        cause: EffectValue,
        input: Option<Box<PlanValueKey>>,
    },
    PreparationUnresolved {
        target: SkillTarget,
        reason: SupportPreparationGap,
        origin_index: Option<usize>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SupportEffectsReport {
    pub identity: OwnedContentDigest,
    pub gaps: Vec<PlanGap>,
    pub outcome: SupportEffectsOutcome,
}

/// Private execution result. Only a consumer running inside the sealed attempt
/// can observe its final bindings and scratch; diagnostic reports cannot resume it.
pub(super) enum SupportAttempt<T> {
    Evaluated(T),
    Unavailable {
        cause: EffectValue,
        input: Option<Box<PlanValueKey>>,
    },
    PreparationUnresolved {
        target: SkillTarget,
        reason: SupportPreparationGap,
        origin_index: Option<usize>,
    },
}

pub(super) struct AdmissionStep {
    pub(super) target: SkillTarget,
    pub(super) assigned: bool,
    pub(super) summoner: Option<SkillTarget>,
}
pub(super) struct PreparedContext {
    pub(super) prepared: Option<PreparedSupports>,
    minion_types: Option<DeclaredSet<OwnedDefinitionKey>>,
}

/// Source-free effect evaluation. Metrics and numerical coverage remain separate.
/// Every report clears private attempt state; callers cannot inject selection proof.
pub struct OwnedSupportEffectPlan<I> {
    plan: OwnedEffectPlan<I>,
    symbolic: SymbolicBindings,
    receiving: BoundSupportReceiving,
    templates: BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
    stages: Arc<OwnedEvaluationStages>,
    inputs: Arc<OwnedSupportInputBindings>,
    preparation: Arc<OwnedSupportPreparation>,
    origins: SupportBuildIndex,
    assignments: BTreeMap<SkillTarget, Vec<SupportAssignmentId>>,
    admissions: BTreeMap<SkillTarget, Vec<AdmissionStep>>,
    outputs: Option<BoundSupportOutputs>,
    schedule: Vec<usize>,
    prefix: BTreeSet<usize>,
    classified: bool,
    identity: OwnedContentDigest,
    limits: SupportPreparationLimits,
}
fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}
fn component(error: SupportPreparationError) -> PlanError {
    match error {
        SupportPreparationError::Limit(name) => PlanError::Limit(name),
        error => PlanError::Invalid(error.to_string()),
    }
}
fn unavailable<T>(cause: EffectValue, input: Option<Box<PlanValueKey>>) -> SupportAttempt<T> {
    SupportAttempt::Unavailable { cause, input }
}
impl<I: DefinitionSchemaIndex> OwnedSupportEffectPlan<I> {
    pub fn compile(
        args: SupportEffectPlanInputs<I>,
        limits: PlanLimits,
        support_limits: SupportPreparationLimits,
    ) -> Result<Self> {
        Self::compile_inner(args, None, limits, support_limits)
    }
    pub fn compile_with_outputs(
        args: SupportEffectPlanInputs<I>,
        outputs: Arc<OwnedSupportOutputBindings>,
        limits: PlanLimits,
        support_limits: SupportPreparationLimits,
    ) -> Result<Self> {
        Self::compile_inner(args, Some(outputs), limits, support_limits)
    }
    fn compile_inner(
        args: SupportEffectPlanInputs<I>,
        output_package: Option<Arc<OwnedSupportOutputBindings>>,
        limits: PlanLimits,
        support_limits: SupportPreparationLimits,
    ) -> Result<Self> {
        let SupportEffectPlanInputs {
            request,
            definitions,
            rules,
            routing,
            stages,
            preparation,
            inputs,
            receiving,
        } = args;
        support_limits.validate().map_err(component)?;
        let stored = rules
            .source_identity()
            .ok_or_else(|| invalid("support effects require stored rule bindings"))?;
        let r = receiving.input();
        let p = preparation.input();
        let s = stages.input();
        let i = inputs.input();
        if r.definitions != *definitions.identity()
            || p.definitions != *definitions.identity()
            || s.definitions != *definitions.identity()
            || i.definitions != *definitions.identity()
            || r.namespace != *definitions.namespace()
            || p.namespace != *definitions.namespace()
            || s.namespace != *definitions.namespace()
            || i.namespace != *definitions.namespace()
            || r.rules != stored
            || p.rules != stored
            || s.rules != stored
            || i.rules != stored
            || r.preparation != *preparation.identity()
            || r.inputs != *inputs.identity()
            || r.stages != *stages.identity()
            || i.preparation != *preparation.identity()
            || i.stages != *stages.identity()
            || s.routing != *routing.identity()
        {
            return Err(invalid("support effect packages have different bindings"));
        }
        if let Some(package) = &output_package {
            let o = package.input();
            if o.definitions != *definitions.identity()
                || o.namespace != *definitions.namespace()
                || o.rules != stored
                || o.preparation != *preparation.identity()
                || o.inputs != *inputs.identity()
                || o.receiving != *receiving.identity()
                || o.stages != *stages.identity()
            {
                return Err(invalid("support output package has different bindings"));
            }
        }
        let compiled =
            compile::compile_receiving(request, definitions, rules, routing, limits, &receiving)?;
        let plan = compiled.plan;
        let receiving = compiled
            .receiving
            .ok_or_else(|| invalid("missing receiving compilation"))?;
        let symbolic = compiled
            .symbolic
            .ok_or_else(|| invalid("missing symbolic support bindings"))?;
        let mut work = limits.max_work;
        let origins =
            SupportBuildIndex::new_with_budget(plan.request.build(), support_limits, &mut work)
                .map_err(component)?;
        let mut assignments: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (id, binding) in &receiving.assignments {
            charge(&mut work, 1)?;
            if binding.origin != SupportOrigin::Assignment(*id) {
                return Err(invalid("support origin identity differs"));
            }
            assignments
                .entry(binding.target.clone())
                .or_default()
                .push(*id);
        }
        let mut admissions =
            admission_orders(&assignments, &compiled.templates, limits, &mut work)?;
        let outputs = if let Some(package) = output_package {
            charge(&mut work, package.input().final_skill_types.len())?;
            let stats = package
                .input()
                .final_skill_types
                .iter()
                .map(|row| row.stat.clone())
                .collect();
            let targets =
                symbolic.support_output_targets(&plan, &compiled.templates, &stats, &mut work)?;
            Some(BoundSupportOutputs::bind(
                package,
                &plan,
                targets,
                &mut assignments,
                &mut admissions,
                &mut work,
            )?)
        } else {
            None
        };
        charge(&mut work, admissions.values().map(Vec::len).sum())?;
        let targets: BTreeSet<_> = admissions.values().flatten().map(|s| &s.target).collect();
        let (schedule, classified) =
            supports::preparation_schedule(&plan, &stages, &inputs, targets, &mut work)?;
        charge(&mut work, schedule.len())?;
        let prefix: BTreeSet<_> = schedule.iter().copied().collect();
        symbolic.validate_prefix(
            &plan,
            &prefix,
            &compiled.templates,
            outputs.as_ref().map(|o| &o.keys),
            &mut work,
        )?;
        let mut identity = digest_owned(
            "owned-support-effect-plan-v1",
            &(
                plan.identity,
                *stages.identity(),
                *preparation.identity(),
                *inputs.identity(),
                receiving.package_identity,
            ),
            limits.max_wire_bytes,
        )?;
        if let Some(outputs) = &outputs {
            identity = digest_owned(
                "owned-support-effect-plan-v2",
                &(identity, *outputs.package.identity()),
                limits.max_wire_bytes,
            )?;
        }
        Ok(Self {
            plan,
            symbolic,
            receiving,
            templates: compiled.templates,
            stages,
            inputs,
            preparation,
            origins,
            assignments,
            admissions,
            outputs,
            schedule,
            prefix,
            classified,
            identity,
            limits: support_limits,
        })
    }
    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    pub fn gaps(&self) -> &[PlanGap] {
        &self.plan.gaps
    }
    pub fn definitions(&self) -> &I {
        self.plan.definitions()
    }
    pub fn request(&self) -> &OwnedEvaluationRequest {
        self.plan.request()
    }
    pub fn binding_report(&self) -> &DefinitionBindingReport {
        self.plan.binding_report()
    }
    pub(super) fn base_plan(&self) -> &OwnedEffectPlan<I> {
        &self.plan
    }
    pub fn new_scratch(&self) -> OwnedPlanScratch {
        self.plan.new_scratch()
    }
    pub fn evaluate(&self, scratch: &mut OwnedPlanScratch) -> Result<SupportEffectsReport> {
        let mut work = self.plan.limits.max_work;
        self.evaluate_with_budget(scratch, &mut work)
    }
    pub fn evaluate_with_budget(
        &self,
        scratch: &mut OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<SupportEffectsReport> {
        self.evaluate_projected(scratch, work, |suffix, scratch, work| {
            self.collect_effects(suffix, scratch, work)
        })
        .map(|outcome| SupportEffectsReport {
            identity: self.identity,
            gaps: self.plan.gaps.clone(),
            outcome: match outcome {
                SupportAttempt::Evaluated(effects) => SupportEffectsOutcome::Evaluated { effects },
                SupportAttempt::Unavailable { cause, input } => {
                    SupportEffectsOutcome::Unavailable { cause, input }
                }
                SupportAttempt::PreparationUnresolved {
                    target,
                    reason,
                    origin_index,
                } => SupportEffectsOutcome::PreparationUnresolved {
                    target,
                    reason,
                    origin_index,
                },
            },
        })
    }
    pub(super) fn evaluate_projected<T>(
        &self,
        scratch: &mut OwnedPlanScratch,
        work: &mut usize,
        finish: impl FnOnce(&SupportSuffix<'_, I>, &OwnedPlanScratch, &mut usize) -> Result<T>,
    ) -> Result<SupportAttempt<T>> {
        let result = self.attempt(scratch, work, finish);
        graph::clear_attempt(scratch);
        result
    }
    fn attempt<T>(
        &self,
        scratch: &mut OwnedPlanScratch,
        work: &mut usize,
        finish: impl FnOnce(&SupportSuffix<'_, I>, &OwnedPlanScratch, &mut usize) -> Result<T>,
    ) -> Result<SupportAttempt<T>> {
        graph::begin_attempt(&self.plan, scratch, work)?;
        charge(work, self.receiving.assignments.len() + 1)?;
        if !self.plan.complete
            || !self.classified
            || self.receiving.assignments.values().any(|a| !a.complete)
        {
            return Ok(unavailable(
                EffectValue::unresolved(PlanGapReason::IncompleteContributors),
                None,
            ));
        }
        graph::execute_indices(&self.plan, scratch, &self.schedule, work)?;
        let inputs = supports::ComputedSupportInputs::new(&self.plan, &self.inputs, self.limits);
        let mut applications = Vec::new();
        let mut prepared_outputs = Vec::new();
        for (assigned, assignments) in &self.assignments {
            charge(work, assignments.len() + 1)?;
            if let Some(cause) = self.activity(assigned, scratch, work)? {
                if cause == EffectValue::Inactive {
                    self.collect_outputs(assigned, None, &mut prepared_outputs, work)?;
                    continue;
                }
                return Ok(unavailable(cause, None));
            }
            // Export only the declared selection inputs, never unrelated disabled
            // physical assignments. The index retains missing-order semantics.
            let mut ordered_origins = Vec::new();
            if let Some(origins) = self.origins.ordered_assignments(assigned) {
                for origin in origins {
                    charge(work, 1)?;
                    if ordered_origins.len() >= self.limits.max_origins {
                        return Err(PlanError::Limit("origins"));
                    }
                    ordered_origins.push(origin);
                }
            }
            let mut values = Vec::with_capacity(ordered_origins.len());
            let mut failures = BTreeMap::new();
            for assignment in &ordered_origins {
                let entity = ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(*assignment));
                let level = inputs.stat(
                    scratch,
                    work,
                    entity.clone(),
                    &self.inputs.input().effective_level,
                )?;
                let quality = inputs.stat(
                    scratch,
                    work,
                    entity,
                    &self.inputs.input().effective_quality,
                )?;
                values.push(EffectiveSupportValues {
                    assignment: *assignment,
                    effective_level: match &level {
                        EffectValue::Known {
                            value: ParameterValue::Integer(value),
                        } => Some(*value),
                        _ => None,
                    },
                    effective_quality: match &quality {
                        EffectValue::Known {
                            value: ParameterValue::Quantity(value),
                        } => Some(value.clone()),
                        _ => None,
                    },
                });
                failures.insert(*assignment, (level, quality));
            }
            let selected = match self
                .origins
                .select_for_target_with_budget(
                    &self.preparation,
                    assigned,
                    &values,
                    self.limits,
                    work,
                )
                .map_err(component)?
            {
                SupportBuildSelectionOutcome::Known(selected) => selected,
                SupportBuildSelectionOutcome::Inactive { .. } => {
                    self.collect_outputs(assigned, None, &mut prepared_outputs, work)?;
                    continue;
                }
                SupportBuildSelectionOutcome::Unresolved {
                    reason,
                    origin_index,
                } => {
                    // Policy input diagnostics remain exact even when a scalar is only
                    // demanded for a replacement/tie comparison.
                    if let Some(position) = origin_index
                        && matches!(
                            reason,
                            SupportPreparationGap::EffectiveLevel
                                | SupportPreparationGap::EffectiveQuality
                        )
                        && let Some(id) = ordered_origins.get(position)
                        && let Some((level, quality)) = failures.get(id)
                    {
                        let (cause, stat) = if reason == SupportPreparationGap::EffectiveLevel {
                            (level, &self.inputs.input().effective_level)
                        } else {
                            (quality, &self.inputs.input().effective_quality)
                        };
                        return Ok(unavailable(
                            cause.clone(),
                            Some(Box::new(PlanValueKey::Stat {
                                entity: ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(
                                    *id,
                                )),
                                stat: stat.clone(),
                            })),
                        ));
                    }
                    return Ok(SupportAttempt::PreparationUnresolved {
                        target: assigned.clone(),
                        reason,
                        origin_index,
                    });
                }
            };
            let mut contexts: BTreeMap<SkillTarget, PreparedContext> = BTreeMap::new();
            for step in &self.admissions[assigned] {
                charge(work, 1)?;
                if let Some(cause) = self.activity(&step.target, scratch, work)? {
                    if cause != EffectValue::Inactive {
                        return Ok(unavailable(cause, None));
                    }
                    contexts.insert(
                        step.target.clone(),
                        PreparedContext {
                            prepared: None,
                            minion_types: None,
                        },
                    );
                    continue;
                }
                let mut target = match if step.assigned {
                    inputs.target_inputs(&step.target, scratch, work)?
                } else {
                    inputs.receiver_inputs(&step.target, scratch, work)?
                } {
                    Ok(target) => target,
                    Err(failure) => return Ok(unavailable(failure.cause, Some(failure.key))),
                };
                if !step.assigned {
                    target.summoner = if let Some(summoner) = &step.summoner {
                        let context = &contexts[summoner];
                        let Some(parent) = &context.prepared else {
                            contexts.insert(
                                step.target.clone(),
                                PreparedContext {
                                    prepared: None,
                                    minion_types: None,
                                },
                            );
                            continue;
                        };
                        charge(
                            work,
                            parent.final_types.len()
                                + context
                                    .minion_types
                                    .as_ref()
                                    .map_or(0, |types| types.members.len()),
                        )?;
                        Some(SupportTypeContext {
                            skill_types: DeclaredSet::complete(parent.final_types.clone()),
                            minion_types: context.minion_types.clone(),
                        })
                    } else {
                        None
                    };
                }
                charge(
                    work,
                    target
                        .types
                        .minion_types
                        .as_ref()
                        .map_or(0, |types| types.members.len()),
                )?;
                let minion_types = target.types.minion_types.clone();
                let prepared = match prepare_selected_supports_with_budget(
                    &self.preparation,
                    &selected,
                    &target,
                    self.limits,
                    work,
                )
                .map_err(component)?
                {
                    SupportPreparationOutcome::Known(prepared) => Some(prepared),
                    SupportPreparationOutcome::Inactive { .. } => None,
                    SupportPreparationOutcome::Unresolved {
                        reason,
                        origin_index,
                    } => {
                        return Ok(SupportAttempt::PreparationUnresolved {
                            target: step.target.clone(),
                            reason,
                            origin_index,
                        });
                    }
                };
                contexts.insert(
                    step.target.clone(),
                    PreparedContext {
                        prepared,
                        minion_types,
                    },
                );
            }
            self.collect_outputs(assigned, Some(&contexts), &mut prepared_outputs, work)?;
            // Position ordering is preserved; a repeated physical assignment may
            // intentionally contribute more than once at distinct selected positions.
            for (position, origin_index) in selected.selected_origin_indices().iter().enumerate() {
                charge(work, 1)?;
                let assignment = selected.origins()[*origin_index].assignment;
                let binding = &self.receiving.assignments[&assignment];
                charge(work, binding.receivers.len())?;
                for receiver in &binding.receivers {
                    let Some(template) = self
                        .templates
                        .get(&(assignment, receiver.context.receiver.clone()))
                    else {
                        continue;
                    };
                    let admission_target = match &receiver.context.admission {
                        BoundSupportAdmission::AssignedSkill { target }
                        | BoundSupportAdmission::ReceivingSkill { target, .. } => target,
                    };
                    let Some(prepared) = &contexts[admission_target].prepared else {
                        continue;
                    };
                    let retained = prepared
                        .selected
                        .get(position)
                        .ok_or_else(|| invalid("receiving admission changed selected positions"))?;
                    if retained.assignment != assignment
                        || retained.position != position
                        || retained.origin_index != *origin_index
                    {
                        return Err(invalid(
                            "receiving admission changed selected origin identity",
                        ));
                    }
                    if applications.len() >= self.plan.limits.max_owner_bindings {
                        return Err(PlanError::Limit("support applications"));
                    }
                    applications.push(RetainedApplication {
                        key: SupportApplicationKey {
                            prepared: PreparedSupportKey {
                                target: assigned.clone(),
                                origin: SupportOrigin::Assignment(assignment),
                                position: u32::try_from(position)
                                    .map_err(|_| PlanError::Limit("support positions"))?,
                            },
                            receiver: receiver.context.receiver.clone(),
                        },
                        eligible: retained.applicable,
                        template,
                    });
                }
            }
        }
        let suffix = self.symbolic.bind_suffix(
            &self.plan,
            &applications,
            &prepared_outputs,
            &self.prefix,
            work,
        )?;
        suffix.validate_stages(&self.stages, work)?;
        graph::extend_attempt(
            self.plan.identity,
            self.plan.effects.len(),
            suffix.effect_count(),
            scratch,
            work,
            self.plan.limits,
        )?;
        graph::execute_view(
            self.plan.identity,
            &suffix,
            scratch,
            &suffix.order,
            work,
            self.plan.limits,
        )?;
        Ok(SupportAttempt::Evaluated(finish(&suffix, scratch, work)?))
    }
    fn collect_outputs(
        &self,
        selection: &SkillTarget,
        contexts: Option<&BTreeMap<SkillTarget, PreparedContext>>,
        values: &mut Vec<PreparedTypeOutput>,
        work: &mut usize,
    ) -> Result<()> {
        if let Some(outputs) = &self.outputs {
            outputs.collect(selection, contexts, values, work)?;
        }
        Ok(())
    }
    fn collect_effects(
        &self,
        suffix: &SupportSuffix<'_, I>,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<OwnedEffectsReport> {
        charge(
            work,
            suffix.effect_count() + suffix.values.len() + self.plan.gaps.len(),
        )?;
        let mut effects = Vec::with_capacity(suffix.effect_count());
        for index in 0..suffix.effect_count() {
            let node = suffix.effect(index).expect("bound support effect");
            effects.push(BoundEffectResult {
                key: node.key.clone(),
                target: node.target.clone(),
                value: scratch.values[index]
                    .as_ref()
                    .ok_or_else(|| invalid("incomplete support effect schedule"))?
                    .clone(),
            });
        }
        let values = suffix
            .values
            .iter()
            .map(|(key, index)| {
                Ok(ResolvedPlanValue {
                    key: key.clone(),
                    value: scratch.values[*index]
                        .as_ref()
                        .ok_or_else(|| invalid("missing final support value"))?
                        .clone(),
                })
            })
            .collect::<Result<_>>()?;
        Ok(OwnedEffectsReport {
            identity: self.identity,
            gaps: self.plan.gaps.clone(),
            effects,
            values,
        })
    }
    fn activity(
        &self,
        target: &SkillTarget,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<Option<EffectValue>> {
        let gates = self
            .plan
            .preparation_gates
            .get(target)
            .ok_or_else(|| invalid("support admission has no bound target activation"))?;
        graph::gate_result(
            gates,
            &scratch.values,
            &self.inputs.input().preparation_stage,
            work,
        )
    }
}

fn admission_orders(
    assignments: &BTreeMap<SkillTarget, Vec<SupportAssignmentId>>,
    templates: &BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
    limits: PlanLimits,
    work: &mut usize,
) -> Result<BTreeMap<SkillTarget, Vec<AdmissionStep>>> {
    let mut rows: BTreeMap<SkillTarget, BTreeMap<SkillTarget, AdmissionStep>> = BTreeMap::new();
    let mut total = 0usize;
    for assigned in assignments.keys() {
        charge(work, 1)?;
        total = total
            .checked_add(1)
            .filter(|count| *count <= limits.max_owner_bindings)
            .ok_or(PlanError::Limit("support admission contexts"))?;
        rows.entry(assigned.clone()).or_default().insert(
            assigned.clone(),
            AdmissionStep {
                target: assigned.clone(),
                assigned: true,
                summoner: None,
            },
        );
    }
    for template in templates.values() {
        charge(work, 1)?;
        let BoundSupportAdmission::ReceivingSkill { target, summoner } =
            &template.context.admission
        else {
            continue;
        };
        let contexts = rows
            .get_mut(&template.target)
            .ok_or_else(|| invalid("receiving template has no assignment target"))?;
        if let Some(previous) = contexts.get(target) {
            if previous.assigned || previous.summoner != *summoner {
                return Err(invalid(
                    "receiving target has competing admission relationships",
                ));
            }
        } else {
            total = total
                .checked_add(1)
                .filter(|count| *count <= limits.max_owner_bindings)
                .ok_or(PlanError::Limit("support admission contexts"))?;
            contexts.insert(
                target.clone(),
                AdmissionStep {
                    target: target.clone(),
                    assigned: false,
                    summoner: summoner.clone(),
                },
            );
        }
    }
    let mut result = BTreeMap::new();
    for (assigned, mut contexts) in rows {
        let mut ordered = Vec::new();
        let mut available = BTreeSet::new();
        while !contexts.is_empty() {
            charge(work, contexts.len())?;
            let next = contexts
                .iter()
                .find(|(_, step)| step.summoner.as_ref().is_none_or(|s| available.contains(s)))
                .map(|(target, _)| target.clone())
                .ok_or_else(|| invalid("support summoner relationships are missing or cyclic"))?;
            available.insert(next.clone());
            ordered.push(contexts.remove(&next).expect("selected admission context"));
        }
        result.insert(assigned, ordered);
    }
    Ok(result)
}
