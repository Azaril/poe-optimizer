//! Existing Actor-relative attribute programs run on exact supplied recipients.
//! This finite graph proves the observed empty attribute domain, not nonempty
//! minion attribute mechanics, resource transformations or complete Life.
use super::*;
use poe_optimizer_core::owned_stages::EvaluationStage;
use poe_optimizer_import::owned_release::StagedOwnedRelease;

pub(super) const STAGE: &str = "minion-inherent-life-contribution";
const BRIDGE: &str = "contribute-inherent-strength-life";

fn owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor_slot()))
}

pub(super) fn install(sniper: &mut sniper::World, endpoint: &StagedOwnedRelease) {
    minion_inherent_family::assert_component(endpoint);
    let program = minion_inherent_family::program();
    assert_eq!(program, player_life_family::program());
    let f = &mut sniper.base.source.base.inner;
    for receiver in minion_inherent_family::receivers() {
        assert_eq!(
            receiver.targets,
            [
                StatReceiverTarget::Player,
                StatReceiverTarget::OwnedSlot { slot: actor_slot() }
            ]
        );
        let rules = f
            .owners
            .iter()
            .find(|o| o.owner == subject(receiver.stat.clone()))
            .unwrap();
        let published = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == rules.owner)
            .unwrap();
        assert_eq!(
            rules, published,
            "reuse the unchanged Actor-relative program"
        );
        assert!(
            rules
                .programs
                .members
                .iter()
                .any(|p| p.id == receiver.program)
        );
        assert_eq!(
            sniper
                .receivers
                .members
                .iter()
                .filter(|r| r.id == receiver.id)
                .collect::<Vec<_>>(),
            [&receiver],
            "reuse the existing receiver row once"
        );
    }
    let actor = f.owner_mut(owner());
    assert!(!actor.programs.members.iter().any(|p| p.id == program.id));
    actor.programs.members.push(program);
}

pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.push(EvaluationStage {
        id: key(STAGE),
        predecessors: vec![key("inherent-strength-life")],
    });
    for row in &mut stages.programs.members {
        if row.owner == owner() && row.program == key(BRIDGE) {
            row.stage = key(STAGE);
        }
    }
    // The existing Player bridge already freezes the same Actor-scoped amount
    // at inherent-strength-life. Do not create another channel or duplicate freeze.
}

pub(super) fn check(w: &World, report: &SupportEffectsReport) {
    let effects = sniper::offering::effects(report);
    let receivers = minion_inherent_family::receivers();
    assert_eq!(receivers.len(), 8);
    let bridges: Vec<_> = effects
        .effects
        .iter()
        .filter(|e| e.key.invocation.owner == owner() && e.key.invocation.program == key(BRIDGE))
        .collect();
    assert_eq!(
        bridges.len(),
        2,
        "one real zero contribution per supplied Actor"
    );
    for index in 0..2 {
        let actor = w.sniper.actor(index);
        for receiver in &receivers {
            let expected = if [d(0x3321), d(0x1d2e)].contains(&receiver.stat) {
                ParameterValue::Integer(BoundedInteger::new(0).unwrap())
            } else if receiver.stat == d(0x331a) {
                quantity(0., &d(0x3119))
            } else {
                ParameterValue::Boolean(false)
            };
            let rows: Vec<_> = effects
                .values
                .iter()
                .filter(|r| {
                    r.key
                        == PlanValueKey::Stat {
                            entity: ConcreteEntity::Actor(actor.clone()),
                            stat: receiver.stat.clone(),
                        }
                })
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].value,
                EffectValue::Known {
                    value: expected.clone()
                }
            );
            let origins: Vec<_> = effects
                .effects
                .iter()
                .filter(|r| {
                    r.key.invocation.origin
                        == RuleOrigin::Receiver {
                            receiver: receiver.id.clone(),
                            actor: actor.clone(),
                        }
                })
                .collect();
            assert_eq!(origins.len(), 1);
            assert_eq!(
                origins[0].key.invocation.owner,
                subject(receiver.stat.clone())
            );
            assert_eq!(origins[0].key.invocation.program, receiver.program);
            assert_eq!(
                origins[0].key.invocation.entity,
                ConcreteEntity::Actor(actor.clone())
            );
            assert_eq!(origins[0].value, EffectValue::Known { value: expected });
        }
        let rows: Vec<_> = bridges
            .iter()
            .filter(|r| r.key.invocation.entity == ConcreteEntity::Actor(actor.clone()))
            .collect();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: d(0x311a),
                    kind: ContributionKind::Add,
                }
            }
        );
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(0., &d(0x3119))
            }
        );
        let ActorKey::Owned(actor) = actor else {
            unreachable!()
        };
        let mut provider = actor.provider;
        provider
            .grant_path
            .push(slot(SlotOwnerDefId::Skill(d(0x12)), 0x20));
        assert_eq!(row.key.invocation.origin, RuleOrigin::Provider { provider });
    }
    assert!(
        !effects
            .values
            .iter()
            .any(|r| matches!(&r.key, PlanValueKey::Stat {stat, ..} if *stat == d(0x311a))),
        "these source contributions are not a canonical final Life pool"
    );
}

#[test]
#[ignore = "requires current published minion inherent Life packet"]
fn actual_empty_attributes_produce_exact_minion_life_zero_without_player_leakage() {
    let w = World::load();
    let report = w.evaluate();
    check(&w, &report);
    player_life_contribution_native::check(&w, &report, 1120., 54.);
    life_increase_native::check(&w, &report);
    // Neither absent attribute MORE source is replaced with a neutral producer.
    for index in 0..2 {
        assert!(!sniper::offering::effects(&report).values.iter().any(
            |r| matches!(&r.key, PlanValueKey::Stat {entity, stat}
                if *entity == ConcreteEntity::Actor(w.sniper.actor(index))
                    && [d(0x3324),d(0x3327)].contains(stat))
        ));
    }
}

#[test]
#[ignore = "requires current published minion inherent Life packet"]
fn missing_minion_strength_is_unavailable_without_affecting_the_player() {
    let original = World::load();
    let mut missing = original.clone();
    let final_strength = minion_inherent_family::receivers()
        .into_iter()
        .find(|r| r.stat == d(0x1d2e))
        .unwrap();
    let receiver = missing
        .sniper
        .receivers
        .members
        .iter_mut()
        .find(|r| r.id == final_strength.id)
        .unwrap();
    assert_eq!(receiver.targets.len(), 2);
    receiver
        .targets
        .retain(|target| *target == StatReceiverTarget::Player);
    assert_eq!(receiver.targets, [StatReceiverTarget::Player]);
    let p = missing.plan();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    let effects = sniper::offering::effects(&report);
    player_life_contribution_native::check(&missing, &report, 1120., 54.);
    for index in 0..2 {
        let row = effects
            .values
            .iter()
            .find(|r| {
                r.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Actor(missing.sniper.actor(index)),
                        stat: d(0x331a),
                    }
            })
            .unwrap();
        assert_eq!(
            row.value,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: Some(key("strength")),
            }
        );
        let bridge: Vec<_> = effects
            .effects
            .iter()
            .filter(|row| {
                row.key.invocation.owner == owner()
                    && row.key.invocation.program == key(BRIDGE)
                    && row.key.invocation.entity
                        == ConcreteEntity::Actor(missing.sniper.actor(index))
            })
            .collect();
        assert_eq!(bridge.len(), 1);
        assert!(
            matches!(
                bridge[0].value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ),
            "missing Strength must not become an enabled Life zero"
        );
    }
    let a = original.plan();
    let expected = a.evaluate(&mut a.new_scratch()).unwrap();
    let mut scratch = a.new_scratch();
    for (plan, expected) in [(&a, &expected), (&p, &report), (&a, &expected)] {
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), expected);
    }
}
