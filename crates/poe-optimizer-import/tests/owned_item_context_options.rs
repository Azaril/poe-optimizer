//! Generic typed context facts; source adapters separately establish authority.
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{owned_item_lines::*, owned_value::*};
use std::collections::BTreeMap;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("context-test", "v1").unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn modifier() -> ModifierDefId {
    ModifierDefId::parse(ns(), "modifier").unwrap()
}
fn option(s: &str) -> OptionDefId {
    OptionDefId::parse(ns(), s).unwrap()
}
fn slot(s: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Modifier(modifier()),
        slot: ParameterSlotDefId::parse(ns(), s).unwrap(),
    }
}
fn raw_schema() -> SchemaPackageInput {
    let mut definitions = vec![DefinitionDescriptor::Modifier(DefinitionEntry {
        id: modifier(),
        schema: SchemaState::Known(ModifierSchema {
            declarations: DeclaredSlots {
                parameters: DeclaredSet::complete(vec![slot("amount"), slot("context")]),
                choices: DeclaredSet::complete(vec![]),
                grants: DeclaredSet::complete(vec![]),
                actors: DeclaredSet::complete(vec![]),
                skill_grants: DeclaredSet::complete(vec![]),
                outputs: DeclaredSet::complete(vec![]),
                sockets: DeclaredSet::complete(vec![]),
            },
        }),
    })];
    for name in ["alpha", "beta", "outside"] {
        definitions.push(DefinitionDescriptor::Option(DefinitionEntry {
            id: option(name),
            schema: SchemaState::Known(OptionSchema {}),
        }));
    }
    SchemaPackageInput {
        schema_version: 4,
        namespace: ns(),
        release: key("fixture"),
        semantics_version: key("inputs"),
        definitions,
        slots: vec![
            SlotDescriptor::Parameter(DefinitionEntry {
                id: slot("amount"),
                schema: SchemaState::Known(ParameterSlotSchema {
                    skill_input: None,
                    value: ValueSchema::Integer(IntegerRange {
                        minimum: BoundedInteger::new(-100).unwrap(),
                        maximum: BoundedInteger::new(100).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                }),
            }),
            SlotDescriptor::Parameter(DefinitionEntry {
                id: slot("context"),
                schema: SchemaState::Known(ParameterSlotSchema {
                    skill_input: None,
                    value: ValueSchema::Option {
                        allowed: DeclaredSet::complete(vec![option("alpha"), option("beta")]),
                    },
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                }),
            }),
        ],
    }
}
fn schema() -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(raw_schema(), Default::default()).unwrap()
}
fn context() -> ItemLineValue {
    ItemLineValue::ContextOption {
        input: key("context"),
    }
}
fn input(s: &OwnedDefinitionSchemaPackage) -> ItemLinePolicyInput {
    ItemLinePolicyInput {
        schema_version: OWNED_ITEM_LINE_POLICY_V7,
        namespace: ns(),
        version: key("fixture"),
        definitions: s.identity().clone(),
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
                    ItemRollTemplate {
                        slot: slot("context"),
                        value: context(),
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
    options: Option<&'a BTreeMap<OwnedDefinitionKey, OptionDefId>>,
) -> ItemLineInput<'a> {
    ItemLineInput {
        index,
        text,
        properties: None,
        range_fraction: None,
        option_inputs: options,
    }
}
fn reason<'a>(e: &'a ItemLineEvidence<'_>) -> &'a ItemLinePending {
    let ItemLineOutcome::Pending { reason, candidates } = &e.outcome else {
        panic!("pending expected")
    };
    assert_eq!(candidates, &[key("roll")]);
    reason
}

#[test]
fn exact_context_options_preserve_distinct_occurrences_and_repeated_emissions() {
    let s = schema();
    let mut raw = input(&s);
    let duplicate = raw.rules[0].emissions[0].clone();
    raw.rules[0].emissions.push(duplicate);
    let p = OwnedItemLinePolicy::new(raw, &s, Default::default()).unwrap();
    let first = BTreeMap::from([(key("context"), option("alpha"))]);
    let second = BTreeMap::from([(key("context"), option("beta"))]);
    let converted = p
        .convert_lines([
            row(1, "Roll: 5", Some(&first)),
            row(2, "Roll: 5", Some(&second)),
        ])
        .unwrap();
    for (line, expected) in converted
        .lines
        .iter()
        .zip([option("alpha"), option("beta")])
    {
        let ItemLineOutcome::Known { emissions, .. } = &line.outcome else {
            panic!("known expected")
        };
        assert_eq!(emissions.len(), 2);
        for e in emissions {
            let ConvertedItemEmission::Modifier {
                rolls,
                rolls_closure,
                ..
            } = e
            else {
                panic!("modifier")
            };
            assert_eq!(
                rolls[0],
                ParameterAssignment {
                    slot: slot("context"),
                    value: ParameterValue::Option(expected.clone())
                }
            );
            assert_eq!(
                rolls[1].value,
                ParameterValue::Integer(BoundedInteger::new(5).unwrap())
            );
            assert_eq!(rolls_closure, &SchemaClosure::Complete);
        }
    }
    assert!(
        converted.modifiers.is_empty(),
        "a context fact does not establish an item template"
    );
}

#[test]
fn missing_context_is_atomic_pending_and_never_a_default() {
    let p = policy(Default::default());
    let expected = ItemLinePending::MissingContextOption {
        input: key("context"),
    };
    assert_eq!(
        reason(&p.convert_line(1, "Roll: 5", None).unwrap()),
        &expected
    );
    assert_eq!(
        reason(&p.convert_text("Roll: 5").unwrap().lines[0]),
        &expected
    );
    for map in [
        BTreeMap::new(),
        BTreeMap::from([(key("other"), option("alpha"))]),
    ] {
        let converted = p.convert_lines([row(1, "Roll: 5", Some(&map))]).unwrap();
        assert_eq!(reason(&converted.lines[0]), &expected);
        assert!(converted.modifiers.is_empty());
    }
    for text in ["Roll: bad", "Roll: 1.5", "Roll: 1e999"] {
        assert!(
            matches!(reason(&p.convert_line(1, text, None).unwrap()), ItemLinePending::MalformedCapture { capture, .. } if capture == &key("amount"))
        );
    }
}

#[test]
fn supplied_option_must_belong_to_the_exact_parameter_and_namespace() {
    let p = policy(Default::default());
    for value in [
        option("outside"),
        option("missing"),
        OptionDefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "alpha").unwrap(),
    ] {
        let map = BTreeMap::from([(key("context"), value)]);
        let converted = p.convert_lines([row(1, "Roll: 5", Some(&map))]).unwrap();
        assert_eq!(
            reason(&converted.lines[0]),
            &ItemLinePending::ValueOutsideSchema {
                slot: Some(Box::new(slot("context")))
            }
        );
    }
}

#[test]
fn context_option_requires_option_schema_and_modifier_roll_site() {
    let mut raw = raw_schema();
    let SlotDescriptor::Parameter(entry) = &mut raw.slots[1] else {
        unreachable!()
    };
    let SchemaState::Known(s) = &mut entry.schema else {
        unreachable!()
    };
    s.value = ValueSchema::Boolean;
    let s = OwnedDefinitionSchemaPackage::new(raw, Default::default()).unwrap();
    assert!(matches!(
        OwnedItemLinePolicy::new(input(&s), &s, Default::default()),
        Err(ItemLineError::InvalidPolicy { .. })
    ));
    let s = schema();
    for emission in [
        ItemEmission::ItemLevel { value: context() },
        ItemEmission::Quality {
            kind: QualityDefId::parse(ns(), "missing").unwrap(),
            amount: context(),
        },
        ItemEmission::ItemParameter {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::ItemTemplate(
                    ItemTemplateDefId::parse(ns(), "missing").unwrap(),
                ),
                slot: ParameterSlotDefId::parse(ns(), "missing").unwrap(),
            },
            value: context(),
        },
    ] {
        let mut raw = input(&s);
        raw.rules[0].emissions = vec![emission];
        assert!(matches!(
            OwnedItemLinePolicy::new(raw, &s, Default::default()),
            Err(ItemLineError::InvalidPolicy { .. })
        ));
    }
}

#[test]
fn version_seven_is_explicit_and_old_policy_wire_domains_remain_frozen() {
    let s = schema();
    for version in 2..=6 {
        let mut raw = input(&s);
        raw.schema_version = version;
        assert!(matches!(
            OwnedItemLinePolicy::new(raw.clone(), &s, Default::default()),
            Err(ItemLineError::InvalidPolicy {
                reason: "context option requires item-line policy v7",
                ..
            })
        ));
        let ItemEmission::Modifier { rolls, .. } = &mut raw.rules[0].emissions[0] else {
            unreachable!()
        };
        rolls[0].value = ItemLineValue::Literal(ParameterValue::Option(option("alpha")));
        let expected = serde_json::to_vec(&raw).unwrap();
        let p = OwnedItemLinePolicy::new(raw.clone(), &s, Default::default()).unwrap();
        assert_eq!(
            encode_item_line_policy(&p, Default::default()).unwrap(),
            expected
        );
        assert_eq!(
            *p.identity(),
            digest_owned(
                match version {
                    2 => "owned-item-line-policy-v2",
                    3 => "owned-item-line-policy-v3",
                    4 => "owned-item-line-policy-v4",
                    5 => "owned-item-line-policy-v5",
                    6 => "owned-item-line-policy-v6",
                    _ => unreachable!(),
                },
                &raw,
                1024 * 1024
            )
            .unwrap()
        );
        assert!(matches!(
            p.convert_line(1, "Roll: 5", None).unwrap().outcome,
            ItemLineOutcome::Known { .. }
        ));
    }
    let p = policy(Default::default());
    let bytes = encode_item_line_policy(&p, Default::default()).unwrap();
    let decoded = decode_item_line_policy(&bytes, &s, Default::default()).unwrap();
    assert_eq!(decoded.identity(), p.identity());
    assert_eq!(
        *p.identity(),
        digest_owned("owned-item-line-policy-v7", p.input(), 1024 * 1024).unwrap()
    );
    for text in [
        r#"{"kind":"context_option","value":{"input":"context","default":"alpha"}}"#,
        r#"{"kind":"context_option","value":{"input":"context","input":"other"}}"#,
    ] {
        assert!(serde_json::from_str::<ItemLineValue>(text).is_err());
    }
}

#[test]
fn context_map_is_bounded_before_unknown_or_malformed_matching() {
    let map = BTreeMap::from([(key("a"), option("alpha")), (key("b"), option("beta"))]);
    for (limits, label) in [
        (
            ItemLineLimits {
                max_output_declarations: 1,
                ..Default::default()
            },
            "output declarations",
        ),
        (
            ItemLineLimits {
                max_work: 1,
                ..Default::default()
            },
            "work",
        ),
        (
            ItemLineLimits {
                max_source_bytes: 9,
                ..Default::default()
            },
            "source bytes",
        ),
    ] {
        let p = policy(limits);
        for text in ["?", "Roll: bad"] {
            assert!(
                matches!(p.convert_lines([row(1, text, Some(&map))]), Err(ItemLineError::Limit(actual)) if actual == label)
            );
        }
    }
    let one = BTreeMap::from([(key("a"), option("alpha"))]);
    let map_bytes =
        1 + "alpha".len() + ns().game().as_str().len() + ns().version().as_str().len() + 4;
    let p = policy(ItemLineLimits {
        max_source_bytes: map_bytes + 1,
        ..Default::default()
    });
    assert!(p.convert_lines([row(1, "?", Some(&one))]).is_ok());
    assert!(matches!(
        p.convert_lines([row(1, "?", Some(&one)), row(2, "?", Some(&one))]),
        Err(ItemLineError::Limit("source bytes"))
    ));
}

#[test]
fn context_keys_charge_policy_text_and_map_order_is_irrelevant() {
    let s = schema();
    let raw = input(&s);
    assert!(
        OwnedItemLinePolicy::new(
            raw.clone(),
            &s,
            ItemLineLimits {
                max_policy_text_bytes: 13,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(matches!(
        OwnedItemLinePolicy::new(
            raw,
            &s,
            ItemLineLimits {
                max_policy_text_bytes: 12,
                ..Default::default()
            }
        ),
        Err(ItemLineError::Limit("policy text bytes"))
    ));
    let p = policy(Default::default());
    let a = BTreeMap::from([
        (key("context"), option("alpha")),
        (key("other"), option("beta")),
    ]);
    let mut b = BTreeMap::new();
    b.insert(key("other"), option("beta"));
    b.insert(key("context"), option("alpha"));
    let x = p.convert_lines([row(1, "Roll: 5", Some(&a))]).unwrap();
    let y = p.convert_lines([row(1, "Roll: 5", Some(&b))]).unwrap();
    assert_eq!(
        serde_json::to_value(x).unwrap(),
        serde_json::to_value(y).unwrap()
    );
}
