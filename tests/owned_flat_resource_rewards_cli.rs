//! Exact reward producers publish while preserving every original input and query.
#[path = "support/owned_flat_resource_rewards.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_permanent_reward_effects.rs"]
mod permanent;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_import::owned_release::StagedOwnedRelease;
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
fn permanent_reward_packet_has_complete_twelve_effect_source_proofs() {
    permanent::check_authored();
}

struct Checkpoint {
    prior_env: &'static str,
    output_env: &'static str,
    stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    bindings: Value,
    new_owners: usize,
    new_effects: usize,
    new_definitions: usize,
    rebind_definitions: bool,
}

#[test]
#[ignore = "requires the exact checked Minion-Life release and authenticated configuration reward source reports"]
fn publish_flat_resource_rewards_preserving_all_five_originals() {
    check_publication(Checkpoint {
        prior_env: "POE_OPTIMIZER_TEST_RESOURCE_REWARDS_PRIOR",
        output_env: "POE_OPTIMIZER_TEST_RESOURCE_REWARDS_OUTPUT",
        stage: family::stage,
        bindings: family::read("bindings.json"),
        new_owners: 4,
        new_effects: 4,
        new_definitions: 0,
        rebind_definitions: false,
    });
}

#[test]
#[ignore = "requires the exact checked flat-resource release and authenticated permanent reward source reports"]
fn publish_permanent_reward_effects_preserving_all_five_originals() {
    check_publication(Checkpoint {
        prior_env: "POE_OPTIMIZER_TEST_PERMANENT_REWARDS_PRIOR",
        output_env: "POE_OPTIMIZER_TEST_PERMANENT_REWARDS_OUTPUT",
        stage: permanent::stage,
        bindings: permanent::read("bindings.json"),
        new_owners: 8,
        new_effects: 12,
        new_definitions: 2,
        rebind_definitions: true,
    });
}

fn check_publication(checkpoint: Checkpoint) {
    let prior_path =
        PathBuf::from(std::env::var_os(checkpoint.prior_env).expect("exact predecessor"));
    let out =
        PathBuf::from(std::env::var_os(checkpoint.output_env).expect("fresh publication output"));
    assert!(!out.exists());
    let old_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = (checkpoint.stage)(&prior);
    assert!(next.evaluation().is_none());
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(
        next.input().recipe.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + checkpoint.new_definitions
    );
    assert_eq!(
        prior.input().recipe.schema.slots,
        next.input().recipe.schema.slots
    );
    if checkpoint.rebind_definitions {
        // The staging helper proves the complete schema/rule/registry delta;
        // this inverse permits only exact dependency identity replacements.
        migration_preservation::assert_import_rebindings_only(&prior, &next);
        assert_ne!(prior.receipt().definitions, next.receipt().definitions);
        assert_ne!(prior.receipt().registry, next.receipt().registry);
    } else {
        assert_eq!(prior.input().recipe.registry, next.input().recipe.registry);
        assert_eq!(prior.input().recipe.schema, next.input().recipe.schema);
        assert_eq!(prior.input().recipe.routing, next.input().recipe.routing);
    }
    let before_receipt = json!(prior.receipt());
    let after_receipt = json!(next.receipt());
    let mut unchanged = vec!["source", "query_sets", "query_rows"];
    if !checkpoint.rebind_definitions {
        // Policy byte counts include serialized dependency digests. A schema
        // rebind changes those bytes even when the exact semantic inverse above
        // proves that all policy and query records survive unchanged.
        unchanged.extend([
            "query_policy_bytes",
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
        ]);
    }
    for field in unchanged {
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
        prior.input().recipe.rules.owners.len() + checkpoint.new_owners
    );
    let bindings = &checkpoint.bindings;
    let rewards = bindings["rewards"].as_array().unwrap();
    assert_eq!(rewards.len(), checkpoint.new_owners);
    let mut effect_count = 0;
    let mut reward_ids = std::collections::BTreeSet::new();
    for binding in rewards {
        assert!(reward_ids.insert(binding["reward"]["key"].as_str().unwrap()));
        let subject =
            json!({"kind":"definition","value":{"kind":"reward","value":binding["reward"]}});
        assert!(
            !prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .any(|owner| json!(owner.owner) == subject),
            "new coverage belongs to an existing Reward with no prior rule owner"
        );
        let owners: Vec<_> = next
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|owner| json!(owner.owner) == subject)
            .collect();
        assert_eq!(owners.len(), 1);
        assert!(owners[0].programs.is_complete());
        assert_eq!(owners[0].programs.members.len(), 1);
        let program = &owners[0].programs.members[0];
        assert_eq!(json!(program.id), binding["program"]);
        effect_count += program.effects.len();
    }
    assert_eq!(effect_count, checkpoint.new_effects);
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
        rebind_definitions: checkpoint.rebind_definitions,
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
            for binding in rewards {
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
            "new_complete_reward_owners": checkpoint.new_owners,
            "new_programs": checkpoint.new_owners, "new_effects": checkpoint.new_effects,
            "new_definitions": checkpoint.new_definitions,
            "selected_original05_reward_owners_each_once": true,
            "schema_registry_import_and_query_identities_unchanged": !checkpoint.rebind_definitions,
            "only_checked_dependency_rebindings": checkpoint.rebind_definitions,
            "published_reducers": 0, "published_receivers": 0, "whole_resource_parity": false
        }),
    );
}
