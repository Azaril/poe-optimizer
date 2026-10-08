//! Exact mechanical activation partition; no new runtime or coverage authority.
#[allow(dead_code)]
#[path = "support/owned_sniper_activation_readiness.rs"]
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
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn activation_partition_retains_exact_programs_and_empty_activation_read_sets() {
    family::check_authored();
    let rows = family::partitions();
    assert_eq!(rows[0].activation.indices.nodes, [6]);
    assert_eq!(rows[0].activation.indices.effects, [2]);
    assert_eq!(rows[0].numerical.indices.nodes, [0, 1, 2, 3, 4, 5, 7, 8, 9]);
    assert_eq!(rows[0].numerical.indices.effects, [0, 1]);
    assert_eq!(rows[1].activation.indices.nodes, [3]);
    assert_eq!(rows[1].activation.indices.effects, [0]);
    assert_eq!(rows[1].numerical.indices.nodes, [0, 1, 2]);
    assert_eq!(rows[1].numerical.indices.effects, [1, 2, 3]);
    assert!(rows.iter().all(|p| p.activation.indices.reads.is_empty()));
}

#[test]
fn activation_partition_rejects_changed_conditions_missing_dependencies_and_duplicate_effects() {
    let packet: Value = family::read("partitions.json");
    for (pointer, value) in [
        (
            "/partitions/0/activation/program/nodes/0/expression/value/value",
            json!(false),
        ),
        (
            "/partitions/1/activation/program/effects/0/when",
            json!("enabled"),
        ),
        (
            "/partitions/0/numerical/program/nodes/7/expression/factor",
            json!("one"),
        ),
        ("/partitions/1/numerical/indices/effects/0", json!(0)),
    ] {
        let mut changed = packet.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        let index = if pointer.contains("/partitions/0/") {
            0
        } else {
            1
        };
        let partition = serde_json::from_value(changed["partitions"][index].clone()).unwrap();
        assert!(
            std::panic::catch_unwind(|| family::check_partition(&partition)).is_err(),
            "{pointer}"
        );
    }
    for original in family::partitions() {
        let mut changed = original.clone();
        changed
            .activation
            .program
            .reads
            .push(original.original.reads[0].clone());
        changed.activation.indices.reads.push(0);
        assert!(
            std::panic::catch_unwind(|| family::check_partition(&changed)).is_err(),
            "unused late activation read"
        );
        let mut changed = original.clone();
        changed.numerical.program.nodes.remove(0);
        changed.numerical.indices.nodes.remove(0);
        assert!(
            std::panic::catch_unwind(|| family::check_partition(&changed)).is_err(),
            "missing transitive dependency"
        );
        let mut changed = original;
        changed
            .numerical
            .program
            .effects
            .push(changed.activation.program.effects[0].clone());
        changed
            .numerical
            .indices
            .effects
            .push(changed.activation.indices.effects[0]);
        assert!(
            std::panic::catch_unwind(|| family::check_partition(&changed)).is_err(),
            "duplicate grant across phases"
        );
    }
}

#[test]
#[ignore = "requires checked SNIPER_ACTIVATION_PRIOR and fresh SNIPER_ACTIVATION_OUTPUT"]
fn publish_activation_readiness_preserving_all_five_originals() {
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_ACTIVATION_OUTPUT")
            .expect("fresh immutable output"),
    );
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_ACTIVATION_PRIOR")
                .expect("checked predecessor"),
        ),
        output.clone(),
        &family::data(),
        &[],
        &["partitions.json", "dependencies.json"],
        family::stage,
        json!({"partitioned_programs":2,"new_programs":2,"new_effects":0,"removed_effects":0,"full_input_inverse":true,"engine_policy_changes":0}),
        [107, 117, 109, 123, 5],
    );
    family::assert_endpoint(&release::load(&output.join("package")));
}
