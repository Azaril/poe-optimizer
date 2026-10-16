//! Checked global-base selection with exact predecessor preservation.
#[allow(dead_code)]
#[path = "support/owned_player_evasion_equipment.rs"]
mod equipment;
#[path = "support/owned_player_evasion_equipment_replay.rs"]
mod equipment_native;
#[allow(dead_code)]
#[path = "support/owned_player_evasion_final.rs"]
mod evasion_family;
#[path = "support/owned_player_evasion_final_replay.rs"]
mod evasion_native;
#[allow(dead_code)]
#[path = "support/owned_item_evasion_property.rs"]
mod family;
#[path = "support/owned_item_evasion_property_native.rs"]
mod item_native;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_player_evasion_override_replay.rs"]
mod native;
#[path = "support/owned_player_evasion_override.rs"]
mod override_family;
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
use poe_optimizer_core::{owned_rules::*, owned_schema::SchemaClosure};
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn exact_source_and_consumer_delta_preserve_the_partial_predecessor() {
    override_family::check_authored();
    for change in 0..6 {
        let mut c = override_family::consumer();
        match change {
            0=>c.owner.programs.closure=SchemaClosure::Complete,
            1=>c.query.groups[0].empty=Some(serde_json::from_value(json!({"kind":"quantity","value":{"value":0,"unit":family::def::<poe_optimizer_core::owned_definitions::UnitDefinition>(evasion_family::UNIT)}})).unwrap()),
            2=>c.query.groups[0].ordering=ContributionOrdering::Ordered,
            3=>c.owner.programs.members[0].reads.pop().map(|_|()).unwrap(),
            4=>c.owner.programs.members[0].nodes.iter_mut().find(|n|n.id.as_str()=="increased").unwrap().expression=RuleExpression::Scale {value:"base".parse().unwrap(),factor:"increase-factor".parse().unwrap()},
            5=>c.owner.programs.members[0].effects[0].when=Some("override-present".parse().unwrap()),
            _=>unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| override_family::check_inverse(&c)).is_err(),
            "mutation {change}"
        );
    }
}
#[test]
fn immutable_source_identity_body_and_scope_changes_are_rejected() {
    let p: Value = override_family::read("source-evidence.json");
    override_family::authenticate_source(&p);
    for change in 0..7 {
        let mut p = p.clone();
        match change {
            0 => p["prior_source_domain"]["sha256"] = json!("changed"),
            1 => p["complete_bodies"][0]["sha256"] = json!("changed"),
            2 => p["spans"][1]["body"] = json!("changed"),
            3 => p["spans"].as_array_mut().unwrap().reverse(),
            4 => p["scope"]["candidate_source_closure"] = json!(true),
            5 => p["scope"]["source_insertion_order_emulated"] = json!(true),
            6 => p["native_admissible_override_domain"]
                .as_array_mut()
                .unwrap()
                .push(json!("conflicting-values")),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| override_family::authenticate_source(&p)).is_err(),
            "mutation {change}"
        );
    }
}
#[test]
fn potential_writer_census_retains_zero_inactive_and_application_rows() {
    let mut r = json!({"owners":[],"effect_applications":{"members":[]}});
    assert_eq!(override_family::override_writers(&r), json!([]));
    let p = json!({"id":"synthetic.unselected","reads":[],"nodes":[{"id":"zero","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":0,"unit":family::def::<poe_optimizer_core::owned_definitions::UnitDefinition>(evasion_family::UNIT)}}}}],"effects":[{"id":"override","when":"inactive","effect":{"kind":"contribute","entity":"enemy","stat":family::def::<poe_optimizer_core::owned_definitions::StatDefinition>(evasion_family::CHANNEL),"contribution":"override","value":"zero"}}]});
    r["owners"] = json!([{"owner":"unselected-owner","programs":{"members":[p.clone()]}}]);
    assert_eq!(
        override_family::override_writers(&r)
            .as_array()
            .unwrap()
            .len(),
        1
    );
    r["effect_applications"]["members"] = json!([{"program":p}]);
    let rows = override_family::override_writers(&r);
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(rows[0]["program"], r["owners"][0]["programs"]["members"][0]);
    assert_eq!(
        rows[1]["application"],
        r["effect_applications"]["members"][0]
    );
}
#[test]
#[ignore = "explicit PLAYER_EVASION_OVERRIDE_PRIOR/OUTPUT; final owner remains Partial"]
fn publish_player_evasion_override_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_EVASION_OVERRIDE_PRIOR").unwrap(),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_EVASION_OVERRIDE_OUTPUT").unwrap(),
        ),
        &override_family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
        ],
        override_family::stage,
        json!({"new_definitions":0,"new_owner_programs":0,"replaced_programs":1,"new_queries":1,"complete_request":false}),
        [106, 117, 109, 123, 4],
    );
}
