//! Actual original item text crosses normalize_fresh with injected owned policy.
//! This establishes partial input conversion, not source lifecycle or numeric parity.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_draft::{DraftField, DraftListCompletion, DraftQuality, ItemDraft},
};
use poe_optimizer_import::{
    build_instance::SourceOccurrenceId,
    owned_item_lines::{ItemLineOutcome, ItemLinePending},
    owned_item_source::{ItemLayoutStatus, ItemRangeDecision},
    owned_normalize::{
        NormalizedImport, NormalizedItemLine, NormalizedItemText, OwnedOriginTarget,
    },
};
use std::collections::BTreeSet;

#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, normalize, source};

const TWISTER: &str = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml");
const SNIPER: &str = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");

fn converted(
    result: &NormalizedImport,
    source: SourceOccurrenceId,
) -> (&ItemDraft, &NormalizedItemText) {
    let origin = result
        .sidecar()
        .origins
        .iter()
        .find(|row| row.source == source)
        .unwrap();
    let ids: Vec<_> = origin
        .links
        .iter()
        .filter_map(|link| match link {
            OwnedOriginTarget::Item(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 1);
    let item = result
        .draft()
        .input()
        .items
        .members
        .iter()
        .find(|item| item.id == ids[0])
        .unwrap();
    let text = result
        .sidecar()
        .item_texts
        .iter()
        .find(|text| text.source == source)
        .unwrap();
    (item, text)
}
fn line<'a>(text: &'a NormalizedItemText, content: &str) -> &'a NormalizedItemLine {
    text.lines
        .iter()
        .find(|line| line.text.trim() == content)
        .unwrap()
}
fn values(item: &ItemDraft, artifacts: &Artifacts, name: &str) -> Vec<Vec<f64>> {
    let definition = &artifacts.modifiers[name].definition;
    item.modifiers
        .members
        .iter()
        .filter(|modifier| modifier.definition.to_resolved().as_ref() == Some(definition))
        .map(|modifier| {
            modifier
                .rolls
                .members
                .iter()
                .map(|roll| match roll.value.to_resolved().unwrap() {
                    ParameterValue::Quantity(v) => v.value(),
                    _ => panic!("quantity fixture roll"),
                })
                .collect()
        })
        .collect()
}
fn partial_collections(item: &ItemDraft) {
    let DraftField::Pending(order) = &item.modifier_order else {
        panic!("source line positions cannot establish semantic modifier order");
    };
    assert_eq!(order.code.as_str(), "item-modifier-order-not-converted");
    assert!(order.candidates.is_empty());
    assert!(matches!(
        item.parameters.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        item.modifiers.completion,
        DraftListCompletion::Pending { .. }
    ));
}
fn quality_twenty(item: &ItemDraft, artifacts: &Artifacts) {
    let Some(Some(quality)) = item.quality.to_resolved() else {
        panic!("known present quality")
    };
    assert_eq!(quality.kind, artifacts.quality);
    assert_eq!(quality.amount.unit(), &artifacts.percent);
    assert_eq!(quality.amount.value(), 20.0);
}
fn wrapped(item: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="50"/><Items><Item id="7">{item}</Item><ItemSet id="1"><Slot name="Weapon 1" itemId="7"/></ItemSet></Items></PathOfBuilding2>"#
    )
}

#[test]
fn original_twister_explicit_item26_fields_and_rolls_survive_pending_metadata() {
    let artifacts = artifacts();
    let original = source(TWISTER);
    let source_id = item_source(&original, "26");
    let result = normalize(&original, &artifacts);
    let (item, text) = converted(&result, source_id);
    assert_eq!(item.template.to_resolved(), Some(artifacts.spear.clone()));
    assert_eq!(item.item_level.to_resolved(), Some(Some(81)));
    quality_twenty(item, &artifacts);
    partial_collections(item);
    assert_eq!(values(item, &artifacts, "physical"), vec![vec![101.0]]);
    assert_eq!(values(item, &artifacts, "cold"), vec![vec![49.0, 74.0]]);
    assert_eq!(
        values(item, &artifacts, "lightning"),
        vec![vec![6.0, 179.0]]
    );
    assert_eq!(values(item, &artifacts, "attack-speed"), vec![vec![49.0]]);
    assert_eq!(values(item, &artifacts, "critical-bonus"), vec![vec![16.0]]);
    assert_eq!(values(item, &artifacts, "leech"), vec![vec![9.51]]);
    // Source rune/tag semantics have no admitted recipe in this fixture.
    assert!(values(item, &artifacts, "broken-armour").is_empty());
    assert!(values(item, &artifacts, "strike-range").is_empty());
    assert_eq!(item.modifiers.members.len(), 6);
    assert!(matches!(
        text.attribution.layout,
        ItemLayoutStatus::Pending(_)
    ));
    let rune = line(text, "{enchant}{rune}18% increased Physical Damage");
    assert!(matches!(
        rune.outcome,
        ItemLineOutcome::Pending {
            reason: ItemLinePending::SourceMeaningUnresolved,
            ..
        }
    ));
    assert!(rune.modifiers.is_empty());
    let metadata = line(text, "Suffix: {range:1}LocalIncreasedAttackSpeed8");
    assert!(matches!(metadata.outcome, ItemLineOutcome::Known { .. }));
    assert!(metadata.modifiers.is_empty());
    assert_eq!(text.skipped, None);
    assert!(text.content_entry.is_some());
    let line_ids: Vec<_> = text
        .lines
        .iter()
        .flat_map(|line| line.modifiers.iter().copied())
        .collect();
    let record_ids: Vec<_> = item.modifiers.members.iter().map(|m| m.id).collect();
    assert_eq!(line_ids, record_ids);
    assert_eq!(
        line_ids.iter().copied().collect::<BTreeSet<_>>().len(),
        line_ids.len()
    );
    let origin = result
        .sidecar()
        .origins
        .iter()
        .find(|row| row.source == source_id)
        .unwrap();
    for id in line_ids {
        assert!(
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::Modifier(value) if *value == id))
        );
    }
    assert_eq!(result.sidecar().schema_version, 12);
    assert_eq!(
        result.sidecar().item_source_policy,
        *artifacts.item_source.identity()
    );
    assert_eq!(result.sidecar().item_policy, *artifacts.items.identity());
    assert_eq!(result.sidecar().source_sha256, original.source_sha256());
    assert_eq!(original.source_xml(), TWISTER);
}

#[test]
fn original_sniper_staff_resolves_grant_without_inventing_item_level_or_gem() {
    let artifacts = artifacts();
    let original = source(SNIPER);
    let source_id = item_source(&original, "28");
    let result = normalize(&original, &artifacts);
    let (item, text) = converted(&result, source_id);
    assert_eq!(item.template.to_resolved(), Some(artifacts.staff.clone()));
    assert!(matches!(item.item_level, DraftField::Pending(_)));
    quality_twenty(item, &artifacts);
    partial_collections(item);
    assert_eq!(values(item, &artifacts, "spell"), vec![vec![128.0]]);
    assert_eq!(item.modifiers.members.len(), 1);
    let grant = line(text, "{range:0.5}Grants Skill: Level (1-20) Firebolt");
    assert!(matches!(grant.outcome, ItemLineOutcome::Known { .. }));
    assert!(matches!(text.attribution.layout, ItemLayoutStatus::Proven));
    let attributed = text
        .attribution
        .lines
        .iter()
        .find(|l| l.index == grant.index)
        .unwrap();
    assert!(matches!(
        attributed.range,
        ItemRangeDecision::Resolved { fraction: 0.5, .. }
    ));
    assert_eq!(item.parameters.members.len(), 1);
    assert!(
        matches!(item.parameters.members[0].value.to_resolved(), Some(ParameterValue::Integer(v)) if v.get() == 11)
    );
    assert!(grant.modifiers.is_empty());
    assert!(text.skipped.is_none()); // The bounded source adapter resolves these exact writes.
    let fragment = original.source_fragment(source_id).unwrap();
    assert!(fragment.contains("<ModRange"));
    assert!(!fragment.contains("Item Level:"));
    assert!(SNIPER.contains("SummonSkeletalSnipersPlayer"));
    let origin = result
        .sidecar()
        .origins
        .iter()
        .find(|row| row.source == source_id)
        .unwrap();
    assert!(origin.links.iter().all(|link| !matches!(
        link,
        OwnedOriginTarget::Gem(_) | OwnedOriginTarget::Skill(_) | OwnedOriginTarget::Support(_)
    )));
    // Empty injected skill-role evidence preserves unrelated manual skill sources as pending.
    assert!(result.draft().input().gems.members.is_empty());
    assert!(result.draft().input().skills.members.is_empty());
    assert_eq!(original.source_xml(), SNIPER);
}

#[test]
fn unknown_line_preserves_known_fields_and_exact_explicit_speed_despite_tier_metadata() {
    let artifacts = artifacts();
    let original = source(&wrapped(
        "Rarity: RARE\nNew Item\nGrand Spear\nItem Level: 81\nQuality: 20\nSuffix: LocalIncreasedAttackSpeed8 (26-28)\n49% increased Attack Speed\nUnconverted game effect",
    ));
    let result = normalize(&original, &artifacts);
    let (item, text) = converted(&result, item_source(&original, "7"));
    assert_eq!(item.template.to_resolved(), Some(artifacts.spear.clone()));
    assert_eq!(item.item_level.to_resolved(), Some(Some(81)));
    quality_twenty(item, &artifacts);
    assert_eq!(values(item, &artifacts, "attack-speed"), vec![vec![49.0]]);
    assert_eq!(item.modifiers.members.len(), 1);
    assert!(
        line(text, "Suffix: LocalIncreasedAttackSpeed8 (26-28)")
            .modifiers
            .is_empty()
    );
    assert!(matches!(
        line(text, "Unconverted game effect").outcome,
        ItemLineOutcome::Pending {
            reason: ItemLinePending::UnknownLine,
            ..
        }
    ));
    partial_collections(item);
}

#[test]
fn repeated_text_resets_and_unknown_children_do_not_apply_a_filtered_item() {
    let artifacts = artifacts();
    for content in [
        "Rarity: RARE\nNew Item\nGrand Spear\nItem Level: 81<ModRange id=\"1\" range=\"0.5\"/>Quality: 20\n49% increased Attack Speed",
        "Rarity: RARE\nNew Item\nGrand Spear\nItem Level: 81\nQuality: 20\n49% increased Attack Speed<UnknownItemOperation/>",
        "Rarity: RARE\nNew Item\nGrand Spear\nItem Level: 81\nQuality: 20\n49% increased Attack Speed<ModRange xmlns=\"urn:other\" id=\"1\" range=\"0.5\"/>",
    ] {
        let original = source(&wrapped(content));
        let result = normalize(&original, &artifacts);
        let (item, text) = converted(&result, item_source(&original, "7"));
        assert_eq!(
            text.skipped.as_ref().map(|s| s.as_str()),
            Some("item-content-lifecycle-not-converted")
        );
        assert!(text.content_entry.is_none());
        assert!(text.lines.is_empty());
        assert!(matches!(item.template, DraftField::Pending(_)));
        assert!(matches!(item.item_level, DraftField::Pending(_)));
        assert!(matches!(item.quality, DraftQuality::Pending(_)));
        assert!(item.modifiers.members.is_empty());
        partial_collections(item);
    }
}

#[test]
fn two_plain_same_definition_modifiers_keep_distinct_line_and_occurrence_identity() {
    let artifacts = artifacts();
    let original = source(&wrapped(
        "Rarity: RARE\nNew Item\nGrand Spear\nItem Level: 81\nQuality: 20\n18% increased Physical Damage\n101% increased Physical Damage<ModRange id=\"1\" range=\"0.5\"/>",
    ));
    let result = normalize(&original, &artifacts);
    let (item, text) = converted(&result, item_source(&original, "7"));
    assert_eq!(
        values(item, &artifacts, "physical"),
        vec![vec![18.0], vec![101.0]]
    );
    assert_eq!(item.modifiers.members.len(), 2);
    assert_ne!(item.modifiers.members[0].id, item.modifiers.members[1].id);
    assert_eq!(
        item.modifiers.members[0].definition,
        item.modifiers.members[1].definition
    );
    let first = line(text, "18% increased Physical Damage");
    let second = line(text, "101% increased Physical Damage");
    assert!(first.index < second.index);
    assert_eq!(first.modifiers, vec![item.modifiers.members[0].id]);
    assert_eq!(second.modifiers, vec![item.modifiers.members[1].id]);
    partial_collections(item);
}

#[test]
fn incomplete_item_policy_does_not_turn_unrecognized_or_missing_level_into_known_null() {
    use poe_optimizer_import::owned_item_lines::{ItemLineLimits, OwnedItemLinePolicy};
    let mut artifacts = artifacts();
    let mut policy = artifacts.items.input().clone();
    policy.rules.clear();
    artifacts.items =
        OwnedItemLinePolicy::new(policy, &artifacts.schema, ItemLineLimits::default()).unwrap();
    artifacts.item_source = support::source_policy(
        &artifacts.items,
        &artifacts.schema,
        [&artifacts.spear, &artifacts.staff],
        artifacts.item_source.input().source.clone(),
    );
    for content in [
        "Ashen Staff",
        "Rarity: RARE\nNew Item\nGrand Spear\nItem Level: 81",
        "Ashen Staff\nLevelReq: 26",
    ] {
        let original = source(&wrapped(content));
        let result = normalize(&original, &artifacts);
        let (item, text) = converted(&result, item_source(&original, "7"));
        assert!(matches!(item.item_level, DraftField::Pending(_)));
        assert_eq!(item.item_level.to_resolved(), None);
        assert!(text.skipped.is_none());
        assert!(!text.lines.is_empty());
        assert!(
            text.lines
                .iter()
                .all(|line| matches!(line.outcome, ItemLineOutcome::Pending { .. }))
        );
    }
}
