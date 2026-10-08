//! The selected unconditional passive subtotal joins the existing item-driven
//! Sniper graph. This finite ten-source census excludes reachability, the other
//! 31 family definitions, conditional passives, Offering and final damage.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};

const GRANTED: &str = "owned-minion-damage-granted";
const RECEIVED: &str = "received-owner-minion-damage";
const REDUCE: &str = "passive-damage-reduce";
const RECEIVE: &str = "passive-damage-receive";

#[derive(Clone, Deserialize)]
struct Binding {
    node: PassiveNodeDefId,
    value: f64,
    pool: PointPoolDefId,
}
#[derive(Deserialize)]
struct Bindings {
    player_stat: StatDefId,
    actor_stat: StatDefId,
    unit: UnitDefId,
    nodes: Vec<Binding>,
}
#[derive(Clone)]
pub(super) struct Census {
    bindings: Vec<Binding>,
    // The finite graph has its own allocator. Imported occurrence identities
    // remain recorded here; only their IDs are explicitly rehomed below.
    original: Vec<Allocation>,
    cases: Vec<passive_damage_evidence::PassiveCase>,
}
fn packet<T: serde::de::DeserializeOwned>(name: &str) -> T {
    shared::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/plain-minion-damage-passives")
            .join(name),
    )
}

pub(super) fn install(
    sniper: &mut sniper::World,
    endpoint: &StagedOwnedRelease,
    path: &Path,
) -> Census {
    passive_damage_evidence::authenticate_component(endpoint);
    let cases = passive_damage_evidence::normalized_allocations(path);
    let original = cases
        .iter()
        .find(|c| c.case_name == "original-05")
        .unwrap()
        .allocations
        .clone()
        .expect("all ten original selected allocations have resolved access");
    assert_eq!(original.len(), 10);
    let recipe = &endpoint.input().recipe;
    let bindings: Bindings = packet("bindings.json");
    assert_eq!(bindings.player_stat, d(0x1d33));
    assert_eq!(bindings.actor_stat, d(0x1d34));
    assert_eq!(bindings.unit, d(2));
    assert_eq!(bindings.nodes.len(), 41);
    let selected: Vec<_> = original
        .iter()
        .map(|a| {
            let rows: Vec<_> = bindings.nodes.iter().filter(|b| b.node == a.node).collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(a.pool, rows[0].pool);
            assert_eq!(a.access, AllocationAccess::Ordinary);
            assert_eq!(a.scope, LoadoutScope::Shared);
            assert!(a.choices.is_empty());
            rows[0].clone()
        })
        .collect();
    assert_eq!(selected.iter().map(|b| b.value).sum::<f64>(), 68.);
    let owners: Vec<_> = original
        .iter()
        .map(|allocation| {
            let actual = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == subject(allocation.node.clone()))
                .unwrap();
            assert!(actual.programs.is_complete());
            assert_eq!(
                actual
                    .programs
                    .members
                    .iter()
                    .filter(|p| p.id == key("ordinary-minion-damage"))
                    .count(),
                1
            );
            actual.clone()
        })
        .collect();
    let mut addresses = vec![
        d::<StatDefinition>(0x1d33).address(),
        d::<StatDefinition>(0x1d34).address(),
    ];
    for allocation in &original {
        if !addresses.contains(&allocation.pool.address()) {
            addresses.push(allocation.pool.address());
        }
    }
    // Retain paired Life and Command-cooldown effects and their typed channels.
    // No consumer for these channels is introduced by this subtotal test.
    for effect in owners
        .iter()
        .flat_map(|o| &o.programs.members)
        .flat_map(|p| &p.effects)
    {
        let RuleEffectKind::Contribute { stat, .. } = &effect.effect else {
            panic!("exact ordinary passive contribution")
        };
        if !addresses.contains(&stat.address()) {
            addresses.push(stat.address());
        }
    }
    let f = &mut sniper.base.source.base.inner;
    assert!(f.build.allocations.is_empty());
    for address in addresses {
        let actual = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
            .unwrap();
        if let Some(existing) = f.schema.definitions.iter().find(|d| d.address() == address) {
            assert_eq!(existing, actual);
        } else {
            f.schema.definitions.push(actual.clone());
        }
        f.owner_mut(SchemaSubject::Definition(address));
    }
    for (allocation, owner) in original.iter().zip(owners) {
        let actual = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == allocation.node.address())
            .unwrap();
        let mut finite = actual.clone();
        let DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = &mut finite
        else {
            panic!("known selected passive descriptor")
        };
        assert!(schema.pools.is_complete());
        assert_eq!(
            schema.pools.members.as_slice(),
            std::slice::from_ref(&allocation.pool)
        );
        // This is an explicitly selected numerical subgraph, not a connected
        // tree candidate. Actual declaration/default-program closure is retained.
        schema.adjacent = DeclaredSet::complete(vec![]);
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|d| d.address() == finite.address())
        );
        f.schema.definitions.push(finite);
        f.owners.push(owner);
    }
    let dependency_owners: Vec<DefinitionRules> = packet("dependency-rules.json");
    assert_eq!(dependency_owners.len(), 2);
    for owner in dependency_owners {
        assert!(recipe.rules.owners.contains(&owner));
        let selected = f.owner_mut(owner.owner.clone());
        assert!(selected.programs.is_complete() && selected.programs.members.is_empty());
        *selected = owner;
    }
    let receivers: Vec<StatReceiver> = packet("receivers.json");
    assert_eq!(receivers.len(), 2);
    for receiver in receivers {
        assert!(recipe.rules.receivers.members.contains(&receiver));
        assert!(!sniper.receivers.members.iter().any(|r| r.id == receiver.id));
        sniper.receivers.members.push(receiver);
    }
    let census = Census {
        bindings: selected,
        original,
        cases,
    };
    f.build.allocations = census.rehome(&census.original);
    census
}

impl Census {
    fn contains(&self, allocation: &Allocation) -> bool {
        self.original.iter().any(|a| a.node == allocation.node)
    }

    fn rehome(&self, allocations: &[Allocation]) -> Vec<Allocation> {
        let mut seen = BTreeSet::new();
        allocations
            .iter()
            .map(|a| {
                assert!(
                    seen.insert(a.id),
                    "distinct imported allocation occurrences"
                );
                let index = self.original.iter().position(|o| o.node == a.node).unwrap();
                let mut mapped = a.clone();
                mapped.id = id(7300 + index as u64);
                assert_ne!(mapped.id, a.id, "explicit fixture-lineage remap");
                assert_eq!(a.access, AllocationAccess::Ordinary);
                assert_eq!(mapped.node, self.original[index].node);
                mapped
            })
            .collect()
    }
}

pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.extend([
        EvaluationStage {
            id: key(REDUCE),
            predecessors: vec![key("deliver")],
        },
        EvaluationStage {
            id: key(RECEIVE),
            predecessors: vec![key(REDUCE)],
        },
    ]);
    for row in &mut stages.programs.members {
        if row.owner == subject(d::<StatDefinition>(0x1d33)) && row.program == key(GRANTED) {
            row.stage = key(REDUCE);
        }
        if row.owner == subject(d::<StatDefinition>(0x1d34)) && row.program == key(RECEIVED) {
            row.stage = key(RECEIVE);
        }
    }
    stages.frozen_channels.extend([
        FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: d(0x1d33),
                contribution: ContributionKind::Increase,
            },
            stage: key("deliver"),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x1d33),
            },
            stage: key(REDUCE),
        },
    ]);
}

fn known(value: f64) -> EffectValue {
    EffectValue::Known {
        value: quantity(value, &d(2)),
    }
}
fn check(w: &World, report: &SupportEffectsReport, total: f64, count: usize) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let r = sniper::offering::effects(report);
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let allocations: Vec<_> = w
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .iter()
        .filter(|a| w.passives.contains(a))
        .collect();
    assert_eq!(allocations.len(), count);
    let contributors: Vec<_> = r
        .effects
        .iter()
        .filter(|e| {
            matches!(&e.target,
        BoundEffectTarget::Contribution { key } if key.stat == d(0x1d33))
        })
        .collect();
    assert_eq!(contributors.len(), count);
    let mut seen = BTreeSet::new();
    let mut sum = 0.;
    for effect in contributors {
        let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
            panic!("actual passive occurrence")
        };
        let ProviderRoot::Allocation(id) = provider.root else {
            panic!("allocation provider")
        };
        assert!(provider.grant_path.is_empty() && seen.insert(id));
        let allocation = allocations.iter().find(|a| a.id == id).unwrap();
        let row = w
            .passives
            .bindings
            .iter()
            .find(|b| b.node == allocation.node)
            .unwrap();
        assert_eq!(
            effect.key.invocation.owner,
            subject(allocation.node.clone())
        );
        assert_eq!(effect.key.invocation.program, key("ordinary-minion-damage"));
        assert_eq!(
            effect.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x1d33),
                    kind: ContributionKind::Increase,
                }
            }
        );
        assert_eq!(effect.value, known(row.value));
        sum += row.value;
    }
    assert_eq!(sum, total);
    assert_eq!(seen, allocations.iter().map(|a| a.id).collect());
    for allocation in allocations {
        let owner = w
            .sniper
            .base
            .source
            .base
            .inner
            .owners
            .iter()
            .find(|o| o.owner == subject(allocation.node.clone()))
            .unwrap();
        let expected = owner
            .programs
            .members
            .iter()
            .map(|p| p.effects.len())
            .sum::<usize>();
        let actual: Vec<_> = r
            .effects
            .iter()
            .filter(|e| {
                e.key.invocation.origin
                    == RuleOrigin::Provider {
                        provider: ProviderKey {
                            root: ProviderRoot::Allocation(allocation.id),
                            grant_path: vec![],
                        },
                    }
            })
            .collect();
        assert_eq!(
            actual.len(),
            expected,
            "paired effects must not be discarded"
        );
        assert!(
            actual
                .iter()
                .all(|e| matches!(e.value, EffectValue::Known { .. }))
        );
    }
    let reducers: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(GRANTED))
        .collect();
    assert_eq!(
        reducers.len(),
        1,
        "one Player reduction, not once per minion"
    );
    assert_eq!(reducers[0].value, known(total));
    assert_eq!(
        reducers[0].key.invocation.origin,
        RuleOrigin::Receiver {
            receiver: key("player-owned-minion-damage-grant"),
            actor: ActorKey::Player,
        }
    );
    let received: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(RECEIVED))
        .collect();
    assert_eq!(received.len(), 2);
    assert_ne!(w.sniper.actor(0), w.sniper.actor(1));
    for index in 0..2 {
        assert_eq!(w.value(r, index, false, 0x1d34), &known(total));
        let rows: Vec<_> = received
            .iter()
            .filter(|e| {
                e.key.invocation.origin
                    == RuleOrigin::Receiver {
                        receiver: key("sniper-received-owner-minion-damage"),
                        actor: w.sniper.actor(index),
                    }
            })
            .collect();
        assert_eq!(rows.len(), 1, "one exact receiver per independent Actor");
        assert_eq!(
            rows[0].target,
            BoundEffectTarget::Value {
                key: PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(w.sniper.actor(index)),
                    stat: d(0x1d34),
                }
            }
        );
        assert_eq!(rows[0].value, known(total));
    }
}

fn repaired(w: &World) -> World {
    let case = w
        .passives
        .cases
        .iter()
        .find(|c| c.case_name == "without-plain-node")
        .unwrap();
    assert!(
        case.allocations.is_none(),
        "pending access is not a resolved source control"
    );
    assert_eq!(case.saved_allocations.len(), 9);
    assert_eq!(case.pending_nodes.len(), 1);
    assert_eq!(case.saved_subtotal, 58.);
    assert_eq!(case.expected_source_subtotal, 48.);
    assert_eq!(case.observed_active_nodes.len(), 8);
    let candidate =
        passive_damage_evidence::explicit_candidate_without(case, &case.pending_nodes[0]);
    assert_eq!(candidate.len(), 8);
    assert_eq!(
        candidate
            .iter()
            .map(|a| a.node.clone())
            .collect::<BTreeSet<_>>(),
        case.observed_active_nodes.iter().cloned().collect()
    );
    let mut changed = w.clone();
    changed
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .retain(|a| !w.passives.contains(a));
    changed
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .extend(w.passives.rehome(&candidate));
    changed
}

#[test]
#[ignore = "requires current release and fresh imported passive controls; finite native integration"]
fn passive_damage_actual_ten_sources_and_explicit_repair_reach_both_item_driven_actors() {
    let w = World::load();
    let original = w
        .passives
        .cases
        .iter()
        .find(|c| c.case_name == "original-05")
        .unwrap();
    let control = w
        .passives
        .cases
        .iter()
        .find(|c| c.case_name == "without-plain-node")
        .unwrap();
    // The evidence helper authenticates both complete XML hashes against the
    // retained reports; keep their distinct source identities in this replay.
    assert_eq!(original.source_xml_sha256.len(), 64);
    assert_eq!(control.source_xml_sha256.len(), 64);
    assert_ne!(original.source_xml_sha256, control.source_xml_sha256);
    assert_eq!(original.saved_allocations.len(), 10);
    assert!(original.pending_nodes.is_empty());
    assert_eq!(original.saved_subtotal, 68.);
    assert_eq!(original.expected_source_subtotal, 68.);
    assert_eq!(original.observed_active_nodes.len(), 10);
    let report = w.evaluate();
    check(&w, &report, 68., 10);
    let intrinsic = evidence::checked_cases();
    let source = intrinsic
        .iter()
        .find(|r| r["physical_level"] == 20)
        .unwrap();
    w.check(&report, [source, source]);
    let accuracy = evidence::accuracy_cases();
    let source = accuracy
        .iter()
        .find(|r| r["case"] == "original-05")
        .unwrap();
    let output = &source["consumer"]["passes"][0]["output"];
    for index in 0..2 {
        w.check_accuracy(
            &report,
            index,
            output["enemyBlockChance"].as_f64().unwrap(),
            output["HitChance"].as_f64().unwrap(),
        );
    }
    // This is an explicitly authored second removal, not automatic pruning of
    // the saved one-node control. Its eight known rows reproduce the source's
    // measured active subtotal, without asserting full-tree or final-DPS parity.
    let changed = repaired(&w);
    check(&changed, &changed.evaluate(), 48., 8);
    let mut empty = w.clone();
    empty
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .retain(|a| !w.passives.contains(a));
    check(&empty, &empty.evaluate(), 0., 0);
    assert_eq!(w.evaluate(), report);
}

fn assert_unclosed(w: &World, subject: &SchemaSubject, reason: PlanGapReason) {
    let p = w.plan();
    assert!(
        p.gaps()
            .iter()
            .any(|g| g.reason == reason && g.subject.as_ref() == Some(subject)),
        "{:?}",
        p.gaps()
    );
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(
        matches!(
            &report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                },
                ..
            }
        ),
        "{report:?}"
    );
}

#[test]
#[ignore = "requires current release; missing and Partial passive inventory refusal"]
fn passive_damage_missing_and_partial_owners_do_not_become_zero() {
    let w = World::load();
    let owner = subject(w.passives.original[0].node.clone());
    let mut missing = w.clone();
    missing
        .sniper
        .base
        .source
        .base
        .inner
        .owners
        .retain(|o| o.owner != owner);
    assert_unclosed(&missing, &owner, PlanGapReason::MissingPrograms);
    let mut partial = w.clone();
    partial
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(owner.clone())
        .programs
        .closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("fixture-unreviewed-passive-damage"),
        }],
    };
    assert_unclosed(&partial, &owner, PlanGapReason::PartialPrograms);
    // The real item/Actor inventories remain independently scoped. Restoring
    // the actual Partial Actor owner cannot be hidden by known passive damage.
    let actor = SchemaSubject::Slot(SlotAddress::Actor(actor_slot()));
    let mut partial_actor = w.clone();
    partial_actor
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(actor.clone())
        .programs
        .closure = w.actual_actor_coverage.clone();
    assert_unclosed(&partial_actor, &actor, PlanGapReason::PartialPrograms);
}

#[test]
#[ignore = "requires current release; exact receiver missingness and staging"]
fn passive_damage_receivers_and_stage_dependencies_cannot_be_bypassed() {
    let w = World::load();
    let mut missing = w.clone();
    missing
        .sniper
        .receivers
        .members
        .retain(|r| r.id != key("player-owned-minion-damage-grant"));
    let report = missing.evaluate();
    let r = sniper::offering::effects(&report);
    for index in 0..2 {
        assert!(
            matches!(
                missing.value(r, index, false, 0x1d34),
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ),
            "{:?}",
            missing.value(r, index, false, 0x1d34)
        );
    }
    let mut no_recipient = w.clone();
    no_recipient
        .sniper
        .receivers
        .members
        .retain(|r| r.id != key("sniper-received-owner-minion-damage"));
    let report = no_recipient.evaluate();
    let r = sniper::offering::effects(&report);
    assert!(
        !r.values
            .iter()
            .any(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if *stat == d(0x1d34)))
    );
    assert_eq!(
        r.effects
            .iter()
            .filter(|e| e.key.invocation.program == key(GRANTED))
            .count(),
        1
    );
    for program in [GRANTED, RECEIVED] {
        let error = w
            .checked_plan_configured(|stages| {
                stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.program == key(program))
                    .unwrap()
                    .stage = key("facts");
            })
            .err()
            .expect("a later input cannot be consumed before its producer/freeze");
        assert!(
            error.contains("stage") || error.contains("frozen"),
            "{error}"
        );
    }
}

#[test]
#[ignore = "requires current release; same graph A-B-A and Rayon scratch reuse"]
fn passive_damage_selection_and_per_actor_inputs_are_stable_across_reuse_and_workers() {
    let a = World::load();
    let mut b = repaired(&a);
    b.sniper.raw(1, 1, 0., 0.);
    let pa = a.plan();
    let pb = b.plan();
    assert!(pa.gaps().is_empty() && pb.gaps().is_empty());
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(&a, &first, 68., 10);
    let changed = pb.evaluate(&mut scratch).unwrap();
    check(&b, &changed, 48., 8);
    let rows = evidence::checked_cases();
    let high = rows.iter().find(|r| r["physical_level"] == 20).unwrap();
    let low = rows.iter().find(|r| r["physical_level"] == 1).unwrap();
    b.check(&changed, [high, low]);
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), first);
    assert_eq!(pa.evaluate(&mut pa.new_scratch()).unwrap(), first);
    assert_eq!(pb.evaluate(&mut pb.new_scratch()).unwrap(), changed);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |scratch, i| {
                    if i % 2 == 0 {
                        pa.evaluate(scratch).unwrap()
                    } else {
                        pb.evaluate(scratch).unwrap()
                    }
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, report) in reports.iter().enumerate() {
        assert_eq!(report, if i % 2 == 0 { &first } else { &changed });
    }
    let mut reversed = a.clone();
    reversed
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .reverse();
    let report = reversed.evaluate();
    check(&reversed, &report, 68., 10);
    for index in 0..2 {
        assert_eq!(
            reversed.value(sniper::offering::effects(&report), index, false, 0x1d34),
            a.value(sniper::offering::effects(&first), index, false, 0x1d34)
        );
    }
}
