#[path = "support/owned_mana_radius_sources.rs"]
mod proof;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn trace() -> Value {
    proof::fixture()["observed"][0].clone()
}

#[test]
fn selected_radius_provider_boundaries_reconstruct_from_portable_evidence() {
    proof::check().unwrap();
    let review = proof::read("radius-review.json");
    assert_eq!(review["observed"]["fresh_environments"], 21);
    assert_eq!(review["observed"]["reused_environments"], 0);
    assert_eq!(review["observed"]["node_constructor_calls"], 2394);
    assert_eq!(review["observed"]["node_list_calls"], 42);
    assert_eq!(review["observed"]["allocation_occurrences"], 55);
    assert_eq!(review["observed"]["implicit_roots"], json!([8305, 54447]));
    assert_eq!(
        review["observed"]["selected_radius_construction_reconciled"],
        true
    );
    for field in [
        "native_coverage_authorized",
        "current_native_package_changed",
        "ordinary_passive_transformations_covered",
        "generated_support_construction_covered",
        "later_modifier_delivery_covered",
        "arbitrary_reuse_or_overrides_covered",
        "generic_radius_behavior_covered",
    ] {
        assert_eq!(review["scope"][field], false, "{field}");
    }
}

#[test]
fn selected_membership_refuses_occupied_unknown_duplicate_and_cross_preset_sources() {
    for mutation in 0..8 {
        let mut input = proof::selected().clone();
        match mutation {
            0 => input["socket_equipment"]["members"] = json!([{"lineage":"x","local":"1"}]),
            1 => {
                input["socket_equipment"]["completion"] =
                    json!({"kind":"pending","issue":"unknown"})
            }
            2 => input["socket_source"]["specs"][2]["source_direct_socket_rows"] = json!(1),
            3 => input["socket_source"]["specs"][2]["jewels"] = json!([{"item":16,"node":7960}]),
            4 => input["passives"][0]["allocation_preset"]["local"] = json!("00000000000001b7"),
            5 => {
                input["passives"][0]["source_token"] = input["passives"][1]["source_token"].clone()
            }
            6 => {
                input["allocation_members"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            7 => {
                input["passives"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["binding"]["kind"] == "implicit_root")
                    .unwrap()["binding"]["kind"] = json!("allocation")
            }
            _ => unreachable!(),
        }
        assert!(
            proof::review_membership(&input).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn body_review_requires_complete_original_source_and_all_references() {
    for mutation in 0..5 {
        let mut bodies = proof::read("radius-bodies.json");
        match mutation {
            0 => bodies["source"]["path"] = json!("invented.lua"),
            1 => bodies["spans"][3]["last"] = json!(462),
            2 => {
                bodies["radius_references"].as_array_mut().unwrap().pop();
            }
            3 => {
                bodies["spans"][6]["body"] = json!("return {}\n");
                bodies["spans"][6]["sha256"] =
                    json!(format!("{:x}", Sha256::digest(b"return {}\n")));
            }
            4 => bodies["scope"]["generic_radius_behavior_covered"] = json!(true),
            _ => unreachable!(),
        }
        assert!(
            proof::review_bodies(&bodies).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn fresh_invocation_authority_refuses_injected_overrides_reuse_and_acceleration() {
    for mutation in 0..7 {
        let mut t = trace();
        let call = &mut t["init_calls"][0];
        match mutation {
            0 => {
                call["override"] = json!({"present":true,"fields":[{"key":"extraJewelFuncs","key_type":"string","value_type":"table","value":{"kind":"table","entries":[]}}]})
            }
            1 => call["override"] = json!({"present":true,"fields":[{"key":"spec","value":{}}]}),
            2 => call["spec_env"]["env_id"] = json!(1),
            3 => {
                call["spec_env"]["accelerate"] = json!({"kind":"table","entries":[{"key":"everything","value":{"kind":"boolean","value":true}}]})
            }
            4 => call["parent_invocation_id"] = json!(1),
            5 => {
                call["item_pass"]["accelerate"] = json!({"kind":"table","entries":[{"key":"requirementsItems","value":{"kind":"boolean","value":true}}]})
            }
            6 => call["selected_spec"]["jewels"]["entries"][0]["value"]["value"] = json!(16),
            _ => unreachable!(),
        }
        assert!(proof::review_trace(&t).is_err(), "mutation {mutation}");
    }
}

#[test]
fn every_ordered_slot_and_actual_candidate_has_exact_provider_authority() {
    for mutation in 0..9 {
        let mut t = trace();
        let slots = &mut t["init_calls"][0]["entry_slots"];
        match mutation {
            0 => {
                slots["ordered"].as_array_mut().unwrap().pop();
            }
            1 => {
                let duplicate = slots["slot_map"][0].clone();
                slots["slot_map"][1] = duplicate;
            }
            2 => slots["item_zero"] = json!({"present":true,"fields":{"id":0}}),
            3 => {
                slots["ordered"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|s| s["is_ui_socket"] == true)
                    .unwrap()["selected_item"] = json!({"present":true,"fields":{"id":16}})
            }
            4 => {
                slots["slot_map"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|s| s["node_id"]["kind"] == "number")
                    .unwrap()["node_id"]["value"] = json!(99999)
            }
            5 => t["slot_candidates"][0]["resolved_item"]["fields"]["jewelRadiusIndex"] = json!(1),
            6 => t["slot_candidates"][1] = t["slot_candidates"][0].clone(),
            7 => t["slot_candidates"][0]["invocation_id"] = json!(2),
            8 => t["slot_candidates"][0]["allocated"] = json!(true),
            _ => unreachable!(),
        }
        assert!(proof::review_trace(&t).is_err(), "mutation {mutation}");
    }
}

#[test]
fn node_list_consumers_preserve_occurrences_parents_and_empty_input_state() {
    for mutation in 0..8 {
        let mut t = trace();
        match mutation {
            0 => {
                t["node_calls"].as_array_mut().unwrap().pop();
            }
            1 => t["node_calls"][1]["node"] = t["node_calls"][0]["node"].clone(),
            2 => t["node_calls"][0]["list_id"] = json!(2),
            3 => t["node_calls"][0]["env_id"] = json!(2),
            4 => t["list_calls"][0]["before_radius"]["radius_jewels"] = json!([{"item":16}]),
            5 => t["node_calls"][0]["after_radius"]["extra_radius_nodes"] = json!([{"node":99999}]),
            6 => t["list_calls"][0]["parent_list_id"] = json!(1),
            7 => t["init_calls"][0]["exit_radius"]["radius_list_present"] = json!(false),
            _ => unreachable!(),
        }
        assert!(proof::review_trace(&t).is_err(), "mutation {mutation}");
    }
}

#[test]
fn supplied_report_pins_are_checked_even_after_authenticated_fixture_is_cached() {
    let source = proof::fixture();
    let bodies = proof::read("radius-bodies.json");
    proof::reconstruct(source, &bodies, proof::selected()).unwrap();
    for mutation in 0..4 {
        let mut changed = source.clone();
        match mutation {
            0 => changed["driver_sha256"] = json!("changed"),
            1 => changed["manifest_sha256"] = json!("changed"),
            2 => changed["native_coverage_authorized"] = json!(true),
            3 => {
                changed["files"][0]["sha256"] =
                    json!("coherent metadata is not independent authority")
            }
            _ => unreachable!(),
        }
        assert!(
            proof::reconstruct(&changed, &bodies, proof::selected()).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
#[ignore = "author portable evidence from the already accepted fixed four-load acquisition"]
fn retain_selected_radius_source_review() {
    proof::retain().unwrap();
}
