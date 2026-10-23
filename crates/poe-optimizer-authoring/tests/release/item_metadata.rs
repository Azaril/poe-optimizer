//! Inert item metadata consumes the maintained Import policies directly.
//! Successful source attribution does not establish complete build coverage.
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::ParameterValue, owned_definitions::SlotOwnerDefId,
    owned_schema::SchemaClosure,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ConvertedItemEmission, ItemLineOutcome, LocatedItemModifier},
    owned_item_source::{
        ItemLayoutStatus, ItemRangeAttribution, ItemRangeDecision, ItemRangeTarget,
        ItemSourceProblem,
    },
    owned_source::SourceProjectEvidence,
};

const BODY: &str = "{range:0.5}(10-14)% increased Physical Damage";
const TAGGED_BODY: &str = "{tags:physical,damage}{range:0.5}(10-14)% increased Physical Damage";
const HEADERS: [(&str, usize); 3] = [
    ("Unique ID: abc-123\n", 1),
    ("Unique ID: alpha\nUnique ID: beta\n", 2),
    ("Unique ID: {range:0.1}{tags:physical}{rune}\n", 1),
];

fn item(headers: &str, body: &str) -> String {
    format!("Rarity: RARE\nMetadata Fixture\nGrand Spear\n{headers}Implicits: 0\n{body}")
}

fn attribute(text: &str) -> ItemRangeAttribution {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">{escaped}</Item></Items></PathOfBuilding2>"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([95; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let rows: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "Item")
        .collect();
    assert_eq!(rows.len(), 1);
    let release = super::tracked_release().release();
    release
        .item_source()
        .attribute(&evidence, rows[0].occurrence().id(), release.items())
        .unwrap()
}

fn assert_metadata_only(outcome: &ItemLineOutcome) {
    let ItemLineOutcome::Known { rule, emissions } = outcome else {
        panic!("metadata grammar must be known: {outcome:?}")
    };
    assert_eq!(rule.as_str(), "unique-id-metadata");
    assert_eq!(emissions.len(), 1);
    assert!(
        matches!(&emissions[0], ConvertedItemEmission::Metadata { role }
        if role.as_str() == "source-identity-metadata")
    );
}

fn assert_physical_amount(modifier: &LocatedItemModifier) {
    // Independent expected raw range result: 10 + (14 - 10) * 0.5 = 12.
    // Metadata must not redirect this quantity to another parameter or owner.
    assert_eq!(modifier.definition.key().as_str(), "def.0000000000002570");
    let amounts: Vec<_> = modifier
        .rolls
        .iter()
        .filter(|roll| roll.slot.slot.key().as_str() == "def.0000000000002571")
        .collect();
    assert_eq!(amounts.len(), 1);
    assert_eq!(
        amounts[0].slot.declaration,
        SlotOwnerDefId::Modifier(modifier.definition.clone())
    );
    let ParameterValue::Quantity(amount) = &amounts[0].value else {
        panic!("physical modifier quantity")
    };
    assert_eq!(amount.value(), 12.0);
    assert_eq!(amount.unit().key().as_str(), "def.0000000000000002");
    assert!(matches!(
        modifier.rolls_closure,
        SchemaClosure::Partial { .. }
    ));
}

#[test]
fn tracked_item_metadata_preserves_modifier_range_and_remains_inert() {
    let lines = super::tracked_release().release().items();
    let baseline = attribute(&item("", BODY));
    assert!(matches!(baseline.report().layout, ItemLayoutStatus::Proven));
    let baseline_values = baseline.convert(lines).unwrap();
    assert_eq!(baseline_values.modifiers.len(), 1);
    assert_physical_amount(&baseline_values.modifiers[0]);
    for (headers, expected_metadata_rows) in HEADERS {
        let attributed = attribute(&item(headers, BODY));
        assert!(
            matches!(attributed.report().layout, ItemLayoutStatus::Proven),
            "metadata preamble: {:?}",
            attributed.report().layout
        );
        let converted = attributed.convert(lines).unwrap();
        assert!(converted.parameters.is_empty());
        assert_eq!(converted.defaults, baseline_values.defaults);
        assert_eq!(converted.modifiers.len(), 1);
        let modifier = &converted.modifiers[0];
        assert_physical_amount(modifier);
        assert_eq!(modifier.rolls, baseline_values.modifiers[0].rolls);
        assert_eq!(attributed.report().writes.len(), 1);
        let write = &attributed.report().writes[0];
        assert_eq!(write.fraction, Some(0.5));
        assert_eq!(write.target, ItemRangeTarget::Line(modifier.line));
        let metadata: Vec<_> = attributed
            .report()
            .lines
            .iter()
            .filter(|row| row.raw.starts_with("Unique ID: "))
            .collect();
        assert_eq!(metadata.len(), expected_metadata_rows);
        for row in metadata {
            assert!(row.member.is_none() && row.blockers.is_empty());
            assert_eq!(row.semantic_text, row.raw);
            assert!(row.properties.is_empty());
            assert!(row.property_tokens.is_empty());
            assert!(row.flag_tokens.is_empty());
            assert!(matches!(row.range, ItemRangeDecision::Absent));
            assert_metadata_only(
                &converted
                    .lines
                    .iter()
                    .find(|line| line.index == row.index)
                    .unwrap()
                    .outcome,
            );
            assert!(
                !converted
                    .modifiers
                    .iter()
                    .any(|value| value.line == row.index)
            );
            assert!(
                !converted
                    .parameters
                    .iter()
                    .any(|value| value.line == row.index)
            );
        }
    }
}

#[test]
fn tracked_item_metadata_cannot_authorize_a_rejected_tagged_modifier() {
    let lines = super::tracked_release().release().items();
    // The current source rule requires no modifier-scaling tags. Its old
    // publication-stage fixture admitted these tags; metadata cannot do so now.
    for headers in std::iter::once("").chain(HEADERS.into_iter().map(|(text, _)| text)) {
        let attributed = attribute(&item(headers, TAGGED_BODY));
        assert!(!matches!(
            attributed.report().layout,
            ItemLayoutStatus::Proven
        ));
        let tagged = attributed
            .report()
            .lines
            .iter()
            .find(|row| row.raw == TAGGED_BODY)
            .unwrap();
        assert!(tagged.member.is_none());
        assert!(
            tagged
                .blockers
                .contains(&ItemSourceProblem::UnprovedMemberConditions)
        );
        let converted = attributed.convert(lines).unwrap();
        assert!(converted.modifiers.is_empty());
        assert!(converted.parameters.is_empty());
        assert!(converted.defaults.parameters.is_empty());
    }
}

#[test]
fn tracked_item_metadata_rejects_invalid_preambles_and_source_controls() {
    let lines = super::tracked_release().release().items();
    for raw in [
        format!(
            "Rarity: RARE\nMetadata Fixture\nUnique ID: too-early\nGrand Spear\nImplicits: 0\n{BODY}"
        ),
        item("", "17% increased Physical Damage\nUnique ID: too-late"),
        item("Unique ID: \n", BODY),
        item("Unknown Header: inert-looking\n", BODY),
    ] {
        let attributed = attribute(&raw);
        assert!(
            !matches!(attributed.report().layout, ItemLayoutStatus::Proven),
            "invalid preamble admitted: {raw}"
        );
        assert!(
            attributed
                .convert(lines)
                .unwrap()
                .defaults
                .parameters
                .is_empty()
        );
    }
    for value in [
        "prefix {variant:1} suffix",
        "{version:1}",
        "{group:1}",
        "prefix Foil Unique suffix",
        "[text]",
        "<text>",
    ] {
        let attributed = attribute(&item(&format!("Unique ID: {value}\n"), BODY));
        assert!(
            matches!(&attributed.report().layout, ItemLayoutStatus::Unsupported(problems)
                if problems.contains(&ItemSourceProblem::UnsupportedSourceControl)),
            "control must be unsupported: {value}"
        );
        assert!(attributed.report().lines.iter().any(|row| {
            row.raw.starts_with("Unique ID: ")
                && row
                    .blockers
                    .contains(&ItemSourceProblem::UnsupportedSourceControl)
        }));
        let converted = attributed.convert(lines).unwrap();
        assert!(converted.modifiers.is_empty());
        assert!(converted.parameters.is_empty());
        assert!(converted.defaults.parameters.is_empty());
    }
}
