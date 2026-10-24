use super::*;
use crate::owned_normalize::source_shape::{fresh_config_sets, value};

pub(super) fn configuration_issue(
    links: &[OwnedOriginTarget],
    scenario: ScenarioPresetId,
    choices: &[ChoicePresetDraft],
) -> Option<(DraftIssueId, ChoicePresetId)> {
    let mut scenarios = links.iter().filter_map(|link| match link {
        OwnedOriginTarget::ScenarioPreset(id) => Some(*id),
        _ => None,
    });
    if scenarios.next() != Some(scenario) || scenarios.next().is_some() {
        return None;
    }
    let mut selected = links.iter().filter_map(|link| match link {
        OwnedOriginTarget::ChoicePreset(id) => Some(*id),
        _ => None,
    });
    let choice = selected.next()?;
    if selected.next().is_some() {
        return None;
    }
    let mut owners = choices.iter().filter(|p| p.id == choice);
    let owner = owners.next()?;
    if owners.next().is_some() {
        return None;
    }
    let DraftListCompletion::Pending { id, code } = &owner.choices.completion else {
        return None;
    };
    (code.as_str() == "configuration-roles-not-converted"
        && links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::Issue(*id))
            .count()
            == 1)
        .then_some((*id, choice))
}

pub(super) fn lane(row: &SourceEvidenceRow<'_>) -> Option<ValueLane> {
    match (
        row.occurrence().name(),
        value(row, "number").is_some(),
        value(row, "string").is_some(),
        value(row, "boolean").is_some(),
    ) {
        ("Input", true, false, false) => Some(ValueLane::InputNumber),
        ("Input", false, true, false) => Some(ValueLane::InputString),
        ("Input", false, false, true) => Some(ValueLane::InputBoolean),
        ("Placeholder", true, false, false) => Some(ValueLane::PlaceholderNumber),
        ("Placeholder", false, true, false) => Some(ValueLane::PlaceholderString),
        _ => None,
    }
}

/// The sole mutation path for configuration leaf accounting. Extra semantic
/// links are retained by refusing the receipt; no inventory issue is retired.
fn apply(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    origin: SourceOccurrenceId,
    issue: DraftIssueId,
    targets: &[OwnedOriginTarget],
    source_only: Option<&OwnedDefinitionKey>,
) -> Result<bool> {
    b.charge(
        targets.len().saturating_add(3).saturating_mul(
            b.origins[origin.ordinal() as usize]
                .links
                .len()
                .saturating_add(1),
        ),
    )?;
    if b.evidence.rows()[origin.ordinal() as usize]
        .occurrence()
        .parent()
        != Some(scope)
    {
        return Ok(false);
    }
    let row = &b.origins[origin.ordinal() as usize];
    let obligation = OwnedOriginTarget::Issue(issue);
    if row.disposition != SourceDisposition::Contributes
        || row.links.iter().filter(|l| **l == obligation).count() != 1
        || row
            .links
            .iter()
            .any(|l| *l != obligation && !targets.contains(l))
        || targets
            .iter()
            .any(|target| row.links.iter().filter(|l| *l == target).count() > 1)
        || (source_only.is_none() && targets.iter().any(|target| !row.links.contains(target)))
    {
        return Ok(false);
    }
    let row = &mut b.origins[origin.ordinal() as usize];
    row.links.retain(|link| *link != obligation);
    if let Some(reason) = source_only {
        row.disposition = SourceDisposition::SourceOnly(reason.clone());
    }
    for target in targets {
        if !b.origins[origin.ordinal() as usize].links.contains(target) {
            b.link(origin, target.clone())?;
        }
    }
    Ok(true)
}

/// Retained only after both the reviewed branch and its leaf disposition have
/// succeeded. This cannot stand in for an adapter's actual output receipt.
pub(super) struct CheckedInapplicable {
    pub(super) scope: SourceOccurrenceId,
    pub(super) origin: SourceOccurrenceId,
    pub(super) scenario: ScenarioPresetId,
    pub(super) rule: OwnedDefinitionKey,
    pub(super) selector: ValueSelector,
    pub(super) reason: OwnedDefinitionKey,
}

/// The family receipt is all-or-nothing. Before retiring any source leaf, every
/// member must retain exactly this inventory's unresolved responsibility. A
/// scalar adapter, foreign issue or prior disposition cannot be overwritten.
fn apply_custom_modifiers(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: ScenarioPresetId,
    issue: DraftIssueId,
    receipt: &custom_modifiers::CheckedCustomModifiers,
) -> Result<()> {
    if !receipt.belongs_to(scope, scenario) {
        return Ok(());
    }
    for origin in receipt.origins() {
        let row = &b.origins[origin.ordinal() as usize];
        b.charge(row.links.len().saturating_add(1))?;
        let row = &b.origins[origin.ordinal() as usize];
        if row.disposition != SourceDisposition::Contributes
            || row.links.as_slice() != [OwnedOriginTarget::Issue(issue)]
        {
            return Ok(());
        }
    }
    let target = OwnedOriginTarget::ScenarioPreset(scenario);
    let reason = key("literal-empty-custom-modifier-source");
    for origin in receipt.origins() {
        apply(
            b,
            scope,
            *origin,
            issue,
            std::slice::from_ref(&target),
            Some(&reason),
        )?;
    }
    Ok(())
}

fn branch_proven(
    b: &mut Builder<'_, '_>,
    origin: SourceOccurrenceId,
    scenario: &ScenarioPresetDraft,
    receipts: &receipts::ScopeReceipts<'_>,
    selector: &ValueSelector,
    recipe: &ValueRecipe,
    absent: &BTreeSet<String>,
) -> Result<bool> {
    let Some(scope) = b.evidence.rows()[origin.ordinal() as usize]
        .occurrence()
        .parent()
    else {
        return Ok(false);
    };
    if !receipts.has_encounter_proof() {
        return Ok(false);
    }
    b.charge(absent.len())?;
    if !receipts.proves_encounter_branch(scope, scenario.id, absent) {
        return Ok(false);
    }
    let evidence = b.evidence;
    let parent = &evidence.rows()[scope.ordinal() as usize];
    b.charge(parent.children().len())?;
    // Keep the first admitted slice conservative for writes outside this lane.
    if parent.children().iter().any(|id| {
        let row = &evidence.rows()[id.ordinal() as usize];
        value(row, "name") == Some(selector.name.as_str()) && lane(row) != Some(selector.lane)
    }) {
        return Ok(false);
    }
    let (index, attribute) = b.attributes[origin.ordinal() as usize]["number"];
    b.charge(attribute.raw().len())?;
    let decision = recipe.decide(&[ValueCandidate {
        selector,
        origin: SourceAttributeRef {
            occurrence: origin,
            index,
        },
        value: match attribute.decoded() {
            Ok(value) => CandidateValue::Decoded(value),
            Err(error) => CandidateValue::Unavailable(error),
        },
    }])?;
    Ok(matches!(
        decision.outcome,
        ValueOutcome::Selected {
            value: ParameterValue::Integer(_) | ParameterValue::Quantity(_),
            ..
        }
    ))
}

pub(in crate::owned_normalize) fn account(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    policy: Option<&CompiledConfigurationDispositions>,
    adapters: AdapterReceipts<'_>,
) -> Result<()> {
    if policy.is_none() && adapters.inputs.is_empty() {
        return Ok(());
    }
    if let Some(policy) = policy {
        b.charge(policy.work)?;
    }
    let Some(scopes) = fresh_config_sets(b)? else {
        return Ok(());
    };
    let mut retirements = Vec::new();
    for scope in scopes {
        b.charge(
            b.origins[scope.ordinal() as usize]
                .links
                .len()
                .saturating_mul(draft.scenario_presets.members.len().saturating_add(4))
                .saturating_add(draft.choice_presets.members.len()),
        )?;
        let links = &b.origins[scope.ordinal() as usize].links;
        let mut scenarios = draft
            .scenario_presets
            .members
            .iter()
            .filter(|s| links.contains(&OwnedOriginTarget::ScenarioPreset(s.id)));
        let Some(scenario) = scenarios.next() else {
            continue;
        };
        if scenarios.next().is_some() {
            continue;
        }
        let Some((issue, choice)) =
            configuration_issue(links, scenario.id, &draft.choice_presets.members)
        else {
            continue;
        };
        let receipts = receipts::ScalarReceipts::collect(b, scope, scenario, choice, &adapters)?;
        // Existing raw-only projection authority still uses this common owner.
        for receipt in receipts.iter() {
            b.charge(policy.map_or(0, |p| p.rules.len()))?;
            let governed = policy.is_some_and(|p| {
                p.rules
                    .iter()
                    .any(|r| receipt.selector.as_ref() == Some(&r.selector))
            });
            if receipt.source_state == ConfigurationSourceState::OverwrittenRawPlaceholder
                && !governed
                && let Some(origin) = receipt.origin
            {
                // A missing usage output is not proof that the source has no
                // usage responsibility. Shared leaves need explicit accounting.
                if let (Some(usage), Some(selector)) =
                    (adapters.usage_policy, receipt.selector.as_ref())
                    && usage.declares_configuration_selector(b, selector)?
                {
                    continue;
                }
                b.charge(b.origins[origin.ordinal() as usize].links.len())?;
                // The raw-only adapter's pre-existing authority says nothing
                // about another adapter already consuming this same leaf.
                if b.origins[origin.ordinal() as usize].links.as_slice()
                    != [OwnedOriginTarget::Issue(issue)]
                {
                    continue;
                }
                apply(
                    b,
                    scope,
                    origin,
                    issue,
                    std::slice::from_ref(&receipt.target),
                    receipt.source_only.as_ref(),
                )?;
            }
        }
        let Some(policy) = policy else {
            continue;
        };
        let mut receipts = receipts.finish(
            b,
            draft,
            &adapters,
            policy
                .inventory
                .as_ref()
                .is_some_and(|inventory| inventory.requests_strict_empty_custom_modifiers()),
        )?;
        let evidence = b.evidence;
        let children = evidence.rows()[scope.ordinal() as usize].children();
        b.charge(children.len().saturating_mul(policy.rules.len()))?;
        for rule in &policy.rules {
            for origin in children {
                let source = &evidence.rows()[origin.ordinal() as usize];
                if value(source, "name") != Some(rule.selector.name.as_str())
                    || lane(source) != Some(rule.selector.lane)
                {
                    continue;
                }
                let mut targets = Vec::new();
                let mut source_only = None;
                let mut contributes = false;
                let mut admitted = true;
                b.charge(rule.destinations.len())?;
                for destination in &rule.destinations {
                    match destination {
                        CompiledDestination::Deferred(responsibility) => {
                            // A deferred role is deliberately not a completion receipt.
                            let _ = responsibility;
                            admitted = false;
                        }
                        CompiledDestination::Inapplicable {
                            recipe,
                            absent,
                            reason,
                        } => {
                            admitted &= branch_proven(
                                b,
                                *origin,
                                scenario,
                                &receipts,
                                &rule.selector,
                                recipe,
                                absent,
                            )?;
                            source_only = Some(reason.clone());
                            targets.push(OwnedOriginTarget::ScenarioPreset(scenario.id));
                        }
                        CompiledDestination::Adapter(control) => {
                            b.charge(receipts.len())?;
                            let mut matches =
                                receipts.matching_leaf(scope, *origin, &rule.selector, control);
                            if let Some(receipt) = matches.next() {
                                if matches.next().is_some() {
                                    admitted = false;
                                }
                                if !targets.contains(&receipt.target) {
                                    targets.push(receipt.target.clone());
                                }
                                b.charge(receipt.additional_targets.len())?;
                                for target in &receipt.additional_targets {
                                    if !targets.contains(target) {
                                        targets.push(target.clone());
                                    }
                                }
                                if let Some(reason) = &receipt.source_only {
                                    if source_only.as_ref().is_some_and(|old| old != reason) {
                                        admitted = false;
                                    }
                                    source_only = Some(reason.clone());
                                } else {
                                    contributes = true;
                                }
                            } else {
                                admitted = false;
                            }
                        }
                    }
                }
                if admitted
                    && apply(
                        b,
                        scope,
                        *origin,
                        issue,
                        &targets,
                        if contributes {
                            None
                        } else {
                            source_only.as_ref()
                        },
                    )?
                    && policy.inventory.is_some()
                    && let [CompiledDestination::Inapplicable { reason, .. }] =
                        rule.destinations.as_slice()
                {
                    b.charge(
                        rule.id
                            .as_str()
                            .len()
                            .saturating_add(rule.selector.name.len())
                            .saturating_add(reason.as_str().len()),
                    )?;
                    receipts.retain_inapplicable(CheckedInapplicable {
                        scope,
                        origin: *origin,
                        scenario: scenario.id,
                        rule: rule.id.clone(),
                        selector: rule.selector.clone(),
                        reason: reason.clone(),
                    });
                }
            }
        }
        if let Some(receipt) = receipts.custom_modifiers() {
            apply_custom_modifiers(b, scope, scenario.id, issue, receipt)?;
        }
        if let Some(inventory) = &policy.inventory
            && let Some(proof) = inventory::prepare(
                b,
                draft,
                inventory::Scope {
                    source: scope,
                    scenario,
                    choice,
                    issue,
                },
                inventory,
                &receipts,
            )?
        {
            retirements.push(proof);
        }
    }
    inventory::commit(b, draft, retirements)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn instance(local: u64) -> InstanceId {
        InstanceId::from_parts(BuildLineage::from_bytes([27; 16]), local).unwrap()
    }

    #[test]
    fn accounting_requires_unique_attached_configuration_owner_and_scenario() {
        let scenario = ScenarioPresetId::from_instance_id(instance(1));
        let choice = ChoicePresetId::from_instance_id(instance(2));
        let issue = DraftIssueId::from_instance_id(instance(3));
        let foreign = DraftIssueId::from_instance_id(instance(4));
        let owner = ChoicePresetDraft {
            id: choice,
            choices: DraftList {
                members: vec![],
                completion: DraftListCompletion::Pending {
                    id: issue,
                    code: key("configuration-roles-not-converted"),
                },
            },
            rewards: complete(vec![]),
        };
        let original = vec![
            OwnedOriginTarget::ScenarioPreset(scenario),
            OwnedOriginTarget::ChoicePreset(choice),
            OwnedOriginTarget::Issue(issue),
        ];
        assert_eq!(
            configuration_issue(&original, scenario, std::slice::from_ref(&owner)),
            Some((issue, choice))
        );
        for mutation in 0..12 {
            let mut links = original.clone();
            let mut owners = vec![owner.clone()];
            match mutation {
                0 => {
                    links.remove(0);
                }
                1 => {
                    links[0] = OwnedOriginTarget::ScenarioPreset(
                        ScenarioPresetId::from_instance_id(instance(5)),
                    )
                }
                2 => links.push(links[0].clone()),
                3 => {
                    links.remove(1);
                }
                4 => links.push(links[1].clone()),
                5 => owners.clear(),
                6 => owners.push(owner.clone()),
                7 => owners[0].choices.completion = DraftListCompletion::Complete,
                8 => {
                    owners[0].choices.completion = DraftListCompletion::Pending {
                        id: issue,
                        code: key("other-pending"),
                    }
                }
                9 => {
                    owners[0].choices.completion = DraftListCompletion::Pending {
                        id: foreign,
                        code: key("configuration-roles-not-converted"),
                    }
                }
                10 => {
                    links.pop();
                }
                11 => links.push(OwnedOriginTarget::Issue(issue)),
                _ => unreachable!(),
            }
            assert_eq!(
                configuration_issue(&links, scenario, &owners),
                None,
                "mutation {mutation}"
            );
        }
    }
}
