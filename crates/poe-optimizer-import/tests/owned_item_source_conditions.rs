//! Injected source-role prerequisites use arbitrary fixture grammar, not game names.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::owned_value::{BooleanToken, RationalScale, ValueCodecKind};
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

fn conditions(input: &mut ItemSourceLayoutPolicyInput) -> &mut Vec<ItemSourceConditionalMember> {
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 {
        single_modifier_conditions,
        ..
    } = &mut input.dialect
    else {
        panic!("condition dialect")
    };
    single_modifier_conditions
}
fn fixture() -> Artifacts {
    let mut a = artifacts();
    let mut lines = a.items.input().clone();
    let recipe = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("spell"))
        .unwrap();
    recipe.id = key("coefficient");
    recipe.pattern = vec![
        ItemPatternPart::Literal("Coefficient ".into()),
        ItemPatternPart::Capture(key("value")),
        ItemPatternPart::Literal(" widgets".into()),
    ];
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.schema_version = OWNED_ITEM_SOURCE_CONDITION_POLICY_VERSION;
    input.item_lines = *a.items.identity();
    let role = input
        .rule_layouts
        .iter_mut()
        .find(|r| r.rule == key("spell"))
        .unwrap();
    role.rule = key("coefficient");
    role.role = ItemRuleSourceRole::Unresolved;
    input.dialect = ItemSourceDialect::PobExportedSingleTextConditionsV1 {
        flag_bindings: vec![],
        metadata_rules: vec![],
        single_modifier_conditions: vec![ItemSourceConditionalMember {
            rule: key("coefficient"),
            all: vec![
                ItemSourceCondition::NoSourceTags,
                ItemSourceCondition::NoGeneratedBuffMembers,
                ItemSourceCondition::UnsignedIntegerCapture {
                    capture: key("value"),
                    min: 0,
                    max: 128,
                },
            ],
        }],
    };
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
    a
}
fn rebuild(a: &mut Artifacts, input: ItemSourceLayoutPolicyInput) {
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
}
fn attribute(
    a: &Artifacts,
    body: &str,
    overlays: &str,
) -> std::result::Result<ItemRangeAttribution, ItemSourceError> {
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nQuality: 20\nRune: None\nImplicits: 0\n{body}{overlays}</Item></Items></PathOfBuilding2>"
    );
    let imported = source(&xml);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
}
fn first_and_follower(a: &Artifacts, first: &str) -> ItemRangeAttribution {
    attribute(
        a,
        &format!("{first}\n49% increased Attack Speed\n+20% to Critical Damage Bonus"),
        "",
    )
    .unwrap()
}

#[test]
fn injected_all_of_guards_admit_exact_unsigned_boundaries_and_preserve_source_indices() {
    let a = fixture();
    for token in ["0", "1", "128", "000128"] {
        let first = format!("Coefficient {token} widgets");
        let plan = attribute(
            &a,
            &format!("{first}\n49% increased Attack Speed"),
            "<ModRange id=\"1\" range=\"0.5\"/>",
        )
        .unwrap();
        assert!(
            matches!(plan.report().layout, ItemLayoutStatus::Proven),
            "{token}"
        );
        let line = plan.report().lines.iter().find(|r| r.raw == first).unwrap();
        assert!(line.member.is_some());
        assert!(line.blockers.is_empty());
        assert_eq!(
            plan.report().writes.last().unwrap().target,
            ItemRangeTarget::Line(line.index)
        );
        let converted = plan.convert(&a.items).unwrap();
        assert_eq!(converted.modifiers.len(), 2);
        assert_eq!(
            converted.modifiers[0].definition,
            a.modifiers["spell"].definition
        );
        assert_eq!(
            converted.modifiers[1].definition,
            a.modifiers["attack-speed"].definition
        );
    }
    // Data controls the inclusive interval; the implementation has no fixed bound.
    let mut narrowed = fixture();
    let mut input = narrowed.item_source.input().clone();
    conditions(&mut input)[0].all[2] = ItemSourceCondition::UnsignedIntegerCapture {
        capture: key("value"),
        min: 7,
        max: 7,
    };
    rebuild(&mut narrowed, input);
    for (token, admitted) in [("6", false), ("7", true), ("8", false)] {
        let plan = first_and_follower(&narrowed, &format!("Coefficient {token} widgets"));
        assert_eq!(
            plan.convert(&narrowed.items).unwrap().modifiers.len(),
            if admitted { 3 } else { 1 }
        );
    }
}

#[test]
fn decoded_value_or_numeric_prefix_does_not_replace_exact_spelling_and_all_guards() {
    let a = fixture();
    for token in [
        "+1",
        "-0",
        "-1",
        "1.0",
        "1.",
        ".5",
        "1e1",
        "129",
        "10001",
        "1 widgets 2",
        "1 ",
        " 1",
    ] {
        let text = format!("Coefficient {token} widgets");
        let plan = first_and_follower(&a, &text);
        assert!(
            matches!(plan.report().layout, ItemLayoutStatus::Pending(_)),
            "{token}"
        );
        let invalid = plan.report().lines.iter().find(|r| r.raw == text).unwrap();
        assert!(invalid.member.is_none(), "{token}");
        let following = plan
            .report()
            .lines
            .iter()
            .find(|r| r.raw == "49% increased Attack Speed")
            .unwrap();
        assert!(
            following
                .blockers
                .contains(&ItemSourceProblem::PossibleCombinedLine),
            "{token}"
        );
        let converted = plan.convert(&a.items).unwrap();
        assert_eq!(converted.modifiers.len(), 1, "{token}");
        assert_eq!(
            converted.modifiers[0].definition,
            a.modifiers["critical-bonus"].definition
        );
    }
    // Semantic decoding may succeed, yet its decimal spelling still lacks the proof.
    assert!(matches!(
        a.items
            .convert_line(1, "Coefficient 1.0 widgets", None)
            .unwrap()
            .outcome,
        ItemLineOutcome::Known { .. }
    ));
}

#[test]
fn source_tags_cannot_be_stripped_before_guarding_or_bypass_guards_via_raw_match() {
    let a = fixture();
    for text in [
        "{range:0.5}Coefficient 12 widgets",
        "{enchant}Coefficient 12 widgets",
        "{implicit}Coefficient 12 widgets",
        "{tags:attack}Coefficient 12 widgets",
        "{variant:1}Coefficient 12 widgets",
        "{rune}Coefficient 12 widgets",
        "Coefficient 12 widgets{tags:attack}",
    ] {
        let plan = first_and_follower(&a, text);
        assert!(
            plan.report()
                .lines
                .iter()
                .find(|r| r.raw == text)
                .unwrap()
                .member
                .is_none(),
            "{text}"
        );
        let following = plan
            .report()
            .lines
            .iter()
            .find(|r| r.raw == "49% increased Attack Speed")
            .unwrap();
        assert!(
            following
                .blockers
                .contains(&ItemSourceProblem::PossibleCombinedLine),
            "{text}"
        );
        assert_eq!(plan.convert(&a.items).unwrap().modifiers.len(), 1, "{text}");
    }
    let mut raw_literal = fixture();
    let mut line_input = raw_literal.items.input().clone();
    let rule = line_input
        .rules
        .iter_mut()
        .find(|r| r.id == key("coefficient"))
        .unwrap();
    rule.pattern[0] = ItemPatternPart::Literal("{enchant}Coefficient ".into());
    raw_literal.items =
        OwnedItemLinePolicy::new(line_input, &raw_literal.schema, Default::default()).unwrap();
    let mut input = raw_literal.item_source.input().clone();
    input.item_lines = *raw_literal.items.identity();
    rebuild(&mut raw_literal, input);
    let plan = first_and_follower(&raw_literal, "{enchant}Coefficient 12 widgets");
    assert!(
        plan.report()
            .lines
            .iter()
            .find(|r| r.raw == "49% increased Attack Speed")
            .unwrap()
            .blockers
            .contains(&ItemSourceProblem::PossibleCombinedLine)
    );
    assert_eq!(plan.convert(&raw_literal.items).unwrap().modifiers.len(), 1);
}

#[test]
fn generated_or_unknown_template_prefixes_cannot_gain_a_conditional_member() {
    let mut a = fixture();
    let mut input = a.item_source.input().clone();
    input
        .template_layouts
        .iter_mut()
        .find(|t| t.template == a.staff)
        .unwrap()
        .load_index_prefix = ItemLoadIndexPrefix::Unresolved;
    rebuild(&mut a, input);
    let plan = first_and_follower(&a, "Coefficient 12 widgets");
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
    assert!(
        plan.report()
            .lines
            .iter()
            .find(|r| r.raw == "Coefficient 12 widgets")
            .unwrap()
            .member
            .is_none()
    );
    assert!(
        plan.report()
            .lines
            .iter()
            .find(|r| r.raw == "49% increased Attack Speed")
            .unwrap()
            .blockers
            .contains(&ItemSourceProblem::PossibleCombinedLine)
    );
    assert!(
        plan.convert(&a.items)
            .unwrap()
            .modifiers
            .iter()
            .all(|m| m.definition != a.modifiers["spell"].definition)
    );
    let mut a = fixture();
    let mut input = a.item_source.input().clone();
    input.template_layouts.retain(|t| t.template != a.staff);
    rebuild(&mut a, input);
    let plan = first_and_follower(&a, "Coefficient 12 widgets");
    assert!(
        plan.report()
            .lines
            .iter()
            .find(|r| r.raw == "Coefficient 12 widgets")
            .unwrap()
            .member
            .is_none()
    );
    for body in [
        "Unknown Base\nCoefficient 12 widgets",
        "Ashen Staff\nCoefficient 12 widgets",
    ] {
        let plan = attribute(&fixture(), body, "").unwrap();

        assert!(
            plan.report()
                .lines
                .iter()
                .find(|r| r.raw == "Coefficient 12 widgets")
                .unwrap()
                .member
                .is_none(),
            "{body}"
        );
    }
}

#[test]
fn invalid_conditional_contracts_are_rejected_before_attribution() {
    let a = fixture();
    let original = a.item_source.input().clone();
    let mut invalid = vec![];
    let mut p = original.clone();
    conditions(&mut p)[0].rule = key("missing");
    invalid.push(p);
    let mut p = original.clone();
    let duplicate = conditions(&mut p)[0].clone();
    conditions(&mut p).push(duplicate);
    invalid.push(p);
    let mut p = original.clone();
    conditions(&mut p)[0].all.clear();
    invalid.push(p);
    for condition in [
        ItemSourceCondition::NoSourceTags,
        ItemSourceCondition::NoGeneratedBuffMembers,
        ItemSourceCondition::UnsignedIntegerCapture {
            capture: key("value"),
            min: 0,
            max: 1,
        },
    ] {
        let mut p = original.clone();
        conditions(&mut p)[0].all.push(condition);
        invalid.push(p);
    }
    let mut p = original.clone();
    conditions(&mut p)[0].all[2] = ItemSourceCondition::UnsignedIntegerCapture {
        capture: key("missing"),
        min: 0,
        max: 1,
    };
    invalid.push(p);
    let mut p = original.clone();
    conditions(&mut p)[0].all[2] = ItemSourceCondition::UnsignedIntegerCapture {
        capture: key("value"),
        min: 2,
        max: 1,
    };
    invalid.push(p);
    for role in [
        ItemRuleSourceRole::Header,
        ItemRuleSourceRole::SingleModifier,
    ] {
        let mut p = original.clone();
        p.rule_layouts
            .iter_mut()
            .find(|r| r.rule == key("coefficient"))
            .unwrap()
            .role = role;
        invalid.push(p);
    }
    for version in [3, 4, 5, 7] {
        let mut p = original.clone();
        p.schema_version = version;
        invalid.push(p);
    }
    for input in invalid {
        assert!(
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_err()
        );
    }
    let mut lines = a.items.input().clone();
    lines.rules.push(ItemLineRule {
        id: key("opaque"),
        pattern: vec![
            ItemPatternPart::Literal("Opaque ".into()),
            ItemPatternPart::Capture(key("value")),
        ],
        captures: vec![ItemCapture {
            id: key("value"),
            codec: ItemCaptureCodec::OpaqueText,
        }],
        emissions: vec![ItemEmission::Metadata { role: key("note") }],
    });
    let lines = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = original.clone();
    input.item_lines = *lines.identity();
    input.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("opaque"),
        role: ItemRuleSourceRole::Unresolved,
    });
    conditions(&mut input)[0].rule = key("opaque");
    assert!(
        ItemSourceLayoutPolicy::new(input, &lines, &a.schema, Default::default()).is_err(),
        "opaque capture cannot supply numeric proof"
    );
    let mut input = original;
    input.item_lines = digest_owned("wrong-lines", &false, 1024).unwrap();
    assert!(ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_err());
}

#[test]
fn v6_roundtrips_with_its_own_domain_and_legacy_dialects_keep_exact_wire_domains() {
    let a = fixture();
    let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
    let decoded =
        decode_item_source_policy(&bytes, &a.items, &a.schema, Default::default()).unwrap();
    assert_eq!(decoded.identity(), a.item_source.identity());
    assert_eq!(
        encode_item_source_policy(&decoded, Default::default()).unwrap(),
        bytes
    );
    assert_eq!(
        a.item_source.identity(),
        &digest_owned(
            "owned-item-source-policy-v6",
            a.item_source.input(),
            ItemSourceLimits::default().max_wire_bytes
        )
        .unwrap()
    );
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(
        text.contains("single_modifier_conditions") && text.contains("unsigned_integer_capture")
    );
    assert!(
        decode_item_source_policy(
            text.replace("\"min\":0", "\"min\":0,\"surprise\":true")
                .as_bytes(),
            &a.items,
            &a.schema,
            Default::default()
        )
        .is_err()
    );
    for (version, dialect, domain) in [
        (
            3,
            ItemSourceDialect::PobExportedSingleTextV1,
            "owned-item-source-policy-v3",
        ),
        (
            4,
            ItemSourceDialect::PobExportedSingleTextFlagsV1 {
                flag_bindings: vec![],
            },
            "owned-item-source-policy-v4",
        ),
        (
            5,
            ItemSourceDialect::PobExportedSingleTextPreambleV1 {
                flag_bindings: vec![],
                metadata_rules: vec![],
            },
            "owned-item-source-policy-v5",
        ),
    ] {
        let mut input = a.item_source.input().clone();
        input.schema_version = version;
        input.dialect = dialect;
        let old =
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
        let old_bytes = encode_item_source_policy(&old, Default::default()).unwrap();
        assert!(
            !String::from_utf8(old_bytes.clone())
                .unwrap()
                .contains("single_modifier_conditions")
        );
        assert_eq!(
            old.identity(),
            &digest_owned(
                domain,
                old.input(),
                ItemSourceLimits::default().max_wire_bytes
            )
            .unwrap()
        );
        let decoded =
            decode_item_source_policy(&old_bytes, &a.items, &a.schema, Default::default()).unwrap();
        assert_eq!(
            encode_item_source_policy(&decoded, Default::default()).unwrap(),
            old_bytes
        );
    }
}

fn minimum(mut high: usize, mut passes: impl FnMut(usize) -> bool) -> usize {
    assert!(passes(high));
    let mut low = 1;
    while low < high {
        let mid = low + (high - low) / 2;
        if passes(mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low
}
#[test]
fn conditional_text_and_schema_costs_survive_compilation_and_tighter_encoding() {
    let a = fixture();
    for text_budget in [false, true] {
        let mut empty = a.item_source.input().clone();
        conditions(&mut empty).clear();
        let empty_policy =
            ItemSourceLayoutPolicy::new(empty.clone(), &a.items, &a.schema, Default::default())
                .unwrap();
        let limits = |budget| {
            if text_budget {
                ItemSourceLimits {
                    max_policy_text_bytes: budget,
                    ..Default::default()
                }
            } else {
                ItemSourceLimits {
                    max_schema_work: budget,
                    ..Default::default()
                }
            }
        };
        let maximum = if text_budget {
            ItemSourceLimits::default().max_policy_text_bytes
        } else {
            ItemSourceLimits::default().max_schema_work
        };
        let base = minimum(maximum, |budget| {
            ItemSourceLayoutPolicy::new(empty.clone(), &a.items, &a.schema, limits(budget)).is_ok()
                && encode_item_source_policy(&empty_policy, limits(budget)).is_ok()
        });
        assert!(
            ItemSourceLayoutPolicy::new(
                a.item_source.input().clone(),
                &a.items,
                &a.schema,
                limits(base)
            )
            .is_err()
        );
        assert!(encode_item_source_policy(&a.item_source, limits(base)).is_err());
        let cost = minimum(maximum, |budget| {
            ItemSourceLayoutPolicy::new(
                a.item_source.input().clone(),
                &a.items,
                &a.schema,
                limits(budget),
            )
            .is_ok()
                && encode_item_source_policy(&a.item_source, limits(budget)).is_ok()
        });
        assert!(cost > base);
        assert!(
            decode_item_source_policy(
                &encode_item_source_policy(&a.item_source, Default::default()).unwrap(),
                &a.items,
                &a.schema,
                limits(base)
            )
            .is_err()
        );
    }
    let mut defaults_only = a.item_source.input().clone();
    conditions(&mut defaults_only).clear();
    defaults_only.template_defaults = vec![ItemSourceTemplateDefaults {
        template: a.staff.clone(),
        parameters: vec![],
        item_level: ItemSourceAbsentPolicy::Absent,
        quality: ItemSourceAbsentPolicy::Absent,
    }];
    let cost = |input: &ItemSourceLayoutPolicyInput| {
        minimum(ItemSourceLimits::default().max_schema_work, |budget| {
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
        })
    };
    let mut metadata_only = a.item_source.input().clone();
    conditions(&mut metadata_only).clear();
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 { metadata_rules, .. } =
        &mut metadata_only.dialect
    else {
        unreachable!()
    };
    metadata_rules.push(key("rune"));
    let separate = cost(&defaults_only)
        .max(cost(a.item_source.input()))
        .max(cost(&metadata_only));
    let mut combined = a.item_source.input().clone();
    combined.template_defaults = defaults_only.template_defaults;
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 { metadata_rules, .. } =
        &mut combined.dialect
    else {
        unreachable!()
    };
    metadata_rules.push(key("rune"));
    let checked =
        ItemSourceLayoutPolicy::new(combined.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    let limits = ItemSourceLimits {
        max_schema_work: separate,
        ..Default::default()
    };
    assert!(ItemSourceLayoutPolicy::new(combined, &a.items, &a.schema, limits).is_err());
    assert!(encode_item_source_policy(&checked, limits).is_err());
}

#[test]
fn failed_stripped_conditions_cannot_release_a_follower_through_a_different_raw_role() {
    let mut a = fixture();
    let mut lines = a.items.input().clone();
    let mut raw = lines
        .rules
        .iter()
        .find(|r| r.id == key("coefficient"))
        .unwrap()
        .clone();
    raw.id = key("tagged-legacy");
    raw.pattern[0] = ItemPatternPart::Literal("{enchant}Coefficient ".into());
    lines.rules.push(raw);
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.item_lines = *a.items.identity();
    input.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("tagged-legacy"),
        role: ItemRuleSourceRole::SingleModifier,
    });
    rebuild(&mut a, input);
    let plan = first_and_follower(&a, "{enchant}Coefficient 12 widgets");
    let line = plan
        .report()
        .lines
        .iter()
        .find(|r| r.raw == "{enchant}Coefficient 12 widgets")
        .unwrap();
    assert!(line.member.is_none());
    assert!(
        line.blockers
            .contains(&ItemSourceProblem::UnprovedMemberConditions)
    );
    let follower = plan
        .report()
        .lines
        .iter()
        .find(|r| r.raw == "49% increased Attack Speed")
        .unwrap();
    assert!(
        follower
            .blockers
            .contains(&ItemSourceProblem::PossibleCombinedLine)
    );
    let converted = plan.convert(&a.items).unwrap();
    assert_eq!(converted.modifiers.len(), 1);
    assert_eq!(
        converted.modifiers[0].definition,
        a.modifiers["critical-bonus"].definition
    );
}

#[test]
fn a_condition_cannot_upgrade_metadata_or_empty_emissions_into_a_source_member() {
    for emissions in [
        vec![],
        vec![ItemEmission::Metadata {
            role: key("annotation"),
        }],
    ] {
        let a = fixture();
        let mut lines = a.items.input().clone();
        lines
            .rules
            .iter_mut()
            .find(|r| r.id == key("coefficient"))
            .unwrap()
            .emissions = emissions;
        let lines = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
        let mut input = a.item_source.input().clone();
        input.item_lines = *lines.identity();
        assert!(ItemSourceLayoutPolicy::new(input, &lines, &a.schema, Default::default()).is_err());
    }
}

#[test]
fn a_known_template_only_proves_prefix_absence_in_its_single_structural_position() {
    let a = fixture();
    for preamble in [
        "Rarity: RARE\nNew Item\nQuality: 20\nAshen Staff",
        "Rarity: RARE\nNew Item\nUnknown Base\nAshen Staff",
        "Rarity: RARE\nNew Item\nAshen Staff\nAshen Staff",
    ] {
        let xml = format!(
            "<PathOfBuilding2><Items><Item id=\"7\">{preamble}\nImplicits: 0\nCoefficient 12 widgets\n49% increased Attack Speed</Item></Items></PathOfBuilding2>"
        );
        let imported = source(&xml);
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let plan = a
            .item_source
            .attribute(&evidence, item_source(&imported, "7"), &a.items)
            .unwrap();
        let line = plan
            .report()
            .lines
            .iter()
            .find(|r| r.raw == "Coefficient 12 widgets")
            .unwrap();
        assert!(line.member.is_none(), "{preamble}");
        assert!(
            line.blockers
                .contains(&ItemSourceProblem::UnprovedMemberConditions),
            "{preamble}"
        );
        assert!(
            plan.convert(&a.items).unwrap().modifiers.is_empty(),
            "{preamble}"
        );
    }
}

#[test]
fn runtime_guards_spend_bounded_work_and_output_without_changing_policy_or_bindings() {
    let a = fixture();
    let raw = format!(
        "{}Coefficient {}128 widgets{}",
        " ".repeat(64),
        "0".repeat(1024),
        " ".repeat(64)
    );
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\n{raw}\n49% increased Attack Speed</Item></Items></PathOfBuilding2>"
    );
    let imported = source(&xml);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let origin = item_source(&imported, "7");
    let input = a.item_source.input().clone();
    let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
    let generous = a
        .item_source
        .attribute(&evidence, origin, &a.items)
        .unwrap();
    assert!(matches!(generous.report().layout, ItemLayoutStatus::Proven));
    assert_eq!(generous.convert(&a.items).unwrap().modifiers.len(), 2);
    let mut unconditional = input.clone();
    conditions(&mut unconditional).clear();
    unconditional
        .rule_layouts
        .iter_mut()
        .find(|r| r.rule == key("coefficient"))
        .unwrap()
        .role = ItemRuleSourceRole::SingleModifier;
    let probe = |input: &ItemSourceLayoutPolicyInput, limits| {
        ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits)
            .unwrap()
            .attribute(&evidence, origin, &a.items)
    };
    let baseline = probe(&unconditional, Default::default()).unwrap();
    assert!(matches!(baseline.report().layout, ItemLayoutStatus::Proven));
    assert_eq!(baseline.convert(&a.items).unwrap().modifiers.len(), 2);
    let run = |input: &ItemSourceLayoutPolicyInput, limits| {
        let plan = probe(input, limits)?;
        plan.convert(&a.items).map(|_| ())
    };
    for output_budget in [false, true] {
        let limits = |budget| {
            if output_budget {
                ItemSourceLimits {
                    max_output_records: budget,
                    ..Default::default()
                }
            } else {
                ItemSourceLimits {
                    max_work: budget,
                    ..Default::default()
                }
            }
        };
        let maximum = if output_budget {
            ItemSourceLimits::default().max_output_records
        } else {
            ItemSourceLimits::default().max_work
        };
        // This fits the exact same source packet and ordinary probes, while leaving
        // no room for the extra guarded proof's capture map and bounded scans.
        let ordinary_cost = minimum(maximum, |budget| {
            run(&unconditional, limits(budget)).is_ok()
        });
        assert!(
            matches!(
                run(&input, limits(ordinary_cost)),
                Err(ItemSourceError::Limit(_))
                    | Err(ItemSourceError::Lines(ItemLineError::Limit(_)))
            ),
            "guard resource exhaustion must be an explicit error"
        );
        let guarded_cost = minimum(maximum, |budget| run(&input, limits(budget)).is_ok());
        assert!(guarded_cost > ordinary_cost);
        let accepted = probe(&input, limits(guarded_cost)).unwrap();
        assert!(matches!(accepted.report().layout, ItemLayoutStatus::Proven));
        assert_eq!(accepted.convert(&a.items).unwrap().modifiers.len(), 2);
        assert_eq!(
            encode_item_source_policy(&a.item_source, Default::default()).unwrap(),
            bytes
        );
    }
    let mut other = a.items.input().clone();
    other.version = key("stale-caller-line-policy");
    let other = OwnedItemLinePolicy::new(other, &a.schema, Default::default()).unwrap();
    assert!(matches!(
        a.item_source.attribute(&evidence, origin, &other),
        Err(ItemSourceError::Binding)
    ));
    assert!(matches!(
        generous.convert(&other),
        Err(ItemSourceError::Binding)
    ));
    assert_eq!(
        encode_item_source_policy(&a.item_source, Default::default()).unwrap(),
        bytes
    );
}

#[test]
fn v6_unconfigured_unresolved_rules_cannot_leak_property_free_values_while_v5_stays_exact() {
    let a = fixture();
    let mut input = a.item_source.input().clone();
    conditions(&mut input).clear();
    for version in [5, 6] {
        let mut a = fixture();
        let mut policy = input.clone();
        if version == 5 {
            policy.schema_version = OWNED_ITEM_SOURCE_PREAMBLE_POLICY_VERSION;
            policy.dialect = ItemSourceDialect::PobExportedSingleTextPreambleV1 {
                flag_bindings: vec![],
                metadata_rules: vec![],
            };
        }
        rebuild(&mut a, policy);
        // A valid member ends the preamble. Without it, the legacy UnknownHeader
        // blocker would mask the unresolved-role conversion behavior under test.
        let plan = attribute(&a, "50% increased Attack Speed\nCoefficient 12 widgets\n49% increased Attack Speed\n+20% to Critical Damage Bonus", "").unwrap();
        let row = plan
            .report()
            .lines
            .iter()
            .find(|r| r.raw == "Coefficient 12 widgets")
            .unwrap();
        assert!(row.member.is_none());
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
        let converted = plan.convert(&a.items).unwrap();
        if version == 6 {
            assert!(row.blockers.contains(&ItemSourceProblem::UnknownMember));
            assert_eq!(converted.modifiers.len(), 2);
            assert_eq!(
                converted.modifiers[0].definition,
                a.modifiers["attack-speed"].definition
            );
            assert_eq!(
                converted.modifiers[1].definition,
                a.modifiers["critical-bonus"].definition
            );
            assert!(
                !converted
                    .modifiers
                    .iter()
                    .any(|m| m.definition == a.modifiers["spell"].definition)
            );
        } else {
            assert!(!row.blockers.contains(&ItemSourceProblem::UnknownMember));
            assert!(
                converted
                    .modifiers
                    .iter()
                    .any(|m| m.definition == a.modifiers["spell"].definition),
                "legacy attribution behavior is not silently reinterpreted"
            );
        }
        let following = plan
            .report()
            .lines
            .iter()
            .find(|r| r.raw == "49% increased Attack Speed")
            .unwrap();
        assert!(
            following
                .blockers
                .contains(&ItemSourceProblem::PossibleCombinedLine)
        );
    }
}

fn decimal_fixture(sign: ItemSourceCaptureSign, min: f64, max: f64, scale: i64) -> Artifacts {
    let mut a = fixture();
    let mut lines = a.items.input().clone();
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("coefficient"))
        .unwrap();
    let ItemCaptureCodec::Value(codec) = &mut rule.captures[0].codec else {
        panic!("value codec")
    };
    let ValueCodecKind::Quantity { scale: factor, .. } = &mut codec.codec else {
        panic!("quantity")
    };
    *factor = RationalScale {
        numerator: BoundedInteger::new(scale).unwrap(),
        denominator: BoundedInteger::new(1).unwrap(),
    };
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut policy = a.item_source.input().clone();
    policy.item_lines = *a.items.identity();
    conditions(&mut policy)[0].all = vec![
        ItemSourceCondition::NoSourceScalingTags,
        ItemSourceCondition::NoGeneratedBuffMembers,
        ItemSourceCondition::DecimalCapture {
            capture: key("value"),
            sign,
            min,
            max,
        },
    ];
    rebuild(&mut a, policy);
    a
}

/// Add genuine typed Boolean consumers, so tag success means an admitted modifier,
/// not merely that the guard did not run or property handling masked its result.
fn scaling_fixture(initial: bool) -> Artifacts {
    let mut a = decimal_fixture(ItemSourceCaptureSign::Unsigned, 0.0, 128.0, 1);
    let modifier = a.modifiers["spell"].definition.clone();
    let mut schema = a.schema.input().clone();
    let mut lines = a.items.input().clone();
    let mut policy = a.item_source.input().clone();
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("coefficient"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!("modifier")
    };
    for property in ["family", "fracture", "desecrate"] {
        let slot = a
            .registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
            .unwrap();
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
        for definition in &mut schema.definitions {
            if let DefinitionDescriptor::Modifier(row) = definition
                && row.id == modifier
                && let SchemaState::Known(s) = &mut row.schema
            {
                s.declarations.parameters.members.push(slot.clone());
            }
        }
        rolls.push(ItemRollTemplate {
            slot,
            value: ItemLineValue::Property {
                property: key(property),
            },
        });
    }
    policy.property_bindings.push(ItemSourcePropertyBinding {
        label: "coefficient_family".into(),
        property: key("family"),
    });
    let ItemSourceDialect::PobExportedSingleTextConditionsV1 {
        flag_bindings,
        metadata_rules,
        ..
    } = &mut policy.dialect
    else {
        unreachable!()
    };
    flag_bindings.extend([
        ItemSourceFlagBinding {
            label: ItemSourceLineFlag::Fractured,
            property: key("fracture"),
        },
        ItemSourceFlagBinding {
            label: ItemSourceLineFlag::Desecrated,
            property: key("desecrate"),
        },
    ]);
    for (id, prefix) in [
        ("scaling-header", "Catalyst: "),
        ("scaling-quality", "CatalystQuality: "),
        ("scaling-alias", "Quality (Widgets Modifiers): "),
        (
            "embedded-scaling-alias",
            "Other Quality (Widgets Modifiers): ",
        ),
    ] {
        lines.rules.push(ItemLineRule {
            id: key(id),
            pattern: vec![
                ItemPatternPart::Literal(prefix.into()),
                ItemPatternPart::Capture(key("text")),
            ],
            captures: vec![ItemCapture {
                id: key("text"),
                codec: ItemCaptureCodec::OpaqueText,
            }],
            emissions: vec![ItemEmission::Metadata {
                role: key("annotation"),
            }],
        });
        policy.rule_layouts.push(ItemRuleSourceLayout {
            rule: key(id),
            role: ItemRuleSourceRole::Header,
        });
        metadata_rules.push(key(id));
    }
    if initial {
        conditions(&mut policy)[0].all[0] = ItemSourceCondition::InitialScalingIsOne;
    }
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    lines.definitions = a.schema.identity().clone();
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    policy.item_lines = *a.items.identity();
    rebuild(&mut a, policy);
    a
}

fn attribute_text(a: &Artifacts, text: &str) -> ItemRangeAttribution {
    let imported = source(&format!(
        "<PathOfBuilding2><Items><Item id=\"7\">{text}</Item></Items></PathOfBuilding2>"
    ));
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
        .unwrap()
}

#[test]
fn decimal_spelling_has_an_exact_sign_and_raw_inclusive_bounds_before_codec_scaling() {
    for (sign, min, max, scale, accepted, rejected) in [
        (
            ItemSourceCaptureSign::Unsigned,
            1.0,
            2.0,
            100,
            vec![
                ("1", 100.0),
                ("1.", 100.0),
                ("01.50", 150.0),
                ("2.00", 200.0),
            ],
            vec![
                "+1", "-1", "0.5", "2.001", ".5", "1e0", " 1", "1 ", "1..0", "１",
            ],
        ),
        (
            ItemSourceCaptureSign::Plus,
            1.0,
            2.0,
            100,
            vec![
                ("+1", 100.0),
                ("+1.", 100.0),
                ("+01.50", 150.0),
                ("+2.00", 200.0),
            ],
            vec!["1", "-1", "+0.5", "+2.001", "+.5", "+1e0", "++1", "+ 1"],
        ),
        (
            ItemSourceCaptureSign::Minus,
            -2.0,
            -1.0,
            -100,
            vec![
                ("-1", 100.0),
                ("-1.", 100.0),
                ("-01.50", 150.0),
                ("-2.00", 200.0),
            ],
            vec!["1", "+1", "-0.5", "-2.001", "-.5", "-1e0", "--1", "- 1"],
        ),
    ] {
        let a = decimal_fixture(sign, min, max, scale);
        for (token, expected) in accepted {
            let text = format!("Coefficient {token} widgets");
            let plan = attribute(&a, &text, "").unwrap();
            assert!(
                matches!(plan.report().layout, ItemLayoutStatus::Proven),
                "{token}: {:?}",
                plan.report()
            );
            let converted = plan.convert(&a.items).unwrap();
            assert_eq!(converted.modifiers.len(), 1, "{token}");
            assert!(
                matches!(&converted.modifiers[0].rolls[0].value, ParameterValue::Quantity(v) if v.value() == expected)
            );
        }
        for token in rejected {
            let text = format!("Coefficient {token} widgets");
            let plan = first_and_follower(&a, &text);
            let line = plan
                .report()
                .lines
                .iter()
                .find(|line| line.raw == text)
                .unwrap();
            assert!(line.member.is_none(), "{token}");
            assert!(
                line.blockers
                    .contains(&ItemSourceProblem::UnprovedMemberConditions),
                "{token}: {line:?}"
            );
            assert!(
                !plan
                    .convert(&a.items)
                    .unwrap()
                    .modifiers
                    .iter()
                    .any(|m| m.definition == a.modifiers["spell"].definition),
                "{token}"
            );
        }
    }
    let a = decimal_fixture(ItemSourceCaptureSign::Minus, 0.0, 0.0, -1);
    for (token, admitted) in [
        ("-0", true),
        ("-00.000", true),
        ("-0.", true),
        ("0", false),
        ("+0", false),
        ("-.0", false),
    ] {
        let plan = first_and_follower(&a, &format!("Coefficient {token} widgets"));
        assert_eq!(
            plan.convert(&a.items)
                .unwrap()
                .modifiers
                .iter()
                .any(|m| m.definition == a.modifiers["spell"].definition),
            admitted,
            "{token}"
        );
    }
}

#[test]
fn no_scaling_tags_preserves_safe_range_and_flags_but_rejects_nonempty_modifier_tags() {
    let a = scaling_fixture(false);
    for (tags, fracture, desecrate) in [
        ("", false, false),
        ("{tags}", false, false),
        ("{tags:123,- /}", false, false),
        ("{tags:}{range:0.5}{implicit}", false, false),
        ("{enchant}{range:1}", false, false),
        ("{fractured}", true, false),
        ("{desecrated}{fractured}{tags:}{range:0}", true, true),
    ] {
        let text = format!("{tags}Coefficient 12.5 widgets");
        // Empty source modTags bypass source catalyst selection even with this header.
        let plan = attribute(&a, &format!("Catalyst: Widgets\n{text}"), "").unwrap();
        assert!(
            matches!(plan.report().layout, ItemLayoutStatus::Proven),
            "{text}: {:?}",
            plan.report()
        );
        let converted = plan.convert(&a.items).unwrap();
        assert_eq!(converted.modifiers.len(), 1, "{text}");
        let rolls = &converted.modifiers[0].rolls;
        assert_eq!(rolls[1].value, ParameterValue::Boolean(false));
        assert_eq!(rolls[2].value, ParameterValue::Boolean(fracture));
        assert_eq!(rolls[3].value, ParameterValue::Boolean(desecrate));
    }
    for tags in [
        "{tags:coefficient_family}",
        "{tags:unknown}",
        "{tags:_}",
        "{tags:}{tags:coefficient_family}",
        "{tags:coefficient_family}{tags:}",
        "{rune}",
        "{corruptedRange:1}",
        "{unscalable}",
        "{mystery}",
        "{variant:1}",
        "{version:1}",
        "{group:1}",
        "{desecrated:yes}",
    ] {
        let text = format!("{tags}Coefficient 12 widgets");
        let plan = first_and_follower(&a, &text);
        let line = plan
            .report()
            .lines
            .iter()
            .find(|line| line.raw == text)
            .unwrap();
        assert!(line.member.is_none(), "{text}");
        assert!(
            line.blockers
                .contains(&ItemSourceProblem::UnprovedMemberConditions),
            "{text}: {line:?}"
        );
        assert!(
            !plan
                .convert(&a.items)
                .unwrap()
                .modifiers
                .iter()
                .any(|m| m.definition == a.modifiers["spell"].definition),
            "{text}"
        );
    }
}

#[test]
fn initial_scaling_uses_only_proven_fresh_source_prefix_and_bound_tag_properties() {
    let a = scaling_fixture(true);
    let text = "{tags:coefficient_family}{range:0.5}Coefficient 12.5 widgets";
    let plan = attribute(&a, text, "").unwrap();
    assert!(
        matches!(plan.report().layout, ItemLayoutStatus::Proven),
        "{:?}",
        plan.report()
    );
    let converted = plan.convert(&a.items).unwrap();
    assert_eq!(converted.modifiers.len(), 1);
    assert_eq!(
        converted.modifiers[0].rolls[1].value,
        ParameterValue::Boolean(true)
    );
    for preamble in [
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nCatalyst: Widgets",
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nQuality (Widgets Modifiers): 0%",
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nOther Quality (Widgets Modifiers): 20%",
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nUnknown Header: value",
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nQuality: broken",
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nCata[lyst]: Widgets",
        "Rarity: RARE\nNew Item\nQuality: 20\nAshen Staff\nImplicits: 0",
        "Rarity: RARE\nNew Item\nAshen Staff\nAshen Staff\nImplicits: 0",
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nImplicits: 0",
        "Rarity: RARE\nNew Item\nAshen Staff",
        "Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\n49% increased Attack Speed\nQuality: 20",
    ] {
        let plan = attribute_text(
            &a,
            &format!(
                "{preamble}\n{text}\n49% increased Attack Speed\n+20% to Critical Damage Bonus"
            ),
        );
        let line = plan
            .report()
            .lines
            .iter()
            .find(|line| line.raw == text)
            .unwrap();
        assert!(line.member.is_none(), "{preamble}");
        assert!(
            line.blockers
                .contains(&ItemSourceProblem::UnprovedMemberConditions),
            "{preamble}: {line:?}"
        );
        assert!(
            !plan
                .convert(&a.items)
                .unwrap()
                .modifiers
                .iter()
                .any(|m| m.definition == a.modifiers["spell"].definition),
            "{preamble}"
        );
    }
    for tags in [
        "{tags:unknown}",
        "{tags:coefficient_family}{corruptedRange:1}",
        "{tags:coefficient_family}{rune}",
        "{tags:coefficient_family}{variant:1}",
    ] {
        let raw = format!("{tags}Coefficient 12 widgets");
        let plan = first_and_follower(&a, &raw);
        assert!(
            plan.report()
                .lines
                .iter()
                .find(|line| line.raw == raw)
                .unwrap()
                .member
                .is_none(),
            "{raw}"
        );
        assert!(
            !plan
                .convert(&a.items)
                .unwrap()
                .modifiers
                .iter()
                .any(|m| m.definition == a.modifiers["spell"].definition)
        );
    }
}

#[test]
fn new_predicates_validate_finite_unique_numeric_contracts_and_roundtrip_under_v6() {
    let a = scaling_fixture(true);
    let original = a.item_source.input().clone();
    let mut invalid = Vec::new();
    for (min, max) in [
        (f64::NAN, 1.0),
        (0.0, f64::NAN),
        (f64::NEG_INFINITY, 1.0),
        (0.0, f64::INFINITY),
        (2.0, 1.0),
    ] {
        let mut input = original.clone();
        conditions(&mut input)[0].all[2] = ItemSourceCondition::DecimalCapture {
            capture: key("value"),
            sign: ItemSourceCaptureSign::Unsigned,
            min,
            max,
        };
        invalid.push(input);
    }
    for condition in [
        ItemSourceCondition::InitialScalingIsOne,
        ItemSourceCondition::DecimalCapture {
            capture: key("value"),
            sign: ItemSourceCaptureSign::Plus,
            min: 0.0,
            max: 2.0,
        },
        ItemSourceCondition::UnsignedIntegerCapture {
            capture: key("value"),
            min: 0,
            max: 2,
        },
    ] {
        let mut input = original.clone();
        conditions(&mut input)[0].all.push(condition);
        invalid.push(input);
    }
    let mut input = original.clone();
    conditions(&mut input)[0].all[0] = ItemSourceCondition::NoSourceScalingTags;
    conditions(&mut input)[0]
        .all
        .push(ItemSourceCondition::NoSourceScalingTags);
    invalid.push(input);
    let mut input = original.clone();
    conditions(&mut input)[0].all[2] = ItemSourceCondition::DecimalCapture {
        capture: key("absent"),
        sign: ItemSourceCaptureSign::Unsigned,
        min: 0.0,
        max: 2.0,
    };
    invalid.push(input);
    for input in invalid {
        assert!(
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_err()
        );
    }
    for initial in [false, true] {
        let a = scaling_fixture(initial);
        let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
        let decoded =
            decode_item_source_policy(&bytes, &a.items, &a.schema, Default::default()).unwrap();
        assert_eq!(decoded.identity(), a.item_source.identity());
        assert_eq!(
            encode_item_source_policy(&decoded, Default::default()).unwrap(),
            bytes
        );
        assert_eq!(
            a.item_source.identity(),
            &digest_owned(
                "owned-item-source-policy-v6",
                a.item_source.input(),
                ItemSourceLimits::default().max_wire_bytes
            )
            .unwrap()
        );
        let json = String::from_utf8(bytes).unwrap();
        assert!(json.contains(if initial {
            "initial_scaling_is_one"
        } else {
            "no_source_scaling_tags"
        }));
        assert!(json.contains("decimal_capture") && json.contains("unsigned"));
        for malformed in [
            json.replace("\"unsigned\"", "\"optional_plus\""),
            json.replace("\"min\":0.0", "\"min\":0.0,\"unknown\":true"),
        ] {
            assert!(
                decode_item_source_policy(
                    malformed.as_bytes(),
                    &a.items,
                    &a.schema,
                    Default::default()
                )
                .is_err()
            );
        }
    }
}

#[test]
fn decimal_guard_failure_blocks_raw_predecessor_fallback_and_later_source_range_indices() {
    let mut a = decimal_fixture(ItemSourceCaptureSign::Plus, 1.0, 128.0, 1);
    let mut lines = a.items.input().clone();
    let mut raw = lines
        .rules
        .iter()
        .find(|r| r.id == key("coefficient"))
        .unwrap()
        .clone();
    raw.id = key("decimal-predecessor");
    raw.pattern[0] = ItemPatternPart::Literal("{enchant}Coefficient ".into());
    lines.rules.push(raw);
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.item_lines = *a.items.identity();
    input.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("decimal-predecessor"),
        role: ItemRuleSourceRole::SingleModifier,
    });
    rebuild(&mut a, input);
    let text = "{enchant}Coefficient 12 widgets";
    let plan = attribute(
        &a,
        &format!("{text}\n49% increased Attack Speed\n+20% to Critical Damage Bonus"),
        "<ModRange id=\"1\" range=\"0.5\"/>",
    )
    .unwrap();
    let line = plan
        .report()
        .lines
        .iter()
        .find(|line| line.raw == text)
        .unwrap();
    assert!(
        line.blockers
            .contains(&ItemSourceProblem::UnprovedMemberConditions)
    );
    assert!(line.member.is_none());
    assert!(
        plan.report()
            .lines
            .iter()
            .find(|line| line.raw == "49% increased Attack Speed")
            .unwrap()
            .blockers
            .contains(&ItemSourceProblem::PossibleCombinedLine)
    );
    assert!(matches!(
        plan.report().writes.last().unwrap().target,
        ItemRangeTarget::Pending
    ));
    assert!(
        !plan
            .convert(&a.items)
            .unwrap()
            .modifiers
            .iter()
            .any(|m| m.definition == a.modifiers["spell"].definition)
    );
}

#[test]
fn guarded_range_endpoints_do_not_turn_invalid_or_unknown_source_ranges_into_values() {
    let mut a = scaling_fixture(true);
    let mut lines = a.items.input().clone();
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("coefficient"))
        .unwrap();
    let mut lower = rule.captures[0].clone();
    lower.id = key("lower");
    let mut upper = lower.clone();
    upper.id = key("upper");
    rule.captures = vec![lower, upper];
    rule.pattern = vec![
        ItemPatternPart::Literal("Coefficient (".into()),
        ItemPatternPart::Capture(key("lower")),
        ItemPatternPart::Literal("-".into()),
        ItemPatternPart::Capture(key("upper")),
        ItemPatternPart::Literal(") widgets".into()),
    ];
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::InterpolateOffset {
        lower: key("lower"),
        upper: key("upper"),
        quantum: ParameterValue::Quantity(FiniteQuantity::new(1.0, a.percent.clone()).unwrap()),
        rounding: ItemRangeRounding::SymmetricHalfOffset,
    };
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut policy = a.item_source.input().clone();
    policy.item_lines = *a.items.identity();
    conditions(&mut policy)[0].all.truncate(2);
    for capture in ["lower", "upper"] {
        conditions(&mut policy)[0]
            .all
            .push(ItemSourceCondition::DecimalCapture {
                capture: key(capture),
                sign: ItemSourceCaptureSign::Unsigned,
                min: 1.0,
                max: 128.0,
            });
    }
    rebuild(&mut a, policy);
    for (range, overlay, expected) in [
        ("{range:0.5}", "", Some(15.0)),
        ("{range:0}", "<ModRange id=\"1\" range=\"1\"/>", Some(20.0)),
        ("", "", None),
        ("{range:NaN}", "", None),
        ("{range:1.1}", "", None),
        ("{range:0.5}", "<ModRange id=\"1\" range=\"bad\"/>", None),
    ] {
        let text = format!("{{tags:coefficient_family}}{range}Coefficient (10-20) widgets");
        let plan = attribute(&a, &text, overlay).unwrap();
        let converted = plan.convert(&a.items).unwrap();
        match expected {
            Some(expected) => {
                assert!(
                    matches!(plan.report().layout, ItemLayoutStatus::Proven),
                    "{text}: {:?}",
                    plan.report()
                );
                assert_eq!(converted.modifiers.len(), 1);
                assert!(
                    matches!(&converted.modifiers[0].rolls[0].value, ParameterValue::Quantity(v) if v.value() == expected)
                );
            }
            None => assert!(converted.modifiers.is_empty(), "{text} {overlay}"),
        }
    }
    for raw in [
        "{range:0.5}Coefficient (0-20) widgets",
        "{range:0.5}Coefficient (10-129) widgets",
        "{range:0.5}Coefficient (.5-20) widgets",
        "{range:0.5Coefficient (10-20) widgets",
    ] {
        let plan = first_and_follower(&a, raw);
        assert!(
            plan.report()
                .lines
                .iter()
                .find(|line| line.raw == raw)
                .unwrap()
                .member
                .is_none()
        );
        assert!(
            !plan
                .convert(&a.items)
                .unwrap()
                .modifiers
                .iter()
                .any(|m| m.definition == a.modifiers["spell"].definition)
        );
    }
}

#[test]
fn decimal_conditions_require_numeric_codecs_even_when_modifier_emission_uses_a_literal() {
    let a = decimal_fixture(ItemSourceCaptureSign::Unsigned, 0.0, 128.0, 1);
    let mut lines = a.items.input().clone();
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("coefficient"))
        .unwrap();
    let ItemCaptureCodec::Value(codec) = &mut rule.captures[0].codec else {
        unreachable!()
    };
    codec.codec = ValueCodecKind::Boolean {
        tokens: vec![BooleanToken {
            token: "1".into(),
            value: true,
        }],
    };
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        unreachable!()
    };
    rolls[0].value = ItemLineValue::Literal(ParameterValue::Quantity(
        FiniteQuantity::new(1.0, a.percent.clone()).unwrap(),
    ));
    let lines = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.item_lines = *lines.identity();
    assert!(ItemSourceLayoutPolicy::new(input, &lines, &a.schema, Default::default()).is_err());
}

#[test]
fn decimal_and_scaling_proofs_retain_shared_policy_and_runtime_resource_limits() {
    let a = scaling_fixture(true);
    let input = a.item_source.input().clone();
    let mut plain = input.clone();
    conditions(&mut plain).clear();
    plain
        .rule_layouts
        .iter_mut()
        .find(|r| r.rule == key("coefficient"))
        .unwrap()
        .role = ItemRuleSourceRole::SingleModifier;
    let plain_policy =
        ItemSourceLayoutPolicy::new(plain.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
    for text_budget in [false, true] {
        let limits = |budget| {
            if text_budget {
                ItemSourceLimits {
                    max_policy_text_bytes: budget,
                    ..Default::default()
                }
            } else {
                ItemSourceLimits {
                    max_schema_work: budget,
                    ..Default::default()
                }
            }
        };
        let maximum = if text_budget {
            ItemSourceLimits::default().max_policy_text_bytes
        } else {
            ItemSourceLimits::default().max_schema_work
        };
        let base = minimum(maximum, |budget| {
            ItemSourceLayoutPolicy::new(plain.clone(), &a.items, &a.schema, limits(budget)).is_ok()
                && encode_item_source_policy(&plain_policy, limits(budget)).is_ok()
        });
        assert!(
            ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits(base)).is_err()
        );
        assert!(encode_item_source_policy(&a.item_source, limits(base)).is_err());
        assert!(decode_item_source_policy(&bytes, &a.items, &a.schema, limits(base)).is_err());
    }
    let text = format!(
        "{{tags:coefficient_family}}{{fractured}}{{range:0.5}}Coefficient {}12.5 widgets",
        "0".repeat(1024)
    );
    let imported = source(&format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\n{text}</Item></Items></PathOfBuilding2>"
    ));
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let origin = item_source(&imported, "7");
    let run =
        |input: &ItemSourceLayoutPolicyInput, limits| -> std::result::Result<(), ItemSourceError> {
            let policy =
                ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).unwrap();
            let plan = policy.attribute(&evidence, origin, &a.items)?;
            let converted = plan.convert(&a.items)?;
            assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
            assert_eq!(converted.modifiers.len(), 1);
            Ok(())
        };
    for output_budget in [false, true] {
        let limits = |budget| {
            if output_budget {
                ItemSourceLimits {
                    max_output_records: budget,
                    ..Default::default()
                }
            } else {
                ItemSourceLimits {
                    max_work: budget,
                    ..Default::default()
                }
            }
        };
        let maximum = if output_budget {
            ItemSourceLimits::default().max_output_records
        } else {
            ItemSourceLimits::default().max_work
        };
        let ordinary_cost = minimum(maximum, |budget| run(&plain, limits(budget)).is_ok());
        assert!(matches!(
            run(&input, limits(ordinary_cost)),
            Err(ItemSourceError::Limit(_)) | Err(ItemSourceError::Lines(ItemLineError::Limit(_)))
        ));
        let guarded_cost = minimum(maximum, |budget| run(&input, limits(budget)).is_ok());
        assert!(guarded_cost > ordinary_cost);
        run(&input, limits(guarded_cost)).unwrap();
    }
    assert_eq!(
        encode_item_source_policy(&a.item_source, Default::default()).unwrap(),
        bytes
    );
}

#[test]
fn escaped_persistent_controls_withhold_every_following_guarded_line_in_v6() {
    let a = scaling_fixture(false);
    for control in ["[{ Modifier - Cold }]", "[ignored|{ Modifier - Cold }]"] {
        let text = format!(
            "Rarity: RARE\nNew Item\nAshen Staff\nCatalyst: Tul's\nCatalystQuality: -200\nImplicits: 0\n{control}\nCoefficient 12 widgets\nCoefficient 14 widgets"
        );
        let plan = attribute_text(&a, &text);
        assert!(!plan.can_convert_lines(), "{control}");
        assert!(
            matches!(plan.report().layout, ItemLayoutStatus::Unsupported(ref problems) if problems.contains(&ItemSourceProblem::UnsupportedSourceControl)),
            "{control}: {:?}",
            plan.report()
        );
        assert!(
            plan.convert(&a.items).unwrap().modifiers.is_empty(),
            "both lines must remain withheld after a hidden persistent control"
        );
        // The new raw-markup restriction must not silently reinterpret old dialects.
        let mut prior = scaling_fixture(false);
        let mut input = prior.item_source.input().clone();
        let ItemSourceDialect::PobExportedSingleTextConditionsV1 {
            flag_bindings,
            metadata_rules,
            ..
        } = input.dialect
        else {
            unreachable!()
        };
        input.schema_version = OWNED_ITEM_SOURCE_PREAMBLE_POLICY_VERSION;
        input.dialect = ItemSourceDialect::PobExportedSingleTextPreambleV1 {
            flag_bindings,
            metadata_rules,
        };
        input
            .rule_layouts
            .iter_mut()
            .find(|r| r.rule == key("coefficient"))
            .unwrap()
            .role = ItemRuleSourceRole::SingleModifier;
        rebuild(&mut prior, input);
        let old = attribute_text(&prior, &text);
        assert!(
            old.can_convert_lines(),
            "legacy raw-control handling stays versioned"
        );
        assert!(!matches!(
            old.report().layout,
            ItemLayoutStatus::Unsupported(_)
        ));
    }
    let plan = attribute_text(
        &a,
        "Rarity: RARE\nTitle [ignored|text]\nAshen Staff\nImplicits: 0\nCoefficient 12 widgets\nCoefficient 14 widgets",
    );
    assert!(
        !plan.can_convert_lines(),
        "markup must be checked before title presentation skips"
    );
    assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
}
