//! Published Action consumers in the same item/preparation/population graph.
//! This finite component admits Command and shared minion increase only; it
//! does not complete physical/fire damage, cooldowns, tree legality or DPS.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
};
use std::path::Path;

const REDUCE: &str = "command-damage-reduce";
const RECEIVE: &str = "command-damage-receive";
const APPLY: &str = "command-damage-action";
const CONSUME: &str = "action-minion-damage";
#[derive(Clone)]
pub(super) struct Census {
    sources: Vec<(String, Allocation)>,
}
pub(super) fn action(w: &World, index: usize, set: Option<u64>) -> ActionSelection {
    let mut a = w.action(index);
    if let Some(set) = set {
        a.action.provider.grant_path[2] = slot(SlotOwnerDefId::Actor(d(0x3091)), 0x3095);
        a.action.output = slot(SlotOwnerDefId::Skill(d(0x24)), 0x25);
        a.stat_set = d(set);
    }
    a
}
fn sets() -> [Option<u64>; 4] {
    [None, Some(0x32ec), Some(0x32ed), Some(0x32ee)]
}
pub(super) fn select(w: &World, queries: &mut Vec<MetricRequest>) {
    for i in 0..2 {
        for set in sets().into_iter().flatten() {
            queries.push(MetricRequest {
                id: QueryId::new(format!("joined-command-{i}-{set:x}")).unwrap(),
                metric: def("fixture.observe"),
                target: MetricTarget::Action(Box::new(action(w, i, Some(set)))),
            });
        }
    }
}
pub(super) fn install(w: &mut sniper::World, endpoint: &StagedOwnedRelease, path: &Path) -> Census {
    action_damage_family::assert_component(endpoint);
    let deps: action_damage_family::Dependencies = action_damage_family::read("dependencies.json");
    let recipe = &endpoint.input().recipe;
    let xml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let allocations = passive_damage_evidence::normalized_selected_allocations(path, &xml);
    let bindings: Value = shared::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/command-damage/bindings.json"),
    );
    let f = &mut w.base.source.base.inner;
    assert!(
        RuleOperationsVersion::parse(f.operations.as_str())
            .unwrap()
            .supports_action_contribution_queries()
    );
    let mut definitions = deps.definitions;
    for entry in action_damage_family::migration().schema {
        let SchemaExtensionEntry::Definition(d) = entry else {
            panic!()
        };
        definitions.push(d);
    }
    for actual in definitions {
        let finite: DefinitionDescriptor = sniper::offering::prolonged::finite(&actual);
        if let Some(old) = f
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == finite.address())
        {
            assert_eq!(old, &finite);
        } else {
            f.schema.definitions.push(finite.clone());
        }
        f.owner_mut(SchemaSubject::Definition(finite.address()));
    }
    for actual in deps.slots {
        let finite: SlotDescriptor = sniper::offering::prolonged::finite(&actual);
        assert!(
            !f.schema
                .slots
                .iter()
                .any(|s| s.address() == finite.address())
        );
        f.owner_mut(SchemaSubject::Slot(finite.address()));
        f.schema.slots.push(finite);
    }
    // Restore the actual two supplied abilities in this finite Actor's schema.
    let actor = recipe
        .schema
        .definitions
        .iter()
        .find(|row| row.address() == d::<ActorDefinition>(0x3091).address())
        .unwrap();
    *f.schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == actor.address())
        .unwrap() = sniper::offering::prolonged::finite(actor);
    let actor_slot = recipe
        .schema
        .slots
        .iter()
        .find(|s| s.address() == SlotAddress::Actor(super::actor_slot()))
        .unwrap();
    *f.schema
        .slots
        .iter_mut()
        .find(|s| s.address() == actor_slot.address())
        .unwrap() = sniper::offering::prolonged::finite(actor_slot);
    let mut sources = Vec::new();
    for (i, binding) in bindings["nodes"].as_array().unwrap().iter().enumerate() {
        let node: PassiveNodeDefId = decode(&binding["definition"]);
        let selected: Vec<_> = allocations
            .iter()
            .filter(|a| a.node.to_resolved() == Some(node.clone()))
            .collect();
        assert_eq!(selected.len(), 1);
        let original = selected[0]
            .to_resolved()
            .expect("selected Command allocation is resolved");
        assert_eq!(original.access, AllocationAccess::Ordinary);
        assert_eq!(original.scope, LoadoutScope::Shared);
        assert!(original.choices.is_empty());
        let mut finite = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == node.address())
            .unwrap()
            .clone();
        let DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = &mut finite
        else {
            panic!()
        };
        assert_eq!(
            schema.pools.members.as_slice(),
            std::slice::from_ref(&original.pool)
        );
        // Numerical source census only: retain real pool/access/choices, exclude
        // topology from this fixture. Production keeps the original adjacency.
        schema.adjacent = DeclaredSet::complete(vec![]);
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|d| d.address() == finite.address())
        );
        f.schema.definitions.push(finite);
        f.owner_mut(subject(node));
        let mut selected = original.clone();
        // 7700 is reserved by the independent inherent-attribute flag controls.
        selected.id = id(8300 + i as u64);
        assert!(
            !f.build
                .allocations
                .iter()
                .any(|a| a.id == selected.id || a.node == selected.node)
        );
        f.build.allocations.push(selected.clone());
        sources.push((binding["source_id"].as_str().unwrap().into(), selected));
    }
    for row in deps.source_programs {
        let owner = f.owner_mut(row.owner);
        assert!(
            !owner
                .programs
                .members
                .iter()
                .any(|p| p.id == row.program.id)
        );
        owner.programs.members.push(row.program);
    }
    for owner in action_damage_family::consumer().owners {
        f.owner_mut(owner.owner)
            .programs
            .members
            .extend(owner.programs.members);
    }
    let partition = action_damage_family::partition();
    f.owner_mut(partition.owner)
        .programs
        .members
        .extend([partition.numerical.program, partition.activation.program]);
    w.receivers.members.extend(deps.receivers);
    w.base
        .contribution_queries
        .members
        .extend(action_damage_family::queries());
    // No Gas Arrow weapon/damage route is admitted by this increase-only fixture.
    // Current and Actor reads above have exact bound occurrences without routes.
    w.base.action_routes.push(ActionOutputRoutes {
        output: slot(SlotOwnerDefId::Skill(d(0x24)), 0x25),
        routes: DeclaredSet::complete(vec![]),
        source_selectors: Some(DeclaredSet::complete(vec![])),
    });
    Census { sources }
}
pub(super) fn configure(s: &mut EvaluationStagesInput) {
    for (name, before) in [
        (REDUCE, vec!["deliver"]),
        (RECEIVE, vec![REDUCE]),
        (APPLY, vec![RECEIVE]),
        (CONSUME, vec![APPLY, "mixed-minion-damage"]),
    ] {
        s.stages.push(EvaluationStage {
            id: key(name),
            predecessors: before.into_iter().map(key).collect(),
        });
    }
    for row in &mut s.programs.members {
        row.stage = match row.program.as_str() {
            "player-owned-command-damage" => key(REDUCE),
            "sniper-received-command-damage" => key(RECEIVE),
            "conditional-command-damage" => key(APPLY),
            action_damage_family::PROGRAM | action_damage_family::FACTOR => key(CONSUME),
            _ => row.stage.clone(),
        };
    }
    s.frozen_channels.extend([
        FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: d(0x3302),
                contribution: ContributionKind::Increase,
            },
            stage: key("deliver"),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x3302),
            },
            stage: key(REDUCE),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x3303),
            },
            stage: key(RECEIVE),
        },
        FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Action,
                stat: d(0x3304),
                contribution: ContributionKind::Increase,
            },
            stage: key(APPLY),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x3353),
            },
            stage: key("mixed-minion-damage"),
        },
    ]);
}
fn value<'a>(r: &'a SupportEffectsReport, a: &ActionSelection, stat: u64) -> &'a EffectValue {
    &sniper::offering::effects(r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Action(Box::new(a.clone())),
                    stat: d(stat),
                }
        })
        .unwrap()
        .value
}
fn check(w: &World, r: &SupportEffectsReport, increases: [f64; 2], command: f64) {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let e = sniper::offering::effects(r);
    assert!(e.gaps.is_empty(), "{:?}", e.gaps);
    assert_eq!(
        e.effects
            .iter()
            .filter(
                |e| [action_damage_family::PROGRAM, action_damage_family::FACTOR]
                    .contains(&e.key.invocation.program.as_str())
            )
            .count(),
        16
    );
    for (i, shared) in increases.into_iter().enumerate() {
        let mut provider = action(w, i, Some(0x32ec)).action.provider;
        provider.grant_path.pop();
        let generated = GeneratedSkillKey {
            provider: provider.clone(),
            slot: slot(SlotOwnerDefId::Actor(d(0x3091)), 0x3094),
        };
        let projected = PlanValueKey::SkillParameter {
            skill: Box::new(generated),
            parameter: slot(SlotOwnerDefId::Skill(d(0x24)), 0x309b),
        };
        let writes: Vec<_> = e
            .effects
            .iter()
            .filter(|r| {
                matches!(&r.target,
            BoundEffectTarget::Value { key } if *key == projected)
            })
            .collect();
        assert_eq!(
            writes.len(),
            1,
            "one exact published Gas actor-level projection"
        );
        assert_eq!(writes[0].key.invocation.program, key("gas-arrow-supply"));
        assert_eq!(
            writes[0].key.invocation.origin,
            RuleOrigin::Provider { provider }
        );
        assert_eq!(&writes[0].value, w.value(e, i, false, 0x1c));
        for set in sets() {
            let a = action(w, i, set);
            let increase = shared + if set.is_some() { command } else { 0. };
            assert_eq!(
                value(r, &a, 0x335c),
                &EffectValue::Known {
                    value: quantity(increase, &d(2))
                }
            );
            assert_eq!(
                value(r, &a, 0x335d),
                &EffectValue::Known {
                    value: quantity(1. + increase / 100., &d(1))
                }
            );
        }
    }
}
fn remove(w: &mut World, sources: &[String]) {
    let ids: Vec<_> = w
        .command
        .sources
        .iter()
        .filter(|(s, _)| sources.contains(s))
        .map(|(_, a)| a.id)
        .collect();
    w.sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .retain(|a| !ids.contains(&a.id));
}
#[test]
#[ignore = "requires current Action-damage release and authenticated Command observations"]
fn action_increase_matches_all_source_modes_and_branch_controls() {
    action_damage_family::check_source(true);
    let original = World::load();
    for vector in action_damage_family::vectors() {
        let mut w = original.clone();
        let removed: Vec<String> = vector["removed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap().to_string())
            .collect();
        remove(&mut w, &removed);
        let report = w.evaluate();
        let command = vector["command_contribution"].as_f64().unwrap();
        let set = if vector["effect_id"] == "MinionMeleeBow" {
            None
        } else {
            Some([0x32ec, 0x32ed, 0x32ee][vector["stat_set"].as_u64().unwrap() as usize - 1])
        };
        // Basic ignores Command even when the actual tree retains its sources.
        // The complete eight-Action check uses the saved allocation subtotal.
        let actual_command: f64 = w
            .command
            .sources
            .iter()
            .filter(|(_, a)| {
                w.sniper
                    .base
                    .source
                    .base
                    .inner
                    .build
                    .allocations
                    .iter()
                    .any(|row| row.id == a.id)
            })
            .map(|(s, _)| if s == "41511" { 15. } else { 20. })
            .sum();
        check(&w, &report, [130.; 2], actual_command);
        for i in 0..2 {
            let a = action(&w, i, set);
            assert_eq!(
                value(&report, &a, 0x335c),
                &EffectValue::Known {
                    value: quantity(vector["subtotal"].as_f64().unwrap(), &d(2))
                }
            );
            assert_eq!(
                value(&report, &a, 0x335d),
                &EffectValue::Known {
                    value: quantity(vector["increase_factor"].as_f64().unwrap(), &d(1))
                }
            );
            if set.is_some() {
                assert_eq!(actual_command, command);
            }
        }
    }
}
#[test]
#[ignore = "requires current Action-damage release; exact recipients, unknowns and workers"]
fn action_increase_tracks_recipient_quality_stacking_and_parallel_scratch() {
    let a = World::load();
    let pa = a.plan();
    let mut b = a.clone();
    b.sniper.raw(0, 20, 1., 0.);
    b.sniper.raw(1, 20, 20., 0.);
    let pb = b.plan();
    let mut unknown = a.clone();
    unknown.offering.preferences.remove(0);
    let pu = unknown.plan();
    let mut scratch = pa.new_scratch();
    let ra = pa.evaluate(&mut scratch).unwrap();
    check(&a, &ra, [130.; 2], 55.);
    let rb = pb.evaluate(&mut scratch).unwrap();
    check(&b, &rb, [130.; 2], 55.);
    let ru = pu.evaluate(&mut scratch).unwrap();
    for i in 0..2 {
        for set in sets() {
            for stat in [0x335c, 0x335d] {
                assert!(matches!(
                    value(&ru, &action(&unknown, i, set), stat),
                    EffectValue::Unresolved { .. }
                ));
            }
        }
    }
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
    let mut disabled = a.clone();
    for i in 0..2 {
        offering_application_native::override_active(&mut disabled, i, false);
    }
    check(&disabled, &disabled.evaluate(), [68.; 2], 55.);
    for levels in [[30, 20], [20, 30]] {
        let mut higher = a.clone();
        for (i, level) in levels.into_iter().enumerate() {
            higher.sniper.base.raw(3 + i, level, 0.);
        }
        check(&higher, &higher.evaluate(), [148.; 2], 55.);
    }
    let mut reordered = a.clone();
    reordered
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .reverse();
    reordered.offering.preferences.reverse();
    assert_eq!(reordered.evaluate(), ra);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |s, i| match i % 3 {
                    0 => pa.evaluate(s).unwrap(),
                    1 => pb.evaluate(s).unwrap(),
                    _ => pu.evaluate(s).unwrap(),
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, r) in results.into_iter().enumerate() {
        assert_eq!(
            r,
            *match i % 3 {
                0 => &ra,
                1 => &rb,
                _ => &ru,
            }
        );
    }
}
#[test]
#[ignore = "requires current Action-damage release; membership and scheduling denial"]
fn action_damage_denies_unknown_membership_and_premature_reads() {
    let w = World::load();
    for i in 0..2 {
        let mut bad = w.clone();
        let q = bad
            .sniper
            .base
            .contribution_queries
            .members
            .iter_mut()
            .find(|q| q.id == key(action_damage_family::QUERY))
            .unwrap();
        q.groups[0].members.members.remove(i);
        assert!(
            bad.checked_plan().is_err(),
            "even Basic's zero writer must be admitted"
        );
    }
    assert!(
        w.checked_plan_configured(|s| {
            for row in &mut s.programs.members {
                if row.program == key(action_damage_family::PROGRAM) {
                    row.stage = key("facts");
                }
            }
        })
        .is_err()
    );
}
