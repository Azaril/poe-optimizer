//! Real package publication and source-template binding for catalyst inputs.
//! These checks prove admitted input transport, not final weapon calculations.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::ParameterValue,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemTextConversion, OwnedItemLinePolicy},
    owned_item_source::{
        ItemLayoutStatus, ItemRangeAttribution, ItemSourceDefaultScope, ItemSourceLayoutPolicy,
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

fn publish(cwd: &Path, prior: &Path, authored: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(authored.join("extension.json"))
        .arg("--items")
        .arg(authored.join("items.json"))
        .arg("--item-source")
        .arg(authored.join("item-source.json"))
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

fn attribute(
    base: &str,
    headers: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nCatalyst Test\n{base}\n{headers}Implicits: 0\n</Item></Items></PathOfBuilding2>"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([93; 16]),
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

fn assignments(converted: &ItemTextConversion<'_>) -> BTreeMap<String, Value> {
    let mut result = BTreeMap::new();
    for assignment in converted
        .parameters
        .iter()
        .map(|p| &p.assignment)
        .chain(&converted.defaults.parameters)
    {
        let slot = serde_json::to_string(&assignment.slot).unwrap();
        assert!(
            result
                .insert(slot, serde_json::to_value(&assignment.value).unwrap())
                .is_none(),
            "duplicate parameter escaped conversion"
        );
    }
    result
}

fn slot_key(binding: &Value, name: &str) -> String {
    // Round-trip through the typed declaration so JSON object ordering cannot
    // affect comparison with actual conversion output.
    let slot: poe_optimizer_core::owned_build::DeclaredSlot<
        poe_optimizer_core::owned_definitions::ParameterSlotDefId,
    > = serde_json::from_value(binding[name].clone()).unwrap();
    serde_json::to_string(&slot).unwrap()
}

fn check_headers(lines: &OwnedItemLinePolicy, source: &ItemSourceLayoutPolicy, bindings: &Value) {
    for (base, template, proven) in [
        ("Sapphire Ring", "def.00000000000009dc", true),
        ("Grand Spear", "def.0000000000001ed9", false),
    ] {
        let binding = bindings["bindings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["template"]["key"] == template)
            .unwrap();
        let selection = slot_key(binding, "selection");
        let amount = slot_key(binding, "amount");
        for (headers, selected, quantity) in [
            ("", false, None),
            ("CatalystQuality: 37\n", false, Some(37.0)),
            ("Catalyst: Tul's\n", true, None),
            ("Catalyst: Tul's\nCatalystQuality: 0\n", true, Some(0.0)),
            ("Catalyst: Tul's\nCatalystQuality: 20\n", true, Some(20.0)),
            // This is the source computational range, not crafting legality.
            ("Catalyst: Tul's\nCatalystQuality: -1\n", true, Some(-1.0)),
        ] {
            let attributed = attribute(base, headers, source, lines);
            assert_eq!(
                matches!(attributed.report().layout, ItemLayoutStatus::Proven),
                proven,
                "{base}: {headers}"
            );
            assert_eq!(
                matches!(
                    attributed.report().default_scope,
                    ItemSourceDefaultScope::Proven { .. }
                ),
                proven
            );
            let converted = attributed.convert(lines).unwrap();
            let actual = assignments(&converted);
            assert_eq!(
                actual.len(),
                usize::from(selected || proven) + usize::from(quantity.is_some() || proven),
                "{base}: {headers}: {converted:?}"
            );
            if selected || proven {
                assert_eq!(actual[&selection]["kind"], "option");
                assert_eq!(
                    actual[&selection]["value"]["key"],
                    if selected {
                        "def.00000000000009f1"
                    } else {
                        "def.00000000000009eb"
                    }
                );
            }
            if quantity.is_some() || proven {
                assert_eq!(actual[&amount]["kind"], "quantity");
                assert_eq!(
                    actual[&amount]["value"]["value"].as_f64().unwrap(),
                    quantity.unwrap_or(20.0)
                );
            }
            assert_eq!(converted.defaults.item_level_absent, proven);
            assert_eq!(converted.defaults.quality_absent, proven);
            if !proven {
                assert!(converted.defaults.parameters.is_empty());
                assert!(
                    actual
                        .keys()
                        .all(|k| k != &slot_key(&bindings["bindings"][0], "selection")
                            && k != &slot_key(&bindings["bindings"][0], "amount")),
                    "weapon must not fall back to Sapphire slots"
                );
            }
        }
        // Present unresolved headers suppress absence defaults; a duplicate
        // cannot quietly become either a selected value or its default.
        for (headers, rejected_slot) in [
            ("Catalyst: unknown\n", &selection),
            ("CatalystQuality: NaN\n", &amount),
            ("CatalystQuality: 1000001\n", &amount),
            ("CatalystQuality: 20 trailing\n", &amount),
            ("Catalyst: Tul's\nCatalyst: Tul's\n", &selection),
            ("Catalyst: Tul's\nCatalyst: unknown\n", &selection),
            ("CatalystQuality: 20\nCatalystQuality: 0\n", &amount),
            ("CatalystQuality: 20\nCatalystQuality: bad\n", &amount),
        ] {
            let attributed = attribute(base, headers, source, lines);
            let converted = attributed.convert(lines).unwrap();
            assert!(
                !assignments(&converted).contains_key(rejected_slot),
                "{base}: {headers}: {converted:?}"
            );
            assert!(
                !converted.issues.is_empty()
                    || converted.lines.iter().any(|line| matches!(
                        line.outcome,
                        poe_optimizer_import::owned_item_lines::ItemLineOutcome::Pending { .. }
                    ))
            );
        }
    }
    for text in [
        "Catalyst: Tul's\nCatalystQuality: 20",
        "Emerald\nCatalyst: Tul's\nCatalystQuality: 20",
        "Sapphire Ring\nGrand Spear\nCatalyst: Tul's\nCatalystQuality: 20",
    ] {
        let converted = lines.convert_text(text).unwrap();
        assert!(
            converted.parameters.is_empty(),
            "unbound/ambiguous template: {converted:?}"
        );
        assert!(converted.defaults.parameters.is_empty());
        assert!(!converted.issues.is_empty());
    }
}

fn check_extension(prior: &Path, output: &Path, bindings: &Value) {
    let targets = bindings["bindings"].as_array().unwrap();
    assert_eq!(targets.len(), 338);
    let weapons: BTreeSet<_> = targets
        .iter()
        .filter(|b| b["existing"] == false)
        .map(|b| b["template"]["key"].as_str().unwrap())
        .collect();
    assert_eq!(weapons.len(), 337);
    let before = json(prior.join("registry.json"));
    let after = json(output.join("registry.json"));
    assert_eq!(before["entries"].as_array().unwrap().len(), 9854);
    assert_eq!(after["entries"].as_array().unwrap().len(), 10528);
    for (index, (old, new)) in before["entries"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["entries"].as_array().unwrap())
        .enumerate()
    {
        assert!(old == new, "registry entry {index} changed");
    }
    let before = json(prior.join("schema.json"));
    let after = json(output.join("schema.json"));
    assert_eq!(
        before["definitions"].as_array().unwrap().len(),
        after["definitions"].as_array().unwrap().len()
    );
    for (old, new) in before["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["definitions"].as_array().unwrap())
    {
        assert_eq!(new["value"]["id"], old["value"]["id"]);
        let key = old["value"]["id"]["key"].as_str().unwrap();
        if weapons.contains(key) {
            let binding = targets
                .iter()
                .find(|b| b["template"]["key"] == key)
                .unwrap();
            let path = "/value/schema/value/declarations/parameters";
            let previous = old.pointer(path).unwrap();
            let next = new.pointer(path).unwrap();
            assert_eq!(previous["members"], serde_json::json!([]));
            assert_eq!(next["closure"], previous["closure"]);
            assert_eq!(next["closure"]["kind"], "partial");
            assert_eq!(
                next["members"],
                serde_json::json!([binding["selection"], binding["amount"]])
            );
            let mut unchanged = new.clone();
            *unchanged.pointer_mut(path).unwrap() = previous.clone();
            assert!(
                &unchanged == old,
                "definition {key}: only catalyst declaration membership may grow"
            );
        } else {
            assert!(
                new == old,
                "existing descriptor changed: {}",
                old["value"]["id"]
            );
        }
    }
    let old_slots = before["slots"].as_array().unwrap();
    let new_slots = after["slots"].as_array().unwrap();
    assert_eq!(new_slots.len(), old_slots.len() + 674);
    // Schema publication sorts typed addresses: added item-template parameter
    // slots can precede existing modifier/other slots. Identity, not position,
    // determines whether a previous descriptor was preserved.
    let indexed_slots: BTreeMap<_, _> = new_slots
        .iter()
        .map(|slot| (serde_json::to_string(&slot["value"]["id"]).unwrap(), slot))
        .collect();
    assert_eq!(indexed_slots.len(), new_slots.len());
    for old in old_slots {
        let id = serde_json::to_string(&old["value"]["id"]).unwrap();
        assert!(
            indexed_slots.get(&id).is_some_and(|new| *new == old),
            "old slot descriptor changed: {id}"
        );
    }
    for binding in targets {
        for name in ["selection", "amount"] {
            let slot = new_slots
                .iter()
                .find(|s| s["kind"] == "parameter" && s["value"]["id"] == binding[name])
                .unwrap();
            assert_eq!(slot["value"]["schema"]["kind"], "known");
            assert_eq!(
                slot["value"]["schema"]["value"]["presence"],
                "required_once"
            );
            assert_eq!(
                slot["value"]["schema"]["value"]["sites"],
                serde_json::json!(["item_parameter"])
            );
            assert_eq!(
                slot["value"]["schema"]["value"]["value"]["kind"],
                if name == "selection" {
                    "option"
                } else {
                    "quantity"
                }
            );
        }
    }
    let before = json(prior.join("rules.json"));
    let after = json(output.join("rules.json"));
    assert!(after["tables"] == before["tables"], "rule tables changed");
    assert!(
        after["receivers"] == before["receivers"],
        "rule receivers changed"
    );
    assert_eq!(
        after["owners"].as_array().unwrap().len(),
        before["owners"].as_array().unwrap().len()
    );
    let mut new_programs = 0;
    for (old, new) in before["owners"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["owners"].as_array().unwrap())
    {
        assert_eq!(new["owner"], old["owner"]);
        let key = old["owner"]["value"]["value"]["key"].as_str();
        if key.is_some_and(|k| weapons.contains(k)) {
            assert_eq!(new["programs"]["closure"], old["programs"]["closure"]);
            let old_members = old["programs"]["members"].as_array().unwrap();
            let members = new["programs"]["members"].as_array().unwrap();
            assert_eq!(members.len(), old_members.len() + 1);
            assert!(old_members.iter().all(|p| members.contains(p)));
            let program = members
                .iter()
                .find(|p| p["id"] == "catalyst-inputs")
                .unwrap();
            assert_eq!(program["context"], "equipment_use");
            assert_eq!(program["reads"].as_array().unwrap().len(), 2);
            assert_eq!(program["effects"].as_array().unwrap().len(), 2);
            new_programs += 1;
        } else {
            assert!(
                new == old,
                "existing descriptor changed: {}",
                old["value"]["id"]
            );
        }
    }
    assert_eq!(new_programs, 337);
    let before = json(prior.join("routing.json"));
    let mut after = json(output.join("routing.json"));
    after["definitions"] = before["definitions"].clone();
    assert!(
        after == before,
        "routing only rebinds the new schema identity"
    );
    let items = json(output.join("items.json"));
    for (rule, name) in [
        ("catalyst-kind", "selection"),
        ("catalyst-amount", "amount"),
    ] {
        let emission = &items["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == rule)
            .unwrap()["emissions"][0];
        assert_eq!(emission["kind"], "template_parameter");
        let actual = emission["value"]["bindings"].as_array().unwrap();
        assert_eq!(actual.len(), 338);
        assert!(targets.iter().all(|target| actual.contains(&target[name])));
    }
    let source = json(output.join("item-source.json"));
    assert_eq!(source["template_defaults"].as_array().unwrap().len(), 338);
    for target in targets {
        let defaults = source["template_defaults"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["template"] == target["template"])
            .unwrap();
        assert_eq!(defaults["parameters"].as_array().unwrap().len(), 2);
        for (index, name, header) in [
            (0, "selection", "Catalyst"),
            (1, "amount", "CatalystQuality"),
        ] {
            assert_eq!(
                defaults["parameters"][index]["assignment"]["slot"],
                target[name]
            );
            assert_eq!(
                defaults["parameters"][index]["headers"],
                serde_json::json!([header])
            );
        }
        assert_eq!(
            defaults["quality"],
            if target["existing"] == true {
                "absent"
            } else {
                "pending"
            }
        );
        assert_eq!(defaults["item_level"], defaults["quality"]);
    }
}

fn check_originals(cwd: &Path, prior: &Path, package: &Path) {
    let mut queries = 0;
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert!(
            fs::read(package.join(&name)).unwrap() == fs::read(prior.join(&name)).unwrap(),
            "query file {name} changed"
        );
        queries += json(package.join(name)).as_array().unwrap().len();
        let destination = cwd.join(format!("weapon-catalyst-original-{case}"));
        assert_eq!(
            success(normalize(cwd, package, case, &destination, true))["normalization_status"],
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
        assert!(!input.items.members.is_empty());
        assert!(!input.allocations.members.is_empty());
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        assert!(input.items.members.iter().any(|item| matches!(
            item.modifiers.completion,
            DraftListCompletion::Pending { .. }
        )));
        assert!(
            input
                .items
                .members
                .iter()
                .any(|item| matches!(item.modifier_order, DraftField::Pending(_)))
        );
        for item in &input.items.members {
            for parameter in &item.parameters.members {
                if let (Some(template), Some(slot)) =
                    (item.template.to_resolved(), parameter.slot.to_resolved())
                {
                    assert_eq!(
                        serde_json::to_value(&slot).unwrap()["declaration"]["definition"],
                        serde_json::to_value(template).unwrap(),
                        "parameter must belong to the selected item template"
                    );
                }
                if let Some(ParameterValue::Quantity(value)) = parameter.value.to_resolved() {
                    assert!(value.value().is_finite());
                }
            }
        }
        let sidecar = json(destination.join("sidecar.json"));
        let previous = json(cwd.join(format!("canonical-admission-original-{case}/sidecar.json")));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(sidecar[field], previous[field]);
        }
    }
    assert_eq!(queries, 110);
}

fn assert_bundle_equal(actual: &BTreeMap<String, Vec<u8>>, expected: &BTreeMap<String, Vec<u8>>) {
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

pub fn check_weapon_catalysts(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("weapon-catalyst-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let output = cwd.join("weapon-catalyst-successor");
    let report = success(publish(cwd, prior, &authored, &output));
    assert_eq!(report["extension"]["allocated_entries"], 674);
    assert_eq!(report["extension"]["refined_subjects"], 337);
    assert_eq!(report["extension"]["appended_programs"], 337);
    assert_eq!(report["extension"]["appended_tables"], 0);
    assert_eq!(report["extension"]["appended_receivers"], 0);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    for name in ["items.json", "item-source.json"] {
        assert!(
            json(output.join(name)) == json(authored.join(name)),
            "published {name} differs from authored input"
        );
    }
    let bindings = json(authored.join("bindings.json"));
    check_extension(prior, &output, &bindings);
    let checked = assemble_owned_recipe(recipe(&output), Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(lines.input().schema_version, 5);
    let source = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(output.join("item-source.json"))).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(source.input().item_lines, *lines.identity());
    check_headers(&lines, &source, &bindings);
    let published = bundle(&output);
    assert!(!publish(cwd, prior, &authored, &output).status.success());
    assert_bundle_equal(&bundle(&output), &published);
    let replay = cwd.join("weapon-catalyst-replay");
    assert_eq!(success(publish(cwd, prior, &authored, &replay)), report);
    assert_bundle_equal(&bundle(&replay), &published);
    check_originals(cwd, prior, &output);
    assert_bundle_equal(&bundle(prior), &prior_bytes);
    assert_bundle_equal(&bundle(&authored), &authored_bytes);
    assert_bundle_equal(&bundle(&output), &published);
    output
}
