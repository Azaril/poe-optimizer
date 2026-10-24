//! Same-attempt proof of a reviewed missing raw source branch. This proves no
//! effective value, creates no adapter output, and never completes a Scenario.
use super::*;
use inventory::{Scope, index_work};

type Control = (ConfigurationInventoryControl, ConfigurationInventoryTarget);

#[derive(Debug, Eq, PartialEq)]
struct Owners {
    scope: SourceOccurrenceId,
    scenario: ScenarioPresetId,
    choice: ChoicePresetId,
    issue: DraftIssueId,
    fields: BTreeMap<ConfigurationScenarioField, DraftIssueId>,
}

pub(super) struct Proof {
    owners: Owners,
    controls: BTreeMap<OwnedDefinitionKey, (ValueSelector, ConfigurationInventoryTarget)>,
}

/// Only the checked collector and current-owner recheck can construct this.
/// Consumption removes one independent census ID; neither a raw leaf nor an
/// adapter receipt can be substituted for this no-leaf source proof.
pub(super) struct Checked(Proof);

fn completion(
    scenario: &ScenarioPresetDraft,
    field: ConfigurationScenarioField,
) -> (&DraftListCompletion, &'static str) {
    match field {
        ConfigurationScenarioField::Assumptions => (
            &scenario.scenario.assumptions.completion,
            "external-assumptions-not-converted",
        ),
        ConfigurationScenarioField::Usage => {
            (&scenario.scenario.usage.completion, "usage-not-converted")
        }
        ConfigurationScenarioField::UsageIntents => (
            &scenario.scenario.usage_intents.completion,
            "scenario-usage-intents-not-converted",
        ),
    }
}

fn pending(value: &DraftListCompletion) -> Option<DraftIssueId> {
    match value {
        DraftListCompletion::Pending { id, .. } => Some(*id),
        DraftListCompletion::Complete => None,
    }
}

fn owners(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    authority: &Scope<'_>,
    fields: &BTreeSet<ConfigurationScenarioField>,
) -> Result<Option<Owners>> {
    let Some(sets) = source_shape::fresh_config_sets(b)? else {
        return Ok(None);
    };
    b.charge(sets.len())?;
    if !sets.contains(&authority.source) {
        return Ok(None);
    }
    let evidence = b.evidence;
    b.charge(draft.scenario_presets.members.len())?;
    let mut scenarios = draft
        .scenario_presets
        .members
        .iter()
        .filter(|p| p.id == authority.scenario.id);
    let Some(scenario) = scenarios.next() else {
        return Ok(None);
    };
    if scenarios.next().is_some() {
        return Ok(None);
    }
    // Check reverse ownership for every saved set, including unselected ones.
    // Descendant projections are not independent ConfigSet owners.
    for source in sets {
        let origin = &b.origins[source.ordinal() as usize];
        let cost = origin.links.len().saturating_add(1);
        b.charge(cost)?;
        let origin = &b.origins[source.ordinal() as usize];
        if origin.source != source || origin.disposition != SourceDisposition::Contributes {
            return Ok(None);
        }
        if source == authority.source {
            b.charge(draft.choice_presets.members.len())?;
            if accounting::configuration_issue(
                &b.origins[source.ordinal() as usize].links,
                scenario.id,
                &draft.choice_presets.members,
            ) != Some((authority.issue, authority.choice))
            {
                return Ok(None);
            }
        } else if origin.links.iter().any(|link| {
            *link == OwnedOriginTarget::ScenarioPreset(scenario.id)
                || *link == OwnedOriginTarget::ChoicePreset(authority.choice)
        }) {
            return Ok(None);
        }
    }
    let mut retained = BTreeMap::new();
    let mut issues = BTreeSet::new();
    for field in fields {
        let (state, expected_code) = completion(scenario, *field);
        b.charge(expected_code.len().saturating_add(1))?;
        let DraftListCompletion::Pending { id, code } = state else {
            return Ok(None);
        };
        if code.as_str() != expected_code || *id == authority.issue || !issues.insert(*id) {
            return Ok(None);
        }
        retained.insert(*field, *id);
    }
    if !issues.is_empty() {
        // A field's pending issue cannot also be a different field/owner's
        // completion authority. This never inspects or changes known members.
        b.charge(
            draft
                .scenario_presets
                .members
                .len()
                .saturating_mul(5)
                .saturating_add(draft.choice_presets.members.len().saturating_mul(2)),
        )?;
        for owner in &draft.scenario_presets.members {
            for field in [
                ConfigurationScenarioField::Assumptions,
                ConfigurationScenarioField::Usage,
                ConfigurationScenarioField::UsageIntents,
            ] {
                if let Some(id) = pending(completion(owner, field).0)
                    && issues.contains(&id)
                    && (owner.id != scenario.id || retained.get(&field) != Some(&id))
                {
                    return Ok(None);
                }
            }
            if matches!(&owner.scenario.enemy.encounter, DraftField::Pending(p) if issues.contains(&p.id))
                || matches!(&owner.scenario.enemy.level, DraftField::Pending(p) if issues.contains(&p.id))
            {
                return Ok(None);
            }
        }
        for owner in &draft.choice_presets.members {
            if [
                pending(&owner.choices.completion),
                pending(&owner.rewards.completion),
            ]
            .iter()
            .flatten()
            .any(|id| issues.contains(id))
            {
                return Ok(None);
            }
        }
        let mut attached = BTreeSet::new();
        for index in 0..b.origins.len() {
            b.charge(b.origins[index].links.len().saturating_add(1))?;
            let origin = &b.origins[index];
            let mut matched = BTreeSet::new();
            for link in &origin.links {
                if let OwnedOriginTarget::Issue(id) = link
                    && issues.contains(id)
                    && !matched.insert(*id)
                {
                    return Ok(None);
                }
            }
            if matched.is_empty() {
                continue;
            }
            if origin.source != evidence.rows()[index].occurrence().id()
                || origin.disposition != SourceDisposition::Contributes
            {
                return Ok(None);
            }
            let source = origin.source;
            let mut ancestor = Some(source);
            while let Some(id) = ancestor {
                b.charge(1)?;
                if id == authority.source {
                    break;
                }
                ancestor = evidence.rows()[id.ordinal() as usize].occurrence().parent();
            }
            if ancestor != Some(authority.source) {
                return Ok(None);
            }
            if source == authority.source {
                attached.extend(matched);
            }
        }
        if attached != issues {
            return Ok(None);
        }
    }
    Ok(Some(Owners {
        scope: authority.source,
        scenario: scenario.id,
        choice: authority.choice,
        issue: authority.issue,
        fields: retained,
    }))
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    authority: &Scope<'_>,
    controls: &[Control],
) -> Result<Option<Proof>> {
    // No source/owner work when the complete census has no missing-raw target.
    b.charge(controls.len())?;
    let mut fields = BTreeSet::new();
    let mut requested = Vec::new();
    for (control, target) in controls {
        match target {
            ConfigurationInventoryTarget::MissingRawNoAdditionalInput {} => {}
            ConfigurationInventoryTarget::MissingRawRetainedScenario {
                fields: requested_fields,
            } => {
                b.charge(requested_fields.len())?;
                if requested_fields.is_empty()
                    || requested_fields.len() > 3
                    || requested_fields.iter().collect::<BTreeSet<_>>().len()
                        != requested_fields.len()
                {
                    return Ok(None);
                }
                fields.extend(requested_fields);
            }
            _ => continue,
        }
        requested.push((control, target));
    }
    if requested.is_empty() {
        return Ok(None);
    }
    let Some(owners) = owners(b, draft, authority, &fields)? else {
        return Ok(None);
    };
    let evidence = b.evidence;
    let children = evidence.rows()[authority.source.ordinal() as usize].children();
    b.charge(children.len())?;
    let mut present = BTreeSet::new();
    for id in children {
        let row = &evidence.rows()[id.ordinal() as usize];
        source_shape::charge_row(b, row)?;
        if matches!(row.occurrence().name(), "Input" | "Placeholder") {
            let Some(name) = source_shape::value(row, "name") else {
                return Ok(None);
            };
            b.charge(index_work(present.len(), name.len()))?;
            present.insert(name);
        }
    }
    let mut proofs = BTreeMap::new();
    for (control, target) in requested {
        let Some(selector) = &control.selector else {
            return Ok(None);
        };
        b.charge(
            index_work(present.len(), selector.name.len())
                .saturating_add(index_work(proofs.len(), control.id.as_str().len()))
                .saturating_add(selector.name.len())
                .saturating_add(4),
        )?;
        if control.source_states != [ConfigurationSourceState::Absent]
            || present.contains(selector.name.as_str())
            || proofs
                .insert(control.id.clone(), (selector.clone(), target.clone()))
                .is_some()
        {
            return Ok(None);
        }
    }
    Ok(Some(Proof {
        owners,
        controls: proofs,
    }))
}

impl Proof {
    pub(super) fn check(
        self,
        b: &mut Builder<'_, '_>,
        draft: &DraftSessionInput,
        authority: &Scope<'_>,
    ) -> Result<Option<Checked>> {
        b.charge(self.owners.fields.len())?;
        let fields = self.owners.fields.keys().copied().collect();
        if owners(b, draft, authority, &fields)?.as_ref() != Some(&self.owners) {
            return Ok(None);
        }
        Ok(Some(Checked(self)))
    }
}

impl Checked {
    pub(super) fn consume(
        &mut self,
        b: &mut Builder<'_, '_>,
        scope: SourceOccurrenceId,
        scenario: ScenarioPresetId,
        control: &ConfigurationInventoryControl,
        target: &ConfigurationInventoryTarget,
    ) -> Result<bool> {
        b.charge(
            index_work(self.0.controls.len(), control.id.as_str().len())
                .saturating_add(control.selector.as_ref().map_or(0, |s| s.name.len()))
                .saturating_add(4),
        )?;
        if self.0.owners.scope != scope
            || self.0.owners.scenario != scenario
            || control.source_states != [ConfigurationSourceState::Absent]
        {
            return Ok(false);
        }
        Ok(self
            .0
            .controls
            .remove(&control.id)
            .is_some_and(|(selector, expected)| {
                control.selector.as_ref() == Some(&selector) && target == &expected
            }))
    }

    pub(super) fn consumed(&self) -> bool {
        self.0.controls.is_empty()
    }
}
