//! Actor preparation ownership, admitted skill composition and hot-loop contracts.
#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_data::game_data::{
    ActorModifierEffect, ActorModifierRecord, ActorNumericOperation as Op, ActorStat,
};
use poe_optimizer_engine::{
    CompiledGameData,
    actor::PreparedActorResources,
    mace::{self, MaceInput, MaceWeapon},
    spark::{self, SparkInput, SparkQuestRewards},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};

// Count only the measured test thread; delegate all allocation unchanged.
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
fn record_allocation() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: Exact caller layout is forwarded to System.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: Exact caller layout is forwarded to System.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        // SAFETY: All pointer/layout arguments are passed unchanged to System.
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer was allocated by System with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn allocations<T>(run: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNTING.with(|flag| flag.set(false));
        }
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|flag| flag.set(true));
    let reset = Reset;
    let result = run();
    drop(reset);
    (result, ALLOCATIONS.with(Cell::get))
}
fn numeric(stat: ActorStat, operation: Op, value: f64) -> ActorModifierRecord {
    ActorModifierRecord {
        stat,
        effect: ActorModifierEffect::Numeric { operation, value },
        source: Some("Custom".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn flag(stat: ActorStat, value: bool) -> ActorModifierRecord {
    ActorModifierRecord {
        stat,
        effect: ActorModifierEffect::Flag { value },
        source: Some("Custom".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
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
        weapon: MaceWeapon::WoodenClub,
        quality: 20,
        item_level: 1,
        brutality: false,
        resistance_penalty: -60.0,
        quests: SparkQuestRewards::default(),
        enemy_armour: 1500.0,
        enemy_evasion: 591.0,
        enemy_fire_resistance: 0.0,
    }
}
fn layers() -> Vec<Vec<ActorModifierRecord>> {
    vec![vec![
        numeric(ActorStat::Str, Op::Base, 20.0),
        numeric(ActorStat::Dex, Op::Increased, 25.0),
        numeric(ActorStat::Int, Op::More, 30.0),
        numeric(ActorStat::Life, Op::More, 20.0),
        numeric(ActorStat::Mana, Op::Base, 11.0),
        numeric(ActorStat::Spirit, Op::Increased, 50.0),
        numeric(ActorStat::Accuracy, Op::More, 40.0),
    ]]
}

#[test]
fn one_prepared_actor_changes_both_skills_and_preserves_empty_adapter_outputs() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PreparedActorResources>();
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let spark = spark_input();
    let mace = mace_input();
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    let quests = data.actor_quest_selection(spark.quests);
    let plain = data
        .prepare_actor_resources(60, quests, &character, &[])
        .unwrap();
    let changed = data
        .prepare_actor_resources(60, quests, &character, &layers())
        .unwrap();
    let old_spark = spark::evaluate_with_data(&spark, &character, &data).unwrap();
    let old_mace =
        mace::evaluate_with_components(&mace, &character, &data, &weapon, supports).unwrap();
    assert_eq!(
        old_spark,
        spark::evaluate_with_actor(&spark, &character, &data, &plain).unwrap()
    );
    assert_eq!(
        old_mace,
        mace::evaluate_with_actor(&mace, &character, &data, &weapon, supports, &plain).unwrap()
    );
    let new_spark = spark::evaluate_with_actor(&spark, &character, &data, &changed).unwrap();
    let new_mace =
        mace::evaluate_with_actor(&mace, &character, &data, &weapon, supports, &changed).unwrap();
    for (a, b, source) in [
        (new_spark.life, new_mace.life, changed.values().life),
        (new_spark.mana, new_mace.mana, changed.values().mana),
        (new_spark.spirit, new_mace.spirit, changed.values().spirit),
        (
            new_spark.strength,
            new_mace.strength,
            changed.values().attributes.strength,
        ),
        (
            new_spark.dexterity,
            new_mace.dexterity,
            changed.values().attributes.dexterity,
        ),
        (
            new_spark.intelligence,
            new_mace.intelligence,
            changed.values().attributes.intelligence,
        ),
    ] {
        assert_eq!(a, b);
        assert_eq!(a, source);
    }
    assert!(new_spark.life > old_spark.life);
    assert!(new_spark.mana > old_spark.mana);
    assert!(new_spark.spirit > old_spark.spirit);
    assert!(new_mace.accuracy > old_mace.accuracy);
    assert!(new_mace.hit_dps > old_mace.hit_dps);
    assert_eq!(new_spark.hit_dps, old_spark.hit_dps);
}

#[test]
fn actor_binding_rejects_foreign_compilation_character_level_and_resource_quests() {
    let data = CompiledGameData::bundled().unwrap();
    let foreign = CompiledGameData::compile(std::sync::Arc::new(
        poe_optimizer_data::game_data::bundled_snapshot().unwrap(),
    ))
    .unwrap();
    assert_eq!(data.identity(), foreign.identity());
    let character = data.default_mace_character();
    let spark = spark_input();
    let mace = mace_input();
    let actor = data
        .prepare_actor_resources(
            60,
            data.actor_quest_selection(spark.quests),
            &character,
            &layers(),
        )
        .unwrap();
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    for (compiled, actor_character, actor_spark, actor_mace) in [
        (&foreign, character, spark, mace),
        (
            &data,
            {
                let mut c = character;
                c.attributes.strength += 1.0;
                c
            },
            spark,
            mace,
        ),
        (
            &data,
            character,
            SparkInput {
                character_level: 59,
                ..spark
            },
            MaceInput {
                character_level: 59,
                ..mace
            },
        ),
        (
            &data,
            character,
            SparkInput {
                quests: SparkQuestRewards {
                    candlemass: false,
                    ..spark.quests
                },
                ..spark
            },
            MaceInput {
                quests: SparkQuestRewards {
                    candlemass: false,
                    ..mace.quests
                },
                ..mace
            },
        ),
    ] {
        assert!(
            spark::evaluate_with_actor(&actor_spark, &actor_character, compiled, &actor).is_err()
        );
        assert!(
            mace::evaluate_with_actor(
                &actor_mace,
                &actor_character,
                compiled,
                &weapon,
                supports,
                &actor
            )
            .is_err()
        );
    }
}

#[test]
fn empty_stack_and_generic_modifier_paths_share_all_class_quest_and_level_arithmetic() {
    let data = CompiledGameData::bundled().unwrap();
    let inert = vec![vec![flag(ActorStat::NoAttributeBonuses, false)]];
    for level in [1, 60, 100] {
        for character in [
            data.default_mace_character(),
            data.default_spark_character(),
        ] {
            for mask in 0..64 {
                let mut quests = data.actor_quest_selection(SparkQuestRewards {
                    candlemass: mask & 1 != 0,
                    molten_shrine: mask & 2 != 0,
                    silent_hall: mask & 4 != 0,
                    ..SparkQuestRewards::default()
                });
                quests.spirit = std::array::from_fn(|i| mask & (8 << i) != 0);
                let stack = data
                    .prepare_actor_resources(level, quests, &character, &[])
                    .unwrap();
                let generic = data
                    .prepare_actor_resources(level, quests, &character, &inert)
                    .unwrap();
                assert_eq!(
                    stack.values(),
                    generic.values(),
                    "level {level} mask {mask}"
                );
            }
        }
    }
}

#[test]
fn raw_pool_conversions_and_ci_cannot_enter_incomplete_skill_defence_pipeline() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let spark = spark_input();
    let mace = mace_input();
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    for record in [
        numeric(ActorStat::LifeConvertToArmour, Op::Base, 50.0),
        flag(ActorStat::ChaosInoculation, true),
    ] {
        let actor = data
            .prepare_actor_resources(
                60,
                data.actor_quest_selection(spark.quests),
                &character,
                &[vec![record]],
            )
            .unwrap();
        assert!(actor.values().life.is_finite());
        assert!(
            spark::evaluate_with_actor(&spark, &character, &data, &actor)
                .unwrap_err()
                .0
                .contains("unsupported receiving defence")
        );
        assert!(
            mace::evaluate_with_actor(&mace, &character, &data, &weapon, supports, &actor)
                .unwrap_err()
                .0
                .contains("unsupported receiving defence")
        );
    }
}

#[test]
fn untrusted_records_are_validated_before_any_query() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(spark_input().quests);
    let valid = numeric(ActorStat::Life, Op::Base, 20.0);
    for bad in [
        ActorModifierRecord {
            flags: 1,
            ..valid.clone()
        },
        ActorModifierRecord {
            keyword_flags: 1,
            ..valid.clone()
        },
        numeric(ActorStat::Life, Op::Base, f64::NAN),
        numeric(ActorStat::Life, Op::Base, 1_000_001.0),
        numeric(ActorStat::ExtraLife, Op::More, 10.0),
        flag(ActorStat::Life, false),
    ] {
        assert!(
            data.prepare_actor_resources(60, quests, &character, &[vec![bad]])
                .is_err()
        );
    }
    assert!(
        data.prepare_actor_resources(60, quests, &character, &vec![vec![]; 17])
            .is_err()
    );
    assert!(
        data.prepare_actor_resources(60, quests, &character, &[vec![valid; 513]])
            .is_err()
    );
    for level in [0, 101] {
        assert!(
            data.prepare_actor_resources(level, quests, &character, &[])
                .is_err()
        );
    }
}

#[test]
fn prepared_skill_loops_and_empty_compatibility_adapters_allocate_nothing() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let spark = spark_input();
    let mace = mace_input();
    let quests = data.actor_quest_selection(spark.quests);
    let records = layers();
    let (prepared, prep_allocations) = allocations(|| {
        data.prepare_actor_resources(60, quests, &character, &records)
            .unwrap()
    });
    assert!(
        prep_allocations > 0,
        "nonempty record assembly belongs to preparation"
    );
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    let (_, hot_allocations) = allocations(|| {
        for _ in 0..1000 {
            black_box(
                spark::evaluate_with_actor(black_box(&spark), &character, &data, &prepared)
                    .unwrap(),
            );
            black_box(
                mace::evaluate_with_actor(
                    black_box(&mace),
                    &character,
                    &data,
                    &weapon,
                    supports,
                    &prepared,
                )
                .unwrap(),
            );
            black_box(
                data.prepare_actor_resources(60, quests, &character, &[])
                    .unwrap(),
            );
            black_box(spark::evaluate_with_data(black_box(&spark), &character, &data).unwrap());
            black_box(
                mace::evaluate_with_components(
                    black_box(&mace),
                    &character,
                    &data,
                    &weapon,
                    supports,
                )
                .unwrap(),
            );
        }
    });
    assert_eq!(hot_allocations, 0);
    // Fixed metadata plus the numeric snapshot; no hidden record/condition heap.
    assert!(std::mem::size_of::<PreparedActorResources>() <= 512);
}
