//! Project the existing full-source observation, without interpreting Lua.
use flate2::read::GzDecoder;
use poe_optimizer_pob::source as pinned;
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};

const PACKET: &str = "data/owned/poe2/3887ae68/configuration-census/catalogue.json.gz";

fn portable_sources(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(portable_sources),
        Value::Object(fields) => {
            if let Some(source) = fields.get_mut("function_source") {
                let path = source["path"].as_str().unwrap();
                let (_, path) = path
                    .split_once("/vendor/path-of-building-poe2/")
                    .expect("original callback has a pinned source path");
                assert!(pinned::expected_file_sha256(path).is_ok(), "{path}");
                source["path"] = path.into();
            }
            fields.values_mut().for_each(portable_sources);
        }
        _ => {}
    }
}

pub(super) fn check(root: &Path, out: &Path, report: &Value, enabled: bool) {
    // The package stores source facts, not selected build outputs. Every
    // successful original/control load must reproduce the same full catalogue.
    let file = fs::File::open(root.join(PACKET)).unwrap();
    assert!(file.metadata().unwrap().len() <= 128 * 1024);
    let mut bytes = Vec::new();
    GzDecoder::new(file)
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    let expected: Value = serde_json::from_slice(&bytes).unwrap();
    let mut checked = 0;
    for case in report["cases"].as_array().unwrap() {
        if case["available"] != true {
            continue;
        }
        let mut rows = case["state"]["catalogue"].clone();
        portable_sources(&mut rows);
        let actual = json!({
            "source_revision": report["source_revision"],
            "source_hash": report["source_hash"],
            "rows": rows,
        });
        assert_eq!(actual, expected, "catalogue changed in {}", case["name"]);
        checked += 1;
    }
    assert!(checked >= 5, "all originals are checked by the parent law");
    let receipt = json!({
        "source_revision": report["source_revision"],
        "source_hash": report["source_hash"],
        "attempts": report["evidence"]["complete_load_attempts_per_jit"],
        "successful_catalogue_comparisons": checked,
        "catalogue_sha256": super::digest(&bytes),
        "observer_sha256": report["evidence"]["observer_sha256"],
        "native_effect_coverage": false,
    });
    fs::write(
        out.join(format!(
            "configuration-census-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
