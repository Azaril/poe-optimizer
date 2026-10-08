//! Published recipient scaling on the existing item-driven Sniper graph. The
//! admitted incoming domain is explicitly empty; no source-Skill scaling or
//! nonempty buff-effect law is supplied by this finite integration.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::owned_release::StagedOwnedRelease;

const INCREASE: &str = "recipient-empty-buff-effect-increase";
const MORE: &str = "recipient-empty-buff-effect-more";
const RESOLVE: &str = "recipient-buff-effect";
const OBSERVE: &str = "recipient-buff-observe";
const WATCH: &str = "counterfactual-recipient-buff-observer";

#[derive(Clone)]
pub(super) struct Census {
    queries: Vec<ContributionQuery>,
    registry_coverage: SchemaClosure,
}

fn owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor_slot()))
}
fn unit(stat: u64) -> UnitDefId {
    match stat {
        0x322b => d(2),
        0x322c => d(1),
        _ => panic!("recipient scaling channel"),
    }
}
fn kind(stat: u64) -> ContributionKind {
    match stat {
        0x322b => ContributionKind::Increase,
        0x322c => ContributionKind::Multiply,
        _ => panic!("recipient scaling channel"),
    }
}
fn program(stat: u64) -> &'static str {
    match stat {
        0x322b => INCREASE,
        0x322c => MORE,
        _ => panic!("recipient scaling channel"),
    }
}
fn known(stat: u64, value: f64) -> EffectValue {
    EffectValue::Known {
        value: quantity(value, &unit(stat)),
    }
}

pub(super) fn install(sniper: &mut sniper::World, endpoint: &StagedOwnedRelease) -> Census {
    buff_effect_family::assert_component(endpoint);
    let recipe = &endpoint.input().recipe;
    let inner = &mut sniper.base.source.base.inner;
    for stat in [0x322b, 0x322c] {
        let address = d::<StatDefinition>(stat).address();
        let actual = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
            .unwrap();
        if let Some(existing) = inner
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
        {
            assert_eq!(existing, actual);
        } else {
            inner.schema.definitions.push(actual.clone());
        }
        inner.owner_mut(SchemaSubject::Definition(address));
    }
    let actual_owner = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == owner())
        .unwrap();
    assert!(!actual_owner.programs.is_complete());
    let selected = inner.owner_mut(owner());
    assert!(
        selected.programs.is_complete(),
        "explicit existing finite owner"
    );
    let programs = buff_effect_family::programs();
    assert_eq!(programs.len(), 2);
    for row in programs {
        assert_eq!(row.owner, owner());
        assert!(actual_owner.programs.members.contains(&row.program));
        assert!(
            !selected
                .programs
                .members
                .iter()
                .any(|p| p.id == row.program.id)
        );
        assert!(
            row.program
                .nodes
                .iter()
                .all(|n| !matches!(n.expression, RuleExpression::Literal { .. }))
        );
        selected.programs.members.push(row.program);
    }
    let registry = recipe.rules.contribution_queries.as_ref().unwrap();
    assert!(!registry.is_complete());
    let queries = buff_effect_family::queries();
    assert_eq!(queries.len(), 2);
    assert!(sniper.base.contribution_queries.is_complete());
    for query in &queries {
        assert!(registry.members.contains(query));
        assert_eq!(query.groups.len(), 1);
        assert!(query.groups[0].members.is_complete());
        assert!(query.groups[0].members.members.is_empty());
        assert!(
            !sniper
                .base
                .contribution_queries
                .members
                .iter()
                .any(|q| q.id == query.id)
        );
        // The actual complete-empty group is retained unchanged. Only the
        // surrounding pre-existing finite fixture closes its selected registry.
        sniper.base.contribution_queries.members.push(query.clone());
    }
    Census {
        queries,
        registry_coverage: registry.closure.clone(),
    }
}

pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.extend([
        EvaluationStage {
            id: key(RESOLVE),
            predecessors: vec![key("deliver")],
        },
        EvaluationStage {
            id: key(OBSERVE),
            predecessors: vec![key(RESOLVE)],
        },
    ]);
    for row in &mut stages.programs.members {
        if row.owner == owner() && matches!(row.program.as_str(), INCREASE | MORE) {
            row.stage = key(RESOLVE);
        }
        if row.program == key(WATCH) {
            row.stage = key(OBSERVE);
        }
    }
    for stat in [0x322b, 0x322c] {
        stages.frozen_channels.extend([
            FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: d(stat),
                    contribution: kind(stat),
                },
                stage: key("deliver"),
            },
            FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Actor,
                    stat: d(stat),
                },
                stage: key(RESOLVE),
            },
        ]);
    }
}

pub(super) fn check_baseline(w: &World, report: &SupportEffectsReport) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let effects = sniper::offering::effects(report);
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    assert_ne!(w.sniper.actor(0), w.sniper.actor(1));
    for (stat, expected) in [(0x322b, 0.), (0x322c, 1.)] {
        let rows: Vec<_> = effects
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key(program(stat)))
            .collect();
        assert_eq!(rows.len(), 2);
        for index in 0..2 {
            let actor = w.sniper.actor(index);
            let target = BoundEffectTarget::Value {
                key: PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: d(stat),
                },
            };
            let matched: Vec<_> = rows.iter().filter(|e| e.target == target).collect();
            assert_eq!(matched.len(), 1, "one exact invocation per recipient");
            assert_eq!(matched[0].key.invocation.owner, owner());
            assert_eq!(
                matched[0].key.invocation.entity,
                ConcreteEntity::Actor(actor)
            );
            let mut provider = w.action(index).action.provider;
            assert_eq!(
                provider.grant_path.pop(),
                Some(slot(SlotOwnerDefId::Actor(d(0x3091)), 0x3093))
            );
            assert_eq!(
                matched[0].key.invocation.origin,
                RuleOrigin::Provider { provider }
            );
            assert_eq!(matched[0].value, known(stat, expected));
            assert_eq!(w.value(effects, index, false, stat), &known(stat, expected));
        }
        assert_eq!(
            effects
                .values
                .iter()
                .filter(|v| matches!(&v.key, PlanValueKey::Stat { stat: s, .. } if *s == d(stat)))
                .count(),
            2,
            "no Player, Skill or Action substitute for the two recipients"
        );
        assert!(!effects.effects.iter().any(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.stat == d(stat))),
            "the actual incoming domain is empty, not neutral-valued producers");
    }
    for query in &w.recipient_buffs.queries {
        let actual = w
            .sniper
            .base
            .contribution_queries
            .members
            .iter()
            .find(|q| q.id == query.id)
            .unwrap();
        assert_eq!(actual, query, "published empty groups are unchanged");
    }
}

fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}
fn query_mut(w: &mut World, stat: u64) -> &mut ContributionQuery {
    w.sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.stat == d(stat))
        .unwrap()
}
fn potential(w: &mut World, stat: u64, value: Option<f64>, enabled: bool, player: bool) {
    // Binding controls only. These are never admitted game modifiers, source
    // parity inputs, query members or alternate scalar producers.
    let missing = def::<StatDefinition>("counterfactual.missing-recipient-buff-input");
    let mut reads = vec![];
    let amount = if let Some(value) = value {
        RuleExpression::Literal {
            value: quantity(value, &unit(stat)),
        }
    } else {
        inner(w)
            .schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: missing.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Quantity { unit: unit(stat) },
                    targets: vec![RuleEntityKind::Actor],
                }),
            }));
        reads.push(RuleRead {
            id: key("missing"),
            value_type: ComputedValueType::Quantity { unit: unit(stat) },
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: missing,
            },
        });
        RuleExpression::Read {
            input: key("missing"),
        }
    };
    inner(w)
        .owner_mut(subject(d::<ActorDefinition>(0x3091)))
        .programs
        .members
        .push(RuleProgram {
            id: key("counterfactual-recipient-buff-potential"),
            context: RuleEntityKind::Actor,
            reads,
            nodes: vec![
                RuleNode {
                    id: key("amount"),
                    expression: amount,
                },
                RuleNode {
                    id: key("enabled"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(enabled),
                    },
                },
            ],
            effects: vec![RuleEffect {
                id: key("potential"),
                when: Some(key("enabled")),
                effect: RuleEffectKind::Contribute {
                    entity: if player {
                        RuleEntity::Player
                    } else {
                        RuleEntity::Current
                    },
                    stat: d(stat),
                    contribution: kind(stat),
                    value: key("amount"),
                },
            }],
        });
}
fn assert_membership_refusal(w: &World) {
    let error = w
        .checked_plan()
        .err()
        .expect("potential contribution must invalidate the empty domain");
    assert!(
        error.contains("actual contribution has no declared membership"),
        "{error}"
    );
}
fn assert_unavailable(report: &SupportEffectsReport) {
    assert_eq!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None,
        }
    );
}
fn observe(w: &mut World, stat: u64) -> StatDefId {
    let output = def::<StatDefinition>("counterfactual.observed-recipient-buff");
    inner(w)
        .schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: output.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: unit(stat) },
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    inner(w)
        .owner_mut(owner())
        .programs
        .members
        .push(RuleProgram {
            id: key(WATCH),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("required"),
                value_type: ComputedValueType::Quantity { unit: unit(stat) },
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: d(stat),
                },
            }],
            nodes: vec![RuleNode {
                id: key("observed"),
                expression: RuleExpression::Read {
                    input: key("required"),
                },
            }],
            effects: vec![RuleEffect {
                id: key("observed"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: output.clone(),
                    value: key("observed"),
                },
            }],
        });
    output
}

#[test]
#[ignore = "requires current recipient-buff publication and retained source evidence"]
fn recipient_buff_actual_empty_domain_reaches_each_item_driven_actor() {
    let w = World::load();
    buff_effect_family::evidence::check(&buff_effect_family::evidence::read(), false);
    buff_effect_family::evidence::assert_empty_baseline();
    let report = w.evaluate();
    check_baseline(&w, &report);
    let cases = evidence::checked_cases();
    let baseline = cases.iter().find(|r| r["physical_level"] == 20).unwrap();
    w.check(&report, [baseline, baseline]);
    let mut different = w.clone();
    different.sniper.raw(1, 1, 20., 0.);
    // Raw occurrence changes must not collapse the two exact recipient outputs.
    check_baseline(&different, &different.evaluate());
}

#[test]
#[ignore = "requires current recipient-buff publication; potential-source refusal controls"]
fn recipient_buff_empty_domains_reject_nonempty_before_guards_or_values() {
    for stat in [0x322b, 0x322c] {
        for value in [Some(0.), Some(1.), Some(1.5), Some(-0.25), None] {
            for enabled in [false, true] {
                let mut w = World::load();
                potential(&mut w, stat, value, enabled, false);
                assert_membership_refusal(&w);
            }
        }
        // The potential inventory cannot be bypassed by moving an effect to an
        // unread recipient of the same checked channel.
        let mut other_recipient = World::load();
        potential(&mut other_recipient, stat, Some(1.), false, true);
        assert_membership_refusal(&other_recipient);
    }
}

#[test]
#[ignore = "requires current recipient-buff publication; current and synthetic incomplete inventories"]
fn recipient_buff_partial_registry_group_or_owner_never_becomes_identity() {
    let original = World::load();
    let mut registry = original.clone();
    registry.sniper.base.contribution_queries.closure =
        original.recipient_buffs.registry_coverage.clone();
    let p = registry.plan();
    assert_eq!(
        p.gaps(),
        &[PlanGap {
            provider: None,
            subject: None,
            reason: PlanGapReason::IncompleteContributors
        }]
    );
    assert_unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
    for stat in [0x322b, 0x322c] {
        let mut partial = original.clone();
        query_mut(&mut partial, stat).groups[0].members.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: subject(d::<StatDefinition>(stat)),
                facet: SchemaFacet::GameRules,
                code: key("counterfactual-unreviewed-recipient-buff"),
            }],
        };
        let p = partial.plan();
        assert_eq!(
            p.gaps(),
            &[PlanGap {
                provider: None,
                subject: Some(subject(d::<StatDefinition>(stat))),
                reason: PlanGapReason::IncompleteContributors
            }]
        );
        assert_unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
    }
    let mut partial = original.clone();
    inner(&mut partial).owner_mut(owner()).programs.closure =
        original.actual_actor_coverage.clone();
    let p = partial.plan();
    assert!(p.gaps().iter().any(
        |g| g.subject.as_ref() == Some(&owner()) && g.reason == PlanGapReason::PartialPrograms
    ));
    assert_unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
}

#[test]
#[ignore = "requires current recipient-buff publication; missing producer and frozen stage controls"]
fn recipient_buff_missing_writer_and_early_reads_stay_unavailable() {
    for stat in [0x322b, 0x322c] {
        let mut w = World::load();
        let observed = observe(&mut w, stat);
        inner(&mut w)
            .owner_mut(owner())
            .programs
            .members
            .retain(|p| p.id != key(program(stat)));
        let report = w.evaluate();
        let effects = sniper::offering::effects(&report);
        assert!(
            !effects
                .values
                .iter()
                .any(|v| matches!(&v.key, PlanValueKey::Stat { stat: s, .. } if *s == d(stat)))
        );
        for index in 0..2 {
            let found: Vec<_> = effects
                .values
                .iter()
                .filter(|v| {
                    v.key
                        == PlanValueKey::Stat {
                            entity: ConcreteEntity::Actor(w.sniper.actor(index)),
                            stat: observed.clone(),
                        }
                })
                .collect();
            assert_eq!(found.len(), 1);
            assert_eq!(
                found[0].value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    read: Some(key("required"))
                }
            );
        }
        let other = if stat == 0x322b { 0x322c } else { 0x322b };
        for index in 0..2 {
            assert_eq!(
                w.value(effects, index, false, other),
                &known(other, if other == 0x322b { 0. } else { 1. })
            );
        }
        let w = World::load();
        let error = w
            .checked_plan_configured(|stages| {
                stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.owner == owner() && p.program == key(program(stat)))
                    .unwrap()
                    .stage = key("facts");
            })
            .err()
            .expect("the resolved recipient scalar cannot precede its incoming freeze");
        assert!(
            error.contains("stage") || error.contains("frozen"),
            "{error}"
        );
    }
}

#[test]
#[ignore = "requires current recipient-buff publication; A-unknown-B-A and parallel scratch"]
fn recipient_buff_recipient_identity_and_coverage_survive_scratch_reuse() {
    let a = World::load();
    let mut b = a.clone();
    b.sniper.raw(1, 1, 20., 0.);
    let pa = a.plan();
    let pb = b.plan();
    let mut incomplete = a.clone();
    incomplete.sniper.base.contribution_queries.closure =
        a.recipient_buffs.registry_coverage.clone();
    let pu = incomplete.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check_baseline(&a, &first);
    assert_unavailable(&pu.evaluate(&mut scratch).unwrap());
    let second = pb.evaluate(&mut scratch).unwrap();
    check_baseline(&b, &second);
    assert_ne!(
        first, second,
        "independent raw Skill inputs still affect the real graph"
    );
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), first);
    assert_eq!(pb.evaluate(&mut pb.new_scratch()).unwrap(), second);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |scratch, i| match i % 3 {
                    0 => pa.evaluate(scratch).unwrap(),
                    1 => pu.evaluate(scratch).unwrap(),
                    _ => pb.evaluate(scratch).unwrap(),
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, report) in reports.iter().enumerate() {
        match i % 3 {
            0 => assert_eq!(report, &first),
            1 => assert_unavailable(report),
            _ => assert_eq!(report, &second),
        }
    }
}
