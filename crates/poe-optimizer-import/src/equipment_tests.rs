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
