//! Local copy coverage refinement; complete build and import coverage stay open.
#[path = "support/owned_amulet_copy_consumer.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::owned_schema::SchemaClosure;
use poe_optimizer_engine::owned_plan::{EffectValue, PlanGapReason};
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn amulet_copy_consumer_retires_only_its_local_contract_obligation() {
    family::check_authored();
}

#[test]
fn both_real_partial_closures_continue_to_refuse_total_evaluation() {
    let coverage: Value = family::read("coverage.json");
    for field in ["before", "after"] {
        // This finite fixture exercises refusal only. It does not substitute
        // synthetic contributors for the selected build's production coverage.
        let mut fixture = native::Fixture::new();
        fixture.complete_domain();
        let programs = fixture.family_owner_mut().programs.members.clone();
        fixture.family_owner_mut().programs.closure =
            serde_json::from_value(coverage[field].clone()).unwrap();
        assert!(matches!(
            fixture.family_owner_mut().programs.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(fixture.family_owner_mut().programs.members, programs);
        let plan = fixture.plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(!report.gaps.is_empty());
        assert_eq!(
            fixture.total(&report),
            &EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            }
        );
    }
}

#[test]
#[ignore = "requires checked AMULET_COPY_CONSUMER_PRIOR, retained source evidence and fresh OUTPUT"]
fn publish_amulet_copy_consumer_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_AMULET_COPY_CONSUMER_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_AMULET_COPY_CONSUMER_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["coverage.json", "source-vectors.json"],
        family::stage,
        json!({"retired_obligations":1,"retained_obligations":5,"new_definitions":0,"new_programs":0,"numerical_program_changes":0,"import_guard_changes":0,"finite_result_arithmetic_parity":true,"nonfinite_refused":true,"source_admission_widened":false}),
        [107, 117, 109, 123, 5],
    );
}
