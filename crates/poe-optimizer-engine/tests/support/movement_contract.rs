use super::*;
use poe_optimizer_data::game_data::{self, ActorCondition, ActorModifierTag, EquipmentSlot};
use poe_optimizer_engine::{
    actor::{ActorModifierLayer, ActorScratch},
    armour::ArmourSlots,
};
#[test]
fn body_penalties_preserve_source_zero_presence_and_owner_guards() {
    let data = CompiledGameData::bundled().unwrap();
    let base = data
        .snapshot()
        .package()
        .armour_bases
        .iter()
        .find(|b| b.slot == EquipmentSlot::BodyArmour)
        .unwrap();
    let source = "Item:42:Source Body, Body Base";
    let authored = vec![
        numeric(ActorStat::Int, Op::Base, 7.0),
        numeric(ActorStat::MovementSpeed, Op::Increased, 13.0),
    ];
    assert!(data.prepare_armour(&base.id, 0, 1, &authored).is_err());
    let item = data
        .prepare_armour_with_source(&base.id, 13, 60, source, &authored)
        .unwrap();
    assert_eq!(item.source_global_records(), authored);
    assert_eq!(item.generated_global_records().len(), 1);
    assert_eq!(item.global_program().record_count(), authored.len() + 1);
    assert_eq!(
        item.generated_global_records()[0].source.as_deref(),
        Some(source)
    );
    let character = data.default_mace_character();
    let input = spark_input();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    let slots = ArmourSlots {
        body_armour: Some(&item),
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
    assert_eq!(
        actor.values().attributes.intelligence,
        character.attributes.intelligence + 7.0
    );
    assert!(actor.movement().effective_movement_speed_mod < 1.13);
    assert!(!actor.movement().has_override);
    let mut offence = character;
    offence.modifiers.skill_speed_increased = 20.0;
    assert_eq!(
        actor.with_character(&offence).unwrap().movement(),
        actor.movement()
    );
    let foreign =
        CompiledGameData::compile(std::sync::Arc::new(game_data::bundled_snapshot().unwrap()))
            .unwrap();
    assert!(
        foreign
            .prepare_actor_with_armour(60, quests, scenario, &character, &[], slots)
            .is_err()
    );
    assert!(
        data.prepare_actor_with_armour(
            60,
            quests,
            scenario,
            &character,
            &[],
            ArmourSlots {
                helmet: Some(&item),
                ..Default::default()
            }
        )
        .is_err()
    );
    for penalty in [None, Some(0.0), Some(0.12345)] {
        let mut package = data.snapshot().package().clone();
        package
            .armour_bases
            .iter_mut()
            .find(|b| b.id == base.id)
            .unwrap()
            .movement_penalty = penalty;
        package.refresh_section_digests().unwrap();
        let injected = CompiledGameData::compile(std::sync::Arc::new(
            game_data::GameDataLoader::from_bytes(
                &package.canonical_bytes().unwrap(),
                &game_data::TrustPolicy::AllowCustom,
                &game_data::LoadLimits::default(),
            )
            .unwrap(),
        ))
        .unwrap();
        let prepared = injected
            .prepare_armour_with_source(&base.id, 0, 1, source, &[])
            .unwrap();
        assert_eq!(
            prepared.generated_global_records().len(),
            usize::from(penalty.is_some())
        );
        assert_eq!(
            injected.prepare_armour(&base.id, 0, 1, &[]).is_ok(),
            penalty.is_none()
        );
        if let Some(penalty) = penalty {
            let ActorModifierEffect::Numeric { value, .. } =
                prepared.generated_global_records()[0].effect
            else {
                panic!("numeric generated")
            };
            assert_eq!(value.to_bits(), (-penalty).to_bits());
        }
    }
}
#[test]
fn movement_condition_cycles_and_unimplemented_actions_are_not_admitted() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    for mut record in [
        numeric(ActorStat::Str, Op::Base, 1.0),
        numeric(ActorStat::Life, Op::Base, 1.0),
        numeric(ActorStat::Armour, Op::Base, 1.0),
        flag(ActorStat::IgnoreMovementPenalties, true),
        flag(ActorStat::MovementSpeedCannotBeBelowBase, true),
    ] {
        record.tags.push(ActorModifierTag::Condition {
            variables: vec![ActorCondition::IgnoreMovementPenalties],
            negated: false,
        });
        assert!(
            data.compile_actor_modifiers(std::slice::from_ref(&record))
                .is_err()
        );
        assert!(
            data.prepare_actor_resources(60, quests, &character, &[vec![record]])
                .is_err()
        );
    }
    let mut package = data.snapshot().package().clone();
    package.movement.default_action_speed_multiplier = 1.5;
    package.refresh_section_digests().unwrap();
    assert!(
        game_data::GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &game_data::TrustPolicy::AllowCustom,
            &game_data::LoadLimits::default()
        )
        .is_err()
    );
}
#[test]
fn varied_four_slot_movement_and_both_skills_allocate_nothing_after_preparation() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let input = spark_input();
    let mace = mace_input();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    let mut ignore = flag(ActorStat::IgnoreMovementPenalties, true);
    ignore.tags.push(ActorModifierTag::Condition {
        variables: vec![ActorCondition::IntHigherThanDex],
        negated: false,
    });
    let config = data
        .compile_actor_modifiers(&[ignore, numeric(ActorStat::MovementSpeed, Op::More, 13.3333)])
        .unwrap();
    // Actor global order is H/Body/G/B; receiving order is H/G/B/Body.
    let items = [
        EquipmentSlot::Helmet,
        EquipmentSlot::BodyArmour,
        EquipmentSlot::Gloves,
        EquipmentSlot::Boots,
    ]
    .map(|slot| {
        let base = data
            .snapshot()
            .package()
            .armour_bases
            .iter()
            .find(|b| b.slot == slot)
            .unwrap();
        [0, 7, 20].map(|quality| {
            data.prepare_armour_with_source(
                &base.id,
                quality,
                60,
                "Item:42:Changing armour",
                &[
                    numeric(ActorStat::Int, Op::Base, f64::from(quality)),
                    numeric(ActorStat::ArmourAndEvasion, Op::Base, 17.5),
                    numeric(ActorStat::MovementSpeed, Op::Increased, f64::from(quality)),
                ],
            )
            .unwrap()
        })
    });
    let passives = [0.0, 20.0, 80.0].map(|dex| {
        data.compile_actor_modifiers(&[numeric(ActorStat::Dex, Op::Base, dex)])
            .unwrap()
    });
    let weapon = data
        .prepare_mace_weapon(mace.weapon, mace.quality, mace.item_level, &[])
        .unwrap();
    let supports = data.mace_support_loadout(&[]).unwrap();
    let mut scratch = ActorScratch::default();
    let ((checksum, ignored, not_ignored), count) = allocations(|| {
        let mut checksum = 0.0;
        let mut ignored = 0;
        let mut not_ignored = 0;
        for i in 0..4860 {
            let helmet = &items[0][i % 3];
            let body = &items[1][(i / 3) % 3];
            let gloves = &items[2][(i / 9) % 3];
            let boots = &items[3][(i / 27) % 3];
            let programs = [
                &config,
                helmet.global_program(),
                body.global_program(),
                gloves.global_program(),
                boots.global_program(),
                &passives[(i / 81) % 3],
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
                        body_armour: Some(body),
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
            let movement = actor.movement();
            ignored += usize::from(movement.ignore_movement_penalties);
            not_ignored += usize::from(!movement.ignore_movement_penalties);
            assert_eq!(
                spark.effective_movement_speed_mod,
                movement.effective_movement_speed_mod
            );
            assert_eq!(
                mace.effective_movement_speed_mod,
                movement.effective_movement_speed_mod
            );
            checksum +=
                movement.effective_movement_speed_mod + spark.life + mace.hit_dps + spark.armour;
        }
        (checksum, ignored, not_ignored)
    });
    assert_eq!(count, 0);
    assert!(checksum.is_finite() && ignored > 0 && not_ignored > 0);
    assert!(ActorScratch::storage_bytes() <= 1024);
}
#[test]
fn injected_movement_constants_penalty_mapping_and_rounding_change_actual_actor_outputs() {
    let bundled = CompiledGameData::bundled().unwrap();
    let mut package = bundled.snapshot().package().clone();
    let base = package
        .armour_bases
        .iter_mut()
        .find(|base| base.slot == EquipmentSlot::BodyArmour)
        .unwrap();
    base.movement_penalty = Some(0.073456);
    let base_id = base.id.clone();
    package.movement.base_multiplier = 1.15;
    package.movement.minimum_multiplier = 1.3;
    package.movement.rounding_precision = 4;
    let game_data::ActorRuleEffect::Numeric {
        value: game_data::ActorRuleValue::Capture { multiplier, .. },
        ..
    } = &mut package.movement.penalty_modifier.effect
    else {
        panic!("source penalty template")
    };
    *multiplier = -2.0;
    package.refresh_section_digests().unwrap();
    let data = CompiledGameData::compile(std::sync::Arc::new(
        game_data::GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &game_data::TrustPolicy::AllowCustom,
            &game_data::LoadLimits::default(),
        )
        .unwrap(),
    ))
    .unwrap();
    let body = data
        .prepare_armour_with_source(&base_id, 0, 60, "Item:5:Injected body", &[])
        .unwrap();
    let character = data.default_mace_character();
    let input = spark_input();
    let quests = data.actor_quest_selection(input.quests);
    let scenario = data.receiving_scenario(input.quests, input.resistance_penalty);
    for (floor, overridden, expected, unrounded) in [
        (false, None, 1.2037, 1.2037),
        (true, None, 1.3, 1.3),
        (false, Some(1.234567), 1.2346, 1.234567),
    ] {
        let mut records = vec![
            numeric(ActorStat::MovementSpeed, Op::Increased, 20.0),
            flag(ActorStat::MovementSpeedCannotBeBelowBase, floor),
        ];
        if let Some(value) = overridden {
            records.push(numeric(ActorStat::MovementSpeed, Op::Override, value));
        }
        records.extend_from_slice(body.global_records());
        let actor = data
            .prepare_actor_with_armour(
                60,
                quests,
                scenario,
                &character,
                &[records.clone()],
                ArmourSlots {
                    body_armour: Some(&body),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(actor.movement().effective_movement_speed_mod, expected);
        assert_eq!(actor.movement().movement_speed_mod, unrounded);
        assert_eq!(actor.movement().has_override, overridden.is_some());
        assert_eq!(
            spark::evaluate_with_actor(&input, &character, &data, &actor)
                .unwrap()
                .effective_movement_speed_mod,
            expected
        );
        let program = data.compile_actor_modifiers(&records).unwrap();
        let compiled = data
            .evaluate_actor_with_armour(
                60,
                quests,
                scenario,
                &character,
                &[ActorModifierLayer {
                    programs: &[&program],
                }],
                ArmourSlots {
                    body_armour: Some(&body),
                    ..Default::default()
                },
                &mut ActorScratch::default(),
            )
            .unwrap();
        assert_eq!(compiled.movement(), actor.movement());
    }
}
