//! Private ownership and provenance controls; no native selection is resolved.
use super::*;
use crate::owned_value::{DecimalSyntax, ValueCodecInput, WhitespacePolicy};

const SOURCE: &str = r#"<PathOfBuilding2><Build targetVersion="v" viewMode="IMPORT" level="73" className="73" ascendClassName="label" mainSocketGroup="2" characterLevelAutoMode="false"><PlayerStat stat="cached" value="99"/><Buffs buffList="saved"/><TimelessData devotionVariant1="1" devotionVariant2="2" searchList="" searchListFallback=""/></Build><Tree activeSpec="2"><Spec><URL>one</URL></Spec><Spec><Sockets/></Spec><Spec><Overrides/></Spec></Tree><Skills activeSkillSet="2"><SkillSet id="9"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#;

struct Case {
    draft: DraftSessionInput,
    characters: BTreeMap<SourceOccurrenceId, usize>,
    skills: BTreeMap<SourceOccurrenceId, usize>,
    build: SourceOccurrenceId,
    specs: Vec<SourceOccurrenceId>,
    sets: Vec<SourceOccurrenceId>,
    selection: DraftIssueId,
    recipe: ValueRecipe,
    frame: Option<CompiledCharacterRewardInventory<'static>>,
}

fn recipe(namespace: GameVersionNamespace) -> ValueRecipe {
    ValueRecipe::new(
        ValueRecipeInput {
            id: key("exact-character-level"),
            codec: ValueCodecInput {
                namespace,
                whitespace: WhitespacePolicy::Exact,
                codec: ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                },
            },
            tiers: vec![ValueTier {
                selectors: vec![ValueSelector {
                    lane: ValueLane::Attribute,
                    name: "level".into(),
                }],
                duplicates: DuplicatePolicy::Reject,
            }],
            missing: MissingValuePolicy::Pending,
            numeric_aliases: vec![],
        },
        Default::default(),
    )
    .unwrap()
}

fn with_source(xml: &str, test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    gem_inventory::with_usage_inventory_destination(xml, |b, skill, preset, _, _| {
        let evidence = b.evidence;
        // The shared attachment fixture does not decode scalar attributes.
        // Prepare the actual source index exactly as normalize_fresh does:
        // retain source ordinals, exclude namespaced attributes, and use the
        // same last insertion for duplicate spellings. The strict source-frame
        // proof independently rejects those duplicates before granting owners.
        assert!(b.attributes.is_empty());
        b.charge(evidence.rows().len()).unwrap();
        for row in evidence.rows() {
            b.charge(row.attributes().len()).unwrap();
            let mut attributes = BTreeMap::new();
            for (index, attribute) in row.attributes().iter().enumerate() {
                if attribute.origin().namespace.is_none() {
                    if matches!(
                        attribute.origin().name.as_str(),
                        "gemId"
                            | "variantId"
                            | "source"
                            | "itemId"
                            | "name"
                            | "classInternalId"
                            | "ascendancyInternalId"
                            | "treeVersion"
                    ) {
                        assert!(attribute.raw().len() <= b.limits.mapping.max_string_bytes);
                    }
                    attributes.insert(attribute.origin().name.as_str(), (index as u32, attribute));
                }
            }
            b.attributes.push(attributes);
        }
        let named = |name| {
            evidence
                .rows()
                .iter()
                .find(|r| r.occurrence().name() == name)
                .unwrap()
        };
        let root = evidence.rows()[0].occurrence().id();
        let build = named("Build").occurrence().id();
        let specs = named("Tree").children().to_vec();
        let sets = named("Skills").children().to_vec();
        assert_eq!(specs.len(), 3);
        assert_eq!(sets.len(), 2);
        let recipe = recipe(b.roles.input().namespace.clone());
        let mut draft = DraftSessionInput {
            allocator: b.allocator.state(),
            revision: evidence.identity().revision,
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
        let second = SkillPresetDraft {
            id: b.id().unwrap(),
            intent: None,
            authored_support_order: None,
            skills: b
                .closure(sets[1], "skill-membership-not-converted", vec![])
                .unwrap(),
            supports: b
                .closure(sets[1], "support-membership-not-converted", vec![])
                .unwrap(),
            payload_links: b
                .closure(sets[1], "payload-membership-not-converted", vec![])
                .unwrap(),
            usage_preferences: Some(
                b.closure(sets[1], "usage-preferences-not-converted", vec![])
                    .unwrap(),
            ),
        };
        b.link(sets[1], OwnedOriginTarget::SkillPreset(second.id))
            .unwrap();
        draft.skill_presets.members.push(second);
        let skills = BTreeMap::from([(sets[0], 0), (sets[1], 1)]);
        let mut characters = BTreeMap::new();
        for (index, source) in specs.iter().copied().enumerate() {
            let owner = CharacterPresetDraft {
                id: b.id().unwrap(),
                class: b.pending(source, "independent-class").unwrap(),
                ascendancy: b.pending(source, "independent-ascendancy").unwrap(),
                level: b
                    .level(
                        Some(&evidence.rows()[build.ordinal() as usize]),
                        source,
                        &recipe,
                    )
                    .unwrap(),
                rewards: b
                    .closure(source, "character-rewards-not-converted", vec![])
                    .unwrap(),
            };
            b.link(source, OwnedOriginTarget::CharacterPreset(owner.id))
                .unwrap();
            // Preserve actual owner projections on children without counting
            // them as additional independent Spec owners.
            for child in evidence.rows()[source.ordinal() as usize].children() {
                b.link(*child, OwnedOriginTarget::CharacterPreset(owner.id))
                    .unwrap();
            }
            characters.insert(source, index);
            draft.character_presets.members.push(owner);
        }
        let DraftListCompletion::Pending { id: selection, .. } = draft.saved_variants.completion
        else {
            unreachable!()
        };
        draft.allocator = b.allocator.state();
        test(
            b,
            Case {
                draft,
                characters,
                skills,
                build,
                specs,
                sets,
                selection,
                recipe,
                // A private injected frame for owner-only unit tests. Integration
                // tests independently pass this through the checked policy compiler.
                frame: Some(CompiledCharacterRewardInventory {
                    tree_version: "v",
                    target_version: "v",
                }),
            },
        );
    });
}

fn with_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    with_source(SOURCE, test);
}

fn run(b: &mut Builder<'_, '_>, case: &Case) -> Result<()> {
    source_header_owners::account_build(
        b,
        &case.draft,
        &case.characters,
        &case.skills,
        case.frame.as_ref(),
        &case.recipe,
    )
}

fn expected(case: &Case) -> Vec<OwnedOriginTarget> {
    let mut links: Vec<_> = case
        .specs
        .iter()
        .map(|source| {
            OwnedOriginTarget::CharacterPreset(
                case.draft.character_presets.members[case.characters[source]].id,
            )
        })
        .collect();
    links.extend([
        OwnedOriginTarget::SkillPreset(
            case.draft.skill_presets.members[case.skills[&case.sets[1]]].id,
        ),
        OwnedOriginTarget::Issue(case.selection),
    ]);
    links
}

fn baseline(b: &mut Builder<'_, '_>, case: &Case) {
    let build_cache = b.fresh_build_frame.clone();
    let (origins, links, work, cache) = (
        b.origins.clone(),
        b.links,
        b.work,
        b.fresh_skill_containers.clone(),
    );
    let (draft, allocator, issues) = (case.draft.clone(), b.allocator.state(), b.issues);
    run(b, case).unwrap();
    let mut after = origins.clone();
    after[case.build.ordinal() as usize].links = expected(case);
    assert_eq!(b.origins, after);
    assert_eq!(b.links, links + 5);
    assert_eq!(b.allocator.state(), allocator);
    assert_eq!(b.issues, issues);
    assert_eq!(case.draft, draft);
    b.origins = origins;
    b.links = links;
    b.work = work;
    b.fresh_skill_containers = cache;
    b.fresh_build_frame = build_cache;
}

fn refused(b: &mut Builder<'_, '_>, case: &Case) {
    let (origins, links, allocator, issues, draft) = (
        b.origins.clone(),
        b.links,
        b.allocator.state(),
        b.issues,
        case.draft.clone(),
    );
    run(b, case).unwrap();
    assert_eq!(b.origins, origins);
    assert_eq!(b.links, links);
    assert_eq!(b.allocator.state(), allocator);
    assert_eq!(b.issues, issues);
    assert_eq!(case.draft, draft);
}

fn foreign(source: SourceOccurrenceId) -> SourceOccurrenceId {
    let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
        crate::decode_build(
            SOURCE
                .replace("className=\"73\"", "className=\"other\"")
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

#[test]
fn build_header_retains_all_character_owners_and_saved_skill_without_changing_children() {
    with_case(|b, mut case| {
        baseline(b, &case);
        case.draft.character_presets.members.swap(0, 2);
        case.characters.insert(case.specs[0], 2);
        case.characters.insert(case.specs[2], 0);
        case.draft.skill_presets.members.swap(0, 1);
        case.skills.insert(case.sets[0], 1);
        case.skills.insert(case.sets[1], 0);
        baseline(b, &case);
        run(b, &case).unwrap();
        refused(b, &case);
        assert!(matches!(
            case.draft.saved_variants.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(case.draft.query_presets.members.is_empty());
        for owner in &case.draft.character_presets.members {
            assert_eq!(owner.level.to_resolved(), Some(73));
            assert!(matches!(owner.class, DraftField::Pending(_)));
            assert!(matches!(owner.ascendancy, DraftField::Pending(_)));
            assert!(matches!(
                owner.rewards.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
    });
}

#[test]
fn build_header_requires_the_actual_selected_level_attribute_not_an_equal_default() {
    for mutation in 0..8 {
        with_case(|b, mut case| {
            baseline(b, &case);
            let mut input = case.recipe.input().clone();
            match mutation {
                0 => input.tiers[0].selectors[0].name = "className".into(),
                1 => {
                    input.tiers.clear();
                    input.missing = MissingValuePolicy::Explicit {
                        value: ParameterValue::Integer(BoundedInteger::new(73).unwrap()),
                    };
                }
                2 => input.tiers[0].selectors[0].lane = ValueLane::ParentAttribute,
                3 => input.tiers[0].selectors.push(ValueSelector {
                    lane: ValueLane::Attribute,
                    name: "className".into(),
                }),
                4 => input.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder,
                5 => {
                    case.frame = None;
                }
                6 => {
                    case.frame.as_mut().unwrap().target_version = "other";
                }
                7 => {
                    let attrs = &mut b.attributes[case.build.ordinal() as usize];
                    let foreign = *attrs.get("className").unwrap();
                    attrs.insert("level", foreign);
                }
                _ => unreachable!(),
            }
            case.recipe = ValueRecipe::new(input, Default::default()).unwrap();
            refused(b, &case);
        });
    }
    // An unused default is not selected when the exact attribute is available.
    with_case(|b, mut case| {
        let mut input = case.recipe.input().clone();
        input.missing = MissingValuePolicy::Explicit {
            value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
        };
        case.recipe = ValueRecipe::new(input, Default::default()).unwrap();
        baseline(b, &case);
    });
}

#[test]
fn build_header_refuses_corrupt_character_skill_and_selection_authority() {
    for mutation in 0..26 {
        with_case(|b, mut case| {
            baseline(b, &case);
            let source = case.specs[1];
            let ordinal = source.ordinal() as usize;
            let owner =
                OwnedOriginTarget::CharacterPreset(case.draft.character_presets.members[1].id);
            match mutation {
                0 => {
                    case.characters.remove(&source);
                }
                1 => {
                    case.characters.insert(source, usize::MAX);
                }
                2 => {
                    case.characters.insert(source, 0);
                }
                3 => {
                    case.characters.remove(&source);
                    case.characters.insert(foreign(source), 1);
                }
                4 => {
                    case.characters.insert(case.build, 1);
                }
                5 => {
                    case.draft.character_presets.members[1].id =
                        case.draft.character_presets.members[0].id
                }
                6 => case.draft.character_presets.members[1].id = b.id().unwrap(),
                7 => case.draft.character_presets.members[1].level = 74.into(),
                8 => {
                    case.draft.character_presets.members[1].level =
                        b.pending(source, "level-unresolved").unwrap()
                }
                9 => b.origins[ordinal].links.retain(|link| link != &owner),
                10 => b.origins[ordinal].links.push(owner),
                11 => b.origins[case.specs[0].ordinal() as usize]
                    .links
                    .push(owner),
                12 => b.origins[ordinal].source = foreign(source),
                13 => b.origins[ordinal].disposition = SourceDisposition::SourceOnly(key("other")),
                14 => b.origins[case.build.ordinal() as usize].source = foreign(case.build),
                15 => b.origins[case.build.ordinal() as usize]
                    .links
                    .push(OwnedOriginTarget::Issue(case.selection)),
                16 => {
                    b.origins[case.build.ordinal() as usize].disposition =
                        SourceDisposition::SourceOnly(key("other"))
                }
                17 => {
                    case.skills.remove(&case.sets[1]);
                }
                18 => {
                    case.skills.insert(case.sets[1], 0);
                }
                19 => {
                    case.draft.skill_presets.members[1].id = case.draft.skill_presets.members[0].id
                }
                20 => b.origins[case.sets[1].ordinal() as usize].source = foreign(case.sets[1]),
                21 => b.origins[0].links.clear(),
                22 => b.origins[0]
                    .links
                    .push(OwnedOriginTarget::Issue(case.selection)),
                23 => b.origins[0].source = foreign(b.evidence.rows()[0].occurrence().id()),
                24 => case.draft.saved_variants.completion = DraftListCompletion::Complete,
                25 => {
                    b.origins.pop();
                }
                _ => unreachable!(),
            }
            refused(b, &case);
        });
    }
}

#[test]
fn build_header_unknown_or_unproved_source_shapes_preserve_fallback() {
    with_case(|b, case| baseline(b, &case));
    for (before, after) in [
        ("viewMode=\"IMPORT\"", "viewMode=\"unknown\""),
        ("targetVersion=\"v\"", "targetVersion=\"other\""),
        (
            "characterLevelAutoMode=\"false\"",
            "characterLevelAutoMode=\"true\"",
        ),
        ("mainSocketGroup=\"2\"", "mainSocketGroup=\"3\""),
        ("mainSocketGroup=\"2\"", "mainSocketGroup=\"02\""),
        ("<Build ", "<Build mainSkillIndex=\"1\" "),
        ("<Build ", "<Build xmlns=\"urn:foreign\" "),
        ("</Build>", "<Unknown/></Build>"),
        ("</Build>", "<Buffs/></Build>"),
        ("</Build>", "</Build><Build/>"),
        ("</Build>", "</Build><Build xmlns=\"urn:foreign\"/>"),
        ("<PathOfBuilding2>", "<PathOfBuilding2 future=\"true\">"),
    ] {
        assert_eq!(SOURCE.matches(before).count(), 1);
        with_source(&SOURCE.replacen(before, after, 1), |b, case| {
            refused(b, &case)
        });
    }
}

#[test]
fn build_header_exact_work_and_origin_limits_never_partially_append_owners() {
    for cached in [false, true] {
        let mut work = 0;
        let mut links = 0;
        with_case(|b, case| {
            if cached {
                assert!(skill_source_census::container_sets(b).unwrap().is_some());
            }
            run(b, &case).unwrap();
            work = b.work;
            links = b.links;
        });
        for (max_work, max_links, error) in [
            (work, links, None),
            (work - 1, links, Some("work")),
            (work, links - 1, Some("origin links")),
        ] {
            with_case(|b, case| {
                baseline(b, &case);
                if cached {
                    assert!(skill_source_census::container_sets(b).unwrap().is_some());
                }
                let (before, prior_links, allocator, issues, draft) = (
                    b.origins.clone(),
                    b.links,
                    b.allocator.state(),
                    b.issues,
                    case.draft.clone(),
                );
                b.limits.max_work = max_work;
                b.limits.max_origin_links = max_links;
                match (run(b, &case), error) {
                    (Ok(()), None) => {
                        assert_eq!(b.work, work);
                        assert_eq!(b.links, links);
                    }
                    (Err(NormalizationError::Limit(actual)), Some(expected)) => {
                        assert_eq!(actual, expected);
                        assert_eq!(b.origins, before);
                        assert_eq!(b.links, prior_links);
                        b.limits = Default::default();
                        run(b, &case).unwrap();
                    }
                    (actual, expected) => panic!("{actual:?}, {expected:?}"),
                }
                let mut after = before;
                after[case.build.ordinal() as usize].links = expected(&case);
                assert_eq!(b.origins, after);
                assert_eq!(b.links, links);
                assert_eq!(b.allocator.state(), allocator);
                assert_eq!(b.issues, issues);
                assert_eq!(case.draft, draft);
            });
        }
    }
}

#[test]
fn build_frame_cache_reuses_only_exact_source_target_and_child_ceiling() {
    with_case(|b, mut case| {
        let row = &b.evidence.rows()[case.build.ordinal() as usize];
        let (origins, allocator, issues, links, draft) = (
            b.origins.clone(),
            b.allocator.state(),
            b.issues,
            b.links,
            case.draft.clone(),
        );
        assert!(b.fresh_build_frame.is_none());
        let before = b.work;
        assert!(fresh_build(b, row, "v").unwrap());
        let cold = b.work - before;
        let before = b.work;
        assert!(fresh_build(b, row, "v").unwrap());
        assert_eq!(b.work - before, 2);
        assert!(cold > 2);

        let before = b.work;
        assert!(!fresh_build(b, row, "other").unwrap());
        assert!(b.work - before > "other".len() + 1);
        assert_eq!(
            b.fresh_build_frame.as_ref().unwrap().target_version,
            "other"
        );
        let before = b.work;
        assert!(!fresh_build(b, row, "other").unwrap());
        assert_eq!(b.work - before, "other".len() + 1);
        assert!(fresh_build(b, row, "v").unwrap());

        // Same ordinal from another immutable snapshot cannot use this proof.
        b.fresh_build_frame.as_mut().unwrap().source = foreign(case.build);
        let before = b.work;
        assert!(fresh_build(b, row, "v").unwrap());
        assert_eq!(b.work - before, cold);
        assert_eq!(b.fresh_build_frame.as_ref().unwrap().source, case.build);
        // An actual different occurrence also needs its own structural check.
        let other = &b.evidence.rows()[case.specs[0].ordinal() as usize];
        assert!(!fresh_build(b, other, "v").unwrap());
        assert_eq!(b.fresh_build_frame.as_ref().unwrap().source, case.specs[0]);
        assert!(fresh_build(b, row, "v").unwrap());

        let ceiling = b.limits.draft.input.max_collection_entries;
        b.limits.draft.input.max_collection_entries = row.children().len() - 1;
        assert!(matches!(
            fresh_build(b, row, "v"),
            Err(NormalizationError::Limit(
                "character reward source children"
            ))
        ));
        assert_eq!(b.fresh_build_frame.as_ref().unwrap().max_children, ceiling);
        b.limits.draft.input.max_collection_entries = ceiling - 1;
        let before = b.work;
        assert!(fresh_build(b, row, "v").unwrap());
        assert_eq!(b.work - before, cold);
        assert_eq!(
            b.fresh_build_frame.as_ref().unwrap().max_children,
            ceiling - 1
        );
        b.limits.draft.input.max_collection_entries = ceiling;

        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), allocator);
        assert_eq!(b.issues, issues);
        assert_eq!(b.links, links);
        assert_eq!(case.draft, draft);
        // A source-frame cache never certifies current owner levels.
        assert!(fresh_build(b, row, "v").unwrap());
        case.draft.character_presets.members[1].level = 74.into();
        refused(b, &case);
    });
}

#[test]
fn build_frame_cache_never_retains_resource_errors_or_scans_a_disabled_pass() {
    let mut exact = 0;
    with_case(|b, case| {
        let row = &b.evidence.rows()[case.build.ordinal() as usize];
        let before = b.work;
        assert!(fresh_build(b, row, "v").unwrap());
        exact = b.work - before;
    });
    for remaining in [2, exact - 1] {
        with_case(|b, case| {
            let row = &b.evidence.rows()[case.build.ordinal() as usize];
            let (origins, allocator, issues, links) =
                (b.origins.clone(), b.allocator.state(), b.issues, b.links);
            b.limits.max_work = b.work + remaining;
            assert!(matches!(
                fresh_build(b, row, "v"),
                Err(NormalizationError::Limit("work"))
            ));
            assert!(b.fresh_build_frame.is_none());
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
            assert_eq!(b.issues, issues);
            assert_eq!(b.links, links);
            b.limits = Default::default();
            assert!(fresh_build(b, row, "v").unwrap());
            b.limits.max_work = b.work + 1;
            assert!(matches!(
                fresh_build(b, row, "v"),
                Err(NormalizationError::Limit("work"))
            ));
            assert!(b.fresh_build_frame.as_ref().unwrap().accepted);
            b.limits = Default::default();
            let before = b.work;
            assert!(fresh_build(b, row, "v").unwrap());
            assert_eq!(b.work - before, 2);
        });
    }
    with_case(|b, mut case| {
        case.frame = None;
        let work = b.work;
        refused(b, &case);
        assert_eq!(b.work, work);
        assert!(b.fresh_build_frame.is_none());
    });
}
