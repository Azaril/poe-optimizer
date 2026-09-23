//! Conditional source-member evidence can admit raw facts without closing evaluation.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::digest_owned,
    owned_definitions::{ItemTemplateDefId, ModifierDefId, ParameterSlotDefId},
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_schema::SchemaClosure,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{
        ItemLoadIndexPrefix, ItemRangeAttribution, ItemSourceLayoutPolicy, ItemSourceProblem,
    },
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_source::SourceProjectEvidence,
};
use serde_json::Value;
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
fn assert_raw(modifier: &LocatedItemModifier, family: &Value, amount: f64) {
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
    for slot in family["property_inputs"].as_object().unwrap().values() {
        assert_eq!(parameter(modifier, slot), &ParameterValue::Boolean(false));
    }
    assert_eq!(modifier.rolls.len(), 23);
}
fn attribute(
    base: &str,
    headers: &str,
    body: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let text =
        format!("Rarity: RARE\nConditional Source Fixture\n{base}\n{headers}Implicits: 0\n{body}");
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">{escaped}</Item></Items></PathOfBuilding2>"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([97; 16]),
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
fn check_policy(prior: &Path, output: &Path) {
    let before = json(prior.join("item-source.json"));
    let after = json(output.join("item-source.json"));
    assert_eq!(before["schema_version"], 5);
    assert_eq!(after["schema_version"], 6);
    let previous = &before["dialect"]["pob_exported_single_text_preamble_v1"];
    let next = &after["dialect"]["pob_exported_single_text_conditions_v1"];
    assert!(
        next["flag_bindings"] == previous["flag_bindings"],
        "flag bindings changed"
    );
    assert!(
        next["metadata_rules"] == previous["metadata_rules"],
        "metadata admission changed"
    );
    assert_eq!(
        next["single_modifier_conditions"],
        serde_json::json!([{
            "rule":"fixed-cold", "all":[
                {"kind":"no_source_tags"},
                {"kind":"no_generated_buff_members"},
                {"kind":"unsigned_integer_capture", "value":{"capture":"amount","min":0,"max":1000000}}
            ]
        }])
    );
    for id in ["fixed-cold", "fixed-minus-cold"] {
        assert_eq!(
            after["rule_layouts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["rule"] == id)
                .unwrap()["role"],
            "unresolved"
        );
    }
    let mut restored = after;
    for field in ["schema_version", "version", "dialect"] {
        restored[field] = before[field].clone();
    }
    assert!(
        restored == before,
        "conditional publication changed unrelated source contracts"
    );
}
fn check_source_cases(prior: &Path, output: &Path, family: &Value) {
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
    let old = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(prior.join("item-source.json"))).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    assert_ne!(source.identity(), old.identity());
    for base in [
        "Frayed Shoes",
        "Iron Ring",
        "Gold Ring",
        "Grand Spear",
        "Sapphire Ring",
        "Emerald",
    ] {
        for (text, amount) in [
            ("+0% to Cold Resistance", 0.0),
            ("+10% to Cold Resistance", 10.0),
            ("+1000000% to Cold Resistance", 1000000.0),
        ] {
            let prior = attribute(base, "", text, &old, &lines);
            assert!(
                prior.convert(&lines).unwrap().modifiers.is_empty(),
                "v5 must not inherit v6 member authority"
            );
            let current = attribute(base, "Unique ID: synthetic\n", text, &source, &lines);
            let converted = current.convert(&lines).unwrap();
            assert_eq!(
                converted.modifiers.len(),
                1,
                "conditional member not admitted: {base}/{text}"
            );
            assert_raw(&converted.modifiers[0], family, amount);
            assert!(current.report().lines.last().unwrap().member.is_some());
        }
    }
    let catalog = json(data().join("item-layouts/catalog.json"));
    let generated = catalog["bases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|base| base["prefix"] == "present")
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 13);
    for base in generated {
        let name = base["source_base"].as_str().unwrap();
        let current = attribute(name, "", "+25% to Cold Resistance", &source, &lines);
        assert!(
            current.convert(&lines).unwrap().modifiers.is_empty(),
            "generated-buff context cannot certify a physical fixed member: {name}"
        );
    }
    // A data-supplied unresolved prefix has the same meaning for another category;
    // neither a flask name nor a modifier family bypasses the conditional proof.
    let layouts = json(data().join("item-layouts/policy.json"));
    let flask = &layouts["templates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_base"] == "Colossal Life Flask")
        .unwrap()["template"];
    let mut unresolved = source.input().clone();
    let template: ItemTemplateDefId = serde_json::from_value(flask.clone()).unwrap();
    unresolved
        .template_layouts
        .iter_mut()
        .find(|row| row.template == template)
        .unwrap()
        .load_index_prefix = ItemLoadIndexPrefix::Unresolved;
    let unresolved =
        ItemSourceLayoutPolicy::new(unresolved, &lines, checked.schema(), Default::default())
            .unwrap();
    assert!(
        attribute(
            "Colossal Life Flask",
            "",
            "+25% to Cold Resistance",
            &unresolved,
            &lines
        )
        .convert(&lines)
        .unwrap()
        .modifiers
        .is_empty()
    );
    // The condition proves tag absence, not absence of item-level catalysts.
    // An untagged integer remains untagged under these explicitly parsed headers.
    for headers in [
        "Catalyst: Tul's\nCatalystQuality: 20\n",
        "Catalyst: Tul's\nCatalystQuality: -200\n",
    ] {
        let current = attribute(
            "Crude Bow",
            headers,
            "+10% to Cold Resistance",
            &source,
            &lines,
        );
        let converted = current.convert(&lines).unwrap();
        assert_eq!(converted.parameters.len(), 2);
        assert_eq!(converted.modifiers.len(), 1);
        assert_raw(&converted.modifiers[0], family, 10.0);
    }
    let catalyst = "Catalyst: Tul's\nCatalystQuality: 20\n";
    let negative_catalyst = "Catalyst: Tul's\nCatalystQuality: -200\n";
    for (headers, first) in [
        ("", "-12% to Cold Resistance"),
        ("", "-0% to Cold Resistance"),
        ("", "-0.1% to Cold Resistance"),
        ("", "10% to Cold Resistance"),
        ("", "+10.0% to Cold Resistance"),
        ("", "+10.25% to Cold Resistance"),
        ("", "+.99999999999999999999% to Cold Resistance"),
        ("", "+1000001% to Cold Resistance"),
        ("", "+1000000000000000% to Cold Resistance"),
        ("", "+1e2% to Cold Resistance"),
        ("", "{tags:cold}+10% to Cold Resistance"),
        ("", "{tags:}+10% to Cold Resistance"),
        (
            catalyst,
            "{tags:cold}+.99999999999999999999% to Cold Resistance",
        ),
        (negative_catalyst, "{tags:cold}+10% to Cold Resistance"),
        ("", "{fractured}+10% to Cold Resistance"),
        ("", "{corruptedRange:1}+10% to Cold Resistance"),
    ] {
        let single = attribute("Crude Bow", headers, first, &source, &lines);
        assert!(
            single.convert(&lines).unwrap().modifiers.is_empty(),
            "unproved condition admitted: {first}"
        );
        let pair = attribute(
            "Crude Bow",
            headers,
            &format!("{first}\n+10% to Cold Resistance"),
            &source,
            &lines,
        );
        let converted = pair.convert(&lines).unwrap();
        assert!(
            converted.modifiers.is_empty(),
            "failed condition cannot certify next physical member: {first}"
        );
        assert!(
            pair.report()
                .lines
                .last()
                .unwrap()
                .blockers
                .contains(&ItemSourceProblem::PossibleCombinedLine)
        );
        if !headers.is_empty() {
            assert_eq!(
                converted.parameters.len(),
                2,
                "catalyst context is an actual imported input"
            );
        }
    }
    for text in [
        "{rune}+10% to Cold Resistance",
        "{crafted}+10% to Cold Resistance",
        "{variant:1}+10% to Cold Resistance",
        "{tags:unknown}+10% to Cold Resistance",
    ] {
        let current = attribute("Iron Ring", "", text, &source, &lines);
        assert!(
            current.convert(&lines).unwrap().modifiers.is_empty(),
            "source control admitted: {text}"
        );
    }
    for (base, headers) in [
        ("Unknown Base", ""),
        ("Iron Ring", "Unknown Header: 1\n"),
        ("Iron Ring", "Corrupted\n"),
    ] {
        let current = attribute(base, headers, "+10% to Cold Resistance", &source, &lines);
        assert!(
            current.convert(&lines).unwrap().modifiers.is_empty(),
            "other source gates bypassed: {base}/{headers}"
        );
    }
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
                        .map(move |emission| (source, index, emission))
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
        assert_eq!(json(output.join(name)).as_array().unwrap().len(), 22);
        let destination = cwd.join(format!("source-conditions-original-{case}"));
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
                if matches!(&modifier.definition,DraftField::Known{value} if value==&canonical) {
                    assert!(
                        matches!(
                            modifier.rolls.completion,
                            DraftListCompletion::Pending { .. }
                        ),
                        "raw cold inputs cannot close scalar eligibility"
                    );
                }
            }
        }
        let prior_draft = decode_draft(
            &fs::read(cwd.join(format!("cold-family-original-{case}/draft.json"))).unwrap(),
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
            non_cold_roll_closures(&prior_draft),
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
        let before = json(cwd.join(format!("cold-family-original-{case}/sidecar.json")));
        let after = json(destination.join("sidecar.json"));
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
                assert!(line["raw"] == old["raw"], "raw source text changed");
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
                if row["modifiers"].as_array().unwrap().is_empty() {
                    blocked += 1;
                    continue;
                }
                assert_eq!(row["modifiers"].as_array().unwrap().len(), 1);
                let source = item["source"]["ordinal"].as_u64().unwrap();
                let index = line["index"].as_u64().unwrap();
                let amount = match (case, source, index) {
                    (2, 482, 13) => 10.0,
                    (2, 519, 17) => 8.0,
                    (4, 192, 12) => 34.0,
                    unexpected => panic!("unexpected fixed admission: {unexpected:?}"),
                };
                let value = &row["outcome"]["value"]["emissions"][0]["value"];
                let modifier = LocatedItemModifier {
                    line: index as usize,
                    emission: 0,
                    definition: serde_json::from_value(value["definition"].clone()).unwrap(),
                    rolls: serde_json::from_value(value["rolls"].clone()).unwrap(),
                    rolls_closure: serde_json::from_value(value["rolls_closure"].clone()).unwrap(),
                };
                assert_raw(&modifier, family, amount);
                assert!(
                    !line["member"].is_null(),
                    "new admission needs a source-member proof"
                );
                assert!(line["blockers"].as_array().unwrap().is_empty());
            }
        }
        summary.push(serde_json::json!({"original":case,"query_rows":22,"normalization":"pending","admitted_before":previous.len(),"admitted_after":current.len(),"new_occurrences":current.difference(&previous).collect::<Vec<_>>(),"whole_build_parity":"not_established"}));
    }
    assert_eq!((total_before, total_after), (7, 10));
    assert_eq!((fixed, blocked), (22, 19));
    assert_eq!(
        new_occurrences,
        BTreeSet::from([(2, 482, 13, 0), (2, 519, 17, 0), (4, 192, 12, 0)])
    );
    fs::write(
        cwd.join("source-conditions-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}
pub fn check_source_conditions(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("source-condition-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let items = prior.join("items.json");
    let source = authored.join("item-source.json");
    let output = cwd.join("source-conditions-successor");
    let report = success(publish(cwd, prior, &authored, &items, &source, &output));
    for field in [
        "allocated_entries",
        "refined_subjects",
        "appended_tables",
        "appended_programs",
        "appended_receivers",
    ] {
        assert_eq!(report["extension"][field], 0, "{field}");
    }
    assert_eq!(
        report["publication"]["before"],
        report["publication"]["after"]
    );
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    let published = bundle(&output);
    assert!(
        published.keys().eq(prior_bytes.keys()),
        "bundle file set changed"
    );
    for (name, bytes) in &prior_bytes {
        if !matches!(
            name.as_str(),
            "item-source.json" | "manifest.json" | "transition.json" | "catalog-append.json"
        ) {
            assert!(
                published.get(name) == Some(bytes),
                "source-only publication changed {name}"
            );
        }
    }
    assert!(json(output.join("item-source.json")) == json(&source));
    check_policy(prior, &output);
    let family = family();
    check_source_cases(prior, &output, &family);
    assert!(
        !publish(cwd, prior, &authored, &items, &source, &output)
            .status
            .success()
    );
    equal_bundle(&bundle(&output), &published);
    let replay = cwd.join("source-conditions-replay");
    assert_eq!(
        success(publish(cwd, prior, &authored, &items, &source, &replay)),
        report
    );
    equal_bundle(&bundle(&replay), &published);
    let mut stale = json(&source);
    stale["item_lines"] =
        serde_json::to_value(digest_owned("stale-conditional-lines", &false, 1024).unwrap())
            .unwrap();
    let path = cwd.join("source-conditions-stale.json");
    fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let rejected = cwd.join("source-conditions-rejected");
    assert!(
        !publish(cwd, prior, &authored, &items, &path, &rejected)
            .status
            .success()
    );
    assert!(!rejected.exists());
    check_originals(cwd, prior, &output, &family);
    equal_bundle(&bundle(&output), &published);
    equal_bundle(&bundle(prior), &prior_bytes);
    equal_bundle(&bundle(&authored), &authored_bytes);
    output
}
