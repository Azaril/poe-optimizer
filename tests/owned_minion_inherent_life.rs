#[path = "support/owned_minion_inherent_life.rs"]
mod family;
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
use poe_optimizer_core::owned_rules::StatReceiverTarget;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn exact_receiver_reuse_preserves_partial_owners_and_existing_law() {
    family::check_authored();
    assert_eq!(family::receivers().len(), 8);
}

#[test]
fn recipient_scope_unguarded_zero_and_changed_membership_are_rejected() {
    for edit in 0..5 {
        let mut receivers = family::receivers();
        let mut bridge = family::program();
        let mut query = family::query();
        match edit {
            0 => receivers[0].targets = vec![StatReceiverTarget::Player],
            1 => receivers.push(receivers[0].clone()),
            2 => bridge.effects[0].when = None,
            3 => {
                query.groups[1].members.members.pop().unwrap();
            }
            4 => {
                query.groups[1].members.members[1]
                    .order
                    .as_mut()
                    .unwrap()
                    .source_rank = 1
            }
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| family::check_changes(&receivers, &bridge, &query))
                .is_err()
        );
    }
}

#[test]
fn absent_strength_unexecuted_actor_and_missing_zero_record_are_not_known_zero() {
    let source = family::checked_source();
    for edit in 0..5 {
        let mut p = source["projection"].clone();
        let a = &mut p["cases"][0]["state"]["main"]["actors"][0]["life_adjustments"];
        match edit {
            0 => a["strength_insertions"][0]["inputs"]["strength"] = json!({"present":false}),
            1 => a["strength_insertions"] = json!({}),
            2 => a["strength_insertions"][0]["record"]["value"] = json!(1),
            3 => {
                a["strength_insertions"][0]["inputs"]["raw_attributes"] =
                    json!([{"name":"Str","value":0}])
            }
            4 => a["strength_insertions"][0]["store_is_player"] = json!(true),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_source_projection(&p)).is_err());
    }
}

#[test]
#[ignore = "requires retained source02 reports; executes no source VM"]
fn retained_source_authenticates_original_zero_insertion_and_exact_recipient() {
    family::check_source(true);
}

#[test]
#[ignore = "requires MINION_INHERENT_LIFE_PRIOR and fresh OUTPUT"]
fn publish_minion_inherent_life_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_INHERENT_LIFE_PRIOR")
                .expect("checked prior"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_INHERENT_LIFE_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["authoring.json", "source-vectors.json", "dependencies.json"],
        family::stage,
        json!({"new_definitions":0,"new_programs":1,"new_receivers":0,"extended_receiver_targets":8,"new_query_members":1,"closed_existing_rule_owners":0,"new_attribute_or_flag_law":false,"injected_observations":false,"final_life":false,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
