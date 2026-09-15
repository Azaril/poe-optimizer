//! Explicit per-line property facts are import inputs, never source-derived defaults.
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{owned_item_lines::*, owned_value::*};
use std::collections::BTreeMap;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("property-test", "v1").unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn modifier() -> ModifierDefId {
    ModifierDefId::parse(ns(), "modifier").unwrap()
}
fn slot(s: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Modifier(modifier()),
        slot: ParameterSlotDefId::parse(ns(), s).unwrap(),
    }
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![slot("amount"), slot("enabled")]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn raw_schema() -> SchemaPackageInput {
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("release"),
        semantics_version: key("input-schema"),
        definitions: vec![DefinitionDescriptor::Modifier(DefinitionEntry {
            id: modifier(),
            schema: SchemaState::Known(ModifierSchema {
                declarations: declarations(),
            }),
        })],
        slots: vec![
            SlotDescriptor::Parameter(DefinitionEntry {
                id: slot("amount"),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Integer(IntegerRange {
                        minimum: BoundedInteger::new(-1000).unwrap(),
                        maximum: BoundedInteger::new(1000).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                }),
            }),
            SlotDescriptor::Parameter(DefinitionEntry {
                id: slot("enabled"),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                }),
            }),
        ],
    }
}
fn schema() -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(raw_schema(), OwnedSchemaLimits::default()).unwrap()
}
fn property() -> ItemLineValue {
    ItemLineValue::Property {
        property: key("tag-enabled"),
    }
}
fn input(schema: &OwnedDefinitionSchemaPackage) -> ItemLinePolicyInput {
    ItemLinePolicyInput {
        schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
        namespace: ns(),
        version: key("policy"),
        definitions: schema.identity().clone(),
        whitespace: WhitespacePolicy::Exact,
        rules: vec![ItemLineRule {
            id: key("roll"),
            pattern: vec![
                ItemPatternPart::Literal("Roll: ".into()),
                ItemPatternPart::Capture(key("amount")),
            ],
            captures: vec![ItemCapture {
                id: key("amount"),
                codec: ItemCaptureCodec::Value(ValueCodecInput {
                    namespace: ns(),
                    whitespace: WhitespacePolicy::Exact,
                    codec: ValueCodecKind::Integer {
                        syntax: DecimalSyntax::Scientific,
                    },
                }),
            }],
            emissions: vec![ItemEmission::Modifier {
                definition: modifier(),
                rolls: vec![
                    // Deliberately first: all captures must decode before this property resolves.
                    ItemRollTemplate {
                        slot: slot("enabled"),
                        value: property(),
                    },
                    ItemRollTemplate {
                        slot: slot("amount"),
                        value: ItemLineValue::Capture(key("amount")),
                    },
                ],
            }],
        }],
    }
}
fn policy(limits: ItemLineLimits) -> OwnedItemLinePolicy {
    let s = schema();
    OwnedItemLinePolicy::new(input(&s), &s, limits).unwrap()
}
fn row<'a>(
    index: usize,
    text: &'a str,
    properties: Option<&'a BTreeMap<OwnedDefinitionKey, bool>>,
) -> ItemLineInput<'a> {
    ItemLineInput {
        index,
        text,
        range_fraction: None,
        properties,
    }
}
fn reason<'a>(line: &'a ItemLineEvidence<'_>) -> &'a ItemLinePending {
    let ItemLineOutcome::Pending { reason, candidates } = &line.outcome else {
        panic!("expected pending")
    };
    assert_eq!(candidates, &[key("roll")]);
    reason
}
fn emissions<'a>(line: &'a ItemLineEvidence<'_>) -> &'a [ConvertedItemEmission] {
    let ItemLineOutcome::Known { emissions, .. } = &line.outcome else {
        panic!("expected admitted line")
    };
    emissions
}

#[test]
fn explicit_false_and_true_are_known_and_preserve_each_modifier_occurrence() {
    let s = schema();
    let mut raw = input(&s);
    let duplicate = raw.rules[0].emissions[0].clone();
    raw.rules[0].emissions.push(duplicate);
    let p = OwnedItemLinePolicy::new(raw, &s, ItemLineLimits::default()).unwrap();
    for enabled in [false, true] {
        let facts = BTreeMap::from([(key("tag-enabled"), enabled)]);
        let result = p.convert_lines([row(1, "Roll: 49", Some(&facts))]).unwrap();
        let known = emissions(&result.lines[0]);
        assert_eq!(known.len(), 2);
        for emission in known {
            let ConvertedItemEmission::Modifier { definition, rolls } = emission else {
                panic!("modifier")
            };
            assert_eq!(definition, &modifier());
            assert_eq!(rolls[0].slot, slot("enabled"));
            assert_eq!(rolls[0].value, ParameterValue::Boolean(enabled));
            assert_eq!(
                rolls[1].value,
                ParameterValue::Integer(BoundedInteger::new(49).unwrap())
            );
        }
        // A declaration alone does not fabricate a template or whole-item acceptance.
        assert!(result.modifiers.is_empty());
    }
}

#[test]
fn absent_empty_and_unknown_property_maps_do_not_default_to_false() {
    let p = policy(ItemLineLimits::default());
    let expected = ItemLinePending::MissingProperty {
        property: key("tag-enabled"),
    };
    assert_eq!(
        reason(&p.convert_line(1, "Roll: 4", None).unwrap()),
        &expected
    );
    assert_eq!(
        reason(&p.convert_text("Roll: 4").unwrap().lines[0]),
        &expected
    );
    for facts in [BTreeMap::new(), BTreeMap::from([(key("other"), true)])] {
        let result = p.convert_lines([row(1, "Roll: 4", Some(&facts))]).unwrap();
        assert_eq!(reason(&result.lines[0]), &expected);
    }
}

#[test]
fn malformed_capture_precedes_missing_property_and_keeps_candidate_identity() {
    let p = policy(ItemLineLimits::default());
    for text in ["Roll: invalid", "Roll: 1.5", "Roll: 1e999"] {
        assert!(
            matches!(reason(&p.convert_line(1, text, None).unwrap()), ItemLinePending::MalformedCapture { capture, .. } if capture == &key("amount"))
        );
    }
}

#[test]
fn property_requires_boolean_schema_and_exact_modifier_declaration() {
    let mut raw = raw_schema();
    let SlotDescriptor::Parameter(entry) = &mut raw.slots[1] else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema.value = ValueSchema::Integer(IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(1).unwrap(),
    });
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    assert!(matches!(
        OwnedItemLinePolicy::new(input(&s), &s, ItemLineLimits::default()),
        Err(ItemLineError::InvalidPolicy { .. })
    ));
    let s = self::schema();
    let mut raw = input(&s);
    let ItemEmission::Modifier { rolls, .. } = &mut raw.rules[0].emissions[0] else {
        unreachable!()
    };
    rolls[0].slot.declaration =
        SlotOwnerDefId::Modifier(ModifierDefId::parse(ns(), "other").unwrap());
    assert!(matches!(
        OwnedItemLinePolicy::new(raw, &s, ItemLineLimits::default()),
        Err(ItemLineError::InvalidPolicy { .. })
    ));
}

#[test]
fn nonmodifier_property_locations_reject_before_missing_schema_lookup() {
    let s = schema();
    let missing_item = ItemTemplateDefId::parse(ns(), "missing").unwrap();
    for emission in [
        ItemEmission::ItemLevel { value: property() },
        ItemEmission::Quality {
            kind: QualityDefId::parse(ns(), "missing").unwrap(),
            amount: property(),
        },
        ItemEmission::ItemParameter {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::ItemTemplate(missing_item),
                slot: ParameterSlotDefId::parse(ns(), "missing").unwrap(),
            },
            value: property(),
        },
    ] {
        let mut raw = input(&s);
        raw.rules[0].emissions = vec![emission];
        assert!(matches!(
            OwnedItemLinePolicy::new(raw, &s, ItemLineLimits::default()),
            Err(ItemLineError::InvalidPolicy {
                reason: "property values require modifier rolls",
                ..
            })
        ));
    }
}

#[test]
fn property_map_resources_are_charged_even_for_unknown_and_malformed_lines() {
    let facts = BTreeMap::from([(key("a"), true), (key("b"), false)]);
    for (limits, label) in [
        (
            ItemLineLimits {
                max_output_declarations: 1,
                ..ItemLineLimits::default()
            },
            "output declarations",
        ),
        (
            ItemLineLimits {
                max_work: 1,
                ..ItemLineLimits::default()
            },
            "work",
        ),
        (
            ItemLineLimits {
                max_source_bytes: 4,
                ..ItemLineLimits::default()
            },
            "source bytes",
        ),
    ] {
        let p = policy(limits);
        for text in ["?", "Roll: bad"] {
            assert!(
                matches!(p.convert_lines([row(1, text, Some(&facts))]), Err(ItemLineError::Limit(actual)) if actual == label)
            );
        }
    }
    // Source bytes are cumulative across lines, including each supplied map.
    let p = policy(ItemLineLimits {
        max_source_bytes: 5,
        ..ItemLineLimits::default()
    });
    let one = BTreeMap::from([(key("p"), false)]);
    assert!(matches!(
        p.convert_lines([row(1, "?", Some(&one)), row(2, "?", Some(&one))]),
        Err(ItemLineError::Limit("source bytes"))
    ));
}

#[test]
fn property_policy_text_version_and_wire_are_strict() {
    let s = schema();
    let raw = input(&s);
    // Six literal bytes plus eleven bytes for the property key.
    assert!(
        OwnedItemLinePolicy::new(
            raw.clone(),
            &s,
            ItemLineLimits {
                max_policy_text_bytes: 17,
                ..ItemLineLimits::default()
            }
        )
        .is_ok()
    );
    assert!(matches!(
        OwnedItemLinePolicy::new(
            raw.clone(),
            &s,
            ItemLineLimits {
                max_policy_text_bytes: 16,
                ..ItemLineLimits::default()
            }
        ),
        Err(ItemLineError::Limit("policy text bytes"))
    ));
    let p = OwnedItemLinePolicy::new(raw.clone(), &s, ItemLineLimits::default()).unwrap();
    let bytes = encode_item_line_policy(&p, ItemLineLimits::default()).unwrap();
    assert_eq!(
        decode_item_line_policy(&bytes, &s, ItemLineLimits::default())
            .unwrap()
            .identity(),
        p.identity()
    );
    let mut stale = raw;
    stale.schema_version = 1;
    assert!(matches!(
        OwnedItemLinePolicy::new(stale, &s, ItemLineLimits::default()),
        Err(ItemLineError::UnsupportedVersion(1))
    ));
    assert!(
        serde_json::from_str::<ItemLineValue>(
            r#"{"kind":"property","value":{"property":"tag-enabled","fallback":false}}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<ItemLineValue>(
            r#"{"kind":"property","value":{"property":"tag-enabled","property":"other"}}"#
        )
        .is_err()
    );
}
