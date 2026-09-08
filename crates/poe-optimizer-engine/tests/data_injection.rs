//! Injection changes values without rebuilding code; expected outputs remain
//! independent goldens or metamorphic relations, never generated fixture files.
#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_data::game_data::{
    self, GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy,
};
use poe_optimizer_engine::{
    CompiledGameData, defence,
    mace::{self, MaceInput, MaceWeapon},
    spark::{self, SparkInput, SparkQuestRewards},
};
use std::sync::Arc;

fn custom(edit: impl FnOnce(&mut GameDataPackage)) -> CompiledGameData {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    edit(&mut package);
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let snapshot =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    CompiledGameData::compile(Arc::new(snapshot)).unwrap()
}
fn spark_input() -> SparkInput {
    SparkInput {
        character_level: 60,
        resistance_penalty: -60.0,
        enemy_lightning_resistance: 0.0,
        quests: SparkQuestRewards::default(),
    }
}
fn mace_input() -> MaceInput {
    MaceInput {
        character_level: 60,
        weapon: MaceWeapon::SmithingHammer,
        quality: 0,
        item_level: 60,
        brutality: false,
        resistance_penalty: -60.0,
        quests: SparkQuestRewards::default(),
        enemy_armour: 1500.0,
        enemy_evasion: 591.0,
        enemy_fire_resistance: 0.0,
    }
}

#[test]
fn separately_loaded_reviewed_data_has_equal_identity_and_outputs() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CompiledGameData>();
    let bundled = CompiledGameData::bundled().unwrap();
    let external = CompiledGameData::compile(Arc::new(
        GameDataLoader::from_bytes(
            game_data::bundled_package_bytes(),
            &TrustPolicy::Reviewed {
                expected_sha256: game_data::bundled_package_sha256().into(),
            },
            &LoadLimits::default(),
        )
        .unwrap(),
    ))
    .unwrap();
    assert_eq!(bundled.identity(), external.identity());
    assert_eq!(
        spark::evaluate(&spark_input()).unwrap(),
        spark::evaluate_with_data(
            &spark_input(),
            &external.default_spark_character(),
            &external
        )
        .unwrap()
    );
    assert_eq!(
        mace::evaluate(&mace_input()).unwrap(),
        mace::evaluate_with_data(&mace_input(), &external.default_mace_character(), &external)
            .unwrap()
    );
}

#[test]
fn balance_records_change_only_the_selected_dataset() {
    let a = CompiledGameData::bundled().unwrap();
    let b = custom(|package| {
        package.spark.lightning_minimum *= 2.0;
        package.spark.lightning_maximum *= 2.0;
        package.character.life_per_level += 1.0;
        package.character.base_evasion += 11.0;
    });
    assert_ne!(a.identity(), b.identity());
    let input = spark_input();
    let before = spark::evaluate_with_data(&input, &a.default_spark_character(), &a).unwrap();
    let changed = spark::evaluate_with_data(&input, &b.default_spark_character(), &b).unwrap();
    assert_eq!(changed.average_hit, 2.0 * before.average_hit);
    assert_eq!(changed.hit_dps, 2.0 * before.hit_dps);
    assert_eq!(changed.life, before.life + 63.0);
    assert_eq!(changed.mana, before.mana);
    assert_eq!(changed.evasion, before.evasion + 11.0);
    assert_eq!(
        before,
        spark::evaluate_with_data(&input, &a.default_spark_character(), &a).unwrap()
    );
}

#[test]
fn weapon_support_monster_and_typed_effect_records_reach_calculation() {
    let a = CompiledGameData::bundled().unwrap();
    let b = custom(|package| {
        let weapon = package
            .weapons
            .iter_mut()
            .find(|w| w.id == "smithing_hammer")
            .unwrap();
        weapon.attack_rate *= 2.0;
        package
            .supports
            .iter_mut()
            .find(|support| support.id == "brutality_i")
            .unwrap()
            .modifiers[0]
            .value += 100.0;
        package.monsters.evasion[59] *= 2.0;
        package.monsters.armour[59] *= 2.0;
        let effects = package
            .passive_effects
            .iter_mut()
            .find(|e| e.class_id == 1 && e.physical_node_id == 4739)
            .unwrap();
        for effect in &mut effects.effects {
            effect.value *= 2.0;
        }
    });
    let input = mace_input();
    let ordinary_a = mace::evaluate_with_data(&input, &a.default_mace_character(), &a).unwrap();
    let ordinary_b = mace::evaluate_with_data(&input, &b.default_mace_character(), &b).unwrap();
    assert_eq!(ordinary_b.hit_dps, 2.0 * ordinary_a.hit_dps);
    assert_eq!(
        b.monster_evasion(60).unwrap(),
        2.0 * a.monster_evasion(60).unwrap()
    );
    assert_eq!(
        b.monster_armour(60).unwrap(),
        2.0 * a.monster_armour(60).unwrap()
    );
    let supported = MaceInput {
        brutality: true,
        ..input
    };
    let supported_a =
        mace::evaluate_with_data(&supported, &a.default_mace_character(), &a).unwrap();
    let supported_b =
        mace::evaluate_with_data(&supported, &b.default_mace_character(), &b).unwrap();
    assert!(supported_b.hit_dps > 2.0 * supported_a.hit_dps);
    assert_eq!(
        b.entrance_modifiers(1, 4739)
            .unwrap()
            .spell_damage_increased,
        2.0 * a
            .entrance_modifiers(1, 4739)
            .unwrap()
            .spell_damage_increased
    );
    assert!(b.monster_armour(0).is_err());
    assert!(b.monster_evasion(101).is_err());
}

#[test]
fn defence_coefficients_are_explicit_inputs() {
    let a = CompiledGameData::bundled().unwrap();
    let b = custom(|package| {
        package.defence.hit_chance_floor = 10.0;
        package.defence.hit_chance_cap = 80.0;
        package.defence.armour_ratio *= 2.0;
        package.defence.deflection_rating_floor = 1000.0;
    });
    assert_eq!(
        defence::hit_chance_with_data(0.0, -1.0, false, b.defence()),
        10.0
    );
    assert_eq!(
        defence::hit_chance_with_data(0.0, 100.0, false, b.defence()),
        80.0
    );
    assert_eq!(
        defence::monster_hit_chance_with_data(0.0, 100.0, b.defence()),
        80.0
    );
    assert_eq!(
        defence::deflect_chance_with_data(100.0, 100.0, b.defence_constants(), b.defence()),
        0.0
    );
    assert!(
        defence::armour_reduction_percent(100.0, 10.0, b.defence_constants())
            < defence::armour_reduction_percent(100.0, 10.0, a.defence_constants())
    );
}

#[test]
fn structurally_valid_unknown_weapon_capabilities_fail_compilation() {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    package.weapons[0].id = "unimplemented_slot".into();
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let error = CompiledGameData::compile(Arc::new(snapshot)).unwrap_err();
    assert!(error.to_string().contains("weapon capability slot"));
}

#[test]
fn prepared_supports_are_bound_to_their_dataset_and_ignore_the_legacy_selector() {
    use poe_optimizer_engine::mace_supports::PreparedMaceSupports;
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PreparedMaceSupports>();
    let a = CompiledGameData::bundled().unwrap();
    let b = custom(|_| {});
    let keys = vec!["heavy_swing".into(), "rapid_attacks_i".into()];
    let prepared = a.mace_support_loadout(&keys).unwrap().clone();
    assert_eq!(prepared.keys(), keys);
    let input = mace_input();
    let actual =
        mace::evaluate_with_supports(&input, &a.default_mace_character(), &a, &prepared).unwrap();
    assert_eq!(
        actual,
        mace::evaluate_with_supports(
            &MaceInput {
                brutality: true,
                ..input
            },
            &a.default_mace_character(),
            &a,
            &prepared
        )
        .unwrap()
    );
    assert!(
        mace::evaluate_with_supports(&input, &b.default_mace_character(), &b, &prepared)
            .unwrap_err()
            .to_string()
            .contains("different compiled dataset")
    );
    for invalid in [
        vec!["unknown".into()],
        vec!["heavy_swing".into(), "heavy_swing".into()],
        vec!["rapid_attacks_i".into(), "heavy_swing".into()],
        vec![
            "brutality_i".into(),
            "heavy_swing".into(),
            "rapid_attacks_i".into(),
        ],
    ] {
        assert!(a.mace_support_loadout(&invalid).is_err());
    }
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let a = &a;
            let prepared = &prepared;
            scope.spawn(move || {
                for _ in 0..100 {
                    assert_eq!(
                        actual,
                        mace::evaluate_with_supports(
                            &input,
                            &a.default_mace_character(),
                            a,
                            prepared
                        )
                        .unwrap()
                    );
                }
            });
        }
    });
}

#[test]
fn injected_support_numeric_operations_and_damage_disables_reach_prepared_calculation() {
    use poe_optimizer_data::game_data::{SupportDamageType, SupportStat};
    let a = CompiledGameData::bundled().unwrap();
    let b = custom(|package| {
        let heavy = package
            .supports
            .iter_mut()
            .find(|support| support.id == "heavy_swing")
            .unwrap();
        heavy
            .modifiers
            .iter_mut()
            .find(|modifier| modifier.stat == SupportStat::Speed)
            .unwrap()
            .value = -50.0;
        heavy.disable_damage.push(SupportDamageType::Physical);
        package
            .supports
            .iter_mut()
            .find(|support| support.id == "brutality_i")
            .unwrap()
            .disable_damage
            .clear();
    });
    let input = mace_input();
    let calculate = |data: &CompiledGameData, keys: &[String]| {
        mace::evaluate_with_supports(
            &input,
            &data.default_mace_character(),
            data,
            data.mace_support_loadout(keys).unwrap(),
        )
        .unwrap()
    };
    let heavy = vec!["heavy_swing".into()];
    let before = calculate(&a, &heavy);
    let changed = calculate(&b, &heavy);
    assert!(before.physical_hit_average > 0.0);
    assert_eq!(changed.physical_hit_average, 0.0);
    assert!(changed.attack_rate < before.attack_rate);
    assert_eq!(changed.fire_hit_average, before.fire_hit_average);
    let brutality = vec!["brutality_i".into()];
    assert_eq!(calculate(&a, &brutality).fire_hit_average, 0.0);
    assert!(calculate(&b, &brutality).fire_hit_average > 0.0);
    assert_eq!(calculate(&a, &heavy), before);
}

#[test]
fn combined_negative_support_increases_reject_before_evaluation() {
    use poe_optimizer_data::game_data::{SupportOperation, SupportScope, SupportStat};
    for stat in [SupportStat::PhysicalDamage, SupportStat::Speed] {
        let mut package = game_data::bundled_snapshot().unwrap().package().clone();
        for id in ["heavy_swing", "rapid_attacks_i"] {
            let support = package
                .supports
                .iter_mut()
                .find(|support| support.id == id)
                .unwrap();
            support.modifiers.clear();
            support
                .modifiers
                .push(poe_optimizer_data::game_data::SupportModifier {
                    stat,
                    operation: SupportOperation::Increased,
                    scope: SupportScope::Attack,
                    value: -60.0,
                });
        }
        package.refresh_section_digests().unwrap();
        let snapshot = GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap();
        assert!(
            CompiledGameData::compile(Arc::new(snapshot))
                .unwrap_err()
                .to_string()
                .contains("negative physical damage or speed")
        );
    }
}

fn local_roll(
    data: &CompiledGameData,
    stat: game_data::LocalWeaponStat,
    values: Vec<f64>,
) -> game_data::ItemModifierRoll {
    game_data::ItemModifierRoll {
        rule_id: data
            .snapshot()
            .package()
            .item_modifier_rules
            .iter()
            .find(|rule| rule.modifiers[0].stat == stat)
            .unwrap()
            .id
            .clone(),
        values,
    }
}
#[test]
fn prepared_weapons_bind_exact_compiled_data_and_selected_input_fields() {
    use game_data::LocalWeaponStat as Stat;
    let first = CompiledGameData::bundled().unwrap();
    let second =
        CompiledGameData::compile(Arc::new(game_data::bundled_snapshot().unwrap())).unwrap();
    let input = mace_input();
    let character = mace::default_character();
    let rolls = [
        local_roll(&first, Stat::PhysicalMinimum, vec![3.0, 7.0]),
        local_roll(&first, Stat::Speed, vec![13.0]),
    ];
    let weapon = first
        .prepare_mace_weapon(input.weapon, input.quality, input.item_level, &rolls)
        .unwrap();
    assert_eq!(weapon.modifier_roll_count(), 2);
    assert_eq!(weapon.consumed_modifier_count(), 3);
    assert!(
        mace::evaluate_with_components(
            &input,
            &character,
            &first,
            &weapon,
            first.mace_support_loadout(&[]).unwrap()
        )
        .is_ok()
    );
    assert!(
        mace::evaluate_with_components(
            &input,
            &character,
            &second,
            &weapon,
            second.mace_support_loadout(&[]).unwrap()
        )
        .is_err()
    );
    for changed in [
        MaceInput {
            quality: 1,
            ..input
        },
        MaceInput {
            item_level: 1,
            ..input
        },
        MaceInput {
            weapon: MaceWeapon::WoodenClub,
            ..input
        },
    ] {
        assert!(
            mace::evaluate_with_components(
                &changed,
                &character,
                &first,
                &weapon,
                first.mace_support_loadout(&[]).unwrap()
            )
            .is_err()
        );
    }
    fn send_sync<T: Send + Sync>() {}
    send_sync::<poe_optimizer_engine::weapon::PreparedWeaponStats>();
}
#[test]
fn custom_item_capture_policy_and_global_crit_cap_are_selected_data() {
    use game_data::LocalWeaponStat as Stat;
    let default = CompiledGameData::bundled().unwrap();
    let changed = custom(|package| {
        let rule = package
            .item_modifier_rules
            .iter_mut()
            .find(|rule| rule.modifiers[0].stat == Stat::Speed)
            .unwrap();
        rule.id = "custom_local_speed".into();
        rule.captures[0] = game_data::ItemCaptureKind::UnsignedDecimal;
        package.character.critical_chance_cap = 0.0;
    });
    let input = mace_input();
    let character = mace::default_character();
    let roll = local_roll(&changed, Stat::Speed, vec![13.5]);
    assert!(
        default
            .prepare_mace_weapon(
                input.weapon,
                0,
                input.item_level,
                std::slice::from_ref(&roll)
            )
            .is_err()
    );
    let integer_rule = local_roll(&default, Stat::Speed, vec![13.5]);
    assert!(
        default
            .prepare_mace_weapon(input.weapon, 0, input.item_level, &[integer_rule])
            .is_err()
    );
    let weapon = changed
        .prepare_mace_weapon(input.weapon, 0, input.item_level, &[roll])
        .unwrap();
    assert_eq!(weapon.stats().attack_speed_increased, 13.5);
    let output = mace::evaluate_with_components(
        &input,
        &character,
        &changed,
        &weapon,
        changed.mace_support_loadout(&[]).unwrap(),
    )
    .unwrap();
    assert_eq!(output.crit_chance, 0.0);
    assert_eq!(
        spark::evaluate_with_data(&spark_input(), &spark::default_character(), &changed)
            .unwrap()
            .crit_chance,
        0.0
    );
}
#[test]
fn local_modifier_roll_limits_and_unknown_or_unconsumed_inputs_reject() {
    use game_data::LocalWeaponStat as Stat;
    use poe_optimizer_engine::{modifiers::*, weapon};
    let data = CompiledGameData::bundled().unwrap();
    for values in [
        vec![],
        vec![1.0],
        vec![1.0, 2.0, 3.0],
        vec![-1.0, 2.0],
        vec![2.0, 1.0],
        vec![f64::NAN, 2.0],
        vec![1.0, f64::INFINITY],
        vec![1.0, 1_000_001.0],
        vec![1.25, 2.0],
    ] {
        let roll = local_roll(&data, Stat::PhysicalMinimum, values);
        assert!(
            data.prepare_mace_weapon(MaceWeapon::WoodenClub, 0, 1, &[roll])
                .is_err()
        );
    }
    let roll = local_roll(&data, Stat::PhysicalMinimum, vec![1.0, 2.0]);
    assert!(
        data.prepare_mace_weapon(MaceWeapon::WoodenClub, 0, 1, &vec![roll.clone(); 65])
            .is_err()
    );
    assert!(
        data.prepare_mace_weapon(MaceWeapon::WoodenClub, 21, 1, &[])
            .is_err()
    );
    for item_level in [0, 101] {
        assert!(
            data.prepare_mace_weapon(MaceWeapon::WoodenClub, 0, item_level, &[])
                .is_err()
        );
    }
    let large = data
        .prepare_mace_weapon(MaceWeapon::WoodenClub, 0, 1, &vec![roll; 64])
        .unwrap();
    assert_eq!(large.consumed_modifier_count(), 128);
    let global = ModifierInput {
        name: "PhysicalMin".into(),
        kind: ModifierKind::Numeric(NumericKind::Base),
        value: ModifierValue::Number(7.0),
        flags: 0,
        keyword_flags: 65_536,
        source: None,
        tag_kinds: vec![],
    };
    let mut remaining = vec![global.clone()];
    weapon::assemble_local_weapon(data.weapon(MaceWeapon::WoodenClub), 0, &mut remaining).unwrap();
    assert_eq!(remaining, vec![global]);
    let mut nonnumeric = vec![ModifierInput {
        name: "PhysicalMin".into(),
        kind: ModifierKind::Numeric(NumericKind::Base),
        value: ModifierValue::Unsupported {
            kind: "boolean".into(),
        },
        flags: 0,
        keyword_flags: 0,
        source: None,
        tag_kinds: vec![],
    }];
    assert!(
        weapon::consume_local_numeric(&mut nonnumeric, "PhysicalMin", NumericKind::Base, 0)
            .is_err()
    );
}
#[test]
fn legacy_mace_entry_points_delegate_to_empty_local_weapon_preparation() {
    let data = CompiledGameData::bundled().unwrap();
    let character = mace::default_character();
    for base in [MaceWeapon::WoodenClub, MaceWeapon::SmithingHammer] {
        for quality in 0..=20 {
            for brutality in [false, true] {
                let input = MaceInput {
                    weapon: base,
                    quality,
                    brutality,
                    ..mace_input()
                };
                let weapon = data
                    .prepare_mace_weapon(base, quality, input.item_level, &[])
                    .unwrap();
                let supports = data
                    .mace_support_loadout(&if brutality {
                        vec!["brutality_i".into()]
                    } else {
                        vec![]
                    })
                    .unwrap();
                assert_eq!(
                    mace::evaluate_with_data(&input, &character, &data).unwrap(),
                    mace::evaluate_with_components(&input, &character, &data, &weapon, supports)
                        .unwrap()
                );
            }
        }
    }
}
