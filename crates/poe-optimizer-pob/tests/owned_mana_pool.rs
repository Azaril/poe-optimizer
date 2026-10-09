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
    ];
    for (name, text, expected) in controls {
        cases.push((
            name.to_owned(),
            controlled(&originals[4], text),
            Some(text.to_owned()),
            Some(expected),
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
        if let Some(expected)=expected {assert_eq!(main["final_mana"].as_f64().unwrap(),*expected,"{name}");}
        json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"control":control,"observed":observed})
    }).collect();
    assert_eq!(rows[4]["observed"], rows[10]["observed"]);
    assert_eq!(rows[4]["observed"], rows[11]["observed"]);
    let warm = observe(root, &originals[4], Some(&warm_xml), jit, OBSERVER);
    assert_eq!(warm, rows[11]["observed"]);
    let vectors:Vec<Value>=rows.iter().map(|r|json!({"name":r["name"],"xml_sha256":r["xml_sha256"],"control":r["control"],"source":r["observed"]["state"]["modes"]["MAIN"]})).collect();
    let report = json!({"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVER.as_bytes()),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),"driver_sha256":hash(include_bytes!("support/player_resource_source.rs")),
        "files":(["src/Modules/CalcDefence.lua","src/Modules/ModParser.lua","src/Classes/ModStore.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "cases":rows,"vectors":vectors,"warm_repeat":warm,"complete_loads":15,"native_final_mana":false,"whole_build":false});
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
