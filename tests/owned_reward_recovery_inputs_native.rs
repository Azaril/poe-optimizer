//! Finite Reward producer checks; no final ailment, recovery or Charm formulas.
//! Actual packet rules run alongside the existing item/reward component.
#[allow(dead_code)]
#[path = "support/owned_reward_effects_native.rs"]
mod shared;

use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::SchemaPackageInput;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use shared::{Fixture, report};

const REWARDS: [u16; 3] = [0x002d, 0x0036, 0x003b];
const CHANNELS: [u16; 4] = [0x32f0, 0x32f1, 0x32f2, 0x32f3];

fn number<K: DefinitionDomain>(id: &DefId<K>) -> u16 {
    u16::from_str_radix(id.key().as_str().strip_prefix("def.").unwrap(), 16).unwrap()
}
fn id<K: DefinitionDomain>(f: &Fixture, number: u16) -> DefId<K> {
    DefId::parse(f.build.game_version.clone(), format!("def.{number:016x}")).unwrap()
}
fn fixture() -> Fixture {
    shared::fixture(&[
        "flat-resource-rewards",
        "permanent-reward-effects",
        "reward-recovery-inputs-v1",
    ])
}
fn expected(reward: u16) -> Vec<(u16, ContributionKind, f64, u16)> {
    use ContributionKind::{Add, Increase};
    match reward {
        0x002d => vec![(0x32f2, Increase, 30.0, 0x0002), (0x32f3, Add, 1.0, 0x295a)],
        0x0036 => vec![(0x32f0, Increase, 30.0, 0x0002)],
        0x003b => vec![(0x32f1, Increase, 30.0, 0x0002)],
        _ => panic!("unlisted finite Reward definition {reward:04x}"),
    }
}
fn is_new(effect: &BoundEffectResult) -> bool {
    matches!(&effect.key.invocation.owner,
        SchemaSubject::Definition(DefinitionAddress::Reward(reward))
            if REWARDS.contains(&number(reward)))
}
fn check_new_rewards(f: &Fixture, result: &OwnedEffectsReport) {
    let selected: Vec<_> = f
        .build
        .character
        .rewards
        .iter()
        .filter(|reward| REWARDS.contains(&number(&reward.definition)))
        .collect();
    let effects: Vec<_> = result
        .effects
        .iter()
        .filter(|effect| is_new(effect))
        .collect();
    assert_eq!(
        effects.len(),
        selected
            .iter()
            .map(|reward| expected(number(&reward.definition)).len())
            .sum::<usize>()
    );
    for reward in selected {
        let origin = RuleOrigin::Provider {
            provider: ProviderKey {
                root: ProviderRoot::Reward(reward.id),
                grant_path: vec![],
            },
        };
        let actual: Vec<_> = effects
            .iter()
            .copied()
            .filter(|effect| effect.key.invocation.origin == origin)
            .collect();
        let wanted = expected(number(&reward.definition));
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
            assert_eq!(matching.len(), 1);
            let effect = matching[0];
            assert_eq!(
                effect.key.invocation.owner,
                SchemaSubject::Definition(reward.definition.address())
            );
            assert_eq!(
                effect.key.invocation.entity,
                ConcreteEntity::Actor(ActorKey::Player)
            );
            assert_eq!(
                effect.key.invocation.program.as_str(),
                "reward-recovery-contributions"
            );
            assert_eq!(effect.key.effect.as_str(), format!("grant-{index}"));
            assert_eq!(
                effect.value,
                EffectValue::Known {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(amount, id(f, unit)).unwrap()
                    ),
                }
            );
        }
    }
    for channel in CHANNELS {
        let channel: StatDefId = id(f, channel);
        assert!(
            !result.values.iter().any(|value| matches!(&value.key,
            PlanValueKey::Stat { stat, .. } if stat == &channel)),
            "no final scalar is published for {}",
            channel.key()
        );
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
fn four_exact_player_contributions_preserve_every_prior_item_and_reward_effect() {
    let prior = shared::fixture(&["flat-resource-rewards", "permanent-reward-effects"]);
    let old = report(&prior);
    let f = fixture();
    let result = report(&f);
    assert!(old.gaps.is_empty());
    assert!(result.gaps.is_empty(), "{:?}", result.gaps);
    assert_eq!(f.build.character.rewards.len(), 15);
    check_new_rewards(&f, &result);
    assert_eq!(
        result
            .effects
            .iter()
            .filter(|effect| !is_new(effect))
            .collect::<Vec<_>>(),
        old.effects.iter().collect::<Vec<_>>()
    );
    // The existing Spirit sentinel is unaffected by these unrelated channels.
    assert_eq!(f.total(&result), &f.expected_total(165.0));
    assert_eq!(result.application_groups, old.application_groups);
}

#[test]
fn published_reward_programs_do_not_replace_missing_or_unknown_support_certificates() {
    for unmapped in [false, true] {
        let mut f = fixture();
        let owner = SchemaSubject::Definition(id::<RewardDefinition>(&f, REWARDS[0]).address());
        let providers = &mut f.recipe.rules.support_discovery.as_mut().unwrap().providers;
        let index = providers.iter().position(|d| d.owner == owner).unwrap();
        if unmapped {
            providers[index].domain = SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: owner,
                    facet: SchemaFacet::GameRules,
                    code: "unknown-reward-support-sources".parse().unwrap(),
                }],
            };
        } else {
            providers.remove(index);
        }
        let result = report(&f);
        let expected = if unmapped {
            PlanGapReason::UnmappedSupportSources
        } else {
            PlanGapReason::MissingSupportSources
        };
        assert!(result.gaps.iter().any(|g| g.reason == expected));
        assert_incomplete(&f, &result);
    }
}

#[test]
fn each_reward_removal_preserves_every_other_effect_and_removes_both_charm_records() {
    let f = fixture();
    let original = report(&f);
    for omitted in REWARDS {
        let mut f = fixture();
        let removed = f
            .build
            .character
            .rewards
            .iter()
            .find(|reward| number(&reward.definition) == omitted)
            .unwrap()
            .id;
        f.build
            .character
            .rewards
            .retain(|reward| reward.id != removed);
        let result = report(&f);
        assert!(result.gaps.is_empty(), "{:?}", result.gaps);
        check_new_rewards(&f, &result);
        let expected: Vec<_> = original
            .effects
            .iter()
            .filter(|effect| {
                !matches!(
            &effect.key.invocation.origin, RuleOrigin::Provider { provider }
                if provider.root == ProviderRoot::Reward(removed))
            })
            .collect();
        assert_eq!(result.effects.iter().collect::<Vec<_>>(), expected);
        assert_eq!(f.total(&result), &f.expected_total(165.0));
    }
}

#[test]
fn alternate_charm_duration_reward_does_not_inherit_charges_or_capacity_programs() {
    let mut f = fixture();
    // This different source option has an existing exact Reward descriptor but
    // no numerical owner in this packet. Import mapping assigns it Reward002f;
    // Option002e is a separate identity. Use the tracked descriptor unchanged.
    let schema: SchemaPackageInput = shared::packet("import/compiled", "schema.json");
    let alternate: RewardDefId = id(&f, 0x002f);
    let descriptor = schema
        .definitions
        .into_iter()
        .find(|definition| definition.address() == alternate.address())
        .unwrap();
    assert!(
        !f.recipe
            .schema
            .definitions
            .iter()
            .any(|definition| definition.address() == alternate.address())
    );
    let entry = f
        .recipe
        .registry
        .entries
        .iter_mut()
        .find(|entry| entry.sequence.get() == 0x002f)
        .unwrap();
    assert!(matches!(
        &entry.target,
        SchemaSubject::Definition(DefinitionAddress::Option(_))
    ));
    entry.target = SchemaSubject::Definition(alternate.address());
    f.recipe.schema.definitions.push(descriptor);
    let selected = f
        .build
        .character
        .rewards
        .iter_mut()
        .find(|reward| number(&reward.definition) == 0x002d)
        .unwrap();
    let occurrence = selected.id;
    selected.definition = alternate.clone();
    f.complete_domain();
    let result = report(&f);
    check_new_rewards(&f, &result);
    assert!(result.gaps.iter().any(|gap| gap.subject
        == Some(SchemaSubject::Definition(alternate.address()))
        && gap.provider
            == Some(ProviderKey {
                root: ProviderRoot::Reward(occurrence),
                grant_path: vec![]
            })
        && gap.reason == PlanGapReason::MissingPrograms));
    assert!(!result.effects.iter().any(|effect| matches!(&effect.target,
        BoundEffectTarget::Contribution { key } if matches!(number(&key.stat), 0x32f2 | 0x32f3))));
    assert_incomplete(&f, &result);
    f.build
        .character
        .rewards
        .retain(|reward| reward.id != occurrence);
    let removed = report(&f);
    assert!(removed.gaps.is_empty(), "{:?}", removed.gaps);
    check_new_rewards(&f, &removed);
    assert_eq!(f.total(&removed), &f.expected_total(165.0));
}

#[test]
fn every_missing_or_partial_reward_owner_keeps_the_coverage_gate() {
    for reward_number in REWARDS {
        for partial in [false, true] {
            let mut f = fixture();
            let reward: RewardDefId = id(&f, reward_number);
            let subject = SchemaSubject::Definition(reward.address());
            let occurrence = f
                .build
                .character
                .rewards
                .iter()
                .find(|selected| selected.definition == reward)
                .unwrap()
                .id;
            if partial {
                f.recipe
                    .rules
                    .owners
                    .iter_mut()
                    .find(|owner| owner.owner == subject)
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
                        && gap.provider
                            == Some(ProviderKey {
                                root: ProviderRoot::Reward(occurrence),
                                grant_path: vec![]
                            })
                        && gap.reason == reason)
            );
            assert_incomplete(&f, &result);
            if partial {
                check_new_rewards(&f, &result);
            }
            f.build
                .character
                .rewards
                .retain(|selected| selected.id != occurrence);
            let restored = report(&f);
            assert!(restored.gaps.is_empty(), "{:?}", restored.gaps);
            check_new_rewards(&f, &restored);
            assert_eq!(f.total(&restored), &f.expected_total(165.0));
        }
    }
    let mut f = fixture();
    f.restore_partial_rules();
    let result = report(&f);
    check_new_rewards(&f, &result);
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
fn changed_reward_sets_restore_scratch_and_are_deterministic_on_each_rayon_worker() {
    let a = fixture();
    let mut b = fixture();
    b.build
        .character
        .rewards
        .retain(|reward| !matches!(number(&reward.definition), 0x002d | 0x003b));
    let (a_expected, b_expected) = (report(&a), report(&b));
    check_new_rewards(&a, &a_expected);
    check_new_rewards(&b, &b_expected);
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
