use super::*;
use poe_optimizer_data::game_data::{self, ActorModifierEffect, ActorNumericOperation, ActorStat};
fn data() -> GameDataPackage {
    game_data::bundled_snapshot().unwrap().package().clone()
}
fn amulet(base: &str, implicit: &str, extra: &str) -> String {
    format!(
        "Rarity: RARE\nTest Pendant\n{base}\nItem Level: 82\nQuality: 0\nImplicits: 1\n{implicit}\n{extra}"
    )
}
#[test]
fn jewellery_implicit_global_records_and_physical_source_identity_are_preserved() {
    let data = data();
    for (base, implicit, stat) in [
        ("Amber Amulet", "+12 to Strength", ActorStat::Str),
        ("Jade Amulet", "+14 to Dexterity", ActorStat::Dex),
        ("Lapis Amulet", "+10 to Intelligence", ActorStat::Int),
        ("Bloodstone Amulet", "+35 to maximum Life", ActorStat::Life),
        ("Solar Amulet", "+15 to Spirit", ActorStat::Spirit),
    ] {
        let text = amulet(
            base,
            implicit,
            "+20 to Accuracy Rating\n+40 to maximum Mana\n",
        );
        let item = parse_equipment_item(&text, &data, 7).unwrap();
        assert_eq!(item.allowed_slots(), &["Amulet"]);
        assert!(item.weapon().is_none());
        assert_eq!(item.actor_modifiers().len(), 3);
        assert_eq!(item.actor_modifiers()[0].stat, stat);
        assert_eq!(item.actor_modifiers()[1].stat, ActorStat::Accuracy);
        assert!(item.actor_modifiers().iter().all(|r| r.tags.is_empty()));
        assert_eq!(
            item.actor_modifiers()[0].source.as_deref(),
            Some(format!("Item:7:Test Pendant, {base}").as_str())
        );
        for line in item.modifier_lines() {
            assert_eq!(&text[line.byte_range.clone()], line.source);
        }
        let duplicate = parse_equipment_item(&text, &data, 8).unwrap();
        assert_eq!(duplicate.source_sha256(), item.source_sha256());
        assert_ne!(
            duplicate.actor_modifiers()[0].source,
            item.actor_modifiers()[0].source
        );
        assert_eq!(
            item.pob_export_lines()[5],
            format!("LevelReq: {}", item.requirements().level)
        );
    }
}
#[test]
fn weapon_local_consumption_and_remaining_global_records_are_disjoint() {
    let data = data();
    let text = "Rarity: RARE\nTest Club\nWooden Club\nItem Level: 1\nQuality: 20\nImplicits: 0\nAdds 2 to 4 Physical Damage\n+20 to Strength\n+30 to Spirit\n";
    let item = parse_equipment_item(text, &data, 12).unwrap();
    let weapon = item.weapon().unwrap();
    assert_eq!(weapon.local_modifiers().len(), 1);
    assert_eq!(weapon.actor_modifiers().len(), 2);
    assert_eq!(item.actor_modifiers().len(), 2);
    assert_eq!(item.actor_modifiers()[0].stat, ActorStat::Str);
    assert_eq!(
        item.actor_modifiers()[0].source.as_deref(),
        Some("Item:12:Test Club, Wooden Club")
    );
    assert_eq!(item.allowed_slots(), &["Weapon 1"]);
    assert!(
        crate::mace_item::parse_mace_item(text, &data).is_err(),
        "legacy API silently expanded"
    );
    assert!(
        parse_equipment_item(
            &text.replace("+20 to Strength", "+20 to Accuracy Rating"),
            &data,
            12
        )
        .unwrap_err()
        .to_string()
        .contains("hand-specific")
    );
}
#[test]
fn implicit_range_metadata_unknown_lines_and_source_transformations_reject() {
    let data = data();
    let good = amulet("Amber Amulet", "+12 to Strength", "+30 to maximum Life");
    for text in [
        good.replace("+12 to Strength", "+9 to Strength"),
        good.replace("+12 to Strength", "+16 to Strength"),
        good.replace("+12 to Strength", "+12 to Dexterity"),
        good.replace("Implicits: 1", "Implicits: 0"),
        good.replace("Implicits: 1", "Implicits: 2"),
        good.replace("Quality: 0", "Quality: 20"),
        good.replace("+30 to maximum Life", "Adds 2 to 4 Physical Damage"),
        good.replace("+30 to maximum Life", "+5 to all Attributes"),
        good.replace("+30 to maximum Life", "Chaos Inoculation"),
        good.replace("Amber Amulet", "Stellar Amulet"),
        good.replace("Item Level: 82", "Item Level: 082"),
    ] {
        assert!(
            parse_equipment_item(&text, &data, 1).is_err(),
            "accepted {text}"
        );
    }
    assert!(parse_equipment_item(&good, &data, 0).is_err());
    let lowered = good.replace("Implicits: 1", "LevelReq: 1\nImplicits: 1");
    assert_eq!(
        parse_equipment_item(&lowered, &data, 1)
            .unwrap()
            .requirements()
            .level,
        1
    );
}
#[test]
fn selected_equipment_data_changes_implicit_rules_effects_and_requirements() {
    let mut data = data();
    let index = data
        .jewellery_bases
        .iter()
        .position(|b| b.name == "Amber Amulet")
        .unwrap();
    data.jewellery_bases[index].implicit.minimum = 20.0;
    data.jewellery_bases[index].implicit.maximum = 25.0;
    data.jewellery_bases[index].requirements.level = 55;
    let id = data.jewellery_bases[index].implicit.actor_rule_id.clone();
    let rule = data
        .actor
        .modifier_rules
        .iter_mut()
        .find(|r| r.id == id)
        .unwrap();
    rule.template = "{0} test Strength".into();
    let text = amulet("Amber Amulet", "+22 test Strength", "");
    let item = parse_equipment_item(&text, &data, 3).unwrap();
    assert_eq!(item.requirements().level, 55);
    assert_eq!(
        item.actor_modifiers()[0].effect,
        ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Base,
            value: 22.0
        }
    );
    assert!(parse_equipment_item(&text, &self::data(), 3).is_err());
    assert!(
        parse_equipment_item(&amulet("Amber Amulet", "+12 to Strength", ""), &data, 3).is_err()
    );
}
#[test]
fn xml_retains_raw_crlf_tabs_named_entities_and_rejects_hidden_item_metadata() {
    let data = data();
    let text =
        amulet("Amber Amulet", "\t+12 to Strength ", "+30 to maximum Life").replace('\n', "\r\n");
    let xml = format!("<Item id=\"4\">{text}</Item>");
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let item = parse_equipment_item_xml(doc.root_element(), &data).unwrap();
    assert_eq!(item.source_text(), text);
    let cdata = format!("<Item id=\"4\"><![CDATA[{text}]]></Item>");
    let doc = roxmltree::Document::parse(&cdata).unwrap();
    assert_eq!(
        parse_equipment_item_xml(doc.root_element(), &data)
            .unwrap()
            .source_text(),
        text
    );
    for xml in [
        xml.replace("id=\"4\"", "id=\"04\""),
        xml.replace("id=\"4\"", "id=\"4\" variant=\"1\""),
        xml.replace("+12", "&#43;12"),
    ] {
        let doc = roxmltree::Document::parse(&xml).unwrap();
        assert!(parse_equipment_item_xml(doc.root_element(), &data).is_err());
    }
}

#[test]
fn lunar_and_pearlescent_source_ranges_global_effects_and_equip_requirements_remain_distinct() {
    use poe_optimizer_data::game_data::ActorModifierTag;
    let data = data();
    for (base, min, max, level, suffix) in [
        ("Lunar Amulet", 20, 30, 14, " to maximum Energy Shield"),
        (
            "Pearlescent Amulet",
            7,
            10,
            30,
            "% to all Elemental Resistances",
        ),
    ] {
        for roll in [min, max] {
            let text = amulet(
                base,
                &format!("+{roll}{suffix}"),
                "15% increased maximum Energy Shield\n+20 to Armour\n",
            )
            .replace('\n', "\r\n");
            let parsed = parse_equipment_item(&text, &data, 73).unwrap();
            assert_eq!(parsed.source_text(), text);
            assert_eq!(parsed.item_level(), 82);
            assert_eq!(parsed.requirements().level, level);
            assert_eq!(parsed.allowed_slots(), &["Amulet"]);
            assert_eq!(
                parsed.actor_modifiers()[1].tags,
                vec![ActorModifierTag::Global]
            );
            assert_eq!(
                parsed.actor_modifiers()[0].source.as_deref(),
                Some(format!("Item:73:Test Pendant, {base}").as_str())
            );
            let explicit = text.replace("Implicits: 1", "LevelReq: 1\r\nImplicits: 1");
            let lowered = parse_equipment_item(&explicit, &data, 73).unwrap();
            assert_eq!(lowered.requirements().level, 1);
            assert_eq!(lowered.item_level(), 82);
        }
        for roll in [min - 1, max + 1] {
            assert!(
                parse_equipment_item(&amulet(base, &format!("+{roll}{suffix}"), ""), &data, 73)
                    .is_err()
            );
        }
    }
    let weapon = "Rarity: RARE\nReceiving Club\nWooden Club\nItem Level: 1\nQuality: 20\nImplicits: 0\nAdds 5 to 10 Physical Damage\n+20 to Armour\n15% increased maximum Energy Shield\n+7% to all Elemental Resistances";
    let parsed = parse_equipment_item(weapon, &data, 11).unwrap();
    assert_eq!(parsed.weapon().unwrap().local_modifiers().len(), 1);
    assert_eq!(parsed.actor_modifiers().len(), 3);
    assert_eq!(
        parsed.actor_modifiers()[1].tags,
        vec![ActorModifierTag::Global]
    );
    assert!(
        parsed
            .actor_modifiers()
            .iter()
            .all(|record| record.source.as_deref() == Some("Item:11:Receiving Club, Wooden Club"))
    );
}

fn armour_item(base: &str, extra: &str) -> String {
    format!(
        "Rarity: RARE\nTrial Armour\n{base}\nItem Level: 82\nQuality: 7\nLevelReq: 3\nImplicits: 0\n{extra}"
    )
}
#[test]
fn armour_source_records_are_partitioned_without_losing_global_tags_or_source_spans() {
    let data = data();
    for base_name in [
        "Rusted Greathelm",
        "Shabby Hood",
        "Twig Circlet",
        "Stocky Mitts",
        "Suede Bracers",
        "Torn Gloves",
        "Rough Greaves",
        "Rawhide Boots",
        "Straw Sandals",
        "Brimmed Helm",
        "Iron Crown",
    ] {
        let text = armour_item(base_name, "+7 to Armour\n11% increased Armour\n+3 to Armour and Energy Shield\n+5 to Global Armour\n13% increased maximum Energy Shield\n+9 to Strength\n23% increased Armour if Strength is higher than Intelligence\n").replace('\n',"\r\n");
        let item = parse_equipment_item(&text, &data, 41).unwrap();
        assert_eq!(item.source_text(), text);
        assert_eq!(item.item_level(), 82);
        assert_eq!(item.quality(), 7);
        assert_eq!(item.explicit_level_requirement(), Some(3));
        let mut requirements = data.armour_base_by_name(base_name).unwrap().requirements;
        requirements.level = 3;
        assert_eq!(item.requirements(), &requirements);
        assert_eq!(item.armour_modifiers().unwrap().len(), 7);
        assert_eq!(item.actor_modifiers().len(), 4);
        assert!(item.weapon().is_none());
        assert_eq!(item.pob_export_lines()[6], "Implicits: 0");
        assert!(
            item.actor_modifiers()
                .iter()
                .all(|r| !poe_optimizer_engine::armour::is_local_modifier(r))
        );
        for line in item.modifier_lines() {
            assert_eq!(&text[line.byte_range.clone()], line.source);
            assert!(!line.implicit);
        }
        let xml = format!("<Item id=\"41\"><![CDATA[{text}]]></Item>");
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let roundtrip = parse_equipment_item_xml(doc.root_element(), &data).unwrap();
        assert_eq!(roundtrip.diagnostic(), item.diagnostic());
        assert_eq!(roundtrip.source_text(), text);
        let duplicate = parse_equipment_item(&text, &data, 42).unwrap();
        assert_eq!(duplicate.source_sha256(), item.source_sha256());
        assert_ne!(
            duplicate.armour_modifiers().unwrap()[0].source,
            item.armour_modifiers().unwrap()[0].source
        );
    }
}
#[test]
fn armour_rejects_incomplete_local_semantics_and_unmodeled_item_properties() {
    let data = data();
    let valid = armour_item("Rusted Greathelm", "+7 to Armour");
    for text in [
        valid.replace("Rarity: RARE", "Rarity: UNIQUE"),
        valid.replace("Rusted Greathelm", "Rotted Round Shield"),
        valid.replace("Quality: 7", "Quality: 21"),
        valid.replace("Item Level: 82", "Item Level: 082"),
        valid.replace("Implicits: 0", "Implicits: 1"),
        valid.replace("+7 to Armour", "Sockets: S"),
        valid.replace("+7 to Armour", "Has +1 to Evasion Rating per Player Level"),
        valid.replace("+7 to Armour", "+7 to Runic Ward"),
        valid.replace("+7 to Armour", "20% increased Action Speed"),
        valid.replace(
            "+7 to Armour",
            "+7 to Armour and Energy Shield if Strength is higher than Intelligence",
        ),
        valid.replace("+7 to Armour", "20% reduced Attribute Requirements"),
        valid.replace("+7 to Armour", "+7 to Armour and Global Energy Shield"),
        valid.replace("+7 to Armour", "Armour: 123"),
    ] {
        assert!(
            parse_equipment_item(&text, &data, 1).is_err(),
            "accepted {text}"
        );
    }
    for line in [
        "+3 to Armour and Energy Shield",
        "17% increased Evasion Rating and Energy Shield",
    ] {
        assert!(
            crate::actor_modifiers::match_actor_modifier_line(line, "Custom:Test", &data).is_err()
        );
        let amulet = amulet("Amber Amulet", "+12 to Strength", line);
        assert!(parse_equipment_item(&amulet, &data, 2).is_err());
        let weapon =
            format!("Rarity: NORMAL\nWooden Club\nItem Level: 1\nQuality: 0\nImplicits: 0\n{line}");
        assert!(parse_equipment_item(&weapon, &data, 3).is_err());
    }
}

#[test]
fn body_source_projection_keeps_generated_penalty_out_of_authored_lines() {
    let compiled = poe_optimizer_engine::CompiledGameData::bundled().unwrap();
    let data = compiled.snapshot().package();
    assert_eq!(
        EQUIPMENT_SOURCE_ORDER,
        [
            "Weapon 1",
            "Helmet",
            "Body Armour",
            "Gloves",
            "Boots",
            "Amulet"
        ]
    );
    for (base, penalty) in [("Rusted Cuirass", 0.05), ("Tattered Robe", 0.03)] {
        let text=armour_item(base,"+7 to Armour\n10% increased Movement Speed\nIgnore all movement penalties from armour\n").replace('\n',"\r\n");
        let item = parse_equipment_item(&text, data, 44).unwrap();
        assert_eq!(item.allowed_slots(), &["Body Armour"]);
        assert!(item.uses_movement());
        assert_eq!(item.source_text(), text);
        assert_eq!(item.modifier_lines().len(), 3);
        let source = item.modifier_source();
        assert_eq!(source, format!("Item:44:Trial Armour, {base}"));
        let records = item.armour_modifiers().unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(item.actor_modifiers().len(), 2);
        let prepared = compiled
            .prepare_armour_with_source(
                item.base_id(),
                item.quality(),
                item.item_level(),
                &source,
                records,
            )
            .unwrap();
        assert_eq!(prepared.source_global_records(), item.actor_modifiers());
        assert_eq!(prepared.generated_global_records().len(), 1);
        let generated = &prepared.generated_global_records()[0];
        assert_eq!(generated.source.as_deref(), Some(source.as_str()));
        assert_eq!(generated.stat, ActorStat::MovementSpeed);
        assert_eq!(
            generated.effect,
            ActorModifierEffect::Numeric {
                operation: ActorNumericOperation::Base,
                value: -penalty
            }
        );
        assert_eq!(prepared.global_records().len(), 3);
        let xml = format!("<Item id=\"44\"><![CDATA[{text}]]></Item>");
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let roundtrip = parse_equipment_item_xml(doc.root_element(), data).unwrap();
        assert_eq!(roundtrip.diagnostic(), item.diagnostic());
        assert_eq!(roundtrip.source_text(), text);
        assert!(
            compiled
                .prepare_armour(item.base_id(), item.quality(), item.item_level(), records)
                .is_err()
        );
    }
}
