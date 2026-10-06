//! Original complete Player Strength bonus calculation and parsed control inputs.
//! This witnesses the inherent amount, not attribute closure or final Life.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVE: &str = include_str!("support/strength_life_source.lua");
const TEST: &str = "actual_strength_life_bonus_preserves_original_stage_and_controls";
const CHILD: &str = "POE_STRENGTH_LIFE_SOURCE_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_STRENGTH_LIFE_SOURCE_OUT";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(array) = value.as_array() {
        array
    } else {
        assert!(value.as_object().is_some_and(|x| x.is_empty()));
        &[]
    }
}
fn observed(root: &Path, xml: &str, enabled: bool, hooked: bool) -> Json {
    let module = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| {
        lua.load(if enabled {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        let observer: Table = lua
            .load(OBSERVE)
            .set_name("@strength_life_source.lua")
            .eval()?;
        let cleanup = observer
            .raw_get::<Function>("begin")?
            .call((hooked, enabled))?;
        *module.borrow_mut() = Some(observer);
        Ok(cleanup)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let result: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("observe")?
            .call(enabled)?;
        Ok(lua.from_value(Value::Table(result))?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before_source),
        Some(&before_build),
        Some(&after),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    json!({"source_hash":report["source_hash"],"selected":report["selected"],"state":report["additional_observation"]})
}
fn without_capture(mut value: Json) -> Json {
    let state = value["state"].as_object_mut().unwrap();
    state.remove("hooked");
    for mode in ["MAIN", "CALCS"] {
        state.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    value
}
const FLAGS: [&str; 5] = [
    "NoAttributeBonuses",
    "NoStrengthAttributeBonuses",
    "NoStrBonusToLife",
    "DoubledInherentAttributeBonuses",
    "HalvesLifeFromStrength",
];
fn check(host: &Json, hooked: bool) {
    let state = &host["state"];
    assert_eq!(state["hooked"], hooked);
    assert_eq!(state["methods"]["actor"]["first"], 264);
    assert_eq!(state["methods"]["attributes"]["first"], 233);
    for mode in ["MAIN", "CALCS"] {
        let r = &state["modes"][mode];
        assert!(r["strength"].as_u64().is_some());
        let flags = &r["flags"];
        assert_eq!(flags.as_object().unwrap().len(), 5);
        for name in FLAGS {
            let f = &flags[name];
            assert!(f["present"].is_boolean());
            assert!(f["resolved"].is_boolean());
            if f["present"] == true {
                assert!(f["value"].is_boolean());
                assert_eq!(f["value"], f["resolved"]);
            } else {
                assert!(f.get("value").is_none());
                assert_eq!(f["resolved"], false);
            }
        }
        let records = rows(&r["records"]);
        assert!(records.len() <= 1);
        if records.is_empty() {
            assert_eq!(r["inherent_life"], 0);
        } else {
            let m = &records[0];
            assert_eq!(m["name"], "Life");
            assert_eq!(m["source"], "Strength");
            assert_eq!(m["type"], "BASE");
            assert_eq!(m["flags"], 0);
            assert_eq!(m["keyword_flags"], 0);
            assert!(rows(&m["tags"]).is_empty());
            assert_eq!(m["value"], r["inherent_life"]);
        }
        if hooked {
            let p = &r["provenance"];
            assert_eq!(p["original_actor_function"], true);
            assert_eq!(p["original_attribute_function"], true);
            assert_eq!(p["before_line"], 496);
            assert_eq!(p["after_line"], 522);
            assert_eq!(p["stage_strength"], r["strength"]);
            assert_eq!(p["stage_flags"], r["flags"]);
            assert_eq!(p["emitted_count"].as_u64().unwrap(), records.len() as u64);
            assert_eq!(p["exact_emitted_record"], records.len() == 1);
            assert!(!rows(&p["before"]).is_empty());
            assert!(!rows(&p["after"]).is_empty());
        } else {
            assert!(r.get("provenance").is_none());
        }
        assert!(!rows(&r["read_set"]).is_empty());
    }
    for field in [
        "original_methods_preserved",
        "original_complete_player_stage",
        "repeated_sum_equal",
        "relevant_read_set_preserved",
        "cached_outputs_preserved",
        "saved_items_preserved",
        "saved_selection_preserved",
    ] {
        assert_eq!(state["evidence"][field], true);
    }
    for field in [
        "business_method_wrappers",
        "final_life_claim",
        "attribute_contributor_closure",
        "flag_producer_closure",
    ] {
        assert_eq!(state["evidence"][field], false);
    }
}
fn controlled_xml(xml: &str, text: &str) -> String {
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
            .any(|n| n.has_tag_name("CustomModifierBlock")),
        "finite controls must not shadow an original custom block"
    );
    let at = set.range().end - "</ConfigSet>".len();
    assert_eq!(&xml[at..set.range().end], "</ConfigSet>");
    let addition = format!(
        "<CustomModifierBlock title=\"Strength Life Source Control\" enabled=\"true\">{text}</CustomModifierBlock>"
    );
    let mut out = xml.to_owned();
    out.insert_str(at, &addition);
    let mut restored = out.clone();
    restored.replace_range(at..at + addition.len(), "");
    assert!(restored == xml, "control has a byte-exact XML inverse");
    let changed = roxmltree::Document::parse(&out).unwrap();
    let blocks: Vec<_> = changed
        .descendants()
        .filter(|n| n.has_tag_name("CustomModifierBlock"))
        .collect();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].text(), Some(text));
    out
}
fn child(root: &Path, out: &Path, enabled: bool) {
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json =
        serde_json::from_slice(&fs::read(fixtures.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|n| fs::read_to_string(fixtures.join(format!("build-{n:02}.xml"))).unwrap())
        .collect();
    for (i, xml) in originals.iter().enumerate() {
        let name = format!("build-{:02}.xml", i + 1);
        let pin = rows(&index["builds"])
            .iter()
            .find(|x| x["xml"] == name)
            .unwrap();
        assert_eq!(pin["xml_sha256"], hash(xml.as_bytes()));
    }
    let double = "Inherent bonuses gained from Attributes are doubled";
    let half = "Inherent Life granted by Strength is halved";
    let controls = [
        ("double", double.to_owned()),
        ("half", half.to_owned()),
        ("double-and-half", format!("{double}\n{half}")),
        (
            "disable-all",
            "Gain no inherent bonuses from Attributes".to_owned(),
        ),
        (
            "disable-strength",
            "Gain no inherent bonuses from Strength".to_owned(),
        ),
        (
            "disable-strength-life",
            "Strength provides no inherent bonus to maximum Life".to_owned(),
        ),
        ("zero-strength", "-1000000 to Strength".to_owned()),
    ];
    let mut cases: Vec<(String, String, Option<String>)> = originals
        .iter()
        .enumerate()
        .map(|(i, xml)| (format!("original-{:02}", i + 1), xml.clone(), None))
        .collect();
    for (name, text) in controls {
        cases.push((
            format!("original-05-{name}"),
            controlled_xml(&originals[4], &text),
            Some(text),
        ));
    }
    cases.push(("repeat-original-05".into(), originals[4].clone(), None));
    assert_eq!(cases.len(), 13);
    let mode = if enabled { "on" } else { "off" };
    let mut captured = vec![];
    for (i, (name, xml, control)) in cases.iter().enumerate() {
        eprintln!("Strength Life {}/{} {name} JIT {mode}", i + 1, cases.len());
        let original = observed(root, xml, enabled, true);
        let unhooked = observed(root, xml, enabled, false);
        let row = json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"control_text":control,"source_only_control":control.is_some(),"original":original,"unhooked":unhooked});
        fs::write(
            out.join(format!("source-jit-{mode}-case-{:02}.raw.json", i + 1)),
            serde_json::to_vec_pretty(&row).unwrap(),
        )
        .unwrap();
        check(&original, true);
        check(&unhooked, false);
        assert!(
            without_capture(original) == without_capture(unhooked),
            "hooked and hookless original results must agree; see retained case JSON"
        );
        captured.push(row);
    }
    assert!(
        captured[4]["original"] == captured[12]["original"],
        "fresh original restoration replay must be exact"
    );
    for mode in ["MAIN", "CALCS"] {
        let baseline = &captured[4]["original"]["state"]["modes"][mode];
        assert_eq!(baseline["strength"], 27);
        assert_eq!(baseline["inherent_life"], 54);
        for (name, strength, amount, count, active) in [
            (
                "original-05-double",
                27,
                108,
                1,
                vec!["DoubledInherentAttributeBonuses"],
            ),
            (
                "original-05-half",
                27,
                27,
                1,
                vec!["HalvesLifeFromStrength"],
            ),
            (
                "original-05-double-and-half",
                27,
                54,
                1,
                vec!["DoubledInherentAttributeBonuses", "HalvesLifeFromStrength"],
            ),
            (
                "original-05-disable-all",
                27,
                0,
                0,
                vec!["NoAttributeBonuses"],
            ),
            (
                "original-05-disable-strength",
                27,
                0,
                0,
                vec!["NoStrengthAttributeBonuses"],
            ),
            (
                "original-05-disable-strength-life",
                27,
                0,
                0,
                vec!["NoStrBonusToLife"],
            ),
            ("original-05-zero-strength", 0, 0, 1, vec![]),
        ] {
            let case = captured.iter().find(|c| c["name"] == name).unwrap();
            let actual = &case["original"]["state"]["modes"][mode];
            assert_eq!(actual["strength"], strength, "{name}");
            assert_eq!(actual["inherent_life"], amount, "{name}");
            assert_eq!(rows(&actual["records"]).len(), count, "{name}");
            for flag in FLAGS {
                assert_eq!(
                    actual["flags"][flag]["resolved"],
                    active.contains(&flag),
                    "{name} {flag}"
                );
            }
        }
    }
    for (i, xml) in originals.iter().enumerate() {
        assert!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", i + 1))).unwrap() == *xml,
            "original fixture bytes unchanged"
        );
    }
    let native_cases:Vec<_>=captured.iter().enumerate().map(|(i,c)|{let m=&c["original"]["state"]["modes"]["MAIN"];
        let flags=FLAGS.map(|flag|(flag.to_owned(),m["flags"][flag]["resolved"].clone())).into_iter().collect::<serde_json::Map<_,_>>();
        json!({"case_index":i,"name":c["name"],"source_only_control":c["source_only_control"],"strength":m["strength"],"flags":flags,"emitted_count":rows(&m["records"]).len(),"inherent_life":m["inherent_life"]})}).collect();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVE.as_bytes()),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":(["src/Modules/CalcPerform.lua","src/Modules/CalcTools.lua","src/Classes/ModStore.lua","src/Classes/ModDB.lua","src/Classes/ConfigTab.lua",
            "src/Modules/ModParser.lua","src/Modules/CalcSetup.lua","src/Modules/Build.lua","src/Modules/Common.lua"]
            .map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "original_sources":originals.iter().enumerate().map(|(i,xml)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":hash(xml.as_bytes())})).collect::<Vec<_>>(),
        "case_count":13,"complete_loads_per_jit":26,"scope":{"original_complete_player_stage":true,"original_attribute_function":true,"original_sum":true,"fresh_unhooked_comparison":true,
            "fresh_repeat":true,"parsed_control_inputs":true,"enabled_zero_distinct_from_disabled_absence":true,"final_life":false,"attribute_contributor_closure":false,"flag_producer_closure":false,
            "game_obtainability_claim":false,"native_source_interpreter":false},"native_cases":native_cases,"cases":captured});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires complete pinned PoB source; bounded original Strength bonus observer"]
fn actual_strength_life_bonus_preserves_original_stage_and_controls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-strength-life-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "use a fresh immutable source output");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        fs::read(out.join("source-jit-off.json")).unwrap()
            == fs::read(out.join("source-jit-on.json")).unwrap(),
        "both modes preserve byte-identical original evidence"
    );
}
