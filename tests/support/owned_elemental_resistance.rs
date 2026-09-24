//! Distinct resistance families reuse owned numeric stages without closing eligibility.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::digest_owned,
    owned_definitions::*,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_rules::{RuleEffectKind, RuleEntity, RuleProgram, RuleReadSource},
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemLineInput, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{ItemLoadIndexPrefix, ItemSourceLayoutPolicy, ItemSourceProblem},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_source::SourceProjectEvidence,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}
fn publish(
    cwd: &Path,
    prior: &Path,
    authored: &Path,
    items: &Path,
    source: &Path,
    output: &Path,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(authored.join("extension.json"))
        .arg("--items")
        .arg(items)
        .arg("--item-source")
        .arg(source)
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
        "fire-resistance" => "Fire",
        "lightning-resistance" => "Lightning",
        other => panic!("unexpected authored family {other}"),
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
        panic!("raw facts cannot close eligibility")
    };
    assert!(
        gaps.iter()
            .any(|gap| gap.code.as_str() == "modifier-eligibility-inputs-unconverted")
    );
    let ParameterValue::Quantity(raw) = parameter(modifier, &family["canonical_inputs"]["amount"])
    else {
        panic!("raw amount")
    };
    assert_eq!(raw.value(), amount);
    assert_eq!(serde_json::to_value(raw.unit()).unwrap(), family["unit"]);
    let ParameterValue::Quantity(factor) = parameter(modifier, &family["corrupted_base_input"])
    else {
        panic!("corrupted-base input")
    };
    assert_eq!(factor.value(), 1.0);
    for slot in family["property_inputs"].as_object().unwrap().values() {
        assert_eq!(parameter(modifier, slot), &ParameterValue::Boolean(false));
    }
    assert_eq!(modifier.rolls.len(), 23);
}
fn remap(value: &mut Value, ids: &BTreeMap<String, String>) {
    match value {
        Value::String(value) => {
            if let Some(replacement) = ids.get(value) {
                *value = replacement.clone();
            }
        }
        Value::Array(values) => values.iter_mut().for_each(|value| remap(value, ids)),
        Value::Object(values) => values.values_mut().for_each(|value| remap(value, ids)),
        _ => {}
    }
}
fn id_map(cold: &Value, family: &Value) -> BTreeMap<String, String> {
    let mut ids = BTreeMap::from([(
        cold["canonical"]["key"].as_str().unwrap().into(),
        family["canonical"]["key"].as_str().unwrap().into(),
    )]);
    for field in ["canonical_inputs", "property_inputs"] {
        let source = cold[field].as_object().unwrap();
        let target = family[field].as_object().unwrap();
        assert!(source.keys().eq(target.keys()));
        for (name, slot) in source {
            ids.insert(
                slot["slot"]["key"].as_str().unwrap().into(),
                target[name]["slot"]["key"].as_str().unwrap().into(),
            );
        }
    }
    ids.insert(
        cold["corrupted_base_input"]["slot"]["key"]
            .as_str()
            .unwrap()
            .into(),
        family["corrupted_base_input"]["slot"]["key"]
            .as_str()
            .unwrap()
            .into(),
    );
    assert_eq!(ids.len(), 24);
    ids
}
fn check_structure(before: &OwnedRecipeInput, after: &OwnedRecipeInput, bindings: &Value) {
    let families = bindings["families"].as_array().unwrap();
    assert_eq!(families.len(), 2);
    assert_eq!(before.registry.entries.len(), 10537);
    assert_eq!(after.registry.entries.len(), 10585);
    assert!(
        after.registry.entries[..before.registry.entries.len()] == before.registry.entries,
        "prior registry allocation sequence changed"
    );
    // Schema storage groups slots by kind; new Parameters precede old Choice,
    // Grant and output entries. Preserve each identity, not a storage prefix.
    let slots: BTreeMap<_, _> = after
        .schema
        .slots
        .iter()
        .map(|s| (s.address(), s))
        .collect();
    assert_eq!(
        slots.len(),
        after.schema.slots.len(),
        "duplicate slot identity"
    );
    assert_eq!(slots.len(), before.schema.slots.len() + 46);
    for prior in &before.schema.slots {
        assert!(
            slots
                .get(&prior.address())
                .is_some_and(|current| *current == prior),
            "prior slot changed: {:?}",
            prior.address()
        );
    }
    let targets: BTreeSet<ItemTemplateDefId> = bindings["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| serde_json::from_value(row["template"].clone()).unwrap())
        .collect();
    assert_eq!(targets.len(), 1756);
    assert_eq!(bindings["refined_templates"], 1756);
    let modifiers: Vec<_> = families.iter().map(modifier_id).collect();
    for prior in &before.schema.definitions {
        let DefinitionDescriptor::ItemTemplate(prior) = prior else {
            assert!(after.schema.definitions.contains(prior));
            continue;
        };
        let current = after
            .schema
            .definitions
            .iter()
            .find_map(|row| match row {
                DefinitionDescriptor::ItemTemplate(row) if row.id == prior.id => Some(row),
                _ => None,
            })
            .unwrap();
        if !targets.contains(&prior.id) {
            assert_eq!(current, prior);
            continue;
        }
        let (SchemaState::Known(old), SchemaState::Known(new)) = (&prior.schema, &current.schema)
        else {
            panic!("known base")
        };
        assert_eq!(old.modifiers.closure, new.modifiers.closure);
        assert!(matches!(
            new.modifiers.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(new.modifiers.members.len(), old.modifiers.members.len() + 2);
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
        assert_eq!(&restored, old);
    }
    assert_eq!(after.rules.owners.len(), before.rules.owners.len() + 2);
    assert!(
        before
            .rules
            .owners
            .iter()
            .all(|owner| after.rules.owners.contains(owner))
    );
    assert_eq!(before.rules.tables, after.rules.tables);
    assert_eq!(before.rules.receivers, after.rules.receivers);
    assert_eq!(
        before.rules.operations_version,
        after.rules.operations_version
    );
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);
    let cold_bindings = json(data().join("modifier-value-inputs/bindings.json"));
    let cold = cold_bindings["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|family| family["family"] == "cold-resistance")
        .unwrap();
    let cold_subject = SchemaSubject::Definition(modifier_id(cold).address());
    let cold_programs = before
        .rules
        .owners
        .iter()
        .find(|row| row.owner == cold_subject)
        .unwrap();
    for family in families {
        let replacements = id_map(cold, family);
        let subject = SchemaSubject::Definition(modifier_id(family).address());
        let actual = after
            .rules
            .owners
            .iter()
            .find(|row| row.owner == subject)
            .unwrap();
        let mut expected = serde_json::to_value(cold_programs).unwrap();
        remap(&mut expected, &replacements);
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            expected,
            "program remapping: {}",
            label(family)
        );
        assert_eq!(actual.programs.members.len(), 4);
        for old in &before.schema.definitions {
            if matches!(old, DefinitionDescriptor::Modifier(row) if row.id == modifier_id(cold)) {
                let mut expected = serde_json::to_value(old).unwrap();
                remap(&mut expected, &replacements);
                let expected: DefinitionDescriptor = serde_json::from_value(expected).unwrap();
                assert!(after.schema.definitions.contains(&expected));
            }
        }
        let mut slots = 0;
        for old in &before.schema.slots {
            let SlotDescriptor::Parameter(old_parameter) = old else {
                continue;
            };
            if old_parameter.id.declaration != SlotOwnerDefId::Modifier(modifier_id(cold)) {
                continue;
            }
            let mut expected = serde_json::to_value(old).unwrap();
            remap(&mut expected, &replacements);
            let expected: SlotDescriptor = serde_json::from_value(expected).unwrap();
            assert!(after.schema.slots.contains(&expected));
            slots += 1;
        }
        assert_eq!(slots, 23);
    }
}
fn check_policies(prior: &Path, output: &Path, bindings: &Value) {
    let catalog_bytes = fs::read(data().join("item-bases/catalog.json")).unwrap();
    assert_eq!(
        bindings["catalog_sha256"],
        format!("{:x}", Sha256::digest(&catalog_bytes))
    );
    let catalog: Value = serde_json::from_slice(&catalog_bytes).unwrap();
    assert_eq!(bindings["catalog_source"], catalog["source"]);
    let bases: BTreeSet<_> = catalog["bases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    let actual: BTreeSet<_> = bindings["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["source_base"].as_str().unwrap())
        .collect();
    assert_eq!(actual, bases);
    let before = json(prior.join("items.json"));
    let mut after = json(output.join("items.json"));
    let old_rules = before["rules"].as_array().unwrap();
    let new_rules = after["rules"].as_array().unwrap();
    assert_eq!(&new_rules[..old_rules.len()], old_rules);
    assert_eq!(new_rules.len(), old_rules.len() + 4);
    let families = bindings["families"].as_array().unwrap();
    let new_ids: BTreeSet<_> = families
        .iter()
        .flat_map(|family| {
            [
                family["source_positive_rule"].as_str().unwrap(),
                family["source_negative_rule"].as_str().unwrap(),
            ]
        })
        .collect();
    assert_eq!(new_ids.len(), 4);
    for row in &new_rules[old_rules.len()..] {
        assert!(new_ids.contains(row["id"].as_str().unwrap()));
    }
    for binding in bindings["templates"].as_array().unwrap() {
        let rule = new_rules
            .iter()
            .find(|rule| rule["id"] == binding["header_rule"])
            .unwrap();
        assert_eq!(
            rule["pattern"],
            serde_json::json!([{"kind":"literal","value":binding["source_base"]}])
        );
        assert_eq!(
            rule["emissions"],
            serde_json::json!([{"kind":"template","value":{"definition":binding["template"]}}])
        );
    }
    for field in ["version", "definitions", "rules"] {
        after[field] = before[field].clone();
    }
    assert_eq!(after, before);
    let before = json(prior.join("item-source.json"));
    let mut after = json(output.join("item-source.json"));
    let roles = after["rule_layouts"].as_array_mut().unwrap();
    assert_eq!(
        roles.len(),
        before["rule_layouts"].as_array().unwrap().len() + 4
    );
    for role in roles
        .iter()
        .filter(|role| new_ids.contains(role["rule"].as_str().unwrap()))
    {
        assert_eq!(role["role"], "unresolved");
    }
    roles.retain(|role| !new_ids.contains(role["rule"].as_str().unwrap()));
    let conditions =
        after["dialect"]["pob_exported_single_text_observations_v1"]["single_modifier_conditions"]
            .as_array_mut()
            .unwrap();
    let old_conditions =
        before["dialect"]["pob_exported_single_text_observations_v1"]["single_modifier_conditions"]
            .as_array()
            .unwrap();
    assert_eq!(conditions.len(), old_conditions.len() + 2);
    for family in families {
        let condition = conditions
            .iter()
            .find(|row| row["rule"] == family["source_positive_rule"])
            .unwrap();
        assert_eq!(
            condition["all"],
            serde_json::json!([
                {"kind":"no_source_tags"},{"kind":"no_generated_buff_members"},
                {"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
            ])
        );
        assert!(
            !conditions
                .iter()
                .any(|row| row["rule"] == family["source_negative_rule"])
        );
    }
    conditions.retain(|row| !new_ids.contains(row["rule"].as_str().unwrap()));
    for field in ["version", "item_lines"] {
        after[field] = before[field].clone();
    }
    assert_eq!(
        after, before,
        "existing source controls, defaults, observations and proofs must remain exact"
    );
}
fn attribute(
    base: &str,
    body: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> poe_optimizer_import::owned_item_source::ItemRangeAttribution {
    let text = format!("Rarity: RARE\nElemental Fixture\n{base}\nImplicits: 0\n{body}")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml =
        format!("<PathOfBuilding2><Items><Item id=\"7\">{text}</Item></Items></PathOfBuilding2>");
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([98; 16]),
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
    let layouts: BTreeMap<_, _> = source
        .input()
        .template_layouts
        .iter()
        .map(|row| (row.template.clone(), row.load_index_prefix))
        .collect();
    let templates = bindings["templates"].as_array().unwrap();
    for family in bindings["families"].as_array().unwrap() {
        let label = label(family);
        let mut admitted = 0;
        for template in templates {
            let base = template["source_base"].as_str().unwrap();
            let id: ItemTemplateDefId =
                serde_json::from_value(template["template"].clone()).unwrap();
            let plan = attribute(base, &format!("+10% to {label} Resistance"), source, lines);
            let converted = plan.convert(lines).unwrap();
            let expected = layouts[&id] == ItemLoadIndexPrefix::NoGeneratedBuffMembers;
            assert_eq!(
                converted.modifiers.len(),
                usize::from(expected),
                "{base}/{label}: {:?}",
                plan.report()
            );
            if expected {
                assert_raw(&converted.modifiers[0], family, 10.0);
                admitted += 1;
            }
        }
        assert!(admitted > 1000 && admitted < templates.len());
        let properties = family["property_inputs"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|name| name.as_str() != "unscalable")
            .map(|name| (key(name), false))
            .collect();
        for (token, amount, admitted) in [
            ("+0", 0.0, true),
            ("+1000000", 1e6, true),
            ("+10.25", 10.25, false),
            ("-12.25", -12.25, false),
            ("-0", 0.0, false),
        ] {
            let text = format!("{token}% to {label} Resistance");
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
            assert_eq!(raw.modifiers.len(), 1);
            assert_raw(&raw.modifiers[0], family, amount);
            let attributed = attribute("Sapphire Ring", &text, source, lines);
            assert_eq!(
                attributed.convert(lines).unwrap().modifiers.len(),
                usize::from(admitted)
            );
        }
        for body in [
            format!("10% to {label} Resistance"),
            format!("+1e2% to {label} Resistance"),
            format!("++10% to {label} Resistance"),
            format!("+1000001% to {label} Resistance"),
            format!("{{tags:fire}}+10% to {label} Resistance"),
            format!("{{tags:lightning}}+10% to {label} Resistance"),
            format!("{{fractured}}+10% to {label} Resistance"),
            format!("{{corruptedRange:1}}+10% to {label} Resistance"),
            format!("{{rune}}+10% to {label} Resistance"),
            format!("unreviewed prefix\n+10% to {label} Resistance"),
        ] {
            let plan = attribute("Sapphire Ring", &body, source, lines);
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
    }
}
fn quantity(program: &RuleProgram, read: &str, amount: f64) -> RuleFact {
    let field = program
        .reads
        .iter()
        .find(|field| field.id.as_str() == read)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &field.value_type else {
        panic!("quantity read")
    };
    RuleFact {
        read: key(read),
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
    for family in bindings["families"].as_array().unwrap() {
        let subject = SchemaSubject::Definition(modifier_id(family).address());
        let owner = after
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == subject)
            .unwrap();
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
                "raw components cannot contribute to actors"
            );
            let report = compiled
                .evaluate(&subject, &program.id, &[], checked.schema(), &mut scratch)
                .unwrap();
            assert!(matches!(
                report.effects[0].disposition,
                EffectDisposition::Unresolved { .. }
            ));
            assert_eq!(report.owner_programs_closure, owner.programs.closure);
        }
        let effective = owner
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == "effective-amount")
            .unwrap();
        for (raw, base, magnitude, expected) in [
            (10.0, 1.0, 1.0, 10.0),
            (10.0, 2.0, 1.5, 30.0),
            // Source corruption truncates (-20 + 0.5) to -19, then magnitude to -28.
            (-10.0, 2.0, 1.5, -28.0),
            (10.5, 1.0, 1.0, 11.0),
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
                panic!("effective component")
            };
            assert_eq!(value.value(), expected);
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
        let ordered = owner
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == "ordered-magnitude-scalar")
            .unwrap();
        assert!(matches!(
            &ordered.reads[0].source,
            RuleReadSource::ModifierTransforms { .. }
        ));
        let catalyst = owner
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == "catalyst-scalar")
            .unwrap();
        let options = json(data().join("resistance/ids.json"));
        for property in ["fire", "lightning"] {
            let option: OptionDefId = serde_json::from_value(
                options["allocations"][format!("catalyst-{property}")].clone(),
            )
            .unwrap();
            for matched in [true, false] {
                let facts: Vec<_> = catalyst
                    .reads
                    .iter()
                    .map(|read| match read.id.as_str() {
                        "unscalable" => RuleFact {
                            read: read.id.clone(),
                            value: ParameterValue::Boolean(false),
                        },
                        "catalyst-kind" => RuleFact {
                            read: read.id.clone(),
                            value: ParameterValue::Option(option.clone()),
                        },
                        "catalyst-amount" => quantity(catalyst, "catalyst-amount", 20.0),
                        name => RuleFact {
                            read: read.id.clone(),
                            value: ParameterValue::Boolean(
                                matched && name == format!("property-{property}"),
                            ),
                        },
                    })
                    .collect();
                let result = compiled
                    .evaluate(
                        &subject,
                        &catalyst.id,
                        &facts,
                        checked.schema(),
                        &mut scratch,
                    )
                    .unwrap();
                let EffectDisposition::Applied {
                    value: ParameterValue::Quantity(value),
                } = &result.effects[0].disposition
                else {
                    panic!("catalyst component")
                };
                assert_eq!(value.value(), if matched { 1.2 } else { 1.0 });
                for omitted in [
                    "catalyst-kind",
                    "unscalable",
                    &format!("property-{property}"),
                ] {
                    let incomplete: Vec<_> = facts
                        .iter()
                        .filter(|fact| fact.read.as_str() != omitted)
                        .cloned()
                        .collect();
                    let result = compiled
                        .evaluate(
                            &subject,
                            &catalyst.id,
                            &incomplete,
                            checked.schema(),
                            &mut scratch,
                        )
                        .unwrap();
                    assert!(
                        matches!(
                            result.effects[0].disposition,
                            EffectDisposition::Unresolved { .. }
                        ),
                        "{omitted}"
                    );
                }
            }
        }
    }
}
fn check_originals(cwd: &Path, prior: &Path, output: &Path, bindings: &Value) {
    let families = bindings["families"].as_array().unwrap();
    let mut summary = vec![];
    let mut total_new = 0;
    let (mut prior_count, mut observed_displays, mut new_cold) = (0, 0, 0);
    let mut totals = Vec::new();
    let mut family_counts = BTreeMap::new();
    let cold_bindings = json(data().join("modifier-value-inputs/bindings.json"));
    let cold = cold_bindings["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|family| family["family"] == "cold-resistance")
        .unwrap();
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior.join(&query)).unwrap(),
            fs::read(output.join(&query)).unwrap()
        );
        assert_eq!(json(output.join(&query)).as_array().unwrap().len(), 22);
        let destination = cwd.join(format!("elemental-resistance-original-{case}"));
        assert_eq!(
            success(normalize(cwd, output, case, &destination, true))["normalization_status"],
            "pending"
        );
        let before = json(cwd.join(format!("item-observations-original-{case}/sidecar.json")));
        let after = json(destination.join("sidecar.json"));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(before[field], after[field]);
        }
        let (mut added, mut original_total) = (0, 0);
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
                prior_count += previous["modifiers"].as_array().unwrap().len();
                original_total += row["modifiers"].as_array().unwrap().len();
                if previous["outcome"]["kind"] == "known"
                    && previous["outcome"]["value"]["rule"]
                        .as_str()
                        .is_some_and(|rule| rule.starts_with("observed-"))
                {
                    observed_displays += 1;
                    assert_eq!(
                        row["outcome"], previous["outcome"],
                        "derived display admission changed"
                    );
                }
                if !previous["modifiers"].as_array().unwrap().is_empty() {
                    assert_eq!(row["outcome"], previous["outcome"]);
                    assert_eq!(
                        row["modifiers"].as_array().unwrap().len(),
                        previous["modifiers"].as_array().unwrap().len()
                    );
                    continue;
                }
                if row["modifiers"].as_array().unwrap().is_empty() {
                    continue;
                }
                let attributed = item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|line| line["index"] == row["index"])
                    .unwrap();
                assert!(attributed["blockers"].as_array().unwrap().is_empty());
                assert!(!attributed["member"].is_null());
                for (emission_index, emission) in row["outcome"]["value"]["emissions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                {
                    if emission["kind"] != "modifier" {
                        continue;
                    }
                    let value = &emission["value"];
                    let family = families
                        .iter()
                        .find(|family| family["canonical"] == value["definition"]);
                    if let Some(family) = family {
                        let raw = row["text"].as_str().unwrap();
                        let amount: f64 = raw
                            .strip_prefix('+')
                            .unwrap()
                            .strip_suffix(&format!("% to {} Resistance", label(family)))
                            .unwrap()
                            .parse()
                            .unwrap();
                        let modifier = LocatedItemModifier {
                            line: row["index"].as_u64().unwrap() as usize,
                            emission: emission_index,
                            definition: serde_json::from_value(value["definition"].clone())
                                .unwrap(),
                            rolls: serde_json::from_value(value["rolls"].clone()).unwrap(),
                            rolls_closure: serde_json::from_value(value["rolls_closure"].clone())
                                .unwrap(),
                        };
                        assert_raw(&modifier, family, amount);
                        added += 1;
                        *family_counts
                            .entry(family["family"].as_str().unwrap())
                            .or_insert(0) += 1;
                    } else {
                        assert_eq!(
                            value["definition"], cold["canonical"],
                            "unexpected newly admitted family"
                        );
                        let raw = row["text"].as_str().unwrap();
                        let amount: f64 = raw
                            .strip_prefix('+')
                            .unwrap()
                            .strip_suffix("% to Cold Resistance")
                            .unwrap()
                            .parse()
                            .unwrap();
                        let modifier = LocatedItemModifier {
                            line: row["index"].as_u64().unwrap() as usize,
                            emission: emission_index,
                            definition: serde_json::from_value(value["definition"].clone())
                                .unwrap(),
                            rolls: serde_json::from_value(value["rolls"].clone()).unwrap(),
                            rolls_closure: serde_json::from_value(value["rolls_closure"].clone())
                                .unwrap(),
                        };
                        assert_raw(&modifier, cold, amount);
                        new_cold += 1;
                    }
                    // Previously supported followers can now resolve after a proved member.
                }
            }
        }
        total_new += added;
        totals.push(original_total);
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
                if matches!(&modifier.definition,DraftField::Known {value} if families.iter().any(|family|modifier_id(family)==*value))
                {
                    assert!(matches!(
                        modifier.rolls.completion,
                        DraftListCompletion::Pending { .. }
                    ));
                }
            }
        }
        summary.push(serde_json::json!({"original":case,"new_elemental_modifiers":added,"queries":22,"normalization":"pending","whole_build_parity":"not_established"}));
    }
    assert_eq!(totals, [3, 18, 4, 3, 6]);
    assert_eq!(totals.iter().sum::<usize>(), 34);
    assert_eq!(prior_count, 10);
    assert_eq!(observed_displays, 53);
    assert_eq!(total_new, 19);
    assert_eq!(new_cold, 5);
    assert_eq!(
        family_counts,
        BTreeMap::from([("fire-resistance", 6), ("lightning-resistance", 13)])
    );
    fs::write(
        cwd.join("elemental-resistance-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}

pub fn check_elemental_resistance(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("elemental-resistance-inputs");
    let authored_before = bundle(&authored);
    let prior_before = bundle(prior);
    let items = authored.join("items.json");
    let source = authored.join("item-source.json");
    let output = cwd.join("elemental-resistance-successor");
    let report = success(publish(cwd, prior, &authored, &items, &source, &output));
    assert_eq!(report["extension"]["allocated_entries"], 48);
    assert_eq!(report["extension"]["refined_subjects"], 1756);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(json(output.join("items.json")), json(&items));
    assert_eq!(json(output.join("item-source.json")), json(&source));
    let bindings = json(authored.join("bindings.json"));
    let before = recipe(prior);
    let after = recipe(&output);
    check_structure(&before, &after, &bindings);
    check_policies(prior, &output, &bindings);
    check_numeric(&after, &bindings);
    let checked = assemble_owned_recipe(after, Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(&items)).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let source_policy = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(&source)).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    check_import(&lines, &source_policy, &bindings);
    check_originals(cwd, prior, &output, &bindings);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &authored, &items, &source, &output)
            .status
            .success()
    );
    assert!(bundle(&output) == published, "published bundle changed");
    let replay = cwd.join("elemental-resistance-replay");
    assert_eq!(
        success(publish(cwd, prior, &authored, &items, &source, &replay)),
        report
    );
    assert!(bundle(&replay) == published, "replayed bundle differs");
    for (name, is_items) in [("definitions", true), ("item-lines", false)] {
        let mut stale = json(if is_items { &items } else { &source });
        if is_items {
            stale["definitions"] = report["publication"]["before"]["definitions"].clone();
        } else {
            stale["item_lines"] = serde_json::to_value(
                digest_owned("stale-elemental-source-v1", &1_u32, 1024).unwrap(),
            )
            .unwrap();
        }
        let path = cwd.join(format!("elemental-resistance-stale-{name}.json"));
        fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
        let rejected = cwd.join(format!("elemental-resistance-rejected-{name}"));
        let result = if is_items {
            publish(cwd, prior, &authored, &path, &source, &rejected)
        } else {
            publish(cwd, prior, &authored, &items, &path, &rejected)
        };
        assert!(!result.status.success());
        assert!(!rejected.exists());
    }
    assert!(bundle(prior) == prior_before, "prior bundle changed");
    assert!(
        bundle(&authored) == authored_before,
        "authored bytes changed"
    );
    output
}
