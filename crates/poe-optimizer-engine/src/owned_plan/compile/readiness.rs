//! Cold readiness proof over the same concrete occurrences and symbolic channels.
//! Scheduling declarations never erase a real read, grant, or coverage gap.
use super::*;
use crate::owned_plan::support_outputs::target_copy_work;
#[cfg(test)]
#[path = "readiness_tests.rs"]
mod tests;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct GateContextKey {
    provider: Option<ProviderKey>,
    actor: ActorKey,
    skill: Option<GeneratedSkillKey>,
    entity: ConcreteEntity,
    phase: ReadinessPhase,
}

pub(super) fn validate_bindings<I: DefinitionSchemaIndex>(
    stages: Option<&OwnedEvaluationStages>,
    definitions: &I,
    rules: &CompiledRulePackage,
    routing: &OwnedActionRouting,
    operations: RuleOperationsVersion,
) -> Result<()> {
    if operations.supports_readiness()
        && stages.and_then(OwnedEvaluationStages::readiness).is_none()
    {
        return Err(PlanError::Invalid(
            "operation v16 requires checked readiness stages".into(),
        ));
    }
    if let Some(stages) = stages {
        let input = stages.input();
        if input.definitions != *definitions.identity()
            || input.namespace != *definitions.namespace()
            || Some(input.rules) != rules.source_identity()
            || input.routing != *routing.identity()
            || stages.readiness().is_some() != operations.supports_readiness()
        {
            return Err(PlanError::Invalid("readiness stage bindings differ".into()));
        }
    }
    Ok(())
}

fn program_phase(
    stages: &OwnedEvaluationStages,
    owner: &SchemaSubject,
    program: &OwnedDefinitionKey,
) -> ReadinessPhase {
    stages
        .program_readiness(owner, program)
        .map_or(ReadinessPhase::Execution, |r| r.phase)
}

fn effect_phase(stages: &OwnedEvaluationStages, node: &EffectNode) -> ReadinessPhase {
    match (&node.operation, &node.key.invocation.origin) {
        (EffectOperation::GeneratedInput { .. }, _) => ReadinessPhase::Structural,
        (_, RuleOrigin::EffectApplication { .. }) => ReadinessPhase::Execution,
        (EffectOperation::Program { .. } | EffectOperation::SupportApplicability { .. }, _) => {
            program_phase(
                stages,
                &node.key.invocation.owner,
                &node.key.invocation.program,
            )
        }
        _ => ReadinessPhase::Execution,
    }
}
fn actor_work(actor: &ActorKey) -> usize {
    match actor {
        ActorKey::Player => 1,
        ActorKey::Owned(actor) => actor.provider.grant_path.len() + 2,
    }
}
fn entity_work(entity: &ConcreteEntity) -> usize {
    match entity {
        ConcreteEntity::Actor(actor) => actor_work(actor),
        ConcreteEntity::Action(action) => {
            action.action.provider.grant_path.len() + actor_work(&action.action.actor) + 5
        }
        ConcreteEntity::Modifier(provider) => provider.grant_path.len() + 1,
        ConcreteEntity::Skill(skill) => target_copy_work(skill),
        _ => 1,
    }
}
fn value_work(key: &PlanValueKey) -> usize {
    match key {
        PlanValueKey::Stat { entity, .. } | PlanValueKey::Capability { entity, .. } => {
            entity_work(entity) + 1
        }
        PlanValueKey::Grant { provider, .. } => provider.grant_path.len() + 2,
        PlanValueKey::SkillParameter { skill, .. } => skill.provider.grant_path.len() + 3,
        // Application-private applicability keys do not exist until suffix binding.
        PlanValueKey::SupportApplicability { .. }
        | PlanValueKey::SupportPreparationApplicability { .. } => 1,
    }
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn index_readiness_topology(&mut self) -> Result<()> {
        if self
            .stages
            .and_then(OwnedEvaluationStages::readiness)
            .is_some()
        {
            for (skill, provider) in &self.skill_supplies {
                charge(
                    &mut self.work,
                    skill.provider.grant_path.len() + provider.grant_path.len() + 1,
                )?;
                if self
                    .readiness_skills
                    .insert(provider.clone(), skill.clone())
                    .is_some()
                {
                    return Err(PlanError::Invalid(
                        "competing generated skills at one provider occurrence".into(),
                    ));
                }
            }
        }
        Ok(())
    }
    pub(super) fn program_gates(
        &mut self,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
        context: &Context,
    ) -> Result<Vec<PendingRead>> {
        let phase = self
            .stages
            .filter(|s| s.readiness().is_some())
            .map_or(ReadinessPhase::Execution, |s| {
                program_phase(s, owner, program)
            });
        self.context_gates_at(context, phase)
    }

    pub(super) fn context_gates_at(
        &mut self,
        context: &Context,
        phase: ReadinessPhase,
    ) -> Result<Vec<PendingRead>> {
        if self
            .stages
            .and_then(OwnedEvaluationStages::readiness)
            .is_none()
        {
            // Preserve both historical gating and historical bounded-work accounting.
            return self.uncached_context_gates(context, ReadinessPhase::Execution);
        }
        let copy_work = context.provider.as_ref().map_or(0, |p| p.grant_path.len())
            + context
                .skill
                .as_ref()
                .map_or(0, |s| s.provider.grant_path.len())
            + actor_work(&context.actor)
            + entity_work(&context.entity)
            + 4;
        charge(&mut self.work, copy_work)?;
        let key = GateContextKey {
            provider: context.provider.clone(),
            actor: context.actor.clone(),
            skill: context.skill.clone(),
            entity: context.entity.clone(),
            phase,
        };
        if let Some(gates) = self.readiness_gates.get(&key) {
            charge(
                &mut self.work,
                gates
                    .len()
                    .checked_mul(copy_work)
                    .ok_or(PlanError::Limit("readiness gate copies"))?,
            )?;
            return Ok(gates.clone());
        }
        if self.readiness_gates.len() >= self.limits.max_edges {
            return Err(PlanError::Limit("readiness gate contexts"));
        }
        let gates = self.uncached_context_gates(context, phase)?;
        charge(
            &mut self.work,
            gates
                .len()
                .checked_mul(copy_work)
                .ok_or(PlanError::Limit("readiness gate copies"))?,
        )?;
        self.readiness_gates.insert(key, gates.clone());
        Ok(gates)
    }

    pub(super) fn validate_readiness(
        &mut self,
        templates: &BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
        sources: &BoundSourceProperties,
        target_gates: &BTreeMap<SkillTarget, Vec<PendingRead>>,
    ) -> Result<()> {
        let Some(stages) = self.stages.filter(|s| s.readiness().is_some()) else {
            return Ok(());
        };
        charge(&mut self.work, self.effects.len())?;
        let mut proof = ReadinessProof {
            stages,
            index: self.index,
            values: BTreeMap::new(),
            contributions: BTreeMap::new(),
            transforms: BTreeMap::new(),
            effects: Vec::with_capacity(self.effects.len()),
            work: &mut self.work,
        };
        for node in &self.effects {
            charge(proof.work, 1)?;
            let phase = effect_phase(stages, node);
            proof.effects.push(phase);
            proof.writer(&node.target, phase)?;
        }
        // Every potential receiving program participates, including unselected
        // supports. Otherwise an empty cold reduction would conceal late writes.
        for template in templates.values() {
            for program in template.programs() {
                let phase = program_phase(stages, &template.owner, &program.program);
                for effect in &program.effects {
                    proof.writer(&effect.target, phase)?;
                }
            }
        }
        for relation in &sources.relations {
            proof.writer(
                &BoundEffectTarget::Value {
                    key: relation.count.clone(),
                },
                ReadinessPhase::Preparation,
            )?;
            for template in relation.programs() {
                let phase = program_phase(stages, &template.owner, &template.program.program);
                for effect in &template.program.effects {
                    proof.writer(&effect.target, phase)?;
                }
            }
        }
        proof.inputs(
            &self.invocations,
            &self.pending,
            &self.gates,
            templates,
            sources,
            target_gates,
        )
    }
}

impl SymbolicBindings {
    /// Native final-type outputs are produced after admission, not structural
    /// input facts. Their exact keys join the same cold readiness proof. Output
    /// stage ordering and the pre-admission prefix prohibition remain separate.
    pub(in crate::owned_plan) fn validate_output_readiness<I: DefinitionSchemaIndex>(
        &self,
        plan: &OwnedEffectPlan<I>,
        stages: &OwnedEvaluationStages,
        templates: &BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
        sources: &BoundSourceProperties,
        outputs: &BTreeSet<PlanValueKey>,
        work: &mut usize,
    ) -> Result<()> {
        if stages.readiness().is_none() {
            return Ok(());
        }
        charge(work, plan.effects.len() + outputs.len())?;
        let mut proof = ReadinessProof {
            stages,
            index: plan.definitions.as_ref(),
            values: BTreeMap::new(),
            contributions: BTreeMap::new(),
            transforms: BTreeMap::new(),
            effects: plan
                .effects
                .iter()
                .map(|node| effect_phase(stages, node))
                .collect(),
            work,
        };
        for key in outputs {
            charge(proof.work, value_work(key))?;
            if let PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(target),
                stat,
            } = key
                && participation::participation_stat(
                    target,
                    &plan.request,
                    plan.definitions.as_ref(),
                    stages,
                    proof.work,
                )? == Some(stat)
            {
                return Err(PlanError::Invalid(
                    "skill participation cannot be supplied by native support output".into(),
                ));
            }
            proof
                .values
                .insert(key.clone(), ReadinessPhase::Preparation);
        }
        proof.inputs(
            &plan.invocations,
            &self.invocations,
            &self.gates,
            templates,
            sources,
            &self.preparation_gates,
        )
    }
}

struct ReadinessProof<'a, I> {
    stages: &'a OwnedEvaluationStages,
    index: &'a I,
    // The latest potential writer is sufficient to reject every late writer.
    values: BTreeMap<PlanValueKey, ReadinessPhase>,
    contributions: BTreeMap<ContributionKey, ReadinessPhase>,
    transforms: BTreeMap<PlanValueKey, ReadinessPhase>,
    effects: Vec<ReadinessPhase>,
    work: &'a mut usize,
}
impl<I: DefinitionSchemaIndex> ReadinessProof<'_, I> {
    fn inputs(
        &mut self,
        invocations: &[Invocation],
        reads: &[Vec<PendingRead>],
        gates: &[Vec<PendingRead>],
        templates: &BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
        sources: &BoundSourceProperties,
        target_gates: &BTreeMap<SkillTarget, Vec<PendingRead>>,
    ) -> Result<()> {
        if invocations.len() != reads.len() || gates.len() != self.effects.len() {
            return Err(PlanError::Invalid(
                "readiness symbolic binding counts differ".into(),
            ));
        }
        charge(
            self.work,
            invocations.len() + gates.len() + templates.len() + target_gates.len(),
        )?;
        for (invocation, reads) in invocations.iter().zip(reads) {
            let phase = if matches!(invocation.key.origin, RuleOrigin::EffectApplication { .. }) {
                ReadinessPhase::Execution
            } else {
                program_phase(self.stages, &invocation.key.owner, &invocation.key.program)
            };
            // All declared reads are checked, not only a selected lazy branch or
            // the reads needed by the first effect in a program.
            for read in reads {
                self.read(read, phase)?;
            }
        }
        for (index, gates) in gates.iter().enumerate() {
            let phase = self.effects[index];
            for gate in gates {
                self.read(gate, phase)?;
            }
        }
        for template in templates.values() {
            for program in template.programs() {
                let phase = program_phase(self.stages, &template.owner, &program.program);
                for read in &program.reads {
                    self.read(read, phase)?;
                }
                for effect in &program.effects {
                    for gate in &effect.gates {
                        self.read(gate, phase)?;
                    }
                }
            }
        }
        for gates in target_gates.values() {
            for gate in gates {
                self.read(gate, ReadinessPhase::Preparation)?;
            }
        }
        for relation in &sources.relations {
            charge(self.work, 1)?;
            for template in relation.programs() {
                let phase = program_phase(self.stages, &template.owner, &template.program.program);
                for read in &template.program.reads {
                    self.read(read, phase)?;
                }
                for effect in &template.program.effects {
                    for gate in &effect.gates {
                        self.read(gate, phase)?;
                    }
                }
            }
        }
        Ok(())
    }
    fn require(&mut self, producer: ReadinessPhase, consumer: ReadinessPhase) -> Result<()> {
        charge(self.work, 1)?;
        if producer > consumer {
            return Err(PlanError::Invalid(
                "early readiness depends on a later input or producer".into(),
            ));
        }
        Ok(())
    }
    fn writer(&mut self, target: &BoundEffectTarget, phase: ReadinessPhase) -> Result<()> {
        charge(self.work, 1)?;
        match target {
            BoundEffectTarget::Value { key } => {
                charge(self.work, value_work(key))?;
                if let Some(previous) = self.values.insert(key.clone(), phase)
                    && (previous < ReadinessPhase::Execution || phase < ReadinessPhase::Execution)
                {
                    return Err(PlanError::Invalid(
                        "readiness has competing potential final producers".into(),
                    ));
                }
            }
            BoundEffectTarget::ModifierTransform { key, .. } => {
                charge(self.work, value_work(key))?;
                self.transforms
                    .entry(key.clone())
                    .and_modify(|p| *p = (*p).max(phase))
                    .or_insert(phase);
            }
            BoundEffectTarget::Contribution { key }
            | BoundEffectTarget::ApplicationCandidate { key, .. } => {
                charge(self.work, entity_work(&key.entity) + 1)?;
                self.contributions
                    .entry(key.clone())
                    .and_modify(|p| *p = (*p).max(phase))
                    .or_insert(phase);
            }
            _ => {}
        }
        Ok(())
    }
    fn value(&mut self, key: &PlanValueKey, phase: ReadinessPhase) -> Result<()> {
        charge(self.work, value_work(key))?;
        // Unclassified Skills keep every historical required gate. Their actual
        // producers determine whether those full inputs can exist early.
        if let PlanValueKey::SkillParameter { skill, parameter } = key
            && let SchemaLookup::Known(grant) = self.index.slot(&skill.slot)
            && let Some(required) = self.stages.parameter_phase(&grant.skill, parameter)
        {
            self.require(required, phase)?;
        }
        if let Some(producer) = self.values.get(key) {
            self.require(*producer, phase)?;
        }
        Ok(())
    }
    fn effect(&mut self, index: usize, phase: ReadinessPhase) -> Result<()> {
        let producer = *self
            .effects
            .get(index)
            .ok_or_else(|| PlanError::Invalid("unknown readiness dependency".into()))?;
        self.require(producer, phase)
    }
    fn read(&mut self, read: &PendingRead, phase: ReadinessPhase) -> Result<()> {
        charge(self.work, 1)?;
        if phase == ReadinessPhase::Execution {
            return Ok(());
        }
        match read {
            PendingRead::Value(key) => self.value(key, phase)?,
            PendingRead::Contributions(key, ..) | PendingRead::OrderedContributions(key, ..) => {
                charge(self.work, entity_work(&key.entity) + 1)?;
                if let Some(producer) = self.contributions.get(key) {
                    self.require(*producer, phase)?;
                }
            }
            PendingRead::Select {
                decision,
                when_true,
                when_false,
            } => {
                self.effect(*decision, phase)?;
                self.read(when_true, phase)?;
                self.read(when_false, phase)?;
            }
            PendingRead::Required(source) => self.read(source, phase)?,
            PendingRead::ModifierTransforms { key, initial } => {
                charge(self.work, value_work(key))?;
                if let Some(producer) = self.transforms.get(key) {
                    self.require(*producer, phase)?;
                }
                self.value(initial, phase)?;
            }
            PendingRead::Ready(binding) => {
                let mut dependencies = BTreeSet::new();
                read_dependencies(binding, &mut dependencies, self.work)?;
                for dependency in dependencies {
                    self.effect(dependency, phase)?;
                }
            }
        }
        Ok(())
    }
}
