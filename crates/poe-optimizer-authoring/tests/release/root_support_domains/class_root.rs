//! Archived constructor evidence and corruption laws; no publication or VM execution.
use super::{FIELDS, hash, root};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/class-start-root")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|map| map.is_empty()));
        &[]
    }
}
/// Only the eight explicitly reviewed inventory closure markers may change.
pub fn check_rows(dependencies: &Value, closure: &Value) {
    assert_eq!(dependencies["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(dependencies["owners"].as_array().unwrap().len(), 1);
    assert_eq!(closure["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(closure["owners"].as_array().unwrap().len(), 1);
    let old = &dependencies["definitions"][0];
    assert_eq!(old["kind"], "passive_node");
    assert_eq!(old["value"]["id"]["key"], "def.0000000000001790");
    assert_eq!(old["value"]["schema"]["kind"], "known");
    let mut expected = old.clone();
    let declarations = &mut expected["value"]["schema"]["value"]["declarations"];
    assert_eq!(declarations.as_object().unwrap().len(), FIELDS.len());
    let subject = &dependencies["owners"][0]["owner"];
    assert_eq!(subject["value"]["value"], old["value"]["id"]);
    for field in FIELDS {
        assert_eq!(declarations[field]["members"], json!([]));
        assert_eq!(
            declarations[field]["closure"],
            json!({"kind":"partial","value":{"gaps":[{
                "subject":subject,"facet":"input_schema","code":"tree-declarations-not-converted"
            }]}})
        );
        declarations[field]["closure"] = json!({"kind":"complete"});
    }
    assert_eq!(closure["definitions"][0], expected);
    let mut expected_owner = dependencies["owners"][0].clone();
    assert_eq!(expected_owner["programs"]["members"], json!([]));
    assert_eq!(
        expected_owner["programs"]["closure"],
        json!({"kind":"partial","value":{"gaps":[{
            "subject":subject,"facet":"game_rules","code":"tree-game-rules-not-converted"
        }]}})
    );
    expected_owner["programs"]["closure"] = json!({"kind":"complete"});
    assert_eq!(closure["owners"][0], expected_owner);
    assert_eq!(dependencies["slots"], json!([]));
    assert_eq!(
        dependencies["supporting_definitions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(dependencies["action_owners"].as_array().unwrap().len(), 2);
    for (index, key) in ["def.0000000000000a1e", "def.0000000000000a23"]
        .iter()
        .enumerate()
    {
        let class = &dependencies["supporting_definitions"][index];
        let owner = &dependencies["action_owners"][index];
        assert_eq!(class["kind"], "class");
        assert_eq!(class["value"]["id"]["key"], *key);
        assert_eq!(owner["owner"]["value"]["value"], class["value"]["id"]);
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
        assert_eq!(owner["programs"]["members"].as_array().unwrap().len(), 3);
        assert!(
            class["value"]["schema"]["value"]["implicit_passives"]["members"]
                .as_array()
                .unwrap()
                .contains(&old["value"]["id"])
        );
    }
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    for (field, expected) in [
        ("allocated_definitions", 0),
        ("new_programs", 0),
        ("new_receivers", 0),
        ("closed_passive_owners", 1),
        ("closed_empty_declaration_inventories", 7),
    ] {
        assert_eq!(a[field], expected);
    }
    assert_eq!(a["registry_last_issued_before"], 0x3320);
    assert_eq!(a["registry_last_issued_after"], 0x3320);
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(
            a[field],
            d["source"][if field == "before" { "input" } else { field }]
        );
    }
    assert_eq!(b["source_node"], 54447);
    assert_eq!(b["node"]["key"], "def.0000000000001790");
    assert_eq!(b["declaration_fields"], json!(FIELDS));
    assert_eq!(b["scope"], a["scope"]);
    assert_eq!(a["scope"]["default_root_only"], true);
    for field in [
        "class_rule_closure_changed",
        "universal_player_initialization_closed",
        "external_transformations_closed",
        "whole_build_parity",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    let names: BTreeSet<_> = a["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let name = row["file"].as_str().unwrap();
            let bytes = fs::read(data().join(name)).unwrap();
            assert_eq!(bytes.len() as u64, row["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), row["sha256"]);
            assert!(!bytes.contains(&b'\r'));
            name.to_owned()
        })
        .collect();
    assert_eq!(
        names,
        [
            "bindings.json",
            "dependencies.json",
            "closure.json",
            "migration.json",
            "source-vectors.json"
        ]
        .map(str::to_owned)
        .into_iter()
        .collect()
    );
    check_rows(&d, &c);
    check_vectors(&a, &v);
    authenticate_source(&a, &v, false);
}

/// Full constructor projection preserves the exact intrinsic inventory.
pub fn check_vectors(a: &Value, v: &Value) {
    check_root_vectors(a, v, false);
}

/// Shared complete root evidence validation; source labels do not substitute for ownership.
pub fn check_root_vectors(a: &Value, v: &Value, ascendancy: bool) {
    let root_id = if ascendancy { 8305 } else { 54447 };
    assert_eq!(v["status"], "passed");
    assert_eq!(a["source_validation"]["status"], "passed");
    let s = &v["projection"];
    assert_eq!(s["executed"], true);
    assert_eq!(s["raw"]["skill"], root_id);
    assert!(rows(&s["raw"]["stats"]).is_empty());
    if ascendancy {
        assert_eq!(s["raw"]["isAscendancyStart"], true);
        assert_eq!(s["raw"]["ascendancyName"], "Disciple of Varashta");
        assert!(s["raw"].get("classesStart").is_none());
    } else {
        assert_eq!(s["raw"]["classesStart"], json!(["Witch", "Sorceress"]));
    }
    let node = &s["constructed"];
    assert_eq!(node["id"], root_id);
    assert_eq!(
        node["type"],
        if ascendancy {
            "AscendClassStart"
        } else {
            "ClassStart"
        }
    );
    assert_eq!(node["default_mod_count"], 0);
    assert_eq!(node["fields"]["modKey"], "");
    for field in ["mods", "stats", "sd"] {
        assert!(rows(&node["fields"][field]).is_empty());
    }
    assert_eq!(s["constructor_probe"], *node);
    let mods = &node["default_modifiers"];
    assert!(rows(&mods["records"]).is_empty());
    assert_eq!(mods["fields"].as_object().unwrap().len(), 4);
    assert_eq!(mods["fields"]["parent"], false);
    for field in ["actor", "multipliers", "conditions"] {
        assert!(mods["fields"][field].as_object().unwrap().is_empty());
    }
    assert_eq!(mods["metatable"], "ModList");
    assert_eq!(mods["parent_constructor_verified"], true);
    let own_keys: BTreeSet<_> = rows(&mods["own_keys"])
        .iter()
        .map(|row| row["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        own_keys,
        [
            "Object",
            "ModStore",
            "_parentInit",
            "actor",
            "conditions",
            "multipliers",
            "parent"
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(own_keys.len(), rows(&mods["own_keys"]).len());
    let declarations = rows(&node["declaration_fields"]);
    let names: BTreeSet<_> = declarations
        .iter()
        .map(|row| {
            if ascendancy
                && ["isAscendancyStart", "ascendancyName"].contains(&row["name"].as_str().unwrap())
            {
                assert_eq!(row["present"], true);
                assert_eq!(row["value"], s["raw"][row["name"].as_str().unwrap()]);
            } else {
                assert_eq!(row["present"], false);
                assert!(row.get("value").is_none());
            }
            row["name"].as_str().unwrap()
        })
        .collect();
    assert_eq!(names.len(), declarations.len());
    assert_eq!(
        names,
        [
            "isJewelSocket",
            "containJewelSocket",
            "charmSocket",
            "expansionJewel",
            "noRadius",
            "isAttribute",
            "isSwitchable",
            "options",
            "isMastery",
            "masteryEffects",
            "isKeystone",
            "keystoneMod",
            "isAscendancyStart",
            "ascendancyName",
            "isProxy",
            "isOnlyImage",
            "recipe",
            "unknown",
            "extra",
            "grantedSkill",
            "grantedSkills",
            "grantedPassive",
            "grantedPassives",
            "socket",
            "sockets",
            "skillId",
        ]
        .into_iter()
        .collect()
    );
    // Every constructed own field is retained, except identity-verified self,
    // the separately complete modifier store, and named render geometry.
    let inventory = rows(&node["field_inventory"]);
    assert!(
        inventory
            .windows(2)
            .all(|pair| pair[0]["key"].as_str() < pair[1]["key"].as_str())
    );
    let mut represented: BTreeSet<_> = node["fields"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(represented.insert("modList"));
    for row in rows(&node["excluded"]) {
        let key = row["key"].as_str().unwrap();
        assert!(represented.insert(key));
        match key {
            "__index" => assert_eq!(row["reason"], "self_identity_verified"),
            "group" => {
                assert_eq!(row["reason"], "geometry_group");
                assert_eq!(row["source_group"], s["raw"]["group"]);
            }
            "overlay" | "targetSize" => assert_eq!(row["reason"], "render_geometry"),
            _ => panic!("unreviewed constructed field exclusion"),
        }
    }
    assert_eq!(
        represented,
        inventory
            .iter()
            .map(|row| row["key"].as_str().unwrap())
            .collect()
    );
    let mut selected_root = json!({"source_id":root_id,"class_id":7,
        "tree_prototype_identity":true,"allocated_object_identity":true,"default_modifier_object_identity":!ascendancy});
    if ascendancy {
        selected_root["default_modifier_value_identity"] = json!(true);
        assert_eq!(s["selected_modifiers"], node["default_modifiers"]);
    }
    assert_eq!(s["selected_root"], selected_root);
    assert_eq!(
        s["selected"],
        json!({"items":2,"spec":3,"skills":4,"config":1,"group":3})
    );
    let classes = rows(&s["classes"]);
    if ascendancy {
        assert!(classes.is_empty());
        assert_eq!(
            s["ascendancies"],
            json!([{"class_id":7,"ascendancy_id":3,
            "internal_id":"Sorceress3","name":"Disciple of Varashta","start_node_id":root_id,"same_root":true}])
        );
    } else {
        assert_eq!(classes.len(), 2);
        for (row, (id, name)) in classes.iter().zip([(1, "Witch"), (7, "Sorceress")]) {
            assert_eq!(row["class_id"], id);
            assert_eq!(row["name"], name);
            assert_eq!(row["start_node_id"], root_id);
            assert_eq!(row["same_root"], true);
        }
    }
    for (field, path, first) in [
        ("constructor_wrapper", "Modules/Common.lua", 167),
        ("constructor", "Classes/PassiveTree.lua", 59),
        ("process_stats", "Classes/PassiveTree.lua", 448),
        ("process_node", "Classes/PassiveTree.lua", 528),
        ("select_class", "Classes/PassiveSpec.lua", 655),
        ("reconnect", "Classes/PassiveSpec.lua", 2067),
        ("build_node_mods", "Modules/CalcSetup.lua", 200),
    ] {
        assert_eq!(s["methods"][field]["path"], path);
        assert_eq!(s["methods"][field]["first"], first);
    }
    let neighbors = rows(&s["neighbor_connection_flags"]);
    assert!(!neighbors.is_empty());
    for neighbor in neighbors {
        let id = neighbor["container_node_id"].as_u64().unwrap();
        assert_ne!(id, root_id);
        assert_eq!(neighbor["distinct_from_root"], true);
        for flag in rows(&neighbor["connection_flags"]) {
            assert_eq!(flag["type"], "FLAG");
            assert_eq!(flag["value"], true);
            assert_eq!(flag["source"], format!("Tree:{id}"));
        }
    }
    for field in [
        "original_constructor",
        "original_process_stats",
        "original_methods_preserved",
        "exact_raw_descriptor",
        "complete_intrinsic_modifier_fields",
        "selected_default_root_unchanged",
        "cached_scalar_outputs_preserved",
        "saved_selection_preserved",
    ] {
        assert_eq!(s["evidence"][field], true);
    }
    assert_eq!(s["evidence"]["both_class_roots_identical"], !ascendancy);
    if ascendancy {
        assert_eq!(s["evidence"]["exact_selected_ascendancy_root"], true);
    }
    for field in [
        "class_owner_closed",
        "universal_player_initialization_closed",
        "external_transformations_closed",
        "all_same_source_labels_owned_by_root",
        "complete_build_claim",
    ] {
        assert_eq!(s["evidence"][field], false);
    }
    let metadata = &v["metadata"];
    assert_eq!(metadata["source_revision"], a["source_revision"]);
    assert_eq!(metadata["complete_loads_per_jit"], 3);
    assert_eq!(metadata["explicit_constructor_probes_per_jit"], 2);
    let files = rows(&metadata["files"]);
    assert_eq!(files.len(), 7);
    assert_eq!(
        files
            .iter()
            .map(|row| row["path"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        [
            "src/Classes/PassiveTree.lua",
            "src/Classes/PassiveSpec.lua",
            "src/Modules/CalcSetup.lua",
            "src/Modules/Common.lua",
            "src/Classes/ModList.lua",
            "src/Classes/ModStore.lua",
            "src/TreeData/0_5/tree.lua",
        ]
        .into_iter()
        .collect()
    );
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    for (row, mode) in reports.iter().zip(["off", "on"]) {
        assert_eq!(row["jit"], mode);
        assert!(
            row["path"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/source-jit-{mode}.json"))
        );
        if !ascendancy {
            assert_eq!(row["bytes"], 229_933);
            assert_eq!(
                row["sha256"],
                "869eb7f6bce5267d0a06970ce75e56f125d59b0c5729ff71ef5abc0b3105d5b4"
            );
        } else {
            assert_eq!(row["bytes"], a["source_validation"]["report_bytes"]);
            assert_eq!(row["sha256"], a["source_validation"]["report_sha256"]);
        }
    }
}

/// Authenticate full source bytes and capture provenance without loading historical run reports.
pub fn authenticate_source(a: &Value, v: &Value, ascendancy: bool) {
    let metadata = &v["metadata"];
    let files = metadata["files"].as_array().unwrap();
    for row in files {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(row["path"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(hash(text.replace("\r\n", "\n").as_bytes()), row["sha256"]);
    }
    // These two pin domains match source::read_verified_text and
    // manifest_sha256. Observer, report and fixture pins remain raw bytes.
    let manifest =
        fs::read_to_string(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .unwrap();
    assert_eq!(
        hash(manifest.replace("\r\n", "\n").as_bytes()),
        metadata["manifest_sha256"]
    );
    for (path, field) in [
        (
            if ascendancy {
                "crates/poe-optimizer-pob/tests/support/implicit_class_start_source.lua"
            } else {
                "data/owned/poe2/3887ae68/class-start-root/evidence/implicit_class_start_source.lua"
            },
            "observer_sha256",
        ),
        (
            "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs",
            "bootstrap_sha256",
        ),
    ] {
        assert_eq!(hash(&fs::read(root().join(path)).unwrap()), metadata[field]);
    }
    let source = &metadata["original_source"];
    assert_eq!(
        source["path"],
        "tests/fixtures/builds/breadth-20260908/build-05.xml"
    );
    assert_eq!(
        hash(&fs::read(root().join(source["path"].as_str().unwrap())).unwrap()),
        source["sha256"]
    );
    assert_eq!(
            hash(
                &fs::read(
                    root().join(
                        if ascendancy { "crates/poe-optimizer-pob/tests/owned_implicit_class_start_source.rs" } else { "data/owned/poe2/3887ae68/class-start-root/evidence/owned_implicit_class_start_source.rs" }
                    )
                )
                .unwrap()
            ),
            a["source_validation"]["witness_test_sha256"]
        );
}
