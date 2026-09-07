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
fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn failure(output: Output, text: &str) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(text),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn objective(id: &str, unit: &str) -> Value {
    json!({
        "schema_version":1,"objective":{"kind":"scalar","metric":{"actor":"player","id":id},"unit":unit,"direction":"minimize"},
        "constraints":[]
    })
}
fn save(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn saved() -> Value {
    json!({
        "schema_version":2,"status":"experimental_evaluation","source":{"format":"test","xml_sha256":"test-source"},
        "evaluation":{
            "backend":{"id":"custom-native-fixture","implementation_version":"1","rules_revision":"test","source_fingerprint":"test","adapter_fingerprint":"test"},
            "build":{"level":80,"class_name":"Test","ascendancy_name":"None","tree_version":"test","main_socket_group":1,"allocated_nodes":[],"skill_groups":0},
            "context":{"requested":{"selection":null,"encounter":null},"calculation_mode":"test","enemy_level":80,"config_inputs":{},"config_placeholders":{},"player_conditions":{},"enemy_conditions":{}},
            "coverage":{"schema_version":1,"active_skill_set_id":null,"groups":[],"selected_player":null,"selected_minion":null,
              "full_dps":{"included_group_count":0,"selected_group_included":false,"active_skills":[],"reported_contributions":[]},"unresolved_entry_count":0,"tree_connections":[]},
            "measurements":[{"query":{"actor":"player","id":"custom.cost"},"unit":"damage","schema_version":7,"value":{"status":"finite","value":42.0}}],
            "exports":[],"warnings":["Synthetic data only"],"elapsed_ms":1.0,"diagnostic_only":true,"attachments":[]
        }
    })
}

#[test]
fn saved_assessment_is_backend_independent_and_preserves_metric_version() {
    let temp = tempfile::tempdir().unwrap();
    let original = saved();
    save(&temp.path().join("evaluation.json"), &original);
    save(
        &temp.path().join("objective.json"),
        &objective("custom.cost", "damage"),
    );
    // There is no vendor checkout here and this metric does not exist in PoB.
    let report = success(run(
        temp.path(),
        &["assess", "evaluation.json", "--objective", "objective.json"],
    ));
    assert_eq!(report["status"], "diagnostic_objective_assessment");
    assert_eq!(report["evaluation_diagnostic_only"], true);
    assert_eq!(report["backend"], original["evaluation"]["backend"]);
    assert_eq!(report["context"], original["evaluation"]["context"]);
    let assessment = &report["objective_assessment"];
    assert_eq!(assessment["status"], "constraints_satisfied");
    assert_eq!(assessment["objective_score"]["value"], -42.0);
    assert_eq!(assessment["measurements"][0]["schema_version"], 7);
    assert_eq!(
        assessment["measurements"][0],
        original["evaluation"]["measurements"][0]
    );
    failure(
        run(
            temp.path(),
            &[
                "assess",
                "evaluation.json",
                "--objective",
                "objective.json",
                "--output",
                "evaluation.json",
            ],
        ),
        "Output already exists",
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(temp.path().join("evaluation.json")).unwrap())
            .unwrap(),
        original
    );
}

#[test]
fn corrupt_saved_results_and_unrecorded_metrics_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    save(
        &temp.path().join("objective.json"),
        &objective("custom.cost", "damage"),
    );
    let mut cases = Vec::new();
    let mut wrong = saved();
    wrong["schema_version"] = 1.into();
    cases.push((wrong, "schema version 2"));
    let mut wrong = saved();
    wrong["evaluation"]["elapsed_ms"] = (-1).into();
    cases.push((wrong, "elapsed"));
    let mut wrong = saved();
    wrong["evaluation"]["context"]["requested"] = json!({"selection":{"socket_group":0}});
    cases.push((wrong, "positive"));
    let mut wrong = saved();
    wrong["evaluation"]["measurements"].as_array_mut().unwrap().push(json!({"query":{"actor":"player","id":"unused"},"unit":"damage","schema_version":0,"value":{"status":"finite","value":1.0}}));
    cases.push((wrong, "schema versions"));
    for (report, error) in cases {
        save(&temp.path().join("evaluation.json"), &report);
        failure(
            run(
                temp.path(),
                &["assess", "evaluation.json", "--objective", "objective.json"],
            ),
            error,
        );
    }
    save(&temp.path().join("evaluation.json"), &saved());
    save(
        &temp.path().join("objective.json"),
        &objective("absent", "damage"),
    );
    failure(
        run(
            temp.path(),
            &["assess", "evaluation.json", "--objective", "objective.json"],
        ),
        "Unknown objective metric",
    );
}

#[test]
fn evaluation_collects_required_metrics_and_can_be_reassessed_without_recalculation() {
    let temp = tempfile::tempdir().unwrap();
    let mut spec = objective("selected_hit_dps", "damage_per_second");
    spec["constraints"] = json!([{"id":"life","metric":{"actor":"player","id":"life"},"unit":"pool_points","operator":">=","threshold":900.0,"violation_scale":100.0}]);
    let spec_path = temp.path().join("objective.json");
    save(&spec_path, &spec);
    let report = success(run(
        &root(),
        &[
            "evaluate",
            "tests/fixtures/calibration/spark-mapping.xml",
            "--objective",
            spec_path.to_str().unwrap(),
            "--metric",
            "player.mana",
            "--timeout-seconds",
            "60",
        ],
    ));
    assert_eq!(
        report["evaluation"]["measurements"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let assessment = &report["objective_assessment"];
    assert_eq!(assessment["status"], "constraints_violated");
    assert_eq!(assessment["constraints"][0]["shortfall"]["value"], 91.0);
    assert!(assessment["objective_score"]["value"].as_f64().unwrap() < 0.0);
    save(&temp.path().join("evaluation.json"), &report);
    spec["constraints"][0]["threshold"] = 800.0.into();
    save(&spec_path, &spec);
    let reassessed = success(run(
        temp.path(),
        &["assess", "evaluation.json", "--objective", "objective.json"],
    ));
    assert_eq!(
        reassessed["objective_assessment"]["status"],
        "constraints_satisfied"
    );
    assert_eq!(
        reassessed["objective_assessment"]["objective_score"],
        assessment["objective_score"]
    );
    assert_eq!(reassessed["backend"], report["evaluation"]["backend"]);
    spec["objective"]["unit"] = "percent".into();
    save(&spec_path, &spec);
    failure(
        run(
            &root(),
            &[
                "evaluate",
                "tests/fixtures/calibration/spark-mapping.xml",
                "--objective",
                spec_path.to_str().unwrap(),
                "--pob",
                "does-not-exist",
            ],
        ),
        "Wrong unit",
    );
}
