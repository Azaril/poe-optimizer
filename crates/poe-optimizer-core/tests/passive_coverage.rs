//! Passive evidence is optional in historical coverage schema 1, strict when present.
use poe_optimizer_core::{coverage::*, evaluation::EvaluationResult};
use serde_json::json;

fn evidence() -> PassiveCoverage {
    serde_json::from_value(json!({
        "schema_version":1,"tree_version":"custom-tree",
        "class":{"index":1,"internal_id":7,"name":"Test","start_node_id":42},
        "ascendancy":null,"secondary_ascendancy":null,
        "allocation_counts":{"ordinary":0,"ascendancy":0,"secondary_ascendancy":0,"sockets":0,"weapon_set_1":0,"weapon_set_2":0},
        "allocated_nodes":[{
            "physical_node_id":42,"node_type":"ClassStart","name":"Inherited","display_name":"Observed",
            "stats":[],"allocation_mode":0,"implicit_roots":["class"],"free_allocation":null,
            "ascendancy_name":null,"is_multiple_choice_option":false,"is_granted_passive":false,
            "is_attribute":false,"is_conquered":false,"has_hash_override":false,"is_switchable":false,"switch":null
        }]
    })).unwrap()
}
fn legacy() -> EvaluationResult {
    serde_json::from_value(json!({
        "backend":{"id":"test","implementation_version":"1","rules_revision":"rules","source_fingerprint":"source","adapter_fingerprint":"adapter"},
        "build":{"level":1,"class_name":"Test","ascendancy_name":"None","tree_version":"custom-tree","main_socket_group":1,"allocated_nodes":[42],"skill_groups":0},
        "context":{"requested":{},"calculation_mode":"MAIN","enemy_level":1,"config_inputs":{},"config_placeholders":{},"player_conditions":{},"enemy_conditions":{}},
        "coverage":{"schema_version":1,"active_skill_set_id":1,"groups":[],"selected_player":null,"selected_minion":null,"full_dps":{"included_group_count":0,"selected_group_included":false,"active_skills":[],"reported_contributions":[]},"unresolved_entry_count":0,"tree_connections":[]},
        "measurements":[],"exports":[],"warnings":[],"elapsed_ms":0.0,"diagnostic_only":true,"attachments":[]
    })).unwrap()
}

#[test]
fn old_schema_one_reports_remain_readable_without_invented_passive_evidence() {
    let result = legacy();
    result.validate_recorded().unwrap();
    assert!(result.coverage.passives.is_none());
    let mut encoded = serde_json::to_value(&result).unwrap();
    assert!(encoded["coverage"].get("passives").is_none());
    encoded["coverage"]["passives"] = serde_json::Value::Null;
    let decoded: EvaluationResult = serde_json::from_value(encoded).unwrap();
    decoded.validate_recorded().unwrap();
    assert!(decoded.coverage.passives.is_none());
}

#[test]
fn present_evidence_round_trips_with_distinct_live_names_and_allocation_flags() {
    let mut result = legacy();
    let mut passive = evidence();
    passive.allocated_nodes[0].free_allocation = Some(false);
    result.coverage.passives = Some(passive.clone());
    result.validate_recorded().unwrap();
    let restored: EvaluationResult =
        serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    restored.validate_recorded().unwrap();
    assert_eq!(restored.coverage.passives, Some(passive));
    let node = &restored.coverage.passives.unwrap().allocated_nodes[0];
    assert_ne!(node.name, node.display_name);
    assert_eq!(node.free_allocation, Some(false));
}

#[test]
fn present_evidence_must_match_the_recorded_build_identity_and_allocations() {
    for field in ["tree", "class", "ascendancy", "nodes", "duplicates"] {
        let mut result = legacy();
        result.coverage.passives = Some(evidence());
        match field {
            "tree" => result.build.tree_version = "other".into(),
            "class" => result.build.class_name = "other".into(),
            "ascendancy" => result.build.ascendancy_name = "other".into(),
            "nodes" => result.build.allocated_nodes.push(43),
            "duplicates" => result.build.allocated_nodes.push(42),
            _ => unreachable!(),
        }
        assert!(
            result.validate_recorded().is_err(),
            "accepted mismatched {field}"
        );
    }
}

#[test]
fn malformed_passive_schemas_modes_roots_and_ordering_are_rejected() {
    for field in [
        "schema",
        "empty",
        "blank",
        "mode",
        "root",
        "duplicate_role",
        "duplicate_node",
        "wrong_order",
        "bucket",
    ] {
        let mut value = evidence();
        match field {
            "schema" => value.schema_version = 2,
            "empty" => value.allocated_nodes.clear(),
            "blank" => value.allocated_nodes[0].display_name = " ".into(),
            "mode" => value.allocated_nodes[0].allocation_mode = 3,
            "root" => value.class.start_node_id = 43,
            "duplicate_role" => value.allocated_nodes[0]
                .implicit_roots
                .push(PassiveRootRole::Class),
            "duplicate_node" => value.allocated_nodes.push(value.allocated_nodes[0].clone()),
            "wrong_order" => {
                let mut earlier = value.allocated_nodes[0].clone();
                earlier.physical_node_id = 1;
                value.allocated_nodes.push(earlier);
            }
            "bucket" => value.allocation_counts.ordinary = 2,
            _ => unreachable!(),
        }
        assert!(value.validate().is_err(), "accepted {field}");
        let mut result = legacy();
        result.coverage.passives = Some(value);
        assert!(result.validate_recorded().is_err());
    }
}

#[test]
fn switch_observation_retains_later_modification_flags_but_requires_its_selector() {
    let mut value = evidence();
    let node = &mut value.allocated_nodes[0];
    node.is_switchable = true;
    node.switch = Some(PassiveSwitchCoverage {
        selected_source_id: 900,
        kind: PassiveSwitchKind::Class,
        selector: Some("Test".into()),
        stats_reference_matches_selected_source: false,
        display_name_matches_selected_source: false,
    });
    // A later modifier is observable evidence, not malformed structure.
    value.validate().unwrap();
    let mut wrong = value.clone();
    wrong.allocated_nodes[0].switch.as_mut().unwrap().selector = Some("Other".into());
    assert!(wrong.validate().is_err());
    let mut wrong = value.clone();
    wrong.allocated_nodes[0].is_switchable = false;
    assert!(wrong.validate().is_err());
    let mut wrong = value.clone();
    wrong.allocated_nodes[0].switch.as_mut().unwrap().kind = PassiveSwitchKind::Base;
    assert!(wrong.validate().is_err());
    let mut wrong = value;
    wrong.allocated_nodes[0].switch.as_mut().unwrap().kind = PassiveSwitchKind::Ascendancy;
    assert!(wrong.validate().is_err());
}

#[test]
fn passive_unknown_fields_and_wrong_scalar_types_cannot_silently_disappear() {
    let mut value = serde_json::to_value(evidence()).unwrap();
    value["unsupported_extra"] = json!(true);
    assert!(serde_json::from_value::<PassiveCoverage>(value).is_err());
    for invalid in [json!(-1), json!(1.5), json!("1"), json!(null)] {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["allocated_nodes"][0]["allocation_mode"] = invalid;
        assert!(serde_json::from_value::<PassiveCoverage>(value).is_err());
    }
}
