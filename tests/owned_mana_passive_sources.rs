#[path = "support/owned_mana_passive_sources.rs"]
mod proof;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn node() -> Value {
    proof::fixture()["off"]["node_construction"][0]["node"].clone()
}

#[test]
fn selected_passive_record_transport_reconstructs_without_lua() {
    proof::check().unwrap();
    let result = proof::read("passive-review.json");
    let observed = &result["observed"];
    assert_eq!(observed["selected_nodes"], 57);
    assert_eq!(observed["distinct_node_bodies"], 55);
    assert_eq!(observed["node_returns"], 2394);
    assert_eq!(observed["ordered_records_returned"], 2394);
    assert_eq!(observed["empty_node_returns"], 504);
    assert_eq!(observed["node_lists"], 42);
    assert_eq!(
        observed["generated_boundaries"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        observed["empty_parsed_declarations"]
            .as_array()
            .unwrap()
            .len(),
        12
    );
    assert_eq!(
        result["mana_adjustment_names"].as_array().unwrap().len(),
        15
    );
    assert_eq!(result["scope"]["native_partial_owners_closed"], 0);
    for field in [
        "native_coverage_authorized",
        "arbitrary_parsing_covered",
        "empty_parse_means_no_mechanics",
        "generated_skill_delivery_covered",
        "support_or_later_recipient_delivery_covered",
        "numerical_scaling_or_order_covered",
        "node_ids_assigned_to_anonymous_returns",
        "current_native_package_changed",
    ] {
        assert_eq!(result["scope"][field], false, "{field}");
    }
}

#[test]
fn all_fifteen_adjustment_names_are_refused_even_nested_zero_or_inactive() {
    let names = [
        "ManaConvertToEnergyShield",
        "ManaConvertToArmour",
        "ManaConvertToEvasion",
        "ExtraMana",
        "ManaTotal",
        "ArmourConvertToMana",
        "ArmourGainAsMana",
        "EvasionConvertToMana",
        "EvasionGainAsMana",
        "EnergyShieldConvertToMana",
        "EnergyShieldGainAsMana",
        "LifeConvertToMana",
        "LifeGainAsMana",
        "WardConvertToMana",
        "WardGainAsMana",
    ];
    for name in names {
        for nested in [false, true] {
            let mut n = node();
            let body = json!({"name":name,"type":"BASE","value":0,"flags":0,"keywordFlags":0,"source":"Tree:6077",
            "_positions":[{"index":1,"value":{"type":"Condition","var":"UnobservedCondition","neg":true}}]});
            n["records"][0]["body"] = if nested {
                json!({"name":"MinionModifier","type":"LIST","value":{"mod":body}})
            } else {
                body
            };
            assert!(proof::review_node(&n).is_err(), "{name}, nested={nested}");
        }
    }
}

#[test]
fn replacement_scaling_explosion_callbacks_and_weapon_routing_stay_open() {
    for name in [
        "PassiveSkillHasNoEffect",
        "AllocatedPassiveSkillHasNoEffect",
        "PassiveSkillEffect",
        "PassiveSkillHasOtherEffect",
        "NodeModifier",
        "CanExplode",
        "JewelSmallPassiveSkillEffect",
        "JewelNotablePassiveSkillEffect",
        "SmallPassiveSkillEffect",
        "ExtraJewelFunc",
    ] {
        let mut n = node();
        n["records"][0]["body"] = json!({"name":name,"type":"FLAG","value":false});
        assert!(proof::review_node(&n).is_err(), "{name}");
    }
    for mutation in 0..4 {
        let mut n = node();
        match mutation {
            0 => n["callbacks"] = json!({"callback":{"kind":"callback_reference"}}),
            1 => n["fields"]["allocMode"] = json!(1),
            2 => n["records"][0]["body"]["parsedLine"] = json!("unreviewed jewel"),
            3 => {
                n["records"][0]["body"]["_positions"] =
                    json!([{"index":1,"value":{"type":"Condition","var":"WeaponSet1"}}])
            }
            _ => unreachable!(),
        }
        assert!(proof::review_node(&n).is_err(), "mutation {mutation}");
    }
}

#[test]
fn anonymous_return_counts_do_not_invent_ids_or_erase_duplicate_bodies() {
    for mutation in 0..5 {
        let mut run = proof::fixture()["off"].clone();
        match mutation {
            0 => {
                run["node_construction"].as_array_mut().unwrap().pop();
            }
            1 => {
                let original = run["node_construction"][0].clone();
                let other = run["node_construction"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| {
                        r["mode"] == original["mode"]
                            && r["include_keystone_mods"] == original["include_keystone_mods"]
                            && r["node"] != original["node"]
                    })
                    .unwrap();
                other["node"] = original["node"].clone();
                other["output_records"] = original["output_records"].clone();
            }
            2 => run["node_construction"][0]["include_keystone_mods"] = Value::Null,
            3 => run["node_construction"][0]["inc_small_passive_skill"] = json!(1),
            4 => run["node_construction"][0]["output_records"][0]["body"]["value"] = json!(0),
            _ => unreachable!(),
        }
        assert!(
            proof::review_run(&run, &proof::fixture()["memberships"]).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn list_union_checks_record_multiplicity_and_retains_within_name_order() {
    for mutation in 0..4 {
        let mut run = proof::fixture()["off"].clone();
        match mutation {
            0 => {
                run["node_lists"][0]["output_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            1 => {
                run["node_lists"][0]["output_records"][1]["body"] =
                    run["node_lists"][0]["output_records"][0]["body"].clone()
            }
            2 => run["node_lists"][0]["nodes"][1] = run["node_lists"][0]["nodes"][0].clone(),
            3 => run["node_lists"][0]["sockets"]["radius_jewels"] = json!([{"provider":16}]),
            _ => unreachable!(),
        }
        assert!(
            proof::review_run(&run, &proof::fixture()["memberships"]).is_err(),
            "mutation {mutation}"
        );
    }
    let mut input = proof::fixture().clone();
    let records = input["off"]["node_lists"][0]["output_records"]
        .as_array_mut()
        .unwrap();
    let indices: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r["body"]["name"] == "Int")
        .map(|(n, _)| n)
        .take(2)
        .collect();
    assert_eq!(indices.len(), 2);
    let a = records[indices[0]]["body"].clone();
    assert_ne!(a, records[indices[1]]["body"]);
    records[indices[0]]["body"] = records[indices[1]]["body"].clone();
    records[indices[1]]["body"] = a;
    let changed = proof::review_run(&input["off"], &input["memberships"]).unwrap();
    let original = proof::review_run(&proof::fixture()["off"], &input["memberships"]).unwrap();
    assert_ne!(
        changed["list_order_receipts"],
        original["list_order_receipts"]
    );
    assert!(proof::reconstruct(&input, &proof::read("passive-bodies.json")).is_err());
}

#[test]
fn complete_reset_and_transport_bodies_cannot_be_reauthored() {
    for mutation in 0..4 {
        let mut bodies = proof::read("passive-bodies.json");
        match mutation {
            0 => {
                let text = bodies["spans"][0]["body"]
                    .as_str()
                    .unwrap()
                    .replace("list.parent = false", "list.parent = oldParent");
                bodies["spans"][0]["sha256"] =
                    json!(format!("{:x}", Sha256::digest(text.as_bytes())));
                bodies["spans"][0]["body"] = json!(text);
            }
            1 => {
                bodies["spans"].as_array_mut().unwrap().pop();
            }
            2 => bodies["files"][1]["path"] = json!("src/Classes/Invented.lua"),
            3 => bodies["scope"]["numerical_scaling_or_order_covered"] = json!(true),
            _ => unreachable!(),
        }
        assert!(
            proof::review_bodies(&bodies).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn changed_generated_roots_empty_text_and_supplied_metadata_refuse() {
    let mut run = proof::fixture()["off"].clone();
    for env in run["environments"].as_array_mut().unwrap() {
        let generated = env["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["key"] == 13289)
            .unwrap();
        generated["value"]["records"][0]["body"]["value"]["skillId"] =
            json!("unreviewed-generated-skill");
    }
    assert!(proof::review_run(&run, &proof::fixture()["memberships"]).is_err());
    let mut changed = proof::fixture().clone();
    changed["off"]["environments"][0]["nodes"][0]["value"]["description"] = json!({});
    assert!(proof::reconstruct(&changed, &proof::read("passive-bodies.json")).is_err());
    let mut changed = proof::fixture().clone();
    changed["radius_report_sha256"] = json!("changed");
    assert!(proof::reconstruct(&changed, &proof::read("passive-bodies.json")).is_err());
}

#[test]
#[ignore = "author finite passive review from already authenticated retained evidence"]
fn retain_selected_passive_source_review() {
    proof::retain().unwrap();
}
