use super::*;
fn effective_values(
    line: &str,
    values: &[f64],
    data: &GameDataPackage,
) -> Result<Vec<f64>, String> {
    super::effective_values(line, values, data, &vec![false; values.len()])
}
use crate::actor_modifiers::{
    match_actor_modifier_line, match_armour_modifier_line, match_equipment_modifier_line,
};
use poe_optimizer_data::game_data::{self, ItemFormattingRule, ItemNumberFormat};
fn data() -> GameDataPackage {
    game_data::bundled_snapshot().unwrap().package().clone()
}
fn rule(template: &str, precision: f64, count: usize) -> ItemFormattingRule {
    ItemFormattingRule {
        template: template.into(),
        captures: vec![
            ItemNumberFormat {
                precision,
                display_precision: None,
                trim_trailing_zeroes: false
            };
            count
        ],
    }
}
#[test]
fn formatting_preserves_raw_values_and_configuration_but_rounds_item_values() {
    let data = data();
    for (line, raw, effective) in [
        ("+17.5 to Armour", 17.5, 18.0),
        ("-17.5 to Armour", -17.5, -18.0),
        ("+17.5 to Evasion Rating", 17.5, 18.0),
        ("+17.5 to maximum Energy Shield", 17.5, 18.0),
        ("+17.5 to Energy Shield", 17.5, 17.5),
        ("+17.5 to Strength", 17.5, 18.0),
        ("-17.5 to maximum Life", -17.5, -18.0),
        ("+17.5 to Global Armour", 17.5, 17.5),
        ("-17.5% to Fire Resistance", -17.5, -18.0),
    ] {
        let item = match_armour_modifier_line(line, "Item:41:Study", &data)
            .unwrap()
            .unwrap();
        assert_eq!(item.values(), &[raw], "{line}");
        assert_eq!(item.effective_values(), &[effective], "{line}");
        let config = match_actor_modifier_line(line, "Custom:Study", &data)
            .unwrap()
            .unwrap();
        assert_eq!(config.effective_values(), &[raw], "{line}");
    }
}
#[test]
fn exact_case_missing_policy_and_literal_specificity_follow_source_lookup_order() {
    let mut data = data();
    data.item_formatting.rules = vec![
        rule("Adds # to # Physical Damage", 1.0, 2),
        rule("Adds 3.5 to # Physical Damage", 10.0, 1),
        rule("Adds # to 7.5 Physical Damage", 100.0, 1),
    ];
    assert_eq!(
        effective_values("Adds 3.5 to 7.5 Physical Damage", &[3.5, 7.5], &data).unwrap(),
        vec![3.5, 7.5]
    );
    assert_eq!(
        effective_values("Adds 3.50 to 7.5 Physical Damage", &[3.5, 7.5], &data).unwrap(),
        vec![3.5, 7.5]
    );
    data.item_formatting.rules[1].captures[0].precision = 1.0;
    assert_eq!(
        effective_values("Adds 3.5 to 7.5 Physical Damage", &[3.5, 7.5], &data).unwrap(),
        vec![3.5, 8.0]
    );
    data.item_formatting
        .rules
        .push(rule("Adds 3.5 to 7.5 Physical Damage", 1.0, 0));
    assert_eq!(
        effective_values("Adds 3.5 to 7.5 Physical Damage", &[3.5, 7.5], &data).unwrap(),
        vec![3.5, 7.5]
    );
    assert_eq!(
        effective_values("adds 3.5 to 7.5 Physical Damage", &[3.5, 7.5], &data).unwrap(),
        vec![3.5, 7.5]
    );
    data.item_formatting.rules = vec![rule("#% to All Resistances", 1.0, 1)];
    assert_eq!(
        effective_values("+17.5% to all Resistances", &[17.5], &data).unwrap(),
        vec![17.5]
    );
    assert_eq!(
        effective_values("+17.5% to All Resistances", &[17.5], &data).unwrap(),
        vec![18.0]
    );
}
#[test]
fn selected_formatting_policy_changes_effective_rolls_without_changing_raw_values() {
    let mut data = data();
    let line = "+25.5 to maximum Energy Shield";
    let parsed = match_equipment_modifier_line(line, "Item:2:Lunar Amulet", &data)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.values(), &[25.5]);
    assert_eq!(parsed.effective_values(), &[26.0]);
    let policy = data
        .item_formatting
        .rules
        .iter_mut()
        .find(|r| r.template == "# to maximum Energy Shield")
        .unwrap();
    policy.captures[0].precision = 10.0;
    assert_eq!(
        match_equipment_modifier_line(line, "Item:2:Lunar Amulet", &data)
            .unwrap()
            .unwrap()
            .effective_values(),
        &[25.5]
    );
    data.item_formatting
        .rules
        .iter_mut()
        .find(|r| r.template == "# to maximum Energy Shield")
        .unwrap()
        .captures[0]
        .precision = 1.0;
    assert_eq!(
        match_equipment_modifier_line(line, "Item:2:Lunar Amulet", &data)
            .unwrap()
            .unwrap()
            .effective_values(),
        &[26.0]
    );
    data.item_formatting
        .rules
        .retain(|r| r.template != "# to maximum Energy Shield");
    let parsed = match_equipment_modifier_line(line, "Item:2:Lunar Amulet", &data)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.effective_values(), &[25.5]);
    assert_eq!(parsed.values(), &[25.5]);
}
#[test]
fn malformed_formatting_shapes_and_unsupported_modifier_syntax_remain_rejected() {
    let mut data = data();
    assert!(effective_values("Adds 1 to 2 Physical Damage", &[1.0], &data).is_err());
    assert!(effective_values("Adds 1 to 2 and 3", &[1.0, 2.0], &data).is_err());
    assert!(effective_values("+1 to Armour", &[2.0], &data).is_err());
    data.item_formatting.rules = vec![rule("# to Armour", 1.0, 2)];
    assert!(effective_values("+1 to Armour", &[1.0], &data).is_err());
    for line in [
        "+1e2 to Armour",
        "+(1-2) to Armour",
        "1.5% increased Armour",
        "-1.5% increased Armour",
    ] {
        assert!(
            match_armour_modifier_line(line, "Item:1:Study", &data)
                .unwrap()
                .is_none(),
            "{line}"
        );
    }
}

#[test]
fn forced_decimal_item_text_cannot_bypass_source_integer_modifier_grammar() {
    let mut data = data();
    let policy = data
        .item_formatting
        .rules
        .iter_mut()
        .find(|rule| rule.template == "#% increased Armour")
        .unwrap();
    policy.captures[0].precision = 10.0;
    policy.captures[0].display_precision = Some(1);
    policy.captures[0].trim_trailing_zeroes = false;
    let line = "20% increased Armour";
    assert!(match_armour_modifier_line(line, "Item:1:Study", &data).is_err());
    assert!(
        match_actor_modifier_line(line, "Custom:Study", &data)
            .unwrap()
            .is_some()
    );
    data.item_formatting
        .rules
        .iter_mut()
        .find(|rule| rule.template == "#% increased Armour")
        .unwrap()
        .captures[0]
        .trim_trailing_zeroes = true;
    assert_eq!(
        match_armour_modifier_line(line, "Item:1:Study", &data)
            .unwrap()
            .unwrap()
            .effective_values(),
        &[20.0]
    );
    let policy = data
        .item_formatting
        .rules
        .iter_mut()
        .find(|rule| rule.template == "# to Armour")
        .unwrap();
    policy.captures[0].precision = 100.0;
    policy.captures[0].display_precision = Some(1);
    policy.captures[0].trim_trailing_zeroes = false;
    assert_eq!(
        match_armour_modifier_line("+17.46 to Armour", "Item:1:Study", &data)
            .unwrap()
            .unwrap()
            .effective_values(),
        &[17.5]
    );
}

#[test]
fn mixed_case_grammar_preserves_case_sensitive_item_formatting_and_raw_captures() {
    let data = data();
    let exact = match_armour_modifier_line("+17.5 to Evasion Rating", "Item:44:Study", &data)
        .unwrap()
        .unwrap();
    let lower = match_armour_modifier_line("+17.5 to evasion rating", "Item:44:Study", &data)
        .unwrap()
        .unwrap();
    assert_eq!(exact.rule_id(), lower.rule_id());
    assert_eq!(exact.values(), lower.values());
    assert_eq!(exact.effective_values(), &[18.0]);
    assert_eq!(lower.effective_values(), &[17.5]);
    assert_eq!(
        match_actor_modifier_line("+17.5 TO EVASION RATING", "Custom:Study", &data)
            .unwrap()
            .unwrap()
            .effective_values(),
        &[17.5]
    );
    for line in [
        "mOvEmEnT SpEeD CaNnOt bE MoDiFiEd tO BeLoW BaSe VaLuE",
        "YoUr MoVeMeNt SpEeD iS 57% oF ItS bAsE VaLuE",
        "IgNoRe AlL mOvEmEnT PeNaLtIeS FrOm ArMoUr",
    ] {
        assert!(
            match_actor_modifier_line(line, "Custom:Study", &data)
                .unwrap()
                .is_some()
        );
        assert!(
            match_equipment_modifier_line(line, "Item:44:Study", &data)
                .unwrap()
                .is_some()
        );
    }
    let mut ambiguous = data.clone();
    let mut duplicate = ambiguous
        .actor
        .modifier_rules
        .iter()
        .find(|rule| rule.id == "evasion_base")
        .unwrap()
        .clone();
    duplicate.id = "case_duplicate".into();
    duplicate.template = duplicate.template.to_ascii_lowercase();
    ambiguous.actor.modifier_rules.push(duplicate);
    assert!(
        match_armour_modifier_line("+17.5 to Evasion Rating", "Item:44:Study", &ambiguous).is_err()
    );
}
