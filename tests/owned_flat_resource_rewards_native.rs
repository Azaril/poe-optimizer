//! Authored reward roots share item resource channels, without a full resource formula.
#[allow(dead_code)]
#[path = "support/owned_ranged_spirit_native.rs"]
mod spirit;

use poe_optimizer_core::{
    build_identity::InstanceAllocator, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use spirit::component::Fixture;
use std::{collections::BTreeSet, fs, path::PathBuf};

fn packet<T: DeserializeOwned>(name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/flat-resource-rewards")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
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
    let mut f = spirit::fixture(None);
    spirit::clear_properties(&mut f);
    for (item, modifier) in [(0, 0), (0, 1), (1, 0)] {
        f.set_raw(item, modifier, 12.5);
    }
    let dependencies: Value = packet("dependencies.json");
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(dependencies["definitions"].clone()).unwrap();
    for definition in definitions {
        if let Some(existing) = f
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == definition.address())
        {
            assert_eq!(existing, &definition);
        } else {
            // Replace only the synthetic registry placeholder for this actual
            // authored address; no new allocation or real catalog mutation.
            let address = SchemaSubject::Definition(definition.address());
            let entry = f
                .recipe
                .registry
                .entries
                .iter_mut()
                .find(|entry| {
                    let key = match &entry.target {
                        SchemaSubject::Definition(id) => id.key(),
                        SchemaSubject::Slot(id) => id.key(),
                    };
                    key == definition.address().key()
                })
                .unwrap();
            entry.target = address;
            f.recipe.schema.definitions.push(definition);
        }
    }
    let closure: Value = packet("closure.json");
    assert_eq!(closure["definitions"], serde_json::json!([]));
    let owners: Vec<DefinitionRules> = serde_json::from_value(closure["owners"].clone()).unwrap();
    for owner in owners {
        assert!(owner.programs.is_complete());
        assert!(
            !f.recipe
                .rules
                .owners
                .iter()
                .any(|old| old.owner == owner.owner)
        );
        f.recipe.rules.owners.push(owner);
    }
    let mut allocator = InstanceAllocator::from_state(f.build.allocator);
    f.build.character.rewards = rewards()
        .into_iter()
        .map(|r| RewardSelection {
            id: allocator.allocate().unwrap(),
            definition: r.reward,
            parameters: vec![],
        })
        .collect();
    f.build.allocator = allocator.state();
    // Closes only this existing finite item fixture. The four real reward
    // owners above are already complete and their bodies remain untouched.
    f.complete_domain();
    f
}
fn report(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.plan().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
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
