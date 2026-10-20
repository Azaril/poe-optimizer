use super::*;
mod group_selectors;

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
            record_input_disposition(b, source, &preset, &skill, Some(&gem_record), &mut receipts)
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
    let source_xml = SOURCE.replace("<Skill><Gem/>", r#"<Skill enabled="true"><Gem/>"#);
    with_case(&source_xml, |b, mut case| {
        let source = b
            .evidence
            .rows()
            .iter()
            .rev()
            .find(|row| row.occurrence().name() == "Gem")
            .unwrap()
            .occurrence()
            .id();
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
        record_input_disposition(b, source, &preset, &skill, Some(&gem), &mut case.receipts)
            .unwrap();
        assert_eq!(case.receipts.inputs.len(), 2);
        case.draft.gems.members.push(gem);
        case.draft.skills.members.push(skill);
        case.presets
            .insert(set, case.draft.skill_presets.members.len());
        case.draft.skill_presets.members.push(preset);
        case.draft.allocator = b.allocator.state();
        test(b, case);
    });
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
                    r#" enableGlobal2="true""#
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
            record_input_disposition(b, source, preset, &skill, Some(&gem), &mut case.receipts)
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
