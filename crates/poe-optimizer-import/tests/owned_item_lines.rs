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
        schema_version: OWNED_ITEM_LINE_POLICY_V2,
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

fn raw_range_value() -> ItemLineValue {
    ItemLineValue::InterpolateUnroundedOffset {
        lower: key("lower"),
        upper: key("upper"),
    }
}
fn raw_range_input(s: &OwnedDefinitionSchemaPackage) -> ItemLinePolicyInput {
    let mut i = input(s);
    i.schema_version = 3;
    i.rules = vec![rule(
        "raw-range",
        vec![
            lit("Raw ("),
            numeric(
                "lower",
                DecimalSyntax::Scientific,
                ItemNumericSign::Optional,
            ),
            lit(","),
            numeric(
                "upper",
                DecimalSyntax::Scientific,
                ItemNumericSign::Optional,
            ),
            lit(")"),
        ],
        vec![
            ItemCapture {
                id: key("lower"),
                codec: codec(false),
            },
            ItemCapture {
                id: key("upper"),
                codec: codec(false),
            },
        ],
        vec![ItemEmission::Modifier {
            definition: modifier(),
            rolls: vec![ItemRollTemplate {
                slot: roll(),
                value: raw_range_value(),
            }],
        }],
    )];
    i
}
fn raw_range_policy(s: &OwnedDefinitionSchemaPackage) -> OwnedItemLinePolicy {
    OwnedItemLinePolicy::new(raw_range_input(s), s, ItemLineLimits::default()).unwrap()
}
fn range_result(p: &OwnedItemLinePolicy, text: &str, fraction: f64) -> ParameterValue {
    let evidence = p.convert_line(1, text, Some(fraction)).unwrap();
    let ItemLineOutcome::Known { emissions, .. } = &evidence.outcome else {
        panic!("expected raw range value: {evidence:?}");
    };
    let ConvertedItemEmission::Modifier { rolls, .. } = &emissions[0] else {
        panic!("expected one modifier emission");
    };
    assert_eq!(emissions.len(), 1);
    assert_eq!(rolls.len(), 1);
    rolls[0].value.clone()
}
fn wide_range_schema() -> OwnedDefinitionSchemaPackage {
    let mut s = raw_schema();
    let SlotDescriptor::Parameter(row) = &mut s.slots[0] else {
        unreachable!()
    };
    let SchemaState::Known(value) = &mut row.schema else {
        unreachable!()
    };
    value.value = ValueSchema::Quantity(qr(-f64::MAX, f64::MAX));
    OwnedDefinitionSchemaPackage::new(s, OwnedSchemaLimits::default()).unwrap()
}

#[test]
fn v3_raw_range_retains_fractional_value_before_existing_component_rounding() {
    let s = schema();
    let raw = raw_range_policy(&s);
    assert_eq!(range_result(&raw, "Raw (2.00,2.01)", 0.5), qty(2.005));
    let mut rounded = raw_range_input(&s);
    rounded.schema_version = OWNED_ITEM_LINE_POLICY_V2;
    let ItemEmission::Modifier { rolls, .. } = &mut rounded.rules[0].emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::InterpolateOffset {
        lower: key("lower"),
        upper: key("upper"),
        quantum: qty(0.01),
        rounding: ItemRangeRounding::SymmetricHalfOffset,
    };
    let rounded = OwnedItemLinePolicy::new(rounded, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(range_result(&rounded, "Raw (2.00,2.01)", 0.5), qty(2.0));
}

#[test]
fn raw_range_preserves_signed_endpoints_and_literal_prefix_has_no_numeric_meaning() {
    let s = schema();
    let p = raw_range_policy(&s);
    for (text, fraction, expected) in [
        ("Raw (2,3)", 0.0, 2.0),
        ("Raw (2,3)", 1.0, 3.0),
        ("Raw (-3,-2)", 0.0, -3.0),
        ("Raw (-3,-2)", 0.5, -2.5),
        ("Raw (-3,-2)", 1.0, -2.0),
        ("Raw (-2,+3)", 0.5, 0.5),
        ("Raw (-0,+0)", 0.5, 0.0),
        ("Raw (4,4)", 0.25, 4.0),
    ] {
        assert_eq!(
            range_result(&p, text, fraction),
            qty(expected),
            "{text} at {fraction}"
        );
    }
    let mut negative_prefix = raw_range_input(&s);
    negative_prefix.rules[0].pattern[0] = lit("-Raw (");
    let negative_prefix =
        OwnedItemLinePolicy::new(negative_prefix, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(range_result(&negative_prefix, "-Raw (2,3)", 0.5), qty(2.5));
}

#[test]
fn raw_range_needs_a_finite_fraction_and_ordered_bounds_without_defaults() {
    let s = schema();
    let p = raw_range_policy(&s);
    for text in ["Raw (1,2)", "Raw (4,4)"] {
        assert_eq!(
            pending(&p.convert_line(1, text, None).unwrap()),
            &ItemLinePending::MissingRangeFraction
        );
        for fraction in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.001, 1.001] {
            assert_eq!(
                pending(&p.convert_line(1, text, Some(fraction)).unwrap()),
                &ItemLinePending::InvalidRangeFraction
            );
        }
    }
    for fraction in [0.0, 0.5, 1.0] {
        assert_eq!(
            pending(&p.convert_line(1, "Raw (3,2)", Some(fraction)).unwrap()),
            &ItemLinePending::InvalidRange
        );
    }
    assert!(matches!(
        pending(&p.convert_line(1, "Raw (1e9999,2)", Some(0.5)).unwrap()),
        ItemLinePending::MalformedCapture { .. }
    ));
}

#[test]
fn raw_offset_checks_each_intermediate_even_when_the_fraction_is_an_endpoint() {
    let s = wide_range_schema();
    let p = raw_range_policy(&s);
    for fraction in [0.0, 0.25, 0.5, 1.0] {
        assert_eq!(
            pending(
                &p.convert_line(1, "Raw (-1e308,1e308)", Some(fraction))
                    .unwrap()
            ),
            &ItemLinePending::InvalidRange
        );
    }
    assert_eq!(range_result(&p, "Raw (1e308,1.5e308)", 0.5), qty(1.25e308));
    // Literal offset arithmetic intentionally preserves endpoint cancellation:
    // -1e16 + 1 * (1 - -1e16) is 0, not an exact-endpoint shortcut to 1.
    assert_eq!(range_result(&p, "Raw (-1e16,1)", 1.0), qty(0.0));
}

#[test]
fn raw_range_requires_quantity_captures_with_identical_units() {
    let s = schema();
    for integers in [1, 2] {
        let mut i = raw_range_input(&s);
        for capture in i.rules[0].captures.iter_mut().take(integers) {
            capture.codec = codec(true);
        }
        let err = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap_err();
        assert!(
            err.to_string()
                .contains("unrounded range endpoints require quantities"),
            "{err}"
        );
    }
    let different = UnitDefId::parse(ns(), "distinct-percent").unwrap();
    let mut raw = raw_schema();
    raw.definitions.push(DefinitionDescriptor::Unit(known(
        different.clone(),
        UnitSchema {
            dimension: UnitDimension::PercentagePoints,
        },
    )));
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let mut i = raw_range_input(&s);
    let ItemCaptureCodec::Value(codec) = &mut i.rules[0].captures[1].codec else {
        unreachable!()
    };
    let ValueCodecKind::Quantity { unit, .. } = &mut codec.codec else {
        unreachable!()
    };
    *unit = different;
    let err = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap_err();
    assert!(err.to_string().contains("same exact unit"), "{err}");
    let mut i = raw_range_input(&s);
    let ItemEmission::Modifier { rolls, .. } = &mut i.rules[0].emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::InterpolateUnroundedOffset {
        lower: key("missing"),
        upper: key("upper"),
    };
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
}

#[test]
fn v2_identity_and_serialized_input_are_unchanged_while_v3_uses_its_own_domain() {
    use poe_optimizer_core::owned_content::digest_owned;
    let s = schema();
    let legacy = input(&s);
    assert_eq!(legacy.schema_version, 2);
    assert_eq!(OWNED_ITEM_LINE_POLICY_V3, 3);
    assert_eq!(OWNED_ITEM_LINE_POLICY_V4, 4);
    assert_eq!(OWNED_ITEM_LINE_POLICY_V5, 5);
    assert_eq!(OWNED_ITEM_LINE_POLICY_VERSION, 6);
    let old_bytes = serde_json::to_vec(&legacy).unwrap();
    let expected = digest_owned(
        "owned-item-line-policy-v2",
        &legacy,
        ItemLineLimits::default().max_wire_bytes,
    )
    .unwrap();
    let p = OwnedItemLinePolicy::new(legacy.clone(), &s, ItemLineLimits::default()).unwrap();
    assert_eq!(p.identity(), &expected);
    assert_eq!(
        encode_item_line_policy(&p, ItemLineLimits::default()).unwrap(),
        old_bytes
    );
    assert_eq!(
        decode_item_line_policy(&old_bytes, &s, ItemLineLimits::default())
            .unwrap()
            .identity(),
        &expected
    );
    let mut upgraded = legacy;
    upgraded.schema_version = 3;
    let expected_v3 = digest_owned(
        "owned-item-line-policy-v3",
        &upgraded,
        ItemLineLimits::default().max_wire_bytes,
    )
    .unwrap();
    let v3_bytes = serde_json::to_vec(&upgraded).unwrap();
    let current = OwnedItemLinePolicy::new(upgraded, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(
        encode_item_line_policy(&current, ItemLineLimits::default()).unwrap(),
        v3_bytes
    );
    assert_eq!(
        decode_item_line_policy(&v3_bytes, &s, ItemLineLimits::default())
            .unwrap()
            .identity(),
        &expected_v3
    );
    assert_eq!(current.identity(), &expected_v3);
    assert_ne!(p.identity(), current.identity());
    for text in ["Speed: 49", "Grant: (1-20)", "Quality: 20"] {
        assert_eq!(
            p.convert_line(1, text, Some(0.5)).unwrap(),
            current.convert_line(1, text, Some(0.5)).unwrap()
        );
    }
}

#[test]
fn v2_rejects_unrounded_interpolation_in_every_value_bearing_emission() {
    let s = schema();
    for emission in [
        ItemEmission::ItemLevel {
            value: raw_range_value(),
        },
        ItemEmission::Quality {
            kind: quality(),
            amount: raw_range_value(),
        },
        ItemEmission::ItemParameter {
            slot: param(),
            value: raw_range_value(),
        },
        ItemEmission::Modifier {
            definition: modifier(),
            rolls: vec![ItemRollTemplate {
                slot: roll(),
                value: raw_range_value(),
            }],
        },
    ] {
        let mut i = raw_range_input(&s);
        i.schema_version = 2;
        i.rules[0].emissions = vec![emission];
        let err = OwnedItemLinePolicy::new(i.clone(), &s, ItemLineLimits::default()).unwrap_err();
        assert!(
            err.to_string().contains("requires item-line policy v3"),
            "{err}"
        );
        let err = decode_item_line_policy(
            &serde_json::to_vec(&i).unwrap(),
            &s,
            ItemLineLimits::default(),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("requires item-line policy v3"),
            "{err}"
        );
    }
    for version in [0, 1, OWNED_ITEM_LINE_POLICY_VERSION + 1, u32::MAX] {
        let mut i = raw_range_input(&s);
        i.schema_version = version;
        assert!(
            matches!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()), Err(ItemLineError::UnsupportedVersion(v)) if v == version)
        );
    }
}

#[test]
fn unrounded_interpolation_wire_is_explicit_strict_and_round_trips() {
    let raw = raw_range_value();
    let wire = serde_json::to_value(&raw).unwrap();
    assert_eq!(
        wire,
        serde_json::json!({"kind":"interpolate_unrounded_offset","value":{"lower":"lower","upper":"upper"}})
    );
    assert_eq!(
        serde_json::from_value::<ItemLineValue>(wire.clone()).unwrap(),
        raw
    );
    for extra in ["quantum", "rounding", "unknown"] {
        let mut invalid = wire.clone();
        invalid["value"][extra] = serde_json::json!(1);
        assert!(serde_json::from_value::<ItemLineValue>(invalid).is_err());
    }
    for missing in ["lower", "upper"] {
        let mut invalid = wire.clone();
        invalid["value"].as_object_mut().unwrap().remove(missing);
        assert!(serde_json::from_value::<ItemLineValue>(invalid).is_err());
    }
    assert!(serde_json::from_str::<ItemLineValue>(r#"{"kind":"interpolate_unrounded_offset","value":{"lower":"lower","lower":"other","upper":"upper"}}"#).is_err());
    let s = schema();
    let p = raw_range_policy(&s);
    let bytes = encode_item_line_policy(&p, ItemLineLimits::default()).unwrap();
    let restored = decode_item_line_policy(&bytes, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(restored.identity(), p.identity());
    assert_eq!(range_result(&restored, "Raw (2.00,2.01)", 0.5), qty(2.005));
}

#[test]
fn unrounded_interpolation_retains_schema_source_output_and_work_limits() {
    let s = schema();
    let i = raw_range_input(&s);
    let defaults = ItemLineLimits::default();
    for limits in [
        ItemLineLimits {
            max_captures: 1,
            ..defaults
        },
        ItemLineLimits {
            max_parts: 1,
            ..defaults
        },
        ItemLineLimits {
            max_schema_work: 1,
            ..defaults
        },
        ItemLineLimits {
            max_wire_bytes: 1,
            ..defaults
        },
    ] {
        assert!(OwnedItemLinePolicy::new(i.clone(), &s, limits).is_err());
    }
    for limits in [
        ItemLineLimits {
            max_source_bytes: 1,
            ..defaults
        },
        ItemLineLimits {
            max_line_bytes: 1,
            ..defaults
        },
        ItemLineLimits {
            max_work: 1,
            ..defaults
        },
        ItemLineLimits {
            max_output_declarations: 1,
            ..defaults
        },
    ] {
        let p = OwnedItemLinePolicy::new(i.clone(), &s, limits).unwrap();
        assert!(matches!(
            p.convert_line(1, "Raw (2.00,2.01)", Some(0.5)),
            Err(ItemLineError::Limit(_))
        ));
    }
    let p = raw_range_policy(&s);
    let bytes = encode_item_line_policy(&p, defaults).unwrap();
    let tight = ItemLineLimits {
        max_wire_bytes: bytes.len() - 1,
        ..defaults
    };
    assert!(decode_item_line_policy(&bytes, &s, tight).is_err());
    assert!(encode_item_line_policy(&p, tight).is_err());
    assert!(matches!(
        pending(&p.convert_line(1, "Raw (1001,1002)", Some(0.5)).unwrap()),
        ItemLinePending::ValueOutsideSchema { .. }
    ));
}

fn projection(source: ItemNumericSource, result: ItemNumericResult) -> ItemNumericProjection {
    ItemNumericProjection {
        source,
        negate: false,
        decimal: ItemNumericDecimal::Exact,
        result,
    }
}
fn projection_input(
    s: &OwnedDefinitionSchemaPackage,
    projection: ItemNumericProjection,
) -> ItemLinePolicyInput {
    let mut i = if matches!(projection.source, ItemNumericSource::Capture(_)) {
        let mut i = input(s);
        i.rules = vec![numeric_rule(
            DecimalSyntax::Scientific,
            ItemNumericSign::Optional,
        )];
        i
    } else {
        raw_range_input(s)
    };
    i.schema_version = OWNED_ITEM_LINE_POLICY_V4;
    let ItemEmission::Modifier { rolls, .. } = &mut i.rules[0].emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::NumericProjection(projection);
    i
}
fn projection_schema(direction: bool) -> OwnedDefinitionSchemaPackage {
    let mut raw = raw_schema();
    let SlotDescriptor::Parameter(row) = &mut raw.slots[0] else {
        unreachable!()
    };
    let SchemaState::Known(slot) = &mut row.schema else {
        unreachable!()
    };
    slot.value = if direction {
        ValueSchema::Boolean
    } else {
        ValueSchema::Quantity(qr(-f64::MAX, f64::MAX))
    };
    OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap()
}
fn projected(p: &OwnedItemLinePolicy, text: &str, fraction: Option<f64>) -> ParameterValue {
    let evidence = p.convert_line(1, text, fraction).unwrap();
    let ItemLineOutcome::Known { emissions, .. } = evidence.outcome else {
        panic!("{evidence:?}")
    };
    let ConvertedItemEmission::Modifier { rolls, .. } = &emissions[0] else {
        unreachable!()
    };
    rolls[0].value.clone()
}
fn projection_policy(
    s: &OwnedDefinitionSchemaPackage,
    recipe: ItemNumericProjection,
) -> OwnedItemLinePolicy {
    OwnedItemLinePolicy::new(projection_input(s, recipe), s, ItemLineLimits::default()).unwrap()
}
fn offset_source() -> ItemNumericSource {
    ItemNumericSource::InterpolateUnroundedOffset {
        lower: key("lower"),
        upper: key("upper"),
    }
}

#[test]
fn v4_projection_preserves_temporary_negative_zero_and_projects_canonical_quantities() {
    for decimal in [
        ItemNumericDecimal::Exact,
        ItemNumericDecimal::SignificantDigits { digits: 14 },
    ] {
        for negate in [false, true] {
            for invert in [false, true] {
                let s = projection_schema(true);
                let mut recipe = projection(
                    ItemNumericSource::Capture(key("x")),
                    ItemNumericResult::NegativeDirection { invert },
                );
                recipe.negate = negate;
                recipe.decimal = decimal;
                let p = projection_policy(&s, recipe);
                for (text, negative) in [
                    ("-0 units", true),
                    ("+0 units", false),
                    ("-0.000 units", true),
                    ("-3 units", true),
                    ("3 units", false),
                ] {
                    assert_eq!(
                        projected(&p, text, None),
                        ParameterValue::Boolean(negative ^ negate ^ invert),
                        "{text} {negate} {invert} {decimal:?}"
                    );
                }
            }
        }
    }
    let s = projection_schema(false);
    for (result, expected) in [
        (ItemNumericResult::SignedQuantity, -2.75),
        (ItemNumericResult::Magnitude, 2.75),
    ] {
        let p = projection_policy(&s, projection(ItemNumericSource::Capture(key("x")), result));
        assert_eq!(projected(&p, "-2.75 units", None), qty(expected));
        let ParameterValue::Quantity(zero) = projected(&p, "-0 units", None) else {
            unreachable!()
        };
        assert!(
            !zero.value().is_sign_negative(),
            "Core quantities retain canonical zero"
        );
    }
    let s = projection_schema(true);
    let mut i = projection_input(
        &s,
        projection(
            ItemNumericSource::Capture(key("x")),
            ItemNumericResult::NegativeDirection { invert: false },
        ),
    );
    let ItemCaptureCodec::Value(codec) = &mut i.rules[0].captures[0].codec else {
        unreachable!()
    };
    let ValueCodecKind::Quantity { scale, .. } = &mut codec.codec else {
        unreachable!()
    };
    scale.numerator = BoundedInteger::new(-1).unwrap();
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(
        projected(&p, "-0 units", None),
        ParameterValue::Boolean(false)
    );
    assert_eq!(
        projected(&p, "+0 units", None),
        ParameterValue::Boolean(true)
    );
}

#[test]
fn v4_projection_negates_after_offset_and_reparses_before_direction() {
    let s = projection_schema(false);
    for (text, fraction, signed) in [
        ("Raw (-3,-2)", 0.5, -2.5),
        ("Raw (-3,2)", 0.5, -0.5),
        ("Raw (-2,3)", 0.5, 0.5),
        ("Raw (-2,2)", 0.5, 0.0),
    ] {
        for negate in [false, true] {
            let mut recipe = projection(offset_source(), ItemNumericResult::SignedQuantity);
            recipe.negate = negate;
            let p = projection_policy(&s, recipe);
            assert_eq!(
                projected(&p, text, Some(fraction)),
                qty(if negate { -signed } else { signed })
            );
        }
    }
    let s = projection_schema(true);
    let mut recipe = projection(
        offset_source(),
        ItemNumericResult::NegativeDirection { invert: false },
    );
    recipe.negate = true;
    recipe.decimal = ItemNumericDecimal::SignificantDigits { digits: 14 };
    let p = projection_policy(&s, recipe);
    assert_eq!(
        projected(&p, "Raw (-2,2)", Some(0.5)),
        ParameterValue::Boolean(true)
    );
    assert_eq!(
        projected(&p, "Raw (-3,2)", Some(0.5)),
        ParameterValue::Boolean(false)
    );
    assert_eq!(
        projected(&p, "Raw (-2,3)", Some(0.5)),
        ParameterValue::Boolean(true)
    );
}

#[test]
fn v4_projection_decimal_transport_is_distinct_from_exact_ieee_and_has_bounded_precision() {
    let s = projection_schema(false);
    let exact = projection_policy(
        &s,
        projection(offset_source(), ItemNumericResult::SignedQuantity),
    );
    let mut recipe = projection(offset_source(), ItemNumericResult::SignedQuantity);
    recipe.decimal = ItemNumericDecimal::SignificantDigits { digits: 14 };
    let decimal = projection_policy(&s, recipe.clone());
    let fraction = 0.49999999999999;
    assert_eq!(
        projected(&exact, "Raw (2,3)", Some(fraction)),
        qty(2.0 + fraction)
    );
    assert_eq!(projected(&decimal, "Raw (2,3)", Some(fraction)), qty(2.5));
    recipe.source = ItemNumericSource::Capture(key("x"));
    recipe.decimal = ItemNumericDecimal::SignificantDigits { digits: 1 };
    let one = projection_policy(&s, recipe.clone());
    for (text, value) in [
        ("125 units", 100.0),
        ("9.99 units", 10.0),
        ("0.00999 units", 0.01),
        ("-0.00999 units", -0.01),
    ] {
        assert_eq!(projected(&one, text, None), qty(value));
    }
    assert_eq!(
        pending(
            &one.convert_line(1, "1.7976931348623157e308 units", None)
                .unwrap()
        ),
        &ItemLinePending::InvalidNumericProjection
    );
    recipe.decimal = ItemNumericDecimal::SignificantDigits { digits: 17 };
    let seventeen = projection_policy(&s, recipe.clone());
    for (text, value) in [
        ("1.7976931348623157e308 units", f64::MAX),
        ("4.9406564584124654e-324 units", f64::from_bits(1)),
    ] {
        assert_eq!(projected(&seventeen, text, None), qty(value));
    }
    for digits in [0, 18, u8::MAX] {
        recipe.decimal = ItemNumericDecimal::SignificantDigits { digits };
        let i = projection_input(&s, recipe.clone());
        assert!(
            OwnedItemLinePolicy::new(i.clone(), &s, ItemLineLimits::default())
                .unwrap_err()
                .to_string()
                .contains("1..=17")
        );
        assert!(
            decode_item_line_policy(
                &serde_json::to_vec(&i).unwrap(),
                &s,
                ItemLineLimits::default()
            )
            .is_err()
        );
    }
}

#[test]
fn v4_projection_rejects_nonquantity_sources_wrong_units_and_invalid_ranges() {
    let s = projection_schema(false);
    for source in [ItemNumericSource::Capture(key("x")), offset_source()] {
        let mut i = projection_input(&s, projection(source, ItemNumericResult::SignedQuantity));
        for capture in &mut i.rules[0].captures {
            capture.codec = codec(true);
        }
        assert!(
            OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default())
                .unwrap_err()
                .to_string()
                .contains("requires Quantity")
        );
    }
    let mut raw = raw_schema();
    let different = UnitDefId::parse(ns(), "distinct").unwrap();
    raw.definitions.push(DefinitionDescriptor::Unit(known(
        different.clone(),
        UnitSchema {
            dimension: UnitDimension::PercentagePoints,
        },
    )));
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let mut i = projection_input(
        &s,
        projection(offset_source(), ItemNumericResult::SignedQuantity),
    );
    let ItemCaptureCodec::Value(codec) = &mut i.rules[0].captures[1].codec else {
        unreachable!()
    };
    let ValueCodecKind::Quantity { unit, .. } = &mut codec.codec else {
        unreachable!()
    };
    *unit = different;
    assert!(
        OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default())
            .unwrap_err()
            .to_string()
            .contains("same exact unit")
    );
    let s = projection_schema(false);
    let p = projection_policy(
        &s,
        projection(offset_source(), ItemNumericResult::SignedQuantity),
    );
    assert_eq!(
        pending(&p.convert_line(1, "Raw (2,3)", None).unwrap()),
        &ItemLinePending::MissingRangeFraction
    );
    for fraction in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert_eq!(
            pending(&p.convert_line(1, "Raw (2,3)", Some(fraction)).unwrap()),
            &ItemLinePending::InvalidRangeFraction
        );
    }
    for text in ["Raw (3,2)", "Raw (-1e308,1e308)"] {
        for fraction in [0.0, 0.5, 1.0] {
            assert_eq!(
                pending(&p.convert_line(1, text, Some(fraction)).unwrap()),
                &ItemLinePending::InvalidRange
            );
        }
    }
    assert!(matches!(
        pending(&p.convert_line(1, "Raw (1e999,2)", Some(0.5)).unwrap()),
        ItemLinePending::MalformedCapture { .. }
    ));
    let bad_shape = projection(
        ItemNumericSource::Capture(key("x")),
        ItemNumericResult::NegativeDirection { invert: false },
    );
    assert!(
        OwnedItemLinePolicy::new(
            projection_input(&s, bad_shape),
            &s,
            ItemLineLimits::default()
        )
        .is_err()
    );
}

#[test]
fn v4_projection_is_strictly_versioned_in_every_emission_and_wire_field() {
    let s = schema();
    let value = ItemLineValue::NumericProjection(projection(
        offset_source(),
        ItemNumericResult::SignedQuantity,
    ));
    for emission in [
        ItemEmission::ItemLevel {
            value: value.clone(),
        },
        ItemEmission::Quality {
            kind: quality(),
            amount: value.clone(),
        },
        ItemEmission::ItemParameter {
            slot: param(),
            value: value.clone(),
        },
        ItemEmission::Modifier {
            definition: modifier(),
            rolls: vec![ItemRollTemplate {
                slot: roll(),
                value: value.clone(),
            }],
        },
    ] {
        for version in [OWNED_ITEM_LINE_POLICY_V2, OWNED_ITEM_LINE_POLICY_V3] {
            let mut i = raw_range_input(&s);
            i.schema_version = version;
            i.rules[0].emissions = vec![emission.clone()];
            assert!(
                OwnedItemLinePolicy::new(i.clone(), &s, ItemLineLimits::default())
                    .unwrap_err()
                    .to_string()
                    .contains("requires item-line policy v4")
            );
            assert!(
                decode_item_line_policy(
                    &serde_json::to_vec(&i).unwrap(),
                    &s,
                    ItemLineLimits::default()
                )
                .unwrap_err()
                .to_string()
                .contains("requires item-line policy v4")
            );
        }
    }
    let wire = serde_json::to_value(&value).unwrap();
    assert_eq!(
        wire,
        serde_json::json!({"kind":"numeric_projection","value":{"source":{"kind":"interpolate_unrounded_offset","value":{"lower":"lower","upper":"upper"}},"negate":false,"decimal":{"kind":"exact"},"result":{"kind":"signed_quantity"}}})
    );
    for field in ["source", "negate", "decimal", "result"] {
        let mut bad = wire.clone();
        bad["value"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ItemLineValue>(bad).is_err());
    }
    for path in ["", "/value", "/value/source/value"] {
        let mut bad = wire.clone();
        bad.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<ItemLineValue>(bad).is_err());
    }
    let duplicate = serde_json::to_string(&wire)
        .unwrap()
        .replace("\"negate\":false", "\"negate\":false,\"negate\":true");
    assert!(serde_json::from_str::<ItemLineValue>(&duplicate).is_err());
    let p = projection_policy(
        &s,
        projection(offset_source(), ItemNumericResult::Magnitude),
    );
    let bytes = encode_item_line_policy(&p, ItemLineLimits::default()).unwrap();
    let copy = decode_item_line_policy(&bytes, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(p.identity(), copy.identity());
    assert_eq!(projected(&copy, "Raw (-3,-2)", Some(0.5)), qty(2.5));
}

#[test]
fn v4_projection_preserves_wire_and_conversion_budgets() {
    let s = schema();
    let i = projection_input(
        &s,
        projection(offset_source(), ItemNumericResult::Magnitude),
    );
    let defaults = ItemLineLimits::default();
    for limits in [
        ItemLineLimits {
            max_schema_work: 1,
            ..defaults
        },
        ItemLineLimits {
            max_wire_bytes: 1,
            ..defaults
        },
        ItemLineLimits {
            max_captures: 1,
            ..defaults
        },
    ] {
        assert!(OwnedItemLinePolicy::new(i.clone(), &s, limits).is_err());
    }
    for limits in [
        ItemLineLimits {
            max_work: 1,
            ..defaults
        },
        ItemLineLimits {
            max_output_declarations: 1,
            ..defaults
        },
        ItemLineLimits {
            max_source_bytes: 1,
            ..defaults
        },
    ] {
        let p = OwnedItemLinePolicy::new(i.clone(), &s, limits).unwrap();
        assert!(matches!(
            p.convert_line(1, "Raw (2,3)", Some(0.5)),
            Err(ItemLineError::Limit(_))
        ));
    }
    let p = OwnedItemLinePolicy::new(i, &s, defaults).unwrap();
    let bytes = encode_item_line_policy(&p, defaults).unwrap();
    let tight = ItemLineLimits {
        max_wire_bytes: bytes.len() - 1,
        ..defaults
    };
    assert!(encode_item_line_policy(&p, tight).is_err());
    assert!(decode_item_line_policy(&bytes, &s, tight).is_err());
    assert!(matches!(
        pending(&p.convert_line(1, "Raw (1001,1002)", Some(0.5)).unwrap()),
        ItemLinePending::ValueOutsideSchema { .. }
    ));
}

fn partial_modifier_schema() -> SchemaPackageInput {
    let mut s = raw_schema();
    let DefinitionDescriptor::Modifier(row) = &mut s.definitions[2] else {
        unreachable!()
    };
    let SchemaState::Known(modifier_schema) = &mut row.schema else {
        unreachable!()
    };
    modifier_schema.declarations.parameters.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(modifier().address()),
            facet: SchemaFacet::InputSchema,
            code: key("remaining-inputs"),
        }],
    };
    s
}

#[test]
fn v4_preserves_known_partial_modifier_rolls_without_changing_legacy_pending() {
    let s =
        OwnedDefinitionSchemaPackage::new(partial_modifier_schema(), OwnedSchemaLimits::default())
            .unwrap();
    for version in [
        OWNED_ITEM_LINE_POLICY_V2,
        OWNED_ITEM_LINE_POLICY_V3,
        OWNED_ITEM_LINE_POLICY_V4,
        OWNED_ITEM_LINE_POLICY_V5,
        OWNED_ITEM_LINE_POLICY_VERSION,
    ] {
        let mut i = input(&s);
        i.schema_version = version;
        let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
        let e = p.convert_line(1, "Speed: 25", None).unwrap();
        if version < OWNED_ITEM_LINE_POLICY_V4 {
            assert!(matches!(
                pending(&e),
                ItemLinePending::Schema {
                    status: ItemSchemaUnknown::Partial,
                    ..
                }
            ));
            continue;
        }
        let ItemLineOutcome::Known { emissions, .. } = &e.outcome else {
            panic!("{e:?}")
        };
        let ConvertedItemEmission::Modifier {
            rolls,
            rolls_closure,
            ..
        } = &emissions[0]
        else {
            unreachable!()
        };
        assert_eq!(rolls[0].value, qty(25.0));
        let SchemaClosure::Partial { gaps } = rolls_closure else {
            panic!("must retain incomplete membership")
        };
        assert_eq!(gaps[0].code, key("remaining-inputs"));
        assert!(serde_json::to_value(&emissions[0]).unwrap()["value"]["rolls_closure"].is_object());
        let aggregate = p.convert_text("Injected Base\nSpeed: 25").unwrap();
        assert_eq!(aggregate.modifiers.len(), 1);
        assert_eq!(&aggregate.modifiers[0].rolls_closure, rolls_closure);
    }
    for version in [
        OWNED_ITEM_LINE_POLICY_V2,
        OWNED_ITEM_LINE_POLICY_V3,
        OWNED_ITEM_LINE_POLICY_V4,
        OWNED_ITEM_LINE_POLICY_V5,
        OWNED_ITEM_LINE_POLICY_VERSION,
    ] {
        let s = schema();
        let mut i = input(&s);
        i.schema_version = version;
        let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
        let e = p.convert_line(1, "Speed: 25", None).unwrap();
        let ItemLineOutcome::Known { emissions, .. } = e.outcome else {
            unreachable!()
        };
        let json = serde_json::to_value(&emissions[0]).unwrap();
        assert!(json["value"].get("rolls_closure").is_none());
        let aggregate = p.convert_text("Injected Base\nSpeed: 25").unwrap();
        assert!(
            serde_json::to_value(&aggregate.modifiers[0])
                .unwrap()
                .get("rolls_closure")
                .is_none()
        );
    }
}

#[test]
fn v4_partial_modifier_membership_never_admits_missing_or_invalid_facts() {
    let s =
        OwnedDefinitionSchemaPackage::new(partial_modifier_schema(), OwnedSchemaLimits::default())
            .unwrap();
    let mut i = input(&s);
    i.schema_version = OWNED_ITEM_LINE_POLICY_V4;
    let modifier_rule = i
        .rules
        .iter_mut()
        .find(|r| r.id == key("modifier"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut modifier_rule.emissions[0] else {
        unreachable!()
    };
    rolls.clear();
    assert!(
        OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default())
            .unwrap_err()
            .to_string()
            .contains("required modifier roll is missing")
    );
    let mut i = input(&s);
    i.schema_version = OWNED_ITEM_LINE_POLICY_V4;
    let modifier_rule = i
        .rules
        .iter_mut()
        .find(|r| r.id == key("modifier"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut modifier_rule.emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::Property {
        property: key("explicit-fact"),
    };
    // Still rejects a Boolean value for a declared numeric roll.
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    let mut i = input(&s);
    i.schema_version = OWNED_ITEM_LINE_POLICY_V4;
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    assert!(matches!(
        pending(&p.convert_line(1, "Speed: 1001", None).unwrap()),
        ItemLinePending::ValueOutsideSchema { .. }
    ));
    assert!(matches!(
        pending(&p.convert_line(1, "Speed: bad", None).unwrap()),
        ItemLinePending::MalformedCapture { .. }
    ));
    let mut raw = partial_modifier_schema();
    let SlotDescriptor::Parameter(row) = &mut raw.slots[0] else {
        unreachable!()
    };
    row.schema = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Slot(ParameterSlotDefId::address(&roll())),
            facet: SchemaFacet::InputSchema,
            code: key("unconverted-roll"),
        }],
    };
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let mut i = input(&s);
    i.schema_version = OWNED_ITEM_LINE_POLICY_V4;
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    assert!(matches!(
        pending(&p.convert_line(1, "Speed: 25", None).unwrap()),
        ItemLinePending::Schema {
            status: ItemSchemaUnknown::Unmapped,
            ..
        }
    ));
}

#[test]
fn v4_numeric_stage_and_partial_gap_expansion_have_explicit_budgets() {
    let s = schema();
    let old = raw_range_input(&s);
    let new = projection_input(
        &s,
        projection(offset_source(), ItemNumericResult::SignedQuantity),
    );
    let defaults = ItemLineLimits::default();
    let old_work = (1..512)
        .find(|work| {
            let p = OwnedItemLinePolicy::new(
                old.clone(),
                &s,
                ItemLineLimits {
                    max_work: *work,
                    ..defaults
                },
            )
            .unwrap();
            p.convert_line(1, "Raw (2,3)", Some(0.5)).is_ok()
        })
        .expect("small fixture has bounded work");
    for (extra, succeeds) in [(63, false), (64, true)] {
        let p = OwnedItemLinePolicy::new(
            new.clone(),
            &s,
            ItemLineLimits {
                max_work: old_work + extra,
                ..defaults
            },
        )
        .unwrap();
        assert_eq!(p.convert_line(1, "Raw (2,3)", Some(0.5)).is_ok(), succeeds);
    }
    let s =
        OwnedDefinitionSchemaPackage::new(partial_modifier_schema(), OwnedSchemaLimits::default())
            .unwrap();
    let mut i = input(&s);
    i.schema_version = OWNED_ITEM_LINE_POLICY_V4;
    // One matched candidate, one modifier, one roll and one retained schema gap.
    for (budget, succeeds) in [(3, false), (4, true)] {
        let p = OwnedItemLinePolicy::new(
            i.clone(),
            &s,
            ItemLineLimits {
                max_output_declarations: budget,
                ..defaults
            },
        )
        .unwrap();
        assert_eq!(p.convert_line(1, "Speed: 25", None).is_ok(), succeeds);
    }
}

#[test]
fn v4_padded_zero_sign_is_scanned_once_and_charged_before_repeated_projection() {
    let s = projection_schema(true);
    let mut i = projection_input(
        &s,
        projection(
            ItemNumericSource::Capture(key("x")),
            ItemNumericResult::NegativeDirection { invert: false },
        ),
    );
    i.whitespace = WhitespacePolicy::Exact;
    i.rules[0].pattern = vec![cap("x")];
    let ItemCaptureCodec::Value(codec) = &mut i.rules[0].captures[0].codec else {
        unreachable!()
    };
    codec.whitespace = WhitespacePolicy::TrimAscii;
    i.rules[0].emissions = vec![i.rules[0].emissions[0].clone(); 32];
    let text = format!("{}-0{}", " ".repeat(4096), " ".repeat(4096));
    let defaults = ItemLineLimits::default();
    let bounded_work = 2 * (text.len() + 1) + 32 * 80 + 100;
    let p = OwnedItemLinePolicy::new(
        i.clone(),
        &s,
        ItemLineLimits {
            max_work: bounded_work,
            ..defaults
        },
    )
    .unwrap();
    let e = p.convert_line(1, &text, None).unwrap();
    let ItemLineOutcome::Known { emissions, .. } = e.outcome else {
        panic!("{e:?}")
    };
    assert_eq!(emissions.len(), 32);
    for emission in emissions {
        let ConvertedItemEmission::Modifier { rolls, .. } = emission else {
            unreachable!()
        };
        assert_eq!(rolls[0].value, ParameterValue::Boolean(true));
    }
    // A budget covering one source decode but not the additional zero-sign scan
    // cannot complete, even though all output values themselves are tiny.
    let p = OwnedItemLinePolicy::new(
        i,
        &s,
        ItemLineLimits {
            max_work: text.len() + 32 * 80 + 100,
            ..defaults
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_line(1, &text, None),
        Err(ItemLineError::Limit("work"))
    ));
}

fn lexical_input(s: &OwnedDefinitionSchemaPackage) -> ItemLinePolicyInput {
    let mut i = projection_input(
        s,
        projection(
            ItemNumericSource::Capture(key("x")),
            ItemNumericResult::NegativeDirection { invert: false },
        ),
    );
    i.schema_version = OWNED_ITEM_LINE_POLICY_VERSION;
    let ItemEmission::Modifier { rolls, .. } = &mut i.rules[0].emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::NumericLexicalProperty {
        capture: key("x"),
        property: ItemNumericLexicalProperty::HasDecimalPoint,
    };
    i
}

#[test]
fn v6_lexical_fact_retains_spelling_without_inventing_numeric_precision() {
    let s = projection_schema(true);
    let p = OwnedItemLinePolicy::new(lexical_input(&s), &s, ItemLineLimits::default()).unwrap();
    for (token, has_point) in [
        ("10", false),
        ("10.0", true),
        ("1e1", false),
        ("1.0e1", true),
        ("-0", false),
        ("-0.0", true),
        ("+.5", true),
        ("1.", true),
        ("00010", false),
        ("10.000000", true),
    ] {
        assert_eq!(
            projected(&p, &format!("{token} units"), None),
            ParameterValue::Boolean(has_point),
            "{token}"
        );
    }
    let evidence = p.convert_line(1, "10.0 units", None).unwrap();
    let ItemLineOutcome::Known { emissions, .. } = evidence.outcome else {
        unreachable!()
    };
    let json = serde_json::to_string(&emissions).unwrap();
    assert!(
        !json.contains("10.0"),
        "native facts must not carry numeric source tokens"
    );
}

#[test]
fn v6_lexical_projection_obeys_both_pattern_and_numeric_decoder() {
    let s = projection_schema(true);
    for (syntax, accepted, rejected) in [
        (DecimalSyntax::Integer, "10", "10.0"),
        (DecimalSyntax::Decimal, "10.0", "1e1"),
        (DecimalSyntax::Scientific, "1.0e1", "1e+"),
    ] {
        let mut i = lexical_input(&s);
        i.rules[0].pattern[0] = numeric("x", syntax, ItemNumericSign::Forbidden);
        let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
        assert!(matches!(
            p.convert_line(1, &format!("{accepted} units"), None)
                .unwrap()
                .outcome,
            ItemLineOutcome::Known { .. }
        ));
        for token in [rejected, "-1", "+1", "NaN", "inf", "1_0", "１", "1..0"] {
            assert_eq!(
                pending(&p.convert_line(1, &format!("{token} units"), None).unwrap()),
                &ItemLinePending::UnknownLine
            );
        }
    }
    let p = OwnedItemLinePolicy::new(lexical_input(&s), &s, ItemLineLimits::default()).unwrap();
    assert!(matches!(
        pending(&p.convert_line(1, "1.0e9999 units", None).unwrap()),
        ItemLinePending::MalformedCapture { .. }
    ));
    let mut i = lexical_input(&s);
    let ItemCaptureCodec::Value(codec) = &mut i.rules[0].captures[0].codec else {
        unreachable!()
    };
    codec.codec = ValueCodecKind::Integer {
        syntax: DecimalSyntax::Scientific,
    };
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(
        projected(&p, "10.0 units", None),
        ParameterValue::Boolean(true)
    );
    assert!(matches!(
        pending(&p.convert_line(1, "10.5 units", None).unwrap()),
        ItemLinePending::MalformedCapture { .. }
    ));
}

#[test]
fn v6_lexical_projection_requires_declared_numeric_token_and_boolean_roll() {
    let s = projection_schema(true);
    for case in 0..4 {
        let mut i = lexical_input(&s);
        match case {
            0 => i.rules[0].pattern[0] = cap("x"),
            1 => {
                let ItemEmission::Modifier { rolls, .. } = &mut i.rules[0].emissions[0] else {
                    unreachable!()
                };
                rolls[0].value = ItemLineValue::NumericLexicalProperty {
                    capture: key("missing"),
                    property: ItemNumericLexicalProperty::HasDecimalPoint,
                };
            }
            2 => {
                let ItemCaptureCodec::Value(codec) = &mut i.rules[0].captures[0].codec else {
                    unreachable!()
                };
                codec.codec = ValueCodecKind::Boolean {
                    tokens: vec![BooleanToken {
                        token: "1.0".into(),
                        value: true,
                    }],
                };
            }
            _ => i.rules[0].captures[0].codec = ItemCaptureCodec::OpaqueText,
        }
        assert!(
            OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err(),
            "case {case}"
        );
    }
    let numeric_schema = projection_schema(false);
    assert!(
        OwnedItemLinePolicy::new(
            lexical_input(&numeric_schema),
            &numeric_schema,
            ItemLineLimits::default()
        )
        .is_err()
    );
    let mut i = lexical_input(&s);
    let ItemEmission::Modifier { rolls, .. } = &i.rules[0].emissions[0] else {
        unreachable!()
    };
    i.rules[0].emissions = vec![ItemEmission::ItemParameter {
        slot: param(),
        value: rolls[0].value.clone(),
    }];
    assert!(
        OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default())
            .unwrap_err()
            .to_string()
            .contains("require modifier rolls")
    );
}

#[test]
fn v6_lexical_fact_is_required_and_never_defaulted_from_missing_input() {
    let s = projection_schema(true);
    let mut i = lexical_input(&s);
    let p = OwnedItemLinePolicy::new(i.clone(), &s, ItemLineLimits::default()).unwrap();
    assert_eq!(
        projected(&p, "10 units", None),
        ParameterValue::Boolean(false)
    );
    assert_eq!(
        pending(&p.convert_line(1, " units", None).unwrap()),
        &ItemLinePending::UnknownLine
    );
    let ItemEmission::Modifier { rolls, .. } = &mut i.rules[0].emissions[0] else {
        unreachable!()
    };
    rolls.clear();
    assert!(
        OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default())
            .unwrap_err()
            .to_string()
            .contains("required modifier roll is missing")
    );
}

#[test]
fn v6_lexical_wire_is_strict_and_prior_identity_domains_are_preserved() {
    use poe_optimizer_core::owned_content::digest_owned;
    let s = projection_schema(true);
    let current = lexical_input(&s);
    for version in [2, 3, 4, 5] {
        let mut i = current.clone();
        i.schema_version = version;
        assert!(
            OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default())
                .unwrap_err()
                .to_string()
                .contains("requires item-line policy v6")
        );
    }
    let p = OwnedItemLinePolicy::new(current, &s, ItemLineLimits::default()).unwrap();
    let wire = encode_item_line_policy(&p, ItemLineLimits::default()).unwrap();
    let raw: serde_json::Value = serde_json::from_slice(&wire).unwrap();
    let value = &raw["rules"][0]["emissions"][0]["value"]["rolls"][0]["value"];
    assert_eq!(value["kind"], "numeric_lexical_property");
    assert_eq!(value["value"]["property"], "has_decimal_point");
    for modified in [
        serde_json::json!({"kind":"numeric_lexical_property","value":{"capture":"x","property":"unknown"}}),
        serde_json::json!({"kind":"numeric_lexical_property","value":{"capture":"x","property":"has_decimal_point","extra":true}}),
    ] {
        assert!(serde_json::from_value::<ItemLineValue>(modified).is_err());
    }
    assert_eq!(
        decode_item_line_policy(&wire, &s, ItemLineLimits::default())
            .unwrap()
            .identity(),
        p.identity()
    );
    assert_eq!(
        &digest_owned(
            "owned-item-line-policy-v6",
            p.input(),
            ItemLineLimits::default().max_wire_bytes
        )
        .unwrap(),
        p.identity()
    );
    let s = schema();
    for (version, domain) in [
        (2, "owned-item-line-policy-v2"),
        (3, "owned-item-line-policy-v3"),
        (4, "owned-item-line-policy-v4"),
        (5, "owned-item-line-policy-v5"),
    ] {
        let mut i = input(&s);
        i.schema_version = version;
        let old_bytes = serde_json::to_vec(&i).unwrap();
        let expected = digest_owned(domain, &i, ItemLineLimits::default().max_wire_bytes).unwrap();
        let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
        assert_eq!(p.identity(), &expected);
        assert_eq!(
            encode_item_line_policy(&p, ItemLineLimits::default()).unwrap(),
            old_bytes
        );
    }
}

#[test]
fn v6_lexical_scans_are_charged_before_repeated_projection() {
    let s = projection_schema(true);
    let lexical = lexical_input(&s);
    let mut literal = lexical.clone();
    let ItemEmission::Modifier { rolls, .. } = &mut literal.rules[0].emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::Literal(ParameterValue::Boolean(false));
    let text = "10.000000 units";
    let defaults = ItemLineLimits::default();
    let minimum = |input: &ItemLinePolicyInput| {
        (1..2048)
            .find(|work| {
                OwnedItemLinePolicy::new(
                    input.clone(),
                    &s,
                    ItemLineLimits {
                        max_work: *work,
                        ..defaults
                    },
                )
                .unwrap()
                .convert_line(1, text, None)
                .is_ok()
            })
            .unwrap()
    };
    let base_work = minimum(&literal);
    let projected_work = minimum(&lexical);
    // Two key lookups of two charged bytes; nine-byte token plus scan sentinel.
    assert_eq!(projected_work - base_work, 14);
    let mut repeated = lexical.clone();
    repeated.rules[0].emissions = vec![repeated.rules[0].emissions[0].clone(); 16];
    let required = minimum(&repeated);
    let p = OwnedItemLinePolicy::new(
        repeated,
        &s,
        ItemLineLimits {
            max_work: required - 1,
            ..defaults
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_line(1, text, None),
        Err(ItemLineError::Limit("work"))
    ));
    for limits in [
        ItemLineLimits {
            max_source_bytes: text.len() - 1,
            ..defaults
        },
        ItemLineLimits {
            max_line_bytes: text.len() - 1,
            ..defaults
        },
        ItemLineLimits {
            max_output_declarations: 1,
            ..defaults
        },
    ] {
        let p = OwnedItemLinePolicy::new(lexical.clone(), &s, limits).unwrap();
        assert!(matches!(
            p.convert_line(1, text, None),
            Err(ItemLineError::Limit(_))
        ));
    }
}
