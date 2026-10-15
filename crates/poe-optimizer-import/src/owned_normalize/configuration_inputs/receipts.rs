//! Private exact scalar receipts for the common configuration accounting owner.
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

fn default_output_matches(
    input: &defaults::ProvenDefault,
    output: &[ExternalAssumptionDraft],
) -> bool {
    let expected = ExternalAssumptionDraft {
        input: input.value_input.clone().into(),
        target: input.target.draft(),
        value: input.value.clone().into(),
    };
    let mut matching = output.iter().filter(|row| {
        row.input.to_resolved().as_ref() == Some(&input.value_input)
            && (input.projected || row.target == input.target.draft())
    });
    matching.next() == Some(&expected) && matching.next().is_none()
}

/// Private per-control receipts are issued only against the actual ordered
/// output, exact target, source scope and source links. They carry no inventory
/// completion authority. The common owner decides whether every role is covered.
pub(in crate::owned_normalize) fn checked_receipts(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: &ScenarioPresetDraft,
    proof: Option<&ProvenConfigurationInputs>,
) -> Result<Vec<configuration_dispositions::CheckedConfigurationReceipt>> {
    use configuration_dispositions::{CheckedConfigurationReceipt, ConfigurationSourceState};
    let mut receipts = Vec::new();
    let Some(proof) = proof else {
        return Ok(receipts);
    };
    if proof.scope != scope
        || scenario.scenario.enemy.encounter.to_resolved().as_ref() != Some(&proof.encounter)
    {
        return Ok(receipts);
    }
    let target = OwnedOriginTarget::ScenarioPreset(scenario.id);
    let linked = |b: &Builder<'_, '_>, origin: SourceOccurrenceId| {
        b.evidence.rows()[origin.ordinal() as usize]
            .occurrence()
            .parent()
            == Some(scope)
            && b.origins[origin.ordinal() as usize]
                .links
                .iter()
                .filter(|link| **link == target)
                .count()
                == 1
    };
    for input in &proof.inputs {
        b.charge(
            scenario
                .scenario
                .assumptions
                .members
                .len()
                .saturating_mul(3)
                .saturating_add(input.source_name.len())
                .saturating_add(1),
        )?;
        if !outputs_match(input, &scenario.scenario.assumptions.members) {
            continue;
        }
        if let Some((origin, _)) = &input.raw {
            b.charge(
                b.origins[origin.ordinal() as usize]
                    .links
                    .len()
                    .saturating_add(1),
            )?;
            if !linked(b, *origin) {
                continue;
            }
        }
        let control = ConfigurationAdapterControl::ScalarInput {
            input: input.value_input.clone(),
        };
        receipts.push(CheckedConfigurationReceipt {
            scope,
            origin: input.raw.as_ref().map(|(origin, _)| *origin),
            selector: Some(ValueSelector {
                lane: input.selected_lane,
                name: input.source_name.clone(),
            }),
            control: control.clone(),
            source_state: if input.raw.is_none() {
                ConfigurationSourceState::Absent
            } else if input.selected_lane == ValueLane::PlaceholderNumber {
                ConfigurationSourceState::Placeholder
            } else {
                ConfigurationSourceState::Authored
            },
            target: target.clone(),
            source_only: None,
        });
        if let Some(origin) = input.overwritten_placeholder {
            receipts.push(CheckedConfigurationReceipt {
                scope,
                origin: Some(origin),
                selector: Some(ValueSelector {
                    lane: ValueLane::PlaceholderNumber,
                    name: input.source_name.clone(),
                }),
                control,
                source_state: ConfigurationSourceState::OverwrittenRawPlaceholder,
                target: target.clone(),
                source_only: Some(key("overwritten-config-placeholder")),
            });
        }
    }
    for input in &proof.defaults {
        b.charge(
            scenario
                .scenario
                .assumptions
                .members
                .len()
                .saturating_add(input.selector.name.len())
                .saturating_add(b.origins[input.origin.ordinal() as usize].links.len())
                .saturating_add(1),
        )?;
        if !default_output_matches(input, &scenario.scenario.assumptions.members) {
            continue;
        }
        if input.constructor {
            if input.origin != scope
                || b.origins[scope.ordinal() as usize]
                    .links
                    .iter()
                    .filter(|link| **link == target)
                    .count()
                    != 1
            {
                continue;
            }
        } else if !linked(b, input.origin) {
            continue;
        }
        let control = ConfigurationAdapterControl::ScalarInput {
            input: input.value_input.clone(),
        };
        receipts.push(CheckedConfigurationReceipt {
            scope,
            origin: (!input.constructor).then_some(input.origin),
            selector: Some(input.selector.clone()),
            control: control.clone(),
            source_state: if input.constructor {
                ConfigurationSourceState::ConstructorDefault
            } else if input.selector.lane == ValueLane::PlaceholderNumber {
                ConfigurationSourceState::Placeholder
            } else {
                ConfigurationSourceState::Authored
            },
            target: target.clone(),
            source_only: None,
        });
        if let Some(origin) = input.ignored_placeholder {
            receipts.push(CheckedConfigurationReceipt {
                scope,
                origin: Some(origin),
                selector: Some(ValueSelector {
                    lane: ValueLane::PlaceholderNumber,
                    name: input.selector.name.clone(),
                }),
                control,
                source_state: ConfigurationSourceState::IgnoredDefaultPlaceholder,
                target: target.clone(),
                source_only: Some(key("unused-config-placeholder")),
            });
        }
        for (origin, selector) in &input.unselected {
            b.charge(selector.name.len().saturating_add(1))?;
            if b.evidence.rows()[origin.ordinal() as usize]
                .occurrence()
                .parent()
                != Some(scope)
            {
                continue;
            }
            receipts.push(CheckedConfigurationReceipt {
                scope,
                origin: Some(*origin),
                selector: Some(selector.clone()),
                control: ConfigurationAdapterControl::ScalarInput {
                    input: input.value_input.clone(),
                },
                source_state: ConfigurationSourceState::UnselectedProjection,
                target: target.clone(),
                source_only: Some(key("unselected-config-projection")),
            });
        }
    }
    Ok(receipts)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_receipts_require_exact_output_value_target_and_multiplicity() {
        let ns = GameVersionNamespace::new("projection-test", "v1").unwrap();
        let id = ExternalInputDefId::new(ns.clone(), key("output"));
        let other = ExternalInputDefId::new(ns.clone(), key("other"));
        let unit = UnitDefId::new(ns, key("unit"));
        let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
            crate::decode_build(b"<PathOfBuilding2/>").unwrap(),
            BuildLineage::from_bytes([28; 16]),
            Default::default(),
        )
        .unwrap();
        for number in [0., 80.] {
            let value =
                ParameterValue::Quantity(FiniteQuantity::new(number, unit.clone()).unwrap());
            let input = defaults::ProvenDefault {
                selector: ValueSelector {
                    lane: ValueLane::InputNumber,
                    name: "control".into(),
                },
                constructor: false,
                ignored_placeholder: None,
                value_input: id.clone(),
                target: ConfigurationInputTarget::Enemy,
                origin: imported.occurrences()[0].id(),
                value: value.clone(),
                unselected: vec![],
                projected: true,
            };
            let expected = ExternalAssumptionDraft {
                input: id.clone().into(),
                target: DraftAssumptionTarget::Enemy,
                value: value.into(),
            };
            assert!(default_output_matches(
                &input,
                std::slice::from_ref(&expected)
            ));
            for mutation in 0..7 {
                let mut actual = vec![expected.clone()];
                match mutation {
                    0 => actual.clear(),
                    1 => {
                        actual[0].value = ParameterValue::Quantity(
                            FiniteQuantity::new(number + 1., unit.clone()).unwrap(),
                        )
                        .into()
                    }
                    2 => actual[0].target = DraftAssumptionTarget::Environment,
                    3 => actual.push(expected.clone()),
                    4 => {
                        let mut alias = expected.clone();
                        alias.target = DraftAssumptionTarget::Environment;
                        actual.push(alias);
                    }
                    5 => actual[0].input = other.clone().into(),
                    6 => actual[0].value = ParameterValue::Boolean(false).into(),
                    _ => unreachable!(),
                }
                assert!(
                    !default_output_matches(&input, &actual),
                    "value {number}, mutation {mutation}"
                );
            }
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
                source_name: String::from("test-raw"),
                selected_lane: ValueLane::InputNumber,
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
