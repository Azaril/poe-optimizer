#![cfg(feature = "pob")]
//! Optional whole-document Mace parity: six variants and two native-export
//! reimports, bounded to two fresh PoB worker processes at a time.
use poe_optimizer_core::{
    EvaluationSnapshot,
    evaluation::*,
    metrics::{ActorScope, MeasurementValue, MetricMeasurement, MetricQuery},
    options::*,
};
use poe_optimizer_native::{NativeBackend, NativeCalculation};
use poe_optimizer_pob::backend::PobBackend;
use std::{collections::BTreeMap, path::PathBuf};

const WOODEN: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const SMITHING: &str = include_str!("fixtures/calibration/mace-smithing.xml");
const WOODEN_BRUTALITY: &str = include_str!("fixtures/calibration/mace-wooden-brutality.xml");
const SMITHING_BRUTALITY: &str = include_str!("fixtures/calibration/mace-smithing-brutality.xml");
const QUESTS: [&str; 6] = [
    "questAct 1Ogham ManorCandlemass",
    "questInterlude 2Khari CrossingMolten Shrine",
    "questAct 4Eye of HinekoraSilent Hall",
    "questAct 1ClearfellBeira",
    "questAct 2Spires of DesharSisters of Garukhan Shrine",
    "questAct 3Jiquani's MachinariumBlackjaw",
];
const FINITE_METRICS: [&str; 8] = [
    "life",
    "mana",
    "energy_shield",
    "fire_resistance_capped_pct",
    "cold_resistance_capped_pct",
    "lightning_resistance_capped_pct",
    "chaos_resistance_capped_pct",
    "selected_hit_dps",
];
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
fn queries() -> Vec<MetricQuery> {
    FINITE_METRICS
        .into_iter()
        .chain(["selected_average_hit"])
        .map(|id| MetricQuery {
            actor: ActorScope::Player,
            id: id.into(),
        })
        .collect()
}
struct Case {
    name: &'static str,
    request: EvaluationRequest,
    oracle_reimport: bool,
}
fn cases() -> Vec<Case> {
    struct Variant {
        name: &'static str,
        xml: &'static str,
        character_level: u32,
        item_level: u32,
        quality: u32,
        enemy_level: u32,
        enemy_armour: f64,
        enemy_evasion: Option<f64>,
        fire_resistance: f64,
        quests: Option<[bool; 6]>,
        penalty: Option<f64>,
        override_level: Option<u32>,
        oracle_reimport: bool,
    }
    let variants = [
        Variant {
            name: "wooden_level1_q20_alternate_class_index",
            xml: WOODEN,
            character_level: 1,
            item_level: 100,
            quality: 20,
            enemy_level: 1,
            enemy_armour: 0.0,
            enemy_evasion: None,
            fire_resistance: 0.0,
            quests: None,
            penalty: None,
            override_level: None,
            oracle_reimport: false,
        },
        Variant {
            name: "smithing_level61_q20_fractional_armour_negative_fire",
            xml: SMITHING,
            character_level: 61,
            item_level: 70,
            quality: 20,
            enemy_level: 60,
            enemy_armour: 75.25,
            enemy_evasion: None,
            fire_resistance: -37.5,
            quests: Some([false; 6]),
            penalty: Some(0.0),
            override_level: Some(65),
            oracle_reimport: true,
        },
        Variant {
            name: "wooden_brutality_level100_fractional_evasion",
            xml: WOODEN_BRUTALITY,
            character_level: 100,
            item_level: 100,
            quality: 0,
            enemy_level: 65,
            enemy_armour: 1000.25,
            enemy_evasion: Some(591.5),
            fire_resistance: 123.75,
            quests: Some([true, false, true, false, true, false]),
            penalty: Some(-30.0),
            override_level: Some(85),
            oracle_reimport: true,
        },
        Variant {
            name: "smithing_brutality_q20_zero_evasion",
            xml: SMITHING_BRUTALITY,
            character_level: 61,
            item_level: 100,
            quality: 20,
            enemy_level: 1,
            enemy_armour: 0.0,
            enemy_evasion: Some(0.0),
            fire_resistance: 200.0,
            quests: None,
            penalty: Some(-60.0),
            override_level: None,
            oracle_reimport: false,
        },
        Variant {
            name: "smithing_level1_fire_resistance_cap",
            xml: SMITHING,
            character_level: 1,
            item_level: 1,
            quality: 0,
            enemy_level: 60,
            enemy_armour: 12.5,
            enemy_evasion: Some(1000.5),
            fire_resistance: 123.75,
            quests: Some([false, true, false, true, false, true]),
            penalty: Some(0.0),
            override_level: None,
            oracle_reimport: false,
        },
        Variant {
            name: "smithing_level100_q19_armour_evasion_bounds",
            xml: SMITHING,
            character_level: 100,
            item_level: 100,
            quality: 19,
            enemy_level: 85,
            enemy_armour: 1_000_000.0,
            enemy_evasion: Some(1_000_000.0),
            fire_resistance: -200.0,
            quests: Some([true; 6]),
            penalty: None,
            override_level: None,
            oracle_reimport: false,
        },
    ];
    variants
        .into_iter()
        .enumerate()
        .map(|(index, variant)| {
            let mut xml = variant
                .xml
                .replace(
                    "level=\"60\"",
                    &format!("level=\"{}\"", variant.character_level),
                )
                .replace(
                    "Item Level: 1",
                    &format!("Item Level: {}", variant.item_level),
                )
                .replace("Quality: 0", &format!("Quality: {}", variant.quality))
                .replace(
                    "enemyLevel\" number=\"60\"",
                    &format!("enemyLevel\" number=\"{}\"", variant.enemy_level),
                )
                .replace(
                    "enemyArmour\" number=\"0\"",
                    &format!("enemyArmour\" number=\"{}\"", variant.enemy_armour),
                )
                .replace(
                    "enemyFireResist\" number=\"0\"",
                    &format!("enemyFireResist\" number=\"{}\"", variant.fire_resistance),
                )
                .replace(
                    "<Items",
                    "<!-- source weapon annotation survives -->\n<Items",
                )
                .replace(
                    "<Notes>",
                    "<Notes>Native Mace parity: café &amp; parameter choices. ",
                );
            if index == 0 {
                // Valid internal ID6 takes precedence over this alternate supplied index.
                xml = xml.replace("classId=\"3\"", "classId=\"6\"");
            }
            let mut extra = String::new();
            if let Some(evasion) = variant.enemy_evasion {
                extra.push_str(&format!(
                    "<Input name=\"enemyEvasion\" number=\"{evasion}\"/>\n"
                ));
            }
            if let Some(quests) = variant.quests {
                for (name, value) in QUESTS.into_iter().zip(quests) {
                    extra.push_str(&format!("<Input name=\"{name}\" boolean=\"{value}\"/>\n"));
                }
            }
            if let Some(penalty) = variant.penalty {
                extra.push_str(&format!(
                    "<Input name=\"resistancePenalty\" number=\"{penalty}\"/>\n"
                ));
            }
            xml = xml.replace("</ConfigSet>", &format!("{extra}</ConfigSet>"));
            let options = EvaluationOptions {
                selection: Some(SkillSelection {
                    socket_group: 1,
                    active_skill: if index % 2 == 0 { Some(1) } else { None },
                    minion_skill: None,
                }),
                encounter: variant
                    .override_level
                    .map(|enemy_level| EncounterOverrides {
                        name: variant.name.into(),
                        enemy_level: Some(enemy_level),
                        boss: Some(BossKind::Normal),
                        incoming_hit: Some(DamageAmounts {
                            physical: 777.0,
                            fire: 21.5,
                            cold: 31.25,
                            lightning: 41.0,
                            chaos: 51.0,
                        }),
                    }),
            };
            Case {
                name: variant.name,
                request: EvaluationRequest {
                    build: BuildDocument {
                        format: BuildFormat::PathOfBuilding2Xml,
                        content: xml,
                    },
                    options,
                    metrics: queries(),
                },
                oracle_reimport: variant.oracle_reimport,
            }
        })
        .collect()
}
fn map(result: &EvaluationResult) -> BTreeMap<&str, &MetricMeasurement> {
    result
        .measurements
        .iter()
        .map(|m| (m.query.id.as_str(), m))
        .collect()
}
fn close(label: &str, id: &str, native: f64, oracle: f64) {
    let tolerance = 1e-8_f64.max(oracle.abs() * 1e-9);
    assert!(
        (native - oracle).abs() <= tolerance,
        "{label} {id}: native {native}, PoB {oracle}, tolerance {tolerance}"
    );
}
fn compare(label: &str, native: &EvaluationResult, oracle: &EvaluationResult) {
    native.validate_recorded().unwrap();
    oracle.validate_recorded().unwrap();
    let native = map(native);
    let oracle = map(oracle);
    assert_eq!(native.len(), 9);
    assert_eq!(oracle.len(), 9);
    for id in FINITE_METRICS {
        assert_eq!(native[id].unit, oracle[id].unit, "{label} {id} unit");
        assert_eq!(
            native[id].schema_version, oracle[id].schema_version,
            "{label} {id} schema"
        );
        close(
            label,
            id,
            native[id].value.finite().unwrap(),
            oracle[id].value.finite().unwrap(),
        );
    }
    for unavailable in [
        &native["selected_average_hit"].value,
        &oracle["selected_average_hit"].value,
    ] {
        assert!(
            matches!(unavailable, MeasurementValue::Unavailable { reason } if !reason.is_empty()),
            "{label}: top-level attack AverageHit must remain unavailable"
        );
    }
}
fn effective<'a>(result: &'a EvaluationResult, name: &str) -> Option<&'a Scalar> {
    result
        .context
        .config_inputs
        .get(name)
        .or_else(|| result.context.config_placeholders.get(name))
}
fn verify(case: &Case) {
    let native_backend = NativeBackend::new();
    let prepared = native_backend.prepare(&case.request).unwrap();
    let native = Engine::new(NativeBackend::new());
    let oracle = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
    ));
    let actual = native
        .evaluate(&case.request, BUDGET)
        .unwrap_or_else(|error| panic!("{} native: {error}", case.name));
    let expected = oracle
        .evaluate(&case.request, BUDGET)
        .unwrap_or_else(|error| panic!("{} PoB: {error}", case.name));
    compare(case.name, &actual, &expected);
    assert_eq!(actual.backend.id, "native-poe2");
    assert_eq!(expected.backend.id, "pob-poe2-mlua");
    assert_eq!(
        actual.backend.rules_revision,
        expected.backend.rules_revision
    );
    assert_ne!(
        actual.backend.source_fingerprint,
        expected.backend.source_fingerprint
    );
    assert!(actual.diagnostic_only && expected.diagnostic_only);
    assert_eq!(actual.context.requested, case.request.options);
    assert_eq!(expected.context.requested, case.request.options);
    assert_eq!(actual.context.enemy_level, expected.context.enemy_level);
    assert_eq!(actual.build.class_name, expected.build.class_name);
    assert_eq!(actual.build.class_name, "Warrior");
    assert_eq!(actual.build.level, expected.build.level);
    assert_eq!(actual.build.allocated_nodes, expected.build.allocated_nodes);
    assert_eq!(actual.build.allocated_nodes, vec![47175]);
    for (name, value) in &actual.context.config_inputs {
        assert_eq!(
            expected.context.config_inputs.get(name),
            Some(value),
            "{} input {name}",
            case.name
        );
    }
    for name in QUESTS.into_iter().chain(["resistancePenalty"]) {
        assert_eq!(
            effective(&actual, name),
            effective(&expected, name),
            "{} effective {name}",
            case.name
        );
    }
    let snapshot: EvaluationSnapshot =
        serde_json::from_str(&expected.attachments[0].content).unwrap();
    let NativeCalculation::Mace(output) = prepared.calculate().unwrap() else {
        panic!("wrong native profile")
    };
    for (id, value) in [
        ("Str", output.strength),
        ("Dex", output.dexterity),
        ("Int", output.intelligence),
        ("Speed", output.attack_rate),
        ("CritChance", output.crit_chance),
        ("CritMultiplier", output.crit_multiplier),
        ("HitChance", output.hit_chance),
        ("AverageDamage", output.average_damage),
    ] {
        close(case.name, id, value, snapshot.player.metrics[id]);
    }
    let diagnostic: serde_json::Value =
        serde_json::from_str(&actual.attachments[0].content).unwrap();
    let Some(Scalar::Number(oracle_evasion)) = effective(&expected, "enemyEvasion") else {
        panic!("missing resolved evasion")
    };
    close(
        case.name,
        "resolved enemy evasion",
        diagnostic["resolved_enemy_evasion"].as_f64().unwrap(),
        *oracle_evasion,
    );
    let native_gems = &actual.coverage.groups[0].gems;
    let oracle_gems = &expected.coverage.groups[0].gems;
    assert_eq!(native_gems.len(), oracle_gems.len());
    for (left, right) in native_gems.iter().zip(oracle_gems) {
        assert_eq!(left.skill_id, right.skill_id);
        assert_eq!(left.gem_game_id, right.gem_game_id);
        assert_eq!(left.level, right.level);
        assert_eq!(left.quality, right.quality);
        assert_eq!(left.is_support, right.is_support);
    }
    let repeated = native_backend.evaluate_prepared(&prepared, BUDGET).unwrap();
    compare(case.name, &repeated, &actual);
    assert_eq!(prepared.request().build.content, case.request.build.content);
    assert!(
        actual.exports[0]
            .content
            .contains("<!-- source weapon annotation survives -->")
    );
    assert!(
        actual.exports[0]
            .content
            .contains("Native Mace parity: café &amp; parameter choices.")
    );
    let reimport_request = EvaluationRequest {
        build: actual.exports[0].clone(),
        options: EvaluationOptions::default(),
        metrics: queries(),
    };
    let reimported = native.evaluate(&reimport_request, BUDGET).unwrap();
    compare(case.name, &reimported, &actual);
    assert_eq!(reimported.context.requested, EvaluationOptions::default());
    assert_eq!(
        reimported.context.config_inputs,
        actual.context.config_inputs
    );
    assert_eq!(
        reimported.context.config_placeholders,
        actual.context.config_placeholders
    );
    if case.oracle_reimport {
        let oracle_reimport = oracle.evaluate(&reimport_request, BUDGET).unwrap();
        compare(case.name, &oracle_reimport, &actual);
        assert_eq!(
            oracle_reimport.context.requested,
            EvaluationOptions::default()
        );
        for (name, value) in &actual.context.config_inputs {
            assert_eq!(
                oracle_reimport.context.config_inputs.get(name),
                Some(value),
                "{} exported {name}",
                case.name
            );
        }
    }
}
#[test]
fn native_mace_pipeline_matches_eight_fresh_pob_evaluations_with_two_workers() {
    let cases = cases();
    assert_eq!(
        cases.len() + cases.iter().filter(|case| case.oracle_reimport).count(),
        8
    );
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2)
            .map(|worker| {
                let cases = &cases;
                scope.spawn(move || {
                    for case in cases
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| index % 2 == worker)
                        .map(|(_, case)| case)
                    {
                        verify(case);
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}
