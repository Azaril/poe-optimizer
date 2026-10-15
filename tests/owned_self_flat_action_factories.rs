#[allow(dead_code)]
#[path = "support/owned_self_flat_action_factories.rs"]
mod proof;
use serde_json::{Value, json};
use std::sync::{Mutex, MutexGuard};
fn lane() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn refuses(result: Result<Value, String>, message: &str) {
    let e = result.expect_err("changed selected source proof must refuse");
    assert!(e.contains(message), "{e}");
}
fn physical(report: &mut Value) -> &mut Value {
    let actor = report["cases"][0]["state"]["main"]["actors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["summon_effect_id"] == "SummonSkeletalSnipersPlayer")
        .unwrap();
    let child = actor["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["effect_id"] == "MinionMeleeBow")
        .unwrap();
    child["base_calls"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["damage_type"] == "Physical")
        .unwrap()
}
#[test]
fn exact_basic_attack_factory_scope_reconstructs() {
    let _lane = lane();
    proof::check().unwrap();
    let r = proof::read("review.json");
    assert_eq!(r["selection"]["spell_factory_write_sites"], 20);
    assert_eq!(r["selection"]["actual_main_hand_flags"], proof::FLAGS);
    assert_eq!(r["selection"]["selected_recipe_callbacks"], 0);
    for key in [
        "all_spell_suppliers_absent",
        "all_minion_store_writers_closed",
        "native_membership_complete",
        "native_coverage_authorized",
        "new_source_execution",
        "source_tag_evaluator_reimplemented",
    ] {
        assert_eq!(r["scope"][key], false);
    }
}
#[test]
fn positive_spell_and_attack_counterexamples_keep_filter_and_absence_distinct() {
    // No numerical values or source-tag evaluation are needed when this
    // necessary predicate already fails. Positive controls prevent treating
    // every supplier as absent or every modifier as inapplicable.
    assert!(!proof::flag_predicate(Some(proof::FLAGS), 2).unwrap());
    assert!(!proof::flag_predicate(Some(proof::FLAGS), 3).unwrap());
    assert!(proof::flag_predicate(Some(proof::FLAGS), 1).unwrap());
    assert!(proof::flag_predicate(Some(proof::FLAGS | 2), 2).unwrap());
    assert!(proof::flag_predicate(Some(proof::FLAGS | 2), 3).unwrap());
    assert!(!proof::flag_predicate(None, 2).unwrap());
    assert!(proof::flag_predicate(None, 0).unwrap());
    assert!(proof::flag_predicate(Some(1_u64 << 53), 2).is_err());
    assert!(proof::flag_predicate(Some(proof::FLAGS), 1_u64 << 32).is_err());
}
#[test]
fn complete_factory_filter_and_callback_bodies_cannot_be_changed_or_omitted() {
    let _lane = lane();
    for case in 0..5 {
        let mut b = proof::read("bodies.json");
        match case {
            0 => {
                b["spans"].as_array_mut().unwrap().pop();
            }
            1 => {
                b["spans"][1]["body"] = json!(
                    "skillModList:NewMod(\"PhysicalMin\", \"BASE\", 1, \"Changed\", ModFlag.Attack)"
                )
            }
            2 => b["spans"][10]["body"] = json!("return true"),
            3 => {
                b["spans"][16]["body"] =
                    json!("skills[\"MinionMeleeBow\"] = { preDamageFunc=function() end }")
            }
            4 => {
                b["files"].as_array_mut().unwrap().pop();
            }
            _ => unreachable!(),
        }
        assert!(proof::review_bodies(&b).is_err());
    }
}
#[test]
fn callbacks_mappings_and_spell_identity_require_new_review() {
    let _lane = lane();
    let actual = proof::fixture()["recipes"]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "MinionMeleeBow")
        .unwrap();
    for case in 0..5 {
        let mut r = actual.clone();
        match case {
            0 => r["callbacks"] = json!({"preDamageFunc":{"source":"callback"}}),
            1 => r["resolved"][0]["mapping"]["callbacks"] = json!([{"source":"new factory"}]),
            2 => {
                r["resolved"][0]["mapping"]["value"] = json!({"skillData":{"preDamageFunc":"new"}})
            }
            3 => r["metadata"]["skillTypes"]["[2]"] = json!(true),
            4 => {
                r["resolved"].as_array_mut().unwrap().pop();
            }
            _ => unreachable!(),
        }
        assert!(proof::review_recipe(&r).is_err());
    }
}
#[test]
fn actual_pass_cfg_actor_and_source_associations_are_required() {
    let _lane = lane();
    let sources = proof::fixture();
    for case in 0..5 {
        let mut report = proof::report().clone();
        let call = physical(&mut report);
        let expected = match case {
            0 => {
                call["preconversion"]["exact_pass_cfg"] = json!(false);
                "physical pass identity"
            }
            1 => {
                call["preconversion"]["selection"]["kind"] = json!("weapon2");
                "weapon1 Attack"
            }
            2 => {
                call["preconversion"]["source_occurrence"]["matches"][0]["group_index"] = json!(7);
                "actual cfg and supplying"
            }
            3 => {
                call["preconversion"]["cfg"]["fields"]["flags"] = json!(proof::FLAGS | 2);
                "actual cfg and supplying"
            }
            4 => {
                call["observed_at"] = json!(0);
                "base call location"
            }
            _ => unreachable!(),
        };
        refuses(proof::review_selection(sources, &report), expected);
    }
}
#[test]
fn parent_attack_factory_keeps_original_flag_provenance() {
    let _lane = lane();
    let report = proof::report();
    for case in 0..3 {
        let mut sources = proof::fixture().clone();
        let expected = match case {
            0 => {
                sources["gates"][0]["flag_calls"][0]["cfg"] =
                    json!({"present":true,"fields":{"flags":proof::FLAGS}});
                "original nil cfg Flag"
            }
            1 => {
                sources["gates"][0]["flag_calls"][0]["original_flag_call_observed"] = json!(false);
                "flag invocation"
            }
            2 => {
                sources["gates"][0]["parent_weapon"][0]["original_branch_entered"] = json!(true);
                "parent-weapon gate"
            }
            _ => unreachable!(),
        };
        refuses(proof::review_selection(&sources, report), expected);
    }
}
#[test]
fn finite_filter_projection_cannot_reauthenticate_changed_corroboration() {
    let _lane = lane();
    let mut report = proof::report().clone();
    report["cases"][0]["name"] = json!("changed provenance");
    proof::review_selection(proof::fixture(), &report).unwrap();
    refuses(
        proof::reconstruct(proof::fixture(), &report, &proof::read("bodies.json")),
        "independently authenticated",
    );
}
#[test]
#[ignore = "explicit offline retention; no VM execution"]
fn retain_selected_action_factories() {
    let _lane = lane();
    proof::retain();
}
