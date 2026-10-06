//! The published item/preparation/population path supplies Actor level. Only the
//! finite passive inventory and selected Actor programs are closed in this test.
//! Contributions are checked separately; no final Life or whole-build claim.
#[allow(dead_code)]
#[path = "owned_sniper_final_inputs_fixture.rs"]
mod fixture;
use super::{family, release};
use fixture::{
    World as SniperWorld, decode, def, id, key, offering, quantity, shared, stat, subject,
};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf, sync::OnceLock};

const INTRINSIC: &str = "intrinsic-allied-minion-life";
const RECEIVED: &str = "received-minion-life-increase";
fn actor_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(fixture::skill()),
        slot: def("def.000000000000001f"),
    }))
}
#[derive(Clone)]
struct World {
    sniper: SniperWorld,
    actual_actor: DefinitionRules,
    nodes: Value,
}
impl World {
    fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_RELEASE")
                .expect("published Life Increase package"),
        );
        let inventory = release::inventory(&path);
        let endpoint = release::load(&path);
        family::assert_endpoint(&endpoint);
        let mut sniper = SniperWorld::load_release(&path);
        let recipe = &endpoint.input().recipe;
        let bindings: Value = family::read("bindings.json");
        assert_eq!(bindings["program"], RECEIVED);
        assert_eq!(bindings["carrier"], json!(stat(0x32e5)));
        assert_eq!(bindings["life"], json!(stat(0x311a)));
        let dependencies: Value = family::read("dependencies.json");
        let passive_owners: Vec<DefinitionRules> = decode(&dependencies["passive_owners"]);
        assert_eq!(passive_owners.len(), 6);
        let actual_actor = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == actor_owner())
            .unwrap()
            .clone();
        assert!(!actual_actor.programs.is_complete());
        let selected_programs: Vec<_> = [INTRINSIC, RECEIVED]
            .iter()
            .map(|name| {
                actual_actor
                    .programs
                    .members
                    .iter()
                    .find(|p| p.id.as_str() == *name)
                    .unwrap()
                    .clone()
            })
            .collect();
        let f = &mut sniper.base.source.base.inner;
        let mut addresses = vec![
            stat(0x32e5).address(),
            stat(0x311a).address(),
            def::<UnitDefinition>("def.0000000000003119").address(),
            def::<PointPoolDefinition>("def.0000000000001bf0").address(),
        ];
        // The four mixed Life/Damage defaults retain every exact effect, even
        // though this component has no Damage consumer.
        for owner in &passive_owners {
            for effect in owner.programs.members.iter().flat_map(|p| &p.effects) {
                let RuleEffectKind::Contribute { stat, .. } = &effect.effect else {
                    panic!("actual ordinary passive contribution")
                };
                if !addresses.contains(&stat.address()) {
                    addresses.push(stat.address());
                }
            }
        }
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
        let actor = f.owner_mut(actor_owner());
        assert!(actor.programs.is_complete() && actor.programs.members.is_empty());
        actor.programs.members = selected_programs;
        assert!(
            f.build.allocations.is_empty(),
            "only six exact passive sources enter this component"
        );
        for (index, owner) in passive_owners.into_iter().enumerate() {
            assert!(owner.programs.is_complete());
            assert_eq!(
                owner
                    .programs
                    .members
                    .iter()
                    .filter(|p| p.id == key("ordinary-minion-life"))
                    .count(),
                1
            );
            assert!(recipe.rules.owners.contains(&owner));
            let node: PassiveNodeDefId = decode(&bindings["nodes"][index]["definition"]);
            assert_eq!(owner.owner, subject(node.clone()));
            let actual = recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == node.address())
                .unwrap();
            let DefinitionDescriptor::PassiveNode(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) = actual
            else {
                panic!("known passive")
            };
            assert!(schema.pools.is_complete());
            assert_eq!(schema.pools.members.len(), 1);
            // Only graph reachability is excluded from this numerical component.
            // Exact default programs and declaration closure stay authenticated.
            let mut finite = actual.clone();
            let DefinitionDescriptor::PassiveNode(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) = &mut finite
            else {
                unreachable!()
            };
            schema.adjacent = DeclaredSet::complete(vec![]);
            let pool = schema.pools.members[0].clone();
            assert!(
                !f.schema
                    .definitions
                    .iter()
                    .any(|d| d.address() == node.address())
            );
            f.schema.definitions.push(finite);
            f.owners.push(owner);
            f.build.allocations.push(Allocation {
                id: id(7300 + index as u64),
                node,
                pool,
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            });
        }
        let table = recipe
            .rules
            .tables
            .iter()
            .find(|t| t.id == key("actor.allied-life-by-level"))
            .unwrap();
        assert!(!sniper.base.tables.iter().any(|t| t.id == table.id));
        sniper.base.tables.push(table.clone());
        assert_eq!(inventory, release::inventory(&path));
        Self {
            sniper,
            actual_actor,
            nodes: bindings["nodes"].clone(),
        }
    }
    fn checked_plan(&self) -> std::result::Result<shared::Plan, String> {
        self.sniper.checked_plan_configured(|stages| {
            stages.stages.push(EvaluationStage {
                id: key("life-increase"),
                predecessors: vec![key("deliver")],
            });
            for row in &mut stages.programs.members {
                if row.owner == actor_owner() {
                    row.stage = key(if row.program == key(RECEIVED) {
                        "life-increase"
                    } else {
                        "facts"
                    });
                }
            }
            stages.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: stat(0x32e5),
                    contribution: ContributionKind::Increase,
                },
                stage: key("deliver"),
            });
        })
    }
    fn plan(&self) -> shared::Plan {
        self.checked_plan().unwrap()
    }
    fn evaluate(&self) -> SupportEffectsReport {
        let p = self.plan();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn allocations(&self) -> &[Allocation] {
        &self.sniper.base.source.base.inner.build.allocations
    }
    fn remove(&mut self, source: &str) {
        let node: PassiveNodeDefId = decode(
            &self
                .nodes
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["source_id"] == source)
                .unwrap()["definition"],
        );
        let before = self.allocations().len();
        self.sniper
            .base
            .source
            .base
            .inner
            .build
            .allocations
            .retain(|a| a.node != node);
        assert_eq!(self.allocations().len(), before - 1);
    }
}
fn contribution<'a>(
    report: &'a OwnedEffectsReport,
    w: &World,
    index: usize,
    kind: ContributionKind,
    program: &str,
) -> &'a BoundEffectResult {
    let rows:Vec<_>=report.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key}
        if key.stat==stat(0x311a)&&key.kind==kind&&key.entity==ConcreteEntity::Actor(w.sniper.actor(index)))).collect();
    assert_eq!(rows.len(), 1, "one contribution per independent Actor");
    let row = rows[0];
    assert_eq!(row.key.invocation.owner, actor_owner());
    assert_eq!(row.key.invocation.program, key(program));
    let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
        panic!("actual population provider")
    };
    let ActorKey::Owned(actor) = w.sniper.actor(index) else {
        unreachable!()
    };
    let mut expected = actor.provider;
    expected.grant_path.push(DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(fixture::skill()),
        slot: def("def.0000000000000020"),
    });
    assert_eq!(provider, &expected);
    row
}
fn known(n: f64, unit: &str) -> EffectValue {
    EffectValue::Known {
        value: quantity(n, &def(unit)),
    }
}
fn check(w: &World, r: &SupportEffectsReport, increase: f64, base: [f64; 2], indices: &[usize]) {
    let r = offering::effects(r);
    let allocations = w.allocations();
    let carrier: Vec<_> = r
        .effects
        .iter()
        .filter(
            |e| matches!(&e.target,BoundEffectTarget::Contribution{key}if key.stat==stat(0x32e5)),
        )
        .collect();
    assert_eq!(carrier.len(), allocations.len());
    let mut witnessed = BTreeSet::new();
    let mut sum = 0.;
    for row in carrier {
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            panic!("passive provider")
        };
        let ProviderRoot::Allocation(id) = provider.root else {
            panic!("allocated source")
        };
        assert!(provider.grant_path.is_empty() && witnessed.insert(id));
        let allocation = allocations.iter().find(|a| a.id == id).unwrap();
        let n = w
            .nodes
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["definition"] == json!(allocation.node))
            .unwrap();
        let amount = n["amount"].as_f64().unwrap();
        sum += amount;
        assert_eq!(row.value, known(amount, "def.0000000000000002"));
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: stat(0x32e5),
                    kind: ContributionKind::Increase
                }
            }
        );
    }
    assert_eq!(sum, increase);
    assert_eq!(witnessed, allocations.iter().map(|a| a.id).collect());
    for index in indices {
        assert_eq!(
            contribution(r, w, *index, ContributionKind::Increase, RECEIVED).value,
            known(increase, "def.0000000000000002")
        );
        assert_eq!(
            contribution(r, w, *index, ContributionKind::Add, INTRINSIC).value,
            known(base[*index], "def.0000000000003119")
        );
    }
    let life: Vec<_> = r
        .effects
        .iter()
        .filter(
            |e| matches!(&e.target,BoundEffectTarget::Contribution{key}if key.stat==stat(0x311a)),
        )
        .collect();
    assert_eq!(
        life.len(),
        indices.len() * 2,
        "only admitted roots receive Life components"
    );
    assert!(
        life.iter()
            .all(|e| matches!(&e.target,BoundEffectTarget::Contribution{key}
        if indices.iter().any(|i|key.entity==ConcreteEntity::Actor(w.sniper.actor(*i)))))
    );
    assert!(
        !r.values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat:s,..}if *s==stat(0x311a))),
        "components are not a final Life pool"
    );
    assert!(
        !r.effects
            .iter()
            .any(|e| e.key.invocation.program == key("fixture-explicit-final-level"))
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_RELEASE"]
fn actual_passive_transport_removes_restores_and_receives_once_per_actor() {
    let w = World::load();
    let source: Value = family::read("source-vectors.json");
    let cases = source["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for (index, removed, total) in [
        (0, vec![], 44.),
        (3, vec!["229"], 38.),
        (4, vec!["1218"], 34.),
        (
            5,
            vec!["19006", "229", "39461", "54453", "1218", "40894"],
            0.,
        ),
    ] {
        let mut eligible: Vec<_> = w
            .nodes
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| {
                let id = n["source_id"].as_str().unwrap();
                (!removed.contains(&id)).then_some(id)
            })
            .collect();
        eligible.sort_unstable();
        assert_eq!(cases[index]["case_index"], index);
        assert_eq!(cases[index]["eligible_sources"], json!(eligible));
        assert_eq!(cases[index]["received_increase"].as_f64(), Some(total));
    }
    let baseline = w.evaluate();
    w.sniper.check(&baseline, [22, 22], [0., 0.]);
    check(&w, &baseline, 44., [1615., 1615.], &[0, 1]);
    for (source, total) in [("229", 38.), ("1218", 34.)] {
        let mut changed = w.clone();
        changed.remove(source);
        check(
            &changed,
            &changed.evaluate(),
            total,
            [1615., 1615.],
            &[0, 1],
        );
        assert_eq!(w.evaluate(), baseline);
    }
    let mut empty = w.clone();
    empty
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .clear();
    check(&empty, &empty.evaluate(), 0., [1615., 1615.], &[0, 1]);
    assert_eq!(w.evaluate(), baseline);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_RELEASE"]
fn real_items_raw_level_and_quality_change_intrinsic_inputs_not_life_increase() {
    let w = World::load();
    for (helmet, amulet, level, base) in [
        (true, false, 21, 1489.),
        (false, true, 21, 1489.),
        (true, true, 20, 1369.),
    ] {
        let mut c = w.clone();
        if helmet {
            c.sniper.base.remove_item("helmet");
        }
        if amulet {
            c.sniper.base.remove_item("amulet");
        }
        let r = c.evaluate();
        c.sniper.check(&r, [level, level], [0., 0.]);
        check(&c, &r, 44., [base, base], &[0, 1]);
    }
    let mut c = w.clone();
    c.sniper.raw(0, 12, 13., 1.);
    c.sniper.raw(1, 5, 7., 0.);
    let r = c.evaluate();
    c.sniper.check(&r, [15, 7], [13., 7.]);
    check(&c, &r, 44., [864., 310.], &[0, 1]);
    let mut quality = w.clone();
    quality.sniper.raw(0, 20, 20., 0.);
    let r = quality.evaluate();
    quality.sniper.check(&r, [22, 22], [20., 0.]);
    check(&quality, &r, 44., [1615., 1615.], &[0, 1]);
}

fn refused(w: &World, reason: PlanGapReason) {
    let r = w.evaluate();
    let mut reasons = r.gaps.iter().map(|g| g.reason).collect::<Vec<_>>();
    match &r.outcome {
        SupportEffectsOutcome::Evaluated { effects } => {
            reasons.extend(effects.gaps.iter().map(|g| g.reason));
            assert!(
                !effects
                    .effects
                    .iter()
                    .any(|e| e.key.invocation.program == key(RECEIVED)
                        && matches!(e.value, EffectValue::Known { .. })),
                "an unclosed incoming inventory cannot become zero"
            );
        }
        SupportEffectsOutcome::Unavailable { cause, .. } => assert!(
            matches!(
                cause,
                EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                }
            ),
            "{r:?}"
        ),
        other => panic!("unexpected refusal {other:?}"),
    }
    assert!(reasons.contains(&reason), "expected {reason:?}: {r:?}");
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_RELEASE"]
fn missing_and_partial_incoming_owners_cannot_become_complete_empty_zero() {
    let w = World::load();
    let selected = subject(w.allocations()[0].node.clone());
    let mut missing = w.clone();
    missing
        .sniper
        .base
        .source
        .base
        .inner
        .owners
        .retain(|o| o.owner != selected);
    refused(&missing, PlanGapReason::MissingPrograms);
    let mut partial = w.clone();
    partial
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(selected.clone())
        .programs
        .closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: selected.clone(),
            facet: SchemaFacet::GameRules,
            code: key("fixture-unreviewed-incoming-life"),
        }],
    };
    refused(&partial, PlanGapReason::PartialPrograms);
    let mut actor = w.clone();
    actor
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(actor_owner())
        .programs
        .closure = actor.actual_actor.programs.closure.clone();
    refused(&actor, PlanGapReason::PartialPrograms);
    let mut empty = w.clone();
    empty
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .clear();
    check(&empty, &empty.evaluate(), 0., [1615., 1615.], &[0, 1]);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_RELEASE"]
fn missing_actor_delivery_does_not_invent_an_increase_contribution() {
    let mut w = World::load();
    w.sniper
        .base
        .source
        .base
        .inner
        .owner_mut(actor_owner())
        .programs
        .members
        .retain(|p| p.id != key(RECEIVED));
    let report = w.evaluate();
    let r = offering::effects(&report);
    assert!(!r.effects.iter().any(|e|matches!(&e.target,BoundEffectTarget::Contribution{key}if key.stat==stat(0x311a)&&key.kind==ContributionKind::Increase)));
    for index in 0..2 {
        assert_eq!(
            contribution(r, &w, index, ContributionKind::Add, INTRINSIC).value,
            known(1615., "def.0000000000003119")
        );
    }
    assert!(
        !r.values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat:s,..}if *s==stat(0x311a)))
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_RELEASE"]
fn occurrence_removal_and_a_b_a_parallel_replays_preserve_exact_life_contributions() {
    let a = World::load();
    let mut b = a.clone();
    b.remove("1218");
    b.sniper.raw(0, 12, 13., 1.);
    b.sniper
        .base
        .source
        .base
        .inner
        .build
        .skills
        .iter_mut()
        .find(|s| s.id == id(7201))
        .unwrap()
        .enabled = false;
    let pa = a.plan();
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(&a, &first, 44., [1615., 1615.], &[0, 1]);
    let changed = pb.evaluate(&mut scratch).unwrap();
    check(&b, &changed, 34., [864., 0.], &[0]);
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |s, i| {
                    if i % 2 == 0 {
                        pa.evaluate(s).unwrap()
                    } else {
                        pb.evaluate(s).unwrap()
                    }
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, r) in reports.iter().enumerate() {
        assert_eq!(r, if i % 2 == 0 { &first } else { &changed });
    }
}
