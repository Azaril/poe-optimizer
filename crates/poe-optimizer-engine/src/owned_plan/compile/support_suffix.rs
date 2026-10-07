//! Retained applications close symbolic channels without copying the executed prefix.
use super::*;
use crate::owned_plan::graph::ExecutionGraphView;
use crate::owned_plan::support_outputs::{PreparedTypeOutput, target_copy_work};
use crate::owned_plan::support_source_properties::SourcePropertyAttempt;

pub(in crate::owned_plan) struct RetainedApplication<'a> {
    pub key: SupportApplicationKey,
    pub eligible: bool,
    pub template: &'a BoundSupportTemplate,
}

/// Immutable static arenas plus this attempt's appended rows and changed suffix.
pub(in crate::owned_plan) struct SupportSuffix<'a, I> {
    base: &'a OwnedEffectPlan<I>,
    invocations: Vec<Invocation>,
    effects: Vec<EffectNode>,
    invocation_overrides: BTreeMap<usize, Invocation>,
    effect_overrides: BTreeMap<usize, EffectNode>,
    pub values: BTreeMap<PlanValueKey, usize>,
    pub order: Vec<usize>,
    pub query_gates: Vec<Vec<ReadBinding>>,
}
impl<I> ExecutionGraphView for SupportSuffix<'_, I> {
    fn effect_count(&self) -> usize {
        self.base.effects.len() + self.effects.len()
    }
    fn invocation_count(&self) -> usize {
        self.base.invocations.len() + self.invocations.len()
    }
    fn effect(&self, index: usize) -> Option<&EffectNode> {
        if index < self.base.effects.len() {
            self.effect_overrides
                .get(&index)
                .or_else(|| self.base.effects.get(index))
        } else {
            self.effects.get(index - self.base.effects.len())
        }
    }
    fn invocation(&self, index: usize) -> Option<&Invocation> {
        if index < self.base.invocations.len() {
            self.invocation_overrides
                .get(&index)
                .or_else(|| self.base.invocations.get(index))
        } else {
            self.invocations.get(index - self.base.invocations.len())
        }
    }
}

fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}

fn touches(
    read: &PendingRead,
    values: &BTreeSet<PlanValueKey>,
    contributions: &BTreeSet<ContributionKey>,
    work: &mut usize,
) -> Result<bool> {
    charge(work, 1)?;
    Ok(match read {
        PendingRead::Value(key) => values.contains(key),
        PendingRead::Contributions(key, ..) | PendingRead::ContributionQuery(key, ..) => {
            contributions.contains(key)
        }
        PendingRead::Select {
            when_true,
            when_false,
            ..
        } => {
            touches(when_true, values, contributions, work)?
                || touches(when_false, values, contributions, work)?
        }
        PendingRead::Required(source) => touches(source, values, contributions, work)?,
        PendingRead::ModifierTransforms { key, initial } => {
            values.contains(key) || values.contains(initial.as_ref())
        }
        PendingRead::Ready(_) => false,
    })
}

impl SymbolicBindings {
    /// Check potential writes, including positions which may later be rejected.
    /// An empty reduction in the cold graph must not disguise a late dependency.
    pub(in crate::owned_plan) fn validate_prefix<I>(
        &self,
        plan: &OwnedEffectPlan<I>,
        prefix: &BTreeSet<usize>,
        templates: &BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
        source_properties: &BoundSourceProperties,
        prepared_values: Option<&BTreeSet<PlanValueKey>>,
        work: &mut usize,
    ) -> Result<()> {
        let mut values = BTreeSet::new();
        let mut contributions = BTreeSet::new();
        if let Some(prepared_values) = prepared_values {
            charge(work, prepared_values.len())?;
            values.extend(prepared_values.iter().cloned());
        }
        for template in templates.values() {
            for program in template.programs() {
                charge(work, program.effects.len() + 1)?;
                for effect in &program.effects {
                    match &effect.target {
                        BoundEffectTarget::Value { key } => {
                            values.insert(key.clone());
                        }
                        BoundEffectTarget::Contribution { key } => {
                            contributions.insert(key.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        for relation in &source_properties.relations {
            charge(work, 1 + relation.supports.len())?;
            values.insert(relation.count.clone());
            for program in relation.external.iter().chain(&relation.assembly).chain(
                relation
                    .supports
                    .values()
                    .flat_map(|support| &support.programs),
            ) {
                charge(work, program.program.effects.len() + 1)?;
                for effect in &program.program.effects {
                    match &effect.target {
                        BoundEffectTarget::Value { key } => {
                            values.insert(key.clone());
                        }
                        BoundEffectTarget::Contribution { key } => {
                            contributions.insert(key.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        charge(work, self.routes.len() + prefix.len())?;
        let routes: BTreeMap<_, _> = self.routes.iter().map(|(i, read)| (*i, read)).collect();
        for index in prefix {
            let node = &plan.effects[*index];
            charge(work, self.gates[*index].len())?;
            let mut reads: Vec<&PendingRead> = self.gates[*index].iter().collect();
            if let EffectOperation::Program { invocation, effect } = &node.operation {
                let read_indices = plan.invocations[*invocation]
                    .program
                    .effect_read_indices(*effect)?;
                charge(work, read_indices.len())?;
                reads.extend(
                    read_indices
                        .iter()
                        .map(|i| &self.invocations[*invocation][*i]),
                );
            }
            if let Some(read) = routes.get(index) {
                reads.push(read);
            }
            for read in reads {
                if touches(read, &values, &contributions, work)? {
                    return Err(invalid(
                        "preparation prefix depends on a potential support delivery channel",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(in crate::owned_plan) fn bind_suffix<'a, I>(
        &self,
        plan: &'a OwnedEffectPlan<I>,
        applications: &[RetainedApplication<'_>],
        prepared_outputs: &[PreparedTypeOutput],
        source_properties: &SourcePropertyAttempt<'_>,
        prefix: &BTreeSet<usize>,
        work: &mut usize,
    ) -> Result<SupportSuffix<'a, I>> {
        let limits = plan.limits;
        charge(
            work,
            plan.values.len()
                + self.contributions.len()
                + self.contributions.values().map(Vec::len).sum::<usize>(),
        )?;
        let mut suffix = SupportSuffix {
            base: plan,
            invocations: vec![],
            effects: vec![],
            invocation_overrides: BTreeMap::new(),
            effect_overrides: BTreeMap::new(),
            values: plan.values.clone(),
            order: vec![],
            query_gates: vec![],
        };
        let mut contributions = self.contributions.clone();
        let mut pending_invocations = Vec::new();
        let mut pending_gates = Vec::new();
        let mut seen = BTreeSet::new();
        for application in applications {
            charge(work, 1)?;
            if !seen.insert(application.key.clone()) {
                return Err(invalid("duplicate retained support application"));
            }
            if application.key.prepared.target != application.template.target
                || application.key.prepared.origin != application.template.origin
                || application.key.receiver != application.template.context.receiver
            {
                return Err(invalid(
                    "retained application differs from its bound template",
                ));
            }
            let applicability = PlanValueKey::SupportApplicability {
                application: Box::new(application.key.clone()),
            };
            let preparation_applicability = application.template.preparation.as_ref().map(|_| {
                PlanValueKey::SupportPreparationApplicability {
                    application: Box::new(application.key.clone()),
                }
            });
            let preparation_programs = application.template.preparation.iter().flat_map(|p| {
                let key = preparation_applicability
                    .as_ref()
                    .expect("preparation applicability key");
                std::iter::once((true, &p.applicability, key))
                    .chain(p.properties.iter().map(move |p| (false, p, key)))
            });
            for (is_applicability, program, applicability) in preparation_programs.chain(
                std::iter::once((true, &application.template.applicability, &applicability)).chain(
                    application
                        .template
                        .delivery
                        .iter()
                        .map(|p| (false, p, &applicability)),
                ),
            ) {
                if suffix.invocation_count() >= limits.max_invocations {
                    return Err(PlanError::Limit("invocations"));
                }
                charge(work, program.reads.len() + program.effects.len() + 1)?;
                let invocation = suffix.invocation_count();
                let key = ProgramOccurrenceKey {
                    origin: RuleOrigin::SupportApplication {
                        application: Box::new(application.key.clone()),
                    },
                    owner: application.template.owner.clone(),
                    program: program.program.clone(),
                    entity: match &application.key.receiver {
                        SupportReceiverKey::Actor(actor) => ConcreteEntity::Actor(actor.clone()),
                        SupportReceiverKey::Action(action) => {
                            ConcreteEntity::Action(action.clone())
                        }
                    },
                };
                suffix.invocations.push(Invocation {
                    key: key.clone(),
                    program: program.prepared.clone(),
                    reads: vec![],
                    read_ids: program.read_ids.clone(),
                });
                pending_invocations.push(&program.reads);
                for effect in &program.effects {
                    if suffix.effect_count() >= limits.max_effects {
                        return Err(PlanError::Limit("effects"));
                    }
                    charge(work, effect.gates.len() + 1)?;
                    let target = if is_applicability {
                        if !matches!(effect.target, BoundEffectTarget::Applicability) {
                            return Err(invalid(
                                "application program has a non-applicability effect",
                            ));
                        }
                        BoundEffectTarget::Value {
                            key: applicability.clone(),
                        }
                    } else {
                        effect.target.clone()
                    };
                    let index = suffix.effect_count();
                    match &target {
                        BoundEffectTarget::Value { key } => {
                            if suffix.values.insert(key.clone(), index).is_some() {
                                return Err(invalid(
                                    "support application introduces competing final producers",
                                ));
                            }
                        }
                        BoundEffectTarget::Contribution { key } => {
                            contributions.entry(key.clone()).or_default().push(index);
                        }
                        BoundEffectTarget::Requirement { .. } => {}
                        _ => return Err(invalid("unsupported support application effect target")),
                    }
                    suffix.effects.push(EffectNode {
                        key: EffectOccurrenceKey {
                            invocation: key.clone(),
                            effect: effect.id.clone(),
                        },
                        target,
                        operation: if is_applicability {
                            EffectOperation::SupportApplicability {
                                invocation,
                                effect: effect.effect_index,
                                eligible: application.eligible,
                            }
                        } else {
                            EffectOperation::Program {
                                invocation,
                                effect: effect.effect_index,
                            }
                        },
                        gates: vec![],
                        dependencies: vec![],
                    });
                    pending_gates.push((
                        &effect.gates,
                        (!is_applicability).then_some(applicability.clone()),
                    ));
                }
            }
        }
        for retained in &source_properties.programs {
            let bound = retained.template;
            let program = &bound.program;
            if suffix.invocation_count() >= limits.max_invocations {
                return Err(PlanError::Limit("invocations"));
            }
            charge(
                work,
                program.reads.len()
                    + program.effects.len()
                    + bound.producer.grant_path.len()
                    + target_copy_work(&retained.relation.owner)
                    + 2,
            )?;
            let invocation = suffix.invocation_count();
            let key = ProgramOccurrenceKey {
                origin: RuleOrigin::SourceProperty {
                    relation: retained.relation.id.clone(),
                    owner: Box::new(retained.relation.owner.clone()),
                    producer: bound.producer.clone(),
                    position: retained.position,
                },
                owner: bound.owner.clone(),
                program: program.program.clone(),
                entity: bound.entity.clone(),
            };
            suffix.invocations.push(Invocation {
                key: key.clone(),
                program: program.prepared.clone(),
                reads: vec![],
                read_ids: program.read_ids.clone(),
            });
            pending_invocations.push(&program.reads);
            for effect in &program.effects {
                if suffix.effect_count() >= limits.max_effects {
                    return Err(PlanError::Limit("effects"));
                }
                charge(work, effect.gates.len() + 1)?;
                let index = suffix.effect_count();
                match &effect.target {
                    BoundEffectTarget::Value { key } => {
                        if suffix.values.insert(key.clone(), index).is_some() {
                            return Err(invalid(
                                "source property introduces competing final producers",
                            ));
                        }
                    }
                    BoundEffectTarget::Contribution { key } => {
                        contributions.entry(key.clone()).or_default().push(index);
                    }
                    BoundEffectTarget::Requirement { .. } => {}
                    _ => return Err(invalid("unsupported source property effect target")),
                }
                suffix.effects.push(EffectNode {
                    key: EffectOccurrenceKey {
                        invocation: key.clone(),
                        effect: effect.id.clone(),
                    },
                    target: effect.target.clone(),
                    operation: EffectOperation::Program {
                        invocation,
                        effect: effect.effect_index,
                    },
                    gates: vec![],
                    dependencies: vec![],
                });
                pending_gates.push((&effect.gates, None));
            }
        }
        for output in &source_properties.counts {
            if suffix.effect_count() >= limits.max_effects {
                return Err(PlanError::Limit("effects"));
            }
            charge(work, target_copy_work(&output.relation.owner) + 1)?;
            if suffix
                .values
                .insert(output.relation.count.clone(), suffix.effect_count())
                .is_some()
            {
                return Err(invalid(
                    "source support count conflicts with another final producer",
                ));
            }
            let PlanValueKey::Stat { entity, stat } = &output.relation.count else {
                return Err(invalid("source support count must be a declared stat"));
            };
            suffix.effects.push(EffectNode {
                key: EffectOccurrenceKey {
                    invocation: ProgramOccurrenceKey {
                        origin: RuleOrigin::SourcePropertyCensus {
                            relation: output.relation.id.clone(),
                            owner: Box::new(output.relation.owner.clone()),
                        },
                        owner: SchemaSubject::Definition(stat.address()),
                        program: output.relation.census_stage.clone(),
                        entity: entity.clone(),
                    },
                    effect: output.relation.id.clone(),
                },
                target: BoundEffectTarget::Value {
                    key: output.relation.count.clone(),
                },
                operation: EffectOperation::SourcePropertyCount {
                    stage: output.relation.census_stage.clone(),
                    count: output.count,
                },
                gates: vec![],
                dependencies: vec![],
            });
        }
        // Native preparation is the only source of these scalar producers. The
        // private output rows retain their exact selection/receiving context.
        charge(work, prepared_outputs.len())?;
        for output in prepared_outputs {
            if suffix.effect_count() >= limits.max_effects {
                return Err(PlanError::Limit("effects"));
            }
            charge(
                work,
                output.context.copy_work() + 4 * target_copy_work(&output.context.target),
            )?;
            let entity = ConcreteEntity::Skill(Box::new(output.context.target.clone()));
            let value = PlanValueKey::Stat {
                entity: entity.clone(),
                stat: output.stat.clone(),
            };
            if suffix
                .values
                .insert(value.clone(), suffix.effect_count())
                .is_some()
            {
                return Err(invalid(
                    "prepared type conflicts with another final producer",
                ));
            }
            suffix.effects.push(EffectNode {
                key: EffectOccurrenceKey {
                    invocation: ProgramOccurrenceKey {
                        origin: RuleOrigin::SupportPreparation {
                            context: Box::new(output.context.clone()),
                        },
                        owner: SchemaSubject::Definition(output.stat.address()),
                        program: output.stage.clone(),
                        entity,
                    },
                    effect: output.support_type.clone(),
                },
                target: BoundEffectTarget::Value { key: value },
                operation: EffectOperation::PreparedSupportType {
                    stage: output.stage.clone(),
                    member: output.member,
                },
                gates: vec![],
                dependencies: vec![],
            });
        }
        // Even empty query gate rows require bounded iteration and allocation.
        charge(work, self.query_gates.len())?;
        let mut edges = 0;
        let ordered_base: Vec<_> = if plan.rules.input().contribution_queries.is_some() {
            charge(work, plan.effects.len())?;
            plan.effects.iter().map(|node| node.key.clone()).collect()
        } else {
            vec![]
        };
        let ordered_appended: Vec<_> = if plan.rules.input().contribution_queries.is_some() {
            charge(work, suffix.effects.len())?;
            suffix.effects.iter().map(|node| node.key.clone()).collect()
        } else {
            vec![]
        };
        let sources = FinalReadSources {
            values: &suffix.values,
            contributions: &contributions,
            transforms: &self.transforms,
            ordered: ordered::Sources {
                rules: plan.rules.input(),
                build: plan.request.build().input(),
                effects: &ordered_base,
                appended: &ordered_appended,
            },
        };
        sources
            .ordered
            .validate_inventory(&contributions, plan.complete, work)?;
        let mut resolve_reads = |reads: &[PendingRead]| -> Result<Vec<ReadBinding>> {
            reads
                .iter()
                .map(|r| {
                    resolve_ref(
                        r,
                        sources,
                        plan.complete,
                        work,
                        &mut edges,
                        limits.max_edges,
                    )
                })
                .collect()
        };
        for (index, reads) in self.invocations.iter().enumerate() {
            let bound = resolve_reads(reads)?;
            if bound != plan.invocations[index].reads {
                let mut replacement = plan.invocations[index].clone();
                replacement.reads = bound;
                suffix.invocation_overrides.insert(index, replacement);
            }
        }
        for (invocation, reads) in suffix.invocations.iter_mut().zip(pending_invocations) {
            invocation.reads = resolve_reads(reads)?;
        }
        for (index, gates) in self.gates.iter().enumerate() {
            let bound = resolve_reads(gates)?;
            if bound != plan.effects[index].gates {
                let mut replacement = plan.effects[index].clone();
                replacement.gates = bound;
                suffix.effect_overrides.insert(index, replacement);
            }
        }
        for (node, (gates, applicability)) in suffix.effects.iter_mut().zip(pending_gates) {
            node.gates = resolve_reads(gates)?;
            if let Some(key) = applicability {
                node.gates
                    .extend(resolve_reads(&[PendingRead::Value(key)])?);
            }
        }
        for (index, source) in &self.routes {
            let bound = resolve_reads(std::slice::from_ref(source))?.remove(0);
            let operation = match &plan.effects[*index].operation {
                EffectOperation::Route { .. } => EffectOperation::Route { source: bound },
                EffectOperation::SelectSource { .. } => {
                    EffectOperation::SelectSource { source: bound }
                }
                _ => return Err(invalid("symbolic route has a different operation")),
            };
            if operation != plan.effects[*index].operation {
                suffix
                    .effect_overrides
                    .entry(*index)
                    .or_insert_with(|| plan.effects[*index].clone())
                    .operation = operation;
            }
        }
        suffix.query_gates = self
            .query_gates
            .iter()
            .map(|gates| resolve_reads(gates))
            .collect::<Result<_>>()?;
        suffix.schedule(prefix, work)?;
        Ok(suffix)
    }
}

impl<I> SupportSuffix<'_, I> {
    pub(in crate::owned_plan) fn validate_stages(
        &self,
        stages: &poe_optimizer_data::owned_stages::OwnedEvaluationStages,
        work: &mut usize,
    ) -> Result<()> {
        charge(work, self.effect_count())?;
        let mut membership = Vec::with_capacity(self.effect_count());
        let mut has_inputs = false;
        for index in 0..self.effect_count() {
            let node = self.effect(index).expect("bound support effect");
            let stage = effect_stage(node, stages, work)?;
            if matches!(node.operation, EffectOperation::GeneratedInput { .. }) {
                // Only original literals sealed into the executed prefix have
                // intrinsic Structural readiness instead of an authored stage.
                if index >= self.base.effects.len() || self.effect_overrides.contains_key(&index) {
                    return Err(invalid("support binding changes a generated input"));
                }
                has_inputs = true;
            } else if stage.is_none() {
                return Err(invalid("support effect has no complete stage membership"));
            }
            membership.push(stage);
        }
        if has_inputs {
            charge(work, self.order.len())?;
            if self.order.iter().any(|index| {
                matches!(
                    self.effect(*index).expect("bound suffix effect").operation,
                    EffectOperation::GeneratedInput { .. }
                )
            }) {
                return Err(invalid(
                    "generated input is not frozen in the preparation prefix",
                ));
            }
        }
        for index in 0..self.effect_count() {
            let node = self.effect(index).expect("bound support effect");
            charge(work, node.dependencies.len())?;
            // Input parent gates were proved Structural and evaluated in the
            // immutable prefix. Ordinary consumers still need stage precedence
            // from every ordinary ancestor reached through those literal nodes.
            let Some(after) = membership[index] else {
                continue;
            };
            for dependency in &node.dependencies {
                let dependencies = crate::owned_plan::supports::input_dependencies(
                    |index| self.effect(index),
                    dependency,
                    work,
                )?;
                for &dependency in dependencies.as_ref() {
                    let before = membership[dependency].expect("ordinary dependency has a stage");
                    if before != after && !stages.precedes(before, after) {
                        return Err(invalid(
                            "support dependency crosses a stage backwards or without precedence",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    fn schedule(&mut self, prefix: &BTreeSet<usize>, work: &mut usize) -> Result<()> {
        let count = self.effect_count();
        charge(work, count)?;
        let mut outgoing = vec![Vec::new(); count];
        let mut remaining = vec![0usize; count];
        let mut edge_count = 0usize;
        for (index, remaining_count) in remaining.iter_mut().enumerate() {
            let node = self.effect(index).expect("bound effect index");
            let mut dependencies = BTreeSet::new();
            for gate in &node.gates {
                read_dependencies(gate, &mut dependencies, work)?;
            }
            match &node.operation {
                EffectOperation::ApplicationMaximum { candidates, .. } => {
                    charge(work, candidates.len())?;
                    dependencies.extend(candidates);
                }
                EffectOperation::PreparedSupportType { .. }
                | EffectOperation::SourcePropertyCount { .. }
                | EffectOperation::GeneratedInput { .. } => {}
                EffectOperation::Program { invocation, effect }
                | EffectOperation::SupportApplicability {
                    invocation, effect, ..
                } => {
                    let invocation = self
                        .invocation(*invocation)
                        .expect("bound invocation index");
                    for read in invocation.program.effect_read_indices(*effect)? {
                        read_dependencies(&invocation.reads[*read], &mut dependencies, work)?;
                    }
                }
                EffectOperation::Route { source } | EffectOperation::SelectSource { source } => {
                    read_dependencies(source, &mut dependencies, work)?
                }
            }
            edge_count = edge_count
                .checked_add(dependencies.len())
                .ok_or(PlanError::Limit("edges"))?;
            if edge_count > self.base.limits.max_edges {
                return Err(PlanError::Limit("edges"));
            }
            if prefix.contains(&index)
                && (dependencies.iter().any(|d| !prefix.contains(d))
                    || self.effect_overrides.contains_key(&index)
                    || match &node.operation {
                        EffectOperation::Program { invocation, .. } => {
                            self.invocation_overrides.contains_key(invocation)
                        }
                        _ => false,
                    })
            {
                return Err(invalid(
                    "support binding changes the executed preparation prefix",
                ));
            }
            for dependency in &dependencies {
                if *dependency >= count {
                    return Err(invalid("support dependency is out of bounds"));
                }
                outgoing[*dependency].push(index);
            }
            *remaining_count = dependencies.len();
            let dependencies: Vec<_> = dependencies.into_iter().collect();
            if dependencies != node.dependencies {
                if index < self.base.effects.len() {
                    self.effect_overrides
                        .entry(index)
                        .or_insert_with(|| self.base.effects[index].clone())
                        .dependencies = dependencies;
                } else {
                    self.effects[index - self.base.effects.len()].dependencies = dependencies;
                }
            }
        }
        let mut ready: BTreeSet<_> = remaining
            .iter()
            .enumerate()
            .filter_map(|(i, n)| (*n == 0).then_some(i))
            .collect();
        let mut visited = 0usize;
        while let Some(index) = ready.pop_first() {
            charge(work, outgoing[index].len() + 1)?;
            visited += 1;
            if !prefix.contains(&index) {
                self.order.push(index);
            }
            for next in &outgoing[index] {
                remaining[*next] -= 1;
                if remaining[*next] == 0 {
                    ready.insert(*next);
                }
            }
        }
        if visited != count {
            return Err(invalid("support effect dependency cycle"));
        }
        Ok(())
    }
}
