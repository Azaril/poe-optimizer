use poe_optimizer_data::game_data::*;
fn custom(mut p: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    p.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &p.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}
#[test]
fn body_penalties_preserve_absence_zero_and_injected_values_in_any_armour_slot() {
    let original = bundled_snapshot().unwrap();
    let p = original.package();
    assert_eq!(p.manifest.schema_version, 10);
    assert_eq!(
        p.armour_bases
            .iter()
            .filter(|b| b.slot == EquipmentSlot::BodyArmour)
            .count(),
        114
    );
    assert_eq!(
        p.armour_base("rusted_cuirass").unwrap().movement_penalty,
        Some(0.05)
    );
    assert_eq!(
        p.armour_base("rusted_greathelm").unwrap().movement_penalty,
        None
    );
    for value in [Some(0.0), Some(0.123), None] {
        let mut p = p.clone();
        p.armour_bases
            .iter_mut()
            .find(|b| b.id == "rusted_greathelm")
            .unwrap()
            .movement_penalty = value;
        let changed = custom(p).unwrap();
        assert_eq!(
            changed
                .package()
                .armour_base("rusted_greathelm")
                .unwrap()
                .movement_penalty,
            value
        );
        if value.is_some() {
            assert_ne!(changed.identity(), original.identity());
        }
    }
    let mut raw = serde_json::to_value(p).unwrap();
    raw["armour_bases"][0]
        .as_object_mut()
        .unwrap()
        .remove("movement_penalty");
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&raw).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}
#[test]
fn movement_formula_values_are_injected_but_action_speed_and_query_scope_remain_closed() {
    let original = bundled_snapshot().unwrap();
    let mut p = original.package().clone();
    p.movement.base_multiplier = 1.125;
    p.movement.minimum_multiplier = 0.75;
    p.movement.rounding_precision = 5;
    p.movement.penalty_modifier.effect = ActorRuleEffect::Numeric {
        operation: ActorNumericOperation::Base,
        value: ActorRuleValue::Capture {
            index: 0,
            multiplier: -2.0,
        },
    };
    let changed = custom(p).unwrap();
    assert_ne!(changed.identity(), original.identity());
    let edits: &[fn(&mut GameDataPackage)] = &[
        |p| p.movement.default_action_speed_multiplier = 1.01,
        |p| p.movement.rounding_precision = 13,
        |p| p.movement.base_multiplier = f64::NAN,
        |p| p.movement.minimum_multiplier = -1.0,
        |p| p.movement.query_stats = vec![ActorStat::Accuracy],
        |p| p.movement.penalty_modifier.flags = 1,
        |p| p.movement.penalty_modifier.tags.clear(),
        |p| p.movement.penalty_modifier.stat = ActorStat::Armour,
        |p| {
            p.movement.penalty_modifier.effect = ActorRuleEffect::Numeric {
                operation: ActorNumericOperation::More,
                value: ActorRuleValue::Capture {
                    index: 0,
                    multiplier: -1.0,
                },
            }
        },
        |p| p.armour_bases[0].movement_penalty = Some(-0.1),
        |p| {
            p.armour_bases[0].movement_penalty = Some(1e6);
            p.movement.penalty_modifier.effect = ActorRuleEffect::Numeric {
                operation: ActorNumericOperation::Base,
                value: ActorRuleValue::Capture {
                    index: 0,
                    multiplier: -2.0,
                },
            };
        },
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut p = original.package().clone();
        edit(&mut p);
        assert!(custom(p).is_err(), "edit{i}");
    }
    let mut raw = serde_json::to_value(original.package()).unwrap();
    raw.as_object_mut().unwrap().remove("movement");
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&raw).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}
#[test]
fn movement_dynamic_condition_cannot_cycle_or_feed_back_into_attributes_or_resources() {
    let mut record = ActorModifierRecord {
        stat: ActorStat::MovementSpeed,
        effect: ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Base,
            value: -0.05,
        },
        source: Some("Item:1:Body".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![ActorModifierTag::Condition {
            variables: vec![ActorCondition::IgnoreMovementPenalties],
            negated: true,
        }],
    };
    record.validate().unwrap();
    for stat in [
        ActorStat::Str,
        ActorStat::Life,
        ActorStat::Armour,
        ActorStat::IgnoreMovementPenalties,
        ActorStat::MovementSpeedCannotBeBelowBase,
    ] {
        record.stat = stat;
        if stat.is_flag() {
            record.effect = ActorModifierEffect::Flag { value: true };
        }
        assert!(record.validate().is_err(), "{stat:?}");
    }
    assert_eq!(
        ActorStat::IgnoreMovementPenalties.upstream_name(),
        "Condition:IgnoreMovementPenalties"
    );
    assert_eq!(
        ActorCondition::IgnoreMovementPenalties.upstream_name(),
        "IgnoreMovementPenalties"
    );
}
#[test]
fn source_division_is_explicit_bounded_and_grammar_injected() {
    let original = bundled_snapshot().unwrap();
    let rule = original
        .package()
        .actor
        .modifier_rule("movement_speed_override")
        .unwrap();
    assert!(matches!(
        rule.modifiers[0].effect,
        ActorRuleEffect::Numeric {
            value: ActorRuleValue::CaptureDivided {
                index: 0,
                divisor: 100.0
            },
            ..
        }
    ));
    for divisor in [0.0, -1.0, f64::NAN, 1_000_001.0] {
        let mut p = original.package().clone();
        p.actor
            .modifier_rules
            .iter_mut()
            .find(|r| r.id == "movement_speed_override")
            .unwrap()
            .modifiers[0]
            .effect = ActorRuleEffect::Numeric {
            operation: ActorNumericOperation::Override,
            value: ActorRuleValue::CaptureDivided { index: 0, divisor },
        };
        assert!(custom(p).is_err());
    }
    let mut p = original.package().clone();
    p.actor
        .modifier_rules
        .iter_mut()
        .find(|r| r.id == "movement_speed_override")
        .unwrap()
        .modifiers[0]
        .effect = ActorRuleEffect::Numeric {
        operation: ActorNumericOperation::Override,
        value: ActorRuleValue::CaptureDivided {
            index: 0,
            divisor: 200.0,
        },
    };
    assert_ne!(custom(p).unwrap().identity(), original.identity());
}
