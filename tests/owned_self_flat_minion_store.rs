#[allow(dead_code)]
#[path = "support/owned_self_flat_minion_store.rs"]
mod proof;
use serde_json::{Value, json};
use std::sync::{Mutex, MutexGuard};
fn lane() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn refuses(result: Result<Value, String>, message: &str) {
    let error = result.expect_err("changed semantic evidence must refuse");
    assert!(error.contains(message), "{error} did not contain {message}");
}
#[test]
fn selected_initializer_and_receiving_supplier_census_is_bounded() {
    let _lane = lane();
    proof::check().unwrap();
    let review = proof::read("review.json");
    assert_eq!(
        review["initialization"]["literal_initializer_writes"],
        json!([84, 24])
    );
    assert_eq!(review["initialization"]["summoner_construction_rows"], 42);
    assert_eq!(review["receiving_gates"]["selected_sniper_direct_buffs"], 2);
    for key in [
        "all_temporal_minion_writers_accounted",
        "native_membership_complete",
        "native_coverage_authorized",
        "new_source_execution",
        "numeric_order_authority",
    ] {
        assert_eq!(review["scope"][key], false);
    }
    assert!(!review["remaining"].as_array().unwrap().is_empty());
}
#[test]
fn initializer_and_full_summon_table_authority_cannot_be_omitted_or_relabelled() {
    let _lane = lane();
    for case in 0..4 {
        let mut body = proof::read("bodies.json");
        match case {
            0 => {
                body["files"].as_array_mut().unwrap().pop();
            }
            1 => {
                body["spans"][1]["body"] = json!("minion.modDB:NewMod(\"PhysicalMin\",\"BASE\",0)")
            }
            2 => body["spans"][5]["body"] = json!("minionUses = { [\"Weapon 1\"] = true }"),
            3 => {
                body["initializer_writers"][0]["literal_writes"][0] =
                    json!("GainMainHandDmgFromParent")
            }
            _ => unreachable!(),
        }
        assert!(proof::review_bodies(&body).is_err());
    }
}
#[test]
fn zero_inactive_nested_or_generic_suppliers_are_still_source_candidates() {
    let _lane = lane();
    let f = proof::fixture();
    for name in [
        "PhysicalMin",
        "PhysicalMax",
        "GainMainHandDmgFromParent",
        "ExtraSkillMod",
        "AffectedByAuraMod",
        "Keystone",
    ] {
        let mut run = f["off"].clone();
        run["environments"][0]["player_store"][0]["buckets"]["Injected"] = json!([{"position":1,"body":{
            "name":"MinionModifier","type":"LIST","value":{"mod":{"name":name,"type":"BASE","value":0,
            "_positions":[{"index":1,"value":{"type":"Condition","var":"Never"}}]}}}}]);
        refuses(proof::review_run(&run, &f["recipes"]), "source record");
    }
}
#[test]
fn profile_equipment_and_summoner_input_changes_require_new_review() {
    let _lane = lane();
    let f = proof::fixture();
    for case in 0..5 {
        let mut run = f["off"].clone();
        let mut recipes = f["recipes"].clone();
        let expected = match case {
            0 => {
                run["environments"][0]["nodes"][0]["key"] = json!(39935);
                "Talisman"
            }
            1 => {
                run["environments"].as_array_mut().unwrap().pop();
                "environment"
            }
            2 => {
                let row = run["skill_construction"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["active_effect"]["id"] == "SummonSkeletalSnipersPlayer")
                    .unwrap();
                row["skill_data"] = json!({"minionUseBowAndQuiver":true});
                "equipment-copy"
            }
            3 => {
                let profile = recipes["minion_profiles"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["id"] == "RaisedSkeletonSniper")
                    .unwrap();
                profile["mod_list"]["value"] = json!([{"name":"Damage","type":"INC","value":1}]);
                "profile modifiers"
            }
            4 => {
                let row = run["skill_construction"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["active_effect"]["id"] == "SummonSkeletalSnipersPlayer")
                    .unwrap();
                row["active_effect"]["granted_effect_fields"]["minionHasItemSet"] = json!(true);
                "equipment metadata"
            }
            _ => unreachable!(),
        };
        refuses(proof::review_run(&run, &recipes), expected);
    }
}
#[test]
fn curse_and_direct_buff_payload_or_recipient_changes_cannot_be_hidden() {
    let _lane = lane();
    let f = proof::fixture();
    let mut run = f["off"].clone();
    let row = run["skill_construction"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["buffs"].as_array().is_some_and(|b| !b.is_empty()))
        .unwrap();
    row["buffs"][0]["fields"]["type"] = json!("CurseBuff");
    refuses(
        proof::review_run(&run, &f["recipes"]),
        "curse/aura/link/warcry",
    );
    for case in 0..3 {
        let mut gate = f["gates"][0].clone();
        let row = gate["group_deliveries"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| {
                r["selected_recipient"]["matches"][0]["effect_id"] == "SummonSkeletalSnipersPlayer"
            })
            .unwrap();
        let expected = match case {
            0 => {
                row["modifiers"][0]["name"] = json!("PhysicalMin");
                "source record"
            }
            1 => {
                row["modifiers"][0] = row["modifiers"][1].clone();
                "both distinct names"
            }
            2 => {
                row["selected_recipient"]["matches"][0]["source"]["matches"][0]["gem_index"] =
                    json!(999);
                "summoner join"
            }
            _ => unreachable!(),
        };
        refuses(proof::review_gates(&gate, f), expected);
    }
}
#[test]
fn dynamic_gates_preserve_absence_and_original_consumer_identity() {
    let _lane = lane();
    let f = proof::fixture();
    for case in 0..8 {
        let mut gate = f["gates"][0].clone();
        let expected = match case {
            0 => {
                gate["flag_calls"][0]["gate_index"] = json!(0);
                "flag gate identity"
            }
            1 => {
                gate["flag_calls"][0]["original_flag_call_observed"] = json!(false);
                "flag invocation"
            }
            2 => {
                gate["parent_weapon"][0]["flag"]["supplemental_original_flag"] =
                    json!({"present":true,"value":false});
                "flag absent"
            }
            3 => {
                gate["hollow_palm"][0]["actor_condition"] = json!({"present":true,"value":false});
                "Hollow Palm short circuit"
            }
            4 => {
                gate["aura_queries"][0]["raw"] =
                    json!([{"name":"ExtraAura","type":"LIST","value":0}]);
                "ExtraAura list absence"
            }
            5 => {
                gate["aura_queries"][0]["original_list_return_observed"] = json!(false);
                "ExtraAura list absence"
            }
            6 => {
                gate["parent_weapon"][0]["summoner_owns_actor"] = json!(false);
                "receiving Sniper Actor"
            }
            7 => {
                let opposite = if gate["flag_calls"][0]["mode"] == "MAIN" {
                    "CALCS"
                } else {
                    "MAIN"
                };
                gate["flag_calls"][0]["mode"] = json!(opposite);
                "flag invocation source join"
            }
            _ => unreachable!(),
        };
        refuses(proof::review_gates(&gate, f), expected);
    }
}
#[test]
fn finite_name_census_does_not_authenticate_changed_formula_or_source_values() {
    let _lane = lane();
    let mut f = proof::fixture().clone();
    // A name census alone deliberately makes no recipe-value or arithmetic
    // claim. The complete captured-input seal must reject even a coherent
    // non-flat value change in both repeated observations.
    for mode in ["off", "on"] {
        f[mode]["environments"][0]["player_store"][0]["buckets"]["Independent"] =
            json!([{"position":1,"body":{"name":"Life","type":"BASE","value":17}}]);
        proof::review_run(&f[mode], &f["recipes"]).unwrap();
    }
    refuses(
        proof::reconstruct(&f, &proof::read("bodies.json")),
        "complete authenticated",
    );
}
#[test]
#[ignore = "explicit offline retention; no source execution"]
fn retain_selected_minion_store_review() {
    let _lane = lane();
    proof::retain();
}
