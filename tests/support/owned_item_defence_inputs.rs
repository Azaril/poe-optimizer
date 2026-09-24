//! Canonical defence magnitudes precede local/global projection and item completion.
use super::{
    scalar_families::{
        assert_qualified_raw, assert_raw, attribute, check_scalar_numeric, check_scalar_originals,
        check_scalar_publication, check_scalar_structure, modifier_id, recipe,
    },
    support::{data, json},
};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, LoadoutScope, ParameterValue},
    owned_definitions::*,
    owned_draft::{DraftField, DraftLimits, DraftListCompletion, decode_draft},
    owned_rules::{RuleEffectKind, RuleEntity, RuleProgram, RuleReadSource},
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineInput, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceProblem},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

struct FamilySpec {
    name: &'static str,
    stats: &'static [&'static str],
    label: &'static str,
    increase: bool,
    percentage: bool,
}

// Independent source-semantic expectations: a compound remains one physical
// modifier. Pair names and the triple expansion are not interchangeable.
fn specifications() -> Vec<FamilySpec> {
    let ordinary: [(&str, &[&str], &str, &str); 8] = [
        ("armour", &["Armour"], "Armour", "Armour"),
        ("evasion", &["Evasion"], "Evasion Rating", "Evasion Rating"),
        (
            "energy-shield",
            &["EnergyShield"],
            "maximum Energy Shield",
            "Energy Shield",
        ),
        ("ward", &["Ward"], "maximum Runic Ward", "Runic Ward"),
        (
            "armour-evasion",
            &["ArmourAndEvasion"],
            "Armour and Evasion Rating",
            "Armour and Evasion",
        ),
        (
            "armour-energy-shield",
            &["ArmourAndEnergyShield"],
            "Armour and Energy Shield",
            "Armour and Energy Shield",
        ),
        (
            "evasion-energy-shield",
            &["EvasionAndEnergyShield"],
            "Evasion Rating and Energy Shield",
            "Evasion and Energy Shield",
        ),
        (
            "triple-defence",
            &["Armour", "Evasion", "EnergyShield"],
            "Armour, Evasion and Energy Shield",
            "Armour, Evasion and Energy Shield",
        ),
    ];
    let names = [
        ("armour-base", "armour-increase"),
        ("evasion-base", "evasion-increase"),
        ("energy-shield-base", "energy-shield-increase"),
        ("ward-base", "ward-increase"),
        ("armour-evasion-base", "armour-evasion-increase"),
        ("armour-energy-shield-base", "armour-energy-shield-increase"),
        (
            "evasion-energy-shield-base",
            "evasion-energy-shield-increase",
        ),
        ("triple-defence-base", "triple-defence-increase"),
    ];
    let mut result = Vec::new();
    for ((_, stats, flat, increase), (base_name, increased_name)) in ordinary.into_iter().zip(names)
    {
        result.push(FamilySpec {
            name: base_name,
            stats,
            label: flat,
            increase: false,
            percentage: false,
        });
        result.push(FamilySpec {
            name: increased_name,
            stats,
            label: increase,
            increase: true,
            percentage: true,
        });
    }
    result.push(FamilySpec {
        name: "defences-increase",
        stats: &["Defences"],
        label: "Defences",
        increase: true,
        percentage: true,
    });
    result.push(FamilySpec {
        name: "block-chance-base",
        stats: &["BlockChance"],
        label: "Chance to Block",
        increase: false,
        percentage: true,
    });
    result.push(FamilySpec {
        name: "block-chance-increase",
        stats: &["BlockChance"],
        label: "Block chance",
        increase: true,
        percentage: true,
    });
    result
}

fn legacy(spec: &FamilySpec) -> bool {
    matches!(
        spec.name,
        "armour-evasion-base"
            | "armour-energy-shield-base"
            | "triple-defence-base"
            | "defences-increase"
    )
}
fn suffix(spec: &FamilySpec, negative: bool) -> String {
    if spec.increase {
        format!(
            "% {} {}",
            if negative { "reduced" } else { "increased" },
            spec.label
        )
    } else if spec.percentage {
        format!("% {}", spec.label)
    } else {
        format!(" to {}", spec.label)
    }
}

fn family<'a>(bindings: &'a Value, spec: &FamilySpec) -> &'a Value {
    bindings["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["family"] == spec.name)
        .unwrap()
}

fn parameter<'a>(modifier: &'a LocatedItemModifier, slot: &Value) -> &'a ParameterValue {
    let slot: DeclaredSlot<ParameterSlotDefId> = serde_json::from_value(slot.clone()).unwrap();
    &modifier
        .rolls
        .iter()
        .find(|roll| roll.slot == slot)
        .unwrap()
        .value
}

fn assert_text(modifier: &LocatedItemModifier, binding: &Value, text: &str) {
    let spec = specifications()
        .into_iter()
        .find(|spec| binding["family"] == spec.name)
        .unwrap();
    // Check both new lexical facts before reusing the common scalar assertions.
    let mut common = modifier.clone();
    if legacy(&spec) {
        let decimal: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(binding["decimal_quantization_input"].clone()).unwrap();
        let present: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(binding["base_rounding_present_input"].clone()).unwrap();
        assert_eq!(
            parameter(modifier, &binding["decimal_quantization_input"]),
            &ParameterValue::Boolean(text.split_whitespace().next().unwrap().contains('.'))
        );
        assert_eq!(
            parameter(modifier, &binding["base_rounding_present_input"]),
            &ParameterValue::Boolean(false)
        );
        assert_eq!(modifier.rolls.len(), if spec.increase { 26 } else { 25 });
        common
            .rolls
            .retain(|roll| roll.slot != decimal && roll.slot != present);
    }
    let modifier = &common;
    if spec.increase {
        let (number, reduced) = if let Some(number) = text.strip_suffix(&suffix(&spec, false)) {
            (number, false)
        } else {
            (text.strip_suffix(&suffix(&spec, true)).unwrap(), true)
        };
        let number: f64 = number.parse().unwrap();
        assert_qualified_raw(
            modifier,
            binding,
            number.abs(),
            number.is_sign_negative() ^ reduced,
        );
    } else {
        assert_raw(
            modifier,
            binding,
            text.strip_suffix(&suffix(&spec, false))
                .unwrap()
                .parse()
                .unwrap(),
        );
    }
}

fn check_structure(before: &OwnedRecipeInput, after: &OwnedRecipeInput, bindings: &Value) {
    let specs = specifications();
    assert_eq!(specs.len(), 19);
    let families = bindings["families"].as_array().unwrap();
    assert_eq!(families.len(), 19);
    assert_eq!(
        families
            .iter()
            .map(|row| row["family"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        specs.iter().map(|spec| spec.name).collect()
    );
    assert_eq!(before.registry.last_issued.get(), 10_754);
    assert_eq!(after.registry.last_issued.get(), 11_228);
    check_scalar_structure(
        before,
        after,
        bindings,
        &json(data().join("item-defence-inputs/membership-patch.json")),
        474,
        455,
        19,
    );
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    for spec in &specs {
        let binding = family(bindings, spec);
        assert_eq!(binding["source_stats"], json!(spec.stats));
        assert_eq!(
            binding["source_mod_type"],
            if spec.increase { "INC" } else { "BASE" }
        );
        assert_eq!(binding["source_scope"], "unqualified");
        assert_eq!(binding["positive_suffix"], suffix(spec, false));
        assert_eq!(binding["negative_suffix"], suffix(spec, true));
        assert_eq!(
            binding["precision"],
            if legacy(spec) { Value::Null } else { json!(1) }
        );
        assert_eq!(
            binding["display_precision"],
            if legacy(spec) { Value::Null } else { json!(0) }
        );
        let unit: UnitDefId = serde_json::from_value(binding["unit"].clone()).unwrap();
        let output: StatDefId = serde_json::from_value(binding["output"].clone()).unwrap();
        assert_eq!(
            unit.key().as_str(),
            if spec.percentage {
                "def.0000000000000002"
            } else {
                "def.000000000000295a"
            }
        );
        assert_eq!(
            output.key().as_str(),
            if spec.percentage {
                "def.000000000000253e"
            } else {
                "def.000000000000295b"
            }
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
                .any(|row| row.address() == output.address())
        );
        let SchemaLookup::Known(unit_schema) = checked.schema().definition(&unit) else {
            panic!("known quantity unit")
        };
        assert_eq!(
            unit_schema.dimension,
            if spec.percentage {
                UnitDimension::PercentagePoints
            } else {
                UnitDimension::Count
            }
        );
        let SchemaLookup::Known(stat_schema) = checked.schema().definition(&output) else {
            panic!("known effective quantity")
        };
        assert_eq!(stat_schema.targets, [RuleEntityKind::Modifier]);
        assert_eq!(
            stat_schema.value,
            ComputedValueType::Quantity { unit: unit.clone() }
        );
        let id = modifier_id(binding);
        let SchemaLookup::Known(owner) = checked.schema().definition(&id) else {
            panic!("canonical modifier")
        };
        assert_eq!(
            owner.declarations.parameters.members.len(),
            (if spec.increase { 24 } else { 23 }) + if legacy(spec) { 2 } else { 0 }
        );
        assert!(matches!(
            owner.declarations.parameters.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(
            binding["formatter"],
            if legacy(spec) {
                "fixed_numeric_legacy_component"
            } else {
                "normal_numeric_component"
            }
        );
        for field in ["decimal_quantization_input", "base_rounding_present_input"] {
            if legacy(spec) {
                let slot: DeclaredSlot<ParameterSlotDefId> =
                    serde_json::from_value(binding[field].clone()).unwrap();
                let SchemaLookup::Known(schema) = checked.schema().slot(&slot) else {
                    panic!("required lexical input")
                };
                assert_eq!(schema.value, ValueSchema::Boolean);
                assert_eq!(schema.presence, SlotPresence::RequiredOnce);
                assert_eq!(schema.sites, [ParameterSite::ModifierRoll]);
            } else {
                assert!(binding[field].is_null());
            }
        }
        let amount: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(binding["canonical_inputs"]["amount"].clone()).unwrap();
        let SchemaLookup::Known(amount_schema) = checked.schema().slot(&amount) else {
            panic!("raw amount")
        };
        let ValueSchema::Quantity(range) = &amount_schema.value else {
            panic!("quantity range")
        };
        assert_eq!(
            range.minimum.value(),
            if spec.increase { 0.0 } else { -1e6 }
        );
        assert_eq!(range.maximum.value(), 1e6);
        assert_eq!(range.minimum.unit(), &unit);
        assert_eq!(range.maximum.unit(), &unit);
        assert_eq!(amount_schema.presence, SlotPresence::RequiredOnce);
        assert_eq!(amount_schema.sites, [ParameterSite::ModifierRoll]);
        if spec.increase {
            let negative: DeclaredSlot<ParameterSlotDefId> =
                serde_json::from_value(binding["negative_input"].clone()).unwrap();
            let SchemaLookup::Known(schema) = checked.schema().slot(&negative) else {
                panic!("required sign input")
            };
            assert_eq!(schema.value, ValueSchema::Boolean);
            assert_eq!(schema.presence, SlotPresence::RequiredOnce);
            assert_eq!(schema.sites, [ParameterSite::ModifierRoll]);
        } else {
            assert!(binding["negative_input"].is_null());
        }
    }
}

fn check_import(lines: &OwnedItemLinePolicy, source: &ItemSourceLayoutPolicy, bindings: &Value) {
    for spec in specifications() {
        let binding = family(bindings, &spec);
        let properties: BTreeMap<OwnedDefinitionKey, bool> = binding["property_inputs"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| key.as_str() != "unscalable")
            .map(|key| (key.parse().unwrap(), false))
            .collect();
        let convert = |text: &str, properties| {
            lines
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
                        properties,
                    },
                ])
                .unwrap()
                .modifiers
        };
        for negative in [false, true] {
            // The raw grammar records signed/fractional values. Current source
            // conditions admit only the separately reviewed integer syntax.
            for (token, admitted) in [
                ("0", true),
                ("10", true),
                ("1000000", true),
                ("10.0", false),
                ("10.00", false),
                ("10.25", false),
                ("-10", false),
                ("-0", false),
            ] {
                let token = if !spec.increase && !token.starts_with('-') {
                    format!("+{token}")
                } else {
                    token.to_owned()
                };
                if !spec.increase && negative {
                    continue;
                }
                let text = format!("{token}{}", suffix(&spec, negative));
                let raw = convert(&text, Some(&properties));
                assert_eq!(raw.len(), 1, "{}: {text}", spec.name);
                assert_text(&raw[0], binding, &text);
                let attributed_plan = attribute(&text, source, lines);
                let attributed = attributed_plan.convert(lines).unwrap();
                assert_eq!(
                    attributed.modifiers.len(),
                    usize::from(admitted),
                    "{}: source {text}",
                    spec.name
                );
                if admitted {
                    assert_text(&attributed.modifiers[0], binding, &text);
                }
            }
        }
        let token = if spec.increase { "10" } else { "+10" };
        let normal = format!("{token}{}", suffix(&spec, false));
        assert!(
            convert(&normal, None).is_empty(),
            "absent property facts are not false"
        );
        let mut incomplete = properties.clone();
        incomplete.remove(&"armour".parse().unwrap());
        assert!(convert(&normal, Some(&incomplete)).is_empty());
        let mut explicit = properties.clone();
        explicit.insert("armour".parse().unwrap(), true);
        let raw = convert(&normal, Some(&explicit));
        assert_eq!(raw.len(), 1);
        assert_eq!(
            parameter(&raw[0], &binding["property_inputs"]["armour"]),
            &ParameterValue::Boolean(true)
        );
        for prefix in [
            "{tags:armour}",
            "{tags:defences}",
            "{crafted}",
            "{fractured}",
            "{desecrated}",
            "{rune}",
            "{corruptedRange:1}",
            "unreviewed prefix\n",
        ] {
            let text = format!("{prefix}{normal}");
            let plan = attribute(&text, source, lines);
            assert!(
                plan.convert(lines).unwrap().modifiers.is_empty(),
                "physical tag or unreviewed source: {text}"
            );
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
        for text in [
            format!("{normal} per Level"),
            format!("{normal} while Moving"),
            format!("{normal} extra"),
            format!(
                "{}1000001{}",
                if spec.increase { "" } else { "+" },
                suffix(&spec, false)
            ),
            format!(
                "{}1e2{}",
                if spec.increase { "" } else { "+" },
                suffix(&spec, false)
            ),
            format!(
                "{}(10-20){}",
                if spec.increase { "" } else { "+" },
                suffix(&spec, false)
            ),
        ] {
            assert!(
                attribute(&text, source, lines)
                    .convert(lines)
                    .unwrap()
                    .modifiers
                    .is_empty(),
                "unreviewed grammar {text}"
            );
        }
        let duplicate_plan = attribute(&format!("{normal}\n{normal}"), source, lines);
        let duplicate = duplicate_plan.convert(lines).unwrap();
        assert_eq!(
            duplicate.modifiers.len(),
            2,
            "compound source expansion must not duplicate or collapse physical occurrences"
        );
        assert_ne!(duplicate.modifiers[0].line, duplicate.modifiers[1].line);
        assert!(
            duplicate
                .modifiers
                .iter()
                .all(|row| row.definition == modifier_id(binding))
        );
    }
    for text in [
        "10% increased maximum Energy Shield",
        "10% more Global Evasion Rating and Energy Shield",
        "Has +3 to Evasion Rating per player level",
        "Has +1 to maximum Energy Shield per player level",
        "Has no Energy Shield",
    ] {
        assert!(
            attribute(text, source, lines)
                .convert(lines)
                .unwrap()
                .modifiers
                .is_empty(),
            "separate effect or global-tag contract: {text}"
        );
    }
}
fn check_policies(prior: &Path, output: &Path, bindings: &Value) {
    let specs = specifications();
    let mut added = BTreeSet::new();
    let mut admitted = BTreeSet::new();
    for spec in &specs {
        let binding = family(bindings, spec);
        let positive = format!("fixed-{}", spec.name);
        let negative = format!(
            "{}-{}",
            if spec.increase {
                "reduced"
            } else {
                "fixed-minus"
            },
            spec.name
        );
        assert_eq!(binding["source_positive_rule"], positive);
        assert_eq!(binding["source_negative_rule"], negative);
        added.insert(positive.clone());
        added.insert(negative.clone());
        admitted.insert(positive);
        if spec.increase {
            admitted.insert(negative);
        }
    }
    assert_eq!(added.len(), 38);
    assert_eq!(admitted.len(), 29);
    let old = json(prior.join("items.json"));
    let mut next = json(output.join("items.json"));
    assert_eq!(old["schema_version"], 5);
    assert_eq!(next["schema_version"], 6);
    let rows = next["rules"].as_array_mut().unwrap();
    assert_eq!(rows.len(), old["rules"].as_array().unwrap().len() + 38);
    assert_eq!(
        rows.iter()
            .filter(|row| added.contains(row["id"].as_str().unwrap()))
            .count(),
        38
    );
    rows.retain(|row| !added.contains(row["id"].as_str().unwrap()));
    for field in ["schema_version", "version", "definitions"] {
        next[field] = old[field].clone();
    }
    assert_eq!(next, old, "unrelated line grammars changed");
    let old = json(prior.join("item-source.json"));
    let mut next = json(output.join("item-source.json"));
    let rows = next["rule_layouts"].as_array_mut().unwrap();
    assert_eq!(
        rows.len(),
        old["rule_layouts"].as_array().unwrap().len() + 38
    );
    for row in rows
        .iter()
        .filter(|row| added.contains(row["rule"].as_str().unwrap()))
    {
        assert_eq!(row["role"], "unresolved");
    }
    rows.retain(|row| !added.contains(row["rule"].as_str().unwrap()));
    let conditions =
        next["dialect"]["pob_exported_single_text_observations_v1"]["single_modifier_conditions"]
            .as_array_mut()
            .unwrap();
    assert_eq!(
        conditions.len(),
        old["dialect"]["pob_exported_single_text_observations_v1"]["single_modifier_conditions"]
            .as_array()
            .unwrap()
            .len()
            + 29
    );
    for id in &added {
        let matches: Vec<_> = conditions.iter().filter(|row| row["rule"] == *id).collect();
        assert_eq!(
            matches.len(),
            usize::from(admitted.contains(id)),
            "exact source admission for {id}"
        );
        if let Some(row) = matches.first() {
            assert_eq!(
                row["all"],
                json!([
                    {"kind":"no_source_tags"}, {"kind":"no_generated_buff_members"},
                    {"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
                ])
            );
        }
    }
    conditions.retain(|row| !added.contains(row["rule"].as_str().unwrap()));
    for field in ["version", "item_lines"] {
        next[field] = old[field].clone();
    }
    assert_eq!(
        next, old,
        "unrelated source metadata or admission conditions changed"
    );
}

fn check_numeric(after: &OwnedRecipeInput, bindings: &Value) {
    for (increase, percentage) in [(false, false), (false, true), (true, true)] {
        let specs = specifications();
        let selected: Vec<_> = specs
            .iter()
            .filter(|spec| {
                !legacy(spec) && spec.increase == increase && spec.percentage == percentage
            })
            .map(|spec| family(bindings, spec).clone())
            .collect();
        assert_eq!(
            selected.len(),
            if increase {
                9
            } else if percentage {
                1
            } else {
                5
            }
        );
        let unit = &selected[0]["unit"];
        let stat = &selected[0]["output"];
        check_scalar_numeric(
            after,
            &selected,
            unit,
            stat,
            &[
                (0.0, 1.0, 1.0, 0.0),
                (10.0, 1.0, 1.0, 10.0),
                (10.0, 2.0, 1.5, 30.0),
                (10.49, 1.0, 1.0, 10.0),
                (10.5, 1.0, 1.0, 11.0),
                (3.0, 1.0, 1.2, 3.0),
                (10.0, 1.0, 1.0, 10.0),
            ],
            increase.then_some(false),
        );
        if increase {
            check_scalar_numeric(
                after,
                &selected,
                unit,
                stat,
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
                after,
                &selected,
                unit,
                stat,
                &[(-10.0, 2.0, 1.5, -28.0), (-10.5, 1.0, 1.0, -11.0)],
                None,
            );
        }
    }
}

fn numeric_fact(program: &RuleProgram, name: &str, amount: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|read| read.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("typed numeric input")
    };
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(amount, unit.clone()).unwrap()),
    }
}
fn boolean_fact(program: &RuleProgram, slot: &Value, value: bool) -> RuleFact {
    let slot: DeclaredSlot<ParameterSlotDefId> = serde_json::from_value(slot.clone()).unwrap();
    let read = program.reads.iter().find(|read| matches!(&read.source, RuleReadSource::Parameter { slot: actual } if actual == &slot)).unwrap();
    assert_eq!(read.value_type, ComputedValueType::Boolean);
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Boolean(value),
    }
}

fn check_legacy_numeric(after: &OwnedRecipeInput, bindings: &Value) {
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let specs: Vec<_> = specifications().into_iter().filter(legacy).collect();
    assert_eq!(specs.len(), 4);
    for spec in &specs {
        let binding = family(bindings, spec);
        let owner_id = SchemaSubject::Definition(modifier_id(binding).address());
        let owner = after
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == owner_id)
            .unwrap();
        assert_eq!(owner.programs.members.len(), 4);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        for program in &owner.programs.members {
            assert_eq!(program.effects.len(), 1);
            assert!(program.effects.iter().all(|effect| matches!(
                &effect.effect,
                RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    ..
                }
            )));
            let result = compiled
                .evaluate(&owner_id, &program.id, &[], checked.schema(), &mut scratch)
                .unwrap();
            assert!(matches!(
                result.effects[0].disposition,
                EffectDisposition::Unresolved { .. }
            ));
            assert_eq!(result.owner_programs_closure, owner.programs.closure);
        }
        let program = owner
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == "effective-amount")
            .unwrap();
        let output: StatDefId = serde_json::from_value(binding["output"].clone()).unwrap();
        let unit: UnitDefId = serde_json::from_value(binding["unit"].clone()).unwrap();
        assert!(
            matches!(&program.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat, .. } if stat == &output)
        );
        // Hand-selected expectations from ItemTools.applyRange/applyValueScalar.
        // Decimal lexical syntax is distinct from integral numeric value. An
        // explicit base=1 rounds before scaling; an absent base does not.
        for (raw, decimal, base_present, base, magnitude, expected) in [
            (0.0, false, false, 1.0, 1.0, 0.0),
            (10.0, false, false, 1.0, 1.0, 10.0),
            (10.5, true, false, 1.0, 1.0, 10.5),
            (10.5, true, true, 1.0, 1.0, 10.5),
            (10.5, true, false, 99.0, 1.0, 10.5),
            (0.15, true, false, 1.0, 1.5, 0.2),
            (0.15, true, true, 1.0, 1.5, 0.3),
            (3.0, false, false, 1.0, 1.25, 3.0),
            (3.0, true, false, 1.0, 1.25, 3.7),
            (10.0, false, false, 1.0, 1.29995, 13.0),
            (10.0, true, false, 1.0, 1.29995, 12.9),
            (1.0, false, true, 1.49, 2.0, 2.0),
            (1.0, false, true, 1.5, 2.0, 4.0),
            (3.0, false, true, 2.0, 1.5, 9.0),
            (10.0, false, false, 1.0, 1.0, 10.0),
        ] {
            for negative in [false, true] {
                let component = if negative && !spec.increase {
                    -raw
                } else {
                    raw
                };
                let expected = if negative { -expected } else { expected };
                let mut facts = vec![
                    numeric_fact(program, "component", component),
                    boolean_fact(program, &binding["decimal_quantization_input"], decimal),
                    boolean_fact(
                        program,
                        &binding["base_rounding_present_input"],
                        base_present,
                    ),
                    numeric_fact(program, "corruption-factor", base),
                    numeric_fact(program, "magnitude-factor", magnitude),
                ];
                if spec.increase {
                    facts.push(boolean_fact(program, &binding["negative_input"], negative));
                }
                assert_eq!(program.reads.len(), facts.len());
                let result = compiled
                    .evaluate(
                        &owner_id,
                        &program.id,
                        &facts,
                        checked.schema(),
                        &mut scratch,
                    )
                    .unwrap();
                let assert_value = |disposition: &EffectDisposition| {
                    let EffectDisposition::Applied {
                        value: ParameterValue::Quantity(value),
                    } = disposition
                    else {
                        panic!(
                            "{}: {component}/{decimal}/{base_present}/{base}/{magnitude}",
                            spec.name
                        )
                    };
                    assert_eq!(value.unit(), &unit);
                    assert_eq!(
                        value.value(),
                        expected,
                        "{}: {component}/{decimal}/{base_present}/{base}/{magnitude}",
                        spec.name
                    );
                };
                assert_value(&result.effects[0].disposition);
                for omitted in 0..facts.len() {
                    let reduced: Vec<_> = facts
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index != omitted)
                        .map(|(_, fact)| fact.clone())
                        .collect();
                    let result = compiled
                        .evaluate(
                            &owner_id,
                            &program.id,
                            &reduced,
                            checked.schema(),
                            &mut scratch,
                        )
                        .unwrap();
                    let unchanged = magnitude == 1.0 && (!base_present || base == 1.0);
                    let unused = (omitted == 1 && unchanged) || (omitted == 3 && !base_present);
                    if unused {
                        assert_value(&result.effects[0].disposition);
                    } else {
                        assert!(
                            matches!(
                                result.effects[0].disposition,
                                EffectDisposition::Unresolved { .. }
                            ),
                            "missing live input {} on {}",
                            facts[omitted].read.as_str(),
                            spec.name
                        );
                    }
                }
            }
        }
    }
}
/// Recognition of a preceding physical defence line can remove ambiguity from
/// an existing following modifier without admitting the preceding line itself.
/// These reviewed original-source witnesses are distinct from the 22 new
/// defence occurrences, and retain their old grammar, quantities and owner.
fn check_older_family_gains(cwd: &Path, prior: &Path, bindings: &Value) {
    let expected = [
        (
            1,
            298,
            19,
            "+39% to Fire Resistance",
            "fixed-fire",
            "def.000000000000292a",
            39.0,
            "62% increased Energy Shield",
            "fixed-energy-shield-increase",
            false,
        ),
        (
            1,
            305,
            12,
            "14% increased Rarity of Items found",
            "fixed-rarity",
            "def.00000000000029bc",
            14.0,
            "+54 to maximum Energy Shield",
            "fixed-energy-shield-base",
            true,
        ),
        (
            1,
            314,
            12,
            "+28% to Cold Resistance",
            "fixed-cold",
            "def.0000000000002542",
            28.0,
            "+217 to Armour",
            "fixed-armour-base",
            true,
        ),
        (
            3,
            272,
            11,
            "14% increased Rarity of Items found",
            "fixed-rarity",
            "def.00000000000029bc",
            14.0,
            "45% increased Armour",
            "fixed-armour-increase",
            false,
        ),
        (
            3,
            295,
            18,
            "+37% to Cold Resistance",
            "fixed-cold",
            "def.0000000000002542",
            37.0,
            "97% increased Evasion Rating",
            "fixed-evasion-increase",
            false,
        ),
        (
            3,
            316,
            17,
            "+25% to Fire Resistance",
            "fixed-fire",
            "def.000000000000292a",
            25.0,
            "89% increased Energy Shield",
            "fixed-energy-shield-increase",
            false,
        ),
        (
            4,
            201,
            23,
            "+40% to Fire Resistance",
            "fixed-fire",
            "def.000000000000292a",
            40.0,
            "94% increased Armour and Evasion",
            "fixed-armour-evasion-increase",
            false,
        ),
        (
            4,
            211,
            18,
            "+34% to Fire Resistance",
            "fixed-fire",
            "def.000000000000292a",
            34.0,
            "+68 to Evasion Rating",
            "fixed-evasion-base",
            false,
        ),
        (
            5,
            517,
            11,
            "+28 to Strength",
            "fixed-strength",
            "def.000000000000295c",
            28.0,
            "+164 to Evasion Rating",
            "fixed-evasion-base",
            true,
        ),
        (
            5,
            543,
            17,
            "+36% to Cold Resistance",
            "fixed-cold",
            "def.0000000000002542",
            36.0,
            "41% increased Energy Shield",
            "fixed-energy-shield-increase",
            false,
        ),
    ];
    let checked = assemble_owned_recipe(recipe(prior), Default::default()).unwrap();
    let old_lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(prior.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut seen = BTreeSet::new();
    let mut gained = [0; 5];
    for case in 1..=5 {
        let old = json(cwd.join(format!("defence-profile-original-{case}/sidecar.json")));
        let new = json(cwd.join(format!("item-defence-inputs-original-{case}/sidecar.json")));
        for item in new["item_texts"].as_array().unwrap() {
            let old_item = old["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["source"] == item["source"])
                .unwrap();
            let ordinal = item["source"]["ordinal"].as_u64().unwrap();
            for line in item["lines"].as_array().unwrap() {
                let previous = old_item["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["index"] == line["index"])
                    .unwrap();
                if !previous["modifiers"].as_array().unwrap().is_empty()
                    || line["modifiers"].as_array().unwrap().is_empty()
                {
                    continue;
                }
                let emissions: Vec<_> = line["outcome"]["value"]["emissions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|row| row["kind"] == "modifier")
                    .collect();
                assert_eq!(emissions.len(), 1);
                let emission = &emissions[0]["value"];
                if bindings["families"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|family| family["canonical"] == emission["definition"])
                {
                    continue;
                }
                let index = line["index"].as_u64().unwrap();
                let witness = expected
                    .iter()
                    .find(|row| (row.0, row.1, row.2) == (case, ordinal, index))
                    .unwrap_or_else(|| {
                        panic!("unexpected older-family gain {case}/{ordinal}/{index}")
                    });
                assert!(seen.insert((case, ordinal, index)));
                gained[case - 1] += 1;
                assert_eq!(line["text"], witness.3);
                assert_eq!(previous["text"], line["text"]);
                assert_eq!(line["outcome"]["value"]["rule"], witness.4);
                assert_eq!(previous["outcome"]["kind"], "pending");
                assert_eq!(
                    previous["outcome"]["value"]["candidates"],
                    json!([witness.4])
                );
                assert_eq!(emission["definition"]["key"], witness.5);
                assert_eq!(line["modifiers"].as_array().unwrap().len(), 1);
                let previous_attribution = old_item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["index"] == index)
                    .unwrap();
                let attribution = item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["index"] == index)
                    .unwrap();
                assert_eq!(previous_attribution["rule"], witness.4);
                assert!(previous_attribution["member"].is_null());
                assert_eq!(
                    previous_attribution["blockers"],
                    if witness.4 == "fixed-strength" {
                        json!(["possible_combined_line"])
                    } else {
                        json!(["possible_combined_line", "unknown_header"])
                    }
                );
                assert_eq!(attribution["rule"], witness.4);
                assert_eq!(attribution["blockers"], json!([]));
                assert_eq!(attribution["property_tokens"], json!([]));
                assert_eq!(attribution["member"]["line"], index);
                assert_eq!(
                    attribution["member"]["category"],
                    if witness.4 == "fixed-strength" {
                        "explicit"
                    } else {
                        "implicit"
                    }
                );
                let properties: BTreeMap<OwnedDefinitionKey, bool> =
                    serde_json::from_value(attribution["properties"].clone()).unwrap();
                assert_eq!(properties.len(), 20);
                assert!(properties.values().all(|value| !value));
                // Reuse the predecessor's unchanged raw grammar with the
                // recovered source property facts; no new family supplies this
                // modifier's owner or numeric meaning.
                let raw = old_lines
                    .convert_lines([
                        ItemLineInput {
                            index: 1,
                            text: "Sapphire Ring",
                            range_fraction: None,
                            properties: None,
                        },
                        ItemLineInput {
                            index: index as usize,
                            text: witness.3,
                            range_fraction: None,
                            properties: Some(&properties),
                        },
                    ])
                    .unwrap();
                assert_eq!(raw.modifiers.len(), 1);
                assert_eq!(
                    serde_json::to_value(&raw.modifiers[0].definition).unwrap(),
                    emission["definition"]
                );
                assert_eq!(
                    serde_json::to_value(&raw.modifiers[0].rolls).unwrap(),
                    emission["rolls"]
                );
                assert_eq!(
                    serde_json::to_value(&raw.modifiers[0].rolls_closure).unwrap(),
                    emission["rolls_closure"]
                );
                let quantity_unit = if witness.4 == "fixed-strength" {
                    "def.000000000000295a"
                } else {
                    "def.0000000000000002"
                };
                let quantities: Vec<_> = raw.modifiers[0]
                    .rolls
                    .iter()
                    .filter_map(|roll| match &roll.value {
                        ParameterValue::Quantity(value)
                            if value.unit().key().as_str() == quantity_unit =>
                        {
                            Some(value.value())
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(quantities, [witness.6]);
                let neighbor = item["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["index"] == index - 1)
                    .unwrap();
                let old_neighbor = old_item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["index"] == index - 1)
                    .unwrap();
                let neighbor_attribution = item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["index"] == index - 1)
                    .unwrap();
                assert_eq!(neighbor["text"], witness.7);
                assert!(old_neighbor["rule"].is_null());
                assert!(
                    old_neighbor["blockers"]
                        .as_array()
                        .unwrap()
                        .contains(&json!("unknown_member"))
                );
                assert_eq!(neighbor_attribution["rule"], witness.8);
                assert_eq!(
                    neighbor["outcome"]["kind"],
                    if witness.9 { "pending" } else { "known" }
                );
                assert_eq!(
                    neighbor["modifiers"].as_array().unwrap().len(),
                    usize::from(!witness.9)
                );
                if witness.9 {
                    assert!(neighbor_attribution["member"].is_null());
                    assert!(
                        neighbor_attribution["blockers"]
                            .as_array()
                            .unwrap()
                            .contains(&json!("possible_combined_line"))
                    );
                }
            }
        }
    }
    assert_eq!(seen.len(), expected.len());
    assert_eq!(gained, [3, 0, 3, 2, 2]);
}
fn check_original_input_counts(cwd: &Path) {
    let mut shared = Vec::new();
    let (mut complete, mut pending) = (0, 0);
    for case in 1..=5 {
        let path = cwd.join(format!("item-defence-inputs-original-{case}/draft.json"));
        let draft = decode_draft(&fs::read(path).unwrap(), DraftLimits::default()).unwrap();
        shared.push(draft.input().skills.members.len());
        assert!(draft.input().skills.members.iter().all(|skill| matches!(
            skill.scope,
            DraftField::Known {
                value: LoadoutScope::Shared
            }
        )));
        for gem in &draft.input().gems.members {
            match gem.parameters.completion {
                DraftListCompletion::Complete => complete += 1,
                DraftListCompletion::Pending { .. } => pending += 1,
            }
        }
    }
    assert_eq!(shared, [9, 63, 9, 13, 46]);
    assert_eq!((complete, pending), (0, 478));
}

pub fn check_item_defence_inputs(cwd: &Path, prior: &Path) -> PathBuf {
    check_scalar_publication(
        cwd,
        prior,
        "item-defence-inputs",
        "item-defence-inputs",
        474,
        33_364,
        |output, bindings| {
            let before = recipe(prior);
            let after = recipe(output);
            check_structure(&before, &after, bindings);
            check_policies(prior, output, bindings);
            check_numeric(&after, bindings);
            check_legacy_numeric(&after, bindings);
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
                bindings["families"].as_array().unwrap(),
                "defence-profile",
                "item-defence-inputs",
                assert_text,
            );
            assert_eq!(counts.preserved, 64);
            assert_eq!(counts.displays, 53);
            assert_eq!(counts.gained, 22);
            assert_eq!(counts.by_original, [3, 3, 5, 4, 7]);
            assert_eq!(counts.total_by_original, [13, 32, 13, 17, 21]);
            assert_eq!(counts.total_by_original.iter().sum::<usize>(), 96);
            assert_eq!(
                counts.by_family,
                BTreeMap::from([
                    ("armour-base".to_owned(), 1),
                    ("armour-evasion-increase".to_owned(), 1),
                    ("armour-increase".to_owned(), 1),
                    ("energy-shield-base".to_owned(), 4),
                    ("energy-shield-increase".to_owned(), 8),
                    ("evasion-base".to_owned(), 4),
                    ("evasion-energy-shield-increase".to_owned(), 1),
                    ("evasion-increase".to_owned(), 2),
                ])
            );
            check_older_family_gains(cwd, prior, bindings);
            check_original_input_counts(cwd);
        },
    )
}
