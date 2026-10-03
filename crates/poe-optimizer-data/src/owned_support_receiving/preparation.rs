//! V2 receiving applications use distinct preparation and execution producers.
use super::*;
use poe_optimizer_core::owned_readiness::{ReadinessPhase, ReadinessProgramRole};
impl<I: DefinitionSchemaIndex> Check<'_, I> {
    pub(super) fn readiness_role(
        &self,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
        role: ReadinessProgramRole,
    ) -> Result<()> {
        let declaration = self
            .stages
            .program_readiness(owner, program)
            .ok_or_else(|| invalid("receiving program lacks readiness classification"))?;
        if declaration.role != role
            || (role == ReadinessProgramRole::Execution
                && declaration.phase != ReadinessPhase::Execution)
        {
            return Err(invalid("receiving program readiness role mismatch"));
        }
        Ok(())
    }
    pub(super) fn preparation_programs(
        &mut self,
        owner: &SchemaSubject,
        row: &SupportRolePrograms,
        kind: SupportReceiverKind,
        programs: &BTreeMap<&OwnedDefinitionKey, &RuleProgram>,
    ) -> Result<()> {
        let Some(preparation) = &row.preparation else {
            return Ok(());
        };
        if preparation.properties.is_empty() {
            return Err(invalid("preparation application must declare properties"));
        }
        let context = match kind {
            SupportReceiverKind::Actor => RuleEntityKind::Actor,
            SupportReceiverKind::Action => RuleEntityKind::Action,
        };
        self.readiness_role(
            owner,
            &preparation.applicability,
            ReadinessProgramRole::SupportPreparationApplicability,
        )?;
        let application = programs
            .get(&preparation.applicability)
            .ok_or_else(|| invalid("unknown preparation applicability program"))?;
        self.used.work(
            application.effects.len() + application.reads.len() + 1,
            self.l,
        )?;
        self.receiving_reads(application)?;
        if application.context != context {
            return Err(invalid(
                "preparation applicability scope differs from receiver",
            ));
        }
        let application_stage = self
            .stages
            .stage_for(owner, &preparation.applicability)
            .ok_or_else(|| invalid("preparation applicability lacks stage"))?;
        if !self
            .stages
            .precedes(self.preparation_stage, application_stage)
        {
            return Err(invalid("preparation applicability must follow admission"));
        }
        let mut seen = BTreeSet::new();
        seen.insert(&row.applicability);
        seen.extend(&row.delivery);
        if !seen.insert(&preparation.applicability) {
            return Err(invalid(
                "preparation applicability overlaps execution programs",
            ));
        }
        for key in &preparation.properties {
            if !seen.insert(key) {
                return Err(invalid(
                    "preparation property overlaps another receiving role",
                ));
            }
            self.readiness_role(
                owner,
                key,
                ReadinessProgramRole::SupportedPreparationProperty,
            )?;
            let program = programs
                .get(key)
                .ok_or_else(|| invalid("unknown preparation property program"))?;
            self.used
                .work(program.effects.len() + program.reads.len() + 1, self.l)?;
            self.receiving_reads(program)?;
            if program.context != context {
                return Err(invalid("preparation property scope differs from receiver"));
            }
            let stage = self
                .stages
                .stage_for(owner, key)
                .ok_or_else(|| invalid("preparation property lacks stage"))?;
            if !self.stages.precedes(application_stage, stage) {
                return Err(invalid(
                    "preparation property must follow its applicability",
                ));
            }
        }
        Ok(())
    }
}
