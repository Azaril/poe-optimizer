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
            .resolve_candidate(&alternative.weapon_id, alternative.support)
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
                data.mace.brutality.color = color;
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
                _ => data.mace.brutality.requirements.level = required,
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
        data.mace.name = "Mace \"&<>' 雪".into();
        data.mace.skill_id = "active\"&<>'雪".into();
        data.mace.game_id = "active-game\"&<>'雪".into();
        data.mace.variant_id = "active-variant\"&<>'雪".into();
        data.mace.brutality.name = "Brutality \"&<>' 雪".into();
        data.mace.brutality.skill_id = "support\"&<>'雪".into();
        data.mace.brutality.game_id = "support-game\"&<>'雪".into();
        data.mace.brutality.variant_id = "support-variant\"&<>'雪".into();
        data.weapons[0].name = "Club \"&<>' 雪".into();
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
        if alternative.support == MaceSupportChoice::BrutalityI {
            let support = document
                .descendants()
                .filter(|node| node.has_tag_name("Gem"))
                .nth(1)
                .unwrap();
            assert_eq!(
                support.attribute("nameSpec"),
                Some(registry.snapshot().package().mace.brutality.name.as_str())
            );
            assert_eq!(
                support.attribute("skillId"),
                Some(
                    registry
                        .snapshot()
                        .package()
                        .mace
                        .brutality
                        .skill_id
                        .as_str()
                )
            );
        }
    }
}
#[test]
fn selected_quest_names_are_validated_and_lexical_normalization_rejects() {
    let data = custom(|data| data.quests.config_keys[0] = "quest\"&'雪".into());
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
