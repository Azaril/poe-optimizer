//! Actual passive contributions and existing Player-to-Sniper receivers in an
//! explicitly finite component. This is neither allocation legality nor DPS parity.
#[path = "support/plain_minion_damage_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_binding::*, owned_build::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;
use std::{collections::BTreeSet, sync::Arc};

#[test]
fn all_reviewed_unconditional_nodes_reach_the_existing_exact_receivers() {
    let bindings = bindings();
    assert_eq!(bindings.nodes.len(), 41);
    assert_eq!(bindings.player_stat, def(0x1d33));
    assert_eq!(bindings.actor_stat, def(0x1d34));
    assert_eq!(bindings.unit, def(2));
    let records: Vec<Value> = asset("source-records.json");
    let source_ids: BTreeSet<_> = bindings.nodes.iter().map(|n| &n.source_id).collect();
    assert_eq!(source_ids.len(), 41);
    assert!(!source_ids.iter().any(|id| id.as_str() == "25927"));
    for node in &bindings.nodes {
        let observed = records
            .iter()
            .find(|r| r["id"].as_u64().unwrap().to_string() == node.source_id)
            .unwrap();
        let mods: Vec<_> = observed["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| {
                m["name"] == "MinionModifier"
                    && m["type"] == "LIST"
                    && m["value"]["mod"]["name"] == "Damage"
                    && m["value"]["mod"]["type"] == "INC"
            })
            .collect();
        assert_eq!(mods.len(), 1, "source node {}", node.source_id);
        assert_eq!(mods[0]["value"]["mod"]["value"].as_f64(), Some(node.value));
        assert_eq!(mods[0]["flags"], 0);
        assert_eq!(mods[0]["keyword_flags"], 0);
        assert!(mods[0]["tags"].as_object().unwrap().is_empty());
        check(
            &World::new(std::slice::from_ref(node)).evaluate(),
            node.value,
        );
    }
}

#[test]
fn original_selected_ten_and_actual_pruning_control_sum_without_recipient_multiplication() {
    let selected = selected();
    assert_eq!(selected.len(), 10);
    assert_eq!(selected.iter().map(|n| n.value).sum::<f64>(), 68.);
    check(&World::new(&selected).evaluate(), 68.);
    // The source's removal of95 also prunes8737. This does not claim that a
    // disconnected set is a legal build; it replays the measured producer set.
    let pruned: Vec<_> = selected
        .into_iter()
        .filter(|n| !["95", "8737"].contains(&n.source_id.as_str()))
        .collect();
    assert_eq!(pruned.len(), 8);
    check(&World::new(&pruned).evaluate(), 48.);
    check(&World::new(&[]).evaluate(), 0.);
}

#[test]
fn contribution_activation_follows_exact_allocation_loadout_scope() {
    let node = bindings()
        .nodes
        .into_iter()
        .find(|n| n.source_id == "95")
        .unwrap();
    let mut world = World::new(&[node]);
    let loadouts = world.intrinsic.f.build.weapon_loadouts.clone();
    world.intrinsic.f.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![loadouts[1]],
    };
    check(&world.evaluate(), 0.);
    world.intrinsic.f.build.active_weapon_loadout = loadouts[1];
    check(&world.evaluate(), 10.);
    world.intrinsic.f.build.active_weapon_loadout = loadouts[0];
    check(&world.evaluate(), 0.);
}

#[test]
fn actual_ascendancy_pool_remains_distinct_from_ordinary_points() {
    let node = bindings()
        .nodes
        .into_iter()
        .find(|n| n.source_id == "762")
        .unwrap();
    assert_eq!(node.pool, def(0x1bf1));
    let mut world = World::new(&[node]);
    assert_eq!(world.binding().schema(), SchemaBindingStatus::Valid);
    world.intrinsic.f.build.allocations[0].pool = def(0x1bf0);
    assert!(
        world
            .binding()
            .issues()
            .iter()
            .any(|i| { i.class == IssueClass::Invalid && i.code == BindingIssueCode::NotDeclared })
    );
    world.intrinsic.f.build.allocations[0].pool = def(0x1bf1);
    world.intrinsic.f.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![world.intrinsic.f.build.active_weapon_loadout],
    };
    assert!(world.binding().issues().iter().any(|i| {
        i.class == IssueClass::Invalid && i.code == BindingIssueCode::IncompatibleScope
    }));
}

#[test]
fn component_closure_does_not_promote_actual_partial_owners_or_other_node_lines() {
    let extension: Value = asset("extension.json");
    assert!(extension["schema"].as_array().unwrap().is_empty());
    assert!(extension["tables"].as_array().unwrap().is_empty());
    assert!(extension["receivers"].as_array().unwrap().is_empty());
    assert!(extension["operations_version"].is_null());
    let actual = owners();
    assert!(actual.iter().all(|o| !o.programs.is_complete()));
    let selected = selected();
    let selected_owner = actual
        .iter()
        .find(|o| o.owner == SchemaSubject::Definition(selected[0].node.address()))
        .unwrap();
    let mut world = World::new(&selected);
    world
        .intrinsic
        .f
        .owner_mut(&selected_owner.owner)
        .programs
        .closure = selected_owner.programs.closure.clone();
    let report = world.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    let records: Vec<Value> = asset("source-records.json");
    assert_eq!(
        records
            .iter()
            .filter(|r| r["stats"].as_array().unwrap().len() > 1)
            .count(),
        21
    );
}

#[test]
fn immutable_native_plans_reuse_scratch_and_parallel_workers_without_cross_candidate_state() {
    let selected = selected();
    let plan = Arc::new(World::new(&selected).compile());
    let mut scratch = plan.new_scratch();
    let first = plan.evaluate(&mut scratch).unwrap();
    check(&first, 68.);
    let empty = World::new(&[]).compile();
    check(&empty.evaluate(&mut scratch).unwrap(), 0.);
    assert_eq!(first, plan.evaluate(&mut scratch).unwrap());
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || plan.new_scratch(),
            |scratch, _| plan.evaluate(scratch).unwrap(),
        )
        .collect();
    assert!(reports.iter().all(|r| r == &first));
}
