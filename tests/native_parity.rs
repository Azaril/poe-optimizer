#![cfg(feature = "pob")]
//! Whole-document native evaluation compared with a fresh optional PoB oracle.
//! Six variants plus two native-export oracle reimports: eight PoB jobs total.
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricMeasurement, MetricQuery},
    options::*,
};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use std::{collections::BTreeMap, fs, path::PathBuf};

const QUESTS: [&str; 6] = [
    "questAct 1Ogham ManorCandlemass",
    "questInterlude 2Khari CrossingMolten Shrine",
    "questAct 4Eye of HinekoraSilent Hall",
    "questAct 1ClearfellBeira",
    "questAct 2Spires of DesharSisters of Garukhan Shrine",
    "questAct 3Jiquani's MachinariumBlackjaw",
];
const METRICS: [&str; 9] = [
    "life",
    "mana",
    "energy_shield",
    "fire_resistance_capped_pct",
    "cold_resistance_capped_pct",
    "lightning_resistance_capped_pct",
    "chaos_resistance_capped_pct",
    "selected_average_hit",
    "selected_hit_dps",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn budget() -> EvaluationBudget {
    EvaluationBudget { timeout_ms: 60_000 }
}
fn queries() -> Vec<MetricQuery> {
    METRICS
        .into_iter()
        .map(|id| MetricQuery {
            actor: ActorScope::Player,
            id: id.into(),
        })
        .collect()
}
fn fixture() -> String {
    fs::read_to_string(root().join("tests/fixtures/calibration/spark-mapping.xml")).unwrap()
}
fn selection(active_skill: Option<u32>) -> Option<SkillSelection> {
    Some(SkillSelection {
        socket_group: 1,
        active_skill,
        minion_skill: None,
    })
}

struct Case {
    name: &'static str,
    request: EvaluationRequest,
    quests: Option<[bool; 6]>,
    penalty: Option<f64>,
    oracle_reimport: bool,
}
fn cases() -> Vec<Case> {
    let selection_only = EvaluationOptions {
        selection: selection(Some(1)),
        encounter: None,
    };
    let override_standard = EvaluationOptions {
        selection: selection(Some(1)),
        encounter: Some(EncounterOverrides {
            name: "native/oracle standard".into(),
            enemy_level: Some(82),
            boss: Some(BossKind::Standard),
            incoming_hit: Some(DamageAmounts {
                physical: 777.0,
                fire: 21.0,
                cold: 31.0,
                lightning: 41.0,
                chaos: 51.0,
            }),
        }),
    };
    let override_pinnacle = EvaluationOptions {
        selection: selection(None),
        encounter: Some(EncounterOverrides {
            name: "native/oracle pinnacle".into(),
            enemy_level: Some(85),
            boss: Some(BossKind::Pinnacle),
            incoming_hit: None,
        }),
    };
    let override_fire = EvaluationOptions {
        selection: None,
        encounter: Some(EncounterOverrides {
            name: "native/oracle pure fire incoming".into(),
            enemy_level: Some(1),
            boss: Some(BossKind::Normal),
            incoming_hit: Some(DamageAmounts {
                physical: 0.0,
                fire: 1.25,
                cold: 0.0,
                lightning: 0.0,
                chaos: 0.0,
            }),
        }),
    };
    let specifications = [
        (
            "level1_default_quests",
            1,
            0.0,
            None,
            None,
            EvaluationOptions::default(),
            false,
        ),
        (
            "level61_fractional_resistance_all_quests_off",
            61,
            17.25,
            Some([false; 6]),
            Some(0.0),
            selection_only,
            false,
        ),
        (
            "level100_negative_resistance_mixed_quests",
            100,
            -37.5,
            Some([true, false, true, false, true, false]),
            Some(-30.0),
            override_standard,
            true,
        ),
        (
            "level61_resistance_above_cap",
            61,
            123.75,
            Some([true; 6]),
            Some(-60.0),
            override_pinnacle,
            false,
        ),
        (
            "level1_resistance_floor_quests_off",
            1,
            -200.0,
            Some([false; 6]),
            Some(-30.0),
            override_fire,
            true,
        ),
        (
            "level100_exact_resistance_cap_complementary_quests",
            100,
            90.0,
            Some([false, true, false, true, false, true]),
            Some(0.0),
            EvaluationOptions {
                selection: selection(None),
                encounter: None,
            },
            false,
        ),
    ];
    specifications
        .into_iter()
        .map(
            |(name, level, resistance, quests, penalty, options, oracle_reimport)| {
                let mut xml = fixture()
                    .replace("level=\"60\"", &format!("level=\"{level}\""))
                    .replace(
                        "name=\"enemyLightningResist\" number=\"0\"",
                        &format!("name=\"enemyLightningResist\" number=\"{resistance}\""),
                    )
                    .replace(
                        "<Notes>",
                        "<Notes>Native parity annotation: café &amp; preserved source. ",
                    );
                let mut extra = String::new();
                if let Some(quests) = quests {
                    for (key, value) in QUESTS.into_iter().zip(quests) {
                        extra.push_str(&format!("<Input name=\"{key}\" boolean=\"{value}\"/>\n"));
                    }
                }
                if let Some(penalty) = penalty {
                    extra.push_str(&format!(
                        "<Input name=\"resistancePenalty\" number=\"{penalty}\"/>\n"
                    ));
                }
                xml = xml.replace("</ConfigSet>", &format!("{extra}</ConfigSet>"));
                Case {
                    name,
                    request: EvaluationRequest {
                        build: BuildDocument {
                            format: BuildFormat::PathOfBuilding2Xml,
                            content: xml,
                        },
                        options,
                        metrics: queries(),
                    },
                    quests,
                    penalty,
                    oracle_reimport,
                }
            },
        )
        .collect()
}
fn map(result: &EvaluationResult) -> BTreeMap<&str, &MetricMeasurement> {
    result
        .measurements
        .iter()
        .map(|value| (value.query.id.as_str(), value))
        .collect()
}
fn compare(label: &str, actual: &EvaluationResult, expected: &EvaluationResult) {
    actual.validate_recorded().unwrap();
    expected.validate_recorded().unwrap();
    let actual = map(actual);
    let expected = map(expected);
    assert_eq!(actual.len(), 9);
    assert_eq!(expected.len(), 9);
    for id in METRICS {
        let left = actual[id];
        let right = expected[id];
        assert_eq!(left.unit, right.unit, "{label}: {id} unit");
        assert_eq!(
            left.schema_version, right.schema_version,
            "{label}: {id} schema"
        );
        let left = left.value.finite().unwrap();
        let right = right.value.finite().unwrap();
        let tolerance = 1e-8_f64.max(right.abs() * 1e-9);
        assert!(
            (left - right).abs() <= tolerance,
            "{label}: {id}: native {left}, oracle {right}, tolerance {tolerance}"
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
    let native = Engine::new(NativeBackend::new());
    let oracle = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        root().join("vendor/path-of-building-poe2"),
    ));
    let actual = native
        .evaluate(&case.request, budget())
        .unwrap_or_else(|error| panic!("{} native: {error}", case.name));
    let expected = oracle
        .evaluate(&case.request, budget())
        .unwrap_or_else(|error| panic!("{} PoB: {error}", case.name));
    compare(case.name, &actual, &expected);
    assert!(actual.diagnostic_only && expected.diagnostic_only);
    assert_eq!(actual.backend.id, "native-poe2");
    assert_eq!(expected.backend.id, "pob-poe2-mlua");
    assert_ne!(
        actual.backend.source_fingerprint, expected.backend.source_fingerprint,
        "the native source subset must not impersonate the full PoB manifest"
    );
    assert_eq!(
        actual.backend.rules_revision,
        expected.backend.rules_revision
    );
    assert_eq!(actual.context.requested, case.request.options);
    assert_eq!(expected.context.requested, case.request.options);
    assert_eq!(actual.context.enemy_level, expected.context.enemy_level);
    assert_eq!(actual.build.allocated_nodes, expected.build.allocated_nodes);
    for (name, value) in &actual.context.config_inputs {
        assert_eq!(
            expected.context.config_inputs.get(name),
            Some(value),
            "{} explicit/effective {name}",
            case.name
        );
    }
    for (name, enabled) in QUESTS.into_iter().zip(case.quests.unwrap_or([true; 6])) {
        assert_eq!(
            effective(&actual, name),
            Some(&Scalar::Boolean(enabled)),
            "{} native {name}",
            case.name
        );
        assert_eq!(
            effective(&expected, name),
            Some(&Scalar::Boolean(enabled)),
            "{} oracle {name}",
            case.name
        );
    }
    assert_eq!(
        effective(&actual, "resistancePenalty"),
        Some(&Scalar::Number(case.penalty.unwrap_or(-60.0)))
    );
    assert_eq!(
        effective(&actual, "resistancePenalty"),
        effective(&expected, "resistancePenalty")
    );
    let selected = actual.coverage.selected_player.as_ref().unwrap();
    assert_eq!(selected.skill_id.as_deref(), Some("SparkPlayer"));
    assert_eq!(selected.group_index, Some(1));
    assert!(!selected.synthesized_default_attack);
    // The prepared boundary caches parsing, then independently recalculates the profile.
    let prepared_backend = NativeBackend::new();
    let prepared = prepared_backend.prepare(&case.request).unwrap();
    let repeated = prepared_backend
        .evaluate_prepared(&prepared, budget())
        .unwrap();
    compare(case.name, &repeated, &expected);
    assert_eq!(
        repeated.backend.adapter_fingerprint,
        actual.backend.adapter_fingerprint
    );
    assert_eq!(actual.exports.len(), 1);
    assert_eq!(actual.exports[0].format, BuildFormat::PathOfBuilding2Xml);
    assert!(
        actual.exports[0]
            .content
            .contains("Native parity annotation: café &amp; preserved source.")
    );
    // Do not replay options: the exported document itself must retain each override.
    let reimport_request = EvaluationRequest {
        build: actual.exports[0].clone(),
        options: EvaluationOptions::default(),
        metrics: queries(),
    };
    let reimported = native.evaluate(&reimport_request, budget()).unwrap();
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
        let reimported_oracle = oracle.evaluate(&reimport_request, budget()).unwrap();
        compare(case.name, &reimported_oracle, &actual);
        assert_eq!(
            reimported_oracle.context.requested,
            EvaluationOptions::default()
        );
        for (name, value) in &actual.context.config_inputs {
            assert_eq!(
                reimported_oracle.context.config_inputs.get(name),
                Some(value),
                "{} exported {name}",
                case.name
            );
        }
    }
}

#[test]
fn native_build_metrics_and_exported_configuration_match_eight_fresh_oracle_evaluations() {
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

#[test]
fn unsupported_build_changes_fail_without_falling_back_to_the_oracle() {
    let mut request = cases().remove(0).request;
    request.build.content = request
        .build
        .content
        .replace("nodes=\"\"", "nodes=\"4739\"");
    let error = Engine::new(NativeBackend::new())
        .evaluate(&request, budget())
        .unwrap_err();
    assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
}
