#![cfg(feature = "pob")]
//! Real MAIN calculations with auxiliary sections transplanted from the broad corpus.
use poe_optimizer_core::{evaluation::*, metrics::*, options::EvaluationOptions};
use poe_optimizer_import::build_source::{RootSectionKind, project_xml};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use std::{fs, path::PathBuf};
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
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
fn compare(label: &str, actual: &EvaluationResult, expected: &EvaluationResult) {
    actual.validate_recorded().unwrap();
    expected.validate_recorded().unwrap();
    assert_eq!(actual.measurements.len(), expected.measurements.len());
    for (a, b) in actual.measurements.iter().zip(&expected.measurements) {
        assert_eq!(a.query, b.query, "{label}");
        assert_eq!(a.unit, b.unit, "{label}");
        match (a.value.finite(), b.value.finite()) {
            (Some(x), Some(y)) => assert!(
                (x - y).abs() <= 1e-8_f64.max(y.abs() * 1e-9),
                "{label}/{}: {x} != {y}",
                a.query.id
            ),
            _ => assert_eq!(a.value, b.value, "{label}"),
        }
    }
    assert_eq!(actual.build.class_name, expected.build.class_name);
    assert_eq!(actual.build.ascendancy_name, expected.build.ascendancy_name);
    assert_eq!(actual.build.allocated_nodes, expected.build.allocated_nodes);
}
#[test]
fn corpus_auxiliary_sections_and_different_calcs_choices_preserve_main_results() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let native = Engine::new(NativeBackend::new());
    let oracle = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        root.join("vendor/path-of-building-poe2"),
    ));
    let mut variants = Vec::new();
    for id in 1..=5 {
        let xml = fs::read_to_string(root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{id:02}.xml"
        )))
        .unwrap();
        let projected = project_xml(&xml).unwrap();
        let metadata: String = projected
            .sections()
            .iter()
            .filter(|s| {
                matches!(
                    s.kind(),
                    RootSectionKind::Import
                        | RootSectionKind::Party
                        | RootSectionKind::Calcs
                        | RootSectionKind::TreeView
                )
            })
            .map(|s| s.element().source_xml())
            .collect();
        assert_eq!(
            projected
                .sections()
                .iter()
                .filter(|s| matches!(
                    s.kind(),
                    RootSectionKind::Import
                        | RootSectionKind::Party
                        | RootSectionKind::Calcs
                        | RootSectionKind::TreeView
                ))
                .count(),
            4
        );
        variants.push((format!("corpus-{id}"), metadata));
    }
    for mode in ["UNBUFFED", "BUFFED", "COMBAT", "EFFECTIVE"] {
        // MAIN has one selected group; CALCS is allowed a different saved selection.
        variants.push((format!("display-{mode}"), format!("<Import exportParty=\"false\"/><Party/><Calcs><Input name=\"skill_number\" number=\"42\"/><Input name=\"misc_buffMode\" string=\"{mode}\"/><Input name=\"showMinion\" boolean=\"true\"/></Calcs><TreeView searchStr=\"caller &amp; query\" zoomLevel=\"3\"/>")));
    }
    for name in ["spark-mapping", "mace-wooden-brutality"] {
        let template =
            fs::read_to_string(root.join(format!("tests/fixtures/calibration/{name}.xml")))
                .unwrap();
        assert!(
            !project_xml(&template)
                .unwrap()
                .sections()
                .iter()
                .any(|s| matches!(
                    s.kind(),
                    RootSectionKind::Import
                        | RootSectionKind::Party
                        | RootSectionKind::Calcs
                        | RootSectionKind::TreeView
                ))
        );
        let baseline = native.evaluate(&request(&template), BUDGET).unwrap();
        let reference = oracle.evaluate(&request(&template), BUDGET).unwrap();
        compare(name, &baseline, &reference);
        for (variant, metadata) in &variants {
            let label = format!("{name}/{variant}");
            let xml = template.replace(
                "</PathOfBuilding2>",
                &format!("{metadata}</PathOfBuilding2>"),
            );
            let result = native
                .evaluate(&request(&xml), BUDGET)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            let source = oracle
                .evaluate(&request(&xml), BUDGET)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            compare(&label, &result, &source);
            compare(&label, &result, &baseline);
            compare(&label, &source, &reference);
            assert_eq!(
                result.exports[0].content, xml,
                "{label}: native changed source"
            );
        }
    }
}
