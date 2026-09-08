//! Cold/warm unchanged CalcDefence receiving branches after actual actor preparation.
use super::*;
use poe_optimizer_data::game_data::{
    self, ActorCondition as Condition, ActorModifierEffect as Effect,
    ActorModifierRecord as Record, ActorModifierTag as Tag, ActorNumericOperation as Op,
    ActorStat as Stat, GameDataLoader, LoadLimits, TrustPolicy,
};
use poe_optimizer_engine::{
    CompiledGameData,
    actor::{ActorModifierLayer, ActorScratch, ReceivingOutput, ReceivingScenario},
    character::{CharacterAttributes, CharacterInput},
};
use std::sync::Arc;
fn record(stat: Stat, operation: Op, value: f64) -> Record {
    Record {
        stat,
        effect: Effect::Numeric { operation, value },
        source: Some("Receiving source oracle".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn conditioned(mut value: Record, condition: Condition) -> Record {
    value.tags.push(Tag::Condition {
        variables: vec![condition],
        negated: false,
    });
    value
}
pub(super) fn compare(actual: ReceivingOutput, reference: &Table) {
    let output: Table = reference.get("output").unwrap();
    for (name, value) in [
        ("Armour", actual.armour),
        ("Evasion", actual.evasion),
        ("EnergyShield", actual.energy_shield),
        ("MaximumEnergyShield", actual.energy_shield),
        ("FireResist", actual.resistances.fire),
        ("ColdResist", actual.resistances.cold),
        ("LightningResist", actual.resistances.lightning),
        ("ChaosResist", actual.resistances.chaos),
        ("FireResistTotal", actual.resistance_totals.fire),
        ("ColdResistTotal", actual.resistance_totals.cold),
        ("LightningResistTotal", actual.resistance_totals.lightning),
        ("ChaosResistTotal", actual.resistance_totals.chaos),
    ] {
        let expected: f64 = output.get(name).unwrap();
        assert_number(Some(value), Some(expected));
        if value == 0.0 && expected == 0.0 {
            assert_eq!(value.to_bits(), expected.to_bits(), "{name} signed zero");
        }
    }
}
fn check(
    oracle: &actor_parity::ActorOracle,
    data: &CompiledGameData,
    character: &CharacterInput,
    scenario: ReceivingScenario,
    layers: &[Vec<Record>],
) {
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    let expected = oracle.calculate_receiving(data, 60, quests, scenario, character, layers);
    let prepared = data
        .prepare_actor(60, quests, scenario, character, layers)
        .unwrap();
    compare(prepared.receiving().unwrap(), &expected);
    let programs = layers
        .iter()
        .map(|layer| {
            layer
                .chunks(2)
                .map(|records| data.compile_actor_modifiers(records).unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let refs = programs
        .iter()
        .map(|layer| layer.iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let borrowed = refs
        .iter()
        .map(|programs| ActorModifierLayer { programs })
        .collect::<Vec<_>>();
    let compiled = data
        .evaluate_actor(
            60,
            quests,
            scenario,
            character,
            &borrowed,
            &mut ActorScratch::default(),
        )
        .unwrap();
    assert_eq!(prepared.receiving(), compiled.receiving());
    assert_eq!(prepared.values(), compiled.values());
}
#[test]
fn original_resource_and_resistance_receivers_match_boundaries_group_queries_and_signed_zero() {
    let data = CompiledGameData::bundled().unwrap();
    let character = spark::default_character();
    let scenario = ReceivingScenario {
        resistance_penalty: -20.5,
        resistance_quests: [true, false, true],
    };
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = actor_parity::ActorOracle::new_receiving(warm);
        for target in [
            Stat::Armour,
            Stat::Evasion,
            Stat::EnergyShield,
            Stat::ArmourAndEvasion,
        ] {
            for base in [
                -1000.5,
                -1.5,
                -0.5,
                -0.0,
                0.0,
                0.499999999999,
                0.5,
                1.5,
                333333.33333333,
            ] {
                for inc in [-250.0, -100.00001, -100.0, -99.5, 0.0, 33.333333, 100.0] {
                    let mut increased = record(target, Op::Increased, inc);
                    increased.tags.push(Tag::Global);
                    check(
                        &oracle,
                        &data,
                        &character,
                        scenario,
                        &[vec![
                            record(target, Op::Base, base),
                            increased,
                            record(Stat::Defences, Op::Increased, 0.25),
                        ]],
                    );
                    cases += 1;
                }
            }
        }
        for target in [
            Stat::FireResist,
            Stat::ColdResist,
            Stat::LightningResist,
            Stat::ChaosResist,
            Stat::ElementalResist,
        ] {
            for base in [
                -1000.5, -199.99, -60.5, -0.99, -0.0, 0.0, 0.99, 74.99, 75.99, 1000.5,
            ] {
                for inc in [-150.0, -100.0, -99.99, 0.0, 33.333333, 100.0] {
                    check(
                        &oracle,
                        &data,
                        &character,
                        scenario,
                        &[vec![
                            record(target, Op::Base, base),
                            record(target, Op::Increased, inc),
                            record(Stat::ElementalResist, Op::Base, 0.01),
                            record(Stat::ElementalResist, Op::Increased, 0.05),
                        ]],
                    );
                    cases += 1;
                }
            }
        }
    }
    eprintln!(
        "Receiving source boundaries: {cases} cold/warm source cases; fresh DB and compiled programs compared for each"
    );
}
#[test]
fn original_receivers_follow_final_attribute_conditions_source_order_layers_and_quests() {
    let data = CompiledGameData::bundled().unwrap();
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = actor_parity::ActorOracle::new_receiving(warm);
        for dexterity in [0.0, 20.0, 40.0] {
            let character = CharacterInput {
                attributes: CharacterAttributes {
                    strength: 20.0,
                    dexterity,
                    intelligence: 20.0,
                },
                ..Default::default()
            };
            for mask in [0, 1, 2, 4, 7] {
                let scenario = ReceivingScenario {
                    resistance_penalty: -0.25,
                    resistance_quests: std::array::from_fn(|i| mask & (1 << i) != 0),
                };
                let records = vec![
                    conditioned(
                        record(Stat::Int, Op::Base, 25.0),
                        Condition::DexHigherThanInt,
                    ),
                    conditioned(
                        record(Stat::Evasion, Op::Base, 10.5),
                        Condition::IntHigherThanDex,
                    ),
                    conditioned(
                        record(Stat::EnergyShield, Op::Base, 25.5),
                        Condition::IntHigherThanDex,
                    ),
                    conditioned(
                        record(Stat::EnergyShield, Op::Increased, 50.0),
                        Condition::DexHigherThanInt,
                    ),
                    conditioned(
                        record(Stat::FireResist, Op::Base, 80.5),
                        Condition::StrHighestAttribute,
                    ),
                    record(Stat::ArmourAndEvasion, Op::Increased, 25.0),
                    record(Stat::Defences, Op::Increased, 10.0),
                ];
                check(&oracle, &data, &character, scenario, &[records]);
                cases += 1;
            }
        }
        // Floating-point-sensitive ordered BASE additions: no pre-aggregation
        // into a scalar tail and no source-fragment-as-parent reinterpretation.
        for reversed in [false, true] {
            let mut local = vec![
                record(Stat::FireResist, Op::Base, 1e6),
                record(Stat::FireResist, Op::Base, 1e-11),
                record(Stat::FireResist, Op::Base, -1e6),
            ];
            if reversed {
                local.reverse();
            }
            for parent in [false, true] {
                let mut layers = vec![local.clone()];
                let extras = vec![
                    record(Stat::ElementalResist, Op::Base, 0.99999999999),
                    record(Stat::EnergyShield, Op::Base, 0.5),
                ];
                if parent {
                    layers.push(extras);
                } else {
                    layers[0].extend(extras);
                }
                check(
                    &oracle,
                    &data,
                    &spark::default_character(),
                    ReceivingScenario {
                        resistance_penalty: 0.0,
                        resistance_quests: [false; 3],
                    },
                    &layers,
                );
                cases += 1;
            }
        }
    }
    eprintln!("Receiving source composition: {cases} cold/warm cases");
}
#[test]
fn injected_receiver_evasion_and_fractional_caps_follow_original_source() {
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = actor_parity::ActorOracle::new_receiving(warm);
        for (evasion, floor, cap, maximum) in [
            (0.5, -0.5, 0.5, 90.0),
            (30.25, -199.5, 74.5, 90.0),
            (1.5, -200.0, 100.0, 90.5),
        ] {
            let mut package = game_data::bundled_snapshot().unwrap().package().clone();
            package.character.base_evasion = evasion;
            package.defence.resistance_floor = floor;
            package.defence.player_resistance_cap = cap;
            package.defence.resistance_maximum_cap = maximum;
            package.refresh_section_digests().unwrap();
            let snapshot = GameDataLoader::from_bytes(
                &package.canonical_bytes().unwrap(),
                &TrustPolicy::AllowCustom,
                &LoadLimits::default(),
            )
            .unwrap();
            let data = CompiledGameData::compile(Arc::new(snapshot)).unwrap();
            for value in [-199.99, -0.5, -0.0, 0.99, 74.5, 100.5] {
                check(
                    &oracle,
                    &data,
                    &spark::default_character(),
                    ReceivingScenario {
                        resistance_penalty: 0.0,
                        resistance_quests: [false; 3],
                    },
                    &[vec![
                        record(Stat::ElementalResist, Op::Base, value),
                        record(Stat::ChaosResist, Op::Base, value),
                        record(Stat::Defences, Op::Increased, 33.333333),
                    ]],
                );
                cases += 1;
            }
        }
    }
    eprintln!("Receiving injected rules: {cases} cold/warm cases");
}
