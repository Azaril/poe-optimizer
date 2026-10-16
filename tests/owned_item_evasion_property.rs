//! Ordinary portable execution of the authored EquipmentUse property programs.
#[allow(dead_code)]
#[path = "support/owned_item_evasion_property.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_item_evasion_property_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};

fn authority_fixture() -> (Value, Value) {
    let d: Value = family::read("dependencies.json");
    let mut owners = d["owners"].as_array().unwrap().clone();
    for row in d["potential_writers"].as_array().unwrap() {
        assert!(row.get("application").is_none());
        let n = if let Some(n) = owners.iter().position(|o| o["owner"] == row["owner"]) {
            n
        } else {
            owners.push(json!({"owner":row["owner"],"programs":{"members":[],"closure":{"kind":"complete"}}}));
            owners.len() - 1
        };
        let programs = owners[n]["programs"]["members"].as_array_mut().unwrap();
        if let Some(p) = programs.iter_mut().find(|p| p["id"] == row["program"]) {
            if !p["effects"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["id"] == row["effect"]["id"])
            {
                p["effects"]
                    .as_array_mut()
                    .unwrap()
                    .push(row["effect"].clone());
            }
        } else {
            programs.push(json!({"id":row["program"],"effects":[row["effect"]]}));
        }
    }
    owners.sort_by_key(|o| {
        o["owner"]["value"]["value"]["key"]
            .as_str()
            .unwrap()
            .to_owned()
    });
    (
        json!({"definitions":d["definitions"]}),
        json!({"owners":owners,"effect_applications":{"members":[]},"contribution_queries":{"members":[],"closure":d["query_registry_closure"]},"receivers":{"members":[],"closure":d["receiver_registry_closure"]}}),
    )
}

#[test]
fn native_full_writer_census_rejects_unselected_zero_inactive_and_application_writers() {
    let (schema, rules) = authority_fixture();
    family::check_native(&schema, &rules, false);
    for edit in 0..3 {
        let mut changed = rules.clone();
        let mut program = json!(family::consumer().delivery[0].program);
        program["id"] = json!("synthetic.unselected-local-writer");
        if edit == 1 {
            program["effects"][0]["when"] = json!("synthetic.inactive");
            program["nodes"].as_array_mut().unwrap().push(json!({"id":"synthetic.inactive","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}));
        }
        if edit == 2 {
            changed["effect_applications"]["members"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":"synthetic.wrong-recipient","program":program}));
        } else {
            changed["owners"][0]["programs"]["members"]
                .as_array_mut()
                .unwrap()
                .push(program);
        }
        assert!(
            std::panic::catch_unwind(|| family::check_native(&schema, &changed, false)).is_err()
        );
    }
}

#[test]
fn source_bound_item_evasion_programs_are_typed_and_preserve_observations() {
    family::check();
}

#[test]
fn source_local_admission_refuses_global_conditional_and_wrong_item_records() {
    let source = "Item:21:New Item, Iron Crown";
    let mut record = serde_json::json!({"name":"ArmourAndEvasion","type":"BASE","value":13,"flags":0,"keyword_flags":0,"tags":{},"source":source});
    family::review_local_record(&record, source).unwrap();
    for tags in [
        serde_json::json!([{"type":"Global"}]),
        serde_json::json!([{"type":"Condition","var":"Never"}]),
    ] {
        record["tags"] = tags;
        record["value"] = serde_json::json!(0);
        assert!(family::review_local_record(&record, source).is_err());
    }
    record["tags"] = serde_json::json!({});
    record["source"] = serde_json::json!("Item:22:Other");
    assert!(family::review_local_record(&record, source).is_err());
}

#[test]
#[ignore = "explicit ITEM_EVASION_PRIOR/OUTPUT; whole-build coverage remains Partial"]
fn publish_item_evasion_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        std::path::PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ITEM_EVASION_PRIOR").unwrap(),
        ),
        std::path::PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ITEM_EVASION_OUTPUT").unwrap(),
        ),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "consumer.json",
            "dependencies.json",
            "source-evidence.json",
            "source-vectors.json",
        ],
        family::stage,
        serde_json::json!({"new_definitions":4,"new_owner_programs":9,"new_queries":11,"new_receivers":2,"complete_request":false}),
        [106, 117, 109, 123, 4],
    );
}
