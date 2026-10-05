//! Exact published conditional contributions in the existing finite two-Sniper
//! component. Parent final inputs and unrelated contributor inventories remain
//! explicit fixture boundaries. This proves neither cooldown duration nor a
//! complete original build, and creates no directly authored minion child.
#[path = "../crates/poe-optimizer-engine/tests/support/plain_minion_damage_fixture.rs"]
mod fixture;

use fixture::{Node, World, def, intrinsic, known, value};
use poe_optimizer_core::{
    owned_binding::*, owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::BTreeSet, fs, sync::Arc};

#[derive(Clone, Deserialize)]
struct CommandNode {
    definition: PassiveNodeDefId,
    source_id: String,
    amount: f64,
    program: OwnedDefinitionKey,
}
#[derive(Deserialize)]
struct Channels {
    player: StatDefId,
    actor: StatDefId,
    action: StatDefId,
    commandable: StatDefId,
}
#[derive(Deserialize)]
struct Programs {
    eligibility: OwnedDefinitionKey,
    action: OwnedDefinitionKey,
}
#[derive(Deserialize)]
struct GasSet {
    id: ActionStatSetDefId,
    source_index: u32,
    label: String,
    scope: String,
}
#[derive(Deserialize)]
struct Bindings {
    channels: Channels,
    percent_unit: UnitDefId,
    gas_stat_sets: Vec<GasSet>,
    nodes: Vec<CommandNode>,
    programs: Programs,
}
#[derive(Deserialize)]
struct Closure {
    definitions: Vec<DefinitionDescriptor>,
    owners: Vec<DefinitionRules>,
}
fn packet<T: DeserializeOwned>(name: &str) -> T {
    let path = intrinsic::repository_root()
        .join("data/owned/poe2/3887ae68/command-cooldown")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn bindings() -> Bindings {
    packet("bindings.json")
}
fn migration() -> OwnedReleaseMigrationInput {
    packet("migration.json")
}
fn closure() -> Closure {
    packet("closure.json")
}
fn output(skill: u64, slot: u64) -> DeclaredSlot<ActionOutputDefId> {
    intrinsic::slot(SlotOwnerDefId::Skill(def(skill)), slot)
}
fn action(index: usize, stat_set: Option<ActionStatSetDefId>) -> ActionSelection {
    let mut selected = intrinsic::selected(index);
    if let Some(stat_set) = stat_set {
        assert_eq!(selected.action.provider.grant_path.len(), 3);
        selected.action.provider.grant_path[2] =
            intrinsic::slot(SlotOwnerDefId::Actor(def(0x3091)), 0x3095);
        selected.action.output = output(0x24, 0x25);
        selected.stat_set = stat_set;
    }
    selected
}
fn actions(index: usize) -> Vec<ActionSelection> {
    let mut actions = vec![action(index, None)];
    actions.extend(
        bindings()
            .gas_stat_sets
            .into_iter()
            .map(|set| action(index, Some(set.id))),
    );
    actions
}
fn finite<T: Serialize + DeserializeOwned>(input: &T) -> T {
    fn close(value: &mut Value) {
        match value {
            Value::Object(object) => {
                if object.contains_key("members") && object.contains_key("closure") {
                    object.insert("closure".into(), serde_json::json!({"kind":"complete"}));
                }
                object.values_mut().for_each(close);
            }
            Value::Array(values) => values.iter_mut().for_each(close),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(input).unwrap();
    close(&mut value);
    serde_json::from_value(value).unwrap()
}
fn nodes() -> Vec<Node> {
    let declarations = closure().definitions;
    let damage = fixture::bindings();
    bindings()
        .nodes
        .into_iter()
        .map(|node| {
            if let Some(existing) = damage.nodes.iter().find(|n| n.node == node.definition) {
                assert_eq!(existing.source_id, node.source_id);
                return existing.clone();
            }
            let descriptor = declarations
                .iter()
                .find(|d| d.address() == node.definition.address())
                .unwrap();
            let DefinitionDescriptor::PassiveNode(entry) = descriptor else {
                panic!("exact passive descriptor")
            };
            let SchemaState::Known(schema) = &entry.schema else {
                panic!("known passive descriptor")
            };
            assert_eq!(schema.pools.members.len(), 1);
            Node {
                node: node.definition,
                source_id: node.source_id,
                value: 0.0,
                pool: schema.pools.members[0].clone(),
            }
        })
        .collect()
}
fn world(nodes: &[Node]) -> World {
    let mut world = World::new(nodes);
    let packet = migration();
    assert!(packet.tables.is_empty());
    let f = &mut world.intrinsic.f;
    for entry in packet.schema {
        match entry {
            SchemaExtensionEntry::Definition(descriptor) => {
                assert!(
                    !f.schema
                        .definitions
                        .iter()
                        .any(|d| d.address() == descriptor.address())
                );
                f.schema.definitions.push(descriptor);
            }
            SchemaExtensionEntry::Slot(descriptor) => {
                // Only the actual Gas output is replaced. Its complete finite
                // action choice census still does not close production mechanics.
                assert_eq!(
                    descriptor.address(),
                    SlotAddress::ActionOutput(output(0x24, 0x25))
                );
                let row = f
                    .schema
                    .slots
                    .iter_mut()
                    .find(|s| s.address() == descriptor.address())
                    .unwrap();
                *row = finite(&descriptor);
            }
        }
    }
    for definition in closure().definitions {
        let definition = fixture::finite_node(&definition);
        if let Some(row) = f
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == definition.address())
        {
            *row = definition;
        } else {
            f.schema.definitions.push(definition);
        }
    }
    for owner in packet.owners {
        if let Some(existing) = f.owners.iter_mut().find(|o| o.owner == owner.owner) {
            for program in owner.programs.members {
                if let Some(prior) = existing
                    .programs
                    .members
                    .iter()
                    .find(|p| p.id == program.id)
                {
                    assert_eq!(prior, &program, "actual prior program survives unchanged");
                } else {
                    existing.programs.members.push(program);
                }
            }
        } else {
            f.owners.push(finite(&owner));
        }
    }
    for owner in closure().owners {
        assert!(owner.programs.is_complete());
        if let Some(existing) = f.owners.iter_mut().find(|o| o.owner == owner.owner) {
            // Every already-installed program must survive the actual complete
            // owner, including the previous Minion Damage recipe byte-for-byte.
            for program in &existing.programs.members {
                assert!(owner.programs.members.contains(program));
            }
            *existing = owner;
        } else {
            f.owners.push(owner);
        }
    }
    f.receivers.members.extend(packet.receivers);
    let metric = f.queries.requests[0].metric.clone();
    for index in 0..2 {
        for set in bindings().gas_stat_sets {
            f.queries.requests.push(MetricRequest {
                id: QueryId::new(format!("gas-{index}-{}", set.source_index)).unwrap(),
                metric: metric.clone(),
                target: MetricTarget::Action(Box::new(action(index, Some(set.id)))),
            });
        }
    }
    // Preserve the physical-Gem roots and the actual parent Action queries.
    assert_eq!(f.build.skills.len(), 2);
    assert!(
        f.build
            .skills
            .iter()
            .all(|s| matches!(s.source, AuthoredSkillSource::Gem(_)))
    );
    assert_eq!(f.queries.requests.len(), 10);
    world
}
fn contributions<'a>(
    report: &'a OwnedEffectsReport,
    stat: &StatDefId,
) -> Vec<&'a BoundEffectResult> {
    report.effects.iter().filter(|effect| {
        matches!(&effect.target, BoundEffectTarget::Contribution { key } if &key.stat == stat)
    }).collect()
}
fn action_contribution<'a>(
    report: &'a OwnedEffectsReport,
    selected: &ActionSelection,
) -> &'a BoundEffectResult {
    let stat = bindings().channels.action;
    let matches: Vec<_> = contributions(report, &stat)
        .into_iter()
        .filter(|effect| {
            matches!(&effect.target, BoundEffectTarget::Contribution { key }
            if key.entity == ConcreteEntity::Action(Box::new(selected.clone())))
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "one exact conditional contribution per requested Action"
    );
    matches[0]
}
fn check_action(report: &OwnedEffectsReport, selected: &ActionSelection, expected: f64) {
    let effect = action_contribution(report, selected);
    assert_eq!(known(&effect.value), expected);
    let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
        panic!("Action must retain its generated provider path")
    };
    assert_eq!(provider, &selected.action.provider);
    assert_eq!(
        effect.key.invocation.entity,
        ConcreteEntity::Action(Box::new(selected.clone()))
    );
    assert_eq!(effect.key.invocation.program, bindings().programs.action);
    let BoundEffectTarget::Contribution { key } = &effect.target else {
        unreachable!()
    };
    assert_eq!(key.kind, ContributionKind::Increase);
}
fn check(world: &World, report: &OwnedEffectsReport, expected: f64) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(
        known(value(
            report,
            ConcreteEntity::Actor(ActorKey::Player),
            0x32e8
        )),
        expected
    );
    let binding = bindings();
    let damage: f64 = world
        .intrinsic
        .f
        .build
        .allocations
        .iter()
        .map(|allocation| {
            fixture::bindings()
                .nodes
                .into_iter()
                .find(|n| n.node == allocation.node)
                .map_or(0.0, |n| n.value)
        })
        .sum();
    fixture::check(report, damage);
    let actual = contributions(report, &binding.channels.player);
    assert_eq!(actual.len(), world.intrinsic.f.build.allocations.len());
    let mut seen = BTreeSet::new();
    for effect in actual {
        let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
            panic!("allocation origin")
        };
        let ProviderRoot::Allocation(id) = provider.root else {
            panic!("no synthetic contributor")
        };
        assert!(provider.grant_path.is_empty());
        assert!(seen.insert(id));
        let allocation = world
            .intrinsic
            .f
            .build
            .allocations
            .iter()
            .find(|a| a.id == id)
            .unwrap();
        let declared = binding
            .nodes
            .iter()
            .find(|n| n.definition == allocation.node)
            .unwrap();
        assert_eq!(
            effect.key.invocation.owner,
            SchemaSubject::Definition(allocation.node.address())
        );
        assert_eq!(effect.key.invocation.program, declared.program);
        assert_eq!(known(&effect.value), declared.amount);
    }
    for index in 0..2 {
        assert_eq!(known(fixture::actor_value(report, index, 0x32e9)), expected);
        assert_eq!(
            fixture::actor_value(report, index, 0x1c),
            &EffectValue::Known {
                value: ParameterValue::Integer(BoundedInteger::new([44, 2][index]).unwrap()),
            },
            "independent physical parents retain their distinct actor input"
        );
        for selected in actions(index) {
            let gas = selected.action.output == output(0x24, 0x25);
            check_action(report, &selected, if gas { expected } else { 0.0 });
            assert_eq!(
                value(report, ConcreteEntity::Action(Box::new(selected)), 0x32eb),
                &EffectValue::Known {
                    value: ParameterValue::Boolean(gas)
                },
            );
        }
    }
    assert_ne!(intrinsic::actor(0), intrinsic::actor(1));
    assert!(
        !report.values.iter().any(|v| matches!(&v.key,
            PlanValueKey::Stat { stat, .. } if *stat == binding.channels.action
        )),
        "packet publishes contributions, not a cooldown scalar or duration"
    );
}

#[test]
fn actual_packet_reaches_each_gas_stat_set_and_preserves_basic_zero() {
    let binding = bindings();
    assert_eq!(binding.channels.player, def(0x32e8));
    assert_eq!(binding.channels.actor, def(0x32e9));
    assert_eq!(binding.channels.action, def(0x32ea));
    assert_eq!(binding.channels.commandable, def(0x32eb));
    assert_eq!(binding.percent_unit, def(2));
    let observed: BTreeSet<_> = binding
        .nodes
        .iter()
        .map(|n| (n.source_id.as_str(), n.amount as u32))
        .collect();
    assert_eq!(
        observed,
        BTreeSet::from([
            ("14598", 8),
            ("4345", 8),
            ("43979", 8),
            ("50837", 8),
            ("6077", 20),
            ("35645", 20)
        ])
    );
    assert_eq!(binding.gas_stat_sets.len(), 3);
    for (index, set) in binding.gas_stat_sets.iter().enumerate() {
        assert_eq!(set.source_index as usize, index + 1);
        assert_eq!(set.id, def(0x32ec + index as u64));
        assert_eq!(set.label, ["Impact", "Poison Cloud", "Explosion"][index]);
        assert_eq!(set.scope, format!("sniper_gas_shot_statset_{index}"));
    }
    let all = nodes();
    let four: Vec<_> = all
        .iter()
        .filter(|n| {
            binding
                .nodes
                .iter()
                .any(|b| b.definition == n.node && b.amount == 8.0)
        })
        .cloned()
        .collect();
    assert_eq!(four.len(), 4);
    for (nodes, expected) in [(&four[..], 32.0), (&all[..], 72.0), (&[][..], 0.0)] {
        let world = world(nodes);
        check(&world, &world.evaluate(), expected);
    }
    for (omitted, node) in all.iter().enumerate() {
        let remaining: Vec<_> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != omitted)
            .map(|(_, n)| n.clone())
            .collect();
        let amount = binding
            .nodes
            .iter()
            .find(|n| n.definition == node.node)
            .unwrap()
            .amount;
        let world = world(&remaining);
        check(&world, &world.evaluate(), 72.0 - amount);
    }
}

#[test]
fn gas_uses_only_its_three_constructed_alternatives_without_relabeling_basic() {
    let mut world = world(&nodes());
    let target = world
        .intrinsic
        .f
        .queries
        .requests
        .iter_mut()
        .find(|q| q.id.as_str() == "gas-0-1")
        .unwrap();
    let MetricTarget::Action(selected) = &mut target.target else {
        unreachable!()
    };
    selected.stat_set = def(9);
    assert!(
        world
            .binding()
            .issues()
            .iter()
            .any(|issue| issue.class == IssueClass::Invalid
                && issue.code == BindingIssueCode::NotDeclared),
        "the old generic stat set is not silently accepted as a Gas alternative"
    );
    assert_eq!(action(0, None).stat_set, def(9));
}

#[test]
fn missing_eligibility_is_not_false_or_an_unconditional_carrier() {
    let mut world = world(&nodes());
    let programs = &mut world
        .intrinsic
        .f
        .owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(output(
            0x24, 0x25,
        ))))
        .programs
        .members;
    let before = programs.len();
    programs.retain(|p| p.id != bindings().programs.eligibility);
    assert_eq!(programs.len() + 1, before);
    let report = world.evaluate();
    assert!(
        report.gaps.is_empty(),
        "missing value producer is an execution result"
    );
    for index in 0..2 {
        check_action(&report, &action(index, None), 0.0);
        for set in bindings().gas_stat_sets {
            let selected = action(index, Some(set.id));
            assert!(
                matches!(
                    &action_contribution(&report, &selected).value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        read: Some(_)
                    }
                ),
                "{:?}",
                action_contribution(&report, &selected).value
            );
        }
    }
}

#[test]
fn missing_parent_final_inputs_and_missing_entering_grant_stay_unavailable() {
    let mut world = world(&nodes());
    world.intrinsic.missing_final_input();
    let report = world.evaluate();
    for index in 0..2 {
        for selected in actions(index) {
            assert!(
                matches!(
                    &action_contribution(&report, &selected).value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ),
                "missing parent final input: {:?}",
                action_contribution(&report, &selected).value
            );
        }
    }
    let mut world = self::world(&nodes());
    let actor = world
        .intrinsic
        .f
        .owner_mut(&SchemaSubject::Definition(DefinitionAddress::Actor(def(
            0x3091,
        ))));
    let producer = actor
        .programs
        .members
        .iter_mut()
        .find(|p| p.id.as_str() == "gas-arrow-supply")
        .unwrap();
    let before = producer.effects.len();
    producer.effects.retain(|e| !matches!(&e.effect, RuleEffectKind::ActivateGrant { slot, .. } if slot.slot == def(0x3095)));
    assert_eq!(producer.effects.len() + 1, before);
    let report = world.evaluate();
    for index in 0..2 {
        // An unproved selected grant keeps the whole-plan contributor gate
        // closed, including the otherwise independent Basic eligibility read.
        // Do not turn this missing source authority into a known zero.
        assert!(matches!(
            &action_contribution(&report, &action(index, None)).value,
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
        for set in bindings().gas_stat_sets {
            let actual = &action_contribution(&report, &action(index, Some(set.id))).value;
            assert!(
                matches!(
                    actual,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::IncompleteContributors,
                        ..
                    }
                ),
                "an unproved selected grant blocks the complete contributor inventory: {actual:?}"
            );
        }
    }
}

#[test]
fn disabled_physical_root_never_creates_an_active_second_actor() {
    let mut world = world(&nodes());
    world.intrinsic.f.build.skills[1].enabled = false;
    let root = ProviderRoot::SkillUse(world.intrinsic.f.build.skills[1].id);
    let plan = world.compile();
    let disabled: Vec<_> = plan
        .binding_report()
        .issues()
        .iter()
        .filter(|issue| issue.code == BindingIssueCode::DisabledProvider)
        .collect();
    assert_eq!(
        disabled.len(),
        5,
        "Basic, three Gas sets and the parent query retain unavailable selectors"
    );
    assert!(
        disabled
            .iter()
            .all(|issue| issue.class == IssueClass::Unavailable)
    );
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::UnresolvedTopology)
    );
    assert!(
        !report
            .effects
            .iter()
            .any(|effect| matches!(&effect.key.invocation.entity,
                ConcreteEntity::Action(selected) if selected.action.provider.root == root
            )),
        "disabled physical source must not create generated Action effects"
    );
    assert!(!report.values.iter().any(|entry| matches!(&entry.key,
        PlanValueKey::Stat {entity: ConcreteEntity::Actor(actor), ..} if *actor == intrinsic::actor(1)
    )), "unavailable source must not create a receiving Actor");
    // Remove only the deliberately disabled root's queries before asking for
    // known results from the independent active population.
    world.intrinsic.f.queries.requests.retain(|q| {
        !matches!(&q.target,
            MetricTarget::Action(selected) if selected.action.provider.root == root
        )
    });
    let report = world.evaluate();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for selected in actions(0) {
        check_action(
            &report,
            &selected,
            if selected.action.output == output(0x24, 0x25) {
                72.0
            } else {
                0.0
            },
        );
    }
    assert!(!report.effects.iter().any(|e| e.key.invocation.entity
        == ConcreteEntity::Actor(intrinsic::actor(1))
        && matches!(e.value, EffectValue::Known { .. })));
}

#[test]
fn actual_partial_action_program_inventory_keeps_the_global_gate_closed() {
    let partial_action = migration()
        .owners
        .into_iter()
        .find(|o| o.owner == SchemaSubject::Slot(SlotAddress::ActionOutput(output(0x24, 0x25))))
        .unwrap();
    assert!(!partial_action.programs.is_complete());
    let mut world = world(&nodes());
    world
        .intrinsic
        .f
        .owner_mut(&partial_action.owner)
        .programs
        .closure = partial_action.programs.closure;
    let report = world.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(matches!(
        value(&report, ConcreteEntity::Actor(ActorKey::Player), 0x32e8),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
    for index in 0..2 {
        for set in bindings().gas_stat_sets {
            assert!(matches!(
                &action_contribution(&report, &action(index, Some(set.id))).value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                }
            ));
        }
    }
}

#[test]
fn allocation_activation_and_reused_parallel_scratch_preserve_exact_candidates() {
    let mut world = world(&nodes());
    let first = world.intrinsic.f.build.allocations[0].clone();
    let amount = bindings()
        .nodes
        .into_iter()
        .find(|n| n.definition == first.node)
        .unwrap()
        .amount;
    let loadouts = world.intrinsic.f.build.weapon_loadouts.clone();
    world.intrinsic.f.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![loadouts[1]],
    };
    let without = world.compile();
    world.intrinsic.f.build.active_weapon_loadout = loadouts[1];
    let with = Arc::new(world.compile());
    let mut scratch = with.new_scratch();
    let original = with.evaluate(&mut scratch).unwrap();
    check(&world, &original, 72.0);
    let changed = without.evaluate(&mut scratch).unwrap();
    assert_eq!(
        known(value(
            &changed,
            ConcreteEntity::Actor(ActorKey::Player),
            0x32e8
        )),
        72.0 - amount
    );
    let removed = contributions(&changed, &bindings().channels.player).into_iter().find(|e| matches!(&e.key.invocation.origin,
        RuleOrigin::Provider { provider } if provider.root == ProviderRoot::Allocation(first.id)
    ));
    assert!(
        removed.is_none(),
        "out-of-loadout allocation does not enter this candidate's source census"
    );
    for index in 0..2 {
        for set in bindings().gas_stat_sets {
            check_action(&changed, &action(index, Some(set.id)), 72.0 - amount);
        }
    }
    assert_eq!(original, with.evaluate(&mut scratch).unwrap());
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || with.new_scratch(),
            |scratch, _| with.evaluate(scratch).unwrap(),
        )
        .collect();
    assert!(reports.iter().all(|r| r == &original));
}
