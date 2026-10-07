//! V3 source relations retain producer authority and require finite closure.
use super::*;
use poe_optimizer_core::{
    owned_readiness::{ReadinessPhase, ReadinessProgramRole},
    owned_source_properties::*,
};

fn complete<T>(set: &DeclaredSet<T>) -> Result<()> {
    if !set.is_complete() {
        return Err(invalid("source property inventories must be Complete"));
    }
    Ok(())
}
fn unique<T: Ord>(values: &mut [T]) -> Result<()> {
    values.sort();
    if values.windows(2).any(|v| v[0] == v[1]) {
        return Err(invalid("duplicate source property declaration"));
    }
    Ok(())
}
pub(super) fn charge(
    source: &SourcePropertyPreparationInput,
    used: &mut SupportReceivingStorageUse,
    limits: SupportReceivingStorageLimits,
) -> Result<()> {
    used.entries(source.relations.members.len(), limits)?;
    for row in &source.relations.members {
        used.entries(
            usize::from(matches!(
                row.occurrence,
                SourcePropertyOccurrence::GeneratedSkill { .. }
            )),
            limits,
        )?;
        for n in [
            row.effects.members.len(),
            row.inputs.len(),
            row.channels.members.len(),
            row.external.members.len(),
            row.supports.members.len(),
            row.assembly.members.len(),
        ] {
            used.entries(n, limits)?;
        }
        for support in &row.supports.members {
            used.entries(support.programs.members.len(), limits)?;
        }
        for effect in &row.effects.members {
            if let SourcePropertyEffectEndpoint::Generated { path, .. } = &effect.endpoint {
                used.path_depth = used.path_depth.max(path.len());
                used.entries(path.len() + 1, limits)?;
            }
            if let SupportAdmissionContext::ReceivingSkill {
                summoner_path: Some(path),
            } = &effect.admission
            {
                used.path_depth = used.path_depth.max(path.len());
                used.entries(path.len(), limits)?;
            }
        }
    }
    Ok(())
}
fn numeric(value: &ComputedValueType) -> bool {
    matches!(
        value,
        ComputedValueType::Integer | ComputedValueType::Quantity { .. }
    )
}
impl<'a, I: DefinitionSchemaIndex> Check<'a, I> {
    // Source applicability admits exact mechanical providers; this intentionally
    // does not widen the separate receiving-path traversal contract.
    fn source_declarations(&mut self, owner: &SlotOwnerDefId) -> Result<&'a DeclaredSlots> {
        self.used.work(1, self.l)?;
        Ok(match owner {
            SlotOwnerDefId::Class(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::Ascendancy(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::Reward(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::ItemTemplate(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::Modifier(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::Gem(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::Skill(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::Actor(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::PassiveNode(id) => &known(self.index.definition(id))?.declarations,
            SlotOwnerDefId::UsagePolicy(_) => {
                return Err(invalid("source supply requires a mechanical provider"));
            }
        })
    }
    fn source_occurrence(&mut self, relation: &SourcePropertyRelation) -> Result<()> {
        match (&relation.occurrence, &relation.owner) {
            (SourcePropertyOccurrence::AuthoredSkillUse {}, SupportTargetDefinition::Gem(gem)) => {
                let schema = known(self.index.definition(gem))?;
                self.used.work(schema.roles.len(), self.l)?;
                if !schema.roles.contains(&AuthoredGemRole::SkillUse) {
                    return Err(invalid("source owner Gem lacks SkillUse"));
                }
            }
            (
                SourcePropertyOccurrence::AuthoredSkillUse {},
                SupportTargetDefinition::Skill(skill),
            ) => {
                if !known(self.index.definition(skill))?.directly_selectable {
                    return Err(invalid("source owner Skill is not directly selectable"));
                }
            }
            (
                SourcePropertyOccurrence::GeneratedSkill { skill_supply },
                SupportTargetDefinition::Skill(skill),
            ) => {
                let declarations = self.source_declarations(&skill_supply.declaration)?;
                self.source_member(&declarations.skill_grants, skill_supply)?;
                let supply = known(self.index.slot(skill_supply))?;
                if &supply.skill != skill {
                    return Err(invalid(
                        "generated source owner differs from exact supplied Skill",
                    ));
                }
                self.source_declared_skill(&skill_supply.declaration, skill)?;
                known(self.index.definition(skill))?;
            }
            (SourcePropertyOccurrence::GeneratedSkill { .. }, SupportTargetDefinition::Gem(_)) => {
                return Err(invalid("generated source owner must be a Skill"));
            }
        }
        Ok(())
    }
    fn source_stat(&mut self, stat: &StatDefId, number: bool) -> Result<&ComputedValueType> {
        self.used.work(1, self.l)?;
        let schema = known(self.index.definition(stat))?;
        self.used.work(schema.targets.len(), self.l)?;
        if schema.targets != [RuleEntityKind::Skill] || (number && !numeric(&schema.value)) {
            return Err(invalid(
                "source property channel must have exact typed Skill scope",
            ));
        }
        Ok(&schema.value)
    }
    fn source_member<T: PartialEq>(&mut self, set: &DeclaredSet<T>, member: &T) -> Result<()> {
        complete(set)?;
        self.used.work(set.members.len() + 1, self.l)?;
        if !set.members.contains(member) {
            return Err(invalid("source property endpoint is not declared"));
        }
        Ok(())
    }
    fn source_effect(
        &mut self,
        owner: &SupportTargetDefinition,
        row: &SourcePropertyEffect,
    ) -> Result<()> {
        self.used.expanded(1, self.l)?;
        match &row.endpoint {
            SourcePropertyEffectEndpoint::OwnerSkill {} => {
                if !matches!(owner, SupportTargetDefinition::Skill(_))
                    || row.admission != SupportAdmissionContext::AssignedSkill
                {
                    return Err(invalid(
                        "owner Skill source effect requires assigned owner admission",
                    ));
                }
            }
            SourcePropertyEffectEndpoint::Generated { path, skill_supply } => {
                let SupportAdmissionContext::ReceivingSkill { summoner_path } = &row.admission
                else {
                    return Err(invalid(
                        "generated source effect requires receiving-skill admission",
                    ));
                };
                // Exact providers only: unlike legacy receiving, no implicit
                // Gem-to-Skill declaration jump or owned actor context is allowed.
                let mut current = target_owner(owner);
                for step in path {
                    if step.declaration != current {
                        return Err(invalid(
                            "source property path crosses unrelated declaration",
                        ));
                    }
                    let declarations = self.declarations(&current)?;
                    self.source_member(&declarations.grants, step)?;
                    let grant = known(self.index.slot(step))?;
                    let GrantTarget::Skill(supply) = &grant.target else {
                        return Err(invalid(
                            "source property context is limited to Player skill providers",
                        ));
                    };
                    if supply.declaration != current {
                        return Err(invalid("source property grant supplies an unrelated skill"));
                    }
                    self.source_member(&declarations.skill_grants, supply)?;
                    let supplied = known(self.index.slot(supply))?;
                    self.source_declared_skill(&current, &supplied.skill)?;
                    current = SlotOwnerDefId::Skill(supplied.skill.clone());
                }
                if skill_supply.declaration != current {
                    return Err(invalid(
                        "source property supply belongs to another provider",
                    ));
                }
                let declarations = self.declarations(&current)?;
                self.source_member(&declarations.skill_grants, skill_supply)?;
                let supplied = known(self.index.slot(skill_supply))?;
                self.source_declared_skill(&current, &supplied.skill)?;
                known(self.index.definition(&supplied.skill))?;
                if let Some(parent) = summoner_path {
                    if parent.is_empty() {
                        if !matches!(owner, SupportTargetDefinition::Skill(_)) {
                            return Err(invalid("physical input owner is not a summoner effect"));
                        }
                    } else {
                        if !path.starts_with(parent) {
                            return Err(invalid("source summoner must be an exact ancestor skill"));
                        }
                        let context = self.path(owner, parent)?;
                        if !context.exact_skill_supply || context.actor != RelativeActor::Assigned {
                            return Err(invalid(
                                "source summoner lacks exact Player skill context",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn source_declared_skill(&mut self, owner: &SlotOwnerDefId, skill: &SkillDefId) -> Result<()> {
        if let SlotOwnerDefId::Gem(gem) = owner {
            self.source_member(&known(self.index.definition(gem))?.skills, skill)?;
        }
        Ok(())
    }
    fn source_program<'b>(
        &mut self,
        rules: &'b OwnedRulePackage,
        owner: &SchemaSubject,
        id: &OwnedDefinitionKey,
    ) -> Result<&'b RuleProgram> {
        self.used.work(rules.input().owners.len(), self.l)?;
        let row = rules
            .input()
            .owners
            .iter()
            .find(|row| &row.owner == owner)
            .ok_or_else(|| invalid("source property owner has no rule inventory"))?;
        complete(&row.programs)?;
        self.used.work(row.programs.members.len(), self.l)?;
        row.programs
            .members
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| invalid("unknown source property program"))
    }
    fn source_owner_complete(
        &mut self,
        rules: &OwnedRulePackage,
        owner: &SchemaSubject,
    ) -> Result<()> {
        self.used.work(rules.input().owners.len(), self.l)?;
        let row = rules
            .input()
            .owners
            .iter()
            .find(|row| &row.owner == owner)
            .ok_or_else(|| invalid("source property owner has no rule inventory"))?;
        complete(&row.programs)
    }
    fn source_reads(
        &mut self,
        relation: &SourcePropertyRelation,
        program: &RuleProgram,
    ) -> Result<()> {
        self.used.work(program.reads.len(), self.l)?;
        for read in &program.reads {
            match &read.source {
                RuleReadSource::Stat {
                    entity: RuleEntity::PropertyOwner,
                    stat,
                } => {
                    self.used.work(relation.inputs.len() + 1, self.l)?;
                    if stat != &relation.non_hidden_count && !relation.inputs.contains(stat) {
                        return Err(invalid("undeclared source property input read"));
                    }
                    if self.source_stat(stat, false)? != &read.value_type {
                        return Err(invalid("source property input type mismatch"));
                    }
                }
                RuleReadSource::Contributions {
                    entity: RuleEntity::PropertyOwner,
                    stat,
                    contribution,
                    ..
                } => {
                    self.used.work(relation.channels.members.len(), self.l)?;
                    if !relation
                        .channels
                        .members
                        .iter()
                        .any(|v| &v.stat == stat && &v.contribution == contribution)
                        || self.source_stat(stat, true)? != &read.value_type
                    {
                        return Err(invalid(
                            "undeclared or mistyped source property contribution read",
                        ));
                    }
                }
                RuleReadSource::Capability {
                    entity: RuleEntity::PropertyOwner,
                    ..
                }
                | RuleReadSource::External {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } => {
                    return Err(invalid(
                        "source property scope grants only declared scalar/channel reads",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn source_bound_program(
        &mut self,
        relation: &SourcePropertyRelation,
        owner: &SchemaSubject,
        program: &RuleProgram,
        role: ReadinessProgramRole,
        assembly_binding: Option<SourcePropertyAssemblyBinding>,
    ) -> Result<()> {
        self.readiness_role(owner, &program.id, role)?;
        let readiness = self
            .stages
            .program_readiness(owner, &program.id)
            .expect("checked role");
        if readiness.phase != ReadinessPhase::Preparation {
            return Err(invalid("source properties require preparation phase"));
        }
        let stage = self
            .stages
            .stage_for(owner, &program.id)
            .ok_or_else(|| invalid("source property program lacks stage"))?;
        if !self.stages.precedes(&relation.census_stage, stage) {
            return Err(invalid("source property program must follow census stage"));
        }
        self.source_reads(relation, program)?;
        self.used.work(program.effects.len(), self.l)?;
        match role {
            ReadinessProgramRole::SourceSupportedProperty
            | ReadinessProgramRole::SourceExternalProperty => {
                let proper_context = if role == ReadinessProgramRole::SourceSupportedProperty {
                    program.context == RuleEntityKind::SupportOrigin
                } else {
                    matches!(
                        program.context,
                        RuleEntityKind::Actor | RuleEntityKind::EquipmentUse
                    )
                };
                if !proper_context || program.effects.is_empty() {
                    return Err(invalid(
                        "source property producer has invalid context or no output",
                    ));
                }
                for effect in &program.effects {
                    let RuleEffectKind::Contribute {
                        entity: RuleEntity::PropertyOwner,
                        stat,
                        contribution,
                        ..
                    } = &effect.effect
                    else {
                        return Err(invalid(
                            "source property producer may only contribute to PropertyOwner",
                        ));
                    };
                    self.used.work(relation.channels.members.len(), self.l)?;
                    if !relation
                        .channels
                        .members
                        .iter()
                        .any(|v| &v.stat == stat && &v.contribution == contribution)
                    {
                        return Err(invalid(
                            "source property producer writes undeclared channel",
                        ));
                    }
                }
            }
            ReadinessProgramRole::SourceFinalInputAssembly => {
                let binding = assembly_binding.expect("source assembly binding is explicit");
                let exact_supply = match binding {
                    SourcePropertyAssemblyBinding::InputOwner => None,
                    SourcePropertyAssemblyBinding::ExactSupplyingProvider => {
                        let SourcePropertyOccurrence::GeneratedSkill { skill_supply } =
                            &relation.occurrence
                        else {
                            return Err(invalid(
                                "exact supplying-provider assembly requires a generated source",
                            ));
                        };
                        Some(skill_supply)
                    }
                };
                let context = match exact_supply {
                    Some(supply) => source_provider_context(&supply.declaration)?,
                    None => match relation.owner {
                        SupportTargetDefinition::Gem(_) => RuleEntityKind::Actor,
                        SupportTargetDefinition::Skill(_) => RuleEntityKind::Skill,
                    },
                };
                if program.context != context {
                    return Err(invalid(
                        "source assembly must preserve its input owner context",
                    ));
                }
                for effect in &program.effects {
                    match &effect.effect {
                        RuleEffectKind::ProjectSkillParameter {
                            skill, parameter, ..
                        } => {
                            if let Some(supply) = exact_supply {
                                if skill != supply {
                                    return Err(invalid(
                                        "source assembly must project its exact supplied Skill",
                                    ));
                                }
                            } else if skill.declaration != target_owner(&relation.owner) {
                                return Err(invalid("source assembly projects a foreign child"));
                            }
                            let declarations = self.source_declarations(&skill.declaration)?;
                            self.source_member(&declarations.skill_grants, skill)?;
                            let supplied = known(self.index.slot(skill))?;
                            if parameter.declaration
                                != SlotOwnerDefId::Skill(supplied.skill.clone())
                            {
                                return Err(invalid(
                                    "source assembly projects a foreign parameter",
                                ));
                            }
                            let child = known(self.index.definition(&supplied.skill))?;
                            self.source_member(&child.declarations.parameters, parameter)?;
                            if exact_supply.is_some() {
                                let schema = known(self.index.slot(parameter))?;
                                if schema.presence != SlotPresence::RequiredOnce
                                    || !matches!(
                                        schema.skill_input,
                                        Some(
                                            SkillInputAuthority::Projected
                                                | SkillInputAuthority::AuthoredOrProjected
                                        )
                                    )
                                    || self.stages.parameter_phase(&supplied.skill, parameter)
                                        != Some(ReadinessPhase::Execution)
                                {
                                    return Err(invalid(
                                        "exact supplying-provider assembly requires projected required execution inputs",
                                    ));
                                }
                                if let Some(permission) = &supplied.preset_inputs {
                                    self.used
                                        .work(permission.parameters.members.len(), self.l)?;
                                    if permission.parameters.members.contains(parameter) {
                                        return Err(invalid(
                                            "source final input overlaps preset raw input authority",
                                        ));
                                    }
                                }
                            }
                        }
                        RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat,
                            ..
                        } if exact_supply.is_none()
                            && context == RuleEntityKind::Skill
                            && stat != &relation.non_hidden_count =>
                        {
                            self.source_stat(stat, true)?;
                        }
                        _ => return Err(invalid("source assembly has unauthorized effect")),
                    }
                }
            }
            _ => return Err(invalid("not a source property role")),
        }
        Ok(())
    }
    pub(super) fn source_properties(
        &mut self,
        input: &mut SourcePropertyPreparationInput,
        rules: &OwnedRulePackage,
        preparation: &OwnedSupportPreparation,
    ) -> Result<()> {
        complete(&input.relations)?;
        input.relations.members.sort_by(|a, b| a.id.cmp(&b.id));
        let mut owners = BTreeSet::new();
        let mut registered: Vec<(SchemaSubject, OwnedDefinitionKey)> = Vec::new();
        for (i, relation) in input.relations.members.iter().enumerate() {
            if (i > 0 && input.relations.members[i - 1].id == relation.id)
                || !owners.insert((relation.owner.clone(), relation.occurrence.clone()))
            {
                return Err(invalid(
                    "duplicate source property relation identity or applicability",
                ));
            }
        }
        for relation in &mut input.relations.members {
            self.source_owner_complete(rules, &target_subject(&relation.owner))?;
            for yes in [
                relation.effects.is_complete(),
                relation.channels.is_complete(),
                relation.external.is_complete(),
                relation.supports.is_complete(),
                relation.assembly.is_complete(),
            ] {
                if !yes {
                    return Err(invalid("source property inventories must be Complete"));
                }
            }
            if relation.effects.members.is_empty() {
                return Err(invalid(
                    "source property relation requires eligible effects",
                ));
            }
            self.source_occurrence(relation)?;
            if !self
                .stages
                .precedes(self.preparation_stage, &relation.census_stage)
            {
                return Err(invalid("source census must follow support preparation"));
            }
            unique(&mut relation.effects.members)?;
            let mut endpoints = BTreeSet::new();
            for effect in &relation.effects.members {
                if !endpoints.insert(&effect.endpoint) {
                    return Err(invalid("duplicate source effect endpoint"));
                }
                self.source_effect(&relation.owner, effect)?;
            }
            unique(&mut relation.inputs)?;
            unique(&mut relation.channels.members)?;
            if self.source_stat(&relation.non_hidden_count, false)? != &ComputedValueType::Integer {
                return Err(invalid("source support count requires Integer Skill stat"));
            }
            for stat in &relation.inputs {
                self.source_stat(stat, false)?;
                if stat == &relation.non_hidden_count {
                    return Err(invalid("source count cannot be an input fact"));
                }
            }
            for channel in &relation.channels.members {
                self.source_stat(&channel.stat, true)?;
                self.used.work(relation.inputs.len(), self.l)?;
                if channel.stat == relation.non_hidden_count
                    || relation.inputs.contains(&channel.stat)
                {
                    return Err(invalid("source property input/count/channel overlap"));
                }
            }
            relation.external.members.sort_by(|a, b| {
                source_owner_key(&a.owner)
                    .cmp(&source_owner_key(&b.owner))
                    .then(a.program.cmp(&b.program))
            });
            for pair in relation.external.members.windows(2) {
                if pair[0] == pair[1] {
                    return Err(invalid("duplicate external source property program"));
                }
            }
            let mut producers = Vec::new();
            for external in &relation.external.members {
                let p = self.source_program(rules, &external.owner, &external.program)?;
                self.source_bound_program(
                    relation,
                    &external.owner,
                    p,
                    ReadinessProgramRole::SourceExternalProperty,
                    None,
                )?;
                producers.push((external.owner.clone(), external.program.clone()));
            }
            relation.supports.members.sort_by(|a, b| a.gem.cmp(&b.gem));
            for pair in relation.supports.members.windows(2) {
                if pair[0].gem == pair[1].gem {
                    return Err(invalid("duplicate source support inventory"));
                }
            }
            for support in &mut relation.supports.members {
                complete(&support.programs)?;
                unique(&mut support.programs.members)?;
            }
            for support in &relation.supports.members {
                let schema = known(self.index.definition(&support.gem))?;
                self.used.work(
                    schema.roles.len() + preparation.input().supports.len(),
                    self.l,
                )?;
                if !schema.roles.contains(&AuthoredGemRole::SupportAssignment)
                    || !preparation.input().supports.iter().any(|r| {
                        r.gem == support.gem && matches!(r.preparation, SchemaState::Known(_))
                    })
                {
                    return Err(invalid(
                        "source support lacks known preparation/assignment schema",
                    ));
                }
                let owner = SchemaSubject::Definition(support.gem.address());
                self.source_owner_complete(rules, &owner)?;
                for id in &support.programs.members {
                    let p = self.source_program(rules, &owner, id)?;
                    self.source_bound_program(
                        relation,
                        &owner,
                        p,
                        ReadinessProgramRole::SourceSupportedProperty,
                        None,
                    )?;
                    producers.push((owner.clone(), id.clone()));
                }
            }
            unique(&mut relation.assembly.members)?;
            for assembly in &relation.assembly.members {
                let owner = match assembly.binding {
                    SourcePropertyAssemblyBinding::InputOwner => target_subject(&relation.owner),
                    SourcePropertyAssemblyBinding::ExactSupplyingProvider => {
                        let SourcePropertyOccurrence::GeneratedSkill { skill_supply } =
                            &relation.occurrence
                        else {
                            return Err(invalid(
                                "exact supplying-provider assembly requires a generated source",
                            ));
                        };
                        source_provider_subject(&skill_supply.declaration)?
                    }
                };
                let id = &assembly.program;
                let p = self.source_program(rules, &owner, id)?;
                self.source_bound_program(
                    relation,
                    &owner,
                    p,
                    ReadinessProgramRole::SourceFinalInputAssembly,
                    Some(assembly.binding),
                )?;
                let stage = self
                    .stages
                    .stage_for(&owner, id)
                    .expect("checked assembly stage");
                self.used.work(producers.len(), self.l)?;
                for (producer, program) in &producers {
                    if !self.stages.precedes(
                        self.stages
                            .stage_for(producer, program)
                            .expect("checked producer stage"),
                        stage,
                    ) {
                        return Err(invalid(
                            "source assembly must follow all property producers",
                        ));
                    }
                }
                registered.push((owner.clone(), id.clone()));
            }
            registered.extend(producers);
            // Synthetic count has no ordinary scalar producer, including dormant
            // or unselected branches. Never reinterpret such a collision as zero.
            self.used.work(rules.input().owners.len(), self.l)?;
            for owner in &rules.input().owners {
                self.used.work(owner.programs.members.len(), self.l)?;
                for p in &owner.programs.members {
                    self.used.work(p.effects.len(), self.l)?;
                    if p.effects.iter().any(|e| matches!(&e.effect, RuleEffectKind::Derive { stat, .. } if stat == &relation.non_hidden_count)) {
                        return Err(invalid("source count competes with an ordinary scalar producer"));
                    }
                }
            }
        }
        // Metadata cannot grant new scope to an unregistered program.
        if let Some(readiness) = self.stages.readiness() {
            self.used.work(readiness.programs.members.len(), self.l)?;
            for p in &readiness.programs.members {
                if matches!(
                    p.role,
                    ReadinessProgramRole::SourceSupportedProperty
                        | ReadinessProgramRole::SourceExternalProperty
                        | ReadinessProgramRole::SourceFinalInputAssembly
                ) {
                    self.used.work(registered.len(), self.l)?;
                    if !registered
                        .iter()
                        .any(|(owner, id)| owner == &p.owner && id == &p.program)
                    {
                        return Err(invalid(
                            "source readiness program lacks a complete relation binding",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
fn source_owner_key(owner: &SchemaSubject) -> (Option<&DefinitionAddress>, Option<&SlotAddress>) {
    match owner {
        SchemaSubject::Definition(value) => (Some(value), None),
        SchemaSubject::Slot(value) => (None, Some(value)),
    }
}
fn source_provider_subject(owner: &SlotOwnerDefId) -> Result<SchemaSubject> {
    Ok(SchemaSubject::Definition(match owner {
        SlotOwnerDefId::Class(id) => id.address(),
        SlotOwnerDefId::Ascendancy(id) => id.address(),
        SlotOwnerDefId::Reward(id) => id.address(),
        SlotOwnerDefId::ItemTemplate(id) => id.address(),
        SlotOwnerDefId::Modifier(id) => id.address(),
        SlotOwnerDefId::Gem(id) => id.address(),
        SlotOwnerDefId::Skill(id) => id.address(),
        SlotOwnerDefId::Actor(id) => id.address(),
        SlotOwnerDefId::PassiveNode(id) => id.address(),
        SlotOwnerDefId::UsagePolicy(_) => {
            return Err(invalid("source supply requires a mechanical provider"));
        }
    }))
}
fn source_provider_context(owner: &SlotOwnerDefId) -> Result<RuleEntityKind> {
    Ok(match owner {
        SlotOwnerDefId::Class(_)
        | SlotOwnerDefId::Ascendancy(_)
        | SlotOwnerDefId::Reward(_)
        | SlotOwnerDefId::Gem(_)
        | SlotOwnerDefId::Actor(_)
        | SlotOwnerDefId::PassiveNode(_) => RuleEntityKind::Actor,
        SlotOwnerDefId::ItemTemplate(_) | SlotOwnerDefId::Modifier(_) => {
            RuleEntityKind::EquipmentUse
        }
        SlotOwnerDefId::Skill(_) => RuleEntityKind::Skill,
        SlotOwnerDefId::UsagePolicy(_) => {
            return Err(invalid("source supply requires a mechanical provider"));
        }
    })
}
