#[path = "support/owned_self_flat_skill_construction.rs"]
mod proof;

use serde_json::{Value, json};
use std::sync::{Mutex, MutexGuard};

fn source_lane() -> MutexGuard<'static, ()> {
    static LANE: Mutex<()> = Mutex::new(());
    LANE.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn refused(result: Result<Value, String>, message: &str) {
    let failure = result.expect_err("changed source evidence must refuse");
    assert!(failure.contains(message), "{failure}");
}

#[test]
fn retained_complete_construction_and_observed_payloads_reconstruct() {
    let _lane = source_lane();
    proof::check().unwrap();
    let retained = proof::read("review.json");
    assert_eq!(retained["construction"]["constructor_pairs"], 768);
    assert_eq!(retained["construction"]["recipe_definitions"], 40);
    assert_eq!(retained["construction"]["support_definitions"], 12);
    assert_eq!(retained["construction"]["basic_attack_pairs"], 18);
    assert_eq!(retained["delivery"]["group_deliveries"], 28);
    assert_eq!(
        retained["delivery"]["unique_delivering_source_attribution"],
        false
    );
    assert_eq!(retained["scope"]["native_coverage_authorized"], false);
}

#[test]
fn nested_zero_inactive_and_external_or_callback_records_are_not_filtered_out() {
    let _lane = source_lane();
    for name in ["PhysicalMin", "PhysicalMax", "ExtraSkillStat"] {
        let record = json!({"name":"MinionModifier","type":"LIST","value":{
            "type":"inactive-recipient","mod":{"name":name,"type":"BASE","value":0,
            "_positions":[{"index":1,"value":{"type":"Condition","var":"Never"}}]}}});
        assert!(proof::review_record_domain(&record).is_err());
    }
    for record in [
        json!({"quality":{"kind":"callback_reference","reference":"new callback"}}),
        json!({"mapping":{"kind":"table_reference","reference":"unreviewed alias"}}),
        json!({"callbacks":["afterMerge"]}),
        json!({"aliases":["renamed output"]}),
    ] {
        assert!(proof::review_record_domain(&record).is_err());
    }
    let f = proof::fixture();
    let mut recipes = f["recipes"].clone();
    recipes["skills"][0]["stat_set_base_mods"][0]["records"]["value"]["new"] =
        json!({"name":"PhysicalMin","value":0});
    refused(
        proof::review_run(&f["off"], &recipes, &f["selected"]),
        "physical",
    );
}

#[test]
fn malformed_or_sparse_captured_tags_cannot_become_tagless() {
    let _lane = source_lane();
    for tags in [
        json!(null),
        json!("unavailable"),
        json!({"missing":true}),
        json!([{"index":2,"value":{"type":"GlobalEffect"}}]),
        json!([{"index":1}]),
        json!([{"index":1,"value":{},"unreviewed":true}]),
        json!([{"index":1,"value":{}},{"index":1,"value":{}}]),
    ] {
        let record = json!({"name":"Damage","_positions":tags});
        assert!(proof::review_record_domain(&record).is_err(), "{record}");
    }
    for record in [
        json!({"name":"Damage"}),
        json!({"name":"Damage","_positions":{}}),
        json!({"name":"Damage","_positions":[]}),
    ] {
        proof::review_record_domain(&record).unwrap();
    }
    let f = proof::fixture();
    let mut run = f["off"].clone();
    run["skill_construction"][0]["records"][0]["body"]["_positions"] = json!(false);
    refused(
        proof::review_run(&run, &f["recipes"], &f["selected"]),
        "sequence",
    );
}

#[test]
fn complete_pairs_and_exact_source_context_are_required() {
    let _lane = source_lane();
    let f = proof::fixture();
    for mutation in 0..5 {
        let mut run = f["off"].clone();
        let rows = run["skill_construction"].as_array_mut().unwrap();
        let expected = match mutation {
            0 => {
                rows.pop();
                "every before/after"
            }
            1 => {
                rows[0]["phase"] = json!("after-construction");
                "observer call/return"
            }
            2 => {
                rows[1]["source"] = json!({"source_present":false,"matches":{}});
                "source/recipient context"
            }
            3 => {
                rows[0]["actor_parent_is_player"] = json!(true);
                "own child"
            }
            _ => {
                rows[1]["effects"] = json!({});
                "source/recipient context"
            }
        };
        refused(
            proof::review_run(&run, &f["recipes"], &f["selected"]),
            expected,
        );
    }
    let mut run = f["off"].clone();
    let rows = run["skill_construction"].as_array_mut().unwrap();
    let child = rows
        .iter_mut()
        .find(|r| r["active_effect"]["id"] == "MinionMeleeBow")
        .unwrap();
    child["summoner"]["matches"][0]["gem_index"] = json!(999);
    refused(
        proof::review_run(&run, &f["recipes"], &f["selected"]),
        "exact supplied source",
    );
}

#[test]
fn record_order_multiplicity_and_global_conservation_are_checked() {
    let _lane = source_lane();
    let f = proof::fixture();
    for mutation in 0..4 {
        let mut run = f["off"].clone();
        let rows = run["skill_construction"].as_array_mut().unwrap();
        if mutation < 2 {
            let row = rows
                .iter_mut()
                .find(|r| {
                    r["phase"] == "after-construction"
                        && r["records"].as_array().is_some_and(|a| a.len() > 1)
                })
                .unwrap();
            if mutation == 0 {
                row["records"][0]["body"]["value"] = json!(123456);
            } else {
                let old = row["records"][0]["body"].clone();
                row["records"][0]["body"] = row["records"][1]["body"].clone();
                row["records"][1]["body"] = old;
            }
        } else {
            let row = rows
                .iter_mut()
                .find(|r| {
                    r["phase"] == "after-construction"
                        && r["buffs"].as_array().is_some_and(|a| !a.is_empty())
                })
                .unwrap();
            if mutation == 2 {
                row["buffs"][0]["records"][0]["body"]["value"] = json!(123456);
            } else {
                let records = row["buffs"][0]["records"].as_array_mut().unwrap();
                let mut duplicate = records[0].clone();
                duplicate["position"] = json!(records.len() + 1);
                records.push(duplicate);
            }
        }
        refused(
            proof::review_run(&run, &f["recipes"], &f["selected"]),
            if mutation < 2 {
                "ordinary local records"
            } else {
                "global bodies"
            },
        );
    }
}

#[test]
fn delivery_requires_full_payload_and_the_exact_observed_receiving_actor() {
    let _lane = source_lane();
    let f = proof::fixture();
    for mutation in 0..7 {
        let mut gate = f["gates"][0].clone();
        let deliveries = gate["group_deliveries"].as_array_mut().unwrap();
        let row = deliveries
            .iter_mut()
            .find(|d| d["recipient"] == "selected_minion")
            .unwrap();
        let expected = match mutation {
            0 => {
                row["exact_recipient_store"] = json!(false);
                "store identity"
            }
            1 => {
                row["selected_recipient"]["matches"][0]["ordinal"] = json!(999);
                "typed selected recipient"
            }
            2 => {
                row["modifiers"][0]["value"] = json!(123456);
                "full payload"
            }
            3 => {
                let modifiers = row["modifiers"].as_array_mut().unwrap();
                modifiers.push(modifiers[0].clone());
                "full payload"
            }
            4 => {
                row["line"] = json!(3132);
                "minion-buff delivery"
            }
            5 => {
                row["modifiers"] = json!(null);
                "sequence"
            }
            _ => {
                row["selected_recipient"]["matches"][0]["effect_id"] =
                    json!("SummonSkeletonReaversPlayer");
                "typed selected recipient"
            }
        };
        refused(
            proof::review_delivery(&f["off"], &gate, &f["selected"]),
            expected,
        );
    }
    let mut gate = f["gates"][0].clone();
    gate["player_buff_visits"][0]["source"] = json!({"source_present":false,"matches":{}});
    refused(
        proof::review_delivery(&f["off"], &gate, &f["selected"]),
        "constructed source, Actor",
    );
}

#[test]
fn complete_original_source_archives_and_reviewed_bodies_are_mandatory() {
    let _lane = source_lane();
    for mutation in 0..5 {
        let mut bodies = proof::read("bodies.json");
        match mutation {
            0 => {
                bodies["files"].as_array_mut().unwrap().pop();
            }
            1 => {
                bodies["spans"].as_array_mut().unwrap().pop();
            }
            2 => {
                bodies["spans"][0]["body"] = json!("return unchanged\n");
            }
            3 => {
                bodies["files"][0]["sha256"] = json!("forged");
            }
            _ => {
                bodies["spans"][0]["last"] = json!(50);
            }
        }
        assert!(
            proof::review_bodies(&bodies).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn cross_stat_inventory_policy_keeps_fields_duplicates_and_within_stat_order() {
    let left = json!({"fields":{"type":"Buff"},"modifiers":[
        {"name":"Damage","value":1,"source":"first"},
        {"name":"Speed","value":2},
        {"name":"Damage","value":3,"source":"second"}
    ]});
    let mut reordered = left.clone();
    reordered["modifiers"].as_array_mut().unwrap().swap(0, 1);
    assert!(proof::buff_payload_corresponds(&left, &reordered).unwrap());
    for mutation in 0..4 {
        let mut changed = left.clone();
        match mutation {
            0 => changed["modifiers"].as_array_mut().unwrap().swap(0, 2),
            1 => {
                changed["modifiers"].as_array_mut().unwrap().remove(2);
            }
            2 => changed["fields"]["type"] = json!("Debuff"),
            _ => changed["additional_source"] = json!(true),
        }
        assert!(!proof::buff_payload_corresponds(&left, &changed).unwrap());
    }
}

#[test]
fn equal_payload_cannot_authenticate_a_changed_valid_recipient() {
    let _lane = source_lane();
    let f = proof::fixture();
    let mut changed = f.clone();
    let gate = &mut changed["gates"][0];
    let rows = gate["group_deliveries"].as_array_mut().unwrap();
    let sniper = rows
        .iter()
        .position(|r| {
            r["recipient"] == "selected_minion"
                && r["selected_recipient"]["matches"][0]["effect_id"]
                    == "SummonSkeletalSnipersPlayer"
        })
        .unwrap();
    let other = rows
        .iter()
        .position(|r| {
            r["recipient"] == "selected_minion"
                && r["selected_recipient"]["matches"][0]["effect_id"]
                    == "SummonSkeletalArsonistsPlayer"
                && r["mode"] == rows[sniper]["mode"]
        })
        .unwrap();
    assert_eq!(rows[sniper]["modifiers"], rows[other]["modifiers"]);
    let prior = rows[sniper]["selected_recipient"].clone();
    rows[sniper]["selected_recipient"] = rows[other]["selected_recipient"].clone();
    rows[other]["selected_recipient"] = prior;
    // Both are valid observed constructors and can receive this payload. Only
    // the pinned observation authorizes which store this particular call used.
    proof::review_delivery(&f["off"], gate, &f["selected"]).unwrap();
    // Swapping preserves the census too. It still cannot authenticate the
    // changed exact receiving store for either individual observed call.
    refused(
        proof::reconstruct(&changed, &proof::read("bodies.json")),
        "authenticated acquisition",
    );
}

#[test]
fn coherent_transport_change_is_not_mistaken_for_pinned_recipe_evaluation() {
    let _lane = source_lane();
    let f = proof::fixture();
    let mut changed = f.clone();
    // A coherent local before/after change preserves the transport law but is
    // neither the authenticated acquisition nor a proof of recipe evaluation.
    for run in ["off", "on"] {
        for row in changed[run]["skill_construction"].as_array_mut().unwrap() {
            if row["active_effect"]["id"] == "MinionMeleeBow" {
                row["records"][0]["body"]["value"] = json!(16);
            }
        }
    }
    proof::review_run(&changed["off"], &changed["recipes"], &changed["selected"]).unwrap();
    refused(
        proof::reconstruct(&changed, &proof::read("bodies.json")),
        "authenticated acquisition",
    );
    let mut changed = f.clone();
    changed["authority"]["scope"]["native_coverage_authorized"] = json!(true);
    refused(
        proof::reconstruct(&changed, &proof::read("bodies.json")),
        "proof authority",
    );
}

#[test]
#[ignore = "explicit offline retention from existing authenticated acquisitions; no PoB execution"]
fn retain_selected_skill_construction_proof() {
    let _lane = source_lane();
    proof::retain();
}
