//! Original Mana emission under parsed suppression, doubling and zero controls.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/player_resource_source.rs"]
mod resource;
use poe_optimizer_pob::source as pinned;
use resource::{hash, observe};
use serde_json::{Value as Json, json};
use std::{fs, path::Path};
const INTRINSIC: &str = include_str!("support/player_intrinsic_mana_source.lua");
const ATTRIBUTE: &str = include_str!("support/intelligence_mana_source.lua");
const FLAGS: [&str; 4] = [
    "NoAttributeBonuses",
    "NoIntelligenceAttributeBonuses",
    "NoIntBonusToMana",
    "DoubledInherentAttributeBonuses",
];
const DOUBLE: &str = "Inherent bonuses gained from Attributes are doubled";
const NO_ALL: &str = "Gain no inherent bonuses from Attributes";
const NO_INT: &str = "Gain no inherent bonuses from Intelligence";
const NO_MANA: &str = "Intelligence provides no inherent bonus to maximum Mana";
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
        "<CustomModifierBlock title=\"Intelligence Mana Control\" enabled=\"true\">{text}</CustomModifierBlock>"
    );
    let mut next = xml.to_owned();
    next.insert_str(at, &addition);
    let mut inverse = next.clone();
    inverse.replace_range(at..at + addition.len(), "");
    assert_eq!(inverse, xml);
    next
}
fn records(mana: &Json) -> Vec<&Json> {
    mana["read_set"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["mana"].as_array().into_iter().flatten())
        .filter(|m| m["source"] == "Intelligence" && m["type"] == "BASE")
        .collect()
}
fn check(row: &Json) {
    for mode in ["MAIN", "CALCS"] {
        let attr = &row["observed"]["state"]["attribute"]["modes"][mode];
        let mana = &row["observed"]["state"]["intrinsic"]["modes"][mode];
        let int = attr["intelligence"].as_u64().unwrap();
        let flags: Vec<_> = FLAGS
            .iter()
            .map(|f| attr["flags"][f]["resolved"].as_bool().unwrap())
            .collect();
        let emitted = records(mana);
        let disabled = flags[..3].iter().any(|v| *v);
        assert_eq!(emitted.len(), usize::from(!disabled));
        // This expectation checks the source; the observed original Sum and
        // exact emitted records, not this formula, supply the parity vectors.
        let expected = if disabled {
            0
        } else {
            int * 2 * if flags[3] { 2 } else { 1 }
        };
        assert_eq!(attr["amount"], expected);
        if let Some(m) = emitted.first() {
            assert_eq!(m["name"], "Mana");
            assert_eq!(m["value"], attr["amount"]);
            assert_eq!(m["flags"], 0);
            assert_eq!(m["keyword_flags"], 0);
            assert_eq!(m["tags"], json!({}));
        }
    }
}
fn child(root: &Path, out: &Path, jit: bool) {
    let originals = resource::originals(root);
    // Only trusted static observer code is composed. Build XML remains data.
    let observer = format!(
        "local intrinsic=(function()\n{INTRINSIC}\nend)();local attribute=(function()\n{ATTRIBUTE}\nend)();return {{intrinsic=intrinsic,attribute=attribute}}"
    );
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, xml)| (format!("original-{:02}", i + 1), xml.clone(), None, None))
        .collect();
    let controls = [
        ("double", DOUBLE.to_owned(), vec![3], 105, 420, 1),
        ("disable-all", NO_ALL.to_owned(), vec![0], 105, 0, 0),
        (
            "disable-intelligence",
            NO_INT.to_owned(),
            vec![1],
            105,
            0,
            0,
        ),
        ("disable-mana", NO_MANA.to_owned(), vec![2], 105, 0, 0),
        (
            "disable-all-and-double",
            format!("{NO_ALL}\n{DOUBLE}"),
            vec![0, 3],
            105,
            0,
            0,
        ),
        (
            "disable-intelligence-and-double",
            format!("{NO_INT}\n{DOUBLE}"),
            vec![1, 3],
            105,
            0,
            0,
        ),
        (
            "disable-mana-and-double",
            format!("{NO_MANA}\n{DOUBLE}"),
            vec![2, 3],
            105,
            0,
            0,
        ),
        (
            "zero-intelligence",
            "-1000000 to Intelligence".into(),
            vec![],
            0,
            0,
            1,
        ),
        (
            "zero-intelligence-and-double",
            format!("-1000000 to Intelligence\n{DOUBLE}"),
            vec![3],
            0,
            0,
            1,
        ),
        (
            "duplicate-double",
            format!("{DOUBLE}\n{DOUBLE}"),
            vec![3],
            105,
            420,
            1,
        ),
    ];
    for (name, text, _, _, _, _) in &controls {
        cases.push((
            format!("original-05-{name}"),
            controlled(&originals[4], text),
            Some(text.clone()),
            None,
        ));
    }
    cases.push((
        "repeat-original-05".into(),
        originals[4].clone(),
        None,
        None,
    ));
    cases.push((
        "warm-suppressed-to-original-05".into(),
        originals[4].clone(),
        None,
        Some(controlled(&originals[4], NO_ALL)),
    ));
    let mode = if jit { "on" } else { "off" };
    let mut observed = vec![];
    for (name, xml, control, warm) in &cases {
        eprintln!("Intelligence Mana {name}, JIT {mode}");
        let actual = observe(root, xml, warm.as_deref(), jit, &observer);
        let row = json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"control":control,"warm_xml_sha256":warm.as_ref().map(|x|hash(x.as_bytes())),"observed":actual});
        fs::write(
            out.join(format!("{mode}-{name}.json")),
            serde_json::to_vec_pretty(&row).unwrap(),
        )
        .unwrap();
        check(&row);
        observed.push(row);
    }
    for (i, (_, _, active, int, amount, count)) in controls.iter().enumerate() {
        for mode in ["MAIN", "CALCS"] {
            let state = &observed[i + 5]["observed"]["state"];
            let a = &state["attribute"]["modes"][mode];
            assert_eq!(a["intelligence"], *int);
            assert_eq!(a["amount"], *amount);
            assert_eq!(records(&state["intrinsic"]["modes"][mode]).len(), *count);
            for (f, name) in FLAGS.iter().enumerate() {
                assert_eq!(a["flags"][name]["resolved"], active.contains(&f));
            }
        }
    }
    assert_eq!(observed[4]["observed"], observed[15]["observed"]);
    assert_eq!(observed[4]["observed"], observed[16]["observed"]);
    let warm = observe(
        root,
        &originals[4],
        Some(&controlled(&originals[4], NO_ALL)),
        jit,
        &observer,
    );
    assert_eq!(warm, observed[16]["observed"]);
    let vectors:Vec<_>=observed.iter().map(|r|{
        let state=&r["observed"]["state"];let a=&state["attribute"]["modes"]["MAIN"];
        json!({"name":r["name"],"xml_sha256":r["xml_sha256"],"source_only_control":r["control"].is_string(),
            "intelligence":a["intelligence"],"flags":FLAGS.iter().map(|f|((*f).to_string(),a["flags"][f]["resolved"].clone())).collect::<serde_json::Map<_,_>>(),
            "amount":a["amount"],"emitted_records":records(&state["intrinsic"]["modes"]["MAIN"])})
    }).collect();
    let report = json!({"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observers":[{"path":"crates/poe-optimizer-pob/tests/support/player_intrinsic_mana_source.lua","sha256":hash(INTRINSIC.as_bytes())},{"path":"crates/poe-optimizer-pob/tests/support/intelligence_mana_source.lua","sha256":hash(ATTRIBUTE.as_bytes())}],
        "bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),"driver_sha256":hash(include_bytes!("support/player_resource_source.rs")),
        "files":(["src/Modules/CalcPerform.lua","src/Modules/ModParser.lua","src/Classes/ModStore.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "cases":observed,"vectors":vectors,"warm_repeat":warm,"complete_loads":20,
        "final_mana_claim":false,"game_flag_source_closure":false});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires pinned PoB; independent complete-source controls"]
fn original_intelligence_mana_controls_and_replays_match() {
    resource::supervise(
        "original_intelligence_mana_controls_and_replays_match",
        "POE_INTELLIGENCE_MANA_SOURCE_CHILD",
        "POE_OPTIMIZER_TEST_INTELLIGENCE_MANA_SOURCE_OUT",
        child,
    );
}
