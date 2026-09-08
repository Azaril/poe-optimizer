use super::*;
use poe_optimizer_data::game_data::{self, ActorModifierMapping, ActorNumericOperation};
use roxmltree::Document;
fn data() -> GameDataPackage {
    game_data::bundled_snapshot().unwrap().package().clone()
}
fn configured() -> GameDataPackage {
    let mut data = data();
    data.actor.modifier_rules = vec![
        rule(
            "str_base",
            "{0} to Strength",
            ActorCaptureKind::SignedDecimal,
            ActorStat::Str,
            ActorNumericOperation::Base,
        ),
        rule(
            "str_inc",
            "{0}% increased Strength",
            ActorCaptureKind::UnsignedInteger,
            ActorStat::Str,
            ActorNumericOperation::Increased,
        ),
        rule(
            "mana_more",
            "{0}% more maximum Mana",
            ActorCaptureKind::UnsignedInteger,
            ActorStat::Mana,
            ActorNumericOperation::More,
        ),
    ];
    data
}
fn rule(
    id: &str,
    template: &str,
    capture: ActorCaptureKind,
    stat: ActorStat,
    operation: ActorNumericOperation,
) -> ActorModifierRule {
    ActorModifierRule {
        id: id.into(),
        template: template.into(),
        captures: vec![capture],
        modifiers: vec![ActorModifierMapping {
            stat,
            effect: ActorRuleEffect::Numeric {
                operation,
                value: ActorRuleValue::Capture {
                    index: 0,
                    multiplier: 1.0,
                },
            },
            flags: 0,
            keyword_flags: 0,
            tags: vec![],
        }],
    }
}
fn config(text: &str, data: &GameDataPackage) -> Result<ValidatedActorModifiers> {
    let doc = Document::parse(text).unwrap();
    parse_actor_configuration(doc.root_element(), data)
}
#[test]
fn configured_capture_signs_exact_templates_order_duplicates_and_removal_are_preserved() {
    let data = configured();
    let source = "\r\n\t+2.5 to Strength \r\n+2.5 to Strength\n20% increased Strength\n15% more maximum Mana\n";
    let parsed = parse_actor_modifier_text(source, &data).unwrap();
    assert_eq!(parsed.records().len(), 4);
    assert_eq!(parsed.records()[0], parsed.records()[1]);
    assert_eq!(parsed.lines()[0].line_number, 2);
    assert_eq!(
        &source[parsed.lines()[0].byte_range.clone()],
        parsed.lines()[0].source
    );
    assert_eq!(parsed.lines()[0].values, vec![2.5]);
    assert_eq!(parsed.blocks()[0].text, source);
    assert!(
        parsed
            .records()
            .iter()
            .all(|r| r.source.as_deref() == Some("Custom:Default"))
    );
    let removed = parse_actor_modifier_text("15% more maximum Mana", &data).unwrap();
    assert_eq!(removed.records(), &parsed.records()[3..]);
    assert_ne!(
        parsed.diagnostic()["source_sha256"],
        removed.diagnostic()["source_sha256"]
    );
    for line in [
        "2.5 to Strength",
        "+2.5 to Strength extra",
        "+2e2 to Strength",
        "NaN to Strength",
        "+1. to Strength",
        "+.5 to Strength",
        "+1000001 to Strength",
        "20.0% increased Strength",
        "-20% increased Strength",
        "20%  increased Strength",
        "+5 to all Attributes",
        "^xFFFFFF+5 to Strength",
        "20% increased Strength while moving",
    ] {
        assert!(
            parse_actor_modifier_text(line, &data).is_err(),
            "accepted {line}"
        );
    }
    assert_eq!(
        parse_actor_modifier_text("-2.5 to Strength", &data)
            .unwrap()
            .lines()[0]
            .values,
        vec![-2.5]
    );
}
#[test]
fn actor_rules_reject_ambiguity_unsupported_downstream_scope_and_partial_capture_values() {
    let mut data = configured();
    let mut duplicate = data.actor.modifier_rules[0].clone();
    duplicate.id = "duplicate".into();
    data.actor.modifier_rules.push(duplicate);
    assert!(parse_actor_modifier_text("+5 to Strength", &data).is_err());
    for stat in [
        ActorStat::LifeConvertToEnergyShield,
        ActorStat::ManaConvertToArmour,
        ActorStat::SpiritConvertToEvasion,
    ] {
        let mut data = configured();
        data.actor.modifier_rules[0].modifiers[0].stat = stat;
        assert!(parse_actor_modifier_text("+5 to Strength", &data).is_err());
    }
    let mut data = configured();
    data.actor.modifier_rules[0].modifiers[0].flags = 1;
    assert!(parse_actor_modifier_text("+5 to Strength", &data).is_err());
    let mut data = configured();
    data.actor.modifier_rules[0].modifiers[0].effect = ActorRuleEffect::Numeric {
        operation: ActorNumericOperation::Base,
        value: ActorRuleValue::Capture {
            index: 1,
            multiplier: 1.0,
        },
    };
    assert!(parse_actor_modifier_text("+5 to Strength", &data).is_err());
}
#[test]
fn literal_config_blocks_and_legacy_attributes_keep_raw_crlf_and_named_entity_source() {
    let data = configured();
    let text = "\r\n+2.5 to Strength\r\n20% increased Strength\r\n";
    for body in [text.to_owned(), format!("<![CDATA[{text}]]>")] {
        let xml = format!(
            "<ConfigSet><Input name=\"enemyLevel\" number=\"60\"/><CustomModifierBlock title=\"A &amp; B\" enabled=\"true\">{body}</CustomModifierBlock></ConfigSet>"
        );
        let parsed = config(&xml, &data).unwrap();
        assert_eq!(parsed.blocks()[0].text, text);
        assert_eq!(parsed.blocks()[0].title, "A & B");
        assert_eq!(parsed.records()[0].source.as_deref(), Some("Custom:A & B"));
        assert_eq!(parsed.diagnostic()["source_format"], "blocks");
    }
    let xml = format!("<ConfigSet><Input name=\"customMods\" string=\"{text}\"/></ConfigSet>");
    let legacy = config(&xml, &data).unwrap();
    assert_eq!(
        legacy.blocks()[0].text,
        text,
        "must read raw XML attribute before XML whitespace normalization"
    );
    assert_eq!(legacy.records().len(), 2);
    assert_eq!(legacy.diagnostic()["source_format"], "legacy_input");
    assert!(legacy.uses_extended_scope());
    let explicit = config("<ConfigSet><CustomModifierBlock/></ConfigSet>", &data).unwrap();
    assert!(explicit.is_empty() && explicit.uses_extended_scope());
    let absent = config("<ConfigSet/>", &data).unwrap();
    assert!(absent.is_empty() && !absent.uses_extended_scope());
}
#[test]
fn disabled_unknown_blocks_remain_exact_evidence_and_normalization_is_ordered_and_strict() {
    let data = configured();
    let xml = "<ConfigSet><CustomModifierBlock title=\"Disabled\" enabled=\"false\">not yet modeled</CustomModifierBlock><CustomModifierBlock title=\"Active\" enabled=\"true\">+3 to Strength</CustomModifierBlock></ConfigSet>";
    let parsed = config(xml, &data).unwrap();
    assert_eq!(parsed.records().len(), 1);
    assert_eq!(parsed.lines()[0].block_index, 1);
    assert_eq!(parsed.blocks()[0].text, "not yet modeled");
    let check = |text: &str| {
        let doc = Document::parse(text).unwrap();
        parsed.validate_reference_blocks(doc.root_element())
    };
    check(xml).unwrap();
    for bad in [
        xml.replace("enabled=\"false\"", "enabled=\"true\""),
        xml.replace("Disabled", "Another"),
        xml.replace("+3 to Strength", "+4 to Strength"),
        xml.replace("not yet modeled", "changed disabled text"),
        xml.replace(
            "</ConfigSet>",
            "<CustomModifierBlock title=\"Default\" enabled=\"true\"/></ConfigSet>",
        ),
    ] {
        assert!(check(&bad).is_err(), "accepted {bad}");
    }
    let doc = Document::parse(
        "<ConfigSet><CustomModifierBlock title=\"Default\" enabled=\"true\"/></ConfigSet>",
    )
    .unwrap();
    config("<ConfigSet/>", &data)
        .unwrap()
        .validate_reference_blocks(doc.root_element())
        .unwrap();
    let legacy = config(
        "<ConfigSet><Input name=\"customMods\" string=\"+3 to Strength\"/></ConfigSet>",
        &data,
    )
    .unwrap();
    let doc=Document::parse("<ConfigSet><CustomModifierBlock title=\"Default\" enabled=\"true\">\n\t+3 to Strength\n</CustomModifierBlock></ConfigSet>").unwrap();
    legacy
        .validate_reference_blocks(doc.root_element())
        .unwrap();
}
#[test]
fn actor_config_rejects_unknown_metadata_split_text_numeric_entities_mixed_sources_and_bounds() {
    let data = configured();
    for body in [
        "<CustomModifierBlock enabled=\"yes\">+3 to Strength</CustomModifierBlock>",
        "<CustomModifierBlock extra=\"x\">+3 to Strength</CustomModifierBlock>",
        "<CustomModifierBlock>+3<!--split--> to Strength</CustomModifierBlock>",
        "<CustomModifierBlock><Future/></CustomModifierBlock>",
        "<CustomModifierBlock>&#43;3 to Strength</CustomModifierBlock>",
        "<CustomModifierBlock><![CDATA[+3<!--comment--> to Strength]]></CustomModifierBlock>",
        "<Input name=\"customMods\" number=\"3\"/>",
        "<Input name=\"customMods\" string=\"+3 to Strength\"/><Input name=\"customMods\" string=\"+3 to Strength\"/>",
        "<Input name=\"customMods\" string=\"+3 to Strength\"/><CustomModifierBlock/>",
    ] {
        assert!(
            config(&format!("<ConfigSet>{body}</ConfigSet>"), &data).is_err(),
            "accepted {body}"
        );
    }
    for count in [64, 65] {
        assert_eq!(
            parse_actor_modifier_text(&vec!["+1 to Strength"; count].join("\n"), &data).is_ok(),
            count == 64
        );
    }
    assert!(parse_actor_modifier_text(&" ".repeat(MAX_ACTOR_MODIFIER_BYTES + 1), &data).is_err());
    assert!(parse_actor_modifier_text(&" ".repeat(MAX_LINE_BYTES + 1), &data).is_err());
    assert!(
        config(
            &format!(
                "<ConfigSet>{}</ConfigSet>",
                "<CustomModifierBlock/>".repeat(17)
            ),
            &data
        )
        .is_err()
    );
}

#[test]
fn reviewed_forms_produce_full_signed_numeric_condition_and_fixed_flag_records() {
    let data = data();
    let parsed=parse_actor_modifier_text("+5 to Strength\n20% increased Strength\n+10 to maximum Life\n15% more maximum Mana\n+30 to Spirit\n+4 to Dexterity if Strength is higher than Intelligence\nRemoves all mana\nGain no inherent bonuses from attributes",&data).unwrap();
    assert_eq!(parsed.records().len(), 8);
    assert_eq!(
        parsed.records()[0].effect,
        ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Base,
            value: 5.0
        }
    );
    assert_eq!(parsed.records()[5].tags.len(), 1);
    assert_eq!(
        parsed.records()[6].effect,
        ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Override,
            value: 0.0
        }
    );
    assert_eq!(parsed.records()[7].stat, ActorStat::NoAttributeBonuses);
    assert_eq!(
        parsed.records()[7].effect,
        ActorModifierEffect::Flag { value: true }
    );
}

#[test]
fn maximum_admitted_source_and_normalized_record_evidence_fit_the_media_bound() {
    use poe_optimizer_data::game_data::{ActorCondition, ActorModifierTag};
    let reviewed = data();
    let source =
        vec!["100% increased Strength if Strength is higher than Intelligence"; 64].join("\n");
    let reviewed_bytes = parse_actor_modifier_text(&source, &reviewed)
        .unwrap()
        .diagnostic()
        .to_string()
        .len();
    let mut expanded = configured();
    let mapping = &mut expanded.actor.modifier_rules[0].modifiers[0];
    mapping.tags = vec![
        ActorModifierTag::Condition {
            variables: vec![ActorCondition::TwoHighestAttributesEqual; 12],
            negated: false
        };
        8
    ];
    expanded.actor.modifier_rules[0].modifiers = vec![mapping.clone(); 8];
    let source = vec!["+0 to Strength"; 64].join("\n");
    let parsed = parse_actor_modifier_text(&source, &expanded).unwrap();
    assert_eq!(parsed.records().len(), 512);
    let expanded_bytes = parsed.diagnostic().to_string().len();
    assert!(
        expanded_bytes > 64 * 1024,
        "must exercise the former evidence limit"
    );
    assert!(
        expanded_bytes + 256 * 1024 < super::super::controlled_mace::MAX_NATIVE_MACE_PROFILE_BYTES
    );
    eprintln!(
        "actor evidence bytes: reviewed64lines={reviewed_bytes}, configured512records={expanded_bytes}"
    );
}

#[test]
fn encoded_actor_source_bounds_legacy_formatting_and_all_block_fragments() {
    let data = data();
    let input = "<Input name=\"customMods\" string=\"+3 to Strength\"/>";
    let padding = MAX_ACTOR_SOURCE_BYTES - input.len();
    for extra in [0, 1] {
        let source = input.replace("/>", &format!("{}/>", " ".repeat(padding + extra)));
        let xml = format!("<ConfigSet>{source}</ConfigSet>");
        let parsed = config(&xml, &data);
        assert_eq!(parsed.is_ok(), extra == 0);
        if let Ok(parsed) = parsed {
            assert_eq!(parsed.diagnostic()["source_fragments"][0], source);
        }
    }
    // Individual block source is bounded independently; several legal blocks
    // must also fit the aggregate evidence budget even when their text is tiny.
    let block = format!(
        "<CustomModifierBlock{}>+3 to Strength</CustomModifierBlock>",
        " ".repeat(24 * 1024)
    );
    assert!(config(&format!("<ConfigSet>{block}</ConfigSet>"), &data).is_ok());
    assert!(
        config(
            &format!("<ConfigSet>{}</ConfigSet>", block.repeat(3)),
            &data
        )
        .is_err()
    );
}

#[test]
fn source_neutral_line_parser_preserves_item_identity_and_rejects_ambiguity() {
    let mut data = configured();
    let parsed = match_actor_modifier_line("+20 to Strength", "Item:7:Amber Amulet", &data)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.rule_id(), "str_base");
    assert_eq!(parsed.values(), &[20.0]);
    assert_eq!(
        parsed.records()[0].source.as_deref(),
        Some("Item:7:Amber Amulet")
    );
    assert!(
        match_actor_modifier_line("unknown", "Node:1", &data)
            .unwrap()
            .is_none()
    );
    assert!(match_actor_modifier_line("+20 to Strength\n", "Node:1", &data).is_err());
    assert!(match_actor_modifier_line("+20 to Strength", "", &data).is_err());
    let mut rule = data.actor.modifier_rules[0].clone();
    rule.id = "ambiguous".into();
    data.actor.modifier_rules.push(rule);
    assert!(match_actor_modifier_line("+20 to Strength", "Node:1", &data).is_err());
}

#[test]
fn receiving_records_preserve_global_markers_pairs_conditions_and_strict_scope() {
    use poe_optimizer_data::game_data::ActorModifierTag;
    let data = data();
    let text = "\r\n\t+11 to Armour\r\n15% increased maximum Energy Shield\r\n+7% to all Resistances\r\n20% increased Armour if Strength is higher than Intelligence\r\n";
    let parsed = parse_actor_modifier_text(text, &data).unwrap();
    assert_eq!(parsed.records().len(), 5);
    assert_eq!(parsed.records()[0].stat, ActorStat::Armour);
    assert_eq!(parsed.records()[1].tags, vec![ActorModifierTag::Global]);
    assert_eq!(parsed.records()[2].stat, ActorStat::ElementalResist);
    assert_eq!(parsed.records()[3].stat, ActorStat::ChaosResist);
    assert!(matches!(
        parsed.records()[4].tags.as_slice(),
        [ActorModifierTag::Condition { .. }]
    ));
    assert_eq!(parsed.blocks()[0].text, text);
    for line in parsed.lines() {
        assert_eq!(&text[line.byte_range.clone()], line.source);
    }
    for bad in [
        "10% more Armour",
        "+5% to maximum Fire Resistance",
        "+5 to Defences",
        "10% increased Armour while on Low Life",
        "Gain 10% of maximum Life as Extra maximum Energy Shield",
    ] {
        assert!(
            parse_actor_modifier_text(bad, &data).is_err(),
            "accepted {bad}"
        );
    }
}

#[test]
fn movement_override_retains_exact_division_and_rejects_dynamic_condition_cycles() {
    let mut data = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    let parsed = match_actor_modifier_line(
        "Your movement speed is 57% of its base value",
        "Custom:Study",
        &data,
    )
    .unwrap()
    .unwrap();
    assert_eq!(parsed.values(), &[57.0]);
    assert_eq!(
        parsed.records()[0].effect,
        ActorModifierEffect::Numeric {
            operation: poe_optimizer_data::game_data::ActorNumericOperation::Override,
            value: 57.0 / 100.0
        }
    );
    let rule = data
        .actor
        .modifier_rules
        .iter_mut()
        .find(|rule| rule.id == "movement_speed_override")
        .unwrap();
    rule.modifiers[0].effect = ActorRuleEffect::Numeric {
        operation: poe_optimizer_data::game_data::ActorNumericOperation::Override,
        value: ActorRuleValue::CaptureDivided {
            index: 0,
            divisor: 7.0,
        },
    };
    let parsed = match_actor_modifier_line(
        "Your movement speed is 57% of its base value",
        "Custom:Study",
        &data,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        parsed.records()[0].effect,
        ActorModifierEffect::Numeric {
            operation: poe_optimizer_data::game_data::ActorNumericOperation::Override,
            value: 57.0 / 7.0
        }
    );
    let rule = data
        .actor
        .modifier_rules
        .iter_mut()
        .find(|rule| rule.id == "movement_speed_ignore_penalties")
        .unwrap();
    rule.modifiers[0].tags = vec![poe_optimizer_data::game_data::ActorModifierTag::Condition {
        variables: vec![poe_optimizer_data::game_data::ActorCondition::IgnoreMovementPenalties],
        negated: false,
    }];
    assert!(
        match_actor_modifier_line(
            "Ignore all movement penalties from armour",
            "Custom:Study",
            &data
        )
        .is_err()
    );
    for line in [
        "20% increased Cooldown Recovery Rate",
        "20% increased Movement Speed while using a Skill",
    ] {
        assert!(
            match_actor_modifier_line(line, "Custom:Study", &data)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn action_speed_rules_preserve_global_effect_metadata_and_reject_unmodeled_producers() {
    use poe_optimizer_data::game_data::{ActorGlobalEffectType, ActorModifierTag};
    let data = data();
    for line in [
        "Action Speed cannot be modified to below base value",
        "You cannot be slowed to below base speed",
        "Cannot be slowed to below base speed",
    ] {
        let parsed = match_actor_modifier_line(line, "Custom:Action", &data)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.records()[0].stat, ActorStat::MinimumActionSpeed);
        assert_eq!(
            parsed.records()[0].effect,
            ActorModifierEffect::Numeric {
                operation: ActorNumericOperation::Max,
                value: 100.0
            }
        );
        assert_eq!(
            parsed.records()[0].tags,
            [ActorModifierTag::GlobalEffect {
                effect_type: ActorGlobalEffectType::Global,
                unscalable: true
            }]
        );
        let equipment = match_equipment_modifier_line(line, "Item:41:Helmet", &data)
            .unwrap()
            .unwrap();
        assert_eq!(equipment.records()[0].tags, parsed.records()[0].tags);
    }
    for (line, stat, value) in [
        (
            "Your Action Speed is at least 0% of base value",
            ActorStat::MinimumActionSpeed,
            0.0,
        ),
        (
            "Action Speed cannot be modified to below 80% base value",
            ActorStat::MinimumActionSpeed,
            80.0,
        ),
        ("20% increased Action Speed", ActorStat::ActionSpeed, 20.0),
        ("20% reduced Action Speed", ActorStat::ActionSpeed, -20.0),
    ] {
        let parsed = match_actor_modifier_line(line, "Custom:Action", &data)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.records()[0].stat, stat);
        let ActorModifierEffect::Numeric { value: actual, .. } = parsed.records()[0].effect else {
            panic!("numeric");
        };
        assert_eq!(actual, value);
    }
    // Public unvalidated authored packages must not bypass whole-source admission.
    for stat in [
        ActorStat::TemporalChainsActionSpeed,
        ActorStat::MaximumActionSpeedReduction,
    ] {
        let mut altered = data.clone();
        altered.actor.modifier_rules = vec![rule(
            "unsupported_producer",
            "{0}% Authored Hidden Producer",
            ActorCaptureKind::UnsignedInteger,
            stat,
            if stat == ActorStat::TemporalChainsActionSpeed {
                ActorNumericOperation::Increased
            } else {
                ActorNumericOperation::Max
            },
        )];
        assert!(
            match_actor_modifier_line("20% Authored Hidden Producer", "Custom:Action", &altered)
                .is_err()
        );
        assert!(
            match_equipment_modifier_line(
                "20% Authored Hidden Producer",
                "Item:41:Helmet",
                &altered
            )
            .is_err()
        );
    }
    for line in [
        "Nearby allies' Action Speed cannot be modified to below base value",
        "Nearby Enemy Monsters' Action Speed is at most 80% of base value",
        "20% more Action Speed",
    ] {
        assert!(
            match_actor_modifier_line(line, "Custom:Action", &data)
                .unwrap()
                .is_none()
        );
    }
}
