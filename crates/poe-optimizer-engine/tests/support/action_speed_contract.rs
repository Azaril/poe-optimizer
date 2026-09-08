use super::*;
use poe_optimizer_data::game_data::{self, ActorCondition as Condition, ActorModifierTag as Tag};
use poe_optimizer_engine::actor::{ActorModifierLayer, ActorScratch};
use std::sync::Arc;
fn conditional(mut row: ActorModifierRecord, condition: Condition) -> ActorModifierRecord {
    row.tags.push(Tag::Condition {
        variables: vec![condition],
        negated: false,
    });
    row
}
fn custom(edit: impl FnOnce(&mut game_data::GameDataPackage)) -> CompiledGameData {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    edit(&mut package);
    package.refresh_section_digests().unwrap();
    CompiledGameData::compile(Arc::new(
        game_data::GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &game_data::TrustPolicy::AllowCustom,
            &game_data::LoadLimits::default(),
        )
        .unwrap(),
    ))
    .unwrap()
}
#[test]
fn action_stage_binding_unsupported_operations_and_inactive_metadata_remain_strict() {
    let data = CompiledGameData::bundled().unwrap();
    let other =
        CompiledGameData::compile(Arc::new(game_data::bundled_snapshot().unwrap())).unwrap();
    let character = data.default_mace_character();
    let input = mace_input();
    let quests = data.actor_quest_selection(input.quests);
    for record in [
        numeric(ActorStat::ActionSpeed, Op::Base, 1.0),
        numeric(ActorStat::ActionSpeed, Op::More, 1.0),
        numeric(ActorStat::MovementSpeed, Op::Max, 1.0),
        numeric(ActorStat::MinimumActionSpeed, Op::Override, 1.0),
    ] {
        let record = conditional(record, Condition::IntSingleHighestAttribute);
        assert!(
            data.compile_actor_modifiers(std::slice::from_ref(&record))
                .is_err()
        );
        assert!(
            data.prepare_actor_resources(60, quests, &character, &[vec![record]])
                .is_err()
        );
    }
    let records = vec![numeric(ActorStat::ActionSpeed, Op::Increased, 25.0)];
    let program = data.compile_actor_modifiers(&records).unwrap();
    let actor = data
        .prepare_actor_resources(60, quests, &character, &[records])
        .unwrap();
    let weapon = data
        .prepare_mace_weapon(input.weapon, input.quality, input.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    assert!(
        other
            .evaluate_actor_resources(
                60,
                quests,
                &character,
                &[ActorModifierLayer {
                    programs: &[&program]
                }],
                &mut ActorScratch::default()
            )
            .is_err()
    );
    assert!(
        mace::evaluate_with_actor(&input, &character, &other, &weapon, supports, &actor).is_err()
    );
    assert!(mace::action_timing(&other, &character, &weapon, supports, 1.25).is_err());
    let mut changed = character;
    changed.modifiers.skill_speed_increased = 100.0;
    assert!(mace::evaluate_with_actor(&input, &changed, &data, &weapon, supports, &actor).is_err());
    let rebound = actor.with_character(&changed).unwrap();
    assert_eq!(rebound.action_speed(), actor.action_speed());
    assert!(
        mace::evaluate_with_actor(&input, &changed, &data, &weapon, supports, &rebound)
            .unwrap()
            .timing
            .cast_rate
            > mace::evaluate_with_actor(&input, &character, &data, &weapon, supports, &actor)
                .unwrap()
                .timing
                .cast_rate
    );
}
#[test]
fn injected_action_and_timing_constants_change_every_consumer_and_preserve_nonfinite_evidence() {
    let data = custom(|package| {
        package.action_speed.base_multiplier = 1.5;
        package.action_speed.default_minimum_percent = 25.0;
        package.action_speed.percent_divisor = 80.0;
        package.action_speed.temporal_chains_effect_cap = 20.0;
        package.direct_action_timing.server_tick_rate = 0.75;
        package
            .direct_action_timing
            .speed_multiplier_rounding_precision = 3;
    });
    let mut character = data.default_mace_character();
    character.modifiers.skill_speed_increased = 13.555;
    let spark_input = spark_input();
    let input = mace_input();
    let records = vec![
        numeric(ActorStat::ActionSpeed, Op::Increased, 40.0),
        numeric(ActorStat::TemporalChainsActionSpeed, Op::Increased, -50.0),
    ];
    let actor = data
        .prepare_actor_resources(
            60,
            data.actor_quest_selection(input.quests),
            &character,
            &[records],
        )
        .unwrap();
    assert_eq!(actor.action_speed().action_speed_mod, 1.75);
    assert_eq!(actor.movement().action_speed_mod, 1.75);
    let spark = spark::evaluate_with_actor(&spark_input, &character, &data, &actor).unwrap();
    let weapon = data
        .prepare_mace_weapon(input.weapon, input.quality, input.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    let mace =
        mace::evaluate_with_actor(&input, &character, &data, &weapon, supports, &actor).unwrap();
    assert_eq!(spark.timing.speed_multiplier, 1.136);
    assert_eq!(mace.timing.speed_multiplier, 1.136);
    assert_eq!(spark.timing.speed, 0.75);
    assert_eq!(mace.attack_rate, 0.75);
    assert!(spark.cast_rate > spark.timing.speed);
    assert!(mace.timing.cast_rate > mace.attack_rate);
    assert_eq!(mace.hit_dps, mace.average_damage * 0.75);
    assert_eq!(spark.hit_dps, spark.average_hit * 100.0 / 100.0 * 0.75);
    let timing = spark::action_timing(&data, &character, 1e-310).unwrap();
    assert!(timing.speed > 0.0);
    assert_eq!(timing.time, f64::INFINITY);
    let zero = spark::action_timing(&data, &character, -0.0).unwrap();
    assert_eq!(zero.speed.to_bits(), (-0.0f64).to_bits());
    assert_eq!(zero.time.to_bits(), 0.0f64.to_bits());
}
#[test]
fn changing_actor_programs_and_all_support_timings_allocate_nothing_and_match_fresh_preparation() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let input = mace_input();
    let spark_input = spark_input();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    let config = vec![
        conditional(
            flag(ActorStat::UnaffectedBySlows, true),
            Condition::StrHighestAttribute,
        ),
        numeric(ActorStat::TemporalChainsActionSpeed, Op::Increased, -25.0),
        numeric(ActorStat::TemporalChainsActionSpeed, Op::Increased, 12.5),
        conditional(
            numeric(ActorStat::ActionSpeed, Op::Increased, -60.0),
            Condition::DexHighestAttribute,
        ),
    ];
    let gear = [0.0, 7.5, 30.0, 150.0].map(|value| {
        vec![
            numeric(ActorStat::ActionSpeed, Op::Increased, value),
            numeric(ActorStat::Str, Op::Base, value),
        ]
    });
    let trees = [0.0, 30.0, 300.0].map(|value| {
        vec![
            numeric(ActorStat::Dex, Op::Base, value),
            conditional(
                numeric(ActorStat::MinimumActionSpeed, Op::Max, 110.0),
                Condition::DexHighestAttribute,
            ),
        ]
    });
    let config_program = data.compile_actor_modifiers(&config).unwrap();
    let gear_programs = gear
        .each_ref()
        .map(|records| data.compile_actor_modifiers(records).unwrap());
    let tree_programs = trees
        .each_ref()
        .map(|records| data.compile_actor_modifiers(records).unwrap());
    let expected: Vec<_> = gear
        .iter()
        .flat_map(|gear| {
            trees.iter().map(|tree| {
                let mut records = config.clone();
                records.extend_from_slice(gear);
                records.extend_from_slice(tree);
                data.prepare_actor(60, quests, scenario, &character, &[records])
                    .unwrap()
            })
        })
        .collect();
    let keys = [
        vec![],
        vec!["brutality_i"],
        vec!["heavy_swing"],
        vec!["rapid_attacks_i"],
        vec!["brutality_i", "heavy_swing"],
        vec!["brutality_i", "rapid_attacks_i"],
        vec!["heavy_swing", "rapid_attacks_i"],
    ]
    .map(|keys| keys.into_iter().map(str::to_owned).collect::<Vec<_>>());
    let supports = keys
        .each_ref()
        .map(|keys| data.mace_support_loadout(keys).unwrap());
    let weapon = data
        .prepare_mace_weapon(input.weapon, input.quality, input.item_level, &[])
        .unwrap();
    let mut scratch = ActorScratch::default();
    let ((checksum, slow, fast), count) = allocations(|| {
        let mut checksum = 0.0;
        let mut slow = 0;
        let mut fast = 0;
        for i in 0..8400 {
            let g = i % 4;
            let t = (i / 4) % 3;
            let actor = data
                .evaluate_actor(
                    60,
                    quests,
                    scenario,
                    &character,
                    &[ActorModifierLayer {
                        programs: &[&config_program, &gear_programs[g], &tree_programs[t]],
                    }],
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(actor.action_speed(), expected[g * 3 + t].action_speed());
            assert_eq!(actor.movement(), expected[g * 3 + t].movement());
            slow += usize::from(actor.action_speed().action_speed_mod < 1.5);
            fast += usize::from(actor.action_speed().action_speed_mod >= 1.5);
            let spark = black_box(
                spark::evaluate_with_actor(&spark_input, &character, &data, &actor).unwrap(),
            );
            let mace = black_box(
                mace::evaluate_with_actor(
                    &input,
                    &character,
                    &data,
                    &weapon,
                    supports[(i / 12) % 7],
                    &actor,
                )
                .unwrap(),
            );
            assert_eq!(
                spark.action_speed_mod,
                actor.action_speed().action_speed_mod
            );
            assert_eq!(mace.action_speed_mod, actor.action_speed().action_speed_mod);
            checksum += spark.hit_dps
                + mace.hit_dps
                + spark.timing.time
                + mace.timing.time
                + actor.movement().effective_movement_speed_mod;
        }
        (checksum, slow, fast)
    });
    assert_eq!(count, 0);
    assert!(checksum.is_finite() && slow > 0 && fast > 0);
    assert!(ActorScratch::storage_bytes() <= 1024);
}
