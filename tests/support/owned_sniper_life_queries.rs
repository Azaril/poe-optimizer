//! Checked Life reductions in the existing finite joined graph. These probes
//! derive diagnostic test stats, never canonical/final Life. Production owners
//! and incoming inventories remain Partial; minion INC, conversions, overrides
//! and complete resource arithmetic are not certified by this fixture.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};

const READY: &str = "life-contributions-ready";
const PROBE: &str = "life-query-probes";
const CHANNELS: [(&str, ContributionKind, u64); 3] = [
    ("fixture.life-add", ContributionKind::Add, 0x3119),
    ("fixture.life-inc", ContributionKind::Increase, 2),
    ("fixture.life-more", ContributionKind::Multiply, 1),
];
const QUERY_IDS: [&str; 3] = [
    "life-base-contributions",
    "life-increased-contributions",
    "life-more-contributions",
];
fn world() -> World {
    let mut w = World::load();
    let f = &mut w.sniper.base.source.base.inner;
    assert_eq!(f.operations, key(OWNED_RULE_OPERATIONS_V23));
    let mut queries = life_query_family::queries();
    // This existing finite graph has no received-minion-Life producer. Keep
    // that one exclusion explicit rather than silently filtering by selection.
    let excluded = queries[1].groups[1].members.members.pop().unwrap();
    assert_eq!(excluded.program, key("received-minion-life-increase"));
    assert!(queries[1].groups[1].members.members.is_empty());
    assert!(
        !f.owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .any(|p| p.id == excluded.program)
    );
    for descriptor in life_query_family::definitions() {
        if matches!(descriptor, DefinitionDescriptor::EquipmentSlot(_))
            && !f
                .schema
                .definitions
                .iter()
                .any(|d| d.address() == descriptor.address())
        {
            f.owner_mut(SchemaSubject::Definition(descriptor.address()));
            f.schema.definitions.push(descriptor);
        }
    }
    for ((name, kind, unit), mut query) in CHANNELS.into_iter().zip(queries) {
        assert_eq!(query.contribution, kind);
        // Only this finite diagnostic graph closes equipment's remaining gap.
        // The packet retains that gap, all donor bodies and no final pool.
        for group in &mut query.groups {
            group.members.closure = SchemaClosure::Complete;
        }
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
        let mut reads = vec![];
        let mut nodes = vec![];
        let mut total = key("part-0");
        for (i, group) in query.groups.iter().enumerate() {
            let part = key(&format!("part-{i}"));
            reads.push(RuleRead {
                id: part.clone(),
                value_type: ty.clone(),
                source: RuleReadSource::ContributionQuery {
                    entity: RuleEntity::Current,
                    query: query.id.clone(),
                    group: group.id.clone(),
                },
            });
            nodes.push(RuleNode {
                id: part.clone(),
                expression: RuleExpression::Read {
                    input: part.clone(),
                },
            });
            if i > 0 {
                assert_ne!(
                    kind,
                    ContributionKind::Multiply,
                    "MORE remains a single explicit group"
                );
                let next = key(&format!("subtotal-{i}"));
                nodes.push(RuleNode {
                    id: next.clone(),
                    expression: RuleExpression::Add {
                        left: total,
                        right: part,
                    },
                });
                total = next;
            }
        }
        f.owners.push(DefinitionRules {
            owner: subject(stat.clone()),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key(name),
                context: RuleEntityKind::Actor,
                reads,
                nodes,
                effects: vec![RuleEffect {
                    id: key("observe"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: stat.clone(),
                        value: total,
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
        w.sniper.base.contribution_queries.members.push(query);
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
    for ((name, _, unit), expected) in CHANNELS.into_iter().zip([base, increase, 1.]) {
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
        .find(|q| q.id == key(QUERY_IDS[0]))
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
        .find(|q| q.id == key(QUERY_IDS[0]))
        .unwrap();
    q.groups[2].members.closure = life_query_family::queries()[0].groups[2]
        .members
        .closure
        .clone();
    let p = plan(&w);
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved { .. },
            ..
        }
    ));
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn singleton_life_groups_reject_duplicate_rewards_before_values_or_activation() {
    for definition in [d::<RewardDefinition>(0x29), d(0x53)] {
        let mut w = world();
        let f = &mut w.sniper.base.source.base.inner;
        let mut duplicate = f
            .build
            .character
            .rewards
            .iter()
            .find(|r| r.definition == definition)
            .unwrap()
            .clone();
        duplicate.id = id(9101);
        f.build.character.rewards.push(duplicate);
        assert!(
            w.checked_plan_configured(configure)
                .is_err_and(|e| e.contains("semantic positions are tied"))
        );
    }
}
