#[allow(dead_code)]
#[path = "support/owned_self_flat_flask_keystone.rs"]
mod proof;
use serde_json::{Value, json};
use std::sync::{Mutex, MutexGuard};
fn lane() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn refuses(result: Result<Value, String>, needle: &str) {
    let error = result.expect_err("changed source evidence must refuse");
    assert!(error.contains(needle), "{error} did not contain {needle}");
}
fn primary(trace: &mut Value) -> &mut Value {
    trace["deliveries"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["line"] == 1854 && d["recipient"]["profile"] == "RaisedSkeletonSniper")
        .unwrap()
}
#[test]
fn complete_selected_flask_charm_inputs_and_primary_secondary_keystone_paths_are_checked() {
    let _lane = lane();
    proof::check().unwrap();
    let v = proof::read("review.json");
    assert_eq!(v["flasks_and_charms"]["fresh_environments"], 21);
    assert_eq!(v["flasks_and_charms"]["actual_candidates"], 2352);
    assert_eq!(v["flasks_and_charms"]["selected_item_records"], 8);
    assert_eq!(v["flasks_and_charms"]["selected_item_uses"], 9);
    assert_eq!(v["keystones"]["primary"].as_array().unwrap().len(), 2);
    assert_eq!(
        v["keystones"]["primary_query_result_deduced_not_observed"],
        true
    );
    assert_eq!(v["keystones"]["secondary_query_executed"], false);
    for key in [
        "new_source_execution",
        "all_temporal_minion_writers_accounted",
        "native_coverage_authorized",
        "current_native_package_changed",
        "global_flags_assumed_false",
        "cross_acquisition_runtime_ids_joined",
    ] {
        assert_eq!(v["scope"][key], false);
    }
}
#[test]
fn flask_or_charm_in_any_slot_is_a_provider_even_when_inactive() {
    let _lane = lane();
    let base = &proof::radius_fixture()["observed"][0];
    for kind in ["Flask", "Charm"] {
        for actual in [false, true] {
            let mut trace = base.clone();
            let item = if actual {
                &mut trace["slot_candidates"][0]["resolved_item"]
            } else {
                &mut trace["init_calls"][0]["entry_slots"]["ordered"][0]["selected_item"]
            };
            assert_eq!(item["present"], true);
            item["fields"]["type"] = json!(kind);
            item["base_fields"]["type"] = json!(kind);
            refuses(
                proof::review_flasks(&trace, proof::selected()),
                "Flask/Charm provider",
            );
        }
    }
}
#[test]
fn absent_unknown_and_inconsistent_empty_slot_states_are_distinct() {
    let _lane = lane();
    for case in 0..5 {
        let mut trace = proof::radius_fixture()["observed"][0].clone();
        let expected = match case {
            0 => {
                trace["slot_candidates"][0]["resolved_item"]["fields"]
                    .as_object_mut()
                    .unwrap()
                    .remove("type");
                "item type"
            }
            1 => {
                trace["init_calls"][0]["entry_slots"]["item_zero"] =
                    json!({"present":false,"unknown":true});
                "items[0]"
            }
            _ => {
                let slot = trace["init_calls"][0]["entry_slots"]["ordered"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|s| s["fields"]["slotName"] == "Flask 1")
                    .unwrap();
                match case {
                    2 => slot["fields"]["selItemId"] = json!(99),
                    3 => slot["fields"]["active"] = json!(true),
                    4 => slot["selected_item"] = Value::Null,
                    _ => unreachable!(),
                }
                if case == 4 {
                    "presence is unavailable"
                } else {
                    "exact empty"
                }
            }
        };
        refuses(proof::review_flasks(&trace, proof::selected()), expected);
    }
}
#[test]
fn override_reuse_and_incomplete_candidate_census_cannot_inherit_selected_absence() {
    let _lane = lane();
    for case in 0..5 {
        let mut trace = proof::radius_fixture()["observed"][0].clone();
        match case {
            0 => {
                trace["init_calls"][0]["override"] = json!({"present":true,"fields":[{"key":"toggleFlask","key_type":"string","value_type":"table","value":{"kind":"table"}}]})
            }
            1 => trace["init_calls"][0]["spec_env"]["present"] = json!(true),
            2 => {
                trace["init_calls"][0]["spec_env"]["accelerate"] =
                    json!({"kind":"boolean","value":true})
            }
            3 => {
                trace["slot_candidates"].as_array_mut().unwrap().remove(111);
            }
            4 => {
                let extra = trace["slot_candidates"][0].clone();
                trace["slot_candidates"].as_array_mut().unwrap().push(extra);
            }
            _ => unreachable!(),
        }
        assert!(
            proof::review_flasks(&trace, proof::selected()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn exact_selected_itemset_use_and_recipient_ownership_cannot_be_relabelled() {
    let _lane = lane();
    for case in 0..4 {
        let mut selected = proof::selected().clone();
        match case {
            0 => {
                selected["memberships"]["equipment"][0]["source_use"]["parent"]["ordinal"] =
                    json!(468)
            }
            1 => selected["memberships"]["equipment"][0]["source_item_id"] = json!("999"),
            2 => {
                selected["memberships"]["equipment"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            3 => selected["selection"]["build"]["equipment"]["local"] = json!("0000000000000001"),
            _ => unreachable!(),
        }
        assert!(proof::review_flasks(&proof::radius_fixture()["observed"][0], &selected).is_err());
    }
    for case in 0..4 {
        let mut trace = proof::inherited_fixture()["loads"][0]["additional_observation"].clone();
        let gate = trace["gates"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|g| g["line"] == 1844 && g["recipient"]["profile"] == "RaisedSkeletonSniper")
            .unwrap();
        match case {
            0 => gate["hostile"] = json!(true),
            1 => gate["recipient"]["parent_is_player"] = json!(false),
            2 => {
                gate["recipient"]["source"]["matches"][0]["source"]["matches"][0]["gem_index"] =
                    json!(2)
            }
            3 => gate["recipient"]["children"][0]["owns_actor"] = json!(false),
            _ => unreachable!(),
        }
        assert!(proof::review_keystones(&trace, proof::selected()).is_err());
    }
}
#[test]
fn zero_inactive_and_nested_keystone_records_are_not_absence() {
    let _lane = lane();
    for value in [
        json!(0),
        json!(false),
        json!({"mod":{"name":"Keystone","type":"LIST","value":"Synthetic"}}),
    ] {
        let mut trace = proof::inherited_fixture()["loads"][0]["additional_observation"].clone();
        let d = primary(&mut trace);
        if value.is_object() {
            d["added"][0]["body"]["value"] = value;
        } else {
            d["added"][0]["body"]["name"] = json!("Keystone");
            d["added"][0]["body"]["value"] = value;
        }
        refuses(
            proof::review_keystones(&trace, proof::selected()),
            "Keystone candidate",
        );
    }
}
#[test]
fn primary_payload_multiplicity_and_secondary_call_census_remain_exact() {
    let _lane = lane();
    for case in 0..5 {
        let mut trace = proof::inherited_fixture()["loads"][0]["additional_observation"].clone();
        match case {
            0 => {
                primary(&mut trace)["added"].as_array_mut().unwrap().pop();
            }
            1 => {
                let d = primary(&mut trace);
                d["added"].as_array_mut().unwrap().swap(0, 1);
            }
            2 => trace["life_pool_calls"] = json!([{"kind":"spectre"}]),
            3 => {
                let gate = trace["gates"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|g| g["line"] == 3397)
                    .unwrap();
                gate["branch_entered"] = json!(true);
            }
            4 => {
                let gate = trace["gates"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|g| g["line"] == 3406)
                    .unwrap();
                gate["candidates"][0]["names"]["TotalCompanionLife"] = json!(1);
            }
            _ => unreachable!(),
        }
        assert!(
            proof::review_keystones(&trace, proof::selected()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn whole_transition_destinations_and_original_factory_bodies_are_authority() {
    let _lane = lane();
    for case in 0..5 {
        let mut b = proof::read("bodies.json");
        match case {
            0 => b["spans"][6]["body"] = json!("local enemyDB = actor.modDB\n"),
            1 => {
                b["spans"][12]["body"] =
                    json!("return function(db) db:NewMod('Keystone','LIST','X') end\n")
            }
            2 => b["primary_transition"][5]["reason"] = json!("All unknown names are absent"),
            3 => b["primary_transition"][7]["first"] = json!(1464),
            4 => {
                b["files"].as_array_mut().unwrap().pop();
            }
            _ => unreachable!(),
        }
        assert!(proof::review_bodies(&b).is_err(), "case {case}");
    }
}
#[test]
fn consistent_unrelated_capture_changes_do_not_authenticate_themselves() {
    let _lane = lane();
    let mut report = proof::inherited_fixture().clone();
    for i in 0..2 {
        let trace = &mut report["loads"][i]["additional_observation"];
        trace["snapshot"]["unreviewed_extra_diagnostic"] = json!(true);
        proof::review_keystones(trace, proof::selected()).unwrap();
    }
    refuses(
        proof::reconstruct(
            proof::radius_fixture(),
            &report,
            proof::selected(),
            &proof::read("bodies.json"),
        ),
        "independently authenticated captures",
    );
}
#[test]
#[ignore = "explicit offline retention; no source execution"]
fn retain_selected_flask_charm_keystone_proof() {
    let _lane = lane();
    proof::retain();
}
