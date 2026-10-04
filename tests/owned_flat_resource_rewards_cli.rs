//! Four exact reward producers publish without changing any selected input or import identity.
#[path = "support/owned_flat_resource_rewards.rs"]
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
fn flat_resource_reward_packet_has_complete_single_effect_source_proofs() {
    family::check_authored();
}

#[test]
#[ignore = "requires the exact checked Minion-Life release and authenticated configuration reward source reports"]
fn publish_flat_resource_rewards_preserving_all_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RESOURCE_REWARDS_PRIOR").expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RESOURCE_REWARDS_OUTPUT")
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
    assert_eq!(prior.input().recipe.schema, next.input().recipe.schema);
    assert_eq!(prior.input().recipe.routing, next.input().recipe.routing);
    let before_receipt = json!(prior.receipt());
    let after_receipt = json!(next.receipt());
    for field in [
        "registry",
        "definitions",
        "routing",
        "mapping",
        "roles",
        "normalization",
        "rewards",
        "items",
        "item_source",
        "tree",
        "source",
        "query_sets",
        "query_rows",
        "query_policy_bytes",
    ] {
        assert!(
            !before_receipt[field].is_null(),
            "expected receipt identity {field}"
        );
        assert_eq!(
            before_receipt[field], after_receipt[field],
            "unchanged {field}"
        );
    }
    assert_ne!(prior.receipt().input, next.receipt().input);
    assert_eq!(
        next.input().recipe.rules.owners.len(),
        prior.input().recipe.rules.owners.len() + 4
    );
    let bindings: Value = family::read("bindings.json");
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
        rebind_definitions: false,
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
            let preset = draft["draft"]["choice_presets"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["id"] == selection["build"]["choices"])
                .unwrap();
            assert_eq!(preset["rewards"]["completion"]["kind"], "complete");
            let selected_rewards = preset["rewards"]["members"].as_array().unwrap();
            assert_eq!(
                selected_rewards.len(),
                17,
                "the entire original selected reward list survives"
            );
            for binding in bindings["rewards"].as_array().unwrap() {
                let matching: Vec<_> = draft["draft"]["rewards"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|row| {
                        selected_rewards.contains(&row["id"])
                            && row["definition"]
                                == json!({"kind":"known","value":binding["reward"]})
                    })
                    .collect();
                assert_eq!(
                    matching.len(),
                    1,
                    "each completed Reward owner occurs once in the actual selected choice preset"
                );
                assert_eq!(
                    matching[0]["parameters"],
                    json!({"members":[],"completion":{"kind":"complete"}})
                );
                assert_eq!(
                    selected_rewards
                        .iter()
                        .filter(|id| **id == matching[0]["id"])
                        .count(),
                    1
                );
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
            "complete_original_builds": 0, "evaluation_bundle_added": false,
            "new_complete_reward_owners": 4, "new_programs": 4, "new_definitions": 0,
            "selected_original05_reward_owners_each_once": true,
            "schema_registry_import_and_query_identities_unchanged": true,
            "published_reducers": 0, "published_receivers": 0, "whole_resource_parity": false
        }),
    );
}
