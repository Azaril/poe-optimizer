//! Consumer/liveness checks; public tests mint the actual flat-primary proof.
use super::*;

fn with_flat_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_two_usage_presets(|b, mut case| {
        for (receipt, usage) in case.receipts.inputs.iter_mut().zip(&case.receipts.usage) {
            let InputOutput::Authored { scope, .. } = &mut receipt.output else {
                unreachable!()
            };
            *scope = AuthoredInputScope::PrimaryUsage {
                binding: Box::new(usage.binding.clone()),
            };
        }
        test(b, case);
    });
}

#[test]
fn flat_receipts_keep_primary_scope_separate_from_contributing_group_receipts() {
    with_flat_case(|b, mut case| {
        let allocator = b.allocator.state();
        let mut expected = case.draft.clone();
        let mut origins = b.origins.clone();
        for preset in &mut expected.skill_presets.members {
            let usage = &mut preset.intent.as_mut().unwrap().usage;
            let DraftListCompletion::Pending { id, .. } = usage.completion else {
                unreachable!()
            };
            usage.completion = DraftListCompletion::Complete;
            for origin in &mut origins {
                origin
                    .links
                    .retain(|link| *link != OwnedOriginTarget::Issue(id));
            }
        }
        finish(b, &mut case).unwrap();
        assert_eq!(case.draft, expected);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), allocator);
    });
    with_flat_case(|b, mut case| {
        for receipt in &case.receipts.inputs {
            b.origins[receipt.group.ordinal() as usize]
                .links
                .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_)));
        }
        let before = case.draft.clone();
        let origins = b.origins.clone();
        let allocator = b.allocator.state();
        for (receipt, preset) in case
            .receipts
            .inputs
            .iter()
            .zip(&case.draft.skill_presets.members)
        {
            assert!(
                input_present(b, &case.draft, preset, receipt).unwrap(),
                "flat primary input needs exact source/set ownership, not a redundant group link"
            );
        }
        finish(b, &mut case).unwrap();
        assert_eq!(
            case.draft, before,
            "this separate group-enabled usage consumer still requires its contributing group link"
        );
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), allocator);
    });
}

#[test]
fn flat_receipt_cannot_outlive_its_designated_binding_or_exact_source_ownership() {
    for change in 0..9 {
        with_flat_case(|b, mut case| {
            let first = &case.receipts.inputs[0];
            match change {
                0 => {
                    // Keep the ordinary usage receipt current, isolating the
                    // older enabling binding retained by the intrinsic proof.
                    let usage = &mut case.draft.skill_presets.members[0]
                        .intent
                        .as_mut()
                        .unwrap()
                        .usage
                        .members[0];
                    usage.selection.parameters.members[0].value =
                        ParameterValue::Boolean(false).into();
                    case.receipts.usage[0].binding = usage.clone();
                }
                1 => {
                    case.draft.skill_presets.members[0]
                        .intent
                        .as_mut()
                        .unwrap()
                        .usage
                        .members
                        .clear();
                }
                2 => {
                    let rows = &mut case.draft.skill_presets.members[0]
                        .intent
                        .as_mut()
                        .unwrap()
                        .usage
                        .members;
                    rows.push(rows[0].clone());
                }
                3 => b.origins[first.source.ordinal() as usize]
                    .links
                    .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_))),
                4 => {
                    let set = b.evidence.rows()[first.group.ordinal() as usize]
                        .occurrence()
                        .parent()
                        .unwrap();
                    b.origins[set.ordinal() as usize]
                        .links
                        .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_)));
                }
                5 => b.origins[first.group.ordinal() as usize].links.push(
                    OwnedOriginTarget::SkillPreset(case.draft.skill_presets.members[1].id),
                ),
                6 => {
                    let other = match &case.receipts.inputs[1].output {
                        InputOutput::Authored { scope, .. } => scope.clone(),
                        _ => unreachable!(),
                    };
                    let InputOutput::Authored { scope, .. } = &mut case.receipts.inputs[0].output
                    else {
                        unreachable!()
                    };
                    *scope = other;
                }
                7 | 8 => {
                    let origin = if change == 7 {
                        first.source
                    } else {
                        b.evidence.rows()[first.group.ordinal() as usize]
                            .occurrence()
                            .parent()
                            .unwrap()
                    };
                    let links = &mut b.origins[origin.ordinal() as usize].links;
                    links.retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_)));
                    links.push(OwnedOriginTarget::SkillPreset(
                        case.draft.skill_presets.members[1].id,
                    ));
                }
                _ => unreachable!(),
            }
            let before = case.draft.skill_presets.members[0].clone();
            finish(b, &mut case).unwrap();
            assert_eq!(
                case.draft.skill_presets.members[0], before,
                "change {change}"
            );
            assert!(matches!(
                case.draft.skill_presets.members[1]
                    .intent
                    .as_ref()
                    .unwrap()
                    .usage
                    .completion,
                DraftListCompletion::Complete
            ));
        });
    }
}

#[test]
fn flat_receipt_reference_scope_and_late_budget_failure_do_not_retire_any_issue() {
    let mut work = 0;
    with_flat_case(|b, mut case| {
        let start = b.work;
        finish(b, &mut case).unwrap();
        work = b.work - start;
    });
    for sufficient in [true, false] {
        with_flat_case(|b, mut case| {
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            b.limits.max_work = b.work + work - usize::from(!sufficient);
            let result = finish(b, &mut case);
            if sufficient {
                result.unwrap();
                assert!(
                    case.draft
                        .skill_presets
                        .members
                        .iter()
                        .all(|preset| matches!(
                            preset.intent.as_ref().unwrap().usage.completion,
                            DraftListCompletion::Complete
                        ))
                );
            } else {
                assert!(matches!(result, Err(NormalizationError::Limit(_))));
                assert_eq!(case.draft, before);
                assert_eq!(b.origins, origins);
            }
            assert_eq!(b.allocator.state(), allocator);
        });
    }
    with_flat_case(|b, mut case| {
        let group = case.receipts.inputs[0].group;
        b.origins[group.ordinal() as usize]
            .links
            .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_)));
        let InputOutput::Authored { scope, .. } = &mut case.receipts.inputs[0].output else {
            unreachable!()
        };
        *scope = AuthoredInputScope::ReferenceDisposition;
        let before = case.draft.skill_presets.members[0].clone();
        finish(b, &mut case).unwrap();
        assert_eq!(
            case.draft.skill_presets.members[0], before,
            "full reference scope still requires its actual group link"
        );
    });
}

#[test]
fn usage_origin_scopes_keep_exact_source_set_and_contributing_group_authority() {
    for group_contributes in [false, true] {
        for mutation in 0..9 {
            with_flat_case(|b, case| {
                let receipt = &case.receipts.usage[0];
                let other = &case.receipts.usage[1];
                let set = b.evidence.rows()[receipt.group.ordinal() as usize]
                    .occurrence()
                    .parent()
                    .unwrap();
                if !group_contributes {
                    b.origins[receipt.group.ordinal() as usize]
                        .links
                        .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_)));
                }
                let mut source = receipt.source;
                let mut group = receipt.group;
                match mutation {
                    0 => {}
                    1..=4 => {
                        let origin = if mutation <= 2 { source } else { set };
                        let links = &mut b.origins[origin.ordinal() as usize].links;
                        links.retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_)));
                        if mutation % 2 == 0 {
                            links.push(OwnedOriginTarget::SkillPreset(other.preset));
                        }
                    }
                    5 => b.origins[group.ordinal() as usize]
                        .links
                        .push(OwnedOriginTarget::SkillPreset(other.preset)),
                    6 => {
                        b.origins[group.ordinal() as usize]
                            .links
                            .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_)));
                    }
                    7 => group = other.group,
                    8 => source = other.source,
                    _ => unreachable!(),
                }
                let origins = b.origins.clone();
                let allocator = b.allocator.state();
                // These are the same origin precharges as the final receipt
                // consumer; the shared helper adds its containing-set work.
                for origin in [source, group] {
                    b.charge(b.origins[origin.ordinal() as usize].links.len() * 2)
                        .unwrap();
                }
                assert_eq!(
                    usage_origins_present(b, source, group, receipt.preset, group_contributes)
                        .unwrap(),
                    mutation == 0 || mutation == 6 && !group_contributes,
                    "group contributes {group_contributes}, mutation {mutation}"
                );
                assert_eq!(b.origins, origins);
                assert_eq!(b.allocator.state(), allocator);
            });
        }
    }
}
