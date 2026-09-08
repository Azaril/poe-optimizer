//! MAIN root admission is shared by document evaluation and lazy candidate admission.
use poe_optimizer_core::{candidate::*, evaluation::*, options::EvaluationOptions};
use poe_optimizer_import::controlled_build::*;
use poe_optimizer_native::{ActorScratch, NativeBackend};
use std::sync::Arc;
const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-body-armour.xml");
const SPARK: &str = include_str!("../../../tests/fixtures/builds/spark-body-armour.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 30_000 };
fn request(source: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: source.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn with_aux(source: &str, auxiliary: &str) -> String {
    source.replace(
        "</PathOfBuilding2>",
        &format!("{auxiliary}</PathOfBuilding2>"),
    )
}
fn domain(backend: &NativeBackend, source: &str) -> ControlledBuildDomain {
    ControlledBuildDomain::new(
        Arc::new(
            ControlledBuildCatalog::new(backend.data().clone(), source.into(), vec![]).unwrap(),
        ),
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: 20,
                ascendancy_passive_points: 8,
                active_skill_count: 1,
                supports_per_skill: 2,
                ..Default::default()
            },
            ..Default::default()
        },
        AttributeOptionLocks::default(),
    )
    .unwrap()
}
#[test]
fn complete_native_and_typed_candidates_keep_main_numbers_and_exact_auxiliary_bytes() {
    let backend = NativeBackend::new();
    for fixture in [MACE, SPARK] {
        let original = backend.calculate(&request(fixture), BUDGET).unwrap();
        for mode in ["UNBUFFED", "BUFFED", "COMBAT", "EFFECTIVE"] {
            let auxiliary = format!(
                r#"<Import exportParty="false" useGeneratedItemText="true" importLink="{}"/><Party destination="All" append="false" ShowAdvanceTools="false"/><TreeView searchStr="cooldown &amp; cost" zoomX="-91.25" zoomY="12.5" zoomLevel="3" showStatDifferences="true"/><Calcs><Input name="skill_number" number="14"/><Input name="misc_buffMode" string="{mode}"/><Input name="showMinion" boolean="true"/><Section id="unknown UI-only section" collapsed="true"/></Calcs>"#,
                "retained-long-source-link-".repeat(8)
            );
            let source = with_aux(fixture, &auxiliary);
            let full = backend.calculate(&request(&source), BUDGET).unwrap();
            assert_eq!(
                serde_json::to_value(&full.measurements).unwrap(),
                serde_json::to_value(&original.measurements).unwrap()
            );
            assert_eq!(
                serde_json::to_value(&full.context).unwrap(),
                serde_json::to_value(&original.context).unwrap()
            );
            assert_eq!(full.exports[0].content, source);
            let domain = domain(&backend, &source);
            let handle = domain
                .admit(
                    domain.catalog().source_selection(),
                    &mut ActorScratch::default(),
                )
                .unwrap();
            let materialized = domain.materialize(&handle).unwrap();
            assert!(materialized.content.contains(&auxiliary));
            let realized = backend
                .calculate(&request(&materialized.content), BUDGET)
                .unwrap();
            domain
                .catalog()
                .validate_native_realization(&handle, &realized, &backend.identity())
                .unwrap();
            let prepared = backend
                .prepare_controlled_build(domain.catalog(), &[])
                .unwrap();
            let measured = prepared.measure(&handle).unwrap();
            assert_eq!(
                serde_json::to_value(prepared.snapshot_measurements(&measured)).unwrap(),
                serde_json::to_value(&realized.measurements).unwrap()
            );
            let mut tampered = realized;
            tampered.exports[0].content = tampered.exports[0].content.replace(&auxiliary, "");
            assert!(
                domain
                    .catalog()
                    .validate_native_realization(&handle, &tampered, &backend.identity())
                    .is_err(),
                "auxiliary bytes are part of realization identity"
            );
        }
    }
}
#[test]
fn effectful_unknown_and_legacy_root_content_rejects_in_both_complete_paths() {
    let backend = NativeBackend::new();
    for fixture in [MACE, SPARK] {
        for auxiliary in [
            "<Import exportParty=\"true\"/>",
            "<Party><ImportedBuffs/></Party>",
            "<Party><ExportedBuffs/></Party>",
            "<Calcs><Input name=\"misc_enemyLevel\" number=\"90\"/></Calcs>",
            "<Calcs><Input name=\"unknown\" boolean=\"false\"/></Calcs>",
            "<Calcs><Input name=\"showMinion\" string=\"true\"/></Calcs>",
            "<TreeView unknown=\"true\"/>",
            "<Import/><Import/>",
            "<Unknown/>",
        ] {
            let source = with_aux(fixture, auxiliary);
            assert!(
                backend.calculate(&request(&source), BUDGET).is_err(),
                "{auxiliary}"
            );
            assert!(
                ControlledBuildCatalog::new(backend.data().clone(), source, vec![]).is_err(),
                "{auxiliary}"
            );
        }
    }
}
