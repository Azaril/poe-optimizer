//! Generic category-bound physical inputs, independent of production base IDs.
use super::*;

fn census_raw() -> String {
    RAW.replace("Sockets: S S\nRune: None\nRune: None\n", "")
        .replace("Crafted: true", "Charm Slots: 9\nCrafted: true")
        .replace("Implicits: 0", "Implicits: 2")
        + "\nOne authored member\nOne authored member"
}

fn census_fixture() -> Fixture {
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
    lines.rules.extend([
        ItemLineRule {
            id: key("census-implicit-count"),
            pattern: vec![ItemPatternPart::Literal("Implicits: 2".into())],
            captures: vec![],
            emissions: vec![ItemEmission::Metadata {
                role: key("census-count"),
            }],
        },
        ItemLineRule {
            id: key("reviewed-derived-header"),
            pattern: vec![
                ItemPatternPart::Literal("Charm Slots: ".into()),
                ItemPatternPart::NumericCapture {
                    capture: key("observed-amount"),
                    syntax: DecimalSyntax::Integer,
                    sign: ItemNumericSign::Forbidden,
                },
            ],
            captures: vec![ItemCapture {
                id: key("observed-amount"),
                codec: ItemCaptureCodec::Value(ValueCodecInput {
                    namespace: ns(),
                    whitespace: WhitespacePolicy::Exact,
                    codec: ValueCodecKind::Integer {
                        syntax: DecimalSyntax::Integer,
                    },
                }),
            }],
            emissions: vec![ItemEmission::Metadata {
                role: key("derived-observation"),
            }],
        },
    ]);
    f.items = OwnedItemLinePolicy::new(lines, &f.artifacts.schema, Default::default()).unwrap();
    let mut source = f.item_source.input().clone();
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 {
        flag_bindings,
        metadata_rules,
        mut single_modifier_conditions,
    } = source.dialect
    else {
        panic!()
    };
    single_modifier_conditions[0].all = vec![ItemSourceCondition::NoSourceScalingTags];
    source.schema_version = OWNED_ITEM_SOURCE_OBSERVATION_POLICY_VERSION;
    source.item_lines = *f.items.identity();
    source.dialect = ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings,
        metadata_rules,
        single_modifier_conditions,
        preamble_observations: vec![ItemSourcePreambleObservation {
            rule: key("reviewed-derived-header"),
            field: key("derived-display-field"),
            templates: vec![template.clone()],
        }],
    };
    source.rule_layouts.extend([
        ItemRuleSourceLayout {
            rule: key("census-implicit-count"),
            role: ItemRuleSourceRole::Header,
        },
        ItemRuleSourceLayout {
            rule: key("reviewed-derived-header"),
            role: ItemRuleSourceRole::Unresolved,
        },
    ]);
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
    f.policy.item_modifier_membership = Some(
        ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            definitions,
            item_lines: *f.items.identity(),
            item_source: *f.item_source.identity(),
            templates: vec![],
            paired_templates: vec![],
            modifier_rules,
            census_templates: vec![OrdinaryMemberCensusBase {
                template,
                generated_members: OrdinaryMemberGeneration::NoBuffEnchantRuneOrClassMembers,
                implicit_members: 2,
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
    templates[0].construction = OrdinaryItemConstruction::FreshRareSavedCategoryCensusV3 {
        derived_observations: vec![OrdinaryItemDerivedObservation {
            rule: key("reviewed-derived-header"),
            capture: key("observed-amount"),
            kind: OrdinaryItemDerivedObservationKind::CharmSlots,
        }],
    };
    f
}

fn construction(policy: &mut NormalizationPolicy) -> &mut OrdinaryItemConstruction {
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut policy.item_parameter_inputs
    else {
        panic!()
    };
    &mut templates[0].construction
}

#[test]
fn declared_category_inputs_consume_reviewed_observation_without_raw_capacity_alias() {
    let f = census_fixture();
    for raw in [
        census_raw(),
        census_raw().replace("Charm Slots: 9", "Charm Slots: 0"),
        census_raw().replace("Charm Slots: 9\n", ""),
    ] {
        let result = run(&xml(&raw, SLOT), &f);
        assert!(closed(&result));
        assert!(complete(&result));
        let item = &result.draft().input().items.members[0];
        assert_eq!(item.modifiers.members.len(), 3);
        assert_eq!(item.parameters.members.len(), 4);
        let evidence = result.sidecar().item_texts[0]
            .parameter_inputs
            .as_ref()
            .unwrap();
        assert_eq!(evidence.len(), 4);
        assert!(
            evidence
                .iter()
                .any(|e| e.origin == ItemParameterInputOrigin::AbsentSocketHeader
                    && e.value == ParameterValue::Integer(BoundedInteger::new(0).unwrap()))
        );
        assert!(!evidence.iter().any(|e| matches!(&e.origin, ItemParameterInputOrigin::Header {rule, ..} if rule == &key("reviewed-derived-header"))));
        let SchemaLookup::Known(schema) = f
            .artifacts
            .schema
            .definition(&item.template.to_resolved().unwrap())
        else {
            panic!()
        };
        assert!(!schema.declarations.parameters.is_complete());
    }
    let raw = census_raw().replace(
        "LevelReq: 0",
        "Sockets: S S\nRune: None\nRune: None\nLevelReq: 77",
    );
    let result = run(&xml(&raw, SLOT), &f);
    assert!(complete(&result));
    let evidence = result.sidecar().item_texts[0]
        .parameter_inputs
        .as_ref()
        .unwrap();
    assert!(evidence.iter().any(|e| matches!(
        e.origin,
        ItemParameterInputOrigin::EmptySocketCapacity { .. }
    ) && e.value
        == ParameterValue::Integer(BoundedInteger::new(2).unwrap())));
}

#[test]
fn malformed_duplicate_unreviewed_and_stateful_headers_do_not_complete_raw_inputs() {
    let f = census_fixture();
    for replacement in [
        "Charm Slots: Nope",
        "Charm Slots: -1",
        "Charm Slots: +1",
        "Charm Slots: 1.5",
        "Charm Slots: 09",
        "Charm Slots: 9007199254740992",
        "Charm Slots: 9\nCharm Slots: 2",
        "Spirit: 2",
        "Corrupted",
        "Twice Corrupted",
        "Quality (Life Modifiers): 20%",
        "Requires Level: 77",
    ] {
        let raw = census_raw().replace("Charm Slots: 9", replacement);
        let result = run(&xml(&raw, SLOT), &f);
        assert!(!complete(&result), "{replacement}");
        assert!(result.sidecar().item_texts[0].parameter_inputs.is_none());
    }
    let result = run(
        &xml(
            &census_raw().replace("Charm Slots: 9", "Charm Slots: 09"),
            SLOT,
        ),
        &f,
    );
    assert!(closed(&result), "independent membership with numeric alias");
    assert!(matches!(
        result.sidecar().item_texts[0].attribution.layout,
        ItemLayoutStatus::Proven
    ));
    assert!(!complete(&result));
}

#[test]
fn exact_construction_and_observation_bindings_are_validated_offline() {
    let f = census_fixture();
    for case in 0..6 {
        let mut policy = f.policy.clone();
        if case < 2 {
            *construction(&mut policy) = if case == 0 {
                OrdinaryItemConstruction::FreshRareSavedAffixesV1
            } else {
                OrdinaryItemConstruction::FreshRareSavedImplicitExplicitV2
            };
        } else {
            let OrdinaryItemConstruction::FreshRareSavedCategoryCensusV3 {
                derived_observations,
            } = construction(&mut policy)
            else {
                panic!()
            };
            match case {
                2 => derived_observations[0].rule = key("crafted"),
                3 => derived_observations[0].capture = key("wrong-capture"),
                4 => derived_observations.push(derived_observations[0].clone()),
                _ => derived_observations[0].rule = key("rarity"),
            }
        }
        assert!(
            normalize(&xml(&census_raw(), SLOT), &f, &policy, Default::default()).is_err(),
            "case {case}"
        );
    }
    let mut policy = f.policy.clone();
    let OrdinaryItemConstruction::FreshRareSavedCategoryCensusV3 {
        derived_observations,
    } = construction(&mut policy)
    else {
        panic!()
    };
    derived_observations.clear();
    let result = normalize(&xml(&census_raw(), SLOT), &f, &policy, Default::default()).unwrap();
    assert!(closed(&result));
    assert!(
        !complete(&result),
        "a generic preamble observation is not raw-input authority"
    );
}

#[test]
fn census_mismatch_missing_member_and_unsafe_augments_withhold_parameter_authority() {
    let f = census_fixture();
    for raw in [
        census_raw().replace("Implicits: 2", "Implicits: 0"),
        census_raw().replacen("One authored member\n", "", 1),
        census_raw() + "\nOne authored member",
        census_raw().replace("LevelReq: 0", "Sockets: S\nRune: Unknown\nLevelReq: 0"),
        census_raw().replace("LevelReq: 0", "LevelReq: 101"),
    ] {
        let result = run(&xml(&raw, SLOT), &f);
        assert!(!complete(&result));
        assert!(result.sidecar().item_texts[0].parameter_inputs.is_none());
    }
}

#[test]
fn legacy_construction_wire_forms_remain_exact_and_new_observation_fields_are_closed() {
    for (value, wire) in [
        (
            OrdinaryItemConstruction::FreshRareSavedAffixesV1,
            "\"fresh_rare_saved_affixes_v1\"",
        ),
        (
            OrdinaryItemConstruction::FreshRareSavedImplicitExplicitV2,
            "\"fresh_rare_saved_implicit_explicit_v2\"",
        ),
    ] {
        assert_eq!(serde_json::to_string(&value).unwrap(), wire);
        assert_eq!(
            serde_json::from_str::<OrdinaryItemConstruction>(wire).unwrap(),
            value
        );
    }
    let mut f = census_fixture();
    let value = construction(&mut f.policy).clone();
    let mut wire = serde_json::to_value(&value).unwrap();
    assert_eq!(
        serde_json::from_value::<OrdinaryItemConstruction>(wire.clone()).unwrap(),
        value
    );
    wire["fresh_rare_saved_category_census_v3"]["unreviewed_setter"] = serde_json::json!(true);
    assert!(serde_json::from_value::<OrdinaryItemConstruction>(wire).is_err());
}

#[test]
fn category_raw_proof_respects_shared_work_budget_and_policy_omission() {
    let mut f = census_fixture();
    assert!(
        normalize(
            &xml(&census_raw(), SLOT),
            &f,
            &f.policy,
            NormalizationLimits {
                max_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    f.policy.item_parameter_inputs = None;
    let result = run(&xml(&census_raw(), SLOT), &f);
    assert!(
        !closed(&result),
        "missing required template inputs retain the existing membership obligation"
    );
    assert!(!complete(&result));
    assert!(result.sidecar().item_texts[0].parameter_inputs.is_none());
}

#[test]
fn derived_observation_requires_exact_numeric_grammar_and_template_review() {
    for change_prefix in [false, true] {
        let mut f = census_fixture();
        let mut source = f.item_source.input().clone();
        if change_prefix {
            let mut input = f.items.input().clone();
            let rule = input
                .rules
                .iter_mut()
                .find(|r| r.id == key("reviewed-derived-header"))
                .unwrap();
            rule.pattern[0] = ItemPatternPart::Literal("Spirit: ".into());
            f.items =
                OwnedItemLinePolicy::new(input, &f.artifacts.schema, Default::default()).unwrap();
        } else {
            let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
                preamble_observations,
                ..
            } = &mut source.dialect
            else {
                panic!()
            };
            preamble_observations.clear();
        }
        source.item_lines = *f.items.identity();
        f.item_source =
            ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
                .unwrap();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            item_lines,
            item_source,
            ..
        }) = &mut f.policy.item_modifier_membership
        else {
            panic!()
        };
        *item_lines = *f.items.identity();
        *item_source = *f.item_source.identity();
        let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
            item_lines,
            item_source,
            ..
        }) = &mut f.policy.item_parameter_inputs
        else {
            panic!()
        };
        *item_lines = *f.items.identity();
        *item_source = *f.item_source.identity();
        assert!(normalize(&xml(&census_raw(), SLOT), &f, &f.policy, Default::default()).is_err());
    }
}
