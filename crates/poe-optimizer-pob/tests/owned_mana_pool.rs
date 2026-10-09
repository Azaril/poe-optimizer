//! Complete original resource formula observations; no native completeness claim.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/player_resource_source.rs"]
mod resource;
use poe_optimizer_pob::source as pinned;
use resource::{hash, observe};
use serde_json::{Value, json};
use std::{fs, path::Path};
const OBSERVER: &str = include_str!("support/mana_pool_source.lua");
fn controlled(xml: &str, text: &str) -> String {
    assert!(!text.contains(['<', '>', '&', '"']));
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .root_element()
        .children()
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
    assert_eq!(&xml[at..set.range().end], "</ConfigSet>");
    let addition = format!(
        "<CustomModifierBlock title=\"Mana Pool Control\" enabled=\"true\">{text}</CustomModifierBlock>"
    );
    let mut next = xml.to_owned();
    next.insert_str(at, &addition);
    let mut inverse = next.clone();
    inverse.replace_range(at..at + addition.len(), "");
    assert_eq!(inverse, xml);
    next
}
fn allocated_override(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let tree = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Tree"))
        .unwrap();
    let active: usize = tree.attribute("activeSpec").unwrap().parse().unwrap();
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(active - 1)
        .unwrap();
    assert_eq!(spec.attribute("treeVersion"), Some("0_5"));
    let nodes = spec.attribute_node("nodes").unwrap();
    // The source removes disconnected allocations. This connected control is
    // pinned to the exported tree; it is not an optimizer path or budget proof.
    let path = [
        "7960", "23382", "59093", "4140", "33722", "47931", "32474", "29611", "41768", "28982",
        "55190", "30141", "51749",
    ];
    let tree: Value = serde_json::from_str(include_str!(
        "../../../data/owned/poe2/3887ae68/tree/tree-catalog.json"
    ))
    .unwrap();
    for edge in path.windows(2) {
        assert!(
            tree["edges"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| (e["left"] == edge[0] && e["right"] == edge[1])
                    || (e["left"] == edge[1] && e["right"] == edge[0]))
        );
    }
    assert!(nodes.value().split(',').any(|id| id == path[0]));
    for node in &path[1..] {
        assert!(!nodes.value().split(',').any(|id| id == *node));
    }
    let addition = format!(",{}", path[1..].join(","));
    let range = nodes.range_value();
    let mut changed = xml.to_owned();
    changed.replace_range(range.clone(), &format!("{}{addition}", nodes.value()));
    let mut inverse = changed.clone();
    inverse.replace_range(range.start..range.end + addition.len(), nodes.value());
    assert_eq!(inverse, xml);
    changed
}
fn child(root: &Path, out: &Path, jit: bool) {
    let originals = resource::originals(root);
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(n, x)| (format!("original-{:02}", n + 1), x.clone(), None, None))
        .collect();
    let controls = [
        ("flat-tie", "+2 to maximum Mana", 641.),
        ("increased", "10% increased maximum Mana", 699.),
        ("more", "50% more maximum Mana", 958.),
        ("reduced-clamp", "200% reduced maximum Mana", 1.),
        ("override-zero", "You have no Mana", 0.),
        (
            "override-zero-duplicates",
            "You have no Mana\nYou have no Mana",
            0.,
        ),
    ];
    for (name, text, expected) in controls {
        cases.push((
            name.to_owned(),
            controlled(&originals[4], text),
            Some(text.to_owned()),
            Some(expected),
        ));
    }
    cases.push((
        "allocated-override".to_owned(),
        allocated_override(&originals[4]),
        Some(
            "allocate connected path to tree node 51749; numeric control, no point-budget claim"
                .to_owned(),
        ),
        Some(0.),
    ));
    let conversion = "Convert 25% of maximum Energy Shield to maximum Mana";
    for (name, text, expected) in [
        ("incoming-conversion", conversion.to_owned(), None),
        (
            "incoming-conversion-zero-override",
            format!("{conversion}\nYou have no Mana"),
            Some(0.),
        ),
    ] {
        cases.push((
            name.to_owned(),
            controlled(&originals[4], &text),
            Some(text),
            expected,
        ));
    }
    cases.push(("fresh-repeat".to_owned(), originals[4].clone(), None, None));
    cases.push((
        "warm-restoration".to_owned(),
        originals[4].clone(),
        None,
        None,
    ));
    let warm_xml = controlled(&originals[4], "You have no Mana");
    let rows:Vec<_>=cases.iter().map(|(name,xml,control,expected)|{
        let warm=(name=="warm-restoration").then_some(warm_xml.as_str());
        let observed=observe(root,xml,warm,jit,OBSERVER);
        let main=&observed["state"]["modes"]["MAIN"];
        assert_eq!(main,&observed["state"]["modes"]["CALCS"]);
        if let Some(expected)=expected {assert_eq!(main["final_mana"].as_f64().unwrap(),*expected,"{name}: {} {}", observed["selected"], main["override_source"]);}
        if name.starts_with("incoming-conversion") {
            assert!(main["inputs"]["extra"].as_f64().unwrap()>0.,"conversion must really contribute");
            if name=="incoming-conversion" { assert!(main["final_mana"].as_f64().unwrap()>638.); }
        }
        if name=="allocated-override" {
            assert_eq!(main["override_source"]["allocated"], true);
            assert_eq!(main["inputs"]["override"], json!({"present":true,"value":0}));
        }
        if name=="override-zero-duplicates" {
            let count:usize=main["read_set"].as_array().unwrap().iter().map(|row| row["Mana"].as_array().unwrap().iter().filter(|m| m["type"]=="OVERRIDE").count()).sum();
            assert_eq!(count,2,"duplicate sources must really reach the original database");
        }
        json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"control":control,"observed":observed})
    }).collect();
    assert_eq!(rows[4]["observed"], rows[rows.len() - 2]["observed"]);
    assert_eq!(rows[4]["observed"], rows[rows.len() - 1]["observed"]);
    let warm = observe(root, &originals[4], Some(&warm_xml), jit, OBSERVER);
    assert_eq!(warm, rows[rows.len() - 1]["observed"]);
    let vectors:Vec<Value>=rows.iter().map(|r|json!({"name":r["name"],"xml_sha256":r["xml_sha256"],"control":r["control"],"source":r["observed"]["state"]["modes"]["MAIN"]})).collect();
    let report = json!({"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVER.as_bytes()),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),"driver_sha256":hash(include_bytes!("support/player_resource_source.rs")),
        "files":(["src/Modules/CalcDefence.lua","src/Modules/ModParser.lua","src/Classes/ModStore.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "cases":rows,"vectors":vectors,"warm_repeat":warm,"complete_loads":cases.len()+3,"native_final_mana":false,"whole_build":false});
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
#[ignore = "requires pinned PoB and fresh MANA_POOL_SOURCE_OUT"]
fn original_mana_pool_controls_and_replays_match() {
    resource::supervise(
        "original_mana_pool_controls_and_replays_match",
        "POE_MANA_POOL_SOURCE_CHILD",
        "POE_OPTIMIZER_TEST_MANA_POOL_SOURCE_OUT",
        child,
    );
}
