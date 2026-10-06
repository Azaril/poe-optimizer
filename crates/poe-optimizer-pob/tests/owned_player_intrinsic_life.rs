//! Original Player base-Life initialization and evaluation across real builds.
//! This witnesses one contribution, not a final pool or complete contributors.
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
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVE: &str = include_str!("support/player_intrinsic_life_source.lua");
const TEST: &str = "actual_player_intrinsic_life_preserves_original_initializer_and_evaluation";
const CHILD: &str = "POE_PLAYER_INTRINSIC_LIFE_SOURCE_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_LIFE_SOURCE_OUT";
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
fn observed(root: &Path, xml: &str, warm: Option<&str>, enabled: bool, hooked: bool) -> Json {
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
            .set_name("@player_intrinsic_life_source.lua")
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
        warm,
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
    state.remove("initializers");
    state.remove("hooked");
    for mode in ["MAIN", "CALCS"] {
        state.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    value
}
fn without_initializer_history(mut value: Json) -> Json {
    value["state"]
        .as_object_mut()
        .unwrap()
        .remove("initializers");
    value
}
fn check(host: &Json, level: u64, expected: u64, hooked: bool) {
    let state = &host["state"];
    assert_eq!(state["hooked"], hooked);
    assert_eq!(state["constant"], 12);
    assert_eq!(state["selected"]["character_level"], level);
    let names: BTreeSet<_> = rows(&state["classes"])
        .iter()
        .map(|x| x["name"].as_str().unwrap())
        .collect();
    assert!(names.len() >= 8);
    for mode in ["MAIN", "CALCS"] {
        let actual = &state["modes"][mode];
        assert_eq!(actual["character_level"], level);
        assert!(names.contains(actual["class_name"].as_str().unwrap()));
        assert_eq!(
            actual["record"],
            json!({"name":"Life","type":"BASE","value":12,"source":"Base",
            "flags":0,"keyword_flags":0,"tags":[{"type":"Multiplier","var":"Level","base":16}]})
        );
        assert_eq!(actual["effective_value"], expected);
        assert_eq!(actual["base_source_sum"], expected);
        assert_eq!(actual["multiplier"]["effective"], level);
        assert_eq!(actual["multiplier"]["extra_base"], 0);
        assert_eq!(actual["multiplier"]["override"], json!({"present":false}));
        for entry in rows(&actual["multiplier"]["read_set"]) {
            assert!(
                rows(&entry["multiplier_level"]).is_empty(),
                "actual modifier inventory is inspected, not assumed empty"
            );
        }
        if hooked {
            let proof = &actual["provenance"];
            assert_eq!(proof["exact_initialized_record"], true);
            assert_eq!(proof["initialized_character_level"], level);
            assert_eq!(proof["initialized_stored_level"], level);
            assert_eq!(proof["initialized_constant"], 12);
            assert_eq!(proof["initializer_class_id"], actual["class_id"]);
            assert!(proof["initializer_store_is_owner"].is_boolean());
        } else {
            assert!(actual.get("provenance").is_none());
        }
    }
    assert_eq!(!rows(&state["initializers"]).is_empty(), hooked);
    for row in rows(&state["initializers"]) {
        assert_eq!(row["character_level"], level);
        assert_eq!(row["stored_level"], level);
        assert_eq!(row["constant"], 12);
        assert_eq!(row["record"], state["modes"]["MAIN"]["record"]);
    }
    for field in [
        "original_methods_preserved",
        "direct_original_eval",
        "repeated_eval_equal",
        "relevant_read_set_preserved",
        "saved_items_preserved",
        "saved_selection_preserved",
        "cached_outputs_preserved",
    ] {
        assert_eq!(state["evidence"][field], true);
    }
    for field in [
        "business_method_wrappers",
        "final_life_claim",
        "contributor_closure",
        "multiplier_compatibility_admitted",
    ] {
        assert_eq!(state["evidence"][field], false);
    }
}
fn level_xml(xml: &str, level: u64) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let build = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    assert_eq!(build.attribute("characterLevelAutoMode"), Some("false"));
    let attr = build.attributes().find(|a| a.name() == "level").unwrap();
    let mut out = xml.to_owned();
    out.replace_range(attr.range(), &format!("level=\"{level}\""));
    // Byte inversion proves the control changes only this saved field.
    let changed = roxmltree::Document::parse(&out).unwrap();
    let changed_attr = changed
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap()
        .attributes()
        .find(|a| a.name() == "level")
        .unwrap();
    let mut restored = out.clone();
    restored.replace_range(changed_attr.range(), &xml[attr.range()]);
    assert_eq!(restored, xml);
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
    let mut cases: Vec<(String, String, Option<String>, u64, u64)> = originals
        .iter()
        .enumerate()
        .zip([(96, 1168), (88, 1072), (93, 1132), (96, 1168), (92, 1120)])
        .map(|((i, xml), (level, value))| {
            (
                format!("original-{:02}", i + 1),
                xml.clone(),
                None,
                level,
                value,
            )
        })
        .collect();
    for (level, value) in [(1, 28), (91, 1108), (100, 1216)] {
        cases.push((
            format!("original-05-level-{level}"),
            level_xml(&originals[4], level),
            None,
            level,
            value,
        ));
    }
    cases.push((
        "repeat-original-05".into(),
        originals[4].clone(),
        None,
        92,
        1120,
    ));
    cases.push((
        "warm-level-one-to-original-05".into(),
        originals[4].clone(),
        Some(level_xml(&originals[4], 1)),
        92,
        1120,
    ));
    assert_eq!(cases.len(), 10);
    let mut captured = vec![];
    let mode = if enabled { "on" } else { "off" };
    for (i, (name, xml, warm, level, value)) in cases.iter().enumerate() {
        eprintln!(
            "Player intrinsic Life {}/{} {name} JIT {}",
            i + 1,
            cases.len(),
            if enabled { "on" } else { "off" }
        );
        let actual = observed(root, xml, warm.as_deref(), enabled, true);
        let unhooked = observed(root, xml, warm.as_deref(), enabled, false);
        let capture = json!({"name":name,"xml_sha256":hash(xml.as_bytes()),
            "warm_xml_sha256":warm.as_ref().map(|x|hash(x.as_bytes())),"original":actual,"unhooked":unhooked});
        fs::write(
            out.join(format!("source-jit-{mode}-case-{:02}.raw.json", i + 1)),
            serde_json::to_vec_pretty(&capture).unwrap(),
        )
        .unwrap();
        check(&actual, *level, *value, true);
        check(&unhooked, *level, *value, false);
        assert!(
            without_capture(actual.clone()) == without_capture(unhooked.clone()),
            "original outputs and exact input state are independent of the hook; see retained case raw JSON"
        );
        captured.push(capture);
    }
    assert!(
        captured[4]["original"] == captured[8]["original"],
        "fresh replay is exact, including initializer history; see retained cases05/09"
    );
    // A warm prior build changes the number of original initializations. Keep
    // that trace as diagnostics, while comparing every current-state field,
    // including the exact selected record's original initializer provenance.
    assert!(
        without_initializer_history(captured[4]["original"].clone())
            == without_initializer_history(captured[9]["original"].clone()),
        "warm prior build preserves exact current state and provenance; see retained cases05/10"
    );
    let warm_xml = level_xml(&originals[4], 1);
    let warm_repeat = observed(root, &originals[4], Some(&warm_xml), enabled, true);
    fs::write(
        out.join(format!("source-jit-{mode}-warm-repeat.raw.json")),
        serde_json::to_vec_pretty(&warm_repeat).unwrap(),
    )
    .unwrap();
    check(&warm_repeat, 92, 1120, true);
    assert!(
        captured[9]["original"] == warm_repeat,
        "independent warm replay is exact, including initializer history; see retained warm-repeat raw JSON"
    );
    for (i, xml) in originals.iter().enumerate() {
        assert!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", i + 1))).unwrap() == *xml,
            "original fixture bytes remain unchanged"
        );
    }
    let native_cases: Vec<_> = captured
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let m = &c["original"]["state"]["modes"]["MAIN"];
            json!({"case_index":i,"name":c["name"],"class_name":m["class_name"],
            "character_level":m["character_level"],"intrinsic_life":m["effective_value"]})
        })
        .collect();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":hash(OBSERVE.as_bytes()),
        "bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":(["src/Modules/CalcSetup.lua","src/Classes/ModStore.lua","src/Classes/ModDB.lua",
            "src/Modules/Build.lua","src/Modules/Common.lua","src/Data/Misc.lua","src/Classes/PassiveTree.lua"]
            .map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "original_sources":originals.iter().enumerate().map(|(i,xml)|json!({
            "path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":hash(xml.as_bytes())})).collect::<Vec<_>>(),
        "case_count":10,"complete_loads_per_jit":24,
        "scope":{"original_initializer":true,"original_eval_mod":true,"fresh_unhooked_comparison":true,
            "fresh_repeat":true,"warm_repeat":true,"independent_warm_repeat":true,
            "warm_comparison_ignores_initializer_history_only":true,"final_life":false,"contributor_closure":false,
            "generic_level_multiplier_native_admission":false},
        "native_cases":native_cases,"cases":captured,"warm_repeat":warm_repeat});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires complete pinned PoB source; bounded original Player Life observer"]
fn actual_player_intrinsic_life_preserves_original_initializer_and_evaluation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-player-intrinsic-life-source-01"));
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
