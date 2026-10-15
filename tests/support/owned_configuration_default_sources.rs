//! Offline source-body, dispatch receipt and exact constructor checks.
#[allow(dead_code)]
#[path = "owned_mana_supplier_sources.rs"]
mod suppliers;
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};
pub const CONFIGURATION_ARCHIVES: &[&str] = &[
    "src/Modules/ConfigOptions.lua",
    "src/Modules/CalcDefence.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/EditControl.lua",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
pub fn retained_gzip(entry: &Value, limit: usize) -> Vec<u8> {
    let stored = fs::read(root().join(entry["file"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&stored), entry["artifact_sha256"]);
    let expected = entry["bytes"].as_u64().unwrap() as usize;
    assert!(expected <= limit);
    let mut bytes = Vec::new();
    GzDecoder::new(stored.as_slice())
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= limit);
    assert_eq!(bytes.len(), expected);
    assert_eq!(hash(&bytes), entry["sha256"]);
    bytes
}
pub fn check(
    e: &Value,
    required_archives: &[&str],
    required_spans: &[(&str, usize, usize, &str)],
    source_name: &str,
    default: f64,
) {
    authenticate(e, required_archives, required_spans);
    let acquisition = acquisition();
    for report in [&acquisition.jit_off, &acquisition.jit_on] {
        let catalogue = report["state"]["configuration_catalogue"]
            .as_array()
            .unwrap();
        let matches: Vec<_> = catalogue
            .iter()
            .filter(|r| r["fields"]["var"] == source_name)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["fields"]["type"], "countAllowZero");
        assert_eq!(matches[0]["default_placeholder"].as_f64(), Some(default));
        assert!(matches[0].get("apply").is_none());
        let configs = report["state"]["configuration"].as_array().unwrap();
        assert_eq!(configs.len(), 2);
        for config in configs {
            assert!(config["input"].get(source_name).is_none());
            assert_eq!(config["placeholder"][source_name].as_f64(), Some(default));
            assert_eq!(config["default_state"][source_name], 0);
        }
    }
}
pub fn acquisition() -> suppliers::SupplierAcquisition {
    suppliers::acquisition_fixture().unwrap()
}
/// One complete skill catalogue exceeds the ordinary source-file allowance.
/// This is an offline evidence bound, independent of native package limits.
pub fn archive_limit(path: &str) -> usize {
    match path {
        "src/Data/Skills/other.lua" => 2 * 1024 * 1024,
        _ => 1024 * 1024,
    }
}
/// Authenticate source identity, complete archived bytes, exact body spans and
/// shared constructor dependencies. Individual controls retain separate dispatch
/// checks rather than weakening the countAllowZero checks above.
pub fn authenticate(
    e: &Value,
    required_archives: &[&str],
    required_spans: &[(&str, usize, usize, &str)],
) {
    let manifest: Value = serde_json::from_slice(
        &fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(e["source_revision"], manifest["upstream_revision"]);
    assert_eq!(
        e["archives"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["path"].as_str().unwrap())
            .collect::<Vec<_>>(),
        required_archives
    );
    assert_eq!(e["spans"].as_array().unwrap().len(), required_spans.len());
    for (span, (path, first, last, role)) in e["spans"]
        .as_array()
        .unwrap()
        .iter()
        .zip(required_spans.iter().copied())
    {
        assert_eq!(
            (&span["path"], &span["first"], &span["last"], &span["role"]),
            (&json!(path), &json!(first), &json!(last), &json!(role))
        );
    }
    assert_eq!(e["constructor_dependencies"].as_array().unwrap().len(), 4);
    for (dependency, name) in e["constructor_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .zip([
            "source-comparison.json",
            "configuration-review.json",
            "configuration-bodies.json",
            "configuration-auxiliary-bodies.json",
        ])
    {
        assert_eq!(
            dependency["file"],
            format!("data/owned/poe2/3887ae68/mana-supplier-sources/{name}")
        );
        assert_eq!(
            hash(&fs::read(root().join(dependency["file"].as_str().unwrap())).unwrap()),
            dependency["sha256"]
        );
    }
    assert_eq!(
        e["license"],
        "data/owned/poe2/3887ae68/configuration-accounting/evidence/pob/LICENSE.md"
    );
    assert!(root().join(e["license"].as_str().unwrap()).is_file());
    for archive in e["archives"].as_array().unwrap() {
        let limit = archive_limit(archive["path"].as_str().unwrap());
        let stored = fs::read(root().join(archive["file"].as_str().unwrap())).unwrap();
        assert_eq!(hash(&stored), archive["artifact_sha256"]);
        let bytes = match archive["encoding"].as_str().unwrap() {
            "utf8" => stored,
            "gzip" => retained_gzip(archive, limit),
            "source_collection" => {
                let collection: Value = serde_json::from_slice(&stored).unwrap();
                let matching: Vec<_> = collection["sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|s| s["path"] == archive["path"])
                    .collect();
                assert_eq!(matching.len(), 1);
                matching[0]["source"].as_str().unwrap().as_bytes().to_vec()
            }
            _ => panic!("reviewed source encoding"),
        };
        assert!(bytes.len() <= limit);
        assert_eq!(bytes.len(), archive["bytes"]);
        assert_eq!(hash(&bytes), archive["sha256"]);
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["path"] == archive["path"] && p["sha256"] == archive["sha256"])
        );
        let text = String::from_utf8(bytes).unwrap();
        for span in e["spans"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["path"] == archive["path"])
        {
            let first = span["first"].as_u64().unwrap() as usize;
            let last = span["last"].as_u64().unwrap() as usize;
            assert!(first > 0 && first <= last && last <= text.split('\n').count());
            let body = text
                .split('\n')
                .skip(first - 1)
                .take(last - first + 1)
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(body, span["body"]);
            assert_eq!(hash(body.as_bytes()), span["sha256"]);
        }
    }
}
