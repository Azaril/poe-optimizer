//! Actual four-node Life producers in the existing finite passive component.
//! The test-only reducer measures a closed contribution inventory, not received
//! minion Life, a Life pool formula, tree legality or a complete original build.
#[path = "support/plain_minion_damage_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::StatDefinition, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

fn packet<T: DeserializeOwned>(name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/plain-minion-life-passives")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn published() -> Vec<DefinitionRules> {
    let data: Value = packet("closure.json");
    serde_json::from_value(data["owners"].clone()).unwrap()
}
fn nodes() -> Vec<Node> {
    let owners = published();
    let nodes: Vec<_> = selected()
        .into_iter()
        .filter(|n| {
            owners
                .iter()
                .any(|o| o.owner == SchemaSubject::Definition(n.node.address()))
        })
        .collect();
    assert_eq!(nodes.len(), 4);
    assert!(nodes.iter().all(|n| n.value == 6.0));
    nodes
}
fn world(nodes: &[Node], reducer: bool) -> World {
    let mut world = World::new(nodes);
    let data: Value = packet("closure.json");
    let descriptors: Vec<DefinitionDescriptor> =
        serde_json::from_value(data["definitions"].clone()).unwrap();
    let stats: Vec<_> = descriptors
        .into_iter()
        .filter(|d| matches!(d, DefinitionDescriptor::Stat(_)))
        .collect();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].address(), def::<StatDefinition>(0x32e5).address());
    world.intrinsic.f.schema.definitions.extend(stats);
    for owner in published() {
        assert!(owner.programs.is_complete());
        let prior = owners()
            .into_iter()
            .find(|p| p.owner == owner.owner)
            .unwrap();
        assert!(!prior.programs.is_complete());
        assert_eq!(
            owner.programs.members.len(),
            prior.programs.members.len() + 1
        );
        assert_eq!(
            owner.programs.members[..prior.programs.members.len()],
            prior.programs.members
        );
        let subject = owner.owner.clone();
        *world.intrinsic.f.owner_mut(&subject) = owner;
    }
    if reducer {
        // This reducer is intentionally absent from the published packet: its
        // empty identity is authorized only by this finite test's input census.
        let key = intrinsic::key;
        world.intrinsic.f.owners.push(DefinitionRules {
            owner: SchemaSubject::Definition(def::<StatDefinition>(0x32e5).address()),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("fixture-minion-life-total"),
                context: RuleEntityKind::Actor,
                reads: vec![RuleRead {
                    id: key("incoming"),
                    value_type: ComputedValueType::Quantity { unit: def(2) },
                    source: RuleReadSource::Contributions {
                        entity: RuleEntity::Current,
                        stat: def(0x32e5),
                        contribution: ContributionKind::Increase,
                        reduction: ContributionReduction::Sum,
                        empty: intrinsic::quantity(0.0, 2),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("total"),
                    expression: RuleExpression::Read {
                        input: key("incoming"),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("final"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: def(0x32e5),
                        value: key("total"),
                    },
                }],
            }]),
        });
        world.intrinsic.f.receivers.members.push(StatReceiver {
            id: key("fixture-minion-life-reducer"),
            stat: def(0x32e5),
            program: key("fixture-minion-life-total"),
            targets: vec![StatReceiverTarget::Player],
        });
    }
    world
}
fn contributions(report: &OwnedEffectsReport) -> Vec<&BoundEffectResult> {
    report
        .effects
        .iter()
        .filter(|effect| {
            matches!(&effect.target,
                BoundEffectTarget::Contribution { key } if key.stat == def(0x32e5)
            )
        })
        .collect()
}
fn check_life(report: &OwnedEffectsReport, count: usize) {
    check(report, 6.0 * count as f64); // The old Damage producer is unchanged.
    let incoming = contributions(report);
    assert_eq!(incoming.len(), count);
    let mut origins = BTreeSet::new();
    for effect in incoming {
        let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
            panic!("the original allocation remains the contribution origin")
        };
        let ProviderRoot::Allocation(allocation) = provider.root else {
            panic!("only actual selected allocations contribute")
        };
        assert!(provider.grant_path.is_empty());
        assert!(origins.insert(allocation), "one contribution per source");
        assert_eq!(
            effect.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(0x32e5),
                    kind: ContributionKind::Increase,
                }
            }
        );
        assert_eq!(known(&effect.value), 6.0);
    }
    assert_eq!(
        known(value(
            report,
            ConcreteEntity::Actor(ActorKey::Player),
            0x32e5
        )),
        6.0 * count as f64
    );
    assert!(
        !report.values.iter().any(|r| matches!(&r.key,
            PlanValueKey::Stat { entity: ConcreteEntity::Actor(ActorKey::Owned(_)), stat }
                if *stat == def(0x32e5)
        )),
        "no fabricated Minion Life receiver"
    );
}
fn incomplete(report: &OwnedEffectsReport) {
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(matches!(
        value(report, ConcreteEntity::Actor(ActorKey::Player), 0x32e5),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}

#[test]
fn published_producers_do_not_claim_a_scalar_or_received_life_result() {
    let nodes = nodes();
    let report = world(&nodes, false).evaluate();
    check(&report, 24.0);
    assert_eq!(contributions(&report).len(), 4);
    assert!(!report.values.iter().any(|r| matches!(&r.key,
        PlanValueKey::Stat { stat, .. } if *stat == def(0x32e5)
    )));
}

#[test]
fn four_actual_nodes_and_each_removal_keep_life_and_damage_channels_separate() {
    let nodes = nodes();
    check_life(&world(&nodes, true).evaluate(), 4);
    for omitted in 0..4 {
        let remaining: Vec<_> = nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != omitted)
            .map(|(_, n)| n.clone())
            .collect();
        check_life(&world(&remaining, true).evaluate(), 3);
    }
    check_life(&world(&[], true).evaluate(), 0);
}

#[test]
fn selected_partial_owners_block_totals_but_unselected_catalog_rows_do_not() {
    let nodes = nodes();
    let subject = SchemaSubject::Definition(nodes[0].node.address());
    let mut prior = world(&nodes, true);
    *prior.intrinsic.f.owner_mut(&subject) =
        owners().into_iter().find(|o| o.owner == subject).unwrap();
    incomplete(&prior.evaluate());

    let extra = selected()
        .into_iter()
        .find(|n| !nodes.iter().any(|a| a.node == n.node))
        .unwrap();
    let subject = SchemaSubject::Definition(extra.node.address());
    let mut extended = nodes;
    extended.push(extra);
    let mut blocked = world(&extended, true);
    *blocked.intrinsic.f.owner_mut(&subject) =
        owners().into_iter().find(|o| o.owner == subject).unwrap();
    incomplete(&blocked.evaluate());
    blocked.intrinsic.f.build.allocations.pop().unwrap();
    check_life(&blocked.evaluate(), 4);
}

#[test]
fn exact_allocation_scope_and_parallel_scratch_never_share_candidate_state() {
    let nodes = nodes();
    let mut world = world(&nodes, true);
    let loadouts = world.intrinsic.f.build.weapon_loadouts.clone();
    world.intrinsic.f.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![loadouts[1]],
    };
    let three = world.compile();
    world.intrinsic.f.build.active_weapon_loadout = loadouts[1];
    let four = Arc::new(world.compile());
    let mut scratch = four.new_scratch();
    let original = four.evaluate(&mut scratch).unwrap();
    check_life(&original, 4);
    check_life(&three.evaluate(&mut scratch).unwrap(), 3);
    assert_eq!(original, four.evaluate(&mut scratch).unwrap());
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || four.new_scratch(),
            |scratch, _| four.evaluate(scratch).unwrap(),
        )
        .collect();
    assert!(reports.iter().all(|r| *r == original));
}
