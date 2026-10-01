//! Imported construction is injected and proves fresh physical state once.
//! Synthetic names, amounts, and instance keys deliberately differ from corpus data.
use super::*;

fn imported_raw() -> String {
    format!(
        "Rarity: RARE\nAn unrelated title\nOrdinary Base\nUnique ID: {}\nItem Level: 37\nLevelReq: 0\nImplicits: 0\nOne authored member",
        "a2".repeat(32)
    )
}

fn imported_fixture() -> Fixture {
    let mut f = raw_fixture();
    let mut lines = f.items.input().clone();
    lines.rules.retain(|r| r.id != key("implicit-count"));
    for (id, prefix) in [
        ("unique-id", "Unique ID: "),
        ("implicit-count", "Implicits: "),
    ] {
        lines.rules.push(ItemLineRule {
            id: key(id),
            pattern: vec![
                ItemPatternPart::Literal(prefix.into()),
                ItemPatternPart::Capture(key("text")),
            ],
            captures: vec![ItemCapture {
                id: key("text"),
                codec: ItemCaptureCodec::OpaqueText,
            }],
            emissions: vec![ItemEmission::Metadata { role: key(id) }],
        });
    }
    lines.rules.push(ItemLineRule {
        id: key("saved-level"),
        pattern: vec![
            ItemPatternPart::Literal("Item Level: ".into()),
            ItemPatternPart::NumericCapture {
                capture: key("amount"),
                syntax: DecimalSyntax::Integer,
                sign: ItemNumericSign::Forbidden,
            },
        ],
        captures: vec![ItemCapture {
            id: key("amount"),
            codec: ItemCaptureCodec::Value(ValueCodecInput {
                namespace: ns(),
                whitespace: WhitespacePolicy::Exact,
                codec: ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                },
            }),
        }],
        emissions: vec![ItemEmission::ItemLevel {
            value: ItemLineValue::Capture(key("amount")),
        }],
    });
    f.items = OwnedItemLinePolicy::new(lines, &f.artifacts.schema, Default::default()).unwrap();
    let mut source = f.item_source.input().clone();
    source.item_lines = *f.items.identity();
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 { metadata_rules, .. } =
        &mut source.dialect
    else {
        panic!()
    };
    metadata_rules.push(key("unique-id"));
    source
        .rule_layouts
        .extend(["unique-id", "saved-level"].map(|id| ItemRuleSourceLayout {
            rule: key(id),
            role: ItemRuleSourceRole::Header,
        }));
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions,
        templates,
        source_base_names,
        loader_jewel_fallback_titles,
    }) = f.policy.equipment_membership.take()
    else {
        panic!()
    };
    let template = templates[0].template.clone();
    let headers = [
        (
            ImportedItemHeaderField::Rarity,
            "rarity",
            "text",
            ImportedHeaderValue::Literal {
                value: "RARE".into(),
            },
        ),
        (
            ImportedItemHeaderField::UniqueId,
            "unique-id",
            "text",
            ImportedHeaderValue::LowerHex { bytes: 64 },
        ),
        (
            ImportedItemHeaderField::ItemLevel,
            "saved-level",
            "amount",
            ImportedHeaderValue::CanonicalUnsigned { maximum: 100 },
        ),
        (
            ImportedItemHeaderField::RequirementLevel,
            "level-requirement",
            "text",
            ImportedHeaderValue::CanonicalUnsigned { maximum: 100 },
        ),
        (
            ImportedItemHeaderField::ImplicitCount,
            "implicit-count",
            "text",
            ImportedHeaderValue::CanonicalUnsigned { maximum: 64 },
        ),
    ]
    .into_iter()
    .map(|(field, rule, capture, value)| ImportedItemHeader {
        field,
        rule: key(rule),
        capture: key(capture),
        cardinality: ImportedHeaderCardinality::RequiredOnce,
        value,
    })
    .collect();
    f.policy.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
            item_lines: *f.items.identity(),
            item_source: *f.item_source.identity(),
            imported_profiles: vec![ImportedItemConstructionProfile {
                template: template.clone(),
                headers,
            }],
        },
    );
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        modifier_rules,
        ..
    }) = f.policy.item_modifier_membership.take()
    else {
        panic!()
    };
    f.policy.item_modifier_membership = Some(
        ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            definitions,
            item_lines: *f.items.identity(),
            item_source: *f.item_source.identity(),
            templates: vec![],
            modifier_rules,
            paired_templates: vec![],
            census_templates: vec![OrdinaryMemberCensusBase {
                template,
                generated_members: OrdinaryMemberGeneration::NoBuffEnchantRuneOrClassMembers,
                implicit_members: 0,
                explicit_members: 1,
            }],
        },
    );
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        item_lines,
        item_source,
        templates,
        ..
    }) = &mut f.policy.item_parameter_inputs
    else {
        panic!()
    };
    *item_lines = *f.items.identity();
    *item_source = *f.item_source.identity();
    templates[0].construction = OrdinaryItemConstruction::FreshImportedCategoryCensusV4;
    f
}

fn profile(policy: &mut NormalizationPolicy) -> &mut ImportedItemConstructionProfile {
    let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        imported_profiles, ..
    }) = &mut policy.equipment_membership
    else {
        panic!()
    };
    &mut imported_profiles[0]
}
fn assert_pending(result: &NormalizedImport) {
    assert!(!closed(result));
    assert!(!complete(result));
    assert!(
        result.draft().input().items.members[0]
            .modifier_order
            .to_resolved()
            .is_none()
    );
    assert!(result.sidecar().item_texts[0].parameter_inputs.is_none());
}
fn assert_policy_error(f: &Fixture, policy: &NormalizationPolicy) {
    assert!(normalize(&xml(&imported_raw(), SLOT), f, policy, Default::default()).is_err());
}

#[test]
fn injected_imported_construction_closes_only_physical_gates_and_keeps_identity_generic() {
    let f = imported_fixture();
    for raw in [
        imported_raw(),
        imported_raw()
            .replace("An unrelated title", "Another completely different title")
            .replace("Item Level: 37", "Item Level: 81")
            .replace(&"a2".repeat(32), &"9e".repeat(32)),
    ] {
        let source = xml(&raw, SLOT)
            .replace("itemId=\"1\"", "itemId=\"47\"")
            .replacen("<Item id=\"1\">", "<Item id=\"47\">", 1);
        let r = run(&source, &f);
        assert!(closed(&r), "{:?}", r.sidecar().item_texts[0].attribution);
        assert!(complete(&r));
        assert_eq!(r.sidecar().schema_version, 15);
        let item = &r.draft().input().items.members[0];
        assert_eq!(item.parameters.members.len(), 4);
        assert_eq!(item.modifier_order.to_resolved().unwrap().len(), 1);
        let evidence = r.sidecar().item_texts[0].parameter_inputs.as_ref().unwrap();
        assert_eq!(
            evidence
                .iter()
                .filter(|e| matches!(e.origin, ItemParameterInputOrigin::Header { .. }))
                .count(),
            2
        );
        assert!(
            evidence
                .iter()
                .any(|e| e.origin == ItemParameterInputOrigin::AbsentSocketHeader
                    && e.value == ParameterValue::Integer(BoundedInteger::new(0).unwrap()))
        );
        assert!(
            evidence
                .iter()
                .any(|e| e.origin == ItemParameterInputOrigin::FreshUncorrupted
                    && e.value == ParameterValue::Boolean(false))
        );
        let SchemaLookup::Known(schema) = f
            .artifacts
            .schema
            .definition(&item.template.to_resolved().unwrap())
        else {
            panic!()
        };
        assert!(!schema.declarations.parameters.is_complete());
    }
}

#[test]
fn missing_duplicate_alias_setter_flags_and_unknown_layouts_never_fall_back() {
    let f = imported_fixture();
    let raw = imported_raw();
    let id = format!("Unique ID: {}\n", "a2".repeat(32));
    let mut cases = vec![
        raw.replace(
            "LevelReq: 0\nImplicits: 0\nOne authored member",
            "Implicits: 0\nOne authored member\nLevelReq: 0",
        ),
        raw.replace(&id, ""),
        raw.replace(&id, &id.repeat(2)),
        raw.replace(&id, &format!("Unique ID: {}\n", "A2".repeat(32))),
        raw.replace("Item Level: 37", "Item Level: 037"),
        raw.replace("LevelReq: 0", "LevelReq: -1"),
        raw.replace("Implicits: 0", "Implicits: 00"),
        raw.replace("LevelReq: 0", "LevelReq: 0\nLevelReq: 0"),
        raw.replace("Ordinary Base\n", "Superior Ordinary Base\n"),
        raw.replace("LevelReq: 0", "LevelReq: 0\nOrdinary Base"),
        raw.replace("One authored member", "Unknown member"),
        raw.replace("One authored member", "{range:0.5}One authored member"),
        raw.replace("One authored member", "One authored member (crafted)"),
        raw.replace("An unrelated title", "Legacy Jewel Title"),
    ];
    for setter in [
        "Crafted: true",
        "Crafted: false",
        "Sockets: S",
        "Rune: None",
        "Catalyst: Life",
        "CatalystQuality: 20",
        "Corrupted",
        "Unidentified",
        "(Reminder block)",
        "Unknown Header: value",
        "Prefix: None",
        "--------",
    ] {
        cases.push(raw.replace("LevelReq: 0", &format!("{setter}\nLevelReq: 0")));
    }
    for changed in cases {
        let r = run(&xml(&changed, SLOT), &f);
        assert_pending(&r);
    }
}

#[test]
fn exact_xml_range_overlay_is_accepted_but_invalid_duplicate_or_foreign_targets_are_not() {
    let f = imported_fixture();
    let raw = imported_raw();
    for overlay in [
        "<ModRange id=\"1\" range=\"0\"/>",
        "<ModRange id=\"1\" range=\"0.75\"/>",
        "<ModRange id=\"1\" range=\"1\"/>",
    ] {
        let r = run(
            &xml(&raw, SLOT).replace("</Item>", &format!("{overlay}</Item>")),
            &f,
        );
        assert!(closed(&r));
        assert!(complete(&r));
    }
    for overlay in [
        "<ModRange id=\"0\" range=\"0.5\"/>",
        "<ModRange id=\"01\" range=\"0.5\"/>",
        "<ModRange id=\"2\" range=\"0.5\"/>",
        "<ModRange id=\"1\" range=\"NaN\"/>",
        "<ModRange id=\"1\" range=\"1.01\"/>",
        "<ModRange id=\"1\" range=\"0.2\"/><ModRange id=\"1\" range=\"0.8\"/>",
        "<ModRange id=\"1\" range=\"0.5\" other=\"x\"/>",
        "<Unknown/>",
    ] {
        assert_pending(&run(
            &xml(&raw, SLOT).replace("</Item>", &format!("{overlay}</Item>")),
            &f,
        ));
    }
}

#[test]
fn imported_profile_shape_binding_and_limits_fail_before_source_normalization() {
    let f = imported_fixture();
    for change in 0..7 {
        let mut p = f.policy.clone();
        match change {
            0 => profile(&mut p).headers[1].field = ImportedItemHeaderField::Rarity,
            1 => profile(&mut p).headers[1].rule = key("crafted"),
            2 => profile(&mut p).headers[1].capture = key("wrong-capture"),
            3 => profile(&mut p).headers[1].cardinality = ImportedHeaderCardinality::OptionalOnce,
            4 => profile(&mut p).headers[1].value = ImportedHeaderValue::LowerHex { bytes: 129 },
            5 => {
                let repeated = profile(&mut p).headers[0].clone();
                profile(&mut p).headers.push(repeated);
            }
            _ => profile(&mut p).template = ItemTemplateDefId::new(ns(), key("foreign-template")),
        }
        assert_policy_error(&f, &p);
    }
    let mut p = f.policy.clone();
    let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        item_lines,
        item_source,
        ..
    }) = &mut p.equipment_membership
    else {
        panic!()
    };
    std::mem::swap(item_lines, item_source);
    assert_policy_error(&f, &p);
    let mut p = f.policy.clone();
    let Some(EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
        imported_profiles, ..
    }) = &mut p.equipment_membership
    else {
        panic!()
    };
    imported_profiles.resize(4097, imported_profiles[0].clone());
    assert_policy_error(&f, &p);
}

#[test]
fn old_selectors_cannot_consume_imported_evidence_and_legacy_wire_is_unchanged() {
    let f = imported_fixture();
    let mut p = f.policy.clone();
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut p.item_parameter_inputs
    else {
        panic!()
    };
    templates[0].construction = OrdinaryItemConstruction::FreshRareSavedCategoryCensusV3 {
        derived_observations: vec![],
    };
    assert_policy_error(&f, &p);
    let legacy = raw_fixture();
    let value = serde_json::to_value(&legacy.policy.equipment_membership).unwrap();
    assert_eq!(value["kind"], "pob_ordinary_item_sets_v1");
    assert!(value.get("imported_profiles").is_none());
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<EquipmentMembershipPolicy>(value.clone()).unwrap()
        )
        .unwrap(),
        value
    );
    assert_eq!(
        serde_json::to_value(OrdinaryItemConstruction::FreshRareSavedAffixesV1).unwrap(),
        "fresh_rare_saved_affixes_v1"
    );
    assert!(complete(&run(&xml(RAW, SLOT), &legacy)));
}
