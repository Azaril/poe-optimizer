//! Shared schema checking for stored preset intent and selected raw producers.
use super::*;

pub(crate) struct StoredIntentValidation {
    pub issues: Vec<BindingIssue>,
    pub work_used: usize,
}

/// Validate the explicit scenario layer in its actual selected context before
/// replacement. Character roots and required choices are meaningful here.
pub(crate) fn validate_selected_usage<I: DefinitionSchemaIndex>(
    index: &I,
    request: &OwnedEvaluationRequest,
    limits: BindingLimits,
) -> Result<StoredIntentValidation> {
    validated_request_digest(index, request, limits)?;
    let mut checker = Checker {
        index,
        request: Some(request),
        tables: RecordTables::from_build(request.build().input()),
        limits,
        work: limits.max_work,
        issues: Vec::new(),
    };
    let usage = &request.scenario().input().usage;
    checker.charge(usage.len() + 1)?;
    for (index, row) in usage.iter().enumerate() {
        checker.usage_selection(
            row,
            &BindingSite::new(BindingLocation::Usage { index }, BindingFacet::Target),
        )?;
    }
    Ok(StoredIntentValidation {
        work_used: limits.max_work - checker.work,
        issues: checker.issues,
    })
}

/// Caller proves global occurrence structure, including unresolved draft roots,
/// before this schema-only check. No selected build or union of choices is made.
/// Required choice satisfaction and activation are checked on the final request.
pub(crate) fn validate_stored_intent<I: DefinitionSchemaIndex>(
    index: &I,
    namespace: &GameVersionNamespace,
    tables: RecordTables<'_>,
    usage: &[UsagePolicySelection],
    inputs: &[SelectedGeneratedSkillInput],
    limits: BindingLimits,
) -> Result<StoredIntentValidation> {
    limits.input.validate()?;
    if limits.max_work == 0
        || limits.max_work > 100_000_000
        || limits.max_issues == 0
        || limits.max_issues > 65_536
    {
        return Err(BindingError::InvalidLimit);
    }
    if index.namespace() != namespace {
        return Err(BindingError::ForeignNamespace);
    }
    if index.identity().validate().is_err() || index.identity().game != namespace.game().as_str() {
        return Err(BindingError::Index {
            subject: None,
            fault: IndexFault::InvalidIdentity,
        });
    }
    let mut checker = Checker {
        index,
        request: None,
        tables,
        limits,
        work: limits.max_work,
        issues: Vec::new(),
    };
    checker.charge(usage.len() + inputs.len() + 1)?;
    for (index, row) in usage.iter().enumerate() {
        checker.usage_selection(
            row,
            &BindingSite::new(BindingLocation::Usage { index }, BindingFacet::Target),
        )?;
    }
    for (index, row) in inputs.iter().enumerate() {
        checker.generated_input(
            row,
            &BindingSite::new(
                BindingLocation::GeneratedInput { index },
                BindingFacet::Target,
            ),
        )?;
    }
    Ok(StoredIntentValidation {
        work_used: limits.max_work - checker.work,
        issues: checker.issues,
    })
}

impl<I: DefinitionSchemaIndex> Checker<'_, I> {
    pub(super) fn generated_input(
        &mut self,
        row: &SelectedGeneratedSkillInput,
        site: &BindingSite,
    ) -> Result {
        self.bind_skill_target(
            &SkillTarget::Generated(Box::new(row.target.clone())),
            site,
            Purpose::Authored,
        )?;
        let supply_subject = SchemaSubject::Slot(SkillGrantSlotDefId::address(&row.target.slot));
        let Some(supply) = self.slot(&row.target.slot, site)? else {
            return Ok(());
        };
        let Some(permission) = &supply.preset_inputs else {
            return self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::ValueForbidden,
                Some(supply_subject),
            );
        };
        if permission.schema_version != 1 || !permission.parameters.is_complete() {
            return self.fault(Some(supply_subject));
        }
        let owner = SlotOwnerDefId::Skill(supply.skill.clone());
        let declarations = self.declarations(&owner, site)?;
        let parameter_site = site.at(BindingFacet::Parameter);
        self.charge(row.parameters.len() + permission.parameters.members.len() + 1)?;
        for parameter in &row.parameters {
            let subject = SchemaSubject::Slot(ParameterSlotDefId::address(&parameter.slot));
            self.membership(
                &permission.parameters,
                &parameter.slot,
                subject.clone(),
                &parameter_site,
                Purpose::Authored,
            )?;
            if parameter.slot.declaration != owner {
                self.issue(
                    &parameter_site,
                    IssueClass::Invalid,
                    BindingIssueCode::WrongOwner,
                    Some(subject.clone()),
                )?;
            }
            if let Some(declarations) = declarations {
                self.membership(
                    &declarations.parameters,
                    &parameter.slot,
                    subject.clone(),
                    &parameter_site,
                    Purpose::Authored,
                )?;
            }
            if let Some(schema) = self.slot(&parameter.slot, &parameter_site)? {
                if !schema.permits_projected_skill_input() || schema.skill_input.is_none() {
                    self.issue(
                        &parameter_site,
                        IssueClass::Invalid,
                        BindingIssueCode::ValueForbidden,
                        Some(subject.clone()),
                    )?;
                }
                self.value(&parameter.value, &schema.value, &subject, &parameter_site)?;
            }
        }
        // Check permission metadata as well as the authored subset: custom index
        // implementations must not smuggle foreign or implicit-authority slots.
        let mut seen = std::collections::BTreeSet::new();
        for slot in &permission.parameters.members {
            if slot.declaration != owner || !seen.insert(slot) {
                return self.fault(Some(supply_subject));
            }
            let subject = SchemaSubject::Slot(ParameterSlotDefId::address(slot));
            if let Some(declarations) = declarations {
                self.membership(
                    &declarations.parameters,
                    slot,
                    subject.clone(),
                    &parameter_site,
                    Purpose::Authored,
                )?;
            }
            if let Some(schema) = self.slot(slot, &parameter_site)? {
                if !schema.permits_projected_skill_input() || schema.skill_input.is_none() {
                    return self.fault(Some(subject));
                }
                self.charge(row.parameters.len() + schema.sites.len())?;
                if schema.presence == SlotPresence::RequiredOnce
                    && !row.parameters.iter().any(|p| &p.slot == slot)
                {
                    self.issue(
                        &site.at(BindingFacet::RequiredValues),
                        IssueClass::Invalid,
                        BindingIssueCode::RequiredValueMissing,
                        Some(subject),
                    )?;
                }
            }
        }
        Ok(())
    }
}
