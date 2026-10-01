//! Complete source evidence for saved weapon-set allocation paths, not legality.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
#[path = "support/scoped_allocation_access_source.rs"]
mod witness;

use mlua::Lua;
use poe_optimizer_pob::source as pinned;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_source_scoped_allocations_preserve_modes_and_prune_unstable_paths";
const CHILD: &str = "POE_SCOPED_ALLOCATION_ACCESS_CHILD";
const HASH02: &str = "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631";
const HASH04: &str = "62d760d326e21291cd1024f20660bd5046043c4e242b761df5d9a764be61e711";

#[test]
fn complete_source_scoped_allocations_preserve_modes_and_prune_unstable_paths() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-scoped-allocation-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let path02 = root.join("tests/fixtures/builds/breadth-20260908/build-02.xml");
        let path04 = root.join("tests/fixtures/builds/breadth-20260908/build-04.xml");
        let original02 = fs::read_to_string(&path02).unwrap();
        let original04 = fs::read_to_string(&path04).unwrap();
        assert_eq!(sha(&original02), HASH02);
        assert_eq!(sha(&original04), HASH04);
        let cases = vec![
            ("original02", original02.clone()),
            ("original04", original04.clone()),
            (
                "removed02_mode1_connector",
                edit_spec(&original02, 5, |s| remove_node(s, "42658")),
            ),
            (
                "wrong02_mode1_connector",
                edit_spec(&original02, 5, |s| move_node(s, "42658", 2)),
            ),
            (
                "wrong02_mode2_connector",
                edit_spec(&original02, 5, |s| move_node(s, "10909", 1)),
            ),
            (
                "shared02_leaf_behind_scoped",
                edit_spec(&original02, 5, |s| move_node(s, "45100", 0)),
            ),
            (
                "orphan_shared02_bridge",
                edit_spec(&original02, 5, |s| move_node(s, "63566", 0)),
            ),
            (
                "overlap02_set2_last",
                edit_spec(&original02, 5, |s| append_overlay(s, 2, "42658")),
            ),
            (
                "overlap02_set1_last",
                edit_spec(&original02, 5, |s| {
                    swap_overlays(&append_overlay(s, 2, "42658"))
                }),
            ),
            (
                "duplicate02_same_set_token",
                edit_spec(&original02, 5, |s| append_overlay(s, 1, "42658")),
            ),
            (
                "removed04_mode2_connector",
                edit_spec(&original04, 0, |s| remove_node(s, "52274")),
            ),
            (
                "scoped02_class_root",
                edit_spec(&original02, 5, |s| append_overlay(s, 1, "50986")),
            ),
            (
                "scoped02_ascendancy_root",
                edit_spec(&original02, 5, |s| append_overlay(s, 2, "55536")),
            ),
        ];
        let mut observations = Vec::new();
        let mut source_hash = None;
        for (label, xml) in &cases {
            let before = |lua: &Lua| {
                lua.globals().set("accessXml", xml.as_str())?;
                lua.globals().set("accessCase", *label)?;
                lua.globals().set("accessJit", enabled)?;
                lua.load("if accessJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let result = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                xml,
                None,
                false,
                Some(&before),
                Some(&witness::install),
                Some(&witness::observe),
            )
            .unwrap();
            if let Some(expected) = &source_hash {
                assert_eq!(&result["source_hash"], expected);
            } else {
                source_hash = Some(result["source_hash"].clone());
            }
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            observations.push(json!({"case":label,"input_sha256":sha(xml),"state":result["additional_observation"]}));
        }
        assert_eq!(fs::read_to_string(&path02).unwrap(), original02);
        assert_eq!(fs::read_to_string(&path04).unwrap(), original04);
        let result = json!({
            "source_hash":source_hash,"manifest_sha256":pinned::manifest_sha256(),
            "files":(["src/Classes/PassiveSpec.lua","src/Classes/PassiveTree.lua",
                "src/Classes/TreeTab.lua","src/Modules/CalcSetup.lua","src/Modules/Common.lua"].map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()}))),
            "native_access_authority":false,"native_build_parity":false,
            "original_sha256":[HASH02,HASH04],"observations":observations,
        });
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        check(&result);
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}

fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn edit_spec(xml: &str, index: usize, edit: impl FnOnce(&str) -> String) -> String {
    let start = xml.match_indices("<Spec ").nth(index).unwrap().0;
    let end = start + xml[start..].find("</Spec>").unwrap() + "</Spec>".len();
    let mut result = xml.to_owned();
    result.replace_range(start..end, &edit(&xml[start..end]));
    result
}
fn attr<'a>(xml: &'a str, name: &str) -> &'a str {
    let key = format!(" {name}=\"");
    let start = xml.find(&key).unwrap() + key.len();
    &xml[start..start + xml[start..].find('"').unwrap()]
}
fn set_attr(xml: &str, name: &str, value: &str) -> String {
    let key = format!(" {name}=\"");
    let start = xml.find(&key).unwrap() + key.len();
    let end = start + xml[start..].find('"').unwrap();
    let mut result = xml.to_owned();
    result.replace_range(start..end, value);
    result
}
fn overlay(spec: &str, mode: u8) -> std::ops::Range<usize> {
    let start = spec.find(&format!("<WeaponSet{mode} ")).unwrap();
    start..start + spec[start..].find("/>").unwrap() + 2
}
fn change_overlay(spec: &str, mode: u8, change: impl FnOnce(&str) -> String) -> String {
    let range = overlay(spec, mode);
    let child = &spec[range.clone()];
    let new_child = set_attr(child, "nodes", &change(attr(child, "nodes")));
    let mut result = spec.to_owned();
    result.replace_range(range, &new_child);
    result
}
fn append_overlay(spec: &str, mode: u8, token: &str) -> String {
    change_overlay(spec, mode, |nodes| format!("{nodes},{token}"))
}
fn move_node(spec: &str, token: &str, mode: u8) -> String {
    assert!(attr(spec, "nodes").split(',').any(|n| n == token));
    let mut result = spec.to_owned();
    for current in [1, 2] {
        result = change_overlay(&result, current, |nodes| {
            nodes
                .split(',')
                .filter(|n| *n != token)
                .collect::<Vec<_>>()
                .join(",")
        });
    }
    if mode == 0 {
        result
    } else {
        append_overlay(&result, mode, token)
    }
}
fn remove_node(spec: &str, token: &str) -> String {
    let spec = move_node(spec, token, 0);
    let nodes = attr(&spec, "nodes")
        .split(',')
        .filter(|n| *n != token)
        .collect::<Vec<_>>()
        .join(",");
    set_attr(&spec, "nodes", &nodes)
}
fn swap_overlays(spec: &str) -> String {
    let first = overlay(spec, 1);
    let second = overlay(spec, 2);
    assert!(first.end < second.start);
    let mut result = spec.to_owned();
    result.replace_range(second.clone(), &spec[first.clone()]);
    result.replace_range(first, &spec[second]);
    result
}
fn rows(value: &Json) -> &[Json] {
    if let Some(array) = value.as_array() {
        array
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn case<'a>(result: &'a Json, name: &str) -> &'a Json {
    &rows(&result["observations"])
        .iter()
        .find(|row| row["case"] == name)
        .unwrap()["state"]
}
fn node(state: &Json, id: u64) -> &Json {
    rows(&state["nodes"])
        .iter()
        .find(|row| row["id"] == id)
        .unwrap()
}
fn check(result: &Json) {
    assert_eq!(rows(&result["observations"]).len(), 13);
    for row in rows(&result["observations"]) {
        for field in [
            "fresh_specs",
            "original_functions_preserved",
            "saved_state_preserved",
            "main_output_preserved",
            "main_spec_matches_selected",
        ] {
            assert_eq!(row["state"][field], true, "{} {field}", row["case"]);
        }
    }
    for (name, selected_spec) in [("original02", 6), ("original04", 1)] {
        let original = case(result, name);
        assert_eq!(original["selected"]["spec"], selected_spec);
        assert_eq!(original["requested_ids"], original["retained"]["ids"]);
        assert!(rows(&original["removed_ids"]).is_empty());
        for mode in [1, 2] {
            let overlay = rows(&original["source_children"])
                .iter()
                .find(|child| child["elem"] == format!("WeaponSet{mode}"))
                .unwrap();
            let expected: BTreeSet<u64> = overlay["attrib"]["nodes"]
                .as_str()
                .unwrap()
                .split(',')
                .map(|token| token.parse().unwrap())
                .collect();
            let scoped: Vec<_> = rows(&original["retained"]["nodes"])
                .iter()
                .filter(|n| n["alloc_mode"] == mode)
                .collect();
            assert_eq!(scoped.len(), 24);
            let retained: BTreeSet<_> = scoped
                .iter()
                .map(|node| node["id"].as_u64().unwrap())
                .collect();
            assert_eq!(
                retained, expected,
                "{name} exact WeaponSet{mode} membership"
            );
            for n in scoped {
                assert_eq!(n["connected"]["value"], true);
                assert_eq!(n["free"]["type"], "nil");
                assert_eq!(n["granted"]["type"], "nil");
                assert!(rows(&n["radius_providers"]).is_empty());
                assert_eq!(
                    node(&original["main"], n["id"].as_u64().unwrap())["alloc_mode"],
                    mode
                );
            }
        }
    }
    assert_eq!(
        case(result, "duplicate02_same_set_token")["retained"],
        case(result, "original02")["retained"]
    );
    assert_eq!(
        case(result, "overlap02_set1_last")["retained"],
        case(result, "original02")["retained"]
    );
    assert_eq!(
        case(result, "overlap02_set2_last")["retained"],
        case(result, "wrong02_mode1_connector")["retained"]
    );
    assert!(
        rows(&case(result, "removed02_mode1_connector")["removed_ids"]).contains(&json!(45100))
    );
    assert!(rows(&case(result, "wrong02_mode1_connector")["removed_ids"]).contains(&json!(45100)));
    assert!(rows(&case(result, "wrong02_mode2_connector")["removed_ids"]).contains(&json!(58013)));
    assert!(
        rows(&case(result, "removed04_mode2_connector")["removed_ids"]).contains(&json!(29514))
    );
    assert!(
        rows(&case(result, "shared02_leaf_behind_scoped")["removed_ids"]).contains(&json!(45100))
    );
    let bridge = case(result, "orphan_shared02_bridge");
    assert!(rows(&bridge["removed_ids"]).contains(&json!(63566)));
    assert!(rows(&bridge["removed_ids"]).contains(&json!(7163)));
    assert_eq!(
        node(&case(result, "scoped02_class_root")["retained"], 50986)["alloc_mode"],
        1
    );
    assert_eq!(
        node(&case(result, "scoped02_ascendancy_root")["retained"], 55536)["alloc_mode"],
        2
    );
}
