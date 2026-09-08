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
