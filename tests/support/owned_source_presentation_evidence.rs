//! Shared bounded archive/span authentication for source-only import evidence.
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn authenticate(
    directory: &Path,
    facts: &Value,
    files: &[(&str, &str)],
    spans: &[(&str, u64, u64, &str)],
) -> BTreeMap<String, String> {
    let manifest_bytes = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("crates/poe-optimizer-pob/data/pob-source-manifest.json"),
    )
    .unwrap();
    assert_eq!(
        sha(&manifest_bytes),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(facts["schema_version"], 1);
    assert_eq!(facts["source_revision"], manifest["upstream_revision"]);
    assert_eq!(facts["source_manifest_sha256"], sha(&manifest_bytes));
    assert_eq!(facts["runtime_observations"], "none");
    assert_eq!(
        facts["upstream_license"],
        json!({"path":"../configuration-accounting/evidence/pob/LICENSE.md","sha256":"22d2d075c1d361971764fbbd1e12e1485bdf35f0769ffac4eca8a79afc60dda8"})
    );
    assert_eq!(
        sha(
            &fs::read(directory.join("../configuration-accounting/evidence/pob/LICENSE.md"))
                .unwrap()
        ),
        facts["upstream_license"]["sha256"]
    );
    let archives = facts["archives"].as_array().unwrap();
    assert_eq!(archives.len(), files.len());
    let mut sources = BTreeMap::new();
    for (row, (path, archive)) in archives.iter().zip(files) {
        assert_eq!(row["path"], *path);
        assert_eq!(row["archive"], *archive);
        let stored = directory.join(archive);
        let compressed = fs::metadata(&stored).unwrap().len();
        assert!(compressed <= 128 * 1024);
        assert_eq!(row["compressed_bytes"], compressed);
        let raw = fs::read(stored).unwrap();
        assert_eq!(row["compressed_sha256"], sha(&raw));
        let mut bytes = Vec::new();
        GzDecoder::new(raw.as_slice())
            .take(256 * 1024 + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 256 * 1024);
        assert_eq!(row["bytes"], bytes.len());
        assert_eq!(row["sha256"], sha(&bytes));
        let entry = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["path"] == *path)
            .unwrap();
        assert_eq!(row["sha256"], entry["sha256"]);
        assert!(
            sources
                .insert(path.to_string(), String::from_utf8(bytes).unwrap())
                .is_none()
        );
    }
    assert_eq!(facts["excerpts"].as_array().unwrap().len(), spans.len());
    for (row, (path, first, last, role)) in facts["excerpts"].as_array().unwrap().iter().zip(spans)
    {
        assert_eq!(
            (&row["path"], &row["first"], &row["last"], &row["role"]),
            (&json!(path), &json!(first), &json!(last), &json!(role))
        );
        let source = &sources[*path];
        assert!(*first > 0 && last >= first && *last as usize <= source.lines().count());
        let body = source
            .lines()
            .skip(*first as usize - 1)
            .take((last - first + 1) as usize)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(row["sha256"], sha(body.as_bytes()));
    }
    sources
}
