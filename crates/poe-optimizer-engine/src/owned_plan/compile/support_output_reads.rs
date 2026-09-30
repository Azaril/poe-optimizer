//! Cold exact-target demand discovery for native preparation output channels.
use super::*;

struct OutputReads<'a> {
    stats: &'a BTreeSet<StatDefId>,
    targets: BTreeSet<SkillTarget>,
    limits: PlanLimits,
    work: &'a mut usize,
}
impl OutputReads<'_> {
    fn value(&mut self, key: &PlanValueKey) -> Result<()> {
        charge(self.work, 1)?;
        let PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(target),
            stat,
        } = key
        else {
            return Ok(());
        };
        if !self.stats.contains(stat) {
            return Ok(());
        }
        let depth = match target.as_ref() {
            SkillTarget::Authored(_) => 0,
            SkillTarget::Generated(skill) => skill.provider.grant_path.len(),
        };
        charge(self.work, depth + 1)?;
        if !self.targets.contains(target.as_ref()) {
            if self.targets.len() >= self.limits.max_providers {
                return Err(PlanError::Limit("support output targets"));
            }
            self.targets.insert(target.as_ref().clone());
        }
        Ok(())
    }
    fn read(&mut self, read: &PendingRead) -> Result<()> {
        charge(self.work, 1)?;
        match read {
            PendingRead::Value(key) => self.value(key),
            PendingRead::Required(source) => self.read(source),
            PendingRead::Select {
                when_true,
                when_false,
                ..
            } => {
                // Both potential branches are part of the cold dependency proof.
                self.read(when_true)?;
                self.read(when_false)
            }
            PendingRead::ModifierTransforms { key, initial } => {
                self.value(key)?;
                self.value(initial)
            }
            // A contribution channel is not a final-value demand. Ready values
            // have no semantic key to fabricate a preparation target from.
            PendingRead::Ready(_) | PendingRead::Contributions(..) => Ok(()),
        }
    }
    fn reads(&mut self, reads: &[PendingRead]) -> Result<()> {
        charge(self.work, reads.len())?;
        for read in reads {
            self.read(read)?;
        }
        Ok(())
    }
    fn program_reads(
        &mut self,
        program: &PreparedRuleProgram,
        effect: usize,
        reads: &[PendingRead],
    ) -> Result<()> {
        let indices = program.effect_read_indices(effect)?;
        charge(self.work, indices.len())?;
        for index in indices {
            let read = reads
                .get(*index)
                .ok_or_else(|| PlanError::Invalid("support output read binding differs".into()))?;
            self.read(read)?;
        }
        Ok(())
    }
}

impl SymbolicBindings {
    /// Enumerate consumers, not the global preparation-gate inventory. Unknown
    /// targets are returned for the driver to reject against its exact topology.
    /// Potential support templates participate before selection or false gates.
    pub(in crate::owned_plan) fn support_output_targets<I>(
        &self,
        plan: &OwnedEffectPlan<I>,
        templates: &BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>,
        output_stats: &BTreeSet<StatDefId>,
        work: &mut usize,
    ) -> Result<BTreeSet<SkillTarget>> {
        if self.gates.len() != plan.effects.len()
            || self.invocations.len() != plan.invocations.len()
            || self.query_gates.len() != plan.query_gates.len()
        {
            return Err(PlanError::Invalid(
                "support output symbolic bindings differ".into(),
            ));
        }
        charge(
            work,
            plan.effects.len() + self.routes.len() + self.query_gates.len() + templates.len(),
        )?;
        let mut scan = OutputReads {
            stats: output_stats,
            targets: BTreeSet::new(),
            limits: plan.limits,
            work,
        };
        for (index, node) in plan.effects.iter().enumerate() {
            scan.reads(&self.gates[index])?;
            if let EffectOperation::Program { invocation, effect }
            | EffectOperation::SupportApplicability {
                invocation, effect, ..
            } = &node.operation
            {
                let program = plan.invocations.get(*invocation).ok_or_else(|| {
                    PlanError::Invalid("support output invocation is absent".into())
                })?;
                scan.program_reads(&program.program, *effect, &self.invocations[*invocation])?;
            }
        }
        for (index, read) in &self.routes {
            if *index >= plan.effects.len() {
                return Err(PlanError::Invalid("support output route is absent".into()));
            }
            scan.read(read)?;
        }
        for gates in &self.query_gates {
            scan.reads(gates)?;
        }
        for template in templates.values() {
            charge(scan.work, template.delivery.len() + 1)?;
            for program in std::iter::once(&template.applicability).chain(&template.delivery) {
                charge(scan.work, program.effects.len())?;
                for effect in &program.effects {
                    scan.reads(&effect.gates)?;
                    scan.program_reads(&program.prepared, effect.effect_index, &program.reads)?;
                }
            }
        }
        Ok(scan.targets)
    }
}
