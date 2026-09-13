use poe_optimizer_data::game_data::*;
use serde_json::json;
fn custom(mut package: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    package.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &package.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}
#[test]
fn actor_schema_is_required_and_all_source_records_are_retained() {
    let snapshot = bundled_snapshot().unwrap();
    let package = snapshot.package();
    assert_eq!(snapshot.identity().schema_version, 31);
    assert_eq!(package.actor.high_precision_mods.len(), 40);
    assert_eq!(package.actor.modifier_rules.len(), 359);
    assert_eq!(package.actor.spirit_quests.len(), 3);
    let more: Vec<_> = package
        .actor
        .high_precision_mods
        .iter()
        .filter_map(|(name, operations)| {
            operations
                .get(&ActorNumericOperation::More)
                .map(|v| (name.as_str(), *v))
        })
        .collect();
    assert_eq!(
        more,
        vec![("ReservationMultiplier", 4), ("SupportManaMultiplier", 4)]
    );
    for quest in &package.actor.spirit_quests {
        assert!(quest.default_enabled);
        quest.modifiers[0].validate().unwrap();
    }
    let mut old = package.clone();
    old.manifest.schema_version = 5;
    assert!(custom(old).is_err());
    for removed in [
        "actor",
        "high_precision_mods",
        "modifier_rules",
        "spirit_quests",
        "low_life_threshold",
    ] {
        let mut raw = serde_json::to_value(package).unwrap();
        if removed == "actor" {
            raw.as_object_mut().unwrap().remove(removed);
        } else {
            raw["actor"].as_object_mut().unwrap().remove(removed);
        }
        assert!(
            GameDataPackage::decode_for_authoring(
                &serde_json::to_vec(&raw).unwrap(),
                &LoadLimits::default()
            )
            .is_err(),
            "{removed}"
        );
    }
}
#[test]
fn actor_constants_precision_and_quests_are_injected_and_content_bound() {
    let original = bundled_snapshot().unwrap();
    let mut p = original.package().clone();
    p.actor.minimum_spirit = 0.0;
    p.actor.low_life_threshold = 0.25;
    p.actor.doubled_attribute_bonus_multiplier = 3.0;
    p.actor
        .high_precision_mods
        .entry("Life".into())
        .or_default()
        .insert(ActorNumericOperation::More, 5);
    p.actor.spirit_quests[0].config_key = "custom_spirit_reward".into();
    p.actor.spirit_quests[0].default_enabled = false;
    let previous_rule_id = p.actor.modifier_rules[0].id.clone();
    for base in &mut p.jewellery_bases {
        if base.implicit.actor_rule_id == previous_rule_id {
            base.implicit.actor_rule_id = "custom_strength".into();
        }
    }
    p.actor.modifier_rules[0].id = "custom_strength".into();
    p.actor.modifier_rules[0].template = "{0} to Authored Strength".into();
    let edited = custom(p).unwrap();
    assert_ne!(edited.identity(), original.identity());
    assert_eq!(edited.trust(), &DataTrust::CustomUnreviewed);
    assert!(
        edited
            .package()
            .actor
            .modifier_rule("custom_strength")
            .is_some()
    );
    assert_eq!(edited.package().character, original.package().character);
}
#[test]
fn actor_constants_precision_and_quest_shape_reject_invalid_records() {
    type Edit = fn(&mut GameDataPackage);
    let edits: &[Edit] = &[
        |p| p.actor.low_life_threshold = -0.1,
        |p| p.actor.full_life_threshold = 1.1,
        |p| p.actor.minimum_spirit = -1.0,
        |p| p.actor.initial_spirit = f64::INFINITY,
        |p| p.actor.halved_life_per_strength = 1_000_001.0,
        |p| {
            p.actor
                .high_precision_mods
                .insert("Life".into(), Default::default());
        },
        |p| {
            p.actor
                .high_precision_mods
                .entry("Bad Name".into())
                .or_default()
                .insert(ActorNumericOperation::More, 2);
        },
        |p| {
            p.actor
                .high_precision_mods
                .entry("Life".into())
                .or_default()
                .insert(ActorNumericOperation::More, 16);
        },
        |p| {
            p.actor.spirit_quests.pop();
        },
        |p| p.actor.spirit_quests[0].config_key = p.actor.spirit_quests[1].config_key.clone(),
        |p| p.actor.spirit_quests[0].config_key = p.quests.config_keys[0].clone(),
        |p| p.actor.spirit_quests[0].modifiers[0].stat = ActorStat::Mana,
        |p| {
            p.actor.spirit_quests[0].modifiers[0].effect = ActorModifierEffect::Numeric {
                operation: ActorNumericOperation::More,
                value: 30.0,
            }
        },
        |p| {
            p.actor.spirit_quests[0].modifiers[0].effect = ActorModifierEffect::Numeric {
                operation: ActorNumericOperation::Base,
                value: -1.0,
            }
        },
        |p| p.actor.spirit_quests[0].modifiers[0].source = Some("bad\nsource".into()),
    ];
    for (index, edit) in edits.iter().enumerate() {
        let mut p = bundled_snapshot().unwrap().package().clone();
        edit(&mut p);
        assert!(custom(p).is_err(), "edit {index}");
    }
}
#[test]
fn actor_grammar_mapping_and_scope_fail_closed() {
    type Edit = fn(&mut GameDataPackage);
    let edits: &[Edit] = &[
        |p| p.actor.modifier_rules.clear(),
        |p| p.actor.modifier_rules[0].id = p.actor.modifier_rules[1].id.clone(),
        |p| p.actor.modifier_rules[0].template = p.actor.modifier_rules[1].template.clone(),
        |p| p.actor.modifier_rules[0].template = "{1} to Strength".into(),
        |p| p.actor.modifier_rules[0].template = "{0} to Strength {0}".into(),
        |p| p.actor.modifier_rules[0].captures.clear(),
        |p| p.actor.modifier_rules[0].modifiers[0].flags = 1,
        |p| p.actor.modifier_rules[0].modifiers[0].keyword_flags = 1,
        |p| p.actor.modifier_rules[0].modifiers[0].stat = ActorStat::NoAttributeBonuses,
        |p| p.actor.modifier_rules[0].modifiers[0].effect = ActorRuleEffect::Flag { value: true },
        |p| {
            p.actor.modifier_rules[0].modifiers[0].effect = ActorRuleEffect::Numeric {
                operation: ActorNumericOperation::Base,
                value: ActorRuleValue::Capture {
                    index: 1,
                    multiplier: 1.0,
                },
            }
        },
        |p| {
            p.actor.modifier_rules[0].modifiers[0].effect = ActorRuleEffect::Numeric {
                operation: ActorNumericOperation::Base,
                value: ActorRuleValue::Capture {
                    index: 0,
                    multiplier: 0.0,
                },
            }
        },
        |p| {
            p.actor.modifier_rules[0].modifiers[0].effect = ActorRuleEffect::Numeric {
                operation: ActorNumericOperation::More,
                value: ActorRuleValue::Constant { value: 0.0 },
            }
        },
        |p| {
            p.actor.modifier_rules[0].modifiers[0].tags = vec![ActorModifierTag::Condition {
                variables: vec![],
                negated: false,
            }]
        },
    ];
    for (index, edit) in edits.iter().enumerate() {
        let mut p = bundled_snapshot().unwrap().package().clone();
        edit(&mut p);
        assert!(custom(p).is_err(), "edit {index}");
    }
    let original = bundled_snapshot().unwrap();
    for (field, value) in [
        ("stat", json!("ward")),
        ("unknown", json!(0)),
        (
            "tags",
            json!([{"type":"condition","variables":["str_single_highest_attribute"],"negated":false}]),
        ),
        (
            "tags",
            json!([{"type":"condition","variables":["str_highest_attribute"],"negated":false,"actor":"enemy"}]),
        ),
        ("tags", json!([{"type":"per_stat","stat":"str"}])),
    ] {
        let mut raw = serde_json::to_value(original.package()).unwrap();
        raw["actor"]["modifier_rules"][0]["modifiers"][0][field] = value;
        assert!(
            GameDataPackage::decode_for_authoring(
                &serde_json::to_vec(&raw).unwrap(),
                &LoadLimits::default()
            )
            .is_err(),
            "{field}"
        );
    }
}
#[test]
fn actor_ir_validates_complete_stage_targets_and_retains_ordered_conditions() {
    let mut record = ActorModifierRecord {
        stat: ActorStat::Life,
        effect: ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Override,
            value: 0.0,
        },
        source: Some("Custom:Fixture".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![ActorModifierTag::Condition {
            variables: vec![
                ActorCondition::DexHigherThanInt,
                ActorCondition::StrHigherThanInt,
            ],
            negated: true,
        }],
    };
    record.validate().unwrap();
    assert_eq!(record.stat.upstream_name(), "Life");
    assert_eq!(
        ActorCondition::TwoHighestAttributesEqual.upstream_name(),
        "TwoHighestAttributesEqual"
    );
    record.stat = ActorStat::LifeConvertToEnergyShield;
    assert!(record.validate().is_err());
    record.effect = ActorModifierEffect::Numeric {
        operation: ActorNumericOperation::Base,
        value: 100.0,
    };
    record.validate().unwrap();
    record.stat = ActorStat::ChaosInoculation;
    record.effect = ActorModifierEffect::Flag { value: false };
    record.validate().unwrap();
    record.flags = 1;
    assert!(record.validate().is_err());
    record.flags = 0;
    record.stat = ActorStat::Life;
    record.effect = ActorModifierEffect::Numeric {
        operation: ActorNumericOperation::Base,
        value: f64::NAN,
    };
    assert!(record.validate().is_err());
}

#[test]
fn actor_and_local_weapon_templates_reject_ascii_casefold_duplicates() {
    let original = bundled_snapshot().unwrap();
    let mut p = original.package().clone();
    let mut rule = p.actor.modifier_rules[0].clone();
    rule.id = "case_duplicate_actor".into();
    rule.template = rule.template.to_ascii_uppercase();
    p.actor.modifier_rules.push(rule);
    assert!(custom(p).is_err());
    let mut p = original.package().clone();
    let mut rule = p.item_modifier_rules[0].clone();
    rule.id = "case_duplicate_local".into();
    rule.template = rule.template.to_ascii_uppercase();
    p.item_modifier_rules.push(rule);
    assert!(custom(p).is_err());
    let mut p = original.package().clone();
    p.actor.modifier_rules[0].template = p.actor.modifier_rules[0].template.to_ascii_uppercase();
    p.item_modifier_rules[0].template = p.item_modifier_rules[0].template.to_ascii_uppercase();
    assert!(custom(p).is_ok());
    // The distinct pre-parser ItemTools key lookup remains case-sensitive.
    let mut p = original.package().clone();
    let mut key = p.item_formatting.rules[0].clone();
    key.template = key.template.to_ascii_uppercase();
    p.item_formatting.rules.push(key);
    assert!(custom(p).is_ok());
}
