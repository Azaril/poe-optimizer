//! Signed BASE resistance checks against unchanged pinned parser and defence
//! branches, not expected values recomputed by another handwritten formula.
use super::*;
use poe_optimizer_data::game_data::{self, GameDataLoader, LoadLimits, TrustPolicy};
use poe_optimizer_engine::{
    CompiledGameData,
    character::{CharacterInput, CharacterModifiers},
};
use std::sync::Arc;

pub(super) const SETUP_PREFIXES: &[&str] = &[
    "modDB:NewMod(\"FireResist\",",
    "modDB:NewMod(\"ColdResist\",",
    "modDB:NewMod(\"LightningResist\",",
    "modDB:NewMod(\"ChaosResist\",",
    "modDB:NewMod(\"FireResistMax\",",
    "modDB:NewMod(\"ColdResistMax\",",
    "modDB:NewMod(\"LightningResistMax\",",
    "modDB:NewMod(\"ChaosResistMax\",",
];

pub(super) fn install_defence_oracle(lua: &Lua) {
    let defence = SPARK_DEFENCE.replace("\r\n", "\n");
    let mut body = String::from(
        "return function(modDB,output) local m_min,m_max,m_modf=math.min,math.max,math.modf; local isElemental={Fire=true,Cold=true,Lightning=true}; local resistTypeList={'Fire','Cold','Lightning','Chaos'}; ",
    );
    body.push_str(section(
        &defence,
        "\tfor _, elem in ipairs(resistTypeList) do\n\t\tlocal min, max, total, dotTotal",
        "\n\t\toutput[elem..\"ResistOverCap\"]",
    ));
    body.push_str("\nend; end");
    lua.globals()
        .set(
            "playerResistances",
            lua.load(body)
                .set_name("pinned-player-resistance-branch")
                .eval::<Function>()
                .unwrap(),
        )
        .unwrap();
}

fn parser(lua: &Lua) -> Function {
    lua.load("data.gems={}; data.keystones={}; data.skills={};")
        .exec()
        .unwrap();
    lua.load(section(
        &DATA.replace("\r\n", "\n"),
        "data.ailmentTypeList =",
        "data.buildupTypes =",
    ))
    .exec()
    .unwrap();
    lua.load(character_parity::PARSER)
        .set_name("pinned-signed-resistance-ModParser")
        .eval()
        .unwrap()
}

#[test]
fn resistance_operations_match_original_parser_for_signed_fractional_values_and_owned_nodes() {
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm);
        let lua = &oracle.oracle.lua;
        let parser = parser(lua);
        for (label, stat) in [
            ("Fire Resistance", "FireResist"),
            ("Cold Resistance", "ColdResist"),
            ("Lightning Resistance", "LightningResist"),
            ("Chaos Resistance", "ChaosResist"),
            ("to all Elemental Resistances", "ElementalResist"),
        ] {
            for number in [
                -1_000_000.0,
                -20.5,
                -0.5,
                0.0,
                0.5,
                3.0,
                7.0,
                8.0,
                1_000_000.0,
            ] {
                let text = format!("{number:+}% {label}");
                for _ in 0..if warm { 200 } else { 1 } {
                    let (mods, remainder): (Table, Option<String>) =
                        parser.call(text.as_str()).unwrap();
                    assert!(remainder.is_none(), "{text}: {remainder:?}");
                    assert_eq!(mods.raw_len(), 1, "{text}");
                    let modifier: Table = mods.get(1).unwrap();
                    assert_eq!(modifier.get::<String>("name").unwrap(), stat);
                    assert_eq!(modifier.get::<String>("type").unwrap(), "BASE");
                    assert_eq!(modifier.get::<f64>("value").unwrap(), number);
                    assert_eq!(modifier.get::<u32>("flags").unwrap(), 0);
                    assert_eq!(modifier.get::<u32>("keywordFlags").unwrap(), 0);
                    assert_eq!(modifier.raw_len(), 0, "No actor or condition tags allowed");
                }
            }
        }
        let tree: Table = lua.load(SPARK_TREE).eval().unwrap();
        let nodes: Table = tree.get("nodes").unwrap();
        let compiled = CompiledGameData::bundled().unwrap();
        for (class, owner, id) in [
            (6, "Warrior3", 14960),
            (11, "Druid2", 61722),
            (10, "Monk3", 24475),
            (8, "Huntress3", 17058),
        ] {
            let node: Table = nodes.get(id).unwrap();
            let stats: Table = node.get("stats").unwrap();
            assert_eq!(stats.raw_len(), 1);
            let (mods, remainder): (Table, Option<String>) =
                parser.call(stats.get::<String>(1).unwrap()).unwrap();
            assert!(remainder.is_none());
            let modifier: Table = mods.get(1).unwrap();
            // Schema8 retains source receiver records exclusively in the actor
            // program stream. The compatibility scalar projection is offence-only.
            let source = compiled
                .snapshot()
                .passive_effects(class, Some(owner), id)
                .unwrap();
            assert!(source.effects.is_empty());
            assert_eq!(source.actor_modifiers.len(), 1);
            let actual = &source.actor_modifiers[0];
            assert_eq!(
                actual.stat.upstream_name(),
                modifier.get::<String>("name").unwrap()
            );
            let game_data::ActorModifierEffect::Numeric { operation, value } = actual.effect else {
                panic!("numeric source receiver")
            };
            assert_eq!(
                operation.upstream_name(),
                modifier.get::<String>("type").unwrap()
            );
            assert_eq!(value, modifier.get::<f64>("value").unwrap());
            assert_eq!(actual.flags as f64, modifier.get::<f64>("flags").unwrap());
            assert_eq!(
                actual.keyword_flags as f64,
                modifier.get::<f64>("keywordFlags").unwrap()
            );
            assert!(actual.tags.is_empty());
            assert_eq!(modifier.raw_len(), 0);
            assert_eq!(
                *compiled.passive_modifiers(class, Some(owner), id).unwrap(),
                CharacterModifiers::NONE
            );
            assert!(compiled.passive_modifiers(class, None, id).is_none());
            assert!(
                compiled
                    .passive_modifiers(class, Some("WrongOwner"), id)
                    .is_none()
            );
        }
    }
}

fn compare_spark(output: spark::SparkOutput, expected: Table) {
    for (name, actual) in [
        ("FireResist", output.fire_resistance),
        ("ColdResist", output.cold_resistance),
        ("LightningResist", output.lightning_resistance),
        ("ChaosResist", output.chaos_resistance),
        ("Life", output.life),
        ("Mana", output.mana),
        ("Armour", output.armour),
        ("Evasion", output.evasion),
        ("EnergyShield", output.energy_shield),
        ("TotalDPS", output.hit_dps),
        ("AverageHit", output.average_hit),
        ("Speed", output.cast_rate),
    ] {
        let value: f64 = expected.get(name).unwrap();
        assert!(
            actual != 0.0 || value != 0.0 || actual.to_bits() == value.to_bits(),
            "{name}: {actual:?} != {value:?}"
        );
        assert_number(Some(actual), Some(value));
    }
}

#[test]
fn signed_resistances_match_original_defence_in_both_profiles_interpreted_and_warmed() {
    for warm in [false, true] {
        let spark_oracle = SparkOracle::new(warm);
        let mace_oracle = mace_parity::MaceOracle::new(warm);
        for number in [
            -1_000_000.0,
            -200.99,
            -200.0,
            -199.99,
            -60.5,
            -20.0,
            -0.99,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.99,
            74.99,
            75.0,
            75.99,
            90.5,
            1_000_000.0,
        ] {
            for field in 0..6 {
                let mut modifiers = CharacterModifiers::NONE;
                match field {
                    0 => modifiers.fire_resistance_flat = number,
                    1 => modifiers.cold_resistance_flat = number,
                    2 => modifiers.lightning_resistance_flat = number,
                    3 => modifiers.chaos_resistance_flat = number,
                    4 => modifiers.elemental_resistance_flat = number,
                    _ => {
                        modifiers.fire_resistance_flat = -number;
                        modifiers.cold_resistance_flat = number;
                        modifiers.lightning_resistance_flat = 0.5;
                        modifiers.chaos_resistance_flat = number;
                        modifiers.elemental_resistance_flat = number;
                    }
                }
                for penalty in [-60.0, -20.5, -0.25, 0.0] {
                    for mask in [0u8, 8, 16, 32, 63] {
                        let quests = SparkQuestRewards::from_enabled(std::array::from_fn(|bit| {
                            mask & (1 << bit) != 0
                        }));
                        let character = CharacterInput {
                            modifiers,
                            ..spark::default_character()
                        };
                        let input = SparkInput {
                            character_level: 60,
                            resistance_penalty: penalty,
                            enemy_lightning_resistance: 50.0,
                            quests,
                        };
                        compare_spark(
                            spark::evaluate_with_character(&input, &character).unwrap(),
                            spark_oracle.calculate_with_character(&input, &character),
                        );
                        let character = CharacterInput {
                            modifiers,
                            ..poe_optimizer_engine::mace::default_character()
                        };
                        let input = poe_optimizer_engine::mace::MaceInput {
                            resistance_penalty: penalty,
                            quests,
                            brutality: mask % 2 == 1,
                            ..mace_parity::input()
                        };
                        mace_parity::compare(
                            poe_optimizer_engine::mace::evaluate_with_character(&input, &character)
                                .unwrap(),
                            mace_oracle.calculate_with_character(&input, &character),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn injected_fractional_resistance_limits_follow_original_truncation_before_cap_and_floor() {
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm);
        for (floor, cap, maximum) in [
            (-199.5, 74.5, 90.0),
            (-0.5, 0.5, 90.0),
            (-25.99, 80.99, 90.0),
            (-200.0, 100.0, 90.5),
            (-200.0, 75.5, 60.5),
            (-200.0, 75.5, 75.99),
            (-0.5, 0.5, 0.5),
            (-200.0, 90.0, 100.0),
        ] {
            let mut package = game_data::bundled_snapshot().unwrap().package().clone();
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
            let lua_data: Table = oracle.oracle.lua.globals().get("data").unwrap();
            let misc: Table = lua_data.get("misc").unwrap();
            misc.set("ResistFloor", floor).unwrap();
            misc.set("MaxResistCap", maximum).unwrap();
            let constants: Table = lua_data.get("characterConstants").unwrap();
            constants
                .set("base_maximum_all_resistances_%", cap)
                .unwrap();
            for value in [-1_000_000.0, -200.5, -0.9, 0.9, 74.9, 75.5, 1_000_000.0] {
                let input = SparkInput {
                    character_level: 60,
                    resistance_penalty: -20.5,
                    enemy_lightning_resistance: 0.0,
                    quests: SparkQuestRewards::default(),
                };
                let character = CharacterInput {
                    modifiers: CharacterModifiers {
                        elemental_resistance_flat: value,
                        chaos_resistance_flat: value,
                        ..Default::default()
                    },
                    ..spark::default_character()
                };
                compare_spark(
                    spark::evaluate_with_data(&input, &character, &data).unwrap(),
                    oracle.calculate_with_character(&input, &character),
                );
            }
        }
    }
}

#[test]
fn signed_modifiers_compose_with_ordinary_values_without_hiding_invalid_inputs() {
    let ordinary = CharacterModifiers {
        armour_flat: 20.0,
        skill_speed_increased: 4.0,
        ..Default::default()
    };
    let resistance = CharacterModifiers {
        elemental_resistance_flat: -20.5,
        chaos_resistance_flat: 7.5,
        ..Default::default()
    };
    let combined = ordinary.checked_add(resistance).unwrap();
    assert_eq!(combined.armour_flat, ordinary.armour_flat);
    assert_eq!(
        combined.elemental_resistance_flat,
        resistance.elemental_resistance_flat
    );
    assert_eq!(
        combined.checked_add(CharacterModifiers::NONE).unwrap(),
        combined
    );
    assert_eq!(
        ordinary.checked_add(resistance),
        resistance.checked_add(ordinary)
    );
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1_000_000.1,
        1_000_000.1,
    ] {
        for field in 0..5 {
            let mut invalid = CharacterModifiers::NONE;
            match field {
                0 => invalid.fire_resistance_flat = value,
                1 => invalid.cold_resistance_flat = value,
                2 => invalid.lightning_resistance_flat = value,
                3 => invalid.chaos_resistance_flat = value,
                _ => invalid.elemental_resistance_flat = value,
            }
            let spark_input = SparkInput {
                character_level: 60,
                resistance_penalty: -60.0,
                enemy_lightning_resistance: 0.0,
                quests: SparkQuestRewards::default(),
            };
            let character = CharacterInput {
                modifiers: invalid,
                ..spark::default_character()
            };
            assert!(spark::evaluate_with_character(&spark_input, &character).is_err());
            assert!(
                poe_optimizer_engine::mace::evaluate_with_character(
                    &mace_parity::input(),
                    &character
                )
                .is_err()
            );
            assert!(invalid.validate().is_err());
            assert!(invalid.checked_add(CharacterModifiers::NONE).is_err());
        }
    }
    for value in [-1_000_000.0, 1_000_000.0] {
        let bounded = CharacterModifiers {
            elemental_resistance_flat: value,
            ..Default::default()
        };
        assert!(bounded.validate().is_ok());
        assert!(bounded.checked_add(bounded).is_err());
    }
    let invalid = CharacterModifiers {
        elemental_resistance_flat: 1_000_001.0,
        ..Default::default()
    };
    assert!(
        invalid
            .checked_add(CharacterModifiers {
                elemental_resistance_flat: -1.0,
                ..Default::default()
            })
            .is_err()
    );
    assert!(
        CharacterModifiers {
            armour_flat: -0.5,
            ..Default::default()
        }
        .checked_add(ordinary)
        .is_err()
    );
}
