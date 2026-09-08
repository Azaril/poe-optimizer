use poe_optimizer_data::game_data::*;

fn custom(mut package: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    package.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &package.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}

#[test]
fn receiving_queries_are_required_complete_ordered_source_capabilities() {
    let original = bundled_snapshot().unwrap();
    assert_eq!(original.package().receiving_defence.resources.len(), 3);
    assert_eq!(original.package().receiving_defence.resistances.len(), 4);
    let edits: &[fn(&mut GameDataPackage)] = &[
        |p| p.receiving_defence.resources.clear(),
        |p| p.receiving_defence.resistances.clear(),
        |p| p.receiving_defence.resources.swap(0, 1),
        |p| p.receiving_defence.resources[0].query_stats.swap(0, 1),
        |p| {
            p.receiving_defence.resources[0]
                .query_stats
                .push(ActorStat::Life)
        },
        |p| p.receiving_defence.resources[2].stat = ActorStat::Evasion,
        |p| p.receiving_defence.resistances[0].query_stats.clear(),
        |p| {
            p.receiving_defence.resistances[3]
                .query_stats
                .push(ActorStat::ElementalResist)
        },
        |p| p.receiving_defence.resistances[1].query_stats[1] = ActorStat::ColdResist,
    ];
    for (index, edit) in edits.iter().enumerate() {
        let mut package = original.package().clone();
        edit(&mut package);
        assert!(custom(package).is_err(), "query mutation {index}");
    }
    let mut raw = serde_json::to_value(original.package()).unwrap();
    raw.as_object_mut().unwrap().remove("receiving_defence");
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&raw).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}

#[test]
fn receiving_ir_retains_global_marker_conditions_and_signed_values_without_broadening_scope() {
    let record = ActorModifierRecord {
        stat: ActorStat::Armour,
        effect: ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Base,
            value: -0.75,
        },
        source: Some("Item:1:Fixture".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![
            ActorModifierTag::Global,
            ActorModifierTag::Condition {
                variables: vec![ActorCondition::StrHigherThanInt],
                negated: false,
            },
        ],
    };
    for stat in [
        ActorStat::Armour,
        ActorStat::Evasion,
        ActorStat::EnergyShield,
        ActorStat::ArmourAndEvasion,
        ActorStat::FireResist,
        ActorStat::ColdResist,
        ActorStat::LightningResist,
        ActorStat::ChaosResist,
        ActorStat::ElementalResist,
    ] {
        for operation in [
            ActorNumericOperation::Base,
            ActorNumericOperation::Increased,
        ] {
            for value in [-1e6, -0.75, 0.0, 1e6] {
                let mut edited = record.clone();
                edited.stat = stat;
                edited.effect = ActorModifierEffect::Numeric { operation, value };
                edited.validate().unwrap();
                let decoded: ActorModifierRecord =
                    serde_json::from_slice(&serde_json::to_vec(&edited).unwrap()).unwrap();
                assert_eq!(decoded, edited);
            }
        }
    }
    for (stat, operation) in [
        (ActorStat::Defences, ActorNumericOperation::Base),
        (ActorStat::Armour, ActorNumericOperation::More),
        (ActorStat::EnergyShield, ActorNumericOperation::Override),
        (ActorStat::FireResist, ActorNumericOperation::More),
        (ActorStat::Life, ActorNumericOperation::Base),
    ] {
        let mut edited = record.clone();
        edited.stat = stat;
        edited.effect = ActorModifierEffect::Numeric {
            operation,
            value: 10.0,
        };
        assert!(edited.validate().is_err(), "{stat:?} {operation:?}");
    }
    for value in [f64::NAN, f64::INFINITY, -1_000_001.0, 1_000_001.0] {
        let mut edited = record.clone();
        edited.effect = ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Base,
            value,
        };
        assert!(edited.validate().is_err());
    }
    let mut edited = record.clone();
    edited.flags = 1;
    assert!(edited.validate().is_err());
    edited.flags = 0;
    edited.keyword_flags = 1;
    assert!(edited.validate().is_err());
    let mut raw = serde_json::to_value(record).unwrap();
    raw["tags"][0]["actor"] = "enemy".into();
    assert!(serde_json::from_value::<ActorModifierRecord>(raw).is_err());
}

#[test]
fn receiving_rules_and_numeric_values_are_injected_but_unsupported_operations_reject() {
    let original = bundled_snapshot().unwrap();
    let index = original
        .package()
        .actor
        .modifier_rules
        .iter()
        .position(|rule| rule.id == "global_armour_increased")
        .unwrap();
    let mut package = original.package().clone();
    let rule = &mut package.actor.modifier_rules[index];
    assert_eq!(rule.modifiers[0].tags, [ActorModifierTag::Global]);
    rule.template = "{0}% authored Global Armour".into();
    rule.modifiers[0].effect = ActorRuleEffect::Numeric {
        operation: ActorNumericOperation::Increased,
        value: ActorRuleValue::Capture {
            index: 0,
            multiplier: -1.25,
        },
    };
    let changed = custom(package).unwrap();
    assert_ne!(original.identity(), changed.identity());
    assert_eq!(changed.trust(), &DataTrust::CustomUnreviewed);
    for (stat, operation) in [
        (ActorStat::Defences, ActorNumericOperation::Base),
        (ActorStat::Armour, ActorNumericOperation::More),
        (ActorStat::FireResist, ActorNumericOperation::Override),
        (ActorStat::Life, ActorNumericOperation::Increased),
    ] {
        let mut package = original.package().clone();
        let mapping = &mut package.actor.modifier_rules[index].modifiers[0];
        mapping.stat = stat;
        mapping.effect = ActorRuleEffect::Numeric {
            operation,
            value: ActorRuleValue::Capture {
                index: 0,
                multiplier: 1.0,
            },
        };
        assert!(custom(package).is_err(), "{stat:?} {operation:?}");
    }
    let data = original.package();
    assert_eq!(
        data.actor
            .modifier_rule("all_resistances_base")
            .unwrap()
            .modifiers
            .iter()
            .map(|m| m.stat)
            .collect::<Vec<_>>(),
        [ActorStat::ElementalResist, ActorStat::ChaosResist]
    );
    assert_eq!(
        data.actor
            .modifier_rule("energy_shield_maximum_increased")
            .unwrap()
            .modifiers[0]
            .tags,
        [ActorModifierTag::Global]
    );
    assert!(
        data.actor
            .modifier_rule("energy_shield_maximum_base")
            .unwrap()
            .modifiers[0]
            .tags
            .is_empty()
    );
}
