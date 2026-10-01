//! Scoped raw-input proof uses the ordinary source/member fixture unchanged.
use super::*;
use poe_optimizer_import::owned_value::{
    DecimalSyntax, OptionToken, ValueCodecInput, ValueCodecKind,
};

#[path = "owned_item_implicit_explicit.rs"]
mod implicit_explicit_tests;

#[path = "owned_item_category_inputs.rs"]
mod category_input_tests;

#[path = "owned_imported_item_construction.rs"]
mod imported_construction_tests;

const RAW: &str = "Rarity: RARE\nTest Title\nOrdinary Base\nCrafted: true\nPrefix: None\nPrefix: None\nPrefix: None\nSuffix: None\nSuffix: None\nSuffix: None\nSockets: S S\nRune: None\nRune: None\nLevelReq: 0\nImplicits: 0\nOne authored member";

fn raw_fixture() -> Fixture {
    let mut f = member_fixture();
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 { templates, .. }) =
        &f.policy.item_modifier_membership
    else {
        panic!()
    };
    let template = templates[0].template.clone();
    let rarity = f
        .artifacts
        .registry
        .allocate_definition::<OptionDefinition>()
        .unwrap();
    let owner = SlotOwnerDefId::ItemTemplate(template.clone());
    let slots: Vec<_> = (0..4)
        .map(|_| {
            f.artifacts
                .registry
                .allocate_slot::<ParameterSlotDefinition>(owner.clone())
                .unwrap()
        })
        .collect();
    let mut schema = f.artifacts.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Option(DefinitionEntry {
            id: rarity.clone(),
            schema: SchemaState::Known(OptionSchema {}),
        }));
    let types = [
        ValueSchema::Option {
            allowed: DeclaredSet::complete(vec![rarity.clone()]),
        },
        ValueSchema::Boolean,
        ValueSchema::Integer(IntegerRange {
            minimum: BoundedInteger::new(0).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        }),
        ValueSchema::Integer(IntegerRange {
            minimum: BoundedInteger::new(0).unwrap(),
            maximum: BoundedInteger::new(64).unwrap(),
        }),
    ];
    for (index, (slot, value)) in slots.iter().zip(types).enumerate() {
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value,
                    presence: if index == 2 {
                        SlotPresence::OptionalOnce
                    } else {
                        SlotPresence::RequiredOnce
                    },
                    sites: vec![ParameterSite::ItemParameter],
                }),
            }));
    }
    for d in &mut schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(e) = d
            && e.id == template
        {
            let SchemaState::Known(s) = &mut e.schema else {
                panic!()
            };
            s.declarations.parameters.members = slots.clone();
        }
    }
    rebind_quality_schema(&mut f.artifacts, &mut f.policy, schema);
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions, .. }) =
        &mut f.policy.equipment_membership
    else {
        panic!()
    };
    *definitions = f.artifacts.schema.identity().clone();
    let mut input = f.items.input().clone();
    input.definitions = f.artifacts.schema.identity().clone();
    input.rules.retain(|r| r.id != key("rarity"));
    for (id, prefix) in [
        ("rarity", "Rarity: "),
        ("crafted", "Crafted: "),
        ("prefix", "Prefix: "),
        ("suffix", "Suffix: "),
        ("sockets", "Sockets: "),
        ("rune", "Rune: "),
        ("level-requirement", "LevelReq: "),
    ] {
        input.rules.push(ItemLineRule {
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
    f.items = OwnedItemLinePolicy::new(input, &f.artifacts.schema, Default::default()).unwrap();
    let mut source = f.item_source.input().clone();
    source.item_lines = *f.items.identity();
    for id in [
        "crafted",
        "prefix",
        "suffix",
        "sockets",
        "rune",
        "level-requirement",
    ] {
        source.rule_layouts.push(ItemRuleSourceLayout {
            rule: key(id),
            role: ItemRuleSourceRole::Header,
        });
    }
    f.item_source =
        ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
            .unwrap();
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
    f.policy.item_parameter_inputs = Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
        definitions: f.artifacts.schema.identity().clone(),
        item_lines: *f.items.identity(),
        item_source: *f.item_source.identity(),
        templates: vec![OrdinaryItemParameterInputs {
            template,
            header_inputs: vec![
                ItemParameterHeaderInput {
                    rule: key("rarity"),
                    capture: key("text"),
                    codec: ValueCodecInput {
                        namespace: ns(),
                        whitespace: WhitespacePolicy::Exact,
                        codec: ValueCodecKind::Option {
                            tokens: vec![OptionToken {
                                token: "RARE".into(),
                                value: rarity,
                            }],
                        },
                    },
                    slot: slots[0].clone(),
                },
                ItemParameterHeaderInput {
                    rule: key("level-requirement"),
                    capture: key("text"),
                    codec: ValueCodecInput {
                        namespace: ns(),
                        whitespace: WhitespacePolicy::Exact,
                        codec: ValueCodecKind::Integer {
                            syntax: DecimalSyntax::Integer,
                        },
                    },
                    slot: slots[2].clone(),
                },
            ],
            corruption_slot: slots[1].clone(),
            capacity_slot: slots[3].clone(),
            construction: OrdinaryItemConstruction::FreshRareSavedAffixesV1,
        }],
    });
    f
}
fn complete(r: &NormalizedImport) -> bool {
    r.draft()
        .input()
        .items
        .members
        .first()
        .is_some_and(|i| matches!(i.parameters.completion, DraftListCompletion::Complete))
}
#[test]
fn scoped_raw_fields_complete_physical_inventory_without_closing_static_schema() {
    let f = raw_fixture();
    for (text, level) in [
        (RAW.to_string(), 0),
        (RAW.replace("LevelReq: 0", "LevelReq: 77"), 77),
        (RAW.replace("Crafted: true", "Crafted: false"), 0),
    ] {
        let r = run(&xml(&text, SLOT), &f);
        assert!(closed(&r));
        assert!(complete(&r), "{:?}", r.sidecar().item_texts[0].issues);
        let item = &r.draft().input().items.members[0];
        assert_eq!(item.parameters.members.len(), 4);
        let proof = r.sidecar().item_texts[0].parameter_inputs.as_ref().unwrap();
        assert_eq!(proof.len(), 4);
        assert!(proof.iter().any(|v| v.value
            == ParameterValue::Integer(BoundedInteger::new(level).unwrap())
            && matches!(v.origin, ItemParameterInputOrigin::Header { .. })));
        assert!(
            proof
                .iter()
                .any(|v| v.value == ParameterValue::Boolean(false)
                    && v.origin == ItemParameterInputOrigin::FreshUncorrupted)
        );
        assert!(proof.iter().any(|v| v.value
            == ParameterValue::Integer(BoundedInteger::new(2).unwrap())
            && matches!(
                v.origin,
                ItemParameterInputOrigin::EmptySocketCapacity { .. }
            )));
        let SchemaLookup::Known(s) = f
            .artifacts
            .schema
            .definition(&item.template.to_resolved().unwrap())
        else {
            panic!()
        };
        assert!(!s.declarations.parameters.is_complete());
    }
}
#[test]
fn missing_alias_duplicate_malformed_state_and_augments_never_complete_parameters() {
    let f = raw_fixture();
    for raw in [
        RAW.replace("LevelReq: 0\n", ""),
        RAW.replace("LevelReq: 0", "Requires Level: 0"),
        RAW.replace("LevelReq: 0", "Level: 0"),
        RAW.replace("LevelReq: 0", "LevelReq: 0\nLevelReq: 1"),
        RAW.replace("LevelReq: 0", "LevelReq: nope"),
        RAW.replace("LevelReq: 0", "LevelReq: 101"),
        RAW.replace("LevelReq: 0", "LevelReq: -1"),
        RAW.replace("LevelReq: 0", "LevelReq: 0.5"),
        RAW.replace("Rarity: RARE", "Rarity: RARE\nRarity: RARE"),
        RAW.replace("Rarity: RARE", "Rarity: MAGIC"),
        RAW.replace("Crafted: true\n", ""),
        RAW.replace("Crafted: true", "Crafted: true\nCrafted: false"),
        RAW.replace("Implicits: 0", "Corrupted\nImplicits: 0"),
        RAW.replace("Implicits: 0", "Twice Corrupted\nImplicits: 0"),
        RAW.replace("Implicits: 0", "Mirrored\nImplicits: 0"),
        RAW.replace("Rune: None", "Rune: Unknown"),
        RAW.replace(
            "One authored member",
            "One authored member\nOne authored member",
        ),
        RAW.replace("Implicits: 0", "Unknown: input\nImplicits: 0"),
    ] {
        let r = run(&xml(&raw, SLOT), &f);
        assert!(!complete(&r), "unexpected complete input: {raw}");
        assert!(
            r.sidecar()
                .item_texts
                .iter()
                .all(|t| t.parameter_inputs.is_none())
        );
    }
}
#[test]
fn bad_raw_projection_does_not_prevent_independent_singleton_membership() {
    let f = raw_fixture();
    let r = run(
        &xml(&RAW.replace("LevelReq: 0", "LevelReq: nope"), SLOT),
        &f,
    );
    assert!(closed(&r));
    assert!(!complete(&r));
}
#[test]
fn omission_and_unconfigured_templates_keep_historical_parameter_obligations() {
    let mut f = raw_fixture();
    f.policy.item_parameter_inputs = None;
    assert!(
        serde_json::to_value(&f.policy)
            .unwrap()
            .get("item_parameter_inputs")
            .is_none()
    );
    let r = run(&xml(RAW, SLOT), &f);
    assert!(!complete(&r));
    assert!(r.sidecar().item_texts[0].parameter_inputs.is_none());
    let mut f = raw_fixture();
    let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) =
        &mut f.policy.item_parameter_inputs
    else {
        panic!()
    };
    templates.clear();
    assert!(!complete(&run(&xml(RAW, SLOT), &f)));
}
#[test]
fn stale_bindings_and_foreign_or_duplicate_projections_reject_offline() {
    let f = raw_fixture();
    for index in 0..7 {
        let mut p = f.policy.clone();
        let Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
            definitions,
            item_lines,
            item_source,
            templates,
        }) = &mut p.item_parameter_inputs
        else {
            panic!()
        };
        match index {
            0 => definitions.content_sha256 = "0".repeat(64),
            1 => *item_lines = "0".repeat(64).parse().unwrap(),
            2 => *item_source = "0".repeat(64).parse().unwrap(),
            3 => templates[0].header_inputs[1].slot = templates[0].corruption_slot.clone(),
            4 => templates[0].header_inputs[0].rule = key("crafted"),
            5 => templates[0].header_inputs[0].capture = key("missing"),
            _ => {
                let copy = templates[0].clone();
                templates.push(copy);
            }
        }
        assert!(
            normalize(&xml(RAW, SLOT), &f, &p, Default::default()).is_err(),
            "invalid case {index}"
        );
    }
}
#[test]
fn proof_work_is_bounded_before_raw_input_completion() {
    let f = raw_fixture();
    assert!(
        normalize(
            &xml(RAW, SLOT),
            &f,
            &f.policy,
            NormalizationLimits {
                max_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn declared_preamble_observations_cannot_hide_raw_state_setters_or_aliases() {
    for text in ["Corrupted", "Requires Level: 77", "Level: 77"] {
        let mut f = raw_fixture();
        let template = match &f.policy.item_parameter_inputs {
            Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 { templates, .. }) => {
                templates[0].template.clone()
            }
            _ => panic!(),
        };
        let mut lines = f.items.input().clone();
        lines.rules.push(ItemLineRule {
            id: key("misclassified-observation"),
            pattern: vec![ItemPatternPart::Literal(text.into())],
            captures: vec![],
            emissions: vec![ItemEmission::Metadata {
                role: key("display"),
            }],
        });
        f.items = OwnedItemLinePolicy::new(lines, &f.artifacts.schema, Default::default()).unwrap();
        let mut source = f.item_source.input().clone();
        let ItemSourceDialect::PobExportedSingleTextConditionsV1 {
            flag_bindings,
            metadata_rules,
            single_modifier_conditions,
        } = source.dialect
        else {
            panic!()
        };
        source.schema_version = OWNED_ITEM_SOURCE_OBSERVATION_POLICY_VERSION;
        source.item_lines = *f.items.identity();
        source.dialect = ItemSourceDialect::PobExportedSingleTextObservationsV1 {
            flag_bindings,
            metadata_rules,
            single_modifier_conditions,
            preamble_observations: vec![ItemSourcePreambleObservation {
                rule: key("misclassified-observation"),
                field: key("misclassified-field"),
                templates: vec![template],
            }],
        };
        source.rule_layouts.push(ItemRuleSourceLayout {
            rule: key("misclassified-observation"),
            role: ItemRuleSourceRole::Unresolved,
        });
        f.item_source =
            ItemSourceLayoutPolicy::new(source, &f.items, &f.artifacts.schema, Default::default())
                .unwrap();
        let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
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
        let raw = RAW.replace("Crafted: true", &format!("{text}\nCrafted: true"));
        let r = run(&xml(&raw, SLOT), &f);
        assert!(matches!(
            r.sidecar().item_texts[0].attribution.layout,
            ItemLayoutStatus::Proven
        ));
        assert!(!complete(&r), "observation must not erase {text}");
        assert!(r.sidecar().item_texts[0].parameter_inputs.is_none());
    }
    let f = raw_fixture();
    assert!(!complete(&run(
        &xml(&RAW.replace("Test Title", "Unidentified"), SLOT),
        &f
    )));
}
