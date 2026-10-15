#[path = "support/owned_mana_configuration_sources.rs"]
mod proof;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

fn fixture() -> &'static Value {
    static SOURCE: OnceLock<Value> = OnceLock::new();
    SOURCE.get_or_init(|| proof::fixture().unwrap())
}

#[test]
fn retained_configuration_body_domains_reconstruct_without_pob() {
    proof::check().unwrap();
    let retained = |name: &str, expected_size, expected_hash| {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(proof::DIR)
            .join(name);
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(bytes.len(), expected_size);
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected_hash);
        serde_json::from_slice::<Value>(&bytes).unwrap()
    };
    let original = retained(
        "configuration-review.json",
        150_147,
        "d8f7abf2202ace98318d9c3c80be31ea77c3d9d17a307a174e866fc9c44fa9d5",
    );
    let report = retained(
        "configuration-reward-review.json",
        183_350,
        "929e8805a7e1ec72ef9d022354309fde61b1e85c12032173fda6550e23ddcccf",
    );
    // The immutable constructor evidence still authenticates historical input
    // publications. The separate reconciliation changes only its quest proof.
    let mut inverse = report.clone();
    assert!(
        inverse
            .as_object_mut()
            .unwrap()
            .remove("reward_reconciliation")
            .is_some()
    );
    inverse["remaining"] = original["remaining"].clone();
    for mode in ["off", "on"] {
        assert_eq!(report[mode]["phases"][0]["invoked"], 62);
        assert_eq!(report[mode]["phases"][1]["invoked"], 63);
        assert_eq!(report[mode]["scope"]["quest_parser_closure"], false);
        assert_eq!(
            report[mode]["scope"]["selected_quest_construction_reconciled"],
            true
        );
        for phase in 0..2 {
            assert_eq!(
                report[mode]["phases"][phase]["quest_parser_boundaries"],
                json!([])
            );
            assert_eq!(
                report[mode]["phases"][phase]["reconciled_quests"]
                    .as_array()
                    .unwrap()
                    .len(),
                17
            );
        }
        assert_eq!(report[mode]["scope"]["native_coverage_authorized"], false);
        assert_eq!(
            report[mode]["scope"]["later_direct_configuration_consumers_covered"],
            false
        );
        assert_eq!(
            inverse[mode]["scope"]
                .as_object_mut()
                .unwrap()
                .remove("selected_quest_construction_reconciled"),
            Some(json!(true))
        );
        for phase in 0..2 {
            assert_eq!(
                original[mode]["phases"][phase]["quest_parser_boundaries"]
                    .as_array()
                    .unwrap()
                    .len(),
                17
            );
            assert!(
                inverse[mode]["phases"][phase]
                    .as_object_mut()
                    .unwrap()
                    .remove("reconciled_quests")
                    .is_some()
            );
            inverse[mode]["phases"][phase]["quest_parser_boundaries"] =
                original[mode]["phases"][phase]["quest_parser_boundaries"].clone();
        }
    }
    assert_eq!(report["saved_input_dispositions_changed"], false);
    assert_eq!(inverse, original);
}

fn quest_call(key: &str, phase: usize) -> Value {
    fixture()["off"]["calls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["var"] == key)
        .nth(phase)
        .unwrap()
        .clone()
}

#[test]
fn finite_quest_defaults_and_saved_native_occurrences_remain_separate() {
    let rewards = proof::reward_fixture().unwrap();
    let reviewed = proof::review_rewards(rewards).unwrap();
    assert_eq!(
        reviewed["selected_native_rewards"]
            .as_array()
            .unwrap()
            .len(),
        17
    );
    assert_eq!(reviewed["global_reward_completion"]["kind"], "pending");
    assert_eq!(reviewed["choice_completion"]["kind"], "pending");
    assert_eq!(reviewed["global_quest_parser_closure"], false);
    assert_eq!(reviewed["native_coverage_authorized"], false);
    let mut totals = [0u64; 2];
    let mut empty = 0;
    for row in rewards["census"].as_array().unwrap() {
        for (phase, total) in totals.iter_mut().enumerate() {
            let call = quest_call(row["key"].as_str().unwrap(), phase);
            let receipt = proof::review_quest_call(&call, phase, rewards).unwrap();
            *total += receipt["player_records"].as_u64().unwrap();
            assert_eq!(receipt["enemy_records"], 0);
            if phase == 0 && call["argument_fields"]["val"] == "None" {
                empty += 1;
                assert_eq!(receipt["player_records"], 0);
            }
        }
    }
    assert_eq!(totals, [9, 22]);
    assert_eq!(empty, 8);
    // False skips check dispatch; it is not an invoked-empty finite matrix row.
    let mut call = quest_call("questAct 1ClearfellBeira", 0);
    call["argument_fields"]["val"] = json!(false);
    assert_eq!(
        proof::review_quest_call(&call, 0, rewards).unwrap_err(),
        "quest argument outside finite invoked domain"
    );
}

#[test]
fn quest_record_transport_keeps_tags_positions_order_and_multiplicity() {
    let rewards = proof::reward_fixture().unwrap();
    let original = quest_call("questAct 4Eye of HinekoraTribal Medicine", 1);
    let before = original["before"]["player_records"]
        .as_array()
        .unwrap()
        .len();
    for mutation in 0..10 {
        let mut call = original.clone();
        let records = call["after"]["player_records"].as_array_mut().unwrap();
        match mutation {
            0 => records[before]["body"]["value"] = json!(0),
            1 => records[before]["body"]["_positions"][0]["value"]["type"] = json!("Condition"),
            2 => records[before]["body"]["_positions"][0]["index"] = json!(2),
            3 => records[before]["body"]["keywordFlags"] = json!(1),
            4 => records[before]["body"]["source"] = json!("Quest:another source"),
            5 => records[before]["position"] = json!(before),
            6 => {
                records.remove(before);
            }
            7 => records.push(records[before].clone()),
            8 => records.swap(before, before + 1),
            9 => {
                records[before]["body"]["unreviewed"] =
                    json!({"name":"ExtraMana","type":"BASE","value":0})
            }
            _ => unreachable!(),
        }
        assert!(
            proof::review_quest_call(&call, 1, rewards).is_err(),
            "mutation {mutation}"
        );
    }
    let mut call = original.clone();
    call["before"]["active_set"] = json!(2);
    call["after"]["active_set"] = json!(2);
    assert_eq!(
        proof::review_quest_call(&call, 1, rewards).unwrap_err(),
        "quest callback belongs to a different ConfigSet"
    );
    let mut call = original;
    call["body"]["path"] = json!("@different/ConfigOptions.lua");
    assert_eq!(
        proof::review_quest_call(&call, 1, rewards).unwrap_err(),
        "quest callback identity changed"
    );
}

#[test]
fn unknown_quest_strings_and_coherently_changed_matrix_are_not_authority() {
    let original = proof::reward_fixture().unwrap();
    let mut call = quest_call("questAct 2Valley of the TitansMedallion", 1);
    call["argument_fields"]["val"] = json!("+123 to maximum Life");
    assert_eq!(
        proof::review_quest_call(&call, 1, original).unwrap_err(),
        "quest argument outside finite invoked domain"
    );
    let mut rewards = original.clone();
    let at = rewards["matrix"]
        .as_array()
        .unwrap()
        .iter()
        .position(|r| r["key"] == call["var"] && r["value"] != "None")
        .unwrap();
    rewards["matrix"][at]["value"] = call["argument_fields"]["val"].clone();
    assert_eq!(
        proof::review_quest_call(&call, 1, &rewards).unwrap_err(),
        "quest matrix/metadata is not independent acquisition authority"
    );
    rewards["authority"]["files"]["source"]["sha256"] = json!("coherently-repinned-matrix");
    assert_eq!(
        proof::review_quest_call(&call, 1, &rewards).unwrap_err(),
        "quest matrix/metadata is not independent acquisition authority"
    );
    assert!(proof::review_rewards(&rewards).is_err());
}

#[test]
fn current_reward_owner_mapping_and_selected_origin_are_independently_sealed() {
    let original = proof::reward_fixture().unwrap();
    for mutation in 0..11 {
        let mut rewards = original.clone();
        match mutation {
            0 => {
                rewards["native"][0]["owner"]["programs"]["members"][0]["effects"][0]["effect"]["entity"] =
                    json!("enemy")
            }
            1 => {
                rewards["native"][0]["owner"]["programs"]["members"][0]["effects"][0]["when"] =
                    json!("disabled")
            }
            2 => rewards["native"][0]["owner"]["programs"]["closure"]["kind"] = json!("partial"),
            3 => {
                rewards["native"].as_array_mut().unwrap().remove(0);
            }
            4 => {
                let row = rewards["native"][0].clone();
                rewards["native"].as_array_mut().unwrap().push(row);
            }
            5 => {
                rewards["native"][0]["occurrence"]["id"]["lineage"] =
                    json!("00000000000000000000000000000000")
            }
            6 => {
                let row = rewards["mapping"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| {
                        r["source"]["value"]["key"]["value"] == "questAct 1ClearfellBeira"
                            && r["source"]["value"]["source"] == "input"
                    })
                    .unwrap();
                row["outcome"]["value"]["target"]["value"]["value"]["key"] =
                    json!("def.0000000000000028");
            }
            7 => rewards["selected_config_origin"]["source"]["ordinal"] = json!(422),
            8 => {
                rewards["selected_config_origin"]["links"][0]["value"]["local"] =
                    json!("0000000000000001")
            }
            9 => rewards["census"][0]["source_index"] = json!(2),
            10 => rewards["global_reward_completion"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        // First exercise the target mutation without a guaranteed bad digest.
        let error = proof::review_rewards(&rewards).unwrap_err();
        let expected = match mutation {
            0..=2 => "selected native reward whole owner/recipient differs",
            3 | 4 => "selected native reward missing or duplicated",
            6 => "quest mapping selects different reward",
            9 => "quest full source record/body/order differs from accepted effect binding",
            _ => "reward reconciliation differs from independent source/publication authority",
        };
        assert_eq!(error, expected, "mutation {mutation}");
        // A separately changed compact digest does not replace that authority.
        rewards["authority"]["files"]["rules"]["sha256"] = json!("repinned-mutated-rules");
        assert!(proof::review_rewards(&rewards).is_err());
    }
}

#[test]
fn complete_catalogue_and_actual_invocations_are_required() {
    let original = &fixture()["off"];
    let bodies = proof::bodies();
    for mutation in 0..6 {
        let mut state = original.clone();
        match mutation {
            0 => {
                state["catalogue"].as_array_mut().unwrap().remove(0);
            }
            1 => state["catalogue"][1]["index"] = json!(1),
            2 => {
                state["calls"].as_array_mut().unwrap().remove(0);
            }
            3 => state["calls"][0]["argument_fields"]["val"] = json!(0),
            4 => state["calls"][0]["body"]["first_line"] = json!(1),
            5 => {
                state["calls"][0]["body"]["path"] = json!("@unreviewed/ConfigOptions.lua");
                let row = state["catalogue"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["fields"]["var"] == "multiplierCurrentManaPercentage")
                    .unwrap();
                row["apply"]["path"] = json!("@unreviewed/ConfigOptions.lua");
            }
            _ => unreachable!(),
        }
        assert!(
            proof::review_state(&state, &bodies).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn inactive_and_nested_zero_mana_records_cannot_pass_body_exclusion() {
    let original = &fixture()["off"];
    for name in [
        "ExtraMana",
        "ManaTotal",
        "LifeGainAsMana",
        "ManaConvertToEnergyShield",
    ] {
        let mut state = original.clone();
        let records = &mut state["calls"][0]["after"]["player_records"];
        if !records.is_array() {
            *records = json!([]);
        }
        records.as_array_mut().unwrap().push(json!({"name":"MinionModifier","type":"LIST","value":{"mod":{"name":name,"type":"BASE","value":0,"_positions":[{"index":1,"value":{"type":"Condition","var":"Disabled"}}]}}}));
        let error = proof::review_state(&state, &proof::bodies()).unwrap_err();
        assert!(
            error.starts_with("unreviewed Mana source record at "),
            "{error}"
        );
    }
}

#[test]
fn constructor_chain_and_empty_custom_path_are_checked() {
    let original = &fixture()["off"];
    let bodies = proof::bodies();
    let mut state = original.clone();
    let index = state["calls"]
        .as_array()
        .unwrap()
        .iter()
        .position(|call| {
            call["before"]["player_records"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
        })
        .unwrap();
    state["calls"][index]["after"]["player_records"][0]["value"] = json!(999);
    assert_eq!(
        proof::review_state(&state, &bodies).unwrap_err(),
        "callback rewrites or removes prior modifier records"
    );

    let mut state = original.clone();
    for call in state["calls"].as_array_mut().unwrap() {
        for phase in ["before", "after"] {
            call[phase]["custom_blocks"]["_positions"][0]["value"]["text"] =
                json!("+1 to maximum Mana");
        }
    }
    for configuration in state["configuration"].as_array_mut().unwrap() {
        configuration["custom_blocks"]["_positions"][0]["value"]["text"] =
            json!("+1 to maximum Mana");
    }
    assert_eq!(
        proof::review_state(&state, &bodies).unwrap_err(),
        "custom modifier block membership or text changed"
    );
}

#[test]
fn source_defaults_and_complete_callback_body_declarations_are_bound() {
    let original = &fixture()["off"];
    let mut state = original.clone();
    let row = state["catalogue"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["fields"]["var"] == "multiplierCurrentManaPercentage")
        .unwrap();
    row["default_state"] = json!(99);
    assert_eq!(
        proof::review_state(&state, &proof::bodies()).unwrap_err(),
        "declared defaults differ from first actual construction"
    );
    for mutation in 0..5 {
        let mut bodies = proof::bodies();
        match mutation {
            0 => bodies["bodies"][0]["body"] = json!("function() end"),
            1 => bodies["bodies"][0]["declared_names"] = json!(["ExtraMana"]),
            2 => bodies["scope"]["quest_parser_closure"] = json!(true),
            3 => bodies["source"]["path"] = json!("src/Classes/ConfigTab.lua"),
            4 => bodies["source"]["sha256"] = json!("changed-with-the-archive"),
            _ => unreachable!(),
        }
        assert!(proof::review_state(original, &bodies).is_err());
    }
}

#[test]
fn source_dispatch_does_not_use_display_gates_or_numeric_truthiness() {
    for kind in ["count", "integer", "countAllowZero", "float"] {
        let row = json!({"apply":{"kind":"callback_reference"},"fields":{"var":"selected","type":kind,"ifSkill":"Unselected","ifOption":"Disabled","ifMult":"Unavailable"}});
        let state = json!({"input":{"selected":0},"placeholder":{"selected":11}});
        assert_eq!(
            proof::dispatch_argument(&row, &state).unwrap(),
            Some(if kind == "count" { json!(11) } else { json!(0) })
        );
        let state = json!({"input":{"selected":false},"placeholder":{"selected":11}});
        assert_eq!(
            proof::dispatch_argument(&row, &state).unwrap(),
            Some(json!(11))
        );
    }
    let row =
        json!({"apply":{"kind":"callback_reference"},"fields":{"var":"selected","type":"check"}});
    assert_eq!(
        proof::dispatch_argument(&row, &json!({"input":{"selected":0}})).unwrap(),
        Some(json!(true))
    );
    assert_eq!(
        proof::dispatch_argument(&row, &json!({"input":{"selected":false}})).unwrap(),
        None
    );
    assert!(proof::dispatch_argument(&row, &json!({"input":{"selected":[]}})).is_err());
}

#[test]
#[ignore = "explicit offline retained-proof authoring; no PoB execution"]
fn retain_reviewed_configuration_bodies() {
    proof::retain();
}
