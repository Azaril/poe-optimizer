//! Offline authoring controls; this target makes no native admission claim.
#[path = "support/owned_outgoing_critical.rs"]
mod family;
use serde_json::json;
use sha2::{Digest, Sha256};

#[test]
fn complete_source_authority_and_typed_unpublished_programs_are_retained() {
    family::check_authored();
}

#[test]
fn constructed_census_rejects_nested_zero_inactive_and_duplicate_suppliers() {
    let original = family::construction(0);
    family::review_construction(&original, 0).unwrap();
    for name in [
        "CritChance",
        "CritMultiplier",
        "NeverCrit",
        "Condition:ApplyCriticalWeakness",
    ] {
        let mut changed = original.clone();
        changed["configuration"][0]["enemy_records"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "position":22,"body":{"name":"MinionModifier","type":"LIST","value":{
                    "mod":{"name":name,"type":"BASE","value":0,"_positions":[
                        {"index":1,"value":{"type":"Condition","var":"NeverActive"}}
                    ]}
                }}
            }));
        assert!(family::review_construction(&changed, 0).is_err(), "{name}");
    }
    let mut changed = original.clone();
    let record = changed["configuration"][0]["enemy_records"][4].clone();
    changed["configuration"][0]["enemy_records"]
        .as_array_mut()
        .unwrap()
        .push(record);
    assert!(family::review_construction(&changed, 0).is_err());
}

#[test]
fn inactive_critical_weakness_remains_a_real_potential_supplier() {
    let original = family::construction(0);
    for operation in 0..4 {
        let mut changed = original.clone();
        let rows = changed["configuration"][0]["enemy_records"]
            .as_array_mut()
            .unwrap();
        match operation {
            0 => {
                rows.remove(4);
            }
            1 => rows[4]["body"]["value"] = json!(0),
            2 => rows[4]["body"]["_positions"] = json!([]),
            _ => rows[4]["body"]["_positions"][0]["value"]["var"] = json!("DifferentCondition"),
        }
        assert!(family::review_construction(&changed, 0).is_err());
    }
    for operation in 0..4 {
        let mut changed = original.clone();
        let call = changed["callback_invocations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|call| call["var"] == "enemyCriticalWeaknessStacks")
            .unwrap();
        match operation {
            0 => call["argument_fields"]["val"] = json!(0),
            1 => call["after"]["enemy_records"][4]["body"]["value"] = json!(0),
            2 => call["after"]["player_records"] = json!([]),
            _ => call["body"]["path"] = json!("other.lua"),
        }
        assert!(family::review_construction(&changed, 0).is_err());
    }
}

#[test]
fn declared_input_absence_is_distinct_from_false_zero_and_ui_defaults() {
    let original = family::construction(0);
    for (field, name, value) in [
        ("input", "conditionEnemyCriticalWeakness", json!(false)),
        ("input", "enemyCriticalWeaknessStacks", json!(0)),
        ("placeholder", "enemyCriticalWeaknessStacks", json!(0)),
        (
            "default_state",
            "conditionEnemyCriticalWeakness",
            json!(true),
        ),
        ("default_state", "enemyCriticalWeaknessStacks", json!(20)),
    ] {
        let mut changed = original.clone();
        changed["configuration"][0][field][name] = value;
        assert!(family::review_construction(&changed, 0).is_err());
    }
    let mut callback = original;
    callback["configuration_catalogue"][593]["apply"]["path"] = json!("other.lua");
    assert!(family::review_construction(&callback, 0).is_err());
}

#[test]
fn source_claims_cannot_be_resealed_from_neutral_observations() {
    let evidence = family::read("source-evidence.json");
    let domain = family::read("source-domain.json");
    let vectors = family::read("source-vectors.json");
    for mutation in 0..5 {
        let mut changed = domain.clone();
        match mutation {
            0 => changed["anomalous_more"]["compatibility_accepted"] = json!(true),
            1 => changed["limits"]["complete_source_supplier_census"] = json!(true),
            2 => changed["known_positive_supplier"]["condition_value_proven"] = json!(true),
            3 => changed["constructed_reports"][0]["records"][0]["body"]["value"] = json!(0),
            _ => changed["baseline"]["bonus_terms"] = json!([100]),
        }
        assert!(family::authenticate_source(&evidence, &changed, &vectors).is_err());
    }
    let mut changed = evidence.clone();
    changed["spans"][11]["body"] = json!("ordinary More dispatch");
    changed["spans"][11]["sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(changed["spans"][11]["body"].as_str().unwrap().as_bytes())
    ));
    assert!(family::authenticate_source(&changed, &domain, &vectors).is_err());
}

#[test]
fn baseline_pair_cannot_reauthor_the_original_owner_or_added_source_constants() {
    let original = family::read("baseline.json");
    family::review_baseline(&original).unwrap();
    for operation in 0..6 {
        let mut changed = original.clone();
        match operation {
            0 => {
                changed["before"]["nodes"][0]["id"] = json!("coherently-changed");
                changed["after"]["nodes"][0]["id"] = json!("coherently-changed");
            }
            1 => changed["owner"] = json!({"kind":"definition"}),
            2 => changed["closure_unchanged"] = json!({"kind":"complete"}),
            3 => {
                let node = changed["after"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap();
                node["expression"]["value"]["value"]["value"] = json!(99);
            }
            4 => {
                let effect = changed["after"]["effects"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap();
                effect["effect"]["entity"] = json!("enemy");
            }
            _ => {
                let effect = changed["after"]["effects"]
                    .as_array()
                    .unwrap()
                    .last()
                    .unwrap()
                    .clone();
                changed["after"]["effects"]
                    .as_array_mut()
                    .unwrap()
                    .push(effect);
            }
        }
        let expected = if operation < 3 {
            "independent original baseline authority"
        } else {
            "exact baseline source constants and contribution append"
        };
        assert_eq!(family::review_baseline(&changed).unwrap_err(), expected);
    }
}

#[test]
fn reserved_descriptors_and_binding_roles_cannot_be_reassigned() {
    let definitions = family::read("definitions.json");
    let bindings = family::read("bindings.json");
    family::review_definitions(&definitions, &bindings).unwrap();
    for operation in 0..7 {
        let mut changed = definitions.clone();
        let mut roles = bindings.clone();
        match operation {
            0 => changed[1] = changed[0].clone(),
            1 => {
                changed[0]["value"]["id"]["key"] = json!("def.00000000000033a4");
                roles["definitions"]["chance"] = changed[0]["value"]["id"].clone();
            }
            2 => {
                changed[0]["value"]["schema"]["value"]["value"]["value"]["unit"]["key"] =
                    json!("def.0000000000000001")
            }
            3 => changed[0]["value"]["schema"]["value"]["targets"] = json!(["enemy"]),
            4 => {
                changed[6]["value"]["schema"]["value"]["value"] =
                    changed[0]["value"]["schema"]["value"]["value"].clone()
            }
            5 => roles["definitions"]["actor_cap"] = roles["definitions"]["actor_bonus"].clone(),
            _ => roles["definitions"]["extra"] = roles["definitions"]["chance"].clone(),
        }
        let expected = if operation < 5 {
            "exact reserved descriptor authority"
        } else {
            "exact reserved binding correspondence"
        };
        assert_eq!(
            family::review_definitions(&changed, &roles).unwrap_err(),
            expected
        );
    }
}

#[test]
fn equal_critical_values_do_not_erase_recipient_or_source_identity() {
    let evidence = family::read("source-evidence.json");
    let domain = family::read("source-domain.json");
    let original = family::read("source-vectors.json");
    for mutation in 0..5 {
        let mut vectors = original.clone();
        match mutation {
            0 => vectors["rows"][0]["actor_profile"] = json!("SandDjinn"),
            1 => vectors["rows"][0]["source_occurrence"]["matches"][0]["group_index"] = json!(5),
            2 => vectors["rows"][0]["output"]["AccuracyHitChance"] = json!(80),
            3 => vectors["rows"][0]["output"]["CritChance"] = json!(15),
            _ => {
                vectors["rows"].as_array_mut().unwrap().pop();
            }
        }
        assert!(family::authenticate_source(&evidence, &domain, &vectors).is_err());
    }
}
