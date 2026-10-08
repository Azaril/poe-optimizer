//! Replace two historical integer flags without extending their mechanic scope.
#[allow(dead_code)]
#[path = "support/owned_minion_accuracy_flags.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn accuracy_cutover_uses_current_boolean_contract_and_preserves_numerical_semantics() {
    family::check_authored();
    let rows = family::checked_source_vectors();
    let output = |name| {
        let selected: Vec<_> = rows.iter().filter(|r| r["case"] == name).collect();
        assert_eq!(selected.len(), 1);
        selected[0]["consumer"]["passes"][0]["output"]["HitChance"].clone()
    };
    assert_eq!(output("original-05"), 100);
    assert_eq!(output("block-placeholder-37"), 63);
}

#[test]
fn accuracy_cutover_rejects_changed_recipients_arithmetic_and_boolean_conditions() {
    let baseline = json!(family::programs());
    for (pointer, value) in [
        ("/0/program/reads/0/source/value/entity", json!("current")),
        ("/1/program/reads/3/source/value/entity", json!("player")),
        ("/0/program/nodes/1/expression/kind", json!("read")),
        ("/1/program/nodes/8/expression/kind", json!("add")),
        ("/1/program/effects/0/when", json!("flags")),
    ] {
        let mut changed = baseline.clone();
        *changed.pointer_mut(pointer).expect(pointer) = value;
        let result = std::panic::catch_unwind(|| {
            let rows: Vec<family::ProgramReplacement> = serde_json::from_value(changed).unwrap();
            family::check_replacements(&rows);
        });
        assert!(result.is_err(), "{pointer}");
    }
}

#[test]
#[ignore = "requires retained full source reports; no source execution"]
fn accuracy_cutover_authenticates_existing_source_vectors() {
    family::authenticate_source();
}

#[test]
#[ignore = "requires MINION_ACCURACY_FLAGS_PRIOR and fresh MINION_ACCURACY_FLAGS_OUTPUT"]
fn publish_accuracy_flags_preserving_all_five_originals() {
    let prior = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_ACCURACY_FLAGS_PRIOR")
            .expect("checked predecessor"),
    );
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_ACCURACY_FLAGS_OUTPUT")
            .expect("fresh immutable output"),
    );
    publication::run_with_expected_selected_counts(
        prior.clone(),
        output.clone(),
        &family::data(),
        &[],
        &[
            "schemas.json",
            "programs.json",
            "queries.json",
            "dependencies.json",
        ],
        family::stage,
        json!({"schema_replacements":2,"replaced_programs":2,"new_queries":2,
            "new_definitions":0,"new_programs":0,"new_producers":0,
            "closed_query_memberships":0,"full_input_inverse":true,"engine_policy_changes":0}),
        [107, 117, 109, 123, 5],
    );
    family::assert_transition(
        &release::load(&prior),
        &release::load(&output.join("package")),
    );
}
