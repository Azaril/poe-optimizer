#![cfg(feature = "pob")]
//! Fresh complete-build oracle checks for connected physical allocations/source views.
use poe_optimizer_core::{evaluation::*, options::EvaluationOptions};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use serde_json::Value;
use std::path::PathBuf;
const SPARK: &str = include_str!("fixtures/calibration/spark-mapping.xml");
const MACE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
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
fn attachment(result: &EvaluationResult, kind: &str) -> Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type.contains(kind))
            .unwrap()
            .content,
    )
    .unwrap()
}
fn near(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-8_f64.max(expected.abs() * 1e-9),
        "{label}: native {actual} vs PoB {expected}"
    );
}
fn compare(label: &str, native: &EvaluationResult, pob: &EvaluationResult) {
    native.validate_recorded().unwrap();
    pob.validate_recorded().unwrap();
    for value in &native.measurements {
        if let Some(actual) = value.value.finite() {
            let reference = pob
                .measurements
                .iter()
                .find(|m| m.query == value.query)
                .unwrap();
            near(
                &format!("{label}/{}", value.query.id),
                actual,
                reference.value.finite().unwrap(),
            );
        }
    }
    let profile = attachment(native, "native-profile+");
    let reference = attachment(pob, "pob-snapshot+");
    for (key, source) in [
        ("strength", "Str"),
        ("dexterity", "Dex"),
        ("intelligence", "Int"),
        ("life", "Life"),
        ("mana", "Mana"),
        ("spirit", "Spirit"),
        ("total_attributes", "TotalAttr"),
        ("lowest_attribute", "LowestAttribute"),
    ] {
        near(
            &format!("{label}/{key}"),
            profile["actor_resources"][key].as_f64().unwrap(),
            reference["player"]["metrics"][source].as_f64().unwrap(),
        );
    }
    assert_eq!(native.build.class_name, pob.build.class_name);
    assert_eq!(native.build.allocated_nodes, pob.build.allocated_nodes);
}
fn tree(template: &str, nodes: &str, overrides: Option<(&str, &str, &str)>) -> String {
    let xml = template.replace("nodes=\"\"", &format!("nodes=\"{nodes}\""));
    if let Some((strength, dexterity, intelligence)) = overrides {
        xml.replace("masteryEffects=\"\"/>",&format!("masteryEffects=\"\"><Overrides><AttributeOverride strNodes=\"{strength}\" dexNodes=\"{dexterity}\" intNodes=\"{intelligence}\"/></Overrides></Spec>"))
    } else {
        xml
    }
}
#[test]
fn connected_travel_attribute_options_notables_and_automatic_views_match_fresh_pob() {
    let native = Engine::new(NativeBackend::new());
    let pob = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
    ));
    let mut cases = Vec::new();
    for (name, template, nodes, attribute) in [
        ("spark", SPARK, "4739,22419", "22419"),
        ("mace", MACE, "3936,13397", "13397"),
    ] {
        for (option, overrides) in [
            ("str", (attribute, "", "")),
            ("dex", ("", attribute, "")),
            ("int", ("", "", attribute)),
        ] {
            cases.push((
                format!("{name}/{option}"),
                tree(template, nodes, Some(overrides)),
            ));
        }
    }
    cases.push((
        "sorceress/notable".into(),
        tree(SPARK, "4739,18845,1755,41965,51184", None),
    ));
    let witch = SPARK
        .replace("className=\"Sorceress\"", "className=\"Witch\"")
        .replace(
            "classId=\"7\" classInternalId=\"7\"",
            "classId=\"1\" classInternalId=\"1\"",
        );
    cases.push((
        "witch/notable-replacement".into(),
        tree(&witch, "4739,18845,1755,41965,51184", None),
    ));
    // Two distinct allocated physical nodes selecting the same effective attribute source
    // must both contribute, and neither can be replaced by that source's nonphysical ID.
    cases.push((
        "spark/two-attribute-physical-nodes".into(),
        tree(
            SPARK,
            "4739,22419,44871,56216",
            Some(("22419,56216", "", "")),
        ),
    ));
    cases.push((
        "mace/joint-passive-equipment".into(),
        include_str!("fixtures/builds/mace-passive-equipment.xml").into(),
    ));
    cases.push((
        "spark/joint-passive-equipment".into(),
        include_str!("fixtures/builds/spark-passive-equipment.xml").into(),
    ));
    let count = cases.len();
    for (label, xml) in cases {
        let actual = native
            .evaluate(&request(&xml), BUDGET)
            .unwrap_or_else(|e| panic!("{label} native: {e}"));
        let expected = pob
            .evaluate(&request(&xml), BUDGET)
            .unwrap_or_else(|e| panic!("{label} PoB: {e}"));
        compare(&label, &actual, &expected);
        assert_eq!(actual.exports[0].content, xml);
        let again = native
            .evaluate(&request(&actual.exports[0].content), BUDGET)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&actual.measurements).unwrap(),
            serde_json::to_value(&again.measurements).unwrap()
        );
        let exported = pob
            .evaluate(&request(&actual.exports[0].content), BUDGET)
            .unwrap();
        compare(&format!("{label}/export"), &actual, &exported);
    }
    eprintln!("Passive assembly: {count} fresh full-build pairs and {count} PoB export reimports");
}
#[test]
fn disconnected_unknown_or_ambiguous_attribute_allocations_reject_before_calculation() {
    let backend = NativeBackend::new();
    let valid = tree(SPARK, "4739,22419", Some(("22419", "", "")));
    for xml in [
        tree(SPARK, "22419", Some(("22419", "", ""))),
        tree(SPARK, "4739,22419", None),
        tree(SPARK, "4739", Some(("22419", "", ""))),
        tree(SPARK, "4739", Some(("4739", "", ""))),
        tree(SPARK, "4739,26297", None),
        valid.replace("dexNodes=\"\"", "dexNodes=\"22419\""),
        valid.replace("strNodes=\"22419\"", "strNodes=\"22419,22419\""),
        valid.replace(
            "</Overrides>",
            "<AttributeOverride strNodes=\"\" dexNodes=\"\" intNodes=\"\"/></Overrides>",
        ),
    ] {
        assert!(backend.prepare(&request(&xml)).is_err(), "accepted {xml}");
    }
}
