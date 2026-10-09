//! Replace Gigantic's integer-presence plumbing without widening mechanic scope.
#[allow(dead_code)]
#[path = "support/owned_gigantic_flags.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::owned_rules::ContributionQuery;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn gigantic_cutover_preserves_exact_real_producer_recipient_and_benefits() {
    family::check_authored();
}

#[test]
fn gigantic_cutover_rejects_changed_player_routing_and_numeric_presence() {
    let baseline = json!(family::programs());
    for (pointer, value) in [
        ("/0/program/effects/0/effect/entity", json!("current")),
        ("/0/program/effects/0/effect/contribution", json!("add")),
        (
            "/0/program/nodes/0/expression/value",
            json!({"kind":"integer","value":1}),
        ),
        ("/1/program/reads/0/source/value/entity", json!("current")),
        ("/1/program/effects/0/when", json!("grants")),
    ] {
        let mut changed = baseline.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(
            std::panic::catch_unwind(|| {
                let p: Vec<family::ProgramReplacement> = serde_json::from_value(changed).unwrap();
                family::check_replacements(&p);
            })
            .is_err(),
            "{pointer}"
        );
    }
    let baseline = json!(family::queries());
    for (pointer, value) in [
        (
            "/0/groups/0/members/members/0/producer/origin/kind",
            json!("character"),
        ),
        ("/0/groups/0/members/closure", json!({"kind":"complete"})),
        ("/0/groups/0/empty/value", json!(true)),
        ("/0/groups/0/ordering", json!("ordered")),
    ] {
        let mut changed = baseline.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(
            std::panic::catch_unwind(|| {
                let q: Vec<ContributionQuery> = serde_json::from_value(changed).unwrap();
                family::check_queries(&q);
            })
            .is_err(),
            "{pointer}"
        );
    }
}

#[test]
#[ignore = "requires retained source reports; no source execution"]
fn gigantic_cutover_authenticates_existing_source_vectors() {
    family::authenticate_source();
}

#[test]
#[ignore = "requires GIGANTIC_FLAGS_PRIOR and fresh GIGANTIC_FLAGS_OUTPUT"]
fn publish_gigantic_flags_preserving_all_five_originals() {
    let prior = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_FLAGS_PRIOR").expect("checked predecessor"),
    );
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_FLAGS_OUTPUT")
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
        json!({"schema_replacements":1,"replaced_programs":2,"new_queries":1,"new_definitions":0,
            "new_programs":0,"new_producers":0,"closed_query_memberships":0,"full_input_inverse":true,"engine_policy_changes":0}),
        [107, 117, 109, 123, 5],
    );
    family::assert_transition(
        &release::load(&prior),
        &release::load(&output.join("package")),
    );
}
