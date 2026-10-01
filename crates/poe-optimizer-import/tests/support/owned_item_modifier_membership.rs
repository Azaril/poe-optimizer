//! Synthetic converter contracts; the real source witness is a separate target.
use super::*;

const SINGLE: &str = "Rarity: RARE\nTest Title\nOrdinary Base\nImplicits: 0\nOne authored member";

fn partial(owner: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::InputSchema,
            code: key("template-inputs-unconverted"),
        }],
    }
}

fn member_fixture() -> Fixture {
    let mut f = fixture();
    let modifier = f
        .artifacts
        .registry
        .allocate_definition::<ModifierDefinition>()
        .unwrap();
    let mut schema = f.artifacts.schema.input().clone();
    let mut declarations = DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    };
    schema
        .definitions
        .push(DefinitionDescriptor::Modifier(DefinitionEntry {
            id: modifier.clone(),
            schema: SchemaState::Known(ModifierSchema {
                declarations: declarations.clone(),
            }),
        }));
    for d in &mut schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(entry) = d {
            let SchemaState::Known(s) = &mut entry.schema else {
                continue;
            };
            s.modifiers.members.push(modifier.clone());
            declarations.parameters.closure = partial(subject(&entry.id));
            s.declarations.parameters = declarations.parameters.clone();
        }
    }
    rebind_quality_schema(&mut f.artifacts, &mut f.policy, schema);
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions,
        templates,
        ..
    }) = &mut f.policy.equipment_membership
    else {
        panic!()
    };
    *definitions = f.artifacts.schema.identity().clone();
    let template = templates[0].template.clone();
    let mut input = f.items.input().clone();
    input.definitions = f.artifacts.schema.identity().clone();
    input.rules.extend([
        ItemLineRule {
            id: key("implicit-count"),
            pattern: vec![ItemPatternPart::Literal("Implicits: 0".into())],
            captures: vec![],
            emissions: vec![ItemEmission::Metadata {
                role: key("implicit-count"),
            }],
        },
        ItemLineRule {
            id: key("one-member"),
            pattern: vec![ItemPatternPart::Literal("One authored member".into())],
            captures: vec![],
            emissions: vec![ItemEmission::Modifier {
                definition: modifier,
                rolls: vec![],
            }],
        },
    ]);
    f.items = OwnedItemLinePolicy::new(input, &f.artifacts.schema, Default::default()).unwrap();
    let mut source = f.item_source.input().clone();
    source.schema_version = OWNED_ITEM_SOURCE_CONDITION_POLICY_VERSION;
    source.item_lines = *f.items.identity();
    source.dialect = ItemSourceDialect::PobExportedSingleTextConditionsV1 {
        flag_bindings: vec![],
        metadata_rules: vec![key("rarity")],
        single_modifier_conditions: vec![ItemSourceConditionalMember {
            rule: key("one-member"),
            all: vec![ItemSourceCondition::NoSourceTags],
        }],
    };
    source.rule_layouts.extend([
        ItemRuleSourceLayout {
            rule: key("implicit-count"),
            role: ItemRuleSourceRole::Header,
        },
        ItemRuleSourceLayout {
            rule: key("one-member"),
            role: ItemRuleSourceRole::Unresolved,
        },
    ]);
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    f.policy.item_modifier_membership =
        Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions: f.artifacts.schema.identity().clone(),
            item_lines: *f.items.identity(),
            item_source: *f.item_source.identity(),
            templates: vec![OrdinarySingletonBase {
                template,
                generated_members: OrdinaryBaseMembers::NoBuffImplicitRuneOrClassMembers,
            }],
            modifier_rules: vec![key("one-member")],
        });
    f
}

fn closed(result: &NormalizedImport) -> bool {
    let Some(i) = result.draft().input().items.members.first() else {
        return false;
    };
    matches!(i.modifiers.completion, DraftListCompletion::Complete)
        && i.modifier_order.to_resolved()
            == Some(i.modifiers.members.iter().map(|v| v.id).collect())
}

#[test]
fn checked_conditional_singleton_closes_only_physical_modifier_facets() {
    let f = member_fixture();
    let input = xml(SINGLE, SLOT);
    let result = run(&input, &f);
    assert!(closed(&result));
    let i = &result.draft().input().items.members[0];
    assert_eq!(i.modifiers.members.len(), 1);
    assert!(
        matches!(i.parameters.completion,DraftListCompletion::Pending{ref code,..} if code.as_str()=="item-parameters-not-converted")
    );
    let template = i.template.to_resolved().unwrap();
    let SchemaLookup::Known(s) = f.artifacts.schema.definition(&template) else {
        panic!()
    };
    assert!(!s.declarations.parameters.is_complete());
    assert!(matches!(
        result.sidecar().item_texts[0].attribution.layout,
        ItemLayoutStatus::Proven
    ));
    assert!(matches!(
        result.sidecar().item_texts[0].lines[1].outcome,
        ItemLineOutcome::Pending {
            reason: ItemLinePending::SourcePresentation,
            ..
        }
    ));
    // Identical fresh source/allocator yields deterministic exact outputs.
    assert_eq!(
        serde_json::to_value(result.draft().input()).unwrap(),
        serde_json::to_value(run(&input, &f).draft().input()).unwrap()
    );
}

#[test]
fn omission_preserves_wire_and_original_pending_behavior() {
    let mut f = member_fixture();
    f.policy.item_modifier_membership = None;
    let bytes = serde_json::to_value(&f.policy).unwrap();
    assert!(bytes.get("item_modifier_membership").is_none());
    let decoded: NormalizationPolicy = serde_json::from_value(bytes.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), bytes);
    let r = run(&xml(SINGLE, SLOT), &f);
    assert!(!closed(&r));
    let i = &r.draft().input().items.members[0];
    assert!(matches!(
        i.modifiers.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(i.modifier_order, DraftField::Pending(_)));
    assert_eq!(i.modifiers.members.len(), 1);
}

#[test]
fn complete_layout_and_known_member_are_not_sufficient_without_opt_in_scope() {
    let mut f = member_fixture();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 { templates, .. }) =
        &mut f.policy.item_modifier_membership
    else {
        panic!()
    };
    templates.clear();
    let r = run(&xml(SINGLE, SLOT), &f);
    assert!(matches!(
        r.sidecar().item_texts[0].attribution.layout,
        ItemLayoutStatus::Proven
    ));
    assert!(!closed(&r));
    assert_eq!(
        r.draft().input().items.members[0].modifiers.members.len(),
        1
    );
}

#[test]
fn ambiguity_extra_members_augments_and_loader_overrides_keep_membership_pending() {
    let f = member_fixture();
    for raw in [
        SINGLE.replace("One authored member", ""),
        format!("{SINGLE}\nOne authored member"),
        format!("{SINGLE}\nUnconverted semantic member"),
        SINGLE.replace("One authored member", "{enchant}One authored member"),
        SINGLE.replace("Implicits: 0", "Implicits: 1"),
        SINGLE.replace("Test Title", "Legacy Jewel Title"),
        SINGLE.replace("Implicits: 0", "Rune: Unknown\nImplicits: 0"),
        SINGLE.replace("Ordinary Base", "Ordinary Base\nSocket Base"),
    ] {
        assert!(!closed(&run(&xml(&raw, SLOT), &f)), "{raw}");
    }
    for input in [
        xml(SINGLE, SLOT).replace("<Item id=\"1\">", "<Item id=\"1\" variant=\"1\">"),
        xml(SINGLE, SLOT).replace("</Item>", "<Other/></Item>"),
        xml(SINGLE, SLOT).replace("<Items ", "<Items xmlns=\"urn:x\" "),
        xml(SINGLE, SLOT).replace("</Item>", "<ModRange id=\"1\" range=\"bad\"/></Item>"),
        xml(SINGLE, SLOT).replace("</Items>", "<Item id=\"01\">Other</Item></Items>"),
    ] {
        let r = run(&input, &f);
        assert!(!closed(&r), "{input}");
    }
    let overlay = xml(SINGLE, SLOT).replace("</Item>", "<ModRange id=\"1\" range=\"0.5\"/></Item>");
    assert!(closed(&run(&overlay, &f)));
}

#[test]
fn all_artifact_commitments_and_finite_policy_members_are_validated() {
    let f = member_fixture();
    for field in [
        "definitions",
        "item_lines",
        "item_source",
        "duplicate_template",
        "duplicate_rule",
        "unknown_rule",
        "missing_augments",
    ] {
        let mut p = f.policy.clone();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions,
            item_lines,
            item_source,
            templates,
            modifier_rules,
        }) = &mut p.item_modifier_membership
        else {
            panic!()
        };
        match field {
            "definitions" => definitions.release.push_str("-stale"),
            "item_lines" => *item_lines = *f.item_source.identity(),
            "item_source" => *item_source = *f.items.identity(),
            "duplicate_template" => templates.push(templates[0].clone()),
            "duplicate_rule" => modifier_rules.push(modifier_rules[0].clone()),
            "unknown_rule" => modifier_rules[0] = key("missing"),
            "missing_augments" => p.equipment_membership = None,
            _ => unreachable!(),
        }
        assert!(
            normalize(&xml(SINGLE, SLOT), &f, &p, Default::default()).is_err(),
            "{field}"
        );
    }
    let mut p = f.policy.clone();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        templates,
        modifier_rules,
        ..
    }) = &mut p.item_modifier_membership
    else {
        panic!()
    };
    templates.clear();
    modifier_rules.clear();
    assert!(!closed(
        &normalize(&xml(SINGLE, SLOT), &f, &p, Default::default()).unwrap()
    ));
}

#[test]
fn incomplete_rolls_or_schema_filtered_member_cannot_be_declared_complete() {
    for remove_membership in [false, true] {
        let mut f = member_fixture();
        let mut schema = f.artifacts.schema.input().clone();
        for d in &mut schema.definitions {
            match d {
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
        let mut i = f.items.input().clone();
        i.schema_version = OWNED_ITEM_LINE_POLICY_V4;
        i.definitions = f.artifacts.schema.identity().clone();
        f.items = OwnedItemLinePolicy::new(i, &f.artifacts.schema, Default::default()).unwrap();
        let mut s = f.item_source.input().clone();
        s.item_lines = *f.items.identity();
        f.item_source =
            ItemSourceLayoutPolicy::new(s, &f.items, &f.artifacts.schema, Default::default())
                .unwrap();
        let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. }) =
            &mut f.policy.equipment_membership
        else {
            panic!()
        };
        *definitions = f.artifacts.schema.identity().clone();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
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
        let r = run(&xml(SINGLE, SLOT), &f);
        assert!(!closed(&r));
    }
}

#[test]
fn work_and_policy_size_limits_still_fail_closed() {
    let f = member_fixture();
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
        assert!(normalize(&xml(SINGLE, SLOT), &f, &f.policy, limits).is_err());
    }
}
