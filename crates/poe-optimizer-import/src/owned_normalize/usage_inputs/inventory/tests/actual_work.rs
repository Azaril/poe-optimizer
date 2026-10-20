//! Consumer work accounting only; these fixtures do not mint source authority.
use super::*;

fn original_usage_present(preset: &SkillPresetDraft, binding: &PresetUsageBindingDraft) -> bool {
    if let Some(intent) = &preset.intent {
        intent
            .usage
            .members
            .iter()
            .filter(|row| *row == binding)
            .count()
            == 1
    } else {
        binding.applicability == PresetApplicability::Required
            && preset.usage_preferences.as_ref().is_some_and(|rows| {
                rows.members
                    .iter()
                    .filter(|row| **row == binding.selection)
                    .count()
                    == 1
            })
    }
}

#[test]
fn actual_binding_work_preserves_exact_duplicate_and_stale_value_semantics() {
    for mutation in 0..10 {
        with_two_usage_presets(|b, case| {
            let mut preset = case.draft.skill_presets.members[0].clone();
            let mut binding = case.receipts.usage[0].binding.clone();
            match mutation {
                0 => {}
                1 => {
                    binding.selection.target =
                        case.receipts.usage[1].binding.selection.target.clone()
                }
                2 => {
                    binding.selection.parameters.members[0].value =
                        ParameterValue::Boolean(false).into()
                }
                3 => {
                    binding.selection.parameters.completion = DraftListCompletion::Pending {
                        id: case.issue,
                        code: key("unreviewed-parameters"),
                    }
                }
                4 => preset
                    .intent
                    .as_mut()
                    .unwrap()
                    .usage
                    .members
                    .push(binding.clone()),
                5 => preset.intent.as_mut().unwrap().usage.members.clear(),
                6 => binding
                    .selection
                    .parameters
                    .members
                    .push(binding.selection.parameters.members[0].clone()),
                7 => binding.applicability = PresetApplicability::WhenExactSourceSelected,
                8 | 9 => {
                    let intent = preset.intent.take().unwrap();
                    preset.usage_preferences = Some(DraftList {
                        members: intent
                            .usage
                            .members
                            .into_iter()
                            .map(|row| row.selection)
                            .collect(),
                        completion: intent.usage.completion,
                    });
                    if mutation == 9 {
                        binding.applicability = PresetApplicability::WhenExactSourceSelected;
                    }
                }
                _ => unreachable!(),
            }
            let expected = original_usage_present(&preset, &binding);
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            assert_eq!(
                usage_present(b, &preset, &binding).unwrap(),
                expected,
                "mutation {mutation}"
            );
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
        });
    }
}

#[test]
fn actual_parameter_lengths_have_exact_precomparison_budget_boundaries() {
    for count in [0, 1, 2, 64] {
        for sufficient in [true, false] {
            with_two_usage_presets(|b, case| {
                let mut preset = case.draft.skill_presets.members[0].clone();
                let mut binding = case.receipts.usage[0].binding.clone();
                let parameter = binding.selection.parameters.members[0].clone();
                // Vector-comparison fixture only: repeated slots do not establish
                // a valid policy or completion authority.
                binding.selection.parameters.members = vec![parameter; count];
                preset.intent.as_mut().unwrap().usage.members = vec![binding.clone()];
                let before = b.work;
                let cost = count + 1;
                b.limits.max_work = before + cost - usize::from(!sufficient);
                let origins = b.origins.clone();
                let allocator = b.allocator.state();
                let result = usage_present(b, &preset, &binding);
                if sufficient {
                    assert!(result.unwrap());
                } else {
                    assert!(matches!(result, Err(NormalizationError::Limit(_))));
                }
                assert_eq!(b.work, before + cost);
                assert_eq!(b.origins, origins);
                assert_eq!(b.allocator.state(), allocator);
            });
        }
    }
}

fn original_fields_declared(receipt: &UsageReceipt, policy: &UsageInventoryRow) -> bool {
    receipt.fields.iter().all(|source_field| {
        policy.fields.iter().any(|field| {
            source_field == &field.field && field.destinations.iter().any(|destination| {
            matches!(destination, UsageInventoryDestination::UsageParameter { policy, parameter }
                if receipt.binding.selection.policy.to_resolved().as_ref() == Some(policy)
                    && &receipt.parameter == parameter)
        })
        })
    })
}

#[test]
fn actual_destination_scan_keeps_every_emitted_lane_conjunctive() {
    for mutation in 0..5 {
        with_two_usage_presets(|b, case| {
            let mut receipt = copy_usage_receipt(&case.receipts.usage[0]);
            let mut policy = case.rows[0].clone();
            match mutation {
                0 => {}
                1 => {
                    receipt
                        .fields
                        .insert(occurrence_field("unrepresented-lane"));
                }
                2 => policy.fields[0].destinations.clear(),
                3 => policy.fields[0]
                    .destinations
                    .retain(|d| !matches!(d, UsageInventoryDestination::UsageParameter { .. })),
                4 => {
                    policy.fields.push(UsageInventoryField {
                        field: occurrence_field("other-field"),
                        destinations: vec![UsageInventoryDestination::InputDisposition; 64],
                    });
                }
                _ => unreachable!(),
            }
            let expected = original_fields_declared(&receipt, &policy);
            let width: usize = policy
                .fields
                .iter()
                .map(|field| field.destinations.len() + 1)
                .sum();
            let cost = policy.fields.len() + receipt.fields.len() * width;
            let start = b.work;
            assert_eq!(
                emitted_fields_declared(b, &receipt, &policy).unwrap(),
                expected,
                "mutation {mutation}"
            );
            assert_eq!(b.work - start, cost);
        });
    }
}

fn with_multiple_parameters(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_two_usage_presets(|b, mut case| {
        let mut registry =
            OwnedIdRegistry::empty(b.roles.input().namespace.clone(), Default::default()).unwrap();
        let policy = registry
            .allocate_definition::<UsagePolicyDefinition>()
            .unwrap();
        let parameters: Vec<_> = (0..4)
            .map(|_| {
                registry
                    .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(
                        policy.clone(),
                    ))
                    .unwrap()
            })
            .collect();
        assert_eq!(parameters[0], case.receipts.usage[0].parameter);
        case.rows[0].fields[0]
            .destinations
            .retain(|d| !matches!(d, UsageInventoryDestination::UsageParameter { .. }));
        for parameter in &parameters {
            case.rows[0].fields[0]
                .destinations
                .push(UsageInventoryDestination::UsageParameter {
                    policy: policy.clone(),
                    parameter: parameter.clone(),
                });
        }
        let originals = std::mem::take(&mut case.receipts.usage);
        for (index, original) in originals.iter().enumerate() {
            let binding = &mut case.draft.skill_presets.members[index]
                .intent
                .as_mut()
                .unwrap()
                .usage
                .members[0];
            binding.selection.parameters.members = parameters
                .iter()
                .enumerate()
                .map(|(index, slot)| ParameterDraft {
                    slot: slot.clone().into(),
                    value: ParameterValue::Boolean(index % 2 == 0).into(),
                })
                .collect();
            for parameter in &parameters {
                let mut receipt = copy_usage_receipt(original);
                receipt.binding = binding.clone();
                receipt.parameter = parameter.clone();
                case.receipts.usage.push(receipt);
            }
        }
        test(b, case);
    });
}

#[test]
fn multiple_parameter_completion_preserves_full_inverse_and_atomic_budget() {
    let mut cost = 0;
    with_multiple_parameters(|b, mut case| {
        let mut expected = case.draft.clone();
        let mut origins = b.origins.clone();
        for preset in &mut expected.skill_presets.members {
            let usage = &mut preset.intent.as_mut().unwrap().usage;
            let DraftListCompletion::Pending { id, .. } = usage.completion else {
                panic!()
            };
            usage.completion = DraftListCompletion::Complete;
            for origin in &mut origins {
                origin
                    .links
                    .retain(|link| *link != OwnedOriginTarget::Issue(id));
            }
        }
        let start = b.work;
        finish(b, &mut case).unwrap();
        cost = b.work - start;
        assert_eq!(case.draft, expected);
        assert_eq!(b.origins, origins);
    });
    for sufficient in [true, false] {
        with_multiple_parameters(|b, mut case| {
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            b.limits.max_work = b.work + cost - usize::from(!sufficient);
            let result = finish(b, &mut case);
            if sufficient {
                result.unwrap();
                assert!(
                    case.draft
                        .skill_presets
                        .members
                        .iter()
                        .all(|p| matches!(completion(p), Some(DraftListCompletion::Complete)))
                );
            } else {
                assert!(matches!(result, Err(NormalizationError::Limit(_))));
                assert_eq!(case.draft, before);
                assert_eq!(b.origins, origins);
            }
            assert_eq!(b.allocator.state(), allocator);
        });
    }
    with_multiple_parameters(|b, mut case| {
        case.receipts.usage.remove(3);
        let before = case.draft.skill_presets.members[0].clone();
        finish(b, &mut case).unwrap();
        assert_eq!(case.draft.skill_presets.members[0], before);
        assert!(matches!(
            completion(&case.draft.skill_presets.members[1]),
            Some(DraftListCompletion::Complete)
        ));
    });
}
