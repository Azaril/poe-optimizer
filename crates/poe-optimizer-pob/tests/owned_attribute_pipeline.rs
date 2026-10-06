//! Original full Player attribute pipeline and exact attribute-choice edit.
//! This witnesses original stage execution; it does not close native contributors or choose MORE semantics.
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
const OBSERVE: &str = include_str!("support/attribute_pipeline_source.lua");
const TEST: &str = "actual_attribute_pipeline_preserves_stages_and_choice_control";
const CHILD: &str = "POE_ATTRIBUTE_PIPELINE_SOURCE_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_ATTRIBUTE_PIPELINE_SOURCE_OUT";
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
            .set_name("@attribute_pipeline_source.lua")
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

fn check_output(snapshot: &Json) {
    let scalars = snapshot["scalar_fields"].as_object().unwrap();
    let excluded = rows(&snapshot["excluded_fields"]);
    assert_eq!(
        snapshot["field_count"].as_u64().unwrap() as usize,
        scalars.len() + excluded.len()
    );
    let mut previous = None;
    for field in excluded {
        let key = field["key"].as_str().unwrap();
        let value_type = field["value_type"].as_str().unwrap();
        assert!(!["nil", "number", "boolean", "string"].contains(&value_type));
        assert!(!scalars.contains_key(key));
        if let Some(last) = previous {
            assert!(last < key, "excluded keys are complete and unique");
        }
        previous = Some(key);
    }
    for value in scalars.values() {
        match value["kind"].as_str().unwrap() {
            "boolean" => assert!(value["value"].is_boolean()),
            "string" => assert!(value["value"].is_string()),
            "number" => assert!(
                value["value"].is_number()
                    || matches!(
                        value["value"].as_str(),
                        Some("nan" | "positive_infinity" | "negative_infinity")
                    )
            ),
            _ => panic!("unexpected scalar output type"),
        }
    }
}
fn check(host: &Json, hooked: bool) {
    let state = &host["state"];
    assert_eq!(state["hooked"], hooked);
    assert_eq!(state["methods"]["attributes"]["first"], 233);
    assert_eq!(state["methods"]["val"]["first"], 50);
    for mode in ["MAIN", "CALCS"] {
        let m = &state["modes"][mode];
        for name in ["Str", "Dex", "Int"] {
            assert_eq!(m["attributes"][name]["present"], true);
            assert!(m["attributes"][name]["value"].as_u64().is_some());
        }
        assert!(!rows(&m["read_set"]).is_empty());
        check_output(&m["player_output"]);
        assert!(
            !m["player_output"]["scalar_fields"]
                .as_object()
                .unwrap()
                .is_empty()
        );
        for name in ["Str", "Dex", "Int"] {
            assert_eq!(m["player_output"]["scalar_fields"][name]["kind"], "number");
            assert_eq!(
                m["player_output"]["scalar_fields"][name]["value"],
                m["attributes"][name]["value"]
            );
        }
        if !hooked {
            assert!(m.get("provenance").is_none());
            continue;
        }
        let p = &m["provenance"];
        assert_eq!(p["original_actor_caller"], true);
        assert_eq!(p["exact_player_output_table"], true);
        let stages = rows(&p["stages"]);
        assert_eq!(stages.len(), 6);
        for (i, stage) in stages.iter().enumerate() {
            let stat = ["Str", "Dex", "Int"][i % 3];
            assert_eq!(stage["index"], i + 1);
            assert_eq!(stage["pass"], i / 3 + 1);
            assert_eq!(stage["stat"], stat);
            assert_eq!(stage["returned"], true);
            assert_eq!(stage["output"][stat]["value"], stage["value"]);
            assert!(stage["value"].as_u64().is_some());
            assert!(!rows(&stage["before_chain"]).is_empty());
            assert!(!rows(&stage["after_chain"]).is_empty());
            let queries = rows(&stage["queries"]);
            let top: Vec<_> = queries
                .iter()
                .filter(|q| q["store_depth"] == 0 && q["name"] == stat)
                .collect();
            let base: Vec<_> = top.iter().filter(|q| q["contribution"] == "BASE").collect();
            assert_eq!(
                base.len(),
                1,
                "one original BASE query at each attribute evaluation"
            );
            let kinds: Vec<_> = top
                .iter()
                .map(|q| q["contribution"].as_str().unwrap())
                .collect();
            if base[0]["result"].as_f64().unwrap() == 0.0 {
                assert_eq!(
                    kinds,
                    ["BASE"],
                    "zero BASE retains original lazy query behavior"
                );
            } else {
                assert_eq!(kinds, ["BASE", "INC", "MORE"]);
            }
            for q in queries {
                assert!(q["result"].is_number());
                assert_eq!(q["actual_return_local"], true);
                assert_eq!(q["original_context_is_player"], true);
            }
            for read in rows(&stage["reads"]) {
                assert!(read["store_depth"].as_u64().is_some());
                assert_eq!(read["actual_call_observed"], true);
                assert_eq!(read["direct_return_value_claim"], false);
                assert_eq!(read["replay_inputs_unchanged"], true);
                assert!(read.get("same_state_original_replay").is_some());
                check_output(&read["before_output"]);
            }
            if i > 0 {
                // Only calculated attribute fields are sequential here: the
                // comparison/LowestAttribute snapshot advances after each pass.
                for name in ["Str", "Dex", "Int"] {
                    assert_eq!(stage["input"][name], stages[i - 1]["output"][name]);
                }
            }
        }
        for name in ["Str", "Dex", "Int"] {
            assert_eq!(
                p["entry"][name]["present"], false,
                "complete perform resets Player output"
            );
            assert_eq!(p["exit"][name], m["attributes"][name]);
        }
        for i in [0, 1] {
            assert_eq!(
                stages[i]["conditions_before"],
                stages[i + 1]["conditions_before"]
            );
            assert_eq!(
                stages[i + 3]["conditions_before"],
                stages[i + 4]["conditions_before"]
            );
        }
    }
    for key in [
        "original_methods_preserved",
        "original_complete_attribute_stage",
        "full_attribute_ancestry",
        "scalar_output_comparison",
        "excluded_nonscalar_field_inventory",
        "top_level_output_identities_preserved",
        "nonscalar_output_graph_excluded",
        "getter_targets_limited_to_player_ancestry",
        "cached_scalar_outputs_preserved",
        "saved_items_preserved",
        "saved_selection_preserved",
    ] {
        assert_eq!(state["evidence"][key], true);
    }
    for key in [
        "business_method_wrappers",
        "native_contributor_closure",
        "default_condition_authority",
        "more_grouping_policy",
        "full_output_graph",
        "deep_output_identity_claim",
    ] {
        assert_eq!(state["evidence"][key], false);
    }
}
fn choice_control(xml: &str) -> String {
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
    assert_eq!(active, 3);
    let node_ids: Vec<_> = spec.attribute("nodes").unwrap().split(',').collect();
    assert_eq!(node_ids.iter().filter(|n| **n == "15782").count(), 1);
    let overrides: Vec<_> = spec
        .descendants()
        .filter(|n| n.has_tag_name("AttributeOverride"))
        .collect();
    assert_eq!(overrides.len(), 1);
    let original = &xml[overrides[0].range()];
    let strength = overrides[0].attribute("strNodes").unwrap();
    let dexterity = overrides[0].attribute("dexNodes").unwrap();
    let values: Vec<_> = strength.split(',').collect();
    assert_eq!(values.iter().filter(|v| **v == "15782").count(), 1);
    assert!(dexterity.is_empty());
    let remaining = values
        .into_iter()
        .filter(|v| *v != "15782")
        .collect::<Vec<_>>()
        .join(",");
    let from = format!("strNodes=\"{strength}\"");
    let to = format!("strNodes=\"{remaining}\"");
    assert_eq!(original.matches(&from).count(), 1);
    assert_eq!(original.matches("dexNodes=\"\"").count(), 1);
    let changed =
        original
            .replacen(&from, &to, 1)
            .replacen("dexNodes=\"\"", "dexNodes=\"15782\"", 1);
    let mut result = xml.to_owned();
    result.replace_range(overrides[0].range(), &changed);
    let mut inverse = result.clone();
    inverse.replace_range(
        overrides[0].range().start..overrides[0].range().start + changed.len(),
        original,
    );
    assert!(
        inverse == xml,
        "exact control inverse must preserve every other source byte"
    );
    let modified = roxmltree::Document::parse(&result).unwrap();
    let same_spec = modified
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Tree"))
        .unwrap()
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(active - 1)
        .unwrap();
    assert_eq!(spec.attribute("nodes"), same_spec.attribute("nodes"));
    assert_eq!(spec.attribute("classId"), same_spec.attribute("classId"));
    assert_eq!(
        spec.attribute("ascendClassId"),
        same_spec.attribute("ascendClassId")
    );
    result
}
fn child(root: &Path, out: &Path, enabled: bool) {
    // Authenticate every reported source path before any expensive build loads.
    // Runtime tree data is tree.lua; tree.json is an unmanifested export input.
    let source_root = root.join("vendor/path-of-building-poe2");
    let source_files = [
        "src/Modules/CalcPerform.lua",
        "src/Modules/CalcTools.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/Build.lua",
        "src/Modules/Common.lua",
        "src/Classes/PassiveSpec.lua",
        "src/TreeData/0_5/tree.lua",
    ]
    .map(|path| {
        let expected = pinned::expected_file_sha256(path).unwrap();
        let text = pinned::read_verified_text(&source_root, path).unwrap();
        assert_eq!(hash(text.as_bytes()), expected, "source pin {path}");
        json!({"path":path,"sha256":expected})
    });
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
            .find(|v| v["xml"] == name)
            .unwrap();
        assert_eq!(pin["xml_sha256"], hash(xml.as_bytes()));
    }
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, x)| (format!("original-{:02}", i + 1), x.clone(), false))
        .collect();
    cases.push((
        "original-05-strength-choice-to-dexterity".into(),
        choice_control(&originals[4]),
        true,
    ));
    cases.push(("restored-original-05".into(), originals[4].clone(), false));
    let mode = if enabled { "on" } else { "off" };
    let mut captured = vec![];
    for (i, (name, xml, control)) in cases.iter().enumerate() {
        eprintln!(
            "Attribute pipeline {}/{} {name} JIT {mode}",
            i + 1,
            cases.len()
        );
        let original = observed(root, xml, enabled, true);
        let unhooked = observed(root, xml, enabled, false);
        let row = json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"choice_control":control,"original":original,"unhooked":unhooked});
        fs::write(
            out.join(format!("source-jit-{mode}-case-{:02}.raw.json", i + 1)),
            serde_json::to_vec_pretty(&row).unwrap(),
        )
        .unwrap();
        check(&original, true);
        check(&unhooked, false);
        assert!(
            without_capture(original) == without_capture(unhooked),
            "hooked and hookless original state differs; inspect retained raw case"
        );
        captured.push(row);
    }
    assert!(
        captured[4]["original"] == captured[6]["original"],
        "independent restored original must replay exactly"
    );
    for mode in ["MAIN", "CALCS"] {
        let initial = &captured[4]["original"]["state"]["modes"][mode];
        let edited = &captured[5]["original"]["state"]["modes"][mode];
        for (name, expected) in [("Str", 27), ("Dex", 7), ("Int", 105)] {
            assert_eq!(
                initial["attributes"][name]["value"], expected,
                "retained original05 witness"
            );
        }
        assert_ne!(initial["attributes"]["Str"], edited["attributes"]["Str"]);
        assert_ne!(initial["attributes"]["Dex"], edited["attributes"]["Dex"]);
        assert_eq!(initial["attributes"]["Int"], edited["attributes"]["Int"]);
        assert_eq!(
            captured[4]["original"]["selected"],
            captured[5]["original"]["selected"]
        );
    }
    for (i, xml) in originals.iter().enumerate() {
        assert!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", i + 1))).unwrap() == *xml
        );
    }
    let native_cases: Vec<_> = captured.iter().enumerate().map(|(i,c)| {
        let m = &c["original"]["state"]["modes"]["MAIN"];
        json!({"case_index":i,"name":c["name"],"choice_control":c["choice_control"],"attributes":m["attributes"],"stages":m["provenance"]["stages"]})
    }).collect();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVE.as_bytes()),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":source_files,
        "original_sources":originals.iter().enumerate().map(|(i,x)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":hash(x.as_bytes())})).collect::<Vec<_>>(),
        "case_count":7,"complete_loads_per_jit":14,"scope":{"six_original_attribute_evaluations":true,"actual_sum_more_return_locals":true,
            "full_attribute_ancestry_at_each_stage":true,"actual_getter_calls_with_explicit_original_replay":true,"scalar_output_comparison":true,
            "full_output_graph":false,"nonscalar_output_graph_excluded":true,"excluded_nonscalar_field_inventory":true,
            "top_level_output_identities_preserved":true,"deep_output_identity_claim":false,
            "fresh_unhooked_comparison":true,"independent_restoration_replay":true,"choice_only_xml_control":true,
            "native_contributor_closure":false,"c0_default_policy":false,"more_grouping_policy":false,"final_life":false,
            "getter_targets_limited_to_player_ancestry":true},
        "native_cases":native_cases,"cases":captured});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires complete pinned PoB source; bounded original attribute pipeline observer"]
fn actual_attribute_pipeline_preserves_stages_and_choice_control() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-attribute-pipeline-source-01"));
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
        "fresh JIT lanes must retain byte-identical source evidence"
    );
}
