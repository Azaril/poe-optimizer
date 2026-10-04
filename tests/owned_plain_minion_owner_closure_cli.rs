//! Two proved default passive owners close without changing original inputs.
#[path = "support/owned_plain_minion_owner_closure.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn publish(input: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn two_default_node_closures_have_complete_evidence_and_unchanged_programs() {
    family::check_authored();
}

#[test]
#[ignore = "requires the checked Amulet-copy release and authenticated passive source evidence"]
fn publish_two_passive_owner_closures_preserving_all_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_OWNER_CLOSURE_PRIOR")
            .expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_OWNER_CLOSURE_OUTPUT")
            .expect("fresh publication output"),
    );
    assert!(!out.exists());
    let old_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    assert!(next.evaluation().is_none());
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(prior.input().recipe.registry, next.input().recipe.registry);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let files = release::inventory(&package);
    assert_eq!(files.len(), 18);
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &[],
        selected_before: [113, 116, 108, 121, 11],
        selected_after: [113, 116, 108, 121, 11],
        rebind_definitions: true,
    };
    let mut originals = Vec::new();
    for case in 1..=5 {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&source).unwrap();
        release::normalize(
            &prior_path,
            &source,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &source,
            case,
            &out.join(format!("original-{case:02}")),
        );
        let result = preservation::compare_original(case, &bytes, &comparison);
        assert_eq!(result["physical_lists_completed"], 0);
        if case == 5 {
            let directory = out.join("original-05");
            let selection = selected::selection(&bytes, &directory);
            let draft: Value =
                serde_json::from_slice(&fs::read(directory.join("draft.json")).unwrap()).unwrap();
            let preset = draft["draft"]["allocation_presets"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == selection["build"]["allocations"])
                .unwrap();
            assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
            let selected_ids = preset["allocations"]["members"].as_array().unwrap();
            let bindings: Value = family::read("bindings.json");
            for node in bindings["nodes"].as_array().unwrap() {
                let found = draft["draft"]["allocations"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|a| {
                        selected_ids.contains(&a["id"])
                            && a["node"] == json!({"kind":"known","value":node["node"]})
                    })
                    .count();
                assert_eq!(found, 1, "each closed owner belongs to the real selection");
            }
        }
        assert_eq!(fs::read(&source).unwrap(), bytes);
        originals.push(result);
    }
    assert_eq!(old_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before": prior.receipt().input, "after": next.receipt().input,
            "originals": originals, "queries": 110, "artifacts": 18,
            "rebuild_byte_identical": true, "prior_unchanged": true,
            "completed_default_passive_owners": 2, "new_programs": 0,
            "new_definitions": 0, "complete_original_builds": 0,
            "evaluation_bundle_added": false
        }),
    );
}
