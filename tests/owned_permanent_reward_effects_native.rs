//! Finite item/reward component checks, not full resource pools or whole builds.
//! Reward descriptors and ordinary rule bodies come unchanged from the packets.
//! Only the shared fixture's existing Spirit reducer provides a scalar sentinel.
#[allow(dead_code)]
#[path = "support/owned_reward_effects_native.rs"]
mod shared;

use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::Deserialize;
use shared::{Fixture, report};

const NEW_REWARDS: [u16; 8] = [
    0x002a, 0x0031, 0x003e, 0x0041, 0x0048, 0x0052, 0x0053, 0x005a,
];

#[derive(Deserialize)]
struct Bindings {
    rewards: Vec<Reward>,
}
#[derive(Deserialize)]
struct Reward {
    reward: RewardDefId,
    program: OwnedDefinitionKey,
    effects: Vec<Effect>,
}
#[derive(Deserialize)]
struct Effect {
    stat: StatDefId,
    unit: UnitDefId,
    contribution: ContributionKind,
    amount: f64,
}

fn number<K: DefinitionDomain>(id: &DefId<K>) -> u16 {
    u16::from_str_radix(id.key().as_str().strip_prefix("def.").unwrap(), 16).unwrap()
}
fn id<K: DefinitionDomain>(f: &Fixture, number: u16) -> DefId<K> {
    DefId::parse(f.build.game_version.clone(), format!("def.{number:016x}")).unwrap()
}

// Independent finite expectations for the actual twelve selected Reward owners.
// Increase contributions use percent, even when the target's base unit is Life,
// Mana, Armour, Evasion or Energy Shield. No pool formula is implemented here.
fn expected(reward: u16) -> Vec<(u16, ContributionKind, f64, u16)> {
    use ContributionKind::{Add, Increase};
    match reward {
        0x0028 | 0x0030 => vec![(0x3166, Add, 30.0, 0x0004)],
        0x0029 => vec![(0x311a, Add, 20.0, 0x3119)],
        0x0063 => vec![(0x3166, Add, 40.0, 0x0004)],
        0x002a => vec![(0x32e7, Add, 10.0, 0x0002)],
        0x0031 => vec![(0x32e6, Add, 10.0, 0x0002)],
        0x003e => vec![(0x29f9, Increase, 5.0, 0x0002)],
        0x0048 => vec![(0x32e6, Add, 5.0, 0x0002)],
        0x0052 => vec![(0x32e7, Add, 5.0, 0x0002)],
        0x0053 => vec![(0x311a, Increase, 5.0, 0x0002)],
        0x0041 => vec![
            (0x29f2, Increase, 30.0, 0x0002),
            (0x29f1, Increase, 30.0, 0x0002),
            (0x29f0, Increase, 30.0, 0x0002),
        ],
        0x005a => vec![
            (0x29f2, Increase, 15.0, 0x0002),
            (0x29f1, Increase, 15.0, 0x0002),
            (0x29f0, Increase, 15.0, 0x0002),
        ],
        _ => panic!("unlisted finite Reward definition {reward:04x}"),
    }
}
fn fixture() -> Fixture {
    shared::fixture(&["flat-resource-rewards", "permanent-reward-effects"])
}
fn check_rewards(f: &Fixture, result: &OwnedEffectsReport) {
    let effects: Vec<_> = result
        .effects
        .iter()
        .filter_map(|effect| {
            let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
                return None;
            };
            let ProviderRoot::Reward(id) = provider.root else {
                return None;
            };
            assert!(provider.grant_path.is_empty());
            let selected = f
                .build
                .character
                .rewards
                .iter()
                .find(|r| r.id == id)
                .unwrap();
            assert_eq!(
                effect.key.invocation.owner,
                SchemaSubject::Definition(selected.definition.address())
            );
            assert_eq!(
                effect.key.invocation.entity,
                ConcreteEntity::Actor(ActorKey::Player)
            );
            Some((id, effect))
        })
        .collect();
    assert_eq!(
        effects.len(),
        f.build
            .character
            .rewards
            .iter()
            .map(|r| expected(number(&r.definition)).len())
            .sum::<usize>()
    );
    for reward in &f.build.character.rewards {
        let wanted = expected(number(&reward.definition));
        let actual: Vec<_> = effects
            .iter()
            .filter(|(id, _)| *id == reward.id)
            .map(|(_, effect)| *effect)
            .collect();
        assert_eq!(
            actual.len(),
            wanted.len(),
            "whole Reward occurrence {}",
            reward.definition.key()
        );
        for (index, (stat, kind, amount, unit)) in wanted.into_iter().enumerate() {
            let target = BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: id(f, stat),
                    kind,
                },
            };
            let matching: Vec<_> = actual
                .iter()
                .filter(|effect| effect.target == target)
                .collect();
            assert_eq!(
                matching.len(),
                1,
                "exact channel for {}",
                reward.definition.key()
            );
            let effect = matching[0];
            assert_eq!(
                effect.value,
                EffectValue::Known {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(amount, id(f, unit)).unwrap()
                    ),
                }
            );
            if NEW_REWARDS.contains(&number(&reward.definition)) {
                assert_eq!(
                    effect.key.invocation.program.as_str(),
                    "permanent-reward-contributions"
                );
                assert_eq!(effect.key.effect.as_str(), format!("grant-{index}"));
            }
        }
    }
}
fn assert_incomplete(f: &Fixture, result: &OwnedEffectsReport) {
    assert!(matches!(
        f.total(result),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}

#[test]
fn twelve_exact_effects_preserve_reward_roots_global_scope_and_channel_units() {
    let bindings: Bindings = shared::packet("permanent-reward-effects", "bindings.json");
    assert_eq!(
        bindings
            .rewards
            .iter()
            .map(|r| number(&r.reward))
            .collect::<Vec<_>>(),
        NEW_REWARDS
    );
    assert_eq!(
        bindings
            .rewards
            .iter()
            .map(|r| r.effects.len())
            .sum::<usize>(),
        12
    );
    for reward in bindings.rewards {
        assert_eq!(reward.program.as_str(), "permanent-reward-contributions");
        let actual: Vec<_> = reward
            .effects
            .iter()
            .map(|e| (number(&e.stat), e.contribution, e.amount, number(&e.unit)))
            .collect();
        assert_eq!(actual, expected(number(&reward.reward)));
    }
    let f = fixture();
    let result = report(&f);
    assert!(result.gaps.is_empty(), "{:?}", result.gaps);
    assert_eq!(f.build.character.rewards.len(), 12);
    check_rewards(&f, &result);
    // Five selected item uses contribute 13 each; the prior flat rewards add100.
    // None of the new resistance/percentage contributions can enter that sum.
    assert_eq!(f.total(&result), &f.expected_total(165.0));
    for stat in [0x32e6, 0x32e7, 0x29f9, 0x311a, 0x29f2, 0x29f1, 0x29f0] {
        let stat: StatDefId = id(&f, stat);
        assert!(
            !result.values.iter().any(|value| matches!(&value.key,
                PlanValueKey::Stat { stat: actual, .. } if actual == &stat
            )),
            "no published or fixture scalar for {}",
            stat.key()
        );
    }
}

#[test]
fn each_whole_reward_omission_removes_exact_effects_without_channel_cross_talk() {
    for omitted in NEW_REWARDS {
        let mut f = fixture();
        f.build
            .character
            .rewards
            .retain(|r| number(&r.definition) != omitted);
        let result = report(&f);
        assert!(result.gaps.is_empty(), "{:?}", result.gaps);
        check_rewards(&f, &result);
        // Includes both three-effect global-defence owners: all three disappear
        // together while the other owner's independent contributions survive.
        assert_eq!(f.total(&result), &f.expected_total(165.0));
    }
}

#[test]
fn item_and_reward_providers_remain_independent() {
    let mut f = fixture();
    f.build.character.rewards.clear();
    let items = report(&f);
    assert!(items.gaps.is_empty());
    check_rewards(&f, &items);
    assert_eq!(f.total(&items), &f.expected_total(65.0));

    let mut f = fixture();
    f.build.equipment.clear();
    let rewards = report(&f);
    assert!(rewards.gaps.is_empty());
    check_rewards(&f, &rewards);
    assert_eq!(f.total(&rewards), &f.expected_total(100.0));
    assert!(!rewards.effects.iter().any(|effect| matches!(&effect.key.invocation.origin,
        RuleOrigin::Provider { provider } if matches!(provider.root, ProviderRoot::ItemModifier { .. })
    )));
}

#[test]
fn flat_life_and_percentage_life_keep_distinct_keys_and_units() {
    let mut f = fixture();
    f.build
        .character
        .rewards
        .retain(|r| matches!(number(&r.definition), 0x0029 | 0x0053));
    f.build.equipment.clear();
    let result = report(&f);
    assert!(result.gaps.is_empty());
    check_rewards(&f, &result);
    let life: StatDefId = id(&f, 0x311a);
    let effects: Vec<_> = result
        .effects
        .iter()
        .filter(|effect| {
            matches!(&effect.target,
                BoundEffectTarget::Contribution { key } if key.stat == life
            )
        })
        .collect();
    assert_eq!(effects.len(), 2);
    assert_eq!(f.total(&result), &f.expected_total(0.0));
}

#[test]
fn missing_and_partial_reward_owners_preserve_the_global_coverage_gate() {
    for partial in [false, true] {
        let mut f = fixture();
        let reward: RewardDefId = id(&f, 0x0041);
        let subject = SchemaSubject::Definition(reward.address());
        let occurrence = f
            .build
            .character
            .rewards
            .iter()
            .find(|r| r.definition == reward)
            .unwrap()
            .id;
        if partial {
            f.recipe
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == subject)
                .unwrap()
                .programs
                .closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: subject.clone(),
                    facet: SchemaFacet::GameRules,
                    code: OwnedDefinitionKey::new("fixture-incomplete-reward").unwrap(),
                }],
            };
        } else {
            f.recipe.rules.owners.retain(|owner| owner.owner != subject);
        }
        let result = report(&f);
        let reason = if partial {
            PlanGapReason::PartialPrograms
        } else {
            PlanGapReason::MissingPrograms
        };
        assert!(
            result
                .gaps
                .iter()
                .any(|gap| gap.subject.as_ref() == Some(&subject)
                    && gap.provider.as_ref()
                        == Some(&ProviderKey {
                            root: ProviderRoot::Reward(occurrence),
                            grant_path: vec![]
                        })
                    && gap.reason == reason)
        );
        assert_incomplete(&f, &result);
        if partial {
            check_rewards(&f, &result);
        }
        f.build.character.rewards.retain(|r| r.id != occurrence);
        let restored = report(&f);
        assert!(restored.gaps.is_empty(), "{:?}", restored.gaps);
        check_rewards(&f, &restored);
        assert_eq!(f.total(&restored), &f.expected_total(165.0));
    }
    // An unrelated selected item owner's authentic Partial closure must remain
    // unresolved even though every newly authored Reward program is complete.
    let mut f = fixture();
    f.restore_partial_rules();
    let result = report(&f);
    check_rewards(&f, &result);
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms
                && gap.subject == Some(SchemaSubject::Definition(f.family.modifier.address())))
    );
    assert_incomplete(&f, &result);
}

#[test]
fn permanent_reward_rejects_foreign_raw_parameter_ownership() {
    let mut f = fixture();
    let selected = f
        .build
        .character
        .rewards
        .iter_mut()
        .find(|r| number(&r.definition) == 0x0053)
        .unwrap();
    selected.parameters.push(ParameterAssignment {
        slot: f.family.amount.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(99.0, f.family.unit.clone()).unwrap()),
    });
    let error = BuildSpec::new(f.build, OwnedInputLimits::default()).unwrap_err();
    assert_eq!(error.kind, StructuralErrorKind::WrongDeclaration);
    assert!(error.path.starts_with("build.character.rewards["));
    assert!(error.path.ends_with(".parameters[0]"));
}

#[test]
fn changed_reward_sets_restore_scratch_and_reuse_each_rayon_worker() {
    let a = fixture();
    let mut b = fixture();
    b.build
        .character
        .rewards
        .retain(|r| !matches!(number(&r.definition), 0x0031 | 0x0041 | 0x0053));
    let (a_expected, b_expected) = (report(&a), report(&b));
    check_rewards(&a, &a_expected);
    check_rewards(&b, &b_expected);
    let (a_plan, b_plan) = (a.plan().unwrap(), b.plan().unwrap());
    let mut scratch = a_plan.new_scratch();
    for _ in 0..3 {
        assert_eq!(a_plan.evaluate(&mut scratch).unwrap(), a_expected);
        assert_eq!(b_plan.evaluate(&mut scratch).unwrap(), b_expected);
        assert_eq!(a_plan.evaluate(&mut scratch).unwrap(), a_expected);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        (0..32).into_par_iter().for_each_init(
            || a_plan.new_scratch(),
            |scratch, _| {
                assert_eq!(a_plan.evaluate(scratch).unwrap(), a_expected);
                assert_eq!(b_plan.evaluate(scratch).unwrap(), b_expected);
                assert_eq!(a_plan.evaluate(scratch).unwrap(), a_expected);
            },
        )
    });
}
