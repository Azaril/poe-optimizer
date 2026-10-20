use super::*;
mod actual_work;
mod group_selectors;
mod primary_completion;
mod reference_absence;

const SOURCE: &str = r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true"><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#;

fn blank(b: &Builder<'_, '_>) -> DraftSessionInput {
    DraftSessionInput {
        allocator: b.allocator.state(),
        revision: b.evidence.identity().revision,
        game_version: b.roles.input().namespace.clone(),
        weapon_loadouts: super::super::super::complete(vec![]),
        items: super::super::super::complete(vec![]),
        gems: super::super::super::complete(vec![]),
        rewards: super::super::super::complete(vec![]),
        equipment: super::super::super::complete(vec![]),
        allocations: super::super::super::complete(vec![]),
        skills: super::super::super::complete(vec![]),
        supports: super::super::super::complete(vec![]),
        payload_links: super::super::super::complete(vec![]),
        character_presets: super::super::super::complete(vec![]),
        equipment_presets: super::super::super::complete(vec![]),
        allocation_presets: super::super::super::complete(vec![]),
        skill_presets: super::super::super::complete(vec![]),
        choice_presets: super::super::super::complete(vec![]),
        scenario_presets: super::super::super::complete(vec![]),
        query_presets: super::super::super::complete(vec![]),
        saved_variants: super::super::super::complete(vec![]),
    }
}

struct Case {
    draft: DraftSessionInput,
    receipts: InventoryReceipts,
    rows: Vec<UsageInventoryRow>,
    presets: BTreeMap<SourceOccurrenceId, usize>,
    source: SourceOccurrenceId,
    issue: DraftIssueId,
}

fn with_case(source_xml: &str, test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    gem_inventory::with_usage_inventory_destination(
        source_xml,
        |b, skill, mut preset, gem, source| {
            let DraftAuthoredSkillSource::Gem(DraftField::Known { value: physical }) =
                &skill.source
            else {
                unreachable!()
            };
            let gem_record = GemDraft {
                id: *physical,
                definition: gem.clone().into(),
                parameters: DraftList {
                    completion: DraftListCompletion::Complete,
                    members: vec![],
                },
                level: 17.into(),
                quality: DraftQuality::Known { value: None },
            };
            let mut receipts = InventoryReceipts::default();
            let reference = skill_input_disposition::CheckedSourceReference::consumer_fixture(
                source,
                b.evidence.rows()[source.ordinal() as usize]
                    .occurrence()
                    .parent(),
                vec![],
            );
            record_input_disposition(
                b,
                source,
                &preset,
                &skill,
                Some(&gem_record),
                AuthoredInputProof::ReferenceDisposition(&reference),
                &mut receipts,
            )
            .unwrap();
            let usage = preset.usage_preferences.take().unwrap();
            let DraftListCompletion::Pending { id: issue, .. } = usage.completion else {
                unreachable!()
            };
            preset.intent = Some(SkillPresetIntentDraftV1 {
                schema_version: 1,
                usage: DraftList {
                    completion: usage.completion,
                    members: vec![],
                },
                generated_inputs: b
                    .closure(source, "generated-skill-inputs-not-converted", vec![])
                    .unwrap(),
            });
            let set = b.ancestor(source, "SkillSet").unwrap().unwrap();
            let mut draft = blank(b);
            draft.gems.members.push(gem_record);
            draft.skills.members.push(skill);
            draft.skill_presets.members.push(preset);
            let rows = vec![UsageInventoryRow {
                source: UsageInventorySource::Physical { gem },
                fields: vec![
                    UsageInventoryField {
                        field: group_field("enabled"),
                        destinations: vec![UsageInventoryDestination::InputDisposition],
                    },
                    UsageInventoryField {
                        field: UsageInventoryFieldSelector::ReferenceDescendants,
                        destinations: vec![UsageInventoryDestination::InputDisposition],
                    },
                ],
            }];
            test(
                b,
                Case {
                    draft,
                    receipts,
                    rows,
                    presets: [(set, 0)].into(),
                    source,
                    issue,
                },
            );
        },
    );
}

fn finish(b: &mut Builder<'_, '_>, case: &mut Case) -> Result<()> {
    complete(
        b,
        &mut case.draft,
        &case.presets,
        Some(&CompiledInventory { rows: &case.rows }),
        &case.receipts,
    )
}

fn require_primary_gate(case: &mut Case) {
    case.rows[0].fields.push(UsageInventoryField {
        field: occurrence_field("enableGlobal1"),
        destinations: vec![UsageInventoryDestination::InputDisposition],
    });
}

#[test]
fn refused_preset_stops_receipt_work_after_the_first_unproved_consumer() {
    let mut costs = Vec::new();
    for occurrences in [1, 64] {
        let source_xml = SOURCE.replacen("<Gem/>", &"<Gem/>".repeat(occurrences), 1);
        with_case(&source_xml, |b, mut case| {
            case.rows[0].fields[0]
                .destinations
                .push(UsageInventoryDestination::Deferred {
                    reason: key("another-required-consumer"),
                });
            // The whole immutable source has already been checked. Extra tail
            // rows remain present, but cannot rescue this missing consumer.
            assert_eq!(skill_source_census::sets(b).unwrap().unwrap().len(), 2);
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            let start = b.work;
            finish(b, &mut case).unwrap();
            costs.push(b.work - start);
            assert_eq!(case.draft, before);
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
        });
    }
    assert_eq!(
        costs[1] - costs[0],
        63,
        "only the existing group-row precharge grows; refused tails do not rescan receipts"
    );
}

fn with_two_accountable_presets(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_accountable_presets(2, test);
}

fn with_accountable_presets(count: usize, test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    assert!(count >= 2);
    let sets: String = (1..=count)
        .map(|id| format!(r#"<SkillSet id="{id}"><Skill enabled="true"><Gem/></Skill></SkillSet>"#))
        .collect();
    let source_xml =
        format!(r#"<PathOfBuilding2><Skills activeSkillSet="1">{sets}</Skills></PathOfBuilding2>"#);
    with_case(&source_xml, |b, mut case| {
        let sources: Vec<_> = b
            .evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "Gem")
            .skip(1)
            .map(|row| row.occurrence().id())
            .collect();
        for source in sources {
            let group = b.evidence.rows()[source.ordinal() as usize]
                .occurrence()
                .parent()
                .unwrap();
            let set = b.evidence.rows()[group.ordinal() as usize]
                .occurrence()
                .parent()
                .unwrap();
            let gem_id: GemInstanceId = b.id().unwrap();
            let skill_id = b.id().unwrap();
            let preset_id = b.id().unwrap();
            let mut gem = case.draft.gems.members[0].clone();
            gem.id = gem_id;
            let mut skill = case.draft.skills.members[0].clone();
            skill.id = skill_id;
            skill.source = DraftAuthoredSkillSource::Gem(gem_id.into());
            let mut preset = case.draft.skill_presets.members[0].clone();
            preset.id = preset_id;
            preset.skills.members = vec![skill_id];
            let usage = b
                .closure(source, "usage-preferences-not-converted", vec![])
                .unwrap();
            let DraftListCompletion::Pending { id: issue, .. } = usage.completion else {
                unreachable!()
            };
            preset.intent = Some(SkillPresetIntentDraftV1 {
                schema_version: 1,
                usage,
                generated_inputs: super::super::super::complete(vec![]),
            });
            for origin in [set, group, source] {
                b.link(origin, OwnedOriginTarget::SkillPreset(preset_id))
                    .unwrap();
            }
            for origin in [set, group] {
                b.link(origin, OwnedOriginTarget::Issue(issue)).unwrap();
            }
            b.link(source, OwnedOriginTarget::Gem(gem_id)).unwrap();
            b.link(source, OwnedOriginTarget::Skill(skill_id)).unwrap();
            let reference = skill_input_disposition::CheckedSourceReference::consumer_fixture(
                source,
                b.evidence.rows()[source.ordinal() as usize]
                    .occurrence()
                    .parent(),
                vec![],
            );
            record_input_disposition(
                b,
                source,
                &preset,
                &skill,
                Some(&gem),
                AuthoredInputProof::ReferenceDisposition(&reference),
                &mut case.receipts,
            )
            .unwrap();
            case.draft.gems.members.push(gem);
            case.draft.skills.members.push(skill);
            case.presets
                .insert(set, case.draft.skill_presets.members.len());
            case.draft.skill_presets.members.push(preset);
        }
        assert_eq!(case.receipts.inputs.len(), count);
        case.draft.allocator = b.allocator.state();
        test(b, case);
    });
}

fn copy_authored_receipt(receipt: &InputReceipt) -> InputReceipt {
    let InputOutput::Authored { skill, gem, scope } = &receipt.output else {
        unreachable!("this fixture has only checked authored receipts")
    };
    InputReceipt {
        source: receipt.source,
        group: receipt.group,
        preset: receipt.preset,
        identity: receipt.identity.clone(),
        output: InputOutput::Authored {
            skill: skill.clone(),
            gem: gem.clone(),
            scope: scope.clone(),
        },
        fields: receipt.fields.clone(),
        descendants: receipt.descendants.clone(),
    }
}

#[test]
fn complete_presets_do_not_multiply_unrelated_receipts_by_whole_draft_scans() {
    let mut costs = Vec::new();
    for count in [16, 32] {
        with_accountable_presets(count, |b, mut case| {
            let before = case.draft.clone();
            let mut expected = before.clone();
            let mut expected_origins = b.origins.clone();
            for preset in &mut expected.skill_presets.members {
                let usage = &mut preset.intent.as_mut().unwrap().usage;
                let DraftListCompletion::Pending { id, .. } = usage.completion else {
                    unreachable!()
                };
                for origin in &mut expected_origins {
                    origin
                        .links
                        .retain(|link| *link != OwnedOriginTarget::Issue(id));
                }
                usage.completion = DraftListCompletion::Complete;
            }
            let allocator = b.allocator.state();
            let start = b.work;
            finish(b, &mut case).unwrap();
            costs.push(b.work - start);
            assert_eq!(case.draft, expected);
            assert_eq!(b.origins, expected_origins);
            assert_eq!(b.allocator.state(), allocator);
        });
    }
    assert!(
        costs[1] < costs[0] * 4,
        "complete-preset work should retain bounded scans, not a receipts × draft × source product: {costs:?}"
    );
}

#[test]
fn unmatched_input_receipts_pay_only_the_exact_identity_scan() {
    let mut costs = Vec::new();
    for extra in [0, 64] {
        with_two_accountable_presets(|b, mut case| {
            for _ in 0..extra {
                let mut unrelated = copy_authored_receipt(&case.receipts.inputs[0]);
                // Both identities are real, but this combined source/preset
                // pair cannot witness either preset's exact occurrence.
                unrelated.preset = case.draft.skill_presets.members[1].id;
                case.receipts.inputs.push(unrelated);
            }
            let start = b.work;
            finish(b, &mut case).unwrap();
            costs.push(b.work - start);
            assert!(case.draft.skill_presets.members.iter().all(|preset| {
                matches!(
                    preset.intent.as_ref().unwrap().usage.completion,
                    DraftListCompletion::Complete
                )
            }));
        });
    }
    assert_eq!(costs[1] - costs[0], 2 * 64);
}

#[test]
fn matching_input_receipts_still_reject_stale_outputs_and_wrong_recipients() {
    for mutation in 0..6 {
        with_two_accountable_presets(|b, mut case| {
            let first = copy_authored_receipt(&case.receipts.inputs[0]);
            let other = copy_authored_receipt(&case.receipts.inputs[1]);
            match mutation {
                0 => case.receipts.inputs[0].preset = other.preset,
                1 => case.receipts.inputs[0].source = other.source,
                2 => case.receipts.inputs[0].group = other.group,
                3 => {
                    case.receipts.inputs[0].output =
                        copy_authored_receipt(&case.receipts.inputs[1]).output
                }
                4 => {
                    let InputOutput::Authored { skill, .. } = &mut case.receipts.inputs[0].output
                    else {
                        unreachable!()
                    };
                    // The same live ID with altered data cannot use its receipt.
                    skill.enabled = false.into();
                }
                _ => {
                    b.origins[first.source.ordinal() as usize]
                        .links
                        .push(OwnedOriginTarget::SkillPreset(other.preset));
                }
            }
            let mut expected = case.draft.clone();
            let mut expected_origins = b.origins.clone();
            let second = &mut expected.skill_presets.members[1]
                .intent
                .as_mut()
                .unwrap()
                .usage;
            let DraftListCompletion::Pending { id, .. } = second.completion else {
                unreachable!()
            };
            second.completion = DraftListCompletion::Complete;
            for origin in &mut expected_origins {
                origin
                    .links
                    .retain(|link| *link != OwnedOriginTarget::Issue(id));
            }
            let allocator = b.allocator.state();
            finish(b, &mut case).unwrap();
            assert_eq!(case.draft, expected, "mutation {mutation}");
            assert_eq!(b.origins, expected_origins, "mutation {mutation}");
            assert_eq!(b.allocator.state(), allocator, "mutation {mutation}");
        });
    }
}

// These checked-shape fixtures exercise receipt consumption, not proof minting.
// Public owned_usage_inventory tests cover the actual usage adapter producer.
fn with_two_usage_presets(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_two_accountable_presets(|b, mut case| {
        let mut registry =
            OwnedIdRegistry::empty(b.roles.input().namespace.clone(), Default::default()).unwrap();
        let policy = registry
            .allocate_definition::<UsagePolicyDefinition>()
            .unwrap();
        let parameter = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(policy.clone()))
            .unwrap();
        case.rows[0].fields[0]
            .destinations
            .push(UsageInventoryDestination::UsageParameter {
                policy: policy.clone(),
                parameter: parameter.clone(),
            });
        for (input, preset) in case
            .receipts
            .inputs
            .iter()
            .zip(&mut case.draft.skill_presets.members)
        {
            let binding = PresetUsageBindingDraft {
                selection: UsagePolicyDraft {
                    policy: policy.clone().into(),
                    target: DraftUsageTarget::Skill(DraftSkillTarget::Authored(
                        preset.skills.members[0].into(),
                    )),
                    parameters: super::super::super::complete(vec![ParameterDraft {
                        slot: parameter.clone().into(),
                        value: ParameterValue::Boolean(true).into(),
                    }]),
                },
                applicability: PresetApplicability::Required,
            };
            preset
                .intent
                .as_mut()
                .unwrap()
                .usage
                .members
                .push(binding.clone());
            case.receipts.usage.push(UsageReceipt {
                source: input.source,
                group: input.group,
                group_contributes: true,
                preset: input.preset,
                identity: input.identity.clone(),
                binding,
                parameter: parameter.clone(),
                fields: [group_field("enabled")].into(),
            });
        }
        test(b, case);
    });
}

fn copy_usage_receipt(receipt: &UsageReceipt) -> UsageReceipt {
    UsageReceipt {
        source: receipt.source,
        group: receipt.group,
        group_contributes: receipt.group_contributes,
        preset: receipt.preset,
        identity: receipt.identity.clone(),
        binding: receipt.binding.clone(),
        parameter: receipt.parameter.clone(),
        fields: receipt.fields.clone(),
    }
}

#[test]
fn unmatched_usage_receipts_pay_only_identity_scans_but_matching_bindings_stay_exact() {
    let mut costs = Vec::new();
    for extra in [0, 64] {
        with_two_usage_presets(|b, mut case| {
            for _ in 0..extra {
                let mut unrelated = copy_usage_receipt(&case.receipts.usage[0]);
                unrelated.preset = case.draft.skill_presets.members[1].id;
                case.receipts.usage.push(unrelated);
            }
            let start = b.work;
            finish(b, &mut case).unwrap();
            costs.push(b.work - start);
            assert!(case.draft.skill_presets.members.iter().all(|preset| {
                matches!(
                    preset.intent.as_ref().unwrap().usage.completion,
                    DraftListCompletion::Complete
                )
            }));
        });
    }
    assert_eq!(costs[1] - costs[0], 2 * 64);

    for mutation in 0..3 {
        with_two_usage_presets(|b, mut case| {
            match mutation {
                0 => {
                    case.receipts.usage[0].binding.selection.target =
                        case.receipts.usage[1].binding.selection.target.clone();
                }
                1 => {
                    case.receipts.usage[0].binding.selection.parameters.members[0].value =
                        ParameterValue::Boolean(false).into();
                }
                _ => {
                    let usage = &mut case.draft.skill_presets.members[0]
                        .intent
                        .as_mut()
                        .unwrap()
                        .usage;
                    usage.members.push(usage.members[0].clone());
                }
            }
            let mut expected = case.draft.clone();
            let mut expected_origins = b.origins.clone();
            let second = &mut expected.skill_presets.members[1]
                .intent
                .as_mut()
                .unwrap()
                .usage;
            let DraftListCompletion::Pending { id, .. } = second.completion else {
                unreachable!()
            };
            second.completion = DraftListCompletion::Complete;
            for origin in &mut expected_origins {
                origin
                    .links
                    .retain(|link| *link != OwnedOriginTarget::Issue(id));
            }
            finish(b, &mut case).unwrap();
            assert_eq!(case.draft, expected, "usage mutation {mutation}");
            assert_eq!(b.origins, expected_origins, "usage mutation {mutation}");
        });
    }
}

#[test]
fn matching_usage_work_has_an_exact_atomic_budget_boundary() {
    let mut cost = 0;
    with_two_usage_presets(|b, mut case| {
        let start = b.work;
        finish(b, &mut case).unwrap();
        cost = b.work - start;
        assert!(case.draft.skill_presets.members.iter().all(|preset| {
            matches!(
                preset.intent.as_ref().unwrap().usage.completion,
                DraftListCompletion::Complete
            )
        }));
    });
    for sufficient in [true, false] {
        with_two_usage_presets(|b, mut case| {
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            b.limits.max_work = b.work + cost - usize::from(!sufficient);
            let result = finish(b, &mut case);
            if sufficient {
                result.unwrap();
                assert!(case.draft.skill_presets.members.iter().all(|preset| {
                    matches!(
                        preset.intent.as_ref().unwrap().usage.completion,
                        DraftListCompletion::Complete
                    )
                }));
            } else {
                assert!(matches!(result, Err(NormalizationError::Limit(_))));
                assert_eq!(case.draft, before);
                assert_eq!(b.origins, origins);
            }
            assert_eq!(b.allocator.state(), allocator);
        });
    }
}

#[test]
fn later_preset_budget_failure_preserves_every_staged_retirement() {
    let mut cost = 0;
    with_two_accountable_presets(|b, mut case| {
        let before = case.draft.clone();
        let mut expected = before.clone();
        let mut origins = b.origins.clone();
        for preset in &mut expected.skill_presets.members {
            let usage = &mut preset.intent.as_mut().unwrap().usage;
            let DraftListCompletion::Pending { id: issue, .. } = usage.completion else {
                unreachable!()
            };
            for origin in &mut origins {
                origin
                    .links
                    .retain(|link| *link != OwnedOriginTarget::Issue(issue));
            }
            usage.completion = DraftListCompletion::Complete;
        }
        let start = b.work;
        finish(b, &mut case).unwrap();
        cost = b.work - start;
        assert_eq!(case.draft, expected);
        assert_eq!(b.origins, origins);
    });
    for exact_budget in [true, false] {
        with_two_accountable_presets(|b, mut case| {
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            b.limits.max_work = b.work + cost - usize::from(!exact_budget);
            let result = finish(b, &mut case);
            if exact_budget {
                result.unwrap();
                assert!(case.draft.skill_presets.members.iter().all(|preset| {
                    matches!(
                        preset.intent.as_ref().unwrap().usage.completion,
                        DraftListCompletion::Complete
                    )
                }));
            } else {
                assert!(matches!(result, Err(NormalizationError::Limit(_))));
                assert_eq!(case.draft, before);
                assert_eq!(b.origins, origins);
            }
            assert_eq!(b.allocator.state(), allocator);
        });
    }
}

#[test]
fn only_literal_primary_true_adds_a_field_fact_and_preserves_the_raw_source() {
    for attribute in [
        "",
        r#"enableGlobal1="true""#,
        r#"enableGlobal1="false""#,
        r#"enableGlobal1="nil""#,
        r#"enableGlobal1="""#,
        r#"enableGlobal1=" true""#,
        r#"enableGlobal1="true ""#,
        r#"enableGlobal1="TRUE""#,
        r#"enableGlobal1="1""#,
        r#"enableGlobal1="tr&#117;e""#,
    ] {
        let source_xml = SOURCE.replace("<Gem/>", &format!("<Gem {attribute}/>"));
        with_case(&source_xml, |b, mut case| {
            require_primary_gate(&mut case);
            let expected = attribute == r#"enableGlobal1="true""#;
            assert_eq!(
                case.receipts.inputs[0]
                    .fields
                    .contains(&occurrence_field("enableGlobal1")),
                expected
            );
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            let source_before = serde_json::to_value(b.evidence.rows()).unwrap();
            finish(b, &mut case).unwrap();
            let mut exact = before.clone();
            if expected {
                exact.skill_presets.members[0]
                    .intent
                    .as_mut()
                    .unwrap()
                    .usage
                    .completion = DraftListCompletion::Complete;
            }
            assert_eq!(case.draft, exact, "{attribute}");
            if !expected {
                assert_eq!(b.origins, origins, "{attribute}");
            }
            assert_eq!(b.allocator.state(), allocator);
            assert_eq!(
                serde_json::to_value(b.evidence.rows()).unwrap(),
                source_before
            );
        });
    }
}

#[test]
fn neutral_primary_gate_does_not_account_other_fields_or_a_second_consumer() {
    for extra in ["enableGlobal2", "count", "mainActiveSkill", "same-field"] {
        let (gem, group) = match extra {
            "mainActiveSkill" => ("", r#" mainActiveSkill="1""#),
            "same-field" => ("", ""),
            _ => (
                if extra == "count" {
                    r#" count="1""#
                } else {
                    // True now has its own neutral-gate proof. False still
                    // needs independent construction/consumer authority.
                    r#" enableGlobal2="false""#
                },
                "",
            ),
        };
        let source_xml = SOURCE
            .replace("<Gem/>", &format!(r#"<Gem enableGlobal1="true"{gem}/>"#))
            .replace(
                r#"<Skill enabled="true">"#,
                &format!(r#"<Skill enabled="true"{group}>"#),
            );
        with_case(&source_xml, |b, mut case| {
            require_primary_gate(&mut case);
            if extra == "same-field" {
                case.rows[0].fields.last_mut().unwrap().destinations.push(
                    UsageInventoryDestination::Deferred {
                        reason: key("other-consumer"),
                    },
                );
            } else {
                case.rows[0].fields.push(UsageInventoryField {
                    field: if extra == "mainActiveSkill" {
                        group_field(extra)
                    } else {
                        occurrence_field(extra)
                    },
                    destinations: vec![UsageInventoryDestination::InputDisposition],
                });
            }
            let before = case.draft.clone();
            let origins = b.origins.clone();
            finish(b, &mut case).unwrap();
            assert_eq!(case.draft, before, "{extra}");
            assert_eq!(b.origins, origins, "{extra}");
        });
    }
}

#[test]
fn intent_retirement_preserves_other_live_obligations_and_all_watermarks() {
    with_case(SOURCE, |b, mut case| {
        let mut expected = case.draft.clone();
        expected.skill_presets.members[0]
            .intent
            .as_mut()
            .unwrap()
            .usage
            .completion = DraftListCompletion::Complete;
        let mut origins = b.origins.clone();
        for row in &mut origins {
            row.links
                .retain(|link| *link != OwnedOriginTarget::Issue(case.issue));
        }
        let watermark = b.allocator.state();
        let issues = b.issues;
        finish(b, &mut case).unwrap();
        assert_eq!(case.draft, expected);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), watermark);
        assert_eq!(b.issues, issues, "spent issue budget is not refunded");
    });
}

#[test]
fn stale_typed_values_detached_origins_and_foreign_issue_links_cannot_complete_intent() {
    let source_xml = SOURCE.replace("<Gem/>", r#"<Gem enableGlobal1="true"/>"#);
    for mode in 0..6 {
        with_case(&source_xml, |b, mut case| {
            require_primary_gate(&mut case);
            match mode {
                0 => case.draft.skills.members[0].enabled = false.into(),
                1 => case.draft.gems.members[0].level = 18.into(),
                2 => b.origins[case.source.ordinal() as usize]
                    .links
                    .retain(|link| !matches!(link, OwnedOriginTarget::Skill(_))),
                3 => b.origins[case.source.ordinal() as usize]
                    .links
                    .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_))),
                4 => {
                    let other = b
                        .evidence
                        .rows()
                        .iter()
                        .rev()
                        .find(|row| row.occurrence().name() == "Gem")
                        .unwrap()
                        .occurrence()
                        .id();
                    b.link(other, OwnedOriginTarget::Issue(case.issue)).unwrap();
                }
                _ => case.draft.skill_presets.members[0].skills.members.clear(),
            }
            let before = case.draft.clone();
            let origins = b.origins.clone();
            finish(b, &mut case).unwrap();
            assert_eq!(case.draft, before, "mode {mode}");
            assert_eq!(b.origins, origins, "mode {mode}");
        });
    }
}

#[test]
fn work_failure_at_the_final_retirement_charge_is_atomic() {
    let mut cost = 0;
    with_case(SOURCE, |b, mut case| {
        let start = b.work;
        finish(b, &mut case).unwrap();
        cost = b.work - start;
        assert!(matches!(
            case.draft.skill_presets.members[0]
                .intent
                .as_ref()
                .unwrap()
                .usage
                .completion,
            DraftListCompletion::Complete
        ));
    });
    with_case(SOURCE, |b, mut case| {
        b.limits.max_work = b.work + cost - 1;
        let before = case.draft.clone();
        let origins = b.origins.clone();
        let watermark = b.allocator.state();
        assert!(matches!(
            finish(b, &mut case),
            Err(NormalizationError::Limit(_))
        ));
        assert_eq!(case.draft, before);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), watermark);
    });
}

#[test]
fn group_fields_have_one_actual_owner_but_every_occurrence_still_needs_coverage() {
    let source_xml = SOURCE.replace(
        "<Gem/></Skill></SkillSet><SkillSet",
        "<Gem/><Gem enabled=\"true\"/></Skill></SkillSet><SkillSet",
    );
    for mode in 0..3 {
        with_case(&source_xml, |b, mut case| {
            let source = b
                .evidence
                .rows()
                .iter()
                .filter(|row| row.occurrence().name() == "Gem")
                .nth(1)
                .unwrap()
                .occurrence()
                .id();
            let mut registry =
                OwnedIdRegistry::empty(b.roles.input().namespace.clone(), Default::default())
                    .unwrap();
            let _ = registry.allocate_definition::<GemDefinition>().unwrap();
            let definition = registry.allocate_definition::<GemDefinition>().unwrap();
            let gem_id: GemInstanceId = b.id().unwrap();
            let skill_id = b.id().unwrap();
            let mut skill = case.draft.skills.members[0].clone();
            skill.id = skill_id;
            skill.source = DraftAuthoredSkillSource::Gem(gem_id.into());
            let gem = GemDraft {
                id: gem_id,
                definition: definition.clone().into(),
                level: 17.into(),
                quality: DraftQuality::Known { value: None },
                parameters: DraftList {
                    completion: DraftListCompletion::Complete,
                    members: vec![],
                },
            };
            let preset = &mut case.draft.skill_presets.members[0];
            preset.skills.members.push(skill_id);
            for link in [
                OwnedOriginTarget::Skill(skill_id),
                OwnedOriginTarget::Gem(gem_id),
                OwnedOriginTarget::SkillPreset(preset.id),
                OwnedOriginTarget::Issue(case.issue),
            ] {
                b.link(source, link).unwrap();
            }
            let reference = skill_input_disposition::CheckedSourceReference::consumer_fixture(
                source,
                b.evidence.rows()[source.ordinal() as usize]
                    .occurrence()
                    .parent(),
                vec![],
            );
            record_input_disposition(
                b,
                source,
                preset,
                &skill,
                Some(&gem),
                AuthoredInputProof::ReferenceDisposition(&reference),
                &mut case.receipts,
            )
            .unwrap();
            case.draft.skills.members.push(skill);
            case.draft.gems.members.push(gem);
            case.rows.push(UsageInventoryRow {
                source: UsageInventorySource::Physical { gem: definition },
                fields: vec![
                    UsageInventoryField {
                        field: occurrence_field("enabled"),
                        destinations: vec![UsageInventoryDestination::InputDisposition],
                    },
                    UsageInventoryField {
                        field: UsageInventoryFieldSelector::ReferenceDescendants,
                        destinations: vec![UsageInventoryDestination::InputDisposition],
                    },
                ],
            });
            match mode {
                0 => {}
                1 => {
                    case.rows[1].fields.remove(0);
                }
                _ => {
                    case.receipts.inputs.pop();
                }
            }
            finish(b, &mut case).unwrap();
            assert_eq!(
                matches!(
                    case.draft.skill_presets.members[0]
                        .intent
                        .as_ref()
                        .unwrap()
                        .usage
                        .completion,
                    DraftListCompletion::Complete
                ),
                mode == 0
            );
        });
    }
}

// These private receipts test the consumer's current-output checks, not source
// proof minting. gem_inventory::tests exercises the actual support producer.
fn with_support_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_case(SOURCE, |b, mut case| {
        let preset = &mut case.draft.skill_presets.members[0];
        let support = SupportDraft {
            id: b.id().unwrap(),
            support: case.draft.gems.members[0].id.into(),
            target: DraftSkillTarget::Authored(case.draft.skills.members[0].id.into()),
            enabled: true.into(),
        };
        b.origins[case.source.ordinal() as usize]
            .links
            .retain(|link| !matches!(link, OwnedOriginTarget::Skill(_)));
        b.link(case.source, OwnedOriginTarget::Support(support.id))
            .unwrap();
        preset.supports.members.push(support.id);
        case.receipts.inputs[0].output = InputOutput::Support {
            support: Box::new(support.clone()),
            gem: Box::new(case.draft.gems.members[0].clone()),
        };
        case.draft.supports.members.push(support);
        test(b, case);
    });
}

#[test]
fn support_receipt_retires_only_usage_with_live_exact_assignment_gem_and_target() {
    with_support_case(|b, mut case| {
        let mut expected = case.draft.clone();
        expected.skill_presets.members[0]
            .intent
            .as_mut()
            .unwrap()
            .usage
            .completion = DraftListCompletion::Complete;
        let mut origins = b.origins.clone();
        for row in &mut origins {
            row.links
                .retain(|link| *link != OwnedOriginTarget::Issue(case.issue));
        }
        let watermark = b.allocator.state();
        let issues = b.issues;
        finish(b, &mut case).unwrap();
        assert_eq!(case.draft, expected);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), watermark);
        assert_eq!(b.issues, issues);
    });
}

#[test]
fn support_receipt_rejects_stale_outputs_detached_targets_membership_and_origins() {
    for mode in 0..15 {
        with_support_case(|b, mut case| {
            let source = case.source;
            let group = case.receipts.inputs[0].group;
            let set = b.evidence.rows()[group.ordinal() as usize]
                .occurrence()
                .parent()
                .unwrap();
            match mode {
                0 => case.draft.gems.members[0].level = 18.into(),
                1 => case.draft.supports.members[0].enabled = false.into(),
                2 => {
                    case.draft.supports.members[0].target =
                        DraftSkillTarget::Authored(b.id::<SkillUseId>().unwrap().into());
                }
                3 => case.draft.skill_presets.members[0].supports.members.clear(),
                4 => b.origins[source.ordinal() as usize]
                    .links
                    .retain(|link| !matches!(link, OwnedOriginTarget::Support(_))),
                5 => b.origins[source.ordinal() as usize]
                    .links
                    .retain(|link| !matches!(link, OwnedOriginTarget::Gem(_))),
                6 | 7 => {
                    let foreign = b.id().unwrap();
                    b.link(
                        if mode == 6 { source } else { group },
                        OwnedOriginTarget::SkillPreset(foreign),
                    )
                    .unwrap();
                }
                8 => b.origins[set.ordinal() as usize]
                    .links
                    .retain(|link| !matches!(link, OwnedOriginTarget::SkillPreset(_))),
                9 => case.draft.skills.members.clear(),
                10 => case.draft.skill_presets.members[0].skills.members.clear(),
                11 => {
                    let foreign: GemInstanceId = b.id().unwrap();
                    case.draft.supports.members[0].support = foreign.into();
                }
                12 => {
                    let foreign = b.id().unwrap();
                    b.link(source, OwnedOriginTarget::Support(foreign)).unwrap();
                }
                13 => {
                    let target = case.draft.skills.members[0].id;
                    case.draft.skill_presets.members[0]
                        .skills
                        .members
                        .push(target);
                }
                _ => case.receipts.inputs[0].preset = b.id().unwrap(),
            }
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let watermark = b.allocator.state();
            finish(b, &mut case).unwrap();
            assert_eq!(case.draft, before, "mode {mode}");
            assert_eq!(b.origins, origins, "mode {mode}");
            assert_eq!(b.allocator.state(), watermark, "mode {mode}");
        });
    }
}

#[test]
fn support_receipt_late_work_failure_preserves_whole_pending_inventory() {
    let mut cost = 0;
    with_support_case(|b, mut case| {
        let start = b.work;
        finish(b, &mut case).unwrap();
        cost = b.work - start;
        assert!(matches!(
            completion(&case.draft.skill_presets.members[0]),
            Some(DraftListCompletion::Complete)
        ));
    });
    with_support_case(|b, mut case| {
        b.limits.max_work = b.work + cost - 1;
        let before = case.draft.clone();
        let origins = b.origins.clone();
        let watermark = b.allocator.state();
        assert!(matches!(
            finish(b, &mut case),
            Err(NormalizationError::Limit(_))
        ));
        assert_eq!(case.draft, before);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), watermark);
    });
}
