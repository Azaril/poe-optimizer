#![cfg(feature = "pob")]
//! Complete fresh PoB calculations exercise all consumers of shared ActionSpeed.
use poe_optimizer_core::{evaluation::*, metrics::*, options::EvaluationOptions};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use serde_json::Value;
use std::path::PathBuf;

const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
const SPARK: &str = include_str!("fixtures/builds/spark-body-armour.xml");
const MACE: &str = include_str!("fixtures/builds/mace-body-armour.xml");

fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: [
            "life",
            "mana",
            "spirit",
            "energy_shield",
            "armour",
            "evasion",
            "fire_resistance_capped_pct",
            "cold_resistance_capped_pct",
            "lightning_resistance_capped_pct",
            "chaos_resistance_capped_pct",
            "movement_speed_pct",
            "action_speed_pct",
            "selected_hit_dps",
        ]
        .into_iter()
        .map(|id| MetricQuery {
            actor: ActorScope::Player,
            id: id.into(),
        })
        .collect(),
    }
}
fn attachment(result: &EvaluationResult, media: &str) -> Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type.contains(media))
            .unwrap()
            .content,
    )
    .unwrap()
}
fn near(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-8_f64.max(expected.abs() * 1e-9),
        "{label}: native {actual}, PoB {expected}"
    );
}
fn compare(label: &str, actual: &EvaluationResult, expected: &EvaluationResult) {
    actual.validate_recorded().unwrap();
    expected.validate_recorded().unwrap();
    for metric in &actual.measurements {
        let source = expected
            .measurements
            .iter()
            .find(|m| m.query == metric.query)
            .unwrap();
        assert_eq!(metric.unit, source.unit);
        assert_eq!(metric.schema_version, source.schema_version);
        match (metric.value.finite(), source.value.finite()) {
            (Some(a), Some(b)) => near(&format!("{label}/{}", metric.query.id), a, b),
            _ => assert_eq!(metric.value, source.value, "{label}/{}", metric.query.id),
        }
    }
    let info = attachment(actual, "native-profile+");
    let raw = attachment(expected, "pob-snapshot+");
    let source = &raw["player"]["metrics"];
    for (section, field, key) in [
        ("action_speed", "action_speed_mod", "ActionSpeedMod"),
        ("movement", "action_speed_mod", "ActionSpeedMod"),
        ("movement", "movement_speed_mod", "MovementSpeedMod"),
        (
            "movement",
            "effective_movement_speed_mod",
            "EffectiveMovementSpeedMod",
        ),
        ("action_timing", "cast_rate", "CastRate"),
        ("action_timing", "speed", "Speed"),
        ("action_timing", "time", "Time"),
    ] {
        let source_key = if section == "action_timing" && label.starts_with("mace/") {
            format!("MainHand.{key}")
        } else {
            key.to_owned()
        };
        near(
            &format!("{label}/{section}/{source_key}"),
            info[section][field].as_f64().unwrap(),
            source[&source_key]
                .as_f64()
                .unwrap_or_else(|| panic!("{label}/{source_key} missing")),
        );
    }
    assert_eq!(
        info["action_timing"]["non_finite_values"],
        serde_json::json!({})
    );
    assert_eq!(actual.build.allocated_nodes, expected.build.allocated_nodes);
    assert_eq!(actual.build.class_name, expected.build.class_name);
    assert_eq!(actual.build.ascendancy_name, expected.build.ascendancy_name);
}
fn block(xml: &str, text: &str) -> String {
    assert_eq!(xml.matches("</ConfigSet>").count(), 1);
    let text = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    xml.replace("</ConfigSet>", &format!("<CustomModifierBlock title=\"Timing\" enabled=\"true\">{text}</CustomModifierBlock></ConfigSet>"))
}
fn item_line(xml: &str, id: u32, text: &str) -> String {
    let start = xml.find(&format!("<Item id=\"{id}\">")).unwrap();
    let end = start + xml[start..].find("</Item>").unwrap();
    format!("{}\n{text}{}", &xml[..end], &xml[end..])
}

#[test]
fn complete_builds_match_action_floors_positive_filtering_zero_and_tick_cap() {
    let native = Engine::new(NativeBackend::new());
    let oracle = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
    ));
    for (name, template) in [("spark", SPARK), ("mace", MACE)] {
        // Original ModParser uses integer-only ordinary INC captures; never ignore this line.
        assert!(
            native
                .evaluate(
                    &request(&block(template, "17.5% increased Action Speed")),
                    BUDGET
                )
                .is_err()
        );
        for (case, text) in [
            (
                "mixed",
                "20% increased Action Speed\n40% reduced Action Speed",
            ),
            (
                "positive-rows",
                "20% increased Action Speed\n40% reduced Action Speed\nYour speed is unaffected by slows",
            ),
            ("zero", "100% reduced Action Speed"),
            (
                "floor-global",
                "200% reduced Action Speed\nAction Speed cannot be modified to below base value",
            ),
            (
                "numeric-floor",
                "20% increased Action Speed\nYour Action Speed is at least 130% of base value",
            ),
            (
                "zero-floor",
                "20% increased Action Speed\nYour Action Speed is at least 0% of base value",
            ),
            ("small", "1% increased Action Speed"),
            ("high", "10000% increased Action Speed"),
        ] {
            let label = format!("{name}/{case}");
            let xml = block(template, text);
            let result = native
                .evaluate(&request(&xml), BUDGET)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            let source = oracle
                .evaluate(&request(&xml), BUDGET)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            compare(&label, &result, &source);
            assert_eq!(result.exports[0].content, xml);
            let timing = attachment(&result, "native-profile+")["action_timing"].clone();
            if case == "zero" {
                assert_eq!(timing["speed"], 0.0);
                assert_eq!(timing["time"], 0.0);
            }
            if case == "high" {
                assert!(timing["cast_rate"].as_f64().unwrap() > timing["speed"].as_f64().unwrap());
            }
            if ["positive-rows", "high"].contains(&case) {
                let replay = native
                    .evaluate(&request(&result.exports[0].content), BUDGET)
                    .unwrap();
                let fresh = oracle
                    .evaluate(&request(&replay.exports[0].content), BUDGET)
                    .unwrap();
                compare(&format!("{label}/export-reimport"), &replay, &fresh);
                assert_eq!(
                    attachment(&result, "native-profile+"),
                    attachment(&replay, "native-profile+")
                );
            }
        }
        for retained in [true, false] {
            // Separate item and configuration producers must compose once, and removal must undo one.
            let base = block(
                template,
                "Your speed is unaffected by slows\n40% reduced Action Speed",
            );
            let xml = if retained {
                item_line(&base, 41, "20% increased Action Speed")
            } else {
                base
            };
            let label = format!("{name}/item-contribution-{retained}");
            let result = native.evaluate(&request(&xml), BUDGET).unwrap();
            let source = oracle.evaluate(&request(&xml), BUDGET).unwrap();
            compare(&label, &result, &source);
            near(
                &label,
                attachment(&result, "native-profile+")["action_speed"]["action_speed_mod"]
                    .as_f64()
                    .unwrap(),
                if retained { 1.2 } else { 1.0 },
            );
        }
    }
}
