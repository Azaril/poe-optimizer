#![cfg(feature = "pob")]

use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn objective() -> Value {
    json!({
        "schema_version": 1,
        "objective": {
            "kind": "scalar", "metric": {"actor": "player", "id": "selected_hit_dps"},
            "unit": "damage_per_second", "direction": "maximize"
        }
    })
}

fn catalog() -> PathBuf {
    root().join("examples/calibration-catalog.json")
}

fn run(objective: &Path, jobs: usize, evaluations: usize, export: Option<&Path>) -> Output {
    run_catalog(&catalog(), objective, jobs, evaluations, export)
}

fn run_catalog(
    catalog: &Path,
    objective: &Path,
    jobs: usize,
    evaluations: usize,
    export: Option<&Path>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(root())
        .arg("search-calibration")
        .arg("--catalog")
        .arg(catalog)
        .arg("--objective")
        .arg(objective)
        .arg("--jobs")
        .arg(jobs.to_string())
        .arg("--max-evaluations")
        .arg(evaluations.to_string())
        .args(["--timeout-seconds", "90"]);
    if let Some(export) = export {
        command.arg("--export").arg(export);
    }
    command.output().unwrap()
}

fn report(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn serial_and_parallel_calibration_search_agree_and_export_verified_source() {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("objective.json");
    fs::write(&path, serde_json::to_vec(&objective()).unwrap()).unwrap();
    let serial = report(run(&path, 1, 5, None));
    let export = scratch.path().join("winner.xml");
    let parallel = report(run(&path, 2, 5, Some(&export)));
    for result in [&serial, &parallel] {
        assert_eq!(result["schema_version"], 2);
        assert_eq!(result["status"], "experimental_calibration_search");
        assert_eq!(result["scope"], "supplied_build_catalog");
        assert_eq!(result["diagnostic_only"], true);
        assert_eq!(result["search"]["termination"], "finite_domain_processed");
        assert_eq!(result["search"]["statistics"]["evaluations"], 5);
        assert_eq!(
            result["search"]["statistics"]["verification_evaluations"],
            1
        );
        assert_eq!(result["search"]["statistics"]["evaluation_failures"], 0);
        assert_eq!(result["search"]["feasible"].as_array().unwrap().len(), 4);
        assert_eq!(result["search"]["verifications"][0]["consistent"], true);
        assert_eq!(
            result["search"]["verifications"][0]["diagnostic_only"],
            true
        );
        assert_eq!(result["best_verified"]["alternative_id"], "mace-smithing");
        let actual = result["best_verified"]["assessment"]["objective_value"]["value"]
            .as_f64()
            .unwrap();
        assert!(
            (actual - 18.208694).abs() < 1e-8,
            "independent Mace reference: {actual}"
        );
        assert_eq!(result["backend"]["id"], "pob-poe2-mlua");
        assert_eq!(
            result["backend"]["rules_revision"],
            poe_optimizer_pob::runtime::UPSTREAM_REVISION
        );
        assert_eq!(result["alternatives"].as_array().unwrap().len(), 4);
        assert_eq!(result["search"]["budget"]["max_evaluations"], 5);
        for bound in ["max_proposals", "beam_per_status", "archive_size"] {
            assert_eq!(result["search"]["budget"][bound], 4);
        }
        assert_eq!(
            result["candidate_domain"]["kind"],
            "exact_supplied_document_membership"
        );
        assert_eq!(
            result["candidate_domain"]["generic_realization"],
            "unverified"
        );
        assert_eq!(result["candidate_domain"]["game_legality"], "unverified");
        assert_eq!(result["best_verified"]["fresh_numeric_verification"], true);
        assert_eq!(result["best_verified"]["generic_realization"], "unverified");
        let observations = result["realized_observations"].as_array().unwrap();
        assert_eq!(observations.len(), 4);
        assert_eq!(
            observations
                .iter()
                .map(|v| v["successful_evaluations"].as_u64().unwrap())
                .sum::<u64>(),
            5
        );
        for observation in observations {
            assert_eq!(
                observation["requested_xml_sha256"],
                observation["candidate"]["xml_sha256"]
            );
            assert_eq!(observation["generic_realization"], "unverified");
            assert_eq!(observation["game_legality"], "unverified");
            assert!(observation["realized_summary"].is_object());
            assert!(observation["realized_context"].is_object());
            assert!(observation["realized_coverage"].is_object());
            assert_eq!(observation["realized_exports"].as_array().unwrap().len(), 1);
        }
        for alternative in result["alternatives"].as_array().unwrap() {
            let id = alternative["id"].as_str().unwrap();
            let independent: Value = serde_json::from_slice(
                &fs::read(root().join(format!("tests/fixtures/calibration/{id}.reference.json")))
                    .unwrap(),
            )
            .unwrap();
            let ranked = result["search"]["feasible"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["candidate"] == alternative["candidate"])
                .unwrap();
            let actual = ranked["assessment"]["objective_value"]["value"]
                .as_f64()
                .unwrap();
            let expected = independent["metrics"]["TotalDPS"].as_f64().unwrap();
            assert!(
                (actual - expected).abs() < 1e-8,
                "{id}: actual {actual}, reference {expected}"
            );
        }
    }
    assert_eq!(serial["search"]["feasible"], parallel["search"]["feasible"]);
    assert_eq!(
        fs::read(export).unwrap(),
        fs::read(root().join("tests/fixtures/calibration/mace-smithing.xml")).unwrap()
    );
}

#[test]
fn partial_budget_preserves_evidence_and_infeasible_search_omits_export() {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("objective.json");
    fs::write(&path, serde_json::to_vec(&objective()).unwrap()).unwrap();
    let partial = report(run(&path, 2, 2, None));
    assert_eq!(partial["search"]["termination"], "evaluation_budget");
    assert_eq!(partial["search"]["statistics"]["evaluations"], 2);
    assert_eq!(
        partial["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(partial["search"]["feasible"].as_array().unwrap().len(), 1);
    assert!(!partial["best_verified"].is_null());
    let mut impossible = objective();
    impossible["constraints"] = json!([{
        "id": "unreachable_fixture_life", "metric": {"actor": "player", "id": "life"},
        "unit": "pool_points", "operator": ">=", "threshold": 1e9, "violation_scale": 1000.0
    }]);
    fs::write(&path, serde_json::to_vec(&impossible).unwrap()).unwrap();
    let export = scratch.path().join("must-not-exist.xml");
    let infeasible = report(run(&path, 2, 2, Some(&export)));
    assert!(infeasible["best_verified"].is_null());
    assert_eq!(infeasible["search"]["statistics"]["evaluations"], 1);
    assert_eq!(
        infeasible["search"]["statistics"]["verification_evaluations"],
        0
    );
    assert_eq!(
        infeasible["search"]["infeasible"].as_array().unwrap().len(),
        1
    );
    assert_eq!(infeasible["export"]["status"], "not_written");
    assert!(!export.exists());
}

#[test]
fn invalid_policy_limits_and_output_collisions_fail_before_calculation() {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("objective.json");
    let mut invalid = objective();
    invalid["objective"]["metric"]["id"] = "not_a_metric".into();
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let output = run(&path, 1, 5, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unknown objective metric"));
    for (jobs, evaluations) in [(0, 5), (1, 1)] {
        let output = run(&path, jobs, evaluations, None);
        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(2));
    }
    let existing = scratch.path().join("existing.json");
    fs::write(&existing, "keep original").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(root())
        .arg("search-calibration")
        .arg("--catalog")
        .arg(catalog())
        .arg("--objective")
        .arg(&path)
        .arg("--output")
        .arg(&existing)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Output already exists"));
    assert_eq!(fs::read_to_string(existing).unwrap(), "keep original");
    let shared = scratch.path().join("same-destination");
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(root())
        .arg("search-calibration")
        .arg("--catalog")
        .arg(catalog())
        .arg("--objective")
        .arg(path)
        .arg("--output")
        .arg(&shared)
        .arg("--export")
        .arg(&shared)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("different paths"));
    assert!(!shared.exists());
}

#[test]
fn unrelated_caller_documents_use_relative_paths_and_keep_per_document_evidence() {
    let scratch = tempfile::tempdir().unwrap();
    let inputs = scratch.path().join("caller-inputs");
    fs::create_dir(&inputs).unwrap();
    let objective_path = scratch.path().join("objective.json");
    fs::write(&objective_path, serde_json::to_vec(&objective()).unwrap()).unwrap();
    let mut entries = Vec::new();
    let mut documents = std::collections::BTreeMap::new();
    for scenario in ["mapping", "bossing"] {
        let xml = fs::read_to_string(
            root().join(format!("tests/fixtures/calibration/spark-{scenario}.xml")),
        )
        .unwrap()
        .replace(
            "<PathOfBuilding2>",
            "<PathOfBuilding2><!-- caller-owned document, unrelated to the old Mace catalog -->",
        );
        let id = format!("user-spark-{scenario}");
        fs::write(inputs.join(format!("{scenario}.xml")), &xml).unwrap();
        entries.push(json!({"id":id,"path":format!("caller-inputs/{scenario}.xml")}));
        documents.insert(id, xml);
    }
    let manifest = scratch.path().join("catalog.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&json!({"schema_version":1,"builds":entries})).unwrap(),
    )
    .unwrap();
    let export = scratch.path().join("selected.xml");
    let result = report(run_catalog(&manifest, &objective_path, 2, 3, Some(&export)));
    assert_eq!(result["search"]["statistics"]["evaluations"], 3);
    assert_eq!(result["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(result["search"]["feasible"].as_array().unwrap().len(), 2);
    assert_eq!(result["catalog"]["manifest"]["schema_version"], 1);
    assert_eq!(
        result["catalog"]["path"],
        serde_json::to_value(manifest.canonicalize().unwrap()).unwrap()
    );
    assert_eq!(result["search"]["budget"]["max_proposals"], 2);
    assert_eq!(result["search"]["budget"]["beam_per_status"], 2);
    assert_eq!(result["search"]["budget"]["archive_size"], 2);
    let mut contexts = Vec::new();
    for observation in result["realized_observations"].as_array().unwrap() {
        let id = observation["candidate"]["id"].as_str().unwrap();
        let source = documents.get(id).unwrap();
        let decoded = poe_optimizer_import::decode_build(source.as_bytes()).unwrap();
        assert_eq!(observation["requested_xml_sha256"], decoded.sha256);
        assert_eq!(observation["realized_summary"]["class_name"], "Sorceress");
        assert_eq!(
            observation["realized_context"]["requested"],
            json!({"selection":null,"encounter":null})
        );
        assert_eq!(observation["generic_realization"], "unverified");
        contexts.push(observation["realized_context"]["config_inputs"]["enemyIsBoss"].clone());
    }
    assert_ne!(contexts[0], contexts[1]);
    let selected = result["best_verified"]["alternative_id"].as_str().unwrap();
    assert_eq!(fs::read_to_string(export).unwrap(), documents[selected]);
    assert_eq!(result["export"]["source"], "exact_requested_xml");
    assert_eq!(result["export"]["does_not_certify_pob_normalization"], true);
}

#[test]
fn catalog_is_required_and_malformed_catalogs_never_fall_back_to_embedded_inputs() {
    let scratch = tempfile::tempdir().unwrap();
    let objective_path = scratch.path().join("objective.json");
    fs::write(&objective_path, serde_json::to_vec(&objective()).unwrap()).unwrap();
    let missing = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("search-calibration")
        .arg("--objective")
        .arg(&objective_path)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("--catalog"));
    let valid_entry = json!({"id":"caller-build","path":"build.xml"});
    let invalid_cases = [
        (
            json!({"schema_version":2,"builds":[valid_entry.clone()]}),
            "schema_version",
        ),
        (json!({"schema_version":1,"builds":[]}), "1..64"),
        (
            json!({"schema_version":1,"builds":vec![valid_entry.clone();65]}),
            "1..64",
        ),
        (
            json!({"schema_version":1,"builds":[valid_entry.clone(),valid_entry.clone()]}),
            "IDs must be distinct",
        ),
        (
            json!({"schema_version":1,"builds":[{"id":"bad\nid","path":"build.xml"}]}),
            "IDs must be distinct",
        ),
        (
            json!({"schema_version":1,"builds":[{"id":"a".repeat(129),"path":"build.xml"}]}),
            "1..128",
        ),
        (
            json!({"schema_version":1,"builds":[{"id":"okay","path":"bad\npath"}]}),
            "no control characters",
        ),
        (
            json!({"schema_version":1,"builds":[{"id":"okay","path":"a".repeat(4097)}]}),
            "1..4096",
        ),
        (
            json!({"schema_version":1,"builds":[valid_entry.clone()],"fallback":"embedded"}),
            "unknown field",
        ),
        (
            json!({"schema_version":1,"builds":[{"id":"okay","path":"build.xml","extra":true}]}),
            "unknown field",
        ),
        (
            json!({"schema_version":1,"builds":[valid_entry.clone()]}),
            "path",
        ),
    ];
    let manifest = scratch.path().join("invalid.json");
    for (contents, expected) in invalid_cases {
        fs::write(&manifest, serde_json::to_vec(&contents).unwrap()).unwrap();
        let output = run_catalog(&manifest, &objective_path, 1, 2, None);
        assert!(
            !output.status.success(),
            "unexpected acceptance: {contents}"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "{contents}: {error}");
        assert!(
            output.stdout.is_empty(),
            "invalid input must not produce a search report"
        );
    }
    fs::write(&manifest, vec![b' '; 64 * 1024 + 1]).unwrap();
    let oversized = run_catalog(&manifest, &objective_path, 1, 2, None);
    assert!(!oversized.status.success());
    assert!(String::from_utf8_lossy(&oversized.stderr).contains("manifest exceeds"));
    fs::write(
        &manifest,
        serde_json::to_vec(&json!({"schema_version":1,"builds":[valid_entry]})).unwrap(),
    )
    .unwrap();
    fs::write(scratch.path().join("build.xml"), "invalid share-code input").unwrap();
    let invalid_build = run_catalog(&manifest, &objective_path, 1, 2, None);
    assert!(!invalid_build.status.success());
    assert!(
        String::from_utf8_lossy(&invalid_build.stderr)
            .contains("Catalog entry caller-build import")
    );
    fs::write(
        scratch.path().join("build.xml"),
        vec![b' '; poe_optimizer_import::MAX_XML_BYTES + 1],
    )
    .unwrap();
    let large_build = run_catalog(&manifest, &objective_path, 1, 2, None);
    assert!(!large_build.status.success());
    assert!(String::from_utf8_lossy(&large_build.stderr).contains("limit"));
}

#[test]
fn duplicate_xml_and_total_decoded_size_are_rejected_before_comparison() {
    let scratch = tempfile::tempdir().unwrap();
    let objective_path = scratch.path().join("objective.json");
    fs::write(&objective_path, serde_json::to_vec(&objective()).unwrap()).unwrap();
    let xml = fs::read(root().join("tests/fixtures/calibration/spark-mapping.xml")).unwrap();
    fs::write(scratch.path().join("first.xml"), &xml).unwrap();
    fs::write(scratch.path().join("copy.xml"), &xml).unwrap();
    let manifest = scratch.path().join("catalog.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&json!({"schema_version":1,"builds":[
            {"id":"first","path":"first.xml"}, {"id":"copy","path":"copy.xml"}
        ]}))
        .unwrap(),
    )
    .unwrap();
    let duplicate = run_catalog(&manifest, &objective_path, 1, 2, None);
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("Duplicate source XML"));
    let mut entries = Vec::new();
    for index in 0..5 {
        let name = format!("large-{index}.xml");
        let content = format!(
            "<PathOfBuilding2><!--{index}{}--></PathOfBuilding2>",
            "x".repeat(7 * 1024 * 1024)
        );
        fs::write(scratch.path().join(&name), content).unwrap();
        entries.push(json!({"id":format!("large-{index}"),"path":name}));
    }
    fs::write(
        &manifest,
        serde_json::to_vec(&json!({"schema_version":1,"builds":entries})).unwrap(),
    )
    .unwrap();
    let oversized = run_catalog(&manifest, &objective_path, 1, 2, None);
    assert!(!oversized.status.success());
    assert!(
        String::from_utf8_lossy(&oversized.stderr).contains("Catalog decoded XML exceeds 32 MiB"),
        "{}",
        String::from_utf8_lossy(&oversized.stderr)
    );
}
