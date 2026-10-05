//! Authored reward roots share item resource channels, without a full resource formula.
#[allow(dead_code)]
#[path = "support/owned_reward_effects_native.rs"]
mod shared;

use poe_optimizer_core::{
    owned_build::*,
    owned_definitions::*,
    owned_rules::*,
    owned_schema::{SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use shared::{Fixture, report};
use std::collections::BTreeSet;

fn packet<T: DeserializeOwned>(name: &str) -> T {
    shared::packet("flat-resource-rewards", name)
}
#[derive(Deserialize)]
struct Reward {
    reward: RewardDefId,
    stat: StatDefId,
    unit: UnitDefId,
    amount: f64,
}
fn rewards() -> Vec<Reward> {
    let value: Value = packet("bindings.json");
    serde_json::from_value(value["rewards"].clone()).unwrap()
}
fn fixture() -> Fixture {
    shared::fixture(&["flat-resource-rewards"])
}
fn check_rewards(f: &Fixture, report: &OwnedEffectsReport) {
    let bindings = rewards();
    let effects: Vec<_> = report
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
            Some((id, effect))
        })
        .collect();
    assert_eq!(effects.len(), f.build.character.rewards.len());
    let mut seen = BTreeSet::new();
    for (id, effect) in effects {
        assert!(
            seen.insert(id),
            "one contribution per selected reward occurrence"
        );
        let selected = f
            .build
            .character
            .rewards
            .iter()
            .find(|r| r.id == id)
            .unwrap();
        let binding = bindings
            .iter()
            .find(|r| r.reward == selected.definition)
            .unwrap();
        assert_eq!(
            effect.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: binding.stat.clone(),
                    kind: ContributionKind::Add,
                }
            }
        );
        assert_eq!(
            effect.value,
            EffectValue::Known {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(binding.amount, binding.unit.clone()).unwrap(),
                )
            }
        );
    }
    // Life remains a contribution, not a maximum-Life scalar or final metric.
    let life = bindings
        .iter()
        .find(|r| r.stat != f.family.contribution)
        .unwrap();
    assert!(!report.values.iter().any(|value| matches!(&value.key,
        PlanValueKey::Stat { stat, .. } if *stat == life.stat
    )));
}

#[test]
fn exact_rewards_combine_once_with_equipment_in_distinct_resource_units() {
    let f = fixture();
    let result = report(&f);
    assert!(result.gaps.is_empty(), "{:?}", result.gaps);
    check_rewards(&f, &result);
    // Five active item-modifier uses round12.5 to13. Three distinct reward
    // sources add30/30/40. Candlemass's20 Life cannot enter this Spirit sum.
    assert_eq!(f.total(&result), &f.expected_total(165.0));
    for removed in rewards() {
        let mut changed = fixture();
        changed
            .build
            .character
            .rewards
            .retain(|r| r.definition != removed.reward);
        let result = report(&changed);
        check_rewards(&changed, &result);
        let difference = if removed.stat == changed.family.contribution {
            removed.amount
        } else {
            0.0
        };
        assert_eq!(
            changed.total(&result),
            &changed.expected_total(165.0 - difference)
        );
    }
}

#[test]
fn reward_and_item_selection_are_independent_sources() {
    let mut f = fixture();
    f.build.character.rewards.clear();
    let items = report(&f);
    check_rewards(&f, &items);
    assert_eq!(f.total(&items), &f.expected_total(65.0));

    let mut f = fixture();
    f.build.equipment.clear();
    let reward_only = report(&f);
    check_rewards(&f, &reward_only);
    assert_eq!(f.total(&reward_only), &f.expected_total(100.0));
    assert!(!reward_only.effects.iter().any(|effect| matches!(&effect.key.invocation.origin,
        RuleOrigin::Provider { provider } if matches!(provider.root, ProviderRoot::ItemModifier { .. })
    )));
}

#[test]
fn selected_unknown_or_partial_owners_and_foreign_parameters_remain_failures() {
    let mut f = fixture();
    let missing = rewards().remove(0).reward;
    f.recipe
        .rules
        .owners
        .retain(|o| o.owner != SchemaSubject::Definition(missing.address()));
    let unresolved = report(&f);
    assert!(!unresolved.gaps.is_empty());
    assert!(matches!(
        f.total(&unresolved),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
    f.build
        .character
        .rewards
        .retain(|r| r.definition != missing);
    let resolved = report(&f);
    assert!(resolved.gaps.is_empty());
    assert_eq!(f.total(&resolved), &f.expected_total(135.0));

    let mut f = fixture();
    f.restore_partial_rules();
    let unresolved = report(&f);
    check_rewards(&f, &unresolved);
    assert!(matches!(
        f.total(&unresolved),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));

    let mut f = fixture();
    f.build.character.rewards[0]
        .parameters
        .push(ParameterAssignment {
            slot: f.family.amount.clone(),
            value: ParameterValue::Quantity(
                FiniteQuantity::new(999.0, f.family.unit.clone()).unwrap(),
            ),
        });
    let error = BuildSpec::new(f.build, OwnedInputLimits::default()).unwrap_err();
    assert_eq!(error.kind, StructuralErrorKind::WrongDeclaration);
    assert_eq!(error.path, "build.character.rewards[0].parameters[0]");
}

#[test]
fn reward_plans_restore_scratch_and_share_safely_across_rayon_workers() {
    let a = fixture();
    let mut b = fixture();
    b.build.character.rewards.clear();
    let (a_plan, b_plan) = (a.plan().unwrap(), b.plan().unwrap());
    let (a_expected, b_expected) = (report(&a), report(&b));
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
        (0..16).into_par_iter().for_each(|_| {
            let mut scratch = a_plan.new_scratch();
            assert_eq!(a_plan.evaluate(&mut scratch).unwrap(), a_expected);
            assert_eq!(b_plan.evaluate(&mut scratch).unwrap(), b_expected);
            assert_eq!(a_plan.evaluate(&mut scratch).unwrap(), a_expected);
        })
    });
}
