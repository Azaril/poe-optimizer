//! Attribute inputs retain distinct owners and Count units without claiming Actor coverage.
use super::{
    scalar_families::{
        assert_raw, attribute, check_scalar_numeric, check_scalar_originals,
        check_scalar_publication, check_scalar_structure, modifier_id, recipe,
    },
    support::{data, json},
};
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineInput, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceProblem},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
fn label(family: &Value) -> &'static str {
    match family["family"].as_str().unwrap() {
        "strength" => "Strength",
        "dexterity" => "Dexterity",
        "intelligence" => "Intelligence",
        "all-attributes" => "all Attributes",
        other => panic!("unexpected attribute family {other}"),
    }
}

fn check_structure(
    before: &OwnedRecipeInput,
    after: &OwnedRecipeInput,
    bindings: &Value,
    patch: &Value,
) {
    let families = bindings["families"].as_array().unwrap();
    assert_eq!(families.len(), 4);
    let expected = BTreeMap::from([
        ("strength", vec!["Str"]),
        ("dexterity", vec!["Dex"]),
        ("intelligence", vec!["Int"]),
        ("all-attributes", vec!["Str", "Dex", "Int", "All"]),
    ]);
    let mut actual_families = BTreeMap::new();
    let modifiers: BTreeSet<_> = families.iter().map(modifier_id).collect();
    assert_eq!(
        modifiers.len(),
        4,
        "compound attributes must have their own owner"
    );
    let unit: UnitDefId = serde_json::from_value(bindings["attribute_unit"].clone()).unwrap();
    let stat: StatDefId = serde_json::from_value(bindings["attribute_stat"].clone()).unwrap();
    assert!(
        !before
            .schema
            .definitions
            .iter()
            .any(|row| row.address() == unit.address())
    );
    assert!(
        !before
            .schema
            .definitions
            .iter()
            .any(|row| row.address() == stat.address())
    );
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let SchemaLookup::Known(unit_schema) = checked.schema().definition(&unit) else {
        panic!("new Count unit")
    };
    assert_eq!(unit_schema.dimension, UnitDimension::Count);
    let SchemaLookup::Known(stat_schema) = checked.schema().definition(&stat) else {
        panic!("new Modifier output")
    };
    assert_eq!(stat_schema.targets, [RuleEntityKind::Modifier]);
    assert_eq!(
        stat_schema.value,
        ComputedValueType::Quantity { unit: unit.clone() }
    );
    check_scalar_structure(before, after, bindings, patch, 98, 92, 6);
    for family in families {
        assert_eq!(family["unit"], bindings["attribute_unit"]);
        assert_eq!(family["precision"], 1);
        let name = family["family"].as_str().unwrap();
        let names = family["source_stats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(actual_families.insert(name, names).is_none());
        let id = modifier_id(family);
        let SchemaLookup::Known(owner) = checked.schema().definition(&id) else {
            panic!("attribute owner")
        };
        assert_eq!(owner.declarations.parameters.members.len(), 23);
        let amount: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(family["canonical_inputs"]["amount"].clone()).unwrap();
        let SchemaLookup::Known(amount_schema) = checked.schema().slot(&amount) else {
            panic!("typed amount")
        };
        let ValueSchema::Quantity(range) = &amount_schema.value else {
            panic!("Quantity amount")
        };
        assert_eq!(range.minimum.unit(), &unit);
        assert_eq!(range.maximum.unit(), &unit);
        assert_eq!(amount_schema.sites, [ParameterSite::ModifierRoll]);
        assert_eq!(amount_schema.presence, SlotPresence::RequiredOnce);
    }
    assert_eq!(actual_families, expected);
}

fn check_import(lines: &OwnedItemLinePolicy, source: &ItemSourceLayoutPolicy, bindings: &Value) {
    for family in bindings["families"].as_array().unwrap() {
        let label = label(family);
        let properties = family["property_inputs"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| key.as_str() != "unscalable")
            .map(|key| (key.parse().unwrap(), false))
            .collect();
        for (token, amount, admitted) in [
            ("+0", 0.0, true),
            ("+10", 10.0, true),
            ("+1000000", 1e6, true),
            ("+10.25", 10.25, false),
            ("-12.25", -12.25, false),
            ("-0", 0.0, false),
        ] {
            let text = format!("{token} to {label}");
            let raw = lines
                .convert_lines([
                    ItemLineInput {
                        index: 1,
                        text: "Sapphire Ring",
                        range_fraction: None,
                        properties: None,
                    },
                    ItemLineInput {
                        index: 2,
                        text: &text,
                        range_fraction: None,
                        properties: Some(&properties),
                    },
                ])
                .unwrap();
            assert_eq!(
                raw.modifiers.len(),
                1,
                "one compound modifier occurrence, including all Attributes"
            );
            assert_raw(&raw.modifiers[0], family, amount);
            let plan = attribute(&text, source, lines);
            let converted = plan.convert(lines).unwrap();
            assert_eq!(converted.modifiers.len(), usize::from(admitted), "{text}");
            if admitted {
                assert_raw(&converted.modifiers[0], family, amount);
            }
        }
        for body in [
            format!("10 to {label}"),
            format!("+1e2 to {label}"),
            format!("++10 to {label}"),
            format!("+1000001 to {label}"),
            format!("+10% to {label}"),
            format!("+10 to {label} per Level"),
            format!("{{tags:attribute}}+10 to {label}"),
            format!("{{crafted}}+10 to {label}"),
            format!("{{fractured}}+10 to {label}"),
            format!("{{rune}}+10 to {label}"),
            format!("{{corruptedRange:1}}+10 to {label}"),
            format!("unreviewed prefix\n+10 to {label}"),
        ] {
            let plan = attribute(&body, source, lines);
            assert!(plan.convert(lines).unwrap().modifiers.is_empty(), "{body}");
            if body.starts_with("unreviewed") {
                assert!(
                    plan.report()
                        .lines
                        .last()
                        .unwrap()
                        .blockers
                        .contains(&ItemSourceProblem::PossibleCombinedLine)
                );
            }
        }
        let duplicate_plan = attribute(&format!("+10 to {label}\n+10 to {label}"), source, lines);
        let duplicate = duplicate_plan.convert(lines).unwrap();
        assert_eq!(
            duplicate.modifiers.len(),
            2,
            "physical occurrences cannot be deduplicated"
        );
        assert_ne!(duplicate.modifiers[0].line, duplicate.modifiers[1].line);
    }
}

pub fn check_item_attributes(cwd: &Path, prior: &Path) -> PathBuf {
    check_scalar_publication(
        cwd,
        prior,
        "item-attribute-inputs",
        "item-attributes",
        98,
        7024,
        |output, bindings| {
            let before = recipe(prior);
            let after = recipe(output);
            let patch = json(data().join("item-attribute-inputs/membership-patch.json"));
            check_structure(&before, &after, bindings, &patch);
            let families = bindings["families"].as_array().unwrap();
            check_scalar_numeric(
                &after,
                families,
                &bindings["attribute_unit"],
                &bindings["attribute_stat"],
                &[
                    (0.0, 1.0, 1.0, 0.0),
                    (10.0, 1.0, 1.0, 10.0),
                    (10.0, 2.0, 1.5, 30.0),
                    (-10.0, 2.0, 1.5, -28.0),
                    (10.49, 1.0, 1.0, 10.0),
                    (10.5, 1.0, 1.0, 11.0),
                    (-10.5, 1.0, 1.0, -11.0),
                    (3.0, 1.0, 1.2, 3.0),
                ],
                None,
            );
            let checked = assemble_owned_recipe(after, Default::default()).unwrap();
            let lines = OwnedItemLinePolicy::new(
                serde_json::from_value(json(output.join("items.json"))).unwrap(),
                checked.schema(),
                Default::default(),
            )
            .unwrap();
            let source = ItemSourceLayoutPolicy::new(
                serde_json::from_value(json(output.join("item-source.json"))).unwrap(),
                &lines,
                checked.schema(),
                Default::default(),
            )
            .unwrap();
            check_import(&lines, &source, bindings);
            let counts = check_scalar_originals(
                cwd,
                prior,
                output,
                families,
                "elemental-resistance",
                "item-attributes",
                |modifier, family, raw| {
                    let amount = raw
                        .strip_prefix('+')
                        .unwrap()
                        .strip_suffix(&format!(" to {}", label(family)))
                        .unwrap()
                        .parse()
                        .unwrap();
                    assert_raw(modifier, family, amount);
                },
            );
            assert_eq!(counts.preserved, 34);
            assert_eq!(counts.displays, 53);
            assert!(
                counts.gained >= 2,
                "the original plain Intelligence rows must become usable raw inputs"
            );
        },
    )
}
