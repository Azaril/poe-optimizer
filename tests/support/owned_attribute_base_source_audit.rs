//! Bounded audit of retained original BASE calls; this is not native mechanics.
//! Raw record sums and original getter results deliberately remain separate.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const VECTORS: &str = "data/owned/poe2/3887ae68/attribute-base-audit/source-vectors.json";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const ATTRIBUTES: [&str; 3] = ["Str", "Dex", "Int"];
const COUNTS: [[usize; 3]; 5] = [
    [16, 6, 14],
    [8, 28, 12],
    [11, 21, 15],
    [19, 12, 32],
    [5, 1, 19],
];
const RAW_SUMS: [[f64; 3]; 5] = [
    [82., 32., 147.],
    [102., 185., 96.],
    [85., 136., 117.],
    [111., 65., 161.],
    [27., 7., 105.],
];
const GETTER_RESULTS: [[f64; 3]; 5] = [
    [82., 32., 147.],
    [92., 165., 96.],
    [85., 136., 117.],
    [111., 65., 161.],
    [27., 7., 105.],
];
const ZERO_COPIES: [[usize; 3]; 5] = [[0, 0, 2], [1, 1, 1], [1, 1, 1], [1, 1, 1], [0, 0, 0]];
const ITEM_COUNTS: [[usize; 3]; 5] = [[0, 0, 4], [3, 2, 2], [2, 1, 2], [1, 1, 1], [0, 0, 0]];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        // Only the retained source JSON's explicit empty-table diagnostic.
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn check_pin(path: &str, pin: &Value) {
    let bytes = fs::read(root().join(path)).unwrap();
    assert_eq!(pin["bytes"], bytes.len(), "length: {path}");
    assert_eq!(pin["sha256"], hash(&bytes), "digest: {path}");
}
fn scope() -> Value {
    json!({"retained_source_audit":true,"all_five_actual_base_calls":true,
        "complete_ordered_base_records":true,"full_modifier_graphs":true,
        "observed_condition_replays_only":true,"new_source_execution":false,
        "native_contributor_closure":false,"native_order_policy":false,
        "native_numeric_law":false,"full_build_parity":false})
}

fn project_case(case: &Value, index: usize) -> Value {
    let mut modes = serde_json::Map::new();
    for mode in ["MAIN", "CALCS"] {
        let m = &case["original"]["state"]["modes"][mode];
        let provenance = &m["provenance"];
        let stages: Vec<_> = rows(&provenance["stages"])
            .iter()
            .map(|s| {
                assert_eq!(rows(&s["before_chain"]).len(), 1);
                let chain = &s["before_chain"][0];
                let records: Vec<_> = rows(&chain["attributes"][s["stat"].as_str().unwrap()])
                    .iter()
                    .filter(|r| r["record"]["type"] == "BASE")
                    .cloned()
                    .collect();
                let reads: Vec<_> = rows(&s["reads"])
                    .iter()
                    .map(|r| {
                        let mut r = r.clone();
                        r.as_object_mut().unwrap().remove("before_chain");
                        r.as_object_mut().unwrap().remove("before_output");
                        r
                    })
                    .collect();
                json!({"index":s["index"],"pass":s["pass"],"stat":s["stat"],"value":s["value"],
                "depth":chain["depth"],"parent_kind":chain["parent_kind"],
                "base_records":records,"queries":s["queries"],"reads":reads})
            })
            .collect();
        modes.insert(
            mode.into(),
            json!({"class_id":m["class_id"],
            "original_actor_caller":provenance["original_actor_caller"],
            "exact_player_output_table":provenance["exact_player_output_table"],"stages":stages}),
        );
    }
    json!({"case_index":index,"name":case["name"],"xml_sha256":case["xml_sha256"],
        "selected":case["original"]["selected"],"source_selection":case["original"]["state"]["selected"],"modes":modes})
}

// Inspect one already-captured graph table; this does not interpret source code
// or evaluate modifier tags. Exact graph shapes are checked below.
fn table_field(graph: &Value, table: usize, key: Value) -> Option<&Value> {
    let row = &rows(&graph["tables"])[table - 1];
    assert_eq!(row["id"], table);
    let found: Vec<_> = rows(&row["entries"])
        .iter()
        .filter(|e| e["key"] == key)
        .collect();
    assert!(found.len() <= 1);
    found.first().map(|e| &e["value"])
}
fn field<'a>(graph: &'a Value, name: &str) -> Option<&'a Value> {
    assert_eq!(graph["root"], json!({"id":1,"kind":"table"}));
    table_field(graph, 1, json!({"kind":"string","value":name}))
}
fn scalar(kind: &str, value: &Value) -> Value {
    json!({"kind":kind,"value":value})
}

fn check_stage(stage: &Value, build: usize, position: usize) -> usize {
    let attr = position % 3;
    assert_eq!(stage["index"], position + 1);
    assert_eq!(stage["pass"], position / 3 + 1);
    assert_eq!(stage["stat"], ATTRIBUTES[attr]);
    assert_eq!(stage["depth"], 0);
    assert_eq!(stage["parent_kind"], "false");
    let queries = rows(&stage["queries"]);
    assert_eq!(queries.len(), 3);
    for (query, contribution) in queries.iter().zip(["BASE", "INC", "MORE"]) {
        assert_eq!(query["contribution"], contribution);
        assert_eq!(
            query["kind"],
            if contribution == "MORE" {
                "more_internal"
            } else {
                "sum_internal"
            }
        );
        assert_eq!(query["name"], stage["stat"]);
        assert_eq!(query["store_depth"], 0);
        assert_eq!(query["actual_return_local"], true);
        assert_eq!(query["original_context_is_player"], true);
        assert_eq!(query["flags"], 0);
        assert_eq!(query["keyword_flags"], 0);
        assert_eq!(query["cfg"], json!({"root":{"kind":"nil"},"tables":{}}));
    }
    assert!(
        queries
            .windows(2)
            .all(|q| q[0]["sequence"].as_u64() < q[1]["sequence"].as_u64())
    );
    assert_eq!(
        queries[0]["result"].as_f64(),
        Some(GETTER_RESULTS[build][attr])
    );

    let records = rows(&stage["base_records"]);
    assert_eq!(records.len(), COUNTS[build][attr]);
    let mut raw_sum = 0.;
    let mut prior_index = 0;
    let mut prior_lane = 0;
    let (mut items, mut zeros, mut bonuses) = (0, 0, 0);
    let mut tagged = Vec::new();
    for (position, row) in records.iter().enumerate() {
        let index = row["index"].as_u64().unwrap();
        assert!(index > prior_index);
        prior_index = index;
        let r = &row["record"];
        assert_eq!(r["name"], stage["stat"]);
        assert_eq!(r["type"], "BASE");
        assert_eq!(r["flags"], 0);
        assert_eq!(r["keyword_flags"], 0);
        assert_eq!(r["value"]["root"]["kind"], "number");
        assert!(rows(&r["value"]["tables"]).is_empty());
        let value = r["value"]["root"]["value"].as_f64().unwrap();
        assert!(value.is_finite() && value.fract() == 0. && (0.0..=33.0).contains(&value));
        raw_sum += value;
        let graph = &r["full"];
        for (name, kind, v) in [
            ("name", "string", &r["name"]),
            ("type", "string", &r["type"]),
            ("source", "string", &r["source"]),
            ("flags", "number", &r["flags"]),
            ("keywordFlags", "number", &r["keyword_flags"]),
            ("value", "number", &r["value"]["root"]["value"]),
        ] {
            assert_eq!(field(graph, name), Some(&scalar(kind, v)));
        }
        let source = r["source"].as_str().unwrap();
        let lane = if source == "Base" {
            assert_eq!(position, 0);
            0
        } else if source.starts_with("Item:") {
            items += 1;
            1
        } else if source.starts_with("Tree:") {
            2
        } else {
            assert!(source.starts_with("Many Sources:"));
            bonuses += 1;
            if value == 0. {
                assert_eq!(source, "Many Sources:^x88FFFF0% Amulet Bonus Effect");
                assert_eq!(
                    field(graph, "sourceSlot"),
                    Some(&json!({"kind":"string","value":"Amulet"}))
                );
                zeros += 1;
            } else {
                assert_eq!(build, 2);
                let (expected_source, expected_slot, expected_value) = match attr {
                    0 => ("Many Sources:^x88FFFF28% Ring 2 Bonus Effect", "Ring 2", 8.),
                    // The retained PoB record actually says Ring 2 here. Keep
                    // this mismatch visible; it is not native slot authority.
                    2 => ("Many Sources:^x88FFFF25% Ring 1 Bonus Effect", "Ring 2", 7.),
                    _ => panic!("unexpected nonzero bonus"),
                };
                assert_eq!(source, expected_source);
                assert_eq!(value, expected_value);
                assert_eq!(
                    field(graph, "sourceSlot"),
                    Some(&json!({"kind":"string","value":expected_slot}))
                );
                assert_eq!(position + 1, records.len());
            }
            3
        };
        // Observed source lanes, not an adopted native order policy.
        assert!(lane >= prior_lane);
        prior_lane = lane;
        if lane == 1 || lane == 3 {
            assert_eq!(field(graph, "sourceSlot").unwrap()["kind"], "string");
        } else {
            assert!(field(graph, "sourceSlot").is_none());
        }
        let tags = r["tag_count"].as_u64().unwrap();
        assert!(tags <= 1);
        assert_eq!(rows(&graph["tables"]).len(), 1 + tags as usize);
        let expected_fields = 6 + usize::from(lane == 1 || lane == 3) + tags as usize;
        assert_eq!(rows(&graph["tables"][0]["entries"]).len(), expected_fields);
        if tags == 1 {
            assert_eq!(build, 1);
            assert_eq!(lane, 2);
            assert_eq!(
                table_field(graph, 1, json!({"kind":"number","value":1})),
                Some(&json!({"kind":"table","id":2}))
            );
            assert_eq!(rows(&graph["tables"][1]["entries"]).len(), 2);
            assert_eq!(
                table_field(graph, 2, json!({"kind":"string","value":"type"})),
                Some(&json!({"kind":"string","value":"Condition"}))
            );
            let condition = table_field(graph, 2, json!({"kind":"string","value":"var"})).unwrap();
            assert_eq!(condition["kind"], "string");
            tagged.push((source, condition["value"].as_str().unwrap()));
        }
    }
    assert_eq!(raw_sum, RAW_SUMS[build][attr]);
    assert_eq!(items, ITEM_COUNTS[build][attr]);
    assert_eq!(zeros, ZERO_COPIES[build][attr]);
    assert_eq!(bonuses, zeros + usize::from(build == 2 && attr != 1));
    let expected_tags: &[(&str, &str)] = if build != 1 {
        &[]
    } else {
        match attr {
            0 => &[("Tree:4238", "WeaponSet1")],
            1 => &[
                ("Tree:42658", "WeaponSet1"),
                ("Tree:4238", "WeaponSet1"),
                ("Tree:63566", "WeaponSet1"),
                ("Tree:10909", "WeaponSet2"),
            ],
            _ => &[],
        }
    };
    assert_eq!(tagged, expected_tags);
    let reads = rows(&stage["reads"]);
    assert_eq!(reads.len(), tagged.len());
    for (r, (_, condition)) in reads.iter().zip(tagged) {
        assert_eq!(r["kind"], "get_condition");
        assert_eq!(r["name"], condition);
        assert_eq!(r["store_depth"], 0);
        assert_eq!(r["actual_call_observed"], true);
        assert_eq!(r["direct_return_value_claim"], false);
        assert_eq!(r["replay_inputs_unchanged"], true);
        assert_eq!(r["cfg"], json!({"root":{"kind":"nil"},"tables":{}}));
        assert_eq!(r["no_mod"], json!({"present":false}));
        let mut entries = vec![];
        if condition == "WeaponSet2" {
            entries.push(
                json!({"key":{"kind":"number","value":1},"value":{"kind":"boolean","value":true}}),
            );
        } else {
            assert_eq!(condition, "WeaponSet1");
        }
        entries
            .push(json!({"key":{"kind":"string","value":"n"},"value":{"kind":"number","value":1}}));
        // Pin actual same-state original replay (nil or true). Do not turn this
        // diagnostic into a tag evaluator or compute the authoritative getter.
        assert_eq!(
            r["same_state_original_replay"],
            json!({"root":{"kind":"table","id":1},"tables":[{"id":1,"entries":entries}]})
        );
    }
    records.len()
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
    let bytes = fs::read(root().join(VECTORS)).unwrap();
    assert!(bytes.len() < 4 * 1024 * 1024 && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(!bytes.contains(&b'\r'));
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["scope"], scope());
    assert_eq!(v["source_pins"].as_object().unwrap().len(), 2);
    for (path, pin) in v["source_pins"].as_object().unwrap() {
        check_pin(path, pin);
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
    let manifest_text = fs::read_to_string(root().join(MANIFEST))
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
    assert_eq!(rows(&v["source_files"]).len(), 12);
    let mut paths = BTreeSet::new();
    for pin in rows(&v["source_files"]) {
        let path = pin["path"].as_str().unwrap();
        assert!(paths.insert(path));
        assert!(
            rows(&manifest["files"])
                .iter()
                .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
        );
        if full {
            let text = fs::read_to_string(root().join("vendor/path-of-building-poe2").join(path))
                .unwrap()
                .replace("\r\n", "\n");
            assert_eq!(pin["sha256"], hash(text.as_bytes()), "source: {path}");
        }
    }
    for pin in rows(&v["metadata"]["files"]) {
        assert!(rows(&v["source_files"]).contains(pin));
    }
    assert_eq!(rows(&v["xml_pins"]).len(), 5);
    assert_eq!(rows(&v["cases"]).len(), 5);
    let mut records = 0;
    for (i, case) in rows(&v["cases"]).iter().enumerate() {
        let path = format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            i + 1
        );
        let pin = &v["xml_pins"][i];
        assert_eq!(pin["path"], path);
        check_pin(&path, pin);
        assert_eq!(case["xml_sha256"], pin["sha256"]);
        assert_eq!(
            v["metadata"]["original_sources"][i]["sha256"],
            pin["sha256"]
        );
        assert_eq!(v["metadata"]["original_sources"][i]["path"], path);
        assert_eq!(case["case_index"], i);
        assert_eq!(case["name"], format!("original-{:02}", i + 1));
        for mode in ["MAIN", "CALCS"] {
            let m = &case["modes"][mode];
            assert_eq!(m["original_actor_caller"], true);
            assert_eq!(m["exact_player_output_table"], true);
            assert_eq!(rows(&m["stages"]).len(), 6);
            for (j, stage) in rows(&m["stages"]).iter().enumerate() {
                records += check_stage(stage, i, j);
            }
        }
    }
    assert_eq!(records, 876);
    assert_eq!(rows(&v["source_reports"]).len(), 2);
    for (i, pin) in rows(&v["source_reports"]).iter().enumerate() {
        assert_eq!(
            pin["path"],
            format!(
                "runs/owned-attribute-pipeline-source-05/source-jit-{}.json",
                if i == 0 { "on" } else { "off" }
            )
        );
        assert_eq!(pin["bytes"], 103_745_090);
        assert_eq!(
            pin["sha256"],
            "1db2e0d271c386c932e268a4c9a04e522aa57a052d7ec9de4badf57c2e330764"
        );
    }
    if !full {
        return;
    }
    for pin in rows(&v["source_reports"]) {
        let path = pin["path"].as_str().unwrap();
        check_pin(path, pin);
        let mut report = read(path);
        let cases = report.as_object_mut().unwrap().remove("cases").unwrap();
        report.as_object_mut().unwrap().remove("native_cases");
        assert_eq!(report, v["metadata"]);
        assert_eq!(rows(&cases).len(), 7);
        assert_eq!(report["case_count"], 7);
        assert_eq!(report["complete_loads_per_jit"], 14);
        assert!(
            cases[4]["original"] == cases[6]["original"],
            "independent restoration differs"
        );
        for case in rows(&cases) {
            assert!(
                without_capture(case["original"].clone())
                    == without_capture(case["unhooked"].clone()),
                "unhooked scalar/identity inventory differs: {}",
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
            "retained BASE reprojection differs"
        );
    }
}
