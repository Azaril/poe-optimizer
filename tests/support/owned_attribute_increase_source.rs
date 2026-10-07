//! Reuse complete original attribute calls and whole-list passive acquisition.
//! This is an offline proof gate, not a source evaluator or native order policy.
use super::family;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const DATA: &str = "data/owned/poe2/3887ae68";
const ATTRIBUTES: [&str; 3] = ["Str", "Dex", "Int"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        // Retained Lua diagnostics distinguish an empty source table from a
        // missing value. This accommodation never enters an owned runtime.
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn check_pin(path: &str, pin: &Value) {
    let bytes = fs::read(root().join(path)).unwrap();
    assert_eq!(pin["bytes"], bytes.len(), "length: {path}");
    assert_eq!(pin["sha256"], hash(&bytes), "digest: {path}");
}
fn old_attribute(key: &Value) -> Option<usize> {
    (0..3).find(|i| *key == format!("def.{:016x}", 0x1d2e + i))
}

fn project_nodes() -> Vec<Value> {
    let catalog = read(&format!("{DATA}/tree/tree-catalog.json"));
    let count = read(&format!("{DATA}/attribute-stages/count-programs.json"));
    let dependencies: Value = family::read("dependencies.json");
    let policies: Vec<_> = ["passive-attribute-inputs", "passive-defence-inputs"]
        .map(|folder| {
            (
                folder,
                read(&format!("{DATA}/{folder}/policy.json")),
                read(&format!("{DATA}/{folder}/bindings.json")),
            )
        })
        .into();
    let mut out = vec![];
    let mut totals = [0_u64; 3];
    let mut effects = 0;
    for transition in rows(&count["programs"]) {
        let before = &transition["before"];
        let increases: Vec<_> = rows(&before["effects"])
            .iter()
            .filter(|e| {
                e["effect"]["kind"] == "contribute"
                    && e["effect"]["contribution"] == "increase"
                    && old_attribute(&e["effect"]["stat"]["key"]).is_some()
            })
            .collect();
        if increases.is_empty() {
            continue;
        }
        assert_eq!(before["id"], "passive-view");
        assert!(rows(&before["reads"]).is_empty());
        let owner = &transition["owner"]["value"]["value"];
        let current: Vec<_> = rows(&dependencies["owners"])
            .iter()
            .filter(|o| o["owner"] == transition["owner"])
            .collect();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0]["programs"]["closure"], transition["closure"]);
        let current_programs: Vec<_> = rows(&current[0]["programs"]["members"])
            .iter()
            .filter(|p| p["id"] == transition["after"]["id"])
            .collect();
        assert_eq!(current_programs, vec![&transition["after"]]);
        let matches: Vec<_> = policies
            .iter()
            .flat_map(|(folder, policy, bindings)| {
                rows(&bindings["bindings"])
                    .iter()
                    .filter(move |b| b["owner"] == *owner)
                    .map(move |binding| (*folder, policy, binding))
            })
            .collect();
        assert_eq!(matches.len(), 1);
        let (folder, policy, binding) = matches[0];
        let source_node = &binding["source_node"];
        let node = rows(&catalog["nodes"])
            .iter()
            .find(|n| n["key"] == *source_node)
            .unwrap();
        let policy_node = rows(&policy["nodes"])
            .iter()
            .find(|n| n["node"] == *source_node)
            .unwrap();
        assert_eq!(node["stats"], binding["expected_stats"]);
        assert_eq!(node["stats"], policy_node["default"]["expected_stats"]);
        assert!(rows(&node["views"]).is_empty() && rows(&node["unlock"]).is_empty());
        assert!(rows(&policy_node["views"]).is_empty());
        assert_eq!(node["kind"]["value"]["pool"], policy_node["pool"]);
        let declared: Vec<_> = rows(&policy_node["default"]["contributions"])
            .iter()
            .filter(|e| {
                e["contribution"] == "increase" && old_attribute(&e["stat"]["key"]).is_some()
            })
            .collect();
        assert_eq!(declared.len(), increases.len());
        for (effect, declared) in increases.into_iter().zip(declared) {
            let body = &effect["effect"];
            assert_eq!(body["entity"], "player");
            assert_eq!(effect["when"], "default");
            let literal = rows(&before["nodes"])
                .iter()
                .find(|n| n["id"] == body["value"])
                .unwrap();
            assert_eq!(literal["expression"]["kind"], "literal");
            // Acquisition JSON may spell an exact integral quantity as `3`,
            // while typed rule serialization spells it `3.0`. Compare the
            // owned value contract without relaxing units or numeric equality.
            assert_eq!(
                serde_json::from_value::<poe_optimizer_core::owned_build::ParameterValue>(
                    literal["expression"]["value"].clone()
                )
                .unwrap(),
                serde_json::from_value::<poe_optimizer_core::owned_build::ParameterValue>(
                    declared["value"].clone()
                )
                .unwrap(),
            );
            assert_eq!(body["stat"], declared["stat"]);
            let value = declared["value"]["value"]["value"].as_f64().unwrap();
            assert_eq!(declared["value"]["kind"], "quantity");
            assert_eq!(
                declared["value"]["value"]["unit"]["key"],
                "def.0000000000000002"
            );
            assert!(value.fract() == 0. && (3.0..=8.0).contains(&value));
            totals[old_attribute(&body["stat"]["key"]).unwrap()] += value as u64;
            effects += 1;
        }
        out.push(
            json!({"source_node":source_node,"owner":owner,"policy_folder":folder,
            "binding":binding,"catalog_node":node,"policy_node":policy_node}),
        );
    }
    assert_eq!(out.len(), 9);
    assert_eq!(rows(&dependencies["owners"]).len(), out.len());
    assert_eq!(effects, 17);
    assert_eq!(totals, [29, 24, 22]);
    out
}

fn project_case(case: &Value, index: usize) -> Value {
    let mut modes = serde_json::Map::new();
    for mode in ["MAIN", "CALCS"] {
        let m = &case["original"]["state"]["modes"][mode];
        assert_eq!(m["provenance"]["original_actor_caller"], true);
        assert_eq!(m["provenance"]["exact_player_output_table"], true);
        let stages: Vec<_> = rows(&m["provenance"]["stages"])
            .iter()
            .map(|s| {
                assert_eq!(rows(&s["before_chain"]).len(), 1);
                let chain = &s["before_chain"][0];
                let records: Vec<_> = rows(&chain["attributes"][s["stat"].as_str().unwrap()])
                    .iter()
                    .filter(|r| r["record"]["type"] == "INC")
                    .cloned()
                    .collect();
                let reads: Vec<_> = rows(&s["reads"])
                    .iter()
                    .map(|r| {
                        json!({
                "kind":r["kind"],"name":r["name"],"store_depth":r["store_depth"],
                "actual_call_observed":r["actual_call_observed"],
                "direct_return_value_claim":r["direct_return_value_claim"],
                "same_state_original_replay":r["same_state_original_replay"],
                "replay_inputs_unchanged":r["replay_inputs_unchanged"]})
                    })
                    .collect();
                json!({"index":s["index"],"pass":s["pass"],"stat":s["stat"],"value":s["value"],
                "depth":chain["depth"],"parent_kind":chain["parent_kind"],
                "increase_records":records,"queries":s["queries"],"reads":reads})
            })
            .collect();
        modes.insert(
            mode.into(),
            json!({"class_id":m["class_id"],"stages":stages}),
        );
    }
    json!({"case_index":index,"name":case["name"],"xml_sha256":case["xml_sha256"],
        "selected":case["original"]["selected"],"source_selection":case["original"]["state"]["selected"],"modes":modes})
}
fn query<'a>(stage: &'a Value, kind: &str) -> &'a Value {
    let matches: Vec<_> = rows(&stage["queries"])
        .iter()
        .filter(|q| q["contribution"] == kind)
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
fn project_native(case: &Value, mode: &str) -> Value {
    let stages: Vec<_> = rows(&case["modes"][mode]["stages"])
        .iter()
        .map(|s| {
            let sources: Vec<_> = rows(&s["increase_records"])
                .iter()
                .map(|r| {
                    r["record"]["source"]
                        .as_str()
                        .unwrap()
                        .strip_prefix("Tree:")
                        .unwrap()
                })
                .collect();
            json!({"index":s["index"],"pass":s["pass"],"stat":s["stat"],
            "base":query(s,"BASE")["result"],"increase":query(s,"INC")["result"],
            "result":s["value"],"increase_sources":sources})
        })
        .collect();
    json!({"case_index":case["case_index"],"name":case["name"],"selected":case["selected"],
        "source_selection":case["source_selection"],"stages":stages})
}
pub fn native_cases() -> Vec<Value> {
    let v: Value = family::read("source-vectors.json");
    rows(&v["native_cases"]).to_vec()
}
fn without_capture(mut v: Value) -> Value {
    v["state"].as_object_mut().unwrap().remove("hooked");
    for mode in ["MAIN", "CALCS"] {
        v["state"]["modes"][mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    v
}

pub fn verify(full: bool) {
    let v: Value = family::read("source-vectors.json");
    assert_eq!(v["schema_version"], 1);
    assert_eq!(
        v["scope"],
        json!({"actual_all_five_increase_queries":true,
        "whole_list_passive_acquisition_reused":true,"new_source_execution":false,
        "native_base_order_claim":false,"source_passive_sort_claim":false,
        "global_contributor_closure":false,"full_build_claim":false})
    );
    assert_eq!(v["artifacts"].as_object().unwrap().len(), 6);
    assert_eq!(v["source_pins"].as_object().unwrap().len(), 5);
    assert_eq!(
        v["constructor_reference_tests"],
        json!([
            {"target":"owned_passive_attribute_reference","test":"whole_original_tree_stat_processing_matches_all_policy_contributions"},
            {"target":"owned_passive_defence_reference","test":"whole_original_passive_processing_and_moddb_queries_match_defensive_contributions"}
        ])
    );
    for field in ["artifacts", "source_pins"] {
        for (path, pin) in v[field].as_object().unwrap() {
            check_pin(path, pin);
        }
    }
    assert_eq!(v["nodes"], json!(project_nodes()));
    let source_ids: BTreeSet<_> = rows(&v["nodes"])
        .iter()
        .map(|n| n["source_node"].as_str().unwrap())
        .collect();
    assert_eq!(source_ids.len(), 9);
    assert_eq!(rows(&v["cases"]).len(), 5);
    for (i, case) in rows(&v["cases"]).iter().enumerate() {
        assert_eq!(case["name"], format!("original-{:02}", i + 1));
        assert_eq!(case["case_index"], i);
        let path = format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            i + 1
        );
        assert_eq!(
            case["xml_sha256"],
            hash(&fs::read(root().join(path)).unwrap())
        );
        for mode in ["MAIN", "CALCS"] {
            assert_eq!(rows(&case["modes"][mode]["stages"]).len(), 6);
            for (j, stage) in rows(&case["modes"][mode]["stages"]).iter().enumerate() {
                assert_eq!(stage["index"], j + 1);
                assert_eq!(stage["pass"], j / 3 + 1);
                assert_eq!(stage["stat"], ATTRIBUTES[j % 3]);
                assert_eq!(stage["depth"], 0);
                assert_eq!(stage["parent_kind"], "false");
                assert_eq!(rows(&stage["queries"]).len(), 3);
                for kind in ["BASE", "INC", "MORE"] {
                    let q = query(stage, kind);
                    assert_eq!(q["name"], stage["stat"]);
                    assert_eq!(q["store_depth"], 0);
                    assert_eq!(q["actual_return_local"], true);
                    assert_eq!(q["original_context_is_player"], true);
                    assert_eq!(q["flags"], 0);
                    assert_eq!(q["keyword_flags"], 0);
                }
                let mut total = 0.;
                for row in rows(&stage["increase_records"]) {
                    let r = &row["record"];
                    assert_eq!(r["name"], stage["stat"]);
                    assert_eq!(r["type"], "INC");
                    assert_eq!(r["flags"], 0);
                    assert_eq!(r["keyword_flags"], 0);
                    assert_eq!(r["tag_count"], 0);
                    assert_eq!(r["value"]["root"]["kind"], "number");
                    let id = r["source"].as_str().unwrap().strip_prefix("Tree:").unwrap();
                    assert!(source_ids.contains(id));
                    let node = rows(&v["nodes"])
                        .iter()
                        .find(|n| n["source_node"] == id)
                        .unwrap();
                    let contributions: Vec<_> =
                        rows(&node["policy_node"]["default"]["contributions"])
                            .iter()
                            .filter(|c| {
                                c["contribution"] == "increase"
                                    && old_attribute(&c["stat"]["key"]) == Some(j % 3)
                            })
                            .collect();
                    assert_eq!(contributions.len(), 1);
                    assert_eq!(
                        r["value"]["root"]["value"].as_f64(),
                        contributions[0]["value"]["value"]["value"].as_f64()
                    );
                    total += r["value"]["root"]["value"].as_f64().unwrap();
                }
                assert_eq!(query(stage, "INC")["result"].as_f64(), Some(total));
                for read in rows(&stage["reads"]) {
                    assert_eq!(read["store_depth"], 0);
                    assert_eq!(read["actual_call_observed"], true);
                    assert_eq!(read["direct_return_value_claim"], false);
                    assert_eq!(read["replay_inputs_unchanged"], true);
                }
            }
        }
        let projected = project_native(case, "MAIN");
        assert_eq!(projected, project_native(case, "CALCS"));
        assert_eq!(projected, v["native_cases"][i]);
    }
    if !full {
        return;
    }
    let manifest_path = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
    let manifest_text = fs::read_to_string(root().join(manifest_path))
        .unwrap()
        .replace("\r\n", "\n");
    let manifest: Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(
        v["metadata"]["manifest_sha256"],
        hash(manifest_text.as_bytes())
    );
    assert_eq!(
        v["metadata"]["source_revision"],
        manifest["upstream_revision"]
    );
    assert_eq!(
        v["catalog_source"],
        read(&format!("{DATA}/tree/tree-catalog.json"))["source"]
    );
    for pin in rows(&v["metadata"]["files"])
        .iter()
        .chain(rows(&v["catalog_source"]["files"]))
    {
        let path = pin["path"].as_str().unwrap();
        let text = fs::read_to_string(root().join("vendor/path-of-building-poe2").join(path))
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(pin["sha256"], hash(text.as_bytes()), "source file: {path}");
        if path.ends_with(".lua") {
            assert!(
                rows(&manifest["files"])
                    .iter()
                    .any(|m| m["path"] == pin["path"] && m["sha256"] == pin["sha256"])
            );
        }
    }
    assert_eq!(
        v["metadata"]["observer_sha256"],
        v["source_pins"]["crates/poe-optimizer-pob/tests/support/attribute_pipeline_source.lua"]["sha256"]
    );
    assert_eq!(
        v["metadata"]["bootstrap_sha256"],
        v["source_pins"]["crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs"]
            ["sha256"]
    );
    assert_eq!(rows(&v["source_reports"]).len(), 2);
    let mut prior_hash = None;
    for pin in rows(&v["source_reports"]) {
        let path = pin["path"].as_str().unwrap();
        check_pin(path, pin);
        if let Some(prior) = prior_hash.replace(pin["sha256"].clone()) {
            assert_eq!(prior, pin["sha256"]);
        }
        let mut report = read(path);
        let cases = report.as_object_mut().unwrap().remove("cases").unwrap();
        report.as_object_mut().unwrap().remove("native_cases");
        assert_eq!(report, v["metadata"]);
        assert_eq!(rows(&cases).len(), 7);
        assert_eq!(report["case_count"], 7);
        assert_eq!(report["complete_loads_per_jit"], 14);
        assert!(
            cases[4]["original"] == cases[6]["original"],
            "independent restoration changed"
        );
        for case in rows(&cases) {
            assert!(
                without_capture(case["original"].clone())
                    == without_capture(case["unhooked"].clone()),
                "hookless state changed: {}",
                case["name"]
            );
        }
        let projected: Vec<_> = rows(&cases)
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, c)| project_case(c, i))
            .collect();
        assert!(
            json!(projected) == v["cases"],
            "full source reprojection differs"
        );
    }
}
