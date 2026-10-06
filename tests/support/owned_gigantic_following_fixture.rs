//! Shared finite Gigantic fixture. The caller authenticates its full endpoint;
//! exact published status/default bodies are reused without numerical rewrites.
#[path = "../../crates/poe-optimizer-engine/tests/support/plain_minion_damage_fixture.rs"]
pub mod fixture;
use super::status_family;
use fixture::{Node, World, def, intrinsic};
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
    owned_release_migration::OwnedReleaseMigrationInput,
};
pub fn passive() -> SchemaSubject {
    SchemaSubject::Definition(def::<PassiveNodeDefinition>(0x1532).address())
}
pub fn receiver() -> SchemaSubject {
    SchemaSubject::Definition(def::<StatDefinition>(0x3308).address())
}
pub fn from_endpoint(endpoint: &StagedOwnedRelease) -> World {
    status_family::check_authored();
    let recipe = &endpoint.input().recipe;
    let descriptor = recipe
        .schema
        .definitions
        .iter()
        .find(|d| SchemaSubject::Definition(d.address()) == passive())
        .unwrap();
    let DefinitionDescriptor::PassiveNode(node) = descriptor else {
        panic!("passive")
    };
    let SchemaState::Known(schema) = &node.schema else {
        panic!("known passive")
    };
    assert_eq!(schema.pools.members.len(), 1);
    let mut world = World::new(&[Node {
        node: node.id.clone(),
        source_id: "46365".into(),
        value: 0.0,
        pool: schema.pools.members[0].clone(),
    }]);
    let f = &mut world.intrinsic.f;
    // Numerical rules remain byte-identical. Only graph adjacency and the
    // unrelated declarations of this existing finite fixture are narrowed.
    assert!(
        !f.schema
            .definitions
            .iter()
            .any(|d| d.address() == descriptor.address())
    );
    f.schema.definitions.push(fixture::finite_node(descriptor));
    let owner = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == passive())
        .unwrap();
    assert!(owner.programs.is_complete());
    let closed: serde_json::Value = status_family::read("closure.json");
    let expected: Vec<DefinitionRules> = serde_json::from_value(closed["owners"].clone()).unwrap();
    assert!(
        expected.contains(owner),
        "unchanged published Gigantic default body"
    );
    assert!(!f.owners.iter().any(|o| o.owner == owner.owner));
    f.owners.push(owner.clone());
    let migration: OwnedReleaseMigrationInput = status_family::read("migration.json");
    for entry in migration.schema {
        let SchemaExtensionEntry::Definition(row) = entry else {
            panic!("only stat definitions")
        };
        assert!(recipe.schema.definitions.contains(&row));
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|d| d.address() == row.address())
        );
        f.schema.definitions.push(row);
    }
    for owner in migration.owners {
        assert!(recipe.rules.owners.contains(&owner));
        assert_eq!(owner.owner, receiver());
        assert!(owner.programs.is_complete());
        assert!(!f.owners.iter().any(|o| o.owner == owner.owner));
        f.owners.push(owner);
    }
    assert_eq!(migration.receivers.len(), 1);
    for receiver in migration.receivers {
        assert!(recipe.rules.receivers.members.contains(&receiver));
        f.receivers.members.push(receiver);
    }
    // A test-owned mandatory read makes missing status distinguishable from
    // an explicitly computed false. It supplies no status or game formula.
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::Actor(
        intrinsic::actor_slot(),
    )))
    .programs
    .members
    .push(RuleProgram {
        id: intrinsic::key("fixture-status-read"),
        context: RuleEntityKind::Actor,
        reads: vec![RuleRead {
            id: intrinsic::key("active"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: def(0x3308),
            },
        }],
        nodes: vec![RuleNode {
            id: intrinsic::key("active"),
            expression: RuleExpression::Read {
                input: intrinsic::key("active"),
            },
        }],
        effects: vec![RuleEffect {
            id: intrinsic::key("observe"),
            when: None,
            effect: RuleEffectKind::Requirement {
                satisfied: intrinsic::key("active"),
                code: intrinsic::key("fixture-gigantic-active"),
            },
        }],
    });
    world
}
