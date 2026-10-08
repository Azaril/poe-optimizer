//! Checked Life reductions in the existing finite joined graph. These probes
//! derive diagnostic test stats, never canonical/final Life. Production owners
//! and incoming inventories remain Partial; minion INC, conversions, overrides
//! and complete resource arithmetic are not certified by this fixture.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};

const READY: &str = "life-contributions-ready";
const PROBE: &str = "life-query-probes";
const CHANNELS: [(&str, ContributionKind, u64, f64); 3] = [
    ("fixture.life-add", ContributionKind::Add, 0x3119, 0.),
    ("fixture.life-inc", ContributionKind::Increase, 2, 0.),
    ("fixture.life-more", ContributionKind::Multiply, 1, 1.),
];
fn member(
    owner: SchemaSubject,
    program: &str,
    effect: &str,
    origin: ContributionOrigin,
    rank: u32,
) -> ContributionMember {
    ContributionMember {
        owner,
        program: key(program),
        effect: key(effect),
        origin,
        order: Some(ContributionOrder {
            source_rank: rank,
            slot_ranks: vec![],
            program_rank: 0,
            effect_rank: 0,
        }),
    }
}
fn world() -> World {
    let mut w = World::load();
    let f = &mut w.sniper.base.source.base.inner;
    assert_eq!(f.operations, key(OWNED_RULE_OPERATIONS_V22));
    f.operations = key(OWNED_RULE_OPERATIONS_V23);
    let application = f.existing_actor_rules.as_ref().unwrap().members[0]
        .id
        .clone();
    let shared = ContributionOrigin::ExistingActor { application };
    let supplied = ContributionOrigin::SuppliedActor {
        slots: vec![actor_slot()],
    };
    let actor = SchemaSubject::Slot(SlotAddress::Actor(actor_slot()));
    let slots: Vec<_> = [0x67, 0x68, 0x6b, 0x6c, 0x6e].into_iter().map(d).collect();
    let mut equipment = member(
        subject(d::<ModifierDefinition>(0x3100)),
        "contribute-player-flat-life",
        "direct-flat-life",
        ContributionOrigin::ItemModifier {
            slots: slots.clone(),
        },
        2,
    );
    equipment.order.as_mut().unwrap().slot_ranks = slots
        .into_iter()
        .enumerate()
        .map(|(rank, slot)| ContributionSlotRank {
            slot,
            rank: rank as u32,
        })
        .collect();
    // Explicit test order only: current selected Player BASE operands are
    // nonnegative integers with subtotal < 2^53. This does not claim a general
    // game fold law for arbitrary equipment, rewards or fractional Life.
    let mut members = [
        vec![
            member(
                subject(d::<ActorDefinition>(0x332a)),
                "intrinsic-player-life",
                "intrinsic-life",
                shared.clone(),
                0,
            ),
            member(
                subject(d::<ActorDefinition>(0x332a)),
                "contribute-inherent-strength-life",
                "inherent-life",
                shared,
                1,
            ),
            equipment,
            member(
                subject(d::<RewardDefinition>(0x29)),
                "flat-resource-contribution",
                "grant",
                ContributionOrigin::Reward,
                3,
            ),
            member(
                actor.clone(),
                "intrinsic-allied-minion-life",
                "intrinsic-life",
                supplied.clone(),
                0,
            ),
        ],
        vec![member(
            subject(d::<RewardDefinition>(0x53)),
            "permanent-reward-contributions",
            "grant-0",
            ContributionOrigin::Reward,
            0,
        )],
        vec![member(
            actor,
            "gigantic-life-and-damage",
            "life_more",
            supplied,
            0,
        )],
    ];
    // Both shared programs have one owner/source rank; program order is a
    // separate semantic component, not a second shared Actor occurrence.
    let inherent_order = members[0][1].order.as_mut().unwrap();
    inherent_order.source_rank = 0;
    inherent_order.program_rank = 1;
    for ((name, kind, unit, empty), members) in CHANNELS.into_iter().zip(members) {
        let stat: StatDefId = def(name);
        let ty = ComputedValueType::Quantity { unit: d(unit) };
        f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: stat.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: ty.clone(),
                    targets: vec![RuleEntityKind::Actor],
                }),
            }));
        f.owners.push(DefinitionRules {
            owner: subject(stat.clone()),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key(name),
                context: RuleEntityKind::Actor,
                reads: vec![RuleRead {
                    id: key("value"),
                    value_type: ty,
                    source: RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Current,
                        query: key(name),
                        group: key("sources"),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Read {
                        input: key("value"),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("observe"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: stat.clone(),
                        value: key("value"),
                    },
                }],
            }]),
        });
        w.sniper.receivers.members.push(StatReceiver {
            id: key(name),
            stat,
            program: key(name),
            targets: vec![
                StatReceiverTarget::Player,
                StatReceiverTarget::OwnedSlot { slot: actor_slot() },
            ],
        });
        w.sniper
            .base
            .contribution_queries
            .members
            .push(ContributionQuery {
                id: key(name),
                stat: d(0x311a),
                contribution: kind,
                groups: vec![ContributionGroup {
                    id: key("sources"),
                    reduction: if kind == ContributionKind::Multiply {
                        ContributionReduction::Product
                    } else {
                        ContributionReduction::Sum
                    },
                    ordering: ContributionOrdering::Ordered,
                    empty: quantity(empty, &d(unit)),
                    members: DeclaredSet::complete(members),
                }],
            });
    }
    w
}
fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.extend([
        EvaluationStage {
            id: key(READY),
            predecessors: vec![
                key("player-inherent-life-contribution"),
                key("gigantic-benefits"),
            ],
        },
        EvaluationStage {
            id: key(PROBE),
            predecessors: vec![key(READY)],
        },
    ]);
    for row in &mut stages.programs.members {
        if CHANNELS.iter().any(|(name, ..)| row.program == key(name)) {
            row.stage = key(PROBE);
        }
    }
    for row in &mut stages.readiness.as_mut().unwrap().programs.members {
        if CHANNELS.iter().any(|(name, ..)| row.program == key(name)) {
            row.phase = ReadinessPhase::Execution;
            row.role = ReadinessProgramRole::Execution;
            row.outputs.clear();
        }
    }
    for (_, kind, ..) in CHANNELS {
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: d(0x311a),
                contribution: kind,
            },
            stage: key(READY),
        });
    }
}
fn plan(w: &World) -> shared::Plan {
    w.checked_plan_configured(configure).unwrap()
}
fn value<'a>(r: &'a SupportEffectsReport, actor: ActorKey, name: &str) -> &'a EffectValue {
    &sniper::offering::effects(r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: def(name),
                }
        })
        .unwrap()
        .value
}
fn player(r: &SupportEffectsReport, base: f64, increase: f64) {
    for ((name, _, unit, _), expected) in CHANNELS.into_iter().zip([base, increase, 1.]) {
        assert_eq!(
            value(r, ActorKey::Player, name),
            &EffectValue::Known {
                value: quantity(expected, &d(unit))
            }
        );
    }
    assert!(
        !sniper::offering::effects(r)
            .values
            .iter()
            .any(|v| matches!(&v.key, PlanValueKey::Stat {stat,..} if *stat == d(0x311a))),
        "a checked subtotal is still not a final Life pool"
    );
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn actual_life_sources_reduce_on_exact_player_and_minion_recipients() {
    let w = world();
    let p = plan(&w);
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(r.gaps.is_empty());
    player_life_contribution_native::check(&w, &r, 1120., 54.);
    player_life_inputs_native::check(&w, &r);
    player(&r, 1257., 5.);
    let effects = sniper::offering::effects(&r);
    for index in 0..2 {
        for (name, kind, ..) in [CHANNELS[0], CHANNELS[2]] {
            let rows: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    matches!(&e.target,
                BoundEffectTarget::Contribution { key } if key.stat == d(0x311a) && key.kind == kind
                    && key.entity == ConcreteEntity::Actor(w.sniper.actor(index)))
                })
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(value(&r, w.sniper.actor(index), name), &rows[0].value);
        }
        assert_eq!(
            value(&r, w.sniper.actor(index), CHANNELS[1].0),
            &EffectValue::Known {
                value: quantity(0., &d(2))
            }
        );
    }
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn ring_copies_rewards_and_character_changes_flow_through_checked_life_queries() {
    let a = world();
    let pa = plan(&a);
    let ra = pa.evaluate(&mut pa.new_scratch()).unwrap();
    let mut b = a.clone();
    let ring = player_life_inputs_native::change_ring(&mut b, 14.);
    let f = &mut b.sniper.base.source.base.inner;
    f.build.character.level += 1;
    f.build
        .character
        .rewards
        .retain(|r| r.definition != d(0x29));
    let removed = f
        .build
        .equipment
        .iter()
        .find(|e| e.item == ring)
        .unwrap()
        .id;
    f.build.equipment.retain(|e| e.id != removed);
    let pb = plan(&b);
    let rb = pb.evaluate(&mut pb.new_scratch()).unwrap();
    player(&rb, 1243., 5.); // +12 level, -20 quest, two 10-Life uses become one 14-Life use.
    assert_ne!(pa.identity(), pb.identity());
    for index in 0..2 {
        for (name, ..) in CHANNELS {
            assert_eq!(
                value(&ra, a.sniper.actor(index), name),
                value(&rb, b.sniper.actor(index), name)
            );
        }
    }
    let mut scratch = pa.new_scratch();
    for (p, expected) in [(&pa, &ra), (&pb, &rb), (&pa, &ra)] {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), *expected);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |scratch, i| {
                    let (p, expected) = if i % 2 == 0 { (&pa, &ra) } else { (&pb, &rb) };
                    (p.evaluate(scratch).unwrap(), expected)
                },
            )
            .collect()
    });
    for (r, expected) in results {
        assert_eq!(&r, expected);
    }
    let mut reordered = b.clone();
    let f = &mut reordered.sniper.base.source.base.inner;
    f.build.items.reverse();
    f.build.equipment.reverse();
    f.build.character.rewards.reverse();
    f.owners.reverse();
    for q in &mut reordered.sniper.base.contribution_queries.members {
        for g in &mut q.groups {
            g.members.members.reverse();
        }
    }
    let p = plan(&reordered);
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(
        sniper::offering::effects(&r).values,
        sniper::offering::effects(&rb).values
    );
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn unread_minion_membership_and_partial_life_queries_cannot_be_skipped() {
    let mut w = world();
    w.sniper
        .receivers
        .members
        .retain(|r| !CHANNELS.iter().any(|(name, ..)| r.id == key(name)));
    let q = w
        .sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.id == key(CHANNELS[0].0))
        .unwrap();
    q.groups[0]
        .members
        .members
        .retain(|m| m.program != key("intrinsic-allied-minion-life"));
    assert!(
        w.checked_plan_configured(configure)
            .is_err_and(|e| e.contains("no declared membership"))
    );
    let mut w = world();
    let q = w
        .sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.id == key(CHANNELS[0].0))
        .unwrap();
    q.groups[0].members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(d::<StatDefinition>(0x311a)),
            facet: SchemaFacet::GameRules,
            code: key("unconverted-life-sources"),
        }],
    };
    let p = plan(&w);
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved { .. },
            ..
        }
    ));
}
