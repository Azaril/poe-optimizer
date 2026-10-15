#[path = "support/owned_mana_equipment_sources.rs"]
mod proof;

use serde_json::{Value, json};

fn item() -> Value {
    proof::fixture()["recipe"]["items"][0].clone()
}

#[test]
fn selected_equipment_construction_reconstructs_without_lua() {
    proof::check().unwrap();
    let report = proof::read("equipment-review.json");
    assert_eq!(report["observed"]["item_records"], 8);
    assert_eq!(report["observed"]["equipment_uses"], 9);
    assert_eq!(report["observed"]["bonded_active_recipe_lists"], 18);
    assert_eq!(report["observed"]["parsed_lines"], 13);
    assert_eq!(
        report["observed"]["per_source02_report"]["direct_list_joins"],
        126
    );
    assert_eq!(
        report["observed"]["per_source02_report"]["base_and_buff_joins"],
        189
    );
    assert_eq!(
        report["observed"]["per_radius_observation"]["selected_uses"],
        189
    );
    assert_eq!(
        report["generated_skill_boundaries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        report["mana_adjustment_names"].as_array().unwrap().len(),
        15
    );
    assert_eq!(report["scope"]["native_partial_owners_closed"], 0);
    for field in [
        "source02_slot_specific_active_lists_observed",
        "item_to_store_delivery_covered",
        "focus_amulet_copy_replacement_covered",
        "inherited_store_covered",
        "generic_parsing_or_numeric_order_covered",
        "generated_support_delivery_covered",
        "native_coverage_authorized",
        "current_native_package_changed",
    ] {
        assert_eq!(report["scope"][field], false, "{field}");
    }
}

#[test]
fn all_adjustment_names_refuse_zero_inactive_nested_and_bonded_suppliers() {
    for name in [
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
    ] {
        for family in ["base", "bonded", "parsed"] {
            let mut v = item();
            let body = json!({"name":"MinionModifier","type":"LIST","value":{"mod":{
                "name":name,"type":"BASE","value":0,"[1]":{"type":"Condition","var":"Unobserved","neg":true}}}});
            match family {
                "base" => v["base_records"]["value"]["[1]"] = body,
                "bonded" => v["uses"][0]["bonded_records"]["value"]["[1]"] = body,
                "parsed" => {
                    v["parsed_lines"]["runeModLines"]["value"]["[1]"] =
                        json!({"disabled":true,"modList":{"[1]":body}})
                }
                _ => unreachable!(),
            }
            let error = proof::review_item(&v).unwrap_err();
            assert!(
                error.contains("unreviewed Mana source record"),
                "{name}/{family}: {error}"
            );
        }
    }
}

#[test]
fn range_variant_rune_and_bonded_metadata_are_not_discarded() {
    for mutation in 0..12 {
        let mut v = item();
        match mutation {
            0 => v["source"]["children"][0]["attributes"]["range"] = json!("0.1"),
            1 => v["loaded_metadata"]["variant"] = json!(1),
            2 => {
                v["parsed_lines"]["explicitModLines"]["value"]["[1]"]["extra"] =
                    json!("unparsed source")
            }
            3 => v["base_records"]["value"]["parent"] = json!({"name":"UnknownStore"}),
            4 => v["base_records"]["aliases"] = json!([{"path":"unseen"}]),
            5 => v["uses"][0]["bonded_records"]["value"]["[1]"]["value"] = json!(0),
            6 => v["uses"][0]["slot_number"] = json!(2),
            7 => {
                v["parsed_lines"]["runeModLines"]["value"] = json!({"[1]":{"line":"unknown rune"}})
            }
            8 => v["base_records"]["value"]["multipliers"] = json!({"Unknown":0}),
            9 => {
                v["buff_records"].as_object_mut().unwrap().remove("value");
            }
            10 => v["base_records"]["value"] = Value::Null,
            11 => v["uses"][0]["records"]["value"] = Value::Null,
            _ => unreachable!(),
        }
        assert!(proof::review_item(&v).is_err(), "mutation {mutation}");
    }
}

#[test]
fn encoding_preserves_nested_full_records_and_refuses_holes_aliases_and_mixed_keys() {
    assert_eq!(proof::convert_plain(&json!({"name":"Test","value":{"[1]":{"unknown":false}},"[1]":{"type":"Condition","var":"X"}})).unwrap(),
        json!({"name":"Test","value":{"_positions":[{"index":1,"value":{"unknown":false}}]},"_positions":[{"index":1,"value":{"type":"Condition","var":"X"}}]}));
    for v in [
        json!({"[2]":0}),
        json!({"[01]":0}),
        json!({"[x]":0}),
        json!({"[0]":0}),
        json!({"_positions":[]}),
        json!({"value":{"kind":"callback_reference"}}),
        json!({"kind":"table_reference"}),
    ] {
        assert!(proof::convert_plain(&v).is_err(), "{v}");
    }
}

#[test]
fn actual_full_item_bodies_order_duplicates_and_empty_slot_views_are_checked() {
    let f = proof::fixture();
    for mutation in 0..13 {
        let mut env = f["off"][0].clone();
        match mutation {
            0 => {
                env["items"].as_array_mut().unwrap().pop();
            }
            1 => env["items"][1] = env["items"][0].clone(),
            2 => env["items"][0]["value"]["fields"]["id"] = json!(26),
            3 => env["items"][0]["value"]["base_records"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            4 => {
                let r = env["items"][0]["value"]["records"][0].clone();
                env["items"][0]["value"]["records"]
                    .as_array_mut()
                    .unwrap()
                    .push(r);
            }
            5 => env["items"][0]["value"]["records"][0]["body"]["sourceSlot"] = json!("Ring 2"),
            6 => {
                env["items"][6]["value"]["records"] =
                    json!([{"position":1,"body":{"name":"Life","type":"BASE","value":10}}])
            }
            7 => {
                env["items"][2]["value"]["rune_fields"]["_positions"][0]["value"] =
                    json!("UnknownRune")
            }
            8 => {
                env["items"][0]["value"]["buff_records"] =
                    json!([{"position":1,"body":{"name":"ExtraMana","type":"BASE","value":0}}])
            }
            9 => env["items"][0]["value"]["fields"]["raw"] = json!("changed raw item"),
            10 => env["items"][0]["value"]["socketed_items"]["hidden"] = json!(true),
            11 => env["items"][0]["value"]["jewel_data"] = Value::Null,
            12 => {
                env["items"][8]["value"]["base_records"][0]["body"]["value"]["skillId"] =
                    json!("DifferentGeneratedSkill")
            }
            _ => unreachable!(),
        }
        assert!(
            proof::review_environment(&env, f).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn repeated_equipment_and_exact_provider_selection_cannot_be_collapsed() {
    let f = proof::fixture();
    for mutation in 0..5 {
        let mut changed = f.clone();
        match mutation {
            0 => {
                changed["memberships"][5]["owned_use"]["id"] =
                    changed["memberships"][3]["owned_use"]["id"].clone()
            }
            1 => changed["memberships"][0]["preset"]["lineage"] = json!("foreign"),
            2 => changed["memberships"][0]["source_item_id"] = json!("26"),
            3 => changed["memberships"][0]["owned_item"]["id"]["lineage"] = json!("foreign"),
            4 => changed["memberships"][0]["use_recipe_sha256"] = json!("wrong"),
            _ => unreachable!(),
        }
        assert!(
            proof::review_environment(&changed["off"][0], &changed).is_err(),
            "membership {mutation}"
        );
    }
    for mutation in 0..5 {
        let mut trace = f["radius"][0].clone();
        let rows = trace["slot_candidates"].as_array_mut().unwrap();
        let index = rows
            .iter()
            .position(|r| r["resolved_item"]["present"] == true)
            .unwrap();
        match mutation {
            0 => rows[index]["resolved_item"]["fields"]["id"] = json!(0),
            1 => rows[index]["resolved_item"] = json!({"present":false}),
            2 => rows[index]["slot_fields"]["selItemId"] = json!(0),
            3 => rows[index]["invocation_id"] = json!(999),
            4 => rows[index]["slot_name"] = json!("Ring 3"),
            _ => unreachable!(),
        }
        assert!(
            proof::review_radius(&trace, f).is_err(),
            "candidate {mutation}"
        );
    }
}

#[test]
fn complete_source_bodies_and_coherent_reauthoring_refuse() {
    for mutation in 0..4 {
        let mut bodies = proof::read("equipment-bodies.json");
        match mutation {
            0 => bodies["spans"][0]["body"] = json!("ParseRaw only"),
            1 => bodies["files"][1]["sha256"] = json!("changed source"),
            2 => {
                bodies["spans"].as_array_mut().unwrap().remove(2);
            }
            3 => bodies["scope"]["native_coverage_authorized"] = json!(true),
            _ => unreachable!(),
        };
        assert!(proof::review_bodies(&bodies).is_err(), "body {mutation}");
    }
    let mut f = proof::fixture().clone();
    f["recipe"]["items"][0]["original_parsed_xml"]["[2]"]["attrib"]["range"] = json!("0.1");
    use sha2::{Digest, Sha256};
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&f["recipe"]["items"][0]).unwrap())
    );
    for m in f["memberships"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .filter(|m| m["source_item_id"] == "19")
    {
        m["recipe_sha256"] = json!(hash);
    }
    let error = proof::reconstruct(&f, &proof::read("equipment-bodies.json")).unwrap_err();
    assert!(error.contains("independently authenticated"), "{error}");
}

#[test]
#[ignore = "explicit offline authoring; no Lua or runtime package changes"]
fn retain_selected_equipment_construction_review() {
    proof::retain();
}
