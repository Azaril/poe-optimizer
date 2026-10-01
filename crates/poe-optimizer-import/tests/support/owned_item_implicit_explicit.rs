//! V2 physical membership and raw-input construction, with renamed fixture IDs.
use super::*;

fn pair_raw() -> String {
    RAW.replace("Sockets: S S\nRune: None\nRune: None\n", "")
        .replace("Implicits: 0", "Implicits: 1")
        + "\nOne authored member"
}

fn pair_fixture() -> Fixture {
    let mut f = raw_fixture();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        templates,
        modifier_rules,
        ..
    }) = f.policy.item_modifier_membership.take()
    else {
        panic!()
    };
    let template = templates[0].template.clone();
    let mut lines = f.items.input().clone();
    let mut source = f.item_source.input().clone();
    for count in [1, 2] {
        let id = key(&format!("implicit-count-{count}"));
        lines.rules.push(ItemLineRule {
            id: id.clone(),
            pattern: vec![ItemPatternPart::Literal(format!("Implicits: {count}"))],
            captures: vec![],
            emissions: vec![ItemEmission::Metadata { role: id.clone() }],
        });
        source.rule_layouts.push(ItemRuleSourceLayout {
            rule: id,
            role: ItemRuleSourceRole::Header,
        });
    }
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 {
        single_modifier_conditions,
        ..
    } = &mut source.dialect
    else {
        panic!()
    };
    single_modifier_conditions[0].all = vec![ItemSourceCondition::NoSourceScalingTags];
    f.items = OwnedItemLinePolicy::new(lines, &f.artifacts.schema, Default::default()).unwrap();
    source.item_lines = *f.items.identity();
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    f.policy.item_modifier_membership = Some(
        ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
            definitions,
            item_lines: *f.items.identity(),
            item_source: *f.item_source.identity(),
            templates: vec![],
            modifier_rules,
            paired_templates: vec![OrdinaryImplicitExplicitBase {
                template,
                generated_members:
                    OrdinaryImplicitExplicitMembers::OneImplicitNoBuffEnchantRuneOrClassMembers,
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
    templates[0].construction = OrdinaryItemConstruction::FreshRareSavedImplicitExplicitV2;
    f
}

#[test]
fn paired_members_keep_distinct_occurrences_and_exact_category_order() {
    let f = pair_fixture();
    let r = run(&xml(&pair_raw(), SLOT), &f);
    assert!(closed(&r));
    assert!(complete(&r));
    assert_eq!(r.sidecar().schema_version, 14);
    let item = &r.draft().input().items.members[0];
    assert_eq!(item.modifiers.members.len(), 2);
    // The same authored rule/definition occurs twice. Physical members must not
    // be deduplicated by definition or mistaken for two emissions of one line.
    assert_eq!(
        item.modifiers.members[0].definition,
        item.modifiers.members[1].definition
    );
    assert_ne!(item.modifiers.members[0].id, item.modifiers.members[1].id);
    let text = &r.sidecar().item_texts[0];
    let members: Vec<_> = text
        .attribution
        .lines
        .iter()
        .filter_map(|l| l.member)
        .collect();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].category, SourceModifierCategory::Implicit);
    assert_eq!(members[1].category, SourceModifierCategory::Explicit);
    assert_eq!(
        members.iter().map(|m| m.ordinal).collect::<Vec<_>>(),
        vec![1, 1]
    );
    let expected: Vec<_> = members
        .iter()
        .map(|m| {
            let line = text.lines.iter().find(|l| l.index == m.line).unwrap();
            assert_eq!(line.modifiers.len(), 1);
            line.modifiers[0]
        })
        .collect();
    assert_eq!(item.modifier_order.to_resolved(), Some(expected));
    let inputs = text.parameter_inputs.as_ref().unwrap();
    assert_eq!(inputs.len(), 4);
    assert!(
        inputs
            .iter()
            .any(|v| v.origin == ItemParameterInputOrigin::AbsentSocketHeader
                && v.value == ParameterValue::Integer(BoundedInteger::new(0).unwrap()))
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

#[test]
fn explicit_empty_socket_capacity_is_separate_from_absent_header_evidence() {
    let f = pair_fixture();
    let raw = pair_raw().replace("LevelReq: 0", "Sockets: S\nRune: None\nLevelReq: 77");
    let r = run(&xml(&raw, SLOT), &f);
    assert!(closed(&r) && complete(&r));
    let inputs = r.sidecar().item_texts[0].parameter_inputs.as_ref().unwrap();
    assert!(inputs.iter().any(|v| matches!(
        v.origin,
        ItemParameterInputOrigin::EmptySocketCapacity { .. }
    ) && v.value
        == ParameterValue::Integer(BoundedInteger::new(1).unwrap())));
    assert!(
        !inputs
            .iter()
            .any(|v| v.origin == ItemParameterInputOrigin::AbsentSocketHeader)
    );
}

#[test]
fn wrong_member_categories_counts_or_lifecycle_never_complete_the_pair() {
    let f = pair_fixture();
    let raw = pair_raw();
    for changed in [
        raw.replace("Implicits: 1", "Implicits: 0"),
        raw.replace("Implicits: 1", "Implicits: 2"),
        raw.replacen("One authored member\n", "", 1),
        format!("{raw}\nOne authored member"),
        raw.replacen("One authored member", "Unconverted preceding member", 1),
        raw.replacen("One authored member", "{enchant}One authored member", 1),
        raw.replace("LevelReq: 0", "Sockets: S\nRune: Unknown\nLevelReq: 0"),
        raw.replace("LevelReq: 0", "Sockets: J\nLevelReq: 0"),
        raw.replace("Test Title", "Legacy Jewel Title"),
    ] {
        let r = run(&xml(&changed, SLOT), &f);
        assert!(!closed(&r), "pair must remain pending: {changed}");
        assert!(!complete(&r));
    }
    // Source can label a later member implicit, but the existing finite fresh
    // augment grammar deliberately excludes that tagged lifecycle. Do not
    // broaden admission merely to test reversed physical line order.
    let reversed = raw.replace("Implicits: 1", "Implicits: 0").replace(
        "One authored member\nOne authored member",
        "One authored member\n{implicit}One authored member",
    );
    let r = run(&xml(&reversed, SLOT), &f);
    assert!(matches!(
        r.sidecar().item_texts[0].attribution.layout,
        ItemLayoutStatus::Proven
    ));
    let categories: Vec<_> = r.sidecar().item_texts[0]
        .attribution
        .lines
        .iter()
        .filter_map(|l| l.member.map(|m| m.category))
        .collect();
    assert_eq!(
        categories,
        vec![
            SourceModifierCategory::Explicit,
            SourceModifierCategory::Implicit
        ]
    );
    assert!(!closed(&r) && !complete(&r));
}

#[test]
fn paired_raw_failures_leave_independent_member_proof_and_legacy_profiles_unchanged() {
    let f = pair_fixture();
    for raw in [
        pair_raw().replace("LevelReq: 0", "LevelReq: nope"),
        pair_raw().replace("LevelReq: 0\n", ""),
        pair_raw().replace("LevelReq: 0", "LevelReq: 0\nLevelReq: 1"),
    ] {
        let r = run(&xml(&raw, SLOT), &f);
        assert!(closed(&r));
        assert!(!complete(&r));
        assert!(r.sidecar().item_texts[0].parameter_inputs.is_none());
    }
    let mut old = raw_fixture();
    let before = run(&xml(RAW, SLOT), &old);
    assert_eq!(before.sidecar().schema_version, 13);
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        item_lines,
        item_source,
        templates,
        modifier_rules,
    }) = old.policy.item_modifier_membership.take()
    else {
        panic!()
    };
    old.policy.item_modifier_membership = Some(
        ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
            definitions,
            item_lines,
            item_source,
            templates,
            modifier_rules,
            paired_templates: vec![],
        },
    );
    let after = run(&xml(RAW, SLOT), &old);
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(
        serde_json::to_value(&before.sidecar().item_texts).unwrap(),
        serde_json::to_value(&after.sidecar().item_texts).unwrap()
    );
    assert_eq!(after.sidecar().schema_version, 14);
    assert!(complete(&after) && closed(&after));
}

#[test]
fn paired_proof_rejects_source_identity_namespace_and_bounded_work_violations() {
    let f = pair_fixture();
    let input = xml(&pair_raw(), SLOT);
    for changed in [
        input.replace("<Item id=\"1\">", "<Item id=\"1\" variant=\"1\">"),
        input.replace("<Items ", "<Items xmlns=\"urn:foreign\" "),
        input.replace("</Item>", "<Other/></Item>"),
        input.replace("</Items>", "<Item id=\"01\">Other</Item></Items>"),
        input.replace("</Items>", "</Items><Items/>"),
    ] {
        let r = run(&changed, &f);
        assert!(!closed(&r) && !complete(&r));
    }
    for limits in [
        NormalizationLimits {
            max_work: 1,
            ..Default::default()
        },
        NormalizationLimits {
            max_policy_bytes: 16,
            ..Default::default()
        },
    ] {
        assert!(normalize(&input, &f, &f.policy, limits).is_err());
    }
}

#[test]
fn paired_bindings_and_construction_are_checked_before_source_use() {
    let f = pair_fixture();
    for index in 0..8 {
        let mut p = f.policy.clone();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
            definitions,
            item_lines,
            item_source,
            templates,
            modifier_rules,
            paired_templates,
        }) = &mut p.item_modifier_membership
        else {
            panic!()
        };
        match index {
            0 => definitions.content_sha256 = "0".repeat(64),
            1 => *item_lines = *f.item_source.identity(),
            2 => *item_source = *f.items.identity(),
            3 => paired_templates.push(paired_templates[0].clone()),
            4 => templates.push(OrdinarySingletonBase {
                template: paired_templates[0].template.clone(),
                generated_members: OrdinaryBaseMembers::NoBuffImplicitRuneOrClassMembers,
            }),
            5 => modifier_rules.push(modifier_rules[0].clone()),
            6 => p.equipment_membership = None,
            _ => {
                let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
                    &mut p.item_parameter_inputs
                else {
                    panic!()
                };
                templates[0].construction = OrdinaryItemConstruction::FreshRareSavedAffixesV1;
            }
        }
        assert!(
            normalize(&xml(&pair_raw(), SLOT), &f, &p, Default::default()).is_err(),
            "invalid policy {index}"
        );
    }
    let mut old = raw_fixture();
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut old.policy.item_parameter_inputs
    else {
        panic!()
    };
    templates[0].construction = OrdinaryItemConstruction::FreshRareSavedImplicitExplicitV2;
    assert!(normalize(&xml(RAW, SLOT), &old, &old.policy, Default::default()).is_err());
}

fn refresh_pair(
    f: &mut Fixture,
    lines: ItemLinePolicyInput,
    mut source: ItemSourceLayoutPolicyInput,
) {
    f.items = OwnedItemLinePolicy::new(lines, &f.artifacts.schema, Default::default()).unwrap();
    source.item_lines = *f.items.identity();
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
        definitions,
        item_lines,
        item_source,
        ..
    }) = &mut f.policy.item_modifier_membership
    else {
        panic!()
    };
    *definitions = f.artifacts.schema.identity().clone();
    *item_lines = *f.items.identity();
    *item_source = *f.item_source.identity();
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        definitions,
        item_lines,
        item_source,
        ..
    }) = &mut f.policy.item_parameter_inputs
    else {
        panic!()
    };
    *definitions = f.artifacts.schema.identity().clone();
    *item_lines = *f.items.identity();
    *item_source = *f.item_source.identity();
}

#[test]
fn zero_range_or_failed_member_guard_cannot_borrow_its_known_follower() {
    let mut f = pair_fixture();
    let mut lines = f.items.input().clone();
    let emission = lines
        .rules
        .iter()
        .find(|r| r.id == key("one-member"))
        .unwrap()
        .emissions
        .clone();
    lines.rules.push(ItemLineRule {
        id: key("positive-range-member"),
        pattern: vec![
            ItemPatternPart::Literal("+(".into()),
            ItemPatternPart::NumericCapture {
                capture: key("lower"),
                syntax: DecimalSyntax::Integer,
                sign: ItemNumericSign::Forbidden,
            },
            ItemPatternPart::Literal("-".into()),
            ItemPatternPart::NumericCapture {
                capture: key("upper"),
                syntax: DecimalSyntax::Integer,
                sign: ItemNumericSign::Forbidden,
            },
            ItemPatternPart::Literal(") to fixture property".into()),
        ],
        captures: ["lower", "upper"]
            .into_iter()
            .map(|id| ItemCapture {
                id: key(id),
                codec: ItemCaptureCodec::Value(ValueCodecInput {
                    namespace: ns(),
                    whitespace: WhitespacePolicy::Exact,
                    codec: ValueCodecKind::Integer {
                        syntax: DecimalSyntax::Integer,
                    },
                }),
            })
            .collect(),
        emissions: emission,
    });
    let mut source = f.item_source.input().clone();
    source.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("positive-range-member"),
        role: ItemRuleSourceRole::Unresolved,
    });
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 {
        single_modifier_conditions,
        ..
    } = &mut source.dialect
    else {
        panic!()
    };
    single_modifier_conditions.push(ItemSourceConditionalMember {
        rule: key("positive-range-member"),
        all: vec![
            ItemSourceCondition::NoSourceScalingTags,
            ItemSourceCondition::UnsignedIntegerCapture {
                capture: key("lower"),
                min: 1,
                max: 100,
            },
            ItemSourceCondition::UnsignedIntegerCapture {
                capture: key("upper"),
                min: 1,
                max: 100,
            },
        ],
    });
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
        modifier_rules,
        ..
    }) = &mut f.policy.item_modifier_membership
    else {
        panic!()
    };
    modifier_rules.push(key("positive-range-member"));
    refresh_pair(&mut f, lines, source);
    let raw = pair_raw().replacen("One authored member", "+(10-15) to fixture property", 1);
    assert!(closed(&run(&xml(&raw, SLOT), &f)));
    for replacement in ["+(0-0)", "+(bad-15)", "+(0-15)"] {
        let changed = raw.replace("+(10-15)", replacement);
        let r = run(&xml(&changed, SLOT), &f);
        assert!(!matches!(
            r.sidecar().item_texts[0].attribution.layout,
            ItemLayoutStatus::Proven
        ));
        assert!(!closed(&r) && !complete(&r));
    }
}

#[test]
fn pair_requires_every_canonical_roll_inventory_and_retained_template_member() {
    for remove_membership in [false, true] {
        let mut f = pair_fixture();
        let mut schema = f.artifacts.schema.input().clone();
        for definition in &mut schema.definitions {
            match definition {
                DefinitionDescriptor::Modifier(entry) if !remove_membership => {
                    let SchemaState::Known(s) = &mut entry.schema else {
                        panic!()
                    };
                    s.declarations.parameters.closure = partial(subject(&entry.id));
                }
                DefinitionDescriptor::ItemTemplate(entry) if remove_membership => {
                    let SchemaState::Known(s) = &mut entry.schema else {
                        panic!()
                    };
                    s.modifiers.members.clear();
                }
                _ => {}
            }
        }
        rebind_quality_schema(&mut f.artifacts, &mut f.policy, schema);
        let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. }) =
            &mut f.policy.equipment_membership
        else {
            panic!()
        };
        *definitions = f.artifacts.schema.identity().clone();
        let mut lines = f.items.input().clone();
        lines.schema_version = OWNED_ITEM_LINE_POLICY_V4;
        lines.definitions = f.artifacts.schema.identity().clone();
        let source = f.item_source.input().clone();
        refresh_pair(&mut f, lines, source);
        let r = run(&xml(&pair_raw(), SLOT), &f);
        assert!(!closed(&r) && !complete(&r));
    }
}
