#[allow(unused_imports)]
use super::{preservation, publication, release, selected};
#[allow(dead_code)]
#[path = "../support/owned_charm_capacity.rs"]
mod packet;
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use poe_optimizer_import::owned_recipe::{OwnedRecipeInput, assemble_owned_recipe};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[allow(unused_imports)]
use super::migration_preservation;

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

// Source metadata only: the independent native delivery tests remain unchanged.
#[allow(dead_code)] // stage is also consumed by the explicit shared publication gate.
#[path = "../support/owned_charm_scalability.rs"]
mod scalability;

#[test]
fn charm_scalability_retires_only_the_component_metadata_obligation() {
    scalability::check_authored();
    let before = scalability::expected_owner(false);
    let after = scalability::expected_owner(true);
    assert!(!before.programs.is_complete());
    assert!(!after.programs.is_complete());
    assert_eq!(before.programs.members, after.programs.members);
    assert_eq!(before.programs.members.len(), 5);
    let SchemaClosure::Partial { gaps: a } = before.programs.closure else {
        panic!()
    };
    let SchemaClosure::Partial { gaps: b } = after.programs.closure else {
        panic!()
    };
    assert_eq!((a.len(), b.len()), (4, 3));
    assert_eq!(
        a.iter()
            .filter(|g| g.code.as_str() == scalability::RETIRED)
            .count(),
        1
    );
    assert_eq!(
        b.iter()
            .filter(|g| g.code.as_str() == scalability::RETIRED)
            .count(),
        0
    );
    assert!(
        !packet::consumer().query.groups[0].members.is_complete(),
        "the separate local supplier census remains Partial"
    );
}

#[test]
fn charm_scalability_refuses_new_emitters_changed_guards_and_input_projection() {
    use serde_json::json;
    let c = scalability::certificate();
    let rules = json!([c["dependencies"]["item_rule"]]);
    let guards = json!([c["dependencies"]["source_condition"]]);
    scalability::check_admission(&rules, &guards);
    let mut additional = rules.clone();
    let mut unseen = additional[0].clone();
    unseen["id"] = json!("unselected-unreviewed-charm-emitter");
    additional.as_array_mut().unwrap().push(unseen);
    assert!(
        std::panic::catch_unwind(|| scalability::check_admission(&additional, &guards)).is_err()
    );
    let mut duplicate = rules.clone();
    duplicate.as_array_mut().unwrap().push(rules[0].clone());
    assert!(
        std::panic::catch_unwind(|| scalability::check_admission(&duplicate, &guards)).is_err()
    );
    let mut changed = guards.clone();
    changed[0]["all"].as_array_mut().unwrap().remove(0);
    assert!(std::panic::catch_unwind(|| scalability::check_admission(&rules, &changed)).is_err());
    let mut widened = guards.clone();
    widened[0]["all"][2]["value"]["min"] = json!(0);
    assert!(std::panic::catch_unwind(|| scalability::check_admission(&rules, &widened)).is_err());
    let mut input = rules.clone();
    let raw = input[0]["emissions"][0]["value"]["rolls"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["slot"]["slot"]["key"] == "def.0000000000003173")
        .unwrap();
    raw["value"] = json!({"kind":"literal","value":{"kind":"boolean","value":false}});
    assert!(std::panic::catch_unwind(|| scalability::check_admission(&input, &guards)).is_err());
    let mut plural = rules.clone();
    plural[0]["pattern"][4]["value"] = json!(") Charm Slots");
    assert!(
        std::panic::catch_unwind(|| scalability::check_admission(&plural, &guards)).is_err(),
        "corroborating plural metadata is not a new emitter admission"
    );
}

#[test]
fn charm_scalability_refuses_numerical_edits_and_other_obligation_retirement() {
    use serde_json::json;
    let c = scalability::certificate();
    let before = &c["dependencies"]["owner_before"];
    let after = &c["dependencies"]["owner_after"];
    scalability::check_owner_delta(before, after);
    let mut program = after.clone();
    program["programs"]["members"][3]["id"] = json!("different-effective-amount");
    assert!(std::panic::catch_unwind(|| scalability::check_owner_delta(before, &program)).is_err());
    let mut extra = after.clone();
    extra["programs"]["closure"]["value"]["gaps"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(std::panic::catch_unwind(|| scalability::check_owner_delta(before, &extra)).is_err());
    let mut complete = after.clone();
    complete["programs"]["closure"] = json!({"kind":"complete"});
    assert!(
        std::panic::catch_unwind(|| scalability::check_owner_delta(before, &complete)).is_err()
    );
}

#[test]
#[ignore = "optional offline acquisition check requires the exact pinned source checkout and original reports/packages"]
fn charm_scalability_reproduces_the_retained_source_acquisition() {
    scalability::check_acquisition();
}

#[test]
#[ignore = "requires CHARM_SCALABILITY_PRIOR, fresh CHARM_SCALABILITY_OUTPUT and pinned acquisition evidence"]
fn publish_charm_scalability_preserving_all_five_originals() {
    scalability::check_acquisition();
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_CHARM_SCALABILITY_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_CHARM_SCALABILITY_OUTPUT").expect("fresh output"),
        ),
        &scalability::data(),
        &[],
        &["scalability-certificate.json"],
        scalability::stage,
        serde_json::json!({
            "retired_obligations":1,"retained_obligations":3,
            "new_definitions":0,"new_programs":0,"numerical_program_changes":0,
            "import_guard_changes":0,"schema_identity_changed":false,
            "whole_build_parity":false,"final_charm_capacity":false,
            "expected_retired_item_text_diagnostics":0,
            "expected_retired_selected_input_issues":0
        }),
        // Historical-facet scope: the shared preservation check separately
        // authenticates numeric-order obligations and reports full current totals.
        [107, 118, 110, 124, 5],
    );
}
