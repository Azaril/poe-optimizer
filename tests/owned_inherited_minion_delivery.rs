#[path = "../crates/poe-optimizer-pob/tests/support/inherited_minion_capture_assessment.rs"]
mod capture_assessment;
#[allow(dead_code)]
#[path = "support/owned_inherited_minion_delivery.rs"]
mod proof;
use serde_json::{Value, json};
use std::sync::{Mutex, MutexGuard};
fn lane() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn trace() -> &'static Value {
    &proof::fixture()["loads"][0]["additional_observation"]
}
fn other() -> &'static Value {
    &proof::fixture()["loads"][1]["additional_observation"]
}
fn refuses(result: Result<Value, String>, needle: &str) {
    let error = result.expect_err("changed authority must refuse");
    assert!(error.contains(needle), "{error}");
}

#[test]
fn fixed_acquisition_and_per_name_receiving_proof_reconstruct() {
    let _lane = lane();
    proof::check().unwrap();
    let p = proof::read("review.json");
    assert_eq!(p["comparison"]["raw_execution_traces_equal"], false);
    assert_eq!(p["comparison"]["per_name_receiving_effects_equal"], true);
    assert_eq!(p["four_scalar_controls_exact"], true);
    assert_eq!(p["comparison"]["temporal"]["performs"], 18);
    assert_eq!(p["comparison"]["temporal"]["deliveries"], 10);
    assert_eq!(p["comparison"]["temporal"]["appended"], 307);
    assert_eq!(p["scope"]["native_coverage_authorized"], false);
}
#[test]
fn original_call_return_pointer_and_append_correspondence_is_required() {
    let _lane = lane();
    for kind in 0..8 {
        let mut d = trace()["deliveries"][0].clone();
        let expected = match kind {
            0 => {
                d["original_list_call_count"] = json!(2);
                "one original List"
            }
            1 => {
                d["returned"] = json!(false);
                "one original List"
            }
            2 => {
                d["source_stores_after"][2]["records"][0]["body"]["flags"] = json!(1);
                "source stores unchanged"
            }
            3 => {
                d["list_return"][0]["exact_raw_value_candidates"][0]["position"] = json!(999);
                "raw value source"
            }
            4 => {
                d["added"][0]["returned_value_matches"] = json!([2]);
                "returned payload pointer"
            }
            5 => {
                d["added"][0]["after_count"] = d["added"][0]["before_count"].clone();
                "one original bucket append"
            }
            6 => {
                d["list_return"].as_array_mut().unwrap().pop();
                "all original"
            }
            _ => {
                d["added"].as_array_mut().unwrap().pop();
                "appended once"
            }
        };
        refuses(proof::review_delivery(&d), expected);
    }
}
#[test]
fn outer_filters_unknown_recipient_conditions_and_flat_names_are_not_ignored() {
    let _lane = lane();
    for (key, value) in [
        ("flags", json!(1)),
        (
            "_positions",
            json!([{"index":1,"value":{"type":"Condition","var":"Never"}}]),
        ),
        (
            "value",
            json!({"mod":{"name":"PhysicalMin","type":"BASE","value":0}}),
        ),
    ] {
        let mut d = trace()["deliveries"][0].clone();
        d["source_stores"][2]["records"][0]["body"][key] = value;
        d["source_stores_after"] = d["source_stores"].clone();
        assert!(proof::review_delivery(&d).is_err());
    }
}
#[test]
fn per_perform_gate_environment_and_orphan_calls_are_checked() {
    let _lane = lane();
    for kind in 0..6 {
        let mut t = trace().clone();
        let expected = match kind {
            0 => {
                let rows = t["deliveries"].as_array_mut().unwrap();
                let mut orphan = rows[0].clone();
                orphan["delivery_id"] = json!(rows.len() + 1);
                orphan["perform_id"] = json!(0);
                rows.push(orphan);
                "delivery identity census"
            }
            1 => {
                t["gates"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|g| !(g["perform_id"] == 1 && g["line"] == 2037));
                "per-perform gate census"
            }
            2 => {
                t["gates"][0]["env_id"] = json!(999);
                "gate environment"
            }
            3 => {
                let g = t["gates"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|g| g["line"] == 3397)
                    .unwrap();
                g["branch_entered"] = json!(true);
                "Life-pool branch"
            }
            4 => {
                t["deliveries"][0]["env_id"] = json!(999);
                "delivery environment"
            }
            _ => {
                t["gates"][0]["perform_id"] = json!(0);
                "gate census"
            }
        };
        refuses(proof::review_trace(&t), expected);
    }
}
#[test]
fn spectre_and_exact_receiving_actor_changes_are_not_inferred_absent() {
    let _lane = lane();
    for kind in 0..3 {
        let mut t = trace().clone();
        let expected = if kind == 2 {
            t["deliveries"][0]["recipient"]["source"]["matches"][0]["source"]["matches"][0]["gem_index"] =
                json!(999);
            "receiving Actor"
        } else {
            let g = t["gates"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|g| g["line"] == 2037)
                .unwrap();
            if kind == 0 {
                g["branch_entered"] = json!(true);
                "Spectre branch"
            } else {
                g["skills"][0]["source"]["effect_id"] = json!("SummonSpectreNew");
                "Spectre prefix"
            }
        };
        refuses(proof::review_trace(&t), expected);
    }
}
#[test]
fn equal_payload_or_new_name_cannot_replace_exact_constructed_source() {
    let _lane = lane();
    let mut t = trace().clone();
    t["deliveries"][0]["source"]["source"]["matches"][0]["gem_index"] = json!(999);
    refuses(
        proof::review_sources(&t, proof::sources()),
        "supplying Skill constructor",
    );
    let mut t = trace().clone();
    let d = &mut t["deliveries"][0];
    d["source_stores"][2]["records"][0]["body"]["value"]["mod"]["name"] = json!("UnknownNewStat");
    d["source_stores_after"] = d["source_stores"].clone();
    d["list_return"][0]["body"] = d["source_stores"][2]["records"][0]["body"]["value"].clone();
    d["added"][0]["body"] = d["list_return"][0]["body"]["mod"].clone();
    proof::review_delivery(d).unwrap();
    refuses(
        proof::review_sources(&t, proof::sources()),
        "constructed sources",
    );
}
#[test]
fn coherent_same_name_reordering_is_not_cross_name_inventory_variability() {
    let _lane = lane();
    let mut changed = trace().clone();
    let d = &mut changed["deliveries"][0];
    let records = d["source_stores"][2]["records"].as_array().unwrap();
    let indices: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r["body"]["value"]["mod"]["name"] == "Damage")
        .map(|(i, _)| i)
        .collect();
    let (a, b) = (indices[0], indices[1]);
    let x = d["source_stores"][2]["records"][a]["body"].clone();
    d["source_stores"][2]["records"][a]["body"] =
        d["source_stores"][2]["records"][b]["body"].clone();
    d["source_stores"][2]["records"][b]["body"] = x;
    d["source_stores_after"] = d["source_stores"].clone();
    for i in [a, b] {
        d["list_return"][i]["body"] = d["source_stores"][2]["records"][i]["body"]["value"].clone();
        d["added"][i]["body"] = d["list_return"][i]["body"]["mod"].clone();
    }
    proof::review_trace(&changed).unwrap();
    proof::review_sources(&changed, proof::sources()).unwrap();
    refuses(
        proof::compare_traces(trace(), &changed, proof::sources()),
        "per-name ordered receiving effects",
    );
    proof::compare_traces(trace(), other(), proof::sources()).unwrap();
}
#[test]
fn complete_original_files_and_acquisition_metadata_remain_sealed() {
    let _lane = lane();
    let mut bodies = proof::read("bodies.json");
    bodies["spans"][0]["body"] = json!("different implementation");
    assert!(proof::authenticate_bodies(&bodies).is_err());
    let mut input = proof::fixture().clone();
    input["evidence"]["strict_trace_comparison_passed"] = json!(true);
    refuses(
        proof::reconstruct(&input, &proof::read("bodies.json")),
        "temporal acquisition identity",
    );
}
#[test]
#[ignore = "explicit offline retention of independently checked original evidence"]
fn retain_inherited_minion_delivery_proof() {
    let _lane = lane();
    proof::retain();
}

#[test]
fn maintained_capture_assessment_accepts_recorded_order_variability_but_requires_scalars() {
    let _lane = lane();
    let raw = proof::fixture()["loads"].as_array().unwrap();
    let result = capture_assessment::assess(raw);
    assert_eq!(result["comparison"]["all_trace_fields_exact"], false);
    assert_eq!(
        result["comparison"]["offline_receiving_proof_required"],
        true
    );
    assert_eq!(
        result["comparison"]["semantic_receiving_proof_executed"],
        false
    );
    let mut changed = raw.clone();
    changed[3]["additional_observation"]["unexpected_scalar"] = json!(1);
    assert!(std::panic::catch_unwind(|| capture_assessment::assess(&changed)).is_err());
    for case in 0..10 {
        // Erase the same fields coherently in all four captures: equality alone
        // must never turn absent evidence into deterministic scalar agreement.
        let mut changed = raw.clone();
        for (index, load) in changed.iter_mut().enumerate() {
            let snapshot = if index < 2 {
                &mut load["additional_observation"]["snapshot"]
            } else {
                &mut load["additional_observation"]
            };
            match case {
                0 => *snapshot = Value::Null,
                1 => *snapshot = json!({}),
                2 => {
                    snapshot.as_object_mut().unwrap().remove("calcs");
                }
                3 => snapshot["main"]["player_output"]["scalars"] = json!({}),
                4 => snapshot["calcs"]["player_output"] = json!({"available":false,"scalars":{}}),
                5 => snapshot["main"]["selected_minion"]["matches"] = json!([]),
                6 => snapshot["main"]["player_selected"]["source_occurrence"] = json!({}),
                7..=9 => {
                    let actor = snapshot["main"]["actors"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|a| a["selected"] == true)
                        .unwrap();
                    match case {
                        7 => actor["output"] = json!({"available":false,"scalars":{}}),
                        8 => actor["children"] = json!([]),
                        9 => actor["children"][0]["preconversion"] = json!({}),
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        assert!(
            std::panic::catch_unwind(|| capture_assessment::assess(&changed)).is_err(),
            "coherently erased snapshot case {case}"
        );
    }
}
