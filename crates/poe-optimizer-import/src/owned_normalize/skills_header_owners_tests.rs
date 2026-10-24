//! Saved source ownership without Gem-input, usage or final-selection authority.
use super::*;

const PREFERENCES: &str = r#" defaultGemLevel="normalMaximum" defaultGemQuality="0" sortGemsByDPS="true" showLegacyGems="false" showSupportGemTypes="ALL" sortGemsByDPSField="CombinedDPS""#;

fn document(preferences: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="20"{preferences}><SkillSet id="7" title="same"><Skill><Gem level="8" quality="3"/></Skill></SkillSet><SkillSet id="20" title="same"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#
    )
}

struct Case {
    draft: DraftSessionInput,
    sets: BTreeMap<SourceOccurrenceId, usize>,
    header: SourceOccurrenceId,
    sources: Vec<SourceOccurrenceId>,
    selection: DraftIssueId,
}

fn foreign(source: SourceOccurrenceId) -> SourceOccurrenceId {
    let xml = document(PREFERENCES).replace("activeSkillSet=\"20\"", "activeSkillSet=\"7\"");
    let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
        crate::decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([91; 16]),
        Default::default(),
    )
    .unwrap();
    let other = imported.occurrences()[source.ordinal() as usize].id();
    assert_eq!(other.ordinal(), source.ordinal());
    assert_ne!(other, source);
    other
}

fn with_source(xml: &str, test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    gem_inventory::with_usage_inventory_destination(xml, |b, mut skill, mut preset, _, gem| {
        let root = b.evidence.rows()[0].occurrence().id();
        let header = b.evidence.rows()[1].occurrence().id();
        assert_eq!(b.evidence.rows()[1].occurrence().name(), "Skills");
        let sources = b.evidence.rows()[1].children().to_vec();
        assert_eq!(sources.len(), 2);
        skill.parameters = Some(
            b.closure(gem, "skill-inputs-not-converted", vec![])
                .unwrap(),
        );
        preset.skills = b
            .closure(sources[0], "skill-membership-not-converted", vec![skill.id])
            .unwrap();
        preset.supports = b
            .closure(sources[0], "support-membership-not-converted", vec![])
            .unwrap();
        preset.payload_links = b
            .closure(sources[0], "payload-membership-not-converted", vec![])
            .unwrap();
        let mut draft = DraftSessionInput {
            allocator: b.allocator.state(),
            revision: b.evidence.identity().revision,
            game_version: b.roles.input().namespace.clone(),
            weapon_loadouts: complete(vec![]),
            items: complete(vec![]),
            gems: complete(vec![]),
            rewards: complete(vec![]),
            equipment: complete(vec![]),
            allocations: complete(vec![]),
            skills: complete(vec![skill.clone()]),
            supports: complete(vec![]),
            payload_links: complete(vec![]),
            character_presets: complete(vec![]),
            equipment_presets: complete(vec![]),
            allocation_presets: complete(vec![]),
            skill_presets: complete(vec![preset]),
            choice_presets: complete(vec![]),
            scenario_presets: complete(vec![]),
            query_presets: complete(vec![]),
            saved_variants: b
                .closure(root, "saved-selection-not-converted", vec![])
                .unwrap(),
        };
        let second_group = b.evidence.rows()[sources[1].ordinal() as usize].children()[0];
        let second_gem = b.evidence.rows()[second_group.ordinal() as usize].children()[0];
        let physical: GemInstanceId = b.id().unwrap();
        skill.id = b.id().unwrap();
        skill.source = DraftAuthoredSkillSource::Gem(physical.into());
        skill.parameters = Some(
            b.closure(second_gem, "skill-inputs-not-converted", vec![])
                .unwrap(),
        );
        b.link(second_gem, OwnedOriginTarget::Gem(physical))
            .unwrap();
        b.link(second_gem, OwnedOriginTarget::Skill(skill.id))
            .unwrap();
        let second = SkillPresetDraft {
            id: b.id().unwrap(),
            intent: None,
            authored_support_order: None,
            skills: b
                .closure(sources[1], "skill-membership-not-converted", vec![skill.id])
                .unwrap(),
            supports: b
                .closure(sources[1], "support-membership-not-converted", vec![])
                .unwrap(),
            payload_links: b
                .closure(sources[1], "payload-membership-not-converted", vec![])
                .unwrap(),
            usage_preferences: Some(
                b.closure(second_gem, "usage-preferences-not-converted", vec![])
                    .unwrap(),
            ),
        };
        b.link(sources[1], OwnedOriginTarget::SkillPreset(second.id))
            .unwrap();
        draft.skills.members.push(skill);
        draft.skill_presets.members.push(second);
        draft.allocator = b.allocator.state();
        let DraftListCompletion::Pending { id: selection, .. } = draft.saved_variants.completion
        else {
            unreachable!()
        };
        let sets = BTreeMap::from([(sources[0], 0), (sources[1], 1)]);
        test(
            b,
            Case {
                draft,
                sets,
                header,
                sources,
                selection,
            },
        );
    });
}

fn with_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_source(&document(PREFERENCES), test);
}

fn run(b: &mut Builder<'_, '_>, case: &Case) -> Result<()> {
    account_skills(b, &case.draft, &case.sets)
}

fn expected(case: &Case) -> [OwnedOriginTarget; 2] {
    [
        OwnedOriginTarget::SkillPreset(
            case.draft.skill_presets.members[case.sets[&case.sources[1]]].id,
        ),
        OwnedOriginTarget::Issue(case.selection),
    ]
}

fn assert_baseline(b: &mut Builder<'_, '_>, case: &Case) {
    let before = b.origins.clone();
    let (links, work, cached) = (b.links, b.work, b.fresh_skill_containers.clone());
    let (allocator, issues, draft) = (b.allocator.state(), b.issues, case.draft.clone());
    run(b, case).unwrap();
    let mut after = before.clone();
    after[case.header.ordinal() as usize].links = expected(case).to_vec();
    assert_eq!(b.origins, after);
    assert_eq!(b.links, links + 2);
    assert_eq!(b.allocator.state(), allocator);
    assert_eq!(b.issues, issues);
    assert_eq!(case.draft, draft);
    b.origins = before;
    b.links = links;
    b.work = work;
    b.fresh_skill_containers = cached;
}

fn assert_refused(b: &mut Builder<'_, '_>, case: &Case) {
    let before = b.origins.clone();
    let (links, allocator, issues, draft) =
        (b.links, b.allocator.state(), b.issues, case.draft.clone());
    run(b, case).unwrap();
    assert_eq!(b.origins, before);
    assert_eq!(b.links, links);
    assert_eq!(b.allocator.state(), allocator);
    assert_eq!(b.issues, issues);
    assert_eq!(case.draft, draft);
}

#[test]
fn skills_header_follows_saved_key_and_keeps_actual_child_obligations() {
    with_case(|b, mut case| {
        assert_baseline(b, &case);
        case.draft.skill_presets.members.swap(0, 1);
        case.sets.insert(case.sources[0], 1);
        case.sets.insert(case.sources[1], 0);
        assert_baseline(b, &case);
        run(b, &case).unwrap();
        assert_refused(b, &case); // Repetition never duplicates attribution.
        for preset in &case.draft.skill_presets.members {
            for state in [
                &preset.skills.completion,
                &preset.supports.completion,
                &preset.payload_links.completion,
                &preset.usage_preferences.as_ref().unwrap().completion,
            ] {
                assert!(matches!(state, DraftListCompletion::Pending { .. }));
            }
        }
        for skill in &case.draft.skills.members {
            assert!(matches!(
                skill.parameters.as_ref().unwrap().completion,
                DraftListCompletion::Pending { .. }
            ));
        }
        assert!(matches!(
            case.draft.saved_variants.completion,
            DraftListCompletion::Pending { .. }
        ));
    });
}

#[test]
fn skills_header_admits_missing_preferences_and_each_reviewed_finite_alternative() {
    with_source(&document(""), |b, case| assert_baseline(b, &case));
    for (name, values) in [
        (
            "defaultGemLevel",
            &["normalMaximum", "corruptedMaximum", "characterLevel"][..],
        ),
        ("sortGemsByDPS", &["true", "false"][..]),
        ("showLegacyGems", &["true", "false"][..]),
        ("showSupportGemTypes", &["ALL", "LINEAGE", "NORMAL"][..]),
        (
            "sortGemsByDPSField",
            &[
                "FullDPS",
                "CombinedDPS",
                "TotalDPS",
                "AverageDamage",
                "TotalDot",
                "BleedDPS",
                "IgniteDPS",
                "TotalPoisonDPS",
                "TotalEHP",
            ][..],
        ),
    ] {
        for value in values {
            with_source(&document(&format!(r#" {name}="{value}""#)), |b, case| {
                assert_baseline(b, &case)
            });
        }
    }
    for quality in 0..=23 {
        with_source(
            &document(&format!(r#" defaultGemQuality="{quality}""#)),
            |b, case| assert_baseline(b, &case),
        );
    }
}

#[test]
fn skills_header_refuses_unknown_legacy_clamped_and_malformed_preferences() {
    with_case(|b, case| assert_baseline(b, &case));
    for (name, values) in [
        (
            "defaultGemLevel",
            &["awakenedMaximum", "20", "unknown", "", "normalMaximu&#109;"][..],
        ),
        (
            "defaultGemQuality",
            &[
                "24", "-1", "00", "01", "1.0", " 1", "1 ", "1e1", "+1", "nil", "", "&#49;",
            ][..],
        ),
        ("sortGemsByDPS", &["nil", "TRUE", "0", "", "&#116;rue"][..]),
        ("showLegacyGems", &["nil", "FALSE", "1", "false "][..]),
        ("showSupportGemTypes", &["all", "UNKNOWN", ""][..]),
        (
            "sortGemsByDPSField",
            &["TotalDamage", "DPS", "CombinedDPS ", ""][..],
        ),
        (
            "matchGemLevelToCharacterLevel",
            &["true", "false", "nil", ""][..],
        ),
        ("unreviewed", &["true"][..]),
    ] {
        for value in values {
            with_source(&document(&format!(r#" {name}="{value}""#)), |b, case| {
                assert_refused(b, &case)
            });
        }
    }
}

#[test]
fn skills_header_refuses_missing_foreign_aliased_or_crossed_preset_authority() {
    for mutation in 0..26 {
        with_case(|b, mut case| {
            assert_baseline(b, &case);
            let first = case.sources[0];
            let selected = case.sources[1];
            let owner = OwnedOriginTarget::SkillPreset(case.draft.skill_presets.members[1].id);
            let origin = selected.ordinal() as usize;
            match mutation {
                0 => {
                    case.sets.remove(&selected);
                }
                1 => {
                    case.sets.insert(selected, usize::MAX);
                }
                2 => {
                    case.sets.insert(selected, 0);
                }
                3 => {
                    case.sets.remove(&first);
                    case.sets.insert(case.header, 0);
                }
                4 => {
                    case.sets.insert(first, 1);
                    case.sets.insert(selected, 0);
                }
                5 => {
                    case.draft.skill_presets.members[1].id = case.draft.skill_presets.members[0].id
                }
                6 => {
                    case.draft.skill_presets.members.pop();
                }
                7 => {
                    case.draft
                        .skill_presets
                        .members
                        .push(case.draft.skill_presets.members[0].clone());
                }
                8 => {
                    case.sets.remove(&selected);
                    case.sets.insert(foreign(selected), 1);
                }
                9 => case.draft.skill_presets.members[1].id = b.id().unwrap(),
                10 => {
                    case.sets.insert(case.header, 0);
                }
                11 => b.origins[origin].links.retain(|link| link != &owner),
                12 => b.origins[origin].links.push(owner),
                13 => b.origins[first.ordinal() as usize].links.push(owner),
                14 => {
                    b.origins[origin].links.retain(|link| link != &owner);
                    b.origins[origin].links.push(OwnedOriginTarget::SkillPreset(
                        case.draft.skill_presets.members[0].id,
                    ));
                }
                15 => b.origins[origin].source = foreign(selected),
                16 => b.origins[case.header.ordinal() as usize].source = foreign(case.header),
                17 => b.origins[0].source = foreign(b.evidence.rows()[0].occurrence().id()),
                18 => {
                    b.origins[origin].disposition =
                        SourceDisposition::SourceOnly(key("foreign-set"))
                }
                19 => {
                    b.origins[case.header.ordinal() as usize].disposition =
                        SourceDisposition::SourceOnly(key("foreign-header"))
                }
                20 => {
                    b.origins.pop();
                }
                21 => case.draft.saved_variants.completion = DraftListCompletion::Complete,
                22 => b.origins[0].links.clear(),
                23 => b.origins[0]
                    .links
                    .push(OwnedOriginTarget::Issue(case.selection)),
                24 => b.origins[case.header.ordinal() as usize]
                    .links
                    .push(OwnedOriginTarget::Issue(case.selection)),
                25 => {
                    let DraftListCompletion::Pending { code, .. } =
                        &mut case.draft.saved_variants.completion
                    else {
                        unreachable!()
                    };
                    *code = key("other-selection");
                }
                _ => unreachable!(),
            }
            assert_refused(b, &case);
        });
    }
}

#[test]
fn skills_header_exact_work_and_link_boundaries_are_atomic_with_cold_and_cached_census() {
    for cached in [false, true] {
        let mut exact_work = 0;
        let mut exact_links = 0;
        with_case(|b, case| {
            if cached {
                assert_eq!(
                    skill_source_census::container_sets(b).unwrap(),
                    Some(case.sources.clone())
                );
            }
            run(b, &case).unwrap();
            exact_work = b.work;
            exact_links = b.links;
        });
        for (work, links, failure) in [
            (exact_work, exact_links, None),
            (exact_work - 1, exact_links, Some("work")),
            (exact_work, exact_links - 1, Some("origin links")),
        ] {
            with_case(|b, case| {
                assert_baseline(b, &case);
                if cached {
                    assert_eq!(
                        skill_source_census::container_sets(b).unwrap(),
                        Some(case.sources.clone())
                    );
                }
                let before = b.origins.clone();
                let (prior_links, allocator, issues, draft) =
                    (b.links, b.allocator.state(), b.issues, case.draft.clone());
                b.limits.max_work = work;
                b.limits.max_origin_links = links;
                match (run(b, &case), failure) {
                    (Ok(()), None) => {
                        assert_eq!(b.work, exact_work);
                        assert_eq!(b.links, exact_links);
                    }
                    (Err(NormalizationError::Limit(actual)), Some(expected)) => {
                        assert_eq!(actual, expected);
                        assert_eq!(b.origins, before);
                        assert_eq!(b.links, prior_links);
                        b.limits = NormalizationLimits::default();
                        run(b, &case).unwrap();
                    }
                    (result, expected) => {
                        panic!("unexpected boundary result: {result:?}, {expected:?}")
                    }
                }
                let mut after = before;
                after[case.header.ordinal() as usize].links = expected(&case).to_vec();
                assert_eq!(b.origins, after);
                assert_eq!(b.links, exact_links);
                assert_eq!(b.allocator.state(), allocator);
                assert_eq!(b.issues, issues);
                assert_eq!(case.draft, draft);
            });
        }
    }
}
