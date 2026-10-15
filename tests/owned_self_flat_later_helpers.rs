#[allow(dead_code)]
#[path = "support/owned_self_flat_later_helpers.rs"]
mod proof;

use serde_json::{Value, json};
use std::sync::{Mutex, MutexGuard};
fn lane() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn refuses<T: std::fmt::Debug>(result: Result<T, String>, expected: &str) {
    let error = result.expect_err("changed source authority must refuse");
    assert!(
        error.contains(expected),
        "{error} did not contain {expected}"
    );
}
fn span_mut<'a>(body: &'a mut Value, role: &str) -> &'a mut Value {
    body["spans"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["role"] == role)
        .unwrap()
}
fn basic_rows(run: &mut Value) -> impl Iterator<Item = &mut Value> {
    run["skill_construction"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .filter(|r| r["active_effect"]["id"] == "MinionMeleeBow")
}

#[test]
fn later_helper_destinations_and_exact_basic_dispatch_are_bounded() {
    let _lane = lane();
    proof::check().unwrap();
    let report = proof::read("review.json");
    assert_eq!(
        report["construction"]["basic_rows_by_mode"],
        json!({"MAIN":32,"CALCS":2,"CALCULATOR":2})
    );
    assert_eq!(report["construction"]["triggered_by_nil_observed"], false);
    assert_eq!(
        report["source_deductions"]["Player_offence_minion_writes"],
        json!(["CritChance", "CritMultiplier"])
    );
    assert_eq!(
        report["source_deductions"]["late_ExtraAura_creation_preserved"],
        true
    );
    assert_eq!(
        report["source_deductions"]["late_ExtraAura_reused_or_future_pass_absence_proved"],
        false
    );
    for (_, value) in report["scope"].as_object().unwrap() {
        assert_eq!(value, false);
    }
    assert!(!report["remaining"].as_array().unwrap().is_empty());
}

#[test]
fn complete_source_files_spans_and_independent_review_cannot_be_reauthored() {
    let _lane = lane();
    for case in 0..5 {
        let mut body = proof::read("bodies.json");
        match case {
            0 => {
                body["files"].as_array_mut().unwrap().pop();
            }
            1 => body["files"][0]["path"] = json!("src/Modules/Other.lua"),
            2 => {
                span_mut(&mut body, "enemy-modifier-transport")["body"] =
                    json!("local enemyDB = actor.modDB")
            }
            3 => body["source_revision"] = json!("another revision"),
            4 => {
                body["writer_sites"][0]["body"] = json!("modDB:NewMod(\"PhysicalMin\",\"BASE\",0)")
            }
            _ => unreachable!(),
        }
        refuses(proof::review_bodies(&body), "independently pinned");
    }
}

#[test]
fn enemy_payload_and_computed_name_domains_keep_their_actual_destination_authority() {
    let _lane = lane();
    for case in 0..3 {
        let mut body = proof::read("bodies.json");
        let expected = match case {
            0 => {
                body["review"]["actor_relations"]["selected_friendly_minion"] = json!("Player");
                "Actor destinations"
            }
            1 => {
                body["review"]["dynamic_name_domains"]["converted_nondefence_resource"] =
                    json!(["PhysicalMin"]);
                "computed writer name domains"
            }
            2 => {
                body["review"]["generic_edges"][0]["arbitrary_nested_name"] = json!(false);
                "generic edges"
            }
            _ => unreachable!(),
        };
        refuses(proof::review_semantics(&body), expected);
    }
}

#[test]
fn coherent_direct_writer_change_is_not_just_a_checksum_mismatch() {
    let _lane = lane();
    let mut body = proof::read("bodies.json");
    let span = span_mut(&mut body, "attributes-and-calculation-callees");
    span["body"] = json!(
        span["body"]
            .as_str()
            .unwrap()
            .replace("ArmourAppliesToFireDamageTaken", "PhysicalMin")
    );
    let site = &mut body["writer_sites"][0];
    site["body"] = json!(
        site["body"]
            .as_str()
            .unwrap()
            .replace("ArmourAppliesToFireDamageTaken", "PhysicalMin")
    );
    refuses(
        proof::review_semantics(&body),
        "direct writer creates reviewed flat or factory name",
    );
    // A real source change still additionally needs independent manifest/body
    // review; coherently updating a local checksum does not reauthenticate it.
    refuses(proof::review_bodies(&body), "independently pinned");
}

#[test]
fn late_nested_aura_cannot_be_erased_or_promoted_to_future_pass_absence() {
    let _lane = lane();
    let mut body = proof::read("bodies.json");
    body["review"]["generic_edges"][1]["reused_or_later_pass_absence_proved"] = json!(true);
    refuses(proof::review_semantics(&body), "generic edges");
    let mut body = proof::read("bodies.json");
    body["scope"]["all_temporal_minion_writers_accounted"] = json!(true);
    refuses(proof::review_semantics(&body), "bounded proof scope");
    let mut body = proof::read("bodies.json");
    body["review"]["player_offence_minion_writes"] = json!([]);
    refuses(proof::review_semantics(&body), "Player offence");
}

#[test]
fn trigger_handler_or_unique_source_changes_require_review_even_when_records_are_empty() {
    let _lane = lane();
    let f = proof::fixture();
    for case in 0..5 {
        let mut run = f["off"].clone();
        for row in basic_rows(&mut run) {
            match case {
                0 => row["active_effect"]["granted_effect_fields"]["name"] = json!("Snipe"),
                1 => row["skill_data"] = json!({"triggerSource":"Snipe"}),
                2 => row["socket_group"] = json!({"source":"Item:1:Snipe,1"}),
                3 => row["supports"] = json!([{"grantedEffect":{"isTrigger":true}}]),
                4 => row["actor_parent_is_player"] = json!(false),
                _ => unreachable!(),
            }
            // Keep call/return and the sole effect coherent; this must still
            // reject changed semantic authority, not merely mismatched copies.
            row["effects"][0] = row["active_effect"].clone();
        }
        assert!(proof::review_run(&run, &f["recipes"], &f["selected"]).is_err());
    }
    let mut body = proof::read("bodies.json");
    body["review"]["trigger_config_keys"][0] = json!("basic attack");
    refuses(proof::review_semantics(&body), "trigger dispatch keys");
}

#[test]
fn equivalent_child_counts_do_not_authenticate_changed_non_child_source_inputs() {
    let _lane = lane();
    let mut f = proof::fixture().clone();
    for mode in ["off", "on"] {
        f[mode]["environments"][0]["player_store"][0]["buckets"]["Additional"] = json!([{
            "position":1,"body":{"name":"ExtraAura","type":"LIST","value":{"mod":{"name":"PhysicalMin","type":"BASE","value":0}}}
        }]);
        proof::review_run(&f[mode], &f["recipes"], &f["selected"]).unwrap();
    }
    refuses(
        proof::reconstruct(&f, &proof::read("bodies.json")),
        "complete independently authenticated source inputs",
    );
}

#[test]
#[ignore = "explicit offline retention; no original-source execution"]
fn retain_later_helper_review() {
    let _lane = lane();
    proof::retain();
}
