//! Source membership proofs cannot borrow a numeric bound that the text violates.
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, source};

const GRANT: &str = "Grants Skill: Level (1-20) Firebolt";

fn dialects() -> [(u32, ItemSourceDialect); 3] {
    [
        (
            OWNED_ITEM_SOURCE_POLICY_VERSION,
            ItemSourceDialect::PobExportedSingleTextV1,
        ),
        (
            OWNED_ITEM_SOURCE_FLAG_POLICY_VERSION,
            ItemSourceDialect::PobExportedSingleTextFlagsV1 {
                flag_bindings: vec![],
            },
        ),
        (
            OWNED_ITEM_SOURCE_PREAMBLE_POLICY_VERSION,
            ItemSourceDialect::PobExportedSingleTextPreambleV1 {
                flag_bindings: vec![],
                metadata_rules: vec![],
            },
        ),
    ]
}
fn set_dialect(a: &mut Artifacts, version: u32, dialect: ItemSourceDialect) {
    let mut input = a.item_source.input().clone();
    input.schema_version = version;
    input.dialect = dialect;
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
}
fn attribute(a: &Artifacts, middle: &str) -> ItemRangeAttribution {
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nQuality: 20\nRune: None\nImplicits: 1\n49% increased Attack Speed\n{middle}\n{{range:0.5}}{GRANT}\n128% increased Spell Damage<ModRange id=\"1\" range=\"1\"/></Item></Items></PathOfBuilding2>"
    );
    let imported = source(&xml);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
        .unwrap()
}

#[test]
fn finite_out_of_schema_values_cannot_prove_source_indices_or_release_the_next_line() {
    for (version, dialect) in dialects() {
        let mut a = artifacts();
        set_dialect(&mut a, version, dialect);
        let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
        for amount in ["10001", "1000000000000000"] {
            let plain = format!("{amount}% increased Spell Damage");
            assert!(matches!(
                a.items.convert_line(1, &plain, None).unwrap().outcome,
                ItemLineOutcome::Pending {
                    reason: ItemLinePending::ValueOutsideSchema { .. },
                    ..
                }
            ));
            // The raw and tag-stripped probes must both withhold a source role.
            for text in [plain.clone(), format!("{{enchant}}{plain}")] {
                let plan = attribute(&a, &text);
                assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
                let invalid = plan.report().lines.iter().find(|r| r.raw == text).unwrap();
                assert_eq!(invalid.rule, Some(support::key("spell")));
                assert!(invalid.member.is_none());
                assert!(invalid.pending_candidates.contains(&support::key("spell")));
                let following = plan
                    .report()
                    .lines
                    .iter()
                    .find(|r| r.raw.contains(GRANT))
                    .unwrap();
                assert!(
                    following
                        .blockers
                        .contains(&ItemSourceProblem::PossibleCombinedLine)
                );
                assert_eq!(
                    plan.report().writes.last().unwrap().target,
                    ItemRangeTarget::Pending
                );
                let converted = plan.convert(&a.items).unwrap();
                assert!(converted.parameters.is_empty());
                assert_eq!(converted.modifiers.len(), 2);
                assert_eq!(
                    converted.modifiers[0].definition,
                    a.modifiers["attack-speed"].definition
                );
                assert_eq!(
                    converted.modifiers[1].definition,
                    a.modifiers["spell"].definition
                );
                assert_eq!(
                    encode_item_source_policy(&a.item_source, Default::default()).unwrap(),
                    bytes,
                    "source checking must not mutate legacy or current policy identity"
                );
            }
        }
    }
}

#[test]
fn admitted_boundary_values_and_missing_range_facts_keep_their_reviewed_single_line_roles() {
    for (version, dialect) in dialects() {
        let mut a = artifacts();
        set_dialect(&mut a, version, dialect);
        let plan = attribute(&a, "10000% increased Spell Damage");
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
        let following = plan
            .report()
            .lines
            .iter()
            .find(|r| r.raw.contains(GRANT))
            .unwrap();
        assert!(following.member.is_some());
        assert!(following.blockers.is_empty());
        let converted = plan.convert(&a.items).unwrap();
        assert_eq!(converted.parameters.len(), 1);
        assert_eq!(converted.modifiers.len(), 3);
        // An absent range is a separate missing value, not a violated numeric
        // bound; its reviewed role still proves the following physical boundary.
        let plan = attribute(&a, GRANT);
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
        let unresolved = plan.report().lines.iter().find(|r| r.raw == GRANT).unwrap();
        assert!(unresolved.member.is_some());
        assert!(unresolved.blockers.is_empty());
    }
}
