//! Attribute inputs retain distinct owners and Count units without claiming Actor coverage.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::digest_owned,
    owned_definitions::*,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_rules::{RuleEffectKind, RuleEntity, RuleProgram},
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemLineInput, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceProblem},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_membership_patch::{RecipeMembershipPatch, RecipeMembershipPatchInput},
    owned_source::SourceProjectEvidence,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}

fn publish(cwd: &Path, prior: &Path, authored: &Path, patch: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(authored.join("extension.json"))
        .arg("--membership-patch")
        .arg(patch)
        .arg("--items")
        .arg(authored.join("items.json"))
        .arg("--item-source")
        .arg(authored.join("item-source.json"))
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

fn modifier_id(family: &Value) -> ModifierDefId {
    serde_json::from_value(family["canonical"].clone()).unwrap()
}

fn label(family: &Value) -> &'static str {
    match family["family"].as_str().unwrap() {
        "strength" => "Strength",
        "dexterity" => "Dexterity",
        "intelligence" => "Intelligence",
        "all-attributes" => "all Attributes",
        other => panic!("unexpected attribute family {other}"),
    }
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

fn assert_raw(modifier: &LocatedItemModifier, family: &Value, amount: f64) {
    assert_eq!(modifier.definition, modifier_id(family));
    let SchemaClosure::Partial { gaps } = &modifier.rolls_closure else {
        panic!("raw attributes cannot close eligibility")
    };
    assert!(
        gaps.iter()
            .any(|gap| gap.code.as_str() == "modifier-eligibility-inputs-unconverted")
    );
    let ParameterValue::Quantity(value) =
        parameter(modifier, &family["canonical_inputs"]["amount"])
    else {
        panic!("attribute amount must be a typed quantity")
    };
    assert_eq!(value.value(), amount);
    assert_eq!(serde_json::to_value(value.unit()).unwrap(), family["unit"]);
    let ParameterValue::Quantity(base) = parameter(modifier, &family["corrupted_base_input"])
    else {
        panic!("corrupted-base factor")
    };
    assert_eq!(base.value(), 1.0);
    for slot in family["property_inputs"].as_object().unwrap().values() {
        assert_eq!(parameter(modifier, slot), &ParameterValue::Boolean(false));
    }
    assert_eq!(modifier.rolls.len(), 23);
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
    assert_eq!(
        after.registry.entries.len(),
        before.registry.entries.len() + 98
    );
    assert_eq!(
        &after.registry.entries[..before.registry.entries.len()],
        before.registry.entries
    );
    let slots: BTreeMap<_, _> = after
        .schema
        .slots
        .iter()
        .map(|row| (row.address(), row))
        .collect();
    assert_eq!(slots.len(), before.schema.slots.len() + 92);
    assert_eq!(slots.len(), after.schema.slots.len());
    for prior in &before.schema.slots {
        assert_eq!(slots[&prior.address()], prior, "prior slot changed");
    }
    let patch: RecipeMembershipPatchInput = serde_json::from_value(patch.clone()).unwrap();
    let mut targets = BTreeSet::new();
    for row in patch.patches {
        let RecipeMembershipPatch::ItemTemplateModifiers { templates, add } = row;
        assert_eq!(add.into_iter().collect::<BTreeSet<_>>(), modifiers);
        for template in templates {
            assert!(targets.insert(template), "duplicate template patch");
        }
    }
    assert_eq!(targets.len(), 1756);
    let definitions: BTreeMap<_, _> = after
        .schema
        .definitions
        .iter()
        .map(|row| (row.address(), row))
        .collect();
    assert_eq!(definitions.len(), before.schema.definitions.len() + 6);
    for prior in &before.schema.definitions {
        let current = definitions[&prior.address()];
        let DefinitionDescriptor::ItemTemplate(old) = prior else {
            assert_eq!(current, prior, "existing definition changed");
            continue;
        };
        if !targets.contains(&old.id) {
            assert_eq!(current, prior);
            continue;
        }
        let DefinitionDescriptor::ItemTemplate(new) = current else {
            panic!("template kind")
        };
        let (SchemaState::Known(old), SchemaState::Known(new)) = (&old.schema, &new.schema) else {
            panic!("known template")
        };
        assert!(matches!(
            new.modifiers.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(new.modifiers.members.len(), old.modifiers.members.len() + 4);
        assert!(
            modifiers
                .iter()
                .all(|id| new.modifiers.members.contains(id))
        );
        let mut restored = new.clone();
        restored
            .modifiers
            .members
            .retain(|id| !modifiers.contains(id));
        assert_eq!(
            &restored, old,
            "membership expansion changed another template fact"
        );
    }
    assert_eq!(after.rules.owners.len(), before.rules.owners.len() + 4);
    assert!(
        before
            .rules
            .owners
            .iter()
            .all(|owner| after.rules.owners.contains(owner))
    );
    assert_eq!(
        before.rules.receivers, after.rules.receivers,
        "Actor receivers changed"
    );
    assert_eq!(before.rules.tables, after.rules.tables);
    assert_eq!(
        before.rules.operations_version,
        after.rules.operations_version
    );
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);
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

fn attribute(
    body: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> poe_optimizer_import::owned_item_source::ItemRangeAttribution {
    let text = format!("Rarity: RARE\nAttribute Fixture\nSapphire Ring\nImplicits: 0\n{body}")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml =
        format!("<PathOfBuilding2><Items><Item id=\"7\">{text}</Item></Items></PathOfBuilding2>");
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([101; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let item = evidence
        .rows()
        .iter()
        .find(|row| row.occurrence().name() == "Item")
        .unwrap();
    source
        .attribute(&evidence, item.occurrence().id(), lines)
        .unwrap()
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

fn quantity(program: &RuleProgram, name: &str, amount: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|read| read.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("Quantity read")
    };
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(amount, unit.clone()).unwrap()),
    }
}

fn check_numeric(after: &OwnedRecipeInput, bindings: &Value) {
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let unit: UnitDefId = serde_json::from_value(bindings["attribute_unit"].clone()).unwrap();
    let stat: StatDefId = serde_json::from_value(bindings["attribute_stat"].clone()).unwrap();
    for family in bindings["families"].as_array().unwrap() {
        let subject = SchemaSubject::Definition(modifier_id(family).address());
        let owner = after
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == subject)
            .unwrap();
        assert_eq!(owner.programs.members.len(), 4);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        for program in &owner.programs.members {
            assert!(
                program.effects.iter().all(|effect| matches!(
                    effect.effect,
                    RuleEffectKind::Derive {
                        entity: RuleEntity::Modifier,
                        ..
                    }
                )),
                "attribute components cannot contribute to Actor channels"
            );
            let result = compiled
                .evaluate(&subject, &program.id, &[], checked.schema(), &mut scratch)
                .unwrap();
            assert!(matches!(
                result.effects[0].disposition,
                EffectDisposition::Unresolved { .. }
            ));
            assert_eq!(result.owner_programs_closure, owner.programs.closure);
        }
        let effective = owner
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == "effective-amount")
            .unwrap();
        assert!(
            matches!(&effective.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat: output, .. } if output == &stat)
        );
        for (raw, base, magnitude, expected) in [
            (0.0, 1.0, 1.0, 0.0),
            (10.0, 1.0, 1.0, 10.0),
            (10.0, 2.0, 1.5, 30.0),
            (-10.0, 2.0, 1.5, -28.0),
            (10.49, 1.0, 1.0, 10.0),
            (10.5, 1.0, 1.0, 11.0),
            (-10.5, 1.0, 1.0, -11.0),
            (3.0, 1.0, 1.2, 3.0),
        ] {
            let facts = [
                quantity(effective, "component", raw),
                quantity(effective, "corruption-factor", base),
                quantity(effective, "magnitude-factor", magnitude),
            ];
            let result = compiled
                .evaluate(
                    &subject,
                    &effective.id,
                    &facts,
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(value),
            } = &result.effects[0].disposition
            else {
                panic!("effective attribute quantity")
            };
            assert_eq!(value.unit(), &unit);
            assert_eq!(
                value.value(),
                expected,
                "{}: {raw}/{base}/{magnitude}",
                label(family)
            );
            for omitted in 0..facts.len() {
                let incomplete: Vec<_> = facts
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != omitted)
                    .map(|(_, fact)| fact.clone())
                    .collect();
                let result = compiled
                    .evaluate(
                        &subject,
                        &effective.id,
                        &incomplete,
                        checked.schema(),
                        &mut scratch,
                    )
                    .unwrap();
                assert!(matches!(
                    result.effects[0].disposition,
                    EffectDisposition::Unresolved { .. }
                ));
            }
        }
    }
}

fn check_originals(cwd: &Path, prior: &Path, output: &Path, bindings: &Value) {
    let families = bindings["families"].as_array().unwrap();
    let mut preserved = 0;
    let mut displays = 0;
    let mut gained = 0;
    let mut summary = Vec::new();
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior.join(&query)).unwrap(),
            fs::read(output.join(&query)).unwrap()
        );
        assert_eq!(json(output.join(query)).as_array().unwrap().len(), 22);
        let destination = cwd.join(format!("item-attributes-original-{case}"));
        assert_eq!(
            success(normalize(cwd, output, case, &destination, true))["normalization_status"],
            "pending"
        );
        let before = json(cwd.join(format!("elemental-resistance-original-{case}/sidecar.json")));
        let after = json(destination.join("sidecar.json"));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(before[field], after[field]);
        }
        let mut attribute_count = 0;
        for item in after["item_texts"].as_array().unwrap() {
            let old = before["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|old| old["source"] == item["source"])
                .unwrap();
            for row in item["lines"].as_array().unwrap() {
                let previous = old["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|old| old["index"] == row["index"])
                    .unwrap();
                assert_eq!(row["text"], previous["text"]);
                let prior_modifiers = previous["modifiers"].as_array().unwrap();
                if !prior_modifiers.is_empty() {
                    preserved += prior_modifiers.len();
                    assert_eq!(
                        row["outcome"], previous["outcome"],
                        "prior source occurrence or rolls changed"
                    );
                    assert_eq!(
                        row["modifiers"].as_array().unwrap().len(),
                        prior_modifiers.len()
                    );
                }
                if previous["outcome"]["kind"] == "known"
                    && previous["outcome"]["value"]["rule"]
                        .as_str()
                        .is_some_and(|rule| rule.starts_with("observed-"))
                {
                    displays += 1;
                    assert_eq!(row["outcome"], previous["outcome"]);
                }
                if row["modifiers"].as_array().unwrap().is_empty() {
                    continue;
                }
                for (index, emission) in row["outcome"]["value"]["emissions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                {
                    if emission["kind"] != "modifier" {
                        continue;
                    }
                    let value = &emission["value"];
                    let Some(family) = families
                        .iter()
                        .find(|family| family["canonical"] == value["definition"])
                    else {
                        continue;
                    };
                    assert!(prior_modifiers.is_empty());
                    let raw = row["text"].as_str().unwrap();
                    let amount: f64 = raw
                        .strip_prefix('+')
                        .unwrap()
                        .strip_suffix(&format!(" to {}", label(family)))
                        .unwrap()
                        .parse()
                        .unwrap();
                    assert_raw(
                        &LocatedItemModifier {
                            line: row["index"].as_u64().unwrap() as usize,
                            emission: index,
                            definition: serde_json::from_value(value["definition"].clone())
                                .unwrap(),
                            rolls: serde_json::from_value(value["rolls"].clone()).unwrap(),
                            rolls_closure: serde_json::from_value(value["rolls_closure"].clone())
                                .unwrap(),
                        },
                        family,
                        amount,
                    );
                    assert_eq!(
                        row["modifiers"].as_array().unwrap().len(),
                        1,
                        "all Attributes is one occurrence"
                    );
                    let attribution = item["attribution"]["lines"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|line| line["index"] == row["index"])
                        .unwrap();
                    assert!(attribution["blockers"].as_array().unwrap().is_empty());
                    assert!(!attribution["member"].is_null());
                    attribute_count += 1;
                }
            }
        }
        gained += attribute_count;
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let input = draft.input();
        assert_eq!(
            input
                .query_presets
                .members
                .iter()
                .map(|preset| preset.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|row| matches!(row.access, DraftAllocationAccess::Pending(_)))
        );
        for item in &input.items.members {
            assert!(matches!(
                item.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                item.modifiers.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(item.modifier_order, DraftField::Pending(_)));
            assert!(item.to_resolved().is_none());
            for modifier in &item.modifiers.members {
                if matches!(&modifier.definition, DraftField::Known { value } if families.iter().any(|family| modifier_id(family) == *value))
                {
                    assert!(matches!(
                        modifier.rolls.completion,
                        DraftListCompletion::Pending { .. }
                    ));
                }
            }
        }
        summary.push(serde_json::json!({"original":case,"attribute_modifiers":attribute_count,"queries":22,"normalization":"pending","whole_build_parity":"not_established"}));
    }
    assert_eq!(preserved, 34);
    assert_eq!(displays, 53);
    assert!(
        gained >= 2,
        "the original plain Intelligence rows must become usable raw inputs"
    );
    fs::write(
        cwd.join("item-attributes-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}

pub fn check_item_attributes(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("item-attribute-inputs");
    let authored_before = bundle(&authored);
    let prior_before = bundle(prior);
    let patch = authored.join("membership-patch.json");
    let output = cwd.join("item-attributes-successor");
    let report = success(publish(cwd, prior, &authored, &patch, &output));
    assert_eq!(report["extension"]["allocated_entries"], 98);
    assert_eq!(report["extension"]["refined_subjects"], 1756);
    assert_eq!(report["membership_patch"]["patched_templates"], 1756);
    assert_eq!(report["membership_patch"]["inserted_members"], 7024);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    for file in ["items.json", "item-source.json"] {
        assert_eq!(json(output.join(file)), json(authored.join(file)));
    }
    let bindings = json(authored.join("bindings.json"));
    let before = recipe(prior);
    let after = recipe(&output);
    check_structure(&before, &after, &bindings, &json(&patch));
    check_numeric(&after, &bindings);
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
    check_import(&lines, &source, &bindings);
    check_originals(cwd, prior, &output, &bindings);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &authored, &patch, &output)
            .status
            .success()
    );
    assert!(bundle(&output) == published, "published bytes changed");
    let replay = cwd.join("item-attributes-replay");
    assert_eq!(
        success(publish(cwd, prior, &authored, &patch, &replay)),
        report
    );
    assert!(bundle(&replay) == published, "replay differs");
    for field in ["before", "extension"] {
        let mut stale = json(&patch);
        let digest =
            serde_json::to_value(digest_owned("stale-item-attributes-v1", &1_u32, 1024).unwrap())
                .unwrap();
        if field == "before" {
            stale["before"]["rules"] = digest;
        } else {
            stale["extension"] = digest;
        }
        let path = cwd.join(format!("item-attributes-stale-{field}.json"));
        fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
        let rejected = cwd.join(format!("item-attributes-rejected-{field}"));
        assert!(
            !publish(cwd, prior, &authored, &path, &rejected)
                .status
                .success()
        );
        assert!(!rejected.exists());
    }
    assert!(bundle(prior) == prior_before, "prior bundle changed");
    assert!(
        bundle(&authored) == authored_before,
        "authored bytes changed"
    );
    output
}
