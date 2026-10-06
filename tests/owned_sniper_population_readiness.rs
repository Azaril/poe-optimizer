//! Checked rule-only partition; source observations and owner coverage do not change.
#[path = "support/owned_sniper_population_readiness.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use family::Partition;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn population_partition_preserves_exact_reachable_dependency_subsequences() {
    family::check_authored();
    let p: Partition = family::read("partition.json");
    assert_eq!(p.facts.indices.reads, [0, 1]);
    assert_eq!(p.requirements.indices.reads, [0, 2]);
    assert_eq!(p.facts.indices.nodes, [0, 1, 3, 6, 7, 8, 9, 10, 11, 12]);
    assert_eq!(p.requirements.indices.nodes, [0, 2, 4, 5]);
    assert_eq!(p.facts.indices.effects, [0, 1, 2]);
    assert_eq!(p.requirements.indices.effects, [3]);
}

#[test]
fn population_partition_rejects_reordered_duplicate_missing_and_extra_rows() {
    let original: Partition = family::read("partition.json");
    let mut cases = Vec::new();
    let mut p = original.clone();
    p.facts.indices.nodes.swap(0, 1);
    p.facts.program.nodes.swap(0, 1);
    cases.push(("reordered original rows", p));
    let mut p = original.clone();
    p.facts.indices.reads.insert(1, 0);
    p.facts
        .program
        .reads
        .insert(1, p.facts.program.reads[0].clone());
    cases.push(("duplicate original index", p));
    let mut p = original.clone();
    p.facts.indices.nodes.pop();
    p.facts.program.nodes.pop();
    cases.push(("missing transitive quality dependency", p));
    let mut p = original.clone();
    p.facts.indices.reads.push(2);
    p.facts.program.reads.push(p.original.reads[2].clone());
    cases.push(("unused character-level input on early program", p));
    let mut p = original.clone();
    p.facts.indices.effects.push(3);
    p.facts.program.effects.push(p.original.effects[3].clone());
    cases.push(("duplicated execution requirement in facts", p));
    for (label, p) in cases {
        assert!(
            std::panic::catch_unwind(|| family::check_partition(&p)).is_err(),
            "{label}"
        );
    }
}

#[test]
fn population_partition_rejects_changed_values_conditions_and_shared_reads() {
    let original: Value = family::read("partition.json");
    for (path, value) in [
        (
            "/facts/program/nodes/6/expression/value/value",
            json!(false),
        ),
        (
            "/requirements/program/effects/0/when",
            json!("level-eligible"),
        ),
        (
            "/requirements/program/effects/0/effect/code",
            json!("changed-requirement"),
        ),
        (
            "/requirements/program/reads/0/source/value/slot/slot/key",
            json!("def.0000000000000014"),
        ),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = value;
        let p: Partition = serde_json::from_value(changed).unwrap();
        assert!(
            std::panic::catch_unwind(|| family::check_partition(&p)).is_err(),
            "{path}"
        );
    }
}

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
#[ignore = "requires checked SNIPER_POPULATION_PRIOR and fresh SNIPER_POPULATION_OUTPUT paths"]
fn publish_population_readiness_preserving_all_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_POPULATION_PRIOR")
            .expect("checked Command Damage predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_POPULATION_OUTPUT")
            .expect("fresh immutable output"),
    );
    assert!(!out.exists(), "evidence output is immutable");
    let before_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
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
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        before_files.keys().collect::<Vec<_>>()
    );
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    let changed: Vec<_> = files
        .iter()
        .filter(|(name, digest)| before_files.get(*name) != Some(*digest))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(changed, ["manifest.json", "release.json", "rules.json"]);
    family::assert_endpoint(&release::load(&package));
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    assert_eq!(
        next.input().provenance.len(),
        prior.input().provenance.len() + 1
    );
    assert_eq!(
        &next.input().provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &[],
        selected_before: [106, 117, 109, 122, 5],
        selected_after: [106, 117, 109, 122, 5],
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
        let row = preservation::compare_original(case, &bytes, &comparison);
        assert_eq!(row["physical_lists_completed"], 0);
        // Independent imports allocate distinct lineages. The shared checker
        // authenticates each unmodified sidecar/draft before comparing every
        // field with only that established lineage canonicalization.
        assert_eq!(fs::read(&source).unwrap(), bytes);
        originals.push(row);
    }
    assert_eq!(before_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
            "queries":110,"artifacts":18,"changed_artifacts":changed,"rebuild_byte_identical":true,
            "definitions_unchanged":true,"prior_unchanged":true,"provenance_preserved":true,
            "full_input_inverse":true,"exact_dependency_partition":true,
            "new_definitions":0,"new_stats":0,"new_effects":0,"removed_effects":0,
            "closed_rule_owners":0,"retired_input_issues":0,"complete_original_builds":0,
            "evaluation_bundle_added":false,"new_source_parity_claim":false
        }),
    );
}
