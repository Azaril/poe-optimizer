use poe_optimizer_core::{evaluation::*, metrics::MeasurementValue, options::*};
use poe_optimizer_native::{NativeBackend, NativeCalculation};
use std::collections::BTreeMap;

const WOODEN: &str = include_str!("../../../tests/fixtures/calibration/mace-wooden.xml");
const SMITHING: &str = include_str!("../../../tests/fixtures/calibration/mace-smithing.xml");
const WOODEN_BRUTALITY: &str =
    include_str!("../../../tests/fixtures/calibration/mace-wooden-brutality.xml");
const SMITHING_BRUTALITY: &str =
    include_str!("../../../tests/fixtures/calibration/mace-smithing-brutality.xml");
const SPARK: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 10_000 };

fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn evaluate(xml: &str) -> EvaluationResult {
    Engine::new(NativeBackend::new())
        .evaluate(&request(xml), BUDGET)
        .unwrap()
}
fn values(result: &EvaluationResult) -> BTreeMap<String, MeasurementValue> {
    result
        .measurements
        .iter()
        .map(|m| (m.query.id.clone(), m.value.clone()))
        .collect()
}
fn number(result: &EvaluationResult, id: &str) -> f64 {
    values(result)[id].finite().unwrap()
}
fn calculate(xml: &str) -> poe_optimizer_engine::mace::MaceOutput {
    match NativeBackend::new()
        .prepare(&request(xml))
        .unwrap()
        .calculate()
        .unwrap()
    {
        NativeCalculation::Mace(value) => value,
        NativeCalculation::Spark(_) => panic!("Mace source must select the Mace profile"),
    }
}
fn close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
        "{context}: {actual} vs {expected}"
    );
}
fn add_config(xml: &str, input: &str) -> String {
    xml.replace("</ConfigSet>", &format!("{input}</ConfigSet>"))
}

#[test]
fn four_complete_mace_documents_match_unchanged_independent_goldens() {
    let metric_names = BTreeMap::from([
        ("life", "Life"),
        ("mana", "Mana"),
        ("energy_shield", "EnergyShield"),
        ("fire_resistance_capped_pct", "FireResist"),
        ("cold_resistance_capped_pct", "ColdResist"),
        ("lightning_resistance_capped_pct", "LightningResist"),
        ("chaos_resistance_capped_pct", "ChaosResist"),
        ("selected_hit_dps", "TotalDPS"),
    ]);
    for (xml, golden, supported) in [
        (
            WOODEN,
            include_str!("../../../tests/fixtures/calibration/mace-wooden.reference.json"),
            false,
        ),
        (
            SMITHING,
            include_str!("../../../tests/fixtures/calibration/mace-smithing.reference.json"),
            false,
        ),
        (
            WOODEN_BRUTALITY,
            include_str!(
                "../../../tests/fixtures/calibration/mace-wooden-brutality.reference.json"
            ),
            true,
        ),
        (
            SMITHING_BRUTALITY,
            include_str!(
                "../../../tests/fixtures/calibration/mace-smithing-brutality.reference.json"
            ),
            true,
        ),
    ] {
        let result = evaluate(xml);
        let expected: serde_json::Value = serde_json::from_str(golden).unwrap();
        assert_eq!(result.backend.id, "native-poe2");
        assert_eq!(
            result.backend.rules_revision,
            expected["provenance"]["upstream_revision"]
        );
        assert_eq!(result.measurements.len(), 12);
        for (name, raw) in &metric_names {
            close(
                number(&result, name),
                expected["metrics"][raw].as_f64().unwrap(),
                name,
            );
        }
        assert!(matches!(
            &values(&result)["selected_average_hit"],
            MeasurementValue::Unavailable { reason } if !reason.is_empty()
        ));
        let attack = calculate(xml);
        for (actual, raw) in [
            (attack.main_hand_average_hit, "AverageHit"),
            (attack.average_damage, "AverageDamage"),
            (attack.accuracy, "Accuracy"),
            (attack.hit_chance, "HitChance"),
            (attack.physical_hit_average, "PhysicalHitAverage"),
            (attack.fire_hit_average, "FireHitAverage"),
        ] {
            close(
                actual,
                expected["attack"]["main_hand"][raw].as_f64().unwrap(),
                raw,
            );
        }
        assert_eq!(result.build.class_name, "Warrior");
        assert_eq!(result.build.ascendancy_name, "None");
        assert_eq!(result.build.allocated_nodes, vec![47175]);
        let selected = result.coverage.selected_player.as_ref().unwrap();
        assert_eq!(selected.skill_id.as_deref(), Some("Melee1HMacePlayer"));
        assert_eq!(selected.group_index, Some(1));
        let gems = &result.coverage.groups[0].gems;
        assert_eq!(gems.len(), if supported { 2 } else { 1 });
        assert_eq!(gems[0].name.as_deref(), Some("Mace Strike"));
        assert_eq!(
            gems[0].gem_game_id.as_deref(),
            Some("Metadata/Items/Gem/SkillGemPlayerDefault1HMace")
        );
        assert_eq!(gems[0].is_support, Some(false));
        if supported {
            assert_eq!(gems[1].name.as_deref(), Some("Brutality I"));
            assert_eq!(gems[1].skill_id.as_deref(), Some("SupportBrutalityPlayer"));
            assert_eq!(gems[1].is_support, Some(true));
        }
        assert_eq!(result.exports[0].content, xml);
        assert!(result.diagnostic_only);
        assert_eq!(result.coverage.unresolved_entry_count, 0);
        result.validate_recorded().unwrap();
    }
    // The preferred weapon reverses when the coordinated support choice changes.
    assert!(
        number(&evaluate(SMITHING), "selected_hit_dps")
            > number(&evaluate(WOODEN), "selected_hit_dps")
    );
    assert!(
        number(&evaluate(WOODEN_BRUTALITY), "selected_hit_dps")
            > number(&evaluate(SMITHING_BRUTALITY), "selected_hit_dps")
    );
}

#[test]
fn structural_parameter_variants_preserve_source_and_do_not_use_fixture_hashes() {
    let xml = SMITHING_BRUTALITY
        .replace("level=\"60\"", "level=\"61\"")
        .replace("Item Level: 1", "Item Level: 70")
        .replace("Quality: 0", "Quality: 20")
        .replace("classId=\"3\"", "classId=\"6\"")
        .replace("nodes=\"\"", "nodes=\"47175\"")
        .replace(
            "<Items",
            "<!-- hand-authored gear choices stay here -->\n<Items",
        )
        .replace("<Notes>", "<Notes>Keep my notes &amp; tradeoffs. ");
    let result = evaluate(&xml);
    assert_eq!(result.build.level, 61);
    assert_eq!(result.exports[0].content, xml);
    assert_eq!(
        values(&evaluate(&result.exports[0].content)),
        values(&result)
    );
    let output = calculate(&xml);
    assert_eq!(output.weapon_physical_minimum, 6.0);
    assert_eq!(output.weapon_physical_maximum, 11.0);
    assert_eq!(output.fire_hit_average, 0.0);
    assert_eq!(output.effective_enemy_evasion, 591.0);
    assert_ne!(
        number(&result, "life"),
        number(&evaluate(SMITHING_BRUTALITY), "life")
    );
    assert_eq!(
        values(&evaluate(&xml.replace("Item Level: 70", "Item Level: 100"))),
        values(&result)
    );
    assert!(
        number(&result, "selected_hit_dps")
            > number(
                &evaluate(&xml.replace("Quality: 20", "Quality: 0")),
                "selected_hit_dps"
            )
    );
}

#[test]
fn all_quest_inputs_and_penalty_keep_shared_resource_semantics() {
    let mut quest_inputs = [
        "questAct 1Ogham ManorCandlemass",
        "questInterlude 2Khari CrossingMolten Shrine",
        "questAct 4Eye of HinekoraSilent Hall",
        "questAct 1ClearfellBeira",
        "questAct 2Spires of DesharSisters of Garukhan Shrine",
        "questAct 3Jiquani's MachinariumBlackjaw",
    ]
    .map(|name| format!("<Input name=\"{name}\" boolean=\"false\"/>"))
    .to_vec();
    let data = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    quest_inputs.extend(
        data.package()
            .actor
            .spirit_quests
            .iter()
            .map(|quest| format!("<Input name=\"{}\" boolean=\"false\"/>", quest.config_key)),
    );
    let xml = add_config(
        WOODEN,
        &format!(
            "{}<Input name=\"resistancePenalty\" number=\"-30\"/>",
            quest_inputs.join("")
        ),
    );
    let result = evaluate(&xml);
    assert_eq!(number(&result, "life"), 766.0);
    assert_eq!(number(&result, "mana"), 284.0);
    // The pinned maximum-resource function applies its minimum without an override.
    assert_eq!(number(&result, "spirit"), 1.0);
    for metric in [
        "fire_resistance_capped_pct",
        "cold_resistance_capped_pct",
        "lightning_resistance_capped_pct",
    ] {
        assert_eq!(number(&result, metric), -30.0);
    }
    assert_eq!(number(&result, "chaos_resistance_capped_pct"), 0.0);
    assert!(result.context.config_placeholders.is_empty());
    let all_defaults = add_config(WOODEN, &quest_inputs.join("").replace("false", "true"));
    assert_eq!(values(&evaluate(&all_defaults)), values(&evaluate(WOODEN)));
}

#[test]
fn effective_encounter_defaults_and_explicit_evasion_survive_export_reimport() {
    let backend = NativeBackend::new();
    let mut req = request(SMITHING);
    req.options = EvaluationOptions {
        selection: Some(SkillSelection {
            socket_group: 1,
            active_skill: Some(1),
            minion_skill: None,
        }),
        encounter: Some(EncounterOverrides {
            name: "normal enemy variant".into(),
            enemy_level: Some(65),
            boss: Some(BossKind::Normal),
            incoming_hit: Some(DamageAmounts {
                physical: 2100.0,
                fire: 321.0,
                cold: 43.0,
                lightning: 4.0,
                chaos: 5.0,
            }),
        }),
    };
    let prepared = backend.prepare(&req).unwrap();
    let a = backend.evaluate_prepared(&prepared, BUDGET).unwrap();
    let b = backend.evaluate_prepared(&prepared, BUDGET).unwrap();
    assert_eq!(values(&a), values(&b));
    assert_eq!(prepared.request().build.content, SMITHING);
    assert_eq!(a.context.enemy_level, 65);
    assert_eq!(a.context.requested, req.options);
    assert_eq!(
        a.context.config_inputs["enemyFireDamage"],
        Scalar::Number(321.0)
    );
    assert_eq!(
        a.context.config_inputs["enemyChaosDamage"],
        Scalar::Number(5.0)
    );
    assert_eq!(
        calculate(&a.exports[0].content).effective_enemy_evasion,
        663.0
    );
    assert_eq!(values(&evaluate(&a.exports[0].content)), values(&a));
    for evasion in [0.0, 591.5, 1000.0] {
        let explicit = add_config(
            SMITHING,
            &format!("<Input name=\"enemyEvasion\" number=\"{evasion}\"/>"),
        );
        req.build.content = explicit.clone();
        let result = backend
            .evaluate_prepared(&backend.prepare(&req).unwrap(), BUDGET)
            .unwrap();
        assert_eq!(
            result.context.config_inputs["enemyEvasion"],
            Scalar::Number(evasion)
        );
        assert_eq!(
            calculate(&result.exports[0].content).effective_enemy_evasion,
            evasion.round()
        );
        assert_eq!(
            values(&evaluate(&result.exports[0].content)),
            values(&result)
        );
        assert_eq!(
            calculate(&explicit).hit_dps,
            calculate(&result.exports[0].content).hit_dps
        );
    }
}

#[test]
fn cached_statistics_are_discarded_but_notes_comments_and_explicit_inputs_survive() {
    let xml = WOODEN
        .replace("viewMode=\"CALCS\"/>", "viewMode=\"CALCS\"><!-- cached user view --><PlayerStat stat=\"Life\" value=\"999999999\"/><MinionStat stat=\"TotalDPS\" value=\"999999999\"/></Build>")
        .replace("<Notes>", "<!-- explain the test -->\n<Notes>Tradeoffs &amp; alternatives. ");
    let result = evaluate(&xml);
    assert_eq!(values(&result), values(&evaluate(WOODEN)));
    assert!(!result.exports[0].content.contains("999999999"));
    for preserved in [
        "<!-- cached user view -->",
        "<!-- explain the test -->",
        "Tradeoffs &amp; alternatives.",
        "Rarity: NORMAL",
        "Item Level: 1",
        "Quality: 0",
    ] {
        assert!(result.exports[0].content.contains(preserved));
    }
    assert_eq!(
        values(&evaluate(&result.exports[0].content)),
        values(&result)
    );
}

#[test]
fn unsupported_mace_mechanics_are_rejected_at_preparation() {
    let invalid = [
        (
            "wrong class identity",
            WOODEN.replace("classInternalId=\"6\"", "classInternalId=\"7\""),
        ),
        (
            "wrong class index",
            WOODEN.replace("classId=\"3\"", "classId=\"7\""),
        ),
        (
            "class and skill mismatch",
            WOODEN.replace("className=\"Warrior\"", "className=\"Sorceress\""),
        ),
        (
            "ascendancy",
            WOODEN.replace("ascendClassName=\"None\"", "ascendClassName=\"Titan\""),
        ),
        (
            "unknown passive",
            WOODEN.replace("nodes=\"\"", "nodes=\"4294967295\""),
        ),
        (
            "foreign implicit root",
            WOODEN.replace("nodes=\"\"", "nodes=\"54447\""),
        ),
        (
            "mastery",
            WOODEN.replace("masteryEffects=\"\"", "masteryEffects=\"1,2\""),
        ),
        (
            "character level",
            WOODEN.replace("level=\"60\"", "level=\"101\""),
        ),
        (
            "magic rarity",
            WOODEN.replace("Rarity: NORMAL", "Rarity: MAGIC"),
        ),
        ("unknown mace", WOODEN.replace("Wooden Club", "Slim Mace")),
        (
            "quality outside range",
            WOODEN.replace("Quality: 0", "Quality: 21"),
        ),
        (
            "negative quality",
            WOODEN.replace("Quality: 0", "Quality: -1"),
        ),
        (
            "noncanonical quality",
            WOODEN.replace("Quality: 0", "Quality: 00"),
        ),
        (
            "zero item level",
            WOODEN.replace("Item Level: 1", "Item Level: 0"),
        ),
        (
            "item level outside range",
            WOODEN.replace("Item Level: 1", "Item Level: 101"),
        ),
        (
            "global attack modifier",
            WOODEN.replace(
                "Implicits: 0",
                "Implicits: 0\nAdds 10 to 20 Physical Damage to Attacks",
            ),
        ),
        (
            "item implicit",
            WOODEN.replace("Implicits: 0", "Implicits: 1"),
        ),
        (
            "item rune",
            WOODEN.replace("Implicits: 0", "Implicits: 0\nSockets: S"),
        ),
        (
            "item attribute",
            WOODEN.replace("<Item id=\"1\">", "<Item id=\"1\" variant=\"1\">"),
        ),
        (
            "item element",
            WOODEN.replace("</Item>", "<ModRange range=\"1\"/></Item>"),
        ),
        (
            "duplicate item",
            WOODEN.replace(
                "<Items activeItemSet=\"1\">",
                "<Items activeItemSet=\"1\"><Item id=\"2\">Rarity: NORMAL</Item>",
            ),
        ),
        (
            "wrong equipped reference",
            WOODEN.replace("itemId=\"1\"", "itemId=\"2\""),
        ),
        (
            "extra equipment",
            WOODEN.replace(
                "</ItemSet>",
                "<Slot name=\"Weapon 2\" itemId=\"1\"/></ItemSet>",
            ),
        ),
        (
            "weapon set swap",
            WOODEN.replace(
                "useSecondWeaponSet=\"false\"",
                "useSecondWeaponSet=\"true\"",
            ),
        ),
        (
            "active skill level",
            WOODEN.replace("level=\"1\" quality=\"0\"", "level=\"2\" quality=\"0\""),
        ),
        (
            "active skill quality",
            WOODEN.replace("level=\"1\" quality=\"0\"", "level=\"1\" quality=\"1\""),
        ),
        (
            "skill alias",
            WOODEN.replace("Melee1HMacePlayer", "SparkPlayer"),
        ),
        (
            "skill game identity",
            WOODEN.replace(
                "Metadata/Items/Gem/SkillGemPlayerDefault1HMace",
                "Metadata/Items/Gems/SkillGemSpark",
            ),
        ),
        (
            "unknown support",
            WOODEN_BRUTALITY.replace("SupportBrutalityPlayer", "SupportMartialTempoPlayer"),
        ),
        (
            "support disabled",
            WOODEN_BRUTALITY.replace(
                "variantId=\"BrutalitySupport\" level=\"1\" quality=\"0\" enabled=\"true\"",
                "variantId=\"BrutalitySupport\" level=\"1\" quality=\"0\" enabled=\"false\"",
            ),
        ),
        (
            "extra gem",
            WOODEN_BRUTALITY.replace("</Skill>", "<Gem nameSpec=\"Spark\"/></Skill>"),
        ),
        (
            "additional active group",
            WOODEN.replace("</SkillSet>", "<Skill/></SkillSet>"),
        ),
        (
            "unknown mechanic",
            add_config(
                WOODEN,
                "<Input name=\"customMods\" string=\"100% more Damage\"/>",
            ),
        ),
        (
            "missing armour",
            WOODEN.replace("<Input name=\"enemyArmour\" number=\"0\"/>", ""),
        ),
        (
            "negative armour",
            WOODEN.replace("enemyArmour\" number=\"0\"", "enemyArmour\" number=\"-1\""),
        ),
        (
            "wrong evasion type",
            add_config(WOODEN, "<Input name=\"enemyEvasion\" string=\"591\"/>"),
        ),
        (
            "negative evasion",
            add_config(WOODEN, "<Input name=\"enemyEvasion\" number=\"-1\"/>"),
        ),
        (
            "evasion above bound",
            add_config(WOODEN, "<Input name=\"enemyEvasion\" number=\"1000001\"/>"),
        ),
        (
            "nonfinite evasion",
            add_config(WOODEN, "<Input name=\"enemyEvasion\" number=\"NaN\"/>"),
        ),
        (
            "enemy above host cap",
            WOODEN.replace("enemyLevel\" number=\"60\"", "enemyLevel\" number=\"86\""),
        ),
        (
            "enabled condition",
            WOODEN.replace(
                "conditionCritRecently\" boolean=\"false\"",
                "conditionCritRecently\" boolean=\"true\"",
            ),
        ),
    ];
    for (label, xml) in invalid {
        assert_ne!(
            xml, WOODEN,
            "test mutation must change the fixture: {label}"
        );
        let error = NativeBackend::new()
            .prepare(&request(&xml))
            .err()
            .unwrap_or_else(|| panic!("accepted unsupported {label}"));
        assert_eq!(
            error.kind,
            EvaluationErrorKind::UnsupportedCapability,
            "{label}: {error:?}"
        );
    }
    // New attack configuration fields must not silently widen the Spark profile.
    for key in ["enemyArmour", "enemyEvasion"] {
        let xml = add_config(SPARK, &format!("<Input name=\"{key}\" number=\"0\"/>"));
        assert!(NativeBackend::new().prepare(&request(&xml)).is_err());
    }
    assert!(matches!(
        NativeBackend::new()
            .prepare(&request(SPARK))
            .unwrap()
            .calculate()
            .unwrap(),
        NativeCalculation::Spark(_)
    ));
}

#[test]
fn unvalidated_boss_profiles_and_invalid_selections_cannot_be_hidden_by_overrides() {
    for boss in ["Boss", "Pinnacle"] {
        let xml = WOODEN.replace(
            "enemyIsBoss\" string=\"None\"",
            &format!("enemyIsBoss\" string=\"{boss}\""),
        );
        let mut req = request(&xml);
        req.options.encounter = Some(EncounterOverrides {
            name: "normal cannot erase unsupported source".into(),
            enemy_level: None,
            boss: Some(BossKind::Normal),
            incoming_hit: None,
        });
        assert!(NativeBackend::new().prepare(&req).is_err());
    }
    for boss in [BossKind::Standard, BossKind::Pinnacle, BossKind::Uber] {
        let mut req = request(WOODEN);
        req.options.encounter = Some(EncounterOverrides {
            name: "unsupported boss".into(),
            enemy_level: None,
            boss: Some(boss),
            incoming_hit: None,
        });
        assert!(NativeBackend::new().prepare(&req).is_err());
    }
    for selection in [
        SkillSelection {
            socket_group: 2,
            active_skill: Some(1),
            minion_skill: None,
        },
        SkillSelection {
            socket_group: 1,
            active_skill: Some(2),
            minion_skill: None,
        },
        SkillSelection {
            socket_group: 1,
            active_skill: Some(1),
            minion_skill: Some(1),
        },
    ] {
        let mut req = request(WOODEN);
        req.options.selection = Some(selection);
        assert!(NativeBackend::new().prepare(&req).is_err());
    }
}
