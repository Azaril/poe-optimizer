//! One-shot data authoring only. No maintained path is written.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_draft::{
        EquipmentDraft, GeneratedSkillInputBindingDraft, ItemDraft, PresetUsageBindingDraft,
    },
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

const DATA: &str = "data/owned/poe2/3887ae68/generated-firebolt-dispositions";
const OLD_REVIEW: &str = "e19c90d5fb3a3cac549e6a19095711b8d3643e2f6d17e4cec92a3309feb4ec94";
const OLD_BINDINGS: &str = "4206a1fd027da710478a789acfd00277f54a21d5f2465d2f1ffdc84d23c67924";
const BEFORE: &str = "6f58eba00e71984264123419f51114f375bdce23440eaa4ff3ffa4999e2beb11";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn encode(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}
fn relative(root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    assert!(!path.as_os_str().is_empty());
    assert!(
        path.components()
            .all(|part| matches!(part, Component::Normal(_)))
    );
    let result = root.join(path).canonicalize().unwrap();
    assert!(result.starts_with(root));
    result
}
fn normalized(bytes: &[u8], encoding: &str) -> Vec<u8> {
    match encoding {
        "raw" => bytes.to_vec(),
        "utf8-lf" => String::from_utf8(bytes.to_vec())
            .unwrap()
            .replace("\r\n", "\n")
            .into_bytes(),
        other => panic!("unsupported pinned encoding: {other}"),
    }
}
fn pin(path: &str, bytes: &[u8], encoding: &str) -> Value {
    json!({"path":path,"sha256":sha(bytes),"bytes":bytes.len(),"encoding":encoding})
}
fn verify_pin(root: &Path, value: &Value) -> (PathBuf, Vec<u8>) {
    let path = relative(root, value["path"].as_str().unwrap());
    let raw = fs::read(&path).unwrap();
    let bytes = normalized(&raw, value["encoding"].as_str().unwrap());
    assert_eq!(sha(&bytes), value["sha256"]);
    assert_eq!(bytes.len(), value["bytes"]);
    (path, raw)
}
fn token(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn lineages(value: &mut Value, from: &Value, to: &Value) -> usize {
    match value {
        Value::Object(fields) => {
            let mut count = 0;
            if let Some(current) = fields.get_mut("lineage") {
                assert_eq!(current, from, "foreign lineage cannot be normalized");
                *current = to.clone();
                count += 1;
            }
            count
                + fields
                    .values_mut()
                    .map(|child| lineages(child, from, to))
                    .sum::<usize>()
        }
        Value::Array(values) => values
            .iter_mut()
            .map(|child| lineages(child, from, to))
            .sum(),
        _ => 0,
    }
}

fn typed<T: DeserializeOwned + Serialize>(value: &Value) -> Value {
    let canonical = json!("checked-test-lineage");
    let temporary = json!(BuildLineage::from_bytes([93; 16]));
    let mut input = value.clone();
    let count = lineages(&mut input, &canonical, &temporary);
    assert!(count > 0);
    let record: T = serde_json::from_value(input).unwrap();
    let mut output = serde_json::to_value(record).unwrap();
    assert_eq!(lineages(&mut output, &temporary, &canonical), count);
    output
}

/// A bounded authoring check, never a production equality/coercion rule.
/// Accept only lossless integer-token -> declared f64-token representation.
fn numeric_representation(before: &Value, after: &Value, path: &str, changes: &mut Vec<Value>) {
    if before == after {
        return;
    }
    match (before, after) {
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "object membership differs at {path}");
            for (key, value) in a {
                numeric_representation(
                    value,
                    b.get(key).unwrap(),
                    &format!("{path}/{}", token(key)),
                    changes,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "array membership differs at {path}");
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                numeric_representation(a, b, &format!("{path}/{index}"), changes);
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            assert!(
                !a.is_f64() && b.is_f64(),
                "not an integer-to-f64 token change at {path}"
            );
            let float = b.as_f64().unwrap();
            assert!(float.is_finite() && float.fract() == 0.0);
            const EXACT: i64 = 1_i64 << 53;
            if let Some(integer) = a.as_i64() {
                assert!((-EXACT..=EXACT).contains(&integer));
                assert_eq!(float, integer as f64);
                assert_eq!(float as i64, integer);
            } else {
                let integer = a.as_u64().unwrap();
                assert!(integer <= EXACT as u64);
                assert_eq!(float, integer as f64);
                assert_eq!(float as u64, integer);
            }
            changes.push(json!({"pointer":path,"before":before,"after":after}));
        }
        _ => panic!("non-numeric representation change at {path}: {before:?} -> {after:?}"),
    }
}
fn typed_records(review: &mut Value) -> Vec<Value> {
    let old = review["original05"]["observations"].clone();
    let mut expected = old.clone();
    expected["bindings"] = typed::<Vec<GeneratedSkillInputBindingDraft>>(&old["bindings"]);
    expected["usages"] = typed::<Vec<PresetUsageBindingDraft>>(&old["usages"]);
    expected["equipment"] = typed::<EquipmentDraft>(&old["equipment"]);
    expected["item"] = typed::<ItemDraft>(&old["item"]);
    let mut changes = Vec::new();
    numeric_representation(&old, &expected, "/original05/observations", &mut changes);
    for change in &changes {
        let field = review
            .pointer_mut(change["pointer"].as_str().unwrap())
            .unwrap();
        assert_eq!(field, &change["before"]);
        *field = change["after"].clone();
    }
    assert_eq!(review["original05"]["observations"], expected);
    assert_eq!(
        typed::<Vec<GeneratedSkillInputBindingDraft>>(&expected["bindings"]),
        expected["bindings"]
    );
    assert_eq!(
        typed::<Vec<PresetUsageBindingDraft>>(&expected["usages"]),
        expected["usages"]
    );
    assert_eq!(
        typed::<EquipmentDraft>(&expected["equipment"]),
        expected["equipment"]
    );
    assert_eq!(typed::<ItemDraft>(&expected["item"]), expected["item"]);
    assert_eq!(expected["modifier"], old["modifier"]);
    assert_eq!(expected["origins"], old["origins"]);
    assert_eq!(expected["archived"], old["archived"]);
    assert_eq!(expected["completion"], old["completion"]);
    changes
}
fn deferred(row: &Value) -> usize {
    row["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|field| {
            field["destinations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["kind"] == "deferred")
        })
        .count()
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        4,
        "usage: <repository-root> <fresh-output-dir> <root-relative-enabled-proof.json> <proof-sha256>"
    );
    let root = Path::new(&args[0]).canonicalize().unwrap();
    let requested = Path::new(&args[1]);
    let requested = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    let parent = requested.parent().unwrap().canonicalize().unwrap();
    assert!(parent.starts_with(root.join("runs").canonicalize().unwrap()));
    let output = parent.join(requested.file_name().unwrap());
    assert!(
        !output.exists(),
        "output must be fresh: {}",
        output.display()
    );

    let review_path = relative(&root, &format!("{DATA}/review.json"));
    let bindings_path = relative(&root, &format!("{DATA}/bindings.json"));
    let review_bytes = fs::read(&review_path).unwrap();
    let bindings_bytes = fs::read(&bindings_path).unwrap();
    assert_eq!(sha(&review_bytes), OLD_REVIEW);
    assert_eq!(sha(&bindings_bytes), OLD_BINDINGS);
    let old_review = read_json(&review_bytes);
    let old_bindings = read_json(&bindings_bytes);
    // preserve_order plus exact encoder identity prevents unrelated formatting churn.
    assert_eq!(encode(&old_review), review_bytes);
    assert_eq!(encode(&old_bindings), bindings_bytes);
    assert_eq!(old_review["predecessor"]["input"], BEFORE);
    assert_eq!(old_bindings["before"], BEFORE);
    assert_eq!(old_bindings["evidence"], OLD_REVIEW);
    assert_eq!(old_bindings["selectors"], old_review["selectors"]);
    assert_eq!(old_review["selectors"].as_array().unwrap().len(), 13);
    assert_eq!(old_bindings["rows"].as_array().unwrap().len(), 2);
    assert_eq!(deferred(&old_bindings["rows"][0]["after"]), 6);
    assert_eq!(deferred(&old_bindings["rows"][1]["after"]), 19);

    let proof_path = relative(&root, &args[2]);
    let proof_bytes = fs::read(&proof_path).unwrap();
    assert_eq!(sha(&proof_bytes), args[3]);
    let proof = read_json(&proof_bytes);
    assert_eq!(proof["schema_version"], 1);
    assert_eq!(proof["scope"], "literal-enabled-true-nonrestricting");
    assert_eq!(proof["source_revision"], REVISION);
    assert!(!proof["files"].as_array().unwrap().is_empty());
    let mut read_set = vec![
        (review_path, review_bytes.clone()),
        (bindings_path, bindings_bytes.clone()),
        (proof_path, proof_bytes.clone()),
    ];
    for entry in proof["files"].as_array().unwrap() {
        read_set.push(verify_pin(&root, entry));
    }
    // Ordinary verification uses the bundled manifest, not the vendor checkout.
    // Source execution and body acquisition remain the separate test's job.
    let manifest_name = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
    let manifest_pins: Vec<_> = proof["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|pin| pin["path"] == manifest_name)
        .collect();
    assert_eq!(manifest_pins.len(), 1);
    let manifest_path = relative(&root, manifest_name);
    let manifest_bytes = &read_set
        .iter()
        .find(|(path, _)| path == &manifest_path)
        .unwrap()
        .1;
    let manifest = read_json(manifest_bytes);
    assert_eq!(manifest["upstream_revision"], REVISION);
    assert!(!proof["source_files"].as_array().unwrap().is_empty());
    let mut source_paths = std::collections::BTreeSet::new();
    for source in proof["source_files"].as_array().unwrap() {
        assert!(source_paths.insert(source["path"].as_str().unwrap()));
        assert_eq!(source["encoding"], "utf8-lf");
        let matches: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|pin| pin["path"] == source["path"])
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["sha256"], source["sha256"]);
        assert_eq!(matches[0]["bytes"], source["bytes"]);
    }

    let mut review = old_review.clone();
    let mut bindings = old_bindings.clone();
    let representation = typed_records(&mut review);
    assert!(
        !representation.is_empty(),
        "expected the authenticated legacy integral-f64 snapshot"
    );
    let mut producer_changes = Vec::new();
    for entry in review["producer_consumer_files"].as_array_mut().unwrap() {
        assert_eq!(entry["encoding"], "utf8-lf");
        let path = relative(&root, entry["path"].as_str().unwrap());
        let raw = fs::read(&path).unwrap();
        let bytes = normalized(&raw, "utf8-lf");
        let before = entry.clone();
        entry["sha256"] = json!(sha(&bytes));
        entry["bytes"] = json!(bytes.len());
        producer_changes.push(json!({"before":before,"after":entry}));
        read_set.push((path, raw));
    }

    let selectors = [
        json!({"kind":"group_attribute","name":"enabled"}),
        json!({"kind":"occurrence_attribute","name":"enabled"}),
    ];
    let mut destination_changes = Vec::new();
    for (selector, parameter) in selectors
        .iter()
        .zip(["def.000000000000332c", "def.000000000000332d"])
    {
        let fields = bindings["rows"][0]["after"]["fields"]
            .as_array_mut()
            .unwrap();
        let indices: Vec<_> = fields
            .iter()
            .enumerate()
            .filter(|(_, row)| &row["field"] == selector)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(indices.len(), 1);
        let index = indices[0];
        let destinations = fields[index]["destinations"].as_array_mut().unwrap();
        assert_eq!(destinations.len(), 2);
        let usage: Vec<_> = destinations
            .iter()
            .filter(|d| d["kind"] == "usage_parameter")
            .collect();
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0]["policy"]["key"], "def.000000000000332b");
        assert_eq!(usage[0]["parameter"]["slot"]["key"], parameter);
        let deferred: Vec<_> = destinations
            .iter()
            .enumerate()
            .filter(|(_, d)| d["kind"] == "deferred")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(deferred.len(), 1);
        let destination = deferred[0];
        let before = destinations[destination].clone();
        destinations[destination] = json!({"kind":"input_disposition"});
        destination_changes.push(json!({"pointer":format!("/rows/0/after/fields/{index}/destinations/{destination}"),"before":before,"after":destinations[destination]}));
        assert!(!review["selectors"].as_array().unwrap().contains(selector));
        review["selectors"]
            .as_array_mut()
            .unwrap()
            .push(selector.clone());
    }
    bindings["selectors"] = review["selectors"].clone();
    review["field_authority"]["group"]
        .as_array_mut()
        .unwrap()
        .push(json!("enabled"));
    review["field_authority"]["occurrence"]
        .as_array_mut()
        .unwrap()
        .push(json!("enabled"));
    review["field_authority"]["enabled_gate"] = json!(
        "raw and decoded literal true only; retain exact typed participation destinations; no activation or topology claim"
    );
    review["unchanged_deferred_fields"]
        .as_array_mut()
        .unwrap()
        .retain(|field| !selectors.contains(field));
    review["scope"] = json!(
        "fifteen checked generated InputDisposition destinations and two reporting Deferred removals; enabled=true is nonrestricting and retains typed participation; no completion"
    );
    assert!(review.get("enabled_gate").is_none());
    review["enabled_gate"] = proof;
    assert_eq!(deferred(&bindings["rows"][0]["after"]), 4);
    assert_eq!(deferred(&bindings["rows"][1]["after"]), 19);
    assert_eq!(bindings["rows"][1], old_bindings["rows"][1]);
    assert_eq!(review["reporting"], old_review["reporting"]);
    assert_eq!(review["predecessor"], old_review["predecessor"]);
    assert_eq!(
        review["unresolved_alternative_before"],
        old_review["unresolved_alternative_before"]
    );
    assert_eq!(
        review["unresolved_alternative_after"],
        old_review["unresolved_alternative_after"]
    );
    let next_review = encode(&review);
    bindings["evidence"] = json!(sha(&next_review));
    let next_bindings = encode(&bindings);

    let mut inverse = review.clone();
    inverse.as_object_mut().unwrap().remove("enabled_gate");
    for key in [
        "scope",
        "producer_consumer_files",
        "selectors",
        "field_authority",
        "unchanged_deferred_fields",
    ] {
        inverse[key] = old_review[key].clone();
    }
    for change in &representation {
        let field = inverse
            .pointer_mut(change["pointer"].as_str().unwrap())
            .unwrap();
        assert_eq!(field, &change["after"]);
        *field = change["before"].clone();
    }
    assert_eq!(inverse, old_review, "whole review semantic inverse");
    assert_eq!(encode(&inverse), review_bytes, "whole review byte inverse");
    let mut inverse = bindings.clone();
    inverse["selectors"] = old_bindings["selectors"].clone();
    inverse["evidence"] = old_bindings["evidence"].clone();
    for change in &destination_changes {
        let field = inverse
            .pointer_mut(change["pointer"].as_str().unwrap())
            .unwrap();
        assert_eq!(field, &change["after"]);
        *field = change["before"].clone();
    }
    assert_eq!(inverse, old_bindings, "whole bindings semantic inverse");
    assert_eq!(
        encode(&inverse),
        bindings_bytes,
        "whole bindings byte inverse"
    );

    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read(source_dir.join("src/main.rs")).unwrap();
    let cargo = fs::read(source_dir.join("Cargo.toml")).unwrap();
    let report = json!({
        "schema_version":1,
        "scope":"reviewed two-field data authoring and exact typed snapshot representation",
        "authorer":{"rust_sha256":sha(&source),"cargo_sha256":sha(&cargo)},
        "predecessor_input":BEFORE,
        "enabled_proof":pin(&args[2], &proof_bytes, "raw"),
        "source_execution_by_authorer":false,
        "validation_by_authorer":"data inverses and file pins only; no Cargo tests, CLI publication or source execution",
        "source_proof_status":review["enabled_gate"]["status"],
        "producer_pins":producer_changes,
        "integer_to_declared_f64_representation":representation,
        "destination_changes":destination_changes,
        "live_deferred":{"before":6,"after":4},
        "alternative_deferred":{"before":19,"after":19},
        "selectors":{"before":13,"after":15},
        "reporting_transition_preserved":true,
        "whole_byte_inverses":true,
        "files":[
            {"path":format!("{DATA}/review.json"),"before":pin("originals/review.json",&review_bytes,"raw"),"after":pin("files/review.json",&next_review,"raw")},
            {"path":format!("{DATA}/bindings.json"),"before":pin("originals/bindings.json",&bindings_bytes,"raw"),"after":pin("files/bindings.json",&next_bindings,"raw")}
        ],
        "claims":{"retired_issues":0,"complete_builds":0,"native_rules_changed":false,"private_actual_receipts_observed":false}
    });
    for (path, raw) in &read_set {
        assert_eq!(
            &fs::read(path).unwrap(),
            raw,
            "input changed before output: {}",
            path.display()
        );
    }
    // All checks precede any creation; only the fresh, resolved runs directory is writable.
    fs::create_dir(&output).unwrap();
    fs::create_dir(output.join("files")).unwrap();
    fs::create_dir(output.join("originals")).unwrap();
    for (name, bytes) in [
        ("files/review.json", next_review),
        ("files/bindings.json", next_bindings),
        ("originals/review.json", review_bytes),
        ("originals/bindings.json", bindings_bytes),
        ("report.json", encode(&report)),
    ] {
        fs::write(output.join(name), &bytes).unwrap();
        assert_eq!(fs::read(output.join(name)).unwrap(), bytes);
    }
    println!("Wrote reviewed data candidate only: {}", output.display());
    println!(
        "report SHA-256: {}",
        sha(&fs::read(output.join("report.json")).unwrap())
    );
}
