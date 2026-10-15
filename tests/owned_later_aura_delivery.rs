#[allow(dead_code)]
#[path = "support/owned_later_aura_delivery.rs"]
mod proof;
use serde_json::json;
use std::sync::{Mutex, MutexGuard};
fn lane() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
#[test]
fn actual_later_aura_consumers_reconstruct_from_fixed_fresh_loads() {
    let _lane = lane();
    proof::check();
}
#[test]
fn missing_misbound_or_filtered_candidates_are_not_absence() {
    let _lane = lane();
    let original = proof::fixture()["loads"].as_array().unwrap();
    for case in 0..10 {
        let mut loads = original.clone();
        for load in &mut loads[..2] {
            let t = &mut load["additional_observation"];
            match case {
                0 => {
                    t["later_aura_queries"].as_array_mut().unwrap().pop();
                }
                1 => t["later_aura_queries"][0]["perform_id"] = json!(999),
                2 => t["later_aura_queries"][0]["exact_player_store"] = json!(false),
                3 => {
                    t["later_aura_queries"][0]["source_stores"] = json!([]);
                    t["later_aura_queries"][0]["source_stores_after"] = json!([]);
                }
                4 => {
                    t["later_aura_queries"][0]["source_stores"][0]["records"] = json!([{"body":{"name":"AffectedByAuraMod","type":"LIST","value":{"mod":{"name":"PhysicalMin","value":0}}}}]);
                    t["later_aura_queries"][0]["source_stores"][0]["names"]["AffectedByAuraMod"] =
                        json!(1);
                    t["later_aura_queries"][0]["source_stores_after"] =
                        t["later_aura_queries"][0]["source_stores"].clone();
                }
                5 => {
                    t["later_aura_queries"][0]["list_return"] =
                        json!([{"position":1,"body":{"mod":{"name":"PhysicalMin","value":0}}}])
                }
                6 => {
                    t["later_aura_queries"][0]["recipient_state"] =
                        json!({"kind":"table","entries":[]})
                }
                7 => t["party_aura_gates"][0]["extra_aura"] = json!({"payload":"changed"}),
                8 => t["later_aura_adds"] = json!([{"event":"call","line":3221}]),
                _ => {
                    t["later_aura_queries"][0]["selected_recipient"] = json!({"present":false});
                    t["party_aura_gates"][0]["recipient"] = json!({"present":false});
                }
            }
        }
        assert!(
            std::panic::catch_unwind(|| proof::assess(&loads)).is_err(),
            "case {case}"
        );
    }
}
#[test]
#[ignore = "offline retention from one immutable fixed acquisition"]
fn retain_later_aura_review() {
    let _lane = lane();
    proof::retain();
}
