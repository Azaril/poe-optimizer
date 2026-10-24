//! Checked per-ConfigSet adapter correspondence, before any inventory decision.
//!
//! Preserve producer order, multiplicity, exact targets and no-origin facts. The
//! two phases are intentional: existing raw-only scalar accounting occurs before
//! the remaining adapters inspect current provenance. Neither phase retires an
//! obligation or asserts that all source controls have been accounted for.
use super::*;

pub(in crate::owned_normalize) struct AdapterReceipts<'a> {
    pub encounters: &'a BTreeMap<SourceOccurrenceId, encounter::ProvenEncounter>,
    pub levels: &'a BTreeMap<SourceOccurrenceId, enemy_level::ProvenEnemyLevel>,
    pub inputs: &'a BTreeMap<SourceOccurrenceId, configuration_inputs::ProvenConfigurationInputs>,
    pub usage: &'a BTreeMap<SourceOccurrenceId, usage_inputs::ProvenConfigurationUsage>,
    pub usage_policy: Option<&'a usage_inputs::CompiledUsageInputs<'a>>,
    pub rewards: &'a BTreeMap<
        SourceOccurrenceId,
        configuration_reward_inventory::ProvenConfigurationRewards,
    >,
}

pub(super) struct ScalarReceipts<'d> {
    scope: SourceOccurrenceId,
    scenario: &'d ScenarioPresetDraft,
    choice: ChoicePresetId,
    rows: Vec<CheckedConfigurationReceipt>,
}

impl<'d> ScalarReceipts<'d> {
    pub(super) fn collect(
        b: &mut Builder<'_, '_>,
        scope: SourceOccurrenceId,
        scenario: &'d ScenarioPresetDraft,
        choice: ChoicePresetId,
        adapters: &AdapterReceipts<'_>,
    ) -> Result<Self> {
        Ok(Self {
            scope,
            scenario,
            choice,
            rows: configuration_inputs::checked_receipts(
                b,
                scope,
                scenario,
                adapters.inputs.get(&scope),
            )?,
        })
    }

    pub(super) fn iter(&self) -> std::slice::Iter<'_, CheckedConfigurationReceipt> {
        self.rows.iter()
    }

    /// Called only after the existing raw-only pass, and only when accounting
    /// was requested. Each producer retains its original bounded validation and
    /// work charge. No receipts are cloned, sorted, coalesced or manufactured.
    pub(super) fn finish<'p>(
        mut self,
        b: &mut Builder<'_, '_>,
        draft: &DraftSessionInput,
        adapters: &AdapterReceipts<'p>,
        strict_empty_custom_modifiers: bool,
    ) -> Result<ScopeReceipts<'p>> {
        self.rows
            .extend(usage_inputs::checked_configuration_receipts(
                b,
                self.scope,
                self.scenario,
                adapters.usage.get(&self.scope),
            )?);
        self.rows
            .extend(configuration_reward_inventory::checked_receipts(
                b,
                self.scope,
                self.choice,
                draft,
                adapters.rewards.get(&self.scope),
            )?);
        self.rows.extend(enemy_level::checked_receipts(
            b,
            self.scope,
            self.scenario,
            adapters.levels.get(&self.scope),
        )?);
        let proof = adapters.encounters.get(&self.scope);
        let encounter_absence =
            match encounter::checked_receipt(b, self.scope, self.scenario, proof)? {
                Some(checked) => {
                    self.rows.push(checked.receipt);
                    EncounterAbsence::Checked(checked.absent)
                }
                None if proof.is_some() => EncounterAbsence::Unavailable,
                None => EncounterAbsence::MissingProof,
            };
        let custom_modifiers = if strict_empty_custom_modifiers {
            custom_modifiers::collect(b, self.scope, self.scenario.id)?
        } else {
            None
        };
        Ok(ScopeReceipts {
            scope: self.scope,
            scenario: self.scenario.id,
            rows: self.rows,
            encounter_absence,
            custom_modifiers,
            inapplicable: Vec::new(),
        })
    }
}

enum EncounterAbsence<'p> {
    MissingProof,
    Unavailable,
    Checked(&'p BTreeSet<String>),
}

pub(super) struct ScopeReceipts<'p> {
    scope: SourceOccurrenceId,
    scenario: ScenarioPresetId,
    rows: Vec<CheckedConfigurationReceipt>,
    encounter_absence: EncounterAbsence<'p>,
    custom_modifiers: Option<custom_modifiers::CheckedCustomModifiers>,
    inapplicable: Vec<accounting::CheckedInapplicable>,
}

impl ScopeReceipts<'_> {
    pub(super) fn retain_inapplicable(&mut self, receipt: accounting::CheckedInapplicable) {
        self.inapplicable.push(receipt);
    }

    pub(super) fn inapplicable(&self) -> &[accounting::CheckedInapplicable] {
        &self.inapplicable
    }

    pub(super) fn custom_modifiers(&self) -> Option<&custom_modifiers::CheckedCustomModifiers> {
        self.custom_modifiers
            .as_ref()
            .filter(|receipt| receipt.belongs_to(self.scope, self.scenario))
    }

    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }

    pub(super) fn iter(&self) -> std::slice::Iter<'_, CheckedConfigurationReceipt> {
        self.rows.iter()
    }

    /// The caller charges the same complete receipt scan as before. Preserve
    /// duplicate matches so ambiguity remains a refusal, and never let a
    /// constructor/absence receipt stand in for an authored source occurrence.
    pub(super) fn matching_leaf<'r>(
        &'r self,
        scope: SourceOccurrenceId,
        origin: SourceOccurrenceId,
        selector: &'r ValueSelector,
        control: &'r ConfigurationAdapterControl,
    ) -> impl Iterator<Item = &'r CheckedConfigurationReceipt> {
        self.iter().filter(move |receipt| {
            receipt.scope == scope
                && receipt.origin == Some(origin)
                && receipt.selector.as_ref() == Some(selector)
                && &receipt.control == control
        })
    }

    /// Preserve the former branch check's charge even if a supplied proof failed
    /// current target validation. Presence alone grants no absence authority.
    pub(super) fn has_encounter_proof(&self) -> bool {
        !matches!(self.encounter_absence, EncounterAbsence::MissingProof)
    }

    pub(super) fn proves_encounter_branch(
        &self,
        scope: SourceOccurrenceId,
        scenario: ScenarioPresetId,
        guards: &BTreeSet<String>,
    ) -> bool {
        self.scope == scope
            && self.scenario == scenario
            && matches!(&self.encounter_absence,
                EncounterAbsence::Checked(absent) if guards.is_subset(absent))
    }
}

#[cfg(test)]
mod tests;
