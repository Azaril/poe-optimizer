//! Source-only attribution with injected semantic recipes; no reference VM dependency.
use poe_optimizer_core::owned_build::ParameterValue;
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, source};

const GRANT: &str = "Grants Skill: Level (1-20) Firebolt";
fn staff(body: &str, overlays: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nPrefix: {{range:1}}SpellDamageOnTwoHandWeapon4\nQuality: 20\nRune: None\nImplicits: 1\n{body}{overlays}</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(
    a: &Artifacts,
    xml: &str,
) -> std::result::Result<ItemRangeAttribution, ItemSourceError> {
    let imported = source(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
}
fn grant_level(plan: &ItemRangeAttribution, a: &Artifacts) -> Option<i64> {
    let converted = plan.convert(&a.items).unwrap();
    assert!(converted.parameters.len() <= 1);
    converted
        .parameters
        .first()
        .map(|p| match p.assignment.value {
            ParameterValue::Integer(v) => v.get(),
            _ => panic!("integer grant"),
        })
}
fn rebuild(a: &mut Artifacts, input: ItemSourceLayoutPolicyInput, limits: ItemSourceLimits) {
    a.item_source = ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, limits).unwrap();
}

#[test]
fn inline_fraction_preserves_zero_and_metadata_braces_are_inert() {
    let a = artifacts();
    for (fraction, expected) in [("0", 1), ("0.5", 11), ("1", 20)] {
        let plan = attribute(
            &a,
            &staff(
                &format!("{{range:{fraction}}}{GRANT}\n128% increased Spell Damage"),
                "",
            ),
        )
        .unwrap();
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
        assert_eq!(grant_level(&plan, &a), Some(expected));
        assert_eq!(plan.report().writes.len(), 1);
        let ItemRangeOrigin::Inline { line, span } = &plan.report().writes[0].origin else {
            panic!("inline")
        };
        let row = &plan.report().lines[*line - 1];
        assert_eq!(
            &row.raw[span.start - row.decoded_span.start..span.end - row.decoded_span.start],
            format!("{{range:{fraction}}}")
        );
        assert_eq!(
            plan.report().writes[0].fraction,
            Some(fraction.parse().unwrap())
        );
    }
}

#[test]
fn xml_overrides_inline_and_duplicate_ids_use_last_write_without_clamping_literals() {
    let a = artifacts();
    let xml = staff(
        &format!("{{range:0.5}}{GRANT}\n128% increased Spell Damage"),
        "<ModRange id=\"1\" range=\"1\"/><ModRange id=\"2\" range=\"0\"/><ModRange id=\"1\" range=\"0\"/>",
    );
    let plan = attribute(&a, &xml).unwrap();
    assert_eq!(grant_level(&plan, &a), Some(1));
    let converted = plan.convert(&a.items).unwrap();
    assert_eq!(converted.modifiers.len(), 1);
    assert!(
        matches!(&converted.modifiers[0].rolls[0].value, ParameterValue::Quantity(v) if v.value()==128.0)
    );
    assert_eq!(plan.report().writes.len(), 4);
    assert!(matches!(
        plan.report().writes.last().unwrap().origin,
        ItemRangeOrigin::Xml {
            id: Some(_),
            range: Some(_),
            ..
        }
    ));
    assert_eq!(plan.report().item_lines, *a.items.identity());
    assert_eq!(plan.report().policy, *a.item_source.identity());
}

#[test]
fn load_indices_use_category_order_rather_than_display_positions() {
    let a = artifacts();
    // An enchant appears after the implicit; source load lists put enchants first.
    let plan = attribute(
        &a,
        &staff(
            &format!("{{implicit}}{{range:0}}{GRANT}\n{{enchant}}128% increased Spell Damage"),
            "<ModRange id=\"2\" range=\"1\"/>",
        ),
    )
    .unwrap();
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
    assert_eq!(grant_level(&plan, &a), Some(20));
    let target = plan.report().writes.last().unwrap().target.clone();
    let ItemRangeTarget::Line(line) = target else {
        panic!("target")
    };
    assert!(plan.report().lines[line - 1].raw.contains(GRANT));
}

#[test]
fn out_of_bounds_valid_writes_are_recorded_without_erasing_prior_value() {
    let a = artifacts();
    let plan = attribute(
        &a,
        &staff(
            &format!("{{range:0}}{GRANT}"),
            "<ModRange id=\"100\" range=\"1\"/>",
        ),
    )
    .unwrap();
    assert_eq!(grant_level(&plan, &a), Some(1));
    assert_eq!(
        plan.report().writes.last().unwrap().target,
        ItemRangeTarget::IgnoredOutOfBounds
    );
}

#[test]
fn unresolved_writes_invalidate_then_a_later_exact_write_can_recover_its_target() {
    let a = artifacts();
    for invalid in [
        "<ModRange range=\"0.5\"/>",
        "<ModRange id=\"x\" range=\"0\"/>",
        "<ModRange id=\"1\" range=\"NaN\"/>",
        "<ModRange id=\"0\" range=\"1\"/>",
        "<ModRange id=\"1\" range=\"2\"/>",
    ] {
        let body = format!("{{range:0.5}}{GRANT}");
        let plan = attribute(&a, &staff(&body, invalid)).unwrap();
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
        assert_eq!(grant_level(&plan, &a), None);
        let recovered = attribute(
            &a,
            &staff(&body, &format!("{invalid}<ModRange id=\"1\" range=\"1\"/>")),
        )
        .unwrap();
        assert_eq!(grant_level(&recovered, &a), Some(20));
    }
}

#[test]
fn missing_fraction_is_pending_and_invalid_inline_fraction_can_be_overridden() {
    let a = artifacts();
    for prefix in ["", "{range:NaN}", "{range:-0.1}", "{range:1.1}"] {
        let body = format!("{prefix}{GRANT}");
        assert_eq!(
            grant_level(&attribute(&a, &staff(&body, "")).unwrap(), &a),
            None
        );
        assert_eq!(
            grant_level(
                &attribute(&a, &staff(&body, "<ModRange id=\"1\" range=\"0.5\"/>")).unwrap(),
                &a
            ),
            Some(11)
        );
    }
}

#[test]
fn unknown_or_malformed_members_cannot_silently_shift_overlay_ids() {
    let a = artifacts();
    for extra in [
        "Unconverted effect",
        "nope% increased Spell Damage",
        "Grants Skill: Level (x-20) Firebolt",
    ] {
        let plan = attribute(
            &a,
            &staff(
                &format!("{extra}\n{{range:0.5}}{GRANT}\n128% increased Spell Damage"),
                "<ModRange id=\"1\" range=\"1\"/>",
            ),
        )
        .unwrap();
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
        assert_eq!(grant_level(&plan, &a), None);
        assert_eq!(
            plan.report().writes.last().unwrap().target,
            ItemRangeTarget::Pending
        );
        assert_eq!(plan.convert(&a.items).unwrap().modifiers.len(), 1);
    }
}

#[test]
fn tagged_unknown_meaning_remains_raw_and_never_emits_known_facts() {
    let a = artifacts();
    for prefix in ["{rune}", "{tags:attack}", "{variant:1}", "{broken"] {
        let raw = format!("{prefix}128% increased Spell Damage");
        let plan = attribute(&a, &staff(&raw, "")).unwrap();
        let row = plan.report().lines.last().unwrap();
        assert_eq!(row.raw, raw);
        assert!(!row.blockers.is_empty());
        assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
        assert!(matches!(
            plan.convert(&a.items)
                .unwrap()
                .lines
                .last()
                .unwrap()
                .outcome,
            ItemLineOutcome::Pending {
                reason: ItemLinePending::SourceMeaningUnresolved,
                ..
            }
        ));
    }
}

#[test]
fn malformed_canonical_headers_and_unknown_base_prefix_prevent_range_proof() {
    let a = artifacts();
    let canonical = staff(
        &format!("{{range:0.5}}{GRANT}"),
        "<ModRange id=\"1\" range=\"1\"/>",
    );
    for xml in [
        canonical.replace("Rarity: RARE\n", ""),
        canonical.replace("New Item\nAshen Staff", "Ashen Staff\nNew Item"),
        canonical.replace("Implicits: 1", "Implicits: nope"),
        canonical.replace("Implicits: 1", ""),
        canonical.replace("Rune: None", "Rune: Greater Iron Rune"),
    ] {
        let plan = attribute(&a, &xml).unwrap();
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
        assert_eq!(grant_level(&plan, &a), None);
    }
    let mut a = artifacts();
    let mut input = a.item_source.input().clone();
    input.template_layouts.clear();
    rebuild(&mut a, input, ItemSourceLimits::default());
    assert_eq!(grant_level(&attribute(&a, &canonical).unwrap(), &a), None);
}

#[test]
fn unsupported_content_lifecycle_cannot_apply_filtered_values() {
    let a = artifacts();
    for xml in [
        staff(
            &format!("{{range:0.5}}{GRANT}"),
            "<ModRange id=\"1\" range=\"1\"/>more text",
        ),
        staff(GRANT, "<Unknown/>"),
        staff(GRANT, "").replace("<Item id", "<Item unknown=\"true\" id"),
    ] {
        let plan = attribute(&a, &xml).unwrap();
        assert!(!plan.can_convert_lines());
        assert!(matches!(
            plan.report().layout,
            ItemLayoutStatus::Unsupported(_)
        ));
        let result = plan.convert(&a.items).unwrap();
        assert!(result.parameters.is_empty() && result.modifiers.is_empty());
        assert!(!matches!(result.template, ItemField::Known { .. }));
    }
}

#[test]
fn policy_roundtrip_requires_exact_bindings_and_rejects_unreviewed_declarations() {
    let a = artifacts();
    let bytes = encode_item_source_policy(&a.item_source, ItemSourceLimits::default()).unwrap();
    let decoded =
        decode_item_source_policy(&bytes, &a.items, &a.schema, ItemSourceLimits::default())
            .unwrap();
    assert_eq!(decoded.identity(), a.item_source.identity());
    for mutate in 0..5 {
        let mut input = a.item_source.input().clone();
        match mutate {
            0 => input.rule_layouts.push(input.rule_layouts[0].clone()),
            1 => input.rule_layouts[0].rule = support::key("unknown-rule"),
            2 => input
                .template_layouts
                .push(input.template_layouts[0].clone()),
            3 => input.source.files[0].path = "../escape.lua".into(),
            4 => input.item_lines = "c".repeat(64).parse().unwrap(),
            _ => unreachable!(),
        }
        assert!(
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, ItemSourceLimits::default())
                .is_err()
        );
    }
    let mut input = a.items.input().clone();
    input.version = support::key("changed-semantic-policy");
    let changed = OwnedItemLinePolicy::new(input, &a.schema, ItemLineLimits::default()).unwrap();
    let plan = attribute(&a, &staff(GRANT, "")).unwrap();
    assert!(matches!(
        plan.convert(&changed),
        Err(ItemSourceError::Binding)
    ));
}

#[test]
fn encode_respects_tightened_policy_text_and_wire_limits() {
    let a = artifacts();
    for limits in [
        ItemSourceLimits {
            max_policy_text_bytes: 1,
            ..Default::default()
        },
        ItemSourceLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(encode_item_source_policy(&a.item_source, limits).is_err());
        assert!(
            ItemSourceLayoutPolicy::new(a.item_source.input().clone(), &a.items, &a.schema, limits)
                .is_err()
        );
    }
}

#[test]
fn all_text_fragments_count_toward_source_budget_even_when_lifecycle_is_unsupported() {
    let mut a = artifacts();
    let input = a.item_source.input().clone();
    rebuild(
        &mut a,
        input,
        ItemSourceLimits {
            max_source_bytes: 10,
            ..Default::default()
        },
    );
    let xml = "<PathOfBuilding2><Items><Item id=\"7\">a<ModRange id=\"1\" range=\"0\"/>0123456789</Item></Items></PathOfBuilding2>";
    assert!(matches!(
        attribute(&a, xml),
        Err(ItemSourceError::Limit("source bytes"))
    ));
}

#[test]
fn unresolved_overlay_sweeps_are_charged_to_work_budget() {
    let mut a = artifacts();
    let input = a.item_source.input().clone();
    rebuild(
        &mut a,
        input,
        ItemSourceLimits {
            max_work: 8000,
            ..Default::default()
        },
    );
    let body = format!("x\n{}x", " \n".repeat(1000));
    let xml = |n| {
        format!(
            "<PathOfBuilding2><Items><Item id=\"7\">{body}{}</Item></Items></PathOfBuilding2>",
            "<ModRange/>".repeat(n)
        )
    };
    assert!(attribute(&a, &xml(1)).is_ok());
    assert!(matches!(
        attribute(&a, &xml(100)),
        Err(ItemSourceError::Limit("work"))
    ));
}

#[test]
fn presentation_positions_never_become_item_fields_or_modifiers() {
    let a = artifacts();
    for title in [
        "Item Level: 81",
        "Quality: 20",
        "49% increased Attack Speed",
        "Ashen Staff",
        "Rarity: RARE",
    ] {
        let xml = staff("128% increased Spell Damage", "")
            .replace("New Item", title)
            .replace("Quality: 20\nRune", "Rune");
        let plan = attribute(&a, &xml).unwrap();
        let converted = plan.convert(&a.items).unwrap();
        assert!(matches!(converted.item_level, ItemField::Absent));
        assert!(matches!(converted.quality, ItemField::Absent));
        assert_eq!(converted.modifiers.len(), 1);
        assert_eq!(
            converted.modifiers[0].definition,
            a.modifiers["spell"].definition
        );
        assert!(matches!(converted.template, ItemField::Known { .. }));
        assert!(plan.report().lines[1].presentation);
    }
}

#[test]
fn misplaced_known_headers_remain_pending_and_invalidate_competing_fields() {
    let a = artifacts();
    let xml = staff("128% increased Spell Damage\nQuality: 10", "");
    let plan = attribute(&a, &xml).unwrap();
    assert!(matches!(
        plan.convert(&a.items).unwrap().quality,
        ItemField::Pending { .. }
    ));
    let row = plan.report().lines.last().unwrap();
    assert!(
        row.blockers
            .contains(&ItemSourceProblem::HeaderAfterModifiers)
    );
    assert!(!row.pending_candidates.is_empty());
}

#[test]
fn unknown_tag_preserves_body_candidate_conflicts_in_both_line_orders() {
    let mut a = artifacts();
    let mut input = a.items.input().clone();
    let rule = input
        .rules
        .iter_mut()
        .find(|r| r.id == support::key("ranged-staff-grant"))
        .unwrap();
    let ItemEmission::ItemParameter { value, .. } = &mut rule.emissions[0] else {
        panic!("parameter")
    };
    *value = ItemLineValue::Literal(ParameterValue::Integer(
        poe_optimizer_core::owned_definitions::BoundedInteger::new(11).unwrap(),
    ));
    let mut alias = rule.clone();
    alias.id = support::key("alternate-grant");
    alias.pattern = vec![ItemPatternPart::Literal("Alternate grant".into())];
    alias.captures.clear();
    input.rules.push(alias);
    a.items = OwnedItemLinePolicy::new(input, &a.schema, ItemLineLimits::default()).unwrap();
    a.item_source = support::source_policy(
        &a.items,
        &a.schema,
        [&a.staff, &a.spear],
        a.item_source.input().source.clone(),
    );
    assert_eq!(
        grant_level(&attribute(&a, &staff(GRANT, "")).unwrap(), &a),
        Some(11)
    );
    for body in [
        format!("{GRANT}\n{{variant:1}}Alternate grant"),
        format!("{{variant:1}}Alternate grant\n{GRANT}"),
    ] {
        let plan = attribute(&a, &staff(&body, "")).unwrap();
        assert_eq!(grant_level(&plan, &a), None);
        let row = plan
            .report()
            .lines
            .iter()
            .find(|l| l.raw.contains("variant"))
            .unwrap();
        assert_eq!(row.semantic_text, "Alternate grant");
        assert!(
            row.pending_candidates
                .contains(&support::key("alternate-grant"))
        );
    }
}

#[test]
fn an_unrecognized_line_can_consume_the_next_modifier_but_not_arbitrarily_later_lines() {
    let a = artifacts();
    for prefix in ["Minions deal", "nope% increased Spell Damage"] {
        let plan = attribute(
            &a,
            &staff(
                &format!("{prefix}\n128% increased Spell Damage\n49% increased Attack Speed"),
                "",
            ),
        )
        .unwrap();
        let result = plan.convert(&a.items).unwrap();
        assert_eq!(result.modifiers.len(), 1);
        assert_eq!(
            result.modifiers[0].definition,
            a.modifiers["attack-speed"].definition
        );
        let row = plan
            .report()
            .lines
            .iter()
            .find(|l| l.raw == "128% increased Spell Damage")
            .unwrap();
        assert!(
            row.blockers
                .contains(&ItemSourceProblem::PossibleCombinedLine)
        );
        assert!(row.pending_candidates.contains(&support::key("spell")));
    }
}

#[test]
fn unresolved_chains_and_headers_do_not_release_following_facts_and_reminder_blocks_are_unsupported()
 {
    let a = artifacts();
    for prefix in [
        "Unknown first\nUnknown second",
        "Unknown first\nQuality: 10",
        "Unknown first\n\n",
    ] {
        let plan = attribute(
            &a,
            &staff(&format!("{prefix}\n128% increased Spell Damage"), ""),
        )
        .unwrap();
        assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
    }
    let plan = attribute(
        &a,
        &staff(
            "(Unmapped reminder\n49% increased Attack Speed\n128% increased Spell Damage\nend)",
            "",
        ),
    )
    .unwrap();
    assert!(!plan.can_convert_lines());
    assert!(
        matches!(&plan.report().layout, ItemLayoutStatus::Unsupported(problems) if problems.contains(&ItemSourceProblem::UnsupportedSourceControl))
    );
    let result = plan.convert(&a.items).unwrap();
    assert!(result.modifiers.is_empty() && result.parameters.is_empty());
}
