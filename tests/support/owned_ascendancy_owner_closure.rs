//! Intrinsic named-Ascendancy metadata closure. Descendant passives, Player
//! initialization and external transformations retain their separate coverage.
use super::passive_publication;
use poe_optimizer_core::{owned_content::digest_owned, owned_schema::DefinitionDescriptor};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "ascendancy-owner-default-inventory";
const DOMAIN: &str = "owned-ascendancy-owner-closure-v1";
const OWNER: &str = "def.0000000000000a36";
const FIELDS: [&str; 7] = [
    "parameters",
    "choices",
    "grants",
    "actors",
    "skill_grants",
    "outputs",
    "sockets",
];
const SUPPORTERS: [&str; 17] = [
    "0a23", "332a", "1790", "1b5d", "10d4", "173f", "0bb3", "0e15", "0b34", "1760", "11c2", "0d0f",
    "14d8", "1b48", "0a6d", "19ec", "0f2a",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/ascendancy-owner-closure")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}
fn pin_bytes(pin: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    let bytes = match pin["normalization"].as_str().unwrap() {
        "raw" => bytes,
        "crlf-to-lf" => String::from_utf8(bytes)
            .unwrap()
            .replace("\r\n", "\n")
            .into_bytes(),
        other => panic!("unreviewed pin domain {other}"),
    };
    assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
    assert_eq!(hash(&bytes), pin["sha256"]);
    bytes
}

pub fn check_rows(d: &Value, c: &Value) {
    for rows in [
        &d["definitions"],
        &d["owners"],
        &c["definitions"],
        &c["owners"],
    ] {
        assert_eq!(rows.as_array().unwrap().len(), 1);
    }
    let old = &d["definitions"][0];
    assert_eq!(old["kind"], "ascendancy");
    assert_eq!(old["value"]["id"]["key"], OWNER);
    let subject = &d["owners"][0]["owner"];
    assert_eq!(subject["value"]["value"], old["value"]["id"]);
    let mut expected = old.clone();
    let declarations = &mut expected["value"]["schema"]["value"]["declarations"];
    assert_eq!(keys(declarations), FIELDS.into_iter().collect());
    for field in FIELDS {
        assert_eq!(declarations[field]["members"], json!([]));
        assert_eq!(
            declarations[field]["closure"],
            json!({"kind":"partial","value":{"gaps":[{"subject":subject,"facet":"input_schema","code":"tree-declarations-not-converted"}]}})
        );
        declarations[field]["closure"] = json!({"kind":"complete"});
    }
    assert_eq!(
        c["definitions"][0], expected,
        "only seven empty inventory closures change"
    );
    let mut owner = d["owners"][0].clone();
    assert_eq!(owner["programs"]["members"], json!([]));
    assert_eq!(
        owner["programs"]["closure"],
        json!({"kind":"partial","value":{"gaps":[{"subject":subject,"facet":"game_rules","code":"tree-game-rules-not-converted"}]}})
    );
    owner["programs"]["closure"] = json!({"kind":"complete"});
    assert_eq!(c["owners"][0], owner);
    assert_eq!(d["slots"], json!([]));
    let definitions = d["supporting_definitions"].as_array().unwrap();
    let owners = d["action_owners"].as_array().unwrap();
    assert_eq!(definitions.len(), SUPPORTERS.len());
    assert_eq!(owners.len(), SUPPORTERS.len());
    for ((definition, owner), id) in definitions.iter().zip(owners).zip(SUPPORTERS) {
        assert_eq!(
            definition["value"]["id"]["key"],
            format!("def.000000000000{id}")
        );
        assert_eq!(owner["owner"]["value"]["value"], definition["value"]["id"]);
        assert_eq!(
            owner["programs"]["closure"]["kind"],
            if ["1790", "1b5d"].contains(&id) {
                "complete"
            } else {
                "partial"
            }
        );
    }
    let schema = &old["value"]["schema"]["value"];
    assert_eq!(
        schema["classes"],
        json!({"members":[definitions[0]["value"]["id"]],"closure":{"kind":"complete"}})
    );
    assert_eq!(
        schema["implicit_passives"],
        json!({"members":[definitions[3]["value"]["id"]],"closure":{"kind":"complete"}})
    );
}

pub fn check_vectors(v: &Value, authenticate: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "passed_retained_full_source_evidence");
    assert_eq!(
        v["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let bindings: Value = read("bindings.json");
    let rows = v["projections"].as_array().unwrap();
    assert_eq!(rows.len(), 6);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(
            row["name"],
            if i == 5 {
                "repeat-original-05".into()
            } else {
                format!("original-{:02}", i + 1)
            }
        );
        assert_eq!(row["tree_version"], "0_5");
        assert_eq!(row["raw_class_id"], 7);
        assert_eq!(row["constructed_class_id"], 7);
        assert_eq!(row["classes_alias_ascendancies"], true);
        assert_eq!(row["exact_selected_tree"], true);
        for key in [
            "exact_selected_tree",
            "exact_selected_class_row",
            "original_constructor",
            "original_initializer",
            "original_methods_preserved",
            "saved_items_preserved",
            "saved_selection_preserved",
            "scalar_outputs_preserved",
        ] {
            assert_eq!(row["evidence"][key], true);
        }
        for key in [
            "class_coverage_closed",
            "shared_actor_state_closed",
            "complete_build_claim",
            "full_output_graph",
            "business_method_wrappers",
            "copied_initializer",
        ] {
            assert_eq!(row["evidence"][key], false);
        }
        assert_eq!(row["constructed"]["index"], 3);
        assert_eq!(
            keys(&row["constructed"]),
            ["fields", "index"].into_iter().collect()
        );
        for (object, field) in [
            (&row["raw"], "source_fields"),
            (&row["constructed"]["fields"], "constructed_fields"),
        ] {
            let policy = bindings[field].as_array().unwrap();
            let mut declared = BTreeSet::new();
            for item in policy {
                let name = item["name"].as_str().unwrap();
                assert!(declared.insert(name));
                assert_eq!(
                    item["disposition"],
                    match name {
                        "background" => "presentation",
                        "id" | "internalId" | "name" => "exact_identity",
                        "startNodeId" => "separate_implicit_passive_owner",
                        _ => panic!("unreviewed Ascendancy field"),
                    }
                );
            }
            assert_eq!(keys(object), declared, "unknown raw or constructed field");
            assert_eq!(object["id"], "Disciple of Varashta");
            assert_eq!(object["name"], "Disciple of Varashta");
            assert_eq!(object["internalId"], "Sorceress3");
            assert_eq!(
                keys(&object["background"]),
                ["height", "image", "section", "width", "x", "y"]
                    .into_iter()
                    .collect()
            );
            assert_eq!(object["background"], rows[0]["raw"]["background"]);
        }
        let mut expected = row["raw"].clone();
        expected["startNodeId"] = json!(8305);
        assert_eq!(
            row["constructed"]["fields"], expected,
            "constructor adds only the separately owned start root"
        );
    }
    assert_eq!(
        v["root_ascendancy"],
        json!({"ascendancy_id":3,"class_id":7,"internal_id":"Sorceress3","name":"Disciple of Varashta","same_root":true,"start_node_id":8305})
    );
    assert_eq!(
        v["root_selected"],
        json!({"allocated_object_identity":true,"class_id":7,"default_modifier_object_identity":false,"default_modifier_value_identity":true,"source_id":8305,"tree_prototype_identity":true})
    );
    assert_eq!(
        v["presentation_consumer"]["path"],
        "vendor/path-of-building-poe2/src/Classes/PassiveTreeView.lua"
    );
    assert_eq!(v["presentation_consumer"]["first_line"], 636);
    assert_eq!(v["presentation_consumer"]["last_line"], 658);
    assert_eq!(
        v["presentation_consumer"]["lines"]
            .as_array()
            .unwrap()
            .len(),
        23
    );
    assert_eq!(v["loaded_metadata"]["case_count"], 6);
    assert_eq!(v["loaded_metadata"]["complete_loads_per_jit"], 12);
    assert_eq!(v["root_metadata"]["complete_loads_per_jit"], 3);
    assert_eq!(v["root_metadata"]["explicit_constructor_probes_per_jit"], 2);
    assert_eq!(v["root_metadata"]["source_root"], 8305);
    assert_eq!(
        v["loaded_driver_history"]["commit"],
        "95b07c0dbd172a4eb72ccfbbb422a2af3f31511b"
    );
    let helpers = v["capture_helpers"].as_array().unwrap();
    assert_eq!(helpers.len(), 5);
    assert_eq!(helpers[0]["sha256"], v["loaded_driver_history"]["sha256"]);
    assert_eq!(
        helpers[1]["sha256"],
        v["loaded_metadata"]["observer_sha256"]
    );
    assert_eq!(
        helpers[2]["sha256"],
        v["loaded_metadata"]["bootstrap_sha256"]
    );
    assert_eq!(helpers[2]["sha256"], v["root_metadata"]["bootstrap_sha256"]);
    assert_eq!(helpers[4]["sha256"], v["root_metadata"]["observer_sha256"]);
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 4);
    for pair in reports.as_chunks::<2>().0 {
        assert_eq!(pair[0]["sha256"], pair[1]["sha256"]);
        assert_eq!(pair[0]["bytes"], pair[1]["bytes"]);
    }
    assert_eq!(
        reports[0]["sha256"],
        "8cbd3841d48b7ac98d9f10668e1ca14f7e80b0a17d9f4bf466786550376103d2"
    );
    assert_eq!(
        reports[2]["sha256"],
        "5202a6d68cd2514643997d13cb69a5b2ac78f46f417fb6feb82d0f2a60c07dca"
    );
    let inventory = v["consumer_inventory"].as_array().unwrap();
    let matched: BTreeSet<_> = inventory
        .iter()
        .filter(|r| !r["matches"].as_array().unwrap().is_empty())
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        matched,
        [
            "vendor/path-of-building-poe2/src/Classes/PassiveSpec.lua",
            "vendor/path-of-building-poe2/src/Classes/PassiveTree.lua",
            "vendor/path-of-building-poe2/src/Modules/Build.lua",
            "vendor/path-of-building-poe2/src/Modules/BuildExportPoE2.lua",
            "vendor/path-of-building-poe2/src/Modules/BuildListHelpers.lua"
        ]
        .into_iter()
        .collect()
    );
    if !authenticate {
        return;
    }
    for pin in helpers {
        pin_bytes(pin);
    }
    let presentation = String::from_utf8(pin_bytes(&v["presentation_consumer"])).unwrap();
    assert_eq!(
        json!(
            presentation
                .split('\n')
                .skip(635)
                .take(23)
                .collect::<Vec<_>>()
        ),
        v["presentation_consumer"]["lines"]
    );
    pin_bytes(&v["root_packet"]);
    let root_authoring: Value = serde_json::from_slice(&pin_bytes(&v["root_authoring"])).unwrap();
    assert_eq!(
        root_authoring["source_validation"]["witness_test_sha256"],
        helpers[3]["sha256"]
    );
    pin_bytes(&v["manifest"]);
    assert_eq!(
        v["manifest"]["sha256"],
        v["loaded_metadata"]["manifest_sha256"]
    );
    assert_eq!(
        v["manifest"]["sha256"],
        v["root_metadata"]["manifest_sha256"]
    );
    for metadata in [&v["loaded_metadata"], &v["root_metadata"]] {
        for file in metadata["files"].as_array().unwrap() {
            let source = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(file["path"].as_str().unwrap()),
            )
            .unwrap();
            assert_eq!(
                hash(source.replace("\r\n", "\n").as_bytes()),
                file["sha256"]
            );
        }
        assert!(
            metadata["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["path"] == "src/TreeData/0_5/tree.lua")
        );
    }
    for pin in v["loaded_metadata"]["original_sources"].as_array().unwrap() {
        assert_eq!(
            hash(&fs::read(root().join(pin["path"].as_str().unwrap())).unwrap()),
            pin["sha256"]
        );
    }
    for pin in v["loaded_metadata"]["external_files"].as_array().unwrap() {
        assert_eq!(pin["normalization"], "crlf-to-lf");
        let text = fs::read_to_string(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(hash(text.replace("\r\n", "\n").as_bytes()), pin["sha256"]);
    }
    let mut actual_paths: BTreeSet<_> =
        fs::read_dir(root().join("vendor/path-of-building-poe2/src/Modules"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "lua"))
            .map(|p| {
                format!(
                    "vendor/path-of-building-poe2/src/Modules/{}",
                    p.file_name().unwrap().to_str().unwrap()
                )
            })
            .collect();
    actual_paths.extend([
        "vendor/path-of-building-poe2/src/Classes/PassiveTree.lua".into(),
        "vendor/path-of-building-poe2/src/Classes/PassiveSpec.lua".into(),
    ]);
    assert_eq!(
        actual_paths,
        inventory
            .iter()
            .map(|r| r["path"].as_str().unwrap().to_owned())
            .collect()
    );
    for file in inventory {
        let source = String::from_utf8(pin_bytes(file)).unwrap();
        let matches: Vec<_> = source
            .split('\n')
            .enumerate()
            .filter(|(_, text)| {
                let lower = text.to_ascii_lowercase();
                lower.contains("curascend") || lower.contains("ascendclass")
            })
            .map(|(i, text)| json!({"line":i+1,"text":text}))
            .collect();
        assert_eq!(json!(matches), file["matches"]);
    }
    for report_pin in reports {
        let report: Value = serde_json::from_slice(&pin_bytes(report_pin)).unwrap();
        let metadata = &v[if report_pin["kind"] == "loaded" {
            "loaded_metadata"
        } else {
            "root_metadata"
        }];
        for (key, value) in metadata.as_object().unwrap() {
            assert_eq!(&report[key], value);
        }
        if report_pin["kind"] == "loaded" {
            assert_eq!(report["cases"].as_array().unwrap().len(), rows.len());
            for (case, expected) in report["cases"].as_array().unwrap().iter().zip(rows) {
                for field in ["name", "xml_sha256"] {
                    assert_eq!(case[field], expected[field]);
                }
                let state = &case["original"]["state"];
                let constructor = &state["constructors"][0];
                assert_eq!(state["constructors"].as_array().unwrap().len(), 1);
                assert_eq!(state["evidence"], expected["evidence"]);
                assert_eq!(state["tree_version"], expected["tree_version"]);
                assert_eq!(case["original"]["selected"], expected["selected"]);
                let raw = constructor["raw_classes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["integerId"] == 7)
                    .unwrap();
                assert_eq!(
                    constructor["raw_classes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|r| r["integerId"] == 7)
                        .count(),
                    1
                );
                let constructed = constructor["constructed_classes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["id"] == 7)
                    .unwrap();
                assert_eq!(
                    constructor["constructed_classes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|r| r["id"] == 7)
                        .count(),
                    1
                );
                assert_eq!(
                    raw["ascendancies"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|r| r["internalId"] == "Sorceress3")
                        .count(),
                    1
                );
                assert_eq!(
                    constructed["ascendancies"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|r| r["index"] == 3)
                        .count(),
                    1
                );
                assert_eq!(raw["ascendancies"][2], expected["raw"]);
                assert_eq!(constructed["ascendancies"][3], expected["constructed"]);
                assert_eq!(
                    constructor["exact_selected_tree"],
                    expected["exact_selected_tree"]
                );
                assert_eq!(
                    constructed["classes_alias_ascendancies"],
                    expected["classes_alias_ascendancies"]
                );
            }
            for field in ["original", "unhooked"] {
                assert_eq!(report["cases"][4][field], report["cases"][5][field]);
            }
        } else {
            assert_eq!(report["original"], report["repeat"]);
            for case in ["original", "repeat", "control"] {
                assert_eq!(report[case]["state"]["selected_root"], v["root_selected"]);
                assert_eq!(
                    report[case]["state"]["ascendancies"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|r| r["class_id"] == 7 && r["ascendancy_id"] == 3)
                        .count(),
                    1
                );
                let observed = report[case]["state"]["ascendancies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["class_id"] == 7 && r["ascendancy_id"] == 3)
                    .unwrap();
                assert_eq!(observed, &v["root_ascendancy"]);
            }
        }
    }
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
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
    for field in ["allocated_definitions", "new_programs", "new_receivers"] {
        assert_eq!(a[field], 0);
    }
    assert_eq!(a["closed_ascendancy_owners"], 1);
    assert_eq!(a["closed_empty_declaration_inventories"], 7);
    assert_eq!(a["registry_last_issued_before"], 0x336a);
    assert_eq!(
        a["registry_last_issued_after"],
        a["registry_last_issued_before"]
    );
    assert_eq!(b["registry_last_issued"], a["registry_last_issued_before"]);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"]["intrinsic_named_ascendancy_only"], true);
    for (field, value) in a["scope"].as_object().unwrap() {
        if field != "intrinsic_named_ascendancy_only" {
            assert_eq!(value, false);
        }
    }
    assert_eq!(b["declaration_fields"], json!(FIELDS));
    assert_eq!(b["ascendancy"], d["definitions"][0]["value"]["id"]);
    assert_eq!(b["class"], d["supporting_definitions"][0]["value"]["id"]);
    assert_eq!(b["root"], d["supporting_definitions"][3]["value"]["id"]);
    assert_eq!(b["source_class"], 7);
    assert_eq!(b["source_ascendancy"], 3);
    assert_eq!(b["source_root"], 8305);
    assert_eq!(
        b["preserved_owners"],
        json!(
            d["action_owners"]
                .as_array()
                .unwrap()
                .iter()
                .map(|o| &o["owner"])
                .collect::<Vec<_>>()
        )
    );
    let mapping = d["mapping_rows"].as_array().unwrap();
    assert_eq!(mapping.len(), 3);
    for (row, (source, target)) in mapping.iter().zip([
        (json!({"kind":"definition","value":{"kind":"class","value":{"key":{"kind":"text","value":"7"}}}}), &b["class"]),
        (json!({"kind":"definition","value":{"kind":"ascendancy","value":{"class":{"kind":"text","value":"7"},"key":{"kind":"text","value":"Sorceress3"}}}}), &b["ascendancy"]),
        (json!({"kind":"definition","value":{"kind":"passive_node","value":{"tree_version":{"kind":"text","value":"0_5"},"node_id":{"kind":"text","value":"8305"},"view":{"kind":"missing"}}}}), &b["root"]),
    ]) {
        assert_eq!(row["source"], source);
        assert_eq!(row["outcome"]["kind"], "mapped");
        assert_eq!(&row["outcome"]["value"]["target"]["value"]["value"], target);
        assert_eq!(row["outcome"]["value"]["basis"], json!({"kind":"exact"}));
    }
    let mut names = BTreeSet::new();
    for row in a["artifacts"].as_array().unwrap() {
        let name = row["file"].as_str().unwrap();
        assert!(names.insert(name));
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(bytes.len() as u64, row["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), row["sha256"]);
        assert!(!bytes.contains(&b'\r'));
    }
    assert_eq!(
        names,
        [
            "bindings.json",
            "dependencies.json",
            "closure.json",
            "migration.json",
            "source-vectors.json"
        ]
        .into_iter()
        .collect()
    );
    check_rows(&d, &c);
    check_vectors(&v, false);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    check_vectors(&v, true);
    let next = passive_publication::stage_declaration_refinement(prior, &data(), KIND, DOMAIN);
    let proof = next.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(proof.prior_input, prior.receipt().input);
    assert_eq!(
        proof.authoring_input,
        digest_owned(DOMAIN, &(a, b, d, c.clone(), v), 4 * 1024 * 1024).unwrap()
    );
    let expected: DefinitionDescriptor =
        serde_json::from_value(c["definitions"][0].clone()).unwrap();
    assert!(next.input().recipe.schema.definitions.contains(&expected));
    assert_eq!(next.input().recipe.registry, prior.input().recipe.registry);
    next
}

pub fn assert_source_from_disk() {
    check_vectors(&read("source-vectors.json"), true);
}
