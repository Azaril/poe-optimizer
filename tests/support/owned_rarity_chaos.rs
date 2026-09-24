//! Rarity qualifier direction stays separate from magnitude; Chaos uses signed values.
use super::{
    scalar_families::{
        assert_qualified_raw, assert_raw, attribute, check_scalar_numeric, check_scalar_originals,
        check_scalar_publication, check_scalar_structure, modifier_id, recipe,
    },
    support::{data, json},
};
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineInput, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceProblem},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

fn assert_text(modifier: &LocatedItemModifier, family: &Value, text: &str) {
    match family["family"].as_str().unwrap() {
        "item-rarity" => {
            let (number, reduced) =
                if let Some(value) = text.strip_suffix("% increased Rarity of Items found") {
                    (value, false)
                } else {
                    (
                        text.strip_suffix("% reduced Rarity of Items found")
                            .unwrap(),
                        true,
                    )
                };
            let number: f64 = number.parse().unwrap();
            // Source capture sign is meaningful even for -0, before the typed
            // quantity canonicalizes zero. Reduced reverses that direction.
            assert_qualified_raw(
                modifier,
                family,
                number.abs(),
                number.is_sign_negative() ^ reduced,
            );
        }
        "chaos-resistance" => assert_raw(
            modifier,
            family,
            text.strip_suffix("% to Chaos Resistance")
                .unwrap()
                .parse()
                .unwrap(),
        ),
        other => panic!("unexpected scalar fixture family {other}"),
    }
}

fn check_structure(before: &OwnedRecipeInput, after: &OwnedRecipeInput, bindings: &Value) {
    let families = bindings["families"].as_array().unwrap();
    assert_eq!(
        families
            .iter()
            .map(|family| family["family"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["item-rarity", "chaos-resistance"])
    );
    check_scalar_structure(
        before,
        after,
        bindings,
        &json(data().join("item-rarity-chaos-inputs/membership-patch.json")),
        49,
        47,
        2,
    );
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let unit: UnitDefId = serde_json::from_value(bindings["scalar_unit"].clone()).unwrap();
    let stat: StatDefId = serde_json::from_value(bindings["effective_stat"].clone()).unwrap();
    let SchemaLookup::Known(unit_schema) = checked.schema().definition(&unit) else {
        panic!("percentage unit")
    };
    assert_eq!(unit_schema.dimension, UnitDimension::PercentagePoints);
    let SchemaLookup::Known(stat_schema) = checked.schema().definition(&stat) else {
        panic!("Modifier scalar output")
    };
    assert_eq!(stat_schema.targets, [RuleEntityKind::Modifier]);
    assert_eq!(
        stat_schema.value,
        ComputedValueType::Quantity { unit: unit.clone() }
    );
    assert!(
        before
            .schema
            .definitions
            .iter()
            .any(|row| row.address() == unit.address())
    );
    assert!(
        before
            .schema
            .definitions
            .iter()
            .any(|row| row.address() == stat.address())
    );
    for family in families {
        let rarity = family["family"] == "item-rarity";
        assert_eq!(family["unit"], bindings["scalar_unit"]);
        assert_eq!(family["precision"], 1);
        assert_eq!(
            family["source_stats"],
            serde_json::json!([if rarity { "LootRarity" } else { "ChaosResist" }])
        );
        let SchemaLookup::Known(owner) = checked.schema().definition(&modifier_id(family)) else {
            panic!("scalar owner")
        };
        assert_eq!(
            owner.declarations.parameters.members.len(),
            if rarity { 24 } else { 23 }
        );
        let amount: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(family["canonical_inputs"]["amount"].clone()).unwrap();
        let SchemaLookup::Known(amount_schema) = checked.schema().slot(&amount) else {
            panic!("amount slot")
        };
        let ValueSchema::Quantity(range) = &amount_schema.value else {
            panic!("amount quantity")
        };
        assert_eq!(range.minimum.unit(), &unit);
        assert_eq!(range.maximum.unit(), &unit);
        assert_eq!(
            range.minimum.value(),
            if rarity { 0.0 } else { -1_000_000.0 }
        );
        assert_eq!(range.maximum.value(), 1_000_000.0);
        assert_eq!(amount_schema.presence, SlotPresence::RequiredOnce);
        assert_eq!(amount_schema.sites, [ParameterSite::ModifierRoll]);
        if rarity {
            let negative: DeclaredSlot<ParameterSlotDefId> =
                serde_json::from_value(family["negative_input"].clone()).unwrap();
            let SchemaLookup::Known(negative_schema) = checked.schema().slot(&negative) else {
                panic!("qualifier slot")
            };
            assert_eq!(negative_schema.value, ValueSchema::Boolean);
            assert_eq!(negative_schema.presence, SlotPresence::RequiredOnce);
            assert_eq!(negative_schema.sites, [ParameterSite::ModifierRoll]);
        } else {
            assert!(family["negative_input"].is_null());
        }
    }
}

fn check_import(lines: &OwnedItemLinePolicy, source: &ItemSourceLayoutPolicy, bindings: &Value) {
    for family in bindings["families"].as_array().unwrap() {
        let rarity = family["family"] == "item-rarity";
        let properties = family["property_inputs"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|name| name.as_str() != "unscalable")
            .map(|name| (name.parse().unwrap(), false))
            .collect();
        let cases: &[(&str, bool)] = if rarity {
            &[
                ("0% increased Rarity of Items found", true),
                ("10% increased Rarity of Items found", true),
                ("10% reduced Rarity of Items found", true),
                ("0% reduced Rarity of Items found", true),
                ("1000000% increased Rarity of Items found", true),
                ("1000000% reduced Rarity of Items found", true),
                ("-10% increased Rarity of Items found", false),
                ("-10% reduced Rarity of Items found", false),
                ("-0% increased Rarity of Items found", false),
                ("-0% reduced Rarity of Items found", false),
                ("10.25% increased Rarity of Items found", false),
                ("-10.25% reduced Rarity of Items found", false),
            ]
        } else {
            &[
                ("+0% to Chaos Resistance", true),
                ("+10% to Chaos Resistance", true),
                ("+1000000% to Chaos Resistance", true),
                ("+10.25% to Chaos Resistance", false),
                ("-12.25% to Chaos Resistance", false),
                ("-0% to Chaos Resistance", false),
            ]
        };
        for &(text, admitted) in cases {
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
                        text,
                        range_fraction: None,
                        properties: Some(&properties),
                    },
                ])
                .unwrap();
            assert_eq!(raw.modifiers.len(), 1, "{text}");
            assert_text(&raw.modifiers[0], family, text);
            let plan = attribute(text, source, lines);
            let converted = plan.convert(lines).unwrap();
            assert_eq!(converted.modifiers.len(), usize::from(admitted), "{text}");
            if admitted {
                assert_text(&converted.modifiers[0], family, text);
            }
        }
        let normal = if rarity {
            "10% increased Rarity of Items found"
        } else {
            "+10% to Chaos Resistance"
        };
        let malformed: &[&str] = if rarity {
            &[
                "+10% increased Rarity of Items found",
                "1e2% increased Rarity of Items found",
                "1000001% increased Rarity of Items found",
                "10% increased Rarity of Items found per Level",
                "10% increased Quantity of Items found",
            ]
        } else {
            &[
                "10% to Chaos Resistance",
                "+1e2% to Chaos Resistance",
                "++10% to Chaos Resistance",
                "+1000001% to Chaos Resistance",
                "+10% to Chaos Resistance per Level",
            ]
        };
        for text in malformed {
            let plan = attribute(text, source, lines);
            assert!(plan.convert(lines).unwrap().modifiers.is_empty(), "{text}");
        }
        for prefix in [
            "{tags:chaos}",
            "{tags:attribute}",
            "{crafted}",
            "{fractured}",
            "{rune}",
            "{corruptedRange:1}",
            "unreviewed prefix\n",
        ] {
            let text = format!("{prefix}{normal}");
            let plan = attribute(&text, source, lines);
            assert!(plan.convert(lines).unwrap().modifiers.is_empty(), "{text}");
            if prefix.starts_with("unreviewed") {
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
        let duplicate_plan = attribute(&format!("{normal}\n{normal}"), source, lines);
        let duplicate = duplicate_plan.convert(lines).unwrap();
        assert_eq!(
            duplicate.modifiers.len(),
            2,
            "physical occurrences remain separate"
        );
        assert_ne!(duplicate.modifiers[0].line, duplicate.modifiers[1].line);
    }
}

fn check_policies(prior: &Path, output: &Path) {
    let old = json(prior.join("items.json"));
    let mut current = json(output.join("items.json"));
    let rules = current["rules"].as_array_mut().unwrap();
    let added = BTreeSet::from([
        "fixed-rarity",
        "reduced-rarity",
        "fixed-chaos",
        "fixed-minus-chaos",
    ]);
    assert_eq!(rules.len(), old["rules"].as_array().unwrap().len() + 4);
    rules.retain(|rule| !added.contains(rule["id"].as_str().unwrap()));
    for field in ["version", "definitions"] {
        current[field] = old[field].clone();
    }
    assert!(current == old, "unrelated item recipes changed");
    let old = json(prior.join("item-source.json"));
    let mut current = json(output.join("item-source.json"));
    let roles = current["rule_layouts"].as_array_mut().unwrap();
    assert_eq!(
        roles.len(),
        old["rule_layouts"].as_array().unwrap().len() + 4
    );
    for role in roles
        .iter()
        .filter(|row| added.contains(row["rule"].as_str().unwrap()))
    {
        assert_eq!(role["role"], "unresolved");
    }
    roles.retain(|role| !added.contains(role["rule"].as_str().unwrap()));
    let conditions = current["dialect"]["pob_exported_single_text_observations_v1"]["single_modifier_conditions"].as_array_mut().unwrap();
    let prior_conditions =
        old["dialect"]["pob_exported_single_text_observations_v1"]["single_modifier_conditions"]
            .as_array()
            .unwrap();
    assert_eq!(conditions.len(), prior_conditions.len() + 3);
    for id in ["fixed-rarity", "reduced-rarity", "fixed-chaos"] {
        let row = conditions.iter().find(|row| row["rule"] == id).unwrap();
        assert_eq!(
            row["all"],
            serde_json::json!([
                {"kind":"no_source_tags"}, {"kind":"no_generated_buff_members"},
                {"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
            ])
        );
    }
    assert!(
        !conditions
            .iter()
            .any(|row| row["rule"] == "fixed-minus-chaos")
    );
    conditions.retain(|row| !added.contains(row["rule"].as_str().unwrap()));
    for field in ["version", "item_lines"] {
        current[field] = old[field].clone();
    }
    assert!(
        current == old,
        "unrelated source conditions or metadata changed"
    );
}

pub fn check_rarity_chaos(cwd: &Path, prior: &Path) -> PathBuf {
    check_scalar_publication(
        cwd,
        prior,
        "item-rarity-chaos-inputs",
        "item-rarity-chaos",
        49,
        3512,
        |output, bindings| {
            let before = recipe(prior);
            let after = recipe(output);
            check_structure(&before, &after, bindings);
            check_policies(prior, output);
            let families = bindings["families"].as_array().unwrap();
            for family in families {
                let rarity = family["family"] == "item-rarity";
                let selected = std::slice::from_ref(family);
                check_scalar_numeric(
                    &after,
                    selected,
                    &bindings["scalar_unit"],
                    &bindings["effective_stat"],
                    &[
                        (0.0, 1.0, 1.0, 0.0),
                        (10.0, 1.0, 1.0, 10.0),
                        (10.0, 2.0, 1.5, 30.0),
                        (10.49, 1.0, 1.0, 10.0),
                        (10.5, 1.0, 1.0, 11.0),
                        (3.0, 1.0, 1.2, 3.0),
                    ],
                    if rarity { Some(false) } else { None },
                );
                if rarity {
                    check_scalar_numeric(
                        &after,
                        selected,
                        &bindings["scalar_unit"],
                        &bindings["effective_stat"],
                        &[
                            (0.0, 1.0, 1.0, 0.0),
                            (10.0, 1.0, 1.0, -10.0),
                            (10.0, 2.0, 1.5, -30.0),
                            (10.5, 1.0, 1.0, -11.0),
                            (3.0, 1.0, 1.2, -3.0),
                        ],
                        Some(true),
                    );
                } else {
                    check_scalar_numeric(
                        &after,
                        selected,
                        &bindings["scalar_unit"],
                        &bindings["effective_stat"],
                        &[(-10.0, 2.0, 1.5, -28.0), (-10.5, 1.0, 1.0, -11.0)],
                        None,
                    );
                }
            }
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
                "item-attributes",
                "item-rarity-chaos",
                assert_text,
            );
            assert_eq!(counts.preserved, 47);
            assert_eq!(counts.displays, 53);
            assert_eq!(counts.by_original.len(), 5);
            assert_eq!(counts.total_by_original.len(), 5);
            assert_eq!(counts.by_original.iter().sum::<usize>(), counts.gained);
            assert_eq!(counts.by_family.values().sum::<usize>(), counts.gained);
            assert!(
                counts.gained > 0,
                "real corpus must exercise the new source families"
            );
        },
    )
}
