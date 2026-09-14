use super::*;
impl<'a, I: DefinitionSchemaIndex> Checker<'a, I> {
    pub(super) fn integer(
        &mut self,
        value: BoundedInteger,
        range: &IntegerRange,
        subject: &SchemaSubject,
        site: &BindingSite,
    ) -> Result {
        if range.minimum > range.maximum {
            return self.fault(Some(subject.clone()));
        }
        if value < range.minimum || value > range.maximum {
            self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::OutOfRange,
                Some(subject.clone()),
            )?;
        }
        Ok(())
    }
    pub(super) fn level(
        &mut self,
        value: u16,
        range: &IntegerRange,
        subject: &SchemaSubject,
        site: &BindingSite,
    ) -> Result {
        // Every u16 is exactly representable in the bounded integer domain.
        let value = BoundedInteger::new(i64::from(value)).expect("u16 fits owned integer");
        self.integer(value, range, subject, site)
    }
    pub(super) fn quantity(
        &mut self,
        value: &FiniteQuantity,
        range: &QuantityRange,
        subject: &SchemaSubject,
        site: &BindingSite,
    ) -> Result {
        if range.minimum.unit() != range.maximum.unit()
            || range.minimum.value() > range.maximum.value()
        {
            return self.fault(Some(subject.clone()));
        }
        self.definition(value.unit(), site)?;
        self.definition(range.minimum.unit(), site)?;
        if value.unit() != range.minimum.unit() {
            self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::UnitMismatch,
                Some(subject.clone()),
            )?;
        } else if value.value() < range.minimum.value() || value.value() > range.maximum.value() {
            self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::OutOfRange,
                Some(subject.clone()),
            )?;
        }
        Ok(())
    }
    pub(super) fn value(
        &mut self,
        value: &ParameterValue,
        schema: &ValueSchema,
        subject: &SchemaSubject,
        site: &BindingSite,
    ) -> Result {
        self.charge(1)?;
        match (value, schema) {
            (ParameterValue::Boolean(_), ValueSchema::Boolean) => Ok(()),
            (ParameterValue::Integer(value), ValueSchema::Integer(range)) => {
                self.integer(*value, range, subject, site)
            }
            (ParameterValue::Quantity(value), ValueSchema::Quantity(range)) => {
                self.quantity(value, range, subject, site)
            }
            (ParameterValue::Option(value), ValueSchema::Option { allowed }) => {
                self.definition(value, site)?;
                self.membership(allowed, value, subject.clone(), site, Purpose::Authored)?;
                Ok(())
            }
            _ => self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::ValueKindMismatch,
                Some(subject.clone()),
            ),
        }
    }
    pub(super) fn parameters(
        &mut self,
        owner: &SlotOwnerDefId,
        rows: &[ParameterAssignment],
        kind: ParameterSite,
        site: &BindingSite,
    ) -> Result {
        self.charge(rows.len() + 1)?;
        let declarations = self.declarations(owner, site)?;
        for row in rows {
            let subject = SchemaSubject::Slot(ParameterSlotDefId::address(&row.slot));
            if let Some(declarations) = declarations {
                self.membership(
                    &declarations.parameters,
                    &row.slot,
                    subject.clone(),
                    site,
                    Purpose::Authored,
                )?;
            }
            if let Some(schema) = self.slot(&row.slot, site)? {
                self.role(&schema.sites, &kind, Some(subject.clone()), site)?;
                self.value(&row.value, &schema.value, &subject, site)?;
            }
        }
        if let Some(declarations) = declarations {
            self.closure(
                &declarations.parameters,
                owner_subject(owner),
                &site.at(BindingFacet::RequiredValues),
            )?;
            self.charge(declarations.parameters.members.len())?;
            for slot in &declarations.parameters.members {
                if let Some(schema) = self.slot(slot, site)? {
                    self.charge(rows.len() + schema.sites.len())?;
                    if schema.presence == SlotPresence::RequiredOnce
                        && schema.sites.contains(&kind)
                        && !rows.iter().any(|row| &row.slot == slot)
                    {
                        self.issue(
                            &site.at(BindingFacet::RequiredValues),
                            IssueClass::Invalid,
                            BindingIssueCode::RequiredValueMissing,
                            Some(SchemaSubject::Slot(ParameterSlotDefId::address(slot))),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn quality(
        &mut self,
        quality: &Option<QualitySelection>,
        schema: &QualityUseSchema,
        subject: &SchemaSubject,
        site: &BindingSite,
    ) -> Result {
        match quality {
            None if schema.presence == QualityPresence::Required => self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::RequiredValueMissing,
                Some(subject.clone()),
            ),
            None => Ok(()),
            Some(quality) => {
                if schema.presence == QualityPresence::Forbidden {
                    self.issue(
                        site,
                        IssueClass::Invalid,
                        BindingIssueCode::ValueForbidden,
                        Some(subject.clone()),
                    )?;
                }
                self.membership(
                    &schema.allowed_kinds,
                    &quality.kind,
                    subject.clone(),
                    site,
                    Purpose::Authored,
                )?;
                if let Some(schema) = self.definition(&quality.kind, site)? {
                    self.quantity(
                        &quality.amount,
                        &schema.amount,
                        &SchemaSubject::Definition(quality.kind.address()),
                        site,
                    )?;
                }
                Ok(())
            }
        }
    }
    pub(super) fn scope(
        &mut self,
        scope: &LoadoutScope,
        policy: ScopePolicy,
        subject: SchemaSubject,
        site: &BindingSite,
    ) -> Result {
        if !matches!(
            (scope, policy),
            (
                LoadoutScope::Shared,
                ScopePolicy::Shared | ScopePolicy::Either
            ) | (
                LoadoutScope::Selected { .. },
                ScopePolicy::Selected | ScopePolicy::Either
            )
        ) {
            self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::IncompatibleScope,
                Some(subject),
            )?;
        }
        Ok(())
    }
}
