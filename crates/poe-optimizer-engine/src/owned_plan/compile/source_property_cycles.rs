//! A source program cannot hide a dependency cycle behind support selection.
//! This checks the existing symbolic channels; it neither executes programs nor
//! supplies numerical values for the native census or any missing producer.
use super::*;

enum Writer<'a> {
    Effect(&'a BoundEffectTarget),
    Count(&'a PlanValueKey),
}
struct Node<'a> {
    writer: Writer<'a>,
    reads: Vec<&'a PendingRead>,
    ready: Vec<&'a ReadBinding>,
    direct: &'a [usize],
    source: bool,
    stage: Option<&'a OwnedDefinitionKey>,
}
#[derive(Default)]
struct Writers<'a> {
    values: BTreeMap<&'a PlanValueKey, Vec<usize>>,
    contributions: BTreeMap<&'a ContributionKey, Vec<usize>>,
    transforms: BTreeMap<&'a PlanValueKey, Vec<usize>>,
}
impl<'a> Writers<'a> {
    fn insert(&mut self, index: usize, writer: &Writer<'a>) {
        match writer {
            Writer::Count(key) => {
                self.values.entry(key).or_default().push(index);
            }
            Writer::Effect(BoundEffectTarget::Value { key }) => {
                self.values.entry(key).or_default().push(index);
            }
            Writer::Effect(BoundEffectTarget::Contribution { key })
            | Writer::Effect(BoundEffectTarget::ApplicationCandidate { key, .. }) => {
                self.contributions.entry(key).or_default().push(index);
            }
            Writer::Effect(BoundEffectTarget::ModifierTransform { key, .. }) => {
                self.transforms.entry(key).or_default().push(index);
            }
            _ => {}
        }
    }
    fn extend(
        indices: Option<&Vec<usize>>,
        out: &mut BTreeSet<usize>,
        work: &mut usize,
    ) -> Result<()> {
        if let Some(indices) = indices {
            charge(work, indices.len())?;
            out.extend(indices);
        }
        Ok(())
    }
    fn dependencies(
        &self,
        read: &PendingRead,
        out: &mut BTreeSet<usize>,
        work: &mut usize,
    ) -> Result<()> {
        charge(work, 1)?;
        match read {
            PendingRead::Value(key) => Self::extend(self.values.get(key), out, work)?,
            PendingRead::Contributions(key, ..) => {
                Self::extend(self.contributions.get(key), out, work)?;
            }
            PendingRead::ModifierTransforms { key, initial } => {
                Self::extend(self.transforms.get(key), out, work)?;
                Self::extend(self.values.get(initial.as_ref()), out, work)?;
            }
            PendingRead::Select {
                decision,
                when_true,
                when_false,
            } => {
                out.insert(*decision);
                self.dependencies(when_true, out, work)?;
                self.dependencies(when_false, out, work)?;
            }
            PendingRead::Required(read) => self.dependencies(read, out, work)?,
            PendingRead::Ready(read) => read_dependencies(read, out, work)?,
        }
        Ok(())
    }
}

fn push<'a>(
    nodes: &mut Vec<Node<'a>>,
    node: Node<'a>,
    limits: PlanLimits,
    work: &mut usize,
) -> Result<()> {
    charge(
        work,
        1 + node.reads.len() + node.ready.len() + node.direct.len(),
    )?;
    if nodes.len() >= limits.max_effects {
        return Err(PlanError::Limit("potential source effects"));
    }
    nodes.push(node);
    Ok(())
}
fn program<'a>(
    nodes: &mut Vec<Node<'a>>,
    program: &'a BoundSupportProgram,
    source: bool,
    stage: Option<&'a OwnedDefinitionKey>,
    limits: PlanLimits,
    work: &mut usize,
) -> Result<()> {
    for effect in &program.effects {
        let indices = program.prepared.effect_read_indices(effect.effect_index)?;
        charge(work, indices.len() + effect.gates.len())?;
        let mut reads: Vec<_> = effect.gates.iter().collect();
        reads.extend(indices.iter().map(|index| &program.reads[*index]));
        push(
            nodes,
            Node {
                writer: Writer::Effect(&effect.target),
                reads,
                ready: vec![],
                direct: &[],
                source,
                stage,
            },
            limits,
            work,
        )?;
    }
    Ok(())
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn validate_source_cycles(
        &mut self,
        templates: &BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
        sources: &BoundSourceProperties,
    ) -> Result<()> {
        if sources.relations.is_empty() {
            return Ok(());
        }
        let stages = self.stages.ok_or_else(|| {
            PlanError::Invalid("potential source proof requires checked stages".into())
        })?;
        charge(&mut self.work, self.symbolic_routes.len())?;
        let routes: BTreeMap<_, _> = self
            .symbolic_routes
            .iter()
            .map(|(i, read)| (*i, read))
            .collect();
        let mut nodes = Vec::new();
        for (index, effect) in self.effects.iter().enumerate() {
            charge(&mut self.work, self.gates[index].len())?;
            let mut node = Node {
                writer: Writer::Effect(&effect.target),
                reads: self.gates[index].iter().collect(),
                ready: vec![],
                direct: &[],
                source: false,
                stage: effect_stage(effect, stages, &mut self.work)?,
            };
            match &effect.operation {
                EffectOperation::Program { invocation, effect }
                | EffectOperation::SupportApplicability {
                    invocation, effect, ..
                } => {
                    let indices = self.invocations[*invocation]
                        .program
                        .effect_read_indices(*effect)?;
                    charge(&mut self.work, indices.len())?;
                    node.reads
                        .extend(indices.iter().map(|read| &self.pending[*invocation][*read]));
                }
                EffectOperation::Route { source } | EffectOperation::SelectSource { source } => {
                    if let Some(read) = routes.get(&index) {
                        node.reads.push(read);
                    } else {
                        node.ready.push(source);
                    }
                }
                EffectOperation::ApplicationMaximum { candidates, .. } => node.direct = candidates,
                EffectOperation::PreparedSupportType { .. }
                | EffectOperation::SourcePropertyCount { .. }
                | EffectOperation::GeneratedInput { .. } => {}
            }
            push(&mut nodes, node, self.limits, &mut self.work)?;
        }
        // Preserve ordinary template semantics outside the dependency closure of
        // source properties. Every potential writer inside that closure matters,
        // including programs whose assignment will later lose selection.
        for template in templates.values() {
            for row in template.programs() {
                program(
                    &mut nodes,
                    row,
                    false,
                    stages.stage_for(&template.owner, &row.program),
                    self.limits,
                    &mut self.work,
                )?;
            }
        }
        for relation in &sources.relations {
            push(
                &mut nodes,
                Node {
                    writer: Writer::Count(&relation.count),
                    reads: vec![],
                    ready: vec![],
                    direct: &[],
                    source: true,
                    stage: Some(&relation.census_stage),
                },
                self.limits,
                &mut self.work,
            )?;
            for row in relation.programs() {
                program(
                    &mut nodes,
                    &row.program,
                    true,
                    stages.stage_for(&row.owner, &row.program.program),
                    self.limits,
                    &mut self.work,
                )?;
            }
        }
        prove(
            &nodes,
            self.effects.len(),
            self.limits,
            Some(stages),
            &mut self.work,
        )
    }
}

fn prove(
    nodes: &[Node<'_>],
    base_count: usize,
    limits: PlanLimits,
    stages: Option<&OwnedEvaluationStages>,
    work: &mut usize,
) -> Result<()> {
    charge(work, nodes.len())?;
    let mut writers = Writers::default();
    for (index, node) in nodes.iter().enumerate() {
        writers.insert(index, &node.writer);
    }
    let mut dependencies = Vec::with_capacity(nodes.len());
    let mut edges = 0usize;
    for node in nodes {
        let mut reads = BTreeSet::new();
        charge(work, node.direct.len())?;
        reads.extend(node.direct);
        for read in &node.ready {
            read_dependencies(read, &mut reads, work)?;
        }
        // Ready bindings and explicit decision indices refer only to the
        // immutable base graph, never to invented appended effect indices.
        if reads.iter().any(|index| *index >= base_count) {
            return Err(PlanError::Invalid(
                "invalid potential source base dependency".into(),
            ));
        }
        for read in &node.reads {
            writers.dependencies(read, &mut reads, work)?;
        }
        if reads.iter().any(|index| *index >= nodes.len()) {
            return Err(PlanError::Invalid(
                "invalid potential source dependency".into(),
            ));
        }
        edges = edges
            .checked_add(reads.len())
            .filter(|n| *n <= limits.max_edges)
            .ok_or(PlanError::Limit("potential source edges"))?;
        charge(work, reads.len() + 1)?;
        dependencies.push(reads);
    }
    let mut reachable = vec![false; nodes.len()];
    let mut pending: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| n.source.then_some(i))
        .collect();
    while let Some(index) = pending.pop() {
        charge(work, 1)?;
        if std::mem::replace(&mut reachable[index], true) {
            continue;
        }
        charge(work, dependencies[index].len())?;
        pending.extend(&dependencies[index]);
    }
    charge(work, nodes.len())?;
    let mut outgoing = vec![vec![]; nodes.len()];
    let mut remaining = vec![0; nodes.len()];
    for (index, reads) in dependencies.iter().enumerate() {
        if reachable[index] {
            charge(work, reads.len())?;
            remaining[index] = reads.len();
            for dependency in reads {
                if let Some(stages) = stages {
                    let before = nodes[*dependency].stage.ok_or_else(|| {
                        PlanError::Invalid("potential source dependency has no stage".into())
                    })?;
                    let after = nodes[index].stage.ok_or_else(|| {
                        PlanError::Invalid("potential source consumer has no stage".into())
                    })?;
                    if before != after && !stages.precedes(before, after) {
                        return Err(PlanError::Invalid(
                            "potential source dependency crosses a stage backwards or without precedence".into(),
                        ));
                    }
                }
                outgoing[*dependency].push(index);
            }
        }
    }
    if topological_order(&outgoing, remaining, work)?.1.is_some() {
        return Err(PlanError::Invalid(
            "potential source-property dependency cycle".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::owned_plan::compile::support_fixture as fixture;

    fn channel(name: &str) -> ContributionKey {
        ContributionKey {
            entity: ConcreteEntity::Actor(ActorKey::Player),
            stat: fixture::def(name),
            kind: ContributionKind::Add,
        }
    }
    fn read(key: &ContributionKey) -> PendingRead {
        PendingRead::Contributions(
            key.clone(),
            ContributionReduction::Sum,
            ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
        )
    }
    fn node<'a>(
        target: &'a BoundEffectTarget,
        reads: Vec<&'a PendingRead>,
        source: bool,
    ) -> Node<'a> {
        Node {
            writer: Writer::Effect(target),
            reads,
            ready: vec![],
            direct: &[],
            source,
            stage: None,
        }
    }

    #[test]
    fn potential_source_cycles_follow_ordinary_contributors_and_false_branches() {
        let a = channel("source-a");
        let b = channel("source-b");
        let outputs = [
            BoundEffectTarget::Contribution { key: a.clone() },
            BoundEffectTarget::Contribution { key: b.clone() },
        ];
        let a_read = read(&a);
        let b_read = read(&b);
        for source_flags in [[true, true], [true, false]] {
            let nodes = [
                node(&outputs[0], vec![&b_read], source_flags[0]),
                node(&outputs[1], vec![&a_read], source_flags[1]),
            ];
            assert!(
                matches!(prove(&nodes, 0, PlanLimits::default(), None, &mut 1000), Err(PlanError::Invalid(message)) if message.contains("dependency cycle"))
            );
        }
        let hidden = PendingRead::Select {
            decision: 0,
            when_true: Box::new(PendingRead::Ready(ReadBinding::Constant(Some(
                ParameterValue::Boolean(true),
            )))),
            when_false: Box::new(b_read),
        };
        let decision = BoundEffectTarget::Requirement {
            code: fixture::key("decision"),
        };
        let nodes = [
            node(&decision, vec![], false),
            node(&outputs[0], vec![&hidden], true),
            node(&outputs[1], vec![&a_read], true),
        ];
        assert!(
            matches!(prove(&nodes, 1, PlanLimits::default(), None, &mut 1000), Err(PlanError::Invalid(message)) if message.contains("dependency cycle"))
        );
    }

    #[test]
    fn acyclic_contribution_chains_are_valid_and_failed_budget_has_no_state() {
        let a = channel("source-a");
        let b = channel("source-b");
        let outputs = [
            BoundEffectTarget::Contribution { key: a.clone() },
            BoundEffectTarget::Contribution { key: b },
        ];
        let a_read = read(&a);
        let nodes = [
            node(&outputs[0], vec![], true),
            node(&outputs[1], vec![&a_read], true),
        ];
        assert!(matches!(
            prove(&nodes, 0, PlanLimits::default(), None, &mut 1),
            Err(PlanError::Limit(_))
        ));
        prove(&nodes, 0, PlanLimits::default(), None, &mut 1000).unwrap();
        let limits = PlanLimits {
            max_edges: 0,
            ..PlanLimits::default()
        };
        assert!(matches!(
            prove(&nodes, 0, limits, None, &mut 1000),
            Err(PlanError::Limit(_))
        ));
    }

    #[test]
    fn unrelated_legacy_potential_cycles_remain_outside_the_new_proof() {
        let a = channel("legacy-a");
        let b = channel("legacy-b");
        let outputs = [
            BoundEffectTarget::Contribution { key: a.clone() },
            BoundEffectTarget::Contribution { key: b.clone() },
        ];
        let reads = [read(&a), read(&b)];
        let count = PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(ActorKey::Player),
            stat: fixture::def("source-count"),
        };
        let nodes = [
            node(&outputs[0], vec![&reads[1]], false),
            node(&outputs[1], vec![&reads[0]], false),
            Node {
                writer: Writer::Count(&count),
                reads: vec![],
                ready: vec![],
                direct: &[],
                source: true,
                stage: None,
            },
        ];
        prove(&nodes, 0, PlanLimits::default(), None, &mut 1000).unwrap();
    }

    #[test]
    fn unselected_source_reads_cannot_cross_a_later_or_incomparable_stage() {
        use poe_optimizer_core::owned_stages::EvaluationStage;
        use poe_optimizer_data::{
            owned_rules::{OwnedRulePackage, RuleStorageLimits},
            owned_stages::StageStorageLimits,
        };
        let f = fixture::generated_fixture();
        let inputs = fixture::compile_inputs(&f, fixture::target(30, "first"));
        // Stages bind the original storage package, not the compiler's separately
        // canonicalized executable declaration order.
        let stored = OwnedRulePackage::new(
            fixture::raw_rules(
                &f,
                &inputs.definitions,
                poe_optimizer_core::owned_rules::OWNED_RULE_OPERATIONS_V12,
            ),
            inputs.definitions.as_ref(),
            RuleStorageLimits::default(),
        )
        .unwrap();
        assert_eq!(Some(*stored.identity()), inputs.rules.source_identity());
        let early = fixture::key("prepare");
        let late = fixture::key("later");
        let independent = fixture::key("independent");
        let mut input = inputs.stages.input().clone();
        input.stages.extend([
            EvaluationStage {
                id: late.clone(),
                predecessors: vec![early.clone()],
            },
            EvaluationStage {
                id: independent.clone(),
                predecessors: vec![],
            },
        ]);
        let stages = OwnedEvaluationStages::new(
            input,
            inputs.definitions.as_ref(),
            &stored,
            &inputs.routing,
            StageStorageLimits::default(),
        )
        .unwrap();
        let final_stat = PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(ActorKey::Player),
            stat: fixture::def("assembly-final"),
        };
        let outputs = [
            BoundEffectTarget::Requirement {
                code: fixture::key("false-condition"),
            },
            BoundEffectTarget::Value {
                key: final_stat.clone(),
            },
            BoundEffectTarget::Contribution {
                key: channel("unused-property"),
            },
        ];
        let hidden = PendingRead::Select {
            decision: 0,
            when_true: Box::new(PendingRead::Ready(ReadBinding::Constant(Some(
                ParameterValue::Boolean(false),
            )))),
            when_false: Box::new(PendingRead::Value(final_stat)),
        };
        for (producer, consumer, accepted) in [
            (&late, &early, false),
            (&independent, &early, false),
            (&early, &early, true),
            (&early, &late, true),
        ] {
            let mut nodes = [
                node(&outputs[0], vec![], false),
                node(&outputs[1], vec![], false),
                node(&outputs[2], vec![&hidden], true),
            ];
            nodes[0].stage = Some(&early);
            nodes[1].stage = Some(producer);
            nodes[2].stage = Some(consumer);
            let result = prove(&nodes, 1, PlanLimits::default(), Some(&stages), &mut 1000);
            if accepted {
                result.unwrap();
            } else {
                assert!(
                    matches!(result, Err(PlanError::Invalid(message)) if message.contains("crosses a stage"))
                );
            }
        }
    }
}
