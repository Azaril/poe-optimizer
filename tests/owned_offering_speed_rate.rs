//! Actual Offering Speed source/application data; no final timing authority is
//! inferred from a measured rate or a finite test world's complete memberships.
#[allow(dead_code)]
#[path = "support/owned_offering_speed_rate.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_offering_speed_rate_replay.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_selected_request.rs"]
mod selected;
#[path = "support/owned_offering_speed_source_domain.rs"]
mod source_domain;
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn timing_source_domain_refuses_zero_inactive_suppliers_and_recipient_relocation() {
    use std::io::Read;
    let domain = source_domain::domain();
    let pin = &domain["inputs"][0]["archive"];
    let path = family::root().join(pin["path"].as_str().unwrap());
    assert!(std::fs::metadata(&path).unwrap().len() < 2 * 1024 * 1024);
    let bytes = std::fs::read(path).unwrap();
    let mut raw = vec![];
    flate2::read::GzDecoder::new(bytes.as_slice())
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() <= 32 * 1024 * 1024);
    let mut capture: Value = serde_json::from_slice(&raw).unwrap();
    source_domain::review_capture(&capture, "jit-off").unwrap();
    // One full capture is reused sequentially; no parallel whole-report clones.
    for name in [
        "Speed",
        "SkillAttackTime",
        "TotalAttackTime",
        "TotalCastTime",
        "ActionSpeed",
        "TemporalChainsActionSpeed",
        "MinimumActionSpeed",
        "RepeatCount",
        "CooldownRecovery",
    ] {
        capture["state"]["baseline"][0]["player_store"][0]["buckets"]["SyntheticTiming"] = json!([{"position":1,"body":{"name":name,"type":"BASE","value":0,"flags":0,"keywordFlags":0,"source":"Synthetic","_positions":[{"index":1,"value":{"type":"Condition","var":"SyntheticNever"}}]}}]);
        assert!(
            source_domain::review_capture(&capture, "jit-off").is_err(),
            "{name}"
        );
        capture["state"]["baseline"][0]["player_store"][0]["buckets"]
            .as_object_mut()
            .unwrap()
            .remove("SyntheticTiming");
    }
    let original = capture["state"]["baseline"][0]["player_store"][0]["conditions"].clone();
    capture["state"]["baseline"][0]["player_store"][0]["conditions"] = json!({"Chilled":false});
    assert!(source_domain::review_capture(&capture, "jit-off").is_err());
    if original.is_null() {
        capture["state"]["baseline"][0]["player_store"][0]
            .as_object_mut()
            .unwrap()
            .remove("conditions");
    } else {
        capture["state"]["baseline"][0]["player_store"][0]["conditions"] = original;
    }
    let at = capture["state"]["skill_construction"]
        .as_array()
        .unwrap()
        .iter()
        .position(|r| r["active_effect"]["id"] == "MinionMeleeBow")
        .unwrap();
    let original = capture["state"]["skill_construction"][at]["summoner"].clone();
    capture["state"]["skill_construction"][at]["summoner"] = json!({"id":"different-source"});
    assert!(source_domain::review_capture(&capture, "jit-off").is_err());
    capture["state"]["skill_construction"][at]["summoner"] = original;
    source_domain::review_capture(&capture, "jit-off").unwrap();
}

#[test]
fn timing_constructor_and_pass_do_not_treat_zero_overrides_or_other_skill_types_as_absent() {
    let d = source_domain::domain();
    let row = &d["inputs"][0]["basic_constructors"][0];
    source_domain::review_basic_constructor(row).unwrap();
    for edit in 0..5 {
        let mut r = row.clone();
        match edit {
            0 => r["skill_flags"]["selfCast"] = json!(false),
            1 => r["skill_flags"]["triggered"] = json!(false),
            2 => r["supports"] = json!([{"id":"SpellEcho"}]),
            3 => r["skill_data"] = json!({"cooldown":0}),
            _ => r["actor_profile"] = json!("RaisedSkeletonReaver"),
        }
        assert!(
            source_domain::review_basic_constructor(&r).is_err(),
            "edit{edit}"
        );
    }
    let report = family::source_report();
    let original = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "original-05")
        .unwrap();
    let actor = original["state"]["main"]["actors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["actor_profile"] == "RaisedSkeletonSniper")
        .unwrap();
    let child = actor["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["effect_id"] == "MinionMeleeBow")
        .unwrap();
    let input = &child["passes"][0]["inputs"];
    source_domain::review_observed_pass(input).unwrap();
    for field in d["branch_fields"].as_array().unwrap() {
        let mut value = input.clone();
        value["skill_data"][field.as_str().unwrap()] = json!(0);
        assert!(
            source_domain::review_observed_pass(&value).is_err(),
            "{field}"
        );
    }
}

fn native_fixture() -> (Value, Value, Value) {
    let d: Value = family::read("dependencies.json");
    let mut definitions = d["definitions"].as_array().unwrap().clone();
    for v in d["source_receiver_definitions"].as_array().unwrap() {
        if !definitions.contains(v) {
            definitions.push(v.clone());
        }
    }
    let schema = json!({"definitions":definitions,"slots":d["source_receiver_slots"]});
    let mut owners = d["source_receiver_owners"].as_array().unwrap().clone();
    for writer in d["application_input_query_writers"].as_array().unwrap() {
        assert_eq!(writer["kind"], "owner");
        if let Some(owner) = owners.iter_mut().find(|o| o["owner"] == writer["owner"]) {
            if !owner["programs"]["members"]
                .as_array()
                .unwrap()
                .contains(&writer["program"])
            {
                owner["programs"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .push(writer["program"].clone());
            }
        } else {
            owners.push(json!({"owner":writer["owner"],"programs":{"members":[writer["program"]],"closure":{"kind":"complete"}}}));
        }
    }
    let rules = json!({"owners":owners,"effect_applications":d["prior_application_registry"],"existing_actor_rules":d["existing_actor_rules"],"contribution_queries":{"members":d["application_input_queries"],"closure":d["remaining_query_registry_closure"]},"receivers":{"closure":d["remaining_receiver_registry_closure"]}});
    (schema, rules, d["intrinsic_routes"].clone())
}

#[test]
fn potential_speed_rate_and_input_writers_cannot_be_added_or_relabelled() {
    let (s, original, routes) = native_fixture();
    family::check_native(&s, &original, &routes, false);
    for edit in 0..7 {
        let mut rules = original.clone();
        let mut p = json!(family::consumer().owners[0].programs.members[0]);
        p["id"] = json!("synthetic.unselected-timing-writer");
        p["reads"] = json!([]);
        p["nodes"] = json!([
            {"id":"zero","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":0,"unit":{"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"def.0000000000000002"}}}}},
            {"id":"inactive","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}
        ]);
        let effect = &mut p["effects"][0];
        effect["effect"]["stat"]["key"] = json!(match edit {
            2 => "def.00000000000033a3",
            3 => "def.0000000000003214",
            4 => "def.000000000000322b",
            _ => "def.00000000000033a2",
        });
        effect["effect"]["value"] = json!("zero");
        effect["when"] = if edit == 1 {
            json!("inactive")
        } else {
            Value::Null
        };
        if edit != 2 {
            effect["effect"]["kind"] = json!("contribute");
            effect["effect"]["contribution"] = json!("add");
        }
        let _: poe_optimizer_core::owned_rules::RuleProgram =
            serde_json::from_value(p.clone()).unwrap();
        if edit == 5 {
            rules["effect_applications"]["members"][0]["program"] = p;
        } else if edit == 6 {
            rules["effect_applications"]["members"][0]["targets"] = json!([]);
        } else {
            rules["owners"].as_array_mut().unwrap().push(json!({"owner":{"kind":"definition","value":{"kind":"skill","value":{"kind":"skill","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"synthetic.unselected-skill"}}},"programs":{"members":[p],"closure":{"kind":"complete"}}}));
        }
        assert!(
            std::panic::catch_unwind(|| family::check_native(&s, &rules, &routes, false)).is_err(),
            "edit{edit}"
        );
    }
}

#[test]
fn timing_admission_keeps_exact_provider_slot_and_partial_domain_dependencies() {
    let (schema, rules, routes) = native_fixture();
    for edit in 0..5 {
        let (mut s, mut r, mut t) = (schema.clone(), rules.clone(), routes.clone());
        match edit {
            0 => {
                s["slots"].as_array_mut().unwrap().pop();
            }
            1 => {
                r["owners"][0]["programs"]["closure"] = json!({"kind":"complete"});
            }
            2 => {
                t[0]["routes"]["members"] = json!([]);
            }
            3 => {
                r["existing_actor_rules"]["members"] = json!([]);
            }
            _ => {
                r["receivers"]["closure"] = json!({"kind":"unknown"});
            }
        }
        assert!(
            std::panic::catch_unwind(|| family::check_native(&s, &r, &t, false)).is_err(),
            "edit{edit}"
        );
    }
}

#[test]
#[ignore = "explicit OFFERING_SPEED_RATE_PRIOR/OUTPUT; preserves complete-request refusal"]
fn publish_offering_speed_and_basic_rate_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_OFFERING_SPEED_RATE_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_OFFERING_SPEED_RATE_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
            "source-vectors.json",
            "timing-source-domain.json",
        ],
        family::stage,
        json!({"new_definitions":2,"new_owner_programs":1,"extended_effect_applications":1,"new_application_effects":1,"new_queries":1,"new_receivers":0,"complete_request":false}),
        [106, 117, 109, 123, 4],
    );
}

#[test]
fn speed_table_application_and_source_controls_are_authenticated() {
    family::check_authored();
}

#[test]
fn adding_speed_cannot_change_damage_identity_recipient_activation_or_stacking() {
    let original: Value = family::read("applications.json");
    family::check_application(&original).unwrap();
    for mutation in 0..8 {
        let mut value = original.clone();
        let app = &mut value["members"][0];
        match mutation {
            0 => app["activation"] = json!("speed-delivered"),
            1 => app["targets"] = json!([{"kind":"player"}]),
            2 => app["source"]["value"]["skill"]["key"] = json!("fixture.other-source"),
            3 => app["program"]["effects"][0]["effect"]["value"] = json!("speed-delivered"),
            4 => app["stacking"][1]["modifier"] = json!("unconditional-damage-increase"),
            5 => app["stacking"][1]["reduction"] = json!("sum"),
            6 => app["program"]["effects"][1]["effect"]["entity"] = json!("effect_source"),
            7 => value["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(
            family::check_application(&value).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn finite_level_table_and_quality_semantics_cannot_be_relabelled() {
    let vectors: Value = family::read("source-vectors.json");
    let source = &vectors["source_definition"];
    let tables: Value = family::read("tables.json");
    family::review_definition(source, &tables[0]).unwrap();
    for mutation in 0..6 {
        let mut s = source.clone();
        let mut table = tables[0].clone();
        match mutation {
            0 => table["rows"][21]["value"]["value"] = json!(31),
            1 => table["rows"].as_array_mut().unwrap().truncate(39),
            2 => s["stat_sets"][0]["stats"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            3 => s["quality_stats"] = s["alternate_quality_stats"].clone(),
            4 => table["rows"][0]["value"]["unit"]["key"] = json!("def.0000000000000001"),
            5 => s["stat_sets"][0]["levels"][0]["level"] = json!(0),
            _ => unreachable!(),
        }
        assert!(
            family::review_definition(&s, &table).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn retained_raw_level_control_uses_effective_level_and_independent_speed_values() {
    let vectors: Value = family::read("source-vectors.json");
    let rows = vectors["vectors"].as_array().unwrap();
    for (name, speed, level, delivered) in [
        ("original-05", 0.8666666666666667, 22, 30),
        ("offering-level-1", 0.8066666666666666, 3, 21),
        ("offering-higher-first", 0.9000000000000001, 32, 35),
        ("offering-higher-last", 0.9000000000000001, 32, 35),
    ] {
        let row = rows
            .iter()
            .find(|r| r["case"] == name && r["mode"] == "main")
            .unwrap();
        assert_eq!(row["basic_passes"][0]["speed"], speed);
        let event = row["offering_merge_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["source_level"] == level)
            .unwrap();
        assert_eq!(event["source_level"], level);
        let record = event["merged_modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "Speed")
            .unwrap();
        assert_eq!(record["type"], "INC");
        assert_eq!(record["value"], delivered);
        assert_eq!(event["recipient_profile"], "RaisedSkeletonSniper");
        assert_eq!(event["minion_destination"], true);
    }
    let disabled = rows
        .iter()
        .find(|r| r["case"] == "offering-disabled" && r["mode"] == "main")
        .unwrap();
    assert!(
        disabled["offering_merge_events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(disabled["basic_passes"][0]["speed"], 0.6666666666666666);
}
