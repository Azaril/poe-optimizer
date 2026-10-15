#[path = "support/owned_mana_configuration_sources.rs"]
mod proof;

use serde_json::{Value, json};
use std::sync::OnceLock;

fn fixture() -> &'static Value {
    static SOURCE: OnceLock<Value> = OnceLock::new();
    SOURCE.get_or_init(|| proof::fixture().unwrap())
}

#[test]
fn retained_configuration_body_domains_reconstruct_without_pob() {
    proof::check().unwrap();
    let report = proof::read("configuration-review.json");
    for mode in ["off", "on"] {
        assert_eq!(report[mode]["phases"][0]["invoked"], 62);
        assert_eq!(report[mode]["phases"][1]["invoked"], 63);
        assert_eq!(report[mode]["scope"]["quest_parser_closure"], false);
        assert_eq!(report[mode]["scope"]["native_coverage_authorized"], false);
        assert_eq!(
            report[mode]["scope"]["later_direct_configuration_consumers_covered"],
            false
        );
    }
    assert_eq!(report["saved_input_dispositions_changed"], false);
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
