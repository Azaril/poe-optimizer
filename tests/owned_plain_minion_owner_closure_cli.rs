//! Proved default passive owners close without changing original inputs.
#[path = "support/owned_command_cooldown.rs"]
mod command;
#[path = "support/owned_plain_minion_owner_closure.rs"]
mod family;
#[path = "support/owned_growing_swarm.rs"]
mod growing_swarm;
#[path = "support/owned_plain_minion_life_passives.rs"]
mod life;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_passive_refinement_publication.rs"]
mod passive_publication;
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
fn two_default_node_closures_have_complete_evidence_and_unchanged_programs() {
    family::check_authored();
}

#[test]
fn four_life_node_closures_preserve_damage_and_have_complete_source_evidence() {
    life::check_authored();
}

#[test]
fn command_receiving_preserves_prior_damage_and_partial_action_mechanics() {
    command::check_authored();
}

#[test]
fn growing_swarm_keeps_both_source_effects_and_existing_conditional_action_rules() {
    growing_swarm::check_authored();
}

struct Checkpoint {
    prior_env: &'static str,
    output_env: &'static str,
    stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    bindings: Value,
    node_field: &'static str,
    new_definitions: usize,
    new_programs: usize,
    selected_counts: [usize; 5],
}

#[test]
#[ignore = "requires the checked Amulet-copy release and authenticated passive source evidence"]
fn publish_two_passive_owner_closures_preserving_all_five_originals() {
    check_publication(Checkpoint {
        prior_env: "POE_OPTIMIZER_TEST_MINION_OWNER_CLOSURE_PRIOR",
        output_env: "POE_OPTIMIZER_TEST_MINION_OWNER_CLOSURE_OUTPUT",
        stage: family::stage,
        bindings: family::read("bindings.json"),
        node_field: "node",
        new_definitions: 0,
        new_programs: 0,
        selected_counts: [113, 116, 108, 121, 11],
    });
}

#[test]
#[ignore = "requires the checked passive-closure release and authenticated Life source evidence"]
fn publish_four_life_owner_closures_preserving_all_five_originals() {
    check_publication(Checkpoint {
        prior_env: "POE_OPTIMIZER_TEST_MINION_LIFE_PRIOR",
        output_env: "POE_OPTIMIZER_TEST_MINION_LIFE_OUTPUT",
        stage: life::stage,
        bindings: life::read("bindings.json"),
        node_field: "node",
        new_definitions: 1,
        new_programs: 4,
        selected_counts: [113, 116, 108, 121, 11],
    });
}

#[test]
#[ignore = "requires the permanent-reward release and authenticated Command/actor source evidence"]
fn publish_command_receiving_preserving_all_five_originals() {
    check_publication(Checkpoint {
        prior_env: "POE_OPTIMIZER_TEST_COMMAND_COOLDOWN_PRIOR",
        output_env: "POE_OPTIMIZER_TEST_COMMAND_COOLDOWN_OUTPUT",
        stage: command::stage,
        bindings: command::read("bindings.json"),
        node_field: "definition",
        new_definitions: 7,
        new_programs: 12,
        selected_counts: [113, 116, 108, 121, 11],
    });
}

#[test]
#[ignore = "requires the exact reward-recovery02 release and authenticated Growing Swarm source evidence"]
fn publish_growing_swarm_preserving_all_five_originals() {
    let bindings: Value = growing_swarm::read("bindings.json");
    // This packet has one whole source owner. The shared publication census
    // accepts the same explicit node list used by the earlier multi-node gates.
    let selected_nodes = json!({"nodes":[{"definition":bindings["node"]}]});
    check_publication(Checkpoint {
        prior_env: "POE_OPTIMIZER_TEST_GROWING_SWARM_PRIOR",
        output_env: "POE_OPTIMIZER_TEST_GROWING_SWARM_OUTPUT",
        stage: growing_swarm::stage,
        bindings: selected_nodes,
        node_field: "definition",
        new_definitions: 4,
        new_programs: 7,
        selected_counts: [114, 117, 109, 122, 11],
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
    if checkpoint.new_definitions == 0 {
        assert_eq!(prior.input().recipe.registry, next.input().recipe.registry);
    }
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
        selected_before: checkpoint.selected_counts,
        selected_after: checkpoint.selected_counts,
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
            for node in checkpoint.bindings["nodes"].as_array().unwrap() {
                let found = draft["draft"]["allocations"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|a| {
                        selected_ids.contains(&a["id"])
                            && a["node"]
                                == json!({"kind":"known","value":node[checkpoint.node_field]})
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
            "completed_default_passive_owners": checkpoint.bindings["nodes"].as_array().unwrap().len(),
            "new_programs": checkpoint.new_programs,
            "new_definitions": checkpoint.new_definitions, "complete_original_builds": 0,
            "evaluation_bundle_added": false
        }),
    );
}
