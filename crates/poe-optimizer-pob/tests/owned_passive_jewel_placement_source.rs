//! Complete original-source evidence for ordinary passive-jewel receiving uses.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "ordinary_passive_jewel_placement_preserves_complete_source_state";
const CHILD: &str = "POE_PASSIVE_JEWEL_PLACEMENT_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_passive_jewel_placement_source.lua");
const PINNED_FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Classes/Item.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Classes/PassiveTree.lua",
    "src/Classes/TreeTab.lua",
    "src/Classes/ItemSlotControl.lua",
    "src/Modules/ItemTools.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/CalcSetup.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModStore.lua",
    "src/Data/ModScalability.lua",
    "src/Data/Bases/jewel.lua",
    "src/TreeData/0_1/tree.lua",
    "src/TreeData/0_4/tree.lua",
    "src/TreeData/0_5/tree.lua",
    "runtime/lua/xml.lua",
];
#[derive(Clone)]
struct Case {
    name: String,
    xml: String,
    warm: bool,
}
#[test]
fn ordinary_passive_jewel_placement_preserves_complete_source_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-passive-jewel-placement-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
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
                assert!(
                    status.success(),
                    "source child failed {}; evidence {}\n{}",
                    path.display(),
                    out.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "source deadline {}; evidence {}\n{}",
                    path.display(),
                    out.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    same(
        &read(&out.join("source-jit-off.json")),
        &read(&out.join("source-jit-on.json")),
        "exact scoped JIT evidence",
    );
}
fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index = read(&fixtures.join("index.json"));
    let originals: Vec<_> = (1..=5)
        .map(|n| {
            let filename = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&filename)).unwrap();
            let row = rows(&index["builds"])
                .iter()
                .find(|r| r["xml"] == filename)
                .unwrap();
            assert_eq!(row["xml_sha256"], digest(xml.as_bytes()));
            Case {
                name: format!("original-{n:02}"),
                xml,
                warm: false,
            }
        })
        .collect();
    let original = &originals[3].xml;
    let socket = "<Socket itemId=\"1\" nodeId=\"46882\"/>";
    assert_eq!(original.matches(socket).count(), 1);
    let mut inputs = originals.clone();
    for (name, replacement) in [
        (
            "missing-item",
            "<Socket itemId=\"999999\" nodeId=\"46882\"/>",
        ),
        ("zero-item", "<Socket itemId=\"0\" nodeId=\"46882\"/>"),
        ("wrong-item-kind", "<Socket itemId=\"9\" nodeId=\"46882\"/>"),
        (
            "duplicate-node",
            "<Socket itemId=\"1\" nodeId=\"46882\"/><Socket itemId=\"2\" nodeId=\"46882\"/>",
        ),
        ("numeric-alias", "<Socket itemId=\"01\" nodeId=\"046882\"/>"),
        (
            "malformed-node",
            "<Socket itemId=\"1\" nodeId=\"unknown\"/>",
        ),
        (
            "malformed-item",
            "<Socket itemId=\"unknown\" nodeId=\"46882\"/>",
        ),
        ("unknown-node", "<Socket itemId=\"1\" nodeId=\"999999\"/>"),
        ("missing-node-attribute", "<Socket itemId=\"1\"/>"),
        ("missing-item-attribute", "<Socket nodeId=\"46882\"/>"),
        (
            "unknown-socket-attribute",
            "<Socket itemId=\"1\" nodeId=\"46882\" owned=\"unknown\"/>",
        ),
        (
            "namespaced-socket",
            "<p:Socket xmlns:p=\"urn:owned-probe\" itemId=\"1\" nodeId=\"46882\"/>",
        ),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: original.replace(socket, replacement),
            warm: false,
        });
    }
    inputs.push(Case {
        name: "unallocated-node".into(),
        xml: edit_spec(original, |spec| {
            let doc = roxmltree::Document::parse(spec).unwrap();
            let nodes = doc.root_element().attribute("nodes").unwrap();
            let retained = nodes
                .split(',')
                .filter(|n| *n != "46882")
                .collect::<Vec<_>>()
                .join(",");
            assert_ne!(nodes, retained);
            spec.replacen(
                &format!("nodes=\"{nodes}\""),
                &format!("nodes=\"{retained}\""),
                1,
            )
        }),
        warm: false,
    });
    for mode in ["empty", "absent"] {
        let xml = edit_spec(original, |spec| {
            let doc = roxmltree::Document::parse(spec).unwrap();
            let range = doc
                .root_element()
                .children()
                .find(|n| n.has_tag_name("Sockets"))
                .unwrap()
                .range();
            let mut result = spec.to_owned();
            result.replace_range(range, if mode == "empty" { "<Sockets/>" } else { "" });
            result
        });
        inputs.push(Case {
            name: format!("sockets-{mode}"),
            xml: xml.clone(),
            warm: false,
        });
        if mode == "absent" {
            inputs.push(Case {
                name: "warm-then-sockets-absent".into(),
                xml,
                warm: true,
            });
        }
    }
    for allocation_mode in [1, 2] {
        for active in [1, 2] {
            let xml = edit_spec(original, |spec| {
                spec.replacen(
                    &format!("<WeaponSet{allocation_mode} nodes=\""),
                    &format!("<WeaponSet{allocation_mode} nodes=\"46882,"),
                    1,
                )
            });
            inputs.push(Case {
                name: format!("socket-mode-{allocation_mode}-active-{active}"),
                xml: xml.replace(
                    "useSecondWeaponSet=\"nil\"",
                    if active == 2 {
                        "useSecondWeaponSet=\"true\""
                    } else {
                        "useSecondWeaponSet=\"false\""
                    },
                ),
                warm: false,
            });
        }
    }
    for selected in [1, 2] {
        let xml = edit_spec(original, |spec| {
            format!(
                "{spec}\n{}",
                spec.replace(socket, "<Socket itemId=\"2\" nodeId=\"46882\"/>")
            )
        });
        inputs.push(Case {
            name: format!("cross-spec-selected-{selected}"),
            xml: xml.replacen(
                "<Tree activeSpec=\"1\">",
                &format!("<Tree activeSpec=\"{selected}\">"),
                1,
            ),
            warm: false,
        });
    }
    for (name, version) in [
        ("older-tree", Some("0_4")),
        ("missing-tree-version", None),
        ("unknown-tree-version", Some("owned_unknown")),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: edit_spec(original, |spec| {
                spec.replacen(
                    "treeVersion=\"0_5\"",
                    &version.map_or_else(String::new, |v| format!("treeVersion=\"{v}\"")),
                    1,
                )
            }),
            warm: false,
        });
    }
    for order in ["before", "after"] {
        inputs.push(Case {
            name: format!("unknown-version-sibling-{order}"),
            xml: edit_spec(original, |spec| {
                let unknown =
                    spec.replacen("treeVersion=\"0_5\"", "treeVersion=\"owned_unknown\"", 1);
                if order == "before" {
                    format!("{unknown}\n{spec}")
                } else {
                    format!("{spec}\n{unknown}")
                }
            })
            .replacen(
                "<Tree activeSpec=\"1\">",
                if order == "before" {
                    "<Tree activeSpec=\"2\">"
                } else {
                    "<Tree activeSpec=\"1\">"
                },
                1,
            ),
            warm: false,
        });
    }
    inputs.push(Case {
        name: "late-replacement-base".into(),
        xml: original.replacen(
            "14% increased Fire Damage",
            "Emerald\n14% increased Fire Damage",
            1,
        ),
        warm: false,
    });
    for (name, legacy, internal, invalid) in [
        ("missing-class", "classId", "classInternalId", false),
        ("invalid-class", "classId", "classInternalId", true),
        (
            "missing-ascendancy",
            "ascendClassId",
            "ascendancyInternalId",
            false,
        ),
        (
            "invalid-ascendancy",
            "ascendClassId",
            "ascendancyInternalId",
            true,
        ),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: edit_spec(original, |spec| {
                let doc = roxmltree::Document::parse(spec).unwrap();
                let mut changed = spec.to_owned();
                for attribute in [legacy, internal] {
                    let value = doc.root_element().attribute(attribute).unwrap();
                    let replacement = if invalid {
                        format!(
                            "{attribute}=\"{}\"",
                            if attribute == legacy {
                                "-1"
                            } else if attribute == "classInternalId" {
                                "999999"
                            } else {
                                "owned_unknown"
                            }
                        )
                    } else {
                        String::new()
                    };
                    changed =
                        changed.replacen(&format!("{attribute}=\"{value}\""), &replacement, 1);
                }
                changed
            }),
            warm: false,
        });
    }
    for selected in [5, 6] {
        let baseline = &originals[4].xml;
        assert_eq!(baseline.matches("<Tree activeSpec=\"3\">").count(), 1);
        inputs.push(Case {
            name: format!("original-05-archived-spec-{selected}"),
            xml: baseline.replacen(
                "<Tree activeSpec=\"3\">",
                &format!("<Tree activeSpec=\"{selected}\">"),
                1,
            ),
            warm: false,
        });
    }
    let mut cases = Vec::new();
    for case in &inputs {
        eprintln!("complete passive jewel source case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("jewelPlacementXml", case.xml.as_str())?;
            lua.globals()
                .set("jewelPlacementControls", case.name == "original-04")?;
            lua.globals().set("jewelPlacementJit", enabled)?;
            lua.load("if jewelPlacementJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("jewelPlacementPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@passive-jewel-source-authentication")
                .eval::<Function>()?)
        };
        let temp = tempfile::tempdir().unwrap();
        let observed = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &case.xml,
            case.warm.then_some(original.as_str()),
            !case.name.starts_with("original-"),
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        let row = match observed {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"expected_selections":selections(&case.xml),"available":true,"state":value["additional_observation"]})
            }
            Err(error) => {
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"available":false,"source_error":error.to_string()})
            }
        };
        cases.push(row);
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
    for (n, original) in originals.iter().enumerate() {
        assert_eq!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", n + 1))).unwrap(),
            original.xml
        );
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),"evidence":{
        "complete_load_attempts_per_jit":inputs.len()+inputs.iter().filter(|c|c.warm).count(),"case_count":inputs.len(),"full_controls":inputs.len()-5,"warm_loads":inputs.iter().filter(|c|c.warm).count(),"isolated_fresh_reused_pairs":4,"isolated_character_load_controls":4,
        "selected_assignments":15,"archived_assignments_selected_by_controls":6,"ordinary_node_count":12,"eligible_base_count":6,
        "native_effect_coverage":false,"whole_build_parity":false,"business_method_wrappers":false,
        "observer_sha256":digest(OBSERVE.as_bytes()),"originals":originals.iter().map(|c|json!({"name":c.name,"sha256":digest(c.xml.as_bytes())})).collect::<Vec<_>>(),
        "files":PINNED_FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>()},"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result);
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("jewelPlacementPhase", "after")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@passive-jewel-source-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    let mut count = 0;
    let mut nodes = BTreeSet::new();
    let mut bases = BTreeSet::new();
    for (index, expected_count) in [3, 3, 3, 6, 0].into_iter().enumerate() {
        let case = &cases[index];
        assert_eq!(
            case["available"], true,
            "{}: {}",
            case["name"], case["source_error"]
        );
        let state = &case["state"];
        preserved(case);
        assert_eq!(state["tree_version"], "0_5");
        assert_eq!(rows(&state["assignments"]).len(), expected_count);
        assert_eq!(
            rows(&state["eligibility_matrix"]).len(),
            expected_count * 12
        );
        for row in rows(&state["assignments"]) {
            count += 1;
            nodes.insert(row["node"].as_u64().unwrap());
            bases.insert(row["item"]["base"].as_str().unwrap());
            assert_eq!(row["item_id"], row["item"]["id"]);
            assert_eq!(row["spec_assignment"], row["item_id"]);
            assert_eq!(row["slot"]["selected_item"], row["item_id"]);
            assert_eq!(row["item"]["type"], "Jewel");
            assert_eq!(row["valid"], true);
            assert_eq!(row["node_facts"]["is_jewel_socket"], true);
            assert_eq!(row["node_facts"]["allocated"], true);
            assert_eq!(row["node_facts"]["same_allocation_object"], true);
            assert_eq!(row["node_facts"]["alloc_mode"], 0);
            assert!(row["slot"]["weapon_set"].is_null());
            for mode in ["main", "calcs"] {
                assert_eq!(row["receiving"][mode], true, "{} {mode}", case["name"]);
            }
        }
        for row in rows(&state["eligibility_matrix"]) {
            assert_eq!(row["valid"], true, "{} {row}", case["name"]);
        }
        for row in rows(&state["ordinary_nodes"]) {
            assert_eq!(row["is_jewel_socket"], true);
            for field in ["contain_jewel_socket", "sinister", "charm_socket"] {
                assert_eq!(row[field], false);
            }
            for field in ["expansion_jewel", "ascendancy_name"] {
                assert!(row[field].is_null());
            }
        }
    }
    assert_eq!(count, 15);
    assert_eq!(nodes.len(), 12);
    assert_eq!(bases.len(), 6);
    let find = |name: &str| cases.iter().find(|c| c["name"] == name).unwrap();
    let assignment = |case: &Json| {
        rows(&case["state"]["assignments"])
            .iter()
            .find(|r| r["node"] == 46882)
            .unwrap()
            .clone()
    };
    let ruby = assignment(&cases[3]);
    for mode in ["main", "calcs"] {
        let mods = rows(&ruby["merged_modifiers"][mode]);
        let fire: Vec<_> = mods
            .iter()
            .filter(|m| m["name"] == "FireDamage" && m["type"] == "INC")
            .collect();
        assert_eq!(fire.len(), 1);
        assert_eq!(fire[0]["value"], 14);
    }
    // Controls retain the original methods; permissive loader behavior is not
    // automatically permission for the stricter ordinary-placement adapter.
    for name in [
        "missing-item",
        "zero-item",
        "numeric-alias",
        "malformed-node",
        "malformed-item",
        "unknown-socket-attribute",
        "unallocated-node",
        "sockets-empty",
        "sockets-absent",
        "warm-then-sockets-absent",
        "socket-mode-1-active-1",
        "socket-mode-1-active-2",
        "socket-mode-2-active-1",
        "socket-mode-2-active-2",
        "cross-spec-selected-1",
        "cross-spec-selected-2",
        "late-replacement-base",
    ] {
        assert_eq!(
            find(name)["available"],
            true,
            "{name}: {}",
            find(name)["source_error"]
        );
        preserved(find(name));
    }
    let unallocated = assignment(find("unallocated-node"));
    assert_eq!(unallocated["spec_assignment"], 1);
    assert_eq!(unallocated["node_facts"]["allocated"], false);
    assert_eq!(unallocated["receiving"]["main"], false);
    assert_eq!(unallocated["receiving"]["calcs"], false);
    assert_eq!(assignment(find("numeric-alias"))["spec_assignment"], 1);
    for name in ["missing-item", "zero-item", "malformed-item"] {
        let row = assignment(find(name));
        assert!(
            !row["spec_assignment"].as_u64().is_some_and(|v| v > 0),
            "{name}"
        );
    }
    for name in [
        "sockets-empty",
        "sockets-absent",
        "warm-then-sockets-absent",
    ] {
        assert!(rows(&find(name)["state"]["assignments"]).is_empty());
    }
    for probe in rows(&cases[3]["state"]["isolated_load_controls"]) {
        assert_eq!(probe["separate_objects"], true);
        let jewel = |state: &Json| {
            rows(&state["jewels"])
                .iter()
                .find(|j| j["node"] == 46882)
                .map(|j| j["item"].clone())
        };
        assert_eq!(jewel(&probe["reused_before"]), Some(json!(1)));
        assert_eq!(jewel(&probe["reused_after"]), Some(json!(1)));
        assert_eq!(jewel(&probe["fresh"]), None);
    }
    let character_controls = rows(&cases[3]["state"]["isolated_character_load_controls"]);
    assert_eq!(character_controls.len(), 4);
    for probe in character_controls {
        assert_eq!(probe["call_succeeded"], true, "{}", probe["name"]);
        assert_eq!(probe["returned"], true, "{}", probe["name"]);
        assert_eq!(probe["separate_object"], true);
        assert_eq!(probe["saved_assignment"], 1);
        assert_eq!(probe["node"]["allocated"], false);
        assert_eq!(probe["node"]["same_allocation_object"], false);
    }
    assert_eq!(
        assignment(find("cross-spec-selected-1"))["spec_assignment"],
        1
    );
    assert_eq!(
        assignment(find("cross-spec-selected-2"))["spec_assignment"],
        2
    );
    for selected in [5, 6] {
        let case = find(&format!("original-05-archived-spec-{selected}"));
        assert_eq!(case["available"], true, "{}", case["source_error"]);
        preserved(case);
        assert_eq!(case["state"]["selected"]["spec"], selected);
        let assignments = rows(&case["state"]["assignments"]);
        assert_eq!(assignments.len(), 3);
        for (node, item) in [(21984, 17), (61419, 18), (7960, 16)] {
            let row = assignments.iter().find(|row| row["node"] == node).unwrap();
            assert_eq!(row["item_id"], item);
            assert_eq!(row["spec_assignment"], item);
            assert_eq!(row["slot"]["selected_item"], item);
            assert_eq!(row["item"]["base"], "Sapphire");
            assert_eq!(row["valid"], true);
            assert_eq!(row["receiving"]["main"], true);
            assert_eq!(row["receiving"]["calcs"], true);
        }
    }
}
fn preserved(case: &Json) {
    for flag in [
        "saved_items_preserved",
        "saved_specs_preserved",
        "saved_selections_preserved",
        "main_output_preserved",
        "calcs_output_preserved",
        "original_functions_preserved",
        "fresh_loaded_objects",
    ] {
        assert_eq!(case["state"][flag], true, "{} {flag}", case["name"]);
    }
    same(
        &case["state"]["selected"],
        &case["expected_selections"],
        case["name"].as_str().unwrap(),
    );
}
fn selections(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut result = serde_json::Map::new();
    for (key, element, attribute) in [
        ("config", "Config", "activeConfigSet"),
        ("items", "Items", "activeItemSet"),
        ("skills", "Skills", "activeSkillSet"),
        ("spec", "Tree", "activeSpec"),
        ("group", "Build", "mainSocketGroup"),
    ] {
        let node = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name(element))
            .unwrap();
        result.insert(
            key.into(),
            json!(node.attribute(attribute).unwrap().parse::<usize>().unwrap()),
        );
    }
    Json::Object(result)
}
fn edit_spec(xml: &str, edit: impl FnOnce(&str) -> String) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let tree = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Tree"))
        .unwrap();
    let specs: Vec<_> = tree.children().filter(|n| n.has_tag_name("Spec")).collect();
    assert_eq!(specs.len(), 1);
    let range = specs[0].range();
    let replacement = edit(&xml[range.clone()]);
    let mut result = xml.to_owned();
    result.replace_range(range, &replacement);
    result
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "not source list: {value}"
        );
        &[]
    }
}
fn tail(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(30)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.chars().take(600).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
fn same(a: &Json, b: &Json, context: &str) {
    fn first(a: &Json, b: &Json, path: String) -> String {
        if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
            for key in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
                if a.get(key) != b.get(key) {
                    return first(
                        a.get(key).unwrap_or(&Json::Null),
                        b.get(key).unwrap_or(&Json::Null),
                        format!("{path}.{key}"),
                    );
                }
            }
        } else if let (Some(a), Some(b)) = (a.as_array(), b.as_array()) {
            if a.len() != b.len() {
                return format!("{path}: lengths {} != {}", a.len(), b.len());
            }
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                if a != b {
                    return first(a, b, format!("{path}[{index}]"));
                }
            }
        }
        format!(
            "{path}: {} != {}",
            a.to_string().chars().take(180).collect::<String>(),
            b.to_string().chars().take(180).collect::<String>()
        )
    }
    assert!(
        a == b,
        "{context}: {}; full evidence retained",
        first(a, b, "$".into())
    );
}
