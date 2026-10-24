//! Actual private Builder relationships; no source or completion authority is minted.
use super::*;

const SOURCE: &str = r#"<PathOfBuilding2><Tree activeSpec="2"><Spec><URL>first</URL></Spec><Spec><Overrides><AttributeOverride/></Overrides><Sockets/></Spec></Tree><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#;

struct Case {
    draft: DraftSessionInput,
    specs: BTreeMap<SourceOccurrenceId, usize>,
    characters: BTreeMap<SourceOccurrenceId, usize>,
    tree: SourceOccurrenceId,
    sources: Vec<SourceOccurrenceId>,
    selection: DraftIssueId,
}

fn foreign_snapshot(source: SourceOccurrenceId) -> SourceOccurrenceId {
    let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
        crate::decode_build(
            SOURCE
                .replace("activeSpec=\"2\"", "activeSpec=\"1\"")
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

fn with_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    gem_inventory::with_usage_inventory_destination(SOURCE, |b, skill, preset, _, _| {
        let root = b.evidence.rows()[0].occurrence().id();
        let tree = b
            .evidence
            .rows()
            .iter()
            .find(|row| row.occurrence().name() == "Tree")
            .unwrap()
            .occurrence()
            .id();
        let sources = b.evidence.rows()[tree.ordinal() as usize]
            .children()
            .to_vec();
        assert_eq!(sources.len(), 2);
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
        let mut specs = BTreeMap::new();
        let mut characters = BTreeMap::new();
        for (index, source) in sources.iter().copied().enumerate() {
            let allocation = AllocationPresetDraft {
                id: b.id().unwrap(),
                allocations: b
                    .closure(source, "allocation-membership-not-converted", vec![])
                    .unwrap(),
                equipment: b
                    .closure(
                        source,
                        "allocation-equipment-membership-not-converted",
                        vec![],
                    )
                    .unwrap(),
            };
            let character = CharacterPresetDraft {
                id: b.id().unwrap(),
                class: b.pending(source, "tree-class-unresolved").unwrap(),
                ascendancy: b.pending(source, "tree-ascendancy-unresolved").unwrap(),
                level: 70.into(),
                rewards: b
                    .closure(source, "character-rewards-not-converted", vec![])
                    .unwrap(),
            };
            b.link(source, OwnedOriginTarget::AllocationPreset(allocation.id))
                .unwrap();
            b.link(source, OwnedOriginTarget::CharacterPreset(character.id))
                .unwrap();
            // Real tree materialization also attaches owner projections to
            // descendants. They must not be mistaken for other Spec owners.
            let descendants: Vec<_> = b
                .evidence
                .rows()
                .iter()
                .filter(|row| {
                    row.occurrence().parent() == Some(source)
                        || row.occurrence().parent().is_some_and(|parent| {
                            b.evidence.rows()[parent.ordinal() as usize]
                                .occurrence()
                                .parent()
                                == Some(source)
                        })
                })
                .map(|row| row.occurrence().id())
                .collect();
            for child in descendants {
                b.link(child, OwnedOriginTarget::AllocationPreset(allocation.id))
                    .unwrap();
            }
            specs.insert(source, index);
            characters.insert(source, index);
            draft.allocation_presets.members.push(allocation);
            draft.character_presets.members.push(character);
        }
        draft.allocator = b.allocator.state();
        test(
            b,
            Case {
                draft,
                specs,
                characters,
                tree,
                sources,
                selection,
            },
        );
    });
}

fn run(b: &mut Builder<'_, '_>, case: &Case) -> Result<()> {
    account_tree(b, &case.draft, &case.specs, &case.characters)
}

fn assert_baseline(b: &mut Builder<'_, '_>, case: &Case) {
    let before = b.origins.clone();
    let links = b.links;
    let work = b.work;
    let allocator = b.allocator.state();
    let issues = b.issues;
    let draft = case.draft.clone();
    run(b, case).unwrap();
    let mut expected = before.clone();
    expected[case.tree.ordinal() as usize].links = vec![
        OwnedOriginTarget::AllocationPreset(case.draft.allocation_presets.members[1].id),
        OwnedOriginTarget::CharacterPreset(case.draft.character_presets.members[1].id),
        OwnedOriginTarget::Issue(case.selection),
    ];
    assert_eq!(b.origins, expected);
    assert_eq!(b.links, links + 3);
    assert_eq!(b.allocator.state(), allocator);
    assert_eq!(b.issues, issues);
    assert_eq!(case.draft, draft);
    b.origins = before;
    b.links = links;
    b.work = work;
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
fn tree_header_attributes_exact_selected_owners_without_completing_children() {
    with_case(|b, case| {
        assert_baseline(b, &case);
        run(b, &case).unwrap();
        assert_no_change(b, &case); // A second pass cannot duplicate links.
        assert!(matches!(
            case.draft.saved_variants.completion,
            DraftListCompletion::Pending { .. }
        ));
        for owner in &case.draft.allocation_presets.members {
            assert!(matches!(
                owner.allocations.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                owner.equipment.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
        for owner in &case.draft.character_presets.members {
            assert!(matches!(owner.class, DraftField::Pending(_)));
            assert!(matches!(
                owner.rewards.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
    });
}

#[test]
fn tree_header_refuses_missing_stale_foreign_and_aliased_owner_maps() {
    for mutation in 0..16 {
        with_case(|b, mut case| {
            assert_baseline(b, &case);
            let first = case.sources[0];
            let selected = case.sources[1];
            match mutation {
                0 => {
                    case.specs.remove(&selected);
                }
                1 => {
                    case.characters.remove(&selected);
                }
                2 => {
                    case.specs.insert(selected, usize::MAX);
                }
                3 => {
                    case.characters.insert(selected, usize::MAX);
                }
                4 => {
                    case.specs.insert(selected, 0);
                }
                5 => {
                    case.characters.insert(selected, 0);
                }
                6 => {
                    case.specs.remove(&first);
                    case.specs.insert(case.tree, 0);
                }
                7 => {
                    case.characters.remove(&first);
                    case.characters.insert(case.tree, 0);
                }
                8 => {
                    case.specs.insert(first, 1);
                    case.specs.insert(selected, 0);
                }
                9 => {
                    case.characters.insert(first, 1);
                    case.characters.insert(selected, 0);
                }
                10 => {
                    case.draft.allocation_presets.members[1].id =
                        case.draft.allocation_presets.members[0].id
                }
                11 => {
                    case.draft.character_presets.members[1].id =
                        case.draft.character_presets.members[0].id
                }
                12 => {
                    case.draft.allocation_presets.members.pop();
                }
                13 => {
                    case.draft
                        .character_presets
                        .members
                        .push(case.draft.character_presets.members[0].clone());
                }
                14 => {
                    case.specs.remove(&selected);
                    case.specs.insert(foreign_snapshot(selected), 1);
                }
                15 => {
                    case.characters.remove(&selected);
                    case.characters.insert(foreign_snapshot(selected), 1);
                }
                _ => unreachable!(),
            }
            assert_no_change(b, &case);
        });
    }
}

#[test]
fn tree_header_refuses_missing_crossed_duplicate_or_foreign_source_links() {
    for mutation in 0..17 {
        with_case(|b, case| {
            assert_baseline(b, &case);
            let selected = case.sources[1].ordinal() as usize;
            let first = case.sources[0].ordinal() as usize;
            let allocation =
                OwnedOriginTarget::AllocationPreset(case.draft.allocation_presets.members[1].id);
            let character =
                OwnedOriginTarget::CharacterPreset(case.draft.character_presets.members[1].id);
            match mutation {
                0 => b.origins[selected].links.retain(|v| v != &allocation),
                1 => b.origins[selected].links.retain(|v| v != &character),
                2 => b.origins[selected].links.push(allocation),
                3 => b.origins[selected].links.push(character),
                4 => b.origins[first].links.push(allocation),
                5 => b.origins[first].links.push(character),
                6 => {
                    b.origins[selected].links.retain(|v| v != &character);
                    b.origins[selected]
                        .links
                        .push(OwnedOriginTarget::CharacterPreset(
                            case.draft.character_presets.members[0].id,
                        ));
                }
                7 => b.origins[selected].source = case.sources[0],
                8 => b.origins[case.tree.ordinal() as usize].source = case.sources[0],
                9 => b.origins[0].source = case.tree,
                10 => {
                    b.origins[selected].disposition =
                        SourceDisposition::SourceOnly(key("foreign-source"))
                }
                11 => {
                    b.origins[case.tree.ordinal() as usize].disposition =
                        SourceDisposition::SourceOnly(key("foreign-header"))
                }
                12 => {
                    b.origins.pop();
                }
                13 => b.origins[case.tree.ordinal() as usize]
                    .links
                    .push(OwnedOriginTarget::Issue(case.selection)),
                14 => b.origins[selected].source = foreign_snapshot(case.sources[1]),
                15 => b.origins[case.tree.ordinal() as usize].source = foreign_snapshot(case.tree),
                16 => {
                    b.origins[0].source = foreign_snapshot(b.evidence.rows()[0].occurrence().id())
                }
                _ => unreachable!(),
            }
            assert_no_change(b, &case);
        });
    }
}

#[test]
fn tree_header_requires_the_actual_unresolved_root_selection_owner() {
    for mutation in 0..8 {
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
                7 => {
                    let source = case.sources[1];
                    case.draft.saved_variants.members.push(SavedVariantDraft {
                        id: b.id().unwrap(),
                        selection: SelectionDraft {
                            character: case.draft.character_presets.members[1].id.into(),
                            equipment: b.pending(source, "selection-equipment").unwrap(),
                            allocations: case.draft.allocation_presets.members[1].id.into(),
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
fn tree_header_work_and_origin_limits_fail_atomically_at_exact_boundary() {
    let mut exact_work = 0;
    let mut exact_links = 0;
    with_case(|b, case| {
        run(b, &case).unwrap();
        exact_work = b.work;
        exact_links = b.links;
        assert_eq!(b.origins[case.tree.ordinal() as usize].links.len(), 3);
    });
    for (work, links, failure) in [
        (exact_work, exact_links, None),
        (exact_work - 1, exact_links, Some("work")),
        (exact_work, exact_links - 1, Some("origin links")),
    ] {
        with_case(|b, case| {
            assert_baseline(b, &case);
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
                    assert_eq!(b.origins[case.tree.ordinal() as usize].links.len(), 3);
                }
                (Err(NormalizationError::Limit(actual)), Some(expected)) => {
                    assert_eq!(actual, expected);
                    assert_eq!(b.origins, origins);
                    assert_eq!(b.links, prior_links);
                    b.limits = NormalizationLimits::default();
                    run(b, &case).unwrap();
                    assert_eq!(b.origins[case.tree.ordinal() as usize].links.len(), 3);
                    assert_eq!(b.links, exact_links);
                }
                (result, expected) => {
                    panic!("unexpected boundary result: {result:?}, {expected:?}")
                }
            }
            assert_eq!(b.allocator.state(), allocator);
            assert_eq!(b.issues, issues);
            assert_eq!(case.draft, draft);
        });
    }
}
