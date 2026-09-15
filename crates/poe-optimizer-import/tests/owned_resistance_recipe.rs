//! Validate the persisted producer component through production constructors.
//! No PoB checkout, source VM, legacy profile, final metric or build-parity claim.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::ParameterValue,
    owned_definitions::{ModifierDefId, OwnedDefinitionKey, RewardDefId, StatDefId},
    owned_rules::{ContributionKind, RuleEffectKind, RuleEntity},
    owned_schema::{SchemaClosure, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_item_lines::{
        ConvertedItemEmission, ItemField, ItemLineInput, ItemLineLimits, ItemLineOutcome,
        OwnedItemLinePolicy, decode_item_line_policy,
    },
    owned_item_source::{
        ItemLayoutStatus, ItemRangeAttribution, ItemSourceDefaultScope, ItemSourceLayoutPolicy,
        ItemSourceLimits, decode_item_source_policy,
    },
    owned_recipe::{OwnedRecipeLimits, StagedOwnedRecipe, decode_owned_recipe},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn data(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/owned/poe2/3887ae68/resistance")
            .join(name),
    )
    .unwrap()
}
struct Component {
    staged: StagedOwnedRecipe,
    rules: CompiledRulePackage,
    items: OwnedItemLinePolicy,
    layout: ItemSourceLayoutPolicy,
    ids: Value,
}
impl Component {
    fn load() -> Self {
        let raw = data("recipe.json");
        let facts: Value = serde_json::from_slice(&data("source-facts.json")).unwrap();
        assert_eq!(
            facts["recipe_sha256"].as_str().unwrap(),
            format!("{:x}", Sha256::digest(&raw))
        );
        assert_eq!(facts["metric_producer"], false);
        assert_eq!(facts["range_conversion"]["implemented"], true);
        assert_eq!(facts["whole_original_native_completion"], "0/5");
        // This path deserializes every exact DTO, checks registry/schema closure,
        // compiles all programs, and validates the injected routing package.
        let staged = decode_owned_recipe(&raw, OwnedRecipeLimits::default()).unwrap();
        let rules = CompiledRulePackage::compile(
            staged.rules().input(),
            staged.schema(),
            RuleLimits::default(),
        )
        .unwrap();
        let items = decode_item_line_policy(
            &data("items.json"),
            staged.schema(),
            ItemLineLimits::default(),
        )
        .unwrap();
        let layout = decode_item_source_policy(
            &data("item-source.json"),
            &items,
            staged.schema(),
            ItemSourceLimits::default(),
        )
        .unwrap();
        assert_eq!(&layout.input().item_lines, items.identity());
        assert_eq!(items.input().definitions, *staged.schema().identity());
        Self {
            staged,
            rules,
            items,
            layout,
            ids: serde_json::from_slice(&data("ids.json")).unwrap(),
        }
    }
    fn id<T: DeserializeOwned>(&self, label: &str) -> T {
        serde_json::from_value(self.ids["allocations"][label].clone()).unwrap()
    }
}

#[test]
fn persisted_recipe_and_exact_item_policies_pass_production_constructors() {
    let c = Component::load();
    assert_eq!(c.staged.registry().input().entries.len(), 2554);
    assert_eq!(c.staged.registry().input().last_issued.get(), 2554);
    assert!(c.staged.manifest().partial_rule_owners > 0);
    assert_eq!(c.staged.manifest().calculation, "not_run");
    let base: Value = serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/import/compiled/registry.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let successor = serde_json::to_value(c.staged.registry().input()).unwrap();
    assert_eq!(
        &successor["entries"].as_array().unwrap()[..2515],
        base["entries"].as_array().unwrap()
    );
}

#[test]
fn generic_signed_fixed_lines_supply_typed_rolls_to_real_owned_programs() {
    let c = Component::load();
    let mut scratch = c.rules.new_scratch();
    for (text, expected, kind) in [
        ("+17% to Cold Resistance", 17.0, "cold"),
        ("-3% to Cold Resistance", -3.0, "cold"),
        ("0% to Cold Resistance", 0.0, "cold"),
        ("349% to Cold Resistance", 349.0, "cold"),
        ("+16% to all Elemental Resistances", 16.0, "elemental"),
        ("-5% to all Elemental Resistances", -5.0, "elemental"),
    ] {
        let line = c.items.convert_line(1, text, None).unwrap();
        let ItemLineOutcome::Known { emissions, .. } = line.outcome else {
            panic!("fixed line should decode: {text}");
        };
        let [ConvertedItemEmission::Modifier { definition, rolls }] = emissions.as_slice() else {
            panic!("one modifier per fixed line");
        };
        let wanted: ModifierDefId = c.id(&format!("flat-{kind}-modifier"));
        let stat: StatDefId = c.id(&format!("{kind}-resistance-base-contributions"));
        assert_eq!(definition, &wanted);
        assert_eq!(rolls.len(), 1);
        let ParameterValue::Quantity(amount) = &rolls[0].value else {
            panic!("exact quantity roll");
        };
        assert_eq!(amount.value(), expected);
        let result = c
            .rules
            .evaluate(
                &SchemaSubject::Definition(definition.address()),
                &key("resistance-contribution"),
                &[RuleFact {
                    read: key("amount"),
                    value: rolls[0].value.clone(),
                }],
                c.staged.schema(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        assert!(matches!(
            &result.effects[0].effect,
            RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: actual,
                contribution: ContributionKind::Add,
                ..
            } if actual == &stat
        ));
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: rolls[0].value.clone()
            }
        );
    }
}

#[test]
fn repeated_equal_definition_lines_remain_separate_modifiers_and_missing_roll_is_unresolved() {
    let c = Component::load();
    let converted = c
        .items
        .convert_text("Sapphire Ring\n+17% to Cold Resistance\n+19% to Cold Resistance")
        .unwrap();
    assert_eq!(converted.modifiers.len(), 2);
    assert_eq!(
        converted.modifiers[0].definition,
        converted.modifiers[1].definition
    );
    assert_ne!(converted.modifiers[0].line, converted.modifiers[1].line);
    assert_ne!(converted.modifiers[0].rolls, converted.modifiers[1].rolls);
    let owner: ModifierDefId = c.id("flat-cold-modifier");
    let result = c
        .rules
        .evaluate(
            &SchemaSubject::Definition(owner.address()),
            &key("resistance-contribution"),
            &[],
            c.staged.schema(),
            &mut c.rules.new_scratch(),
        )
        .unwrap();
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Unresolved {
            input: key("amount")
        }
    );
}

#[test]
fn unsupported_ranges_unknown_lines_and_metadata_do_not_become_numeric_facts() {
    let c = Component::load();
    for text in [
        "+(+20-30)% to Cold Resistance",
        "-(20-30)% to Cold Resistance",
        "+(20.0-30)% to Cold Resistance",
        "{range:0.5}+(20-30)% to Cold Resistance",
        "unknown nearby effect",
    ] {
        for fraction in [None, Some(0.0), Some(0.5), Some(1.0)] {
            assert!(matches!(
                c.items.convert_line(1, text, fraction).unwrap().outcome,
                ItemLineOutcome::Pending { .. }
            ));
        }
    }
    let converted = c
        .items
        .convert_text("Sapphire Ring\nItem Level: 77\nQuality: 20\n+17% to Cold Resistance")
        .unwrap();
    assert!(matches!(converted.item_level, ItemField::Absent));
    assert!(matches!(converted.quality, ItemField::Absent));
    assert_eq!(converted.modifiers.len(), 1);
}

#[test]
fn source_reviewed_reward_programs_contribute_without_hiding_partial_effects() {
    let c = Component::load();
    let facts: Value = serde_json::from_slice(&data("source-facts.json")).unwrap();
    let mut scratch = c.rules.new_scratch();
    for row in facts["reward_outcomes"].as_array().unwrap() {
        let reward: RewardDefId = serde_json::from_value(row["target"].clone()).unwrap();
        let expected = row["values"][0]["amount"].as_f64().unwrap();
        let result = c
            .rules
            .evaluate(
                &SchemaSubject::Definition(reward.address()),
                &key("resistance-contribution"),
                &[],
                c.staged.schema(),
                &mut scratch,
            )
            .unwrap();
        let EffectDisposition::Applied {
            value: ParameterValue::Quantity(amount),
        } = &result.effects[0].disposition
        else {
            panic!("known finite reward contribution");
        };
        assert_eq!(amount.value(), expected);
        assert_eq!(
            matches!(result.owner_programs_closure, SchemaClosure::Complete),
            row["complete_effect_text"].as_bool().unwrap()
        );
    }
}

#[test]
fn obsolete_read_field_and_stale_line_binding_are_rejected() {
    let mut recipe: Value = serde_json::from_slice(&data("recipe.json")).unwrap();
    let node = recipe["rules"]["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"]["value"]["value"]["key"] == "def.00000000000009d6")
        .unwrap()["programs"]["members"][0]["nodes"][0]["expression"]
        .as_object_mut()
        .unwrap();
    let input = node.remove("input").unwrap();
    node.insert("read".into(), input);
    assert!(
        decode_owned_recipe(
            &serde_json::to_vec(&recipe).unwrap(),
            OwnedRecipeLimits::default()
        )
        .is_err()
    );
    let c = Component::load();
    let mut changed = c.items.input().clone();
    changed.version = key("changed-policy-binding");
    let changed =
        OwnedItemLinePolicy::new(changed, c.staged.schema(), ItemLineLimits::default()).unwrap();
    assert!(
        decode_item_source_policy(
            &data("item-source.json"),
            &changed,
            c.staged.schema(),
            ItemSourceLimits::default(),
        )
        .is_err()
    );
}

#[test]
fn signed_ranges_use_literal_source_interpolation_and_half_offset_rounding() {
    let c = Component::load();
    for (text, fraction, expected) in [
        ("+(20-30)% to Cold Resistance", 0.0, 20.0),
        ("+(20-30)% to Cold Resistance", 0.5, 25.0),
        ("+(20-30)% to Cold Resistance", 1.0, 30.0),
        ("+(-3--2)% to Cold Resistance", 0.5, -3.0),
        ("(2-3)% to all Elemental Resistances", 0.5, 3.0),
        // Stable convex interpolation instead rounds to zero for this case.
        ("+(-3-2)% to Cold Resistance", 0.7, 1.0),
    ] {
        let properties: BTreeMap<_, _> = PROPERTY_LABELS
            .iter()
            .map(|label| (key(label), false))
            .collect();
        let converted = c
            .items
            .convert_lines([ItemLineInput {
                index: 1,
                text,
                range_fraction: Some(fraction),
                properties: Some(&properties),
            }])
            .unwrap();
        let ItemLineOutcome::Known { emissions, .. } = &converted.lines[0].outcome else {
            panic!("one lexical ranged match for {text}");
        };
        let [ConvertedItemEmission::Modifier { rolls, .. }] = emissions.as_slice() else {
            panic!("one modifier");
        };
        let ParameterValue::Quantity(value) = &rolls[0].value else {
            panic!("quantity");
        };
        assert_eq!(value.value(), expected, "{text}, fraction={fraction}");
        assert_eq!(rolls.len(), 6);
        assert!(
            rolls[1..]
                .iter()
                .all(|roll| roll.value == ParameterValue::Boolean(false))
        );
        assert!(matches!(
            c.items
                .convert_lines([ItemLineInput {
                    index: 1,
                    text,
                    range_fraction: None,
                    properties: Some(&properties),
                }])
                .unwrap()
                .lines[0]
                .outcome,
            ItemLineOutcome::Pending { .. }
        ));
        // A bare caller range does not prove the source property set.
        assert!(matches!(
            c.items
                .convert_line(1, text, Some(fraction))
                .unwrap()
                .outcome,
            ItemLineOutcome::Pending { .. }
        ));
    }
}
const PROPERTY_LABELS: [&str; 5] = [
    "cold_resistance",
    "elemental_resistance",
    "elemental",
    "cold",
    "resistance",
];

fn original_five() -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap()
}
fn imported(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([73; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn ring_plan(c: &Component, xml: &str) -> ItemRangeAttribution {
    let source = imported(xml);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let records: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| {
            row.occurrence().name() == "Item"
                && row.attribute("id").and_then(|a| a.decoded().ok()) == Some("26")
        })
        .collect();
    assert_eq!(records.len(), 1);
    c.layout
        .attribute(&evidence, records[0].occurrence().id(), &c.items)
        .unwrap()
}
fn ring_range_copy(xml: &str, fraction: &str, extra_line: bool) -> String {
    let source = imported(xml);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let item = evidence
        .rows()
        .iter()
        .find(|row| {
            row.occurrence().name() == "Item"
                && row.attribute("id").and_then(|a| a.decoded().ok()) == Some("26")
        })
        .unwrap();
    let span = item.occurrence().range();
    let original = &xml[span.clone()];
    assert_eq!(original.matches("range=\"0.5\"").count(), 2);
    let mut changed = original.replace("range=\"0.5\"", &format!("range=\"{fraction}\""));
    // Test-only range edit preserves the original property annotation verbatim.
    const TAG: &str = "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}";
    assert_eq!(changed.matches(TAG).count(), 1);
    if extra_line {
        const COLD: &str = "{range:0.5}+(20-30)% to Cold Resistance";
        assert_eq!(changed.matches(COLD).count(), 1);
        changed = changed.replacen(COLD, &format!("Unconverted inserted modifier\n{COLD}"), 1);
    }
    let mut copy = xml.to_owned();
    copy.replace_range(span, &changed);
    copy
}
#[test]
fn actual_tagged_original_ring_preserves_eight_uses_and_nominal_properties_without_scaling() {
    let c = Component::load();
    let xml = original_five();
    let source = imported(&xml);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let uses: BTreeSet<_> = evidence
        .rows()
        .iter()
        .filter(|row| {
            row.occurrence().name() == "Slot"
                && row.attribute("itemId").and_then(|a| a.decoded().ok()) == Some("26")
        })
        .map(|row| row.occurrence().id())
        .collect();
    assert_eq!(uses.len(), 8);
    let plan = ring_plan(&c, &xml);
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Proven));
    let raw_members: Vec<_> = plan
        .report()
        .lines
        .iter()
        .filter(|l| {
            l.raw.contains("+(20-30)% to Cold Resistance") || l.raw.trim() == "+10 to maximum Life"
        })
        .collect();
    assert_eq!(raw_members.len(), 2);
    assert!(
        raw_members[0]
            .raw
            .contains("{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}")
    );
    assert!(raw_members[0].blockers.is_empty());
    assert_eq!(plan.report().writes.len(), 3);
    let cold: ModifierDefId = c.id("nominal-cold-modifier");
    let converted = plan.convert(&c.items).unwrap();
    assert_eq!(converted.modifiers.len(), 2);
    assert!(matches!(
        plan.report().default_scope,
        ItemSourceDefaultScope::Proven { .. }
    ));
    assert!(
        converted.parameters.is_empty(),
        "the original authored no catalyst headers"
    );
    assert_eq!(converted.defaults.parameters.len(), 2);
    assert_eq!(
        converted.defaults.parameters[0].slot,
        c.id("item-catalyst-kind")
    );
    assert_eq!(
        converted.defaults.parameters[0].value,
        ParameterValue::Option(c.id("catalyst-none"))
    );
    assert_eq!(
        converted.defaults.parameters[1].slot,
        c.id("item-catalyst-enabled-amount")
    );
    let ParameterValue::Quantity(default_amount) = &converted.defaults.parameters[1].value else {
        panic!("default catalyst amount");
    };
    assert_eq!(default_amount.value(), 20.0);
    assert!(converted.defaults.item_level_absent);
    assert!(converted.defaults.quality_absent);
    let modifier = converted
        .modifiers
        .iter()
        .find(|m| m.definition == cold)
        .unwrap();
    assert_eq!(modifier.rolls.len(), 6);
    let ParameterValue::Quantity(amount) = &modifier.rolls[0].value else {
        panic!("nominal amount");
    };
    assert_eq!(amount.value(), 25.0);
    for (roll, property) in modifier.rolls[1..].iter().zip(PROPERTY_LABELS) {
        assert_eq!(
            roll.slot,
            c.id(&format!("nominal-cold-property-{property}"))
        );
        assert_eq!(roll.value, ParameterValue::Boolean(true));
    }
    let life: ModifierDefId = c.id("flat-life-modifier");
    assert!(converted.modifiers.iter().any(|m| m.definition == life));
    let owner = c
        .staged
        .rules()
        .input()
        .owners
        .iter()
        .find(|o| o.owner == SchemaSubject::Definition(cold.address()))
        .unwrap();
    assert!(owner.programs.members.is_empty());
    assert!(matches!(
        owner.programs.closure,
        SchemaClosure::Partial { .. }
    ));
    assert_eq!(original_five(), xml);
}

#[test]
fn tagged_ring_range_edits_preserve_properties_and_exact_xml_attribution() {
    let c = Component::load();
    let xml = original_five();
    let cold: ModifierDefId = c.id("nominal-cold-modifier");
    for (fraction, expected) in [("0", 20.0), ("0.5", 25.0), ("1", 30.0)] {
        let copy = ring_range_copy(&xml, fraction, false);
        let plan = ring_plan(&c, &copy);
        assert!(
            matches!(plan.report().layout, ItemLayoutStatus::Proven),
            "layout={:?}; blocked lines={:?}",
            plan.report().layout,
            plan.report()
                .lines
                .iter()
                .filter(|l| !l.blockers.is_empty())
                .map(|l| (l.index, &l.raw, &l.rule, &l.blockers))
                .collect::<Vec<_>>()
        );
        assert!(
            plan.report()
                .lines
                .iter()
                .any(|l| l.raw.contains("{range:0.5}"))
        );
        let converted = plan.convert(&c.items).unwrap();
        assert_eq!(converted.modifiers.len(), 2);
        let modifier = converted
            .modifiers
            .iter()
            .find(|m| m.definition == cold)
            .unwrap();
        let ParameterValue::Quantity(amount) = &modifier.rolls[0].value else {
            panic!("cold quantity");
        };
        assert_eq!(amount.value(), expected);
        assert_eq!(modifier.rolls.len(), 6);
        assert!(
            modifier.rolls[1..]
                .iter()
                .all(|r| r.value == ParameterValue::Boolean(true))
        );
        assert!(matches!(converted.item_level, ItemField::Absent));
        assert!(matches!(converted.quality, ItemField::Absent));
    }
    // Edited XML fractions prove nominal conversion, not effective scaling or whole-build success.
    assert_eq!(original_five(), xml);
    assert!(c.staged.manifest().partial_rule_owners > 0);
}
#[test]
fn unknown_inserted_source_member_blocks_the_known_ring_range() {
    let c = Component::load();
    let copy = ring_range_copy(&original_five(), "0.5", true);
    let plan = ring_plan(&c, &copy);
    assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
    let cold: ModifierDefId = c.id("nominal-cold-modifier");
    let converted = plan.convert(&c.items).unwrap();
    assert!(!converted.modifiers.iter().any(|m| m.definition == cold));
}

#[test]
fn unknown_or_unconsumed_property_labels_cannot_be_stripped_into_fixed_modifiers() {
    let c = Component::load();
    let xml = original_five();
    const TAG: &str = "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}";
    for copy in [
        xml.replacen(TAG, "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance,unreviewed_property}", 1),
        xml.replacen("+(20-30)% to Cold Resistance", "+25% to Cold Resistance", 1),
    ] {
        let plan = ring_plan(&c, &copy);
        assert!(matches!(plan.report().layout, ItemLayoutStatus::Pending(_)));
        let converted = plan.convert(&c.items).unwrap();
        let nominal: ModifierDefId = c.id("nominal-cold-modifier");
        let fixed: ModifierDefId = c.id("flat-cold-modifier");
        assert!(!converted.modifiers.iter().any(|m| m.definition == nominal || m.definition == fixed));
    }
    assert_eq!(original_five(), xml);
}

const CATALYSTS: [(&str, &str); 13] = [
    ("Flesh", "life"),
    ("Neural", "mana"),
    ("Carapace", "defence"),
    ("Uul-Netol's", "physical"),
    ("Xoph's", "fire"),
    ("Tul's", "cold"),
    ("Esh's", "lightning"),
    ("Chayula's", "chaos"),
    ("Reaver", "attack"),
    ("Sibilant", "caster"),
    ("Skittering", "speed"),
    ("Adaptive", "attribute"),
    ("Necrotic", "minion"),
];

fn ring_with_headers(xml: &str, headers: &str) -> String {
    let source = imported(xml);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let item = evidence
        .rows()
        .iter()
        .find(|row| {
            row.occurrence().name() == "Item"
                && row.attribute("id").and_then(|a| a.decoded().ok()) == Some("26")
        })
        .unwrap();
    let span = item.occurrence().range();
    let original = &xml[span.clone()];
    assert_eq!(original.matches("Implicits: 1").count(), 1);
    let changed = original.replacen("Implicits: 1", &format!("{headers}\nImplicits: 1"), 1);
    let mut copy = xml.to_owned();
    copy.replace_range(span, &changed);
    copy
}

#[test]
fn canonical_catalyst_headers_cover_exact_owned_options_and_decimal_amounts() {
    let c = Component::load();
    for (token, label) in CATALYSTS {
        let raw = format!("Catalyst: {token}");
        let line = c.items.convert_line(1, &raw, None).unwrap();
        let ItemLineOutcome::Known { emissions, .. } = line.outcome else {
            panic!("known catalyst {token}");
        };
        let [ConvertedItemEmission::ItemParameter { assignment }] = emissions.as_slice() else {
            panic!("one catalyst parameter");
        };
        assert_eq!(assignment.slot, c.id("item-catalyst-kind"));
        assert_eq!(
            assignment.value,
            ParameterValue::Option(c.id(&format!("catalyst-{label}")))
        );
    }
    for (text, expected) in [("0", 0.0), ("0.7", 0.7), ("-2.5", -2.5), ("20", 20.0)] {
        let raw = format!("CatalystQuality: {text}");
        let line = c.items.convert_line(1, &raw, None).unwrap();
        let ItemLineOutcome::Known { emissions, .. } = line.outcome else {
            panic!("known decimal {text}");
        };
        let [ConvertedItemEmission::ItemParameter { assignment }] = emissions.as_slice() else {
            panic!("one amount parameter");
        };
        let ParameterValue::Quantity(value) = &assignment.value else {
            panic!("quantity");
        };
        assert_eq!(value.value(), expected);
        assert_eq!(assignment.slot, c.id("item-catalyst-enabled-amount"));
    }
    for raw in [
        "Catalyst: Unknown",
        "Catalyst: None",
        "Catalyst: Tul's Catalyst",
        "Catalyst: tul's",
        "CatalystQuality: NaN",
        "CatalystQuality: 1e2",
        "CatalystQuality: bad",
        "CatalystQuality: 1000001",
    ] {
        assert!(
            matches!(
                c.items.convert_line(1, raw, None).unwrap().outcome,
                ItemLineOutcome::Pending { .. }
            ),
            "{raw}"
        );
    }
}

#[test]
fn source_catalyst_defaults_preserve_authored_zero_and_keep_ordinary_headers_separate() {
    let c = Component::load();
    let xml = original_five();
    for (headers, selection, amount, authored, defaulted) in [
        ("Catalyst: Tul's", "cold", 20.0, 1, 1),
        ("Catalyst: Tul's\nCatalystQuality: 0", "cold", 0.0, 2, 0),
        ("CatalystQuality: 0", "none", 0.0, 1, 1),
        ("Catalyst: Xoph's\nCatalystQuality: 0.7", "fire", 0.7, 2, 0),
        ("Quality: 20", "none", 20.0, 0, 2),
        ("Item Level: 77", "none", 20.0, 0, 2),
    ] {
        let copy = ring_with_headers(&xml, headers);
        let plan = ring_plan(&c, &copy);
        assert!(
            matches!(
                plan.report().default_scope,
                ItemSourceDefaultScope::Proven { .. }
            ),
            "{headers}: {:?}",
            plan.report().default_scope
        );
        let converted = plan.convert(&c.items).unwrap();
        assert_eq!(converted.parameters.len(), authored, "{headers}");
        assert_eq!(converted.defaults.parameters.len(), defaulted, "{headers}");
        let all: Vec<_> = converted
            .parameters
            .iter()
            .map(|p| &p.assignment)
            .chain(&converted.defaults.parameters)
            .collect();
        assert_eq!(all.len(), 2);
        let kind = all
            .iter()
            .find(|p| p.slot == c.id("item-catalyst-kind"))
            .unwrap();
        assert_eq!(
            kind.value,
            ParameterValue::Option(c.id(&format!("catalyst-{selection}")))
        );
        let value = all
            .iter()
            .find(|p| p.slot == c.id("item-catalyst-enabled-amount"))
            .unwrap();
        let ParameterValue::Quantity(value) = &value.value else {
            panic!("quantity");
        };
        assert_eq!(value.value(), amount);
        assert_eq!(
            converted.defaults.quality_absent,
            !headers.starts_with("Quality:")
        );
        assert_eq!(
            converted.defaults.item_level_absent,
            !headers.starts_with("Item Level:")
        );
        let cold: ModifierDefId = c.id("nominal-cold-modifier");
        let modifier = converted
            .modifiers
            .iter()
            .find(|m| m.definition == cold)
            .unwrap();
        let ParameterValue::Quantity(nominal) = &modifier.rolls[0].value else {
            panic!("nominal");
        };
        assert_eq!(nominal.value(), 25.0, "headers do not calculate scaling");
    }
    assert_eq!(original_five(), xml);
}

#[test]
fn unproved_malformed_and_duplicate_headers_do_not_turn_into_defaults() {
    let c = Component::load();
    let xml = original_five();
    for headers in [
        "Catalyst: Unknown",
        "CatalystQuality: bad",
        "Catalyst: Tul's\nCatalyst: Tul's",
        "CatalystQuality: 0\nCatalystQuality: 20",
        "Quality (Cold Modifiers): 20%",
        "UnreviewedHeader: 5",
    ] {
        let copy = ring_with_headers(&xml, headers);
        let plan = ring_plan(&c, &copy);
        assert!(
            !matches!(
                plan.report().default_scope,
                ItemSourceDefaultScope::Proven { .. }
            ),
            "{headers}"
        );
        let converted = plan.convert(&c.items).unwrap();
        assert!(converted.defaults.parameters.is_empty(), "{headers}");
        assert!(!converted.defaults.quality_absent);
        assert!(!converted.defaults.item_level_absent);
    }
    assert_eq!(original_five(), xml);
}
