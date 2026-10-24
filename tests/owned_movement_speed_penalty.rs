#[allow(dead_code)]
#[path = "support/owned_movement_speed_penalty.rs"]
mod packet;
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use poe_optimizer_import::owned_recipe::{OwnedRecipeInput, assemble_owned_recipe};
use serde_json::Value;
use std::{fs, path::PathBuf};
#[test]
fn authored_body_consumer_preserves_unavailable_suppliers_and_exact_raw_units() {
    packet::check_authored();
    let c = packet::consumer();
    assert_eq!(c.program.id.as_str(), packet::PROGRAM);
    assert_eq!(c.query.stat.key().as_str(), packet::IGNORE);
    assert!(matches!(
        c.program.effects[0].effect,
        RuleEffectKind::Contribute {
            contribution: ContributionKind::Add,
            ..
        }
    ));
    let deps = packet::dependencies();
    let raw = &deps.raw_profiles[0];
    assert_eq!(raw.template.key().as_str(), "def.000000000000238c");
    let n = raw
        .program
        .nodes
        .iter()
        .find(|n| n.id.as_str() == "value-4")
        .unwrap();
    let RuleExpression::Literal {
        value: poe_optimizer_core::owned_build::ParameterValue::Quantity(v),
    } = &n.expression
    else {
        panic!()
    };
    assert_eq!(v.value().to_bits(), 0.03f64.to_bits());
    assert_eq!(v.unit().key().as_str(), "def.0000000000000001");
    assert!(!c.query.groups[0].members.is_complete());
    assert!(c.query.groups[0].members.members.is_empty());
}
#[test]
fn maintained_profile_policy_and_descriptor_use_fraction_without_changed_numbers() {
    let base = packet::root().join("data/owned/poe2/3887ae68/defence-profiles");
    let p: Value = serde_json::from_slice(&fs::read(base.join("policy.json")).unwrap()).unwrap();
    let field = p["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["source_field"] == "MovementPenalty")
        .unwrap();
    assert_eq!(field["unit"]["key"], "def.0000000000000001");
    assert_eq!(field["when_absent"]["kind"], "omit");
    let defs: Vec<DefinitionDescriptor> =
        serde_json::from_slice(&fs::read(base.join("definitions.json")).unwrap()).unwrap();
    let d = defs
        .iter()
        .find(|d| serde_json::to_value(d).unwrap()["value"]["id"]["key"] == packet::RAW)
        .unwrap();
    let DefinitionDescriptor::Stat(e) = d else {
        panic!()
    };
    let SchemaState::Known(s) = &e.schema else {
        panic!()
    };
    let ComputedValueType::Quantity { unit } = &s.value else {
        panic!()
    };
    assert_eq!(unit.key().as_str(), "def.0000000000000001");
}
/// Explicit offline checkpoint; no ordinary test depends on a runs directory.
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_MOVEMENT_PENALTY_PRIOR current package"]
fn exact_current_recipe_unit_repair_rejects_stale_values_and_compiles_new_ingredient() {
    let path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MOVEMENT_PENALTY_PRIOR").unwrap());
    let read = |name: &str| -> Value {
        serde_json::from_slice(&fs::read(path.join(name)).unwrap()).unwrap()
    };
    let prior:OwnedRecipeInput=serde_json::from_value(serde_json::json!({"schema_version":1,"registry":read("registry.json"),"schema":poe_optimizer_data::owned_schema::decode_schema_input(&fs::read(path.join("schema.json")).unwrap(), Default::default()).unwrap(),"rules":read("rules.json"),"routing":read("routing.json")})).unwrap();
    let next = packet::correct_raw_units(&prior).unwrap();
    assert!(packet::correct_raw_units(&next).is_err());
    let mut bad = prior.clone();
    let owner = bad
        .rules
        .owners
        .iter_mut()
        .find(|o| {
            serde_json::to_value(&o.owner).unwrap()["value"]["value"]["key"]
                == "def.000000000000238c"
        })
        .unwrap();
    let p = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id.as_str() == "raw-base-defence-profile")
        .unwrap();
    let node = p
        .nodes
        .iter_mut()
        .find(|n| n.id.as_str() == "value-4")
        .unwrap();
    let RuleExpression::Literal {
        value: poe_optimizer_core::owned_build::ParameterValue::Quantity(q),
    } = &mut node.expression
    else {
        panic!()
    };
    *q = poe_optimizer_core::owned_definitions::FiniteQuantity::new(3.0, q.unit().clone()).unwrap();
    assert!(packet::correct_raw_units(&bad).is_err());
    let mut missing = prior.clone();
    missing.rules.owners.retain(|o| {
        serde_json::to_value(&o.owner).unwrap()["value"]["value"]["key"] != "def.000000000000238c"
    });
    assert!(packet::correct_raw_units(&missing).is_err());
    let staged = packet::staged_recipe(&prior).unwrap();
    assert_eq!(staged.registry.last_issued.get(), 0x33cc);
    let _ = assemble_owned_recipe(staged, Default::default()).unwrap();
}
