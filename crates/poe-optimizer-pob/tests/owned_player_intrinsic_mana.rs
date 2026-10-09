//! Full-source observations of intrinsic Mana, without modifying source methods.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/player_resource_source.rs"]
mod resource;
use poe_optimizer_pob::source as pinned;
use resource::hash;
use serde_json::{Value as Json, json};
use std::{fs, path::Path};
const OBSERVER: &str = include_str!("support/player_intrinsic_mana_source.lua");
const OUTPUT: &str = "POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_MANA_SOURCE_OUT";
const CHILD: &str = "POE_PLAYER_INTRINSIC_MANA_SOURCE_CHILD";
const TEST: &str = "original_intrinsic_mana_is_stable_across_builds_levels_and_jit";
fn observe(root: &Path, xml: &str, warm: Option<&str>, jit: bool) -> Json {
    resource::observe(root, xml, warm, jit, OBSERVER)
}
fn at_level(xml: &str, level: u64) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let build = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    assert_eq!(build.attribute("characterLevelAutoMode"), Some("false"));
    let range = build
        .attributes()
        .find(|a| a.name() == "level")
        .unwrap()
        .range();
    let mut next = xml.to_owned();
    next.replace_range(range.clone(), &format!("level=\"{level}\""));
    let d = roxmltree::Document::parse(&next).unwrap();
    let new_range = d
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap()
        .attributes()
        .find(|a| a.name() == "level")
        .unwrap()
        .range();
    let mut inverse = next.clone();
    inverse.replace_range(new_range, &xml[range]);
    assert_eq!(inverse, xml);
    next
}
fn check(report: &Json, level: u64, expected: u64) {
    let state = &report["state"];
    assert_eq!(state["constant"], 4);
    for mode in ["MAIN", "CALCS"] {
        let m = &state["modes"][mode];
        assert_eq!(m["level"], level);
        assert_eq!(m["amount"], expected);
        assert_eq!(m["base_source_sum"], expected);
        assert_eq!(
            m["record"],
            json!({"name":"Mana","type":"BASE","value":4,"source":"Base",
            "flags":0,"keyword_flags":0,"tags":[{"type":"Multiplier","var":"Level","base":30}]})
        );
        assert_eq!(
            m["multiplier"],
            json!({"effective":level,"extra":0,"override":{"present":false}})
        );
        for row in m["read_set"].as_array().unwrap() {
            assert_eq!(row["level_modifiers"], json!({}));
        }
    }
    for field in [
        "original_methods",
        "repeated_equal",
        "input_and_output_preserved",
    ] {
        assert_eq!(state[field], true);
    }
    assert_eq!(state["final_pool_claim"], false);
    assert_eq!(state["contributor_closure"], false);
}
fn child(root: &Path, output: &Path, jit: bool) {
    let originals = resource::originals(root);
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .zip([(96, 414), (88, 382), (93, 402), (96, 414), (92, 398)])
        .map(|((i, xml), (level, expected))| {
            (
                format!("original-{:02}", i + 1),
                xml.clone(),
                None,
                level,
                expected,
            )
        })
        .collect();
    for (level, expected) in [(1, 34), (91, 394), (100, 430)] {
        cases.push((
            format!("original-05-level-{level}"),
            at_level(&originals[4], level),
            None,
            level,
            expected,
        ));
    }
    cases.push((
        "repeat-original-05".into(),
        originals[4].clone(),
        None,
        92,
        398,
    ));
    cases.push((
        "warm-original-05".into(),
        originals[4].clone(),
        Some(at_level(&originals[4], 1)),
        92,
        398,
    ));
    let mut observations = vec![];
    let mode = if jit { "on" } else { "off" };
    for (name, xml, warm, level, expected) in &cases {
        eprintln!("intrinsic Mana {name}, JIT {mode}");
        let actual = observe(root, xml, warm.as_deref(), jit);
        let row = json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"warm_xml_sha256":warm.as_ref().map(|s|hash(s.as_bytes())),"observed":actual});
        fs::write(
            output.join(format!("{mode}-{name}.json")),
            serde_json::to_vec_pretty(&row).unwrap(),
        )
        .unwrap();
        check(&actual, *level, *expected);
        observations.push(row);
    }
    assert_eq!(observations[4]["observed"], observations[8]["observed"]);
    assert_eq!(observations[4]["observed"], observations[9]["observed"]);
    let warm = observe(root, &originals[4], Some(&at_level(&originals[4], 1)), jit);
    assert_eq!(warm, observations[9]["observed"]);
    let report = json!({"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVER.as_bytes()),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":(["src/Modules/CalcSetup.lua","src/Classes/ModStore.lua","src/Classes/ModDB.lua","src/Data/Misc.lua"].map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()}))),
        "cases":observations,"complete_loads":13,"warm_repeat":warm,"final_mana_claim":false});
    fs::write(
        output.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires pinned PoB; supervised fresh/warm complete-source comparison"]
fn original_intrinsic_mana_is_stable_across_builds_levels_and_jit() {
    resource::supervise(TEST, CHILD, OUTPUT, child);
}
