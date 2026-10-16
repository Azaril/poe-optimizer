//! Actual item property transport into the Partial final Player Evasion body.
#[allow(dead_code)]
#[path = "support/owned_player_evasion_equipment.rs"]
mod equipment;
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
#[path = "support/owned_player_evasion_equipment_replay.rs"]
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
use poe_optimizer_core::owned_rules::*;
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn exact_slot_source_and_receiver_inverse_are_authenticated() {
    equipment::check_authored();
    family::check();
    let mut changed = equipment::consumer();
    changed.final_owner.programs.members[0].effects[0].when =
        Some("equipment-subtotal".parse().unwrap());
    assert!(std::panic::catch_unwind(|| equipment::check_final_inverse(&changed)).is_err());
    let mut changed = equipment::consumer();
    changed.final_owner.programs.closure =
        poe_optimizer_core::owned_schema::SchemaClosure::Complete;
    assert!(std::panic::catch_unwind(|| equipment::check_final_inverse(&changed)).is_err());
}
#[test]
fn each_slot_scales_before_source_ordered_addition() {
    let c = equipment::consumer();
    let p = &c.slot_program;
    let mut previous = "zero".to_string();
    for slot in &c.slots {
        let lookup = |name: &str| {
            &p.nodes
                .iter()
                .find(|n| n.id.as_str() == name)
                .unwrap()
                .expression
        };
        assert_eq!(
            lookup(&format!("{}-increased", slot.key)),
            &RuleExpression::Scale {
                value: format!("{}-base", slot.key).parse().unwrap(),
                factor: "increase-factor".parse().unwrap()
            }
        );
        assert_eq!(
            lookup(&format!("{}-scaled", slot.key)),
            &RuleExpression::Scale {
                value: format!("{}-increased", slot.key).parse().unwrap(),
                factor: "more".parse().unwrap()
            }
        );
        assert_eq!(
            lookup(&format!("{}-sum", slot.key)),
            &RuleExpression::Add {
                left: previous.parse().unwrap(),
                right: format!("{}-scaled", slot.key).parse().unwrap()
            }
        );
        previous = format!("{}-sum", slot.key);
    }
    assert_eq!(previous, "offhand-sum");
    assert_eq!(p.effects.len(), 1);
    assert!(
        matches!(&p.effects[0].effect,RuleEffectKind::Derive{value,..}if value.as_str()=="offhand-sum")
    );
    assert!(!p.nodes.iter().any(|n| n.id.as_str().contains("weapon3")));
    assert!(
        p.reads
            .iter()
            .filter(|r| matches!(r.source, RuleReadSource::PlayerEquipmentSlot { .. }))
            .count()
            == 10
    );
}
#[test]
fn unconstructed_slot_zero_is_neutral_in_owned_finite_arithmetic() {
    use poe_optimizer_core::owned_definitions::{FiniteQuantity, UnitDefinition};
    let unit = family::def::<UnitDefinition>(0x29ee);
    let finite = |n| FiniteQuantity::new(n, unit.clone()).unwrap().value();
    for factor in [f64::MIN, -1., -0., 0., f64::from_bits(1), 1., f64::MAX] {
        let zero = finite(finite(-0. * factor) * factor);
        assert_eq!(zero.to_bits(), 0f64.to_bits());
        for subtotal in [f64::MIN, -1., -0., 0., f64::from_bits(1), 1., f64::MAX] {
            assert_eq!(
                finite(finite(subtotal) + zero).to_bits(),
                finite(subtotal).to_bits()
            );
        }
    }
}
#[test]
fn subtotal_writer_census_keeps_zero_inactive_and_application_effects() {
    let c = equipment::consumer();
    let mut r = json!({"owners":[],"effect_applications":{"members":[]}});
    assert_eq!(equipment::subtotal_writers(&r), json!([]));
    let mut p = json!(c.slot_program);
    p["effects"][0]["when"] = json!("inactive");
    p["nodes"].as_array_mut().unwrap().push(json!({"id":"inactive","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}));
    r["owners"] = json!([{"owner":"synthetic-unused-owner","programs":{"members":[p.clone()]}}]);
    assert_eq!(equipment::subtotal_writers(&r).as_array().unwrap().len(), 1);
    r["effect_applications"]["members"] = json!([{"program":p}]);
    assert_eq!(equipment::subtotal_writers(&r).as_array().unwrap().len(), 2);
}
#[test]
fn exact_slot_mapping_and_loadout_scopes_are_not_inferred_from_names() {
    let d: Value = equipment::read("dependencies.json");
    let mapping = json!({"entries":d["slot_mappings"]});
    let normalization = json!({"equipment_loadouts":d["equipment_loadouts"]});
    equipment::check_slot_bindings(&mapping, &normalization);
    let mut changed = mapping.clone();
    changed["entries"].as_array_mut().unwrap().pop();
    assert!(
        std::panic::catch_unwind(|| equipment::check_slot_bindings(&changed, &normalization))
            .is_err()
    );
    let mut changed = normalization.clone();
    changed["equipment_loadouts"][0]["scope"] = json!({"kind":"shared"});
    assert!(
        std::panic::catch_unwind(|| equipment::check_slot_bindings(&mapping, &changed)).is_err()
    );
}
#[test]
#[ignore = "explicit PLAYER_EVASION_EQUIPMENT_PRIOR/OUTPUT; final owner remains Partial"]
fn publish_player_equipment_evasion_preserving_all_five_originals() {
    let a: Value = equipment::read("authoring.json");
    assert_eq!(a["scope"]["publication_ready"], true);
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_EVASION_EQUIPMENT_PRIOR").unwrap(),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_EVASION_EQUIPMENT_OUTPUT").unwrap(),
        ),
        &equipment::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
        ],
        equipment::stage,
        json!({"new_definitions":1,"new_owner_programs":1,"replaced_programs":1,"complete_request":false}),
        [106, 117, 109, 123, 4],
    );
}
