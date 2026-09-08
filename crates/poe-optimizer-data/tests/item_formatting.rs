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
fn item_formatting_preserves_exact_case_and_allows_explicit_custom_policy() {
    let original = bundled_snapshot().unwrap();
    let p = original.package();
    assert_eq!(p.item_formatting.rules.len(), 77);
    let evasion = p
        .item_formatting
        .rules
        .iter()
        .find(|r| r.template == "# to Evasion Rating")
        .unwrap();
    assert_eq!(
        evasion.captures,
        vec![ItemNumberFormat {
            precision: 1.0,
            display_precision: None,
            trim_trailing_zeroes: false
        }]
    );
    assert!(
        !p.item_formatting
            .rules
            .iter()
            .any(|r| r.template == "# to evasion rating")
    );
    let mut p = p.clone();
    let rule = p
        .item_formatting
        .rules
        .iter_mut()
        .find(|r| r.template == "# to Evasion Rating")
        .unwrap();
    rule.captures[0] = ItemNumberFormat {
        precision: 100.0,
        display_precision: Some(1),
        trim_trailing_zeroes: true,
    };
    p.actor
        .modifier_rules
        .iter_mut()
        .find(|r| r.id == "evasion_base")
        .unwrap()
        .template = "{0} to Authored Evasion".into();
    p.item_formatting.rules.push(ItemFormattingRule {
        template: "# to Authored Evasion".into(),
        captures: vec![ItemNumberFormat {
            precision: 10.0,
            display_precision: Some(1),
            trim_trailing_zeroes: false,
        }],
    });
    let changed = custom(p).unwrap();
    assert_ne!(original.identity(), changed.identity());
    assert_eq!(changed.trust(), &DataTrust::CustomUnreviewed);
}
#[test]
fn item_formatting_rejects_missing_or_ambiguous_unbounded_fields() {
    let original = bundled_snapshot().unwrap();
    let edits: &[fn(&mut ItemFormattingData)] = &[
        |d| d.rules.clear(),
        |d| d.rules.push(d.rules[0].clone()),
        |d| d.rules[0].template = " # to Armour".into(),
        |d| d.rules[0].template = "{0} to Armour".into(),
        |d| d.rules[0].template = "#[.*]".into(),
        |d| d.rules[0].template = "#\nto Armour".into(),
        |d| d.rules[0].captures.clear(),
        |d| d.rules[0].captures[0].precision = 0.0,
        |d| d.rules[0].captures[0].precision = f64::NAN,
        |d| d.rules[0].captures[0].precision = 10001.0,
        |d| d.rules[0].captures[0].display_precision = Some(3),
        |d| d.rules[0].captures[0].trim_trailing_zeroes = true,
    ];
    for (index, edit) in edits.iter().enumerate() {
        let mut p = original.package().clone();
        edit(&mut p.item_formatting);
        assert!(custom(p).is_err(), "edit{index}");
    }
    let mut raw = serde_json::to_value(original.package()).unwrap();
    raw.as_object_mut().unwrap().remove("item_formatting");
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&raw).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
    let mut raw = serde_json::to_value(original.package()).unwrap();
    raw["item_formatting"]["rules"][0]["captures"][0]["scalar"] = 2.into();
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&raw).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}
