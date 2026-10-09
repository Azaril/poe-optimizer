#[path = "support/owned_intrinsic_added_attack_damage.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_plain_minion_damage.rs"]
mod plain_damage_source;
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
fn intrinsic_added_damage_retains_exact_profile_scope_and_individual_source_factor() {
    family::check_authored();
    let v = family::checked_source();
    assert_eq!(
        v["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["observed"] == true)
            .count(),
        13
    );
}
#[test]
fn source_action_programs_cannot_bypass_percentage_route_or_identity_guard() {
    let c: Value = family::read("consumer.json");
    let r: Value = family::read("routes.json");
    for edit in 0..5 {
        let mut c = c.clone();
        let mut r = r.clone();
        match edit {
            0 => {
                c["owners"][1]["programs"]["members"][0]["reads"][0]["source"]["value"]["stat"]["key"] =
                    json!("def.0000000000002537")
            }
            1 => c["owners"][1]["programs"]["members"][0]["effects"][0]["when"] = Value::Null,
            2 => {
                c["owners"][0]["programs"]["members"][0]["nodes"][4]["expression"] = json!({"kind":"literal","value":{"kind":"quantity","value":{"value":15.0,"unit":{"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"def.0000000000000002"}}}})
            }
            3 => {
                r[0]["routes"]["members"][0]["source"]["value"]["stat"]["key"] =
                    json!("def.0000000000002537")
            }
            4 => r[0]["routes"]["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_programs(&c, &r)).is_err());
    }
}
#[test]
fn evidence_rejects_fixup_presence_rounded_percent_and_ineligible_source_filters() {
    let original: Value = family::read("source-vectors.json");
    for edit in 0..5 {
        let mut v = original.clone();
        match edit {
            0 => v["full_profile"]["damageFixup"] = json!(0),
            1 => v["full_profile"]["damageFixup"] = json!(false),
            2 => v["vectors"][0]["added_more"]["records"][0]["mod"]["value"] = json!(15.0),
            3 => v["vectors"][0]["cfg"]["flags"] = json!(0),
            4 => v["vectors"][0]["added_more"]["records"][0]["mod"]["tags"] = json!([]),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_source_vectors(&v)).is_err());
    }
}
#[test]
#[ignore = "requires retained physical damage and intrinsic Life reports; executes no source VM"]
fn retained_source_reports_authenticate_raw_percentage_and_complete_profile() {
    family::check_source(true);
}
#[test]
#[ignore = "requires INTRINSIC_ADDED_ATTACK_PRIOR, fresh OUTPUT and retained reports"]
fn publish_intrinsic_added_attack_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_INTRINSIC_ADDED_ATTACK_PRIOR")
                .expect("checked prior"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_INTRINSIC_ADDED_ATTACK_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["authoring.json", "source-vectors.json", "dependencies.json"],
        family::stage,
        json!({"new_definitions":3,"new_programs":2,"new_routes":1,"new_queries":0,"whole_build_parity":false,"final_group_factor":false}),
        [106, 117, 109, 123, 4],
    );
}
