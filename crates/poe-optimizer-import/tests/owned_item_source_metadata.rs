//! Explicit source preamble classification, independent of native calculation semantics.
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

fn metadata(id: &str, prefix: &str) -> ItemLineRule {
    ItemLineRule {
        id: key(id),
        pattern: vec![
            ItemPatternPart::Literal(prefix.into()),
            ItemPatternPart::Capture(key("metadata")),
        ],
        captures: vec![ItemCapture {
            id: key("metadata"),
            codec: ItemCaptureCodec::OpaqueText,
        }],
        emissions: vec![ItemEmission::Metadata {
            role: key("source-metadata"),
        }],
    }
}
fn rebuild(a: &mut Artifacts, mut input: ItemSourceLayoutPolicyInput) {
    input.item_lines = *a.items.identity();
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
}
fn fixture() -> Artifacts {
    let mut a = artifacts();
    let mut lines = a.items.input().clone();
    let mut input = a.item_source.input().clone();
    for rule in [
        metadata("unique-id", "Unique ID: "),
        metadata("external-note", "External Note: "),
    ] {
        input.rule_layouts.push(ItemRuleSourceLayout {
            rule: rule.id.clone(),
            role: ItemRuleSourceRole::Header,
        });
        lines.rules.push(rule);
    }
    let mut numeric = lines
        .rules
        .iter()
        .find(|r| r.id == key("item-level"))
        .unwrap()
        .clone();
    numeric.id = key("record-number");
    numeric.pattern[0] = ItemPatternPart::Literal("Record Number: ".into());
    numeric.emissions = vec![ItemEmission::Metadata {
        role: key("source-metadata"),
    }];
    input.rule_layouts.push(ItemRuleSourceLayout {
        rule: numeric.id.clone(),
        role: ItemRuleSourceRole::Header,
    });
    lines.rules.push(numeric);
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    input.schema_version = OWNED_ITEM_SOURCE_PREAMBLE_POLICY_VERSION;
    input.dialect = ItemSourceDialect::PobExportedSingleTextPreambleV1 {
        flag_bindings: vec![],
        metadata_rules: vec![key("unique-id"), key("external-note"), key("record-number")],
    };
    rebuild(&mut a, input);
    a
}
fn version(a: &mut Artifacts, version: u32) {
    let mut input = a.item_source.input().clone();
    input.schema_version = version;
    input.dialect = match version {
        3 => ItemSourceDialect::PobExportedSingleTextV1,
        4 => ItemSourceDialect::PobExportedSingleTextFlagsV1 {
            flag_bindings: vec![],
        },
        5 => ItemSourceDialect::PobExportedSingleTextPreambleV1 {
            flag_bindings: vec![],
            metadata_rules: vec![],
        },
        _ => panic!("fixture version"),
    };
    rebuild(a, input);
}
fn metadata_ids(input: &mut ItemSourceLayoutPolicyInput) -> &mut Vec<OwnedDefinitionKey> {
    let ItemSourceDialect::PobExportedSingleTextPreambleV1 { metadata_rules, .. } =
        &mut input.dialect
    else {
        panic!("preamble fixture")
    };
    metadata_rules
}
fn xml(headers: &str, body: &str, overlays: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\n{headers}Implicits: 0\n{body}{overlays}</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(a: &Artifacts, xml: &str) -> ItemRangeAttribution {
    let imported = source(xml);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
        .unwrap()
}
fn proven(plan: &ItemRangeAttribution) {
    assert!(
        matches!(plan.report().layout, ItemLayoutStatus::Proven),
        "{:?}",
        plan.report()
    );
}

#[test]
fn wire_versions_keep_their_exact_domains_and_strict_dialect_shapes() {
    assert_eq!(OWNED_ITEM_SOURCE_POLICY_VERSION, 3);
    assert_eq!(OWNED_ITEM_SOURCE_FLAG_POLICY_VERSION, 4);
    assert_eq!(OWNED_ITEM_SOURCE_PREAMBLE_POLICY_VERSION, 5);
    for v in [3, 4, 5] {
        let mut a = fixture();
        if v != 5 {
            version(&mut a, v);
        }
        let input = a.item_source.input();
        let domain = match v {
            3 => "owned-item-source-policy-v3",
            4 => "owned-item-source-policy-v4",
            5 => "owned-item-source-policy-v5",
            _ => unreachable!(),
        };
        assert_eq!(
            *a.item_source.identity(),
            digest_owned(domain, input, 4 * 1024 * 1024).unwrap()
        );
        let expected_dialect = match v {
            3 => serde_json::json!("pob_exported_single_text_v1"),
            4 => serde_json::json!({"pob_exported_single_text_flags_v1":{"flag_bindings":[]}}),
            5 => {
                serde_json::json!({"pob_exported_single_text_preamble_v1":{"flag_bindings":[],"metadata_rules":["unique-id","external-note","record-number"]}})
            }
            _ => unreachable!(),
        };
        assert_eq!(
            serde_json::to_value(&input.dialect).unwrap(),
            expected_dialect
        );
        let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
        assert_eq!(bytes, serde_json::to_vec(input).unwrap());
        let decoded =
            decode_item_source_policy(&bytes, &a.items, &a.schema, Default::default()).unwrap();
        assert_eq!(decoded.identity(), a.item_source.identity());
        assert_eq!(
            encode_item_source_policy(&decoded, Default::default()).unwrap(),
            bytes
        );
        for wrong in [3, 4, 5].into_iter().filter(|n| *n != v) {
            let mut input = input.clone();
            input.schema_version = wrong;
            assert!(
                ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default())
                    .is_err()
            );
        }
        let limits = ItemSourceLimits {
            max_wire_bytes: bytes.len() - 1,
            ..Default::default()
        };
        assert!(encode_item_source_policy(&a.item_source, limits).is_err());
        assert!(decode_item_source_policy(&bytes, &a.items, &a.schema, limits).is_err());
    }
    for wire in [
        r#"{"pob_exported_single_text_preamble_v1":{"flag_bindings":[]}}"#,
        r#"{"pob_exported_single_text_preamble_v1":{"metadata_rules":[]}}"#,
        r#"{"pob_exported_single_text_preamble_v1":{"flag_bindings":[],"metadata_rules":null}}"#,
        r#"{"pob_exported_single_text_preamble_v1":{"flag_bindings":[],"metadata_rules":[],"extra":true}}"#,
        r#"{"pob_exported_single_text_preamble_v1":{"flag_bindings":[],"metadata_rules":[],"metadata_rules":[]}}"#,
    ] {
        assert!(
            serde_json::from_str::<ItemSourceDialect>(wire).is_err(),
            "{wire}"
        );
    }
}

#[test]
fn metadata_references_require_unique_known_headers_with_only_nonempty_metadata() {
    let a = fixture();
    for refs in [
        vec!["missing"],
        vec!["unique-id", "unique-id"],
        vec!["spell"],
        vec!["staff-template"],
        vec!["quality"],
        vec!["item-level"],
        vec!["ranged-staff-grant"],
    ] {
        let mut input = a.item_source.input().clone();
        *metadata_ids(&mut input) = refs.iter().map(|id| key(id)).collect();
        assert!(
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_err(),
            "{refs:?}"
        );
    }
    for role in [
        ItemRuleSourceRole::SingleModifier,
        ItemRuleSourceRole::Unresolved,
    ] {
        let mut input = a.item_source.input().clone();
        input
            .rule_layouts
            .iter_mut()
            .find(|r| r.rule == key("unique-id"))
            .unwrap()
            .role = role;
        assert!(
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_err()
        );
    }
    // Empty and mixed emission lists are valid general recipes but not metadata declarations.
    for mixed in [false, true] {
        let mut lines = a.items.input().clone();
        lines.rules.push(ItemLineRule {
            id: key("invalid-metadata"),
            pattern: vec![ItemPatternPart::Literal("Opaque State".into())],
            captures: vec![],
            emissions: if mixed {
                vec![
                    ItemEmission::Metadata {
                        role: key("source-metadata"),
                    },
                    ItemEmission::Template {
                        definition: a.staff.clone(),
                    },
                ]
            } else {
                vec![]
            },
        });
        let lines = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
        let mut input = a.item_source.input().clone();
        input.item_lines = *lines.identity();
        input.rule_layouts.push(ItemRuleSourceLayout {
            rule: key("invalid-metadata"),
            role: ItemRuleSourceRole::Header,
        });
        *metadata_ids(&mut input) = vec![key("invalid-metadata")];
        assert!(ItemSourceLayoutPolicy::new(input, &lines, &a.schema, Default::default()).is_err());
    }
    let mut input = a.item_source.input().clone();
    metadata_ids(&mut input).clear();
    assert!(ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_ok());
}

#[test]
fn admitted_metadata_preserves_raw_evidence_and_never_consumes_range_members() {
    let a = fixture();
    let raw = "Unique ID: {range:NaN}{tags:cold}{fractured}abc";
    let input = xml(
        &format!("{raw}\nExternal Note: arbitrary exporter field\n"),
        "{range:0}Grants Skill: Level (1-20) Firebolt\n128% increased Spell Damage",
        "<ModRange id=\"1\" range=\"0.5\"/>",
    );
    let plan = attribute(&a, &input);
    proven(&plan);
    let metadata = plan
        .report()
        .lines
        .iter()
        .find(|line| line.raw == raw)
        .unwrap();
    assert_eq!(metadata.semantic_text, raw);
    assert_eq!(metadata.rule, Some(key("unique-id")));
    assert!(metadata.member.is_none() && metadata.blockers.is_empty());
    assert!(
        metadata.property_tokens.is_empty()
            && metadata.flag_tokens.is_empty()
            && metadata.properties.is_empty()
    );
    assert_eq!(metadata.range, ItemRangeDecision::Absent);
    let members: Vec<_> = plan
        .report()
        .lines
        .iter()
        .filter_map(|line| line.member)
        .collect();
    assert_eq!(members.len(), 2);
    assert_eq!(
        (members[0].category, members[0].ordinal),
        (SourceModifierCategory::Explicit, 1)
    );
    assert_eq!(
        (members[1].category, members[1].ordinal),
        (SourceModifierCategory::Explicit, 2)
    );
    assert_eq!(plan.report().writes.len(), 2);
    assert_eq!(
        plan.report().writes[1].target,
        ItemRangeTarget::Line(members[0].line)
    );
    let converted = plan.convert(&a.items).unwrap();
    assert_eq!(converted.parameters.len(), 1);
    assert_eq!(
        converted.parameters[0].assignment.value,
        ParameterValue::Integer(BoundedInteger::new(11).unwrap())
    );
    assert!(
        matches!(&converted.modifiers[0].rolls[0].value, ParameterValue::Quantity(value) if value.value() == 128.0)
    );
    assert!(converted.issues.is_empty());
}

#[test]
fn old_or_unlisted_headers_stay_pending_and_duplicate_source_metadata_is_preserved() {
    let input = xml(
        "Unique ID: first\nUnique ID: second\n",
        "128% increased Spell Damage",
        "",
    );
    let a = fixture();
    let plan = attribute(&a, &input);
    proven(&plan);
    let raw: Vec<_> = plan
        .report()
        .lines
        .iter()
        .filter(|line| line.rule == Some(key("unique-id")))
        .map(|line| (line.raw.as_str(), line.member))
        .collect();
    assert_eq!(raw.len(), 2);
    assert_eq!(raw[0], ("Unique ID: first", None));
    assert_eq!(raw[1], ("Unique ID: second", None));
    assert_eq!(plan.convert(&a.items).unwrap().modifiers.len(), 1);
    for v in [3, 4, 5] {
        let mut a = fixture();
        version(&mut a, v);
        let plan = attribute(&a, &input);
        assert!(!matches!(plan.report().layout, ItemLayoutStatus::Proven));
        assert!(
            plan.report()
                .lines
                .iter()
                .filter(|line| line.raw.starts_with("Unique ID:"))
                .all(|line| line.blockers.contains(&ItemSourceProblem::UnknownHeader))
        );
    }
}

#[test]
fn metadata_requires_valid_capture_and_the_exact_preamble_position() {
    let a = fixture();
    let cases = [
        (
            xml("Record Number: nope\n", "128% increased Spell Damage", ""),
            "Record Number: nope",
            ItemSourceProblem::MalformedCapture,
        ),
        (
            xml("", "128% increased Spell Damage\nUnique ID: after", ""),
            "Unique ID: after",
            ItemSourceProblem::HeaderAfterModifiers,
        ),
        (
            xml("Unique ID: before\n", "128% increased Spell Damage", "").replace(
                "Ashen Staff\nUnique ID: before",
                "Unique ID: before\nAshen Staff",
            ),
            "Unique ID: before",
            ItemSourceProblem::UnknownHeader,
        ),
    ];
    for (input, raw, problem) in cases {
        let plan = attribute(&a, &input);
        assert!(!matches!(plan.report().layout, ItemLayoutStatus::Proven));
        assert!(
            plan.report()
                .lines
                .iter()
                .find(|line| line.raw == raw)
                .unwrap()
                .blockers
                .contains(&problem),
            "{:?}",
            plan.report()
        );
    }
    let plan = attribute(
        &a,
        &xml("Record Number: 0\n", "128% increased Spell Damage", ""),
    );
    proven(&plan);
    assert!(
        plan.report()
            .lines
            .iter()
            .find(|line| line.raw == "Record Number: 0")
            .unwrap()
            .member
            .is_none()
    );
}

#[test]
fn ambiguous_metadata_and_source_control_lifecycles_are_not_admitted() {
    let mut a = fixture();
    let mut lines = a.items.input().clone();
    lines
        .rules
        .push(metadata("ambiguous-note", "External Note: "));
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("ambiguous-note"),
        role: ItemRuleSourceRole::Header,
    });
    metadata_ids(&mut input).push(key("ambiguous-note"));
    rebuild(&mut a, input);
    let plan = attribute(
        &a,
        &xml(
            "External Note: duplicate grammar\n",
            "128% increased Spell Damage",
            "",
        ),
    );
    assert!(!matches!(plan.report().layout, ItemLayoutStatus::Proven));
    let line = plan
        .report()
        .lines
        .iter()
        .find(|line| line.raw.starts_with("External Note:"))
        .unwrap();
    assert!(line.member.is_none());
    assert!(
        line.pending_candidates.contains(&key("external-note"))
            && line.pending_candidates.contains(&key("ambiguous-note"))
    );
    for (prefix, problem) in [
        ("(Reminder", ItemSourceProblem::UnsupportedSourceControl),
        ("{ Advanced", ItemSourceProblem::UnsupportedSourceControl),
    ] {
        let mut a = fixture();
        let mut lines = a.items.input().clone();
        let rule = metadata("reviewed-control", prefix);
        lines.rules.push(rule);
        a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
        let mut input = a.item_source.input().clone();
        input.rule_layouts.push(ItemRuleSourceLayout {
            rule: key("reviewed-control"),
            role: ItemRuleSourceRole::Header,
        });
        metadata_ids(&mut input).push(key("reviewed-control"));
        rebuild(&mut a, input);
        let plan = attribute(
            &a,
            &xml(
                &format!("{prefix} body\n"),
                "128% increased Spell Damage",
                "",
            ),
        );
        match &plan.report().layout {
            ItemLayoutStatus::Pending(problems) | ItemLayoutStatus::Unsupported(problems) => {
                assert!(problems.contains(&problem), "{:?}", plan.report())
            }
            ItemLayoutStatus::Proven => panic!("source lifecycle was admitted"),
        }
    }
}

fn with_property(mut a: Artifacts) -> Artifacts {
    let modifier = a.modifiers["spell"].definition.clone();
    let slot = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let mut schema = a.schema.input().clone();
    schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: slot.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ModifierRoll],
            }),
        }));
    for row in &mut schema.definitions {
        if let DefinitionDescriptor::Modifier(row) = row
            && row.id == modifier
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.declarations.parameters.members.push(slot.clone());
        }
    }
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut lines = a.items.input().clone();
    lines.definitions = a.schema.identity().clone();
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("spell"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!("spell modifier")
    };
    rolls.push(ItemRollTemplate {
        slot,
        value: ItemLineValue::Property {
            property: key("marked"),
        },
    });
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.property_bindings = vec![ItemSourcePropertyBinding {
        label: "cold".into(),
        property: key("marked"),
    }];
    rebuild(&mut a, input);
    a
}

#[test]
fn v5_keeps_v4_flag_facts_and_invalid_range_strictness_even_with_empty_flag_map() {
    for v in [4, 5] {
        let mut a = with_property(fixture());
        version(&mut a, v);
        let input = xml("", "{range:NaN}{tags:cold}128% increased Spell Damage", "");
        let plan = attribute(&a, &input);
        let line = plan.report().lines.last().unwrap();
        assert!(line.blockers.contains(&ItemSourceProblem::InvalidRange));
        assert_eq!(line.property_tokens.len(), 1);
        assert!(
            line.properties.is_empty(),
            "v{v} must not synthesize facts on blocked members"
        );
        assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
        let mut input = a.item_source.input().clone();
        match &mut input.dialect {
            ItemSourceDialect::PobExportedSingleTextFlagsV1 { flag_bindings }
            | ItemSourceDialect::PobExportedSingleTextPreambleV1 { flag_bindings, .. } => {
                flag_bindings.push(ItemSourceFlagBinding {
                    label: ItemSourceLineFlag::Fractured,
                    property: key("marked"),
                })
            }
            _ => unreachable!(),
        }
        // One semantic property may have one reviewed source spelling in each channel.
        input.property_bindings.clear();
        rebuild(&mut a, input);
        for (prefix, expected) in [
            ("", false),
            ("{fractured}", true),
            ("{fractured}{fractured}", true),
        ] {
            let plan = attribute(
                &a,
                &xml("", &format!("{prefix}128% increased Spell Damage"), ""),
            );
            proven(&plan);
            let converted = plan.convert(&a.items).unwrap();
            assert_eq!(
                converted.modifiers[0].rolls[1].value,
                ParameterValue::Boolean(expected)
            );
        }
    }
    let mut a = with_property(fixture());
    version(&mut a, 3);
    let plan = attribute(
        &a,
        &xml("", "{range:NaN}{tags:cold}128% increased Spell Damage", ""),
    );
    assert_eq!(
        plan.report()
            .lines
            .last()
            .unwrap()
            .properties
            .get(&key("marked")),
        Some(&true),
        "legacy v3 InvalidRange behavior stays explicit"
    );
}

#[test]
fn metadata_aliases_block_only_their_defaults_and_unknown_scope_never_defaults() {
    let mut a = fixture();
    let slot = a
        .items
        .input()
        .rules
        .iter()
        .find_map(|rule| {
            rule.emissions.iter().find_map(|emission| match emission {
                ItemEmission::ItemParameter { slot, .. } => Some(slot.clone()),
                _ => None,
            })
        })
        .unwrap();
    let mut input = a.item_source.input().clone();
    input.template_defaults = vec![ItemSourceTemplateDefaults {
        template: a.staff.clone(),
        item_level: ItemSourceAbsentPolicy::Absent,
        quality: ItemSourceAbsentPolicy::Absent,
        parameters: vec![ItemSourceParameterDefault {
            assignment: ParameterAssignment {
                slot,
                value: ParameterValue::Integer(BoundedInteger::new(7).unwrap()),
            },
            headers: vec!["External Note".into(), "Alternate Note".into()],
        }],
    }];
    rebuild(&mut a, input);
    for (headers, expected) in [
        ("", 1),
        ("Unique ID: source-only\n", 1),
        ("External Note: opaque\n", 0),
    ] {
        let plan = attribute(&a, &xml(headers, "128% increased Spell Damage", ""));
        proven(&plan);
        let converted = plan.convert(&a.items).unwrap();
        assert_eq!(converted.defaults.parameters.len(), expected);
        // This tests explicit absence policy only, not quality normalization with a UID.
        assert!(converted.defaults.quality_absent && converted.defaults.item_level_absent);
    }
    for headers in [
        "Record Number: broken\n",
        "Alternate Note: unknown\n",
        "Missing Header: opaque\n",
        "Quality: broken\n",
    ] {
        let plan = attribute(&a, &xml(headers, "128% increased Spell Damage", ""));
        assert_eq!(
            plan.convert(&a.items).unwrap().defaults,
            ItemDefaultedInputs::default(),
            "{headers}"
        );
    }
    let plan = attribute(
        &a,
        &xml(
            "Unique ID: source-only\nQuality: 20\n",
            "128% increased Spell Damage",
            "",
        ),
    );
    assert!(!plan.convert(&a.items).unwrap().defaults.quality_absent);
}

#[test]
fn normalization_keeps_sidecar_v12_and_exact_source_policy_identity() {
    let a = fixture();
    let imported = source(&xml("Unique ID: abc\n", "128% increased Spell Damage", ""));
    let normalized = support::normalize(&imported, &a);
    assert_eq!(normalized.sidecar().schema_version, 12);
    assert_eq!(
        normalized.sidecar().item_source_policy,
        *a.item_source.identity()
    );
    assert_eq!(normalized.sidecar().item_policy, *a.items.identity());
    let replay = support::normalize(&imported, &a);
    assert_eq!(
        serde_json::to_vec(normalized.sidecar()).unwrap(),
        serde_json::to_vec(replay.sidecar()).unwrap()
    );
}

#[test]
fn metadata_prescans_reject_selection_controls_without_interpreting_inert_braces() {
    let a = fixture();
    for payload in [
        "{variant:1}{group:1}x",
        "prefix {version:2} suffix",
        "prefix {group:1} suffix",
        "Foil Unique",
        "[hidden] metadata",
        "<hidden> metadata",
    ] {
        let encoded = payload.replace('<', "&lt;").replace('>', "&gt;");
        let plan = attribute(
            &a,
            &xml(
                &format!("Unique ID: {encoded}\n"),
                "128% increased Spell Damage",
                "",
            ),
        );
        assert!(
            matches!(&plan.report().layout, ItemLayoutStatus::Unsupported(problems) if problems.contains(&ItemSourceProblem::UnsupportedSourceControl)),
            "{payload}: {:?}",
            plan.report()
        );
        assert!(!plan.can_convert_lines());
        let line = plan
            .report()
            .lines
            .iter()
            .find(|line| line.raw.starts_with("Unique ID:"))
            .unwrap();
        assert_eq!(line.raw, format!("Unique ID: {payload}"));
        assert!(line.member.is_none());
        assert!(
            line.blockers
                .contains(&ItemSourceProblem::UnsupportedSourceControl)
        );
    }
    for payload in [
        "{range:NaN}{tags:cold}{rune}{unrecognized}text",
        "{variant:1",
        "{version:2",
        "{group:1",
    ] {
        let plan = attribute(
            &a,
            &xml(
                &format!("Unique ID: {payload}\n"),
                "128% increased Spell Damage",
                "",
            ),
        );
        proven(&plan);
        assert!(plan.report().writes.is_empty());
        let line = plan
            .report()
            .lines
            .iter()
            .find(|line| line.raw.starts_with("Unique ID:"))
            .unwrap();
        assert!(line.member.is_none() && line.properties.is_empty());
        assert!(line.property_tokens.is_empty() && line.flag_tokens.is_empty());
    }
    for v in [3, 4] {
        let mut a = fixture();
        version(&mut a, v);
        let plan = attribute(
            &a,
            &xml(
                "Unique ID: {variant:1}\n",
                "128% increased Spell Damage",
                "",
            ),
        );
        assert!(
            matches!(&plan.report().layout, ItemLayoutStatus::Pending(problems) if problems.contains(&ItemSourceProblem::UnknownHeader))
        );
        assert!(
            plan.can_convert_lines(),
            "legacy lookup and source control semantics stay unchanged"
        );
    }
    let plan = attribute(
        &a,
        &xml("Rune: Active\n", "128% increased Spell Damage", ""),
    );
    assert!(
        matches!(&plan.report().layout, ItemLayoutStatus::Pending(problems) if problems.contains(&ItemSourceProblem::RuneLifecycle))
    );
}

// Find a passing resource boundary through the public API so these tests do not
// duplicate the compiler's internal comparison-count formula.
fn minimum_budget(maximum: usize, mut accepts: impl FnMut(usize) -> bool) -> usize {
    assert!(accepts(maximum));
    let (mut lower, mut upper) = (1, maximum);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        if accepts(middle) {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    lower
}

#[test]
fn metadata_references_consume_text_budget_in_constructor_and_tighter_encoder() {
    let a = fixture();
    let mut empty = a.item_source.input().clone();
    metadata_ids(&mut empty).clear();
    let empty_policy =
        ItemSourceLayoutPolicy::new(empty.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    let text_budget = minimum_budget(
        ItemSourceLimits::default().max_policy_text_bytes,
        |budget| {
            let limits = ItemSourceLimits {
                max_policy_text_bytes: budget,
                ..Default::default()
            };
            ItemSourceLayoutPolicy::new(empty.clone(), &a.items, &a.schema, limits).is_ok()
                && encode_item_source_policy(&empty_policy, limits).is_ok()
        },
    );
    let limits = ItemSourceLimits {
        max_policy_text_bytes: text_budget,
        ..Default::default()
    };
    assert!(matches!(
        ItemSourceLayoutPolicy::new(a.item_source.input().clone(), &a.items, &a.schema, limits),
        Err(ItemSourceError::Limit("policy text"))
    ));
    // Compiling under the default limit does not grant permission to encode later
    // under a smaller caller limit which omits the metadata reference strings.
    assert!(matches!(
        encode_item_source_policy(&a.item_source, limits),
        Err(ItemSourceError::Limit("policy text"))
    ));
    for v in [3, 4, 5] {
        let mut old = fixture();
        version(&mut old, v);
        assert!(
            ItemSourceLayoutPolicy::new(
                old.item_source.input().clone(),
                &old.items,
                &old.schema,
                limits
            )
            .is_ok(),
            "empty metadata v{v}"
        );
        assert!(
            encode_item_source_policy(&old.item_source, limits).is_ok(),
            "empty metadata v{v}"
        );
    }
}

#[test]
fn metadata_and_default_schema_work_share_one_constructor_and_encoder_budget() {
    let a = fixture();
    let mut defaults_only = a.item_source.input().clone();
    metadata_ids(&mut defaults_only).clear();
    defaults_only.template_defaults = vec![ItemSourceTemplateDefaults {
        template: a.staff.clone(),
        parameters: vec![],
        item_level: ItemSourceAbsentPolicy::Absent,
        quality: ItemSourceAbsentPolicy::Absent,
    }];
    let defaults_policy = ItemSourceLayoutPolicy::new(
        defaults_only.clone(),
        &a.items,
        &a.schema,
        Default::default(),
    )
    .unwrap();
    let schema_budget = |input: &ItemSourceLayoutPolicyInput, policy: &ItemSourceLayoutPolicy| {
        minimum_budget(ItemSourceLimits::default().max_schema_work, |budget| {
            let limits = ItemSourceLimits {
                max_schema_work: budget,
                ..Default::default()
            };
            ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).is_ok()
                && encode_item_source_policy(policy, limits).is_ok()
        })
    };
    let metadata_work = schema_budget(a.item_source.input(), &a.item_source);
    let defaults_work = schema_budget(&defaults_only, &defaults_policy);
    assert!(metadata_work > 1 && defaults_work > 1);
    let limits = ItemSourceLimits {
        max_schema_work: metadata_work.max(defaults_work),
        ..Default::default()
    };
    assert!(encode_item_source_policy(&a.item_source, limits).is_ok());
    assert!(encode_item_source_policy(&defaults_policy, limits).is_ok());
    let mut combined = a.item_source.input().clone();
    combined.template_defaults = defaults_only.template_defaults;
    let combined_policy =
        ItemSourceLayoutPolicy::new(combined.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    assert!(matches!(
        ItemSourceLayoutPolicy::new(combined, &a.items, &a.schema, limits),
        Err(ItemSourceError::Limit("schema work"))
    ));
    assert!(
        matches!(
            encode_item_source_policy(&combined_policy, limits),
            Err(ItemSourceError::Limit("schema work"))
        ),
        "cached metadata and default work must both survive precompilation"
    );
    let bytes = encode_item_source_policy(&combined_policy, Default::default()).unwrap();
    assert!(matches!(
        decode_item_source_policy(&bytes, &a.items, &a.schema, limits),
        Err(ItemSourceError::Limit("schema work"))
    ));
    let tiny = ItemSourceLimits {
        max_schema_work: 1,
        ..Default::default()
    };
    assert!(matches!(
        ItemSourceLayoutPolicy::new(a.item_source.input().clone(), &a.items, &a.schema, tiny),
        Err(ItemSourceError::Limit("schema work"))
    ));
    assert!(matches!(
        encode_item_source_policy(&a.item_source, tiny),
        Err(ItemSourceError::Limit("schema work"))
    ));
    for v in [3, 4, 5] {
        let mut empty = fixture();
        version(&mut empty, v);
        assert!(
            ItemSourceLayoutPolicy::new(
                empty.item_source.input().clone(),
                &empty.items,
                &empty.schema,
                tiny
            )
            .is_ok(),
            "empty metadata/defaults v{v}"
        );
        assert!(
            encode_item_source_policy(&empty.item_source, tiny).is_ok(),
            "empty metadata/defaults v{v}"
        );
    }
}

#[test]
fn metadata_binding_scales_with_catalog_breadth_and_preserves_wire_and_attribution() {
    let mut a = fixture();
    let mut lines = a.items.input().clone();
    let mut input = a.item_source.input().clone();
    // Metadata membership is sparse in a large catalog. Many unrelated known
    // rules must not multiply each reference into a full catalog scan.
    for index in 0..1800 {
        let id = format!("catalog-metadata-{index:04}");
        let rule = metadata(&id, &format!("Catalog Metadata {index:04}: "));
        input.rule_layouts.push(ItemRuleSourceLayout {
            rule: rule.id.clone(),
            role: ItemRuleSourceRole::Header,
        });
        lines.rules.push(rule);
    }
    // Deliberately unordered declarations exercise lookup independently of the
    // authored input order, including both ends of the catalog's sorted index.
    for index in [
        1799, 0, 900, 1, 1798, 50, 1700, 1500, 3, 42, 888, 99, 100, 1000, 1600, 1234,
    ] {
        metadata_ids(&mut input).push(key(&format!("catalog-metadata-{index:04}")));
    }
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    input.item_lines = *a.items.identity();
    let limits = ItemSourceLimits {
        max_schema_work: 50_000,
        ..Default::default()
    };
    // This bound includes allocating the indexes and every metadata lookup.
    // The previous catalog-linear charge exceeds even the default one-million
    // budget for these nineteen references.
    let bounded = ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).unwrap();
    let default =
        ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    let expected = serde_json::to_vec(&input).unwrap();
    let bytes = encode_item_source_policy(&bounded, limits).unwrap();
    assert_eq!(bytes, expected);
    assert_eq!(
        bytes,
        encode_item_source_policy(&default, Default::default()).unwrap()
    );
    assert_eq!(bounded.identity(), default.identity());
    assert_eq!(
        *bounded.identity(),
        digest_owned("owned-item-source-policy-v5", &input, 4 * 1024 * 1024).unwrap()
    );
    let decoded = decode_item_source_policy(&bytes, &a.items, &a.schema, limits).unwrap();
    assert_eq!(decoded.identity(), bounded.identity());
    assert_eq!(encode_item_source_policy(&decoded, limits).unwrap(), bytes);

    let raw = xml(
        "Catalog Metadata 1799: literal {tags:cold}\nCatalog Metadata 0000: first\nUnique ID: original\n",
        "128% increased Spell Damage",
        "",
    );
    a.item_source = bounded;
    let bounded_plan = attribute(&a, &raw);
    proven(&bounded_plan);
    assert_eq!(bounded_plan.convert(&a.items).unwrap().modifiers.len(), 1);
    assert!(
        bounded_plan
            .report()
            .lines
            .iter()
            .filter(|line| line.raw.starts_with("Catalog Metadata"))
            .all(|line| line.member.is_none()
                && line.properties.is_empty()
                && line.blockers.is_empty())
    );
    a.item_source = default;
    let default_plan = attribute(&a, &raw);
    assert_eq!(
        serde_json::to_vec(bounded_plan.report()).unwrap(),
        serde_json::to_vec(default_plan.report()).unwrap()
    );

    // Resource accounting remains enforceable after precompilation and across
    // decoding; find the public boundary rather than copy its implementation.
    let minimum = minimum_budget(limits.max_schema_work, |budget| {
        ItemSourceLayoutPolicy::new(
            input.clone(),
            &a.items,
            &a.schema,
            ItemSourceLimits {
                max_schema_work: budget,
                ..Default::default()
            },
        )
        .is_ok()
    });
    assert!(minimum > 1);
    let exact = ItemSourceLimits {
        max_schema_work: minimum,
        ..Default::default()
    };
    let tight = ItemSourceLimits {
        max_schema_work: minimum - 1,
        ..Default::default()
    };
    assert!(encode_item_source_policy(&a.item_source, exact).is_ok());
    assert!(decode_item_source_policy(&bytes, &a.items, &a.schema, exact).is_ok());
    assert!(matches!(
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, tight),
        Err(ItemSourceError::Limit("schema work"))
    ));
    assert!(matches!(
        encode_item_source_policy(&a.item_source, tight),
        Err(ItemSourceError::Limit("schema work"))
    ));
    assert!(matches!(
        decode_item_source_policy(&bytes, &a.items, &a.schema, tight),
        Err(ItemSourceError::Limit("schema work"))
    ));
}
