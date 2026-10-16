//! Offline authoring controls; this target makes no native admission claim.
#[path = "support/owned_outgoing_critical.rs"]
mod family;
#[path = "support/owned_outgoing_critical_domain.rs"]
mod ordinary;
#[path = "support/owned_outgoing_critical_rounding.rs"]
mod rounding;
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

#[test]
fn selected_constructor_factory_and_destination_domains_are_authenticated() {
    ordinary::check();
}

#[test]
fn special_branch_and_condition_candidates_refuse_even_zero_false_and_nested_values() {
    let original = family::construction(0);
    for name in [
        "Condition:DiamondShrine",
        "Condition:ApplyCriticalWeakness",
        "AbyssalWastingImpliesCondition",
        "EnemyModifier",
        "ExtraAura",
        "AffectedByAuraMod",
        "SkillCritChanceIncreasedPerBlueSupport",
        "ClawCritChanceAppliesToMinions",
        "InevitableCriticalHits",
    ] {
        let mut changed = original.clone();
        changed["configuration"][0]["enemy_records"].as_array_mut().unwrap().push(json!({"position":23,"body":{"name":"MinionModifier","type":"LIST","value":{"mod":{"name":name,"type":"BASE","value":0}}}}));
        assert_eq!(
            ordinary::review_capture(&changed, 0).unwrap_err(),
            "new potential critical supplier or direct condition, including zero/false",
            "{name}"
        );
    }
    for name in [
        "DiamondShrine",
        "ApplyCriticalWeakness",
        "OnProfaneGround",
        "Elusive",
    ] {
        let mut changed = original.clone();
        changed["environments"][0]["player_store"][0]["conditions"][name] = json!(false);
        assert_eq!(
            ordinary::review_capture(&changed, 0).unwrap_err(),
            "new potential critical supplier or direct condition, including zero/false"
        );
    }
}

#[test]
fn ordinary_recipe_callbacks_basic_branches_and_foreign_recipients_cannot_change() {
    let recipes = ordinary::recipes();
    ordinary::review_recipes(&recipes).unwrap();
    let mut changed = recipes.clone();
    changed["skills"][0]["callbacks"]["initialFunc"] = json!({"path":"unreviewed"});
    assert_eq!(
        ordinary::review_recipes(&changed).unwrap_err(),
        "selected recipe callback domain"
    );
    let mut changed = recipes;
    changed["skills"].as_array_mut().unwrap().pop();
    assert_eq!(
        ordinary::review_recipes(&changed).unwrap_err(),
        "complete selected recipe multiplicity"
    );
    let original = family::construction(0);
    let basic = original["skill_construction"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["active_effect"]["id"] == "MinionMeleeBow")
        .unwrap();
    for edit in 0..4 {
        let mut row = basic.clone();
        match edit {
            0 => row["skill_data"]["critChanceOverride"] = json!(0),
            1 => row["skill_flags"]["triggered"] = json!(false),
            2 => row["actor_profile"] = json!("SandDjinn"),
            _ => row["supports"] = json!([{"id":"unreviewed"}]),
        };
        assert!(ordinary::review_basic(&row).is_err());
    }
    let mut changed = original;
    let row = changed["skill_construction"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["active_effect"]["id"] == "SummonSandDjinnPlayer")
        .unwrap();
    row["actor_profile"] = json!("RaisedSkeletonSniper");
    assert_eq!(
        ordinary::review_capture(&changed, 0).unwrap_err(),
        "foreign critical supplier is not Sniper"
    );
}

#[test]
fn finite_source_certificate_cannot_authorize_native_or_changed_candidate_absence() {
    let original = ordinary::domain();
    ordinary::review_certificate(original).unwrap();
    for edit in 0..5 {
        let mut changed = original.clone();
        match edit {
            0 => changed["scope"]["native_membership_authorized"] = json!(true),
            1 => changed["scope"]["changed_candidate_authorized"] = json!(true),
            2 => changed["scope"]["native_boolean_absence_authorized"] = json!(true),
            3 => changed["writer_domains"][10]["body_sha256"] = json!("changed"),
            _ => {
                changed["inherited_proofs"].as_array_mut().unwrap().pop();
            }
        };
        assert!(ordinary::review_certificate(&changed).is_err());
    }
}

#[test]
fn six_authored_rounding_fragments_use_semantic_rounding_and_exact_domain() {
    use poe_optimizer_core::owned_build::ParameterValue;
    use poe_optimizer_engine::owned_rules::EffectDisposition;
    let proof = family::read("rounding-domain.json");
    assert_eq!(proof["rounding_sites"], json!(rounding::SITES));
    let values: Vec<_> = proof["counterexamples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| Some(v["scaled_operand"].as_f64().unwrap()))
        .collect();
    for site in rounding::SITES {
        let fragment = rounding::fragment(site, false);
        let actual = fragment.evaluate(&values);
        assert_eq!(actual.len(), values.len());
        for ((row, operand), result) in proof["counterexamples"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&values)
            .zip(actual)
        {
            let x = operand.unwrap();
            let in_domain = (-4503599627370496.0..=0.4999999999999999).contains(&x)
                || (0.5..=4503599627370496.0).contains(&x);
            if in_domain {
                let EffectDisposition::Applied {
                    value: ParameterValue::Quantity(value),
                } = result
                else {
                    panic!("{site}: admitted operand {x} did not evaluate")
                };
                assert_eq!(
                    value.value(),
                    row["native_nearest_ties_positive"].as_f64().unwrap() / 100.0
                );
                assert_eq!(row["agree"], true);
                assert_eq!(
                    row["source_add_half_floor"].as_f64().unwrap(),
                    (x + 0.5).floor()
                );
            } else {
                assert!(
                    matches!(result, EffectDisposition::UnsupportedDomain { .. }),
                    "{site}: {x}"
                );
            }
        }
        let reordered =
            fragment.evaluate(&[Some(500.0), Some(0.49999999999999994), None, Some(500.0)]);
        assert_eq!(reordered[0], reordered[3]);
        assert_eq!(
            vec![reordered[0].clone()],
            fragment.evaluate(&[Some(500.0)])
        );
        assert!(matches!(
            reordered[1],
            EffectDisposition::UnsupportedDomain { .. }
        ));
        assert!(
            matches!(&reordered[2], EffectDisposition::Unresolved { input } if input.as_str() == "operand")
        );
    }
}

#[test]
fn semantic_rounding_refuses_accident_nodes_but_preserves_lazy_bypass() {
    use poe_optimizer_core::owned_build::ParameterValue;
    use poe_optimizer_engine::owned_rules::EffectDisposition;
    let original = family::read("consumer.json");
    rounding::review_nodes(&original).unwrap();
    for edit in 0..4 {
        let mut changed = original.clone();
        let nodes = changed["owners"][0]["programs"]["members"][0]["nodes"]
            .as_array_mut()
            .unwrap();
        let node = nodes
            .iter_mut()
            .find(|n| {
                n["id"]
                    == if edit < 2 {
                        "rounded-chance-integer"
                    } else {
                        "rounded-chance-lower-domain"
                    }
            })
            .unwrap();
        match edit {
            0 => node["expression"]["mode"] = json!("floor"),
            1 => node["expression"]["value"] = json!("rounded-chance-scaled"),
            2 => {
                node["expression"]["domain"]["value"]["maximum"]["value"] =
                    json!(0.49999999999999994)
            }
            _ => {
                node["expression"]["domain"]["value"]["minimum"]["value"] =
                    json!(-4503599627370497.0)
            }
        };
        assert!(rounding::review_nodes(&changed).is_err());
    }
    for site in rounding::SITES {
        let bypass = rounding::fragment(site, true);
        for actual in bypass.evaluate(&[None, Some(0.49999999999999994), Some(f64::MAX)]) {
            assert!(
                matches!(actual,EffectDisposition::Applied{value:ParameterValue::Quantity(v)} if v.value()==7.0)
            );
        }
        assert!(!matches!(
            rounding::fragment(site, false).evaluate(&[None])[0],
            EffectDisposition::Applied { .. }
        ));
    }
}
