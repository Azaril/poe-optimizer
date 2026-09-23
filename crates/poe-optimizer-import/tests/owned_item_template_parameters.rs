//! Contextual item headers use exact injected template bindings, never game aliases.
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    owned_item_lines::*, owned_item_source::*, owned_mapping::*, owned_source::*, owned_value::*,
};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("context-test", "v1").unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn item(s: &str) -> ItemTemplateDefId {
    ItemTemplateDefId::parse(ns(), s).unwrap()
}
fn unit() -> UnitDefId {
    UnitDefId::parse(ns(), "percent").unwrap()
}
fn option(s: &str) -> OptionDefId {
    OptionDefId::parse(ns(), s).unwrap()
}
fn qty(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, unit()).unwrap())
}
fn slot(owner: &str, name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(item(owner)),
        slot: ParameterSlotDefId::parse(ns(), format!("{owner}-{name}")).unwrap(),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn ports(members: Vec<DeclaredSlot<ParameterSlotDefId>>) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(members),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn range(max: f64) -> QuantityRange {
    QuantityRange {
        minimum: FiniteQuantity::new(0.0, unit()).unwrap(),
        maximum: FiniteQuantity::new(max, unit()).unwrap(),
    }
}
fn raw_schema() -> SchemaPackageInput {
    let mut s = SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("release"),
        semantics_version: key("inputs"),
        definitions: vec![
            DefinitionDescriptor::Unit(known(
                unit(),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
            DefinitionDescriptor::Option(known(option("red"), OptionSchema {})),
            DefinitionDescriptor::Option(known(option("blue"), OptionSchema {})),
        ],
        slots: vec![],
    };
    for (name, maximum) in [("alpha", 100.0), ("beta", 10.0), ("gamma", 100.0)] {
        s.definitions.push(DefinitionDescriptor::ItemTemplate(known(
            item(name),
            ItemTemplateSchema {
                item_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                equipment_slots: DeclaredSet::complete(vec![]),
                socket_destinations: DeclaredSet::complete(vec![]),
                modifiers: DeclaredSet::complete(vec![]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: ports(vec![slot(name, "amount"), slot(name, "kind")]),
            },
        )));
        for (kind, value) in [
            ("amount", ValueSchema::Quantity(range(maximum))),
            (
                "kind",
                ValueSchema::Option {
                    allowed: DeclaredSet::complete(if name == "beta" {
                        vec![option("blue")]
                    } else {
                        vec![option("red"), option("blue")]
                    }),
                },
            ),
        ] {
            s.slots.push(SlotDescriptor::Parameter(known(
                slot(name, kind),
                ParameterSlotSchema {
                    value,
                    presence: SlotPresence::OptionalOnce,
                    sites: vec![ParameterSite::ItemParameter],
                },
            )));
        }
    }
    s
}
fn schema() -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(raw_schema(), OwnedSchemaLimits::default()).unwrap()
}
fn capture(option_value: bool) -> ItemCapture {
    ItemCapture {
        id: key("value"),
        codec: ItemCaptureCodec::Value(ValueCodecInput {
            namespace: ns(),
            whitespace: WhitespacePolicy::Exact,
            codec: if option_value {
                ValueCodecKind::Option {
                    tokens: vec![
                        OptionToken {
                            token: "red".into(),
                            value: option("red"),
                        },
                        OptionToken {
                            token: "blue".into(),
                            value: option("blue"),
                        },
                    ],
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
        }),
    }
}
fn contextual(id: &str, prefix: &str, kind: &str) -> ItemLineRule {
    ItemLineRule {
        id: key(id),
        pattern: vec![
            ItemPatternPart::Literal(prefix.into()),
            ItemPatternPart::Capture(key("value")),
        ],
        captures: vec![capture(kind == "kind")],
        emissions: vec![ItemEmission::TemplateParameter {
            bindings: vec![slot("alpha", kind), slot("beta", kind)],
            value: ItemLineValue::Capture(key("value")),
        }],
    }
}
fn policy_input(s: &OwnedDefinitionSchemaPackage) -> ItemLinePolicyInput {
    let mut rules = vec![
        contextual("amount", "CatalystQuality: ", "amount"),
        contextual("alias", "AmountAlias: ", "amount"),
        contextual("kind", "Catalyst: ", "kind"),
    ];
    for name in ["alpha", "beta", "gamma"] {
        rules.push(ItemLineRule {
            id: key(name),
            pattern: vec![ItemPatternPart::Literal(name.into())],
            captures: vec![],
            emissions: vec![ItemEmission::Template {
                definition: item(name),
            }],
        });
    }
    for (id, text) in [
        ("rarity", "Rarity: RARE"),
        ("title", "Fixture"),
        ("implicits", "Implicits: 0"),
    ] {
        rules.push(ItemLineRule {
            id: key(id),
            pattern: vec![ItemPatternPart::Literal(text.into())],
            captures: vec![],
            emissions: vec![ItemEmission::Metadata {
                role: key("preamble"),
            }],
        });
    }
    ItemLinePolicyInput {
        schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
        namespace: ns(),
        version: key("headers"),
        definitions: s.identity().clone(),
        whitespace: WhitespacePolicy::Exact,
        rules,
    }
}
fn policy(s: &OwnedDefinitionSchemaPackage) -> OwnedItemLinePolicy {
    OwnedItemLinePolicy::new(policy_input(s), s, ItemLineLimits::default()).unwrap()
}
fn issue(result: &ItemTextConversion<'_>, problem: ItemTextProblem) -> bool {
    result.issues.iter().any(|issue| issue.problem == problem)
}
fn static_alias(i: &mut ItemLinePolicyInput) {
    let mut alias = contextual("static", "Static: ", "amount");
    alias.emissions = vec![ItemEmission::ItemParameter {
        slot: slot("alpha", "amount"),
        value: ItemLineValue::Capture(key("value")),
    }];
    i.rules.push(alias);
}

#[test]
fn contextual_header_is_deferred_and_resolves_in_either_source_order() {
    let s = schema();
    let p = policy(&s);
    let line = p.convert_line(1, "CatalystQuality: 7", None).unwrap();
    let ItemLineOutcome::Known { emissions, .. } = line.outcome else {
        panic!("{line:?}")
    };
    assert_eq!(
        serde_json::to_value(&emissions[0]).unwrap(),
        serde_json::json!({"kind":"template_parameter","value":{"value":{"kind":"quantity","value":{"value":7.0,"unit":unit()}}}})
    );
    for (text, line) in [
        ("alpha\nCatalystQuality: 7", 2),
        ("CatalystQuality: 7\nalpha", 1),
    ] {
        let converted = p.convert_text(text).unwrap();
        assert_eq!(converted.parameters.len(), 1);
        assert_eq!(
            converted.parameters[0].assignment,
            ParameterAssignment {
                slot: slot("alpha", "amount"),
                value: qty(7.0)
            }
        );
        assert_eq!(converted.parameters[0].line, line);
        assert!(converted.issues.is_empty(), "{converted:?}");
    }
    let converted = p.convert_text("beta\nCatalystQuality: 7").unwrap();
    assert_eq!(
        converted.parameters[0].assignment.slot,
        slot("beta", "amount")
    );
    let mut i = policy_input(&s);
    static_alias(&mut i);
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    let result = p.convert_text("Catalyst: red\nStatic: 7\nalpha").unwrap();
    assert_eq!(
        result.parameters.iter().map(|p| p.line).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn selected_target_alone_controls_range_and_option_membership() {
    let s = schema();
    let p = policy(&s);
    for (text, accepted) in [
        ("alpha\nCatalystQuality: 50", true),
        ("beta\nCatalystQuality: 50", false),
        ("alpha\nCatalyst: red", true),
        ("beta\nCatalyst: red", false),
        ("beta\nCatalyst: blue", true),
    ] {
        let result = p.convert_text(text).unwrap();
        assert_eq!(!result.parameters.is_empty(), accepted, "{text}");
        assert_eq!(
            issue(&result, ItemTextProblem::ParameterOutsideSchema),
            !accepted
        );
    }
    // A literal acceptable on one target need not lie inside every target range.
    let mut i = policy_input(&s);
    let ItemEmission::TemplateParameter { value, .. } = &mut i.rules[0].emissions[0] else {
        unreachable!()
    };
    *value = ItemLineValue::Literal(qty(50.0));
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    assert_eq!(
        p.convert_text("alpha\nCatalystQuality: 7")
            .unwrap()
            .parameters[0]
            .assignment
            .value,
        qty(50.0)
    );
}

#[test]
fn missing_or_ambiguous_template_never_fabricates_a_binding() {
    let s = schema();
    let p = policy(&s);
    for text in [
        "CatalystQuality: 7",
        "alpha\nbeta\nCatalystQuality: 7",
        "alpha\nalpha\nCatalystQuality: 7",
    ] {
        let result = p.convert_text(text).unwrap();
        assert!(result.parameters.is_empty());
        assert!(issue(&result, ItemTextProblem::TemplateUnavailable));
    }
    let result = p.convert_text("gamma\nCatalystQuality: 7").unwrap();
    assert!(result.parameters.is_empty());
    assert!(issue(
        &result,
        ItemTextProblem::TemplateParameterUnavailable
    ));
}

#[test]
fn mapped_static_and_unresolved_sibling_aliases_share_duplicate_detection() {
    let s = schema();
    let mut i = policy_input(&s);
    static_alias(&mut i);
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    for text in [
        "alpha\nCatalystQuality: 7\nAmountAlias: 8",
        "alpha\nStatic: 7\nCatalystQuality: 8",
        "alpha\nStatic: 7\nAmountAlias: malformed",
        "alpha\nAmountAlias: malformed\nCatalystQuality: 8",
    ] {
        let result = p.convert_text(text).unwrap();
        assert!(result.parameters.is_empty(), "{result:?}");
        assert!(issue(&result, ItemTextProblem::DuplicateParameter));
    }
    let mut i = policy_input(&s);
    let mut duplicate = i.rules[1].clone();
    duplicate.id = key("ambiguous");
    i.rules.push(duplicate);
    let p = OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).unwrap();
    let result = p
        .convert_text("alpha\nCatalystQuality: 7\nAmountAlias: 8")
        .unwrap();
    assert!(result.parameters.is_empty());
    assert!(issue(&result, ItemTextProblem::DuplicateParameter));
}

#[test]
fn partial_target_declarations_keep_known_assignment_without_claiming_completeness() {
    let mut raw = raw_schema();
    let row = raw
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::ItemTemplate(r) if r.id == item("alpha") => Some(r),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(owner) = &mut row.schema else {
        unreachable!()
    };
    owner.declarations.parameters.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(item("alpha").address()),
            facet: SchemaFacet::InputSchema,
            code: key("remaining-inputs"),
        }],
    };
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let p = policy(&s);
    let result = p.convert_text("alpha\nCatalystQuality: 7").unwrap();
    assert_eq!(result.parameters.len(), 1);
    assert!(issue(&result, ItemTextProblem::SchemaPartial));
    let mut raw = s.input().clone();
    let row = raw
        .slots
        .iter_mut()
        .find_map(|s| match s {
            SlotDescriptor::Parameter(r) if r.id == slot("beta", "amount") => Some(r),
            _ => None,
        })
        .unwrap();
    row.schema = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Slot(ParameterSlotDefId::address(&slot("beta", "amount"))),
            facet: SchemaFacet::InputSchema,
            code: key("unknown-slot"),
        }],
    };
    let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let p = policy(&s);
    assert_eq!(
        p.convert_text("alpha\nCatalystQuality: 7")
            .unwrap()
            .parameters
            .len(),
        1
    );
    assert!(
        p.convert_text("beta\nCatalystQuality: 7")
            .unwrap()
            .parameters
            .is_empty()
    );
}

#[test]
fn contextual_bindings_reject_duplicates_foreign_owners_types_and_sites() {
    let s = schema();
    for bindings in [
        vec![],
        vec![slot("alpha", "amount"), slot("alpha", "amount")],
        vec![slot("alpha", "amount"), slot("alpha", "kind")],
        vec![DeclaredSlot {
            declaration: SlotOwnerDefId::Modifier(ModifierDefId::parse(ns(), "foreign").unwrap()),
            slot: ParameterSlotDefId::parse(ns(), "foreign").unwrap(),
        }],
    ] {
        let mut i = policy_input(&s);
        let ItemEmission::TemplateParameter {
            bindings: targets, ..
        } = &mut i.rules[0].emissions[0]
        else {
            unreachable!()
        };
        *targets = bindings;
        assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
    }
    for invalid in [0, 1, 2] {
        let mut raw = raw_schema();
        let row = raw
            .slots
            .iter_mut()
            .find_map(|s| match s {
                SlotDescriptor::Parameter(r) if r.id == slot("beta", "amount") => Some(r),
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(slot_schema) = &mut row.schema else {
            unreachable!()
        };
        match invalid {
            0 => slot_schema.value = ValueSchema::Boolean,
            1 => slot_schema.sites = vec![ParameterSite::ModifierRoll],
            _ => {
                let different = UnitDefId::parse(ns(), "other-unit").unwrap();
                slot_schema.value = ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(0.0, different.clone()).unwrap(),
                    maximum: FiniteQuantity::new(10.0, different.clone()).unwrap(),
                });
                raw.definitions.push(DefinitionDescriptor::Unit(known(
                    different,
                    UnitSchema {
                        dimension: UnitDimension::PercentagePoints,
                    },
                )));
            }
        }
        let s = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default());
        // The schema loader may reject an impossible input site before policy binding.
        if let Ok(s) = s {
            assert!(
                OwnedItemLinePolicy::new(policy_input(&s), &s, ItemLineLimits::default()).is_err()
            );
        }
    }
    let mut i = policy_input(&s);
    let ItemEmission::TemplateParameter { value, .. } = &mut i.rules[0].emissions[0] else {
        unreachable!()
    };
    *value = ItemLineValue::Property {
        property: key("unknown"),
    };
    assert!(OwnedItemLinePolicy::new(i, &s, ItemLineLimits::default()).is_err());
}

#[test]
fn v5_gate_and_identity_do_not_change_older_policy_bytes_or_domains() {
    let s = schema();
    let i = policy_input(&s);
    for version in [
        OWNED_ITEM_LINE_POLICY_V2,
        OWNED_ITEM_LINE_POLICY_V3,
        OWNED_ITEM_LINE_POLICY_V4,
    ] {
        let mut old = i.clone();
        old.schema_version = version;
        assert!(
            OwnedItemLinePolicy::new(old.clone(), &s, ItemLineLimits::default())
                .unwrap_err()
                .to_string()
                .contains("requires item-line policy v5")
        );
        assert!(
            decode_item_line_policy(
                &serde_json::to_vec(&old).unwrap(),
                &s,
                ItemLineLimits::default()
            )
            .is_err()
        );
        old.rules.retain(|rule| {
            !rule
                .emissions
                .iter()
                .any(|e| matches!(e, ItemEmission::TemplateParameter { .. }))
        });
        let bytes = serde_json::to_vec(&old).unwrap();
        let domain = match version {
            OWNED_ITEM_LINE_POLICY_V2 => "owned-item-line-policy-v2",
            OWNED_ITEM_LINE_POLICY_V3 => "owned-item-line-policy-v3",
            OWNED_ITEM_LINE_POLICY_V4 => "owned-item-line-policy-v4",
            _ => unreachable!("only explicit legacy versions are tested"),
        };
        let expected =
            digest_owned(domain, &old, ItemLineLimits::default().max_wire_bytes).unwrap();
        let p = OwnedItemLinePolicy::new(old, &s, ItemLineLimits::default()).unwrap();
        assert_eq!(p.identity(), &expected);
        assert_eq!(
            encode_item_line_policy(&p, ItemLineLimits::default()).unwrap(),
            bytes
        );
        assert_eq!(
            decode_item_line_policy(&bytes, &s, ItemLineLimits::default())
                .unwrap()
                .identity(),
            &expected
        );
    }
    let p = policy(&s);
    let bytes = encode_item_line_policy(&p, ItemLineLimits::default()).unwrap();
    assert_eq!(
        p.identity(),
        &digest_owned(
            "owned-item-line-policy-v5",
            p.input(),
            ItemLineLimits::default().max_wire_bytes
        )
        .unwrap()
    );
    assert_eq!(
        decode_item_line_policy(&bytes, &s, ItemLineLimits::default())
            .unwrap()
            .identity(),
        p.identity()
    );
    let wire = serde_json::to_value(&i.rules[0].emissions[0]).unwrap();
    for field in ["bindings", "value"] {
        let mut invalid = wire.clone();
        invalid["value"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ItemEmission>(invalid).is_err());
    }
    let mut invalid = wire;
    invalid["value"]["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ItemEmission>(invalid).is_err());
}

fn source_policy(
    s: &OwnedDefinitionSchemaPackage,
    p: &OwnedItemLinePolicy,
) -> ItemSourceLayoutPolicy {
    ItemSourceLayoutPolicy::new(
        ItemSourceLayoutPolicyInput {
            schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
            namespace: ns(),
            version: key("source"),
            source: SourcePin {
                system: ExternalSourceSystem::PathOfBuilding2,
                revision: "test".into(),
                files: vec![SourceFilePin {
                    path: "test.json".into(),
                    sha256: "a".repeat(64),
                }],
            },
            item_lines: *p.identity(),
            dialect: ItemSourceDialect::PobExportedSingleTextV1,
            property_bindings: vec![],
            rule_layouts: p
                .input()
                .rules
                .iter()
                .map(|r| ItemRuleSourceLayout {
                    rule: r.id.clone(),
                    role: ItemRuleSourceRole::Header,
                })
                .collect(),
            template_layouts: ["alpha", "beta", "gamma"]
                .into_iter()
                .map(|n| ItemTemplateSourceLayout {
                    template: item(n),
                    load_index_prefix: ItemLoadIndexPrefix::NoGeneratedBuffMembers,
                })
                .collect(),
            template_defaults: ["alpha", "beta", "gamma"]
                .into_iter()
                .map(|n| ItemSourceTemplateDefaults {
                    template: item(n),
                    item_level: ItemSourceAbsentPolicy::Absent,
                    quality: ItemSourceAbsentPolicy::Absent,
                    parameters: vec![ItemSourceParameterDefault {
                        assignment: ParameterAssignment {
                            slot: slot(n, "amount"),
                            value: qty(5.0),
                        },
                        headers: vec!["Catalyst".into()],
                    }],
                })
                .collect(),
        },
        p,
        s,
        ItemSourceLimits::default(),
    )
    .unwrap()
}
fn source_plan(
    s: &OwnedDefinitionSchemaPackage,
    p: &OwnedItemLinePolicy,
    name: &str,
    headers: &str,
) -> ItemRangeAttribution {
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nFixture\n{name}\n{headers}Implicits: 0</Item></Items></PathOfBuilding2>"
    );
    let imported = support::source(&xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    source_policy(s, p)
        .attribute(&evidence, support::item_source(&imported, "7"), p)
        .unwrap()
}

#[test]
fn source_defaults_never_override_contextual_headers_and_missing_map_blocks_only_parameters() {
    let s = schema();
    let p = policy(&s);
    let plan = source_plan(&s, &p, "alpha", "");
    let empty = plan.convert(&p).unwrap();
    assert_eq!(empty.defaults.parameters.len(), 1);
    assert!(empty.defaults.item_level_absent && empty.defaults.quality_absent);
    // This admitted header is intentionally absent from the source default's
    // configured header names. The selected contextual slot must independently
    // suppress that fallback; arbitrary aliases are not admitted by this dialect.
    let plan = source_plan(&s, &p, "alpha", "CatalystQuality: 7\n");
    let explicit = plan.convert(&p).unwrap();
    assert_eq!(explicit.parameters.len(), 1);
    assert!(explicit.defaults.parameters.is_empty());
    let plan = source_plan(&s, &p, "gamma", "CatalystQuality: 7\n");
    let unmapped = plan.convert(&p).unwrap();
    assert!(unmapped.parameters.is_empty() && unmapped.defaults.parameters.is_empty());
    assert!(issue(
        &unmapped,
        ItemTextProblem::TemplateParameterUnavailable
    ));
    assert!(unmapped.defaults.item_level_absent && unmapped.defaults.quality_absent);
    for headers in [
        "CatalystQuality: broken\n",
        "CatalystQuality: 7\nCatalystQuality: 8\n",
        "CatalystQuality: 500\n",
    ] {
        let plan = source_plan(&s, &p, "alpha", headers);
        let converted = plan.convert(&p).unwrap();
        assert!(converted.parameters.is_empty(), "{headers} {converted:?}");
        assert!(converted.defaults.parameters.is_empty(), "{headers}");
    }
}

#[test]
fn contextual_fanout_work_and_output_are_bounded_without_copying_all_targets() {
    let s = schema();
    let input = policy_input(&s);
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
    ] {
        assert!(OwnedItemLinePolicy::new(input.clone(), &s, limits).is_err());
    }
    let p = OwnedItemLinePolicy::new(
        input.clone(),
        &s,
        ItemLineLimits {
            max_output_declarations: 2,
            ..defaults
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_line(1, "CatalystQuality: 7", None)
            .unwrap()
            .outcome,
        ItemLineOutcome::Known { .. }
    ));
    assert!(matches!(
        p.convert_text("alpha\nCatalystQuality: 7"),
        Err(ItemLineError::Limit("output declarations"))
    ));
    let p = OwnedItemLinePolicy::new(
        input.clone(),
        &s,
        ItemLineLimits {
            max_work: 1,
            ..defaults
        },
    )
    .unwrap();
    assert!(matches!(
        p.convert_text("alpha\nCatalystQuality: 7"),
        Err(ItemLineError::Limit("work"))
    ));
    let p = policy(&s);
    let bytes = encode_item_line_policy(&p, defaults).unwrap();
    let tight = ItemLineLimits {
        max_wire_bytes: bytes.len() - 1,
        ..defaults
    };
    assert!(decode_item_line_policy(&bytes, &s, tight).is_err());
    assert!(encode_item_line_policy(&p, tight).is_err());
}
