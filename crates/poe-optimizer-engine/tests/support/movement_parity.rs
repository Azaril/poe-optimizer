//! Original movement and generated item penalty branches, cold and warmed.
use super::*;
use poe_optimizer_data::game_data::{
    self, ActorCondition as Condition, ActorModifierEffect as Effect,
    ActorModifierRecord as Record, ActorModifierTag as Tag, ActorNumericOperation as Op,
    ActorStat as Stat, EquipmentSlot,
};
use poe_optimizer_engine::{
    CompiledGameData,
    actor::{ActorModifierLayer, ActorScratch, MovementOutput},
    armour::{ArmourBaseValues, ArmourSlots},
    character::CharacterInput,
    movement::{self, MovementInput},
};
use std::sync::Arc;

fn record(stat: Stat, operation: Op, value: f64) -> Record {
    Record {
        stat,
        effect: Effect::Numeric { operation, value },
        source: Some("movement source".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn flag(stat: Stat, value: bool) -> Record {
    Record {
        stat,
        effect: Effect::Flag { value },
        source: Some("movement flag source".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn conditioned(mut record: Record, condition: Condition, negated: bool) -> Record {
    record.tags.push(Tag::Condition {
        variables: vec![condition],
        negated,
    });
    record
}
fn compare(actual: MovementOutput, expected: &Table) {
    let output: Table = expected.get("output").unwrap();
    for (name, value) in [
        ("MovementSpeedMod", actual.movement_speed_mod),
        ("ActionSpeedMod", actual.action_speed_mod),
        (
            "EffectiveMovementSpeedMod",
            actual.effective_movement_speed_mod,
        ),
    ] {
        let source: f64 = output.get(name).unwrap();
        assert_number(Some(value), Some(source));
        if source == 0.0 && value == 0.0 {
            assert_eq!(source.to_bits(), value.to_bits(), "{name} signed zero");
        }
    }
    assert_eq!(
        actual.ignore_movement_penalties,
        expected
            .get::<Option<bool>>("ignore_movement")
            .unwrap()
            .unwrap_or(false)
    );
    assert_eq!(
        actual.cannot_be_below_base,
        expected.get::<bool>("movement_floor").unwrap()
    );
    assert_eq!(
        actual.has_override,
        expected.get::<bool>("movement_override").unwrap()
    );
}
fn check(
    data: &CompiledGameData,
    source: &actor_parity::ActorOracle,
    character: &CharacterInput,
    layers: &[Vec<Record>],
) -> MovementOutput {
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    let scenario = data.receiving_scenario(SparkQuestRewards::default(), -60.0);
    let expected = source.calculate_receiving(data, 60, quests, scenario, character, layers);
    let prepared = data
        .prepare_actor(60, quests, scenario, character, layers)
        .unwrap();
    compare(prepared.movement(), &expected);
    let components: Vec<Vec<_>> = layers
        .iter()
        .map(|layer| {
            layer
                .chunks(2)
                .map(|records| data.compile_actor_modifiers(records).unwrap())
                .collect()
        })
        .collect();
    let refs: Vec<Vec<_>> = components
        .iter()
        .map(|layer| layer.iter().collect())
        .collect();
    let borrowed: Vec<_> = refs
        .iter()
        .map(|programs| ActorModifierLayer { programs })
        .collect();
    let program = data
        .evaluate_actor(
            60,
            quests,
            scenario,
            character,
            &borrowed,
            &mut ActorScratch::default(),
        )
        .unwrap();
    assert_eq!(prepared.movement(), program.movement());
    assert_eq!(prepared.values(), program.values());
    assert_eq!(prepared.receiving(), program.receiving());
    prepared.movement()
}

#[test]
fn original_movement_matches_signed_rounding_overrides_flags_conditions_and_parent_order() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let mut count = 0;
    for warm in [false, true] {
        let source = actor_parity::ActorOracle::new_receiving(warm);
        for base in [-2.5, -1.0, -0.05, -0.0, 0.0, 0.0005, 0.10000000000001] {
            for inc in [-250.0, -100.0, -0.05, 0.0, 13.33333, 150.0] {
                for more in [-200.0, -100.0, 0.0, 13.3333] {
                    let layers = vec![vec![
                        record(Stat::MovementSpeed, Op::Base, base),
                        record(Stat::MovementSpeed, Op::Increased, inc),
                        record(Stat::MovementSpeed, Op::More, more),
                    ]];
                    check(&data, &source, &character, &layers);
                    count += 1;
                }
            }
        }
        for override_value in [
            None,
            Some(-1.1255),
            Some(-0.0),
            Some(0.0),
            Some(0.50049),
            Some(1.33349),
        ] {
            for floor in [false, true] {
                for ignore in [false, true] {
                    let mut local = vec![
                        conditioned(
                            record(Stat::MovementSpeed, Op::Base, -0.05),
                            Condition::IgnoreMovementPenalties,
                            true,
                        ),
                        flag(Stat::IgnoreMovementPenalties, ignore),
                        flag(Stat::MovementSpeedCannotBeBelowBase, floor),
                        record(Stat::MovementSpeed, Op::Increased, 20.0),
                    ];
                    if let Some(value) = override_value {
                        local.push(record(Stat::MovementSpeed, Op::Override, value));
                    }
                    check(&data, &source, &character, &[local]);
                    count += 1;
                }
            }
        }
        // Genuine two-pass condition change: initial Str=18, Int=5; Str drops to
        // zero on pass two, so only the final Int>Str flag ignores the penalty.
        let layers = vec![vec![
            conditioned(
                record(Stat::Str, Op::Base, -30.0),
                Condition::StrHigherThanInt,
                false,
            ),
            conditioned(
                flag(Stat::IgnoreMovementPenalties, true),
                Condition::IntHigherThanStr,
                false,
            ),
            conditioned(
                record(Stat::MovementSpeed, Op::Base, -0.05),
                Condition::IgnoreMovementPenalties,
                true,
            ),
            conditioned(
                record(Stat::MovementSpeed, Op::Increased, 25.0),
                Condition::IntHigherThanStr,
                false,
            ),
        ]];
        let movement = check(&data, &source, &character, &layers);
        assert!(movement.ignore_movement_penalties);
        count += 1;
        for parent_override in [None, Some(0.0), Some(1.22249)] {
            let mut parent = vec![record(Stat::MovementSpeed, Op::More, 13.3333)];
            if let Some(value) = parent_override {
                parent.push(record(Stat::MovementSpeed, Op::Override, value));
            }
            let local = vec![
                record(Stat::MovementSpeed, Op::Base, -0.04),
                record(Stat::MovementSpeed, Op::More, 13.3333),
                record(Stat::MovementSpeed, Op::More, 13.3333),
            ];
            check(&data, &source, &character, &[local.clone(), parent.clone()]);
            let mut overridden = vec![record(Stat::MovementSpeed, Op::Override, 0.0)];
            overridden.extend(local);
            assert_eq!(
                check(&data, &source, &character, &[overridden, parent]).movement_speed_mod,
                0.0
            );
            count += 2;
        }
    }
    eprintln!("Original complete actor movement: {count} cold/warm query/condition cases");
}

#[test]
fn original_movement_numeric_branch_matches_explicit_action_speed_and_query_short_circuit() {
    let data = CompiledGameData::bundled().unwrap();
    let mut count = 0;
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm).oracle;
        let block = section(
            SPARK_DEFENCE,
            "\toutput.MovementSpeedMod =",
            "\tif breakdown then",
        );
        let calculate: Function = oracle.lua.load(format!("local m_max=math.max; return function(input,warm) local result; for i=1,(warm and 200 or 1) do local output={{ActionSpeedMod=input.action}}; local modDB={{}}; function modDB:Override(_,name) return input.override end; function modDB:Flag(_,name) return name=='MovementSpeedCannotBeBelowBase' and input.floor end; function modDB:Sum(kind,_,name) return kind=='BASE' and input.base or input.inc end; function modDB:More(_,_,name) return input.more end; {block} result=output end; return result end")).eval().unwrap();
        for override_value in [None, Some(-0.0), Some(0.0), Some(0.1234567), Some(-2.5)] {
            for action in [-2.0, -0.0, 0.0, 0.55555, 1.0, 1.3333] {
                for floor in [false, true] {
                    let input = MovementInput {
                        base: -0.035,
                        increased: 12.345,
                        more: 1.1234567,
                        override_value,
                        cannot_be_below_base: floor,
                        ignore_movement_penalties: false,
                        action_speed_mod: action,
                    };
                    let table = oracle.lua.create_table().unwrap();
                    for (name, value) in [
                        ("base", input.base),
                        ("inc", input.increased),
                        ("more", input.more),
                        ("action", action),
                    ] {
                        table.set(name, value).unwrap();
                    }
                    table.set("override", override_value).unwrap();
                    table.set("floor", floor).unwrap();
                    let expected: Table = calculate.call((table, warm)).unwrap();
                    let result =
                        movement::calculate(&data.snapshot().package().movement, input).unwrap();
                    for (name, value) in [
                        ("MovementSpeedMod", result.movement_speed_mod),
                        (
                            "EffectiveMovementSpeedMod",
                            result.effective_movement_speed_mod,
                        ),
                    ] {
                        assert_number(Some(value), Some(expected.get(name).unwrap()));
                    }
                    count += 1;
                }
            }
        }
        // Unused numeric query inputs are not evaluated when an override exists.
        let result = movement::calculate(
            &data.snapshot().package().movement,
            MovementInput {
                base: f64::NAN,
                increased: f64::NAN,
                more: f64::NAN,
                override_value: Some(0.0),
                cannot_be_below_base: false,
                ignore_movement_penalties: false,
                action_speed_mod: 1.0,
            },
        )
        .unwrap();
        assert_eq!(result.effective_movement_speed_mod, 0.0);
    }
    eprintln!("Original movement arithmetic: {count} cold/warm explicit action-speed cases");
}

#[test]
fn original_body_item_penalty_and_four_slot_receiver_preserve_generation_and_source_order() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    let scenario = data.receiving_scenario(SparkQuestRewards::default(), -60.0);
    let bases: Vec<_> = [
        EquipmentSlot::Helmet,
        EquipmentSlot::BodyArmour,
        EquipmentSlot::Gloves,
        EquipmentSlot::Boots,
    ]
    .into_iter()
    .map(|slot| {
        data.snapshot()
            .package()
            .armour_bases
            .iter()
            .find(|base| base.slot == slot)
            .unwrap()
    })
    .collect();
    let mut count = 0;
    for warm in [false, true] {
        let local = armour_parity::ArmourOracle::new(warm);
        let source = actor_parity::ActorOracle::new_receiving(warm);
        for quality in [0, 13, 20] {
            let items: Vec<_> = bases
                .iter()
                .enumerate()
                .map(|(index, base)| {
                    let source_name = format!("Item:{}:source-order armour", index + 1);
                    let records = vec![
                        record(Stat::ArmourAndEvasion, Op::Base, 17.5),
                        record(Stat::Int, Op::Base, 1.0 + index as f64),
                        record(
                            Stat::MovementSpeed,
                            Op::Increased,
                            [1000000.0, 0.00000000001, -1000000.0, 33.0][index],
                        ),
                    ];
                    let mut raw = Vec::new();
                    for record in &records {
                        let Effect::Numeric { operation, value } = record.effect else {
                            unreachable!()
                        };
                        raw.push(ModifierInput {
                            name: record.stat.upstream_name().into(),
                            kind: ModifierKind::Numeric(match operation {
                                Op::Base => NumericKind::Base,
                                Op::Increased => NumericKind::Increased,
                                _ => unreachable!(),
                            }),
                            value: ModifierValue::Number(value),
                            source: record.source.clone(),
                            flags: 0,
                            keyword_flags: 0,
                            tag_kinds: vec![],
                        });
                    }
                    let (expected, remaining) = local.assembly_with_penalty(
                        ArmourBaseValues {
                            armour: base.armour,
                            evasion: base.evasion,
                            energy_shield: base.energy_shield,
                        },
                        quality,
                        &raw,
                        base.movement_penalty,
                        &source_name,
                    );
                    let prepared = data
                        .prepare_armour_with_source(&base.id, quality, 60, &source_name, &records)
                        .unwrap();
                    assert_eq!(prepared.stats(), expected);
                    assert_eq!(prepared.source_global_records(), &records[1..]);
                    assert_eq!(remaining.raw_len(), prepared.global_records().len());
                    if let Some(penalty) = base.movement_penalty {
                        let generated = prepared.generated_global_records();
                        assert_eq!(generated.len(), 1);
                        let original: Table = remaining.get(remaining.raw_len()).unwrap();
                        assert_eq!(original.get::<String>("name").unwrap(), "MovementSpeed");
                        assert_eq!(original.get::<String>("type").unwrap(), "BASE");
                        assert_eq!(original.get::<f64>("value").unwrap(), -penalty);
                        assert_eq!(original.get::<String>("source").unwrap(), source_name);
                        assert_eq!(original.get::<u64>("flags").unwrap(), 0);
                        assert_eq!(original.get::<u64>("keywordFlags").unwrap(), 0);
                        assert_eq!(original.raw_len(), 1);
                        let tag: Table = original.get(1).unwrap();
                        assert_eq!(tag.get::<String>("type").unwrap(), "Condition");
                        assert_eq!(tag.get::<String>("var").unwrap(), "IgnoreMovementPenalties");
                        assert!(tag.get::<bool>("neg").unwrap());
                        assert_eq!(generated[0].source.as_deref(), Some(source_name.as_str()));
                        assert_eq!(
                            generated[0].effect,
                            Effect::Numeric {
                                operation: Op::Base,
                                value: -penalty
                            }
                        );
                    } else {
                        assert!(prepared.generated_global_records().is_empty());
                    }
                    prepared
                })
                .collect();
            for mask in 0..16 {
                let mut records = vec![record(Stat::MovementSpeed, Op::Increased, 7.0)];
                let mut original_slots = vec![];
                for (index, item) in items.iter().enumerate() {
                    if mask & (1 << index) != 0 {
                        records.extend_from_slice(item.global_records());
                        original_slots.push((
                            ["Helmet", "Body Armour", "Gloves", "Boots"][index],
                            item.stats(),
                        ));
                    }
                }
                let slots = ArmourSlots {
                    helmet: (mask & 1 != 0).then_some(&items[0]),
                    body_armour: (mask & 2 != 0).then_some(&items[1]),
                    gloves: (mask & 4 != 0).then_some(&items[2]),
                    boots: (mask & 8 != 0).then_some(&items[3]),
                };
                let expected = source.calculate_receiving_with_armour(
                    &data,
                    60,
                    quests,
                    scenario,
                    &character,
                    &[records.clone()],
                    &original_slots,
                );
                let prepared = data
                    .prepare_actor_with_armour(
                        60,
                        quests,
                        scenario,
                        &character,
                        &[records.clone()],
                        slots,
                    )
                    .unwrap();
                compare(prepared.movement(), &expected);
                receiving_parity::compare(prepared.receiving().unwrap(), &expected);
                let program = data.compile_actor_modifiers(&records).unwrap();
                let repeated = data
                    .evaluate_actor_with_armour(
                        60,
                        quests,
                        scenario,
                        &character,
                        &[ActorModifierLayer {
                            programs: &[&program],
                        }],
                        slots,
                        &mut ActorScratch::default(),
                    )
                    .unwrap();
                assert_eq!(prepared.movement(), repeated.movement());
                assert_eq!(prepared.receiving(), repeated.receiving());
                count += 1;
            }
        }
    }
    eprintln!(
        "Original generated body penalties and four-slot actor composition: {count} cold/warm actor cases"
    );
}

#[test]
fn injected_movement_precision_and_parent_more_rules_drive_shared_queries() {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    package
        .actor
        .high_precision_mods
        .insert("MovementSpeed".into(), BTreeMap::from([(Op::More, 4)]));
    package.refresh_section_digests().unwrap();
    let data = CompiledGameData::compile(Arc::new(
        game_data::GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &game_data::TrustPolicy::AllowCustom,
            &game_data::LoadLimits::default(),
        )
        .unwrap(),
    ))
    .unwrap();
    for warm in [false, true] {
        let source = actor_parity::ActorOracle::new_receiving(warm);
        for more in [0.12345, 13.33333, -7.77777] {
            check(
                &data,
                &source,
                &data.default_mace_character(),
                &[
                    vec![
                        record(Stat::MovementSpeed, Op::More, more),
                        record(Stat::MovementSpeed, Op::More, more),
                    ],
                    vec![record(Stat::MovementSpeed, Op::More, more)],
                ],
            );
        }
    }
}
