//! Published default-owner closure in the existing finite passive component.
//! This retains the fixture's explicit final-level and topology boundaries;
//! whole-tree legality and the complete original build are not evaluated here.
#[path = "support/plain_minion_damage_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::ActorKey,
    owned_rules::DefinitionRules,
    owned_schema::{SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_plan::*;
use serde_json::Value;
use std::{fs, path::PathBuf};

fn closure() -> Vec<DefinitionRules> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/plain-minion-owner-closure/closure.json");
    let packet: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    serde_json::from_value(packet["owners"].clone()).unwrap()
}
fn closed_nodes() -> Vec<Node> {
    let owners = closure();
    let result: Vec<_> = selected()
        .into_iter()
        .filter(|n| {
            owners
                .iter()
                .any(|o| o.owner == SchemaSubject::Definition(n.node.address()))
        })
        .collect();
    assert_eq!(result.len(), 2);
    assert_eq!(result.iter().map(|n| n.value).sum::<f64>(), 20.0);
    result
}
fn world(nodes: &[Node]) -> World {
    let mut world = World::new(nodes);
    for actual in closure() {
        assert!(actual.programs.is_complete());
        let prior = owners()
            .into_iter()
            .find(|p| p.owner == actual.owner)
            .unwrap();
        assert!(!prior.programs.is_complete());
        assert_eq!(actual.programs.members, prior.programs.members);
        let subject = actual.owner.clone();
        *world.intrinsic.f.owner_mut(&subject) = actual;
    }
    world
}
fn incomplete(report: &OwnedEffectsReport) {
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(matches!(
        value(report, ConcreteEntity::Actor(ActorKey::Player), 0x1d33),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
    for index in 0..2 {
        assert!(matches!(
            actor_value(report, index, 0x1d34),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
    }
}

#[test]
fn exact_published_owners_reach_existing_receivers_without_changing_arithmetic() {
    let nodes = closed_nodes();
    check(&world(&nodes).evaluate(), 20.0);
    for node in nodes {
        check(&world(&[node]).evaluate(), 10.0);
    }
}

#[test]
fn prior_partial_owner_still_prevents_a_known_contribution_total() {
    let nodes = closed_nodes();
    let mut world = world(&nodes);
    let subject = SchemaSubject::Definition(nodes[0].node.address());
    let prior = owners().into_iter().find(|o| o.owner == subject).unwrap();
    *world.intrinsic.f.owner_mut(&subject) = prior;
    incomplete(&world.evaluate());
}

#[test]
fn another_selected_partial_owner_blocks_the_plan_but_unselected_catalog_does_not() {
    let mut nodes = closed_nodes();
    let other = selected()
        .into_iter()
        .find(|n| !nodes.iter().any(|closed| closed.node == n.node))
        .unwrap();
    let subject = SchemaSubject::Definition(other.node.address());
    nodes.push(other);
    let mut world = world(&nodes);
    let prior = owners().into_iter().find(|o| o.owner == subject).unwrap();
    assert!(!prior.programs.is_complete());
    *world.intrinsic.f.owner_mut(&subject) = prior;
    let blocked = world.compile();
    let mut scratch = blocked.new_scratch();
    let before = blocked.evaluate(&mut scratch).unwrap();
    incomplete(&before);
    world.intrinsic.f.build.allocations.pop().unwrap();
    let selected_two = world.compile();
    check(&selected_two.evaluate(&mut scratch).unwrap(), 20.0);
    assert_eq!(before, blocked.evaluate(&mut scratch).unwrap());
}
