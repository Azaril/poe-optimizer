//! Intrinsic range provenance uses actual normalized item/roll identities.
//! This fixture declares input recipes only; it grants no numerical coverage.
use super::{converted, support};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::FiniteQuantity,
    owned_draft::DraftListCompletion,
    owned_schema::{DefinitionDescriptor, SchemaState},
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    owned_item_lines::{
        ItemEmission, ItemLineLimits, ItemLineOutcome, ItemLineValue, ItemPatternPart,
        ItemRangeRounding, ItemTextProblem, OwnedItemLinePolicy,
    },
    owned_item_source::{ItemLayoutStatus, ItemRangeDecision, ItemRangeOrigin, ItemRangeTarget},
    owned_mapping::{OwnedMappingIndex, OwnedMappingLimits},
    owned_normalize::{
        NormalizationError, NormalizationLimits, NormalizedImport, OwnedOriginTarget,
        SourceDisposition, SourceOwnedOrigin,
    },
    owned_reward_policy::{OwnedRewardPolicy, RewardPolicyLimits},
    owned_skill_catalog::{OwnedSkillRoleIndex, SkillCatalogLimits},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};

fn item_xml(body: &str, overlays: &str) -> String {
    format!(
        "<PathOfBuilding2><Build level=\"50\"/><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nQuality: 20\nImplicits: 0\n{body}{overlays}</Item><ItemSet id=\"1\"><Slot name=\"Weapon 1\" itemId=\"7\"/></ItemSet></Items><Notes>Unrelated retained content</Notes></PathOfBuilding2>"
    )
}
fn overlay(id: &str, fraction: &str) -> String {
    format!("<ModRange id=\"{id}\" range=\"{fraction}\"/>")
}
fn overlay_origins(result: &NormalizedImport) -> Vec<&SourceOwnedOrigin> {
    result
        .sidecar()
        .item_texts
        .iter()
        .flat_map(|item| item.attribution.writes.iter())
        .filter_map(|write| match &write.origin {
            ItemRangeOrigin::Xml { occurrence, .. } => Some(*occurrence),
            _ => None,
        })
        .map(|source| {
            result
                .sidecar()
                .origins
                .iter()
                .find(|origin| origin.source == source)
                .unwrap()
        })
        .collect()
}
fn assert_fallback(origin: &SourceOwnedOrigin) {
    assert!(matches!(origin.disposition, SourceDisposition::Contributes));
    assert_eq!(origin.links.len(), 1);
    assert!(matches!(origin.links[0], OwnedOriginTarget::Issue(_)));
}
fn assert_all_fallback(result: &NormalizedImport) {
    assert_eq!(result.sidecar().schema_version, 12);
    let origins = overlay_origins(result);
    assert!(!origins.is_empty());
    for origin in origins {
        assert_fallback(origin);
    }
}
fn run(xml: &str, artifacts: &support::Artifacts) -> NormalizedImport {
    support::normalize(&support::source(xml), artifacts)
}
fn ranged_artifacts(two_emissions: bool) -> support::Artifacts {
    let mut artifacts = support::artifacts();
    let mut input = artifacts.items.input().clone();
    let mut rule = input
        .rules
        .iter()
        .find(|r| r.id.as_str() == "physical")
        .unwrap()
        .clone();
    rule.id = support::key("ranged-physical");
    let mut upper = rule.captures[0].clone();
    rule.captures[0].id = support::key("lower");
    upper.id = support::key("upper");
    rule.captures.push(upper);
    rule.pattern = vec![
        ItemPatternPart::Literal("(".into()),
        ItemPatternPart::Capture(support::key("lower")),
        ItemPatternPart::Literal("-".into()),
        ItemPatternPart::Capture(support::key("upper")),
        ItemPatternPart::Literal(") increased Physical Damage".into()),
    ];
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!("modifier fixture");
    };
    rolls[0].value = ItemLineValue::Interpolate {
        lower: support::key("lower"),
        upper: support::key("upper"),
        quantum: ParameterValue::Quantity(
            FiniteQuantity::new(1.0, artifacts.percent.clone()).unwrap(),
        ),
        rounding: ItemRangeRounding::NearestTiesPositive,
    };
    if two_emissions {
        // Same definition, different emission and roll value: source-line identity
        // alone cannot prove either allocated modifier's correspondence.
        let mut second = rule.emissions[0].clone();
        let ItemEmission::Modifier { rolls, .. } = &mut second else {
            unreachable!()
        };
        rolls[0].value = ItemLineValue::Literal(ParameterValue::Quantity(
            FiniteQuantity::new(77.0, artifacts.percent.clone()).unwrap(),
        ));
        rule.emissions.push(second);
    }
    input.rules.push(rule);
    artifacts.items =
        OwnedItemLinePolicy::new(input, &artifacts.schema, ItemLineLimits::default()).unwrap();
    artifacts.item_source = support::source_policy(
        &artifacts.items,
        &artifacts.schema,
        [&artifacts.spear, &artifacts.staff],
        artifacts.item_source.input().source.clone(),
    );
    artifacts
}

/// Keep a valid line recipe while excluding one emitted definition from the
/// selected template. Recognition and aggregate output admission are distinct.
fn filtered_artifacts(retained_position: Option<usize>) -> support::Artifacts {
    let mut artifacts = support::artifacts();
    let mut schema = artifacts.schema.input().clone();
    let staff = schema
        .definitions
        .iter_mut()
        .find_map(|definition| match definition {
            DefinitionDescriptor::ItemTemplate(entry) if entry.id == artifacts.staff => Some(entry),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(staff) = &mut staff.schema else {
        panic!("fixture template");
    };
    staff
        .modifiers
        .members
        .retain(|definition| definition != &artifacts.modifiers["physical"].definition);
    artifacts.schema =
        OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let mut mapping = artifacts.mapping.input().clone();
    mapping.definitions = artifacts.schema.identity().clone();
    artifacts.mapping = OwnedMappingIndex::new(
        mapping,
        &artifacts.registry,
        &artifacts.schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = artifacts.roles.input().clone();
    roles.definitions = artifacts.schema.identity().clone();
    roles.mapping = *artifacts.mapping.identity();
    artifacts.roles = OwnedSkillRoleIndex::new(
        roles,
        &artifacts.mapping,
        &artifacts.schema,
        SkillCatalogLimits::default(),
    )
    .unwrap();
    let mut rewards = artifacts.rewards.input().clone();
    rewards.definitions = artifacts.schema.identity().clone();
    rewards.mapping = *artifacts.mapping.identity();
    artifacts.rewards = OwnedRewardPolicy::new(
        rewards,
        &artifacts.mapping,
        &artifacts.schema,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    let mut items = artifacts.items.input().clone();
    items.definitions = artifacts.schema.identity().clone();
    if let Some(position) = retained_position {
        let retained = items
            .rules
            .iter()
            .find(|rule| rule.id.as_str() == "spell")
            .unwrap()
            .emissions[0]
            .clone();
        items
            .rules
            .iter_mut()
            .find(|rule| rule.id.as_str() == "physical")
            .unwrap()
            .emissions
            .insert(position, retained);
    }
    artifacts.items =
        OwnedItemLinePolicy::new(items, &artifacts.schema, ItemLineLimits::default()).unwrap();
    artifacts.item_source = support::source_policy(
        &artifacts.items,
        &artifacts.schema,
        [&artifacts.spear, &artifacts.staff],
        artifacts.item_source.input().source.clone(),
    );
    artifacts
}

#[test]
fn recognized_but_filtered_modifier_emissions_keep_fallback_without_partial_links() {
    // Exercise a missing first output and a missing later output after a retained
    // emission has already been inspected. Neither may produce a partial link.
    for retained_position in [None, Some(0), Some(1)] {
        let artifacts = filtered_artifacts(retained_position);
        let source = support::source(&item_xml(
            "18% increased Physical Damage",
            &overlay("1", "0.5"),
        ));
        let result = support::normalize(&source, &artifacts);
        let (item, text) = converted(&result, support::item_source(&source, "7"));
        assert!(matches!(text.attribution.layout, ItemLayoutStatus::Proven));
        let output = text
            .lines
            .iter()
            .find(|line| line.text == "18% increased Physical Damage")
            .unwrap();
        let ItemLineOutcome::Known { emissions, .. } = &output.outcome else {
            panic!("recognized line");
        };
        assert_eq!(
            emissions.len(),
            1 + usize::from(retained_position.is_some())
        );
        assert_eq!(
            item.modifiers.members.len(),
            usize::from(retained_position.is_some())
        );
        assert_eq!(
            output.modifiers,
            item.modifiers
                .members
                .iter()
                .map(|modifier| modifier.id)
                .collect::<Vec<_>>()
        );
        assert!(super::values(item, &artifacts, "physical").is_empty());
        assert_eq!(
            super::values(item, &artifacts, "spell"),
            if retained_position.is_some() {
                vec![vec![18.0]]
            } else {
                vec![]
            }
        );
        assert!(
            text.issues
                .iter()
                .any(|issue| issue.problem == ItemTextProblem::ModifierNotAllowed
                    && issue.lines == vec![output.index])
        );
        assert_all_fallback(&result);
        super::partial_collections(item);
    }
}

#[test]
fn actual_modifier_roll_and_repeated_same_definition_occurrences_are_exact() {
    let artifacts = ranged_artifacts(false);
    let xml = item_xml(
        "(10-30) increased Physical Damage\n(40-60) increased Physical Damage",
        &(overlay("1", "0.25") + &overlay("2", "0.75")),
    );
    let source = support::source(&xml);
    let result = support::normalize(&source, &artifacts);
    let (item, text) = converted(&result, support::item_source(&source, "7"));
    assert_eq!(result.sidecar().schema_version, 19);
    assert!(matches!(text.attribution.layout, ItemLayoutStatus::Proven));
    assert_eq!(
        super::values(item, &artifacts, "physical"),
        vec![vec![15.0], vec![55.0]]
    );
    assert_ne!(item.modifiers.members[0].id, item.modifiers.members[1].id);
    for (origin, modifier) in overlay_origins(&result).iter().zip(&item.modifiers.members) {
        assert_eq!(
            origin.links,
            vec![
                OwnedOriginTarget::Item(item.id),
                OwnedOriginTarget::Modifier(modifier.id)
            ]
        );
    }
    super::partial_collections(item);
}

#[test]
fn all_emissions_of_one_line_keep_their_actual_order_and_identity() {
    let artifacts = ranged_artifacts(true);
    let xml = item_xml("(10-30) increased Physical Damage", &overlay("1", "0.5"));
    let source = support::source(&xml);
    let result = support::normalize(&source, &artifacts);
    let (item, text) = converted(&result, support::item_source(&source, "7"));
    assert_eq!(
        super::values(item, &artifacts, "physical"),
        vec![vec![20.0], vec![77.0]]
    );
    assert_eq!(item.modifiers.members.len(), 2);
    assert_ne!(item.modifiers.members[0].id, item.modifiers.members[1].id);
    let line = text.lines.iter().find(|l| l.modifiers.len() == 2).unwrap();
    assert_eq!(
        line.modifiers,
        item.modifiers
            .members
            .iter()
            .map(|m| m.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        overlay_origins(&result)[0].links,
        vec![
            OwnedOriginTarget::Item(item.id),
            OwnedOriginTarget::Modifier(item.modifiers.members[0].id),
            OwnedOriginTarget::Modifier(item.modifiers.members[1].id),
        ]
    );
}

#[test]
fn fixed_literal_has_correspondence_without_becoming_a_fractional_roll() {
    let artifacts = support::artifacts();
    for fraction in ["0", "0.5", "1"] {
        let xml = item_xml("128% increased Spell Damage", &overlay("1", fraction));
        let source = support::source(&xml);
        let result = support::normalize(&source, &artifacts);
        let (item, text) = converted(&result, support::item_source(&source, "7"));
        assert_eq!(result.sidecar().schema_version, 19);
        assert_eq!(super::values(item, &artifacts, "spell"), vec![vec![128.0]]);
        assert_eq!(
            overlay_origins(&result)[0].links,
            vec![
                OwnedOriginTarget::Item(item.id),
                OwnedOriginTarget::Modifier(item.modifiers.members[0].id)
            ]
        );
        assert!(text.attribution.lines.iter().any(|l| matches!(l.range, ItemRangeDecision::Resolved { fraction: actual, .. } if actual == fraction.parse::<f64>().unwrap())));
    }
}

#[test]
fn unresolved_target_attaches_only_its_own_live_item_inventory_obligation() {
    let artifacts = support::artifacts();
    let xml = item_xml("Unknown game effect\n128% increased Spell Damage", &overlay("1", "0.5"))
        .replace("<ItemSet", "<Item id=\"8\">Rarity: RARE\nNew Item\nAshen Staff\nImplicits: 0\nOther unknown effect<ModRange id=\"1\" range=\"0.5\"/></Item><ItemSet");
    let source = support::source(&xml);
    let result = support::normalize(&source, &artifacts);
    assert_eq!(result.sidecar().schema_version, 19);
    for (origin, id) in overlay_origins(&result).iter().zip(["7", "8"]) {
        let (item, text) = converted(&result, support::item_source(&source, id));
        assert!(matches!(
            text.attribution.writes[0].target,
            ItemRangeTarget::Pending
        ));
        let DraftListCompletion::Pending { id: issue, code } = &item.modifiers.completion else {
            panic!("actual item obligation");
        };
        assert_eq!(code.as_str(), "item-modifiers-not-converted");
        assert_eq!(
            origin.links,
            vec![
                OwnedOriginTarget::Item(item.id),
                OwnedOriginTarget::Issue(*issue)
            ]
        );
        assert!(
            origin
                .links
                .iter()
                .all(|l| !matches!(l, OwnedOriginTarget::Modifier(_)))
        );
    }
}

#[test]
fn malformed_nested_unknown_and_reset_frames_keep_historical_fallback() {
    let artifacts = support::artifacts();
    for mutation in [
        "<ModRange id=\"1\" range=\"NaN\"/>",
        "<ModRange id=\"1\" range=\"1.01\"/>",
        "<ModRange range=\"0.5\"/>",
        "<ModRange id=\"0\" range=\"0.5\"/>",
        "<ModRange id=\"1\" range=\"0.5\" extra=\"unknown\"/>",
        "<ModRange id=\"1\" range=\"0.5\"><Nested/></ModRange>",
        "<ModRange id=\"1\" range=\"0.5\">unknown text</ModRange>",
        "<ModRange xmlns=\"urn:foreign\" id=\"1\" range=\"0.5\"/>",
        "<ModRange id=\"1\" range=\"0.5\"/><UnknownItemOperation/>",
        "<ModRange id=\"1\" range=\"0.5\"/>Rarity: NORMAL\nAshen Staff",
    ] {
        let result = run(
            &item_xml("128% increased Spell Damage", mutation),
            &artifacts,
        );
        assert_all_fallback(&result);
    }
}

#[test]
fn duplicate_or_numeric_alias_item_ids_do_not_claim_loader_ownership() {
    let artifacts = support::artifacts();
    for duplicate in ["7", "07"] {
        let xml = item_xml("128% increased Spell Damage", &overlay("1", "0.5")).replace(
            "<ItemSet",
            &format!("<Item id=\"{duplicate}\">Rarity: NORMAL\nAshen Staff</Item><ItemSet"),
        );
        assert_all_fallback(&run(&xml, &artifacts));
    }
}

#[test]
fn superseded_and_out_of_bounds_writes_are_not_promoted_to_output_authority() {
    let artifacts = support::artifacts();
    let xml = item_xml(
        "128% increased Spell Damage",
        &(overlay("1", "0") + &overlay("1", "1")),
    );
    let result = run(&xml, &artifacts);
    let origins = overlay_origins(&result);
    assert_fallback(origins[0]);
    assert_eq!(result.sidecar().schema_version, 19);
    assert!(
        origins[1]
            .links
            .iter()
            .any(|l| matches!(l, OwnedOriginTarget::Modifier(_)))
    );
    let out_of_bounds = run(
        &item_xml("128% increased Spell Damage", &overlay("99", "1")),
        &artifacts,
    );
    assert_all_fallback(&out_of_bounds);
}

#[test]
fn recovered_line_does_not_override_a_pending_whole_item_layout() {
    let artifacts = ranged_artifacts(false);
    let xml = item_xml(
        "(10-30) increased Physical Damage",
        &(overlay("1", "NaN") + &overlay("1", "0.5")),
    );
    let source = support::source(&xml);
    let result = support::normalize(&source, &artifacts);
    let (item, text) = converted(&result, support::item_source(&source, "7"));
    assert!(matches!(
        text.attribution.layout,
        ItemLayoutStatus::Pending(_)
    ));
    assert!(text.attribution.lines.iter().any(|line| matches!(
        line.range,
        ItemRangeDecision::Resolved {
            winning_write: 1,
            fraction: 0.5
        }
    )));
    assert_eq!(
        super::values(item, &artifacts, "physical"),
        vec![vec![20.0]]
    );
    assert_all_fallback(&result);
}

#[test]
fn parameter_emissions_and_overlay_absence_keep_the_legacy_sidecar() {
    let artifacts = support::artifacts();
    let result = run(
        &item_xml("Grants Skill: Level (1-20) Firebolt", &overlay("1", "0.5")),
        &artifacts,
    );
    assert_all_fallback(&result);
    assert!(result.draft().input().items.members[0].parameters.members.iter().any(|p| matches!(p.value.to_resolved(), Some(ParameterValue::Integer(v)) if v.get() == 11)));
    let none = run(&item_xml("128% increased Spell Damage", ""), &artifacts);
    assert_eq!(none.sidecar().schema_version, 12);
    assert!(overlay_origins(&none).is_empty());
}

#[test]
fn attachment_changes_only_the_overlay_origin_not_draft_or_unrelated_links() {
    let artifacts = support::artifacts();
    let positive = run(
        &item_xml("128% increased Spell Damage", &overlay("1", "0.5")),
        &artifacts,
    );
    let refused = run(
        &item_xml("128% increased Spell Damage", &overlay("1", "NaN")),
        &artifacts,
    );
    assert_eq!(
        serde_json::to_value(positive.draft().input()).unwrap(),
        serde_json::to_value(refused.draft().input()).unwrap()
    );
    assert_eq!(
        positive.sidecar().allocator_after,
        refused.sidecar().allocator_after
    );
    assert_eq!(positive.sidecar().draft, refused.sidecar().draft);
    let changed = overlay_origins(&positive)[0].source.ordinal();
    assert_eq!(
        positive.sidecar().origins.len(),
        refused.sidecar().origins.len()
    );
    for (a, b) in positive
        .sidecar()
        .origins
        .iter()
        .zip(&refused.sidecar().origins)
    {
        assert_eq!(a.source.ordinal(), b.source.ordinal());
        if a.source.ordinal() != changed {
            assert_eq!(a.disposition, b.disposition);
            assert_eq!(a.links, b.links);
        }
    }
}

#[test]
fn work_and_origin_link_limits_fail_atomically_and_repeat_deterministically() {
    let artifacts = support::artifacts();
    let source: ImportedBuildInstance = support::source(&item_xml(
        "128% increased Spell Damage",
        &overlay("1", "0.5"),
    ));
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    assert!(evidence.rows().len() > 1);
    for limits in [
        NormalizationLimits {
            max_work: 1,
            ..Default::default()
        },
        NormalizationLimits {
            max_origin_links: 1,
            ..Default::default()
        },
    ] {
        for _ in 0..2 {
            assert!(matches!(
                support::normalize_with_limits(&source, &artifacts, limits),
                Err(NormalizationError::Limit(_))
            ));
        }
    }
    let first = support::normalize(&source, &artifacts);
    let second = support::normalize(&source, &artifacts);
    assert_eq!(
        serde_json::to_value(first.sidecar()).unwrap(),
        serde_json::to_value(second.sidecar()).unwrap()
    );
    let (mut low, mut high) = (1, NormalizationLimits::default().max_work);
    while low < high {
        let middle = low + (high - low) / 2;
        match support::normalize_with_limits(
            &source,
            &artifacts,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit("work")) => low = middle + 1,
            other => panic!("unexpected work-bound result: {other:?}"),
        }
    }
    assert!(low > 1);
    for _ in 0..2 {
        let at_limit = support::normalize_with_limits(
            &source,
            &artifacts,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(first.sidecar()).unwrap(),
            serde_json::to_value(at_limit.sidecar()).unwrap()
        );
        assert!(matches!(
            support::normalize_with_limits(
                &source,
                &artifacts,
                NormalizationLimits {
                    max_work: low - 1,
                    ..Default::default()
                }
            ),
            Err(NormalizationError::Limit("work"))
        ));
    }
    let exact_links = first
        .sidecar()
        .origins
        .iter()
        .map(|row| row.links.len())
        .sum();
    let at_limit = support::normalize_with_limits(
        &source,
        &artifacts,
        NormalizationLimits {
            max_origin_links: exact_links,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(first.sidecar()).unwrap(),
        serde_json::to_value(at_limit.sidecar()).unwrap()
    );
    assert!(matches!(
        support::normalize_with_limits(
            &source,
            &artifacts,
            NormalizationLimits {
                max_origin_links: exact_links - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit("origin links"))
    ));
}
