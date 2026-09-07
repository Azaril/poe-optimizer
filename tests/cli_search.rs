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

fn run(objective: &Path, jobs: usize, evaluations: usize, export: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(root())
        .arg("search-calibration")
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
        assert_eq!(result["schema_version"], 1);
        assert_eq!(result["status"], "experimental_calibration_search");
        assert_eq!(result["scope"], "four_calibrated_mace_fixture_alternatives");
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
