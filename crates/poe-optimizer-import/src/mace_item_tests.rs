use super::*;
use poe_optimizer_data::game_data::bundled_snapshot;
const NORMAL: &str = "Rarity: NORMAL\nWooden Club\nItem Level: 1\nQuality: 0\nImplicits: 0";
const RARE: &str = "Rarity: RARE\nStudy Hammer\nWooden Club\nItem Level: 10\nQuality: 20\nLevelReq: 1\nImplicits: 0";
const MODS: [&str; 5] = [
    "Adds 2 to 5 Physical Damage",
    "Adds 3 to 7 Fire Damage",
    "50% increased Physical Damage",
    "20% increased Attack Speed",
    "100% increased Critical Hit Chance",
];
fn data() -> GameDataPackage {
    bundled_snapshot().unwrap().package().clone()
}
fn rare() -> String {
    format!("{RARE}\n{}", MODS.join("\n"))
}

#[test]
fn parses_reviewed_local_families_and_retains_exact_ordered_roll_evidence() {
    let data = data();
    let text = rare();
    let weapon = parse_mace_item(&text, &data).unwrap();
    assert_eq!(weapon.weapon_key(), "wooden_club");
    assert_eq!(weapon.rarity(), MaceItemRarity::Rare);
    assert_eq!(weapon.rare_name(), Some("Study Hammer"));
    assert_eq!(weapon.quality(), 20);
    assert_eq!(weapon.item_level(), 10);
    assert_eq!(weapon.explicit_level_requirement(), Some(1));
    assert_eq!(weapon.effective_level_requirement(), 1);
    assert_eq!(weapon.source_text(), text);
    assert_eq!(weapon.local_modifiers().len(), 5);
    assert_eq!(
        weapon
            .local_modifiers()
            .iter()
            .map(|roll| roll.rule_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "local_physical_added",
            "local_fire_added",
            "local_physical_increased",
            "local_attack_speed_increased",
            "local_critical_chance_increased",
        ]
    );
    assert_eq!(
        weapon
            .local_modifiers()
            .iter()
            .map(|roll| roll.values.clone())
            .collect::<Vec<_>>(),
        vec![vec![2., 5.], vec![3., 7.], vec![50.], vec![20.], vec![100.]]
    );
    for (index, line) in weapon.modifier_lines().iter().enumerate() {
        assert_eq!(line.line_number, index + 8);
        assert_eq!(&text[line.byte_range.clone()], MODS[index]);
        assert_eq!(line.source, MODS[index]);
    }
    assert_eq!(weapon.diagnostic()["affix_legality_verified"], false);
    assert!(!weapon.is_legacy_normal_payload());
}
#[test]
fn line_edge_whitespace_and_blank_lines_preserve_bytes_hashes_and_locations() {
    let data = data();
    let plain = rare();
    let decorated = format!("\r\n\t{}  \r\n\r\n", plain.replace('\n', "  \r\n\t"));
    let a = parse_mace_item(&plain, &data).unwrap();
    let b = parse_mace_item(&decorated, &data).unwrap();
    assert_eq!(a.local_modifiers(), b.local_modifiers());
    assert_eq!(b.source_text(), decorated);
    assert_ne!(a.source_sha256(), b.source_sha256());
    for (index, line) in b.modifier_lines().iter().enumerate() {
        assert_eq!(line.line_number, index + 9);
        assert_eq!(&decorated[line.byte_range.clone()], line.source);
        assert_eq!(line.source.trim_ascii(), MODS[index]);
    }
    assert_eq!(a.pob_export_lines(), b.pob_export_lines());
}
#[test]
fn duplicate_lines_stay_distinct_and_removal_rebuilds_only_retained_rolls() {
    let data = data();
    let repeated = format!("{RARE}\n{}\n{}\n{}", MODS[0], MODS[0], MODS[3]);
    let item = parse_mace_item(&repeated, &data).unwrap();
    assert_eq!(item.local_modifiers().len(), 3);
    assert_eq!(item.local_modifiers()[0], item.local_modifiers()[1]);
    assert_ne!(
        item.modifier_lines()[0].line_number,
        item.modifier_lines()[1].line_number
    );
    let removed = parse_mace_item(&format!("{RARE}\n{}", MODS[3]), &data).unwrap();
    assert_eq!(removed.local_modifiers(), &item.local_modifiers()[2..]);
    assert!(
        parse_mace_item(RARE, &data)
            .unwrap()
            .local_modifiers()
            .is_empty()
    );
}
#[test]
fn all_lines_require_an_exact_complete_rule_match_and_bounded_numeric_syntax() {
    let data = data();
    for line in [
        "Adds 2 to 5 Physical Damage to Attacks",
        "Adds 2 to 5 Cold Damage",
        "Adds 2 to 5 Fire Damage while moving",
        "20% increased Global Physical Damage",
        "20% increased Attack Speed if you have killed recently",
        "20% increased Critical Hit Chance with Maces",
        "{crafted}20% increased Attack Speed",
        "{disabled}20% increased Attack Speed",
        "{rune}20% increased Attack Speed",
        "20% increased Attack Speed (enchant)",
        "Grants Skill: Spark",
        "Adds -2 to 5 Physical Damage",
        "Adds +2 to 5 Physical Damage",
        "Adds 2.0 to 5 Physical Damage",
        "Adds 5 to 2 Physical Damage",
        "Adds 0 to 1000001 Physical Damage",
        "Adds (2-4) to (5-7) Physical Damage",
        "1e2% increased Attack Speed",
        "1.5% increased Attack Speed",
        "NaN% increased Attack Speed",
        "-1% increased Attack Speed",
        "+1% increased Attack Speed",
        "1000001% increased Physical Damage",
        "20%  increased Attack Speed",
        "20% increased Attack Speed extra",
        "Quality: 20",
        "LevelReq: 1",
        "# comment",
        "-- comment",
        "Corrupted",
        "Sockets: S",
        "Rune: None",
        "Implicits: 1",
    ] {
        assert!(
            parse_mace_item(&format!("{RARE}\n{line}"), &data).is_err(),
            "accepted {line}"
        );
    }
    for line in [
        "Adds 0 to 0 Physical Damage",
        "Adds 1000000 to 1000000 Fire Damage",
        "0% increased Attack Speed",
        "1000000% increased Critical Hit Chance",
    ] {
        assert!(
            parse_mace_item(&format!("{RARE}\n{line}"), &data).is_ok(),
            "rejected {line}"
        );
    }
}
#[test]
fn metadata_requires_ordered_unique_fields_and_admitted_rarity_base_and_limits() {
    let data = data();
    for text in [
        RARE.replace("RARE", "UNIQUE"),
        RARE.replace("RARE", "MAGIC"),
        RARE.replace("RARE", "Rare"),
        RARE.replace("Study Hammer\n", ""),
        RARE.replace("Study Hammer", "{variant}Study Hammer"),
        RARE.replace("Wooden Club", "Unknown Club"),
        RARE.replace("LevelReq: 1", "LevelReq: 1\nLevelReq: 2"),
        RARE.replace("Item Level: 10", "Item Level: 0"),
        RARE.replace("Item Level: 10", "Item Level: 101"),
        RARE.replace("Quality: 20", "Quality: 21"),
        RARE.replace("Quality: 20", "Quality: +20"),
        RARE.replace("LevelReq: 1", "LevelReq: 101"),
        RARE.replace("LevelReq: 1", "Requires Level: 1"),
        RARE.replace("Quality: 20\nLevelReq: 1", "LevelReq: 1\nQuality: 20"),
        RARE.replace("Implicits: 0", "Implicits: 1"),
        RARE.replace("Study Hammer", "Study\0Hammer"),
    ] {
        assert!(parse_mace_item(&text, &data).is_err(), "accepted {text:?}");
    }
    assert!(parse_mace_item("", &data).is_err());
    assert!(parse_mace_item(&format!("{RARE}{}", " ".repeat(MAX_MACE_ITEM_BYTES)), &data).is_err());
    assert!(parse_mace_item(&format!("{RARE}\n{}", " ".repeat(257)), &data).is_err());
    for count in [MAX_MACE_MODIFIER_LINES, MAX_MACE_MODIFIER_LINES + 1] {
        let text = format!("{RARE}\n{}", vec![MODS[3]; count].join("\n"));
        assert_eq!(
            parse_mace_item(&text, &data).is_ok(),
            count <= MAX_MACE_MODIFIER_LINES
        );
    }
}
#[test]
fn authored_equip_level_overrides_base_level_without_using_item_level() {
    let mut data = data();
    data.weapons
        .iter_mut()
        .find(|w| w.id == "wooden_club")
        .unwrap()
        .requirements
        .level = 27;
    let base = parse_mace_item(&NORMAL.replace("Item Level: 1", "Item Level: 100"), &data).unwrap();
    assert_eq!(base.item_level(), 100);
    assert_eq!(base.explicit_level_requirement(), None);
    assert_eq!(base.effective_level_requirement(), 27);
    assert!(base.is_legacy_normal_payload());
    for level in [0, 1, 26, 27, 60, 100] {
        let text = NORMAL.replace("Implicits: 0", &format!("LevelReq: {level}\nImplicits: 0"));
        let item = parse_mace_item(&text, &data).unwrap();
        assert_eq!(item.effective_level_requirement(), level);
        assert!(!item.is_legacy_normal_payload());
    }
}
#[test]
fn configured_grammar_and_rule_id_are_injected_but_ambiguity_never_picks_a_rule() {
    let mut data = data();
    let speed = data
        .item_modifier_rules
        .iter_mut()
        .find(|r| r.id == "local_attack_speed_increased")
        .unwrap();
    speed.id = "configured_local_speed".into();
    speed.template = "Attack rate is increased by {0}%".into();
    speed.captures = vec![ItemCaptureKind::UnsignedDecimal];
    let parsed =
        parse_mace_item(&format!("{RARE}\nAttack rate is increased by 12.5%"), &data).unwrap();
    assert_eq!(
        parsed.local_modifiers()[0].rule_id,
        "configured_local_speed"
    );
    assert_eq!(parsed.local_modifiers()[0].values, vec![12.5]);
    assert!(parse_mace_item(&format!("{RARE}\n{}", MODS[3]), &data).is_err());
    for number in [".5", "1.", "1.2.3", "+1.5", "-1.5", "1e2"] {
        assert!(
            parse_mace_item(
                &format!("{RARE}\nAttack rate is increased by {number}%"),
                &data
            )
            .is_err()
        );
    }
    let duplicate = data
        .item_modifier_rules
        .iter()
        .find(|r| r.id == "configured_local_speed")
        .unwrap()
        .clone();
    data.item_modifier_rules.push(duplicate);
    assert!(
        parse_mace_item(&format!("{RARE}\nAttack rate is increased by 12.5%"), &data)
            .unwrap_err()
            .to_string()
            .contains("multiple")
    );
}

#[test]
fn xml_item_source_helper_retains_crlf_and_decodes_only_compatible_complete_text() {
    let mut data = data();
    data.weapons
        .iter_mut()
        .find(|w| w.id == "wooden_club")
        .unwrap()
        .name = "Wooden & <Club>".into();
    let source = rare()
        .replace("Wooden Club", "Wooden & <Club>")
        .replace('\n', "\r\n");
    for payload in [
        source
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;"),
        format!("<![CDATA[{source}]]>"),
    ] {
        let xml = format!("<Item id=\"1\">{payload}</Item>");
        crate::xml_compat::validate_native(&xml).unwrap();
        let document = roxmltree::Document::parse(&xml).unwrap();
        let item = parse_mace_item_element(document.root_element(), &data).unwrap();
        assert_eq!(item.source_text(), source);
        assert_eq!(
            item.diagnostic(),
            parse_mace_item(&source, &data).unwrap().diagnostic()
        );
    }
    let plain = rare();
    for xml in [
        format!(
            "<Item>{}</Item>",
            plain.replace("Study Hammer", "Study &#72;ammer")
        ),
        format!(
            "<Item>{}</Item>",
            plain.replace("Study Hammer", "Study<!--x--> Hammer")
        ),
        format!("<Item>{plain}<ModRange id=\"1\" range=\"0.5\"/></Item>"),
    ] {
        let document = roxmltree::Document::parse(&xml).unwrap();
        assert!(parse_mace_item_element(document.root_element(), &data).is_err());
    }
}

#[test]
fn canonical_metadata_headers_preserve_legacy_scope_without_restricting_modifier_capture_spelling()
{
    let data = data();
    assert!(
        parse_mace_item(NORMAL, &data)
            .unwrap()
            .is_legacy_normal_payload()
    );
    for source in [
        NORMAL.replace("Item Level: 1", "Item Level: 01"),
        NORMAL.replace("Item Level: 1", "Item Level: 001"),
        NORMAL.replace("Quality: 0", "Quality: 00"),
        NORMAL.replace("Quality: 0", "Quality: 020"),
        RARE.replace("LevelReq: 1", "LevelReq: 01"),
        RARE.replace("LevelReq: 1", "LevelReq: 00"),
    ] {
        assert!(
            parse_mace_item(&source, &data).is_err(),
            "accepted {source:?}"
        );
    }
    for quality in [0, 1, 20] {
        let source = NORMAL.replace("Quality: 0", &format!("Quality: {quality}"));
        assert!(
            parse_mace_item(&source, &data)
                .unwrap()
                .is_legacy_normal_payload()
        );
    }
    let source = format!("{RARE}\n020% increased Attack Speed");
    let item = parse_mace_item(&source, &data).unwrap();
    assert_eq!(item.local_modifiers()[0].values, [20.0]);
    assert_eq!(
        item.modifier_lines()[0].source,
        "020% increased Attack Speed"
    );
}

#[test]
fn local_weapon_modifier_case_follows_source_parser_without_rewriting_source() {
    let data = data();
    let text = format!("{RARE}\nAdDs 2 To 5 PhYsIcAl DaMaGe\n20% INCREASED ATTACK SPEED");
    let weapon = parse_mace_item(&text, &data).unwrap();
    assert_eq!(weapon.source_text(), text);
    assert_eq!(weapon.local_modifiers()[0].values, vec![2.0, 5.0]);
    assert_eq!(weapon.local_modifiers()[1].values, vec![20.0]);
    assert_eq!(
        weapon.modifier_lines()[0].source,
        "AdDs 2 To 5 PhYsIcAl DaMaGe"
    );
    let mut ambiguous = data;
    let mut duplicate = ambiguous.item_modifier_rules[0].clone();
    duplicate.id = "case_duplicate".into();
    duplicate.template = duplicate.template.to_ascii_lowercase();
    ambiguous.item_modifier_rules.push(duplicate);
    assert!(parse_mace_item(&text, &ambiguous).is_err());
}
