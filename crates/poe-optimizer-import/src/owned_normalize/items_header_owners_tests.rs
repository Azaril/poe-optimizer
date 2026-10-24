//! Private owner correspondence, independent of equipment or saved-selection completion.
use super::*;

// The selected key is deliberately neither a position nor the first saved set.
const SOURCE: &str = r#"<PathOfBuilding2><Items activeItemSet="20" showStatDifferences="true" useSecondWeaponSet="nil"><Item id="1">retained item</Item><ItemSet id="7"><Slot name="Weapon 1" itemId="1"/></ItemSet><ItemSet id="20"><Slot name="Helmet" itemId="1"/></ItemSet></Items><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#;

struct Case {
    draft: DraftSessionInput,
    sets: BTreeMap<SourceOccurrenceId, usize>,
    items: SourceOccurrenceId,
    sources: Vec<SourceOccurrenceId>,
    selection: DraftIssueId,
}

fn foreign_snapshot(source: SourceOccurrenceId) -> SourceOccurrenceId {
    let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
        crate::decode_build(
            SOURCE
                .replace("activeItemSet=\"20\"", "activeItemSet=\"7\"")
                .as_bytes(),
        )
        .unwrap(),
        BuildLineage::from_bytes([91; 16]),
        Default::default(),
    )
    .unwrap();
    let foreign = imported.occurrences()[source.ordinal() as usize].id();
    assert_eq!(foreign.ordinal(), source.ordinal());
    assert_ne!(foreign, source);
    foreign
}

fn with_source(source_xml: &str, test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    gem_inventory::with_usage_inventory_destination(source_xml, |b, skill, preset, _, _| {
        let root = b.evidence.rows()[0].occurrence().id();
        let items = b
            .evidence
            .rows()
            .iter()
            .find(|row| row.occurrence().name() == "Items")
            .unwrap()
            .occurrence()
            .id();
        let sources: Vec<_> = b.evidence.rows()[items.ordinal() as usize]
            .children()
            .iter()
            .copied()
            .filter(|source| {
                b.evidence.rows()[source.ordinal() as usize]
                    .occurrence()
                    .name()
                    == "ItemSet"
            })
            .collect();
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
            skills: complete(vec![skill]),
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
        let DraftListCompletion::Pending { id: selection, .. } = draft.saved_variants.completion
        else {
            unreachable!()
        };
        let mut sets = BTreeMap::new();
        for (index, source) in sources.iter().copied().enumerate() {
            let mut owner = EquipmentPresetDraft {
                id: b.id().unwrap(),
                equipment: b
                    .closure(source, "equipment-membership-not-converted", vec![])
                    .unwrap(),
            };
            b.link(source, OwnedOriginTarget::EquipmentPreset(owner.id))
                .unwrap();
            // Real Slot projections belong to their set but own independent
            // equipment records. Header attribution must preserve them exactly.
            let children = b.evidence.rows()[source.ordinal() as usize]
                .children()
                .to_vec();
            for child in children {
                let id = b.id().unwrap();
                b.link(child, OwnedOriginTarget::Equipment(id)).unwrap();
                draft.equipment.members.push(EquipmentDraft {
                    id,
                    item: b.pending(child, "item-reference-unresolved").unwrap(),
                    destination: DraftEquipmentDestination::CharacterSlot(
                        b.pending(child, "equipment-slot-unresolved").unwrap(),
                    ),
                    scope: b.pending(child, "equipment-scope-not-converted").unwrap(),
                });
                owner.equipment.members.push(id);
            }
            sets.insert(source, index);
            draft.equipment_presets.members.push(owner);
        }
        draft.allocator = b.allocator.state();
        test(
            b,
            Case {
                draft,
                sets,
                items,
                sources,
                selection,
            },
        );
    });
}

fn with_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_source(SOURCE, |b, case| {
        assert_eq!(case.sources.len(), 2);
        for source in &case.sources {
            assert!(matches!(
                b.evidence.rows()[source.ordinal() as usize].authored_instance(),
                Some(AuthoredInstanceId::ItemSet(_))
            ));
        }
        test(b, case);
    });
}

fn run(b: &mut Builder<'_, '_>, case: &Case) -> Result<()> {
    account_items(b, &case.draft, &case.sets)
}

fn assert_baseline(b: &mut Builder<'_, '_>, case: &Case) {
    let before = b.origins.clone();
    let (links, work, cached) = (b.links, b.work, b.ordinary_items_source);
    let allocator = b.allocator.state();
    let issues = b.issues;
    let draft = case.draft.clone();
    run(b, case).unwrap();
    let mut expected = before.clone();
    expected[case.items.ordinal() as usize].links = vec![
        OwnedOriginTarget::EquipmentPreset(case.draft.equipment_presets.members[1].id),
        OwnedOriginTarget::Issue(case.selection),
    ];
    assert_eq!(b.origins, expected);
    assert_eq!(b.links, links + 2);
    assert_eq!(b.allocator.state(), allocator);
    assert_eq!(b.issues, issues);
    assert_eq!(case.draft, draft);
    b.origins = before;
    b.links = links;
    b.work = work;
    b.ordinary_items_source = cached;
}

fn assert_no_change(b: &mut Builder<'_, '_>, case: &Case) {
    let origins = b.origins.clone();
    let links = b.links;
    let allocator = b.allocator.state();
    let issues = b.issues;
    let draft = case.draft.clone();
    run(b, case).unwrap();
    assert_eq!(b.origins, origins);
    assert_eq!(b.links, links);
    assert_eq!(b.allocator.state(), allocator);
    assert_eq!(b.issues, issues);
    assert_eq!(case.draft, draft);
}

#[test]
fn items_header_uses_exact_saved_key_and_preserves_descendants_and_pending_membership() {
    with_case(|b, case| {
        assert_baseline(b, &case);
        // Map indices are correspondence, not source ordinal or key order.
        let mut reordered = case;
        reordered.draft.equipment_presets.members.swap(0, 1);
        reordered.sets.insert(reordered.sources[0], 1);
        reordered.sets.insert(reordered.sources[1], 0);
        let before = b.origins.clone();
        run(b, &reordered).unwrap();
        let mut expected = before;
        expected[reordered.items.ordinal() as usize].links = vec![
            OwnedOriginTarget::EquipmentPreset(reordered.draft.equipment_presets.members[0].id),
            OwnedOriginTarget::Issue(reordered.selection),
        ];
        assert_eq!(b.origins, expected);
        assert_no_change(b, &reordered);
        assert!(matches!(
            reordered.draft.saved_variants.completion,
            DraftListCompletion::Pending { .. }
        ));
        for owner in &reordered.draft.equipment_presets.members {
            assert!(matches!(
                owner.equipment.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
    });
}

#[test]
fn items_header_refuses_incomplete_foreign_stale_and_aliased_set_maps() {
    for mutation in 0..12 {
        with_case(|b, mut case| {
            assert_baseline(b, &case);
            let first = case.sources[0];
            let selected = case.sources[1];
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
                    case.sets.insert(case.items, 0);
                }
                4 => {
                    case.sets.insert(first, 1);
                    case.sets.insert(selected, 0);
                }
                5 => {
                    case.draft.equipment_presets.members[1].id =
                        case.draft.equipment_presets.members[0].id
                }
                6 => {
                    case.draft.equipment_presets.members.pop();
                }
                7 => {
                    case.draft
                        .equipment_presets
                        .members
                        .push(case.draft.equipment_presets.members[0].clone());
                }
                8 => {
                    case.sets.remove(&selected);
                    case.sets.insert(foreign_snapshot(selected), 1);
                }
                9 => case.draft.equipment_presets.members[1].id = b.id().unwrap(),
                10 => {
                    case.draft.equipment_presets.members[1].id =
                        EquipmentPresetId::from_instance_id(
                            InstanceId::from_parts(BuildLineage::from_bytes([92; 16]), 1).unwrap(),
                        );
                }
                11 => {
                    case.sets.insert(case.items, 0);
                }
                _ => unreachable!(),
            }
            assert_no_change(b, &case);
        });
    }
}

#[test]
fn items_header_refuses_crossed_duplicate_and_foreign_owner_provenance() {
    for mutation in 0..13 {
        with_case(|b, case| {
            assert_baseline(b, &case);
            let selected = case.sources[1].ordinal() as usize;
            let first = case.sources[0].ordinal() as usize;
            let owner =
                OwnedOriginTarget::EquipmentPreset(case.draft.equipment_presets.members[1].id);
            match mutation {
                0 => b.origins[selected].links.retain(|v| v != &owner),
                1 => b.origins[selected].links.push(owner),
                2 => b.origins[first].links.push(owner),
                3 => {
                    b.origins[selected].links.retain(|v| v != &owner);
                    b.origins[selected]
                        .links
                        .push(OwnedOriginTarget::EquipmentPreset(
                            case.draft.equipment_presets.members[0].id,
                        ));
                }
                4 => b.origins[selected].source = case.sources[0],
                5 => b.origins[case.items.ordinal() as usize].source = case.sources[0],
                6 => b.origins[0].source = case.items,
                7 => {
                    b.origins[selected].disposition =
                        SourceDisposition::SourceOnly(key("foreign-set"))
                }
                8 => {
                    b.origins[case.items.ordinal() as usize].disposition =
                        SourceDisposition::SourceOnly(key("foreign-header"))
                }
                9 => {
                    b.origins.pop();
                }
                10 => b.origins[selected].source = foreign_snapshot(case.sources[1]),
                11 => {
                    b.origins[case.items.ordinal() as usize].source = foreign_snapshot(case.items)
                }
                12 => {
                    b.origins[0].source = foreign_snapshot(b.evidence.rows()[0].occurrence().id())
                }
                _ => unreachable!(),
            }
            assert_no_change(b, &case);
        });
    }
}

#[test]
fn items_header_requires_actual_pending_selection_and_unattributed_header() {
    for mutation in 0..9 {
        with_case(|b, mut case| {
            assert_baseline(b, &case);
            match mutation {
                0 => case.draft.saved_variants.completion = DraftListCompletion::Complete,
                1 => {
                    let DraftListCompletion::Pending { code, .. } =
                        &mut case.draft.saved_variants.completion
                    else {
                        unreachable!()
                    };
                    *code = key("other-selection");
                }
                2 => b.origins[0].links.clear(),
                3 => b.origins[0]
                    .links
                    .push(OwnedOriginTarget::Issue(case.selection)),
                4 => b.origins[0].disposition = SourceDisposition::SourceOnly(key("foreign-root")),
                5 => {
                    let DraftListCompletion::Pending { id, .. } =
                        &mut case.draft.saved_variants.completion
                    else {
                        unreachable!()
                    };
                    *id = b.id().unwrap();
                }
                6 => {
                    b.origins[0].links.clear();
                    b.origins[case.sources[1].ordinal() as usize]
                        .links
                        .push(OwnedOriginTarget::Issue(case.selection));
                }
                7 => b.origins[case.items.ordinal() as usize]
                    .links
                    .push(OwnedOriginTarget::Issue(case.selection)),
                8 => {
                    let source = case.sources[1];
                    case.draft.saved_variants.members.push(SavedVariantDraft {
                        id: b.id().unwrap(),
                        selection: SelectionDraft {
                            character: b.pending(source, "selection-character").unwrap(),
                            equipment: case.draft.equipment_presets.members[1].id.into(),
                            allocations: b.pending(source, "selection-allocations").unwrap(),
                            skills: case.draft.skill_presets.members[0].id.into(),
                            choices: b.pending(source, "selection-choices").unwrap(),
                            active_weapon_loadout: b.pending(source, "selection-loadout").unwrap(),
                            scenario: b.pending(source, "selection-scenario").unwrap(),
                            queries: b.pending(source, "selection-queries").unwrap(),
                        },
                    });
                }
                _ => unreachable!(),
            }
            assert_no_change(b, &case);
        });
    }
}

#[test]
fn items_header_refuses_ambiguous_frames_and_missing_or_noncanonical_selected_keys() {
    with_case(|b, case| assert_baseline(b, &case));
    let changes = [
        (" activeItemSet=\"20\"", ""),
        ("activeItemSet=\"20\"", "activeItemSet=\"2\""),
        ("activeItemSet=\"20\"", "activeItemSet=\"020\""),
        ("activeItemSet=\"20\"", "activeItemSet=\"0\""),
        ("<PathOfBuilding2>", "<PathOfBuilding2 unknown=\"true\">"),
        ("<PathOfBuilding2>", "<PathOfBuilding2>unreviewed"),
        ("<Items ", "<Items unknown=\"true\" "),
        ("</Items>", "</Items><Items/>"),
        ("<ItemSet id=\"20\">", "<ItemSet id=\"7\">"),
        ("<ItemSet id=\"20\">", "<ItemSet id=\"020\">"),
        (
            "<ItemSet id=\"20\">",
            "<ItemSet id=\"20\" xmlns=\"urn:foreign\">",
        ),
    ];
    for (from, to) in changes {
        let changed = SOURCE.replace(from, to);
        assert_ne!(changed, SOURCE);
        with_source(&changed, |b, case| assert_no_change(b, &case));
    }
}

#[test]
fn items_header_exact_work_and_link_bounds_are_atomic_for_cold_and_cached_frames() {
    for cached in [false, true] {
        let mut exact_work = 0;
        let mut exact_links = 0;
        with_case(|b, case| {
            if cached {
                assert_eq!(
                    equipment_membership::ordinary_items(b).unwrap(),
                    Some(case.items)
                );
            }
            let prior_links = b.links;
            run(b, &case).unwrap();
            exact_work = b.work;
            exact_links = b.links;
            assert_eq!(exact_links, prior_links + 2);
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
                        equipment_membership::ordinary_items(b).unwrap(),
                        Some(case.items)
                    );
                }
                let origins = b.origins.clone();
                let prior_links = b.links;
                let allocator = b.allocator.state();
                let issues = b.issues;
                let draft = case.draft.clone();
                b.limits.max_work = work;
                b.limits.max_origin_links = links;
                match (run(b, &case), failure) {
                    (Ok(()), None) => {
                        assert_eq!(b.work, exact_work);
                        assert_eq!(b.links, exact_links);
                    }
                    (Err(NormalizationError::Limit(actual)), Some(expected)) => {
                        assert_eq!(actual, expected);
                        assert_eq!(b.origins, origins);
                        assert_eq!(b.links, prior_links);
                        b.limits = NormalizationLimits::default();
                        run(b, &case).unwrap();
                        assert_eq!(b.links, exact_links);
                    }
                    (result, expected) => {
                        panic!("unexpected boundary result: {result:?}, {expected:?}")
                    }
                }
                let mut expected = origins;
                expected[case.items.ordinal() as usize].links = vec![
                    OwnedOriginTarget::EquipmentPreset(case.draft.equipment_presets.members[1].id),
                    OwnedOriginTarget::Issue(case.selection),
                ];
                assert_eq!(b.origins, expected);
                assert_eq!(b.allocator.state(), allocator);
                assert_eq!(b.issues, issues);
                assert_eq!(case.draft, draft);
            });
        }
    }
}
