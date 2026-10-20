//! Shared authenticated comparison for unchanged and deliberately derived inputs.
use super::{read, sha};
use crate::preservation;
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde_json::{Value, json};
use std::{fs, path::Path};
#[derive(Debug, PartialEq)]
pub struct Output {
    pub draft: Value,
    pub sidecar: Value,
    pub report: Value,
}
fn canonical(v: &mut Value, lineage: &Value) {
    match v {
        Value::Object(o) => {
            if let Some(l) = o.get_mut("lineage") {
                assert_eq!(l, lineage);
                *l = json!("checked-test-lineage")
            }
            for v in o.values_mut() {
                canonical(v, lineage)
            }
        }
        Value::Array(a) => {
            for v in a {
                canonical(v, lineage)
            }
        }
        _ => {}
    }
}
pub fn checked_output(
    dir: &Path,
    package: &Path,
    case: usize,
    release: &StagedOwnedRelease,
    xml: &[u8],
) -> Output {
    let mut draft = read(dir.join("draft.json"));
    let bytes = fs::read(dir.join("sidecar.json")).unwrap();
    let mut sidecar: Value = serde_json::from_slice(&bytes).unwrap();
    let mut report = read(dir.join("report.json"));
    preservation::authenticate(&sidecar, dir, package, case, release);
    assert_eq!(sidecar["source_sha256"], sha(xml));
    assert_eq!(sidecar["source_bytes"], xml.len());
    assert_eq!(report["source"]["sha256"], sha(xml));
    assert_eq!(report["source"]["bytes"], xml.len());
    assert_eq!(report["allocator_before"], sidecar["allocator_before"]);
    assert_eq!(report["allocator_after"], sidecar["allocator_after"]);
    assert_eq!(report["draft_digest"], sidecar["draft"]);
    assert_eq!(report["sidecar_sha256"], sha(&bytes));
    assert_eq!(report["sidecar_bytes"], bytes.len());
    assert_eq!(
        report["output"]["directory"],
        json!(fs::canonicalize(dir).unwrap())
    );
    let lineage = sidecar["allocator_before"]["lineage"].clone();
    assert!(lineage.is_string());
    // Every removed field is authenticated above against real bytes/typed release.
    for key in ["policy", "tree_policy", "draft"] {
        sidecar[key] = Value::Null
    }
    for key in ["draft_digest", "sidecar_sha256", "sidecar_bytes"] {
        report[key] = Value::Null
    }
    report["output"]["directory"] = Value::Null;
    for v in [&mut draft, &mut sidecar, &mut report] {
        canonical(v, &lineage)
    }
    Output {
        draft,
        sidecar,
        report,
    }
}
