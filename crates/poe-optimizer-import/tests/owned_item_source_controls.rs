//! Stateful source controls cannot make later standalone facts authoritative.
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

const GRANT: &str = "Grants Skill: Level (1-20) Firebolt";
fn xml(title: &str, metadata: &str, body: &str, overlays: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\n{title}\nAshen Staff\n{metadata}\nQuality: 20\nRune: None\nImplicits: 0\n{body}{overlays}</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(a: &Artifacts, xml: &str) -> ItemRangeAttribution {
    let imported = source(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
        .unwrap()
}

#[test]
fn advanced_copy_control_blocks_all_following_members_and_preserves_range_evidence() {
    let a = artifacts();
    for header in [
        "{ Unscalable Modifier }",
        "{ Crafted Modifier }",
        "{ Fractured Modifier }",
        "{ Modifier - cold  - 20% Increased }",
    ] {
        let plan = attribute(
            &a,
            &xml(
                "New Item",
                "Prefix: ignored",
                &format!(
                    "{header}\n128% increased Spell Damage\n129% increased Spell Damage\n{{range:0.5}}{GRANT}"
                ),
                "<ModRange id=\"3\" range=\"1\"/>",
            ),
        );
        assert!(!plan.can_convert_lines(), "{header}");
        assert!(
            matches!(&plan.report().layout, ItemLayoutStatus::Unsupported(problems)
                if problems.contains(&ItemSourceProblem::UnsupportedSourceControl)),
            "{header}"
        );
        assert!(plan.report().lines.iter().any(|line| line.raw == header));
        let converted = plan.convert(&a.items).unwrap();
        assert!(converted.modifiers.is_empty(), "{header}");
        assert!(converted.parameters.is_empty(), "{header}");
        // More than one later literal is essential: a possible combined-line
        // blocker alone only suppresses the immediate successor of a header.
        for text in ["128% increased Spell Damage", "129% increased Spell Damage"] {
            let line = converted
                .lines
                .iter()
                .find(|line| line.text == text)
                .unwrap();
            assert!(
                matches!(&line.outcome, ItemLineOutcome::Pending {
                    reason: ItemLinePending::SourceMeaningUnresolved, candidates
                } if candidates.contains(&key("spell"))),
                "{header}: {text}"
            );
        }
        assert_eq!(plan.report().writes.len(), 2, "{header}");
        assert_eq!(plan.report().writes[0].fraction, Some(0.5));
        assert_eq!(plan.report().writes[1].fraction, Some(1.0));
        assert_eq!(plan.report().writes[1].target, ItemRangeTarget::Pending);
        assert!(plan.report().lines.iter().any(
            |line| line.raw.contains(GRANT) && matches!(line.range, ItemRangeDecision::Pending)
        ));
    }
}

#[test]
fn presentation_title_and_prefixed_metadata_are_not_advanced_source_controls() {
    let a = artifacts();
    let plan = attribute(
        &a,
        &xml(
            "{ Unscalable Modifier }",
            "Prefix: { Unscalable Modifier }",
            "128% increased Spell Damage",
            "",
        ),
    );
    assert!(plan.can_convert_lines());
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
    assert!(plan.report().lines[1].presentation);
    let converted = plan.convert(&a.items).unwrap();
    assert_eq!(converted.modifiers.len(), 1);
    assert!(matches!(
        converted.lines[1].outcome,
        ItemLineOutcome::Pending {
            reason: ItemLinePending::SourcePresentation,
            ..
        }
    ));
}
