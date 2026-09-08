//! Complete shared action and timing original-source oracles.
use super::*;
use poe_optimizer_data::game_data::{
    self, ActorCondition as AC, ActorGlobalEffectType, ActorModifierEffect as Effect,
    ActorModifierRecord as Record, ActorModifierTag as Tag, ActorNumericOperation as Op,
    ActorStat as Stat,
};
use poe_optimizer_engine::{
    CompiledGameData,
    actor::{ActorModifierLayer, ActorScratch},
    timing::{self, DirectActionTimingInput as TimingInput},
};
use std::sync::Arc;
fn numeric(stat: Stat, operation: Op, value: f64) -> Record {
    Record {
        stat,
        effect: Effect::Numeric { operation, value },
        source: Some("Item:42:Action test".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn conditioned(mut row: Record, condition: AC, negated: bool) -> Record {
    row.tags.push(Tag::Condition {
        variables: vec![condition],
        negated,
    });
    row
}
fn flag(value: bool) -> Record {
    Record {
        stat: Stat::UnaffectedBySlows,
        effect: Effect::Flag { value },
        source: Some("Passive:Source".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn custom(mut package: game_data::GameDataPackage) -> CompiledGameData {
    package.refresh_section_digests().unwrap();
    let snapshot = game_data::GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &game_data::TrustPolicy::AllowCustom,
        &game_data::LoadLimits::default(),
    )
    .unwrap();
    CompiledGameData::compile(Arc::new(snapshot)).unwrap()
}
#[test]
fn max_and_positive_rows_preserve_source_filters_order_and_requesting_store_reevaluation() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for maximum in [
            -1e16,
            -0.0,
            0.0,
            0.125,
            100.0,
            1e16,
            f64::INFINITY,
            f64::NAN,
        ] {
            let mut max = modifier("Max", NumericKind::Max, maximum);
            max.source = Some("Item:42".into());
            let layers = vec![
                vec![
                    max.clone().into(),
                    TaggedModifierInput {
                        modifier: modifier("Max", NumericKind::Max, 3.5),
                        tags: vec![condition("Root", false)],
                    },
                    modifier("Speed", NumericKind::Increased, 1e16).into(),
                    modifier("Speed", NumericKind::Increased, -1e16).into(),
                ],
                vec![
                    TaggedModifierInput {
                        modifier: modifier("Max", NumericKind::Max, 70.0),
                        tags: vec![condition("Root", false)],
                    },
                    modifier("Speed", NumericKind::Increased, 1.0).into(),
                ],
                vec![modifier("Speed", NumericKind::Increased, 1.0).into()],
            ];
            let native = ModifierDatabase::try_new_tagged(layers.clone()).unwrap();
            for root in [false, true] {
                let stores = vec![
                    BTreeMap::from([("Root".into(), root)]),
                    BTreeMap::from([("ParentOnly".into(), true)]),
                    BTreeMap::new(),
                ];
                let environment = ConditionEnvironment::try_new(ConditionEnvironmentInput {
                    store_conditions: stores.clone(),
                    actors: vec![ConditionActor::default()],
                    ..Default::default()
                })
                .unwrap();
                let db = oracle.tagged_database(&layers);
                oracle.populate_conditions(&db, &stores);
                // Wrap, never replace, original EvalMod. MAX must invoke the root
                // store on surviving rows a second time, even parent modifiers.
                oracle.lua.load("return function(db) local original=db.EvalMod; db.calls=0; db.EvalMod=function(self,...) assert(self==db); db.calls=db.calls+1; return original(self,...) end end").eval::<Function>().unwrap().call::<()>(db.clone()).unwrap();
                for source in [None, Some("Item".to_owned()), Some("Item:42".to_owned())] {
                    let query = QueryContext {
                        source,
                        ..Default::default()
                    };
                    let cfg = oracle.lua.create_table().unwrap();
                    cfg.set("source", query.source.as_deref()).unwrap();
                    let names = oracle.lua.create_sequence_from(["Max"]).unwrap();
                    let expected: Option<f64> = oracle
                        .query
                        .call((db.clone(), cfg.clone(), names, "MAX", warm))
                        .unwrap();
                    assert_number(
                        native
                            .max_with_conditions(&query, &["Max"], &environment)
                            .unwrap(),
                        expected,
                    );
                    let expected: Option<f64> = oracle
                        .query
                        .call((
                            db.clone(),
                            cfg,
                            oracle
                                .lua
                                .create_sequence_from(["Speed", "IgnoredExtraName"])
                                .unwrap(),
                            "POSITIVE_INC",
                            warm,
                        ))
                        .unwrap();
                    assert_number(
                        Some(
                            native
                                .sum_positive_with_conditions(
                                    SumKind::Increased,
                                    &query,
                                    "Speed",
                                    &environment,
                                )
                                .unwrap(),
                        ),
                        expected,
                    );
                }
                assert!(db.get::<u64>("calls").unwrap() > 0);
            }
        }
        let empty = ModifierDatabase::try_new(vec![vec![
            modifier("M", NumericKind::Max, 0.0),
            modifier("M", NumericKind::Max, -1.0),
        ]])
        .unwrap();
        assert_eq!(empty.max(&QueryContext::default(), &["M"]).unwrap(), None);
    }
}
#[test]
fn action_speed_full_and_compiled_actors_match_cold_warm_source_with_conditions_and_layers() {
    let reference = CompiledGameData::bundled().unwrap();
    let mut package = reference.snapshot().package().clone();
    package.action_speed.temporal_chains_effect_cap = 41.25;
    let custom = custom(package);
    let mut comparisons = 0;
    for warm in [false, true] {
        let source = actor_parity::ActorOracle::new_receiving(warm);
        for data in [reference.as_ref(), &custom] {
            let quests = data.actor_quest_selection(SparkQuestRewards::default());
            let scenario = data.receiving_scenario(SparkQuestRewards::default(), -60.0);
            for str_bonus in [0.0, 40.0, 300.0] {
                for slows in [false, true] {
                    for (speed, temporal, min, max) in [
                        (-75.0, -120.0, 0.0, 0.0),
                        (-100.0, 0.0, 0.0, -1.0),
                        (15.125, -12.75, 110.0, 20.0),
                        (1e6, -1e6, 13.0, 150.0),
                        (0.0, 15.5, -1.0, 1.0),
                    ] {
                        let mut implicit = numeric(Stat::MinimumActionSpeed, Op::Max, min);
                        implicit.tags.push(Tag::GlobalEffect {
                            effect_type: ActorGlobalEffectType::Global,
                            unscalable: true,
                        });
                        let layers = vec![
                            vec![
                                numeric(Stat::Str, Op::Base, str_bonus),
                                conditioned(
                                    numeric(Stat::Dex, Op::Base, 40.0),
                                    AC::IntHighestAttribute,
                                    false,
                                ),
                                numeric(Stat::ActionSpeed, Op::Increased, speed),
                                numeric(Stat::ActionSpeed, Op::Increased, 1.125),
                                conditioned(flag(slows), AC::StrHighestAttribute, false),
                                conditioned(implicit, AC::StrHighestAttribute, false),
                            ],
                            vec![
                                numeric(Stat::TemporalChainsActionSpeed, Op::Increased, temporal),
                                numeric(Stat::TemporalChainsActionSpeed, Op::Increased, 0.75),
                                numeric(Stat::MaximumActionSpeedReduction, Op::Max, max),
                                conditioned(
                                    numeric(Stat::ActionSpeed, Op::Increased, -40.0),
                                    AC::IntHighestAttribute,
                                    false,
                                ),
                            ],
                            vec![numeric(Stat::ActionSpeed, Op::Increased, 1e-11)],
                        ];
                        let character = data.default_spark_character();
                        let actor = data
                            .prepare_actor(60, quests, scenario, &character, &layers)
                            .unwrap();
                        let expected = source
                            .calculate_receiving(data, 60, quests, scenario, &character, &layers);
                        let output: Table = expected.get("output").unwrap();
                        assert_number(
                            Some(actor.action_speed().action_speed_mod),
                            Some(output.get("ActionSpeedMod").unwrap()),
                        );
                        assert_number(
                            Some(actor.movement().effective_movement_speed_mod),
                            Some(output.get("EffectiveMovementSpeedMod").unwrap()),
                        );
                        let programs: Vec<Vec<_>> = layers
                            .iter()
                            .map(|layer| {
                                layer
                                    .chunks(2)
                                    .map(|records| data.compile_actor_modifiers(records).unwrap())
                                    .collect()
                            })
                            .collect();
                        let refs: Vec<Vec<_>> = programs
                            .iter()
                            .map(|layer| layer.iter().collect())
                            .collect();
                        let borrowed: Vec<_> = refs
                            .iter()
                            .map(|programs| ActorModifierLayer { programs })
                            .collect();
                        let compiled = data
                            .evaluate_actor(
                                60,
                                quests,
                                scenario,
                                &character,
                                &borrowed,
                                &mut ActorScratch::default(),
                            )
                            .unwrap();
                        assert_eq!(compiled.action_speed(), actor.action_speed());
                        assert_eq!(compiled.movement(), actor.movement());
                        assert_eq!(compiled.values(), actor.values());
                        comparisons += 1;
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 120);
}
struct TimingOracle {
    oracle: Oracle,
    calculate: Function,
}
impl TimingOracle {
    fn new(warm: bool) -> Self {
        let oracle = SparkOracle::new(warm).oracle;
        let source = SPARK_OFFENCE.replace("\r\n", "\n");
        let mut body = String::from(
            "return function(input) local m_min=math.min; local skillModList=new('ModDB'):ModDB(); local cfg={}; local skillFlags={selfCast=true}; local skillData={}; local activeSkill={skillTypes={}}; local globalOutput={ActionSpeedMod=input.action}; local output={Repeats=input.repeats}; local baseTime=input.base; local inc=input.inc; local more=input.more; skillModList:NewMod('TotalAttackTime','BASE',input.attack); skillModList:NewMod('TotalCastTime','BASE',input.cast); ",
        );
        // The selected-data precision is the source-extracted literal. Keep the
        // actual arithmetic statement intact and replace only that declared input.
        body.push_str("local sourceRound=round; local round=function(value,precision) assert(precision==2); return sourceRound(value,input.precision) end; ");
        body.push_str(source_line(
            &source,
            "output.Speed = 1 / (baseTime / round(",
        ));
        append_direct_action_tail(&mut body, &source);
        body.push_str(" return output end");
        let calculate = oracle
            .lua
            .load(body)
            .set_name("original-ordinary-self-cast-nonchannel-speed-and-time")
            .eval()
            .unwrap();
        Self { oracle, calculate }
    }
    fn calculate(&self, input: TimingInput, tick: f64, precision: u32) -> Table {
        self.oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("misc")
            .unwrap()
            .set("ServerTickRate", tick)
            .unwrap();
        let table = self.oracle.lua.create_table().unwrap();
        table.set("precision", precision).unwrap();
        for (name, value) in [
            ("base", input.base_time),
            ("inc", input.increased),
            ("more", input.more),
            ("attack", input.additional_attack_time),
            ("cast", input.additional_cast_time),
            ("action", input.action_speed_mod),
            ("repeats", input.repeats),
        ] {
            table.set(name, value).unwrap();
        }
        let wrapper:Function=self.oracle.lua.load("return function(f,i,w) local v; for n=1,(w and 200 or 1) do v=f(i) end; return v end").eval().unwrap();
        wrapper
            .call((self.calculate.clone(), table, self.oracle.warm))
            .unwrap()
    }
}
#[test]
fn ordinary_direct_timing_source_pins_reciprocals_zero_tiny_cap_and_negative_boundaries() {
    let data = CompiledGameData::bundled().unwrap();
    let mut settings = data.snapshot().package().direct_action_timing.clone();
    let mut checks = 0;
    for warm in [false, true] {
        let source = TimingOracle::new(warm);
        for tick in [1.0 / 0.033, 7.125, 1e6] {
            settings.server_tick_rate = tick;
            settings.speed_multiplier_rounding_precision = if tick == 7.125 { 3 } else { 2 };
            for (base, inc, more, action, attack, cast, repeats) in [
                (0.7, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
                (0.7, 13.555, 0.87, 1.25, 0.0, 0.0, 1.0),
                (0.7, -100.0, 1.0, 1.0, 0.0, 0.0, 1.0),
                (0.7, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0),
                (0.7, 0.0, 1.0, -0.0, 0.0, 0.0, 1.0),
                (0.7, 0.0, 1.0, -1.25, 0.0, 0.0, 1.0),
                (0.7, 0.0, 1.0, 1e-310, 0.0, 0.0, 1.0),
                (1e-310, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
                (0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
                (0.0, -100.0, 1.0, 1.0, 0.0, 0.0, 1.0),
                (1e6, 1e6, -1.0, 1e6, 0.0, 0.0, 1.0),
                (-0.7, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
                (0.7, 1e6, 1e6, 1e6, 0.0, 0.0, 1.0),
                (0.7, 25.0, 1.25, 1.5, 0.125, 0.25, 2.0),
                (0.7, 25.0, 1.25, 1.5, -0.125, -0.25, 0.0),
            ] {
                let input = TimingInput {
                    base_time: base,
                    increased: inc,
                    more,
                    action_speed_mod: action,
                    additional_attack_time: attack,
                    additional_cast_time: cast,
                    repeats,
                };
                let actual = timing::calculate(&settings, input);
                let expected =
                    source.calculate(input, tick, settings.speed_multiplier_rounding_precision);
                for (name, value) in [
                    ("CastRate", actual.cast_rate),
                    ("Speed", actual.speed),
                    ("Time", actual.time),
                ] {
                    assert_number(Some(value), Some(expected.get(name).unwrap()));
                }
                checks += 1;
            }
        }
    }
    assert_eq!(checks, 90);
}
#[test]
fn both_complete_skill_pipelines_apply_action_then_cap_with_all_seven_loadouts() {
    let data = CompiledGameData::bundled().unwrap();
    let input = mace_parity::input();
    let spark_input = SparkInput {
        character_level: 60,
        resistance_penalty: -60.0,
        enemy_lightning_resistance: 0.0,
        quests: input.quests,
    };
    let mut checks = 0;
    for warm in [false, true] {
        let mace_source = mace_parity::MaceOracle::new(warm);
        let spark_source = SparkOracle::new(warm);
        for action in [0.0, 0.25, 1.25, 100.0] {
            for speed in [0.0, 13.555, 1e6] {
                let mut character = data.default_mace_character();
                character.modifiers.skill_speed_increased = speed;
                let layers = vec![vec![numeric(
                    Stat::ActionSpeed,
                    Op::Increased,
                    (action - 1.0) * 100.0,
                )]];
                let actor = data
                    .prepare_actor_resources(
                        60,
                        data.actor_quest_selection(input.quests),
                        &character,
                        &layers,
                    )
                    .unwrap();
                let resolved = actor.action_speed().action_speed_mod;
                mace_source
                    .oracle
                    .lua
                    .globals()
                    .set("testActionSpeed", resolved)
                    .unwrap();
                spark_source
                    .oracle
                    .lua
                    .globals()
                    .set("testActionSpeed", resolved)
                    .unwrap();
                let spark =
                    spark::evaluate_with_actor(&spark_input, &character, &data, &actor).unwrap();
                let expected = spark_source.calculate_with_character(&spark_input, &character);
                for (name, value) in [
                    ("CastRate", spark.cast_rate),
                    ("Speed", spark.timing.speed),
                    ("Time", spark.timing.time),
                    ("TotalDPS", spark.hit_dps),
                ] {
                    assert_number(Some(value), Some(expected.get(name).unwrap()));
                }
                for keys in [
                    vec![],
                    vec!["brutality_i"],
                    vec!["rapid_attacks_i"],
                    vec!["heavy_swing"],
                    vec!["brutality_i", "rapid_attacks_i"],
                    vec!["brutality_i", "heavy_swing"],
                    vec!["heavy_swing", "rapid_attacks_i"],
                ] {
                    let keys: Vec<String> = keys.into_iter().map(str::to_owned).collect();
                    let supports = data.mace_support_loadout(&keys).unwrap();
                    let ids: Vec<_> = keys
                        .iter()
                        .map(|id| {
                            data.snapshot()
                                .package()
                                .support(id)
                                .unwrap()
                                .skill_id
                                .as_str()
                        })
                        .collect();
                    let weapon = data
                        .prepare_mace_weapon(input.weapon, input.quality, input.item_level, &[])
                        .unwrap();
                    let actual = poe_optimizer_engine::mace::evaluate_with_actor(
                        &input, &character, &data, &weapon, supports, &actor,
                    )
                    .unwrap();
                    let expected = mace_source.calculate_with_support_ids(&input, &character, &ids);
                    mace_parity::compare(actual, expected);
                    checks += 1;
                }
            }
        }
    }
    assert_eq!(checks, 168);
}
