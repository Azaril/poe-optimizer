//! Original Life copy arithmetic and complete-build transport, kept offline.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/player_resource_source.rs"]
mod resource;
use poe_optimizer_pob::source as pinned;
use resource::{hash, observe};
use serde_json::{Value, json};
use std::{fs, path::Path};
const OBSERVER: &str = include_str!("support/amulet_life_copy_source.lua");

fn controlled(xml: &str, lines: &[&str], percent: u32) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc.descendants().find(|n| n.has_tag_name("Items")).unwrap();
    let active = items.attribute("activeItemSet").unwrap();
    let set = items
        .children()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some(active))
        .unwrap();
    let id = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Amulet"))
        .unwrap()
        .attribute("itemId")
        .unwrap();
    let item = items
        .children()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id))
        .unwrap();
    let text = item
        .children()
        .find(|n| n.is_text() && n.text().is_some_and(|t| t.contains("Rarity:")))
        .unwrap();
    // Append within the existing raw-item text, before ModRange children. A
    // second text node after those children replaces raw in PoB's XML loader.
    let at = text.range().end;
    let addition = format!("\n{}\n", lines.join("\n"));
    let mut edited = xml.to_owned();
    edited.insert_str(at, &addition);
    let mut inverse = edited.clone();
    inverse.replace_range(at..at + addition.len(), "");
    assert_eq!(inverse, xml);
    let doc = roxmltree::Document::parse(&edited).unwrap();
    let config = doc
        .descendants()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let active = config.attribute("activeConfigSet").unwrap();
    let set = config
        .children()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(active))
        .unwrap();
    assert!(
        !set.children()
            .any(|n| n.has_tag_name("CustomModifierBlock"))
    );
    let at = set.range().end - "</ConfigSet>".len();
    let addition = format!(
        "<CustomModifierBlock title=\"Amulet Life Copy Control\" enabled=\"true\">{percent}% increased bonuses gained from equipped rings and amulets</CustomModifierBlock>"
    );
    let before = edited.clone();
    edited.insert_str(at, &addition);
    let mut inverse = edited.clone();
    inverse.replace_range(at..at + addition.len(), "");
    assert_eq!(inverse, before);
    edited
}
fn numbers(v: &Value) -> Vec<f64> {
    if v.is_object() {
        assert!(v.as_object().unwrap().is_empty());
        return vec![];
    }
    v.as_array()
        .unwrap()
        .iter()
        .map(|m| m["value"].as_f64().unwrap())
        .collect()
}
// CalcPerform iterates the ring-copy slots with pairs. Their stored record
// sequence is diagnostic, not a stable comparison contract. Keep full raw
// observations separately; compare complete record multisets with multiplicity.
// Direct and copied Amulet sequences and every numerical result stay exact.
fn comparison(mut observation: Value) -> Value {
    for mode in ["MAIN", "CALCS"] {
        for store in observation["state"]["modes"][mode]["read_set"]
            .as_array_mut()
            .unwrap()
        {
            for channel in ["life", "amulet_effect"] {
                if let Some(records) = store[channel].as_array_mut() {
                    records.sort_by_cached_key(Value::to_string);
                } else {
                    assert!(store[channel].as_object().unwrap().is_empty());
                }
            }
        }
    }
    observation
}
fn child(root: &Path, out: &Path, jit: bool) {
    let originals = resource::originals(root);
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, x)| (format!("original-{:02}", i + 1), x.clone(), None))
        .collect();
    for (name, lines, percent, copied) in [
        ("zero-copy", vec!["+17 to maximum Life"], 0, vec![0.]),
        ("quarter-copy", vec!["+17 to maximum Life"], 25, vec![4.]),
        ("identity-copy", vec!["+17 to maximum Life"], 100, vec![17.]),
        (
            "duplicate-copy",
            vec!["+17 to maximum Life", "+19 to maximum Life"],
            25,
            vec![4., 4.],
        ),
    ] {
        let xml = controlled(&originals[4], &lines, percent);
        cases.push((
            name.to_owned(),
            xml,
            Some(json!({"lines":lines,"percent":percent,"copied":copied})),
        ));
    }
    cases.push(("fresh-repeat".to_owned(), originals[4].clone(), None));
    let mut rows = Vec::new();
    for (name, xml, control) in cases {
        let observed = observe(root, &xml, None, jit, OBSERVER);
        fs::write(
            out.join(format!(
                "case-{}-{name}.json",
                if jit { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&observed).unwrap(),
        )
        .unwrap();
        let state = &observed["state"];
        assert_eq!(state["source_methods_replaced"], false);
        assert_eq!(state["life_precision_present"], false);
        assert_eq!(state["default_precision"], 1);
        assert_eq!(state["modes"]["MAIN"], state["modes"]["CALCS"], "{name}");
        if let Some(c) = &control {
            let main = &state["modes"]["MAIN"];
            assert_eq!(main["amulet_type"], "Amulet");
            assert_eq!(
                main["amulet_percent"].as_f64().unwrap(),
                c["percent"].as_f64().unwrap()
            );
            assert_eq!(
                numbers(&main["copied"]),
                c["copied"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_f64().unwrap())
                    .collect::<Vec<_>>(),
                "{name}"
            );
            assert_eq!(
                numbers(&main["direct"]),
                if name == "duplicate-copy" {
                    vec![17., 19.]
                } else {
                    vec![17.]
                }
            );
        }
        let expected = [0., 4., -4., 17., 17., 4.3, -4.4, 17.5, 17.];
        let probes = state["probes"].as_array().unwrap();
        assert_eq!(probes.len(), expected.len());
        for (p, e) in probes.iter().zip(expected) {
            assert_eq!(
                p["output"]["value"].as_f64().unwrap(),
                e,
                "{name}: {}",
                p["name"]
            );
        }
        rows.push(json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"control":control,"observed":comparison(observed)}));
    }
    assert_eq!(rows[4]["observed"], rows[9]["observed"]);
    let changed = controlled(
        &originals[4],
        &["+17 to maximum Life", "+19 to maximum Life"],
        25,
    );
    let warm = observe(root, &originals[4], Some(&changed), jit, OBSERVER);
    fs::write(
        out.join(format!(
            "case-{}-warm-restoration.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&warm).unwrap(),
    )
    .unwrap();
    let warm = comparison(warm);
    assert_eq!(rows[4]["observed"], warm);
    let report = json!({"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVER.as_bytes()),"driver_sha256":hash(include_bytes!("support/player_resource_source.rs")),
        "witness_sha256":hash(include_bytes!("owned_amulet_life_copy.rs")),
        "files":(["src/Modules/CalcSetup.lua","src/Modules/CalcPerform.lua","src/Modules/Common.lua","src/Modules/Data.lua","src/Modules/ModParser.lua","src/Classes/ModStore.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "cases":rows,"warm_restoration":warm,"complete_loads":12,"scalar_probes_per_observation":9,
        "comparison_contract":{"read_set":"complete source-qualified record multiset","raw_sequence":"retained in per-mode case diagnostics","numerics":"exact","amulet_sequences":"exact","source_sequence_parity":false},
        "native_copy_implemented":false,"whole_build_parity":false});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
fn comparison_preserves_duplicate_sources_values_and_amulet_order() {
    let record = |value| json!({"source":"same source","value":value});
    let input = |records| json!({"state":{"modes":{"MAIN":{"read_set":[{"life":records,"amulet_effect":{}}],"copied":[record(1),record(2)]},"CALCS":{"read_set":[]}}}});
    let first = input(vec![record(1), record(2), record(1)]);
    assert_eq!(
        comparison(first.clone()),
        comparison(input(vec![record(1), record(1), record(2)]))
    );
    assert_ne!(
        comparison(first.clone()),
        comparison(input(vec![record(1), record(2)]))
    );
    assert_ne!(
        comparison(first.clone()),
        comparison(input(vec![record(1), record(3), record(1)]))
    );
    let mut reversed = first.clone();
    reversed["state"]["modes"]["MAIN"]["copied"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_ne!(comparison(first), comparison(reversed));
}
#[test]
fn control_preserves_saved_item_structure_and_edits_one_raw_text_node() {
    let original = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let edited = controlled(
        original,
        &["+17 to maximum Life", "+19 to maximum Life"],
        25,
    );
    let before = roxmltree::Document::parse(original).unwrap();
    let after = roxmltree::Document::parse(&edited).unwrap();
    let mut changed = 0;
    for item in before.descendants().filter(|n| n.has_tag_name("Item")) {
        let id = item.attribute("id");
        let next = after
            .descendants()
            .find(|n| n.has_tag_name("Item") && n.attribute("id") == id)
            .unwrap();
        let text = |node: roxmltree::Node<'_, '_>| {
            node.children()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .filter(|t| !t.trim().is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let prior = text(item);
        let current = text(next);
        assert_eq!(prior.len(), 1);
        assert_eq!(
            current.len(),
            1,
            "raw item must not be split around ModRange children"
        );
        if current != prior {
            changed += 1;
            assert_eq!(
                current[0],
                format!("{}\n+17 to maximum Life\n+19 to maximum Life\n", prior[0])
            );
        }
        let children = |node: roxmltree::Node<'_, '_>, source: &str| {
            node.children()
                .filter(|n| n.is_element())
                .map(|n| source[n.range()].to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(children(item, original), children(next, &edited));
    }
    assert_eq!(changed, 1);
}
#[test]
#[ignore = "requires pinned PoB and fresh AMULET_LIFE_SOURCE_OUT"]
fn original_life_copy_transport_and_rounding_are_stable() {
    resource::supervise(
        "original_life_copy_transport_and_rounding_are_stable",
        "POE_AMULET_LIFE_SOURCE_CHILD",
        "POE_OPTIMIZER_TEST_AMULET_LIFE_SOURCE_OUT",
        child,
    );
}
