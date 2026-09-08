use poe_optimizer_core::{evaluation::*, metrics::*, options::*};
use poe_optimizer_native::{EvaluationClock, NativeBackend};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
const MAPPING: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
const BOSSING: &str = include_str!("../../../tests/fixtures/calibration/spark-bossing.xml");
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
fn values(result: &EvaluationResult) -> BTreeMap<String, f64> {
    result
        .measurements
        .iter()
        .map(|m| (m.query.id.clone(), m.value.finite().unwrap()))
        .collect()
}
fn evaluate(xml: &str) -> EvaluationResult {
    Engine::new(NativeBackend::new())
        .evaluate(&request(xml), EvaluationBudget { timeout_ms: 10000 })
        .unwrap()
}

#[test]
fn complete_native_documents_match_both_unchanged_independent_goldens() {
    for (xml, golden) in [
        (
            MAPPING,
            include_str!("../../../tests/fixtures/calibration/spark-mapping.reference.json"),
        ),
        (
            BOSSING,
            include_str!("../../../tests/fixtures/calibration/spark-bossing.reference.json"),
        ),
    ] {
        let result = evaluate(xml);
        let expected: serde_json::Value = serde_json::from_str(golden).unwrap();
        assert_eq!(
            result.backend.rules_revision,
            expected["provenance"]["upstream_revision"]
        );
        let raw = BTreeMap::from([
            ("life", "Life"),
            ("mana", "Mana"),
            ("energy_shield", "EnergyShield"),
            ("fire_resistance_capped_pct", "FireResist"),
            ("cold_resistance_capped_pct", "ColdResist"),
            ("lightning_resistance_capped_pct", "LightningResist"),
            ("chaos_resistance_capped_pct", "ChaosResist"),
            ("selected_hit_dps", "TotalDPS"),
            ("selected_average_hit", "AverageHit"),
        ]);
        // Spirit and armour/evasion ratings are tested against fresh source;
        // the independent original goldens deliberately remain unchanged.
        assert_eq!(result.measurements.len(), raw.len() + 3);
        for (name, value) in values(&result)
            .into_iter()
            .filter(|(name, _)| raw.contains_key(name.as_str()))
        {
            let reference = expected["metrics"][raw[name.as_str()]].as_f64().unwrap();
            assert!(
                (value - reference).abs() <= 1e-9 * reference.abs().max(1.0),
                "{name}: {value} vs {reference}"
            );
        }
        assert!(result.diagnostic_only);
        assert_eq!(result.backend.id, "native-poe2");
        assert_eq!(result.exports[0].content, xml);
        result.validate_recorded().unwrap();
        assert!(
            !result
                .measurements
                .iter()
                .any(|m| m.query.id == "pob_total_ehp")
        );
    }
}
#[test]
fn cached_outputs_never_drive_calculation_or_survive_native_export() {
    let poisoned=MAPPING.replace("viewMode=\"CALCS\"/>","viewMode=\"CALCS\"><PlayerStat stat=\"Life\" value=\"999999999\"/><MinionStat stat=\"TotalDPS\" value=\"999999999\"/></Build>");
    let result = evaluate(&poisoned);
    assert_eq!(values(&result), values(&evaluate(MAPPING)));
    assert!(!result.exports[0].content.contains("999999999"));
    assert_eq!(
        values(&evaluate(&result.exports[0].content)),
        values(&result)
    );
    assert!(poisoned.contains("999999999"));
}
#[test]
fn unknown_mechanics_in_any_section_are_rejected_instead_of_approximated() {
    let cases = [
        MAPPING.replace("enemyLevel\" number=\"60\"", "enemyLevel\" number=\"86\""),
        MAPPING.replace("className=\"Sorceress\"", "className=\"Warrior\""),
        MAPPING.replace(
            "ascendClassName=\"None\"",
            "ascendClassName=\"Stormweaver\"",
        ),
        MAPPING.replace("nodes=\"\"", "nodes=\"4294967295\""),
        MAPPING.replace("masteryEffects=\"\"", "masteryEffects=\"1,2\""),
        MAPPING.replace("level=\"1\" quality=\"0\"", "level=\"2\" quality=\"0\""),
        MAPPING.replace(
            "<Items activeItemSet=\"1\">",
            "<Items activeItemSet=\"1\"><Item id=\"1\">Rarity: NORMAL</Item>",
        ),
        MAPPING.replace(
            "</Skill>",
            "<Gem skillId=\"SupportBrutalityPlayer\"/></Skill>",
        ),
        MAPPING.replace(
            "</ConfigSet>",
            "<Input name=\"customMods\" string=\"100% more Damage\"/></ConfigSet>",
        ),
        MAPPING.replace(
            "conditionEnemyShocked\" boolean=\"false\"",
            "conditionEnemyShocked\" boolean=\"true\"",
        ),
        MAPPING.replace("<Tree activeSpec=\"1\">", "<Tree activeSpec=\"1\"><Spec/>"),
        MAPPING.replace(
            "<PathOfBuilding2>",
            "<PathOfBuilding2 xmlns=\"unexpected\">",
        ),
        MAPPING.replace("</PathOfBuilding2>", "<Party/></PathOfBuilding2>"),
        MAPPING.replace("number=\"60\"", "number=\"NaN\""),
        MAPPING.replace(
            "<Gem nameSpec=\"Spark\"",
            "<Gem unknown=\"ignored\" nameSpec=\"Spark\"",
        ),
    ];
    for (index, xml) in cases.iter().enumerate() {
        assert!(
            NativeBackend::new().prepare(&request(xml)).is_err(),
            "case {index}"
        );
    }
    let mut req = request(MAPPING);
    req.metrics = vec![MetricQuery {
        actor: ActorScope::Player,
        id: "pob_total_ehp".into(),
    }];
    assert!(NativeBackend::new().prepare(&req).is_err());
    req.metrics.clear();
    req.options.selection = Some(SkillSelection {
        socket_group: 1,
        active_skill: Some(2),
        minion_skill: None,
    });
    assert!(NativeBackend::new().prepare(&req).is_err());
}
#[test]
fn explicit_quests_levels_and_resistance_inputs_are_recalculated() {
    let xml=MAPPING.replace("level=\"60\"","level=\"61\"").replace("</ConfigSet>","<Input name=\"questAct 1Ogham ManorCandlemass\" boolean=\"false\"/><Input name=\"resistancePenalty\" number=\"-30\"/></ConfigSet>");
    let result = evaluate(&xml);
    let numbers = values(&result);
    assert_ne!(numbers["life"], values(&evaluate(MAPPING))["life"]);
    assert_eq!(numbers["fire_resistance_capped_pct"], -20.0);
    assert_eq!(
        result.context.config_inputs["resistancePenalty"],
        Scalar::Number(-30.0)
    );
    assert!(
        !result
            .context
            .config_placeholders
            .contains_key("resistancePenalty")
    );
    let more = evaluate(&MAPPING.replace(
        "enemyLightningResist\" number=\"0\"",
        "enemyLightningResist\" number=\"200\"",
    ));
    let cap = evaluate(&MAPPING.replace(
        "enemyLightningResist\" number=\"0\"",
        "enemyLightningResist\" number=\"90\"",
    ));
    assert_eq!(values(&more), values(&cap));
}
#[test]
fn encounter_overrides_are_exported_and_prepared_inputs_remain_immutable() {
    let backend = NativeBackend::new();
    let mut req = request(MAPPING);
    req.options = EvaluationOptions {
        selection: Some(SkillSelection {
            socket_group: 1,
            active_skill: Some(1),
            minion_skill: None,
        }),
        encounter: Some(EncounterOverrides {
            name: "explicit boss".into(),
            enemy_level: Some(82),
            boss: Some(BossKind::Pinnacle),
            incoming_hit: Some(DamageAmounts {
                physical: 2000.0,
                fire: 300.0,
                ..Default::default()
            }),
        }),
    };
    let prepared = backend.prepare(&req).unwrap();
    let a = backend
        .evaluate_prepared(&prepared, EvaluationBudget { timeout_ms: 10000 })
        .unwrap();
    let b = backend
        .evaluate_prepared(&prepared, EvaluationBudget { timeout_ms: 10000 })
        .unwrap();
    assert_eq!(values(&a), values(&b));
    assert_eq!(prepared.request().build.content, MAPPING);
    assert_eq!(a.context.requested, req.options);
    assert_eq!(a.context.enemy_level, 82);
    assert_eq!(values(&evaluate(&a.exports[0].content)), values(&a));
    assert!(
        a.exports[0]
            .content
            .contains("enemyFireDamage\" number=\"300\"")
    );
}
struct AdvancingClock(AtomicU64);
impl EvaluationClock for AdvancingClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.fetch_add(10, Ordering::Relaxed))
    }
}
struct BackwardClock(AtomicU64);
impl EvaluationClock for BackwardClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.fetch_sub(1, Ordering::Relaxed))
    }
}
#[test]
fn native_deadlines_and_clock_errors_are_typed_without_spawning_workers() {
    let backend = NativeBackend::with_clock(AdvancingClock(AtomicU64::new(0)));
    assert_eq!(
        backend
            .calculate(&request(MAPPING), EvaluationBudget { timeout_ms: 1 })
            .unwrap_err()
            .kind,
        EvaluationErrorKind::Timeout
    );
    assert_eq!(
        backend
            .calculate(&request(MAPPING), EvaluationBudget { timeout_ms: 0 })
            .unwrap_err()
            .kind,
        EvaluationErrorKind::InvalidRequest
    );
    let backward = NativeBackend::with_clock(BackwardClock(AtomicU64::new(100)));
    assert_eq!(
        backward
            .calculate(&request(MAPPING), EvaluationBudget { timeout_ms: 1000 })
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
}

#[test]
fn many_cached_rows_and_large_notes_export_in_one_source_preserving_pass() {
    let notes = format!(
        "<Notes>{}</Notes>",
        "cafÃ© &amp; preserve every note byte.\r\n".repeat(100_000)
    );
    let start = MAPPING.find("<Notes>").unwrap();
    let end = MAPPING.find("</Notes>").unwrap() + "</Notes>".len();
    let large = format!("{}{}{}", &MAPPING[..start], notes, &MAPPING[end..]);
    let cached = concat!(
        "<PlayerStat stat=\"Life\" value=\"999999999\"/>",
        "<MinionStat stat=\"TotalDPS\" value=\"999999999\"/>"
    )
    .repeat(10_000);
    let xml = large.replace(
        "viewMode=\"CALCS\"/>",
        &format!("viewMode=\"CALCS\"><!-- cache boundary -->{cached}</Build>"),
    );
    let expected = large
        .replace(
            "viewMode=\"CALCS\"/>",
            "viewMode=\"CALCS\"><!-- cache boundary --></Build>",
        )
        .replace(
            "enemyIsBoss\" string=\"None\"",
            "enemyIsBoss\" string=\"Pinnacle\"",
        )
        .replace("enemyLevel\" number=\"60\"", "enemyLevel\" number=\"82\"")
        .replace(
            "enemyPhysicalDamage\" number=\"1000\"",
            "enemyPhysicalDamage\" number=\"777\"",
        )
        .replace(
            "enemyFireDamage\" number=\"0\"",
            "enemyFireDamage\" number=\"21.5\"",
        )
        .replace(
            "enemyColdDamage\" number=\"0\"",
            "enemyColdDamage\" number=\"31.25\"",
        )
        .replace(
            "enemyLightningDamage\" number=\"0\"",
            "enemyLightningDamage\" number=\"41\"",
        )
        .replace(
            "enemyChaosDamage\" number=\"0\"",
            "enemyChaosDamage\" number=\"51\"",
        );
    let mut req = request(&xml);
    req.options.encounter = Some(EncounterOverrides {
        name: "large document export".into(),
        enemy_level: Some(82),
        boss: Some(BossKind::Pinnacle),
        incoming_hit: Some(DamageAmounts {
            physical: 777.0,
            fire: 21.5,
            cold: 31.25,
            lightning: 41.0,
            chaos: 51.0,
        }),
    });
    let result = Engine::new(NativeBackend::new())
        .evaluate(&req, EvaluationBudget { timeout_ms: 10_000 })
        .unwrap();
    let exported = &result.exports[0].content;
    assert_eq!(exported.len(), expected.len());
    assert!(
        exported == &expected,
        "All untouched source bytes and every requested configuration edit must survive"
    );
    assert!(!exported.contains("<PlayerStat"));
    assert!(!exported.contains("<MinionStat"));
    assert!(!exported.contains("999999999"));
    assert!(exported.ends_with(&format!("{}{}", notes, &MAPPING[end..])));
    assert_eq!(values(&evaluate(exported)), values(&result));
}

#[test]
fn native_rejects_xml_syntax_that_the_pinned_pob_parser_interprets_differently() {
    for xml in [
        format!("\u{feff}{MAPPING}"),
        MAPPING.replace("level=\"60\"", "level = \"60\""),
        MAPPING.replace("level=\"60\"", "level=\"6&#48;\""),
        MAPPING.replace("level=\"60\"", "level=\"6&#x30;\""),
        MAPPING.replace("label=\"Single Spark hit\"", "label=\"a\tb\""),
        MAPPING.replace("label=\"Single Spark hit\"", "label=\"a\nb\""),
        MAPPING.replace("<Notes>", "<Notes>before<![CDATA[middle]]>after"),
    ] {
        let error = NativeBackend::new()
            .prepare(&request(&xml))
            .err()
            .unwrap_or_else(|| panic!("accepted incompatible XML syntax {xml}"));
        assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
    }
}
