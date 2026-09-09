use poe_optimizer_data::game_data::*;
fn custom(mut p: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    p.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &p.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}
fn record(stat: ActorStat, operation: ActorNumericOperation) -> ActorModifierRecord {
    ActorModifierRecord {
        stat,
        effect: ActorModifierEffect::Numeric {
            operation,
            value: 17.0,
        },
        source: None,
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
#[test]
fn action_operation_domains_and_global_effect_metadata_remain_fail_closed() {
    use ActorNumericOperation::*;
    for stat in [
        ActorStat::ActionSpeed,
        ActorStat::TemporalChainsActionSpeed,
        ActorStat::MinimumActionSpeed,
        ActorStat::MaximumActionSpeedReduction,
    ] {
        for op in [Base, Increased, More, Override, Max] {
            let expected = if matches!(
                stat,
                ActorStat::ActionSpeed | ActorStat::TemporalChainsActionSpeed
            ) {
                op == Increased
            } else {
                op == Max
            };
            assert_eq!(
                record(stat, op).validate().is_ok(),
                expected,
                "{stat:?}/{op:?}"
            );
        }
    }
    for stat in [
        ActorStat::Str,
        ActorStat::Life,
        ActorStat::Accuracy,
        ActorStat::MovementSpeed,
    ] {
        assert!(record(stat, Max).validate().is_err());
    }
    let mut r = record(ActorStat::MinimumActionSpeed, Max);
    r.tags.push(ActorModifierTag::GlobalEffect {
        effect_type: ActorGlobalEffectType::Global,
        unscalable: true,
    });
    r.validate().unwrap();
    let raw = serde_json::to_value(&r).unwrap();
    for (field, value) in [
        ("effect_type", serde_json::json!("Aura")),
        ("unscalable", serde_json::json!(false)),
        ("unknown", serde_json::json!(1)),
    ] {
        let mut value_raw = raw.clone();
        value_raw["tags"][0][field] = value;
        let parsed = serde_json::from_value::<ActorModifierRecord>(value_raw);
        assert!(parsed.is_err() || parsed.unwrap().validate().is_err());
    }
    for field in ["effect_type", "unscalable"] {
        let mut v = raw.clone();
        v["tags"][0].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ActorModifierRecord>(v).is_err());
    }
    r.stat = ActorStat::MaximumActionSpeedReduction;
    assert!(r.validate().is_err());
    r.stat = ActorStat::MinimumActionSpeed;
    r.tags.push(ActorModifierTag::Condition {
        variables: vec![ActorCondition::IgnoreMovementPenalties],
        negated: false,
    });
    assert!(r.validate().is_err());
}
#[test]
fn action_and_timing_data_are_injected_with_semantic_capability_guards() {
    let snapshot = bundled_snapshot().unwrap();
    let package = snapshot.package();
    assert_eq!(package.manifest.schema_version, 24);
    assert_eq!(package.action_speed.query_stats.len(), 5);
    let mut p = package.clone();
    p.action_speed.base_multiplier = 1.125;
    p.action_speed.default_minimum_percent = 25.0;
    p.action_speed.temporal_chains_effect_cap = 50.0;
    p.action_speed.percent_divisor = 80.0;
    p.direct_action_timing.server_tick_rate = 60.0;
    p.direct_action_timing.speed_multiplier_rounding_precision = 4;
    assert_ne!(custom(p).unwrap().identity(), snapshot.identity());
    let edits: &[fn(&mut GameDataPackage)] = &[
        |p| p.action_speed.base_multiplier = f64::NAN,
        |p| p.action_speed.percent_divisor = 0.0,
        |p| p.action_speed.default_minimum_percent = -1.0,
        |p| p.action_speed.temporal_chains_effect_cap = f64::INFINITY,
        |p| p.action_speed.query_stats.swap(0, 1),
        |p| p.action_speed.query_stats.push(ActorStat::Str),
        |p| p.direct_action_timing.server_tick_rate = 0.0,
        |p| p.direct_action_timing.speed_multiplier_rounding_precision = 13,
        |p| p.direct_action_timing.default_repeats = 2,
        |p| p.actor.modifier_rules[0].modifiers[0].stat = ActorStat::TemporalChainsActionSpeed,
        |p| {
            let mapping = &mut p.actor.modifier_rules[0].modifiers[0];
            mapping.stat = ActorStat::MaximumActionSpeedReduction;
            mapping.effect = ActorRuleEffect::Numeric {
                operation: ActorNumericOperation::Max,
                value: ActorRuleValue::Capture {
                    index: 0,
                    multiplier: 1.0,
                },
            };
        },
        |p| {
            p.passive_effects
                .iter_mut()
                .find(|p| !p.actor_modifiers.is_empty())
                .unwrap()
                .actor_modifiers
                .push(record(
                    ActorStat::TemporalChainsActionSpeed,
                    ActorNumericOperation::Increased,
                ))
        },
    ];
    for edit in edits {
        let mut p = package.clone();
        edit(&mut p);
        assert!(custom(p).is_err());
    }
    let raw = serde_json::to_value(package).unwrap();
    for key in ["action_speed", "direct_action_timing"] {
        let mut v = raw.clone();
        v.as_object_mut().unwrap().remove(key);
        assert!(
            GameDataPackage::decode_for_authoring(
                &serde_json::to_vec(&v).unwrap(),
                &LoadLimits::default()
            )
            .is_err()
        );
    }
    let mut v = raw;
    v["direct_action_timing"]["eligibility"] = serde_json::json!("triggered");
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&v).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}

#[test]
fn finite_small_decimal_survives_authenticated_package_json_transport() {
    let mut package = bundled_snapshot().unwrap().package().clone();
    package.action_speed.percent_divisor = 1.2e-307;
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let loaded =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    assert_eq!(
        loaded.package().action_speed.percent_divisor.to_bits(),
        package.action_speed.percent_divisor.to_bits()
    );
    assert_eq!(loaded.package().canonical_bytes().unwrap(), bytes);
}
