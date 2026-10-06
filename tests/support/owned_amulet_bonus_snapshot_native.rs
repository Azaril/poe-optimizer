//! Real pre-copy reducer joined to actual item and Offering programs in one graph.
//! The selected item/support inventories and isolated Passive topology are finite
//! test scopes. Selecting Mystic Attunement here is not an allocation-legality or
//! complete Original05 claim. Published Partial owners remain independently tested.
#[allow(dead_code)]
#[path = "owned_offering_final_inputs_fixture.rs"]
mod offering;
use offering::{decode, def, id, key, quantity, shared, subject};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

#[derive(Clone)]
struct World {
    base: offering::World,
    bindings: Value,
    receivers: DeclaredSet<StatReceiver>,
    actual_passive: DefinitionRules,
    allocation: Allocation,
}
impl World {
    fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_RELEASE")
                .expect("checked Amulet snapshot publication"),
        );
        let before = crate::release::inventory(&path);
        let endpoint = crate::release::load(&path);
        crate::family::assert_endpoint(&endpoint);
        let bindings: Value = crate::family::read("bindings.json");
        let authored: Value = crate::family::read("snapshot-authoring.json");
        let owners: Vec<DefinitionRules> = decode(&authored["owners"]);
        let receivers: Vec<StatReceiver> = decode(&authored["receivers"]);
        assert_eq!(owners.len(), 1);
        assert_eq!(receivers.len(), 1);
        let owner = &owners[0];
        assert!(owner.programs.is_complete());
        assert_eq!(
            owner.owner,
            subject(decode::<StatDefId>(
                &bindings["channels"]["pre_amulet_percent"]
            ))
        );
        assert_eq!(owner.programs.members.len(), 1);
        assert_eq!(
            owner.programs.members[0].id,
            decode(&bindings["snapshot"]["program"])
        );
        assert_eq!(receivers[0].id, decode(&bindings["snapshot"]["receiver"]));
        assert_eq!(receivers[0].targets, vec![StatReceiverTarget::Player]);
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|o| *o == owner)
                .count(),
            1
        );
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|r| **r == receivers[0])
                .count(),
            1
        );
        let mut base = offering::World::load_release(&path);
        for o in &mut base.source.base.inner.owners {
            o.programs
                .members
                .retain(|p| p.id != key(offering::SNAPSHOT));
        }
        base.item_programs
            .retain(|(_, p)| *p != key(offering::SNAPSHOT));
        assert!(
            base.source
                .base
                .inner
                .owners
                .iter()
                .flat_map(|o| &o.programs.members)
                .all(|p| p.id != key(offering::SNAPSHOT))
        );
        *base.source.base.inner.owner_mut(owner.owner.clone()) = owner.clone();

        let passive: PassiveNodeDefId = decode(&bindings["passive"]["definition"]);
        let actual_passive = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(passive.clone()))
            .unwrap()
            .clone();
        assert!(!actual_passive.programs.is_complete());
        assert_eq!(actual_passive.programs.members.len(), 1);
        assert_eq!(
            actual_passive.programs.members[0].id,
            decode(&bindings["passive"]["program"])
        );
        // Preserve the actual numeric body, with unrelated game-rule coverage
        // explicitly closed only in this component. The negative restores it.
        *base
            .source
            .base
            .inner
            .owner_mut(actual_passive.owner.clone()) = offering::prolonged::finite(&actual_passive);
        let mut definition: DefinitionDescriptor = offering::prolonged::finite(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == passive.address())
                .unwrap(),
        );
        let DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = &mut definition
        else {
            panic!("actual known Passive")
        };
        assert!(schema.pools.is_complete());
        assert_eq!(schema.pools.members.len(), 1);
        let pool = schema.pools.members[0].clone();
        // This existing numerical fixture does not validate full-tree reachability.
        schema.adjacent = DeclaredSet::complete(vec![]);
        offering::add_definition(&mut base.source, definition);
        offering::add_definition(
            &mut base.source,
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == pool.address())
                .unwrap()
                .clone(),
        );
        base.source.base.inner.owner_mut(subject(pool.clone()));
        let allocation = Allocation {
            id: id(6100),
            node: passive,
            pool,
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        };
        assert!(base.source.base.inner.build.allocations.is_empty());
        assert_eq!(before, crate::release::inventory(&path));
        Self {
            base,
            bindings,
            receivers: DeclaredSet::complete(receivers),
            actual_passive,
            allocation,
        }
    }
    fn select_passive(&mut self, selected: bool) {
        self.base.source.base.inner.build.allocations = if selected {
            vec![self.allocation.clone()]
        } else {
            vec![]
        };
    }
    fn stat(&self) -> StatDefId {
        decode(&self.bindings["channels"]["pre_amulet_percent"])
    }
    fn program(&self) -> OwnedDefinitionKey {
        decode(&self.bindings["snapshot"]["program"])
    }
    fn passive_program(&self) -> OwnedDefinitionKey {
        decode(&self.bindings["passive"]["program"])
    }
    fn checked_plan(&self) -> std::result::Result<shared::Plan, String> {
        self.checked_plan_variant(false, false)
    }
    fn checked_plan_variant(
        &self,
        late_contributor: bool,
        early_copy: bool,
    ) -> std::result::Result<shared::Plan, String> {
        self.base.checked_plan_configured(
            OWNED_EVALUATION_STAGES_V4,
            false,
            self.receivers.clone(),
            |stages| {
                let names = [
                    "prepare",
                    "pre-amulet-contributors",
                    "pre-amulet-snapshot",
                    "amulet-copy",
                    "source-prepare",
                    "source-census",
                    "source-assembly",
                    "facts",
                    "apply",
                    "deliver",
                ];
                stages.stages = names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| EvaluationStage {
                        id: key(name),
                        predecessors: i
                            .checked_sub(1)
                            .map(|p| vec![key(names[p])])
                            .unwrap_or_default(),
                    })
                    .collect();
                let snapshot_owner = subject(self.stat());
                for row in &mut stages.programs.members {
                    if row.owner == snapshot_owner && row.program == self.program() {
                        row.stage = key("pre-amulet-snapshot");
                    } else if row.owner == self.actual_passive.owner
                        && row.program == self.passive_program()
                    {
                        row.stage = key(if late_contributor {
                            "amulet-copy"
                        } else {
                            "pre-amulet-contributors"
                        });
                    } else if row.program == key("amulet-copy-minion-gem-level") {
                        row.stage = key(if early_copy { "prepare" } else { "amulet-copy" });
                    } else if row.program == key("unreviewed-feedback-control") {
                        row.stage = key("pre-amulet-contributors");
                    }
                }
                for row in &mut stages.readiness.as_mut().unwrap().programs.members {
                    if (row.owner == snapshot_owner && row.program == self.program())
                        || (row.owner == self.actual_passive.owner
                            && row.program == self.passive_program())
                        || row.program == key("unreviewed-feedback-control")
                    {
                        let p = self
                            .base
                            .source
                            .base
                            .inner
                            .owners
                            .iter()
                            .find(|o| o.owner == row.owner)
                            .unwrap()
                            .programs
                            .members
                            .iter()
                            .find(|p| p.id == row.program)
                            .unwrap();
                        row.phase = ReadinessPhase::Structural;
                        row.role = ReadinessProgramRole::PreparationFacts;
                        row.outputs = p
                            .effects
                            .iter()
                            .map(|e| offering::channel(p.context, &e.effect))
                            .collect();
                    }
                }
                stages.frozen_channels.extend([
                    FrozenStageChannel {
                        channel: StageChannel::Contributions {
                            scope: RuleEntityKind::Actor,
                            stat: self.stat(),
                            contribution: ContributionKind::Add,
                        },
                        stage: key("pre-amulet-contributors"),
                    },
                    FrozenStageChannel {
                        channel: StageChannel::Stat {
                            scope: RuleEntityKind::Actor,
                            stat: self.stat(),
                        },
                        stage: key("pre-amulet-snapshot"),
                    },
                ]);
            },
        )
    }
    fn plan(&self) -> shared::Plan {
        self.checked_plan().unwrap()
    }
    fn evaluate(&self) -> SupportEffectsReport {
        let p = self.plan();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn check(
        &self,
        r: &SupportEffectsReport,
        percent: f64,
        levels: [i64; 2],
        qualities: [f64; 2],
        allocated: bool,
    ) {
        offering::check(&self.base, r, levels, qualities);
        let effects = offering::effects(r);
        let target = PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(ActorKey::Player),
            stat: self.stat(),
        };
        let rows: Vec<_> = effects
            .effects
            .iter()
            .filter(|e| {
                e.target
                    == BoundEffectTarget::Value {
                        key: target.clone(),
                    }
            })
            .collect();
        assert_eq!(rows.len(), 1, "one source-independent Player snapshot");
        let row = rows[0];
        assert_eq!(row.key.invocation.program, self.program());
        assert_eq!(
            row.key.invocation.origin,
            RuleOrigin::Receiver {
                receiver: decode(&self.bindings["snapshot"]["receiver"]),
                actor: ActorKey::Player,
            }
        );
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(percent, &decode(&self.bindings["units"]["percent"]))
            }
        );
        let incoming: Vec<_> = effects.effects.iter().filter(|e| matches!(&e.target,
            BoundEffectTarget::Contribution{key} if key.entity == ConcreteEntity::Actor(ActorKey::Player)
                && key.stat == self.stat() && key.kind == ContributionKind::Add)).collect();
        assert_eq!(incoming.len(), usize::from(allocated));
        if allocated {
            assert_eq!(incoming[0].key.invocation.program, self.passive_program());
            assert_eq!(
                incoming[0].value,
                EffectValue::Known {
                    value: quantity(25.0, &decode(&self.bindings["units"]["percent"]))
                }
            );
            assert!(
                matches!(&incoming[0].key.invocation.origin,RuleOrigin::Provider{provider}
                if provider.root == ProviderRoot::Allocation(self.allocation.id) && provider.grant_path.is_empty())
            );
        }
        assert!(
            effects
                .effects
                .iter()
                .all(|e| e.key.invocation.program != key(offering::SNAPSHOT))
        );
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_RELEASE; finite native component"]
fn amulet_snapshot_zero_uses_real_receiver_with_both_support_tiers() {
    let mut w = World::load();
    for tier in [0, 1] {
        w.base.source.base.inner.build.supports.clear();
        w.base.source.base.inner.build.support_origins = Some(vec![]);
        for source in [2, 3, 4] {
            w.base.source.base.inner.add_support(source, tier);
        }
        let r = w.evaluate();
        w.check(&r, 0.0, [22, 22], [0.0, 0.0], false);
        offering::check_census_and_delivery(&w.base, &r, Some(tier));
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_RELEASE; finite native component"]
fn amulet_snapshot_actual_passive_precedes_copy_and_independent_offering_inputs() {
    let mut w = World::load();
    w.select_passive(true);
    w.base.item_amount("amulet", 5.0);
    let r = w.evaluate();
    w.check(&r, 25.0, [27, 27], [0.0, 0.0], true);
    let copy: Vec<_> = offering::effects(&r)
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.program == key("amulet-copy-minion-gem-level")
                && e.value != EffectValue::Inactive
        })
        .collect();
    assert_eq!(copy.len(), 1);
    assert_eq!(
        copy[0].value,
        EffectValue::Known {
            value: quantity(1.0, &def("def.000000000000295a"))
        }
    );
    assert!(
        matches!(&copy[0].key.invocation.origin,RuleOrigin::Provider{provider}
        if provider.root==ProviderRoot::ItemModifier{equipment_use:id(6002),modifier:id(6001)})
    );
    w.base.raw(3, 17, 9.0);
    w.check(&w.evaluate(), 25.0, [24, 27], [9.0, 0.0], true);
    w.select_passive(false);
    w.check(&w.evaluate(), 0.0, [23, 26], [9.0, 0.0], false);
    w.select_passive(true);
    w.base.remove_item("amulet");
    w.check(&w.evaluate(), 25.0, [18, 21], [9.0, 0.0], true);
    w.base.remove_item("helmet");
    w.check(&w.evaluate(), 25.0, [17, 20], [9.0, 0.0], true);
}

fn no_final_level(r: &SupportEffectsReport, reason: PlanGapReason) {
    match &r.outcome {
        SupportEffectsOutcome::Unavailable { cause, .. } => assert!(
            matches!(cause,EffectValue::Unresolved{reason:r,..} if *r==reason),
            "{cause:?}"
        ),
        SupportEffectsOutcome::Evaluated { effects } => {
            assert!(
                effects
                    .effects
                    .iter()
                    .any(|e| matches!(&e.value,EffectValue::Unresolved{reason:r,..} if *r==reason)),
                "expected exact refusal {reason:?}"
            );
            assert!(!effects.effects.iter().any(|e| e.key.invocation.program
                == key("pain-offering-final-inputs")
                && e.key.effect == key("project-final-level")
                && matches!(e.value, EffectValue::Known { .. })));
        }
        other => panic!("unexpected preparation refusal {other:?}"),
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_RELEASE; finite native component"]
fn amulet_snapshot_missing_receiver_and_partial_inventory_do_not_become_zero() {
    let mut missing = World::load();
    missing.receivers.members.clear();
    no_final_level(&missing.evaluate(), PlanGapReason::MissingProducer);
    let mut partial = World::load();
    partial.receivers.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(partial.stat()),
            facet: SchemaFacet::GameRules,
            code: key("fixture-unreviewed-snapshot-contributors"),
        }],
    };
    let r = partial.evaluate();
    assert!(
        r.gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialReceivers),
        "{:?}",
        r.gaps
    );
    no_final_level(&r, PlanGapReason::IncompleteContributors);
    let mut partial = World::load();
    partial.select_passive(true);
    *partial
        .base
        .source
        .base
        .inner
        .owner_mut(partial.actual_passive.owner.clone()) = partial.actual_passive.clone();
    let error = partial
        .checked_plan()
        .err()
        .expect("actual Partial Passive inventory must remain refused");
    assert!(error.contains("complete owner programs"), "{error}");
    let mut missing = World::load();
    let stat = missing.stat();
    missing
        .base
        .source
        .base
        .inner
        .owner_mut(subject(stat))
        .programs
        .members
        .clear();
    let error = missing
        .checked_plan()
        .err()
        .expect("receiver cannot invent its missing program");
    assert!(
        error.contains("receiver requires an existing stat-owned program"),
        "{error}"
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_RELEASE; finite native component"]
fn amulet_snapshot_rejects_late_contributors_early_copy_and_feedback() {
    let w = World::load();
    let error = w
        .checked_plan_variant(true, false)
        .err()
        .expect("unselected late writer must still be rejected");
    assert!(
        error.contains("potential writer occurs after or outside frozen stage"),
        "{error}"
    );
    let error = w
        .checked_plan_variant(false, true)
        .err()
        .expect("copy cannot precede captured snapshot");
    assert!(
        error.contains("frozen channel read occurs before or outside frozen stage"),
        "{error}"
    );
    let mut feedback = w.clone();
    let mut program = feedback.actual_passive.programs.members[0].clone();
    program.id = key("unreviewed-feedback-control");
    program.reads = vec![RuleRead {
        id: key("prior"),
        value_type: ComputedValueType::Quantity {
            unit: decode(&w.bindings["units"]["percent"]),
        },
        source: RuleReadSource::Stat {
            entity: RuleEntity::Player,
            stat: w.stat(),
        },
    }];
    program.nodes[0].expression = RuleExpression::Read {
        input: key("prior"),
    };
    feedback
        .base
        .source
        .base
        .inner
        .owner_mut(feedback.actual_passive.owner.clone())
        .programs
        .members
        .push(program);
    let error = feedback
        .checked_plan()
        .err()
        .expect("post-snapshot value cannot feed its own incoming stream");
    assert!(
        error.contains("frozen channel read occurs before or outside frozen stage"),
        "{error}"
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_RELEASE; finite native component"]
fn amulet_snapshot_scratch_and_rayon_preserve_exact_contributor_origins() {
    let a = World::load();
    let mut b = a.clone();
    b.select_passive(true);
    b.base.item_amount("amulet", 5.0);
    b.base.raw(3, 7, 13.0);
    let ap = a.plan();
    let bp = b.plan();
    let mut scratch = ap.new_scratch();
    let first = ap.evaluate(&mut scratch).unwrap();
    a.check(&first, 0.0, [22, 22], [0.0, 0.0], false);
    let second = bp.evaluate(&mut scratch).unwrap();
    b.check(&second, 25.0, [14, 27], [13.0, 0.0], true);
    assert_eq!(first, ap.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let plan = Arc::new(bp);
    let reports: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map(|_| plan.evaluate(&mut plan.new_scratch()).unwrap())
            .collect()
    });
    assert!(reports.iter().all(|r| r == &second));
}
