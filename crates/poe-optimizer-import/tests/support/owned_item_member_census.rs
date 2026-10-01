//! The census contract is exercised through ordinary source attribution and
//! conversion, using renamed finite fixture definitions rather than real IDs.
use super::*;

fn census_fixture(implicit: usize, explicit: usize) -> Fixture {
    let mut f = member_fixture();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        templates,
        modifier_rules,
        ..
    }) = f.policy.item_modifier_membership.take()
    else {
        panic!()
    };
    let mut lines = f.items.input().clone();
    let mut source = f.item_source.input().clone();
    for count in [1, 2, 3, 64] {
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
    f.items = OwnedItemLinePolicy::new(lines, &f.artifacts.schema, Default::default()).unwrap();
    source.item_lines = *f.items.identity();
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    f.policy.item_modifier_membership = Some(
        ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            definitions,
            item_lines: *f.items.identity(),
            item_source: *f.item_source.identity(),
            templates: vec![],
            modifier_rules,
            paired_templates: vec![],
            census_templates: vec![OrdinaryMemberCensusBase {
                template: templates[0].template.clone(),
                generated_members: OrdinaryMemberGeneration::NoBuffEnchantRuneOrClassMembers,
                implicit_members: implicit,
                explicit_members: explicit,
            }],
        },
    );
    f
}

fn census_raw(implicit: usize, explicit: usize) -> String {
    format!(
        "Rarity: RARE\nTest Title\nOrdinary Base\nImplicits: {implicit}\n{}",
        vec!["One authored member"; implicit + explicit].join("\n")
    )
}

#[test]
fn exact_category_census_preserves_distinct_occurrences_and_only_closes_membership() {
    let f = census_fixture(2, 1);
    let raw = census_raw(2, 1);
    let r = run(&xml(&raw, SLOT), &f);
    assert!(closed(&r));
    assert_eq!(r.sidecar().schema_version, 15);
    let item = &r.draft().input().items.members[0];
    assert_eq!(item.modifiers.members.len(), 3);
    assert!(matches!(
        item.parameters.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(r.sidecar().item_texts[0].parameter_inputs.is_none());
    let ids: BTreeSet<_> = item.modifiers.members.iter().map(|m| m.id).collect();
    assert_eq!(ids.len(), 3);
    assert!(
        item.modifiers
            .members
            .windows(2)
            .all(|m| m[0].definition == m[1].definition)
    );
    let text = &r.sidecar().item_texts[0];
    let members: Vec<_> = text
        .attribution
        .lines
        .iter()
        .filter_map(|l| l.member)
        .collect();
    assert_eq!(
        members
            .iter()
            .map(|m| (m.category, m.ordinal))
            .collect::<Vec<_>>(),
        vec![
            (SourceModifierCategory::Implicit, 1),
            (SourceModifierCategory::Implicit, 2),
            (SourceModifierCategory::Explicit, 1),
        ]
    );
    let order: Vec<_> = members
        .iter()
        .map(|m| {
            let line = text.lines.iter().find(|l| l.index == m.line).unwrap();
            assert_eq!(line.modifiers.len(), 1);
            line.modifiers[0]
        })
        .collect();
    assert_eq!(item.modifier_order.to_resolved(), Some(order));
    let SchemaLookup::Known(schema) = f
        .artifacts
        .schema
        .definition(&item.template.to_resolved().unwrap())
    else {
        panic!()
    };
    assert!(!schema.declarations.parameters.is_complete());
    assert_eq!(
        serde_json::to_value(r.draft().input()).unwrap(),
        serde_json::to_value(run(&xml(&raw, SLOT), &f).draft().input()).unwrap()
    );
}

#[test]
fn counts_are_injected_including_zero_categories_and_the_finite_upper_bound() {
    for (implicit, explicit) in [(0, 3), (3, 0), (1, 2), (2, 2), (0, 64)] {
        let f = census_fixture(implicit, explicit);
        let r = run(&xml(&census_raw(implicit, explicit), SLOT), &f);
        assert!(closed(&r), "{implicit}/{explicit}");
        assert_eq!(
            r.draft().input().items.members[0].modifiers.members.len(),
            implicit + explicit
        );
    }
}

#[test]
fn extra_missing_unknown_category_and_lifecycle_members_remain_pending() {
    let f = census_fixture(2, 1);
    let raw = census_raw(2, 1);
    for changed in [
        census_raw(1, 2),
        census_raw(3, 0),
        census_raw(2, 2),
        census_raw(2, 0),
        raw.replacen("One authored member", "Unknown preceding member", 1),
        raw.replacen("One authored member", "{enchant}One authored member", 1),
        raw.replace("Test Title", "Legacy Jewel Title"),
        raw.replace("Implicits: 2", "Sockets: S\nRune: Unknown\nImplicits: 2"),
        raw.replace("Implicits: 2", "Sockets: J\nImplicits: 2"),
        raw.replace("Ordinary Base", "Ordinary Base\nUnreviewed Base"),
    ] {
        assert!(!closed(&run(&xml(&changed, SLOT), &f)), "{changed}");
    }
    for changed in [
        xml(&raw, SLOT).replace("<Item id=", "<Item unknown=\"true\" id="),
        xml(&raw, SLOT).replace("</Item>", "<Unexpected/></Item>"),
        xml(&raw, SLOT).replace("</Item>", "<ModRange id=\"1\" range=\"2\"/></Item>"),
    ] {
        assert!(!closed(&run(&changed, &f)));
    }
}

#[test]
fn census_counts_and_template_domains_are_checked_before_member_allocation() {
    let f = census_fixture(2, 1);
    for shape in [
        "zero",
        "large",
        "overflow",
        "duplicate",
        "singleton-overlap",
        "pair-overlap",
        "missing-augment",
        "unknown-rule",
    ] {
        let mut p = f.policy.clone();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            templates,
            paired_templates,
            census_templates,
            modifier_rules,
            ..
        }) = &mut p.item_modifier_membership
        else {
            panic!()
        };
        let row = census_templates[0].clone();
        match shape {
            "zero" => {
                census_templates[0].implicit_members = 0;
                census_templates[0].explicit_members = 0;
            }
            "large" => census_templates[0].explicit_members = 64,
            "overflow" => census_templates[0].implicit_members = usize::MAX,
            "duplicate" => census_templates.push(row),
            "singleton-overlap" => templates.push(OrdinarySingletonBase {
                template: row.template,
                generated_members: OrdinaryBaseMembers::NoBuffImplicitRuneOrClassMembers,
            }),
            "pair-overlap" => paired_templates.push(OrdinaryImplicitExplicitBase {
                template: row.template,
                generated_members:
                    OrdinaryImplicitExplicitMembers::OneImplicitNoBuffEnchantRuneOrClassMembers,
            }),
            "missing-augment" => p.equipment_membership = None,
            "unknown-rule" => modifier_rules[0] = key("missing"),
            _ => unreachable!(),
        }
        assert!(
            normalize(&xml(&census_raw(2, 1), SLOT), &f, &p, Default::default()).is_err(),
            "{shape}"
        );
    }
}

#[test]
fn exact_artifact_bindings_and_work_limits_remain_required() {
    let f = census_fixture(2, 1);
    for changed in 0..3 {
        let mut p = f.policy.clone();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            definitions,
            item_lines,
            item_source,
            ..
        }) = &mut p.item_modifier_membership
        else {
            panic!()
        };
        match changed {
            0 => definitions.release.push_str("-stale"),
            1 => *item_lines = *f.item_source.identity(),
            _ => *item_source = *f.items.identity(),
        }
        assert!(matches!(
            normalize(&xml(&census_raw(2, 1), SLOT), &f, &p, Default::default()),
            Err(NormalizationError::Binding)
        ));
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
        assert!(normalize(&xml(&census_raw(2, 1), SLOT), &f, &f.policy, limits).is_err());
    }
}

#[test]
fn partial_roll_declarations_cannot_be_closed_by_the_source_census() {
    let mut f = census_fixture(2, 1);
    let mut schema = f.artifacts.schema.input().clone();
    for d in &mut schema.definitions {
        if let DefinitionDescriptor::Modifier(entry) = d {
            let SchemaState::Known(s) = &mut entry.schema else {
                panic!()
            };
            s.declarations.parameters.closure = partial(subject(&entry.id));
        }
    }
    rebind_quality_schema(&mut f.artifacts, &mut f.policy, schema);
    let mut lines = f.items.input().clone();
    lines.schema_version = OWNED_ITEM_LINE_POLICY_V4;
    lines.definitions = f.artifacts.schema.identity().clone();
    f.items = OwnedItemLinePolicy::new(lines, &f.artifacts.schema, Default::default()).unwrap();
    let mut source = f.item_source.input().clone();
    source.item_lines = *f.items.identity();
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. }) =
        &mut f.policy.equipment_membership
    else {
        panic!()
    };
    *definitions = f.artifacts.schema.identity().clone();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
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
    let r = run(&xml(&census_raw(2, 1), SLOT), &f);
    assert!(!closed(&r));
    assert_eq!(
        r.draft().input().items.members[0].modifiers.members.len(),
        3
    );
    assert!(
        r.draft().input().items.members[0]
            .modifiers
            .members
            .iter()
            .all(|m| matches!(m.rolls.completion, DraftListCompletion::Pending { .. }))
    );
}

#[test]
fn legacy_policy_wire_and_legacy_rows_are_retained_in_v3_without_reinterpretation() {
    let mut f = member_fixture();
    let prior_policy = serde_json::to_value(&f.policy).unwrap();
    let decoded: NormalizationPolicy = serde_json::from_value(prior_policy.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), prior_policy);
    assert!(
        prior_policy["item_modifier_membership"]
            .get("census_templates")
            .is_none()
    );
    let prior = run(&xml(SINGLE, SLOT), &f);
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions,
        item_lines,
        item_source,
        templates,
        modifier_rules,
    }) = f.policy.item_modifier_membership.take()
    else {
        panic!()
    };
    f.policy.item_modifier_membership = Some(
        ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            definitions,
            item_lines,
            item_source,
            templates,
            modifier_rules,
            paired_templates: vec![],
            census_templates: vec![],
        },
    );
    let next = run(&xml(SINGLE, SLOT), &f);
    assert_eq!(
        serde_json::to_value(prior.draft().input()).unwrap(),
        serde_json::to_value(next.draft().input()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&prior.sidecar().item_texts).unwrap(),
        serde_json::to_value(&next.sidecar().item_texts).unwrap()
    );
    assert_eq!(prior.sidecar().schema_version, 12);
    assert_eq!(next.sidecar().schema_version, 15);
}
