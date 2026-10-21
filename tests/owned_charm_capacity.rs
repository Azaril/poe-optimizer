#[allow(dead_code)]
#[path = "support/owned_charm_capacity.rs"]
mod packet;
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use poe_optimizer_import::owned_recipe::{OwnedRecipeInput, assemble_owned_recipe};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[test]
fn authenticated_capacity_packet_keeps_exact_local_and_player_ownership() {
    packet::check_authored();
    let c = packet::consumer();
    assert_eq!(c.modifier_program.context, RuleEntityKind::EquipmentUse);
    assert_eq!(c.template_program.context, RuleEntityKind::EquipmentUse);
    assert_eq!(c.query.stat.key().as_str(), packet::LOCAL);
    assert_eq!(c.query.groups[0].reduction, ContributionReduction::Sum);
    assert_eq!(c.query.groups[0].ordering, ContributionOrdering::Ordered);
    assert!(matches!(&c.modifier_program.effects[0].effect,
        RuleEffectKind::Contribute { entity: RuleEntity::Current, stat, contribution: ContributionKind::Add, .. }
        if stat.key().as_str() == packet::LOCAL));
    assert!(matches!(&c.template_program.effects[0].effect,
        RuleEffectKind::Derive { entity: RuleEntity::Current, stat, .. }
        if stat.key().as_str() == packet::CAPACITY));
    assert!(matches!(&c.template_program.effects[1].effect,
        RuleEffectKind::Contribute { entity: RuleEntity::Player, stat, contribution: ContributionKind::Add, .. }
        if stat.key().as_str() == packet::PLAYER));
    assert_eq!(c.modifier_program.effects.len(), 1);
    assert_eq!(c.template_program.effects.len(), 2);
}

#[test]
fn candidate_preserves_all_independent_owner_and_supplier_obligations() {
    packet::check_authored();
    let d = packet::dependencies();
    let SchemaClosure::Partial { gaps } = &d.owners[0].programs.closure else {
        panic!()
    };
    assert_eq!(
        gaps.iter()
            .filter(|g| g.code.as_str() == packet::RETIRED)
            .count(),
        1
    );
    assert_eq!(gaps.len(), 5);
    assert!(!packet::consumer().query.groups[0].members.is_complete());
    let c = packet::consumer();
    for d in c.definitions {
        let DefinitionDescriptor::Stat(d) = d else {
            panic!()
        };
        let SchemaState::Known(s) = d.schema else {
            panic!()
        };
        assert!(
            matches!(s.value, ComputedValueType::Quantity { unit } if unit.key().as_str() == "def.000000000000295a")
        );
    }
    assert_eq!(
        c.template_program.nodes.len(),
        3,
        "no final cap or override consumer is claimed"
    );
}

/// An explicit offline rehearsal against the exact authenticated source snapshot.
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_CHARM_CAPACITY_PRIOR; provisional IDs must be reauthored for publication"]
fn exact_recipe_transition_rejects_drift_preserves_inverse_and_compiles() {
    let path = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_CHARM_CAPACITY_PRIOR").unwrap());
    let read = |name: &str| -> Value {
        serde_json::from_slice(&fs::read(path.join(name)).unwrap()).unwrap()
    };
    let prior: OwnedRecipeInput = serde_json::from_value(serde_json::json!({
        "schema_version":1, "registry":read("registry.json"), "schema":read("schema.json"),
        "rules":read("rules.json"), "routing":read("routing.json")
    }))
    .unwrap();
    let next = packet::staged_recipe(&prior).unwrap();
    assert_eq!(next.registry.last_issued.get(), 13263);
    assert!(packet::staged_recipe(&next).is_err());
    let d = packet::dependencies();
    let owner = next
        .rules
        .owners
        .iter()
        .find(|o| o.owner == d.owners[0].owner)
        .unwrap();
    let SchemaClosure::Partial { gaps } = &owner.programs.closure else {
        panic!()
    };
    assert_eq!(gaps.len(), 4);
    let mut stale = prior.clone();
    stale
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == d.owners[0].owner)
        .unwrap()
        .programs
        .members
        .pop();
    assert!(packet::staged_recipe(&stale).is_err());
    let mut foreign = prior;
    foreign.routing.release =
        poe_optimizer_core::owned_definitions::OwnedDefinitionKey::new("foreign-snapshot").unwrap();
    assert!(
        packet::staged_recipe(&foreign).is_err(),
        "whole prior is bound, not only copied owners"
    );
    assemble_owned_recipe(next, Default::default()).unwrap();
}
