use super::{ascendancy, ascendancy_family, family};
use serde_json::{Value, json};

#[test]
fn default_start_root_has_complete_source_inventory_and_no_class_promotion() {
    family::check_authored();
}

#[test]
fn root_closure_cannot_hide_members_topology_or_class_coverage_changes() {
    let dependencies: Value = family::read("dependencies.json");
    let closure: Value = family::read("closure.json");
    for edit in 0..4 {
        let mut d = dependencies.clone();
        let mut c = closure.clone();
        match edit {
            0 => c["definitions"][0]["value"]["schema"]["value"]["adjacent"]["members"] = json!([]),
            1 => {
                c["definitions"][0]["value"]["schema"]["value"]["declarations"]["grants"]["members"] =
                    json!(["unaccounted-input"])
            }
            2 => c["owners"][0]["programs"]["members"] = json!(["unaccounted-program"]),
            3 => d["action_owners"][1]["programs"]["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_rows(&d, &c)).is_err());
    }
}

#[test]
fn empty_default_proof_rejects_extra_effects_or_declarations() {
    let a: Value = family::read("authoring.json");
    let v: Value = family::read("source-vectors.json");
    for edit in 0..3 {
        let mut changed = v.clone();
        match edit {
            0 => {
                changed["projection"]["constructed"]["default_modifiers"]["records"] =
                    json!([{"name":"unaccounted"}])
            }
            1 => {
                changed["projection"]["constructed"]["declaration_fields"][0]["present"] =
                    json!(true)
            }
            2 => changed["projection"]["constructed"]["excluded"]
                .as_array_mut()
                .unwrap()
                .push(json!({"key":"grantedSkills","reason":"render_geometry"})),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_vectors(&a, &changed)).is_err());
    }
}

#[test]
fn default_ascendancy_root_preserves_separate_owner_and_topology_coverage() {
    ascendancy_family::check_authored();
}

#[test]
fn ascendancy_root_closure_rejects_extra_rules_declarations_and_topology_edits() {
    let dependencies: Value = ascendancy_family::read("dependencies.json");
    let closure: Value = ascendancy_family::read("closure.json");
    for edit in 0..5 {
        let mut d = dependencies.clone();
        let mut c = closure.clone();
        match edit {
            0 => c["definitions"][0]["value"]["schema"]["value"]["adjacent"]["members"] = json!([]),
            1 => {
                c["definitions"][0]["value"]["schema"]["value"]["declarations"]["skill_grants"]["members"] =
                    json!(["unexpected-grant"])
            }
            2 => c["owners"][0]["programs"]["members"] = json!(["unexpected-rule"]),
            3 => d["action_owners"][1]["programs"]["closure"] = json!({"kind":"complete"}),
            4 => d["action_owners"][3]["programs"]["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| ascendancy_family::check_rows(&d, &c)).is_err());
    }
}

#[test]
fn ascendancy_root_proof_rejects_unknown_fields_and_changed_selected_modifiers() {
    let a: Value = ascendancy_family::read("authoring.json");
    let v: Value = ascendancy_family::read("source-vectors.json");
    for edit in 0..6 {
        let mut changed = v.clone();
        match edit {
            0 => changed["projection"]["raw"]["futureMechanic"] = json!(true),
            1 => changed["projection"]["constructed"]["fields"]["futureMechanic"] = json!(true),
            2 => {
                changed["projection"]["selected_modifiers"]["records"] = json!([{"name":"unknown"}])
            }
            3 => {
                changed["projection"]["constructed"]["declaration_fields"][0]["present"] =
                    json!(true)
            }
            4 => {
                changed["projection"]["selected_root"]["default_modifier_object_identity"] =
                    json!(true)
            }
            5 => changed["projection"]["ascendancies"][0]["start_node_id"] = json!(54447),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| ascendancy_family::check_vectors(&a, &changed)).is_err()
        );
    }
}

#[test]
fn authored_metadata_closure_preserves_seventeen_independent_owners() {
    ascendancy::check_authored();
}

#[test]
fn metadata_inventory_rejects_unknown_empty_fields_and_changed_source_relations() {
    let original: Value = ascendancy::read("source-vectors.json");
    for edit in 0..9 {
        let mut changed = original.clone();
        match edit {
            0 => changed["projections"][0]["raw"]["stats"] = json!([]),
            1 => changed["projections"][0]["constructed"]["fields"]["grantedSkills"] = json!({}),
            2 => changed["projections"][0]["raw"]["replace"] = Value::Null,
            3 => changed["projections"][0]["raw_class_id"] = json!(1),
            4 => changed["projections"][0]["constructed"]["index"] = json!(2),
            5 => changed["projections"][0]["raw"]["internalId"] = json!("Sorceress2"),
            6 => changed["projections"][0]["constructed"]["fields"]["startNodeId"] = json!(54447),
            7 => changed["root_ascendancy"]["same_root"] = json!(false),
            8 => changed["projections"][0]["evidence"]["original_constructor"] = json!(false),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| ascendancy::check_vectors(&changed)).is_err());
    }
}

#[test]
fn closure_cannot_hide_new_members_or_promote_class_player_and_passives() {
    let original_d: Value = ascendancy::read("dependencies.json");
    let original_c: Value = ascendancy::read("closure.json");
    for edit in 0..7 {
        let mut d = original_d.clone();
        let mut c = original_c.clone();
        match edit {
            0 => {
                c["definitions"][0]["value"]["schema"]["value"]["declarations"]["grants"]["members"] =
                    json!(["unknown"])
            }
            1 => c["owners"][0]["programs"]["members"] = json!(["unknown"]),
            2 => c["definitions"][0]["value"]["schema"]["value"]["classes"]["members"] = json!([]),
            3 => {
                c["definitions"][0]["value"]["schema"]["value"]["implicit_passives"]["members"] =
                    json!([])
            }
            4 => d["action_owners"][0]["programs"]["closure"] = json!({"kind":"complete"}),
            5 => d["action_owners"][1]["programs"]["closure"] = json!({"kind":"complete"}),
            6 => d["action_owners"][4]["programs"]["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| ascendancy::check_rows(&d, &c)).is_err());
    }
}
