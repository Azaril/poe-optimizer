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
    send_sync::<poe_optimizer_engine::actor::CompiledActorModifiers>();
    send_sync::<poe_optimizer_engine::actor::ActorScratch>();
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

#[test]
fn compiled_fragments_preserve_one_layer_more_rounding_and_source_override_order() {
    use poe_optimizer_engine::actor::{ActorModifierLayer, ActorScratch};
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(spark_input().quests);
    let sources = [
        vec![
            numeric(ActorStat::Life, Op::More, 13.0),
            numeric(ActorStat::Mana, Op::Override, 0.0),
        ],
        vec![
            numeric(ActorStat::Life, Op::More, 13.0),
            numeric(ActorStat::Mana, Op::Override, 17.5),
        ],
        vec![numeric(ActorStat::Str, Op::Base, 11.0)],
    ];
    let programs = sources
        .iter()
        .map(|source| data.compile_actor_modifiers(source).unwrap())
        .collect::<Vec<_>>();
    let refs = programs.iter().collect::<Vec<_>>();
    let mut scratch = ActorScratch::default();
    let actual = data
        .evaluate_actor_resources(
            60,
            quests,
            &character,
            &[ActorModifierLayer { programs: &refs }],
            &mut scratch,
        )
        .unwrap()
        .values();
    let full = data
        .prepare_actor_resources(60, quests, &character, &[sources.concat()])
        .unwrap()
        .values();
    assert_eq!(actual, full);
    assert_eq!(actual.mana, 0.0);
    let parent_refs = refs.iter().map(std::slice::from_ref).collect::<Vec<_>>();
    let parents = parent_refs
        .iter()
        .map(|programs| ActorModifierLayer { programs })
        .collect::<Vec<_>>();
    let grouped_as_parents = data
        .evaluate_actor_resources(60, quests, &character, &parents, &mut scratch)
        .unwrap()
        .values();
    assert_ne!(
        actual.life, grouped_as_parents.life,
        "fragment boundaries cannot introduce a MORE rounding step"
    );
    let reversed = [&programs[1], &programs[0], &programs[2]];
    assert_eq!(
        data.evaluate_actor_resources(
            60,
            quests,
            &character,
            &[ActorModifierLayer {
                programs: &reversed
            }],
            &mut scratch
        )
        .unwrap()
        .values()
        .mana,
        17.5
    );
}

#[test]
fn changing_tree_and_equipment_programs_match_fresh_preparation_without_allocating() {
    use poe_optimizer_data::game_data::{ActorCondition, ActorModifierTag};
    use poe_optimizer_engine::actor::{ActorModifierLayer, ActorScratch};
    let data = CompiledGameData::bundled().unwrap();
    let condition = |mut record: ActorModifierRecord| {
        record.tags.push(ActorModifierTag::Condition {
            variables: vec![ActorCondition::StrHigherThanInt],
            negated: false,
        });
        record
    };
    let configs = [
        vec![],
        vec![
            condition(numeric(ActorStat::Str, Op::Base, -30.0)),
            numeric(ActorStat::Life, Op::More, 13.0),
        ],
    ];
    let equipment = [
        vec![],
        vec![
            numeric(ActorStat::Str, Op::Base, 20.0),
            numeric(ActorStat::Life, Op::More, 13.0),
        ],
        vec![
            numeric(ActorStat::Dex, Op::Base, 21.0),
            numeric(ActorStat::Int, Op::Increased, 23.0),
            numeric(ActorStat::Mana, Op::Base, 19.0),
        ],
    ];
    let passives = [
        vec![],
        vec![numeric(ActorStat::Str, Op::Base, 10.0)],
        vec![
            numeric(ActorStat::Int, Op::Base, 30.0),
            condition(numeric(ActorStat::Life, Op::More, 17.0)),
        ],
    ];
    let compile = |sources: &[Vec<ActorModifierRecord>]| {
        sources
            .iter()
            .map(|source| data.compile_actor_modifiers(source).unwrap())
            .collect::<Vec<_>>()
    };
    let configs_compiled = compile(&configs);
    let equipment_compiled = compile(&equipment);
    let passives_compiled = compile(&passives);
    let characters = [
        data.default_spark_character(),
        data.default_mace_character(),
    ];
    let mut expected = Vec::new();
    for level in [1, 60, 100] {
        for (class, character) in characters.iter().enumerate() {
            for quest in [false, true] {
                let mut quests = data.actor_quest_selection(spark_input().quests);
                quests.spirit = [quest, !quest, quest];
                for (config, config_records) in configs.iter().enumerate() {
                    for (gear, equipment_records) in equipment.iter().enumerate() {
                        for (tree, passive_records) in passives.iter().enumerate() {
                            let records = [
                                config_records.as_slice(),
                                equipment_records.as_slice(),
                                passive_records.as_slice(),
                            ]
                            .concat();
                            let full = data
                                .prepare_actor_resources(level, quests, character, &[records])
                                .unwrap()
                                .values();
                            expected.push((level, class, quests, config, gear, tree, full));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(expected.len(), 216);
    let mace = mace_input();
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    let mut scratch = ActorScratch::default();
    let (_, count) = allocations(|| {
        for index in 0..2160 {
            let (level, class, quests, config, gear, tree, full) = expected[index % expected.len()];
            let character = &characters[class];
            let sources = [
                &configs_compiled[config],
                &equipment_compiled[gear],
                &passives_compiled[tree],
            ];
            let actor = data
                .evaluate_actor_resources(
                    level,
                    quests,
                    character,
                    &[ActorModifierLayer { programs: &sources }],
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(actor.values(), full);
            black_box(
                spark::evaluate_with_actor(
                    &SparkInput {
                        character_level: level,
                        ..spark_input()
                    },
                    character,
                    &data,
                    &actor,
                )
                .unwrap(),
            );
            black_box(
                mace::evaluate_with_actor(
                    &MaceInput {
                        character_level: level,
                        ..mace
                    },
                    character,
                    &data,
                    &weapon,
                    supports,
                    &actor,
                )
                .unwrap(),
            );
        }
    });
    assert_eq!(
        count, 0,
        "program assembly, actor calculation and both skill kernels allocate nothing"
    );
    assert!(
        expected
            .iter()
            .any(|entry| entry.6.attributes.strength == 0.0),
        "changing sources exercise the two-pass condition flip"
    );
    assert!(ActorScratch::storage_bytes() <= 1024);
    assert!(equipment_compiled[1].owned_heap_bytes() > 0);
    assert_eq!(equipment_compiled[1].record_count(), 2);
}

#[test]
fn compiled_program_ownership_bounds_and_failed_scratch_reuse_are_explicit() {
    use poe_optimizer_data::game_data::{ActorCondition, ActorModifierTag};
    use poe_optimizer_engine::actor::{ActorModifierLayer, ActorScratch};
    let data = CompiledGameData::bundled().unwrap();
    let foreign = CompiledGameData::compile(std::sync::Arc::new(
        poe_optimizer_data::game_data::bundled_snapshot().unwrap(),
    ))
    .unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(spark_input().quests);
    let records = [numeric(ActorStat::Str, Op::Base, 20.0)];
    let program = data.compile_actor_modifiers(&records).unwrap();
    let foreign_program = foreign.compile_actor_modifiers(&records).unwrap();
    let mut scratch = ActorScratch::default();
    let reference = data
        .prepare_actor_resources(60, quests, &character, &[records.to_vec()])
        .unwrap()
        .values();
    let valid = [&program];
    let foreign_refs = [&foreign_program];
    let overflow = data
        .compile_actor_modifiers(&vec![numeric(ActorStat::Str, Op::Base, 1_000_000.0); 2])
        .unwrap();
    let overflow_refs = [&overflow];
    let many = vec![&program; 257];
    for invalid in [&foreign_refs[..], &overflow_refs[..], many.as_slice()] {
        assert!(
            data.evaluate_actor_resources(
                60,
                quests,
                &character,
                &[ActorModifierLayer { programs: invalid }],
                &mut scratch
            )
            .is_err()
        );
        assert_eq!(
            data.evaluate_actor_resources(
                60,
                quests,
                &character,
                &[ActorModifierLayer { programs: &valid }],
                &mut scratch
            )
            .unwrap()
            .values(),
            reference
        );
    }
    let layers = vec![ActorModifierLayer { programs: &valid }; 17];
    assert!(
        data.evaluate_actor_resources(60, quests, &character, &layers, &mut scratch)
            .is_err()
    );
    let oversized = data
        .compile_actor_modifiers(&vec![numeric(ActorStat::Life, Op::Base, 1.0); 257])
        .unwrap();
    assert!(
        data.evaluate_actor_resources(
            60,
            quests,
            &character,
            &[ActorModifierLayer {
                programs: &[&oversized, &oversized]
            }],
            &mut scratch
        )
        .is_err()
    );
    let mut unsupported = numeric(ActorStat::Life, Op::Base, 1.0);
    unsupported.flags = 1;
    unsupported.tags = vec![ActorModifierTag::Condition {
        variables: vec![ActorCondition::StrHigherThanInt],
        negated: true,
    }];
    assert!(
        data.compile_actor_modifiers(&[unsupported]).is_err(),
        "unsupported metadata cannot hide behind a false condition"
    );
    assert!(
        data.compile_actor_modifiers(&vec![numeric(ActorStat::Life, Op::Base, 1.0); 513])
            .is_err()
    );
    for record in [
        numeric(ActorStat::LifeConvertToArmour, Op::Base, 50.0),
        flag(ActorStat::ChaosInoculation, true),
    ] {
        let program = data
            .compile_actor_modifiers(std::slice::from_ref(&record))
            .unwrap();
        let actor = data
            .evaluate_actor_resources(
                60,
                quests,
                &character,
                &[ActorModifierLayer {
                    programs: &[&program],
                }],
                &mut scratch,
            )
            .unwrap();
        assert_eq!(
            actor.values(),
            data.prepare_actor_resources(60, quests, &character, &[vec![record]])
                .unwrap()
                .values()
        );
        assert!(
            spark::evaluate_with_actor(&spark_input(), &character, &data, &actor)
                .unwrap_err()
                .0
                .contains("unsupported receiving defence")
        );
    }
}

#[test]
fn actor_can_bind_validated_scalar_character_modifiers_without_recalculating_attributes() {
    let data = CompiledGameData::bundled().unwrap();
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
    let mut full = character;
    full.modifiers.skill_speed_increased = 15.0;
    full.modifiers.energy_shield_flat = 17.0;
    assert!(
        spark::evaluate_with_actor(&spark, &full, &data, &actor).is_err(),
        "implicit rebinding remains forbidden"
    );
    let (rebound, count) = allocations(|| actor.with_character(&full).unwrap());
    assert_eq!(count, 0);
    assert_eq!(rebound.values(), actor.values());
    assert_eq!(rebound.character_level(), actor.character_level());
    assert_eq!(rebound.quests(), actor.quests());
    let before = spark::evaluate_with_actor(&spark, &character, &data, &actor).unwrap();
    let after = spark::evaluate_with_actor(&spark, &full, &data, &rebound).unwrap();
    assert_eq!(before.life, after.life);
    assert_eq!(before.mana, after.mana);
    assert_eq!(before.spirit, after.spirit);
    assert!(after.hit_dps > before.hit_dps);
    assert_eq!(after.energy_shield, 17.0);
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    assert_eq!(
        mace::evaluate_with_actor(&mace, &full, &data, &weapon, supports, &rebound)
            .unwrap()
            .life,
        after.life
    );
    let mut wrong_attributes = full;
    wrong_attributes.attributes.strength += 1.0;
    assert!(actor.with_character(&wrong_attributes).is_err());
    let mut wrong_scalar = full;
    wrong_scalar.modifiers.energy_shield_flat = -1.0;
    assert!(actor.with_character(&wrong_scalar).is_err());
    let foreign = CompiledGameData::compile(std::sync::Arc::new(
        poe_optimizer_data::game_data::bundled_snapshot().unwrap(),
    ))
    .unwrap();
    assert!(spark::evaluate_with_actor(&spark, &full, &foreign, &rebound).is_err());
    assert!(
        spark::evaluate_with_actor(
            &SparkInput {
                character_level: 59,
                ..spark
            },
            &full,
            &data,
            &rebound
        )
        .is_err()
    );
    let donor = data
        .prepare_actor_resources(
            60,
            data.actor_quest_selection(spark.quests),
            &character,
            &[vec![numeric(
                ActorStat::LifeConvertToArmour,
                Op::Base,
                50.0,
            )]],
        )
        .unwrap();
    let donor = donor.with_character(&full).unwrap();
    assert!(
        spark::evaluate_with_actor(&spark, &full, &data, &donor)
            .unwrap_err()
            .0
            .contains("unsupported receiving defence")
    );
}

#[test]
fn complete_receiving_preparation_binds_scenario_and_refuses_scalar_reordering_or_missing_stage() {
    let data = CompiledGameData::bundled().unwrap();
    let input = spark_input();
    let character = data.default_spark_character();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    let records = vec![vec![
        numeric(ActorStat::EnergyShield, Op::Base, 11.5),
        numeric(ActorStat::Defences, Op::Increased, 50.0),
        numeric(ActorStat::FireResist, Op::Base, 81.5),
    ]];
    let prepared = data
        .prepare_actor(60, quests, scenario, &character, &records)
        .unwrap();
    let output = spark::evaluate_with_actor(&input, &character, &data, &prepared).unwrap();
    assert_eq!(output.energy_shield, 17.0);
    assert_eq!(
        output.energy_shield,
        prepared.receiving().unwrap().energy_shield
    );
    assert_eq!(
        output.fire_resistance,
        prepared.receiving().unwrap().resistances.fire
    );
    assert!(
        spark::evaluate_with_actor(
            &SparkInput {
                resistance_penalty: -20.0,
                ..input
            },
            &character,
            &data,
            &prepared
        )
        .is_err()
    );
    let mut different = input;
    different.quests.beira = !different.quests.beira;
    assert!(spark::evaluate_with_actor(&different, &character, &data, &prepared).is_err());
    let foreign = CompiledGameData::compile(std::sync::Arc::new(
        poe_optimizer_data::game_data::bundled_snapshot().unwrap(),
    ))
    .unwrap();
    assert!(spark::evaluate_with_actor(&input, &character, &foreign, &prepared).is_err());
    let mut different = character;
    different.modifiers.attack_damage_increased = 25.0;
    let rebound = prepared.with_character(&different).unwrap();
    assert_eq!(rebound.receiving(), prepared.receiving());
    different.modifiers.fire_resistance_flat = 1.0;
    assert!(prepared.with_character(&different).is_err());
    assert!(
        data.prepare_actor(60, quests, scenario, &different, &records)
            .is_err()
    );
    let resources = data
        .prepare_actor_resources(60, quests, &character, &records)
        .unwrap();
    assert!(resources.receiving().is_none());
    assert!(spark::evaluate_with_actor(&input, &character, &data, &resources).is_err());
    for record in [
        numeric(ActorStat::LifeConvertToEnergyShield, Op::Base, 20.0),
        flag(ActorStat::ChaosInoculation, true),
    ] {
        let incomplete = data
            .prepare_actor(60, quests, scenario, &character, &[vec![record]])
            .unwrap();
        assert!(spark::evaluate_with_actor(&input, &character, &data, &incomplete).is_err());
    }
    for invalid in [
        numeric(ActorStat::Defences, Op::Base, 20.0),
        numeric(ActorStat::Armour, Op::More, 20.0),
        numeric(ActorStat::FireResist, Op::Override, 50.0),
    ] {
        assert!(data.compile_actor_modifiers(&[invalid]).is_err());
    }
}
#[test]
fn changing_actor_receiver_source_programs_and_both_prepared_skills_allocate_nothing() {
    use poe_optimizer_data::game_data::{ActorCondition, ActorModifierTag};
    use poe_optimizer_engine::actor::{ActorModifierLayer, ActorScratch};
    let data = CompiledGameData::bundled().unwrap();
    let input = spark_input();
    let mace = mace_input();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    let mut es = numeric(ActorStat::EnergyShield, Op::Base, 20.5);
    es.tags = vec![
        ActorModifierTag::Global,
        ActorModifierTag::Condition {
            variables: vec![ActorCondition::DexHigherThanInt],
            negated: false,
        },
    ];
    let config = data
        .compile_actor_modifiers(&[es, numeric(ActorStat::Defences, Op::Increased, 25.0)])
        .unwrap();
    let equipment = [0.0, 10.0, 40.0].map(|value| {
        data.compile_actor_modifiers(&[
            numeric(ActorStat::Dex, Op::Base, value),
            numeric(ActorStat::FireResist, Op::Base, value),
        ])
        .unwrap()
    });
    let passives = [0.0, 25.0, 50.0].map(|value| {
        data.compile_actor_modifiers(&[
            numeric(ActorStat::Int, Op::Base, value),
            numeric(ActorStat::ArmourAndEvasion, Op::Base, value),
        ])
        .unwrap()
    });
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    let mut scratch = ActorScratch::default();
    let (checksum, count) = allocations(|| {
        let mut checksum = 0.0;
        for iteration in 0..3000 {
            let programs = [
                &config,
                &equipment[iteration % 3],
                &passives[(iteration / 3) % 3],
            ];
            let actor = data
                .evaluate_actor(
                    60,
                    quests,
                    scenario,
                    &character,
                    &[ActorModifierLayer {
                        programs: &programs,
                    }],
                    &mut scratch,
                )
                .unwrap();
            let output =
                black_box(spark::evaluate_with_actor(&input, &character, &data, &actor).unwrap());
            let melee = black_box(
                mace::evaluate_with_actor(&mace, &character, &data, &weapon, supports, &actor)
                    .unwrap(),
            );
            checksum += output.energy_shield
                + output.evasion
                + output.fire_resistance
                + melee.armour
                + melee.hit_dps;
        }
        checksum
    });
    assert!(checksum.is_finite());
    assert_eq!(count, 0);
    let foreign = CompiledGameData::compile(std::sync::Arc::new(
        poe_optimizer_data::game_data::bundled_snapshot().unwrap(),
    ))
    .unwrap();
    let before = data
        .evaluate_actor(
            60,
            quests,
            scenario,
            &character,
            &[ActorModifierLayer {
                programs: &[&config],
            }],
            &mut scratch,
        )
        .unwrap();
    assert!(
        foreign
            .evaluate_actor(
                60,
                quests,
                scenario,
                &character,
                &[ActorModifierLayer {
                    programs: &[&config]
                }],
                &mut scratch
            )
            .is_err()
    );
    let after = data
        .evaluate_actor(
            60,
            quests,
            scenario,
            &character,
            &[ActorModifierLayer {
                programs: &[&config],
            }],
            &mut scratch,
        )
        .unwrap();
    assert_eq!(before.receiving(), after.receiving());
}

#[test]
fn local_armour_components_bind_data_slots_and_reject_unconsumed_local_only_records() {
    use poe_optimizer_data::game_data::{ActorCondition, ActorModifierTag, EquipmentSlot};
    use poe_optimizer_engine::{
        actor::{ActorModifierLayer, ActorScratch},
        armour::{self, ArmourSlots},
    };
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let input = spark_input();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    let base = data
        .snapshot()
        .package()
        .armour_bases
        .iter()
        .find(|base| base.slot == EquipmentSlot::Helmet)
        .unwrap();
    let mut global = numeric(ActorStat::EnergyShield, Op::Increased, 45.0);
    global.tags.push(ActorModifierTag::Global);
    let records = vec![
        numeric(ActorStat::ArmourAndEnergyShield, Op::Base, 25.0),
        numeric(ActorStat::EnergyShield, Op::Increased, -250.0),
        global,
    ];
    let item = data.prepare_armour(&base.id, 7, 1, &records).unwrap();
    assert_eq!(item.consumed_modifier_count(), 2);
    assert_eq!(item.global_records(), &records[2..]);
    assert_eq!(item.global_program().record_count(), 1);
    assert!(item.stats().energy_shield < 0.0);
    assert!(item.footprint() >= std::mem::size_of_val(&item));
    for quality in [21, u32::MAX] {
        assert!(data.prepare_armour(&base.id, quality, 1, &[]).is_err());
    }
    for level in [0, 101, u32::MAX] {
        assert!(data.prepare_armour(&base.id, 0, level, &[]).is_err());
    }
    assert!(data.prepare_armour("unknown", 0, 1, &[]).is_err());
    let slots = ArmourSlots {
        helmet: Some(&item),
        ..Default::default()
    };
    let actor = data
        .prepare_actor_with_armour(
            60,
            quests,
            scenario,
            &character,
            &[item.global_records().to_vec()],
            slots,
        )
        .unwrap();
    let mut scratch = ActorScratch::default();
    let programs = [item.global_program()];
    let layers = [ActorModifierLayer {
        programs: &programs,
    }];
    assert_eq!(
        actor.receiving(),
        data.evaluate_actor_with_armour(
            60,
            quests,
            scenario,
            &character,
            &layers,
            slots,
            &mut scratch
        )
        .unwrap()
        .receiving()
    );
    let foreign = CompiledGameData::compile(std::sync::Arc::new(
        poe_optimizer_data::game_data::bundled_snapshot().unwrap(),
    ))
    .unwrap();
    assert!(
        foreign
            .prepare_actor_with_armour(60, quests, scenario, &character, &[], slots)
            .unwrap_err()
            .0
            .contains("different compiled dataset")
    );
    assert!(
        foreign
            .evaluate_actor_with_armour(60, quests, scenario, &character, &[], slots, &mut scratch)
            .is_err()
    );
    let wrong = ArmourSlots {
        gloves: Some(&item),
        ..Default::default()
    };
    assert!(
        data.prepare_actor_with_armour(60, quests, scenario, &character, &[], wrong)
            .is_err()
    );
    assert!(
        data.evaluate_actor_with_armour(60, quests, scenario, &character, &[], wrong, &mut scratch)
            .is_err()
    );
    assert_eq!(
        actor.receiving(),
        data.evaluate_actor_with_armour(
            60,
            quests,
            scenario,
            &character,
            &layers,
            slots,
            &mut scratch
        )
        .unwrap()
        .receiving()
    );
    for stat in [
        ActorStat::ArmourAndEnergyShield,
        ActorStat::EvasionAndEnergyShield,
    ] {
        for op in [Op::Base, Op::Increased] {
            for conditional in [false, true] {
                let mut record = numeric(stat, op, 25.0);
                if conditional {
                    record.tags.push(ActorModifierTag::Condition {
                        variables: vec![ActorCondition::DexHigherThanInt],
                        negated: false,
                    });
                }
                assert_eq!(armour::is_local_modifier(&record), !conditional);
                assert!(
                    data.compile_actor_modifiers(std::slice::from_ref(&record))
                        .is_err()
                );
                assert!(
                    data.prepare_actor_resources(60, quests, &character, &[vec![record.clone()]])
                        .is_err()
                );
                assert!(
                    data.prepare_actor(60, quests, scenario, &character, &[vec![record.clone()]])
                        .is_err()
                );
                assert!(
                    data.prepare_actor_with_armour(
                        60,
                        quests,
                        scenario,
                        &character,
                        &[vec![record.clone()]],
                        slots
                    )
                    .is_err()
                );
                assert_eq!(
                    data.prepare_armour(&base.id, 0, 1, &[record]).is_ok(),
                    !conditional
                );
            }
        }
    }
    let mut different = character;
    different.modifiers.evasion_flat = 1.0;
    assert!(actor.with_character(&different).is_err());
    assert!(
        spark::evaluate_with_actor(
            &SparkInput {
                resistance_penalty: 0.0,
                ..input
            },
            &character,
            &data,
            &actor
        )
        .is_err()
    );
}
#[test]
fn changing_three_local_armour_slots_actor_composition_and_both_skills_allocate_nothing() {
    use poe_optimizer_data::game_data::{ActorCondition, ActorModifierTag, EquipmentSlot};
    use poe_optimizer_engine::{
        actor::{ActorModifierLayer, ActorScratch},
        armour::ArmourSlots,
    };
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let input = spark_input();
    let mace = mace_input();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    let mut conditional = numeric(ActorStat::EnergyShield, Op::Base, 12.5);
    conditional.tags.push(ActorModifierTag::Condition {
        variables: vec![ActorCondition::IntHigherThanDex],
        negated: false,
    });
    let config = data
        .compile_actor_modifiers(&[
            conditional,
            numeric(ActorStat::Defences, Op::Increased, 25.0),
        ])
        .unwrap();
    let items = [
        EquipmentSlot::Helmet,
        EquipmentSlot::Gloves,
        EquipmentSlot::Boots,
    ]
    .map(|slot| {
        let base = data
            .snapshot()
            .package()
            .armour_bases
            .iter()
            .find(|base| base.slot == slot)
            .unwrap();
        [0, 7, 20].map(|quality| {
            data.prepare_armour(
                &base.id,
                quality,
                1,
                &[
                    numeric(ActorStat::ArmourAndEvasion, Op::Base, 20.5),
                    numeric(ActorStat::ArmourAndEnergyShield, Op::Increased, 33.333333),
                    numeric(ActorStat::EnergyShield, Op::Base, quality as f64 + 1.5),
                    numeric(ActorStat::Int, Op::Base, quality as f64),
                    numeric(ActorStat::ElementalResist, Op::Base, quality as f64),
                ],
            )
            .unwrap()
        })
    });
    let passive = [0.0, 15.0, 40.0].map(|dex| {
        data.compile_actor_modifiers(&[
            numeric(ActorStat::Dex, Op::Base, dex),
            numeric(ActorStat::Armour, Op::Base, dex),
        ])
        .unwrap()
    });
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    let mut scratch = ActorScratch::default();
    let (checksum, count) = allocations(|| {
        let mut checksum = 0.0;
        for i in 0..3240 {
            let helmet = &items[0][i % 3];
            let gloves = &items[1][(i / 3) % 3];
            let boots = &items[2][(i / 9) % 3];
            let programs = [
                &config,
                helmet.global_program(),
                gloves.global_program(),
                boots.global_program(),
                &passive[(i / 27) % 3],
            ];
            let actor = data
                .evaluate_actor_with_armour(
                    60,
                    quests,
                    scenario,
                    &character,
                    &[ActorModifierLayer {
                        programs: &programs,
                    }],
                    ArmourSlots {
                        helmet: Some(helmet),
                        gloves: Some(gloves),
                        boots: Some(boots),
                    },
                    &mut scratch,
                )
                .unwrap();
            let spark =
                black_box(spark::evaluate_with_actor(&input, &character, &data, &actor).unwrap());
            let mace = black_box(
                mace::evaluate_with_actor(&mace, &character, &data, &weapon, supports, &actor)
                    .unwrap(),
            );
            checksum +=
                spark.energy_shield + spark.armour + spark.evasion + spark.life + mace.hit_dps;
        }
        checksum
    });
    assert!(checksum.is_finite());
    assert_eq!(count, 0);
    assert!(std::mem::size_of::<ActorScratch>() <= 1024);
}
