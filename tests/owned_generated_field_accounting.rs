//! Current importer proof cutover against stored pre-cutover originals.
//! No old importer mode is retained merely to reconstruct the comparison.
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path, path::PathBuf};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn issue_codes(value: &Value, codes: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(object) => {
            if object.get("kind") == Some(&json!("pending"))
                && let (Some(id), Some(code)) = (object.get("id"), object.get("code"))
            {
                let key = serde_json::to_string(id).unwrap();
                let code = code.as_str().unwrap().to_owned();
                if let Some(previous) = codes.insert(key, code.clone()) {
                    assert_eq!(previous, code);
                }
            }
            for child in object.values() {
                issue_codes(child, codes);
            }
        }
        Value::Array(array) => {
            for child in array {
                issue_codes(child, codes);
            }
        }
        _ => {}
    }
}
fn issue_code<'a>(link: &Value, codes: &'a BTreeMap<String, String>) -> &'a str {
    assert_eq!(link["kind"], "issue");
    codes
        .get(&serde_json::to_string(&link["value"]).unwrap())
        .expect("issue must still exist in unchanged draft")
}
fn prove_origin_change(old: &Value, new: &Value, draft: &Value) -> Value {
    let mut old_shape = old.clone();
    let mut new_shape = new.clone();
    old_shape.as_object_mut().unwrap().remove("links");
    new_shape.as_object_mut().unwrap().remove("links");
    assert_eq!(old_shape, new_shape, "only provenance links may change");
    let a = old["links"].as_array().unwrap();
    let b = new["links"].as_array().unwrap();
    let removed: Vec<_> = a.iter().filter(|link| !b.contains(link)).collect();
    let added: Vec<_> = b.iter().filter(|link| !a.contains(link)).collect();
    assert_eq!(removed.len(), 1, "one exact configuration fallback");
    assert_eq!(added.len(), 1, "one exact existing usage obligation");
    let mut codes = BTreeMap::new();
    issue_codes(draft, &mut codes);
    assert_eq!(
        issue_code(removed[0], &codes),
        "configuration-roles-not-converted"
    );
    assert_eq!(
        issue_code(added[0], &codes),
        "usage-preferences-not-converted"
    );

    let bindings: Vec<_> = b
        .iter()
        .filter(|link| link["kind"] == "generated_skill_input")
        .collect();
    assert_eq!(bindings.len(), 1, "exact generated input target retained");
    assert!(
        a.contains(bindings[0]),
        "accounting cannot invent raw inputs"
    );
    let preset_id = &bindings[0]["value"]["skill_preset"];
    let presets: Vec<_> = draft["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|preset| &preset["id"] == preset_id)
        .collect();
    assert_eq!(presets.len(), 1);
    let usage = &presets[0]["intent"]["usage"]["completion"];
    assert_eq!(usage["kind"], "pending");
    assert_eq!(usage["code"], "usage-preferences-not-converted");
    assert_eq!(usage["id"], added[0]["value"]);
    assert!(
        b.iter()
            .any(|link| link["kind"] == "skill_preset" && &link["value"] == preset_id)
    );
    json!({"source":new["source"],"removed":removed,"added":added,
        "generated_input":bindings[0]["value"]})
}

#[test]
#[ignore = "requires stored pre-cutover originals and a fresh output directory"]
fn current_accounting_reimports_all_five_without_changing_build_inputs() {
    let baseline = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_ACCOUNTING_BASELINE")
            .expect("stored pre-cutover checkpoint directory"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_ACCOUNTING_OUTPUT")
            .expect("fresh output directory"),
    );
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let package = baseline.join("package");
    let inventory = release::inventory(&package);
    let staged = release::load(&package);
    assert!(staged.normalization().generated_skill_inputs.is_some());
    assert_eq!(staged.receipt().query_rows, 110);
    let mut cases = vec![];
    for case in 1..=5 {
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml_bytes = fs::read(&xml).unwrap();
        let before = baseline.join(format!("original-{case:02}"));
        let after = out.join(format!("original-{case:02}"));
        let old_files = release::inventory(&before);
        let report = release::normalize(&package, &xml, case, &after);
        let sidecar_bytes = fs::read(after.join("sidecar.json")).unwrap();
        assert_eq!(report["sidecar_schema_version"], 21);
        assert_eq!(report["sidecar_bytes"], sidecar_bytes.len());
        assert_eq!(report["sidecar_sha256"], hash(&sidecar_bytes));
        let mut old_draft = read(before.join("draft.json"));
        let mut new_draft = read(after.join("draft.json"));
        let mut old = read(before.join("sidecar.json"));
        let mut new: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
        assert!(matches!(old["schema_version"].as_u64(), Some(19 | 20)));
        assert_eq!(new["schema_version"], 21);
        assert_eq!(new["source_sha256"], hash(&xml_bytes));
        assert_eq!(new["source_bytes"], xml_bytes.len());
        for value in [&mut old_draft, &mut new_draft, &mut old, &mut new] {
            selected::canonical(value);
        }
        assert_eq!(
            old_draft, new_draft,
            "all values, IDs and allocator: original{case}"
        );
        assert_eq!(old["allocator_before"], new["allocator_before"]);
        assert_eq!(old["allocator_after"], new["allocator_after"]);
        let old_origins = old.as_object_mut().unwrap().remove("origins").unwrap();
        let new_origins = new.as_object_mut().unwrap().remove("origins").unwrap();
        for value in [&mut old, &mut new] {
            let object = value.as_object_mut().unwrap();
            object.remove("schema_version");
            // Fresh import lineage changes this digest, while the complete
            // canonical draft comparison above proves actual content equality.
            object.remove("draft");
        }
        assert_eq!(old, new, "all other proof dependencies stay exact");
        let a = old_origins.as_array().unwrap();
        let b = new_origins.as_array().unwrap();
        assert_eq!(a.len(), b.len());
        let changes: Vec<_> = a
            .iter()
            .zip(b)
            .filter(|(a, b)| a != b)
            .map(|(a, b)| prove_origin_change(a, b, &new_draft))
            .collect();
        if case == 5 {
            assert_eq!(changes.len(), 6, "the three reviewed generated pairs");
        }
        let mut old_selection = selected::selection(&xml_bytes, &before);
        let mut new_selection = selected::selection(&xml_bytes, &after);
        selected::canonical(&mut old_selection);
        selected::canonical(&mut new_selection);
        assert_eq!(old_selection, new_selection);
        assert_eq!(old_files, release::inventory(&before));
        assert_eq!(fs::read(&xml).unwrap(), xml_bytes);
        cases.push(
            json!({"original":case,"changes":changes,"sidecar_schema_version":21,
            "sidecar_sha256":report["sidecar_sha256"],"draft_unchanged":true,
            "allocator_unchanged":true,"selection_unchanged":true}),
        );
    }
    assert_eq!(inventory, release::inventory(&package));
    fs::write(
        out.join("validation.json"),
        serde_json::to_vec_pretty(&json!({"package":staged.receipt().input,"cases":cases,
            "query_rows":110,"runtime_data_changed":false,"old_importer_mode_retained":false,
            "complete_original_builds":0}))
        .unwrap(),
    )
    .unwrap();
}
