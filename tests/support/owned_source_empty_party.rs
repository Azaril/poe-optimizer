//! Full pinned source authority for the childless fresh Party presentation frame.
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/source-empty-party")
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn facts() -> Value {
    serde_json::from_slice(&fs::read(data().join("source-facts.json")).unwrap()).unwrap()
}
pub fn authenticate(facts: &Value) {
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
        sha(&fs::read(data().join("../configuration-accounting/evidence/pob/LICENSE.md")).unwrap()),
        facts["upstream_license"]["sha256"]
    );
    assert_eq!(
        facts["destinations"],
        json!([
            "All",
            "Party Member Stats",
            "Aura",
            "Curse",
            "Warcry Skills",
            "Link Skills",
            "EnemyConditions",
            "EnemyMods"
        ])
    );
    assert_eq!(
        facts["defaults"],
        json!({"destination":"All","append":false,"ShowAdvanceTools":false})
    );
    assert_eq!(
        facts["scope"],
        json!({"unique_ordinary_root_leaf":true,"child_records":false,"native_inventory_completion":false,"party_payload_delivery":false,"import_exportParty":false,"future_explicit_user_actions":false})
    );
    let files = [
        "src/Classes/PartyTab.lua",
        "src/Classes/DropDownControl.lua",
        "src/Modules/Build.lua",
    ];
    let archives = facts["archives"].as_array().unwrap();
    assert_eq!(archives.len(), files.len());
    let mut sources = std::collections::BTreeMap::new();
    for (row, path) in archives.iter().zip(files) {
        assert_eq!(row["path"], path);
        let archive = format!("evidence/{}.gz", path.rsplit('/').next().unwrap());
        assert_eq!(row["archive"], archive);
        let stored = data().join(archive);
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
            .find(|r| r["path"] == path)
            .unwrap();
        assert_eq!(row["sha256"], entry["sha256"]);
        sources.insert(path, String::from_utf8(bytes).unwrap());
    }
    let spans = [
        (
            files[0],
            16,
            514,
            "fresh_constructor_and_explicit_import_controls",
        ),
        (files[0], 516, 573, "load"),
        (files[0], 575, 716, "save_and_display"),
        (files[1], 20, 65, "dropdown_constructor"),
        (files[1], 142, 157, "selection_without_callback"),
        (files[2], 34, 635, "fresh_build_construction_and_saver_load"),
    ];
    assert_eq!(facts["excerpts"].as_array().unwrap().len(), spans.len());
    for (row, (path, first, last, role)) in facts["excerpts"].as_array().unwrap().iter().zip(spans)
    {
        assert_eq!(
            (
                row["path"].as_str().unwrap(),
                row["first"].as_u64().unwrap(),
                row["last"].as_u64().unwrap(),
                row["role"].as_str().unwrap()
            ),
            (path, first, last, role)
        );
        let body = sources[path]
            .lines()
            .skip(first as usize - 1)
            .take((last - first + 1) as usize)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(row["sha256"], sha(body.as_bytes()));
    }
}
