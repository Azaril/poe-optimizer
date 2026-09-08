use super::*;
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
const TEMPLATE: &str = include_str!("../../../tests/fixtures/calibration/mace-wooden.xml");
fn weapons(data: &GameDataPackage) -> Vec<NormalMaceAlternative> {
    data.weapons
        .iter()
        .map(|weapon| NormalMaceAlternative {
            id: weapon.id.clone(),
            item_text: format!(
                "Rarity: NORMAL\n{}\nItem Level: 1\nQuality: 0\nImplicits: 0",
                weapon.name
            ),
        })
        .collect()
}
fn custom(edit: impl FnOnce(&mut GameDataPackage)) -> Arc<GameDataSnapshot> {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    edit(&mut package);
    package.refresh_section_digests().unwrap();
    Arc::new(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap(),
    )
}
fn catalog(data: Arc<GameDataSnapshot>) -> ControlledMaceCatalog {
    let alternatives = weapons(data.package());
    ControlledMaceCatalog::with_data(
        data,
        TEMPLATE.into(),
        alternatives,
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
    )
    .unwrap()
}
fn assessment(
    registry: &ControlledMaceCatalog,
    weapon: usize,
    support: MaceSupportChoice,
) -> MaceRequirementAssessment {
    let candidate = registry
        .resolve_candidate(&registry.snapshot().package().weapons[weapon].id, support)
        .unwrap();
    registry.requirements(candidate).unwrap()
}
#[test]
fn reviewed_default_preserves_materialization_and_maximum_semantics() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let default = ControlledMaceCatalog::new(
        TEMPLATE.into(),
        weapons(data.package()),
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
    )
    .unwrap();
    let explicit = catalog(data);
    assert_eq!(default.catalog().identity, explicit.catalog().identity);
    assert_eq!(default.alternatives().len(), 4);
    let source = Document::parse(TEMPLATE).unwrap();
    let active_range = source
        .descendants()
        .find(|node| node.has_tag_name("Gem"))
        .unwrap()
        .range();
    for alternative in default.alternatives() {
        let candidate = explicit
            .resolve_loadout_candidate(&alternative.weapon_id, &alternative.support)
            .unwrap();
        assert_eq!(&alternative.candidate, candidate);
        assert_eq!(
            default.materialize(candidate).unwrap().content,
            explicit.materialize(candidate).unwrap().content
        );
        assert!(default.requirements(candidate).unwrap().is_legal());
        let xml = default.materialize(candidate).unwrap().content;
        let actual = Document::parse(&xml).unwrap();
        let actual_range = actual
            .descendants()
            .find(|node| node.has_tag_name("Gem"))
            .unwrap()
            .range();
        assert_eq!(&TEMPLATE[active_range.clone()], &xml[actual_range]);
        assert!(xml.contains("Controlled attack/weapon/support host-calibration fixture"));
    }
    let with_support = assessment(&default, 1, MaceSupportChoice::BrutalityI);
    assert_eq!(
        with_support.required.strength, 11,
        "red support cost 5 does not add to weapon STR 11"
    );
    assert_eq!(with_support.required.level, 0);
}
#[test]
fn weapon_attribute_boundaries_keep_all_diagnostic_alternatives() {
    let base = game_data::bundled_snapshot()
        .unwrap()
        .tree()
        .class(6)
        .unwrap()
        .base_strength;
    for required in [base - 1, base, base + 1] {
        let registry = catalog(custom(|data| {
            data.weapons[1].requirements.attributes.strength = required
        }));
        assert_eq!(registry.alternatives().len(), 4);
        for support in [MaceSupportChoice::None, MaceSupportChoice::BrutalityI] {
            let result = assessment(&registry, 1, support);
            assert_eq!(result.available.strength, base);
            assert_eq!(result.required.strength, required);
            assert_eq!(result.is_legal(), required <= base);
            let candidate = registry
                .resolve_candidate(&registry.snapshot().package().weapons[1].id, support)
                .unwrap();
            assert_eq!(
                registry.validate_requirements(candidate).is_ok(),
                required <= base
            );
            assert!(registry.materialize(candidate).is_ok());
            if required > base {
                assert_eq!(
                    result.violations,
                    vec![MaceRequirementViolation {
                        requirement: "strength".into(),
                        required,
                        available: base
                    }]
                );
                assert!(
                    registry
                        .validate_requirements(candidate)
                        .unwrap_err()
                        .to_string()
                        .contains("strength requires")
                );
            }
        }
    }
}
#[test]
fn support_color_cost_boundaries_and_active_attributes_use_maximum() {
    for color in [SupportColor::Red, SupportColor::Green, SupportColor::Blue] {
        let initial = game_data::bundled_snapshot().unwrap();
        let class = initial.tree().class(6).unwrap();
        let available = match color {
            SupportColor::Red => class.base_strength,
            SupportColor::Green => class.base_dexterity,
            SupportColor::Blue => class.base_intelligence,
        };
        for cost in [available - 1, available, available + 1] {
            let registry = catalog(custom(|data| {
                data.supports
                    .iter_mut()
                    .find(|support| support.id == "brutality_i")
                    .unwrap()
                    .color = color;
                match color {
                    SupportColor::Red => data.mace.support_attribute_costs.strength = cost,
                    SupportColor::Green => data.mace.support_attribute_costs.dexterity = cost,
                    SupportColor::Blue => data.mace.support_attribute_costs.intelligence = cost,
                }
            }));
            assert!(assessment(&registry, 0, MaceSupportChoice::None).is_legal());
            assert_eq!(
                assessment(&registry, 0, MaceSupportChoice::BrutalityI).is_legal(),
                cost <= available
            );
        }
    }
    let registry = catalog(custom(|data| {
        let class = data.tree.class(6).unwrap();
        data.mace.requirements.attributes.strength = class.base_strength;
        data.mace.requirements.attributes.dexterity = class.base_dexterity;
        data.mace.requirements.attributes.intelligence = class.base_intelligence;
    }));
    let result = assessment(&registry, 1, MaceSupportChoice::BrutalityI);
    assert!(
        result.is_legal(),
        "independent requirements fit despite their sums exceeding attributes"
    );
    assert_eq!(result.required.strength, result.available.strength);
    assert_eq!(result.required.dexterity, result.available.dexterity);
    assert_eq!(result.required.intelligence, result.available.intelligence);
    let registry = catalog(custom(|data| {
        data.mace.requirements.attributes.intelligence =
            data.tree.class(6).unwrap().base_intelligence + 1
    }));
    assert_eq!(
        assessment(&registry, 0, MaceSupportChoice::None).violations[0].requirement,
        "intelligence"
    );
}
#[test]
fn equip_and_gem_level_requirements_are_distinct_from_item_level() {
    for required in [59, 60, 61] {
        for target in ["weapon", "active", "support"] {
            let registry = catalog(custom(|data| match target {
                "weapon" => data.weapons[0].requirements.level = required,
                "active" => data.mace.requirements.level = required,
                _ => {
                    data.supports
                        .iter_mut()
                        .find(|support| support.id == "brutality_i")
                        .unwrap()
                        .requirements
                        .level = required
                }
            }));
            let result = assessment(&registry, 0, MaceSupportChoice::BrutalityI);
            assert_eq!(result.required.level, required);
            assert_eq!(result.is_legal(), required <= 60, "{target}");
        }
    }
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let mut alternatives = weapons(data.package());
    alternatives[0].item_text = alternatives[0]
        .item_text
        .replace("Item Level: 1", "Item Level: 100");
    let registry = ControlledMaceCatalog::with_data(
        data,
        TEMPLATE.replacen("level=\"60\"", "level=\"1\"", 1),
        alternatives,
        vec![MaceSupportChoice::None],
    )
    .unwrap();
    assert!(assessment(&registry, 0, MaceSupportChoice::None).is_legal());
}
#[test]
fn identical_content_snapshots_share_candidates_but_cross_data_rejects() {
    let first = catalog(Arc::new(game_data::bundled_snapshot().unwrap()));
    let bytes = Arc::new(
        GameDataLoader::from_bytes(
            game_data::bundled_package_bytes(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap(),
    );
    let second = catalog(bytes);
    let changed = catalog(custom(|data| data.character.initial_life += 1.0));
    assert_ne!(first.snapshot().trust(), second.snapshot().trust());
    assert_eq!(first.data_identity(), second.data_identity());
    assert_eq!(first.catalog().identity, second.catalog().identity);
    assert_ne!(first.catalog().identity, changed.catalog().identity);
    let candidate = &first.alternatives()[0].candidate;
    assert_eq!(
        first.materialize(candidate).unwrap().content,
        second.materialize(candidate).unwrap().content
    );
    assert!(matches!(
        changed.materialize(candidate),
        Err(ControlledMutationError::UnknownCandidate)
    ));
    assert!(matches!(
        changed.requirements(candidate),
        Err(ControlledMutationError::UnknownCandidate)
    ));
}
#[test]
fn selected_skill_support_and_weapon_strings_round_trip_through_xml() {
    let data = custom(|data| {
        data.mace.name = "Mace \"&<>' ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.mace.skill_id = "active\"&<>'ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.mace.game_id = "active-game\"&<>'ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.mace.variant_id = "active-variant\"&<>'ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.supports
            .iter_mut()
            .find(|support| support.id == "brutality_i")
            .unwrap()
            .name = "Brutality \"&<>' ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.supports
            .iter_mut()
            .find(|support| support.id == "brutality_i")
            .unwrap()
            .skill_id = "support\"&<>'ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.supports
            .iter_mut()
            .find(|support| support.id == "brutality_i")
            .unwrap()
            .game_id = "support-game\"&<>'ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.supports
            .iter_mut()
            .find(|support| support.id == "brutality_i")
            .unwrap()
            .variant_id = "support-variant\"&<>'ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
        data.weapons[0].name = "Club \"&<>' ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into();
    });
    let package = data.package();
    let original = game_data::bundled_snapshot().unwrap();
    let mut template = TEMPLATE.to_owned();
    for (old, new) in [
        (&original.package().mace.name, &package.mace.name),
        (&original.package().mace.skill_id, &package.mace.skill_id),
        (&original.package().mace.game_id, &package.mace.game_id),
        (
            &original.package().mace.variant_id,
            &package.mace.variant_id,
        ),
        (
            &original.package().weapons[0].name,
            &package.weapons[0].name,
        ),
    ] {
        template = template.replace(old, &escape_attribute(new));
    }
    let alternatives = weapons(package);
    let registry = ControlledMaceCatalog::with_data(
        data,
        template,
        alternatives,
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
    )
    .unwrap();
    assert_eq!(
        registry.required_skill_id(),
        registry.snapshot().package().mace.skill_id
    );
    for alternative in registry.alternatives() {
        let xml = registry
            .materialize(&alternative.candidate)
            .unwrap()
            .content;
        crate::xml_compat::validate_native(&xml).unwrap();
        let reparsed = profile(&xml, registry.snapshot().package()).unwrap();
        assert_eq!(
            reparsed.active_attributes["skillId"],
            registry.required_skill_id()
        );
        let document = Document::parse(&xml).unwrap();
        if alternative.support.legacy_choice() == Some(MaceSupportChoice::BrutalityI) {
            let support = document
                .descendants()
                .filter(|node| node.has_tag_name("Gem"))
                .nth(1)
                .unwrap();
            assert_eq!(
                support.attribute("nameSpec"),
                Some(
                    registry
                        .snapshot()
                        .package()
                        .support("brutality_i")
                        .unwrap()
                        .name
                        .as_str()
                )
            );
            assert_eq!(
                support.attribute("skillId"),
                Some(
                    registry
                        .snapshot()
                        .package()
                        .support("brutality_i")
                        .unwrap()
                        .skill_id
                        .as_str()
                )
            );
        }
    }
}
#[test]
fn selected_quest_names_are_validated_and_lexical_normalization_rejects() {
    let data = custom(|data| data.quests.config_keys[0] = "quest\"&'ÃƒÂ©Ã¢â‚¬ÂºÃ‚Âª".into());
    let key = &data.package().quests.config_keys[0];
    let template = TEMPLATE.replace(
        "</ConfigSet>",
        &format!(
            "<Input name=\"{}\" boolean=\"false\"/></ConfigSet>",
            escape_attribute(key)
        ),
    );
    let alternatives = weapons(data.package());
    let registry = ControlledMaceCatalog::with_data(
        data.clone(),
        template.clone(),
        alternatives.clone(),
        vec![MaceSupportChoice::None],
    )
    .unwrap();
    assert_eq!(registry.profile.config[key], Scalar::Boolean(false));
    let invalid = template.replace(&escape_attribute(key), "questCandlemass");
    assert!(
        ControlledMaceCatalog::with_data(
            data.clone(),
            invalid,
            alternatives.clone(),
            vec![MaceSupportChoice::None]
        )
        .is_err()
    );
    let invalid = template.replace("Single Mace Strike", "Single\nMace Strike");
    assert!(
        ControlledMaceCatalog::with_data(
            data,
            invalid,
            alternatives,
            vec![MaceSupportChoice::None]
        )
        .unwrap_err()
        .to_string()
        .contains("normalize differently")
    );
}

#[test]
fn all_admitted_tree_choices_compose_and_round_trip_without_touching_source_prose() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let trees = class_tree::selections(data.tree()).unwrap();
    assert_eq!(trees.len(), 105);
    let source = TEMPLATE
        .replace(
            "<Tree activeSpec=\"1\">",
            "<!-- Warrior classId=\"3\" nodes=\"\" -->\n  <Tree activeSpec=\"1\">",
        )
        .replace(
            "Unallocated Warrior",
            "Literal Warrior and nodes=&quot;47175&quot;",
        );
    let registry = ControlledMaceCatalog::with_tree_choices(
        data.clone(),
        source.clone(),
        weapons(data.package()),
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
        trees.clone(),
    )
    .unwrap();
    assert_eq!(registry.tree_choices(), trees);
    assert_eq!(registry.alternatives().len(), 420);
    let source_document = Document::parse(&source).unwrap();
    let source_build = child(source_document.root_element(), "Build").unwrap();
    let source_spec = child(
        child(source_document.root_element(), "Tree").unwrap(),
        "Spec",
    )
    .unwrap();
    let rules = CandidateDomain::new(
        registry.catalog().clone(),
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: 1,
                ascendancy_passive_points: 1,
                active_skill_count: 1,
                supports_per_skill: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let mut fingerprints = BTreeSet::new();
    for alternative in registry.alternatives() {
        let resolved = alternative.tree.resolve(data.tree()).unwrap();
        let candidate = registry
            .resolve_tree_loadout_candidate(
                &alternative.tree,
                &alternative.weapon_id,
                &alternative.support,
            )
            .unwrap();
        assert_eq!(candidate, &alternative.candidate);
        let admitted = rules.validate(candidate);
        assert!(
            admitted.is_searchable(),
            "{}: {:?}",
            alternative.id,
            admitted
        );
        assert_eq!(candidate.class_id, resolved.class.integer_id.to_string());
        assert_eq!(
            candidate.ascendancy_id,
            resolved
                .ascendancy
                .as_ref()
                .map(|asc| asc.internal_id.clone())
        );
        assert_eq!(
            candidate.passives,
            alternative
                .tree
                .entrance_node_id
                .into_iter()
                .chain(alternative.tree.ascendancy_node_id)
                .collect()
        );
        let xml = registry.materialize(candidate).unwrap().content;
        assert_eq!(hash(&xml), alternative.xml_sha256);
        assert!(fingerprints.insert(alternative.xml_sha256.clone()));
        assert!(xml.contains("<!-- Warrior classId=\"3\" nodes=\"\" -->"));
        assert!(xml.contains("title=\"Literal Warrior and nodes=&quot;47175&quot;\""));
        assert!(xml.contains("Controlled attack/weapon/support host-calibration fixture"));
        let document = Document::parse(&xml).unwrap();
        let build = child(document.root_element(), "Build").unwrap();
        let spec = child(child(document.root_element(), "Tree").unwrap(), "Spec").unwrap();
        assert_eq!(
            build.attribute("className"),
            Some(resolved.class.name.as_str())
        );
        assert_eq!(
            spec.attribute("classId"),
            Some(resolved.class.integer_id.to_string().as_str())
        );
        assert_eq!(
            spec.attribute("nodes"),
            Some(
                alternative
                    .tree
                    .entrance_node_id
                    .into_iter()
                    .chain(alternative.tree.ascendancy_node_id)
                    .collect::<BTreeSet<_>>()
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
                    .as_str()
            )
        );
        for attribute in source_build
            .attributes()
            .filter(|a| !["className", "ascendClassName"].contains(&a.name()))
        {
            assert_eq!(build.attribute(attribute.name()), Some(attribute.value()));
        }
        for attribute in source_spec.attributes().filter(|a| {
            ![
                "classId",
                "classInternalId",
                "ascendClassId",
                "ascendancyInternalId",
                "nodes",
            ]
            .contains(&a.name())
        }) {
            assert_eq!(spec.attribute(attribute.name()), Some(attribute.value()));
        }
        let parsed = profile(&xml, data.package()).unwrap();
        assert_eq!(parsed.tree.selection, alternative.tree);
        let requirements = registry.requirements(candidate).unwrap();
        assert_eq!(
            requirements.available.strength,
            resolved.base_attributes.strength
        );
        assert_eq!(
            requirements.available.dexterity,
            resolved.base_attributes.dexterity
        );
        assert_eq!(
            requirements.available.intelligence,
            resolved.base_attributes.intelligence
        );
    }
}

#[test]
fn expanded_domain_is_order_independent_and_source_template_can_be_any_admitted_tree() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let trees = class_tree::selections(data.tree()).unwrap();
    let make = |trees: Vec<ClassTreeSelection>, source: String| {
        ControlledMaceCatalog::with_tree_choices(
            data.clone(),
            source,
            weapons(data.package()),
            vec![MaceSupportChoice::None],
            trees,
        )
        .unwrap()
    };
    let first = make(trees.clone(), TEMPLATE.into());
    let second = make(trees.iter().cloned().rev().collect(), TEMPLATE.into());
    assert_eq!(first.catalog().identity, second.catalog().identity);
    let selected = trees
        .iter()
        .find(|tree| {
            tree.class_id != 6 && tree.ascendancy_id.is_some() && tree.entrance_node_id.is_some()
        })
        .unwrap();
    let candidate = first
        .resolve_tree_candidate(
            selected,
            &data.package().weapons[0].id,
            MaceSupportChoice::None,
        )
        .unwrap();
    let source = first.materialize(candidate).unwrap().content;
    let switched = make(vec![fixed_warrior(), selected.clone()], source.clone());
    let legacy_resolved = switched
        .resolve_candidate(&data.package().weapons[0].id, MaceSupportChoice::None)
        .unwrap();
    assert_eq!(legacy_resolved.class_id, selected.class_id.to_string());
    assert!(
        ControlledMaceCatalog::with_data(
            data.clone(),
            source,
            weapons(data.package()),
            vec![MaceSupportChoice::None]
        )
        .is_err()
    );
    let back = switched
        .resolve_tree_candidate(
            &fixed_warrior(),
            &data.package().weapons[0].id,
            MaceSupportChoice::None,
        )
        .unwrap();
    assert_eq!(
        profile(&switched.materialize(back).unwrap().content, data.package())
            .unwrap()
            .tree
            .selection,
        fixed_warrior()
    );
    assert!(first.materialize(back).is_err());
}

#[test]
fn invalid_tree_ownership_roots_duplicates_and_allocation_count_fail_closed() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let make = |trees, xml: String| {
        ControlledMaceCatalog::with_tree_choices(
            data.clone(),
            xml,
            weapons(data.package()),
            vec![MaceSupportChoice::None],
            trees,
        )
    };
    let root = data.tree().class(6).unwrap().start_node_id;
    let foreign = class_tree::selections(data.tree())
        .unwrap()
        .into_iter()
        .find(|tree| tree.class_id != 6 && tree.ascendancy_id.is_some())
        .unwrap();
    for trees in [
        vec![],
        vec![fixed_warrior(), fixed_warrior()],
        vec![ClassTreeSelection {
            entrance_node_id: Some(root),
            ..fixed_warrior()
        }],
        vec![ClassTreeSelection {
            ascendancy_id: foreign.ascendancy_id,
            ..fixed_warrior()
        }],
    ] {
        assert!(make(trees, TEMPLATE.into()).is_err());
    }
    for xml in [
        TEMPLATE.replace("className=\"Warrior\"", "className=\"Witch\""),
        TEMPLATE.replace("ascendClassId=\"0\"", "ascendClassId=\"1\""),
        TEMPLATE.replace("nodes=\"\"", &format!("nodes=\"{root},{root}\"")),
        TEMPLATE.replace("nodes=\"\"", "nodes=\"1,2\""),
    ] {
        assert!(make(vec![fixed_warrior()], xml).is_err());
    }
}

#[test]
fn selected_class_changes_requirement_legality_without_changing_maximum_semantics() {
    let data = custom(|package| {
        package.weapons[1].requirements.attributes.strength = 12;
    });
    let trees = class_tree::selections(data.tree()).unwrap();
    let registry = ControlledMaceCatalog::with_tree_choices(
        data.clone(),
        TEMPLATE.into(),
        weapons(data.package()),
        vec![MaceSupportChoice::BrutalityI],
        trees,
    )
    .unwrap();
    let mut legal = BTreeSet::new();
    let mut illegal = BTreeSet::new();
    for alternative in registry
        .alternatives()
        .iter()
        .filter(|alt| alt.weapon_id == data.package().weapons[1].id)
    {
        let assessment = registry.requirements(&alternative.candidate).unwrap();
        assert_eq!(assessment.required.strength, 12);
        if assessment.available.strength >= 12 {
            assert!(assessment.is_legal());
            legal.insert(alternative.tree.class_id);
        } else {
            assert!(!assessment.is_legal());
            illegal.insert(alternative.tree.class_id);
        }
    }
    assert!(!legal.is_empty());
    assert!(!illegal.is_empty());
}

#[test]
fn expanded_materialization_preparation_work_is_bounded() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let source = TEMPLATE.replace(
        "<PathOfBuilding2>",
        &format!("<PathOfBuilding2><!--{}-->", "x".repeat(800_000)),
    );
    let result = ControlledMaceCatalog::with_tree_choices(
        data.clone(),
        source,
        weapons(data.package()),
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
        class_tree::selections(data.tree()).unwrap(),
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("256 MiB preparation budget")
    );
}

#[test]
fn ascendancy_passive_materialization_preserves_two_allocations_and_rejects_foreign_ownership() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let chosen = class_tree::selections(data.tree())
        .unwrap()
        .into_iter()
        .find(|tree| tree.entrance_node_id.is_some() && tree.ascendancy_node_id.is_some())
        .unwrap();
    let make = |source: String, trees| {
        ControlledMaceCatalog::with_tree_choices(
            data.clone(),
            source,
            weapons(data.package()),
            vec![MaceSupportChoice::None],
            trees,
        )
    };
    let first = make(TEMPLATE.into(), vec![chosen.clone()]).unwrap();
    let source = first
        .materialize(&first.alternatives()[0].candidate)
        .unwrap()
        .content;
    let parsed = profile(&source, data.package()).unwrap();
    assert_eq!(parsed.tree.selection, chosen);
    assert_eq!(parsed.tree.allocated_nodes.len(), 4);
    let second = make(source.clone(), vec![chosen.clone(), fixed_warrior()]).unwrap();
    let restored = second
        .resolve_tree_candidate(
            &fixed_warrior(),
            &data.package().weapons[0].id,
            MaceSupportChoice::None,
        )
        .unwrap();
    let restored_xml = second.materialize(restored).unwrap().content;
    assert!(restored_xml.contains("nodes=\"\""));
    assert!(restored_xml.contains("Controlled attack/weapon/support host-calibration fixture"));
    assert_eq!(
        profile(&restored_xml, data.package())
            .unwrap()
            .tree
            .selection,
        fixed_warrior()
    );
    let invalid = ClassTreeSelection {
        ascendancy_id: None,
        ..chosen.clone()
    };
    assert!(make(TEMPLATE.into(), vec![invalid]).is_err());
    let foreign = ClassTreeSelection {
        ascendancy_node_id: chosen.ascendancy_node_id,
        ..fixed_warrior()
    };
    assert!(make(TEMPLATE.into(), vec![foreign]).is_err());
    let old_node = chosen.ascendancy_node_id.unwrap();
    let old_nodes = [chosen.entrance_node_id.unwrap(), old_node]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let duplicated = source.replace(
        &format!("nodes=\"{old_nodes}\""),
        &format!("nodes=\"{old_nodes},{old_node}\""),
    );
    assert_ne!(duplicated, source);
    assert!(make(duplicated, vec![chosen]).is_err());
}

fn all_loadouts() -> Vec<MaceSupportLoadout> {
    [
        vec![],
        vec!["brutality_i"],
        vec!["heavy_swing"],
        vec!["rapid_attacks_i"],
        vec!["brutality_i", "heavy_swing"],
        vec!["brutality_i", "rapid_attacks_i"],
        vec!["heavy_swing", "rapid_attacks_i"],
    ]
    .into_iter()
    .map(|keys| MaceSupportLoadout::new(keys.into_iter().map(str::to_owned).collect()).unwrap())
    .collect()
}
#[test]
fn support_loadouts_are_unordered_unique_bounded_data_keys() {
    let pair: MaceSupportLoadout =
        serde_json::from_str(r#"["rapid_attacks_i","heavy_swing"]"#).unwrap();
    assert_eq!(pair.id(), "heavy_swing+rapid_attacks_i");
    assert_eq!(
        serde_json::to_string(&pair).unwrap(),
        r#"["heavy_swing","rapid_attacks_i"]"#
    );
    for invalid in [
        r#"["heavy_swing","heavy_swing"]"#,
        r#"["brutality_i","heavy_swing","rapid_attacks_i"]"#,
        r#"["heavy_swing+rapid_attacks_i"]"#,
        r#"[""]"#,
    ] {
        assert!(serde_json::from_str::<MaceSupportLoadout>(invalid).is_err());
    }
    assert_eq!(
        MaceSupportLoadout::from(MaceSupportChoice::None).id(),
        "none"
    );
    assert_eq!(
        MaceSupportLoadout::from(MaceSupportChoice::BrutalityI).legacy_choice(),
        Some(MaceSupportChoice::BrutalityI)
    );
    assert_eq!(pair.legacy_choice(), None);
}
#[test]
fn seven_support_loadouts_preserve_exact_gem_instances_and_comments_on_removal() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = ControlledMaceCatalog::with_loadouts(
        data.clone(),
        TEMPLATE.into(),
        weapons(data.package()),
        all_loadouts(),
    )
    .unwrap();
    assert_eq!(registry.alternatives().len(), 14);
    assert_eq!(registry.catalog().supports.len(), 3);
    for alternative in registry.alternatives() {
        assert_eq!(
            registry.resolve_loadout_candidate(&alternative.weapon_id, &alternative.support),
            Some(&alternative.candidate)
        );
        let xml = registry
            .materialize(&alternative.candidate)
            .unwrap()
            .content;
        let parsed = profile(&xml, data.package()).unwrap();
        assert_eq!(parsed.support, alternative.support);
        assert_eq!(parsed.support_order, alternative.support.keys());
        assert_eq!(
            alternative.candidate.skills[SLOT]
                .support_instance_ids
                .len(),
            alternative.support.keys().len()
        );
        assert_eq!(hash(&xml), alternative.xml_sha256);
        for key in alternative.support.keys() {
            let gem = data.package().support(key).unwrap();
            let instance = registry
                .catalog()
                .supports
                .values()
                .find(|instance| instance.definition_id == gem.skill_id)
                .unwrap();
            let fields: BTreeMap<String, String> =
                serde_json::from_str(&instance.payload.content).unwrap();
            assert_eq!(fields["skillId"], gem.skill_id);
            assert_eq!(fields["gemId"], gem.game_id);
            assert_eq!(fields["variantId"], gem.variant_id);
        }
    }
    let pair =
        MaceSupportLoadout::new(vec!["brutality_i".into(), "rapid_attacks_i".into()]).unwrap();
    let pair_candidate = registry
        .resolve_loadout_candidate(&data.package().weapons[0].id, &pair)
        .unwrap();
    let mut source = registry.materialize(pair_candidate).unwrap().content;
    let rapid = support_xml(data.package().support("rapid_attacks_i").unwrap());
    source = source.replace(
        &rapid,
        &format!("<!-- preserve socket annotation -->{rapid}"),
    );
    let removal = ControlledMaceCatalog::with_loadouts(
        data.clone(),
        source,
        weapons(data.package()),
        vec![MaceSupportLoadout::default()],
    )
    .unwrap();
    let xml = removal
        .materialize(&removal.alternatives()[0].candidate)
        .unwrap()
        .content;
    assert!(xml.contains("<!-- preserve socket annotation -->"));
    assert_eq!(
        profile(&xml, data.package()).unwrap().support,
        MaceSupportLoadout::default()
    );
}
#[test]
fn every_enabled_support_contributes_to_its_color_requirement() {
    let data = custom(|data| {
        for weapon in &mut data.weapons {
            weapon.requirements.attributes = Default::default();
        }
        data.mace.requirements.attributes = Default::default();
    });
    let registry = ControlledMaceCatalog::with_tree_loadouts(
        data.clone(),
        TEMPLATE.into(),
        weapons(data.package()),
        all_loadouts(),
        class_tree::selections(data.tree()).unwrap(),
    )
    .unwrap();
    let selection = registry
        .tree_choices()
        .iter()
        .find(|selection| {
            selection.ascendancy_id.is_none()
                && selection.entrance_node_id.is_none()
                && data.tree().class(selection.class_id).unwrap().base_strength == 7
        })
        .unwrap();
    let red_red =
        MaceSupportLoadout::new(vec!["brutality_i".into(), "heavy_swing".into()]).unwrap();
    let red_green =
        MaceSupportLoadout::new(vec!["brutality_i".into(), "rapid_attacks_i".into()]).unwrap();
    let assess = |loadout| {
        registry
            .requirements(
                registry
                    .resolve_tree_loadout_candidate(
                        selection,
                        &data.package().weapons[0].id,
                        loadout,
                    )
                    .unwrap(),
            )
            .unwrap()
    };
    let red_red = assess(&red_red);
    assert_eq!(red_red.required.strength, 10);
    assert!(!red_red.is_legal());
    let red_green = assess(&red_green);
    assert_eq!(red_green.required.strength, 5);
    assert_eq!(red_green.required.dexterity, 5);
    assert!(red_green.is_legal());
}
#[test]
fn unknown_or_ineligible_supports_never_silently_disappear() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let unknown = MaceSupportLoadout::new(vec!["unreviewed".into()]).unwrap();
    assert!(
        ControlledMaceCatalog::with_loadouts(
            data.clone(),
            TEMPLATE.into(),
            weapons(data.package()),
            vec![unknown]
        )
        .is_err()
    );
    let data = custom(|package| {
        package
            .supports
            .iter_mut()
            .find(|gem| gem.id == "heavy_swing")
            .unwrap()
            .eligibility
            .exclude = package.mace.skill_types.clone();
    });
    let ineligible = MaceSupportLoadout::new(vec!["heavy_swing".into()]).unwrap();
    assert!(
        ControlledMaceCatalog::with_loadouts(
            data.clone(),
            TEMPLATE.into(),
            weapons(data.package()),
            vec![ineligible]
        )
        .is_err()
    );
}

// Binding-unit tests construct the private baseline proof so this import-only crate
// does not require a document evaluator. Native integration tests exercise the
// public fresh-baseline admission and full-template parser before using these axes.
fn native_components_for_test(registry: &ControlledMaceCatalog) -> NativeMaceComponents {
    let backend = BackendIdentity {
        data: Some(registry.data_identity().clone()),
        id: "native-poe2".into(),
        implementation_version: "test".into(),
        rules_revision: PINNED_RULES_REVISION.into(),
        source_fingerprint: "test source".into(),
        adapter_fingerprint: "test adapter".into(),
    };
    let scenario = VerifiedNativeMaceScenario {
        identity: registry.catalog.identity.clone(),
        backend: backend.clone(),
        context: EvaluationContext {
            requested: EvaluationOptions::default(),
            calculation_mode: "MAIN".into(),
            enemy_level: scalar_number(&registry.profile.config["enemyLevel"]).unwrap() as u32,
            config_inputs: registry.profile.config.clone(),
            config_placeholders: BTreeMap::new(),
            player_conditions: BTreeMap::new(),
            enemy_conditions: BTreeMap::new(),
        },
    };
    registry.native_components(&scenario, &backend).unwrap()
}
#[test]
fn native_components_share_axis_storage_and_resolve_exact_legal_candidates() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let mut alternatives = weapons(data.package());
    alternatives[0].item_text = alternatives[0]
        .item_text
        .replace("Quality: 0", "Quality: 20")
        .replace("Item Level: 1", "Item Level: 100");
    let loadouts = [
        vec![],
        vec!["brutality_i"],
        vec!["heavy_swing"],
        vec!["rapid_attacks_i"],
        vec!["brutality_i", "heavy_swing"],
        vec!["brutality_i", "rapid_attacks_i"],
        vec!["heavy_swing", "rapid_attacks_i"],
    ]
    .into_iter()
    .map(|keys| MaceSupportLoadout::new(keys.into_iter().map(String::from).collect()).unwrap())
    .collect();
    let registry = ControlledMaceCatalog::with_tree_loadouts(
        data.clone(),
        TEMPLATE.into(),
        alternatives.clone(),
        loadouts,
        class_tree::selections(data.tree()).unwrap(),
    )
    .unwrap();
    let components = native_components_for_test(&registry);
    assert_eq!(components.axis_counts(), [2, 105, 7]);
    assert_eq!(registry.alternatives.len(), 1470);
    assert!(Arc::ptr_eq(&components.axes, &registry.native_axes));
    assert!(Arc::ptr_eq(components.snapshot(), registry.snapshot()));
    assert_eq!(components.data_identity(), registry.data_identity());
    assert_eq!(components.catalog_identity(), &registry.catalog.identity);
    assert_eq!(components.character_level(), registry.profile.level);
    assert_eq!(components.config(), &registry.profile.config);
    assert_eq!(components.template_build().content, TEMPLATE);
    let cloned = components.clone();
    let mut admitted = 0;
    let mut rejected = 0;
    for alternative in registry.alternatives() {
        let legal = registry
            .requirements(&alternative.candidate)
            .unwrap()
            .is_legal();
        let handle = registry.validated_native_candidate(&alternative.candidate, &components);
        assert_eq!(handle.is_ok(), legal, "{}", alternative.id);
        if let Ok(handle) = handle {
            admitted += 1;
            assert!(handle.belongs_to(&components));
            assert!(handle.clone().belongs_to(&cloned));
            assert!(handle.belongs_to_binding(cloned.binding()));
            assert_eq!(
                components.trees()[handle.tree_index()].selection,
                alternative.tree
            );
            assert_eq!(
                &components.loadouts()[handle.loadout_index()],
                &alternative.support
            );
            let weapon = &components.weapons()[handle.weapon_index()];
            let original = alternatives
                .iter()
                .find(|weapon| weapon.id == alternative.weapon_id)
                .unwrap();
            let expected = parse_mace_item(&original.item_text, data.package()).unwrap();
            assert_eq!(weapon.weapon_key(), expected.weapon_key());
            assert_eq!(weapon.quality(), expected.quality());
            assert_eq!(weapon.item_level(), expected.item_level());
        } else {
            rejected += 1;
        }
        assert_eq!(
            hash(
                &registry
                    .materialize(&alternative.candidate)
                    .unwrap()
                    .content
            ),
            alternative.xml_sha256
        );
    }
    assert!(admitted > 0 && rejected > 0);
}
#[test]
fn native_handles_reject_foreign_bindings_and_forged_candidate_state() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = catalog(data.clone());
    let components = native_components_for_test(&registry);
    let candidate = &registry.alternatives[0].candidate;
    let handle = registry
        .validated_native_candidate(candidate, &components)
        .unwrap();
    let equal = catalog(data);
    let equal_components = native_components_for_test(&equal);
    assert_eq!(registry.catalog.identity, equal.catalog.identity);
    assert!(!handle.belongs_to(&equal_components));
    assert!(
        registry
            .validated_native_candidate(candidate, &equal_components)
            .is_err()
    );
    let changed = catalog(custom(|data| data.character.initial_life += 1.0));
    let changed_components = native_components_for_test(&changed);
    assert!(!handle.belongs_to(&changed_components));
    assert!(
        changed
            .validated_native_candidate(candidate, &changed_components)
            .is_err()
    );
    let other_template = ControlledMaceCatalog::with_data(
        registry.snapshot().clone(),
        TEMPLATE.replace(
            "Controlled attack/weapon/support",
            "Another attack/weapon/support",
        ),
        weapons(registry.snapshot().package()),
        vec![MaceSupportChoice::None],
    )
    .unwrap();
    assert!(!handle.belongs_to(&native_components_for_test(&other_template)));
    let mut forgeries = Vec::new();
    let mut value = candidate.clone();
    value.class_id = "10".into();
    forgeries.push(value);
    let mut value = candidate.clone();
    value.ascendancy_id = Some("Monk3".into());
    forgeries.push(value);
    let mut value = candidate.clone();
    value.passives.insert(24475);
    forgeries.push(value);
    let mut value = candidate.clone();
    value.equipment.insert("Weapon 1".into(), "unknown".into());
    forgeries.push(value);
    let mut value = candidate.clone();
    value
        .skills
        .get_mut(SLOT)
        .unwrap()
        .support_instance_ids
        .insert("unknown".into());
    forgeries.push(value);
    let mut value = candidate.clone();
    value.catalog.content_fingerprint.push('0');
    forgeries.push(value);
    for forged in forgeries {
        assert!(matches!(
            registry.validated_native_candidate(&forged, &components),
            Err(ControlledMutationError::UnknownCandidate)
        ));
    }
}
#[test]
fn native_components_reject_changed_verified_scenario_backend_or_data() {
    let registry = catalog(Arc::new(game_data::bundled_snapshot().unwrap()));
    let components = native_components_for_test(&registry);
    let mut scenario = VerifiedNativeMaceScenario {
        identity: components.catalog_identity().clone(),
        backend: components.backend_identity().clone(),
        context: components.context().clone(),
    };
    let backend = components.backend_identity();
    let mut other_backend = backend.clone();
    other_backend.adapter_fingerprint.push('0');
    assert!(
        registry
            .native_components(&scenario, &other_backend)
            .is_err()
    );
    other_backend = backend.clone();
    other_backend.data = Some(
        custom(|data| data.character.initial_life += 1.0)
            .identity()
            .clone(),
    );
    assert!(
        registry
            .native_components(&scenario, &other_backend)
            .is_err()
    );
    scenario.identity.content_fingerprint.push('0');
    assert!(registry.native_components(&scenario, backend).is_err());
    scenario.identity = components.catalog_identity().clone();
    scenario
        .context
        .config_inputs
        .insert("enemyArmour".into(), Scalar::Number(123456.0));
    assert!(registry.native_components(&scenario, backend).is_err());
}

fn local_rare(level: u32) -> String {
    format!(
        "Rarity: RARE\nStudy Hammer\nWooden Club\nItem Level: 100\nQuality: 20\nLevelReq: {level}\nImplicits: 0\nAdds 2 to 5 Physical Damage\n20% increased Attack Speed\n20% increased Attack Speed"
    )
}
#[test]
fn item_definition_uses_validated_base_for_rare_names_and_leading_blank_payloads() {
    let reviewed = Arc::new(game_data::bundled_snapshot().unwrap());
    let injected = custom(|data| data.weapons[0].name = "Injected Club".into());
    for data in [reviewed, injected] {
        let base = &data.package().weapons[0];
        let expected_base = base.name.clone();
        let expected_key = base.id.clone();
        let normal =
            format!("Rarity: NORMAL\n{expected_base}\nItem Level: 1\nQuality: 0\nImplicits: 0");
        let rare = format!(
            "Rarity: RARE\nDistinct Rare Name\n{expected_base}\nItem Level: 1\nQuality: 0\nImplicits: 0"
        );
        let alternatives: Vec<_> = [
            ("normal", normal.clone()),
            ("normal-leading-blanks", format!("\n\n{normal}")),
            ("rare", rare.clone()),
            ("rare-leading-blanks", format!("\r\n\r\n{rare}")),
        ]
        .into_iter()
        .map(|(id, item_text)| MaceWeaponAlternative {
            id: id.into(),
            item_text,
        })
        .collect();
        let registry = ControlledMaceCatalog::with_data(
            data,
            TEMPLATE.replace("Wooden Club", &expected_base),
            alternatives.clone(),
            vec![MaceSupportChoice::None],
        )
        .unwrap();
        let components = native_components_for_test(&registry);
        for alternative in alternatives {
            let candidate = registry
                .resolve_candidate(&alternative.id, MaceSupportChoice::None)
                .unwrap();
            let item_id = &candidate.equipment["Weapon 1"];
            let instance = &registry.catalog().items[item_id];
            assert_eq!(instance.definition_id, expected_base, "{}", alternative.id);
            assert_eq!(instance.payload.content, alternative.item_text);
            let handle = registry
                .validated_native_candidate(candidate, &components)
                .unwrap();
            let weapon = &components.weapons()[handle.weapon_index()];
            assert_eq!(weapon.base_name(), expected_base);
            assert_eq!(weapon.weapon_key(), expected_key);
        }
    }
}
#[test]
fn local_item_catalog_preserves_raw_crlf_payloads_comments_and_private_component_evidence() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let template = TEMPLATE.replace("<Items ", "<!-- equipment marker survives -->\n  <Items ");
    let rare = local_rare(1);
    let decorated = format!("\r\n\t{} \r\n", rare.replace('\n', " \r\n\t"));
    let alternatives = vec![
        MaceWeaponAlternative {
            id: "plain".into(),
            item_text: rare.clone(),
        },
        MaceWeaponAlternative {
            id: "decorated".into(),
            item_text: decorated.clone(),
        },
    ];
    let registry = ControlledMaceCatalog::with_data(
        data.clone(),
        template.clone(),
        alternatives.clone(),
        vec![MaceSupportChoice::None],
    )
    .unwrap();
    assert!(registry.uses_extended_weapon_scope());
    assert_ne!(
        registry.alternatives()[0].candidate,
        registry.alternatives()[1].candidate
    );
    let components = native_components_for_test(&registry);
    for supplied in alternatives {
        let candidate = registry
            .resolve_candidate(&supplied.id, MaceSupportChoice::None)
            .unwrap();
        let handle = registry
            .validated_native_candidate(candidate, &components)
            .unwrap();
        let typed = &components.weapons()[handle.weapon_index()];
        assert_eq!(typed.source_text(), supplied.item_text);
        let materialized = registry.materialize(candidate).unwrap().content;
        crate::xml_compat::validate_native(&materialized).unwrap();
        let document = Document::parse(&materialized).unwrap();
        let item = document
            .descendants()
            .find(|node| node.has_tag_name("Item"))
            .unwrap();
        let reimported = parse_mace_item_element(item, data.package()).unwrap();
        assert_eq!(typed.diagnostic(), reimported.diagnostic());
        assert_eq!(typed.source_text(), reimported.source_text());
        assert_eq!(
            &materialized[..item.range().start],
            &template[..registry.profile.item_range.start]
        );
        assert_eq!(
            &materialized[item.range().end..],
            &template[registry.profile.item_range.end..]
        );
        assert!(materialized.contains("<!-- equipment marker survives -->"));
        let rebuilt = ControlledMaceCatalog::with_data(
            data.clone(),
            materialized,
            vec![supplied],
            vec![MaceSupportChoice::None],
        )
        .unwrap();
        assert!(rebuilt.uses_extended_weapon_scope());
    }
}
#[test]
fn explicit_equip_level_checks_and_handles_use_authoring_independently_of_base_and_item_levels() {
    let data = custom(|data| {
        data.weapons
            .iter_mut()
            .find(|weapon| weapon.id == "wooden_club")
            .unwrap()
            .requirements
            .level = 70
    });
    let alternatives = [0, 59, 60, 61, 100]
        .map(|level| MaceWeaponAlternative {
            id: format!("level-{level}"),
            item_text: local_rare(level),
        })
        .to_vec();
    let registry = ControlledMaceCatalog::with_data(
        data,
        TEMPLATE.into(),
        alternatives,
        vec![MaceSupportChoice::None],
    )
    .unwrap();
    let components = native_components_for_test(&registry);
    for level in [0, 59, 60, 61, 100] {
        let candidate = registry
            .resolve_candidate(&format!("level-{level}"), MaceSupportChoice::None)
            .unwrap();
        let requirements = registry.requirements(candidate).unwrap();
        assert_eq!(
            requirements.required.level,
            level.max(registry.snapshot().package().mace.requirements.level)
        );
        assert_eq!(requirements.available.level, 60);
        assert_eq!(requirements.is_legal(), level <= 60);
        assert_eq!(
            registry
                .validated_native_candidate(candidate, &components)
                .is_ok(),
            level <= 60
        );
    }
}
#[test]
fn extended_item_scope_detects_template_changes_even_when_all_alternatives_are_legacy() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let legacy = catalog(data.clone());
    assert!(!legacy.uses_extended_weapon_scope());
    let mut alternatives = weapons(data.package());
    alternatives[0].item_text = local_rare(1);
    let changed = ControlledMaceCatalog::with_data(
        data.clone(),
        TEMPLATE.into(),
        alternatives,
        vec![MaceSupportChoice::None],
    )
    .unwrap();
    assert!(changed.uses_extended_weapon_scope());
    let source = changed
        .materialize(
            changed
                .resolve_candidate("wooden_club", MaceSupportChoice::None)
                .unwrap(),
        )
        .unwrap()
        .content;
    let template_only = ControlledMaceCatalog::with_data(
        data.clone(),
        source,
        weapons(data.package()),
        vec![MaceSupportChoice::None],
    )
    .unwrap();
    assert!(template_only.uses_extended_weapon_scope());
}
#[test]
fn reference_item_normalization_requires_exact_neutral_ranges_and_keeps_duplicate_lines() {
    let data = game_data::bundled_snapshot().unwrap();
    let source = local_rare(12);
    let parsed = parse_mace_item(&source, data.package()).unwrap();
    let canonical = parsed.pob_export_lines().join("\n");
    let ranges = "<ModRange id=\"1\" range=\"0.5\"/><ModRange id=\"2\" range=\"0.5\"/><ModRange id=\"3\" range=\"0.5\"/>";
    let xml = format!("<Item id=\"1\">\n\t{canonical}\n{ranges}\n</Item>");
    let check = |xml: &str| {
        let document = Document::parse(xml).unwrap();
        check_reference_item(document.root_element(), &source, data.package())
    };
    check(&xml).unwrap();
    for bad in [
        xml.replace("range=\"0.5\"", "range=\"0.6\""),
        xml.replacen("id=\"2\"", "id=\"1\"", 1),
        xml.replace("<ModRange id=\"3\" range=\"0.5\"/>", ""),
        xml.replace("</Item>", "<ModRange id=\"4\" range=\"0.5\"/></Item>"),
        xml.replace("<ModRange id=\"2\"", "<ModRange extra=\"yes\" id=\"2\""),
        xml.replace("LevelReq: 12", "LevelReq: 0"),
        xml.replacen(
            "20% increased Attack Speed",
            "21% increased Attack Speed",
            1,
        ),
        xml.replace("Study Hammer", "Another Hammer"),
        xml.replace("Rarity: RARE", "Rarity: NORMAL"),
    ] {
        assert!(check(&bad).is_err(), "accepted {bad}");
    }
    let normal = weapons(data.package()).remove(0).item_text;
    let normal_xml = format!(
        "<Item id=\"1\">{}</Item>",
        parse_mace_item(&normal, data.package())
            .unwrap()
            .pob_export_lines()
            .join("\n")
    );
    let document = Document::parse(&normal_xml).unwrap();
    check_reference_item(document.root_element(), &normal, data.package()).unwrap();
}

fn actor_template(text: &str) -> String {
    TEMPLATE.replace("</ConfigSet>", &format!("<CustomModifierBlock title=\"Study\" enabled=\"true\">{text}</CustomModifierBlock></ConfigSet>"))
}
#[test]
fn actor_modifiers_change_equipment_and_support_admission_before_private_handles() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let tree = ClassTreeSelection {
        class_id: 1,
        ascendancy_id: None,
        entrance_node_id: None,
        ascendancy_node_id: None,
    };
    for (text, strength) in [
        ("+2.49 to Strength", 9),
        ("+2.5 to Strength", 10),
        ("+3.5 to Strength", 11),
        ("-100 to Strength", 0),
        ("+20 to Strength\n50% increased Strength", 41),
    ] {
        let xml = actor_template(text);
        let registry = ControlledMaceCatalog::with_tree_loadouts(
            data.clone(),
            xml.clone(),
            weapons(data.package()),
            vec![
                MaceSupportLoadout::new(vec!["brutality_i".into(), "heavy_swing".into()]).unwrap(),
            ],
            vec![tree.clone()],
        )
        .unwrap();
        assert!(registry.uses_extended_actor_scope());
        let components = native_components_for_test(&registry);
        assert_eq!(
            components.actor_modifiers().records(),
            registry.profile.actor_modifiers.records()
        );
        for alternative in registry.alternatives() {
            let assessment = registry.requirements(&alternative.candidate).unwrap();
            assert_eq!(assessment.available.strength, strength, "{text}");
            let weapon_requirement = data
                .package()
                .weapons
                .iter()
                .find(|weapon| weapon.id == alternative.weapon_id)
                .unwrap()
                .requirements
                .attributes
                .strength;
            assert_eq!(assessment.required.strength, weapon_requirement.max(10));
            let legal = strength >= weapon_requirement.max(10);
            assert_eq!(assessment.is_legal(), legal, "{text}");
            assert_eq!(
                registry
                    .validated_native_candidate(&alternative.candidate, &components)
                    .is_ok(),
                legal
            );
            let output = registry
                .materialize(&alternative.candidate)
                .unwrap()
                .content;
            let source = Document::parse(&xml).unwrap();
            let result = Document::parse(&output).unwrap();
            let find_config = |doc: &Document<'_>| {
                doc.descendants()
                    .find(|n| n.has_tag_name("Config"))
                    .unwrap()
                    .range()
            };
            assert_eq!(&xml[find_config(&source)], &output[find_config(&result)]);
            let actor = parse_actor_configuration(
                result
                    .descendants()
                    .find(|n| n.has_tag_name("ConfigSet"))
                    .unwrap(),
                data.package(),
            )
            .unwrap();
            assert_eq!(
                actor.diagnostic(),
                registry.profile.actor_modifiers.diagnostic()
            );
        }
    }
}
#[test]
fn actor_scope_tracks_authored_empty_disabled_legacy_and_spirit_quest_configuration() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    assert!(!catalog(data.clone()).uses_extended_actor_scope());
    for element in [
        "<CustomModifierBlock/>",
        "<CustomModifierBlock enabled=\"false\">Unknown future text</CustomModifierBlock>",
        "<Input name=\"customMods\" string=\"\"/>",
    ] {
        let registry = ControlledMaceCatalog::with_data(
            data.clone(),
            TEMPLATE.replace("</ConfigSet>", &format!("{element}</ConfigSet>")),
            weapons(data.package()),
            vec![MaceSupportChoice::None],
        )
        .unwrap();
        assert!(registry.uses_extended_actor_scope());
        assert!(registry.profile.actor_modifiers.is_empty());
    }
    for quest in &data.package().actor.spirit_quests {
        let xml = TEMPLATE.replace(
            "</ConfigSet>",
            &format!(
                "<Input name=\"{}\" boolean=\"false\"/></ConfigSet>",
                quest.config_key
            ),
        );
        let registry = ControlledMaceCatalog::with_data(
            data.clone(),
            xml,
            weapons(data.package()),
            vec![MaceSupportChoice::None],
        )
        .unwrap();
        assert!(registry.uses_extended_actor_scope());
        assert_eq!(
            registry.profile.config[&quest.config_key],
            Scalar::Boolean(false)
        );
        assert!(registry.profile.actor_modifiers.is_empty());
    }
}
#[test]
fn renamed_actor_rule_data_controls_requirement_admission_without_reviewed_text_fallback() {
    let data = custom(|package| {
        let rule = package
            .actor
            .modifier_rules
            .iter_mut()
            .find(|rule| rule.id == "strength_base")
            .unwrap();
        rule.id = "custom_strength".into();
        rule.template = "{0} to configured Strength".into();
    });
    let make = |text: &str| {
        ControlledMaceCatalog::with_data(
            data.clone(),
            actor_template(text),
            weapons(data.package()),
            vec![MaceSupportChoice::None],
        )
    };
    assert!(make("+5 to Strength").is_err());
    let registry = make("+5 to configured Strength").unwrap();
    assert_eq!(
        registry.profile.actor_modifiers.lines()[0].rule_id,
        "custom_strength"
    );
    for alternative in registry.alternatives() {
        assert_eq!(
            registry
                .requirements(&alternative.candidate)
                .unwrap()
                .available
                .strength,
            20
        );
    }
}

#[test]
fn multiline_legacy_actor_inputs_keep_source_and_share_block_requirement_semantics() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let text = "\t+3 to Strength\r\n+5 to Intelligence\n";
    let legacy = TEMPLATE.replace(
        "</ConfigSet>",
        &format!("<Input name=\"customMods\" string=\"{text}\"/></ConfigSet>"),
    );
    let build = |xml: String| {
        ControlledMaceCatalog::with_data(
            data.clone(),
            xml,
            weapons(data.package()),
            vec![MaceSupportChoice::None],
        )
        .unwrap()
    };
    let legacy_catalog = build(legacy.clone());
    let block_catalog = build(actor_template(text));
    assert!(!legacy_catalog.profile.config.contains_key("customMods"));
    assert_eq!(
        legacy_catalog.profile.actor_modifiers.blocks()[0].text,
        text
    );
    let candidate = legacy_catalog
        .resolve_candidate("wooden_club", MaceSupportChoice::None)
        .unwrap();
    assert_eq!(
        legacy_catalog.requirements(candidate).unwrap(),
        assessment(&block_catalog, 0, MaceSupportChoice::None)
    );
    let materialized = legacy_catalog.materialize(candidate).unwrap();
    assert!(materialized.content.contains(&format!("string=\"{text}\"")));
    let reparsed = profile(&materialized.content, data.package()).unwrap();
    assert_eq!(
        reparsed.actor_modifiers.diagnostic(),
        legacy_catalog.profile.actor_modifiers.diagnostic()
    );
}
