//! Closure of one named Ascendancy's intrinsic default metadata inventory.
#[path = "support/owned_ascendancy_owner_closure.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_passive_refinement_publication.rs"]
mod passive_publication;
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
fn authored_metadata_closure_preserves_seventeen_independent_owners() {
    family::check_authored();
}

#[test]
fn metadata_inventory_rejects_unknown_empty_fields_and_changed_source_relations() {
    let original: Value = family::read("source-vectors.json");
    for edit in 0..9 {
        let mut changed = original.clone();
        match edit {
            0 => changed["projections"][0]["raw"]["stats"] = json!([]),
            1 => changed["projections"][0]["constructed"]["fields"]["grantedSkills"] = json!({}),
            2 => changed["projections"][0]["raw"]["replace"] = Value::Null,
            3 => changed["projections"][0]["raw_class_id"] = json!(1),
            4 => changed["projections"][0]["constructed"]["index"] = json!(2),
            5 => changed["projections"][0]["raw"]["internalId"] = json!("Sorceress2"),
            6 => changed["projections"][0]["constructed"]["fields"]["startNodeId"] = json!(54447),
            7 => changed["root_ascendancy"]["same_root"] = json!(false),
            8 => changed["projections"][0]["evidence"]["original_constructor"] = json!(false),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_vectors(&changed, false)).is_err());
    }
}

#[test]
fn closure_cannot_hide_new_members_or_promote_class_player_and_passives() {
    let original_d: Value = family::read("dependencies.json");
    let original_c: Value = family::read("closure.json");
    for edit in 0..7 {
        let mut d = original_d.clone();
        let mut c = original_c.clone();
        match edit {
            0 => {
                c["definitions"][0]["value"]["schema"]["value"]["declarations"]["grants"]["members"] =
                    json!(["unknown"])
            }
            1 => c["owners"][0]["programs"]["members"] = json!(["unknown"]),
            2 => c["definitions"][0]["value"]["schema"]["value"]["classes"]["members"] = json!([]),
            3 => {
                c["definitions"][0]["value"]["schema"]["value"]["implicit_passives"]["members"] =
                    json!([])
            }
            4 => d["action_owners"][0]["programs"]["closure"] = json!({"kind":"complete"}),
            5 => d["action_owners"][1]["programs"]["closure"] = json!({"kind":"complete"}),
            6 => d["action_owners"][4]["programs"]["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_rows(&d, &c)).is_err());
    }
}

#[test]
#[ignore = "requires retained loaded-class/root JIT reports; no source VM executes"]
fn retained_full_source_reports_authenticate_exact_metadata_and_consumer_scope() {
    family::check_authored();
    family::assert_source_from_disk();
}

#[test]
#[ignore = "requires ASCENDANCY_OWNER_PRIOR, fresh OUTPUT and retained passed source reports"]
fn publish_ascendancy_owner_preserving_all_five_originals() {
    publication::run_with_item_parameter_completions_and_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ASCENDANCY_OWNER_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ASCENDANCY_OWNER_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 1,
            passive_refinement: true,
            extra: json!({"new_definitions":0,"new_programs":0,"closed_intrinsic_declaration_inventories":7,"intrinsic_named_ascendancy_only":true,"class_player_and_passive_coverage_preserved":true,"allocation_legality_closed":false,"external_transformations_closed":false,"numerical_resource_claim":false}),
        },
        &[],
        [106, 117, 109, 123, 4],
    );
}
