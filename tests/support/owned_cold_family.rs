//! A semantic cold-resistance family can occur on many items without closing coverage.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::digest_owned,
    owned_definitions::{ModifierDefId, ParameterSlotDefId},
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaState},
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemLineInput, ItemLineOutcome, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{ItemRangeAttribution, ItemSourceLayoutPolicy, ItemSourceProblem},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
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
fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}
fn equal_bundle(actual: &BTreeMap<String, Vec<u8>>, expected: &BTreeMap<String, Vec<u8>>) {
    assert!(
        actual.keys().eq(expected.keys()),
        "bundle filenames changed"
    );
    for (name, bytes) in expected {
        assert!(
            actual.get(name) == Some(bytes),
            "bundle file changed: {name}"
        );
    }
}
fn family() -> Value {
    json(data().join("modifier-value-inputs/bindings.json"))["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "cold-resistance")
        .unwrap()
        .clone()
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
fn assert_raw(modifier: &LocatedItemModifier, family: &Value, amount: f64, present: &[&str]) {
    let canonical: ModifierDefId = serde_json::from_value(family["canonical"].clone()).unwrap();
    assert_eq!(modifier.definition, canonical);
    let SchemaClosure::Partial { gaps } = &modifier.rolls_closure else {
        panic!("eligibility membership cannot become complete")
    };
    assert!(
        gaps.iter()
            .any(|g| g.code.as_str() == "modifier-eligibility-inputs-unconverted")
    );
    let ParameterValue::Quantity(raw) = parameter(modifier, &family["canonical_inputs"]["amount"])
    else {
        panic!("raw quantity")
    };
    assert_eq!(raw.value(), amount);
    assert_eq!(serde_json::to_value(raw.unit()).unwrap(), family["unit"]);
    let ParameterValue::Quantity(factor) = parameter(modifier, &family["corrupted_base_input"])
    else {
        panic!("explicit corrupted-base factor")
    };
    assert_eq!(factor.value(), 1.0);
    for (name, slot) in family["property_inputs"].as_object().unwrap() {
        assert_eq!(
            parameter(modifier, slot),
            &ParameterValue::Boolean(present.contains(&name.as_str())),
            "property {name}"
        );
    }
    assert_eq!(
        modifier.rolls.len(),
        23,
        "raw, factor, twenty source properties and unscalable"
    );
}
fn attribute(
    base: &str,
    headers: &str,
    body: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let text = format!("Rarity: RARE\nCold Family Fixture\n{base}\n{headers}Implicits: 0\n{body}");
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">{escaped}</Item></Items></PathOfBuilding2>"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([96; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let row = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Item")
        .unwrap();
    source
        .attribute(&evidence, row.occurrence().id(), lines)
        .unwrap()
}
fn check_source_cases(output: &Path, family: &Value) {
    let checked = assemble_owned_recipe(recipe(output), Default::default()).unwrap();
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
    let properties = family["property_inputs"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|name| name.as_str() != "unscalable")
        .map(|name| (name.parse().unwrap(), false))
        .collect();
    // These are explicit test facts supplied to the public raw converter. They
    // establish representability on finite templates, not source member authority.
    for base in [
        "Frayed Shoes",
        "Iron Ring",
        "Gold Ring",
        "Grand Spear",
        "Sapphire Ring",
        "Emerald",
    ] {
        for (text, amount) in [
            ("+10% to Cold Resistance", 10.0),
            ("+8% to Cold Resistance", 8.0),
            ("+34% to Cold Resistance", 34.0),
            ("+10.25% to Cold Resistance", 10.25),
            ("-12.25% to Cold Resistance", -12.25),
            ("+0% to Cold Resistance", 0.0),
            ("-0% to Cold Resistance", 0.0),
            ("-0.1% to Cold Resistance", -0.1),
        ] {
            let converted = lines
                .convert_lines([
                    ItemLineInput {
                        index: 1,
                        text: base,
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
            assert_eq!(
                converted.modifiers.len(),
                1,
                "typed raw conversion: {base}/{text}"
            );
            assert_raw(&converted.modifiers[0], family, amount, &[]);
            let attributed = attribute(base, "Unique ID: synthetic\n", text, &source, &lines);
            assert!(
                attributed.convert(&lines).unwrap().modifiers.is_empty(),
                "fixed source context remains unproved: {base}/{text}"
            );
            assert!(attributed.report().lines.last().unwrap().member.is_none());
        }
    }
    let catalyst_headers = "Catalyst: Tul's\nCatalystQuality: 20\n";
    let negative_catalyst = "Catalyst: Tul's\nCatalystQuality: -200\n";
    // Source formatting depends on the item context: negative zero, leading-dot
    // decimals under a catalyst, or a negative catalyst scalar can all prevent a
    // complete parse. Neither fixed grammar therefore promises one source member.
    for (headers, first) in [
        ("", "+10% to Cold Resistance"),
        ("", "-0% to Cold Resistance"),
        ("", "-0.1% to Cold Resistance"),
        ("", "+10.25% to Cold Resistance"),
        ("", "+.99999999999999999999% to Cold Resistance"),
        (catalyst_headers, "{tags:cold}+10.25% to Cold Resistance"),
        (
            catalyst_headers,
            "{tags:cold}+.99999999999999999999% to Cold Resistance",
        ),
        (negative_catalyst, "{tags:cold}+10% to Cold Resistance"),
        ("", "+1000001% to Cold Resistance"),
        ("", "+1000000000000000% to Cold Resistance"),
        (catalyst_headers, "{tags:cold}+1000001% to Cold Resistance"),
    ] {
        for following in [
            "+10% to Cold Resistance",
            "{range:0.5}+(20-30)% to Cold Resistance",
            "while stationary",
        ] {
            let pair = attribute(
                "Crude Bow",
                headers,
                &format!("{first}\n{following}"),
                &source,
                &lines,
            );
            let converted = pair.convert(&lines).unwrap();
            assert!(
                converted.modifiers.is_empty(),
                "unproved member boundary: {first} / {following}"
            );
            assert!(
                pair.report()
                    .lines
                    .last()
                    .unwrap()
                    .blockers
                    .contains(&ItemSourceProblem::PossibleCombinedLine),
                "combination uncertainty lost: {first}"
            );
            if !headers.is_empty() {
                assert_eq!(
                    converted.parameters.len(),
                    2,
                    "catalyst context was actually imported"
                );
            }
        }
    }
    // The previously admitted ranged contract retains its separate source gates.
    let ranged = attribute(
        "Gold Ring",
        "",
        "{range:0.5}+(20-30)% to Cold Resistance",
        &source,
        &lines,
    );
    let converted = ranged.convert(&lines).unwrap();
    assert_eq!(converted.modifiers.len(), 1);
    assert_raw(&converted.modifiers[0], family, 25.0, &[]);
    for text in [
        "10% to Cold Resistance",
        "0% to Cold Resistance",
        "++10% to Cold Resistance",
        "+1e2% to Cold Resistance",
        "+1000001% to Cold Resistance",
        "+10% to Cold Resistance trailing",
        "{corruptedRange:1}+10% to Cold Resistance",
        "{corruptedRange:0.5}+10% to Cold Resistance",
        "{rune}+10% to Cold Resistance",
        "{crafted}+10% to Cold Resistance",
        "{fractured}+10% to Cold Resistance",
        "{tags:unknown}+10% to Cold Resistance",
        "{variant:1}+10% to Cold Resistance",
        "Unknown modifier\n+10% to Cold Resistance",
    ] {
        let attributed = attribute("Iron Ring", "", text, &source, &lines);
        assert!(
            attributed.convert(&lines).unwrap().modifiers.is_empty(),
            "unsupported source admitted: {text}"
        );
    }
    for (base, header) in [
        ("Unrecognized Base", ""),
        ("Iron Ring", "Unknown Header: 1\n"),
        ("Iron Ring", "Corrupted\n"),
    ] {
        let attributed = attribute(base, header, "+10% to Cold Resistance", &source, &lines);
        assert!(
            attributed.convert(&lines).unwrap().modifiers.is_empty(),
            "unproved scope: {base}/{header}"
        );
    }
    for text in ["10% to Cold Resistance", "0% to Cold Resistance"] {
        assert!(
            matches!(
                lines.convert_line(1, text, None).unwrap().outcome,
                ItemLineOutcome::Pending { .. }
            ),
            "bare resistance remains outside the raw grammar"
        );
    }
}
fn check_schema_and_policies(prior: &Path, output: &Path, authored: &Path, family: &Value) {
    let before = recipe(prior);
    let after = recipe(output);
    let canonical: ModifierDefId = serde_json::from_value(family["canonical"].clone()).unwrap();
    assert!(
        after.registry == before.registry,
        "no ID allocation or registry mutation"
    );
    assert_eq!(after.registry.entries.len(), 10537);
    assert_eq!(
        after.schema.definitions.len(),
        before.schema.definitions.len()
    );
    assert!(
        after.schema.slots == before.schema.slots,
        "slot contracts changed"
    );
    let catalog = json(data().join("item-layouts/policy.json"));
    let templates: BTreeSet<_> = catalog["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["template"]["key"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(templates.len(), 1756);
    let evidence = json(authored.join("family.json"));
    let catalog_bytes = fs::read(data().join("item-bases/catalog.json")).unwrap();
    let base_catalog: Value = serde_json::from_slice(&catalog_bytes).unwrap();
    assert_eq!(
        evidence["catalog_sha256"],
        format!("{:x}", Sha256::digest(&catalog_bytes))
    );
    assert!(
        evidence["catalog_source"] == base_catalog["source"],
        "catalog provenance changed"
    );
    assert!(
        evidence["templates"] == catalog["templates"],
        "finite base/template/header mapping changed"
    );
    assert_eq!(
        evidence["prior_definitions"],
        serde_json::to_value(&before.rules.definitions).unwrap()
    );
    assert_eq!(
        evidence["definitions"],
        serde_json::to_value(&after.rules.definitions).unwrap()
    );
    assert_eq!(evidence["canonical"], family["canonical"]);
    assert_eq!(evidence["refined_templates"], 1755);
    assert_eq!(evidence["retained_templates"], 1);
    assert_eq!(
        evidence["membership_scope"],
        "structural_raw_modifier_family_only"
    );
    assert_eq!(evidence["numerical_eligibility"], "partial");
    assert_eq!(evidence["affix_legality"], "not_established");
    let (mut selected, mut refined) = (0, 0);
    for old in &before.schema.definitions {
        let new = after
            .schema
            .definitions
            .iter()
            .find(|entry| entry.address() == old.address())
            .unwrap();
        let DefinitionDescriptor::ItemTemplate(old_item) = old else {
            assert!(new == old, "non-item descriptor changed");
            continue;
        };
        if !templates.contains(old_item.id.key().as_str()) {
            assert!(new == old, "non-catalog item changed");
            continue;
        }
        selected += 1;
        let DefinitionDescriptor::ItemTemplate(new_item) = new else {
            panic!("item kind changed")
        };
        let (SchemaState::Known(old_schema), SchemaState::Known(new_schema)) =
            (&old_item.schema, &new_item.schema)
        else {
            panic!("known constructed bases")
        };
        assert!(matches!(
            old_schema.modifiers.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(new_schema.modifiers.closure, old_schema.modifiers.closure);
        assert!(new_schema.modifiers.members.contains(&canonical));
        assert_eq!(
            new_schema.modifiers.members.len(),
            old_schema.modifiers.members.len()
                + usize::from(!old_schema.modifiers.members.contains(&canonical))
        );
        assert!(
            old_schema
                .modifiers
                .members
                .iter()
                .all(|member| new_schema.modifiers.members.contains(member))
        );
        let mut restored = new_schema.clone();
        restored.modifiers = old_schema.modifiers.clone();
        assert!(
            &restored == old_schema,
            "other base facts changed: {}",
            old_item.id.key().as_str()
        );
        refined += usize::from(new != old);
    }
    assert_eq!((selected, refined), (1756, 1755));
    let mut rules = after.rules.clone();
    rules.definitions = before.rules.definitions.clone();
    assert!(
        rules == before.rules,
        "numerical programs, closures or operation contract changed"
    );
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert!(routing == before.routing, "routing semantics changed");
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let extension: OwnedRecipeExtension =
        serde_json::from_value(json(authored.join("extension.json"))).unwrap();
    let replay = extend_owned_recipe(&checked, &extension, Default::default()).unwrap();
    assert_eq!(replay.receipt.allocated_entries, 0);
    assert_eq!(replay.receipt.refined_subjects, 0);
    assert!(
        replay.successor == after,
        "reapplying membership extension changed data"
    );
    let before = json(prior.join("items.json"));
    let after = json(output.join("items.json"));
    let old_rules = before["rules"].as_array().unwrap();
    let new_rules = after["rules"].as_array().unwrap();
    assert_eq!((old_rules.len(), new_rules.len()), (1799, 1800));
    for rule in old_rules.iter().filter(|r| r["id"] != "fixed-cold") {
        assert!(new_rules.contains(rule), "old rule changed: {}", rule["id"]);
    }
    let mut restored = after.clone();
    for field in ["version", "definitions", "rules"] {
        restored[field] = before[field].clone();
    }
    assert!(restored == before, "other item-policy semantics changed");
    for id in ["fixed-cold", "fixed-minus-cold"] {
        let rule = new_rules.iter().find(|r| r["id"] == id).unwrap();
        assert_eq!(
            rule["emissions"][0]["value"]["definition"],
            family["canonical"]
        );
        assert_eq!(
            rule["emissions"][0]["value"]["rolls"]
                .as_array()
                .unwrap()
                .len(),
            23
        );
    }
    let before = json(prior.join("item-source.json"));
    let mut after = json(output.join("item-source.json"));
    let roles = after["rule_layouts"].as_array_mut().unwrap();
    assert_eq!(
        roles.len(),
        before["rule_layouts"].as_array().unwrap().len() + 1
    );
    assert!(roles.contains(&serde_json::json!({"rule":"fixed-minus-cold", "role":"unresolved"})));
    roles.retain(|r| r["rule"] != "fixed-minus-cold");
    let fixed = roles
        .iter_mut()
        .find(|r| r["rule"] == "fixed-cold")
        .unwrap();
    assert_eq!(fixed["role"], "unresolved");
    let previous = before["rule_layouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "fixed-cold")
        .unwrap();
    assert_eq!(previous["role"], "single_modifier");
    *fixed = previous.clone();
    for field in ["version", "item_lines"] {
        after[field] = before[field].clone();
    }
    assert!(
        after == before,
        "source controls/defaults/layout proofs changed"
    );
}
type Admission = (u64, u64, usize);
fn admissions(sidecar: &Value) -> BTreeSet<Admission> {
    sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| {
            let source = item["source"]["ordinal"].as_u64().unwrap();
            item["lines"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(move |line| {
                    let index = line["index"].as_u64().unwrap();
                    (0..line["modifiers"].as_array().unwrap().len())
                        .map(move |modifier| (source, index, modifier))
                })
        })
        .collect()
}
fn check_originals(cwd: &Path, prior: &Path, output: &Path, family: &Value) {
    let canonical: ModifierDefId = serde_json::from_value(family["canonical"].clone()).unwrap();
    let (mut total_before, mut total_after, mut fixed, mut blocked) = (0, 0, 0, 0);
    let mut new_occurrences = BTreeSet::new();
    let mut summary = vec![];
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert!(
            fs::read(prior.join(&name)).unwrap() == fs::read(output.join(&name)).unwrap(),
            "query bytes changed: {name}"
        );
        assert_eq!(json(output.join(&name)).as_array().unwrap().len(), 22);
        let destination = cwd.join(format!("cold-family-original-{case}"));
        assert_eq!(
            success(normalize(cwd, output, case, &destination, true))["normalization_status"],
            "pending"
        );
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
                .map(|p| p.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        assert!(!input.items.members.is_empty() && !input.allocations.members.is_empty());
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
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
                if matches!(&modifier.definition, DraftField::Known { value } if value == &canonical)
                {
                    assert!(
                        matches!(
                            modifier.rolls.completion,
                            DraftListCompletion::Pending { .. }
                        ),
                        "canonical cold raw facts cannot close eligibility input membership"
                    );
                }
            }
        }
        let predecessor = decode_draft(
            &fs::read(cwd.join(format!("item-metadata-original-{case}/draft.json"))).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let non_cold_roll_closures = |draft: &poe_optimizer_core::owned_draft::DraftSession| {
            let mut counts = BTreeMap::new();
            for modifier in draft
                .input()
                .items
                .members
                .iter()
                .flat_map(|item| &item.modifiers.members)
            {
                let DraftField::Known { value: definition } = &modifier.definition else {
                    panic!("known imported modifier definition")
                };
                if definition == &canonical {
                    continue;
                }
                let complete = matches!(modifier.rolls.completion, DraftListCompletion::Complete);
                *counts
                    .entry((definition.clone(), complete))
                    .or_insert(0usize) += 1;
            }
            counts
        };
        assert_eq!(
            non_cold_roll_closures(&draft),
            non_cold_roll_closures(&predecessor),
            "existing modifier roll-closure contracts changed"
        );
        if matches!(case, 2 | 3) {
            for (completion, code) in [
                (
                    &input.items.completion,
                    "socketed-item-membership-not-converted",
                ),
                (
                    &input.equipment.completion,
                    "socketed-equipment-membership-not-converted",
                ),
            ] {
                let DraftListCompletion::Pending { code: actual, .. } = completion else {
                    panic!("rune collection falsely complete")
                };
                assert_eq!(actual.as_str(), code);
            }
        }
        let after = json(destination.join("sidecar.json"));
        let before = json(cwd.join(format!("item-metadata-original-{case}/sidecar.json")));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(after[field], before[field], "original-{case}: {field}");
        }
        let previous = admissions(&before);
        let current = admissions(&after);
        assert!(
            previous.is_subset(&current),
            "original-{case}: existing admission lost"
        );
        total_before += previous.len();
        total_after += current.len();
        for &(source, line, emission) in current.difference(&previous) {
            new_occurrences.insert((case, source, line, emission));
        }
        for item in after["item_texts"].as_array().unwrap() {
            let old_item = before["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|old| old["source"] == item["source"])
                .unwrap();
            for line in item["attribution"]["lines"].as_array().unwrap() {
                let old = old_item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|old| old["index"] == line["index"])
                    .unwrap();
                assert!(line["raw"] == old["raw"], "original source text changed");
                assert!(
                    old["blockers"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|blocker| line["blockers"].as_array().unwrap().contains(blocker)),
                    "existing source blocker was cleared by membership data"
                );
                if line["rule"] != "fixed-cold" {
                    continue;
                }
                fixed += 1;
                let row = item["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["index"] == line["index"])
                    .unwrap();
                assert!(
                    row["modifiers"].as_array().unwrap().is_empty(),
                    "fixed source context became authoritative"
                );
                blocked += 1;
                assert!(
                    line["member"].is_null(),
                    "unproved fixed line became a source member"
                );
                assert_ne!(item["attribution"]["layout"]["status"], "proven");
            }
        }
        summary.push(serde_json::json!({"original":case,"query_rows":22,"normalization":"pending","admitted_before":previous.len(),"admitted_after":current.len(),"new_occurrences":current.difference(&previous).collect::<Vec<_>>(),"whole_build_parity":"not_established"}));
    }
    assert_eq!((total_before, total_after), (7, 7));
    assert_eq!((fixed, blocked), (22, 22));
    assert!(
        new_occurrences.is_empty(),
        "membership knowledge alone cannot authorize fixed source lines"
    );
    fs::write(
        cwd.join("cold-family-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}
pub fn check_cold_family(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("cold-family-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let items = authored.join("items.json");
    let source = authored.join("item-source.json");
    let output = cwd.join("cold-family-successor");
    let report = success(publish(cwd, prior, &authored, &items, &source, &output));
    assert_eq!(report["extension"]["refined_subjects"], 1755);
    for field in [
        "allocated_entries",
        "appended_tables",
        "appended_programs",
        "appended_receivers",
    ] {
        assert_eq!(report["extension"][field], 0, "{field}");
    }
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    assert_ne!(
        report["publication"]["before"],
        report["publication"]["after"]
    );
    let published = bundle(&output);
    assert!(
        published.keys().eq(prior_bytes.keys()),
        "bundle file set changed"
    );
    assert!(
        published.get("registry.json") == prior_bytes.get("registry.json"),
        "ID registry changed"
    );
    assert!(json(output.join("items.json")) == json(&items));
    assert!(json(output.join("item-source.json")) == json(&source));
    let family = family();
    check_schema_and_policies(prior, &output, &authored, &family);
    check_source_cases(&output, &family);
    assert!(
        !publish(cwd, prior, &authored, &items, &source, &output)
            .status
            .success()
    );
    equal_bundle(&bundle(&output), &published);
    let replay = cwd.join("cold-family-replay");
    assert_eq!(
        success(publish(cwd, prior, &authored, &items, &source, &replay)),
        report
    );
    equal_bundle(&bundle(&replay), &published);
    for (name, items_stale) in [("definitions", true), ("item-lines", false)] {
        let mut value = json(if items_stale { &items } else { &source });
        if items_stale {
            value["definitions"] = report["publication"]["before"]["definitions"].clone();
        } else {
            value["item_lines"] =
                serde_json::to_value(digest_owned("stale-cold-lines", &false, 1024).unwrap())
                    .unwrap();
        }
        let path = cwd.join(format!("cold-family-stale-{name}.json"));
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let rejected = cwd.join(format!("cold-family-rejected-{name}"));
        assert!(
            !publish(
                cwd,
                prior,
                &authored,
                if items_stale { &path } else { &items },
                if items_stale { &source } else { &path },
                &rejected
            )
            .status
            .success(),
            "stale {name} accepted"
        );
        assert!(!rejected.exists());
    }
    check_originals(cwd, prior, &output, &family);
    equal_bundle(&bundle(&output), &published);
    equal_bundle(&bundle(prior), &prior_bytes);
    equal_bundle(&bundle(&authored), &authored_bytes);
    output
}
