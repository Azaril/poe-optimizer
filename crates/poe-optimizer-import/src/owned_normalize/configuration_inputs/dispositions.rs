//! Account only overwritten raw-override placeholders against actual outputs.
use super::*;

fn outputs_match(input: &ProvenInput, output: &[ExternalAssumptionDraft]) -> bool {
    let mut expected = vec![ExternalAssumptionDraft {
        input: input.presence_input.clone().into(),
        target: DraftAssumptionTarget::Enemy,
        value: ParameterValue::Boolean(input.raw.is_some()).into(),
    }];
    if let Some((_, value)) = &input.raw {
        expected.push(ExternalAssumptionDraft {
            input: input.value_input.clone().into(),
            target: DraftAssumptionTarget::Enemy,
            value: value.clone().into(),
        });
    }
    // Require exact typed values, order and multiplicity, including no raw value
    // on the absent branch. A row on another target cannot supply correspondence.
    output
        .iter()
        .filter(|row| {
            row.input
                .to_resolved()
                .is_some_and(|id| id == input.presence_input || id == input.value_input)
        })
        .count()
        == expected.len()
        && output.windows(expected.len()).any(|rows| rows == expected)
}

/// This proof concerns a saved scalar already excluded by the injected raw-only
/// recipe and its exact encounter. It gives no authority to fallback/default
/// recipes, other Placeholder names, containers, or complete inventories.
pub(in crate::owned_normalize) fn account(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: &ScenarioPresetDraft,
    choices: &[ChoicePresetDraft],
    proof: Option<&ProvenConfigurationInputs>,
) -> Result<()> {
    let Some(proof) = proof else { return Ok(()) };
    if proof.scope != scope
        || scenario.scenario.enemy.encounter.to_resolved().as_ref() != Some(&proof.encounter)
    {
        return Ok(());
    }
    b.charge(
        b.origins[scope.ordinal() as usize]
            .links
            .len()
            .saturating_mul(3)
            .saturating_add(choices.len()),
    )?;
    let Some(issue) = configuration_issue(
        &b.origins[scope.ordinal() as usize].links,
        scenario.id,
        choices,
    ) else {
        return Ok(());
    };
    let obligation = OwnedOriginTarget::Issue(issue);
    for input in &proof.inputs {
        b.charge(1)?;
        let Some(source) = input.overwritten_placeholder else {
            continue;
        };
        b.charge(
            scenario
                .scenario
                .assumptions
                .members
                .len()
                .saturating_mul(3)
                .saturating_add(b.origins[source.ordinal() as usize].links.len()),
        )?;
        if !outputs_match(input, &scenario.scenario.assumptions.members)
            || b.evidence.rows()[source.ordinal() as usize]
                .occurrence()
                .parent()
                != Some(scope)
        {
            continue;
        }
        if let Some((source, _)) = &input.raw {
            b.charge(
                b.origins[source.ordinal() as usize]
                    .links
                    .len()
                    .saturating_add(1),
            )?;
            if b.evidence.rows()[source.ordinal() as usize]
                .occurrence()
                .parent()
                != Some(scope)
                || b.origins[source.ordinal() as usize]
                    .links
                    .iter()
                    .filter(|link| **link == OwnedOriginTarget::ScenarioPreset(scenario.id))
                    .count()
                    != 1
            {
                continue;
            }
        }
        let origin = &mut b.origins[source.ordinal() as usize];
        if !matches!(origin.disposition, SourceDisposition::Contributes)
            || origin.links.as_slice() != [obligation.clone()]
        {
            continue;
        }
        // Preserve the exact scenario relationship without claiming that this
        // overwritten saved value contributes a native scalar or pending issue.
        origin.links.clear();
        origin.disposition = SourceDisposition::SourceOnly(key("overwritten-config-placeholder"));
        b.link(source, OwnedOriginTarget::ScenarioPreset(scenario.id))?;
    }
    Ok(())
}

fn configuration_issue(
    links: &[OwnedOriginTarget],
    scenario: ScenarioPresetId,
    choices: &[ChoicePresetDraft],
) -> Option<DraftIssueId> {
    let mut scenario_links = links.iter().filter_map(|link| match link {
        OwnedOriginTarget::ScenarioPreset(id) => Some(*id),
        _ => None,
    });
    if scenario_links.next() != Some(scenario) || scenario_links.next().is_some() {
        return None;
    }
    let mut choice_links = links.iter().filter_map(|link| match link {
        OwnedOriginTarget::ChoicePreset(id) => Some(*id),
        _ => None,
    });
    let choice = choice_links.next()?;
    if choice_links.next().is_some() {
        return None;
    }
    let mut owners = choices.iter().filter(|preset| preset.id == choice);
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
        .then_some(*id)
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
            Some(issue)
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

    #[test]
    fn accounting_checks_actual_presence_raw_value_target_multiplicity_and_order() {
        let ns = GameVersionNamespace::new("disposition-test", "v1").unwrap();
        let presence = ExternalInputDefId::new(ns.clone(), key("presence"));
        let raw = ExternalInputDefId::new(ns.clone(), key("raw"));
        let unit = UnitDefId::new(ns, key("unit"));
        let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
            crate::decode_build(b"<PathOfBuilding2/>").unwrap(),
            BuildLineage::from_bytes([27; 16]),
            Default::default(),
        )
        .unwrap();
        let source = imported.occurrences()[0].id();
        let raw_value = ParameterValue::Quantity(FiniteQuantity::new(0., unit).unwrap());
        for authored in [false, true] {
            let input = ProvenInput {
                presence_input: presence.clone(),
                value_input: raw.clone(),
                raw: authored.then(|| (source, raw_value.clone())),
                overwritten_placeholder: Some(source),
            };
            let presence_row = ExternalAssumptionDraft {
                input: presence.clone().into(),
                target: DraftAssumptionTarget::Enemy,
                value: ParameterValue::Boolean(authored).into(),
            };
            let raw_row = ExternalAssumptionDraft {
                input: raw.clone().into(),
                target: DraftAssumptionTarget::Enemy,
                value: raw_value.clone().into(),
            };
            let expected = if authored {
                vec![presence_row, raw_row.clone()]
            } else {
                vec![presence_row]
            };
            assert!(outputs_match(&input, &expected));
            for mutation in 0..8 {
                let mut output = expected.clone();
                match mutation {
                    0 => output.clear(),
                    1 => output[0].value = ParameterValue::Boolean(!authored).into(),
                    2 => output[0].target = DraftAssumptionTarget::Environment,
                    3 => output.push(output[0].clone()),
                    4 => output.push(raw_row.clone()),
                    5 => output[0].input = raw.clone().into(),
                    6 if authored => {
                        output.pop();
                    }
                    6 => output[0].value = raw_value.clone().into(),
                    7 if authored => output.reverse(),
                    7 => {
                        output[0].input =
                            ExternalInputDefId::new(presence.namespace().clone(), key("other"))
                                .into()
                    }
                    _ => unreachable!(),
                }
                assert!(
                    !outputs_match(&input, &output),
                    "authored {authored}, mutation {mutation}"
                );
            }
            if authored {
                let mut output = expected;
                output[1].value = ParameterValue::Boolean(false).into();
                assert!(!outputs_match(&input, &output));
            }
        }
    }
}
