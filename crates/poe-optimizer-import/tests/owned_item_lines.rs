//! Generic injected policy tests; no source VM or built-in game aliases.
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{owned_item_lines::*, owned_value::*};
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("item-test", "v1").unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn int(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
fn unit() -> UnitDefId {
    UnitDefId::parse(ns(), "percent").unwrap()
}
fn qty(v: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, unit()).unwrap())
}
fn item() -> ItemTemplateDefId {
    ItemTemplateDefId::parse(ns(), "item").unwrap()
}
fn other() -> ItemTemplateDefId {
    ItemTemplateDefId::parse(ns(), "other").unwrap()
}
fn modifier() -> ModifierDefId {
    ModifierDefId::parse(ns(), "modifier").unwrap()
}
fn quality() -> QualityDefId {
    QualityDefId::parse(ns(), "quality").unwrap()
}
fn slot(owner: SlotOwnerDefId, id: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: ParameterSlotDefId::parse(ns(), id).unwrap(),
    }
}
fn roll() -> DeclaredSlot<ParameterSlotDefId> {
    slot(SlotOwnerDefId::Modifier(modifier()), "roll")
}
fn param() -> DeclaredSlot<ParameterSlotDefId> {
    slot(SlotOwnerDefId::ItemTemplate(item()), "parameter")
}
fn ir(a: i64, b: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(a).unwrap(),
        maximum: BoundedInteger::new(b).unwrap(),
    }
}
fn qr(a: f64, b: f64) -> QuantityRange {
    QuantityRange {
        minimum: FiniteQuantity::new(a, unit()).unwrap(),
        maximum: FiniteQuantity::new(b, unit()).unwrap(),
    }
}
fn declarations(p: Vec<DeclaredSlot<ParameterSlotDefId>>) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(p),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn raw_schema() -> SchemaPackageInput {
    let template = |p, m| ItemTemplateSchema {
        item_level: ir(1, 100),
        equipment_slots: DeclaredSet::complete(vec![]),
        socket_destinations: DeclaredSet::complete(vec![]),
        modifiers: DeclaredSet::complete(m),
        quality: QualityUseSchema {
            presence: QualityPresence::Optional,
            allowed_kinds: DeclaredSet::complete(vec![quality()]),
        },
        declarations: declarations(p),
    };
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("release"),
        semantics_version: key("inputs"),
        definitions: vec![
            DefinitionDescriptor::ItemTemplate(known(
                item(),
                template(vec![param()], vec![modifier()]),
            )),
            DefinitionDescriptor::ItemTemplate(known(other(), template(vec![], vec![]))),
            DefinitionDescriptor::Modifier(known(
                modifier(),
                ModifierSchema {
                    declarations: declarations(vec![roll()]),
                },
            )),
            DefinitionDescriptor::Unit(known(
                unit(),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
            DefinitionDescriptor::Quality(known(
                quality(),
                QualitySchema {
                    amount: qr(0.0, 100.0),
                },
            )),
        ],
        slots: vec![
            SlotDescriptor::Parameter(known(
                roll(),
                ParameterSlotSchema {
                    value: ValueSchema::Quantity(qr(-1000.0, 1000.0)),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                },
            )),
            SlotDescriptor::Parameter(known(
                param(),
                ParameterSlotSchema {
                    value: ValueSchema::Integer(ir(0, 100)),
                    presence: SlotPresence::OptionalOnce,
                    sites: vec![ParameterSite::ItemParameter],
                },
            )),
        ],
    }
}
fn schema() -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(raw_schema(), OwnedSchemaLimits::default()).unwrap()
}
fn codec(integer: bool) -> ItemCaptureCodec {
    ItemCaptureCodec::Value(ValueCodecInput {
        namespace: ns(),
        whitespace: WhitespacePolicy::Exact,
        codec: if integer {
            ValueCodecKind::Integer {
                syntax: DecimalSyntax::Scientific,
            }
        } else {
            ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Scientific,
                unit: unit(),
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            }
        },
    })
}
fn lit(s: &str) -> ItemPatternPart {
    ItemPatternPart::Literal(s.into())
}
fn cap(s: &str) -> ItemPatternPart {
    ItemPatternPart::Capture(key(s))
}
fn val(s: &str) -> ItemLineValue {
    ItemLineValue::Capture(key(s))
}
fn rule(
    id: &str,
    pattern: Vec<ItemPatternPart>,
    captures: Vec<ItemCapture>,
    emissions: Vec<ItemEmission>,
) -> ItemLineRule {
    ItemLineRule {
        id: key(id),
        pattern,
        captures,
        emissions,
    }
}
fn simple(id: &str, prefix: &str, integer: bool, e: ItemEmission) -> ItemLineRule {
    rule(
        id,
        vec![lit(prefix), cap("x")],
        vec![ItemCapture {
            id: key("x"),
            codec: codec(integer),
        }],
        vec![e],
    )
}
fn input(s: &OwnedDefinitionSchemaPackage) -> ItemLinePolicyInput {
    ItemLinePolicyInput {
        schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
        namespace: ns(),
        version: key("policy"),
        definitions: s.identity().clone(),
        whitespace: WhitespacePolicy::TrimAscii,
        rules: vec![
            rule(
                "base",
                vec![lit("Injected Base")],
                vec![],
                vec![ItemEmission::Template { definition: item() }],
            ),
            rule(
                "other",
                vec![lit("Other Base")],
                vec![],
                vec![ItemEmission::Template {
                    definition: other(),
                }],
            ),
            rule("blank", vec![lit("")], vec![], vec![]),
            rule(
                "metadata",
                vec![lit("Prefix: "), cap("text")],
                vec![ItemCapture {
                    id: key("text"),
                    codec: ItemCaptureCodec::OpaqueText,
                }],
                vec![ItemEmission::Metadata {
                    role: key("crafting"),
                }],
            ),
            simple(
                "level",
                "Level: ",
                true,
                ItemEmission::ItemLevel { value: val("x") },
            ),
            simple(
                "quality",
                "Quality: ",
                false,
                ItemEmission::Quality {
                    kind: quality(),
                    amount: val("x"),
                },
            ),
            simple(
                "modifier",
                "Speed: ",
                false,
                ItemEmission::Modifier {
                    definition: modifier(),
                    rolls: vec![ItemRollTemplate {
                        slot: roll(),
                        value: val("x"),
                    }],
                },
            ),
            simple(
                "parameter",
                "Parameter: ",
                true,
                ItemEmission::ItemParameter {
                    slot: param(),
                    value: val("x"),
                },
            ),
            rule(
                "range",
                vec![lit("Grant: ("), cap("lo"), lit("-"), cap("hi"), lit(")")],
                vec![
                    ItemCapture {
                        id: key("lo"),
                        codec: codec(true),
                    },
                    ItemCapture {
                        id: key("hi"),
                        codec: codec(true),
                    },
                ],
                vec![ItemEmission::ItemParameter {
                    slot: param(),
                    value: ItemLineValue::Interpolate {
                        lower: key("lo"),
                        upper: key("hi"),
                        quantum: int(1),
                        rounding: ItemRangeRounding::NearestTiesPositive,
                    },
                }],
            ),
        ],
    }
}
fn policy() -> OwnedItemLinePolicy {
    let s = schema();
    OwnedItemLinePolicy::new(input(&s), &s, ItemLineLimits::default()).unwrap()
}
fn pending<'a>(l: &'a ItemLineEvidence<'_>) -> &'a ItemLinePending {
    let ItemLineOutcome::Pending { reason, .. } = &l.outcome else {
        panic!("expected pending: {l:?}")
    };
    reason
}

#[test]
fn explicit_rolls_do_not_recompute_or_double_count_metadata() {
    let v=policy().convert_text("  Injected Base\r\nPrefix: {range:1}Finite26To28\r\nSpeed: 49\r\nLevel: 81\r\nQuality: 20\r\nParameter: 11\r\nUnknown effect\r\n").unwrap();
    assert_eq!(v.lines.len(), 7);
    assert_eq!(v.lines[0].text, "  Injected Base");
    assert!(matches!(&v.template,ItemField::Known{value,line:1} if value==&item()));
    assert!(matches!(&v.item_level,ItemField::Known{value,line:4} if value.get()==81));
    assert!(matches!(&v.quality,ItemField::Known{value,line:5} if value.amount.value()==20.0));
    assert_eq!(v.modifiers.len(), 1);
    assert_eq!(v.modifiers[0].line, 3);
    assert_eq!(v.modifiers[0].rolls[0].value, qty(49.0));
    assert_eq!(v.parameters[0].assignment.value, int(11));
    assert!(
        matches!(&v.lines[1].outcome,ItemLineOutcome::Known{emissions,..} if matches!(emissions.as_slice(),[ConvertedItemEmission::Metadata{..}]))
    );
    assert_eq!(pending(&v.lines[6]), &ItemLinePending::UnknownLine);
    assert!(v.issues.is_empty());
}
#[test]
fn zero_one_many_emissions_preserve_occurrence_order() {
    let s = schema();
    let mut i = input(&s);
    let m = &mut i.rules[6];
    m.emissions.push(m.emissions[0].clone());
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    let v = p
        .convert_text("Injected Base\n\nSpeed: 7\nSpeed: 7")
        .unwrap();
    assert!(
        matches!(&v.lines[1].outcome,ItemLineOutcome::Known{emissions,..} if emissions.is_empty())
    );
    assert_eq!(v.modifiers.len(), 4);
    assert_eq!((v.modifiers[0].line, v.modifiers[0].emission), (3, 0));
    assert_eq!((v.modifiers[1].line, v.modifiers[1].emission), (3, 1));
    assert_eq!((v.modifiers[2].line, v.modifiers[2].emission), (4, 0));
}
#[test]
fn aliases_balances_and_whitespace_are_injected_and_malformed_values_never_fallback() {
    let s = schema();
    let mut i = input(&s);
    i.rules[0].pattern = vec![lit("Different Alias")];
    i.whitespace = WhitespacePolicy::Exact;
    let ItemCaptureCodec::Value(v) = &mut i.rules[6].captures[0].codec else {
        panic!()
    };
    let ValueCodecKind::Quantity { scale, .. } = &mut v.codec else {
        panic!()
    };
    scale.numerator = BoundedInteger::new(2).unwrap();
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(
        pending(&p.convert_line(1, "Injected Base", None).unwrap()),
        &ItemLinePending::UnknownLine
    );
    assert_eq!(
        pending(&p.convert_line(1, " Different Alias", None).unwrap()),
        &ItemLinePending::UnknownLine
    );
    let v = p
        .convert_text("Different Alias\nSpeed: 49\nSpeed: nope\nSpeed: 8 trailing")
        .unwrap();
    assert_eq!(v.modifiers.len(), 1);
    assert_eq!(v.modifiers[0].rolls[0].value, qty(98.0));
    assert!(matches!(
        pending(&v.lines[2]),
        ItemLinePending::MalformedCapture { .. }
    ));
    assert!(matches!(
        pending(&v.lines[3]),
        ItemLinePending::MalformedCapture { .. }
    ));
}
#[test]
fn ambiguity_never_chooses_a_rule_or_capture_boundary() {
    let s = schema();
    let mut i = input(&s);
    let mut r = i.rules[0].clone();
    r.id = key("collision");
    i.rules.push(r);
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    let l = p.convert_line(1, "Injected Base", None).unwrap();
    assert!(
        matches!(&l.outcome,ItemLineOutcome::Pending{reason:ItemLinePending::AmbiguousRules,candidates} if candidates==&vec![key("base"),key("collision")])
    );
    assert_eq!(
        pending(&p.convert_line(1, "Grant: (1-2-3)", Some(0.5)).unwrap()),
        &ItemLinePending::AmbiguousCapture
    );
    let v = p.convert_text("Injected Base\nSpeed: 9").unwrap();
    assert!(v.modifiers.is_empty());
    assert!(matches!(v.template, ItemField::Pending { .. }));
}
#[test]
fn interpolation_requires_fraction_and_explicit_result_kind() {
    let p = policy();
    assert_eq!(
        pending(&p.convert_line(2, "Grant: (1-20)", None).unwrap()),
        &ItemLinePending::MissingRangeFraction
    );
    for x in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert_eq!(
            pending(&p.convert_line(2, "Grant: (1-20)", Some(x)).unwrap()),
            &ItemLinePending::InvalidRangeFraction
        );
    }
    assert_eq!(
        pending(&p.convert_line(2, "Grant: (20-1)", Some(0.5)).unwrap()),
        &ItemLinePending::InvalidRange
    );
    for (fraction, expected) in [(0.0, 1), (0.5, 11), (1.0, 20)] {
        let v = p
            .convert_lines([
                ItemLineInput {
                    index: 1,
                    text: "Injected Base",
                    range_fraction: None,
                    properties: None,
                },
                ItemLineInput {
                    index: 2,
                    text: "Grant: (1-20)",
                    range_fraction: Some(fraction),
                    properties: None,
                },
            ])
            .unwrap();
        assert_eq!(v.parameters[0].assignment.value, int(expected));
    }
}
#[test]
fn aggregate_removes_known_and_pending_singular_conflicts() {
    let p = policy();
    let v = p
        .convert_text("Injected Base\nInjected Base\nSpeed: 49\nQuality: 20\nLevel: 3")
        .unwrap();
    assert!(matches!(v.template, ItemField::Pending { .. }));
    assert!(v.modifiers.is_empty());
    assert!(matches!(v.quality, ItemField::Pending { .. }));
    assert!(matches!(v.item_level, ItemField::Pending { .. }));
    for last in ["Parameter: 1", "Parameter: malformed", "Parameter: 101"] {
        let text = format!("Injected Base\nParameter: 1\n{last}\nSpeed: 49");
        let v = p.convert_text(&text).unwrap();
        assert!(v.parameters.is_empty(), "{last}");
        assert_eq!(v.modifiers.len(), 1);
        assert!(
            v.issues
                .iter()
                .any(|i| i.problem == ItemTextProblem::DuplicateParameter && i.lines == vec![2, 3])
        );
    }
    let v = p
        .convert_text("Other Base\nSpeed: 49\nParameter: 1")
        .unwrap();
    assert!(v.modifiers.is_empty());
    assert!(v.parameters.is_empty());
    assert!(
        v.issues
            .iter()
            .any(|i| i.problem == ItemTextProblem::ModifierNotAllowed)
    );
    assert!(
        v.issues
            .iter()
            .any(|i| i.problem == ItemTextProblem::WrongParameterOwner)
    );
    let v = p
        .convert_text("Injected Base\nLevel: 101\nQuality: 101")
        .unwrap();
    assert!(matches!(v.item_level, ItemField::Pending { .. }));
    assert!(matches!(v.quality, ItemField::Pending { .. }));
}
#[test]
fn repeated_headers_retain_only_linear_diagnostic_origins() {
    let p = policy();
    let text = "Injected Base\n".repeat(500);
    let v = p.convert_text(&text).unwrap();
    assert!(matches!(&v.template,ItemField::Pending{lines} if lines.len()==500));
    assert!(v.issues.iter().all(|i| i.lines.len() <= 2));
    let origins: usize = v.issues.iter().map(|i| i.lines.len()).sum();
    assert!(origins <= 1000, "{origins}");
}

#[test]
fn strict_codec_and_policy_reject_contradictory_inputs() {
    let s = schema();
    let p = policy();
    let bytes = encode_item_line_policy(&p, ItemLineLimits::default()).unwrap();
    assert_eq!(
        decode_item_line_policy(&bytes, &s, ItemLineLimits::default())
            .unwrap()
            .identity(),
        p.identity()
    );
    let mut json = serde_json::to_value(p.input()).unwrap();
    json["extra"] = true.into();
    assert!(
        decode_item_line_policy(
            &serde_json::to_vec(&json).unwrap(),
            &s,
            ItemLineLimits::default()
        )
        .is_err()
    );
    let text = String::from_utf8(bytes).unwrap();
    let duplicated = text.replacen(
        '{',
        &format!("{{\"schema_version\":{},", p.input().schema_version),
        1,
    );
    assert_ne!(duplicated, text);
    assert!(decode_item_line_policy(duplicated.as_bytes(), &s, ItemLineLimits::default()).is_err());
    assert!(
        serde_json::from_str::<ItemCaptureCodec>(r#"{"kind":"opaque_text","extra":1}"#).is_err()
    );
    let mut i = input(&s);
    i.rules[3]
        .emissions
        .push(ItemEmission::Template { definition: item() });
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let mut i = input(&s);
    i.definitions.release = "wrong".into();
    assert!(matches!(
        OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()),
        Err(ItemLineError::Binding)
    ));
    let mut i = input(&s);
    i.namespace = GameVersionNamespace::new("foreign", "v1").unwrap();
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let mut i = input(&s);
    i.rules.push(i.rules[0].clone());
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let mut i = input(&s);
    i.rules[6].pattern.push(cap("x"));
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let mut i = input(&s);
    i.rules[6].captures[0].codec = codec(true);
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
}
#[test]
fn computability_bounds_and_exact_owners_are_separate_from_gameplay_metadata() {
    let s = schema();
    let mut i = input(&s);
    let ItemEmission::Modifier { rolls, .. } = &mut i.rules[6].emissions[0] else {
        panic!()
    };
    rolls[0].value = ItemLineValue::Literal(qty(1001.0));
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let p = policy();
    assert!(matches!(
        pending(&p.convert_line(1, "Speed: 1001", None).unwrap()),
        ItemLinePending::ValueOutsideSchema { .. }
    ));
    assert_eq!(
        p.convert_text("Injected Base\nSpeed: 49")
            .unwrap()
            .modifiers[0]
            .rolls[0]
            .value,
        qty(49.0)
    );
    let mut i = input(&s);
    let ItemEmission::Modifier { rolls, .. } = &mut i.rules[6].emissions[0] else {
        panic!()
    };
    rolls[0].slot = param();
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let mut i = input(&s);
    let ItemEmission::Modifier { rolls, .. } = &mut i.rules[6].emissions[0] else {
        panic!()
    };
    rolls.clear();
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let mut i = input(&s);
    let ItemCaptureCodec::Value(v) = &mut i.rules[6].captures[0].codec else {
        panic!()
    };
    let ValueCodecKind::Quantity { unit, .. } = &mut v.codec else {
        panic!()
    };
    *unit = UnitDefId::parse(ns(), "different-unit").unwrap();
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
}
#[test]
fn missing_unmapped_and_partial_schema_are_not_fabricated() {
    let s = schema();
    let mut i = input(&s);
    i.rules[0].emissions = vec![ItemEmission::Template {
        definition: ItemTemplateDefId::parse(ns(), "missing").unwrap(),
    }];
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    assert!(matches!(
        pending(&p.convert_line(1, "Injected Base", None).unwrap()),
        ItemLinePending::Schema {
            status: ItemSchemaUnknown::Missing,
            ..
        }
    ));
    let mut raw = raw_schema();
    if let DefinitionDescriptor::Modifier(e) = &mut raw.definitions[2] {
        e.schema = SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(modifier().address()),
                facet: SchemaFacet::InputSchema,
                code: key("unknown"),
            }],
        };
    }
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let p = OwnedItemLinePolicy::new(input(&s), &s, ItemLineLimits::default()).unwrap();
    assert!(matches!(
        pending(&p.convert_line(1, "Speed: 49", None).unwrap()),
        ItemLinePending::Schema {
            status: ItemSchemaUnknown::Unmapped,
            ..
        }
    ));
    for present in [true, false] {
        let mut raw = raw_schema();
        if let DefinitionDescriptor::ItemTemplate(e) = &mut raw.definitions[0]
            && let SchemaState::Known(v) = &mut e.schema
        {
            if !present {
                v.modifiers.members.clear();
            }
            v.modifiers.closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(item().address()),
                    facet: SchemaFacet::StaticLinks,
                    code: key("partial"),
                }],
            };
        }
        let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
        let p = OwnedItemLinePolicy::new(input(&s), &s, ItemLineLimits::default()).unwrap();
        let v = p.convert_text("Injected Base\nSpeed: 49").unwrap();
        assert_eq!(v.modifiers.len(), usize::from(present));
        if !present {
            assert!(
                v.issues
                    .iter()
                    .any(|i| i.problem == ItemTextProblem::SchemaPartial)
            );
        }
    }
    let v = policy().convert_text("Unknown Base").unwrap();
    assert!(matches!(v.template, ItemField::Absent));
    assert!(matches!(v.item_level, ItemField::Absent));
    assert!(matches!(v.quality, ItemField::Absent));
}
#[test]
fn collection_byte_and_work_limits_cover_construction_conversion_and_encoding() {
    let s = schema();
    let i = input(&s);
    let d = ItemLineLimits::default();
    for limits in [
        ItemLineLimits { max_rules: 1, ..d },
        ItemLineLimits { max_parts: 1, ..d },
        ItemLineLimits {
            max_captures: 1,
            ..d
        },
        ItemLineLimits {
            max_emissions: 1,
            ..d
        },
        ItemLineLimits {
            max_policy_text_bytes: 1,
            ..d
        },
        ItemLineLimits {
            max_schema_work: 1,
            ..d
        },
        ItemLineLimits {
            max_wire_bytes: 1,
            ..d
        },
    ] {
        assert!(OwnedItemLinePolicy::new(i.clone(), &s, limits).is_err());
    }
    let mut more = i.clone();
    let ItemEmission::Modifier { rolls, .. } = &mut more.rules[6].emissions[0] else {
        panic!()
    };
    rolls.push(rolls[0].clone());
    assert!(matches!(
        OwnedItemLinePolicy::new(more, &s, ItemLineLimits { max_rolls: 1, ..d }),
        Err(ItemLineError::Limit("rolls"))
    ));
    for limits in [
        ItemLineLimits {
            max_source_bytes: 1,
            ..d
        },
        ItemLineLimits {
            max_line_bytes: 1,
            ..d
        },
        ItemLineLimits { max_lines: 1, ..d },
        ItemLineLimits { max_work: 1, ..d },
        ItemLineLimits {
            max_output_declarations: 1,
            ..d
        },
    ] {
        let p = OwnedItemLinePolicy::new(i.clone(), &s, limits).unwrap();
        assert!(p.convert_text("Injected Base\nSpeed: 49").is_err());
    }
    assert!(ItemLineLimits { max_lines: 0, ..d }.validate().is_err());
    assert!(
        ItemLineLimits {
            max_work: usize::MAX,
            ..d
        }
        .validate()
        .is_err()
    );
    let p = policy();
    assert!(p.convert_line(0, "Injected Base", None).is_err());
    assert!(
        p.convert_lines([
            ItemLineInput {
                index: 2,
                text: "Injected Base",
                range_fraction: None,
                properties: None,
            },
            ItemLineInput {
                index: 2,
                text: "Speed: 1",
                range_fraction: None,
                properties: None,
            }
        ])
        .is_err()
    );
    assert!(
        encode_item_line_policy(
            &p,
            ItemLineLimits {
                max_schema_work: 1,
                ..d
            }
        )
        .is_err()
    );
    let p = OwnedItemLinePolicy::new(
        i.clone(),
        &s,
        ItemLineLimits {
            value: OwnedValueLimits {
                max_source_bytes: 1,
                ..d.value
            },
            ..d
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_line(1, "Speed: 49", None),
        Err(ItemLineError::Limit("capture source bytes"))
    ));
    let bytes = serde_json::to_vec(&i).unwrap();
    assert!(
        decode_item_line_policy(
            &bytes,
            &s,
            ItemLineLimits {
                max_wire_bytes: bytes.len() - 1,
                ..d
            }
        )
        .is_err()
    );
    assert!(
        encode_item_line_policy(
            &policy(),
            ItemLineLimits {
                max_wire_bytes: bytes.len() - 1,
                ..d
            }
        )
        .is_err()
    );
}
#[test]
fn ambiguous_candidate_and_pending_emission_expansion_use_the_output_budget() {
    let s = schema();
    let mut i = input(&s);
    i.rules = vec![i.rules[0].clone()];
    let mut r = i.rules[0].clone();
    r.id = key("collision");
    i.rules.push(r);
    let p = OwnedItemLinePolicy::new(
        i,
        &s,
        ItemLineLimits {
            max_output_declarations: 1,
            ..ItemLineLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_line(1, "Injected Base", None),
        Err(ItemLineError::Limit("output declarations"))
    ));
    let s = schema();
    let mut i = input(&s);
    i.rules = vec![i.rules[7].clone()];
    let p = OwnedItemLinePolicy::new(
        i,
        &s,
        ItemLineLimits {
            max_output_declarations: 1,
            ..ItemLineLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_text("Parameter: nope"),
        Err(ItemLineError::Limit("output declarations"))
    ));
}

#[test]
fn option_membership_scans_consume_runtime_work() {
    let selected = slot(SlotOwnerDefId::ItemTemplate(item()), "selected-option");
    let options: Vec<_> = (0..256)
        .map(|i| OptionDefId::parse(ns(), format!("option-{i:03}")).unwrap())
        .collect();
    let mut raw = raw_schema();
    if let DefinitionDescriptor::ItemTemplate(entry) = &mut raw.definitions[0]
        && let SchemaState::Known(v) = &mut entry.schema
    {
        v.declarations.parameters.members.push(selected.clone());
    }
    raw.definitions.extend(
        options
            .iter()
            .cloned()
            .map(|id| DefinitionDescriptor::Option(known(id, OptionSchema {}))),
    );
    raw.slots.push(SlotDescriptor::Parameter(known(
        selected.clone(),
        ParameterSlotSchema {
            value: ValueSchema::Option {
                allowed: DeclaredSet::complete(options.clone()),
            },
            presence: SlotPresence::OptionalOnce,
            sites: vec![ParameterSite::ItemParameter],
        },
    )));
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let mut i = input(&s);
    i.rules = vec![rule(
        "option",
        vec![lit("Option: "), cap("x")],
        vec![ItemCapture {
            id: key("x"),
            codec: ItemCaptureCodec::Value(ValueCodecInput {
                namespace: ns(),
                whitespace: WhitespacePolicy::Exact,
                codec: ValueCodecKind::Option {
                    tokens: vec![OptionToken {
                        token: "last".into(),
                        value: options[255].clone(),
                    }],
                },
            }),
        }],
        vec![ItemEmission::ItemParameter {
            slot: selected,
            value: val("x"),
        }],
    )];
    let p = OwnedItemLinePolicy::new(i.clone(), &s, ItemLineLimits::default()).unwrap();
    assert!(matches!(
        p.convert_line(1, "Option: last", None).unwrap().outcome,
        ItemLineOutcome::Known { .. }
    ));
    let p = OwnedItemLinePolicy::new(
        i,
        &s,
        ItemLineLimits {
            max_work: 128,
            ..ItemLineLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_line(1, "Option: last", None),
        Err(ItemLineError::Limit("work"))
    ));
}

fn numeric(capture: &str, syntax: DecimalSyntax, sign: ItemNumericSign) -> ItemPatternPart {
    ItemPatternPart::NumericCapture {
        capture: key(capture),
        syntax,
        sign,
    }
}
fn numeric_rule(syntax: DecimalSyntax, sign: ItemNumericSign) -> ItemLineRule {
    rule(
        "number",
        vec![numeric("x", syntax, sign), lit(" units")],
        vec![ItemCapture {
            id: key("x"),
            codec: codec(false),
        }],
        vec![ItemEmission::Modifier {
            definition: modifier(),
            rolls: vec![ItemRollTemplate {
                slot: roll(),
                value: val("x"),
            }],
        }],
    )
}
fn with_rules(rules: Vec<ItemLineRule>, limits: ItemLineLimits) -> OwnedItemLinePolicy {
    let s = schema();
    let mut i = input(&s);
    i.rules = rules;
    OwnedItemLinePolicy::new(i, &s, limits).unwrap()
}

#[test]
fn numeric_patterns_use_explicit_ascii_syntax_and_sign_policy() {
    use DecimalSyntax::*;
    use ItemNumericSign::*;
    for (syntax, accepted, rejected) in [
        (
            Integer,
            vec!["12", "0", "-0", "+12"],
            vec!["1.", ".1", "1e2"],
        ),
        (
            Decimal,
            vec!["1.", ".1", "-1.25", "+12"],
            vec!["1e2", "1e+", "."],
        ),
        (
            Scientific,
            vec!["1e2", "-1.25e-1", "+.1E+2"],
            vec!["1e", "1e+", "1e--2"],
        ),
    ] {
        let p = with_rules(
            vec![numeric_rule(syntax, Optional)],
            ItemLineLimits::default(),
        );
        for token in accepted {
            assert!(
                matches!(
                    p.convert_line(1, &format!("{token} units"), None)
                        .unwrap()
                        .outcome,
                    ItemLineOutcome::Known { .. }
                ),
                "{syntax:?} {token}"
            );
        }
        for token in rejected
            .into_iter()
            .chain(["", "+", "-", "NaN", "inf", "0x1", "１２", "−1", "1_0"])
        {
            assert_eq!(
                pending(&p.convert_line(1, &format!("{token} units"), None).unwrap()),
                &ItemLinePending::UnknownLine,
                "{syntax:?} {token}"
            );
        }
    }
    for (sign, plus, minus) in [
        (Optional, true, true),
        (OptionalMinus, false, true),
        (Forbidden, false, false),
    ] {
        let p = with_rules(vec![numeric_rule(Integer, sign)], ItemLineLimits::default());
        for (token, accepted) in [("1", true), ("+1", plus), ("-1", minus)] {
            assert_eq!(
                matches!(
                    p.convert_line(1, &format!("{token} units"), None)
                        .unwrap()
                        .outcome,
                    ItemLineOutcome::Known { .. }
                ),
                accepted,
                "{sign:?} {token}"
            );
        }
    }
}

#[test]
fn numeric_patterns_are_maximal_and_never_resolve_ambiguity_by_decoding() {
    let p = with_rules(
        vec![numeric_rule(
            DecimalSyntax::Scientific,
            ItemNumericSign::Optional,
        )],
        ItemLineLimits::default(),
    );
    assert!(matches!(
        pending(&p.convert_line(1, "1e9999 units", None).unwrap()),
        ItemLinePending::MalformedCapture { .. }
    ));
    let mut decimal = numeric_rule(DecimalSyntax::Decimal, ItemNumericSign::Optional);
    decimal.id = key("decimal");
    let p = with_rules(
        vec![
            numeric_rule(DecimalSyntax::Integer, ItemNumericSign::Optional),
            decimal,
        ],
        ItemLineLimits::default(),
    );
    assert_eq!(
        pending(&p.convert_line(1, "7 units", None).unwrap()),
        &ItemLinePending::AmbiguousRules
    );
    // The lexer does not split a numeric token to make a following literal match.
    let mut r = numeric_rule(DecimalSyntax::Integer, ItemNumericSign::Optional);
    r.pattern[1] = lit("2 units");
    let p = with_rules(vec![r], ItemLineLimits::default());
    assert_eq!(
        pending(&p.convert_line(1, "12 units", None).unwrap()),
        &ItemLinePending::UnknownLine
    );
    // Once a scientific exponent marker appears it must be well formed; no retry as "1".
    let mut r = numeric_rule(DecimalSyntax::Scientific, ItemNumericSign::Optional);
    r.pattern[1] = lit("e units");
    let p = with_rules(vec![r], ItemLineLimits::default());
    assert_eq!(
        pending(&p.convert_line(1, "1e units", None).unwrap()),
        &ItemLinePending::UnknownLine
    );
}

#[test]
fn numeric_patterns_preserve_signed_range_boundaries() {
    let r = rule(
        "range",
        vec![
            lit("+("),
            numeric("a", DecimalSyntax::Integer, ItemNumericSign::OptionalMinus),
            lit("-"),
            numeric("b", DecimalSyntax::Integer, ItemNumericSign::OptionalMinus),
            lit(") units"),
        ],
        vec![
            ItemCapture {
                id: key("a"),
                codec: codec(false),
            },
            ItemCapture {
                id: key("b"),
                codec: codec(false),
            },
        ],
        vec![ItemEmission::Modifier {
            definition: modifier(),
            rolls: vec![ItemRollTemplate {
                slot: roll(),
                value: ItemLineValue::InterpolateOffset {
                    lower: key("a"),
                    upper: key("b"),
                    quantum: qty(1.0),
                    rounding: ItemRangeRounding::SymmetricHalfOffset,
                },
            }],
        }],
    );
    let p = with_rules(
        vec![
            numeric_rule(DecimalSyntax::Integer, ItemNumericSign::Optional),
            r.clone(),
        ],
        ItemLineLimits::default(),
    );
    let line = p.convert_line(1, "+(-3--2) units", Some(0.5)).unwrap();
    let ItemLineOutcome::Known { emissions, .. } = line.outcome else {
        panic!("{line:?}")
    };
    assert!(
        matches!(&emissions[0], ConvertedItemEmission::Modifier { rolls, .. } if rolls[0].value == qty(-3.0))
    );
    assert_eq!(
        pending(&p.convert_line(1, "+(-3-+2) units", Some(0.5)).unwrap()),
        &ItemLinePending::UnknownLine
    );
    let mut raw = r;
    raw.pattern[1] = cap("a");
    raw.pattern[3] = cap("b");
    let p = with_rules(vec![raw], ItemLineLimits::default());
    assert_eq!(
        pending(&p.convert_line(1, "+(-3--2) units", Some(0.5)).unwrap()),
        &ItemLinePending::AmbiguousCapture
    );
}

#[test]
fn numeric_capture_wire_and_declarations_are_strict() {
    let original = numeric("x", DecimalSyntax::Integer, ItemNumericSign::OptionalMinus);
    let wire = serde_json::to_value(&original).unwrap();
    assert_eq!(
        wire,
        serde_json::json!({"kind":"numeric_capture","value":{
        "capture":"x","syntax":"integer","sign":"optional_minus"}})
    );
    assert_eq!(
        serde_json::from_value::<ItemPatternPart>(wire.clone()).unwrap(),
        original
    );
    for field in ["capture", "syntax", "sign"] {
        let mut bad = wire.clone();
        bad["value"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ItemPatternPart>(bad).is_err());
    }
    for (field, value) in [("sign", "automatic"), ("extra", "ignored")] {
        let mut bad = wire.clone();
        bad["value"][field] = serde_json::json!(value);
        assert!(serde_json::from_value::<ItemPatternPart>(bad).is_err());
    }
    let s = schema();
    let base = numeric_rule(DecimalSyntax::Integer, ItemNumericSign::Optional);
    for mutation in 0..4 {
        let mut i = input(&s);
        let mut r = base.clone();
        match mutation {
            0 => r.captures.clear(),
            1 => r.pattern.push(numeric(
                "x",
                DecimalSyntax::Integer,
                ItemNumericSign::Optional,
            )),
            2 => {
                r.captures.push(ItemCapture {
                    id: key("y"),
                    codec: codec(false),
                });
                r.pattern.insert(
                    1,
                    numeric("y", DecimalSyntax::Integer, ItemNumericSign::Optional),
                );
            }
            _ => r.pattern = vec![lit("unused")],
        }
        i.rules = vec![r];
        assert!(
            OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn numeric_prefix_scanning_consumes_the_work_budget() {
    let p = with_rules(
        vec![numeric_rule(
            DecimalSyntax::Scientific,
            ItemNumericSign::Optional,
        )],
        ItemLineLimits {
            max_work: 64,
            ..ItemLineLimits::default()
        },
    );
    let text = format!("{} units", "1".repeat(128));
    assert!(matches!(
        p.convert_line(1, &text, None),
        Err(ItemLineError::Limit("work"))
    ));
}
