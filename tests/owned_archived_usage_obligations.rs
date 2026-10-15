//! A source-discovered Pending obligation must preserve unrelated real builds.
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path, path::PathBuf};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn equal(a: &Value, b: &Value, context: &str) {
    assert!(a == b, "{context}: structural mismatch");
}
fn canonical_equal(mut a: Value, mut b: Value, context: &str) {
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    equal(&a, &b, context);
}
fn watermark(v: &Value) -> u64 {
    u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap()
}
fn lineages(v: &Value, expected: &Value) {
    match v {
        Value::Object(fields) => {
            if let Some(actual) = fields.get("lineage") {
                assert_eq!(actual, expected, "foreign source or local-ID lineage");
            }
            for child in fields.values() {
                lineages(child, expected);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                lineages(row, expected);
            }
        }
        _ => {}
    }
}
fn authenticate_files(directory: &Path, xml: &[u8], draft: &Value, sidecar: &Value) -> Value {
    let lineage = &draft["draft"]["allocator"]["lineage"];
    lineages(draft, lineage);
    lineages(sidecar, lineage);
    assert_eq!(sidecar["allocator_after"], draft["draft"]["allocator"]);
    assert_eq!(
        sidecar["source_sha256"],
        format!("{:x}", Sha256::digest(xml))
    );
    assert_eq!(sidecar["source_bytes"], xml.len());
    let report = read(directory.join("report.json"));
    assert_eq!(report["draft_digest"], sidecar["draft"]);
    assert_eq!(report["sidecar_schema_version"], sidecar["schema_version"]);
    let bytes = fs::read(directory.join("sidecar.json")).unwrap();
    assert_eq!(
        report["sidecar_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(report["sidecar_bytes"], bytes.len());
    for field in ["allocator_before", "allocator_after"] {
        assert_eq!(report[field], sidecar[field]);
    }
    assert_eq!(report["source"]["sha256"], sidecar["source_sha256"]);
    assert_eq!(report["source"]["bytes"], xml.len());
    for field in ["occurrences", "origin_rows"] {
        assert_eq!(
            report["source"][field],
            sidecar["origins"].as_array().unwrap().len()
        );
    }
    report
}
fn config_origins(sidecar: &Value, draft: &Value) -> Vec<usize> {
    let issue = json!({"kind":"issue","value":draft["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"]});
    sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["links"].as_array().unwrap().contains(&issue))
        .map(|row| row["source"]["ordinal"].as_u64().unwrap() as usize)
        .collect()
}

#[test]
#[ignore = "requires retained pre-fix imports, their authenticated package and fresh output"]
fn archived_usage_discovery_preserves_all_five_selected_builds() {
    let baseline = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ARCHIVED_USAGE_BASELINE").expect("baseline"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ARCHIVED_USAGE_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let package = baseline.join("package");
    let inventory = release::inventory(&package);
    let staged = release::load(&package);
    assert_eq!(staged.receipt().query_rows, 110);
    fs::create_dir_all(out.join("package")).unwrap();
    for (name, bytes) in staged.artifacts() {
        fs::write(out.join("package").join(name), bytes).unwrap();
    }
    assert_eq!(release::inventory(&out.join("package")), inventory);
    let mut results = Vec::new();
    for case in 1..=5 {
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&xml).unwrap();
        let old = baseline.join(format!("original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        let replay = out.join(format!("replay-original-{case:02}"));
        let before_inventory = release::inventory(&old);
        release::normalize(&package, &xml, case, &new);
        release::normalize(&package, &xml, case, &replay);
        let a = read(old.join("draft.json"));
        let original_b = read(new.join("draft.json"));
        let sa = read(old.join("sidecar.json"));
        let original_sb = read(new.join("sidecar.json"));
        preservation::authenticate(&sa, &old, &package, case, &staged);
        preservation::authenticate(&original_sb, &new, &package, case, &staged);
        let old_report = authenticate_files(&old, &bytes, &a, &sa);
        let mut new_report = authenticate_files(&new, &bytes, &original_b, &original_sb);
        authenticate_files(
            &replay,
            &bytes,
            &read(replay.join("draft.json")),
            &read(replay.join("sidecar.json")),
        );
        for name in ["draft.json", "sidecar.json"] {
            let mut first = read(new.join(name));
            let mut second = read(replay.join(name));
            // Fresh CLI runs deliberately allocate different lineages. The
            // sidecar binds each full draft before this test compares meaning.
            if name == "sidecar.json" {
                preservation::authenticate(&second, &replay, &package, case, &staged);
                first["draft"] = Value::Null;
                second["draft"] = Value::Null;
            }
            canonical_equal(first, second, "deterministic fresh normalization");
        }
        let before_config = config_origins(&sa, &a);
        let after_config = config_origins(&original_sb, &original_b);
        let mut b = original_b.clone();
        let mut sb = original_sb.clone();
        let expected_new = usize::from(case == 5);
        let mut inserted = None;
        assert_eq!(
            watermark(&b["draft"]["allocator"]),
            watermark(&a["draft"]["allocator"]) + expected_new as u64
        );
        if case == 5 {
            let old_usage = &a["draft"]["skill_presets"]["members"][0]["intent"]["usage"];
            let new_usage = &original_b["draft"]["skill_presets"]["members"][0]["intent"]["usage"];
            assert_eq!(
                old_usage,
                &json!({"members":[],"completion":{"kind":"complete"}})
            );
            assert_eq!(new_usage["members"], json!([]));
            assert_eq!(new_usage["completion"]["kind"], "pending");
            assert_eq!(
                new_usage["completion"]["code"],
                "usage-preferences-not-converted"
            );
            let issue = json!({"kind":"issue","value":new_usage["completion"]["id"]});
            inserted =
                Some(u64::from_str_radix(issue["value"]["local"].as_str().unwrap(), 16).unwrap());
            let configuration =
                &original_b["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"];
            let mut expected_links =
                vec![json!({"kind":"issue","value":configuration}), issue.clone()];
            expected_links.sort_by_key(Value::to_string);
            for ordinal in [174, 175] {
                let mut actual = original_sb["origins"][ordinal]["links"]
                    .as_array()
                    .unwrap()
                    .clone();
                actual.sort_by_key(Value::to_string);
                equal(
                    &json!(actual),
                    &json!(expected_links),
                    "unfinished usage retains independent configuration fallback",
                );
            }
            let attached: Vec<_> = original_sb["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["links"].as_array().unwrap().contains(&issue))
                .map(|row| row["source"]["ordinal"].as_u64().unwrap())
                .collect();
            assert_eq!(
                attached,
                vec![167, 174, 175],
                "only the exact saved preset and pair own the new obligation"
            );
            for row in sb["origins"].as_array_mut().unwrap() {
                row["links"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|link| link != &issue);
            }
            b["draft"]["skill_presets"]["members"][0]["intent"]["usage"] = old_usage.clone();
            // Archive accounting separately needs a checked deferred-usage /
            // selector proof. This release has none for this source family;
            // the selected-source absence fallback is not archive authority.
            assert_eq!(before_config, after_config);
            assert_eq!((before_config.len(), after_config.len()), (32, 32));
        } else {
            assert_eq!(before_config, after_config);
        }
        // Every existing ID still has one injective correspondence, including
        // references issued after the newly discovered obligation.
        b["draft"]["allocator"] = a["draft"]["allocator"].clone();
        let mut ids = BTreeMap::new();
        identity::correspond(&a, &mut b, &mut ids, "whole draft inverse");
        for (new_id, old_id) in &ids {
            let new_id: Value = serde_json::from_str(new_id).unwrap();
            let old_local = u64::from_str_radix(old_id["local"].as_str().unwrap(), 16).unwrap();
            let new_local = u64::from_str_radix(new_id["local"].as_str().unwrap(), 16).unwrap();
            assert_eq!(
                new_local,
                old_local + u64::from(inserted.is_some_and(|n| old_local >= n)),
                "only the new issue may shift later allocation IDs"
            );
        }
        assert_eq!(
            new_report["issue_count"].as_u64().unwrap(),
            old_report["issue_count"].as_u64().unwrap() + expected_new as u64
        );
        new_report["issue_count"] = old_report["issue_count"].clone();
        if expected_new != 0 {
            assert_eq!(
                new_report["issue_codes"]["usage-preferences-not-converted"]
                    .as_u64()
                    .unwrap(),
                old_report["issue_codes"]["usage-preferences-not-converted"]
                    .as_u64()
                    .unwrap()
                    + expected_new as u64
            );
            new_report["issue_codes"]["usage-preferences-not-converted"] =
                old_report["issue_codes"]["usage-preferences-not-converted"].clone();
        }
        for field in [
            "draft_digest",
            "sidecar_sha256",
            "sidecar_bytes",
            "allocator_after",
        ] {
            new_report[field] = old_report[field].clone();
        }
        new_report["output"]["directory"] = old_report["output"]["directory"].clone();
        canonical_equal(
            old_report,
            new_report,
            "only the new issue and authenticated output metadata change",
        );
        identity::relocate(&mut sb, &ids);
        let mut reordered = Vec::new();
        for (old_origin, new_origin) in sa["origins"]
            .as_array()
            .unwrap()
            .iter()
            .zip(sb["origins"].as_array_mut().unwrap())
        {
            if old_origin["links"] == new_origin["links"] {
                continue;
            }
            // The producer now attaches usage before the accounting pass
            // attaches preset/raw/support owners. Preserve the exact four
            // owners and prove this single insertion-order rotation.
            let links = new_origin["links"].as_array().unwrap();
            assert_eq!(links.len(), 4);
            assert_eq!(links[0]["kind"], "issue");
            assert_eq!(links[1]["kind"], "skill_preset");
            let owner = a["draft"]["skill_presets"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|preset| preset["id"] == links[1]["value"])
                .unwrap();
            assert_eq!(
                owner["intent"]["usage"]["completion"]["id"],
                links[0]["value"]
            );
            let restored = json!([links[1], links[2], links[0], links[3]]);
            equal(
                &old_origin["links"],
                &restored,
                "only existing usage link insertion order changes",
            );
            new_origin["links"] = restored;
            reordered.push(old_origin["source"]["ordinal"].as_u64().unwrap());
        }
        assert_eq!(
            reordered,
            if case == 5 {
                vec![
                    185, 186, 190, 191, 246, 247, 251, 252, 269, 270, 349, 350, 351, 352, 353, 354,
                    416, 417, 418, 419, 420, 421,
                ]
            } else {
                vec![]
            }
        );
        sb["allocator_after"] = sa["allocator_after"].clone();
        sb["draft"] = sa["draft"].clone();
        canonical_equal(sa.clone(), sb, "whole sidecar inverse");
        let mut selection = selected::selection(&bytes, &new);
        identity::relocate(&mut selection, &ids);
        equal(
            &selected::selection(&bytes, &old),
            &selection,
            "saved selections preserved",
        );
        let before = selected::finalize_with_definitions(
            &bytes,
            &old,
            &out.join(format!("prior-selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        let after = selected::finalize_with_definitions(
            &bytes,
            &new,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        assert_eq!(after["intent_validation"]["schema_issues"], json!([]));
        let mut issues = after["finalization"]["issues"].clone();
        identity::relocate(&mut issues, &ids);
        equal(
            &before["finalization"]["issues"],
            &issues,
            "selected request obligations preserved",
        );
        assert_eq!(
            issues.as_array().unwrap().len(),
            [106, 117, 109, 123, 4][case - 1]
        );
        assert_eq!(release::inventory(&old), before_inventory);
        results.push(json!({"original":case,"new_usage_obligations":expected_new,"existing_usage_links_reordered":reordered,"configuration_origins_before":before_config.len(),"configuration_origins_after":after_config.len(),"selected_issues":issues.as_array().unwrap().len(),"draft_and_sidecar_inverse":true,"fresh_replay_equal":true}));
    }
    assert_eq!(release::inventory(&package), inventory);
    fs::write(out.join("validation.json"), serde_json::to_vec_pretty(&json!({"release":staged.receipt().input,"queries_preserved":110,"package_bytes_unchanged":true,"cases":results})).unwrap()).unwrap();
}
