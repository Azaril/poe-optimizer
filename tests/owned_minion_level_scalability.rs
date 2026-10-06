//! One retired obligation; unchanged six production programs and import guards.
#[path = "support/owned_minion_level_scalability.rs"]
mod family;
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
fn minion_level_scalability_retires_only_the_proved_obligation() {
    family::check_authored();
}

#[test]
fn both_actual_partial_closures_still_refuse_the_existing_finite_fixture() {
    family::check_authored();
    let coverage: Value = family::read("coverage.json");
    for field in ["before", "after"] {
        // Reuse the historical numerical fixture only to exercise refusal.
        // Its finite synthetic contributors do not prove the current six-program
        // release or its unfinished copy/routing/magnitude producers complete.
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
#[ignore = "requires checked MINION_LEVEL_SCALABILITY_PRIOR and fresh OUTPUT"]
fn publish_minion_level_scalability_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LEVEL_SCALABILITY_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LEVEL_SCALABILITY_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["coverage.json", "source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"retired_obligations":1,"retained_obligations":6,"new_definitions":0,"new_programs":0,"numerical_program_changes":0,"import_guard_changes":0,"schema_identity_changed":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        },
    );
}
